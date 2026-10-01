//! Shared, source-preserving image asset handling for readers and writers.
//!
//! Item HTML remains authored HTML.  This module derives asset records only at
//! the conversion boundary, so writer-specific rewrites cannot mutate a bank.

mod naming;
mod package;
mod policy;
mod resolve;
mod rewrite;

pub use naming::assign_output_names;
pub use package::packageable_assets;
pub use policy::{
    MediaAction, MediaPolicy, MediaPolicyDecision, MediaPolicyError, MediaWarning,
    apply_media_policy, placeholder_text,
};
pub use resolve::{
    AssetKind, ItemMediaAsset, MediaAsset, MediaError, classify_src, compute_content_hash,
    guess_mime_type, item_html_fields, resolve_asset, resolve_item_media_refs, resolve_local_path,
    scan_html_for_assets,
};
pub use rewrite::{
    FieldValue, replace_item_images, rewrite_field_value, rewrite_html_srcs, rewrite_item_media,
};
