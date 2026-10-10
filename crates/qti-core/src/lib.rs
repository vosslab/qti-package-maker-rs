//! Owned, validated assessment items and question-bank infrastructure.
mod bank;
mod crc;
mod fingerprint;
mod item;
mod manifest;
pub mod media;
mod strings;
mod validate;
mod xml_validation;
pub mod zip;

pub use bank::{AddOutcome, AssetCollectionAction, BankError, CollectedAssets, ItemBank};
pub use crc::{CrcError, ItemCrc, get_crc16_from_string, secondary_string};
pub use fingerprint::{FieldId, FingerprintError, ItemFingerprint, MediaRef};
pub use item::{Item, ItemBody, ItemCommon, ItemKind, ItemRenderView};
pub use manifest::{ItemResource, ManifestConfig, ManifestError, QtiVersion, generate_manifest};
pub use media::{AssetSource, MemoryAssets};
pub use strings::*;
pub use validate::{ValidationError, clean_html_for_xml, validate_html, validate_item};
pub use zip::{EntryMap, NamedFile, ZipError, encode_zip, validate_entry_name};
