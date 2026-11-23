//! Native integration with the registration and runner interfaces used by CI.
use chrono_judge_ci::verified_collection::validate_reference_syntax;

#[test]
fn reference_declarations_validate_actual_prefixes_and_explicit_bases() {
    use chrono_harness::verified_verifier::sources::continuity::ReferenceSyntax;
    let check = |prefix: &str, base| {
        let form = ReferenceSyntax {
            file: "x".into(),
            before: prefix.into(),
            after: prefix.into(),
            trailing: String::new(),
            suffix_before: String::new(),
            suffix_after: String::new(),
            base,
        };
        validate_reference_syntax(&vec![form])
    };
    assert!(check("\"", 0).is_ok());
    assert!(check("/", 2).is_ok());
    assert!(check("/", 0).is_err());
    assert!(check("", 0).is_err());
    assert!(check("\"", 3).is_err());
}

#[test]
fn ci_rows_preserve_the_runner_protocol_in_both_directions() {
    use chrono_judge_ci::verified_collection::result_types::{CheckResult, Status};
    for (wire, expected) in [
        ("passed", Status::Passed),
        ("failed", Status::Failed),
        ("not-required", Status::NotRequired),
        ("blocked", Status::Blocked),
    ] {
        let raw = serde_json::json!({
            "id":"build", "status":wire, "cause":"原始错误\0é", "exit_code":-7
        });
        let incoming: chrono_harness::CheckResult = serde_json::from_value(raw.clone()).unwrap();
        let row = CheckResult::from(incoming);
        assert!(row.status == expected);
        assert_eq!(row.cause, "原始错误\0é");
        assert_eq!(row.exit_code, Some(-7));
        let outgoing = chrono_harness::CheckResult::from(row);
        assert_eq!(serde_json::to_value(outgoing).unwrap(), raw);
    }
}
