//! A new diagnostics process needs the metadata of each call site it actually receives.
//! A project replay may use cached metadata without another register_callsite callback.
#[cfg(not(verus_keep_ghost))]
use std::{collections::HashSet, sync::Mutex};
#[cfg(not(verus_keep_ghost))]
use tracing::{
    Event, Metadata, Subscriber,
    span::{Attributes, Id, Record},
    subscriber::Interest,
};
#[cfg(not(verus_keep_ghost))]
use tracing_tunnel::{TracingEvent, TracingEventSender};

#[cfg(not(verus_keep_ghost))]
pub(super) struct Source<F> {
    sender: TracingEventSender<F>,
    metadata: Mutex<HashSet<usize>>,
}

#[cfg(not(verus_keep_ghost))]
impl<F: Fn(TracingEvent) + 'static> Source<F> {
    pub(super) fn new(send: F) -> Self {
        Self {
            sender: TracingEventSender::new(send),
            metadata: Mutex::new(HashSet::new()),
        }
    }

    fn describe(&self, metadata: &'static Metadata<'static>) {
        // Keep metadata and its first use ordered even for concurrent producers.
        let mut seen = self.metadata.lock().unwrap();
        publish_metadata(seen.insert(metadata as *const _ as usize), || {
            self.sender.register_callsite(metadata);
        });
    }
}

#[cfg(not(verus_keep_ghost))]
impl<F: Fn(TracingEvent) + 'static> Subscriber for Source<F> {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        self.describe(attrs.metadata());
        self.sender.new_span(attrs)
    }
    fn record(&self, span: &Id, values: &Record<'_>) {
        self.sender.record(span, values);
    }
    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.sender.record_follows_from(span, follows);
    }
    fn event(&self, event: &Event<'_>) {
        self.describe(event.metadata());
        self.sender.event(event);
    }
    fn enter(&self, span: &Id) {
        self.sender.enter(span);
    }
    fn exit(&self, span: &Id) {
        self.sender.exit(span);
    }
    fn clone_span(&self, span: &Id) -> Id {
        self.sender.clone_span(span)
    }
    fn try_close(&self, span: Id) -> bool {
        self.sender.try_close(span)
    }
}

use vstd::prelude::*;
verus! {
/// FnOnce publishes at most once. The session's set insertion decides whether it is called.
pub fn publish_metadata<F: FnOnce()>(fresh: bool, publish: F) -> (sent: bool)
    requires fresh ==> publish.requires(()),
    ensures sent == fresh, sent ==> publish.ensures((),()),
{
    if fresh { publish(); }
    fresh
}
}
