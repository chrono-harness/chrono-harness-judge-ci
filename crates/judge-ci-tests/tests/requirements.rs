use chrono_judge_ci::requirements::query;
use std::path::Path;

#[test]
fn scoped_query_rejects_an_outside_profile_before_reading_it() {
    let request =
        br#"{"operation":"schedule","profile":"../outside","candidate":"abc","initial":true}"#;
    assert!(
        query(Path::new("."), request)
            .unwrap_err()
            .contains("invalid relative path")
    );
}

#[test]
fn full_schedule_requires_a_base_and_rejects_scoped_fields() {
    for request in [
        br#"{"operation":"full-schedule","profile":"p","candidate":"c"}"#.as_slice(),
        br#"{"operation":"full-schedule","profile":"p","candidate":"c","base":"b","initial":true}"#,
    ] {
        let error = query(Path::new("."), request).unwrap_err();
        assert!(
            error.contains("missing field") || error.contains("unknown field"),
            "{error}"
        );
    }
}

#[test]
fn collection_only_accepts_a_manifest_under_the_declared_state_root() {
    let request = br#"{"operation":"collect","profile":"profile.json","manifest":"README.md","candidate":"x","initial":true}"#;
    assert!(
        query(Path::new("."), request)
            .unwrap_err()
            .contains("must reside")
    );
}

#[allow(dead_code)]
#[path = "../../judge-ci/src/scheduling_git.rs"]
mod scheduling_git;

#[test]
fn scheduling_reads_fixed_snapshots_and_preserves_the_first_git_error() {
    use serde_json::{Value, json};
    let response = |value: Value| format!("{}\n", json!({"Ok":value}));
    let before = response(json!({"effective_path":"before","values":{"before":{"v":1}}}));
    let after = response(json!({"effective_path":"after","values":{"after":{"v":2}}}));
    let setup = [
        response(json!({"bound":true})),
        response(Value::Null),
        response(json!("tree-a")),
        response(json!("tree-b")),
    ]
    .concat();
    let observation = json!({"processes":[{"argv":["git","show"]}]});
    let frames = [
        setup.clone(),
        before.clone(),
        after,
        response(Value::Null),
        response(observation.clone()),
    ]
    .concat();
    let mut sent = Vec::new();
    let pair = scheduling_git::collect(&mut sent, frames.as_bytes(), "policy", "a", "b").unwrap();
    assert_eq!(pair.previous.values["before"]["v"], 1);
    assert_eq!(pair.candidate.effective_path, "after");
    assert_eq!(pair.candidate.values["after"]["v"], 2);
    assert_eq!(pair.observation, observation);
    let operations = String::from_utf8(sent)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        json!(operations),
        json!([
            {"mode":"config","config":"policy","base":"a","candidate":"b"},
            {"op":"verify-config","candidate":"b"},{"op":"verify-tree","endpoint":"base"},
            {"op":"verify-tree","endpoint":"candidate"},{"op":"registry-snapshot","endpoint":"base"},
            {"op":"registry-snapshot","endpoint":"candidate"},{"op":"verify-config","candidate":"b"},{"op":"observation"}
        ])
    );
    let mut sent = Vec::new();
    let refused = format!("{setup}{before}{{\"Err\":\"missing registry\"}}\n");
    let error = scheduling_git::collect(&mut sent, refused.as_bytes(), "policy", "a", "b")
        .err()
        .unwrap();
    assert_eq!(error, "missing registry");
    assert_eq!(String::from_utf8(sent).unwrap().lines().count(), 6);
    for bad in [
        frames.trim_end().to_owned(),
        format!("{frames}extra"),
        frames.replace(&response(observation), &response(Value::Null)),
    ] {
        assert!(scheduling_git::collect(Vec::new(), bad.as_bytes(), "policy", "a", "b").is_err());
    }
}

use chrono_judge_ci::originals;
#[path = "../../judge-ci/src/runtime_session/originals.rs"]
mod collector_reader;

#[test]
fn collector_sends_raw_original_only_after_size_admission() {
    use serde_json::{Value, json};
    let operation = json!({"operation":"restore-scoped-process","limit":3});
    let path = ".chrono-harness/state/out";
    let read = json!({"read_original":{"path":path,"sha256":"retained"}});
    let bytes = [0, 0xff, b'\n'];
    let input = format!("{read}\n{{\"send_original\":true}}\n{{\"Ok\":\"restored\"}}\n");
    let mut output = Vec::new();
    let mut supply = |original: &originals::Original| {
        assert_eq!(original.path, path);
        assert_eq!(original.sha256, "retained");
        Ok(bytes.to_vec())
    };
    let result: String =
        collector_reader::transfer(&mut input.as_bytes(), &mut output, &operation, &mut supply)
            .unwrap();
    assert_eq!(result, "restored");
    let header = format!("{operation}\n{{\"Ok\":3}}\n");
    assert_eq!(output, [header.as_bytes(), &bytes].concat());
    for (reply, message) in [
        (r#"{"Err":"over budget"}"#, "over budget"),
        (
            r#"{"Ok":"early success"}"#,
            "invalid original length acknowledgement",
        ),
        (
            r#"{"send_original":false}"#,
            "invalid original length acknowledgement",
        ),
        ("", "incomplete runner response"),
    ] {
        let input = format!(
            "{read}\n{reply}{}",
            if reply.is_empty() { "" } else { "\n" }
        );
        let mut output = Vec::new();
        let error = collector_reader::transfer::<Value>(
            &mut input.as_bytes(),
            &mut output,
            &operation,
            &mut supply,
        )
        .unwrap_err();
        assert!(error.contains(message), "{error}");
        assert_eq!(output, header.as_bytes());
    }
    let input = format!("{read}\n{{\"Err\":\"reader error\"}}\n");
    let mut output = Vec::new();
    let error = collector_reader::transfer::<Value>(
        &mut input.as_bytes(),
        &mut output,
        &operation,
        &mut |_| Err("reader error".into()),
    )
    .unwrap_err();
    assert_eq!(error, "reader error");
    assert_eq!(
        output,
        format!("{operation}\n{{\"Err\":\"reader error\"}}\n").as_bytes()
    );
}
