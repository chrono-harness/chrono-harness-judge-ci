use chrono_judge_ci::impact_query::Query;
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn fixed_endpoint_query_forwards_only_bound_business_inputs_and_reports_the_child_result() {
    let config = json!({"protocol":{"timeout_seconds":13,"stdout_limit_bytes":4096}});
    let observed = json!({"environment":{"inherited":{
        "PATH":"/selected","EXAMPLE":"","MISSING":null,"ACQUISITION_TOKEN":null}}});
    let query = Query::full(Path::new("."), &config)
        .unwrap()
        .with_git_environment(&observed)
        .unwrap();
    let request = json!({"config":"policy","base":"before","candidate":"after","scheduling":true});
    let run = |reply| {
        query.read_with::<Value>("filemap", "endpoint-tests", &request, |spec, stdin| {
            assert_eq!(spec.program, "filemap");
            assert_eq!(spec.args, ["endpoint-tests"]);
            assert_eq!(spec.timeout_seconds, 13);
            assert_eq!(spec.output_limit_bytes, 4096);
            assert_eq!(
                serde_json::to_value(spec.env).unwrap(),
                json!({"PATH":"/selected","EXAMPLE":""})
            );
            assert_eq!(serde_json::from_slice::<Value>(stdin).unwrap(), request);
            reply
        })
    };
    assert_eq!(run(Ok((0, b"[]".to_vec(), vec![]))).unwrap(), json!([]));
    assert!(run(Ok((0, b"incomplete".to_vec(), vec![]))).is_err());
    assert_eq!(
        run(Ok((2, vec![], b"missing original\n".to_vec()))).unwrap_err(),
        "filemap endpoint-tests exited 2: missing original"
    );
    assert_eq!(run(Err("timeout".into())).unwrap_err(), "timeout");
}

#[path = "../../judge-ci/src/source_observations.rs"]
mod source_observations;

#[test]
fn source_rejection_retains_git_processes_between_the_two_caller_reads() {
    let binding = json!({"config_sha256":"fixed"});
    let reply = json!({"previous":null,"candidate":"commit","error":"原错误\0é",
        "git_facts":{"config_sha256":"fixed","processes":["routes"]}});
    let mut evidence = source_observations::GitEvidence::default();
    let error = evidence
        .admit(
            None,
            "commit",
            &binding,
            Some(1),
            serde_json::from_value(reply.clone()).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error, "原错误\0é");
    let facts = json!({"config_sha256":"fixed","processes":["before","after"]});
    assert_eq!(
        evidence.finish(facts.clone())["processes"],
        json!(["before", "routes", "after"])
    );
    for (candidate, binding, expected) in [
        ("other", binding, "E_PLAN_BINDING"),
        ("commit", json!({"config_sha256":"other"}), "E_GIT_FACTS"),
    ] {
        let mut evidence = source_observations::GitEvidence::default();
        let error = evidence
            .admit(
                None,
                candidate,
                &binding,
                Some(1),
                serde_json::from_value(reply.clone()).unwrap(),
            )
            .unwrap_err();
        assert!(error.starts_with(expected));
        assert_eq!(evidence.finish(facts.clone()), facts);
    }
}

#[test]
fn separate_queries_at_the_same_position_keep_their_original_process_order() {
    let facts = json!({"config_sha256":"fixed","processes":["before"]});
    let (binding, offset) = source_observations::binding(&facts).unwrap();
    let mut evidence = source_observations::GitEvidence::default();
    for program in ["registration", "routes"] {
        evidence
            .record(
                &binding,
                offset,
                json!({"config_sha256":"fixed","processes":[program]}),
            )
            .unwrap();
    }
    assert!(
        evidence
            .record(&binding, offset, json!({"config_sha256":"fixed"}))
            .is_err()
    );
    assert_eq!(
        evidence.finish(facts)["processes"],
        json!(["before", "registration", "routes"])
    );
}
