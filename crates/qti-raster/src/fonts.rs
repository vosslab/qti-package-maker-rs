//! Bundled Atkinson font resources used by the native table rasterizer.
//!
//! The renderer intentionally constructs its font database from these two embedded files.  That
//! makes text metrics independent of the host's installed fonts and keeps table conversion fully
//! offline.

use std::{cell::RefCell, sync::Arc};

use cosmic_text::{FontSystem, fontdb::Source};

/// Family name registered by the Atkinson Hyperlegible Next variable font.
pub const ATKINSON_NEXT_FAMILY: &str = "Atkinson Hyperlegible Next";
/// Family name registered by the Atkinson Hyperlegible Mono variable font.
pub const ATKINSON_MONO_FAMILY: &str = "Atkinson Hyperlegible Mono";

/// Atkinson Hyperlegible Next variable-font bytes, embedded in every renderer binary.
pub const ATKINSON_NEXT_BYTES: &[u8] =
    include_bytes!("../fonts/atkinson_hyperlegible_next_variable.ttf");
/// Atkinson Hyperlegible Mono variable-font bytes, embedded in every renderer binary.
pub const ATKINSON_MONO_BYTES: &[u8] =
    include_bytes!("../fonts/atkinson_hyperlegible_mono_variable.ttf");
/// License distributed with the bundled Atkinson Hyperlegible Next face.
pub const ATKINSON_NEXT_OFL: &str = include_str!("../fonts/OFL-Atkinson-Hyperlegible-Next.txt");
/// License distributed with the bundled Atkinson Hyperlegible Mono face.
pub const ATKINSON_MONO_OFL: &str = include_str!("../fonts/OFL-Atkinson-Hyperlegible-Mono.txt");

/// Creates a font system containing only the two bundled Atkinson faces.
///
/// A caller keeps this system alive while it lays out and paints a table so the font database's
/// face identifiers remain valid.  No host fonts, network fonts, or filesystem font discovery is
/// used.
#[must_use]
fn new_font_system() -> FontSystem {
    FontSystem::new_with_fonts([
        Source::Binary(Arc::new(ATKINSON_NEXT_BYTES.to_vec())),
        Source::Binary(Arc::new(ATKINSON_MONO_BYTES.to_vec())),
    ])
}

thread_local! {
    static FONT_SYSTEM: RefCell<FontSystem> = RefCell::new(new_font_system());
}

/// Borrows the calling thread's renderer font system.
///
/// A per-thread system avoids repeated font database construction while allowing table rendering
/// to run in parallel.  Glyph cache keys are consumed inside the same table-rendering thread.
pub(crate) fn with_font_system<R>(operation: impl FnOnce(&mut FontSystem) -> R) -> R {
    FONT_SYSTEM.with(|system| operation(&mut system.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_two_nonempty_licensed_font_faces() {
        let system = new_font_system();
        let names: Vec<_> = system
            .db()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.as_str()))
            .collect();
        assert!(names.iter().any(|name| name == &ATKINSON_NEXT_FAMILY));
        assert!(names.iter().any(|name| name == &ATKINSON_MONO_FAMILY));
        assert!(ATKINSON_NEXT_OFL.contains("SIL OPEN FONT LICENSE"));
        assert!(ATKINSON_MONO_OFL.contains("SIL OPEN FONT LICENSE"));
    }
}
