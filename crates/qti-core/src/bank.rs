//! Ordered, validated collections of assessment items and their media lifetime.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use tempfile::TempDir;
use thiserror::Error;

use crate::crc::ItemCrc;
use crate::item::{Item, ItemKind};
use crate::media::{self, AssetKind, MediaAsset, MediaError};

/// Derived package-ready media for an [`ItemBank`].
///
/// Assets are keyed by the exact authored `src` spelling and are freshly rebuilt on every
/// [`ItemBank::collect_assets`] call. No asset registry is retained by the bank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedAssets {
    assets: Vec<MediaAsset>,
    item_dependencies: IndexMap<ItemCrc, Vec<MediaAsset>>,
}

impl CollectedAssets {
    /// Returns each distinct authored image source in deterministic source order.
    #[must_use]
    pub fn assets(&self) -> &[MediaAsset] {
        &self.assets
    }

    /// Returns the distinct assets referenced by one item in document order.
    #[must_use]
    pub fn dependencies_for(&self, crc: &ItemCrc) -> Option<&[MediaAsset]> {
        self.item_dependencies.get(crc).map(Vec::as_slice)
    }

    /// Iterates item dependencies in bank insertion order.
    pub fn iter_item_dependencies(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ItemCrc, &[MediaAsset])> {
        self.item_dependencies
            .iter()
            .map(|(crc, assets)| (crc, assets.as_slice()))
    }
}

/// A directory against which local item-media references are resolved.
///
/// A temporary directory is retained by an [`Arc`] until every bank sharing it is dropped.
/// An external path is only a reference and is never deleted by this type.
#[derive(Clone, Debug)]
pub struct MediaBaseDir {
    storage: MediaDirectoryStorage,
}

#[derive(Clone, Debug)]
enum MediaDirectoryStorage {
    External(PathBuf),
    Temporary(Arc<TempDir>),
}

impl MediaBaseDir {
    /// References a directory whose lifecycle is controlled by the caller.
    #[must_use]
    pub fn external(path: impl Into<PathBuf>) -> Self {
        Self {
            storage: MediaDirectoryStorage::External(path.into()),
        }
    }

    /// Creates a bank-owned temporary media directory.
    pub fn temporary() -> Result<Self, BankError> {
        let directory = tempfile::Builder::new().prefix("qti_media_").tempdir()?;
        Ok(Self {
            storage: MediaDirectoryStorage::Temporary(Arc::new(directory)),
        })
    }

    /// Returns the directory used to resolve local media references.
    #[must_use]
    pub fn path(&self) -> &Path {
        match &self.storage {
            MediaDirectoryStorage::External(path) => path,
            MediaDirectoryStorage::Temporary(directory) => directory.path(),
        }
    }

    /// Returns whether this directory will be removed after its last shared owner drops.
    #[must_use]
    pub const fn is_temporary(&self) -> bool {
        matches!(self.storage, MediaDirectoryStorage::Temporary(_))
    }
}

impl PartialEq for MediaBaseDir {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl Eq for MediaBaseDir {}

/// The result of adding one item to an [`ItemBank`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddOutcome {
    /// The item was inserted at the end of the bank.
    Added { crc: ItemCrc },
    /// The bank already held this CRC, so the supplied item was not inserted.
    ///
    /// Callers that present diagnostics should treat this as Python's duplicate-item warning.
    Duplicate { crc: ItemCrc },
}

/// The conversion step that failed while collecting one item's media.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetCollectionAction {
    /// Scanning an authored HTML field for image sources.
    ScanHtml,
    /// Resolving an authored image source into a media asset.
    ResolveAsset,
}

impl std::fmt::Display for AssetCollectionAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanHtml => formatter.write_str("scan HTML"),
            Self::ResolveAsset => formatter.write_str("resolve asset"),
        }
    }
}

