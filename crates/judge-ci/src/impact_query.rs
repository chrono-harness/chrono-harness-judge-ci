#[cfg(not(verus_keep_ghost))]
use crate::Policy;
#[cfg(not(verus_keep_ghost))]
use crate::observed::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg(not(verus_keep_ghost))]
pub(crate) struct Edge {
    pub from: String,
    pub kind: String,
    pub to: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Finding {
    code: String,
    level: String,
    message: String,
    delta_refs: Vec<String>,
    causes: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reachability {
    pub selected_tests: BTreeSet<String>,
    pub reached: BTreeSet<String>,
    pub edges: Vec<Value>,
    pub closure: Option<Value>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Obligations {
    pub tests: Vec<String>,
    pub retired_tests: Vec<String>,
    pub findings: Vec<Finding>,
    pub reached: BTreeSet<String>,
    pub changed_paths: Option<Vec<String>>,
    pub git_facts: Value,
}

#[cfg(not(verus_keep_ghost))]
impl Obligations {
    pub fn require_valid(&self, error: &str) -> Result<(), String> {
        if self.findings.iter().any(|f| f.level == "error") {
            return Err(format!(
                "{error}: {}",
                serde_json::to_string(&self.findings).map_err(|e| e.to_string())?
            ));
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub struct Query<'a> {
    root: &'a Path,
    timeout: u64,
    limit: usize,
    env: BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
impl<'a> Query<'a> {
    pub fn scoped(root: &'a Path, policy: &Policy) -> Result<Self, String> {
        Ok(Self {
            root,
            timeout: policy.operation_timeout_seconds,
            limit: policy.operation_output_limit_bytes,
            env: search_path()?,
        })
    }

    pub fn full(root: &'a Path, config: &Value) -> Result<Self, String> {
        Ok(Self {
            root,
            env: search_path()?,
            timeout: config["protocol"]["timeout_seconds"]
                .as_u64()
                .ok_or("missing full query timeout")?,
            limit: config["protocol"]["stdout_limit_bytes"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("missing full query output limit")?,
        })
    }

    /// Only endpoint scheduling observes the host environment afresh. Pass
    /// precisely the non-null inherited inputs from CI's bound Git reader.
    pub fn with_git_environment(mut self, observation: &Value) -> Result<Self, String> {
        if !observation.is_null() {
            let inherited: BTreeMap<String, Option<String>> =
                serde_json::from_value(observation["environment"]["inherited"].clone())
                    .map_err(|e| e.to_string())?;
            for (key, value) in inherited {
                match value {
                    Some(value) => {
                        self.env.insert(key, value);
                    }
                    None => {
                        self.env.remove(&key);
                    }
                }
            }
        }
        Ok(self)
    }

    pub fn read<T: DeserializeOwned>(
        &self,
        program: &str,
        operation: &str,
        request: &Value,
    ) -> Result<T, String> {
        self.read_with(program, operation, request, |command, input| {
            run_bound(self.root, command, input)
        })
    }

    pub fn read_with<T: DeserializeOwned>(
        &self,
        program: &str,
        operation: &str,
        request: &Value,
        invoke: impl FnOnce(CommandSpec, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
    ) -> Result<T, String> {
        let command = CommandSpec {
            program: program.into(),
            args: vec![operation.into()],
            env: self.env.clone(),
            timeout_seconds: self.timeout,
            output_limit_bytes: self.limit,
        };
        let input = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        let (exit, stdout, stderr) = invoke(command, &input)?;
        let stdout = query_stream(exit, stdout, stderr).map_err(|stderr| {
            String::from_utf8(stderr).map_or_else(
                |error| error.to_string(),
                |cause| format!("{program} {operation} exited {exit}: {}", cause.trim_end()),
            )
        })?;
        crate::codec::decode(&stdout)
    }
}

#[cfg(not(verus_keep_ghost))]
fn search_path() -> Result<BTreeMap<String, String>, String> {
    Ok(BTreeMap::from([(
        "PATH".into(),
        std::env::var("PATH").map_err(|e| e.to_string())?,
    )]))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn run_bound(
    root: &Path,
    mut command: CommandSpec,
    input: &[u8],
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    let path = crate::files::resolve_program(root, &command.program, None)?;
    let digest = crate::files::file_identity(&path)?.0;
    command.program = path.to_str().ok_or("non-UTF8 query executable")?.into();
    let runner = crate::observed::installed(root)?;
    let output = crate::observed::run(
        root,
        &runner,
        &command,
        input,
        &digest,
        std::time::Duration::from_secs(command.timeout_seconds),
    )?;
    if let Some(failure) = output.failure {
        return Err(failure);
    }
    Ok((output.exit_code, output.stdout_bytes, output.stderr_bytes))
}

use vstd::prelude::*;
verus! {
/// Only the successful child produces the stdout which may be decoded.
pub fn query_stream(exit: i32, stdout: Vec<u8>, stderr: Vec<u8>)
    -> (reply: Result<Vec<u8>, Vec<u8>>)
    ensures reply == if exit == 0 { Ok(stdout) } else { Err(stderr) },
{
    if exit == 0 { Ok(stdout) } else { Err(stderr) }
}
}
