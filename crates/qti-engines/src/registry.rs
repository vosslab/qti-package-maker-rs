//! The sole compile-time authority for supported formats and their metadata.

use qti_core::ItemKind;
use qti_core::media::MediaPolicy;

use crate::{Reader, Writer};

/// Host-resolved document labels for a conversion run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    /// Student-facing document title.
    pub title: String,
    /// ISO-8601 civil date in `YYYY-MM-DD` form.
    pub date: String,
}

/// Constructors and fixed metadata for one known engine.
pub struct EngineEntry {
    /// User-visible registry name.
    pub name: &'static str,
    /// Fixed media policy.
    pub media_policy: MediaPolicy,
    /// Item kinds this writer can represent, in registry display order.
    pub supported_kinds: &'static [ItemKind],
    /// Default logical output name for callers without their own naming policy.
    pub default_output_name: &'static str,
    /// Constructs the conventional output name from a caller-resolved content name.
    pub output_name_for_content: fn(&str) -> String,
    /// Whether the native HTML-to-image prepass can target this format.
    pub native_rendering: bool,
    /// Creates the writer if this format emits output.
    pub make_writer: Option<fn() -> Box<dyn Writer>>,
    /// Creates the reader if this format accepts input.
    pub make_reader: Option<fn() -> Box<dyn Reader>>,
}

impl EngineEntry {
    /// Reports whether this statically-known entry has a reader.
    #[must_use]
    pub const fn can_read(&self) -> bool {
        self.make_reader.is_some()
    }

    /// Reports whether this statically-known entry has a writer.
    #[must_use]
    pub const fn can_write(&self) -> bool {
        self.make_writer.is_some()
    }
}

const ALL_KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
    ItemKind::Order,
];
const QTI12_KINDS: &[ItemKind] = &[
    ItemKind::Mc,
    ItemKind::Ma,
    ItemKind::Match,
    ItemKind::Num,
    ItemKind::Fib,
    ItemKind::MultiFib,
];
const OKLA_KINDS: &[ItemKind] = &[ItemKind::Mc, ItemKind::Ma, ItemKind::Match, ItemKind::Fib];
const TEXT_KINDS: &[ItemKind] = &[ItemKind::Mc, ItemKind::Ma, ItemKind::Num, ItemKind::Fib];
const MC_KINDS: &[ItemKind] = &[ItemKind::Mc];

