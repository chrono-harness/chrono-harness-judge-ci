//! The published routes plan and projects receipt consumed by CI. The installed
//! route planner checks their identities and business invariants during replay.
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputContract {
    version: String,
    commit: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompiledArtifact {
    path: Vec<u8>,
    sha256: Vec<u8>,
    length: u64,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisteredAction {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    pub output_contract: Option<OutputContract>,
}

#[cfg(not(verus_keep_ghost))]
/// Published process receipt. Preserve the producer's defaults during replay,
/// including completeness flags on historical inline records.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub environment_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ownership_fds: Option<String>,
    pub stdin_sha256: String,
    pub stdout_bytes: Vec<u8>,
    pub stderr_bytes: Vec<u8>,
    #[serde(default)]
    stdout_complete: bool,
    #[serde(default)]
    stderr_complete: bool,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failure_record: Option<ErrorRecord>,
    pub exit_code: i32,
    #[serde(default)]
    owned_work_joined: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit_signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub executable: PathBuf,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    launcher_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    child_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "crate::route_wire::timeout_wire::optional::deserialize")]
    timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output_limit_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    launch_original: Option<Value>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
enum ErrorSchema {
    #[serde(rename = "chrono-error/v1")]
    V1,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorRecord {
    schema: ErrorSchema,
    error: ErrorNode,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorNode {
    #[serde(rename = "type")]
    error_type: String,
    code: String,
    message: String,
    context: BTreeMap<String, Value>,
    source: Option<Box<ErrorNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    traceback: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    related: Vec<RelatedError>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelatedError {
    role: String,
    error: ErrorNode,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub program: String,
    pub path: PathBuf,
    pub basename: String,
    pub sha256: String,
    #[serde(serialize_with = "crate::route_wire::timeout_wire::observation")]
    pub version: ProcessResult,
}

#[cfg(not(verus_keep_ghost))]
/// The installed registration and routes commands validate these public declarations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<OutputContract>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    pub operations: Vec<String>,
    #[serde(with = "crate::route_wire::timeout_wire")]
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operation_bounds: BTreeMap<String, Bounds>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bounds {
    #[serde(with = "crate::route_wire::timeout_wire")]
    timeout_seconds: u64,
    output_limit_bytes: usize,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scheduling {
    max_running: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    priority: Vec<String>,
    resources: Vec<String>,
    claims: BTreeMap<String, Claims>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    resources: Vec<String>,
    outputs: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Execution {
    schema: String,
    pub identity: String,
    pub binding: Value,
    pub root: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
    pub tools: BTreeMap<String, Tool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<Scheduling>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    pub method: Method,
    predecessors: BTreeSet<String>,
    #[serde(with = "crate::route_wire::timeout_wire")]
    timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub plan: String,
    pub operation: String,
    method: String,
    #[serde(serialize_with = "crate::route_wire::timeout_wire::observation")]
    pub process: ProcessResult,
    pub compiler_dependency: Option<Result<Vec<u8>, String>>,
    pub compiler_artifact: Option<Result<CompiledArtifact, String>>,
    digest: String,
}

/// Public execution bounds use JSON numbers in the interoperable range and
/// canonical decimal strings above it so a v1 JCS transport preserves them.
#[cfg(not(verus_keep_ghost))]
pub(crate) mod timeout_wire {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(seconds: &u64, s: S) -> Result<S::Ok, S::Error> {
        if !super::decimal_timeout(*seconds) {
            s.serialize_u64(*seconds)
        } else {
            s.collect_str(seconds)
        }
    }

    pub fn observation<S: Serializer>(
        process: &crate::route_wire::ProcessResult,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::Serialize;
        if let Some(seconds) = process
            .timeout_seconds
            .filter(|n| super::decimal_timeout(*n))
        {
            let mut value = serde_json::to_value(process).map_err(serde::ser::Error::custom)?;
            value["timeout_seconds"] = serde_json::Value::String(seconds.to_string());
            value.serialize(s)
        } else {
            process.serialize(s)
        }
    }

    pub mod optional {
        use super::*;
        #[derive(Deserialize)]
        struct Wrapped(#[serde(deserialize_with = "super::deserialize")] u64);

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
            Ok(Option::<Wrapped>::deserialize(d)?.map(|wrapped| wrapped.0))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Timeout {
            Number(u64),
            Decimal(String),
        }
        match Timeout::deserialize(d)? {
            Timeout::Number(value) => Ok(value),
            Timeout::Decimal(text) => text
                .parse::<u64>()
                .ok()
                .filter(|n| super::decimal_timeout(*n) && n.to_string() == text)
                .ok_or_else(|| D::Error::custom("invalid exact timeout")),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Only JSON's interoperable integer range can use a number token on the v2 wire.
pub fn decimal_timeout(seconds: u64) -> (use_text: bool)
    ensures use_text == (seconds as int >= 9007199254740992),
{ seconds >= 9007199254740992u64 }
}
