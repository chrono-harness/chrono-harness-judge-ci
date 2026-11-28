//! Collect only the original reports requested by registration's historical conversion.
#[cfg(not(verus_keep_ghost))]
use crate::originals::FullManifest;
#[cfg(not(verus_keep_ghost))]
use crate::scheduling_git::Reader;
#[cfg(not(verus_keep_ghost))]
use crate::{
    FullPlanning,
    impact_query::Query,
    source_observations::{self, GitEvidence},
};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// Coordinates consumed locally after the installed runner verifies the complete original.
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Coordinates {
    pub request_id: String,
    pub config_path: String,
    pub scope: Value,
    pub base: Endpoint,
    pub candidate: Endpoint,
    pub runner: Executable,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Endpoint {
    pub commit: String,
    pub root: PathBuf,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Executable {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    request_id: String,
    commits: [String; 2],
    #[serde(default)]
    git_facts: Value,
    outcome: Outcome,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Ready(FullPlanning),
    Needed(Vec<String>),
    Error(String),
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn collection(
    request: &Coordinates,
    original: &Value,
    reader: &Reader,
    query: &Query<'_>,
    evidence: &mut GitEvidence,
    originals: &dyn Fn(&[String]) -> Result<FullManifest, String>,
) -> Result<(FullPlanning, Value), String> {
    if !collection_scope(request.scope["kind"].as_str()) {
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
        if !projection_matches(
            (
                &request.request_id,
                &request.base.commit,
                &request.candidate.commit,
            ),
            (&reply.request_id, &reply.commits[0], &reply.commits[1]),
        ) {
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

#[cfg(verus_keep_ghost)]
use super::verified_text::equal_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_collection::verified_text::equal_text;
use vstd::prelude::*;

verus! {
pub fn collection_scope(kind: Option<&str>) -> (allowed: bool)
    ensures allowed == match kind { Some(kind) => kind@ == "collect"@, None => false },
{
    match kind {
        Some(kind) => equal_text(kind, "collect"),
        None => false,
    }
}

/// Registration must return both endpoints in the original order.
pub fn endpoints_match(expected: (&str, &str), actual: (&str, &str)) -> (bound: bool)
    ensures bound == (expected.0@ == actual.0@ && expected.1@ == actual.1@),
{
    equal_text(expected.0, actual.0) && equal_text(expected.1, actual.1)
}

/// Bind the registration projection to this exact collection and both commits.
pub fn projection_matches(expected: (&str, &str, &str), actual: (&str, &str, &str)) -> (bound: bool)
    ensures bound == (expected.0@ == actual.0@ && expected.1@ == actual.1@ && expected.2@ == actual.2@),
{
    equal_text(expected.0, actual.0) && endpoints_match((expected.1, expected.2), (actual.1, actual.2))
}
}
