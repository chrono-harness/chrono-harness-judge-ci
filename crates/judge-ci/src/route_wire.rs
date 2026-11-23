//! The published routes plan and projects receipt consumed by CI. The installed
//! route planner checks their identities and business invariants during replay.
use chrono_harness::{ProcessResult, observation::Tool, verified_verifier::CompiledArtifact};
use chrono_judge_registration::execution::{Method, Plan, Scheduling};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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