/// Complete portable reader/writer inventory shared by native and browser hosts.
pub const ENGINES: &[EngineEntry] = &[
    EngineEntry {
        name: "html_selftest",
        media_policy: MediaPolicy::Package,
        supported_kinds: ALL_KINDS,
        default_output_name: "selftest.html",
        output_name_for_content: |name| format!("selftest-{name}.html"),
        native_rendering: false,
        make_writer: Some(crate::html_selftest::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "exam_yaml",
        media_policy: MediaPolicy::ReferenceWarn,
        supported_kinds: ALL_KINDS,
        default_output_name: "exam.yaml",
        output_name_for_content: |name| format!("exam-{name}.yaml"),
        native_rendering: false,
        make_writer: Some(crate::exam_yaml::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "okla_chrst_bqgen",
        media_policy: MediaPolicy::PlaceholderWarn,
        supported_kinds: OKLA_KINDS,
        default_output_name: "okla.txt",
        output_name_for_content: |name| format!("okla-{name}.txt"),
        native_rendering: false,
        make_writer: Some(crate::okla_chrst_bqgen::boxed_writer),
        make_reader: Some(crate::okla_chrst_bqgen::boxed_reader),
    },
    EngineEntry {
        name: "text2qti",
        media_policy: MediaPolicy::ReferenceWarn,
        supported_kinds: TEXT_KINDS,
        default_output_name: "text2qti-package.txt",
        output_name_for_content: |name| format!("text2qti-{name}.txt"),
        native_rendering: false,
        make_writer: Some(crate::text2qti::boxed_writer),
        make_reader: Some(crate::text2qti::boxed_reader),
    },
    EngineEntry {
        name: "blackboard_export_zip",
        media_policy: MediaPolicy::Package,
        supported_kinds: QTI12_KINDS,
        default_output_name: "blackboard-export.zip",
        output_name_for_content: |name| format!("bez-{name}.zip"),
        native_rendering: true,
        make_writer: Some(crate::blackboard_export_zip::boxed_writer),
        make_reader: Some(crate::blackboard_export_zip::boxed_reader),
    },
    EngineEntry {
        name: "moodle_aiken",
        media_policy: MediaPolicy::PlaceholderWarn,
        supported_kinds: MC_KINDS,
        default_output_name: "moodle-aiken.txt",
        output_name_for_content: |name| format!("aiken-{name}.txt"),
        native_rendering: false,
        make_writer: Some(crate::moodle_aiken::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "human_readable",
        media_policy: MediaPolicy::ReferenceWarn,
        supported_kinds: ALL_KINDS,
        default_output_name: "human-readable.html",
        output_name_for_content: |name| format!("human-{name}.html"),
        native_rendering: false,
        make_writer: Some(crate::human_readable::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "canvas_qti_v1_2",
        media_policy: MediaPolicy::Package,
        supported_kinds: QTI12_KINDS,
        default_output_name: "qti12-package.zip",
        output_name_for_content: |name| format!("qti12-{name}.zip"),
        native_rendering: true,
        make_writer: Some(crate::canvas_qti_v1_2::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "blackboard_qti_v2_1",
        media_policy: MediaPolicy::Package,
        supported_kinds: ALL_KINDS,
        default_output_name: "qti21-package.zip",
        output_name_for_content: |name| format!("qti21-{name}.zip"),
        native_rendering: true,
        make_writer: Some(crate::blackboard_qti_v2_1::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "bbq_text_upload",
        media_policy: MediaPolicy::ReferenceWarn,
        supported_kinds: ALL_KINDS,
        default_output_name: "bbq-upload.txt",
        output_name_for_content: |name| format!("bbq-{name}.txt"),
        native_rendering: false,
        make_writer: Some(crate::bbq_text_upload::boxed_writer),
        make_reader: Some(crate::bbq_text_upload::boxed_reader),
    },
    EngineEntry {
        name: "ple_native_json",
        media_policy: MediaPolicy::Package,
        supported_kinds: ALL_KINDS,
        default_output_name: "ple",
        output_name_for_content: |name| format!("ple-{name}"),
        native_rendering: true,
        make_writer: Some(crate::ple_native_json::boxed_writer),
        make_reader: None,
    },
];

/// Looks up a format in the sole metadata inventory.
#[must_use]
pub fn engine(name: &str) -> Option<&'static EngineEntry> {
    ENGINES.iter().find(|entry| entry.name == name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ENGINES, engine};

    #[test]
    fn registry_factories_and_metadata_describe_all_formats() {
        assert_eq!(ENGINES.len(), 11);
        assert_eq!(ENGINES.iter().filter(|entry| entry.can_read()).count(), 4);
        let mut names = BTreeSet::new();
        let content_names = [
            "selftest-genetics.html",
            "exam-genetics.yaml",
            "okla-genetics.txt",
            "text2qti-genetics.txt",
            "bez-genetics.zip",
            "aiken-genetics.txt",
            "human-genetics.html",
            "qti12-genetics.zip",
            "qti21-genetics.zip",
            "bbq-genetics.txt",
            "ple-genetics",
        ];
        for (entry, expected_name) in ENGINES.iter().zip(content_names) {
            assert!(names.insert(entry.name), "duplicate registry name");
            let writer = (entry.make_writer.expect("writer"))();
            assert_eq!(writer.name(), entry.name);
            assert_eq!(writer.media_policy(), entry.media_policy);
            assert_eq!(writer.supported_kinds(), entry.supported_kinds);
            qti_core::validate_entry_name(entry.default_output_name).expect("safe default name");
            assert_eq!((entry.output_name_for_content)("genetics"), expected_name);
            if let Some(factory) = entry.make_reader {
                assert_eq!(factory().name(), entry.name);
            }
            assert_eq!(engine(entry.name).expect("lookup").name, entry.name);
        }
        assert!(engine("unknown").is_none());
        assert_eq!(
            ENGINES
                .iter()
                .filter(|entry| entry.native_rendering)
                .count(),
            4
        );
    }
}
