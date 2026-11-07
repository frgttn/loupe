//! Encryption at rest on macOS: the database key, kept in the login
//! Keychain (as a generic password, the way Raycast keeps its `main.db`
//! key), and sealed files for what lives next to the database (clipboard
//! images).
//!
//! The key is random and never typed: the Keychain ties it to the user's
//! account, so copying the data folder elsewhere doesn't copy it. Crypto is
//! `ring`: AES-256-GCM for files, in the same format as on Windows.
//!
//! The Keychain lets the app that created an entry read it without asking.
//! An unsigned build counts as a new app every time it is rebuilt, so
//! development builds ask for the key once per build ("Always Allow").
//!
//! [`protect_key`] and [`unprotect_key`] are for the Windows index service,
//! which macOS doesn't have.

use std::{fs, io, path::Path, sync::OnceLock};

use ring::{
    aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey},
    hmac,
    rand::{SecureRandom, SystemRandom},
};

use crate::UNSUPPORTED;

pub type Key = [u8; 32];

/// The database key's Keychain entry: service and account.
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "Loupe/database";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "Loupe";
/// Starts every sealed file; files without it are from before encryption.
const MAGIC: &[u8; 4] = b"SPV1";
const TAG: usize = 16;
/// What the file key is derived for, so it is never the database key itself.
const FILES_CONTEXT: &[u8] = b"loupe/files";

/// Set by [`unlock_files`]; until then files are read and written as they are.
static FILES_KEY: OnceLock<Key> = OnceLock::new();

/// The database key, created (and saved) on first use. `true` with a new
/// key: whatever was encrypted before can't be read any more.
///
/// Only a missing entry makes a new key. When the Keychain is locked or the
/// user denies access, this fails and the old key stays: the database is
/// then left alone rather than replaced.
#[cfg(target_os = "macos")]
pub fn database_key() -> crate::Result<(Key, bool)> {
    use security_framework::passwords::{get_generic_password, set_generic_password};

    /// `errSecItemNotFound`.
    const NOT_FOUND: i32 = -25300;

    match get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        // A damaged entry, or someone else's under this name: a key Loupe
        // didn't write can't open anything it wrote.
        Ok(bytes) => match Key::try_from(bytes.as_slice()) {
            Ok(key) => Ok((key, false)),
            Err(_) => save_new_key(set_generic_password),
        },
        Err(err) if err.code() == NOT_FOUND => save_new_key(set_generic_password),
        Err(err) => Err(keychain_error(&err)),
    }
}

#[cfg(target_os = "macos")]
fn save_new_key(
    save: fn(&str, &str, &[u8]) -> security_framework::base::Result<()>,
) -> crate::Result<(Key, bool)> {
    let key = random_key()?;
    save(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, &key).map_err(|err| keychain_error(&err))?;
    Ok((key, true))
}

#[cfg(target_os = "macos")]
fn keychain_error(err: &security_framework::base::Error) -> crate::Error {
    format!("Keychain: {err} ({})", err.code()).as_str().into()
}

/// Without the Keychain (building on Windows with `--cfg loupe_mac_stub`):
/// no key, so nothing is encrypted.
#[cfg(not(target_os = "macos"))]
pub fn database_key() -> crate::Result<(Key, bool)> {
    Err(UNSUPPORTED.into())
}

/// From now on [`write_file`] seals files and [`read_file`] opens them.
/// The files key is derived from the database key. Only the first call counts.
pub fn unlock_files(database_key: &Key) {
    let _ = FILES_KEY.set(files_key(database_key));
}

/// HMAC-SHA256 of [`FILES_CONTEXT`] under the database key, as on Windows.
fn files_key(database_key: &Key) -> Key {
    let tag = hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, database_key),
        FILES_CONTEXT,
    );
    tag.as_ref().try_into().expect("HMAC-SHA256 is 32 bytes")
}

/// Writes `bytes` to `path`, sealed once [`unlock_files`] ran.
pub fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match FILES_KEY.get() {
        Some(key) => fs::write(path, seal(key, bytes)?),
        None => fs::write(path, bytes),
    }
}

/// Reads a file [`write_file`] wrote, sealed or not.
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    if !bytes.starts_with(MAGIC) {
        return Ok(bytes);
    }
    let key = FILES_KEY
        .get()
        .ok_or_else(|| io::Error::other("the file is encrypted and no key is unlocked"))?;
    unseal(key, &bytes)
}

