//! Bounded, non-extracting package input handling.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

use zip::read::ZipArchive;

use crate::checker;
use crate::types::{Provenance, Violation};

const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRY_COUNT: usize = 10_000;

/// Check a ZIP file or an already extracted package directory.
///
/// Input is only read into memory after the entry count, logical path, and
/// declared uncompressed size are bounded. ZIP members are never extracted.
/// This enforces ASVS 5.2.1, 5.2.3, and 5.3.3 at the file boundary.
pub fn check_package(path: impl AsRef<Path>) -> Vec<Violation> {
    let path = path.as_ref();
    let entries = if path.is_dir() {
        read_directory(path)
    } else {
        read_zip(path)
    };
    match entries {
        Ok(entries) => checker::check_entries(&entries),
        Err(violation) => vec![violation],
    }
}

fn read_zip(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, Violation> {
    let file = fs::File::open(path).map_err(|error| input_error(path, error))?;
    let mut archive = ZipArchive::new(file).map_err(|error| input_error(path, error))?;
    if archive.len() > MAX_ENTRY_COUNT {
        return Err(bound_error(path, "archive has too many entries"));
    }

    let mut entries = BTreeMap::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut member = archive
            .by_index(index)
            .map_err(|error| input_error(path, error))?;
        if member.is_dir() {
            continue;
        }
        let name = member.name().to_owned();
        validate_entry_name(&name, path)?;
        let size = member.size();
        if size > MAX_ENTRY_BYTES || total.saturating_add(size) > MAX_PACKAGE_BYTES {
            return Err(bound_error(
                path,
                "archive exceeds configured inspection bounds",
            ));
        }
        total += size;
        let mut bytes = Vec::with_capacity(size as usize);
        member
            .read_to_end(&mut bytes)
            .map_err(|error| input_error(path, error))?;
        if entries.insert(name.clone(), bytes).is_some() {
            return Err(Violation::error(
                "duplicate-entry",
                Provenance::SafeInputHandling,
                path.display().to_string(),
                format!("archive contains duplicate entry '{name}'"),
            ));
        }
    }
    Ok(entries)
}

fn read_directory(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Violation> {
    let mut entries = BTreeMap::new();
    let mut total = 0_u64;
    let mut count = 0_usize;
    visit_directory(root, root, &mut entries, &mut total, &mut count)?;
    Ok(entries)
}

fn visit_directory(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<String, Vec<u8>>,
    total: &mut u64,
    count: &mut usize,
) -> Result<(), Violation> {
    for child in fs::read_dir(directory).map_err(|error| input_error(directory, error))? {
        let child = child.map_err(|error| input_error(directory, error))?;
        *count += 1;
        if *count > MAX_ENTRY_COUNT {
            return Err(bound_error(root, "directory has too many entries"));
        }
        let file_type = child
            .file_type()
            .map_err(|error| input_error(&child.path(), error))?;
        if file_type.is_symlink() {
            return Err(Violation::error(
                "symlink-entry",
                Provenance::SafeInputHandling,
                child.path().display().to_string(),
                "extracted packages may not contain symbolic links",
            ));
        }
        if file_type.is_dir() {
            visit_directory(root, &child.path(), entries, total, count)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = child
            .path()
            .strip_prefix(root)
            .map_err(|error| input_error(&child.path(), error))?
            .to_string_lossy()
            .replace('\\', "/");
        validate_entry_name(&relative, root)?;
        let metadata = child
            .metadata()
            .map_err(|error| input_error(&child.path(), error))?;
        let size = metadata.len();
        if size > MAX_ENTRY_BYTES || total.saturating_add(size) > MAX_PACKAGE_BYTES {
            return Err(bound_error(
                root,
                "directory exceeds configured inspection bounds",
            ));
        }
        *total += size;
        let bytes = fs::read(child.path()).map_err(|error| input_error(&child.path(), error))?;
        entries.insert(relative, bytes);
    }
    Ok(())
}

fn validate_entry_name(name: &str, package: &Path) -> Result<(), Violation> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains('\\')
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(Violation::error(
            "unsafe-entry-path",
            Provenance::SafeInputHandling,
            package.display().to_string(),
            format!("package entry '{name}' is not a safe relative POSIX path"),
        ));
    }
    Ok(())
}

fn input_error(path: &Path, error: impl std::fmt::Display) -> Violation {
    Violation::error(
        "package-input",
        Provenance::SafeInputHandling,
        path.display().to_string(),
        format!("could not inspect package: {error}"),
    )
}

fn bound_error(path: &Path, message: &str) -> Violation {
    Violation::error(
        "package-size-limit",
        Provenance::SafeInputHandling,
        path.display().to_string(),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "qti-integrity-input-{}-{}.{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test"),
            extension
        ))
    }

    #[test]
    fn entry_count_limit_applies_to_zip_and_extracted_tree() {
        let zip_path = temporary_path("zip");
        let _ = fs::remove_file(&zip_path);
        let mut archive = zip::ZipWriter::new(fs::File::create(&zip_path).expect("test ZIP"));
        let options = zip::write::SimpleFileOptions::default();
        for index in 0..=MAX_ENTRY_COUNT {
            archive
                .start_file(format!("entries/{index}"), options)
                .expect("test ZIP entry");
        }
        archive.finish().expect("finish test ZIP");
        assert_eq!(check_package(&zip_path)[0].code, "package-size-limit");
        fs::remove_file(&zip_path).expect("remove test ZIP");

        let directory = temporary_path("directory");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("test directory");
        for index in 0..=MAX_ENTRY_COUNT {
            fs::write(directory.join(format!("entry-{index}")), []).expect("test directory entry");
        }
        assert_eq!(check_package(&directory)[0].code, "package-size-limit");
        fs::remove_dir_all(&directory).expect("remove test directory");
    }
}
