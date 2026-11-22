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
