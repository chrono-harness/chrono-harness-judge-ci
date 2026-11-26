//! Boundary proofs exercise the same executable preparation predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verified_consumption.rs"]
mod policy;
use vstd::prelude::*;

// Cargo builds this same release obligation as an executable for the publishing workflow.
mod release_observation {
    #[cfg(not(verus_keep_ghost))]
    use vstd::prelude::verus;
    #[cfg(verus_keep_ghost)]
    use vstd::prelude::*;

verus! {
pub fn query_diagnostic() -> (value: &'static str)
    ensures value@ == "E_REQUIREMENTS: invalid relative path"@,
{ "E_REQUIREMENTS: invalid relative path" }

pub fn judge_diagnostic() -> (value: &'static str)
    ensures value@ == "E_REQUEST:"@,
{ "E_REQUEST:" }

pub fn accepts(query_exit: i32, query_match: bool, judge_exit: i32, judge_match: bool) -> (accepted: bool)
    ensures accepted == (query_exit == 2 && query_match && judge_exit == 2 && judge_match),
{
    query_exit == 2 && query_match && judge_exit == 2 && judge_match
}
}

#[cfg(not(verus_keep_ghost))]
fn grep(mode: &str, pattern: &str, path: &str) -> Result<bool, String> {
    // grep interprets its documented matching mode and reports the original matching lines.
    std::process::Command::new("grep")
        .args([mode, pattern, path])
        .status()
        .map(|result| result.success())
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn evaluate_observations() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [query_exit, query_error, judge_exit, judge_error] = arguments.as_slice() else {
        return Err(
            "usage: release_observation QUERY_EXIT QUERY_ERROR JUDGE_EXIT JUDGE_ERROR".into(),
        );
    };
    let query_exit = query_exit.parse::<i32>().map_err(|e| e.to_string())?;
    let judge_exit = judge_exit.parse::<i32>().map_err(|e| e.to_string())?;
    let query_match = grep("-F", query_diagnostic(), query_error)?;
    let judge_match = grep("-F", judge_diagnostic(), judge_error)?;
    let admitted = accepts(query_exit, query_match, judge_exit, judge_match);
    if admitted {
        Ok(())
    } else {
        Err(format!("unexpected release observation for {arguments:?}"))
    }
}
}

#[cfg(all(not(verus_keep_ghost), not(test)))]
fn main() -> Result<(), String> {
    release_observation::evaluate_observations()
}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_accepts_the_expected_observations_only() {
    let accepted = release_observation::accepts(2, true, 2, true);
    assert(accepted);
    let refused_1 = release_observation::accepts(0, true, 2, true);
    assert(!refused_1);
    let refused_2 = release_observation::accepts(2, false, 2, true);
    assert(!refused_2);
    let refused_3 = release_observation::accepts(2, true, 0, true);
    assert(!refused_3);
    let refused_4 = release_observation::accepts(2, true, 2, false);
    assert(!refused_4);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn preparation_requires_exact_command_and_prerequisite() {
    proof {
        reveal_strlit("python"); reveal_strlit("bootstrap"); reveal_strlit(".");
        reveal_strlit("profile"); reveal_strlit("projects"); reveal_strlit("verify");
        reveal_strlit("chrono-bootstrap/v1"); reveal_strlit("other"); reveal_strlit("extra");
        assert("verify"@.len() != "other"@.len());
        assert("profile"@.len() != "other"@.len());
        assert("chrono-bootstrap/v1"@.len() != "other"@.len());
        assert("."@.len() != "other"@.len());
        assert("python"@.len() != "other"@.len());
        assert("projects"@.len() != "other"@.len());
    }
    let command = vec!["python", "bootstrap", ".", "profile"];
    let prerequisites = vec!["verify"];
    let accepted = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(accepted);
    let missing: Vec<&str> = Vec::new();
    let absent = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &missing, "verify", "python", "python", "projects", "projects");
    assert(!absent);
    let duplicate = vec!["verify", "verify"];
    let duplicated = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &duplicate, "verify", "python", "python", "projects", "projects");
    assert(!duplicated);
    let wrong = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "other", "python", "python", "projects", "projects");
    assert(!wrong);
    let changed = policy::matches_preparation(&command, "python", "bootstrap", "other",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!changed);
    let schema = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "other", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!schema);
    let root = vec!["python", "bootstrap", "other", "profile"];
    let wrong_root = policy::matches_preparation(&root, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!wrong_root);
    let extra = vec!["python", "bootstrap", ".", "profile", "extra"];
    let extra_arg = policy::matches_preparation(&extra, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!extra_arg);
    let executor = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "other", "python", "projects", "projects");
    assert(!executor);
    let registry = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "other", "projects");
    assert(!registry);
}

}
