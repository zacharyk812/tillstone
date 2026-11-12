//! The owner authority key (PLAN §6, §9): created at setup, kept encrypted
//! with the owner passphrase, and backed up as a printable recovery code.
//!
//! A stolen device without the passphrase can't add devices or change who
//! belongs to the business. If every owner device is lost, the recovery code
//! recreates the key.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305,
    aead::{Aead, Payload},
};
use data_encoding::BASE32_NOPAD;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{AppError, Result};

/// Shortest owner passphrase accepted.
pub const MIN_PASSPHRASE_CHARS: usize = 10;

const SEALED_VERSION: u8 = 1;
const AAD: &[u8] = b"tillstone owner key v1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const RECOVERY_PREFIX: &str = "TS1";

/// 64 MiB, 3 passes: slow enough to make guessing expensive, fast enough
/// (well under a second) on a low-end Android register.
fn kdf(passphrase: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(64 * 1024, 3, 1, Some(32))
        .map_err(|_| AppError::Invalid("bad key derivation settings"))?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut_slice())
        .map_err(|_| AppError::Invalid("couldn't derive the key"))?;
    Ok(key)
}

fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>> {
    let mut buf = Zeroizing::new([0u8; N]);
    getrandom::fill(buf.as_mut_slice())
        .map_err(|_| AppError::Invalid("the system random number generator failed"))?;
    Ok(buf)
}

/// A new owner key.
pub fn generate() -> Result<Zeroizing<[u8; 32]>> {
    random::<32>()
}

pub fn check_passphrase(passphrase: &str) -> Result<()> {
    if passphrase.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err(AppError::Invalid("the owner passphrase needs at least 10 characters"));
    }
    Ok(())
}

/// Encrypts the owner key with the passphrase:
/// `version(1) | salt(16) | nonce(24) | ciphertext+tag(48)`.
pub fn seal(secret: &[u8; 32], passphrase: &str) -> Result<Vec<u8>> {
    check_passphrase(passphrase)?;
    let salt = random::<SALT_LEN>()?;
    let nonce = random::<NONCE_LEN>()?;
    let key = kdf(passphrase, salt.as_slice())?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let sealed = cipher
        .encrypt((&*nonce).into(), Payload { msg: secret, aad: AAD })
        .map_err(|_| AppError::Invalid("couldn't encrypt the owner key"))?;
    let mut out = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + sealed.len());
    out.push(SEALED_VERSION);
    out.extend_from_slice(salt.as_slice());
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Decrypts the owner key. A wrong passphrase and a damaged blob look the
/// same on purpose.
pub fn unseal(sealed: &[u8], passphrase: &str) -> Result<Zeroizing<[u8; 32]>> {
    let wrong = || AppError::Invalid("wrong owner passphrase");
    if sealed.len() != 1 + SALT_LEN + NONCE_LEN + 48 || sealed[0] != SEALED_VERSION {
        return Err(AppError::Invalid("the stored owner key is damaged"));
    }
    let (salt, rest) = sealed[1..].split_at(SALT_LEN);
    let (nonce, ciphertext) = rest.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| wrong())?;
    let key = kdf(passphrase, salt)?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let plain = Zeroizing::new(
        cipher
            .decrypt((&nonce).into(), Payload { msg: ciphertext, aad: AAD })
            .map_err(|_| wrong())?,
    );
    let mut out = Zeroizing::new([0u8; 32]);
    out.copy_from_slice(plain.get(..32).ok_or_else(wrong)?);
    Ok(out)
}

/// The recovery code to print or write down: `TS1-` then the key and a
/// 2-byte checksum in base32, in groups of four (e.g. `TS1-ABCD-EFGH-…`).
/// Base32 avoids lower case and symbols, so it survives handwriting.
pub fn recovery_code(secret: &[u8; 32]) -> String {
    let mut bytes = Zeroizing::new(secret.to_vec());
    bytes.extend_from_slice(&Sha256::digest(secret)[..2]);
    let encoded = Zeroizing::new(BASE32_NOPAD.encode(&bytes));
    let groups: Vec<&str> =
        encoded.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or_default()).collect();
    format!("{RECOVERY_PREFIX}-{}", groups.join("-"))
}

/// Reads a recovery code back. Spaces, dashes and case don't matter; a
/// mistyped character fails the checksum.
pub fn from_recovery_code(code: &str) -> Result<Zeroizing<[u8; 32]>> {
    let bad = || AppError::Invalid("that recovery code isn't right; check each group");
    let cleaned: Zeroizing<String> = Zeroizing::new(
        code.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect(),
    );
    let body = cleaned.strip_prefix(RECOVERY_PREFIX).ok_or_else(bad)?;
    let bytes = Zeroizing::new(BASE32_NOPAD.decode(body.as_bytes()).map_err(|_| bad())?);
    if bytes.len() != 34 {
        return Err(bad());
    }
    let (secret, check) = bytes.split_at(32);
    if Sha256::digest(secret)[..2] != *check {
        return Err(bad());
    }
    let mut out = Zeroizing::new([0u8; 32]);
    out.copy_from_slice(secret);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASS: &str = "correct horse battery";

    #[test]
    fn seal_and_unseal() {
        let secret = [7u8; 32];
        let sealed = seal(&secret, PASS).unwrap();
        assert_eq!(*unseal(&sealed, PASS).unwrap(), secret);
        assert!(unseal(&sealed, "wrong passphrase!").is_err());
        // Each seal is different (fresh salt and nonce).
        assert_ne!(sealed, seal(&secret, PASS).unwrap());
    }

    #[test]
    fn tampering_is_detected() {
        let mut sealed = seal(&[1u8; 32], PASS).unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert!(unseal(&sealed, PASS).is_err());
        assert!(unseal(&sealed[..10], PASS).is_err());
    }

    #[test]
    fn short_passphrases_are_refused() {
        assert!(seal(&[1u8; 32], "short").is_err());
    }

    #[test]
    fn recovery_code_round_trips_and_forgives_formatting() {
        let secret = *generate().unwrap();
        let code = recovery_code(&secret);
        assert!(code.starts_with("TS1-"));
        assert_eq!(*from_recovery_code(&code).unwrap(), secret);
        let messy = code.to_lowercase().replace('-', " ");
        assert_eq!(*from_recovery_code(&messy).unwrap(), secret);
    }

    #[test]
    fn a_typo_in_the_recovery_code_is_caught() {
        let code = recovery_code(&[9u8; 32]);
        let mut chars: Vec<char> = code.chars().collect();
        let i = 10;
        chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
        let typo: String = chars.into_iter().collect();
        assert!(from_recovery_code(&typo).is_err());
        assert!(from_recovery_code("TS1-AAAA").is_err());
        assert!(from_recovery_code("hello").is_err());
    }
}
