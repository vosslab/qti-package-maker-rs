//! Chromium-backed table screenshots for the opt-in HTML-to-image lane.
//!
//! This is intentionally a small browser adapter. It renders the already-selected table
//! fragment in a private Chromium profile; selection, media preparation, and package ownership
//! remain in the surrounding conversion module.

use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use base64::{Engine, engine::general_purpose::STANDARD};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use tempfile::TempDir;
use thiserror::Error;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

const VIEWPORT_WIDTH: u32 = 1280;
const VIEWPORT_HEIGHT: u32 = 720;
const DEVICE_SCALE_FACTOR: f64 = 2.0;

const NEXT_FONT: &[u8] =
    include_bytes!("../../../qti-raster/fonts/atkinson_hyperlegible_next_variable.ttf");
const MONO_FONT: &[u8] =
    include_bytes!("../../../qti-raster/fonts/atkinson_hyperlegible_mono_variable.ttf");

/// A Chromium table-rendering failure with a direct recovery path when Chromium is unavailable.
#[derive(Debug, Error)]
pub(crate) enum ChromiumError {
    /// The Tokio runtime required to drive Chromium could not start.
    #[error("could not start the Chromium rendering runtime: {0}")]
    Runtime(#[source] std::io::Error),
    /// The configured executable path cannot be launched.
    #[error(
        "QTI_CHROMIUM points to `{path}`, but it is not a file; set QTI_CHROMIUM to a Chromium or Chrome executable"
    )]
    InvalidExecutable {
        /// The requested executable path.
        path: PathBuf,
    },
    /// The isolated browser profile could not be created.
    #[error("could not create Chromium's isolated temporary profile: {0}")]
    Profile(#[source] std::io::Error),
    /// Chromiumoxide could not configure a browser executable.
    #[error(
        "could not configure Chromium: {0}. Install Chromium/Chrome or set QTI_CHROMIUM to its executable"
    )]
    Configuration(String),
    /// Chromium failed during launch or CDP rendering.
    #[error(
        "Chromium could not render this table: {0}. Ensure Chromium can run headlessly, or set QTI_CHROMIUM to its executable"
    )]
    Browser(String),
    /// A poisoned mutex means an earlier renderer call panicked while owning browser state.
    #[error("Chromium renderer state is unavailable after an earlier rendering panic")]
    StatePoisoned,
    /// The prepared fragment unexpectedly contains no table to screenshot.
    #[error("prepared HTML-to-image fragment contains no table element")]
    MissingTable,
}

/// A lazily started, serialized Chromium page used only for table screenshots.
///
/// Keeping one page avoids launching a browser per fragment. The mutex is intentionally held for
/// a full render because the page's DOM is replaced for every input.
pub(crate) struct ChromiumRenderer {
    state: Mutex<Option<Session>>,
}

impl fmt::Debug for ChromiumRenderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChromiumRenderer")
            .field(
                "started",
                &self.state.lock().is_ok_and(|state| state.is_some()),
            )
            .finish_non_exhaustive()
    }
}

impl Default for ChromiumRenderer {
    fn default() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }
}

impl ChromiumRenderer {
    /// Screenshots the first table in a prepared fragment as a PNG.
    ///
    /// Browser startup is deliberately deferred to this method, so runs without a selected table
    /// never require Chromium.
    pub(crate) fn render(&self, html: &str) -> Result<Vec<u8>, ChromiumError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ChromiumError::StatePoisoned)?;
        if state.is_none() {
            *state = Some(Session::start()?);
        }
        let session = state.as_ref().expect("browser state was initialized above");
        session.runtime.block_on(session.browser.render(html))
    }
}

impl Drop for ChromiumRenderer {
    fn drop(&mut self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some(session) = state.take() else {
            return;
        };
        let Session {
            runtime,
            mut browser,
        } = session;
        runtime.block_on(async move {
            // Browser::close asks Chromium to end cleanly; Browser::drop still kills the child if
            // that request fails. This keeps the isolated profile owned until the child exits.
            let _ = browser.browser.close().await;
            let _ = browser.browser.wait().await;
            browser.handler.abort();
        });
    }
}

