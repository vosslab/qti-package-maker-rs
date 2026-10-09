//! Shared embedded-font rendering document.

use base64::{Engine, engine::general_purpose::STANDARD};

const NEXT_FONT: &[u8] =
    include_bytes!("../../qti-raster/fonts/atkinson_hyperlegible_next_variable.ttf");
const MONO_FONT: &[u8] =
    include_bytes!("../../qti-raster/fonts/atkinson_hyperlegible_mono_variable.ttf");

/// Wraps prepared table HTML in the shared static rendering document.
pub fn static_document(html: &str) -> String {
    let next = STANDARD.encode(NEXT_FONT);
    let mono = STANDARD.encode(MONO_FONT);
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'none'; connect-src 'none'; img-src data:; font-src data:; style-src 'unsafe-inline'; media-src data:; object-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'">
<style>
@font-face {{ font-family: 'Atkinson Hyperlegible Next'; font-style: normal; font-weight: 200 800; src: url(data:font/ttf;base64,{next}) format('truetype'); }}
@font-face {{ font-family: 'Atkinson Hyperlegible Mono'; font-style: normal; font-weight: 200 800; src: url(data:font/ttf;base64,{mono}) format('truetype'); }}
html {{ margin: 0; background: white; }}
body {{ margin: 16px; font-family: 'Atkinson Hyperlegible Next', sans-serif; }}
#qti-render-root {{ display: flow-root; }}
</style></head><body><main id="qti-render-root">{html}</main></body></html>"#,
    )
}
