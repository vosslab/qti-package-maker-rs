//! Persist completed portable artifacts at the native filesystem boundary.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use qti_engines::WriteArtifact;

use crate::{NativeError, error::io};

pub fn persist_artifact(
    artifact: &WriteArtifact,
    destination: &Path,
) -> Result<PathBuf, NativeError> {
    match artifact {
        WriteArtifact::File {
            primary,
            companions,
        } => {
            let parent = parent_of(destination);
            if !parent.is_dir() {
                return Err(NativeError::Invalid(format!(
                    "output parent does not exist: {}",
                    parent.display()
                )));
            }
            preflight_target(destination)?;
            let primary_target =
                parent.join(destination.file_name().ok_or_else(|| {
                    NativeError::Invalid("primary output has no filename".into())
                })?);
            let mut targets = BTreeMap::new();
            for companion in companions {
                // ASVS 5.3.2: validate portable names again before joining the native output root.
                qti_core::NamedFile::new(companion.name().to_owned(), Vec::new())
                    .map_err(|error| NativeError::Invalid(error.to_string()))?;
                let target = parent.join(companion.name());
                if target == primary_target {
                    return Err(NativeError::Invalid(
                        "companion conflicts with primary output".into(),
                    ));
                }
                preflight_parent(parent, Path::new(companion.name()))?;
                preflight_target(&target)?;
                if let Some(previous) = targets.insert(target, companion.bytes())
                    && previous != companion.bytes()
                {
                    return Err(NativeError::Invalid(
                        "conflicting companion output bytes".into(),
                    ));
                }
            }
            // A file target cannot also be a directory needed by another output.
            let all_targets = std::iter::once(primary_target.as_path())
                .chain(targets.keys().map(PathBuf::as_path))
                .collect::<Vec<_>>();
            for target in &all_targets {
                if all_targets
                    .iter()
                    .any(|other| other != target && other.starts_with(target))
                {
                    return Err(NativeError::Invalid(format!(
                        "output file conflicts with another output parent: {}",
                        target.display()
                    )));
                }
            }
            preflight_distinct_paths(parent, &all_targets)?;
            for (target, bytes) in targets {
                fs::create_dir_all(parent_of(&target)).map_err(|source| io(&target, source))?;
                write_atomic(&target, bytes)?;
            }
            write_atomic(destination, primary.bytes())?;
        }
        WriteArtifact::Directory { entries, .. } => {
            crate::ple_output::write_entries(entries, destination)?
        }
    }
    Ok(destination.to_path_buf())
}

// Probe the destination filesystem's name rules without changing any final output path.
fn preflight_distinct_paths(root: &Path, targets: &[&Path]) -> Result<(), NativeError> {
    let temporary = tempfile::tempdir_in(root).map_err(|source| io(root, source))?;
    for target in targets {
        let relative = target.strip_prefix(root).map_err(|error| {
            NativeError::Invalid(format!("output target is outside its parent: {error}"))
        })?;
        let probe = temporary.path().join(relative);
        fs::create_dir_all(parent_of(&probe)).map_err(|source| io(target, source))?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    NativeError::Invalid(format!(
                        "output targets alias on this filesystem: {}",
                        target.display()
                    ))
                } else {
                    io(target, source)
                }
            })?;
    }
    Ok(())
}