/// Errors produced while assembling or combining item banks.
#[derive(Debug, Error)]
pub enum BankError {
    /// A bank configured for one item kind received another.
    #[error("mixing item types is not allowed: allowed {allowed:?}, attempted {attempted:?}")]
    MixedItemKinds {
        /// The kind established by the first item in the bank.
        allowed: ItemKind,
        /// The kind of item which could not be added.
        attempted: ItemKind,
    },
    /// Two banks cannot use one resolution directory when they name different directories.
    #[error(
        "cannot merge item banks with different media base directories: left={left:?}, right={right:?}"
    )]
    DifferentMediaBaseDirs {
        /// The left bank's media directory.
        left: PathBuf,
        /// The right bank's media directory.
        right: PathBuf,
    },
    /// The operating system could not create a temporary media directory.
    #[error("could not create temporary media directory")]
    CreateMediaDirectory(#[source] std::io::Error),
    /// An authored image source requires a local, file-backed path.
    #[error("image source '{src}' must name a local file")]
    NonLocalImageSource {
        /// The rejected external URL or data URI.
        src: String,
    },
    /// A local image path escaped the configured directory through a symlink.
    #[error("image path '{path}' escapes media base directory '{base_dir}' through a symlink")]
    SymlinkEscape {
        /// The destination path after lexical normalization.
        path: PathBuf,
        /// The canonical configured media base directory.
        base_dir: PathBuf,
    },
    /// The operating system could not create a parent directory or write supplied image bytes.
    #[error("could not write image bytes for src '{src}' at {path}: {source}")]
    WriteImage {
        /// The authored image source.
        src: String,
        /// The validated destination path.
        path: PathBuf,
        /// The underlying filesystem failure.
        #[source]
        source: std::io::Error,
    },
    /// The configured media base could not be canonicalized before a write.
    #[error("could not canonicalize media base directory {path}: {source}")]
    CanonicalizeMediaBase {
        /// The configured directory.
        path: PathBuf,
        /// The underlying filesystem failure.
        #[source]
        source: std::io::Error,
    },
    /// Collecting media for an item failed with its source and target provenance.
    #[error(
        "could not {action} while collecting media for item {item_crc} (src={src:?}, resolved_path={resolved_path:?}): {source}"
    )]
    CollectAsset {
        /// The item whose authored HTML contained the failed reference.
        item_crc: ItemCrc,
        /// The exact authored source, when a source had already been identified.
        src: Option<String>,
        /// The lexical local path that was going to be resolved, when available.
        resolved_path: Option<PathBuf>,
        /// The collection operation that failed.
        action: AssetCollectionAction,
        /// The underlying HTML or media-resolution failure.
        #[source]
        source: MediaError,
    },
    /// An HTML, URI, MIME, or media-resolution operation failed.
    #[error(transparent)]
    Media(#[from] MediaError),
}

impl From<std::io::Error> for BankError {
    fn from(error: std::io::Error) -> Self {
        Self::CreateMediaDirectory(error)
    }
}

/// An insertion-ordered collection of validated assessment items.
#[derive(Clone, Debug)]
pub struct ItemBank {
    allow_mixed: bool,
    media_base_dir: Option<MediaBaseDir>,
    items: IndexMap<ItemCrc, Item>,
}

impl ItemBank {
    /// Creates an empty item bank.
    #[must_use]
    pub fn new(allow_mixed: bool) -> Self {
        Self {
            allow_mixed,
            media_base_dir: None,
            items: IndexMap::new(),
        }
    }

    /// Creates an empty item bank with a typed media-directory reference.
    #[must_use]
    pub fn with_media_base_dir(allow_mixed: bool, media_base_dir: MediaBaseDir) -> Self {
        Self {
            allow_mixed,
            media_base_dir: Some(media_base_dir),
            items: IndexMap::new(),
        }
    }

    /// Returns whether this bank accepts multiple assessment-item kinds.
    #[must_use]
    pub const fn allow_mixed(&self) -> bool {
        self.allow_mixed
    }

    /// Returns the local media resolution directory, if one is configured.
    #[must_use]
    pub fn media_base_dir(&self) -> Option<&Path> {
        self.media_base_dir.as_ref().map(MediaBaseDir::path)
    }

    /// Replaces the typed media-directory reference.
    ///
    /// Replacing a temporary reference only releases this bank's share; another bank retaining
    /// that reference keeps its directory alive.
    pub fn set_media_base_dir(&mut self, media_base_dir: Option<MediaBaseDir>) {
        self.media_base_dir = media_base_dir;
    }

