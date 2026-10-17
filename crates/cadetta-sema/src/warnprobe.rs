//! Test-only probe counting warn-or-worse tracing events (no dependency on the log crate).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{Event, Level, Metadata, Subscriber, span};

struct Counter(Arc<AtomicUsize>);

impl Subscriber for Counter {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, e: &Event<'_>) {
        if *e.metadata().level() <= Level::WARN {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

/// Runs `f` and returns how many warn-or-worse events it emitted.
pub(crate) fn warn_count<F: FnOnce()>(f: F) -> usize {
    let n = Arc::new(AtomicUsize::new(0));
    tracing::subscriber::with_default(Counter(Arc::clone(&n)), f);
    n.load(Ordering::SeqCst)
}
