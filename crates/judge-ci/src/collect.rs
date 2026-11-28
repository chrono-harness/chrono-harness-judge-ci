//! Offline verification of explicit unit reports. Business operations are never executed here.
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::route_wire::Method;
#[cfg(not(verus_keep_ghost))]
use crate::route_wire::{Execution, Receipt};
#[cfg(not(verus_keep_ghost))]
use crate::{
    originals::{self, ArtifactTransport, Manifest, Original, PreparedCheck, ReportReference},
    route_wire::ProcessResult,
    wire::Scope,
};
#[cfg(not(verus_keep_ghost))]
use std::io::Read;

#[cfg(verus_keep_ghost)]
use super::{
    result_types::{CheckResult, Status},
    unit_protocol::SCOPED_PROTOCOL,
    verified_text::equal_text,
};
#[cfg(not(verus_keep_ghost))]
use crate::{units::SCOPED_PROTOCOL, verified_collection::verified_text::equal_text};
use vstd::prelude::*;

verus! {
/// Choose the collector's validation contract from the actual preparation and report label.
pub fn identity_environment(label: Option<&str>, prepared: bool) -> (mode: Option<bool>)
    ensures mode == match label {
        None => Some(false),
        Some(value) => if !prepared && value@ == "sha256"@ { Some(true) } else { None },
    },
{
    match label {
        None => Some(false),
        Some(value) if !prepared && equal_text(value,"sha256") => Some(true),
        Some(_) => None,
    }
}

/// Bind the original reply to the expected scoped protocol, request and evidence.
pub fn response_header(protocol: &str, reported: &str, expected: &str, count: usize, evidence: bool)
    -> (valid: bool)
    ensures valid == (protocol@ == "chrono-ci-judge/v2"@ && reported@ == expected@ && count > 0 && evidence),
{
    equal_text(protocol, SCOPED_PROTOCOL) && equal_text(reported, expected) && count > 0 && evidence
}

/// The set insertion is performed once by the reader, in original row order.
pub fn row_error(row: &CheckResult, new_id: bool) -> (error: Option<&'static str>)
    ensures error == if row.id@ == ""@ || row.cause@ == ""@ || !new_id {
        Some("invalid/duplicate judge result")
    } else { match (row.status, row.exit_code) {
        (Status::Passed, Some(code)) if code != 0 => Some("passed result has failing exit"),
        (Status::Failed, Some(0)) => Some("failed result has successful exit"),
        (Status::NotRequired, Some(_)) => Some("not-required result must have null exit_code"),
        _ => None,
    } },
{
    if !new_id || equal_text(row.id.as_str(), "") || equal_text(row.cause.as_str(), "") {
        return Some("invalid/duplicate judge result");
    }
    match (row.status, row.exit_code) {
        (Status::Passed, Some(code)) if code != 0 => Some("passed result has failing exit"),
        (Status::Failed, Some(0)) => Some("failed result has successful exit"),
        (Status::NotRequired, Some(_)) => Some("not-required result must have null exit_code"),
        _ => None,
    }
}
}

/// Decode the response recorded by a unit and bind it to the request and process exit.
#[cfg(not(verus_keep_ghost))]
pub fn unit_response(value: &Value, request_id: &str, exit: i32) -> Result<Response, String> {
    let response = Response::deserialize(value).map_err(|error| error.to_string())?;
    if !response_header(
        &response.protocol,
        &response.request_id,
        request_id,
        response.results.len(),
        response.evidence.as_object().is_some_and(|e| !e.is_empty()),
    ) {
        return Err("empty or mismatched judge response".into());
    }
    let mut ids = BTreeSet::new();
    for row in &response.results {
        if let Some(error) = row_error(row, ids.insert(&row.id)) {
            return Err(error.into());
        }
    }
    if !verified_collection::unit_outcome_matches(&response.results, response.status, exit) {
        return Err("judge exit/status/results disagreement".into());
    }
    Ok(response)
}

#[cfg(not(verus_keep_ghost))]
fn read(
    root: &Path,
    path: &str,
    policy: &Policy,
    limit_name: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    units::artifact_path(path)?;
    if !policy.artifacts.iter().any(|a| path.starts_with(a)) {
        return Err("collection input is not a declared artifact".into());
    }
    let storage = crate::storage::Storage::open(root)?;
    storage.call::<()>(object!({"operation":"retain-read","path":path}))?;
    let resolved: std::path::PathBuf =
        storage.call(object!({"operation":"checked-path","path":path}))?;
    let meta = fs::symlink_metadata(&resolved).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(format!("collection input must be a regular file: {path}"));
    }
    let overflow = |observed| {
        format!("collection input exceeds {limit_name}: {path} ({observed} > {limit} bytes)")
    };
    if !verified_collection::original_size_allowed(meta.len(), limit) {
        return Err(overflow(meta.len()));
    }
    let mut bytes = Vec::new();
    fs::File::open(resolved)
        .map_err(|e| e.to_string())?
        .take(verified_collection::original_read_cap(limit))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    verified_collection::bounded_original(bytes, limit).map_err(|size| overflow(size as u64))
}

