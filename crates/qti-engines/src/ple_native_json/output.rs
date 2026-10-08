//! Staged loose-file output with manifest-verified replacement.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::NativeExport;
use crate::EngineError;

#[path = "ownership.rs"]
mod ownership;

const ENGINE: &str = "ple_native_json";
const FORMAT: &str = "PLE Native JSON output";

pub(super) fn write_export(export: &NativeExport, destination: &Path) -> Result<(), EngineError> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if destination.file_name().is_none() {
        return Err(invalid("output must name a directory"));
    }
    let staged = tempfile::Builder::new()
        .prefix(".ple-stage-")
        .tempdir_in(parent)
        .map_err(|source| io(parent, source))?;
    let mut files = BTreeMap::<PathBuf, &[u8]>::new();
    for question in &export.questions {
        if question.item_number == 0 {
            return Err(invalid("item number must be positive"));
        }
        insert_file(
            &mut files,
            PathBuf::from(question_filename(question.item_number)),
            question.source_json.as_bytes(),
        )?;
        for file in &question.files {
            // ASVS 5.3.2: output paths come from generated item numbers or one validated
            // media filename component, never directly from an authored source path.
            if !is_canonical_media_filename(&file.path) {
                return Err(invalid("export contains a media path outside media/"));
            }
            insert_file(&mut files, file.path.clone(), &file.bytes)?;
        }
    }
    let manifest = ownership::manifest_bytes(&files)?;
    for (path, bytes) in files {
        let target = staged.path().join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| io(parent, source))?;
        }
        fs::write(&target, bytes).map_err(|source| io(&target, source))?;
    }
    let manifest_path = staged.path().join(ownership::MANIFEST_NAME);
    fs::write(&manifest_path, manifest).map_err(|source| io(&manifest_path, source))?;
    ownership::verify_owned_directory(staged.path()).map_err(|error| {
        invalid(format!(
            "staged output does not match its ownership manifest; filesystem may alias output names: {error}"
        ))
    })?;

    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            if !metadata.file_type().is_dir() {
                return Err(invalid("destination is not a real directory"));
            }
            ownership::verify_owned_directory(destination)?;
            let backup = tempfile::Builder::new()
                .prefix(".ple-backup-")
                .tempdir_in(parent)
                .map_err(|source| io(parent, source))?;
            let previous = backup.path().join("previous");
            fs::rename(destination, &previous).map_err(|source| io(destination, source))?;
            if let Err(error) = ownership::verify_owned_directory(&previous) {
                if let Err(restore_error) = fs::rename(&previous, destination) {
                    let retained = backup.keep();
                    return Err(invalid(format!(
                        "prior output changed during replacement: {error}; could not restore it: {restore_error}; prior output is at {}",
                        retained.join("previous").display()
                    )));
                }
                return Err(error);
            }
            if let Err(source) = fs::rename(staged.path(), destination) {
                // Keep the old output intact if publishing the complete staged tree fails.
                if let Err(restore_error) = fs::rename(&previous, destination) {
                    // Preserve the backup on disk for manual recovery if restoration fails.
                    let retained = backup.keep();
                    return Err(invalid(format!(
                        "could not publish output: {source}; could not restore prior output: {restore_error}; prior output is at {}",
                        retained.join("previous").display()
                    )));
                }
                return Err(io(destination, source));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::rename(staged.path(), destination).map_err(|source| io(destination, source))?;
        }
        Err(source) => return Err(io(destination, source)),
    }
    Ok(())
}

fn insert_file<'a>(
    files: &mut BTreeMap<PathBuf, &'a [u8]>,
    path: PathBuf,
    bytes: &'a [u8],
) -> Result<(), EngineError> {
    if let Some(previous) = files.insert(path.clone(), bytes)
        && previous != bytes
    {
        return Err(invalid(format!(
            "conflicting export bytes for {}",
            path.display()
        )));
    }
    Ok(())
}

fn question_filename(number: usize) -> String {
    format!("item_{number:05}.json")
}

fn is_question_filename(name: &str) -> bool {
    let Some(number) = name
        .strip_prefix("item_")
        .and_then(|value| value.strip_suffix(".json"))
    else {
        return false;
    };
    number
        .parse::<usize>()
        .is_ok_and(|parsed| parsed > 0 && question_filename(parsed) == name)
}

fn media_filename(path: &Path) -> Option<&std::ffi::OsStr> {
    let mut components = path.components();
    match (components.next(), components.next(), components.next()) {
        (Some(Component::Normal(root)), Some(Component::Normal(name)), None) if root == "media" => {
            Some(name)
        }
        _ => None,
    }
}

fn is_canonical_media_filename(path: &Path) -> bool {
    media_filename(path).is_some_and(|name| Path::new("media").join(name) == path)
}

fn foreign(path: &Path) -> EngineError {
    invalid(format!(
        "destination contains a file not owned by this engine: {}",
        path.display()
    ))
}

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidFormat {
        engine: ENGINE,
        format: FORMAT,
        message: message.into(),
    }
}

fn io(path: &Path, source: std::io::Error) -> EngineError {
    EngineError::Io {
        engine: ENGINE,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
