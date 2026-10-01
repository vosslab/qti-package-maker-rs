# Engine authoring

Add a format as a focused module in `crates/qti-engines/src/`, then register it in the one
compile-time `ENGINES` slice. The registry stores dynamic `Box<dyn Writer>` and `Box<dyn Reader>`
objects, but it is not a runtime plugin system. A new engine is a source change reviewed alongside
its formats, item kinds, media behavior, and tests.

## Required contract

A writer implements `Writer` and returns a `WriteOutcome`:

```rust
pub trait Writer: Send + Sync {
    fn name(&self) -> &'static str;
    fn media_policy(&self) -> MediaPolicy;
    fn supported_kinds(&self) -> &'static [ItemKind];
    fn save_package(
        &self,
        bank: &ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError>;
}
```

The result's `path` is `Some` only when a completed artifact exists. Put nonfatal image decisions
in `warnings` in writer then item order. Do not print warnings from library code. Use a concrete
`EngineError` variant for every fatal condition and propagate it with `?` to the CLI boundary.

A reader implements `Reader::read_items`. It must return every valid record in input order and a
located `ReadWarning` for recoverable skips. It must reject invalid input shape, unsafe archive
paths, malformed required data, or unresolved declared media as `EngineError`.

## Worked format module

This is the Rust translation of the former `template_class` example. It shows the shape of a
small text-only writer; it is a module pattern, not a Python subclass or dynamically imported
plugin.

```rust
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qti_core::media::{
    MediaPolicy, MediaWarning, apply_media_policy, replace_item_images,
};
use qti_core::{Item, ItemBank, ItemBody, ItemKind, ItemRenderView};

use crate::{
    EngineError, EngineOptions, RenderHooks, WriteOutcome, Writer, render_bank,
};

pub(crate) const NAME: &str = "template_text";
const KINDS: &[ItemKind] = &[ItemKind::Mc];

pub fn boxed_writer(_: EngineOptions) -> Box<dyn Writer> {
    Box::new(TemplateTextWriter)
}

struct TemplateTextWriter;

impl Writer for TemplateTextWriter {
    fn name(&self) -> &'static str {
        NAME
    }

    fn media_policy(&self) -> MediaPolicy {
        MediaPolicy::PlaceholderWarn
    }

    fn supported_kinds(&self) -> &'static [ItemKind] {
        KINDS
    }

    fn save_package(
        &self,
        bank: &ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError> {
        let output = output
            .unwrap_or_else(|| Path::new("template-text.txt"))
            .to_path_buf();
        let assets = bank.collect_assets()?;
        let pending_warnings = RefCell::new(BTreeMap::<String, Vec<MediaWarning>>::new());
        let warnings = RefCell::new(Vec::new());
        let pre_render = |item: &Item| {
            let decision = apply_media_policy(
                MediaPolicy::PlaceholderWarn,
                assets.dependencies_for(item.crc()).unwrap_or_default(),
                NAME,
                &item.crc().to_string(),
            )
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "media",
                message: error.to_string(),
            })?;
            pending_warnings
                .borrow_mut()
                .insert(item.crc().to_string(), decision.warnings);
            let placeholders = decision.placeholders;
            replace_item_images(item, |src, _alt| {
                placeholders
                    .get(src)
                    .cloned()
                    .unwrap_or_else(|| src.to_owned())
            })
            .map_err(|error| EngineError::InvalidFormat {
                engine: NAME,
                format: "HTML",
                message: error.to_string(),
            })
        };
        let post_render = |item: &Item, text| {
            warnings.borrow_mut().extend(
                pending_warnings
                    .borrow_mut()
                    .remove(&item.crc().to_string())
                    .unwrap_or_default(),
            );
            Ok(text)
        };
        let records = render_bank(
            bank,
            KINDS,
            render_mc,
            RenderHooks {
                pre_render: Some(&pre_render),
                post_render: Some(&post_render),
            },
        )?;
        if records.is_empty() {
            return Ok(WriteOutcome {
                path: None,
                warnings: warnings.into_inner(),
            });
        }
        fs::write(&output, records.concat()).map_err(|source| EngineError::Io {
            engine: NAME,
            path: output.clone(),
            source,
        })?;
        Ok(WriteOutcome {
            path: Some(output),
            warnings: warnings.into_inner(),
        })
    }
}

fn render_mc(item: &ItemRenderView) -> Result<Option<String>, EngineError> {
    let ItemBody::Mc { choices, answer } = item.body() else {
        return Err(EngineError::UnsupportedItemKind {
            engine: NAME,
            kind: item.kind(),
        });
    };
    let mut record = format!("{}\n", item.common().question_text);
    for (index, choice) in choices.iter().enumerate() {
        record.push_str(&format!("{}. {choice}\n", index + 1));
    }
    record.push_str(&format!("ANSWER: {answer}\n\n"));
    Ok(Some(record))
}
```

The module is pedagogical and is not registered by this repository. Its complete flow matters:
media is resolved before file creation, placeholder rewrites apply only to an `ItemRenderView`,
warnings are retained only for rendered records, and filesystem failure carries the engine and path.

Declare the module in `qti-engines/src/lib.rs`, then add one `EngineEntry` to
`qti-engines/src/registry.rs`:

```rust
EngineEntry {
    name: "template_text",
    media_policy: MediaPolicy::PlaceholderWarn,
    make_writer: Some(crate::template_text::boxed_writer),
    make_reader: None,
}
```

The registry test must list the new writer, its policy, and any reader. Do not add duplicate
name-to-engine tables in the CLI or tests; the registry is the authority.

## Representation and media

Choose `supported_kinds()` from formats the target can actually represent. `render_bank` skips
unsupported kinds before pre- or post-render hooks. A text writer that requires all input to be
representable may validate its bank first and return `UnsupportedItemKind` instead.

Apply a policy from [MEDIA.md](MEDIA.md) before creating output. Rewrites belong in an
`ItemRenderView`, never the validated `Item`: item CRC identity and source HTML stay immutable.
For package writers, collect media, assign collision-safe names, rewrite the view, and include
local bytes in the package. For text-only output, preserve references or substitute readable text
according to the policy.

## Factory and option rules

Factories receive owned `EngineOptions`. Use `DocumentMetadata` only when the format needs a title
or date. A factory must not obtain a new clock value; the CLI resolves its local civil date once.
Only the shared conversion layer owns `--html-to-image`: a ZIP writer receiving a direct true
option must either use that shared facade or reject the request clearly. It must not perform a
second per-writer conversion during CLI fan-out.

## Authoring checklist

- Document the format and its exact supported kinds in [FORMATS.md](FORMATS.md).
- State media behavior in [MEDIA.md](MEDIA.md) and match its registry policy.
- Add unit tests for rendering and meaningful fatal/recoverable input boundaries.
- Add a semantic round-trip or package-integrity test when the format has a reader or archive.
- Add CLI coverage when a new public flag, selection behavior, or artifact contract is introduced.
- Update `docs/CHANGELOG.md` and record a settled cross-cutting decision in
  `docs/DESIGN_DECISIONS.md`.
