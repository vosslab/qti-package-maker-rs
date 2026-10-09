//! Capture the current Python source for an explicit migration comparison.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PythonProvenance {
    pub(crate) source_path: String,
    pub(crate) snapshot_path: String,
    pub(crate) git_commit: String,
    pub(crate) working_tree_sha256: String,
    pub(crate) source_tree_sha256: String,
    pub(crate) imported_package_path: String,
}

#[derive(Debug)]
pub(crate) struct CurrentPython {
    pub(crate) root: PathBuf,
    pub(crate) provenance: PythonProvenance,
}

/// Snapshots tracked and non-ignored current source files, including working modifications.
pub(crate) fn resolve(
    repository: &Path,
    requested: Option<&Path>,
) -> Result<CurrentPython, String> {
    let requested = requested
        .map(PathBuf::from)
        .unwrap_or_else(|| repository.join("..").join("qti-package-maker"));
    let source = requested.canonicalize().map_err(|_| {
        format!(
            "current Python qti-package-maker checkout is unavailable: {}; pass --python-qti PATH",
            requested.display()
        )
    })?;
    if !source.join("qti_package_maker").is_dir()
        || !source.join("tools/bbq_converter.py").is_file()
        || !source.join("source_me.sh").is_file()
    {
        return Err(format!(
            "current Python source is not a qti-package-maker checkout: {}",
            source.display()
        ));
    }
    let git_commit = git_text(&source, &["rev-parse", "HEAD"])?;
    let working_tree_sha256 = digest(&git_bytes(&source, &["status", "--porcelain=v1", "-z"])?);
    let files = listed_files(&source)?;
    let source_tree_sha256 = tree_hash(&source, &files)?;
    let snapshot = repository
        .join("output_tables")
        .join("oracle_snapshot")
        .join(format!(
            "current_python_{}_{}",
            std::process::id(),
            &source_tree_sha256[..12]
        ));
    if snapshot.exists() {
        return Err(format!(
            "current Python snapshot path already exists: {}; inspect retained evidence or rerun after cleanup",
            snapshot.display()
        ));
    }
    copy_files(&source, &snapshot, &files)?;
    let files_after_copy = listed_files(&source)?;
    if files_after_copy != files
        || tree_hash(&source, &files_after_copy)? != source_tree_sha256
        || tree_hash(&snapshot, &files)? != source_tree_sha256
    {
        return Err("current Python source changed while its snapshot was captured; rerun the migration command".to_owned());
    }
    let imported_package_path = imported_package_path(&snapshot)?;
    let provenance = PythonProvenance {
        source_path: source.display().to_string(),
        snapshot_path: snapshot.display().to_string(),
        git_commit,
        working_tree_sha256,
        source_tree_sha256,
        imported_package_path,
    };
    fs::write(
        snapshot.join("CURRENT_PYTHON_PROVENANCE.json"),
        serde_json::to_vec_pretty(&provenance).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "current Python migration source: {} commit={} tree_sha256={}",
        provenance.source_path, provenance.git_commit, provenance.source_tree_sha256
    );
    Ok(CurrentPython {
        root: snapshot,
        provenance,
    })
}

fn git_text(source: &Path, arguments: &[&str]) -> Result<String, String> {
    String::from_utf8(git_bytes(source, arguments)?)
        .map(|value| value.trim().to_owned())
        .map_err(|error| error.to_string())
}

fn git_bytes(source: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "current Python source must be a readable Git checkout ({}): {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn listed_files(source: &Path) -> Result<Vec<PathBuf>, String> {
    let bytes = git_bytes(
        source,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let mut files = bytes
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .map(|value| String::from_utf8(value.to_vec()).map(PathBuf::from))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    files.retain(|path| source.join(path).exists());
    files.sort();
    files.dedup();
    if files.is_empty()
        || files.iter().any(|path| {
            path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, Component::ParentDir))
        })
    {
        return Err(format!(
            "current Python checkout has no safe tracked source files: {}",
            source.display()
        ));
    }
    Ok(files)
}

fn copy_files(source: &Path, destination: &Path, files: &[PathBuf]) -> Result<(), String> {
    for relative in files {
        let input = source.join(relative);
        let metadata = fs::symlink_metadata(&input).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "current Python source contains unsupported non-file entry: {}",
                input.display()
            ));
        }
        let output = destination.join(relative);
        fs::create_dir_all(output.parent().ok_or("source file has no parent")?)
            .map_err(|error| error.to_string())?;
        fs::copy(input, output).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn tree_hash(root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for relative in files {
        hasher.update(relative.as_os_str().as_encoded_bytes());
        hasher.update([0]);
        hasher.update(fs::read(root.join(relative)).map_err(|error| error.to_string())?);
        hasher.update([0]);
    }
    Ok(hex_digest(hasher.finalize().as_ref()))
}

fn imported_package_path(root: &Path) -> Result<String, String> {
    let command = format!(
        "source {} >/dev/null && PYTHONPATH={}:$PYTHONPATH python3 -c 'import qti_package_maker; print(qti_package_maker.__file__)'",
        shell_quote(&root.join("source_me.sh")),
        shell_quote(root)
    );
    let output = Command::new("bash")
        .args(["-lc", &command])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "could not import current Python qti-package-maker from {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let path = PathBuf::from(
        String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .trim(),
    );
    if !path.starts_with(root) {
        return Err(format!(
            "current Python import escaped its run snapshot: {}",
            path.display()
        ));
    }
    Ok(path.display().to_string())
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
fn digest(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_ref())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
