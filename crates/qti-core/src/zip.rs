//! Deterministic, in-memory ZIP package construction.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Package content keyed by its POSIX path inside the ZIP archive.
pub type ArchiveMap = BTreeMap<String, ArchiveEntry>;

/// The owned source of one ZIP entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveEntry {
    /// Bytes already prepared by a packaging engine.
    Bytes(Vec<u8>),
    /// A regular file whose bytes are read during the package preflight.
    SourcePath(PathBuf),
}

/// Failures while collecting source files or building a ZIP package.
#[derive(Debug, Error)]
pub enum ZipError {
    /// An archive entry name could be interpreted outside the ZIP root.
    #[error("invalid ZIP entry path '{path}': {reason}")]
    InvalidArchivePath { path: String, reason: &'static str },
    /// An empty-directory marker must be a ZIP-root-relative path ending in `/`.
    #[error("invalid ZIP empty-directory marker '{path}'")]
    InvalidDirectoryMarker { path: String },
    /// A source path was a symbolic link, which is not copied implicitly into a package.
    #[error("ZIP source path must not be a symbolic link: {path}")]
    SourceSymlink { path: PathBuf },
    /// A source path was not a regular file.
    #[error("ZIP source path must be a regular file: {path}")]
    SourceNotRegular { path: PathBuf },
    /// The directory passed to [`collect_directory`] was not a directory.
    #[error("ZIP collection source must be a directory: {path}")]
    CollectionRootNotDirectory { path: PathBuf },
    /// A filesystem name could not be represented as a ZIP UTF-8 entry name.
    #[error("ZIP collection source contains a non-UTF-8 path: {path}")]
    NonUtf8Path { path: PathBuf },
    /// The filesystem could not be inspected or read.
    #[error("could not read ZIP source at {path}: {source}")]
    ReadSource {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A temporary output file could not be created beside the requested destination.
    #[error("could not create temporary ZIP output beside {path}: {source}")]
    CreateOutputTemp {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The fully encoded archive could not be written to its temporary output file.
    #[error("could not write temporary ZIP output for {path}: {source}")]
    WriteOutputTemp {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The temporary ZIP output could not be synchronized before replacement.
    #[error("could not synchronize temporary ZIP output for {path}: {source}")]
    SyncOutputTemp {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The completed temporary archive could not atomically replace the destination.
    #[error("could not replace ZIP output at {path}: {source}")]
    PersistOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The ZIP encoder rejected an otherwise validated archive.
    #[error("could not encode ZIP archive: {0}")]
    Encode(#[from] zip::result::ZipError),
}

/// Collects regular files below `source_dir` into an archive map.
///
/// Archive names are relative to `source_dir`, use forward slashes, and are ordered by
/// [`ArchiveMap`]. Empty directories are deliberately omitted because callers name the
/// required ZIP markers explicitly. Symbolic links and other non-regular filesystem objects
/// cause an error instead of being silently skipped.
pub fn collect_directory(source_dir: impl AsRef<Path>) -> Result<ArchiveMap, ZipError> {
    let source_dir = source_dir.as_ref();
    let metadata = fs::symlink_metadata(source_dir).map_err(|source| ZipError::ReadSource {
        path: source_dir.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(ZipError::SourceSymlink {
            path: source_dir.to_path_buf(),
        });
    }
    if !metadata.is_dir() {
        return Err(ZipError::CollectionRootNotDirectory {
            path: source_dir.to_path_buf(),
        });
    }

    let mut archive_map = ArchiveMap::new();
    collect_directory_entries(source_dir, source_dir, &mut archive_map)?;
    Ok(archive_map)
}

/// Encodes an archive map in memory, then atomically replaces `zip_path` with the completed ZIP.
///
/// All names and file-backed payloads are validated and read before the output path is opened.
/// Consequently, an invalid entry, missing source file, or failed temporary write leaves an
/// existing output untouched.
/// The map's [`BTreeMap`] order supplies deterministic file-entry order; directory markers are
/// deduplicated and added in sorted order after file entries.
pub fn build_zip<I, S>(
    zip_path: impl AsRef<Path>,
    archive_map: &ArchiveMap,
    empty_dirs: I,
) -> Result<PathBuf, ZipError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // ASVS 2.2.1 / 5.3.2: validate every archive name and read every source before a temporary
    // output is opened, so caller-controlled paths cannot partially replace an output.
    let entries = preflight_entries(archive_map)?;
    let directory_markers = collect_directory_markers(archive_map, empty_dirs)?;
    let encoded = encode_zip(entries, directory_markers)?;
    let zip_path = zip_path.as_ref().to_path_buf();
    persist_encoded_zip(&zip_path, &encoded)?;
    Ok(zip_path)
}

fn persist_encoded_zip(zip_path: &Path, encoded: &[u8]) -> Result<(), ZipError> {
    let output_dir = zip_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".qti_zip_")
        .tempfile_in(output_dir)
        .map_err(|source| ZipError::CreateOutputTemp {
            path: zip_path.to_path_buf(),
            source,
        })?;
    temporary
        .write_all(encoded)
        .and_then(|()| temporary.flush())
        .map_err(|source| ZipError::WriteOutputTemp {
            path: zip_path.to_path_buf(),
            source,
        })?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| ZipError::SyncOutputTemp {
            path: zip_path.to_path_buf(),
            source,
        })?;
    temporary
        .into_temp_path()
        .persist(zip_path)
        .map_err(|error| ZipError::PersistOutput {
            path: zip_path.to_path_buf(),
            source: error.error,
        })?;
    Ok(())
}

fn collect_directory_entries(
    source_dir: &Path,
    directory: &Path,
    archive_map: &mut ArchiveMap,
) -> Result<(), ZipError> {
    for child in fs::read_dir(directory).map_err(|source| ZipError::ReadSource {
        path: directory.to_path_buf(),
        source,
    })? {
        let child = child.map_err(|source| ZipError::ReadSource {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = child.path();
        let file_type = child.file_type().map_err(|source| ZipError::ReadSource {
            path: path.clone(),
            source,
        })?;
        if file_type.is_symlink() {
            return Err(ZipError::SourceSymlink { path });
        }
        if file_type.is_dir() {
            collect_directory_entries(source_dir, &path, archive_map)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(ZipError::SourceNotRegular { path });
        }

        let relative = path
            .strip_prefix(source_dir)
            .expect("walked below source directory");
        let archive_path = relative
            .to_str()
            .ok_or_else(|| ZipError::NonUtf8Path { path: path.clone() })?
            .replace(std::path::MAIN_SEPARATOR, "/");
        validate_archive_path(&archive_path)?;
        archive_map.insert(archive_path, ArchiveEntry::SourcePath(path));
    }
    Ok(())
}

fn preflight_entries(archive_map: &ArchiveMap) -> Result<Vec<(String, Vec<u8>)>, ZipError> {
    archive_map
        .iter()
        .map(|(archive_path, entry)| {
            validate_archive_path(archive_path)?;
            let bytes = match entry {
                ArchiveEntry::Bytes(bytes) => bytes.clone(),
                ArchiveEntry::SourcePath(path) => read_regular_file(path)?,
            };
            Ok((archive_path.clone(), bytes))
        })
        .collect()
}

fn collect_directory_markers<I, S>(
    archive_map: &ArchiveMap,
    empty_dirs: I,
) -> Result<BTreeSet<String>, ZipError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut markers = BTreeSet::new();
    for marker in empty_dirs {
        let marker = marker.as_ref();
        if !marker.ends_with('/') {
            return Err(ZipError::InvalidDirectoryMarker {
                path: marker.to_owned(),
            });
        }
        validate_archive_path(marker)?;
        if !archive_map.contains_key(marker) {
            markers.insert(marker.to_owned());
        }
    }
    Ok(markers)
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>, ZipError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| ZipError::ReadSource {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        // ASVS 5.3.2: a package map must name the file to copy directly; it cannot use a
        // symlink to redirect a package read to another location.
        return Err(ZipError::SourceSymlink {
            path: path.to_path_buf(),
        });
    }
    if !metadata.is_file() {
        return Err(ZipError::SourceNotRegular {
            path: path.to_path_buf(),
        });
    }
    fs::read(path).map_err(|source| ZipError::ReadSource {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_archive_path(path: &str) -> Result<(), ZipError> {
    // ASVS 2.2.1 / 5.3.2: ZIP member names are a trusted-root-relative POSIX subset. This
    // prevents zip-slip traversal or platform-specific absolute-path interpretation downstream.
    let invalid = |reason| ZipError::InvalidArchivePath {
        path: path.to_owned(),
        reason,
    };
    if path.is_empty() {
        return Err(invalid("path is empty"));
    }
    if path.starts_with('/') {
        return Err(invalid("path is absolute"));
    }
    if path.contains('\\') {
        return Err(invalid("backslashes are not valid ZIP separators"));
    }
    if path.chars().any(char::is_control) {
        return Err(invalid("path contains a control character"));
    }
    for component in path.trim_end_matches('/').split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(invalid("path contains an empty or traversal component"));
        }
        if component.contains(':') {
            return Err(invalid("path contains a drive-prefix separator"));
        }
    }
    Ok(())
}

fn encode_zip(
    entries: Vec<(String, Vec<u8>)>,
    directory_markers: BTreeSet<String>,
) -> Result<Vec<u8>, ZipError> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (archive_path, bytes) in entries {
        writer.start_file(archive_path, options)?;
        writer
            .write_all(&bytes)
            .map_err(zip::result::ZipError::Io)?;
    }
    for marker in directory_markers {
        writer.add_directory(marker, options)?;
    }
    Ok(writer.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Read};

    use super::{ArchiveEntry, ArchiveMap, ZipError, build_zip, collect_directory};

    #[test]
    fn build_zip_sorts_files_and_preserves_explicit_empty_directories() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let output = temp.path().join("package.zip");
        let mut entries = ArchiveMap::new();
        entries.insert("zeta.txt".to_owned(), ArchiveEntry::Bytes(b"z".to_vec()));
        entries.insert(
            "nested/file with spaces.txt".to_owned(),
            ArchiveEntry::Bytes(b"nested".to_vec()),
        );
        entries.insert("alpha.txt".to_owned(), ArchiveEntry::Bytes(b"a".to_vec()));

        build_zip(&output, &entries, ["empty/nested/", "occupied/"]).expect("archive builds");

        let bytes = fs::read(&output).expect("archive bytes");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("archive opens");
        let names = archive.file_names().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "alpha.txt",
                "nested/file with spaces.txt",
                "zeta.txt",
                "empty/nested/",
                "occupied/",
            ]
        );
        let mut nested = String::new();
        archive
            .by_name("nested/file with spaces.txt")
            .expect("space filename")
            .read_to_string(&mut nested)
            .expect("read nested file");
        assert_eq!(nested, "nested");
    }

    #[test]
    fn a_real_entry_wins_over_matching_directory_marker() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let output = temp.path().join("package.zip");
        let mut entries = ArchiveMap::new();
        entries.insert(
            "occupied/".to_owned(),
            ArchiveEntry::Bytes(b"data".to_vec()),
        );

        build_zip(&output, &entries, ["occupied/"]).expect("archive builds");

        let bytes = fs::read(&output).expect("archive bytes");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("archive opens");
        assert_eq!(archive.len(), 1);
        let mut content = Vec::new();
        archive
            .by_index(0)
            .expect("real entry")
            .read_to_end(&mut content)
            .expect("read real entry");
        assert_eq!(content, b"data");
    }

    #[test]
    fn invalid_or_missing_inputs_preserve_an_existing_output() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let output = temp.path().join("existing.zip");
        fs::write(&output, b"unchanged").expect("existing output");
        let mut traversal = ArchiveMap::new();
        traversal.insert("../escape.txt".to_owned(), ArchiveEntry::Bytes(Vec::new()));

        assert!(matches!(
            build_zip(&output, &traversal, std::iter::empty::<&str>()),
            Err(ZipError::InvalidArchivePath { .. })
        ));
        assert_eq!(
            fs::read(&output).expect("existing output remains"),
            b"unchanged"
        );

        let mut missing = ArchiveMap::new();
        missing.insert(
            "missing.txt".to_owned(),
            ArchiveEntry::SourcePath(temp.path().join("missing.txt")),
        );
        assert!(matches!(
            build_zip(&output, &missing, std::iter::empty::<&str>()),
            Err(ZipError::ReadSource { .. })
        ));
        assert_eq!(
            fs::read(&output).expect("existing output remains"),
            b"unchanged"
        );

        let mut valid = ArchiveMap::new();
        valid.insert(
            "committed.txt".to_owned(),
            ArchiveEntry::Bytes(b"new".to_vec()),
        );
        build_zip(&output, &valid, std::iter::empty::<&str>()).expect("replace output");
        let mut archive = zip::ZipArchive::new(Cursor::new(fs::read(&output).expect("new output")))
            .expect("replacement archive opens");
        let mut committed = String::new();
        archive
            .by_name("committed.txt")
            .expect("committed entry")
            .read_to_string(&mut committed)
            .expect("read committed entry");
        assert_eq!(committed, "new");
    }

    #[test]
    fn collect_directory_keeps_nested_files_and_rejects_symlinks() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let root = temp.path().join("source");
        fs::create_dir_all(root.join("nested")).expect("nested source directory");
        fs::write(root.join("file with spaces.txt"), b"top").expect("top-level source");
        fs::write(root.join("nested/item.txt"), b"nested").expect("nested source");

        let entries = collect_directory(&root).expect("collect files");
        assert_eq!(entries.len(), 2);
        assert!(matches!(
            entries.get("nested/item.txt"),
            Some(ArchiveEntry::SourcePath(_))
        ));
        assert!(entries.contains_key("file with spaces.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn collect_directory_reports_a_symlink_instead_of_skipping_it() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("temporary directory");
        let root = temp.path().join("source");
        fs::create_dir(&root).expect("source directory");
        let target = root.join("target.txt");
        fs::write(&target, b"target").expect("source file");
        symlink(&target, root.join("linked.txt")).expect("source symlink");

        assert!(matches!(
            collect_directory(&root),
            Err(ZipError::SourceSymlink { .. })
        ));
    }
}
