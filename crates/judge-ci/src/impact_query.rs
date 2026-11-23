use crate::Policy;
use chrono_harness::{CommandSpec, wire::Finding};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct Edge {
    pub from: String,
    pub kind: String,
    pub to: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reachability {
    pub selected_tests: BTreeSet<String>,
    pub reached: BTreeSet<String>,
    pub edges: Vec<Value>,
    pub closure: Option<Value>,
}

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

pub struct Query<'a> {
    root: &'a Path,
    timeout: u64,
    limit: usize,
    env: BTreeMap<String, String>,
}

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
        if exit != 0 {
            let cause = String::from_utf8(stderr).map_err(|e| e.to_string())?;
            return Err(format!(
                "{program} {operation} exited {}: {}",
                exit,
                cause.trim_end()
            ));
        }
        chrono_harness::decode(&stdout)
    }
}

fn search_path() -> Result<BTreeMap<String, String>, String> {
    Ok(BTreeMap::from([(
        "PATH".into(),
        std::env::var("PATH").map_err(|e| e.to_string())?,
    )]))
}

pub(crate) fn run_bound(
    root: &Path,
    mut command: CommandSpec,
    input: &[u8],
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    let path = chrono_harness::resolve_program(root, &command.program, None)?;
    let digest = chrono_harness::file_identity(&path)?.0;
    command.program = path.to_str().ok_or("non-UTF8 query executable")?.into();
    let output = chrono_harness::run_process_observed(root, &command, input, &digest)?;
    if let Some(failure) = output.failure {
        return Err(failure);
    }
    Ok((output.exit_code, output.stdout_bytes, output.stderr_bytes))
}
