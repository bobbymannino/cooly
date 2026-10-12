//! Authenticated encryption for files at rest.
//!
//! File layout: `MAGIC (8) | nonce (24) | ciphertext + Poly1305 tag (16)`. The magic and the file's name are bound in
//! as associated data, so a file can't be renamed into another slot or have its version swapped without detection.

use chacha20poly1305::{
    KeyInit as _, XChaCha20Poly1305, XNonce,
    aead::{Aead as _, Generate as _, Payload},
};
use zeroize::Zeroizing;

use crate::{KEY_LEN, StoreError};

/// Identifies the format version. Bump this if the layout or algorithm changes.
const MAGIC: &[u8; 8] = b"COOLYv1\0";
const NONCE_LEN: usize = 24;

pub fn seal(key: &[u8; KEY_LEN], name: &str, plaintext: &[u8]) -> Result<Vec<u8>, StoreError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    // 192-bit random nonces are safe to generate per message without tracking state.
    let nonce = XNonce::try_generate().map_err(|_| StoreError::Random)?;
    let aad = associated_data(name);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| StoreError::Undecryptable)?;

    let mut sealed = Vec::with_capacity(MAGIC.len().saturating_add(NONCE_LEN).saturating_add(ciphertext.len()));
    sealed.extend_from_slice(MAGIC);
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);
    Ok(sealed)
}

pub fn open(key: &[u8; KEY_LEN], name: &str, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
    let rest = sealed.strip_prefix(MAGIC.as_slice()).ok_or(StoreError::Undecryptable)?;
    let (nonce, ciphertext) = rest.split_at_checked(NONCE_LEN).ok_or(StoreError::Undecryptable)?;
    let nonce = XNonce::try_from(nonce).map_err(|_| StoreError::Undecryptable)?;

    let cipher = XChaCha20Poly1305::new(key.into());
    let aad = associated_data(name);
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| StoreError::Undecryptable)
}

fn associated_data(name: &str) -> Vec<u8> {
    [MAGIC.as_slice(), name.as_bytes()].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [7; KEY_LEN];

    #[test]
    fn round_trips() {
        let sealed = seal(&KEY, "a", b"hello").unwrap();
        assert_eq!(open(&KEY, "a", &sealed).unwrap().as_slice(), b"hello");
    }

    #[test]
    fn rejects_other_slot_name() {
        let sealed = seal(&KEY, "a", b"hello").unwrap();
        assert!(matches!(open(&KEY, "b", &sealed), Err(StoreError::Undecryptable)));
    }

    #[test]
    fn rejects_truncated_or_foreign_files() {
        let sealed = seal(&KEY, "a", b"hello").unwrap();
        for bad in [
            &b""[..],
            &b"not a vault"[..],
            &sealed[..20],
            &sealed[..sealed.len() - 1],
        ] {
            assert!(matches!(open(&KEY, "a", bad), Err(StoreError::Undecryptable)));
        }
    }
}
