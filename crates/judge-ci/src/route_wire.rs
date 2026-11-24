//! The published routes plan and projects receipt consumed by CI. The installed
//! route planner checks their identities and business invariants during replay.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputContract {
    version: String,
    commit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompiledArtifact {
    path: Vec<u8>,
    sha256: Vec<u8>,
    length: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisteredAction {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    pub output_contract: Option<OutputContract>,
}

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
    timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output_limit_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    launch_original: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ErrorSchema {
    #[serde(rename = "chrono-error/v1")]
    V1,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorRecord {
    schema: ErrorSchema,
    error: ErrorNode,
}
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelatedError {
    role: String,
    error: ErrorNode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub program: String,
    pub path: PathBuf,
    pub basename: String,
    pub sha256: String,
    pub version: ProcessResult,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    pub operations: Vec<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operation_bounds: BTreeMap<String, Bounds>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bounds {
    timeout_seconds: u64,
    output_limit_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scheduling {
    max_running: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    priority: Vec<String>,
    resources: Vec<String>,
    claims: BTreeMap<String, Claims>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    resources: Vec<String>,
    outputs: Vec<String>,
}

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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    pub method: Method,
    predecessors: BTreeSet<String>,
    timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub plan: String,
    pub operation: String,
    method: String,
    pub process: ProcessResult,
    pub compiler_dependency: Option<Result<Vec<u8>, String>>,
    pub compiler_artifact: Option<Result<CompiledArtifact, String>>,
    digest: String,
}
