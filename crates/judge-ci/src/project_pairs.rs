#[cfg(not(verus_keep_ghost))]
use crate::observed::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(verus_keep_ghost))]
pub fn check(
    registry: &Value,
    owners: &BTreeSet<String>,
    installed: Option<&Value>,
    path: String,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    invoke: impl FnOnce(&CommandSpec, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<(), String> {
    let program = match installed {
        Some(tool) => tool["program"]
            .as_str()
            .filter(|program| !program.is_empty())
            .ok_or("invalid registered chrono-judge-projects program")?,
        None => "chrono-judge-projects",
    };
    let command = CommandSpec {
        program: program.into(),
        args: vec!["check-pairs-v1".into()],
        env: BTreeMap::from([("PATH".into(), path)]),
        timeout_seconds,
        output_limit_bytes,
    };
    let input = serde_json::to_vec(&json!({"registry":registry,"owners":owners}))
        .map_err(|error| error.to_string())?;
    let (exit, stdout, stderr) = invoke(&command, &input)?;
    match pair_response(exit, stdout, stderr) {
        Ok(()) => Ok(()),
        Err(PairFailure::UnexpectedOutput) => Err("unexpected check-pairs-v1 response".into()),
        Err(PairFailure::Child(stderr)) => {
            let message = std::str::from_utf8(&stderr).map_err(|error| error.to_string())?;
            Err(if message.is_empty() {
                format!("chrono-judge-projects check-pairs-v1 exited with {exit}")
            } else {
                message.strip_suffix('\n').unwrap_or(message).into()
            })
        }
    }
}

#[cfg(verus_keep_ghost)]
use super::fixed_query::query_stream;
#[cfg(not(verus_keep_ghost))]
use crate::impact_query::query_stream;
use vstd::prelude::*;

verus! {
pub enum PairFailure { Child(Vec<u8>), UnexpectedOutput }

/// A successful pair check has no reply body. A failed child owns its original stderr.
pub fn pair_response(exit: i32, stdout: Vec<u8>, stderr: Vec<u8>) -> (reply: Result<(), PairFailure>)
    ensures reply == if exit != 0 { Err(PairFailure::Child(stderr)) }
        else if stdout.len() == 0 { Ok(()) } else { Err(PairFailure::UnexpectedOutput) },
{
    match query_stream(exit, stdout, stderr) {
        Err(bytes) => Err(PairFailure::Child(bytes)),
        Ok(bytes) => if bytes.is_empty() { Ok(()) } else { Err(PairFailure::UnexpectedOutput) },
    }
}
}