fn preflight_target(path: &Path) -> Result<(), NativeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(NativeError::Invalid(format!(
            "output target is not a regular file: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io(path, source)),
    }
}

fn parent_of(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), NativeError> {
    let directory = parent_of(path);
    if !directory.is_dir() {
        return Err(NativeError::Invalid(format!(
            "output parent does not exist: {}",
            directory.display()
        )));
    }
    let mut temporary =
        tempfile::NamedTempFile::new_in(directory).map_err(|source| io(directory, source))?;
    temporary
        .write_all(bytes)
        .map_err(|source| io(path, source))?;
    temporary.flush().map_err(|source| io(path, source))?;
    temporary
        .persist(path)
        .map_err(|error| io(path, error.error))?;
    Ok(())
}

// Existing child directories cannot redirect portable companion names outside the output root.
fn preflight_parent(root: &Path, logical: &Path) -> Result<(), NativeError> {
    let mut current = root.to_path_buf();
    if let Some(parent) = logical.parent() {
        for part in parent.components() {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_dir() => {}
                Ok(_) => {
                    return Err(NativeError::Invalid(format!(
                        "companion parent is not a real directory: {}",
                        current.display()
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(io(&current, source)),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qti_core::NamedFile;

    #[test]
    fn file_and_companion_bytes_are_persisted_beside_requested_output() {
        let temp = tempfile::tempdir().expect("directory");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", b"questions".to_vec()).expect("primary"),
            companions: vec![NamedFile::new("media/cell.png", b"image".to_vec()).expect("image")],
        };
        let destination = temp.path().join("custom.txt");
        persist_artifact(&artifact, &destination).expect("persist");
        assert_eq!(fs::read(destination).expect("primary"), b"questions");
        assert_eq!(
            fs::read(temp.path().join("media/cell.png")).expect("image"),
            b"image"
        );
    }

    #[test]
    fn failed_output_preserves_existing_destination() {
        let temp = tempfile::tempdir().expect("directory");
        let destination = temp.path().join("questions.txt");
        fs::write(&destination, b"original").expect("original");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", b"replacement".to_vec()).expect("primary"),
            companions: vec![
                NamedFile::new("questions.txt", b"conflict".to_vec()).expect("conflict"),
            ],
        };
        assert!(persist_artifact(&artifact, &destination).is_err());
        assert_eq!(fs::read(destination).expect("preserved"), b"original");
    }

    #[test]
    fn invalid_later_target_preserves_primary_and_earlier_companion() {
        let temp = tempfile::tempdir().expect("directory");
        let destination = temp.path().join("questions.txt");
        let first = temp.path().join("a.png");
        fs::write(&destination, b"old questions").expect("primary");
        fs::write(&first, b"old image").expect("companion");
        fs::create_dir(temp.path().join("z.png")).expect("invalid target");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", b"replacement".to_vec()).expect("primary"),
            companions: vec![
                NamedFile::new("a.png", b"new image".to_vec()).expect("first"),
                NamedFile::new("z.png", b"invalid".to_vec()).expect("later"),
            ],
        };
        assert!(persist_artifact(&artifact, &destination).is_err());
        assert_eq!(
            fs::read(destination).expect("preserved primary"),
            b"old questions"
        );
        assert_eq!(fs::read(first).expect("preserved companion"), b"old image");
    }

    #[test]
    fn conflicting_file_and_parent_names_fail_before_any_write() {
        let temp = tempfile::tempdir().expect("directory");
        let destination = temp.path().join("questions.txt");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", vec![1]).expect("primary"),
            companions: vec![
                NamedFile::new("media", vec![2]).expect("parent name"),
                NamedFile::new("media/cell.png", vec![3]).expect("child name"),
            ],
        };
        assert!(persist_artifact(&artifact, &destination).is_err());
        assert_eq!(
            fs::read_dir(temp.path()).expect("unchanged root").count(),
            0
        );
    }

    #[test]
    fn case_distinct_targets_respect_destination_filesystem_names() {
        let temp = tempfile::tempdir().expect("directory");
        let destination = temp.path().join("questions.txt");
        let first = temp.path().join("a.png");
        let second = temp.path().join("A.png");
        fs::write(&destination, b"old questions").expect("primary");
        fs::write(&first, b"old image").expect("image");
        let aliases = second.exists();
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", b"replacement".to_vec()).expect("primary"),
            companions: vec![
                NamedFile::new("a.png", b"lowercase".to_vec()).expect("lowercase"),
                NamedFile::new("A.png", b"uppercase".to_vec()).expect("uppercase"),
            ],
        };
        let result = persist_artifact(&artifact, &destination);
        if aliases {
            assert!(result.is_err());
            assert_eq!(
                fs::read(destination).expect("primary preserved"),
                b"old questions"
            );
            assert_eq!(fs::read(first).expect("image preserved"), b"old image");
        } else {
            result.expect("distinct names accepted on case-sensitive filesystem");
            assert_eq!(fs::read(first).expect("lowercase"), b"lowercase");
            assert_eq!(fs::read(second).expect("uppercase"), b"uppercase");
        }
    }

    #[cfg(unix)]
    #[test]
    fn companion_directory_symlink_is_rejected_before_primary_write() {
        let temp = tempfile::tempdir().expect("directory");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), temp.path().join("media")).expect("link");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", b"replacement".to_vec()).expect("primary"),
            companions: vec![NamedFile::new("media/cell.png", b"image".to_vec()).expect("image")],
        };
        assert!(persist_artifact(&artifact, &temp.path().join("questions.txt")).is_err());
        assert!(!temp.path().join("questions.txt").exists());
        assert!(!outside.path().join("cell.png").exists());
    }

    #[cfg(unix)]
    #[test]
    fn primary_symlink_is_rejected_before_companion_write() {
        let temp = tempfile::tempdir().expect("directory");
        let outside = tempfile::tempdir().expect("outside");
        let source = outside.path().join("source.txt");
        fs::write(&source, b"source").expect("source");
        let destination = temp.path().join("questions.txt");
        std::os::unix::fs::symlink(&source, &destination).expect("link");
        let artifact = WriteArtifact::File {
            primary: NamedFile::new("questions.txt", vec![1]).expect("primary"),
            companions: vec![NamedFile::new("cell.png", vec![2]).expect("companion")],
        };
        assert!(persist_artifact(&artifact, &destination).is_err());
        assert!(!temp.path().join("cell.png").exists());
        assert_eq!(fs::read(source).expect("source preserved"), b"source");
    }
}
