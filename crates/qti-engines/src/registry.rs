//! The sole compile-time authority for supported engines.

use std::time::{SystemTime, UNIX_EPOCH};

use qti_core::media::MediaPolicy;

use crate::{Reader, Writer};

/// Stable document labels resolved once before engines are constructed.
///
/// The CLI supplies its local civil date and the input stem.  Direct library
/// callers receive the deterministic UTC default and may replace both fields
/// before cloning options into registry factories.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    /// Student-facing document title.
    pub title: String,
    /// ISO-8601 civil date in `YYYY-MM-DD` form.
    pub date: String,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: "exam".to_owned(),
            date: current_utc_date(),
        }
    }
}

/// Common factory options. Only designated package writers may honor image conversion later.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EngineOptions {
    /// Request native table/canvas image conversion where supported.
    pub html_to_image: bool,
    /// Resolved metadata shared by all factories for one conversion run.
    pub document: DocumentMetadata,
}

/// Constructors and fixed metadata for one known engine.
pub struct EngineEntry {
    /// User-visible registry name.
    pub name: &'static str,
    /// Fixed media policy.
    pub media_policy: MediaPolicy,
    /// Creates the writer if this format emits output.
    pub make_writer: Option<fn(EngineOptions) -> Box<dyn Writer>>,
    /// Creates the reader if this format accepts input.
    pub make_reader: Option<fn(EngineOptions) -> Box<dyn Reader>>,
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

/// Probe registry proving that heterogeneous reader/writer trait objects are object safe.
pub const ENGINES: &[EngineEntry] = &[
    EngineEntry {
        name: "html_selftest",
        media_policy: MediaPolicy::Package,
        make_writer: Some(crate::html_selftest::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "exam_yaml",
        media_policy: MediaPolicy::ReferenceWarn,
        make_writer: Some(crate::exam_yaml::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "okla_chrst_bqgen",
        media_policy: MediaPolicy::PlaceholderWarn,
        make_writer: Some(crate::okla_chrst_bqgen::boxed_writer),
        make_reader: Some(crate::okla_chrst_bqgen::boxed_reader),
    },
    EngineEntry {
        name: "text2qti",
        media_policy: MediaPolicy::ReferenceWarn,
        make_writer: Some(crate::text2qti::boxed_writer),
        make_reader: Some(crate::text2qti::boxed_reader),
    },
    EngineEntry {
        name: "blackboard_export_zip",
        media_policy: MediaPolicy::Package,
        make_writer: Some(crate::blackboard_export_zip::boxed_writer),
        make_reader: Some(crate::blackboard_export_zip::boxed_reader),
    },
    EngineEntry {
        name: "moodle_aiken",
        media_policy: MediaPolicy::PlaceholderWarn,
        make_writer: Some(crate::moodle_aiken::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "human_readable",
        media_policy: MediaPolicy::ReferenceWarn,
        make_writer: Some(crate::human_readable::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "canvas_qti_v1_2",
        media_policy: MediaPolicy::Package,
        make_writer: Some(crate::canvas_qti_v1_2::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "blackboard_qti_v2_1",
        media_policy: MediaPolicy::Package,
        make_writer: Some(crate::blackboard_qti_v2_1::boxed_writer),
        make_reader: None,
    },
    EngineEntry {
        name: "bbq_text_upload",
        media_policy: MediaPolicy::ReferenceWarn,
        make_writer: Some(crate::bbq_text_upload::boxed_writer),
        make_reader: Some(crate::bbq_text_upload::boxed_reader),
    },
];

fn current_utc_date() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let (year, month, day) = civil_date_from_unix_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Converts Unix epoch days to the proleptic Gregorian civil date.
fn civil_date_from_unix_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use qti_core::media::MediaPolicy;

    use super::{DocumentMetadata, ENGINES, EngineOptions, civil_date_from_unix_days};

    #[test]
    fn registry_has_the_complete_writer_and_reader_inventory() {
        let expected_writers = [
            ("html_selftest", MediaPolicy::Package),
            ("exam_yaml", MediaPolicy::ReferenceWarn),
            ("okla_chrst_bqgen", MediaPolicy::PlaceholderWarn),
            ("text2qti", MediaPolicy::ReferenceWarn),
            ("blackboard_export_zip", MediaPolicy::Package),
            ("moodle_aiken", MediaPolicy::PlaceholderWarn),
            ("human_readable", MediaPolicy::ReferenceWarn),
            ("canvas_qti_v1_2", MediaPolicy::Package),
            ("blackboard_qti_v2_1", MediaPolicy::Package),
            ("bbq_text_upload", MediaPolicy::ReferenceWarn),
        ];
        assert_eq!(ENGINES.len(), expected_writers.len());
        for (name, policy) in expected_writers {
            let entry = ENGINES
                .iter()
                .find(|entry| entry.name == name)
                .expect("declared writer");
            assert_eq!(entry.media_policy, policy, "{name}");
            assert!(entry.can_write(), "{name}");
        }
        let expected_readers = [
            "bbq_text_upload",
            "text2qti",
            "okla_chrst_bqgen",
            "blackboard_export_zip",
        ];
        assert_eq!(ENGINES.iter().filter(|entry| entry.can_read()).count(), 4);
        for name in expected_readers {
            assert!(
                ENGINES
                    .iter()
                    .any(|entry| entry.name == name && entry.can_read()),
                "{name} reader"
            );
        }
    }

    #[test]
    fn registry_factories_consume_cloned_owned_options() {
        let options = EngineOptions::default();
        for entry in ENGINES {
            let writer = (entry.make_writer.expect("writer"))(options.clone());
            assert_eq!(writer.name(), entry.name);
        }
        for entry in ENGINES.iter().filter(|entry| entry.can_read()) {
            let reader = (entry.make_reader.expect("reader"))(options.clone());
            assert_eq!(reader.name(), entry.name);
        }
    }

    #[test]
    fn default_metadata_is_owned_and_uses_utc_date() {
        let options = EngineOptions::default();
        assert_eq!(options.document.title, "exam");
        assert_eq!(options.document.date.len(), 10);
        assert_eq!(options.document.date.as_bytes()[4], b'-');
        assert_eq!(options.document.date.as_bytes()[7], b'-');
        assert_eq!(civil_date_from_unix_days(0), (1970, 1, 1));
        assert_eq!(civil_date_from_unix_days(19_782), (2024, 2, 29));

        let replacement = DocumentMetadata {
            title: "Biology 301".to_owned(),
            date: "2026-09-30".to_owned(),
        };
        let copied = EngineOptions {
            html_to_image: false,
            document: replacement,
        }
        .clone();
        assert_eq!(copied.document.title, "Biology 301");
    }
}
