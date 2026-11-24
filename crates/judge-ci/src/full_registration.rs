//! Collect only the original reports requested by registration's historical conversion.
use crate::scheduling_git::Reader;
use crate::{
    FullPlanning,
    impact_query::Query,
    source_observations::{self, GitEvidence},
};
use crate::originals::FullManifest;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// Coordinates consumed locally after the installed runner verifies the complete original.
#[derive(Deserialize)]
pub(super) struct Coordinates {
    pub request_id: String,
    pub config_path: String,
    pub scope: Value,
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub runner: Executable,
}
#[derive(Deserialize)]
pub(super) struct Endpoint {
    pub commit: String,
    pub root: PathBuf,
}
#[derive(Deserialize)]
pub(super) struct Executable {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    request_id: String,
    commits: [String; 2],
    #[serde(default)]
    git_facts: Value,
    outcome: Outcome,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Ready(FullPlanning),
    Needed(Vec<String>),
    Error(String),
}

pub(super) fn collection(
    request: &Coordinates,
    original: &Value,
    reader: &Reader,
    query: &Query<'_>,
    evidence: &mut GitEvidence,
    originals: &dyn Fn(&[String]) -> Result<FullManifest, String>,
) -> Result<(FullPlanning, Value), String> {
    if request.scope["kind"] != "collect" {
        return Err("collection report inputs require collection scope".into());
    }
    let read = |oid: &str| -> Result<BTreeMap<String, String>, String> {
        Ok(reader
            .registry_snapshot(&request.candidate.root, oid, &request.config_path)?
            .originals)
    };
    let documents = [
        read(&request.base.commit)?,
        read(&request.candidate.commit)?,
    ];
    let (binding, offset) = source_observations::binding(&reader.observation()?)?;
    let mut message = json!({"operation":{"mode":"ci-collection","request":original,"migration":null},"originals":documents});
    let mut read_projection = |message: &Value| -> Result<Outcome, String> {
        let reply: Reply = query.read("chrono-judge-registration", "impact-views-v1", message)?;
        if reply.request_id != request.request_id
            || reply.commits[0] != request.base.commit
            || reply.commits[1] != request.candidate.commit
        {
            return Err(
                "E_PLAN_BINDING: registration returned different collection coordinates".into(),
            );
        }
        evidence.record(&binding, offset, reply.git_facts)?;
        Ok(reply.outcome)
    };
    let mut outcome = read_projection(&message)?;
    let mut migration = Value::Null;
    if let Outcome::Needed(units) = outcome {
        if units.is_empty() || units.iter().collect::<BTreeSet<_>>().len() != units.len() {
            return Err("E_COLLECTION_INPUT: invalid migration unit selection".into());
        }
        migration = json!({"units":units, "originals":originals(&units)?});
        message["operation"]["migration"] = migration.clone();
        outcome = read_projection(&message)?;
    }
    match outcome {
        Outcome::Ready(plans) => Ok((plans, migration)),
        Outcome::Error(error) => Err(error),
        Outcome::Needed(_) => Err("E_COLLECTION_INPUT: repeated migration request".into()),
    }
}
