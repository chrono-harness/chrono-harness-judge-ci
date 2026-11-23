use chrono_judge_ci::Policy;
#[path = "../../judge-ci/src/impact_query.rs"]
#[allow(dead_code)]
mod impact_query;

use impact_query::Query;
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