/// Seals the files in `dir` that are still plain (written before
/// encryption), in place. Returns how many. Needs [`unlock_files`].
pub fn seal_folder(dir: &Path) -> usize {
    let (Some(key), Ok(entries)) = (FILES_KEY.get(), fs::read_dir(dir)) else {
        return 0;
    };
    let mut sealed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if bytes.starts_with(MAGIC) {
            continue;
        }
        // Through a temporary file, so a crash leaves the plain or the
        // sealed copy, never half of one.
        let partial = path.with_extension("sealing");
        let done = seal(key, &bytes)
            .and_then(|data| fs::write(&partial, data))
            .and_then(|()| fs::rename(&partial, &path));
        match done {
            Ok(()) => sealed += 1,
            Err(err) => {
                eprintln!("[vault] could not encrypt {}: {err}", path.display());
                let _ = fs::remove_file(&partial);
            }
        }
    }
    sealed
}

/// Seals `plain` with `key`, as files are sealed: for data with a key of its
/// own (the index service's snapshots).
pub fn seal_with(key: &Key, plain: &[u8]) -> io::Result<Vec<u8>> {
    seal(key, plain)
}

/// Opens what [`seal_with`] sealed; fails on a wrong key or changed data.
pub fn unseal_with(key: &Key, sealed: &[u8]) -> io::Result<Vec<u8>> {
    if !sealed.starts_with(MAGIC) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not sealed"));
    }
    unseal(key, sealed)
}

/// A new random key.
pub fn random_key() -> io::Result<Key> {
    let mut key = Key::default();
    random(&mut key)?;
    Ok(key)
}

/// For the Windows index service, which macOS doesn't have.
pub fn protect_key(_key: &Key) -> io::Result<Vec<u8>> {
    Err(io::Error::other(UNSUPPORTED))
}

pub fn unprotect_key(_protected: &[u8]) -> io::Result<Key> {
    Err(io::Error::other(UNSUPPORTED))
}

fn aes_key(key: &Key) -> LessSafeKey {
    LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).expect("an AES-256 key is 32 bytes"))
}

/// `MAGIC`, a random nonce, the AES-256-GCM ciphertext and its tag.
fn seal(key: &Key, plain: &[u8]) -> io::Result<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    random(&mut nonce)?;
    let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + plain.len() + TAG);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    let mut body = plain.to_vec();
    aes_key(key)
        .seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut body)
        .map_err(|_| io::Error::other("could not encrypt"))?;
    out.extend_from_slice(&body);
    Ok(out)
}

fn unseal(key: &Key, data: &[u8]) -> io::Result<Vec<u8>> {
    let header = MAGIC.len() + NONCE_LEN;
    if data.len() < header + TAG {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "truncated file"));
    }
    let nonce: [u8; NONCE_LEN] = data[MAGIC.len()..header].try_into().unwrap();
    let mut body = data[header..].to_vec();
    // A wrong key or a changed file fails the tag check.
    let plain_len = aes_key(key)
        .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut body)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "wrong key or changed data"))?
        .len();
    body.truncate(plain_len);
    Ok(body)
}

fn random(buffer: &mut [u8]) -> io::Result<()> {
    SystemRandom::new()
        .fill(buffer)
        .map_err(|_| io::Error::other("no random bytes from the system"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_data_round_trips() {
        let key = random_key().unwrap();
        let sealed = seal_with(&key, b"snapshot").unwrap();
        assert_eq!(unseal_with(&key, &sealed).unwrap(), b"snapshot");
        assert!(unseal_with(&random_key().unwrap(), &sealed).is_err());
        assert!(unseal_with(&key, b"plain").is_err());
    }

    #[test]
    fn sealing_round_trips_and_detects_changes() {
        let key = [7u8; 32];
        let plain = b"\x89PNG not really an image";
        let sealed = seal(&key, plain).unwrap();
        assert!(sealed.starts_with(MAGIC));
        assert_eq!(sealed.len(), MAGIC.len() + NONCE_LEN + plain.len() + TAG);
        assert!(!sealed.windows(plain.len()).any(|w| w == plain));
        assert_eq!(unseal(&key, &sealed).unwrap(), plain);
        // A random nonce each time.
        assert_ne!(seal(&key, plain).unwrap(), sealed);

        assert!(unseal(&[8u8; 32], &sealed).is_err());
        let mut changed = sealed.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert!(unseal(&key, &changed).is_err());
        assert!(unseal(&key, &sealed[..10]).is_err());
        assert_eq!(unseal(&key, &seal(&key, b"").unwrap()).unwrap(), b"");
    }

    /// The files key is plain HMAC-SHA256, as Windows derives it with CNG,
    /// so the same database key gives the same files key on both.
    #[test]
    fn files_key_is_hmac_sha256_of_the_context() {
        let expected = "9400c1f28a08a952c4bc69e4efb81c466c66c6767ac14020af1278f8045987f3";
        let hex: String = files_key(&[1u8; 32])
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(hex, expected);
    }
}
