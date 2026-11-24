//! Verify the runner response protocol consumed by CI.

#[test]
fn ci_rows_preserve_the_runner_protocol_in_both_directions() {
    use chrono_judge_ci::verified_collection::result_types::Status;
    for (wire, expected, operation_exit, judge_exit) in [
        ("passed", Status::Passed, Some(0), 0),
        ("failed", Status::Failed, Some(-7), 1),
        ("not-required", Status::NotRequired, None, 0),
        ("blocked", Status::Blocked, Some(-7), 1),
    ] {
        let raw = serde_json::json!({
            "protocol":"chrono-ci-judge/v2", "request_id":"unit-fixture", "status":wire,
            "results":[{"id":"build", "status":wire, "cause":"原始错误\0é", "exit_code":operation_exit}],
            "evidence":{"unit":"build"}
        });
        let mut report = chrono_judge_ci::unit_response(&raw, "unit-fixture", judge_exit).unwrap();
        assert!(chrono_judge_ci::unit_response(&raw, "another-unit", judge_exit).is_err());
        assert!(chrono_judge_ci::unit_response(&raw, "unit-fixture", 1 - judge_exit).is_err());
        let row = report.results.pop().unwrap();
        assert!(row.status == expected);
        assert_eq!(row.cause, "原始错误\0é");
        assert_eq!(row.exit_code, operation_exit);
        report.results.push(row);
        assert_eq!(serde_json::to_value(report).unwrap(), raw);
    }
}
