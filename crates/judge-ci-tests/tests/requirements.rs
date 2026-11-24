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