    /// Spills image bytes below the configured media base and returns their resolved local path.
    ///
    /// The authored `src` must be a supported local image name. When no base directory exists,
    /// this creates a bank-owned temporary directory lazily. Existing external base directories
    /// remain caller-owned and are never removed by the bank.
    pub fn add_image(&mut self, src: &str, data_bytes: &[u8]) -> Result<PathBuf, BankError> {
        if media::classify_src(src) != AssetKind::Local {
            return Err(BankError::NonLocalImageSource {
                src: src.to_owned(),
            });
        }
        media::guess_mime_type(src)?;

        let temporary_base = if self.media_base_dir.is_none() {
            Some(MediaBaseDir::temporary()?)
        } else {
            None
        };
        let base_dir = self
            .media_base_dir()
            .or_else(|| temporary_base.as_ref().map(MediaBaseDir::path))
            .expect("an existing or freshly-created temporary directory is available");
        // ASVS 5.3.2: normalize and confine author-controlled paths before filesystem writes.
        let canonical_base = canonical_media_base(base_dir)?;
        let lexical_base = media::resolve_local_path(base_dir, ".")?;
        let lexical_destination = media::resolve_local_path(base_dir, src)?;
        let relative_destination =
            lexical_destination
                .strip_prefix(&lexical_base)
                .map_err(|_| BankError::SymlinkEscape {
                    path: lexical_destination.clone(),
                    base_dir: canonical_base.clone(),
                })?;
        let destination = canonical_base.join(relative_destination);
        let parent = destination
            .parent()
            .ok_or_else(|| BankError::SymlinkEscape {
                path: destination.clone(),
                base_dir: canonical_base.clone(),
            })?;
        create_safe_parent_dirs(&canonical_base, parent, src, &destination)?;
        match fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(BankError::SymlinkEscape {
                    path: destination,
                    base_dir: canonical_base,
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(BankError::WriteImage {
                    src: src.to_owned(),
                    path: destination,
                    source,
                });
            }
        }
        fs::write(&destination, data_bytes).map_err(|source| BankError::WriteImage {
            src: src.to_owned(),
            path: destination.clone(),
            source,
        })?;
        if self.media_base_dir.is_none() {
            self.media_base_dir = temporary_base;
        }
        Ok(destination)
    }

