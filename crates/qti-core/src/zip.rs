//! Deterministic, in-memory ZIP package construction.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};
use std::sync::Arc;

use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Prepared file bytes keyed by their relative POSIX package name.
pub type EntryMap = BTreeMap<String, Vec<u8>>;

/// One named output file, validated when constructed, with cheaply cloned immutable bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedFile {
    name: String,
    bytes: Arc<[u8]>,
}

impl NamedFile {
    /// Takes ownership of file bytes after validating the package-relative name.
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Result<Self, ZipError> {
        Self::from_shared(name, bytes.into())
    }

    /// Shares immutable file bytes after validating the package-relative name.
    pub fn from_shared(name: impl Into<String>, bytes: Arc<[u8]>) -> Result<Self, ZipError> {
        let name = name.into();
        validate_entry_name(&name)?;
        Ok(Self { name, bytes })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    /// Copies the shared payload into independently owned bytes for an output boundary.
    pub fn into_parts(self) -> (String, Vec<u8>) {
        (self.name, self.bytes.to_vec())
    }
}

/// Failures while validating or encoding a ZIP package.
#[derive(Debug, Error)]
pub enum ZipError {
    #[error("invalid ZIP entry path '{path}': {reason}")]
    InvalidArchivePath { path: String, reason: &'static str },
    #[error("invalid ZIP empty-directory marker '{path}'")]
    InvalidDirectoryMarker { path: String },
    #[error("could not encode ZIP archive: {0}")]
    Encode(#[from] zip::result::ZipError),
}

/// Validates a file name shared by portable outputs and asset maps.
///
/// ASVS 5.3.2 / 5.3.3: reject traversal, absolute paths, and platform-dependent separators
/// before a name reaches an archive or a native persistence adapter.
pub fn validate_entry_name(name: &str) -> Result<(), ZipError> {
    let invalid = |reason| ZipError::InvalidArchivePath {
        path: name.to_owned(),
        reason,
    };
    if name.is_empty() {
        return Err(invalid("path is empty"));
    }
    if name.starts_with('/') {
        return Err(invalid("path is absolute"));
    }
    if name.ends_with('/') {
        return Err(invalid("path names a directory"));
    }
    if name.contains('\\') {
        return Err(invalid("backslashes are not valid package separators"));
    }
    if name.chars().any(char::is_control) {
        return Err(invalid("path contains a control character"));
    }
    for component in name.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(invalid("path contains an empty or traversal component"));
        }
        if component.contains(':') {
            return Err(invalid("path contains a drive-prefix separator"));
        }
    }
    Ok(())
}

/// Encodes prepared bytes with deterministic file order and explicit empty directories.
///
/// Every name is validated before encoding. Files follow the map's sorted order; directory
/// markers are deduplicated and added in sorted order after files. Compression and default
/// ZIP metadata retain the existing package contract. Persistence belongs to the caller.
pub fn encode_zip<I, S>(entries: &EntryMap, empty_dirs: I) -> Result<Vec<u8>, ZipError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for name in entries.keys() {
        validate_entry_name(name)?;
    }
    let mut directory_markers = BTreeSet::new();
    for marker in empty_dirs {
        let marker = marker.as_ref();
        let name = marker
            .strip_suffix('/')
            .ok_or_else(|| ZipError::InvalidDirectoryMarker {
                path: marker.to_owned(),
            })?;
        validate_entry_name(name)?;
        directory_markers.insert(marker.to_owned());
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(name, options)?;
        writer.write_all(bytes).map_err(zip::result::ZipError::Io)?;
    }
    for marker in directory_markers {
        writer.add_directory(marker, options)?;
    }
    Ok(writer.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use super::{EntryMap, NamedFile, ZipError, encode_zip, validate_entry_name};

    #[test]
    fn cloned_files_share_payload_but_owned_output_is_independent() {
        let original = NamedFile::new("media/image.png", vec![1, 2, 3]).expect("file");
        let cloned = original.clone();
        assert_eq!(original.bytes().as_ptr(), cloned.bytes().as_ptr());
        let (name, mut bytes) = cloned.into_parts();
        assert_eq!(name, "media/image.png");
        assert_eq!(bytes, [1, 2, 3]);
        bytes[0] = 9;
        assert_eq!(original.bytes(), [1, 2, 3]);
    }

    #[test]
    fn sorts_files_and_preserves_explicit_empty_directories() {
        let entries = EntryMap::from([
            ("zeta.txt".to_owned(), b"z".to_vec()),
            ("nested/file with spaces.txt".to_owned(), b"nested".to_vec()),
            ("alpha.txt".to_owned(), b"a".to_vec()),
        ]);
        let bytes = encode_zip(&entries, ["occupied/", "empty/nested/", "occupied/"])
            .expect("archive builds");
        assert_eq!(
            bytes,
            encode_zip(&entries, ["empty/nested/", "occupied/"]).expect("same archive")
        );
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("archive opens");
        assert_eq!(
            archive.file_names().collect::<Vec<_>>(),
            [
                "alpha.txt",
                "nested/file with spaces.txt",
                "zeta.txt",
                "empty/nested/",
                "occupied/"
            ]
        );
        let mut nested = String::new();
        archive
            .by_name("nested/file with spaces.txt")
            .expect("entry")
            .read_to_string(&mut nested)
            .expect("payload");
        assert_eq!(nested, "nested");
    }

    #[test]
    fn rejects_unsafe_files_and_directory_markers() {
        for name in [
            "",
            "../escape.txt",
            "/absolute.txt",
            "a\\b",
            "C:/file",
            "a//b",
            "a/./b",
            "occupied/",
            "a\0b",
        ] {
            assert!(validate_entry_name(name).is_err(), "{name:?}");
            assert!(NamedFile::new(name, Vec::new()).is_err(), "{name:?}");
            let entries = EntryMap::from([(name.to_owned(), Vec::new())]);
            assert!(matches!(
                encode_zip(&entries, [] as [&str; 0]),
                Err(ZipError::InvalidArchivePath { .. })
            ));
        }
        for marker in ["missing_slash", "../bad/", "a//", "/"] {
            assert!(
                encode_zip(&EntryMap::new(), [marker]).is_err(),
                "{marker:?}"
            );
        }
        assert_eq!(
            NamedFile::new("nested/gene figure.svg", vec![1])
                .expect("safe name")
                .bytes(),
            [1]
        );
    }
}
