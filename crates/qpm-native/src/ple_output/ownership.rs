//! Proof that a prior directory contains exactly this writer's unchanged files.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{foreign, invalid, io, is_canonical_media_filename, is_question_filename};
use crate::NativeError;

pub(super) const MANIFEST_NAME: &str = ".qpm-ple-native-json";
const MARKER: &str = "qti-package-maker-rs/ple_native_json/v1";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnershipManifest {
    engine: String,
    files: BTreeMap<String, String>,
}

pub(super) fn manifest_bytes(files: &BTreeMap<PathBuf, &[u8]>) -> Result<Vec<u8>, NativeError> {
    let mut hashes = BTreeMap::new();
    for (path, bytes) in files {
        let name = path
            .to_str()
            .ok_or_else(|| invalid("export path is not UTF-8"))?;
        hashes.insert(name.to_owned(), digest(bytes));
    }
    serde_json::to_vec_pretty(&OwnershipManifest {
        engine: MARKER.to_owned(),
        files: hashes,
    })
    .map_err(|source| invalid(format!("could not serialize output manifest: {source}")))
}

pub(super) fn verify_owned_directory(destination: &Path) -> Result<(), NativeError> {
    let mut entries = fs::read_dir(destination).map_err(|source| io(destination, source))?;
    if entries
        .next()
        .transpose()
        .map_err(|source| io(destination, source))?
        .is_none()
    {
        return Ok(());
    }

    let manifest_path = destination.join(MANIFEST_NAME);
    let metadata = fs::symlink_metadata(&manifest_path).map_err(|_| {
        invalid("nonempty destination has no valid PLE Native JSON ownership manifest")
    })?;
    if !metadata.file_type().is_file() {
        return Err(foreign(&manifest_path));
    }
    let bytes = fs::read(&manifest_path).map_err(|source| io(&manifest_path, source))?;
    let manifest: OwnershipManifest = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("destination has an invalid PLE Native JSON ownership manifest"))?;
    if manifest.engine != MARKER || manifest.files.is_empty() {
        return Err(invalid(
            "destination has an invalid PLE Native JSON ownership manifest",
        ));
    }
    let mut question_count = 0;
    for (name, hash) in &manifest.files {
        let path = Path::new(name);
        if is_question_filename(name) {
            question_count += 1;
        } else if !is_canonical_media_filename(path) {
            return Err(invalid(
                "ownership manifest contains an invalid output path",
            ));
        }
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(invalid(
                "ownership manifest contains an invalid SHA256 digest",
            ));
        }
    }
    if question_count == 0 {
        return Err(invalid("ownership manifest contains no question JSON"));
    }

    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(destination).map_err(|source| io(destination, source))? {
        let entry = entry.map_err(|source| io(destination, source))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| io(&path, source))?;
        if entry.file_name() == MANIFEST_NAME {
            if !metadata.file_type().is_file() {
                return Err(foreign(&path));
            }
        } else if entry.file_name() == "media" && metadata.file_type().is_dir() {
            let mut found_media = false;
            for media_entry in fs::read_dir(&path).map_err(|source| io(&path, source))? {
                let media_entry = media_entry.map_err(|source| io(&path, source))?;
                let media_path = media_entry.path();
                let media_metadata =
                    fs::symlink_metadata(&media_path).map_err(|source| io(&media_path, source))?;
                if !media_metadata.file_type().is_file() {
                    return Err(foreign(&media_path));
                }
                let name = media_entry.file_name();
                let name = name.to_str().ok_or_else(|| foreign(&media_path))?;
                actual.insert(format!("media/{name}"));
                found_media = true;
            }
            if !found_media {
                return Err(foreign(&path));
            }
        } else {
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| foreign(&path))?;
            if !metadata.file_type().is_file() || !is_question_filename(name) {
                return Err(foreign(&path));
            }
            actual.insert(name.to_owned());
        }
    }
    if actual != manifest.files.keys().cloned().collect() {
        return Err(invalid("destination files do not match ownership manifest"));
    }
    for (name, expected) in manifest.files {
        let path = destination.join(name);
        let bytes = fs::read(&path).map_err(|source| io(&path, source))?;
        if digest(&bytes) != expected {
            return Err(invalid(format!(
                "destination file changed since export: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hash
}
