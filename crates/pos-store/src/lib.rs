//! Tillstone's local storage: an encrypted SQLite database (SQLCipher).
//!
//! It holds every accepted record (append-only: there is no API to delete
//! one, PLAN §15), quarantined records for review, Book checkpoints for fast
//! startup, and customers' personal data, which never enters the event log
//! (PLAN §4).
//!
//! Durability: WAL mode with `synchronous = FULL`, so a committed record
//! survives power loss (PLAN §13).

use std::path::Path;

use pos_core::{
    book::{Book, Checkpoint, Ingested, Rejection},
    ids::{CustomerId, DeviceId},
    ledger::{self, Record},
};
use rusqlite::{Connection, OptionalExtension, params};

mod personal;

pub use personal::PersonalError;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// The database key is wrong, or the file isn't a Tillstone database.
    WrongKeyOrCorrupt,
    Corrupt(String),
    Personal(PersonalError),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "database error: {e}"),
            Self::WrongKeyOrCorrupt => f.write_str("wrong database key, or the file is damaged"),
            Self::Corrupt(why) => write!(f, "stored data is damaged: {why}"),
            Self::Personal(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// Schema changes are additive and applied in order (PLAN §13). Never edit
/// a shipped migration; add a new one.
const MIGRATIONS: &[&str] = &[
    // 1: records, quarantine, checkpoints, personal data
    "CREATE TABLE records (
        id INTEGER PRIMARY KEY,
        device BLOB NOT NULL,
        seq INTEGER NOT NULL,
        hash BLOB NOT NULL,
        wire BLOB NOT NULL,
        UNIQUE (device, seq)
    );
    CREATE TABLE quarantine (
        id INTEGER PRIMARY KEY,
        wire BLOB NOT NULL,
        reason TEXT NOT NULL,
        received_ms INTEGER NOT NULL
    );
    CREATE TABLE checkpoint (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        last_record_id INTEGER NOT NULL,
        bytes BLOB NOT NULL
    );
    CREATE TABLE customer_keys (
        customer BLOB PRIMARY KEY,
        key BLOB NOT NULL
    );
    CREATE TABLE customers (
        customer BLOB PRIMARY KEY,
        nonce BLOB NOT NULL,
        ciphertext BLOB NOT NULL
    );",
];

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (or creates) an encrypted database. `key` comes from the
    /// platform keystore (PLAN §9) and must be the same every time.
    pub fn open(path: impl AsRef<Path>, key: &[u8; 32]) -> Result<Self> {
        Self::setup(Connection::open(path)?, key)
    }

    /// An encrypted in-memory database, for tests.
    pub fn open_in_memory(key: &[u8; 32]) -> Result<Self> {
        Self::setup(Connection::open_in_memory()?, key)
    }

    fn setup(conn: Connection, key: &[u8; 32]) -> Result<Self> {
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        // A raw 256-bit key, so SQLCipher skips its passphrase derivation.
        conn.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";"))?;
        // Reading anything fails here if the key is wrong.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .map_err(|_| StoreError::WrongKeyOrCorrupt)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;",
        )?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Saves an accepted record. Returns `false` if it was already stored.
    pub fn append(&self, record: &Record) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO records (device, seq, hash, wire) VALUES (?1, ?2, ?3, ?4)",
            params![
                record.header.device.0.as_slice(),
                record.header.seq as i64,
                record.hash.as_slice(),
                record.wire
            ],
        )?;
        Ok(n == 1)
    }

    /// Keeps a refused record for a manager to review. Never silently dropped
    /// (PLAN §6).
    pub fn quarantine(&self, wire: &[u8], reason: &str, now_ms: u64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO quarantine (wire, reason, received_ms) VALUES (?1, ?2, ?3)",
            params![wire, reason, now_ms as i64],
        )?;
        Ok(())
    }

    pub fn quarantined(&self) -> Result<Vec<(Vec<u8>, String)>> {
        let mut stmt = self.conn.prepare("SELECT wire, reason FROM quarantine ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The latest record written by `device` (this device's own log head),
    /// used to continue its log after a restart.
    pub fn last_record_of(&self, device: DeviceId) -> Result<Option<Record>> {
        let wire: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT wire FROM records WHERE device = ?1 ORDER BY seq DESC LIMIT 1",
                [device.0.as_slice()],
                |r| r.get(0),
            )
            .optional()?;
        wire.map(|w| ledger::decode_trusted(&w).map_err(|e| StoreError::Corrupt(e.to_string())))
            .transpose()
    }

    pub fn record_count(&self) -> Result<u64> {
        Ok(self.conn.query_row("SELECT count(*) FROM records", [], |r| r.get::<_, i64>(0))? as u64)
    }

    /// Records a peer is missing, given its version vector (PLAN §6), in
    /// per-device order.
    pub fn records_missing_from(&self, theirs: &[(DeviceId, u64)]) -> Result<Vec<Vec<u8>>> {
        let mut stmt =
            self.conn.prepare("SELECT device, seq, wire FROM records ORDER BY device, seq")?;
        let mut out = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let device: Vec<u8> = row.get(0)?;
            let seq = row.get::<_, i64>(1)? as u64;
            let have = theirs
                .iter()
                .find(|(d, _)| d.0.as_slice() == device.as_slice())
                .map_or(0, |(_, s)| *s);
            if seq > have {
                out.push(row.get(2)?);
            }
        }
        Ok(out)
    }

    /// Saves a checkpoint of `book` covering every record stored so far.
    pub fn save_checkpoint(&self, book: &Book) -> Result<()> {
        let last: i64 =
            self.conn.query_row("SELECT coalesce(max(id), 0) FROM records", [], |r| r.get(0))?;
        self.conn.execute(
            "INSERT INTO checkpoint (id, last_record_id, bytes) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET last_record_id = excluded.last_record_id, bytes = excluded.bytes",
            params![last, book.checkpoint().encode()],
        )?;
        Ok(())
    }

    /// Rebuilds the Book: from the checkpoint if there is one, then replays
    /// newer records. Signatures were verified when records were first
    /// accepted, so stored records are decoded without re-verifying.
    pub fn load_book(&self) -> Result<Book> {
        let cp: Option<(i64, Vec<u8>)> = self
            .conn
            .query_row("SELECT last_record_id, bytes FROM checkpoint WHERE id = 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        let (mut book, after) = match cp {
            Some((last, bytes)) => {
                let cp = Checkpoint::decode(&bytes)
                    .ok_or_else(|| StoreError::Corrupt("checkpoint".into()))?;
                (Book::restore(cp).map_err(|e| StoreError::Corrupt(e.to_string()))?, last)
            }
            None => (Book::new(), 0),
        };
        let mut stmt = self.conn.prepare("SELECT wire FROM records WHERE id > ?1 ORDER BY id")?;
        let mut rows = stmt.query([after])?;
        while let Some(row) = rows.next()? {
            let wire: Vec<u8> = row.get(0)?;
            let record =
                ledger::decode_trusted(&wire).map_err(|e| StoreError::Corrupt(e.to_string()))?;
            book.ingest_verified(record).map_err(|e| StoreError::Corrupt(e.to_string()))?;
        }
        Ok(book)
    }

    /// Ingests a record from a peer into `book` and stores it. Refused
    /// records go to quarantine.
    pub fn receive(
        &self,
        book: &mut Book,
        wire: &[u8],
        now_ms: u64,
    ) -> Result<Result<Ingested, Rejection>> {
        match book.ingest(wire) {
            Ok(outcome) => {
                // Pending records are stored too: they apply later, once their
                // predecessors arrive, and must survive a restart meanwhile.
                if !matches!(outcome, Ingested::Duplicate)
                    && let Ok(record) = ledger::verify_record(wire)
                {
                    self.append(&record)?;
                }
                Ok(Ok(outcome))
            }
            Err(rejection) => {
                self.quarantine(wire, &rejection.to_string(), now_ms)?;
                Ok(Err(rejection))
            }
        }
    }

    pub fn put_customer(&self, customer: CustomerId, details: &[u8]) -> Result<()> {
        personal::put(&self.conn, customer, details).map_err(StoreError::Personal)
    }

    pub fn get_customer(&self, customer: CustomerId) -> Result<Option<Vec<u8>>> {
        personal::get(&self.conn, customer).map_err(StoreError::Personal)
    }

    /// Erases a customer's personal data on this device. Pair it with a
    /// `CustomerErased` event so every peer does the same.
    pub fn erase_customer(&self, customer: CustomerId) -> Result<()> {
        personal::erase(&self.conn, customer).map_err(StoreError::Personal)
    }
}
