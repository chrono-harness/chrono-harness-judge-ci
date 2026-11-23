use chrono_harness::CommandSpec;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn check(
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
    if exit != 0 {
        let message = std::str::from_utf8(&stderr).map_err(|error| error.to_string())?;
        return Err(if message.is_empty() {
            format!("chrono-judge-projects check-pairs-v1 exited with {exit}")
        } else {
            message.strip_suffix('\n').unwrap_or(message).into()
        });
    }
    if !stdout.is_empty() {
        return Err("unexpected check-pairs-v1 response".into());
    }
    Ok(())
}
