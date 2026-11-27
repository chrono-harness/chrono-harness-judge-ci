//! The scoped CI request and profile accepted by this command.
#[cfg(not(verus_keep_ghost))]
use crate::observed::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::path::PathBuf;

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default)]
    pub observations: Value,
    pub protocol: String,
    pub request_id: String,
    pub host_root: PathBuf,
    pub config_path: String,
    pub config_sha256: String,
    pub base: Option<String>,
    pub candidate: String,
    pub initial: bool,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig {
    pub schema: String,
    pub judge: CommandSpec,
    pub policy: Value,
    pub report_path: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub tests: Vec<String>,
    pub report_path: String,
}

#[cfg(verus_keep_ghost)]
use super::{unit_protocol::selected_protocol, verified_text::equal_text};
#[cfg(not(verus_keep_ghost))]
use crate::{units::selected_protocol, verified_collection::verified_text::equal_text};

use vstd::prelude::*;
verus! {
pub const PROTOCOL: &'static str = "chrono-ci-judge/v1";
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize, serde::Serialize))]
#[derive(Clone,PartialEq,Eq)]
#[cfg_attr(not(verus_keep_ghost), serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields))]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}

pub fn request_header(scope: &Option<Scope>, protocol: &str, id: &str) -> (valid: bool)
    ensures valid == (id@ != ""@ && protocol@ == if scope.is_some() { "chrono-ci-judge/v2"@ } else { "chrono-ci-judge/v1"@ }),
{
    equal_text(protocol,selected_protocol(scope,PROTOCOL)) && !equal_text(id,"")
}
}