#[cfg(not(verus_keep_ghost))]
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(not(verus_keep_ghost))]
fn process(p: &ProcessResult, environment_matches: bool) -> Result<(), String> {
    if !digest(&p.sha256)
        || p.stdout_sha256 != sha256(&p.stdout_bytes)
        || p.stderr_sha256 != sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
        || !digest(&p.environment_digest)
        || !environment_matches
    {
        return Err("unit original process bytes or identity mismatch".into());
    }
    Ok(())
}

/// Typed report/preparation/receipt/stream acquisition shared by native gather
/// and the canonical collector. Supplying these bytes is not report acceptance.
#[cfg(not(verus_keep_ghost))]
pub struct UnitOriginals {
    pub report: Value,
    pub request: Request,
    pub producer: ProcessResult,
    pub retained_report_path: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
impl UnitOriginals {
    /// Verify the recorded command and its represented environment on the
    /// producer's host, including a bare program selected from its search path.
    pub fn validate_producer(&self, config: &CheckConfig) -> Result<(), String> {
        let producer = &self.producer;
        let original = &self.request;
        let wire = &self.report["judge"];
        let program = &config.judge.program;
        let prepared = original.observations["preparation"].is_object();
        let label = wire
            .get("environment_representation")
            .map(|value| {
                value
                    .as_str()
                    .ok_or("unit environment representation must be a string")
            })
            .transpose()?;
        let path_matches = match identity_environment(label, prepared) {
            Some(false) => {
                if wire.get("unit_environment_digest").is_some()
                    || wire.get("program_candidates_sha256").is_some()
                {
                    return Err("unit environment representation missing".into());
                }
                process(
                    producer,
                    producer.environment_digest == crate::codec::digest(&producer.environment)?,
                )?;
                let search = producer
                    .environment
                    .get("PATH")
                    .map(String::as_str)
                    .unwrap_or("");
                crate::files::program_candidates(&original.host_root, program, Some(search))?
                    .contains(&producer.executable)
            }
            Some(true) => {
                let commitment = crate::codec::digest(&(
                    &producer.environment_digest,
                    &producer.environment,
                    &wire["program_candidates_sha256"],
                ))?;
                process(producer, wire["unit_environment_digest"] == commitment)?;
                if !producer.environment.values().all(|value| digest(value)) {
                    return Err("invalid unit environment identity".into());
                }
                if program.contains('/') {
                    wire.get("program_candidates_sha256").is_none()
                        && producer.executable == original.host_root.join(program)
                } else {
                    let candidates: Vec<String> =
                        serde_json::from_value(wire["program_candidates_sha256"].clone())
                            .map_err(|_| "unit program candidate identities missing")?;
                    producer.environment.contains_key("PATH")
                        && producer.executable.is_absolute()
                        && producer.executable.file_name() == Some(std::ffi::OsStr::new(program))
                        && candidates.iter().all(|value| digest(value))
                        && candidates.contains(&crate::codec::digest(&producer.executable)?)
                }
            }
            None => return Err("unsupported unit environment representation".into()),
        };
        let expected_argv = std::iter::once(producer.executable.to_string_lossy().into_owned())
            .chain(config.judge.args.iter().cloned());
        if !path_matches || !expected_argv.eq(producer.argv.iter().cloned()) {
            return Err("unit judge command differs from profile".into());
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub fn unit_originals(
    root: &Path,
    config: &CheckConfig,
    unit: &str,
    bytes: &[u8],
    transport: Option<&ArtifactTransport>,
    mut read: impl FnMut(&Original, u64) -> Result<Vec<u8>, String>,
) -> Result<UnitOriginals, String> {
    let mut read = |o: &Original, limit: u64| -> Result<Vec<u8>, String> {
        originals::original_path(root, o, transport)?;
        let raw = read(o, limit)?;
        if !verified_collection::original_size_allowed(raw.len() as u64, limit) {
            return Err(format!("unit original digest/limit mismatch: {}", o.path));
        }
        Ok(raw)
    };
    let published = json(bytes)?;
    let retained_streams = units::reference_publication(root, config)?;
    let report_schema = if retained_streams {
        "chrono-check-report/v2"
    } else {
        "chrono-check-report/v1"
    };
    let limits: CollectionLimits = config
        .policy
        .get("collection_limits")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let mut retained_report_path = None;
    let (report, mut remaining) = if retained_streams {
        let reference: ReportReference =
            serde_json::from_value(published).map_err(|e| e.to_string())?;
        if reference.schema != "chrono-check-reference/v1" {
            return Err("unit report reference schema mismatch".into());
        }
        let path = originals::original_path(root, &reference.original, transport)?;
        let raw = read(&reference.original, limits.report_bytes)?;
        let admitted =
            requirements::admitted_report(&raw, &sha256(&raw), &reference.original.sha256)
                .ok_or_else(|| format!("original report digest mismatch: {unit} ({path})"))?;
        let original = json(admitted)?;
        if original["schema"] != report_schema {
            return Err(format!("unit transport failed: {}", unit));
        }
        if original["retained_report"] != reference.original.path {
            return Err(format!("original report path mismatch: {}", unit));
        }
        retained_report_path = Some(path);
        (original, limits.report_bytes - raw.len() as u64)
    } else {
        (published, 0)
    };
    if report["schema"] != report_schema || report.get("transport_failure").is_some() {
        return Err(format!("unit transport failed: {}", unit));
    }
    let original: Request =
        serde_json::from_value(report["request"].clone()).map_err(|e| e.to_string())?;
    if let Some(binding) = original
        .observations
        .get("preparation")
        .filter(|b| b.is_object())
    {
        originals::with_originals::<()>(
            root,
            object!({"operation":"validate-portable-binding","binding":binding,"transport":transport}),
            |o| read(o, 64 * 1024 * 1024),
        )?;
        let prepared: PreparedCheck =
            serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
        if !crate::wire::scope_matches(&prepared.scope, &original.scope) {
            return Err("original preparation scope differs from collected unit".into());
        }
    } else if transport.is_some() {
        return Err("native short report missing original preparation".into());
    }
    let producer: ProcessResult = if retained_streams {
        if report.get("response").is_some() {
            return Err("retained-stream report cannot contain a separate response".into());
        }
        originals::with_originals(
            root,
            object!({"operation":"restore-scoped-process","process":report["judge"],"limit":remaining}),
            |original| {
                let raw = read(original, remaining)?;
                remaining -= raw.len() as u64;
                Ok(raw)
            },
        )?
    } else {
        serde_json::from_value(report["judge"].clone()).map_err(|e| e.to_string())?
    };
    Ok(UnitOriginals {
        report,
        request: original,
        producer,
        retained_report_path,
    })
}

#[cfg(not(verus_keep_ghost))]
fn expanded_receipt(
    session: &mut crate::runtime_session::Session,
    row: &Value,
) -> Result<Receipt, String> {
    if row.get("process").is_some() {
        return Err("unit execution has duplicate process evidence".into());
    }
    let original = &row["receipt"];
    if original["process"].get("encoding").is_none() {
        return Err("unit execution process encoding required".into());
    }
    session.call(object!({"operation":"expand-record","record":original}))
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn collect(
    req: &Request,
    reader: &Reader,
    manifest: &str,
    config: &CheckConfig,
    snapshot: &Snapshot,
    selected: &BTreeSet<String>,
    methods: &BTreeMap<String, Vec<Method>>,
    declarations: &Value,
    environment_policy: &Value,
) -> Result<Response, String> {
    crate::storage::Storage::with(&req.host_root, || {
        collect_checked(
            req,
            reader,
            manifest,
            config,
            snapshot,
            selected,
            methods,
            declarations,
            environment_policy,
        )
    })
}

#[cfg(not(verus_keep_ghost))]
fn collect_checked(
    req: &Request,
    reader: &Reader,
    manifest: &str,
    config: &CheckConfig,
    snapshot: &Snapshot,
    selected: &BTreeSet<String>,
    methods: &BTreeMap<String, Vec<Method>>,
    declarations: &Value,
    environment_policy: &Value,
) -> Result<Response, String> {
    let p = &snapshot.policy;
    let units = p
        .units
        .as_ref()
        .ok_or("collection requires registered units")?;
    let required = units::required(&req.host_root, units, selected)?;
    let limits = p.collection_limits.clone().unwrap_or_default();
    let manifest_bytes = read(
        &req.host_root,
        manifest,
        p,
        "manifest_bytes",
        limits.manifest_bytes,
    )?;
    let inputs: Manifest = decode(&manifest_bytes)?;
    if inputs.schema != "chrono-ci-collection/v1" {
        return Err("unsupported collection manifest".into());
    }
    let mut supplied = BTreeSet::new();
    let mut paths = BTreeSet::from([manifest.to_owned(), config.report_path.clone()]);
    for input in &inputs.reports {
        if !units.contains_key(&input.unit)
            || !supplied.insert(input.unit.clone())
            || !paths.insert(input.path.clone())
            || ![&input.sha256, &input.runner_sha256, &input.judge_sha256]
                .iter()
                .all(|s| digest(s))
        {
            return Err("unknown/duplicate unit report, colliding path or invalid identity".into());
        }
    }
    if !required.iter().all(|u| supplied.contains(u)) {
        return Err("missing required CI unit report".into());
    }
    // Nonselected units may report successful inventory too; unknown or duplicate reports never pass.
    let not_required: Vec<_> = snapshot
        .plans
        .keys()
        .filter(|t| !selected.contains(*t))
        .cloned()
        .collect();
    let mut results = vec![CheckResult {
        id: "ci.inventory".into(),
        status: Status::Passed,
        cause: "fixed endpoints, complete global selection and explicit unit assignments validated"
            .into(),
        exit_code: None,
    }];
    let mut verified = vec![];
    let mut retained_originals = Vec::new();
    let retained_streams = units::reference_publication(&req.host_root, config)?;
    for input in &inputs.reports {
        let bytes = read(
            &req.host_root,
            &input.path,
            p,
            "report_bytes",
            limits.report_bytes,
        )?;
        let admitted = requirements::admitted_report(&bytes, &sha256(&bytes), &input.sha256)
            .ok_or_else(|| format!("unit report digest mismatch: {}", input.unit))?;
        if input
            .artifacts
            .as_ref()
            .is_some_and(|t| !p.artifacts.iter().any(|a| t.directory.starts_with(a)))
        {
            return Err("original evidence transport is not a declared artifact".into());
        }
        let originals = unit_originals(
            &req.host_root,
            config,
            &input.unit,
            admitted,
            input.artifacts.as_ref(),
            |o, limit| {
                let path = originals::original_path(&req.host_root, o, input.artifacts.as_ref())?;
                read(
                    &req.host_root,
                    &path,
                    p,
                    "report_bytes (original/stream budget)",
                    limit,
                )
            },
        )?;
        originals.validate_producer(config)?;
        retained_originals.extend(originals.retained_report_path);
        let report = originals.report;
        let original = originals.request;
        let producer = originals.producer;
        if let Some(binding) = original
            .observations
            .get("preparation")
            .filter(|b| b.is_object())
        {
            let prepared: PreparedCheck =
                serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
            if prepared.base != req.base
                || prepared.candidate != req.candidate
                || prepared.profile != req.config_path
                || !crate::wire::scope_matches(&prepared.scope, &original.scope)
            {
                return Err("original preparation differs from collected unit".into());
            }
        }
        let decoded = json(&producer.stdout_bytes)?;
        let response_value = if retained_streams {
            &decoded
        } else {
            &report["response"]
        };
        let response = unit_response(response_value, &original.request_id, producer.exit_code)?;
        if original.protocol != units::SCOPED_PROTOCOL
            || !crate::wire::scope_matches(
                &original.scope,
                &Some(Scope::Unit {
                    unit: input.unit.clone(),
                }),
            )
            || original.base != req.base
            || original.candidate != req.candidate
            || original.initial != req.initial
            || original.config_path != req.config_path
            || original.config_sha256 != req.config_sha256
            || report["runner"]["sha256"] != input.runner_sha256
            || producer.sha256 != input.judge_sha256
            || producer.stdin_sha256
                != sha256(&serde_json::to_vec(&original).map_err(|e| e.to_string())?)
            || &decoded != response_value
            || producer.cwd != original.host_root
            || producer.failure.is_some()
        {
            return Err(format!(
                "unit endpoints/config/scope/executable/original response mismatch: {}",
                input.unit
            ));
        }
        if response.status != Status::Passed {
            let reference = format!("original report {} ({})", input.path, input.sha256);
            let cause = match verified_collection::first_failure(&response.results) {
                Some(index) => {
                    let row = &response.results[index];
                    format!("{} | {} ({reference})", row.id, row.cause)
                }
                None => format!("unit failed; {reference}"),
            };
            results.push(CheckResult {
                id: format!("ci.unit.{}", input.unit),
                status: Status::Failed,
                cause,
                exit_code: Some(producer.exit_code),
            });
            verified.push(object!({"unit":input.unit,"path":input.path,"sha256":input.sha256,"status":"failed-original-retained"}));
            continue;
        }
        let unit_results = response.results;
        let owned = units::selection(&req.host_root, units, selected, &input.unit)?;
        let e = &response.evidence;
        if e["scope"] != config.schema
            || e["base"] != object!(req.base)
            || e["candidate"] != req.candidate
            || e["execution_scope"] != object!(original.scope)
            || e["selected"] != object!(owned)
            || e["global_selected"] != object!(selected)
            || e["required_units"] != object!(required)
            || e["assigned_elsewhere"] != object!(selected.difference(&owned).collect::<Vec<_>>())
            || e["not_required"] != object!(not_required)
            || e["blocked"] != object!([])
        {
            return Err("unit obligation partition differs from current global plan".into());
        }
        let plan: Execution =
            serde_json::from_value(e["plan"].clone()).map_err(|e| e.to_string())?;
        let routes =
            super::route_query::installed(&req.host_root, &snapshot.tools, "chrono-judge-routes")?;
        super::route_query::check_scoped_plan(
            &req.host_root,
            &routes,
            &plan,
            &super::scoped_binding(&original),
            &super::report_path(&req.host_root, &original, config)?,
        )?;
        if plan.root != original.host_root {
            return Err("unit plan binding mismatch".into());
        }
        let literals = environment_policy["values"]
            .as_object()
            .ok_or("environment literals missing")?;
        if literals.iter().any(|(key, value)| {
            plan.environment.get(key).map(|s| object!(s)) != Some(value.clone())
        }) {
            return Err("unit environment differs from registered literal values".into());
        }
        if let Some(inherit) = environment_policy["inherit"].as_array() {
            let allowed: BTreeSet<_> = inherit
                .iter()
                .filter_map(Value::as_str)
                .chain(literals.keys().map(String::as_str))
                .collect();
            if plan
                .environment
                .keys()
                .any(|key| !allowed.contains(key.as_str()))
            {
                return Err("unit environment contains unregistered inherited input".into());
            }
        }
        let super::route_query::Ordered {
            selected: chosen,
            operations,
        } = super::route_query::Graph {
            selected: &owned,
            plans: &snapshot.plans,
            methods,
            execute: &snapshot.execute,
        }
        .ordered(&req.host_root, &routes, Some(&plan.binding))?;
        if plan.scheduling != snapshot.scheduling
            || object!(chosen) != object!(plan.selected)
            || object!(operations) != object!(plan.operations)
        {
            return Err(
                "unit execution plan differs from registered operations/order/bounds".into(),
            );
        }
        let tool_ids: BTreeSet<_> = operations.iter().map(|o| o.method.tool.clone()).collect();
        if plan.tools.keys().cloned().collect::<BTreeSet<_>>() != tool_ids {
            return Err("unit tool set differs from plan".into());
        }
        for (id, tool) in &plan.tools {
            let definitions: Vec<_> = declarations
                .as_array()
                .ok_or("tool declarations missing")?
                .iter()
                .filter(|v| v["id"] == *id)
                .collect();
            if definitions.len() != 1 {
                return Err("unit tool declaration missing/ambiguous".into());
            }
            let declaration = definitions[0];
            process(
                &tool.version,
                tool.version.environment_digest == crate::codec::digest(&tool.version.environment)?,
            )?;
            let mut argv = vec![tool.path.to_string_lossy().into_owned()];
            argv.extend(strings(&declaration["version_argv"])?);
            if tool.program != declaration["program"].as_str().ok_or("tool program")?
                || tool.version.sha256 != tool.sha256
                || tool.version.executable != tool.path
                || !tool.path.is_absolute()
                || tool.version.argv != argv
                || tool.version.cwd != plan.root
                || tool.version.environment != plan.environment
                || tool.version.stdin_sha256 != sha256(&[])
                || declaration["expected_version"]
                    .as_str()
                    .is_some_and(|expected| {
                        tool.version.exit_code != 0
                            || tool.version.failure.is_some()
                            || tool.version.stdout.trim_end() != expected
                    })
            {
                return Err("unit tool version/binding mismatch".into());
            }
        }
        let expected_ops: Vec<_> = operations
            .iter()
            .map(|o| o.method.operation.clone())
            .collect();
        let expected_ids = verified_collection::borrow_texts(&expected_ops);
        let binding = source_binding::Binding::prepare(
            reader,
            &req.host_root,
            &operations,
            &snapshot.filemap,
        )?;
        let not_required_ids = verified_collection::borrow_texts(&not_required);
        let executed = e["executed"].as_array().ok_or("unit executions missing")?;
        if e["operations"] != object!(expected_ops) || executed.len() != operations.len() {
            return Err("unit execution coverage missing or extra".into());
        }
        if e["execution_evidence_schema"] != "chrono-ci-executions/v2" {
            return Err("unsupported or missing unit execution evidence schema".into());
        }
        let program = crate::observed::installed(&req.host_root)?;
        let mut evidence =
            crate::runtime_session::Session::open(&req.host_root, &program, "judge-evidence-v1")?;
        let receipts: Vec<_> = executed
            .iter()
            .map(|row| expanded_receipt(&mut evidence, row))
            .collect();
        evidence.close()?;
        let supplied = operations
            .iter()
            .zip(&receipts)
            .filter_map(|(op, result)| {
                result
                    .as_ref()
                    .ok()
                    .map(|receipt| (op.method.operation.as_str(), receipt))
            })
            .collect();
        let mut checks =
            super::route_query::check_receipts(&req.host_root, &routes, &plan, supplied)?;
        for ((operation, row), receipt) in operations.iter().zip(executed).zip(receipts) {
            let receipt = receipt?;
            if let Some(error) = checks.remove(&operation.method.operation).flatten() {
                return Err(error);
            }
            binding.check(
                "passed",
                operation.method.operation.as_str(),
                Some(&receipt),
            )?;
            if row["operation"] != operation.method.operation
                || receipt.process.exit_code != 0
                || receipt.process.failure.is_some()
                || !verified_collection::has_result(
                    &unit_results,
                    &operation.method.operation,
                    &Status::Passed,
                    Some(0),
                )
            {
                return Err("unit operation original evidence failed or disagrees".into());
            }
        }
        // Successful replacement markers are not emitted by the scoped producer.
        if !verified_collection::matches_result_states(&unit_results, &not_required_ids)
            || !verified_collection::matches_report_result_ids(
                &unit_results,
                &expected_ids,
                &not_required_ids,
            )
        {
            return Err("unit result coverage differs from plan".into());
        }
        verified.push(object!({"unit":input.unit,"path":input.path,"sha256":input.sha256,"runner_sha256":input.runner_sha256,"judge_sha256":input.judge_sha256,"plan":plan.identity,"status":"verified"}));
        results.push(CheckResult {
            id: format!("ci.unit.{}", input.unit),
            status: Status::Passed,
            cause: "original unit report verified against fixed endpoints and candidate plan"
                .into(),
            exit_code: None,
        });
    }
    Ok(Response {
        protocol: units::scope_protocol(req).into(),
        request_id: req.request_id.clone(),
        status: verified_collection::response_status(&results),
        results,
        evidence: object!({"scope":config.schema,"execution_scope":req.scope,"base":req.base,"candidate":req.candidate,
            "global_selected":selected,"required_units":required,"manifest_sha256":sha256(&manifest_bytes),"reports":verified,
            "collection_limits":limits,"retained_originals":retained_originals,
            "executed":[],"acceptance":"global collection; no business operations executed",
            "input_closure":"incomplete: report identity pins are caller supplied; build provenance and external input closure are not certified",
            "parity":"unestablished"}),
    })
}
