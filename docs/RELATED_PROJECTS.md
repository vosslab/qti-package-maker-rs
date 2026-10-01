# Related projects

This workspace helps instructors and assessment-tool developers turn one BBQ question bank into
portable LMS packages and standalone teaching formats.

## Confirmed related projects

### qti-package-maker

- Relationship: upstream/fork/successor
- Link: https://github.com/vosslab/qti-package-maker
- Why visitors may care: The established Python converter is the behavioral source for this Rust
  port and remains the place to use the mature Python workflow while native feature-parity work is
  completed.
- Evidence: The active Rust-port plan explicitly defines this workspace as a parity port of
  `qti-package-maker`, and the Python project documents the same BBQ-to-Canvas, Blackboard,
  Moodle, readable-text, and HTML-self-test workflow.

## Possible related projects

### text2qti

- Relationship: companion/interoperability tool
- Link: https://github.com/gpoore/text2qti
- Why visitors may care: The native converter reads and writes text2qti-compatible plain-text
  banks, so its documentation helps users understand that interchange format and its QTI 1.2
  destination.
- Evidence: The official text2qti documentation describes Markdown-based plain text converted to
  QTI 1.2 quizzes for Canvas and other educational software; this workspace registers a
  `text2qti` reader and writer.
- Confidence: likely

### 1EdTech QTI 1.2 and 2.1 specifications

- Relationship: domain standard/resource
- Link: https://www.imsglobal.org/question/qtiv1p2/imsqti_asi_infov1p2.html and https://www.imsglobal.org/question/qtiv2p1/imsqti_infov2p1.html
- Why visitors may care: These specifications define the assessment-item models behind the
  Canvas QTI 1.2 and Blackboard QTI 2.1 package writers, and are useful when diagnosing an LMS
  import or validating a generated package.
- Evidence: The 1EdTech documents define QTI 1.2 and QTI 2.1 assessment information models; the
  native registry exposes writers for both versions and the package-integrity guide identifies
  their required constructs.
- Confidence: likely

### Moodle Aiken format

- Relationship: domain standard/resource
- Link: https://docs.moodle.org/405/en/Aiken_format
- Why visitors may care: The native Moodle Aiken writer produces the concise multiple-choice text
  format that Moodle documents for question-bank import.
- Evidence: Moodle's official documentation defines Aiken's question, answer-choice, and
  `ANSWER:` lines; this workspace registers a Moodle Aiken writer for multiple-choice items.
- Confidence: likely

## Evidence notes

The upstream relationship comes from the active Rust-port plan and the Python repository's public
README. The remaining entries passed a bounded seed search on the registered QTI, text2qti, and
Moodle Aiken formats, then a widening search of their official documentation. Links point to the
maintainers' documentation rather than third-party summaries.
