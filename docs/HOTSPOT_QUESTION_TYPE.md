# HOTSPOT question type

HOTSPOT questions ask students to identify one or more regions on an image. In the QPM/PLE
architecture, HOTSPOT currently belongs to PLE. QPM support is deferred until a concrete
interoperability need and a supported exchange path make conversion implementable.

## Current scope

QPM converts between external question formats. Its item model and PLE Native JSON writer support
the seven kinds listed in [QUESTION_TYPES.md](QUESTION_TYPES.md); HOTSPOT is outside that set.
PLE owns HOTSPOT authoring, presentation, and grading.

**LMS HOTSPOT interoperability remains nonexistent.** The QPM maintainer has access to Blackboard
Learn, Blackboard Ultra, and Canvas and reports that all three support creating HOTSPOT questions,
but none supports importing or exporting them. These platforms therefore provide no HOTSPOT
exchange path for QPM. Native HOTSPOT functionality and QTI compliance do not establish successful
transfer between LMS platforms.

## Standards and working interoperability

QTI defines a HOTSPOT representation. The
[1EdTech QTI 3 implementation guide](https://www.imsglobal.org/spec/qti/v3p0/impl)
describes images, geometric regions, response identifiers, and selection constraints. That
establishes a common data format; it does not establish a working LMS import/export pathway.

Evaluate these capabilities separately:

| Capability | What it establishes |
| --- | --- |
| Authoring | An instructor can create the question inside a platform. |
| Rendering | A student can answer the question inside that platform. |
| Import | A named destination accepts an external HOTSPOT package and preserves its behavior. |
| Export | A named source produces HOTSPOT data that another platform can consume faithfully. |

Schema-valid QTI, native authoring, and native rendering are insufficient evidence of the last
two capabilities. QPM planning follows demonstrated transfer and grading behavior.

## LMS evidence reviewed

Evidence date: 2026-10-11. The maintainer's firsthand findings for Blackboard Learn, Blackboard
Ultra, and Canvas are recorded in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Published documentation
and a reported import failure provide supporting context below. QPM has not run separate LMS
import/export tests for this review. Unverified entries concern platforms outside those findings.

| Platform | Native image interaction | HOTSPOT exchange evidence |
| --- | --- | --- |
| Blackboard Learn | HOTSPOT creation is available, per the maintainer. | Neither HOTSPOT import nor export is supported, per the maintainer. |
| Blackboard Ultra | HOTSPOT creation is available, per the maintainer; native variants are documented. | Neither HOTSPOT import nor export is supported, per the maintainer; the documented QTI supported-type list excludes HOTSPOT. |
| Canvas | HOTSPOT creation is available, per the maintainer; New Quizzes also documents an API. | Neither HOTSPOT import nor export is supported, per the maintainer; a QTI 2.1 import failure is also reported. |
| Moodle | Drag and Drop Markers provides image-region interaction. | Moodle XML is a separate native exchange format; standard QTI HOTSPOT transfer remains unverified. |
| D2L Brightspace | Native HOTSPOT support is not established by this review. | General QTI import is documented; HOTSPOT-specific import/export remains unverified. |

Blackboard documents native [HOTSPOT interactions](https://help.anthology.com/blackboard/student/en/assessments/work-on-an-assessment/hotspot-questions.html),
while its [QTI package guide](https://help.anthology.com/blackboard/instructor/en/assessments/questions/reuse-questions/qti-packages.html)
lists True/False, Multiple Choice, Fill in the Blank, and Essay. The native interaction does not
extend that documented exchange coverage.

Canvas documents [QTI 1.2 and 2.x imports](https://community.instructure.com/en/kb/articles/661050-how-do-i-import-a-quiz-from-a-qti-package-in-new-quizzes)
and [New Quizzes exports](https://community.instructure.com/en/kb/articles/661098-how-do-i-export-a-new-quiz-from-a-course).
[Canvas issue #2606](https://github.com/instructure/canvas-lms/issues/2606), opened 2026-03-17 and
still open at review, reports failure to import the QTI conformance example `VE_IP_06.zip`.
This report corroborates the maintainer's import finding; it is not a QPM-run reproduction.
General quiz export documentation does not establish HOTSPOT export support.

Moodle documents [Drag and Drop Markers](https://docs.moodle.org/en/Drag_and_drop_markers_question_type)
and [Moodle XML](https://docs.moodle.org/en/Moodle_XML_format). A Moodle-specific mapping would need
its own behavior checks; native-format exchange does not prove QTI HOTSPOT compatibility.
[D2L support describes QTI 1.2/1.21 imports](https://community.d2l.com/brightspace/discussion/6207/questions-regarding-quiz-questions-and-api-capabilities-in-brightspace),
without demonstrating HOTSPOT transfer.

### Canvas API evidence boundary

The [Canvas New Quiz Items API](https://developerdocs.instructure.com/services/canvas/resources/new_quiz_items)
documents uploading an image and creating a `hot-spot` item with its URL and interaction data.
This is programmatic native authoring, separate from importing or exporting HOTSPOT questions.
It identifies a possible Canvas-specific integration, but QPM has not implemented or verified it.
API creation alone does not reopen the current HOTSPOT conversion scope.

## Why support is deferred

Adding a QPM item kind or emitting PLE's `hotspot` JSON alone would not enable cross-platform
conversion. A usable conversion needs a source and destination that can exchange the image,
regions, response rules, and grading meaning. Without that path, QPM would add model, validation,
media-handling, and writer maintenance without delivering a usable conversion workflow.

PLE's HOTSPOT capability alone therefore does not justify extending QPM. HOTSPOT model changes,
Native JSON emission, and pre-binding surface coordination remain outside active QPM work.
Standards-compliant generation alone is also insufficient: selecting QTI 3 as a canonical HOTSPOT
representation or adding a writer would not resolve the missing demonstrated exchange path.

## Conditions for reconsideration

Reconsider QPM support when both conditions hold:

- A concrete interoperability need identifies the source and destination formats and the required
  conversion direction, such as moving HOTSPOT questions between PLE and another platform.
- A supported exchange path exists for that direction, with enough format documentation and
  representative files to implement a faithful mapping and evidence that the intended consumer
  handles HOTSPOT. Internal LMS functionality and QTI schema conformance alone do not establish
  this path.

A proposed conversion must preserve these parts of the question:

| Part | Meaning to preserve |
| --- | --- |
| Image | Image content, asset reference, and dimensions used to place regions |
| Regions | Geometry, coordinate system, and placement relative to the image |
| Response | Allowed selections and bindings to the correct regions |
| Grading | Accepted responses and scoring rules |

Any resulting implementation is scoped to the demonstrated format pair and direction, with
verification in the destination consumer. These are conditions for reconsidering scope, not
scheduled implementation tasks or a commitment to support every format or both directions.

## Existing image recovery

The Blackboard Original pool reader's
[media_hotspot.rs](../crates/qpm-engines/src/blackboard_export_zip/read/media_hotspot.rs) helper
recovers image files referenced by `matapplication` URIs. The reader skips unsupported HOTSPOT
questions. Recovering image bytes does not import interactive regions, answer bindings, or grading
rules, and does not establish HOTSPOT question support in QPM.

## Historical handoff proposals

The archived [NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md](archive/NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md)
and [NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md](archive/NATIVE_JSON_HANDOFF_AUDIT_2026_10_07.md)
preserve earlier HOTSPOT coordination proposals. Their proposed pre-binding work is superseded by
the current scope decision above. Those documents remain historical records; active QPM plans
carry no HOTSPOT implementation or binding task.
