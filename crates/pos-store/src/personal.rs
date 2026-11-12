//! Customers' personal data (PLAN §4): kept out of the permanent event log,
//! encrypted with a key per customer, and erasable.
//!
//! Erasing deletes the customer's key and ciphertext from this database.
//! Honest limit: older backups taken before the erasure still contain the
//! key until those backups expire, so backup retention must be finite.

use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305,
    aead::{Aead, Payload},
};
use pos_core::ids::CustomerId;
use rusqlite::{Connection, OptionalExtension, params};
use zeroize::Zeroizing;

#[derive(Debug)]
pub enum PersonalError {
    Sqlite(rusqlite::Error),
    Random,
    /// The ciphertext doesn't decrypt with the stored key.
    Damaged,
}

impl std::fmt::Display for PersonalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "database error: {e}"),
            Self::Random => f.write_str("the system random number generator failed"),
            Self::Damaged => f.write_str("customer data is damaged"),
        }
    }
}

impl std::error::Error for PersonalError {}

impl From<rusqlite::Error> for PersonalError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>, PersonalError> {
    let mut buf = Zeroizing::new([0u8; N]);
    getrandom::fill(buf.as_mut_slice()).map_err(|_| PersonalError::Random)?;
    Ok(buf)
}

fn key_for(
    conn: &Connection,
    customer: CustomerId,
) -> Result<Option<Zeroizing<[u8; 32]>>, PersonalError> {
    let key: Option<Vec<u8>> = conn
        .query_row(
            "SELECT key FROM customer_keys WHERE customer = ?1",
            [customer.0.as_bytes()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(key.and_then(|k| <[u8; 32]>::try_from(k.as_slice()).ok()).map(Zeroizing::new))
}

pub fn put(conn: &Connection, customer: CustomerId, details: &[u8]) -> Result<(), PersonalError> {
    let key = match key_for(conn, customer)? {
        Some(k) => k,
        None => {
            let k = random::<32>()?;
            conn.execute(
                "INSERT INTO customer_keys (customer, key) VALUES (?1, ?2)",
                params![customer.0.as_bytes(), k.as_slice()],
            )?;
            k
        }
    };
    let nonce = random::<24>()?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let ciphertext = cipher
        .encrypt((&*nonce).into(), Payload { msg: details, aad: customer.0.as_bytes() })
        .map_err(|_| PersonalError::Damaged)?;
    conn.execute(
        "INSERT INTO customers (customer, nonce, ciphertext) VALUES (?1, ?2, ?3)
         ON CONFLICT (customer) DO UPDATE SET nonce = excluded.nonce, ciphertext = excluded.ciphertext",
        params![customer.0.as_bytes(), nonce.as_slice(), ciphertext],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, customer: CustomerId) -> Result<Option<Vec<u8>>, PersonalError> {
    let Some(key) = key_for(conn, customer)? else { return Ok(None) };
    let row: Option<(Vec<u8>, Vec<u8>)> = conn
        .query_row(
            "SELECT nonce, ciphertext FROM customers WHERE customer = ?1",
            [customer.0.as_bytes()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((nonce, ciphertext)) = row else { return Ok(None) };
    let nonce: [u8; 24] = nonce.as_slice().try_into().map_err(|_| PersonalError::Damaged)?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    cipher
        .decrypt((&nonce).into(), Payload { msg: &ciphertext, aad: customer.0.as_bytes() })
        .map(Some)
        .map_err(|_| PersonalError::Damaged)
}

pub fn erase(conn: &Connection, customer: CustomerId) -> Result<(), PersonalError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM customer_keys WHERE customer = ?1", [customer.0.as_bytes()])?;
    tx.execute("DELETE FROM customers WHERE customer = ?1", [customer.0.as_bytes()])?;
    tx.commit()?;
    Ok(())
}