    /// Resolves all image references into a fresh package-ready asset view.
    ///
    /// Resolution is read-only with respect to item content. Each exact `src` is resolved once,
    /// dependencies retain document order per item, and collision-safe output names are assigned
    /// across the full bank.
    pub fn collect_assets(&self) -> Result<CollectedAssets, BankError> {
        let mut asset_by_src = BTreeMap::new();
        let mut item_dependencies = IndexMap::new();

        for (crc, item) in &self.items {
            let mut dependencies = Vec::new();
            let mut seen_sources = HashSet::new();
            for (_, html) in media::item_html_fields(item) {
                let sources = media::scan_html_for_assets(html).map_err(|source| {
                    BankError::CollectAsset {
                        item_crc: *crc,
                        src: None,
                        resolved_path: None,
                        action: AssetCollectionAction::ScanHtml,
                        source,
                    }
                })?;
                for src in sources {
                    if !asset_by_src.contains_key(&src) {
                        let resolved_path = local_asset_path(self.media_base_dir(), &src);
                        let asset = media::resolve_asset(&src, self.media_base_dir()).map_err(
                            |source| BankError::CollectAsset {
                                item_crc: *crc,
                                src: Some(src.clone()),
                                resolved_path,
                                action: AssetCollectionAction::ResolveAsset,
                                source,
                            },
                        )?;
                        asset_by_src.insert(src.clone(), asset);
                    }
                    let asset = asset_by_src
                        .get(&src)
                        .expect("asset was inserted when its source was first encountered");
                    if seen_sources.insert(src) {
                        dependencies.push(asset.clone());
                    }
                }
            }
            if !dependencies.is_empty() {
                item_dependencies.insert(*crc, dependencies);
            }
        }

        let mut assets = asset_by_src.into_values().collect::<Vec<_>>();
        media::assign_output_names(&mut assets);
        let output_names = assets
            .iter()
            .filter_map(|asset| {
                asset
                    .output_name
                    .as_ref()
                    .map(|name| (asset.src.clone(), name.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        for dependencies in item_dependencies.values_mut() {
            for asset in dependencies {
                asset.output_name = output_names.get(&asset.src).cloned();
            }
        }

        Ok(CollectedAssets {
            assets,
            item_dependencies,
        })
    }

    /// Returns the number of items in insertion order.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns whether this bank holds no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns an item by its zero-based insertion position.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Item> {
        self.items.get_index(index).map(|(_, item)| item)
    }

    /// Returns an item by its stable CRC identity.
    #[must_use]
    pub fn get_by_crc(&self, crc: &ItemCrc) -> Option<&Item> {
        self.items.get(crc)
    }

    /// Iterates over items in their insertion order.
    pub fn iter_ordered(&self) -> impl ExactSizeIterator<Item = &Item> {
        self.items.values()
    }

    /// Adds an item, assigning its one-based bank number when it is not a duplicate.
    pub fn add_item(&mut self, mut item: Item) -> Result<AddOutcome, BankError> {
        self.validate_kind(item.kind())?;
        let crc = *item.crc();
        if self.items.contains_key(&crc) {
            return Ok(AddOutcome::Duplicate { crc });
        }

        item.set_item_number(self.items.len() + 1);
        self.items.insert(crc, item);
        Ok(AddOutcome::Added { crc })
    }

    /// Returns a numbered, insertion-ordered subset of this bank.
    #[must_use]
    pub fn slice(&self, range: std::ops::Range<usize>) -> Self {
        let mut bank = Self {
            allow_mixed: self.allow_mixed,
            media_base_dir: self.media_base_dir.clone(),
            items: self
                .items
                .iter()
                .skip(range.start)
                .take(range.end.saturating_sub(range.start))
                .map(|(crc, item)| (*crc, item.clone()))
                .collect(),
        };
        bank.renumber_items();
        bank
    }

    /// Retains at most the first `limit` items and renumbers the retained items.
    pub fn trim_to(&mut self, limit: usize) {
        self.items = self.items.drain(..limit.min(self.items.len())).collect();
        self.renumber_items();
    }

    /// Orders items by CRC and renumbers them in that new order.
    pub fn sort_by_crc(&mut self) {
        self.items.sort_keys();
        self.renumber_items();
    }

    /// Assigns one-based item numbers in the current insertion order.
    pub fn renumber_items(&mut self) {
        for (number, item) in self.items.values_mut().enumerate() {
            item.set_item_number(number + 1);
        }
    }

    /// Merges two banks, keeping the left order and replacing duplicate CRC values from the right.
    ///
    /// The result shares its retained [`MediaBaseDir`] with its source bank(s), so a temporary
    /// directory remains available until the last relevant bank drops.
    pub fn merge(&self, other: &Self) -> Result<Self, BankError> {
        let allow_mixed = self.allow_mixed || other.allow_mixed;
        validate_merged_kinds(self, other, allow_mixed)?;
        let media_base_dir =
            merge_media_base_dirs(self.media_base_dir.as_ref(), other.media_base_dir.as_ref())?;

        let mut items = self.items.clone();
        for (crc, item) in &other.items {
            items.insert(*crc, item.clone());
        }

        Ok(Self {
            allow_mixed,
            media_base_dir,
            items,
        })
    }

    fn validate_kind(&self, attempted: ItemKind) -> Result<(), BankError> {
        if self.allow_mixed {
            return Ok(());
        }
        if let Some(allowed) = self.items.first().map(|(_, item)| item.kind())
            && allowed != attempted
        {
            return Err(BankError::MixedItemKinds { allowed, attempted });
        }
        Ok(())
    }
}

fn canonical_media_base(base_dir: &Path) -> Result<PathBuf, BankError> {
    fs::canonicalize(base_dir).map_err(|source| BankError::CanonicalizeMediaBase {
        path: base_dir.to_owned(),
        source,
    })
}

fn create_safe_parent_dirs(
    canonical_base: &Path,
    parent: &Path,
    src: &str,
    destination: &Path,
) -> Result<(), BankError> {
    let relative_parent =
        parent
            .strip_prefix(canonical_base)
            .map_err(|_| BankError::SymlinkEscape {
                path: destination.to_owned(),
                base_dir: canonical_base.to_owned(),
            })?;

    // Check the nearest existing ancestor before creating anything.  In particular, this catches
    // `base/escape -> outside` before a recursive directory creator could make `outside/new`.
    let mut existing_ancestor = parent;
    loop {
        match fs::symlink_metadata(existing_ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                existing_ancestor =
                    existing_ancestor
                        .parent()
                        .ok_or_else(|| BankError::SymlinkEscape {
                            path: destination.to_owned(),
                            base_dir: canonical_base.to_owned(),
                        })?;
            }
            Err(source) => {
                return Err(BankError::WriteImage {
                    src: src.to_owned(),
                    path: existing_ancestor.to_owned(),
                    source,
                });
            }
        }
    }
    let canonical_ancestor =
        fs::canonicalize(existing_ancestor).map_err(|source| BankError::WriteImage {
            src: src.to_owned(),
            path: existing_ancestor.to_owned(),
            source,
        })?;
    if canonical_ancestor != canonical_base && !canonical_ancestor.starts_with(canonical_base) {
        return Err(BankError::SymlinkEscape {
            path: destination.to_owned(),
            base_dir: canonical_base.to_owned(),
        });
    }

    let mut directory = canonical_base.to_owned();
    for component in relative_parent.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(BankError::SymlinkEscape {
                path: destination.to_owned(),
                base_dir: canonical_base.to_owned(),
            });
        };
        directory.push(name);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(BankError::SymlinkEscape {
                    path: destination.to_owned(),
                    base_dir: canonical_base.to_owned(),
                });
            }
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(BankError::WriteImage {
                    src: src.to_owned(),
                    path: directory,
                    source: std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "image parent path is not a directory",
                    ),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&directory).map_err(|source| BankError::WriteImage {
                    src: src.to_owned(),
                    path: directory.clone(),
                    source,
                })?;
            }
            Err(source) => {
                return Err(BankError::WriteImage {
                    src: src.to_owned(),
                    path: directory,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn local_asset_path(base_dir: Option<&Path>, src: &str) -> Option<PathBuf> {
    (media::classify_src(src) == AssetKind::Local)
        .then_some(base_dir)
        .flatten()
        .and_then(|base_dir| media::resolve_local_path(base_dir, src).ok())
}

impl Default for ItemBank {
    fn default() -> Self {
        Self::new(false)
    }
}

impl PartialEq for ItemBank {
    fn eq(&self, other: &Self) -> bool {
        self.items.len() == other.items.len()
            && self.items.keys().all(|crc| other.items.contains_key(crc))
    }
}

impl Eq for ItemBank {}

fn validate_merged_kinds(
    left: &ItemBank,
    right: &ItemBank,
    allow_mixed: bool,
) -> Result<(), BankError> {
    if allow_mixed {
        return Ok(());
    }
    let Some(allowed) = left.items.first().map(|(_, item)| item.kind()) else {
        return Ok(());
    };
    if let Some(attempted) = right.items.first().map(|(_, item)| item.kind())
        && allowed != attempted
    {
        return Err(BankError::MixedItemKinds { allowed, attempted });
    }
    Ok(())
}

fn merge_media_base_dirs(
    left: Option<&MediaBaseDir>,
    right: Option<&MediaBaseDir>,
) -> Result<Option<MediaBaseDir>, BankError> {
    match (left, right) {
        (Some(left), Some(right)) if left != right => Err(BankError::DifferentMediaBaseDirs {
            left: left.path().to_owned(),
            right: right.path().to_owned(),
        }),
        // A caller may reference the same path as a temporary owner.  Retain the temporary
        // handle in either order so the merged bank cannot lose the directory's lifetime.
        (Some(left), Some(_right)) if left.is_temporary() => Ok(Some(left.clone())),
        (Some(_), Some(right)) if right.is_temporary() => Ok(Some(right.clone())),
        (Some(left), _) => Ok(Some(left.clone())),
        (None, Some(right)) => Ok(Some(right.clone())),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{AddOutcome, AssetCollectionAction, BankError, ItemBank, MediaBaseDir};
    use crate::item::{Item, ItemBody};
    use crate::media::MediaError;

    fn mc(question: &str, answer: &str) -> Item {
        Item::new(
            question.to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: answer.to_owned(),
            },
        )
        .expect("test item is valid")
    }

    #[test]
    fn adds_in_order_and_reports_duplicate_without_replacing() {
        let item = mc("Question", "one");
        let crc = *item.crc();
        let mut bank = ItemBank::default();

        assert_eq!(
            bank.add_item(item).expect("add succeeds"),
            AddOutcome::Added { crc }
        );
        assert_eq!(
            bank.add_item(mc("Question", "two"))
                .expect("duplicate is valid"),
            AddOutcome::Duplicate { crc }
        );
        assert_eq!(bank.len(), 1);
        assert_eq!(bank.get(0).expect("first item").common().item_number, 1);
        assert!(
            matches!(bank.get(0).expect("first item").body(), ItemBody::Mc { answer, .. } if answer == "one")
        );
    }

    #[test]
    fn merge_replaces_from_right_but_keeps_left_order() {
        let mut left = ItemBank::default();
        left.add_item(mc("First", "one")).expect("add succeeds");
        left.add_item(mc("Shared", "one")).expect("add succeeds");
        let mut right = ItemBank::default();
        right
            .add_item(mc("Shared", "two").with_item_number(99))
            .expect("add succeeds");
        right.add_item(mc("Last", "one")).expect("add succeeds");

        let merged = left.merge(&right).expect("merge succeeds");
        let questions = merged
            .iter_ordered()
            .map(|item| item.common().question_text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(questions, ["First", "Shared", "Last"]);
        assert!(
            matches!(merged.get(1).expect("shared item").body(), ItemBody::Mc { answer, .. } if answer == "two")
        );
        assert_eq!(merged.get(1).expect("shared item").common().item_number, 1);
    }

    #[test]
    fn rejects_mixed_items_when_not_allowed() {
        let mut bank = ItemBank::default();
        bank.add_item(mc("Question", "one")).expect("add succeeds");
        let fib = Item::new(
            "Other question".to_owned(),
            ItemBody::Fib {
                answers: vec!["answer".to_owned()],
            },
        )
        .expect("test item is valid");

        assert!(matches!(
            bank.add_item(fib),
            Err(BankError::MixedItemKinds { .. })
        ));
    }

    #[test]
    fn slice_trim_sort_and_renumber_preserve_a_safe_read_api() {
        let mut bank = ItemBank::default();
        for question in ["Third", "First", "Second"] {
            bank.add_item(mc(question, "one")).expect("add succeeds");
        }
        let slice = bank.slice(1..3);
        assert_eq!(slice.len(), 2);
        assert_eq!(slice.get(0).expect("slice first").common().item_number, 1);

        bank.trim_to(2);
        assert_eq!(bank.len(), 2);
        bank.sort_by_crc();
        let crcs = bank
            .iter_ordered()
            .map(|item| item.crc())
            .collect::<Vec<_>>();
        assert!(crcs.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(bank.get(1).expect("second item").common().item_number, 2);
    }

    #[test]
    fn external_media_directory_is_never_deleted() {
        let directory = tempfile::tempdir().expect("temporary test directory");
        let path = directory.path().to_owned();
        let bank = ItemBank::with_media_base_dir(false, MediaBaseDir::external(path.clone()));
        drop(bank);
        assert!(path.exists());
    }

    #[test]
    fn temporary_media_directory_lives_until_every_shared_bank_drops() {
        let (path, merged) = {
            let media = MediaBaseDir::temporary().expect("create temporary media directory");
            assert!(media.is_temporary());
            let path = media.path().to_owned();
            let bank = ItemBank::with_media_base_dir(false, media);
            let merged = bank.merge(&ItemBank::default()).expect("merge succeeds");
            drop(bank);
            assert!(path.exists());
            (path, merged)
        };
        drop(merged);
        assert!(!path.exists());
    }

    #[test]
    fn merge_retains_same_path_temporary_owner_from_the_left() {
        let (path, merged) = {
            let media = MediaBaseDir::temporary().expect("create temporary media directory");
            let path = media.path().to_owned();
            let left = ItemBank::with_media_base_dir(false, media);
            let right = ItemBank::with_media_base_dir(false, MediaBaseDir::external(path.clone()));
            let merged = left.merge(&right).expect("merge succeeds");
            drop(left);
            drop(right);
            assert!(path.exists());
            (path, merged)
        };
        drop(merged);
        assert!(!path.exists());
    }

    #[test]
    fn merge_retains_same_path_temporary_owner_from_the_right() {
        let (path, merged) = {
            let media = MediaBaseDir::temporary().expect("create temporary media directory");
            let path = media.path().to_owned();
            let left = ItemBank::with_media_base_dir(false, MediaBaseDir::external(path.clone()));
            let right = ItemBank::with_media_base_dir(false, media);
            let merged = left.merge(&right).expect("merge succeeds");
            drop(left);
            drop(right);
            assert!(path.exists());
            (path, merged)
        };
        drop(merged);
        assert!(!path.exists());
    }

    #[test]
    fn add_image_creates_owned_media_lazily_and_rejects_escaping_sources() {
        let mut bank = ItemBank::default();
        assert!(matches!(
            bank.add_image("../escape.png", b"not a PNG"),
            Err(BankError::Media(MediaError::Traversal { .. }))
        ));
        assert!(bank.media_base_dir().is_none());

        let path = bank
            .add_image("figures/chart.png", b"not a PNG")
            .expect("spill image bytes");
        let base_dir = bank
            .media_base_dir()
            .expect("lazy temporary base")
            .to_owned();
        assert!(
            path.starts_with(
                fs::canonicalize(&base_dir).expect("canonical temporary base directory")
            )
        );
        assert_eq!(fs::read(&path).expect("read spilled image"), b"not a PNG");
        drop(bank);
        assert!(!base_dir.exists());
    }

    #[test]
    fn collect_assets_deduplicates_by_source_and_assigns_collision_safe_names() {
        let directory = tempfile::tempdir().expect("temporary media directory");
        fs::create_dir_all(directory.path().join("a")).expect("create a directory");
        fs::create_dir_all(directory.path().join("b")).expect("create b directory");
        fs::write(directory.path().join("a/figure.png"), b"first").expect("write first image");
        fs::write(directory.path().join("b/figure.png"), b"second").expect("write second image");
        let mut bank = ItemBank::with_media_base_dir(
            false,
            MediaBaseDir::external(directory.path().to_owned()),
        );
        let item = mc(
            "Diagram <img src='a/figure.png'/><img src='b/figure.png'/><img src='a/figure.png'/>",
            "one",
        );
        let crc = *item.crc();
        bank.add_item(item).expect("add item");

        let collected = bank.collect_assets().expect("collect media");
        assert_eq!(collected.assets().len(), 2);
        assert_eq!(collected.assets()[0].src, "a/figure.png");
        assert_eq!(
            collected.assets()[0].output_name.as_deref(),
            Some("figure.png")
        );
        assert_eq!(collected.assets()[1].src, "b/figure.png");
        assert_eq!(
            collected.assets()[1].output_name.as_deref(),
            Some("figure(1).png")
        );
        let dependencies = collected.dependencies_for(&crc).expect("item dependencies");
        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies[0].src, "a/figure.png");
        assert_eq!(dependencies[1].src, "b/figure.png");
    }

    #[test]
    fn collect_assets_reports_item_and_local_target_for_missing_files() {
        let directory = tempfile::tempdir().expect("temporary media directory");
        let mut bank = ItemBank::with_media_base_dir(
            false,
            MediaBaseDir::external(directory.path().to_owned()),
        );
        let item = mc("Diagram <img src='figures/missing.png'/>", "one");
        let crc = *item.crc();
        bank.add_item(item).expect("add item");

        match bank.collect_assets() {
            Err(BankError::CollectAsset {
                item_crc,
                src,
                resolved_path,
                action,
                ..
            }) => {
                assert_eq!(item_crc, crc);
                assert_eq!(src.as_deref(), Some("figures/missing.png"));
                assert_eq!(
                    resolved_path.as_deref(),
                    Some(directory.path().join("figures/missing.png").as_path())
                );
                assert_eq!(action, AssetCollectionAction::ResolveAsset);
            }
            result => panic!("expected contextual collection error, received {result:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn add_image_rejects_a_parent_symlink_that_escapes_the_media_base() {
        use std::os::unix::fs::symlink;

        let base = tempfile::tempdir().expect("temporary media base");
        let outside = tempfile::tempdir().expect("temporary outside directory");
        symlink(outside.path(), base.path().join("escape")).expect("create parent symlink");
        let mut bank =
            ItemBank::with_media_base_dir(false, MediaBaseDir::external(base.path().to_owned()));

        assert!(matches!(
            bank.add_image("escape/new/blocked.png", b"not a PNG"),
            Err(BankError::SymlinkEscape { .. })
        ));
        assert!(!outside.path().join("new").exists());
    }
}
