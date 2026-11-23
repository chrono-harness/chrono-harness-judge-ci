#[path = "../../judge-ci/src/trace_source.rs"]
mod trace_source;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tracing_tunnel::{TracingEvent, TracingEventReceiver};

#[test]
fn successive_diagnostic_sessions_each_describe_replayed_project_metadata() {
    let site = json!({"new_call_site":{
        "id":7, "kind":"event", "name":"worker", "target":"projects", "level":"info", "fields":["result"]
    }});
    let event = json!({"new_event":{"metadata_id":7,"values":{"result":{"string":"done"}}}});
    for _ in 0..2 {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let stream = Arc::clone(&sent);
        let sender = trace_source::Source::new(move |event| stream.lock().unwrap().push(event));
        tracing::subscriber::with_default(sender, || {
            let mut project = receiver();
            for item in [&site, &event] {
                project
                    .try_receive(serde_json::from_value(item.clone()).unwrap())
                    .unwrap();
            }
        });
        let mut diagnostics = receiver();
        let frames = std::mem::take(&mut *sent.lock().unwrap());
        assert_eq!(frames.len(), 2);
        assert!(matches!(frames[0], TracingEvent::NewCallSite { .. }));
        assert!(matches!(frames[1], TracingEvent::NewEvent { .. }));
        for frame in frames {
            diagnostics.try_receive(frame).unwrap();
        }
    }
}

fn receiver() -> TracingEventReceiver {
    TracingEventReceiver::new(Default::default(), Default::default(), Default::default())
}
