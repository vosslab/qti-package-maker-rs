//! Public diagnostics emitted by the package integrity oracle.

/// The enforcement weight assigned by the parity authority.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// A broken package-format invariant or observed LMS import failure.
    Error,
    /// A Python-parity smoke signal that does not itself overrule an LMS result.
    Advisory,
}

/// Why a check is present and how strongly it can decide parity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Provenance {
    /// A required relationship in IMS CP, QTI, or Blackboard export data.
    FormatRequirement,
    /// A failure observed while importing into Blackboard.
    BlackboardImportFailure,
    /// Kept for parity with the Python checker but intentionally advisory.
    PythonParityAdvisory,
    /// The checker rejected input before it could safely inspect it.
    SafeInputHandling,
}

/// One precise package-integrity finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    /// Stable category useful to callers that should not parse prose.
    pub code: &'static str,
    /// Whether this finding is structural or advisory.
    pub severity: Severity,
    /// The source that gives the finding its parity authority.
    pub provenance: Provenance,
    /// Package path or input boundary where the problem was found.
    pub path: String,
    /// Human-readable explanation with the unresolved value.
    pub message: String,
}

impl Violation {
    pub(crate) fn error(
        code: &'static str,
        provenance: Provenance,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            provenance,
            path: path.into(),
            message: message.into(),
        }
    }

    pub(crate) fn advisory(
        code: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Advisory,
            provenance: Provenance::PythonParityAdvisory,
            path: path.into(),
            message: message.into(),
        }
    }
}
