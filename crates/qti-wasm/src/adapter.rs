//! Owned transport validation and dispatch; format algorithms remain in shared crates.

use qti_core::media::{MediaPolicy, MemoryAssets};
use qti_core::{EntryMap, ItemKind, NamedFile};
use qti_engines::{ConversionRequest, DocumentMetadata, ReadInput, WriteArtifact, WriteContext};
use qti_integrity::Severity;

use crate::diagnostics;
use crate::{
    Artifact, CheckPackageResult, ConversionInput, ConvertRequest, ConvertResult, Diagnostic,
    FormatInfo, FormatInventory, IntegrityReport, NamedBytes, PackageInput,
};

pub(crate) const MAX_ENTRY_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 10_000;

/// Returns only shared registry metadata, without touching input or ambient host state.
pub fn format_inventory() -> FormatInventory {
    FormatInventory {
        formats: qti_engines::ENGINES
            .iter()
            .map(|entry| FormatInfo {
                name: entry.name.into(),
                can_read: entry.can_read(),
                can_write: entry.can_write(),
                supported_kinds: entry
                    .supported_kinds
                    .iter()
                    .map(|kind| kind_name(*kind).into())
                    .collect(),
                media_policy: match entry.media_policy {
                    MediaPolicy::Package => "package",
                    MediaPolicy::ReferenceWarn => "referenceWarn",
                    MediaPolicy::PlaceholderWarn => "placeholderWarn",
                    MediaPolicy::Fail => "fail",
                }
                .into(),
                default_output_name: entry.default_output_name.into(),
            })
            .collect(),
    }
}

fn kind_name(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Mc => "MC",
        ItemKind::Ma => "MA",
        ItemKind::Match => "MATCH",
        ItemKind::Num => "NUM",
        ItemKind::Fib => "FIB",
        ItemKind::MultiFib => "MULTIFIB",
        ItemKind::Order => "ORDER",
    }
}

/// Native-callable transport oracle. The host supplies its once-resolved default date.
pub fn convert_request(request: ConvertRequest, default_date: &str) -> ConvertResult {
    match dispatch(request, default_date) {
        Ok(result) => result,
        Err(error) => ConvertResult::Error {
            error: *error,
            warnings: Vec::new(),
        },
    }
}

pub(crate) fn prepare_request(
    request: &ConvertRequest,
    default_date: &str,
) -> Result<PreparedRequest, Box<Diagnostic>> {
    let entry = qti_engines::engine(&request.output_format).ok_or_else(|| {
        Box::new(diagnostics::engine_error(
            &qti_engines::EngineError::UnknownEngine {
                name: request.output_format.clone(),
            },
        ))
    })?;
    let output_name = request
        .output_name
        .clone()
        .unwrap_or_else(|| entry.default_output_name.into());
    let date = request.document.date.as_deref().unwrap_or(default_date);
    validate_date(date).map_err(|mut diagnostic| {
        diagnostic.logical_name = Some(output_name.clone());
        diagnostic
    })?;
    let context = WriteContext::new(
        &output_name,
        DocumentMetadata {
            title: request
                .document
                .title
                .clone()
                .unwrap_or_else(|| "Exam".into()),
            date: date.into(),
        },
        request.shuffle_seed.into(),
    )
    .map_err(|error| {
        let mut diagnostic = diagnostics::engine_error(&error);
        diagnostic.logical_name = Some(output_name);
        Box::new(diagnostic)
    })?;
    let (name, bytes, archive, assets) = match request.input.clone() {
        ConversionInput::File {
            name,
            bytes,
            companions,
        } => {
            bound_size(bytes.len(), MAX_TOTAL_BYTES).map_err(|mut diagnostic| {
                diagnostic.logical_name = Some(name.clone());
                diagnostic
            })?;
            let assets = owned_assets(companions, bytes.len()).map_err(|mut diagnostic| {
                diagnostic.logical_name = Some(name.clone());
                diagnostic
            })?;
            (name, Some(bytes), None, assets)
        }
        ConversionInput::Entries { name, entries } => {
            let entries = owned_entries(entries).map_err(|mut diagnostic| {
                diagnostic.logical_name = Some(name.clone());
                diagnostic
            })?;
            (name, None, Some(entries), MemoryAssets::new())
        }
    };
    qti_core::validate_entry_name(&name).map_err(|error| {
        let mut diagnostic = Diagnostic::request("invalidName", error.to_string());
        diagnostic.logical_name = Some(name.clone());
        Box::new(diagnostic)
    })?;
    Ok(PreparedRequest {
        name,
        bytes,
        archive,
        assets,
        context,
    })
}

