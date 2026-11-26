//! Release acceptance for both rejected public requests and their original diagnostic fragments.
#![allow(unexpected_cfgs)]
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
fn main() -> Result<(), String> {
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
