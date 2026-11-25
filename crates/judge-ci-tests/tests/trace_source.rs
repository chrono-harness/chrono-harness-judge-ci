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

#[test]
fn project_diagnostics_forward_only_original_coherent_wake_notices() {
    let base = json!({"schema":"chrono-maintenance-request/v1","reason":"retention-complete","accepted":true,"status":"requested",
        "round_completion":"not-established","disposal":"not-attempted","responsibility":"existing-owner-state",
        "activation_root":"/checkout","socket":"/checkout/.chrono-harness/state/s","policy_sha256":"a".repeat(64)});
    let line = |value: &serde_json::Value| format!("W_MAINTENANCE_ACTIVATION: {value}\n");
    let mut failure = base.clone();
    failure["status"] = json!("wake-failed");
    failure["accepted"] = json!(false);
    failure["error"] = json!("path must be shorter than SUN_LEN");
    let good = format!("{}{}", line(&base), line(&failure));
    let mut malformed = base.clone();
    malformed["accepted"] = json!(false);
    let mut unknown = base.clone();
    unknown["extra"] = json!("diagnostic");
    let mut weak = base.clone();
    weak["policy_sha256"] = json!("missing");
    let bad = format!(
        "{}{}{}other diagnostic\nW_MAINTENANCE_ACTIVATION: broken JSON\n",
        line(&malformed),
        line(&unknown),
        line(&weak)
    );
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let input = format!("{good}{bad}chrono-trace-v1 broken\n");
    let remaining =
        chrono_judge_ci::receive_project_diagnostics(input.as_bytes(), &mut output, &mut errors)
            .unwrap();
    assert_eq!(output, good.as_bytes());
    assert_eq!(remaining, bad.as_bytes());
    assert_eq!(errors.len(), 1);
}
