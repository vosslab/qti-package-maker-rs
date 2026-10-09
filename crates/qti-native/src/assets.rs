//! Lazy local asset reads confined beneath an explicitly authorized directory.

use std::borrow::Cow;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use qti_core::media::{AssetSource, MediaError};

/// Lazy filesystem asset provider confined to a canonical authorized root.
///
/// Relative and authored absolute paths must canonicalize beneath that root; symlink escapes fail.
#[derive(Clone, Debug)]
pub struct DirectoryAssets {
    root: PathBuf,
}

impl DirectoryAssets {
    /// Records an authorized root without scanning or reading any media.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, crate::NativeError> {
        let root = root.into();
        let root = root
            .canonicalize()
            .map_err(|source| crate::error::io(&root, source))?;
        if !root.is_dir() {
            return Err(crate::NativeError::Invalid(
                "asset root is not a directory".into(),
            ));
        }
        Ok(Self { root })
    }

    fn resolve(&self, src: &str) -> Result<PathBuf, MediaError> {
        let source = Path::new(src);
        if src.contains('\\')
            || source
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(invalid(
                src,
                "local image path escapes authorized directory",
            ));
        }
        let root = &self.root;
        let joined = if source.is_absolute() {
            source.to_owned()
        } else {
            root.join(source)
        };
        let path = joined
            .canonicalize()
            .map_err(|error| read_error(src, &joined, error))?;
        // ASVS 5.2/5.3: canonical confinement rejects lexical traversal and symlink escapes.
        if !path.starts_with(root) || path == *root {
            return Err(invalid(
                src,
                "local image path escapes authorized directory",
            ));
        }
        Ok(path)
    }
}

impl AssetSource for DirectoryAssets {
    fn read(&self, src: &str) -> Result<Cow<'_, [u8]>, MediaError> {
        let path = self.resolve(src)?;
        let metadata = fs::metadata(&path).map_err(|error| read_error(src, &path, error))?;
        if !metadata.is_file() {
            return Err(invalid(src, "local image source is not a regular file"));
        }
        let limit = 32 * 1024 * 1024_u64;
        if metadata.len() > limit {
            return Err(invalid(src, "local image exceeds the 32 MiB byte limit"));
        }
        let file = fs::File::open(&path).map_err(|error| read_error(src, &path, error))?;
        let mut bytes = Vec::new();
        file.take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| read_error(src, &path, error))?;
        if bytes.len() as u64 > limit {
            return Err(invalid(src, "local image exceeds the 32 MiB byte limit"));
        }
        Ok(Cow::Owned(bytes))
    }
}

fn invalid(src: &str, reason: &str) -> MediaError {
    MediaError::InvalidName {
        src: src.into(),
        reason: reason.into(),
    }
}

fn read_error(src: &str, path: &Path, error: std::io::Error) -> MediaError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return MediaError::MissingAsset { src: src.into() };
    }
    MediaError::AssetRead {
        src: src.into(),
        message: format!("{}: {error}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_is_lazy_and_reads_only_selected_assets() {
        let temp = tempfile::tempdir().expect("directory");
        let assets = DirectoryAssets::new(temp.path()).expect("root");
        fs::write(temp.path().join("selected.png"), b"image").expect("fixture");
        assert_eq!(
            assets.read("selected.png").expect("lazy read").as_ref(),
            b"image"
        );
        assert!(matches!(
            assets.read("missing.png"),
            Err(MediaError::MissingAsset { src }) if src == "missing.png"
        ));
        assert_eq!(
            assets
                .read(temp.path().join("selected.png").to_str().expect("path"))
                .expect("authorized absolute")
                .as_ref(),
            b"image"
        );
    }

    #[test]
    fn invalid_parent_file_preserves_io_error_taxonomy() {
        let temp = tempfile::tempdir().expect("directory");
        fs::write(temp.path().join("parent"), b"not a directory").expect("fixture");
        let assets = DirectoryAssets::new(temp.path()).expect("root");
        assert!(matches!(
            assets.read("parent/image.png"),
            Err(MediaError::AssetRead { src, .. }) if src == "parent/image.png"
        ));
    }

    #[test]
    fn rejects_parent_traversal() {
        let temp = tempfile::tempdir().expect("directory");
        let assets = DirectoryAssets::new(temp.path()).expect("root");
        assert!(assets.read("../outside.png").is_err());
        assert!(assets.read("nested/../../outside.png").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        let temp = tempfile::tempdir().expect("directory");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        std::os::unix::fs::symlink(outside.path(), temp.path().join("image.png")).expect("link");
        assert!(
            DirectoryAssets::new(temp.path())
                .expect("root")
                .read("image.png")
                .is_err()
        );
    }
}
