#![allow(unexpected_cfgs)]
// Explicit CI policy: fixed Git snapshots, registered old/new impact graph, candidate operations.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{
    CheckConfig, PROTOCOL, Request, Response, decode, json, relative_path, sha256,
};
#[cfg(not(verus_keep_ghost))]
pub mod consumption;
#[cfg(not(verus_keep_ghost))]
pub mod impact_query;
#[cfg(not(verus_keep_ghost))]
mod project_execution;
#[cfg(not(verus_keep_ghost))]
pub mod project_pairs;
#[cfg(not(verus_keep_ghost))]
mod route_query;
#[cfg(not(verus_keep_ghost))]
mod route_wire;
#[cfg(not(verus_keep_ghost))]
mod source_observations;
pub mod verified_consumption;
#[cfg(not(verus_keep_ghost))]
use impact_query::{Edge, Query};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json as object};
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use std::fs;
#[cfg(not(verus_keep_ghost))]
use std::path::Path;
#[cfg(not(verus_keep_ghost))]
mod collect;
#[cfg(not(verus_keep_ghost))]
pub mod requirements;
#[cfg(not(verus_keep_ghost))]
pub use collect::{UnitOriginals, unit_originals, unit_response};
mod source_binding;
#[cfg(not(verus_keep_ghost))]
mod units;
#[allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
pub mod verified_collection;
#[cfg(not(verus_keep_ghost))]
use verified_collection::result_types::{CheckResult, Status};

#[cfg(not(verus_keep_ghost))]
struct Evaluation {
    protocol: String,
    request_id: String,
    status: Status,
    results: Vec<CheckResult>,
    evidence: Value,
}

