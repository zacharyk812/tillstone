use pos_core::{
    book::Book,
    catalog::{RoundingScope, TaxRate},
    events::{BusinessCreated, ConfigChanged, ConfigRecord, Event},
    ids::{CustomerId, LocationId, TaxId},
    ledger::{self, Context, DeviceLog},
    money::{Currency, Rounding},
};
use pos_store::{Store, StoreError};

const KEY: [u8; 32] = [3; 32];

fn ctx() -> Context {
    Context { location: LocationId::from_u128(1), staff: None, approved_by: None, training: false }
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tillstone-store-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("tillstone.db")
}

/// A founder device creating a business and some configuration.
fn history(n_taxes: u128) -> (DeviceLog, Vec<ledger::Record>) {
    let mut log = DeviceLog::new(&[1; 32]);
    let mut records = vec![log.append(
        1_000,
        ctx(),
        &Event::BusinessCreated(BusinessCreated {
            name: "Corner Café & Books".into(),
            currency: Currency::USD,
            owner_key: ledger::owner_public(&[9; 32]),
            first_location: LocationId::from_u128(1),
        }),
    )];
    for i in 0..n_taxes {
        records.push(log.append(
            2_000 + i as u64,
            ctx(),
            &Event::ConfigChanged(ConfigChanged {
                record: ConfigRecord::Tax(TaxRate {
                    id: TaxId::from_u128(i),
                    name: format!("Tax {i}"),
                    rate_bp: 500,
                    inclusive: false,
                    compound: false,
                    rounding: Rounding::HalfUp,
                    scope: RoundingScope::PerLine,
                }),
            }),
        ));
    }
    (log, records)
}

fn store_all(store: &Store, book: &mut Book, records: &[ledger::Record]) {
    for r in records {
        store.receive(book, &r.wire, 0).unwrap().unwrap();
    }
}

#[test]
fn records_survive_reopening_and_rebuild_the_same_book() {
    let path = temp_path("reopen");
    let (_, records) = history(5);
    let mut book = Book::new();
    {
        let store = Store::open(&path, &KEY).unwrap();
        store_all(&store, &mut book, &records);
        // No explicit close: dropping is like the app being killed.
    }
    let store = Store::open(&path, &KEY).unwrap();
    assert_eq!(store.record_count().unwrap(), 6);
    let loaded = store.load_book().unwrap();
    assert_eq!(loaded.config(), book.config());
    assert_eq!(loaded.version_vector(), book.version_vector());
}

#[test]
fn checkpoint_plus_newer_records() {
    let store = Store::open_in_memory(&KEY).unwrap();
    let (_, records) = history(6);
    let mut book = Book::new();
    store_all(&store, &mut book, &records[..4]);
    store.save_checkpoint(&book).unwrap();
    store_all(&store, &mut book, &records[4..]);
    let loaded = store.load_book().unwrap();
    assert_eq!(loaded.config(), book.config());
    assert_eq!(loaded.version_vector(), book.version_vector());
}

#[test]
fn the_wrong_key_cant_open_the_database() {
    let path = temp_path("wrong-key");
    {
        let store = Store::open(&path, &KEY).unwrap();
        let (_, records) = history(1);
        store_all(&store, &mut Book::new(), &records);
    }
    assert!(matches!(Store::open(&path, &[4; 32]), Err(StoreError::WrongKeyOrCorrupt)));
}

#[test]
fn the_file_on_disk_is_encrypted() {
    let path = temp_path("encrypted");
    {
        let store = Store::open(&path, &KEY).unwrap();
        let (_, records) = history(3);
        store_all(&store, &mut Book::new(), &records);
        store
            .put_customer(CustomerId::from_u128(1), b"Jordan Example, jordan@example.com")
            .unwrap();
    }
    let mut bytes = std::fs::read(&path).unwrap();
    if let Ok(wal) = std::fs::read(path.with_extension("db-wal")) {
        bytes.extend(wal);
    }
    let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
    assert!(!contains(b"SQLite format 3"), "header must be encrypted");
    assert!(!contains("Corner Caf".as_bytes()));
    assert!(!contains(b"jordan@example.com"));
}

#[test]
fn refused_records_are_quarantined_not_dropped() {
    let store = Store::open_in_memory(&KEY).unwrap();
    let (_, records) = history(1);
    let mut tampered = records[0].wire.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    let mut book = Book::new();
    assert!(store.receive(&mut book, &tampered, 5).unwrap().is_err());
    let q = store.quarantined().unwrap();
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].0, tampered);
    assert_eq!(store.record_count().unwrap(), 0);
}

#[test]
fn duplicates_are_stored_once() {
    let store = Store::open_in_memory(&KEY).unwrap();
    let (_, records) = history(2);
    let mut book = Book::new();
    store_all(&store, &mut book, &records);
    store_all(&store, &mut book, &records);
    assert_eq!(store.record_count().unwrap(), 3);
}

#[test]
fn records_missing_from_a_peer() {
    let store = Store::open_in_memory(&KEY).unwrap();
    let (log, records) = history(4);
    store_all(&store, &mut Book::new(), &records);
    let missing = store.records_missing_from(&[(log.device(), 2)]).unwrap();
    assert_eq!(missing, records[2..].iter().map(|r| r.wire.clone()).collect::<Vec<_>>());
    assert_eq!(store.records_missing_from(&[]).unwrap().len(), 5);
}

#[test]
fn personal_data_can_be_erased() {
    let store = Store::open_in_memory(&KEY).unwrap();
    let c = CustomerId::from_u128(7);
    store.put_customer(c, b"Jordan, 555-0100").unwrap();
    assert_eq!(store.get_customer(c).unwrap().as_deref(), Some(&b"Jordan, 555-0100"[..]));
    store.put_customer(c, b"Jordan, 555-0199").unwrap();
    assert_eq!(store.get_customer(c).unwrap().as_deref(), Some(&b"Jordan, 555-0199"[..]));
    store.erase_customer(c).unwrap();
    assert_eq!(store.get_customer(c).unwrap(), None);
}

#[test]
fn records_that_arrive_early_survive_a_restart() {
    let path = temp_path("early");
    let (_, records) = history(3);
    {
        let store = Store::open(&path, &KEY).unwrap();
        let mut book = Book::new();
        // Records 2..4 arrive before record 1 (out of order from peers).
        store_all(&store, &mut book, &records[1..]);
        assert_eq!(book.pending_count(), 3);
    }
    let store = Store::open(&path, &KEY).unwrap();
    let mut book = store.load_book().unwrap();
    store.receive(&mut book, &records[0].wire, 0).unwrap().unwrap();
    assert_eq!(book.pending_count(), 0);
    assert_eq!(book.config().taxes.len(), 3);
}
