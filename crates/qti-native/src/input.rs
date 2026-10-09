//! Bounded native input loading and extracted-package integrity adapter.

use std::fs;
use std::io::Read;
use std::path::Path;

use qti_core::EntryMap;
use qti_integrity::{Provenance, Severity, Violation};

use crate::{NativeError, error::io};

const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

pub fn read_source(path: &Path) -> Result<Vec<u8>, NativeError> {
    read_bounded(path, MAX_TOTAL_BYTES)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, NativeError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.file_type().is_file() {
        return Err(NativeError::Invalid(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    if metadata.len() > limit {
        return Err(NativeError::Invalid(format!(
            "{} exceeds the input byte limit",
            path.display()
        )));
    }
    let file = fs::File::open(path).map_err(|source| io(path, source))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| io(path, source))?;
    if bytes.len() as u64 > limit {
        return Err(NativeError::Invalid(format!(
            "{} exceeds the input byte limit",
            path.display()
        )));
    }
    Ok(bytes)
}

pub fn read_package_entries(path: &Path) -> Result<EntryMap, NativeError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if metadata.file_type().is_file() {
        return qti_integrity::read_zip_entries(&read_source(path)?)
            .map_err(|violation| NativeError::Invalid(violation.message));
    }
    if !metadata.file_type().is_dir() {
        return Err(NativeError::Invalid(
            "package input is not a real file or directory".into(),
        ));
    }
    let mut entries = EntryMap::new();
    let mut count = 0;
    let mut total = 0;
    walk_directory(path, path, &mut entries, &mut count, &mut total)?;
    Ok(entries)
}

fn walk_directory(
    root: &Path,
    directory: &Path,
    entries: &mut EntryMap,
    count: &mut usize,
    total: &mut u64,
) -> Result<(), NativeError> {
    for entry in fs::read_dir(directory).map_err(|source| io(directory, source))? {
        let entry = entry.map_err(|source| io(directory, source))?;
        *count += 1;
        if *count > MAX_FILES {
            return Err(NativeError::Invalid(
                "extracted package exceeds 10000 entries".into(),
            ));
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| io(&path, source))?;
        let name = path
            .strip_prefix(root)
            .expect("walk stays beneath root")
            .to_str()
            .ok_or_else(|| NativeError::Invalid("package entry is not UTF-8".into()))?
            .to_owned();
        qti_core::NamedFile::new(name.clone(), Vec::new())
            .map_err(|error| NativeError::Invalid(error.to_string()))?;
        if metadata.file_type().is_dir() {
            walk_directory(root, &path, entries, count, total)?;
        } else if metadata.file_type().is_file() {
            // ASVS 5.2.1/5.2.3: reject declared totals before reading, then enforce actual bytes.
            let remaining = MAX_TOTAL_BYTES - *total;
            if metadata.len() > remaining {
                return Err(NativeError::Invalid(
                    "extracted package exceeds total byte limit".into(),
                ));
            }
            let bytes = read_bounded(&path, MAX_FILE_BYTES.min(remaining))?;
            *total += bytes.len() as u64;
            if *total > MAX_TOTAL_BYTES {
                return Err(NativeError::Invalid(
                    "extracted package exceeds total byte limit".into(),
                ));
            }
            entries.insert(name, bytes);
        } else {
            return Err(NativeError::Invalid(format!(
                "package entry is not a regular file or directory: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn check_package_path(path: &Path) -> Vec<Violation> {
    match read_package_entries(path) {
        Ok(entries) => qti_integrity::check_entries(&entries),
        Err(error) => vec![Violation {
            code: "unsafe-input",
            severity: Severity::Error,
            provenance: Provenance::SafeInputHandling,
            path: path.display().to_string(),
            message: error.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_package_returns_relative_named_bytes() {
        let temp = tempfile::tempdir().expect("directory");
        fs::create_dir(temp.path().join("media")).expect("media directory");
        fs::write(temp.path().join("media/cell.png"), b"image").expect("image");
        fs::write(temp.path().join("imsmanifest.xml"), b"manifest").expect("manifest");
        let entries = read_package_entries(temp.path()).expect("bounded entries");
        assert_eq!(entries.get("media/cell.png").expect("image"), b"image");
        assert_eq!(
            entries.get("imsmanifest.xml").expect("manifest"),
            b"manifest"
        );
    }

    #[test]
    fn extracted_package_rejects_invalid_names_and_oversized_members() {
        let temp = tempfile::tempdir().expect("directory");
        let invalid = temp.path().join("drive:payload.xml");
        fs::write(&invalid, b"invalid name").expect("fixture");
        assert!(read_package_entries(temp.path()).is_err());
        fs::remove_file(&invalid).expect("remove fixture");
        fs::File::create(temp.path().join("oversized.xml"))
            .expect("file")
            .set_len(MAX_FILE_BYTES + 1)
            .expect("sparse oversized file");
        assert!(
            read_package_entries(temp.path())
                .expect_err("oversized")
                .to_string()
                .contains("byte limit")
        );
    }

    #[cfg(unix)]
    #[test]
    fn extracted_package_rejects_symlinks_to_files_and_directories() {
        let temp = tempfile::tempdir().expect("directory");
        let outside = tempfile::tempdir().expect("outside");
        fs::write(outside.path().join("item.xml"), b"outside").expect("outside file");
        for target in [
            outside.path().to_path_buf(),
            outside.path().join("item.xml"),
        ] {
            std::os::unix::fs::symlink(target, temp.path().join("link")).expect("link");
            assert!(read_package_entries(temp.path()).is_err());
            fs::remove_file(temp.path().join("link")).expect("remove link");
        }
    }
}