struct Session {
    runtime: Runtime,
    browser: BrowserState,
}

impl Session {
    fn start() -> Result<Self, ChromiumError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(ChromiumError::Runtime)?;
        let browser = runtime.block_on(BrowserState::start())?;
        Ok(Self { runtime, browser })
    }
}

struct BrowserState {
    browser: Browser,
    page: Page,
    handler: JoinHandle<()>,
    _profile: TempDir,
}

impl BrowserState {
    async fn start() -> Result<Self, ChromiumError> {
        let profile = tempfile::tempdir().map_err(ChromiumError::Profile)?;
        let mut config = BrowserConfig::builder()
            .user_data_dir(profile.path())
            .viewport(Viewport {
                width: VIEWPORT_WIDTH,
                height: VIEWPORT_HEIGHT,
                device_scale_factor: Some(DEVICE_SCALE_FACTOR),
                emulating_mobile: false,
                is_landscape: true,
                has_touch: false,
            });
        if let Some(executable) = configured_executable()? {
            config = config.chrome_executable(executable);
        }
        // Do not call `no_sandbox`: Chromium's sandbox remains enabled.
        let config = config.build().map_err(ChromiumError::Configuration)?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))?;
        let handler = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if event.is_err() {
                    break;
                }
            }
        });
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))?;
        page.set_content(static_document())
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))?;
        Ok(Self {
            browser,
            page,
            handler,
            _profile: profile,
        })
    }

    async fn render(&self, html: &str) -> Result<Vec<u8>, ChromiumError> {
        // ASVS 1.2.3: HTML travels as a CDP JSON value, never as interpolated JavaScript source.
        // ASVS 1.3.1/1.3.5: parse markup in an inert template, remove active document elements,
        // then rely on the static document CSP to prevent authored scripts and network loads.
        let set_html = CallFunctionOnParams::builder()
            .function_declaration(
                "(html) => {\
                    const template = document.createElement('template');\
                    template.innerHTML = html;\
                    template.content\
                        .querySelectorAll('script, iframe, frame, object, embed, base, meta, link')\
                        .forEach((element) => element.remove());\
                    document.getElementById('qti-render-root').replaceChildren(template.content);\
                }",
            )
            .argument(
                CallArgument::builder()
                    .value(serde_json::Value::String(html.to_owned()))
                    .build(),
            )
            .build()
            .expect("static DOM replacement call is valid");
        self.page
            .evaluate_function(set_html)
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))?;
        self.page
            .evaluate_function(
                "async () => {\
                    await document.fonts.ready;\
                    await Promise.all(Array.from(document.images, (image) => {\
                        if (image.complete) return Promise.resolve();\
                        return new Promise((resolve) => {\
                            image.addEventListener('load', resolve, { once: true });\
                            image.addEventListener('error', resolve, { once: true });\
                        });\
                    }));\
                }",
            )
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))?;
        let table = self
            .page
            .find_element("#qti-render-root table")
            .await
            .map_err(|error| match error {
                chromiumoxide::error::CdpError::NotFound => ChromiumError::MissingTable,
                other => ChromiumError::Browser(other.to_string()),
            })?;
        table
            .screenshot(CaptureScreenshotFormat::Png)
            .await
            .map_err(|error| ChromiumError::Browser(error.to_string()))
    }
}

fn configured_executable() -> Result<Option<PathBuf>, ChromiumError> {
    let Some(path) = env::var_os("QTI_CHROMIUM") else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if path.is_file() {
        Ok(Some(path))
    } else {
        Err(ChromiumError::InvalidExecutable { path })
    }
}

fn static_document() -> String {
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
</style></head><body><main id="qti-render-root"></main></body></html>"#,
    )
}
