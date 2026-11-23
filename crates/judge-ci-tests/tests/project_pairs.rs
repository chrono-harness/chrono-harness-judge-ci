use chrono_judge_ci::project_pairs;

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn checks_the_candidate_registry_and_affected_owners_with_the_installed_judge() {
    let registry = json!({"schema_version":2,"scripts":[]});
    let owners = BTreeSet::from(["script:example".into()]);
    for (reply, expected) in [
        (Ok((0, vec![], vec![])), Ok(())),
        (
            Ok((2, vec![], b"E_TEST_LANGUAGE: wrong pair\n".to_vec())),
            Err("E_TEST_LANGUAGE: wrong pair".to_owned()),
        ),
        (
            Ok((2, vec![], vec![])),
            Err("chrono-judge-projects check-pairs-v1 exited with 2".into()),
        ),
        (
            Ok((0, vec![b'?'], vec![])),
            Err("unexpected check-pairs-v1 response".into()),
        ),
        (Err("process timeout".into()), Err("process timeout".into())),
    ] {
        let result = project_pairs::check(
            &registry,
            &owners,
            None,
            "/tools".into(),
            19,
            4096,
            |command, input| {
                assert_eq!(command.program, "chrono-judge-projects");
                assert_eq!(command.args, ["check-pairs-v1"]);
                assert_eq!(
                    command.env,
                    BTreeMap::from([("PATH".into(), "/tools".into())])
                );
                assert_eq!(
                    (command.timeout_seconds, command.output_limit_bytes),
                    (19, 4096)
                );
                assert_eq!(
                    serde_json::from_slice::<Value>(input).unwrap(),
                    json!({"registry":registry,"owners":["script:example"]})
                );
                reply
            },
        );
        assert_eq!(result, expected);
    }
}

#[test]
fn uses_the_host_program_and_rejects_an_invalid_declaration_before_launch() {
    let path = ".chrono-harness/bin/chrono-judge-projects";
    let query = |installed: &Value| {
        project_pairs::check(
            &json!({}),
            &BTreeSet::new(),
            Some(installed),
            "/usr/bin:/bin".into(),
            19,
            4096,
            |command, _| {
                assert_eq!(command.program, path);
                Ok((0, vec![], vec![]))
            },
        )
    };
    assert_eq!(query(&json!({"program": path})), Ok(()));
    for invalid in [json!({}), json!({"program":""}), json!({"program":42})] {
        assert_eq!(
            query(&invalid).unwrap_err(),
            "invalid registered chrono-judge-projects program"
        );
    }
}