pub(crate) struct PreparedRequest {
    pub name: String,
    bytes: Option<Vec<u8>>,
    archive: Option<EntryMap>,
    pub assets: MemoryAssets,
    pub context: WriteContext,
}

impl PreparedRequest {
    pub fn input(&self) -> ReadInput<'_> {
        match (&self.bytes, &self.archive) {
            (Some(bytes), _) => ReadInput::File {
                name: &self.name,
                bytes,
            },
            (_, Some(entries)) => ReadInput::Archive {
                name: &self.name,
                entries,
            },
            _ => unreachable!("validated input has a payload"),
        }
    }
}

fn dispatch(request: ConvertRequest, default_date: &str) -> Result<ConvertResult, Box<Diagnostic>> {
    let prepared = prepare_request(&request, default_date)?;
    let result = qti_engines::convert(ConversionRequest {
        input_format: &request.input_format,
        output_format: &request.output_format,
        input: prepared.input(),
        assets: &prepared.assets,
        allow_mixed: request.allow_mixed,
        max_items: request.limit.map(|limit| limit as usize),
        context: &prepared.context,
    })
    .map_err(|error| {
        let mut diagnostic = diagnostics::engine_error(&error);
        diagnostic
            .logical_name
            .get_or_insert_with(|| prepared.name.clone());
        Box::new(diagnostic)
    })?;
    let warnings = result
        .read_warnings
        .into_iter()
        .map(|warning| diagnostics::read_warning(warning, &request.input_format, &prepared.name))
        .chain(
            result
                .write_warnings
                .into_iter()
                .map(diagnostics::write_warning),
        )
        .collect();
    Ok(ConvertResult::Success {
        artifact: result.artifact.map(artifact),
        item_count: result.item_count,
        warnings,
    })
}

pub(crate) fn artifact(artifact: WriteArtifact) -> Artifact {
    match artifact {
        WriteArtifact::File {
            primary,
            companions,
        } => Artifact::File {
            primary: named_file(primary),
            companions: companions.into_iter().map(named_file).collect(),
        },
        WriteArtifact::Directory { name, entries } => Artifact::Directory {
            name,
            entries: entries
                .into_iter()
                .map(|(name, bytes)| NamedBytes { name, bytes })
                .collect(),
        },
    }
}

fn named_file(file: NamedFile) -> NamedBytes {
    let (name, bytes) = file.into_parts();
    NamedBytes { name, bytes }
}

fn owned_assets(
    files: Vec<NamedBytes>,
    primary_size: usize,
) -> Result<MemoryAssets, Box<Diagnostic>> {
    if files.len() > MAX_FILES {
        return Err(Box::new(Diagnostic::request(
            "inputLimit",
            "input exceeds 10000 files",
        )));
    }
    let mut assets = MemoryAssets::new();
    let mut total = primary_size;
    for file in files {
        bound_size(file.bytes.len(), MAX_ENTRY_BYTES)?;
        total += file.bytes.len();
        bound_size(total, MAX_TOTAL_BYTES)?;
        assets.insert(&file.name, file.bytes).map_err(|error| {
            let mut diagnostic = Diagnostic::request("media", error.to_string());
            diagnostic.source = Some(file.name);
            Box::new(diagnostic)
        })?;
    }
    Ok(assets)
}

