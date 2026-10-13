//! Run-scoped cache for deterministic HTML-to-image renderer inputs.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// A stable key for one renderer family and its exact prepared input.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct RenderKey {
    family: RenderFamily,
    digest: [u8; 32],
}

/// The supported renderer families.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum RenderFamily {
    Canvas,
    Table,
}

/// How one fragment interacted with a run-scoped render cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheOutcome {
    /// A completed PNG was already cached.
    Hit,
    /// Another worker completed the matching render while this worker waited.
    Wait,
    /// This worker claimed an absent key and invoked the renderer.
    Miss,
}

/// Cumulative durations reported by a renderer for one actual render attempt.
///
/// These values are local work durations, so a parallel conversion sums them as work and never
/// presents that sum as elapsed wall time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderMetrics {
    /// Time spent calculating layout.
    pub layout: Duration,
    /// Time spent painting pixels before PNG encoding.
    pub paint: Duration,
    /// Time spent encoding the painted image as PNG.
    pub encode: Duration,
}

impl std::ops::AddAssign for RenderMetrics {
    fn add_assign(&mut self, other: Self) {
        self.layout += other.layout;
        self.paint += other.paint;
        self.encode += other.encode;
    }
}

/// Renderer output with timings for the render attempt that produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedPng {
    /// Complete PNG payload.
    pub bytes: Vec<u8>,
    /// Logical CSS width, independent of device pixel density.
    pub width: f64,
    /// Logical CSS height, independent of device pixel density.
    pub height: f64,
    /// Local renderer-stage durations.
    pub metrics: RenderMetrics,
}

impl RenderKey {
    /// Creates a key from a renderer family, its stable configuration, and exact input.
    pub(crate) fn new(
        family: RenderFamily,
        renderer_discriminator: impl AsRef<[u8]>,
        input: impl AsRef<[u8]>,
    ) -> Self {
        let mut hasher = Sha256::new();
        let discriminator = renderer_discriminator.as_ref();
        hasher.update((discriminator.len() as u64).to_be_bytes());
        hasher.update(discriminator);
        hasher.update(input.as_ref());
        Self {
            family,
            digest: hasher.finalize().into(),
        }
    }
}

/// Reuses successful PNG renderings for the duration of one conversion run.
///
/// Failed renders are intentionally not cached: a caller may correct a renderer failure before a
/// later conversion attempt, while successful raster bytes are immutable and safe to share.
#[derive(Debug, Default)]
pub struct RenderCache {
    state: Mutex<HashMap<RenderKey, CacheEntry>>,
    changed: Condvar,
}

#[derive(Debug)]
enum CacheEntry {
    Rendering,
    Ready(RenderedPng),
}

impl RenderCache {
    /// Creates an empty run-scoped cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns cached PNG bytes or renders once while concurrent callers wait for that result.
    pub(crate) fn get_or_render<E>(
        &self,
        key: RenderKey,
        render: impl FnOnce() -> Result<RenderedPng, E>,
    ) -> (CacheOutcome, Result<RenderedPng, E>) {
        let mut state = self
            .state
            .lock()
            .expect("render cache mutex is not poisoned");
        let mut waited = false;
        loop {
            match state.get(&key) {
                Some(CacheEntry::Ready(png)) => {
                    return (
                        if waited {
                            CacheOutcome::Wait
                        } else {
                            CacheOutcome::Hit
                        },
                        Ok(RenderedPng {
                            bytes: png.bytes.clone(),
                            width: png.width,
                            height: png.height,
                            metrics: RenderMetrics::default(),
                        }),
                    );
                }
                Some(CacheEntry::Rendering) => {
                    waited = true;
                    state = self
                        .changed
                        .wait(state)
                        .expect("render cache mutex is not poisoned");
                }
                None => {
                    state.insert(key.clone(), CacheEntry::Rendering);
                    break;
                }
            }
        }
        drop(state);
        let rendered = render();
        let mut state = self
            .state
            .lock()
            .expect("render cache mutex is not poisoned");
        match &rendered {
            Ok(png) => {
                state.insert(key, CacheEntry::Ready(png.clone()));
            }
            Err(_) => {
                state.remove(&key);
            }
        }
        self.changed.notify_all();
        (CacheOutcome::Miss, rendered)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    use super::{RenderCache, RenderFamily, RenderKey, RenderedPng};

    #[test]
    fn identical_concurrent_inputs_render_once() {
        let cache = Arc::new(RenderCache::new());
        let start = Arc::new(Barrier::new(2));
        let calls = Arc::new(AtomicUsize::new(0));
        let workers = (0..2)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let start = Arc::clone(&start);
                let calls = Arc::clone(&calls);
                thread::spawn(move || {
                    start.wait();
                    cache
                        .get_or_render(RenderKey::new(RenderFamily::Table, "test", "same"), || {
                            calls.fetch_add(1, Ordering::SeqCst);
                            thread::sleep(Duration::from_millis(30));
                            Ok::<_, ()>(RenderedPng {
                                bytes: vec![1, 2, 3],
                                width: 120.0,
                                height: 80.0,
                                metrics: Default::default(),
                            })
                        })
                        .1
                        .expect("renderer succeeds")
                })
            })
            .collect::<Vec<_>>();
        let rendered = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker completes"))
            .collect::<Vec<_>>();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            rendered
                .into_iter()
                .map(|png| png.bytes)
                .collect::<Vec<_>>(),
            [vec![1, 2, 3], vec![1, 2, 3]]
        );
    }

    #[test]
    fn distinct_inputs_can_render_in_parallel() {
        let cache = Arc::new(RenderCache::new());
        let started = Arc::new(Barrier::new(2));
        let workers = ["first", "second"].map(|input| {
            let cache = Arc::clone(&cache);
            let started = Arc::clone(&started);
            thread::spawn(move || {
                cache
                    .get_or_render(RenderKey::new(RenderFamily::Table, "test", input), || {
                        started.wait();
                        Ok::<_, ()>(RenderedPng {
                            bytes: input.as_bytes().to_vec(),
                            width: 120.0,
                            height: 80.0,
                            metrics: Default::default(),
                        })
                    })
                    .1
                    .expect("renderer succeeds")
            })
        });

        let outputs = workers.map(|worker| worker.join().expect("worker completes"));
        assert_eq!(
            outputs.map(|png| png.bytes),
            [b"first".to_vec(), b"second".to_vec()]
        );
    }
}
