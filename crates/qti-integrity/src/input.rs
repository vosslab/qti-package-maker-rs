//! Bounded, non-extracting package input handling.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use zip::read::ZipArchive;

use crate::checker;
use crate::types::{Provenance, Violation};

mod zip_directory;

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRY_COUNT: usize = 10_000;
const ZIP_INPUT: &str = "package.zip";
const ENTRIES_INPUT: &str = "package entries";

/// Check a ZIP package supplied as bytes, without accessing the filesystem.
///
/// Counts, paths, symbolic links, and declared and actual decompressed sizes are
/// checked before the independent format checker receives any entries.
pub fn check_package(bytes: &[u8]) -> Vec<Violation> {
    match read_zip_entries(bytes) {
        Ok(entries) => checker::check_entries(&entries),
        Err(violation) => vec![violation],
    }
}

/// Decode a ZIP to safe, bounded package entries without extracting its members.
///
/// Explicit directory members are validated and counted but omitted from the
/// returned map. ASVS 5.2.1/5.2.3: limit the entry count and both declared and
/// actual uncompressed sizes. ASVS 5.2.5/5.3.3: reject links and unsafe paths.
/// Plausible embedded ZIP directories exceeding the metadata bounds are also
/// rejected because the ZIP decoder may select them after a directory error.
pub fn read_zip_entries(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Violation> {
    // zip-rs deduplicates central-directory filenames and allocates its metadata
    // before exposing archive.len(). Bound every possible fallback directory,
    // and validate our selected directory's original count before either step.
    let directory = zip_directory::inspect(bytes)?;
    let count = directory.count;
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|error| input_error(ZIP_INPUT, error))?;
    if archive.len() != count || archive.central_directory_start() != directory.start as u64 {
        return Err(input_error(
            ZIP_INPUT,
            "central directory entry count mismatch",
        ));
    }

    // Validate every member before any decompression, including directories.
    let mut names = BTreeSet::new();
    let mut declared_total = 0_u64;
    for index in 0..count {
        let member = archive
            .by_index_raw(index)
            .map_err(|error| input_error(ZIP_INPUT, error))?;
        let member_name = member
            .name()
            .map_err(|error| input_error(ZIP_INPUT, error))?;
        let name = member_name.as_ref();
        validate_entry_name(name, member.is_dir())?;
        if member.is_symlink() {
            return Err(Violation::error(
                "symlink-entry",
                Provenance::SafeInputHandling,
                name,
                "packages may not contain symbolic links",
            ));
        }
        if !names.insert(name.trim_end_matches('/').to_owned()) {
            return Err(duplicate_error(name));
        }
        declared_total = checked_total(name, member.size(), declared_total)?;
    }

    let mut entries = BTreeMap::new();
    let mut actual_total = 0_u64;
    for index in 0..count {
        let mut member = archive
            .by_index(index)
            .map_err(|error| input_error(ZIP_INPUT, error))?;
        let name = member
            .name()
            .map_err(|error| input_error(ZIP_INPUT, error))?
            .into_owned();
        let declared_size = member.size();
        let is_directory = member.is_dir();
        let remaining = MAX_ENTRY_BYTES.min(MAX_PACKAGE_BYTES - actual_total);
        // ASVS 5.2.1/5.2.3: metadata is untrusted; read one sentinel byte beyond
        // the remaining budget so forged sizes cannot cause unbounded allocation.
        let mut data = Vec::new();
        (&mut member)
            .take(remaining + 1)
            .read_to_end(&mut data)
            .map_err(|error| input_error(&name, error))?;
        actual_total = checked_total(&name, data.len() as u64, actual_total)?;
        if data.len() as u64 != declared_size {
            return Err(input_error(
                &name,
                "decompressed size differs from declared size",
            ));
        }
        if !is_directory {
            entries.insert(name, data);
        }
    }
    Ok(entries)
}

/// Validate logical names, count, and actual sizes independently of any producer.
///
/// Entries are regular files. A map cannot represent duplicate names or symbolic
/// links; adapters must reject that metadata before constructing it. This applies
/// ASVS 2.2.1/5.2.1/5.2.3/5.3.3 before any format-specific parsing.
pub fn validate_entries(entries: &BTreeMap<String, Vec<u8>>) -> Result<(), Violation> {
    if entries.len() > MAX_ENTRY_COUNT {
        return Err(bound_error(ENTRIES_INPUT, "package has too many entries"));
    }
    let mut total = 0;
    for (name, data) in entries {
        validate_entry_name(name, false)?;
        total = checked_total(name, data.len() as u64, total)?;
    }
    Ok(())
}

fn checked_total(name: &str, size: u64, total: u64) -> Result<u64, Violation> {
    if size > MAX_ENTRY_BYTES || size > MAX_PACKAGE_BYTES - total {
        return Err(bound_error(
            name,
            "package exceeds configured inspection bounds",
        ));
    }
    Ok(total + size)
}

fn validate_entry_name(name: &str, directory: bool) -> Result<(), Violation> {
    let path = if directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    // ASVS 2.2.1/5.3.3: use POSIX components on every target; OS path parsing
    // would accept different names on native and wasm32-unknown-unknown.
    if path.is_empty()
        || name.contains(['\\', ':'])
        || name.chars().any(char::is_control)
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(Violation::error(
            "unsafe-entry-path",
            Provenance::SafeInputHandling,
            name,
            format!("package entry {name:?} is not a safe relative POSIX path"),
        ));
    }
    Ok(())
}

fn duplicate_error(name: &str) -> Violation {
    Violation::error(
        "duplicate-entry",
        Provenance::SafeInputHandling,
        name,
        format!("archive contains duplicate entry {name:?}"),
    )
}

fn input_error(path: &str, error: impl std::fmt::Display) -> Violation {
    Violation::error(
        "package-input",
        Provenance::SafeInputHandling,
        path,
        format!("could not inspect package: {error}"),
    )
}

fn bound_error(path: &str, message: &str) -> Violation {
    Violation::error(
        "package-size-limit",
        Provenance::SafeInputHandling,
        path,
        message,
    )
}