pub(crate) fn owned_entries(files: Vec<NamedBytes>) -> Result<EntryMap, Box<Diagnostic>> {
    if files.len() > MAX_FILES {
        return Err(Box::new(Diagnostic::request(
            "inputLimit",
            "input exceeds 10000 files",
        )));
    }
    let mut entries = EntryMap::new();
    let mut total = 0;
    for file in files {
        bound_size(file.bytes.len(), MAX_ENTRY_BYTES)?;
        total += file.bytes.len();
        bound_size(total, MAX_TOTAL_BYTES)?;
        qti_core::validate_entry_name(&file.name).map_err(|error| {
            let mut diagnostic = Diagnostic::request("invalidName", error.to_string());
            diagnostic.source = Some(file.name.clone());
            Box::new(diagnostic)
        })?;
        if entries.contains_key(&file.name) {
            let mut diagnostic = Diagnostic::request(
                "duplicateEntry",
                "input contains a duplicate logical filename",
            );
            diagnostic.source = Some(file.name);
            return Err(Box::new(diagnostic));
        }
        entries.insert(file.name, file.bytes);
    }
    qti_integrity::validate_entries(&entries)
        .map_err(|violation| Box::new(Diagnostic::request(violation.code, violation.message)))?;
    Ok(entries)
}

pub(crate) fn bound_size(size: usize, limit: usize) -> Result<(), Box<Diagnostic>> {
    if size > limit {
        return Err(Box::new(Diagnostic::request(
            "inputLimit",
            format!("payload exceeds {limit} bytes"),
        )));
    }
    Ok(())
}

fn validate_date(date: &str) -> Result<(), Box<Diagnostic>> {
    let bytes = date.as_bytes();
    let valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, value)| index == 4 || index == 7 || value.is_ascii_digit());
    if !valid {
        return Err(Box::new(Diagnostic::request(
            "invalidRequest",
            "document date must be YYYY-MM-DD",
        )));
    }
    let year: u32 = date[..4]
        .parse()
        .map_err(|_| Box::new(Diagnostic::request("invalidRequest", "invalid year")))?;
    let month: u32 = date[5..7]
        .parse()
        .map_err(|_| Box::new(Diagnostic::request("invalidRequest", "invalid month")))?;
    let day: u32 = date[8..]
        .parse()
        .map_err(|_| Box::new(Diagnostic::request("invalidRequest", "invalid day")))?;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > days {
        return Err(Box::new(Diagnostic::request(
            "invalidRequest",
            "document date is not a valid civil date",
        )));
    }
    Ok(())
}

/// Runs independent package checks and retains findings in their checker order.
pub fn check_package_request(input: PackageInput) -> CheckPackageResult {
    let entries = match input {
        PackageInput::Zip { bytes } => {
            if let Err(error) = bound_size(bytes.len(), MAX_TOTAL_BYTES) {
                return CheckPackageResult::Error { error: *error };
            }
            match qti_integrity::read_zip_entries(&bytes) {
                Ok(entries) => entries,
                Err(violation) => {
                    return CheckPackageResult::Success {
                        report: IntegrityReport {
                            errors: vec![diagnostics::finding(violation)],
                            warnings: Vec::new(),
                            entry_count: 0,
                        },
                    };
                }
            }
        }
        PackageInput::Entries { entries } => match owned_entries(entries) {
            Ok(entries) => entries,
            Err(error) => return CheckPackageResult::Error { error: *error },
        },
    };
    let mut report = IntegrityReport {
        errors: Vec::new(),
        warnings: Vec::new(),
        entry_count: entries.len(),
    };
    for violation in qti_integrity::check_entries(&entries) {
        match violation.severity {
            Severity::Error => report.errors.push(diagnostics::finding(violation)),
            Severity::Advisory => report.warnings.push(diagnostics::finding(violation)),
        }
    }
    CheckPackageResult::Success { report }
}