#[cfg(not(verus_keep_ghost))]
impl From<Evaluation> for Response {
    fn from(value: Evaluation) -> Self {
        Self {
            protocol: value.protocol,
            request_id: value.request_id,
            status: value.status.into(),
            results: value.results.into_iter().map(Into::into).collect(),
            evidence: value.evidence,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg(not(verus_keep_ghost))]
pub enum ExecutionDiagnostics {
    #[serde(rename = "chrono-log-stderr/v1")]
    StderrV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct CollectionLimits {
    pub manifest_bytes: u64,
    pub report_bytes: u64,
}

#[cfg(not(verus_keep_ghost))]
impl Default for CollectionLimits {
    fn default() -> Self {
        Self {
            manifest_bytes: 64 * 1024 * 1024,
            report_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Policy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_diagnostics: Option<ExecutionDiagnostics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_evidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_publication: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_limits: Option<CollectionLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<BTreeMap<String, chrono_harness::units::Unit>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_operations: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts_config: Option<String>,
    #[serde(default)]
    pub registration_config: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causal_consumption: Option<Value>,
    pub filemap: String,
    pub projects: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub artifacts: Vec<String>,
    pub required_inputs: Vec<String>,
    pub adoption_base: Option<String>,
    pub operation_timeout_seconds: u64,
    pub operation_output_limit_bytes: usize,
}
#[derive(Debug)]
#[cfg(not(verus_keep_ghost))]
struct Snapshot {
    registry: Value,
    filemap: Value,
    tools: BTreeMap<String, Value>,
    owner_nodes: BTreeMap<String, String>,
    test_owners: BTreeMap<String, String>,
    files: BTreeMap<String, Value>,
    source_registry: Result<chrono_harness::verified_verifier::sources::SourceRegistry, String>,
    projects: BTreeMap<String, Value>,
    operations: BTreeMap<
        String,
        (
            String,
            String,
            Vec<String>,
            Option<chrono_harness::verified_verifier::OutputContract>,
        ),
    >,
    edges: BTreeSet<Edge>,
    tree: BTreeMap<String, (String, String)>,
    policy: Policy,
    plans: BTreeMap<String, chrono_judge_registration::execution::Plan>,
    execute: BTreeMap<String, String>,
    scheduling: Option<chrono_judge_registration::execution::Scheduling>,
}
#[cfg(not(verus_keep_ghost))]
use chrono_harness::facts::{Reader, utf8};
#[cfg(not(verus_keep_ghost))]
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing/string field {k}"))
}
#[cfg(not(verus_keep_ghost))]
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, String> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing/array field {k}"))
}
#[cfg(not(verus_keep_ghost))]
fn strings(v: &Value) -> Result<Vec<String>, String> {
    v.as_array()
        .ok_or("expected string array")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or("expected string".into())
        })
        .collect()
}
#[cfg(not(verus_keep_ghost))]
fn id(s: &str) -> Result<(), String> {
    if !chrono_harness::verified_identifiers::valid_id(s.as_bytes()) {
        return Err(format!("invalid ID {s:?}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn full_oid(s: &str) -> Result<(), String> {
    if !chrono_harness::verified_identifiers::valid_oid(s.as_bytes()) {
        return Err(format!("expected full nonzero lowercase OID: {s}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn verify_oid(reader: &Reader, root: &Path, oid: &str) -> Result<(), String> {
    full_oid(oid)?;
    reader.verify_oid(root, oid)?;
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn at(reader: &Reader, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
    relative_path(path)?;
    reader.blob(root, oid, path)
}
#[cfg(not(verus_keep_ghost))]
fn tree(
    reader: &Reader,
    root: &Path,
    oid: &str,
) -> Result<BTreeMap<String, (String, String)>, String> {
    reader
        .tree(root, oid)?
        .into_iter()
        .map(|(path, entry)| {
            let path = path.text;
            if entry.kind != "blob"
                || !matches!(entry.mode.as_str(), "100644" | "100755" | "120000")
            {
                return Err(format!(
                    "unsupported tree entry {path}: {} {} {}",
                    entry.mode, entry.kind, entry.oid
                ));
            }
            Ok((path, (entry.mode, entry.oid)))
        })
        .collect()
}
#[cfg(not(verus_keep_ghost))]
fn policy(c: &CheckConfig) -> Result<Policy, String> {
    chrono_harness::units::reference_publication(c)?;
    if !matches!(
        c.schema.as_str(),
        "chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3"
    ) {
        return Err("unsupported CI profile".into());
    }
    if c.schema == "chrono-ci-check/v1" && c.policy.get("facts_config").is_some() {
        return Err("v1 cannot declare facts_config, including null".into());
    }
    let p: Policy = serde_json::from_value(c.policy.clone()).map_err(|e| e.to_string())?;
    if c.schema == "chrono-ci-check/v2" {
        let path = p
            .facts_config
            .as_deref()
            .ok_or("v2 requires facts_config")?;
        relative_path(path)?;
        if !path.starts_with(".chrono-harness/") {
            return Err("facts_config must reside in .chrono-harness/".into());
        }
    }
    relative_path(&p.filemap)?;
    relative_path(&p.projects)?;
    if p.artifacts.is_empty() {
        return Err("artifact declarations required".into());
    }
    for a in &p.artifacts {
        if !a.ends_with('/') {
            return Err("artifact must be a directory prefix ending /".into());
        }
        relative_path(a.trim_end_matches('/'))?;
    }
    if !p.artifacts.iter().any(|a| c.report_path.starts_with(a)) {
        return Err("report must reside in declared artifact directory".into());
    }
    for v in &p.required_inputs {
        relative_path(v)?
    }
    if p.registration_config.is_none() {
        for (k, v) in &p.tools {
            id(k)?;
            if v.is_empty() {
                return Err("empty tool program".into());
            }
        }
    }
    if let Some(b) = &p.adoption_base {
        full_oid(b)?
    }
    if p.operation_timeout_seconds == 0
        || p.operation_output_limit_bytes == 0
        || p.operation_output_limit_bytes > 64 * 1024 * 1024
    {
        return Err("invalid operation bounds".into());
    }
    units::validate(c, &p)?;
    Ok(p)
}
#[cfg(not(verus_keep_ghost))]
struct EndpointTools {
    registration: Option<chrono_harness::facts::RegistrySnapshot>,
    tools: BTreeMap<String, Value>,
}

#[cfg(not(verus_keep_ghost))]
fn endpoint_tools(
    reader: &Reader,
    root: &Path,
    oid: &str,
    p: &Policy,
) -> Result<EndpointTools, String> {
    let registration = p
        .registration_config
        .as_ref()
        .map(|config| reader.registry_snapshot(root, oid, config))
        .transpose()?;
    let tools = if let Some(snapshot) = &registration {
        let config = snapshot
            .values
            .get(&snapshot.effective_path)
            .ok_or("missing effective registration config")?;
        let mut tools = BTreeMap::new();
        for tool in array(config, "tools")? {
            let name = text(tool, "id")?;
            id(name)?;
            if tools.insert(name.to_owned(), tool.clone()).is_some() {
                return Err(format!("duplicate registered tool {name}"));
            }
        }
        tools
    } else {
        p.tools
            .iter()
            .map(|(id, program)| {
                (id.clone(), object!({"id":id,"program":program,"version_argv":["--version"],"expected_version":null}))
            })
            .collect()
    };
    Ok(EndpointTools {
        registration,
        tools,
    })
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct TestBinding {
    owner: String,
    operation: String,
}

#[cfg(not(verus_keep_ghost))]
type TestBindings = BTreeMap<String, Vec<TestBinding>>;

#[cfg(not(verus_keep_ghost))]
fn snapshot(
    reader: &Reader,
    root: &Path,
    oid: &str,
    mut p: Policy,
    endpoint: EndpointTools,
    query: &Query,
    program: &str,
    adoption: bool,
) -> Result<Snapshot, String> {
    let mut fm = json(&at(reader, root, oid, &p.filemap)?)?;
    let EndpointTools {
        registration,
        tools,
    } = endpoint;
    if fm.get("configuration_edges").is_some() {
        let effective_config = registration.as_ref().map(|s| s.effective_path.as_str());
        let extra: Vec<Value> = query.read(
            program,
            "configuration-edges-v1",
            &object!({"filemap":fm, "effective_config":effective_config}),
        )?;
        fm.as_object_mut()
            .ok_or("expected FILEMAP object")?
            .remove("configuration_edges");
        fm["project_edges"]
            .as_array_mut()
            .ok_or("expected FILEMAP project_edges array")?
            .extend(extra);
    }
    let pr = json(&at(reader, root, oid, &p.projects)?)?;
    if ![object!(1), object!(2)].contains(&fm["schema_version"])
        || ![object!(1), object!(2)].contains(&pr["schema_version"])
    {
        return Err("unsupported registry version".into());
    }
    let artifacts = if let Some(values) = &registration {
        values.values[&values.effective_path]["artifacts"].clone()
    } else {
        object!(
            p.artifacts
                .iter()
                .map(|path| object!({"path":path}))
                .collect::<Vec<_>>()
        )
    };
    let declarations: serde_json::Map<_, _> = [
        "schema_version",
        "execution_plans",
        "execution_scheduling",
        "reference_syntax",
    ]
    .into_iter()
    .filter_map(|key| fm.get(key).map(|value| (key.to_owned(), value.clone())))
    .collect();
    let registered_tests: TestBindings = query.read(
        program,
        "scoped-execution-v1",
        &object!({"filemap":declarations,"projects":pr,"artifacts":artifacts}),
    )?;
    if adoption {
        let mut old_operations = BTreeSet::new();
        for collection in ["projects", "scripts"] {
            for entry in array(&pr, collection)? {
                for action in entry["actions"]
                    .as_object()
                    .ok_or("actions missing")?
                    .values()
                {
                    old_operations.insert(text(action, "operation")?);
                }
            }
        }
        p.bindings.retain(|test, operations| {
            registered_tests.contains_key(test)
                && operations
                    .iter()
                    .all(|op| old_operations.contains(op.as_str()))
        });
    }
    let plans = if fm["schema_version"] == 2 {
        if !p.bindings.is_empty() {
            return Err("current FILEMAP v2 cannot duplicate plans in scoped bindings".into());
        }
        let plans: BTreeMap<String, chrono_judge_registration::execution::Plan> =
            serde_json::from_value(fm["execution_plans"].clone())
                .map_err(|error| error.to_string())?;
        p.bindings = plans
            .iter()
            .map(|(id, p)| (id.clone(), p.operations.clone()))
            .collect();
        plans
    } else {
        p.bindings
            .iter()
            .map(|(id, ops)| {
                (
                    id.clone(),
                    chrono_judge_registration::execution::Plan {
                        operations: ops.clone(),
                        timeout_seconds: p.operation_timeout_seconds,
                        output_limit_bytes: p.operation_output_limit_bytes,
                        operation_bounds: BTreeMap::new(),
                    },
                )
            })
            .collect()
    };
    let mut execute = BTreeMap::new();
    let mut test_owners = BTreeMap::new();
    for (test, bindings) in registered_tests {
        let Ok([binding]) = <[_; 1]>::try_from(bindings) else {
            return Err(format!("E_TEST_BINDING: duplicate test identity {test}"));
        };
        test_owners.insert(test.clone(), binding.owner);
        execute.insert(test, binding.operation);
    }
    let owners: BTreeSet<_> = strings(&pr["owners"])?.into_iter().collect();
    if owners.is_empty() {
        return Err("empty owner registry".into());
    }
    let mut projects = BTreeMap::new();
    let mut owner_nodes = BTreeMap::new();
    let mut ops = BTreeMap::new();
    let mut nodes = BTreeSet::new();
    if let Some(snapshot) = &registration {
        let config = snapshot
            .values
            .get(&snapshot.effective_path)
            .ok_or("missing effective registration config")?;
        for tool in config["tools"].as_array().ok_or("tools missing")? {
            nodes.insert(format!("tool:{}", text(tool, "id")?));
        }
        for input in config["environment"]["inputs"]
            .as_array()
            .ok_or("inputs missing")?
        {
            nodes.insert(format!("input:{}", text(input, "id")?));
        }
        for key in config["environment"]["inherit"]
            .as_array()
            .ok_or("inherit missing")?
            .iter()
            .filter_map(Value::as_str)
            .chain(
                config["environment"]["values"]
                    .as_object()
                    .ok_or("values missing")?
                    .keys()
                    .map(String::as_str),
            )
        {
            nodes.insert(format!("environment:{key}"));
        }
    }

    for collection in ["projects", "scripts"] {
        for entry in array(&pr, collection)? {
            let name = text(entry, "id")?;
            id(name)?;
            if !owners.contains(name) {
                return Err(format!("undeclared owner {name}"));
            }
            if projects.insert(name.to_owned(), entry.clone()).is_some() {
                return Err(format!("duplicate project/script {name}"));
            }
            let prefix = if pr["schema_version"] == 2 && collection == "scripts" {
                "script"
            } else {
                "project"
            };
            owner_nodes.insert(name.to_owned(), format!("{prefix}:{name}"));
            nodes.insert(format!("script:{name}"));
            nodes.insert(format!("project:{name}"));
            for a in entry
                .get("actions")
                .and_then(Value::as_object)
                .ok_or("actions object required")?
                .values()
            {
                let action: chrono_harness::verified_verifier::RegisteredAction =
                    serde_json::from_value(a.clone()).map_err(|e| e.to_string())?;
                chrono_harness::verified_verifier::validate_contract(&action.output_contract)?;
                let operation = action.operation.as_str();
                id(operation)?;
                let tool = action.tool.as_str();
                if !verified_collection::action_tool_allowed(
                    registration.is_some(),
                    tools.contains_key(tool),
                    p.tools.contains_key(tool),
                ) {
                    return Err(format!("unknown tool {tool}"));
                }
                let argv = action.argv;
                if argv.iter().any(|s| s.contains('\0')) {
                    return Err(format!("invalid action {operation}"));
                }
                if ops
                    .insert(
                        operation.to_string(),
                        (
                            name.to_string(),
                            tool.to_string(),
                            argv,
                            action.output_contract,
                        ),
                    )
                    .is_some()
                {
                    return Err(format!("duplicate operation {operation}"));
                }
            }
        }
    }
    nodes.extend(execute.keys().cloned());
    for (test, binding) in &p.bindings {
        if !test.starts_with("test:") || !nodes.contains(test) || binding.is_empty() {
            return Err(format!("invalid check binding {test}"));
        }
        let mut seen = BTreeSet::new();
        for op in binding {
            if !ops.contains_key(op) || !seen.insert(op) {
                return Err(format!("invalid bound operation {op}"));
            }
        }
    }
    let tree = tree(reader, root, oid)?;
    let mut files = BTreeMap::new();
    for f in array(&fm, "files")? {
        let path = text(f, "path")?;
        relative_path(path)?;
        if !owners.contains(text(f, "owner")?) {
            return Err(format!("unknown file owner: {path}"));
        }
        if !matches!(
            text(f, "surface")?,
            "product"
                | "test"
                | "documentation"
                | "membership"
                | "instruction-policy"
                | "judge-policy"
                | "judge-implementation"
        ) {
            return Err(format!("unknown surface: {path}"));
        }
        text(f, "cost")?;
        if files.insert(path.to_string(), f.clone()).is_some() {
            return Err(format!("duplicate FILEMAP path {path}"));
        }
        nodes.insert(format!("file:{path}"));
    }
    for path in tree.keys() {
        if !files.contains_key(path) {
            return Err(format!("unregistered tracked path at {oid}: {path}"));
        }
        if p.artifacts.iter().any(|a| path.starts_with(a)) {
            return Err(format!("tracked path overlaps artifact exemption: {path}"));
        }
    }
    for (path, f) in &files {
        let (mode, _) = tree
            .get(path)
            .ok_or_else(|| format!("registered path absent from {oid}: {path}"))?;
        match (mode.as_str(), f.get("symlink")) {
            ("120000", Some(Value::String(target))) => {
                relative_path(target)?;
                if utf8(at(reader, root, oid, path)?)? != *target {
                    return Err(format!("symlink target mismatch: {path}"));
                }
                let dest = Path::new(path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(target);
                let dest = dest.to_str().ok_or("symlink target UTF-8")?;
                if !tree.get(dest).is_some_and(|(m, _)| m != "120000") {
                    return Err(format!("symlink target not regular: {path}"));
                }
            }
            ("120000", _) => return Err(format!("undeclared symlink: {path}")),
            (_, Some(_)) => return Err(format!("declared symlink is regular: {path}")),
            _ => {}
        }
    }
    for required in p.required_inputs.iter().chain([&p.filemap, &p.projects]) {
        if !tree.get(required).is_some_and(|(m, _)| m != "120000") {
            return Err(format!("required regular input absent: {required}"));
        }
    }
    let mut edges = BTreeSet::new();
    let mut add = |from: &str, e: &Value| -> Result<(), String> {
        let kind = text(e, "kind")?;
        let to = text(e, "to")?;
        if !matches!(
            kind,
            "compile" | "build-input" | "runtime-input" | "test-execution" | "judge-trigger"
        ) {
            return Err(format!("unsupported edge kind {kind}"));
        }
        if kind == "judge-trigger" {
            if !to.starts_with("judge:") {
                return Err("judge-trigger target requires judge namespace".into());
            }
            return Ok(());
        }
        if !nodes.contains(from) || !nodes.contains(to) {
            return Err(format!("unresolved edge {from} -> {to}"));
        }
        if kind == "test-execution" && (!to.starts_with("test:") || !p.bindings.contains_key(to)) {
            return Err(format!("test edge lacks active binding: {to}"));
        }
        if kind != "test-execution" && to.starts_with("test:") {
            return Err("test target requires test-execution edge".into());
        }
        if !edges.insert(Edge {
            from: from.into(),
            kind: kind.into(),
            to: to.into(),
        }) {
            return Err(format!("duplicate edge {from} -> {to}"));
        }
        Ok(())
    };
    for (path, f) in &files {
        for e in array(f, "edges")? {
            add(&format!("file:{path}"), e)?
        }
    }
    for e in array(&fm, "project_edges")? {
        add(text(e, "from")?, e)?
    }
    let scheduling = serde_json::from_value(fm["execution_scheduling"].clone())
        .map_err(|error| error.to_string())?;
    let snapshot = Snapshot {
        registry: pr,
        filemap: fm.clone(),
        tools,
        owner_nodes,
        test_owners,
        files,
        source_registry: serde_json::from_value(fm).map_err(|e| e.to_string()),
        projects,
        operations: ops,
        edges,
        tree,
        policy: p,
        plans,
        execute,
        scheduling,
    };
    units::assignments(&snapshot)?;
    Ok(snapshot)
}
#[cfg(not(verus_keep_ghost))]
fn clean(reader: &Reader, root: &Path, candidate: &str, p: &Policy) -> Result<(), String> {
    if utf8(reader.git(root, &["rev-parse", "HEAD"])?)?.trim() != candidate {
        return Err("checkout HEAD does not equal candidate".into());
    }
    let flags = reader.index_flags(root)?;
    if let Some((entry, reason)) =
        chrono_harness::verified_status::first_unsupported_index_flag(&flags)
    {
        return Err(format!(
            "unsupported index flags ({}): {}",
            reason.message(),
            entry.path
        ));
    }
    if !reader.tracked_changes(root, candidate)?.is_empty() {
        return Err("dirty tracked candidate inputs/index".into());
    }
    let artifacts: Vec<_> = p.artifacts.iter().map(String::as_str).collect();
    for path in reader.untracked_excluding(root, &artifacts)? {
        if !p.artifacts.iter().any(|a| path.starts_with(a)) {
            return Err(format!("untracked nonartifact input: {path}"));
        }
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn changed<T: PartialEq>(a: &BTreeMap<String, T>, b: &BTreeMap<String, T>) -> BTreeSet<String> {
    a.keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}
// Scoped CI owns legacy seeds and selections; it consumes only shared union/closure mechanics.
#[cfg(not(verus_keep_ghost))]
fn ci_impact(
    root: &Path,
    old: Option<&Snapshot>,
    new: &Snapshot,
) -> Result<(BTreeSet<String>, BTreeSet<String>, Value, BTreeSet<String>), String> {
    let paths = if let Some(old) = old {
        changed(&old.tree, &new.tree)
    } else {
        new.tree.keys().cloned().collect()
    };
    let mut seeds: BTreeSet<_> = paths.iter().map(|p| format!("file:{p}")).collect();
    let mut extras: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(old) = old {
        // Adopting a new document contract affects every declared owner, even
        // when legacy rows already contain the newly meaningful metadata.
        if old.registry["schema_version"] != new.registry["schema_version"] {
            seeds.extend(
                old.owner_nodes
                    .values()
                    .chain(new.owner_nodes.values())
                    .cloned(),
            );
        }
        for path in changed(&old.files, &new.files) {
            seeds.insert(format!("file:{path}"));
        }
        for p in changed(&old.projects, &new.projects) {
            for snapshot in [old, new] {
                if let Some(node) = snapshot.owner_nodes.get(&p) {
                    seeds.insert(node.clone());
                }
            }
        }
        for test in changed(&old.execute, &new.execute) {
            extras
                .entry(test)
                .or_default()
                .insert("changed test action binding".into());
        }
        for op in changed(&old.operations, &new.operations) {
            for s in [old, new] {
                if let Some((owner, _, _, _)) = s.operations.get(&op) {
                    seeds.insert(s.owner_nodes[owner].clone());
                }
                for (test, ops) in &s.policy.bindings {
                    if ops.contains(&op) {
                        extras
                            .entry(test.clone())
                            .or_default()
                            .insert(format!("changed operation: {op}"));
                    }
                }
            }
        }
        for e in old.edges.symmetric_difference(&new.edges) {
            seeds.insert(e.from.clone());
            if e.kind != "test-execution" {
                seeds.insert(e.to.clone());
            }
        }
        // Effective plans retain explicit v2 bounds and the projected legacy v1 bounds.
        for test in changed(&old.plans, &new.plans) {
            let before = old.plans.get(&test);
            let after = new.plans.get(&test);
            let reasons = extras.entry(test).or_default();
            if before.map(|p| &p.operations) != after.map(|p| &p.operations) {
                reasons.insert("changed binding".into());
            }
            if let (Some(before), Some(after)) = (before, after) {
                if before.timeout_seconds != after.timeout_seconds {
                    reasons.insert("changed operation timeout".into());
                }
                if before.output_limit_bytes != after.output_limit_bytes {
                    reasons.insert("changed operation output limit".into());
                }
                if before.operation_bounds != after.operation_bounds {
                    reasons.insert("changed operation bounds".into());
                }
            }
        }
        if old.scheduling != new.scheduling {
            for s in [old, new] {
                for (test, plan) in &s.plans {
                    let global = old
                        .scheduling
                        .as_ref()
                        .map(|p| (p.max_running, &p.resources, &p.priority))
                        != new
                            .scheduling
                            .as_ref()
                            .map(|p| (p.max_running, &p.resources, &p.priority));
                    if global
                        || plan.operations.iter().any(|op| {
                            old.scheduling.as_ref().and_then(|p| p.claims.get(op))
                                != new.scheduling.as_ref().and_then(|p| p.claims.get(op))
                                || [old.scheduling.as_ref(), new.scheduling.as_ref()]
                                    .into_iter()
                                    .flatten()
                                    .any(|policy| {
                                        policy.claims.keys().any(|changed| {
                                            old.scheduling
                                                .as_ref()
                                                .and_then(|p| p.claims.get(changed))
                                                != new
                                                    .scheduling
                                                    .as_ref()
                                                    .and_then(|p| p.claims.get(changed))
                                                && policy.claims.contains_key(op)
                                                && policy.conflicts(op, changed)
                                        })
                                    })
                        })
                    {
                        extras
                            .entry(test.clone())
                            .or_default()
                            .insert("changed execution scheduling declaration".into());
                    }
                }
            }
        }
        let empty_units = BTreeMap::new();
        let old_units = old.policy.units.as_ref().unwrap_or(&empty_units);
        let new_units = new.policy.units.as_ref().unwrap_or(&empty_units);
        for id in changed(old_units, new_units) {
            for unit in [old_units.get(&id), new_units.get(&id)]
                .into_iter()
                .flatten()
            {
                for test in &unit.tests {
                    extras
                        .entry(test.clone())
                        .or_default()
                        .insert(format!("changed unit assignment: {id}"));
                }
            }
        }
        let changed_tools = changed(&old.tools, &new.tools);
        if !changed_tools.is_empty() || old.policy.environment != new.policy.environment {
            for s in [old, new] {
                for (test, ops) in &s.policy.bindings {
                    if old.policy.environment != new.policy.environment
                        || ops.iter().any(|op| {
                            s.operations
                                .get(op)
                                .is_some_and(|(_, tool, _, _)| changed_tools.contains(tool))
                        })
                    {
                        let reasons = extras.entry(test.clone()).or_default();
                        if old.policy.environment != new.policy.environment {
                            reasons.insert("changed environment".into());
                        }
                        for tool in &changed_tools {
                            if ops.iter().any(|op| {
                                s.operations.get(op).is_some_and(|(_, t, _, _)| t == tool)
                            }) {
                                reasons.insert(format!("changed bound tool: {tool}"));
                            }
                        }
                    }
                }
            }
        }
    }
    let empty = BTreeSet::new();
    let before = old.map(|s| &s.edges).unwrap_or(&empty);
    let seeds: Vec<_> = seeds.into_iter().map(|node| object!({"id":node,"node":node,"reference":node,"reason":"scoped CI path, registration or operation seed"})).collect();
    let replay = new.policy.selection_evidence.as_deref() == Some("reachable-graph/v1");
    let program = new
        .tools
        .get("chrono-judge-filemap")
        .map(|tool| {
            tool["program"]
                .as_str()
                .filter(|path| !path.is_empty())
                .ok_or("invalid registered chrono-judge-filemap program")
        })
        .transpose()?
        .unwrap_or("chrono-judge-filemap");
    let query = Query::scoped(root, &new.policy)?;
    let closure: impact_query::Reachability = query.read(
        program,
        "closure-v1",
        &object!({
            "base":before,"candidate":new.edges,"seeds":seeds,"reachable_only":replay
        }),
    )?;
    let closure_evidence = if replay {
        object!({"schema":"chrono-ci-reachability/v1","reached":closure.reached,"selected_tests":closure.selected_tests})
    } else {
        closure.closure.ok_or("missing full graph closure")?
    };
    let mut selected = closure.selected_tests;
    selected.extend(extras.keys().cloned());
    let mut explanation = object!({"scope":"chrono-ci-check/v1-adapter","edges":closure.edges,"seed_causes":seeds,"closure":closure_evidence,"extra_selections":extras});
    if replay {
        explanation["representation"] = object!("reachable-graph/v1");
    }
    let mut affected = closure.reached;
    if old.is_none() {
        affected.extend(new.owner_nodes.values().cloned());
    }
    // Direct plan/tool selections also carry their explicitly registered test owner.
    affected.extend(
        selected
            .iter()
            .filter_map(|test| new.test_owners.get(test).cloned()),
    );
    Ok((paths, selected, explanation, affected))
}

#[cfg(not(verus_keep_ghost))]
pub fn judge(req: &Request) -> Response {
    #[cfg(unix)]
    let sink = chrono_harness::NativeOutput::new(std::io::stderr());
    #[cfg(not(unix))]
    let sink = std::io::stderr();
    judge_with_diagnostic_sink(req, sink)
}

/// Explicit sink adoption; the sealed host profile still selects diagnostics.
/// Child streams and verdicts do not pass through this diagnostic sink.
#[cfg(not(verus_keep_ghost))]
pub fn judge_with_diagnostic_sink<W: std::io::Write + Send + 'static>(
    req: &Request,
    sink: W,
) -> Response {
    let mut reader = Reader::legacy();
    let mut evidence = source_observations::GitEvidence::default();
    let mut scope = "chrono-ci-check/v1".to_owned();
    let mut diagnostic_errors = Vec::new();
    let mut project_errors = Vec::new();
    let result = Profile::load(req).and_then(|profile| {
        scope = profile.config.schema.clone();
        if profile.policy.execution_diagnostics.is_none() {
            return evaluate(
                req,
                &mut reader,
                &mut evidence,
                &mut scope,
                profile,
                &mut project_errors,
            );
        }
        let logger = chrono_diagnostics::runtime::logger(
            &req.host_root,
            "chrono-judge-ci",
            chrono_diagnostics::logging::Level::INFO,
            sink,
        )
        .map_err(|error| {
            diagnostic_errors.push(error.record());
            error.to_string()
        })?;
        let result = logger.run(|| {
            let span = tracing::info_span!(
                "ci.check",
                request_id = req.request_id,
                candidate = req.candidate,
                base = req.base.as_deref().unwrap_or(""),
                config_sha256 = req.config_sha256,
                judge_pid = std::process::id(),
            );
            let _entered = span.enter();
            tracing::info!(
                event_id = "ci.check_started",
                "sealed request diagnostics adopted; input admission still required"
            );
            let result = evaluate(
                req,
                &mut reader,
                &mut evidence,
                &mut scope,
                profile,
                &mut project_errors,
            );
            tracing::info!(
                event_id = "ci.evaluation_returned",
                has_response = result.is_ok(),
                "judge evaluation returned; response publication is separate"
            );
            result
        });
        if let Err(errors) = logger.finish() {
            use chrono_diagnostics::error::RecordedError;
            diagnostic_errors.extend(errors.iter().map(|error| {
                chrono_diagnostics::error::ErrorRecord {
                    schema: chrono_diagnostics::error::ErrorSchema::V1,
                    error: error.error_node(),
                }
            }));
        }
        result
    });
    let mut response = match result {
        Ok(r) => r,
        Err(e) => {
            let (status, results) = verified_collection::inventory_failure(e);
            Evaluation {
                protocol: units::scope_protocol(req).into(),
                request_id: req.request_id.clone(),
                status,
                results,
                evidence: object!({"scope":scope,"candidate":req.candidate,"base":req.base,"selected":[],"executed":[],"blocked":["selection or snapshot validation failed"],"parity":"unestablished"}),
            }
        }
    };
    let facts = evidence.finish(reader.observation());
    if !facts.is_null() {
        response.evidence["git_facts"] = facts;
    }
    diagnostic_errors.extend(project_errors.into_iter().map(|message| {
        chrono_diagnostics::error::Failure::new("E_PROJECT_TRACE", message).record()
    }));
    if !diagnostic_errors.is_empty() {
        response.status = verified_collection::diagnostic_failure(&mut response.results);
        response.evidence["diagnostic_errors"] = object!(diagnostic_errors);
    }
    response.into()
}
// Shared DELTA planning. Collection acquisition consumes requirements, never an admission result.
#[cfg(not(verus_keep_ghost))]
struct Inventory {
    config: CheckConfig,
    p: Policy,
    new: Snapshot,
    previous: String,
    paths: BTreeSet<String>,
    selected: BTreeSet<String>,
    global_selected: BTreeSet<String>,
    selection_explanation: Value,
    blocked: Vec<String>,
    removed: BTreeMap<String, Option<String>>,
    retirement_context: Value,
    conversion: Value,
    reused: BTreeMap<String, chrono_harness::observation::Tool>,
    environment: BTreeMap<String, String>,
    environment_policy: Value,
    declarations: Value,
    methods: BTreeMap<String, Vec<chrono_judge_registration::execution::Method>>,
}

/// Resolve only DELTA-required collection units through the adjudicator's current inventory/assignment owner.
/// This does not validate a canonical entry, execute operations or grant admission.
/// Full preparation shares FILEMAP impact and routes' global obligation owner.
/// No judge or business/version process is launched here.
/// The acquisition callback is used only for necessary historical conversion.
/// Registration names the decoder test units; the collector reads DELTA-required
/// reports separately after the shared selection owner returns its requirements.
#[cfg(not(verus_keep_ghost))]
pub fn full_collection_requirements_with_inputs(
    root: &Path,
    profile: &str,
    prepared: &chrono_harness::prepared::PreparedCheck,
    originals: &dyn Fn(&[String]) -> Result<chrono_harness::units::FullManifest, String>,
) -> Result<Value, String> {
    let context = prepared
        .context
        .as_ref()
        .ok_or("full collection context missing")?;
    full_collection_at(
        root,
        profile,
        prepared.base.as_deref().ok_or("full base missing")?,
        &prepared.candidate,
        Path::new(&context.path),
        prepared.scope.clone(),
        originals,
    )
}

#[cfg(not(verus_keep_ghost))]
fn full_collection_at(
    root: &Path,
    profile: &str,
    base: &str,
    candidate: &str,
    context: &Path,
    scope: Option<chrono_harness::units::Scope>,
    originals: &dyn Fn(&[String]) -> Result<chrono_harness::units::FullManifest, String>,
) -> Result<Value, String> {
    let (mut req, _, reader, cfg) = chrono_harness::full::prepare_request(
        root,
        profile,
        base,
        candidate,
        context,
        Value::Null,
        scope,
        None,
    )?;
    req.judge_id = "routes".into();
    let result = (|| {
        // prepare_request observes the calling executable.
        // Bootstrap instead carries the independently validated registered runner;
        // no retained report can assign an executable identity to this request.
        let declared = cfg["runner"]["path"]
            .as_str()
            .ok_or("registered runner path")?;
        relative_path(declared)?;
        let link = Path::new(declared);
        if let Some(parent) = link
            .parent()
            .and_then(Path::to_str)
            .filter(|s| !s.is_empty())
        {
            chrono_harness::no_symlink_parents(root, parent)?;
        }
        let runner = fs::canonicalize(root.join(link)).map_err(|e| e.to_string())?;
        let digest = chrono_harness::file_identity(&runner)?.0;
        if cfg["runner"]["sha256"] != digest || cfg["runner"]["version"] != req.runner.version {
            return Err("E_EXECUTABLE_BINDING: registered collection runner differs".into());
        }
        req.runner.path = runner.to_str().ok_or("runner path UTF8")?.into();
        req.runner.sha256 = digest;
        req.seal()?;
        // Only the original producer may satisfy registration's requested migration units.
        let migration = std::cell::RefCell::new(None);
        let (old, r, _) =
            chrono_judge_registration::views_for_collection_inputs_with(&req, &reader, &|units| {
                let reports = originals(units)?;
                migration.replace(Some(object!({"units":units,"originals":reports})));
                Ok(reports)
            })?;
        let query = Query::full(root, r.config())?;
        let mut collection = object!({"request":req,"migration":migration.into_inner()});
        let impact: impact_query::Obligations =
            query.read("chrono-judge-filemap", "collection-tests-v1", &collection)?;
        impact.require_valid("blocked full obligations")?;
        collection["tests"] = object!(impact.tests);
        collection["retired_tests"] = object!(impact.retired_tests);
        let selected = query.read("chrono-judge-routes", "collected-selection-v1", &collection)?;
        let routes = route_query::full_operations(&query, &r, &selected, None)?;
        let mut evidence = source_observations::GitEvidence::default();
        let facts = reader.observation();
        route_query::source_plan(
            &query,
            "chrono-judge-routes",
            profile,
            Some(route_query::SourceEndpoint::registered(
                &req.base.commit,
                &old,
            )?),
            route_query::SourceEndpoint::registered(&req.candidate.commit, &r)?,
            &impact.reached,
            &routes.methods,
            &routes.operations,
            &facts,
            &mut evidence,
        )?;
        let git_facts = evidence.finish(facts);
        let units = serde_json::from_value(r.config()["execution_units"]["units"].clone())
            .map_err(|e| e.to_string())?;
        Ok(
            object!({"required_units":chrono_harness::units::required(&units,&selected),"global_selected":selected,"git_facts":git_facts,"filemap_git_facts":impact.git_facts}),
        )
    })();
    fs::remove_dir_all(&req.base.root).map_err(|e| e.to_string())?;
    result
}
/// Full scheduling reads complete trees and registry/semantic blobs, never exports host sources,
/// captures SDK inputs, or launches a historical decoder. Unsupported historical registries fail.
/// External effective-input changes remain the final full collection owner's obligation.
#[cfg(not(verus_keep_ghost))]
pub fn full_scheduling_requirements(
    root: &Path,
    profile: &str,
    base: &str,
    candidate: &str,
) -> Result<Value, String> {
    let reader = Reader::for_config(root, profile)?;
    reader.verify_config(root, candidate)?;
    reader.verify_oid(root, base)?;
    reader.verify_oid(root, candidate)?;
    let (old, current, view) = chrono_judge_registration::interpret_scheduling_with_reader(
        &reader,
        root,
        base,
        candidate,
        profile,
        reader.registry_values(root, base, profile)?,
        reader.registry_values(root, candidate, profile)?,
    )
    .map_err(|e| {
        format!("full scheduling requires directly readable historical registries: {e}")
    })?;
    let query = Query::full(root, current.config())?;
    let impact: impact_query::Obligations = Query::full(root, current.config())?
        .with_git_environment(&reader.observation())?
        .read(
            "chrono-judge-filemap",
            "endpoint-tests",
            &object!({"config":profile,"base":base,"candidate":candidate,"scheduling":true}),
        )?;
    impact.require_valid("blocked full scheduling obligations")?;
    let selected = query.read("chrono-judge-routes", "global-selection", &object!({
        "config":profile,"base":view["base"],"candidate":view["candidate"],"tests":impact.tests,"retired_tests":impact.retired_tests
    }))?;
    let block = chrono_harness::units::full_execution_units(current.config())?
        .ok_or("full scheduling units missing")?;
    let routes = route_query::full_operations(&query, &current, &selected, Some(&block))?;
    let mut evidence = source_observations::GitEvidence::default();
    let facts = reader.observation();
    route_query::source_plan(
        &query,
        "chrono-judge-routes",
        profile,
        Some(route_query::SourceEndpoint::registered(base, &old)?),
        route_query::SourceEndpoint::registered(candidate, &current)?,
        &impact.reached,
        &routes.methods,
        &routes.operations,
        &facts,
        &mut evidence,
    )?;
    let git_facts = evidence.finish(facts);
    Ok(
        object!({"required_units":routes.required_units,"global_selected":selected,"changed_paths":impact.changed_paths.ok_or("missing FILEMAP scheduling paths")?,"git_facts":git_facts,"filemap_git_facts":impact.git_facts,"input_scope":"git-and-declarations; final-effective-input-admission-required"}),
    )
}

/// Scheduling uses the same scoped inventory without requiring a materialized host checkout.
/// Registry bytes are still bound to the fixed candidate; this grants no check admission.
#[cfg(not(verus_keep_ghost))]
pub fn scheduling_requirements(
    root: &Path,
    profile: &str,
    base: Option<String>,
    candidate: String,
    initial: bool,
) -> Result<Value, String> {
    let req = Request {
        protocol: PROTOCOL.into(),
        request_id: "scheduling-requirements".into(),
        host_root: root.into(),
        config_path: profile.into(),
        config_sha256: sha256(&fs::read(root.join(profile)).map_err(|e| e.to_string())?),
        base,
        candidate,
        initial,
        scope: None,
        observations: object!({}),
    };
    let mut reader = Reader::legacy();
    let mut evidence = source_observations::GitEvidence::default();
    let i = inventory(
        &req,
        &mut reader,
        &mut evidence,
        &mut String::new(),
        false,
        false,
    )?;
    if !i.blocked.is_empty() {
        return Err(format!("blocked global obligations: {:?}", i.blocked));
    }
    Ok(
        object!({"required_units":units::required(i.p.units.as_ref().ok_or("scheduling requires registered units")?, &i.global_selected),
        "global_selected":i.global_selected,"changed_paths":i.paths,"selection_explanation":i.selection_explanation,"git_facts":evidence.finish(reader.observation())}),
    )
}

#[cfg(not(verus_keep_ghost))]
pub fn collection_requirements(
    root: &Path,
    profile: &str,
    manifest: &str,
    base: Option<String>,
    candidate: String,
    initial: bool,
) -> Result<Value, String> {
    let req = Request {
        protocol: chrono_harness::units::PROTOCOL.into(),
        request_id: "collection-requirements".into(),
        host_root: root.into(),
        config_path: profile.into(),
        config_sha256: sha256(&fs::read(root.join(profile)).map_err(|e| e.to_string())?),
        base,
        candidate,
        initial,
        scope: Some(chrono_harness::units::Scope::Collect {
            manifest: manifest.into(),
        }),
        observations: object!({}),
    };
    let mut reader = Reader::legacy();
    let mut evidence = source_observations::GitEvidence::default();
    let i = inventory(
        &req,
        &mut reader,
        &mut evidence,
        &mut String::new(),
        false,
        true,
    )?;
    if !i.blocked.is_empty() {
        return Err(format!("blocked global obligations: {:?}", i.blocked));
    }
    let required = units::required(
        i.p.units
            .as_ref()
            .ok_or("collection requires registered units")?,
        &i.global_selected,
    );
    clean(&reader, root, &req.candidate, &i.p)?;
    Ok(
        object!({"required_units":required,"global_selected":i.global_selected,"selection_explanation":i.selection_explanation,"git_facts":evidence.finish(reader.observation())}),
    )
}

#[cfg(not(verus_keep_ghost))]
struct Profile {
    bytes: Vec<u8>,
    config: CheckConfig,
    policy: Policy,
}
#[cfg(not(verus_keep_ghost))]
impl Profile {
    fn load(req: &Request) -> Result<Self, String> {
        if req.protocol != units::scope_protocol(req) || req.request_id.is_empty() {
            return Err("invalid request protocol/identity".into());
        }
        relative_path(&req.config_path)?;
        let bytes = fs::read(req.host_root.join(&req.config_path)).map_err(|e| e.to_string())?;
        if sha256(&bytes) != req.config_sha256 {
            return Err("config does not match candidate/request".into());
        }
        let config: CheckConfig = decode(&bytes)?;
        // Only the candidate selects a report format. Fixed baseline policy is
        // input data for DELTA planning and never selects an old formatter.
        if config
            .policy
            .get("selection_evidence")
            .is_some_and(|v| v != "reachable-graph/v1")
        {
            return Err("unsupported selection_evidence representation".into());
        }
        let policy = policy(&config)?;
        Ok(Self {
            bytes,
            config,
            policy,
        })
    }
}
#[cfg(not(verus_keep_ghost))]
fn inventory(
    req: &Request,
    reader: &mut Reader,
    evidence: &mut source_observations::GitEvidence,
    scope: &mut String,
    admit_entry: bool,
    verify_checkout: bool,
) -> Result<Inventory, String> {
    inventory_profile(
        req,
        reader,
        evidence,
        scope,
        admit_entry,
        verify_checkout,
        Profile::load(req)?,
    )
}
#[cfg(not(verus_keep_ghost))]
fn inventory_profile(
    req: &Request,
    reader: &mut Reader,
    evidence: &mut source_observations::GitEvidence,
    scope: &mut String,
    admit_entry: bool,
    verify_checkout: bool,
    profile: Profile,
) -> Result<Inventory, String> {
    let root = &req.host_root;
    let Profile {
        bytes,
        config,
        policy: p,
    } = profile;
    *scope = config.schema.clone();
    if let Some(scope) = &req.scope {
        scope.report_path(&config)?;
    }
    full_oid(&req.candidate)?;
    if let Some(path) = &p.facts_config {
        *reader = Reader::for_config_observed(root, path).map_err(|failure| {
            evidence.opening = failure.observation;
            failure.message
        })?;
        if !reader.is_bound() {
            return Err("scoped v2 requires a bound full-v3 facts_config".into());
        }
        reader.verify_config(root, &req.candidate)?;
    }
    verify_oid(reader, root, &req.candidate)?;
    if at(reader, root, &req.candidate, &req.config_path)? != bytes {
        return Err("config does not match candidate/request".into());
    }
    if verify_checkout {
        clean(reader, root, &req.candidate, &p)?;
    }
    let candidate_tools = endpoint_tools(reader, root, &req.candidate, &p)?;
    let program =
        route_query::installed(root, &candidate_tools.tools, "chrono-judge-registration")?;
    let program = program.to_str().ok_or("non-UTF8 registration executable")?;
    let query = Query::scoped(root, &p)?;
    let previous;
    let old = if req.initial {
        if req.base.is_some() {
            return Err("initial cannot have base".into());
        }
        if !reader.parents(root, &req.candidate)?.is_empty() {
            return Err("initial requires parentless commit".into());
        }
        previous = "none".to_owned();
        None
    } else {
        let base = req.base.as_deref().ok_or("base required")?;
        verify_oid(reader, root, base)?;
        let base_tree = tree(reader, root, base)?;
        let op = if base_tree.contains_key(&req.config_path) {
            let c: CheckConfig = decode(&at(reader, root, base, &req.config_path)?)?;
            let op = policy(&c)?;
            previous = c.schema;
            if op.filemap != p.filemap || op.projects != p.projects {
                return Err("registry path migration unsupported".into());
            }
            op
        } else {
            if p.adoption_base.as_deref() != Some(base) {
                return Err("base profile absent; explicit exact adoption_base required".into());
            }
            previous = "none".to_owned();
            // Adoption projects the explicitly supplied bindings onto the real base registry;
            // it never fabricates a base tree or executes historical operations.
            let mut adopted = p.clone();
            adopted
                .required_inputs
                .retain(|path| base_tree.contains_key(path));
            adopted
        };
        let base_tools = endpoint_tools(reader, root, base, &op)?;
        Some(snapshot(
            reader,
            root,
            base,
            op,
            base_tools,
            &query,
            program,
            !base_tree.contains_key(&req.config_path),
        )?)
    };
    let new = snapshot(
        reader,
        root,
        &req.candidate,
        p.clone(),
        candidate_tools,
        &query,
        program,
        false,
    )?;
    let (paths, mut selected, mut selection_explanation, affected) =
        ci_impact(root, old.as_ref(), &new)?;
    let consumption = consumption::check_scoped(
        &(&*reader, root.as_path()),
        req.base.as_deref(),
        &req.candidate,
        &p.filemap,
        &p.projects,
        p.registration_config.as_deref(),
        &req.config_path,
        &paths,
        &affected,
    )?;
    selected.extend(consumption.required_tests.iter().cloned());
    selection_explanation["causal_consumption"] = object!(consumption);
    if new.registry["schema_version"] == 2 {
        project_pairs::check(
            &new.registry,
            &affected,
            new.tools.get("chrono-judge-projects"),
            std::env::var("PATH").map_err(|error| error.to_string())?,
            p.operation_timeout_seconds,
            p.operation_output_limit_bytes,
            |command, input| impact_query::run_bound(root, command.clone(), input),
        )?;
    }
    let mut blocked = vec![];
    let mut removed = BTreeMap::new();
    let mut retirement_context = object!({});
    let mut conversion = Value::Null;
    let mut reused = BTreeMap::new();
    let mut environment: BTreeMap<String, String> = std::env::vars().collect();
    environment.extend(p.environment.clone());
    #[cfg(unix)]
    environment.remove(chrono_harness::process_fds::ENV);
    let mut environment_policy = object!({"values":p.environment,"inherit":null});
    let mut declarations = object!(new.tools.values().cloned().collect::<Vec<_>>());
    if req.base.is_none() {
        if let Some(config_path) = &p.registration_config {
            let values = reader.registry_values(root, &req.candidate, config_path)?;
            let identity = chrono_harness::facts::registry_identity(&values, config_path)?;
            let cfg = &values[&identity.effective_path];
            if cfg["schema_version"] == 4 {
                chrono_judge_registration::Registrations::load(&values, config_path)?;
                let c = chrono_harness::prepared::declaration(cfg)?;
                let mut argv = c.argv;
                if let Some(scope) = &req.scope {
                    match scope {
                        chrono_harness::units::Scope::Unit { unit } => {
                            argv.extend(["--unit".into(), unit.clone()])
                        }
                        chrono_harness::units::Scope::Collect { .. } => {
                            argv.push("--collect".into())
                        }
                    }
                }
                if admit_entry {
                    chrono_harness::resolve_invocation(
                        root,
                        &req.observations["entry"],
                        &argv,
                        &object!({}),
                    )?;
                }
                if admit_entry {
                    chrono_harness::prepared::validate_binding(
                        root,
                        cfg,
                        &req.config_path,
                        None,
                        &req.candidate,
                        req.initial,
                        &req.scope,
                        None,
                        &req.observations["preparation"],
                    )?;
                }
                declarations = cfg["tools"].clone();
                environment.clear();
                for key in cfg["environment"]["inherit"]
                    .as_array()
                    .ok_or("environment inherit missing")?
                {
                    let key = key.as_str().ok_or("environment key")?;
                    if let Ok(v) = std::env::var(key) {
                        environment.insert(key.into(), v);
                    }
                }
                for (key, v) in cfg["environment"]["values"]
                    .as_object()
                    .ok_or("environment values missing")?
                {
                    environment.insert(key.clone(), v.as_str().ok_or("environment value")?.into());
                }
                environment.extend(p.environment.clone());
                environment_policy = cfg["environment"].clone();
                for (key, v) in &p.environment {
                    environment_policy["values"][key] = object!(v);
                }
            }
        }
    }
    if let (Some(config_path), Some(base)) = (&p.registration_config, &req.base) {
        let a = reader.registry_values(root, base, config_path)?;
        let b = reader.registry_values(root, &req.candidate, config_path)?;
        let _a_identity = chrono_harness::facts::registry_identity(&a, config_path)?;
        let b_identity = chrono_harness::facts::registry_identity(&b, config_path)?;
        let b_config = b
            .get(&b_identity.effective_path)
            .ok_or("missing effective candidate registration config")?;
        if b_config["schema_version"] == 4 {
            chrono_judge_registration::Registrations::load(&b, config_path)?;
        }
        let mut template: Vec<String> =
            serde_json::from_value(b_config["canonical_check"]["argv"].clone())
                .map_err(|e| e.to_string())?;
        if let Some(scope) = &req.scope {
            if b_config["schema_version"] == 4 {
                match scope {
                    chrono_harness::units::Scope::Unit { unit } => {
                        template.extend(["--unit".into(), unit.clone()])
                    }
                    chrono_harness::units::Scope::Collect { .. } => {
                        template.push("--collect".into())
                    }
                }
            } else {
                template.extend(scope.argv());
            }
        }
        if admit_entry {
            chrono_harness::resolve_invocation(
                root,
                &req.observations["entry"],
                &template,
                &object!({"base":req.base,"candidate":req.candidate}),
            )?;
        }
        if admit_entry && b_config["schema_version"] == 4 {
            chrono_harness::prepared::validate_binding(
                root,
                b_config,
                &req.config_path,
                req.base.as_deref(),
                &req.candidate,
                req.initial,
                &req.scope,
                None,
                &req.observations["preparation"],
            )?;
        }
        environment.clear();
        for key in b_config["environment"]["inherit"]
            .as_array()
            .ok_or("environment inherit missing")?
        {
            let key = key.as_str().ok_or("environment key")?;
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.into(), value);
            }
        }
        for (key, value) in b_config["environment"]["values"]
            .as_object()
            .ok_or("environment values missing")?
        {
            environment.insert(
                key.clone(),
                value.as_str().ok_or("environment value")?.into(),
            );
        }
        environment.extend(p.environment.clone());
        environment_policy = b_config["environment"].clone();
        for (key, value) in &p.environment {
            environment_policy["values"][key] = object!(value);
        }
        let (old_registration, r, view) = if !verify_checkout {
            chrono_judge_registration::interpret_scheduling_with_reader(
                reader,
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
            )?
        } else if reader.is_bound() {
            chrono_judge_registration::interpret_with_reader(
                reader,
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
                &environment,
            )?
        } else {
            chrono_judge_registration::interpret(
                root,
                base,
                &req.candidate,
                config_path,
                a,
                b,
                &environment,
            )?
        };
        declarations = r.config()["tools"].clone();
        reused = chrono_judge_registration::reused_tools(&view)?;
        conversion = view["conversion"].clone();
        for test in selected.clone() {
            if new.plans.contains_key(&test) {
                continue;
            }
            let replacement =
                chrono_judge_registration::validate_test_retirement(&old_registration, &r, &test)?;
            let cost_rows: Vec<_> = old_registration.filemap()["test_costs"]
                .as_array()
                .ok_or("E_RETIREMENT: old test costs missing")?
                .iter()
                .filter(|row| row["test"].as_str() == test.strip_prefix("test:"))
                .collect();
            if cost_rows.len() != 1 {
                return Err(format!(
                    "E_RETIREMENT: old test cost missing/ambiguous: {test}"
                ));
            }
            let cost = cost_rows[0]["cost"]
                .as_str()
                .ok_or("old cost reference missing")?;
            let model = old_registration.filemap()["cost_models"]
                .get(cost)
                .ok_or("E_RETIREMENT: old cost model missing")?;
            retirement_context[&test] = object!({
                "definitions":old_registration.node_data()[&test].definitions.iter()
                    .map(|d| object!({"identity":d.identity,"value":d.value})).collect::<Vec<_>>(),
                "cost":{"reference":cost,"value":model}
            });
            removed.insert(test.clone(), replacement.clone());
            selected.remove(&test);
            if let Some(replacement) = replacement {
                selected.insert(replacement);
            }
        }
    }
    for test in &selected {
        if !new.plans.contains_key(test) {
            blocked.push(format!("affected binding removed: {test}"));
        }
    }
    let methods: BTreeMap<_, _> = new
        .operations
        .iter()
        .map(|(id, (owner, tool, argv, output_contract))| {
            (
                id.clone(),
                vec![chrono_judge_registration::execution::Method {
                    owner: format!("project:{owner}"),
                    operation: id.clone(),
                    tool: tool.clone(),
                    argv: argv.clone(),
                    output_contract: output_contract.clone(),
                }],
            )
        })
        .collect();
    let global_selected = selected.clone();
    if verified_collection::can_validate_selected_plan(&blocked) {
        // Validate the complete obligation graph before selecting any unit.
        let program = route_query::installed(root, &new.tools, "chrono-judge-routes")?;
        let operations = route_query::Graph {
            selected: &global_selected,
            plans: &new.plans,
            methods: &methods,
            execute: &new.execute,
        }
        .ordered(root, &program, None)?
        .operations;
        let query = Query::scoped(root, &p)?;
        route_query::source_plan(
            &query,
            program.to_str().ok_or("non-UTF8 routes executable")?,
            p.facts_config.as_deref().unwrap_or(&req.config_path),
            req.base
                .as_deref()
                .zip(old.as_ref())
                .map(|(oid, snapshot)| route_query::SourceEndpoint::scoped(oid, snapshot)),
            route_query::SourceEndpoint::scoped(&req.candidate, &new),
            &affected,
            &methods,
            &operations,
            &reader.observation(),
            evidence,
        )?;
    }
    Ok(Inventory {
        config,
        p,
        new,
        previous,
        paths,
        selected,
        global_selected,
        selection_explanation,
        blocked,
        removed,
        retirement_context,
        conversion,
        reused,
        environment,
        environment_policy,
        declarations,
        methods,
    })
}
#[cfg(not(verus_keep_ghost))]
fn report_path(req: &Request, config: &CheckConfig) -> Result<String, String> {
    Ok(req
        .scope
        .as_ref()
        .map(|scope| scope.report_path(config))
        .transpose()?
        .unwrap_or_else(|| config.report_path.clone()))
}

#[cfg(not(verus_keep_ghost))]
fn scoped_binding(req: &Request) -> Value {
    object!({"base":req.base,"candidate":req.candidate,"config":req.config_sha256,
        "run":req.request_id,"entry":req.observations["entry"]})
}

#[cfg(not(verus_keep_ghost))]
fn prepare_plan(
    req: &Request,
    config: &CheckConfig,
    selected: &BTreeSet<String>,
    new: &Snapshot,
    methods: &BTreeMap<String, Vec<chrono_judge_registration::execution::Method>>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, chrono_harness::observation::Tool>,
) -> Result<route_wire::Execution, String> {
    tracing::info_span!("ci.phase", phase = "prepare").in_scope(|| {
        tracing::info!(
            event_id = "ci.phase_started",
            "preparing registered operation plan and tools"
        );
        route_query::Graph {
            selected,
            plans: &new.plans,
            methods,
            execute: &new.execute,
        }
        .prepare(
            &req.host_root,
            new,
            scoped_binding(req),
            &report_path(req, config)?,
            declarations,
            &environment,
            reused,
        )
    })
}

#[cfg(not(verus_keep_ghost))]
fn execute_plan(
    req: &Request,
    plan: &route_wire::Execution,
    tools: &BTreeMap<String, Value>,
    policy: &Policy,
    errors: &mut Vec<String>,
) -> Result<project_execution::Results, String> {
    let span = tracing::info_span!(
        "ci.execution",
        request_id = req.request_id,
        candidate = req.candidate,
        config_sha256 = req.config_sha256,
        phase = "execute"
    );
    let _entered = span.enter();
    tracing::info!(
        event_id = "ci.phase_started",
        "executing registered operations"
    );
    project_execution::execute(
        &req.host_root,
        plan,
        tools,
        policy.operation_timeout_seconds,
        policy.operation_output_limit_bytes,
        errors,
    )
}

#[cfg(not(verus_keep_ghost))]
fn evaluate(
    req: &Request,
    reader: &mut Reader,
    evidence: &mut source_observations::GitEvidence,
    scope: &mut String,
    profile: Profile,
    project_errors: &mut Vec<String>,
) -> Result<Evaluation, String> {
    let Inventory {
        config,
        p,
        new,
        previous,
        paths,
        mut selected,
        global_selected,
        selection_explanation,
        blocked,
        removed,
        retirement_context,
        conversion,
        reused,
        environment,
        environment_policy,
        declarations,
        methods,
    } = tracing::info_span!("ci.phase", phase = "inventory").in_scope(|| {
        tracing::info!(
            event_id = "ci.phase_started",
            "validating inventory and selected inputs"
        );
        inventory_profile(req, reader, evidence, scope, true, true, profile)
    })?;
    let root = &req.host_root;
    if let Some(chrono_harness::units::Scope::Collect { manifest }) = &req.scope {
        if !blocked.is_empty() {
            return Err(format!("blocked global obligations: {blocked:?}"));
        }
        let mut response = collect::collect(
            req,
            reader,
            manifest,
            &config,
            &new,
            &global_selected,
            &methods,
            &declarations,
            &environment_policy,
        )?;
        clean(reader, root, &req.candidate, &p)?;
        response.evidence["changed_paths"] = object!(paths);
        response.evidence["selection_explanation"] = selection_explanation;
        response.evidence["removed"] = object!(removed);
        response.evidence["retirement_context"] = retirement_context;
        return Ok(response);
    }
    if let Some(chrono_harness::units::Scope::Unit { unit }) = &req.scope {
        selected = units::selection(
            p.units.as_ref().ok_or("unit profile missing assignments")?,
            &global_selected,
            unit,
        )?;
    }
    let plan = if blocked.is_empty() {
        Some(prepare_plan(
            req,
            &config,
            &selected,
            &new,
            &methods,
            &declarations,
            environment,
            &reused,
        )?)
    } else {
        None
    };
    let operations: Vec<_> = plan
        .as_ref()
        .into_iter()
        .flat_map(|p| p.operations.iter().map(|o| o.method.operation.clone()))
        .collect();
    let mut results = vec![CheckResult {
        id: "ci.inventory".into(),
        status: Status::Passed,
        cause: "fixed candidate and registered endpoint inventory validated".into(),
        exit_code: None,
    }];
    let mut executed = vec![];
    let mut operation_state = None;
    for reason in &blocked {
        results.push(CheckResult {
            id: format!("ci.blocked.{}", results.len()),
            status: Status::Blocked,
            cause: reason.clone(),
            exit_code: None,
        })
    }
    if let Some(plan) = &plan {
        let outcome = execute_plan(req, plan, &new.tools, &p, project_errors)?;
        operation_state = outcome.operation_state.clone();
        let source_io = source_binding::CandidateIo {
            reader,
            request: req,
            tree: &new.tree,
        };
        let proof_invocations = source_binding::invocations(&plan.operations);
        for result in outcome.executed.iter().chain(&outcome.blocked) {
            results.push(verified_collection::operation_result_with_sources(
                result.operation.as_str(),
                result.status.as_str(),
                &result.error,
                result.receipt.as_ref().map(|r| r.process.exit_code),
                source_binding::bind_output(
                    result.status.as_str(),
                    result.operation.as_str(),
                    &proof_invocations,
                    result.receipt.as_ref(),
                    &new.source_registry,
                    &source_io,
                ),
            ));
            if let Some(receipt) = &result.receipt {
                let mut original = serde_json::to_value(receipt).map_err(|e| e.to_string())?;
                original["process"] = chrono_harness::full::compact_process(&original["process"])?;
                executed.push(object!({"operation":result.operation,"receipt":original}));
            }
        }
        for (old, replacement) in removed
            .iter()
            .filter_map(|(old, to)| to.as_ref().map(|to| (old, to)))
        {
            if selected.contains(replacement)
                && outcome.tests.get(replacement).map(String::as_str) != Some("passed")
            {
                results.push(CheckResult {
                    id: format!("replacement:{old}"),
                    status: Status::Failed,
                    cause: "required replacement did not execute successfully".into(),
                    exit_code: None,
                });
            }
        }
    }
    let not_required: Vec<_> = new
        .policy
        .bindings
        .keys()
        .filter(|t| !global_selected.contains(*t))
        .cloned()
        .collect();
    for t in &not_required {
        results.push(CheckResult {
            id: t.clone(),
            status: Status::NotRequired,
            cause: "outside explicit DELTA execution closure".into(),
            exit_code: None,
        });
    }
    if let Err(e) = tracing::info_span!("ci.phase", phase = "post_snapshot").in_scope(|| {
        tracing::info!(
            event_id = "ci.phase_started",
            "validating post-execution snapshot"
        );
        clean(reader, root, &req.candidate, &p)
    }) {
        results.push(CheckResult {
            id: "ci.post-snapshot".into(),
            status: Status::Failed,
            cause: e,
            exit_code: None,
        })
    }
    let mut response = Evaluation {
        protocol: units::scope_protocol(req).into(),
        request_id: req.request_id.clone(),
        status: verified_collection::response_status(&results),
        results,
        evidence: object!({"scope":config.schema,"mode":if req.initial{"initial-inventory"}else{"delta"},"base":req.base,"candidate":req.candidate,"previous_enforcement":previous,"changed_paths":paths,"selection_explanation":selection_explanation,"selected":selected,"operations":operations,"plan":plan,"conversion":conversion,"removed":removed,"retirement_context":retirement_context,"execution_evidence_schema":"chrono-ci-executions/v2","executed":executed,"not_required":not_required,"blocked":blocked,"input_closure":"incomplete: ambient toolchain, SDK, environment and external inputs are not fully enumerated","parity":"unestablished"}),
    };
    units::decorate(req, &new, &global_selected, &mut response.evidence);
    response.evidence["operation_state"] = object!(operation_state);
    Ok(response)
}
