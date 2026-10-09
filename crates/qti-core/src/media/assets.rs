//! Caller-owned local media payloads for the portable conversion boundary.

use std::borrow::Cow;

use super::MediaError;
use crate::{EntryMap, validate_entry_name};

/// Reads exact authored source keys without imposing storage or filesystem ownership.
/// Implementations own source authorization; the core never fetches external URLs.
pub trait AssetSource: Send + Sync {
    /// Reads the exact authored key, returning borrowed or provider-owned bytes.
    ///
    /// Invalid or missing keys and provider failures return `MediaError`.
    fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError>;
}

/// Validated, relative POSIX source names and their owned payloads.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryAssets {
    entries: EntryMap,
}

impl MemoryAssets {
    /// Creates an empty asset map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates every logical relative POSIX key and takes ownership of its entry map.
    pub fn from_entries(entries: EntryMap) -> Result<Self, MediaError> {
        for name in entries.keys() {
            validate_name(name)?;
        }
        Ok(Self { entries })
    }

    /// Inserts an exact source key; identical reinsertion is idempotent.
    /// A conflicting payload is an error so an overlay cannot silently change source identity.
    pub fn insert(&mut self, name: impl Into<String>, bytes: Vec<u8>) -> Result<(), MediaError> {
        let name = name.into();
        validate_name(&name)?;
        if let Some(existing) = self.entries.get(&name) {
            if existing != &bytes {
                return Err(MediaError::ConflictingAsset { src: name });
            }
            return Ok(());
        }
        self.entries.insert(name, bytes);
        Ok(())
    }

    /// Returns bytes for an exact validated key without copying them.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(Vec::as_slice)
    }

    /// Borrows the complete validated entry map.
    #[must_use]
    pub fn entries(&self) -> &EntryMap {
        &self.entries
    }

    /// Consumes this provider and returns its owned validated entry map.
    #[must_use]
    pub fn into_entries(self) -> EntryMap {
        self.entries
    }
}

impl AssetSource for MemoryAssets {
    fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
        validate_name(src)?;
        self.get(src)
            .map(Cow::Borrowed)
            .ok_or_else(|| MediaError::MissingAsset {
                src: src.to_owned(),
            })
    }
}

fn validate_name(name: &str) -> Result<(), MediaError> {
    // ASVS 5.3.2 / 5.3.3: browser and in-memory sources share the portable file-name boundary.
    validate_entry_name(name).map_err(|error| MediaError::InvalidName {
        src: name.to_owned(),
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_exact_keys_and_rejects_conflicting_or_unsafe_inputs() {
        let mut assets = MemoryAssets::new();
        assets.insert("a/image.png", vec![1]).expect("insert");
        assets
            .insert("b/image.png", vec![2])
            .expect("distinct source");
        assets
            .insert("a/image.png", vec![1])
            .expect("identical duplicate");
        assert!(matches!(
            assets.insert("a/image.png", vec![3]),
            Err(MediaError::ConflictingAsset { .. })
        ));
        assert_eq!(
            assets.read("a/image.png").expect("original bytes").as_ref(),
            [1]
        );
        assert!(matches!(
            assets.read("image.png"),
            Err(MediaError::MissingAsset { .. })
        ));
        for name in [
            "../image.png",
            "/image.png",
            "a\\image.png",
            "a/",
            "C:/image.png",
        ] {
            assert!(matches!(
                assets.insert(name, Vec::new()),
                Err(MediaError::InvalidName { .. })
            ));
            assert!(
                MemoryAssets::from_entries(EntryMap::from([(name.to_owned(), Vec::new())]))
                    .is_err()
            );
        }
    }
}
