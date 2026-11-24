use super::*;
use crate::runtime_session::Session;
use crate::wire::{Scope, Unit};

pub(super) const SCOPED_PROTOCOL: &str = "chrono-ci-judge/v2";

pub(super) fn artifact_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err(
            "unit reports and collection manifests must reside in .chrono-harness/state/".into(),
        );
    }
    Ok(())
}

pub(super) fn reference_publication(root: &Path, config: &CheckConfig) -> Result<bool, String> {
    if config.policy.get("report_publication").is_none() {
        return Ok(false);
    }
    Session::query(
        root,
        "judge-evidence-v1",
        object!({"operation":"scoped-reference-publication","config":config}),
    )
}

pub(super) fn report_path(
    root: &Path,
    config: &CheckConfig,
    scope: &Scope,
) -> Result<String, String> {
    Session::query(
        root,
        "judge-evidence-v1",
        object!({"operation":"scoped-report-path","config":config,"scope":scope}),
    )
}

pub(super) fn validate(config: &CheckConfig, policy: &Policy) -> Result<(), String> {
    if config.schema != "chrono-ci-check/v3" {
        if config.policy.get("units").is_some()
            || config.policy.get("shared_operations").is_some()
            || config.policy.get("collection_limits").is_some()
        {
            return Err("CI units require chrono-ci-check/v3".into());
        }
        return Ok(());
    }
    if config.policy.get("collection_limits").is_some() && policy.collection_limits.is_none() {
        return Err("collection_limits must be an object when supplied".into());
    }
    if let Some(limits) = &policy.collection_limits {
        for (name, value) in [
            ("manifest_bytes", limits.manifest_bytes),
            ("report_bytes", limits.report_bytes),
        ] {
            if value == 0 || value >= usize::MAX as u64 {
                return Err(format!("invalid collection limit {name}: {value}"));
            }
        }
    }
    let units = policy.units.as_ref().ok_or("v3 requires explicit units")?;
    policy
        .shared_operations
        .as_ref()
        .ok_or("v3 requires explicit shared_operations")?;
    if units.is_empty() {
        return Err("CI units must be nonempty".into());
    }
    let mut paths = BTreeSet::from([config.report_path.clone()]);
    for (key, unit) in units {
        id(key).map_err(|_| format!("invalid CI unit ID: {key:?}"))?;
        artifact_path(&unit.report_path)?;
        if !policy
            .artifacts
            .iter()
            .any(|a| unit.report_path.starts_with(a))
            || !paths.insert(unit.report_path.clone())
            || unit.tests.is_empty()
        {
            return Err(
                "CI unit report collision, undeclared artifact or empty test assignment".into(),
            );
        }
    }
    for a in &paths {
        for b in &paths {
            if a != b && b.starts_with(&format!("{a}/")) {
                return Err("CI report paths overlap".into());
            }
        }
    }
    Ok(())
}

pub(super) fn assignments(
    root: &Path,
    query: &Query<'_>,
    candidate_tools: &BTreeMap<String, Value>,
    snapshot: &Snapshot,
) -> Result<(), String> {
    let Some(units) = &snapshot.policy.units else {
        return Ok(());
    };
    let shared = snapshot
        .policy
        .shared_operations
        .as_ref()
        .ok_or("missing shared operations")?;
    let routes = route_query::installed(root, candidate_tools, "chrono-judge-routes")?;
    let program = routes.to_str().ok_or("non-UTF8 routes executable")?;
    query.read(
        program,
        "check-unit-assignments-v1",
        &object!({
            "execution_units":{"units":units,"shared_operations":shared},
            "plans":snapshot.plans,
        }),
    )
}

pub(super) fn selection(
    root: &Path,
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
    unit: &str,
) -> Result<BTreeSet<String>, String> {
    Session::query(
        root,
        "judge-evidence-v1",
        object!({"operation":"unit-tests","units":units,"selected":selected,"unit":unit}),
    )
}

pub(super) fn required(
    root: &Path,
    units: &BTreeMap<String, Unit>,
    selected: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    Session::query(
        root,
        "judge-evidence-v1",
        object!({"operation":"required-units","units":units,"selected":selected}),
    )
}

pub(super) fn scope_protocol(req: &Request) -> &str {
    if req.scope.is_some() {
        SCOPED_PROTOCOL
    } else {
        PROTOCOL
    }
}

pub(super) fn decorate(
    req: &Request,
    snapshot: &Snapshot,
    selected: &BTreeSet<String>,
    evidence: &mut Value,
) -> Result<(), String> {
    if let Some(units) = &snapshot.policy.units {
        evidence["global_selected"] = object!(selected);
        evidence["required_units"] = object!(required(&req.host_root, units, selected)?);
        evidence["execution_scope"] = object!(req.scope);
        let own: BTreeSet<String> = serde_json::from_value(evidence["selected"].clone()).unwrap();
        evidence["assigned_elsewhere"] = object!(selected.difference(&own).collect::<Vec<_>>());
        evidence["acceptance"] = object!(if matches!(req.scope, Some(Scope::Unit { .. })) {
            "unit-only; global completion requires collection"
        } else {
            "global"
        });
    }
    Ok(())
}
