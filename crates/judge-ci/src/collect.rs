//! Offline verification of explicit unit reports. Business operations are never executed here.
use super::*;
use crate::route_wire::Method;
use crate::route_wire::{Execution, Receipt};
use chrono_harness::{
    ProcessResult,
    units::{Manifest, Scope},
};
use std::io::Read;

/// Decode the response recorded by a unit and bind it to the request and process exit.
pub fn unit_response(value: &Value, request_id: &str, exit: i32) -> Result<Response, String> {
    let response = Response::deserialize(value).map_err(|error| error.to_string())?;
    if response.protocol != "chrono-ci-judge/v2"
        || response.request_id != request_id
        || response.results.is_empty()
        || response.evidence.as_object().is_none_or(|e| e.is_empty())
    {
        return Err("empty or mismatched judge response".into());
    }
    let mut ids = BTreeSet::new();
    for row in &response.results {
        if row.id.is_empty() || row.cause.is_empty() || !ids.insert(&row.id) {
            return Err("invalid/duplicate judge result".into());
        }
        match row.status {
            Status::Passed if row.exit_code.is_some_and(|exit| exit != 0) => {
                return Err("passed result has failing exit".into());
            }
            Status::Failed if row.exit_code == Some(0) => {
                return Err("failed result has successful exit".into());
            }
            Status::NotRequired if row.exit_code.is_some() => {
                return Err("not-required result must have null exit_code".into());
            }
            _ => (),
        }
    }
    let failed = response
        .results
        .iter()
        .any(|row| matches!(row.status, Status::Failed | Status::Blocked));
    let unused = response
        .results
        .iter()
        .all(|row| row.status == Status::NotRequired);
    if failed != matches!(response.status, Status::Failed | Status::Blocked)
        || unused != (response.status == Status::NotRequired)
        || failed != (exit != 0)
    {
        return Err("judge exit/status/results disagreement".into());
    }
    Ok(response)
}

fn read(
    root: &Path,
    path: &str,
    policy: &Policy,
    limit_name: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    chrono_harness::units::artifact_path(path)?;
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
    if meta.len() > limit {
        return Err(overflow(meta.len()));
    }
    let mut bytes = Vec::new();
    fs::File::open(resolved)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(overflow(bytes.len() as u64));
    }
    Ok(bytes)
}

fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn process(p: &ProcessResult) -> Result<(), String> {
    if !digest(&p.sha256)
        || p.stdout_sha256 != sha256(&p.stdout_bytes)
        || p.stderr_sha256 != sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
        || p.environment_digest != crate::codec::digest(&p.environment)?
    {
        return Err("unit original process bytes or identity mismatch".into());
    }
    Ok(())
}

/// Typed report/preparation/receipt/stream acquisition shared by native gather
/// and the canonical collector. Supplying these bytes is not report acceptance.
pub struct UnitOriginals {
    pub report: Value,
    pub request: Request,
    pub producer: ProcessResult,
    pub retained_report_path: Option<String>,
}

