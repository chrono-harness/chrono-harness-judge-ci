//! The published routes plan and projects receipt consumed by CI. The installed
//! route planner checks their identities and business invariants during replay.
use chrono_harness::{ProcessResult, observation::Tool, verified_verifier::CompiledArtifact};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// The installed registration and routes commands validate these public declarations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<chrono_harness::verified_verifier::OutputContract>,
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
