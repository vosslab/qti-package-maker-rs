//! Object-safe engine boundaries and the shared generic rendering loop.

use std::path::{Path, PathBuf};

use qti_core::media::{MediaPolicy, MediaWarning};
use qti_core::{Item, ItemBank, ItemKind, ItemRenderView};

use crate::EngineError;

/// A format writer held in the registry as a trait object.
pub trait Writer: Send + Sync {
    /// Stable registry name.
    fn name(&self) -> &'static str;
    /// The declared image behavior for this writer.
    fn media_policy(&self) -> MediaPolicy;
    /// Item kinds that this writer can represent.
    fn supported_kinds(&self) -> &'static [ItemKind];
    /// Writes one completed output package or document.
    fn save_package(
        &self,
        bank: &ItemBank,
        output: Option<&Path>,
    ) -> Result<WriteOutcome, EngineError>;
}

/// A completed output path and recoverable media diagnostics in render order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteOutcome {
    /// Completed package or document path, absent when no supported item rendered.
    pub path: Option<PathBuf>,
    /// Media policy diagnostics for items emitted by the writer.
    pub warnings: Vec<MediaWarning>,
}

/// A format reader held in the registry as a trait object.
pub trait Reader: Send + Sync {
    /// Stable registry name.
    fn name(&self) -> &'static str;
    /// Reads all valid records, retaining recoverable warnings in source order.
    fn read_items(&self, input: &Path, allow_mixed: bool) -> Result<ReadOutcome, EngineError>;
}

/// The precise source location available for a recoverable read warning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadLocation {
    /// The input has no finer record location.
    Input,
    /// A one-based text line.
    Line { line: usize },
    /// A one-based logical block.
    Block { number: usize },
    /// An archive entry.
    ArchiveEntry { name: String },
    /// A one-based item within a pool resource.
    PoolItem { resource: String, number: usize },
    /// A media token in a named resource.
    MediaToken { resource: String, token: String },
}

/// One recoverable reader failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadWarning {
    /// Input location retained for user action.
    pub location: ReadLocation,
    /// Human-readable recovery explanation.
    pub message: String,
}

/// A successful read, including records deliberately skipped after warnings.
#[derive(Clone, Debug)]
pub struct ReadOutcome {
    /// Validated items in input order.
    pub bank: ItemBank,
    /// Recoverable warnings in input order.
    pub warnings: Vec<ReadWarning>,
}

/// Optional generic transforms around a writer's private rendered value.
pub type PreRenderHook<'a> = dyn Fn(&Item) -> Result<ItemRenderView, EngineError> + 'a;
/// Optional post-render transform for a writer-private rendered value.
pub type PostRenderHook<'a, R> = dyn Fn(&Item, R) -> Result<R, EngineError> + 'a;

pub struct RenderHooks<'a, R> {
    /// Runs only after the writer confirms a supported kind.
    pub pre_render: Option<&'a PreRenderHook<'a>>,
    /// Runs only after a writer produces a value.
    pub post_render: Option<&'a PostRenderHook<'a, R>>,
}

impl<'a, R> Default for RenderHooks<'a, R> {
    fn default() -> Self {
        Self {
            pre_render: None,
            post_render: None,
        }
    }
}

/// Renders all supported bank items with writer-owned result type `R`.
///
/// Unsupported kinds are skipped before any hooks, exactly as Python's base engine loop does.
pub fn render_bank<R>(
    bank: &ItemBank,
    supported_kinds: &[ItemKind],
    render_item: impl Fn(&ItemRenderView) -> Result<Option<R>, EngineError>,
    hooks: RenderHooks<'_, R>,
) -> Result<Vec<R>, EngineError> {
    let mut rendered = Vec::new();
    for item in bank.iter_ordered() {
        if !supported_kinds.contains(&item.kind()) {
            continue;
        }
        let view = match hooks.pre_render {
            Some(transform) => transform(item)?,
            None => item.render_view(),
        };
        let Some(value) = render_item(&view)? else {
            continue;
        };
        rendered.push(match hooks.post_render {
            Some(transform) => transform(item, value)?,
            None => value,
        });
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{RenderHooks, render_bank};
    use qti_core::{Item, ItemBank, ItemBody, ItemKind};

    #[test]
    fn hooks_run_after_supported_check_and_only_for_rendered_values() {
        let mut bank = ItemBank::new(true);
        bank.add_item(
            Item::new(
                "MULTIPLE".into(),
                ItemBody::Mc {
                    choices: vec!["a".into(), "b".into()],
                    answer: "a".into(),
                },
            )
            .expect("MC"),
        )
        .expect("add MC");
        bank.add_item(
            Item::new(
                "ORDER".into(),
                ItemBody::Order {
                    answers: vec!["a".into(), "b".into(), "c".into()],
                },
            )
            .expect("ORDER"),
        )
        .expect("add ORDER");
        bank.add_item(
            Item::new(
                "FIB".into(),
                ItemBody::Fib {
                    answers: vec!["answer".into()],
                },
            )
            .expect("FIB"),
        )
        .expect("add FIB");
        let events = RefCell::new(Vec::new());
        let pre = |item: &Item| {
            events
                .borrow_mut()
                .push(format!("pre:{}", item.common().question_text));
            Ok(item.render_view())
        };
        let render = |item: &qti_core::ItemRenderView| {
            events
                .borrow_mut()
                .push(format!("render:{}", item.common().question_text));
            Ok((item.kind() == ItemKind::Mc).then(|| item.common().question_text.clone()))
        };
        let post = |item: &Item, value: String| {
            events
                .borrow_mut()
                .push(format!("post:{}", item.common().question_text));
            Ok(format!("done:{value}"))
        };
        let output = render_bank(
            &bank,
            &[ItemKind::Mc, ItemKind::Fib],
            render,
            RenderHooks {
                pre_render: Some(&pre),
                post_render: Some(&post),
            },
        )
        .expect("render bank");
        assert_eq!(output, ["done:MULTIPLE"]);
        assert_eq!(
            *events.borrow(),
            [
                "pre:MULTIPLE",
                "render:MULTIPLE",
                "post:MULTIPLE",
                "pre:FIB",
                "render:FIB"
            ]
        );
    }
}
