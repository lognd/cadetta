//! Forward tracing events to a host-supplied sink (the Python bridge).
// frob:ticket 01M4SV2P2ZX7BMWKB98CJ14FY0

use std::fmt::{Debug, Write as _};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

use crate::init::InitError;

/// Receives every event as `(level, target, message)`; fields follow the message as `name=value`.
pub type Sink = Box<dyn Fn(&Level, &str, &str) + Send + Sync>;

/// Collects an event's `message` field first, then `name=value` pairs.
#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            let _ = write!(self.fields, "{}={value:?}", field.name());
        }
    }
}

impl MessageVisitor {
    /// The message with its structured fields appended.
    fn render(self) -> String {
        match (self.message.is_empty(), self.fields.is_empty()) {
            (_, true) => self.message,
            (true, false) => self.fields,
            (false, false) => format!("{} {}", self.message, self.fields),
        }
    }
}

/// A layer that hands each event to a [`Sink`], unfiltered (the host filters).
pub(crate) struct SinkLayer {
    sink: Sink,
}

impl SinkLayer {
    /// Wrap `sink` as a layer.
    pub(crate) fn new(sink: Sink) -> Self {
        Self { sink }
    }
}

impl<S: Subscriber> Layer<S> for SinkLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        (self.sink)(meta.level(), meta.target(), &visitor.render());
    }
}

/// Install a global subscriber that forwards every event to `sink`.
///
/// Unlike [`crate::init`] nothing is written to stderr and no level filter
/// applies: the host (Python `logging`) decides what to keep.
///
/// # Errors
///
/// Returns [`InitError::AlreadyInitialized`] if a global subscriber exists.
pub fn init_sink(sink: Sink) -> Result<(), InitError> {
    Registry::default()
        .with(SinkLayer::new(sink))
        .try_init()
        .map_err(InitError::AlreadyInitialized)?;
    tracing::debug!("tracing forwarded to a host sink");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    // frob:tests crates/cadetta-log/src/sink.rs::SinkLayer
    #[test]
    fn events_reach_the_sink_with_level_target_and_fields() {
        let seen: Arc<Mutex<Vec<(Level, String, String)>>> = Arc::default();
        let store = Arc::clone(&seen);
        let layer = SinkLayer::new(Box::new(move |level, target, message| {
            store
                .lock()
                .unwrap()
                .push((*level, target.to_owned(), message.to_owned()));
        }));
        let subscriber = Registry::default().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!(target: "demo::part", n = 3, "careful");
            tracing::trace!("fine");
        });
        let got = seen.lock().unwrap().clone();
        assert_eq!(
            got,
            vec![
                (
                    Level::WARN,
                    "demo::part".to_owned(),
                    "careful n=3".to_owned()
                ),
                (Level::TRACE, module_path!().to_owned(), "fine".to_owned()),
            ]
        );
    }
}
