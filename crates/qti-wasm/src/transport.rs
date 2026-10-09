//! Rust-authoritative transport declarations emitted into the generated TypeScript package.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// Owned logical file; serde copies bytes and emits independent JavaScript Uint8Arrays.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(deny_unknown_fields)]
pub struct NamedBytes {
    /// Logical POSIX name validated by the adapter.
    pub name: String,
    /// Independently owned payload copied across the JavaScript boundary.
    #[serde(with = "serde_bytes")]
    #[tsify(type = "Uint8Array")]
    pub bytes: Vec<u8>,
}

/// One source document and its companions, or an already extracted package.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ConversionInput {
    /// Source document bytes and optional exact-name companion assets.
    File {
        /// Logical name used for input diagnostics.
        name: String,
        /// Independently owned source document bytes.
        #[serde(with = "serde_bytes")]
        #[tsify(type = "Uint8Array")]
        bytes: Vec<u8>,
        /// Companion assets; omitted input defaults to an empty list.
        #[serde(default)]
        companions: Vec<NamedBytes>,
    },
    /// Already extracted package members with a logical package name.
    Entries {
        /// Logical package name used for diagnostics.
        name: String,
        /// Independently owned extracted members.
        entries: Vec<NamedBytes>,
    },
}

/// Document defaults resolved once before invoking the shared engines.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Tsify)]
#[serde(deny_unknown_fields)]
pub struct DocumentOptions {
    /// Optional document title; omission uses `Exam`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional ISO date; omission uses the adapter-resolved UTC date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// A stateless conversion; omitted seed is zero, omitted date is the current UTC date.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertRequest {
    /// Registered reader format name.
    pub input_format: String,
    /// Registered writer format name.
    pub output_format: String,
    /// Owned source document or extracted package input.
    pub input: ConversionInput,
    /// Whether mixed item kinds are allowed; defaults to `false`.
    #[serde(default)]
    pub allow_mixed: bool,
    /// Optional maximum retained item count; omission retains all items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Optional logical output name; omission uses the registry default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_name: Option<String>,
    /// Document metadata; omitted fields use adapter defaults.
    #[serde(default)]
    pub document: DocumentOptions,
    /// Deterministic selection seed; defaults to zero.
    #[serde(default)]
    pub shuffle_seed: u32,
}

/// The exact logical artifact produced by the shared writer.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Artifact {
    /// One primary file with relative companion files.
    File {
        /// Independently owned primary artifact bytes.
        primary: NamedBytes,
        /// Independently owned companions relative to the primary parent.
        companions: Vec<NamedBytes>,
    },
    /// A logical directory with relative entry names.
    Directory {
        /// Logical directory name.
        name: String,
        /// Independently owned relative directory entries.
        entries: Vec<NamedBytes>,
    },
}

/// Recoverable diagnostics retain source ordering across the read and write stages.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct Warning {
    /// Read or write stage that produced this warning.
    pub stage: String,
    /// Stable warning category.
    pub category: String,
    /// Human-readable warning detail.
    pub message: String,
    /// Optional related format name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Optional related item identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Optional authored source or member name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Stable error categories with all provenance available at the boundary.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct Diagnostic {
    /// Stable error category.
    pub category: String,
    /// Human-readable error detail.
    pub message: String,
    /// Optional related format name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Optional related item identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Optional authored source or member name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Logical input or output name, independent of a particular asset/member source.
    #[serde(rename = "logicalName", skip_serializing_if = "Option::is_none")]
    pub logical_name: Option<String>,
}

/// Expected parser and request errors are values rather than JavaScript exceptions.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[tsify(missing_as_null)]
pub enum ConvertResult {
    /// Conversion completed; a writer may intentionally return no artifact.
    Success {
        /// Optional exact logical artifact.
        artifact: Option<Artifact>,
        /// Number of items after any requested limit.
        item_count: usize,
        /// Ordered recoverable diagnostics from reading and writing.
        warnings: Vec<Warning>,
    },
    /// Expected request, reader, or writer failure without a JavaScript exception.
    Error {
        /// Stable failure diagnostic.
        error: Diagnostic,
        /// Warnings produced before the failure.
        warnings: Vec<Warning>,
    },
}

/// Inventory is derived from the shared registry, including read/write directions.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct FormatInventory {
    /// Registry-derived format capabilities.
    pub formats: Vec<FormatInfo>,
}

/// Capabilities, naming and media policy copied from the sole format registry.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct FormatInfo {
    /// Registered format name.
    pub name: String,
    /// Whether the format can read input.
    pub can_read: bool,
    /// Whether the format can write output.
    pub can_write: bool,
    /// Item kinds the format supports.
    pub supported_kinds: Vec<String>,
    /// Format media handling policy.
    pub media_policy: String,
    /// Registry default logical output name.
    pub default_output_name: String,
}

/// Independent integrity input, without creating a mutable item bank.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PackageInput {
    /// Owned ZIP bytes for independent inspection.
    Zip {
        /// Independently owned ZIP payload.
        #[serde(with = "serde_bytes")]
        #[tsify(type = "Uint8Array")]
        bytes: Vec<u8>,
    },
    /// Independently owned extracted package members.
    Entries {
        /// Validated logical package entries.
        entries: Vec<NamedBytes>,
    },
}

/// Independent checker evidence; severity and provenance are preserved separately.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
pub struct IntegrityFinding {
    /// Stable checker finding code.
    pub code: String,
    /// Checker-assigned severity.
    pub severity: String,
    /// Checker subsystem that produced the finding.
    pub provenance: String,
    /// Logical package path involved.
    pub path: String,
    /// Human-readable finding detail.
    pub message: String,
}

/// Structural errors, advisory warnings and the number of decoded regular files.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    /// Structural failures found during inspection.
    pub errors: Vec<IntegrityFinding>,
    /// Advisory findings from inspection.
    pub warnings: Vec<IntegrityFinding>,
    /// Number of decoded regular files.
    pub entry_count: usize,
}

/// Invalid transport yields an error; malformed ZIP content yields integrity findings.
#[derive(Clone, Debug, Serialize, Deserialize, Tsify)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum CheckPackageResult {
    /// Transport was valid and inspection completed.
    Success {
        /// Independent checker findings and entry count.
        report: IntegrityReport,
    },
    /// Invalid boundary transport prevented inspection.
    Error {
        /// Stable request diagnostic.
        error: Diagnostic,
    },
}