pub fn unit_originals(
    root: &Path,
    config: &CheckConfig,
    unit: &str,
    bytes: &[u8],
    transport: Option<&chrono_harness::prepared::ArtifactTransport>,
    mut read: impl FnMut(&chrono_harness::prepared::Original, u64) -> Result<Vec<u8>, String>,
) -> Result<UnitOriginals, String> {
    let mut read =
        |o: &chrono_harness::prepared::Original, limit: u64| -> Result<Vec<u8>, String> {
            chrono_harness::prepared::original_path(o, transport)?;
            let raw = read(o, limit)?;
            if raw.len() as u64 > limit {
                return Err(format!("unit original digest/limit mismatch: {}", o.path));
            }
            Ok(raw)
        };
    let published = json(bytes)?;
    let retained_streams = chrono_harness::units::reference_publication(config)?;
    let report_schema = if retained_streams {
        chrono_harness::scoped_report::SCHEMA
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
        let reference: chrono_harness::units::ReportReference =
            serde_json::from_value(published).map_err(|e| e.to_string())?;
        if reference.schema != chrono_harness::units::REPORT_REFERENCE {
            return Err("unit report reference schema mismatch".into());
        }
        let path = chrono_harness::prepared::original_path(&reference.original, transport)?;
        let raw = read(&reference.original, limits.report_bytes)?;
        if sha256(&raw) != reference.original.sha256 {
            return Err(format!(
                "original report digest mismatch: {} ({path})",
                unit
            ));
        }
        let original = json(&raw)?;
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
        chrono_harness::prepared::validate_portable_binding_with_reader(
            root,
            binding,
            transport,
            |o| read(o, 64 * 1024 * 1024),
        )?;
        let prepared: chrono_harness::prepared::PreparedCheck =
            serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
        if prepared.scope != original.scope {
            return Err("original preparation scope differs from collected unit".into());
        }
    } else if transport.is_some() {
        return Err("native short report missing original preparation".into());
    }
    let producer: ProcessResult = if retained_streams {
        if report.get("response").is_some() {
            return Err("retained-stream report cannot contain a separate response".into());
        }
        chrono_harness::scoped_report::restore_process(&report["judge"], |original| {
            let raw = read(original, remaining)?;
            remaining -= raw.len() as u64;
            Ok(raw)
        })?
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
    let required = units::required(units, selected);
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
    let retained_streams = chrono_harness::units::reference_publication(config)?;
    for input in &inputs.reports {
        let bytes = read(
            &req.host_root,
            &input.path,
            p,
            "report_bytes",
            limits.report_bytes,
        )?;
        if sha256(&bytes) != input.sha256 {
            return Err(format!("unit report digest mismatch: {}", input.unit));
        }
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
            &bytes,
            input.artifacts.as_ref(),
            |o, limit| {
                let path = chrono_harness::prepared::original_path(o, input.artifacts.as_ref())?;
                read(
                    &req.host_root,
                    &path,
                    p,
                    "report_bytes (original/stream budget)",
                    limit,
                )
            },
        )?;
        retained_originals.extend(originals.retained_report_path);
        let report = originals.report;
        let original = originals.request;
        let producer = originals.producer;
        if let Some(binding) = original
            .observations
            .get("preparation")
            .filter(|b| b.is_object())
        {
            let prepared: chrono_harness::prepared::PreparedCheck =
                serde_json::from_value(binding["result"].clone()).map_err(|e| e.to_string())?;
            if prepared.base != req.base
                || prepared.candidate != req.candidate
                || prepared.profile != req.config_path
                || prepared.scope != original.scope
            {
                return Err("original preparation differs from collected unit".into());
            }
        }
        process(&producer)?;
        let decoded = json(&producer.stdout_bytes)?;
        let response_value = if retained_streams {
            &decoded
        } else {
            &report["response"]
        };
        let response = unit_response(response_value, &original.request_id, producer.exit_code)?;
        if original.protocol != chrono_harness::units::PROTOCOL
            || original.scope
                != Some(Scope::Unit {
                    unit: input.unit.clone(),
                })
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
        let candidates = chrono_harness::program_candidates(
            &original.host_root,
            &config.judge.program,
            Some(
                producer
                    .environment
                    .get("PATH")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
        )?;
        let expected_argv: Vec<_> =
            std::iter::once(producer.executable.to_string_lossy().into_owned())
                .chain(config.judge.args.clone())
                .collect();
        if !candidates.contains(&producer.executable) || producer.argv != expected_argv {
            return Err("unit judge command differs from profile".into());
        }
        if response.status != Status::Passed {
            results.push(CheckResult {
                id: format!("ci.unit.{}", input.unit),
                status: Status::Failed,
                cause: format!(
                    "unit failed; original report {} ({})",
                    input.path, input.sha256
                ),
                exit_code: Some(producer.exit_code),
            });
            verified.push(object!({"unit":input.unit,"path":input.path,"sha256":input.sha256,"status":"failed-original-retained"}));
            continue;
        }
        let unit_results = response.results;
        let owned = units::selection(units, selected, &input.unit)?;
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
            &super::report_path(&original, config)?,
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
            process(&tool.version)?;
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
