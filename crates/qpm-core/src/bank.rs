//! Ordered, validated collections of assessment items.
//!
//! Item ordering and validation live here; source metadata and collection live in
//! `media_assets`, while combining banks lives in `merge`.

mod media_assets;
mod merge;
#[cfg(test)]
mod tests;

use indexmap::IndexMap;
use thiserror::Error;

use crate::crc::ItemCrc;
use crate::item::{Item, ItemKind};
use crate::media::MediaError;

pub use media_assets::{AssetCollectionAction, CollectedAssets};

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
    /// Collecting media for an item failed with its source and target provenance.
    #[error(
        "could not {action} while collecting media for item {item_crc} (src={src:?}): {source}"
    )]
    CollectAsset {
        /// The item whose authored HTML contained the failed reference.
        item_crc: ItemCrc,
        /// The exact authored source, when a source had already been identified.
        src: Option<String>,
        /// The collection operation that failed.
        action: AssetCollectionAction,
        /// The underlying HTML or media-resolution failure.
        #[source]
        source: MediaError,
    },
    /// An HTML, URI, MIME, or media-resolution operation failed.
    #[error(transparent)]
    Media(#[from] MediaError),
    /// A presentation rewrite changed the item's source identity or assessment kind.
    #[error("item rewrite changed source identity or kind for item {item_crc}")]
    RewriteIdentity { item_crc: ItemCrc },
}

/// An insertion-ordered collection of validated assessment items.
#[derive(Clone, Debug)]
pub struct ItemBank {
    allow_mixed: bool,
    items: IndexMap<ItemCrc, Item>,
}

impl ItemBank {
    /// Creates an empty item bank.
    #[must_use]
    pub fn new(allow_mixed: bool) -> Self {
        Self {
            allow_mixed,
            items: IndexMap::new(),
        }
    }

    /// Returns whether this bank accepts multiple assessment-item kinds.
    #[must_use]
    pub const fn allow_mixed(&self) -> bool {
        self.allow_mixed
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

    /// Rewrites presentation without changing source identity, kind, order, or bank numbers.
    pub fn with_rewritten_items<E>(
        &self,
        mut rewrite: impl FnMut(&Item) -> Result<Item, E>,
    ) -> Result<Self, E>
    where
        E: From<BankError>,
    {
        let mut items = IndexMap::with_capacity(self.items.len());
        for (crc, item) in &self.items {
            let rewritten = rewrite(item)?;
            if rewritten.crc() != crc || rewritten.kind() != item.kind() {
                return Err(BankError::RewriteIdentity { item_crc: *crc }.into());
            }
            items.insert(*crc, rewritten.with_item_number(item.common().item_number));
        }
        Ok(Self {
            allow_mixed: self.allow_mixed,
            items,
        })
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
