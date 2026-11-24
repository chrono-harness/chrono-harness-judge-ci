//! Read scoped operation declarations and tool observations from the host's routes release.
use super::route_wire::{Execution, Operation, Receipt};
use crate::Snapshot;
use crate::route_wire::{Method, Plan};
use chrono_harness::observation::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Stdio,
};

#[derive(Serialize)]
pub(super) struct Graph<'a> {
    pub selected: &'a BTreeSet<String>,
    pub plans: &'a BTreeMap<String, Plan>,
    pub methods: &'a BTreeMap<String, Vec<Method>>,
    pub execute: &'a BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ordered {
    pub selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FullOperations {
    selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
    pub methods: BTreeMap<String, Vec<Method>>,
    pub required_units: Option<Vec<String>>,
}

/// Schedule and collect against the same interpreted endpoint, including retired bindings.
pub(super) fn full_operations(
    root: &Path,
    query: &crate::impact_query::Query<'_>,
    filemap: &Value,
    projects: &Value,
    actions: &BTreeMap<String, Vec<String>>,
    selected: &BTreeSet<String>,
    execution_units: Option<&Value>,
) -> Result<FullOperations, String> {
    let response: FullOperations = query.read(
        "chrono-judge-routes",
        "full-operations-v1",
        &json!({"selected":selected, "test_operations":actions, "filemap":filemap,
            "projects":projects, "execution_units":execution_units}),
    )?;
    if !response.selected.keys().eq(selected.iter()) {
        return Err("E_PLAN_BINDING: routes selected different test plans".into());
    }
    for (test, plan) in &response.selected {
        let declared: Plan = serde_json::from_value(filemap["execution_plans"][test].clone())
            .map_err(|error| error.to_string())?;
        if plan != &declared {
            return Err("E_PLAN_BINDING: routes selected different test plans".into());
        }
    }
    let expected_units = execution_units
        .map(|block| {
            let units = serde_json::from_value(block["units"].clone())
                .map_err(|error| error.to_string())?;
            crate::units::required(root, &units, selected)
        })
        .transpose()?;
    if response.required_units != expected_units {
        return Err("E_UNIT_ASSIGNMENT: routes returned different required units".into());
    }
    Ok(response)
}

pub(super) fn installed(
    root: &Path,
    tools: &BTreeMap<String, Value>,
    name: &str,
) -> Result<PathBuf, String> {
    let program = match tools.get(name) {
        Some(tool) => tool["program"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("invalid registered {name} program"))?,
        None => name,
    };
    crate::files::resolve_program(root, program, None)
}

impl Graph<'_> {
    pub fn ordered(
        &self,
        root: &Path,
        program: &Path,
        binding: Option<&Value>,
    ) -> Result<Ordered, String> {
        let result: Ordered = read(
            root,
            program,
            "scoped-operations-v1",
            &json!({"graph":self,"binding":binding}),
        )?;
        if !result.selected.keys().eq(self.selected.iter())
            || result
                .selected
                .iter()
                .any(|(id, plan)| self.plans.get(id) != Some(plan))
        {
            return Err("E_PLAN_BINDING: routes selected different test plans".into());
        }
        Ok(result)
    }

    pub fn prepare(
        &self,
        root: &Path,
        snapshot: &Snapshot,
        binding: Value,
        report_path: &str,
        declarations: &Value,
        environment: &BTreeMap<String, String>,
        reused: &BTreeMap<String, Tool>,
    ) -> Result<Execution, String> {
        let program = installed(root, &snapshot.tools, "chrono-judge-routes")?;
        let request = json!({"graph":self,"binding":binding,"report_path":report_path,"declarations":declarations,
            "environment":environment,"reused":reused,"scheduling":snapshot.scheduling});
        let plan: Execution = read(root, &program, "prepare-scoped-plan-v1", &request)?;
        check_scoped_plan(root, &program, &plan, &binding, report_path)?;
        if plan.root != root.canonicalize().map_err(|e| e.to_string())?
            || !plan.selected.keys().eq(self.selected.iter())
            || plan
                .selected
                .iter()
                .any(|(id, plan)| self.plans.get(id) != Some(plan))
            || plan.scheduling != snapshot.scheduling
        {
            return Err("E_PLAN_BINDING: prepared plan differs from request".into());
        }
        Ok(plan)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanIdentity {
    plan: String,
}

pub(super) fn check_scoped_plan(
    root: &Path,
    program: &Path,
    plan: &Execution,
    binding: &Value,
    report_path: &str,
) -> Result<(), String> {
    let reply: PlanIdentity = read(
        root,
        program,
        "check-scoped-plan-v1",
        &json!({"plan":plan, "binding":binding, "report_path":report_path}),
    )?;
    if reply.plan != plan.identity {
        return Err("E_PLAN_IDENTITY: routes returned a different plan".into());
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptChecks {
    plan: String,
    operations: BTreeMap<String, Option<String>>,
}

/// One replay for all receipts. The caller still reports the first error in plan order.
pub(super) fn check_receipts(
    root: &Path,
    program: &Path,
    plan: &Execution,
    receipts: BTreeMap<&str, &Receipt>,
) -> Result<BTreeMap<String, Option<String>>, String> {
    #[derive(Serialize)]
    struct Replay<'a> {
        plan: &'a Execution,
        receipts: BTreeMap<&'a str, &'a Receipt>,
    }
    let reply: ReceiptChecks = read(
        root,
        program,
        "check-plan-receipts-v1",
        &Replay { plan, receipts },
    )?;
    if reply.plan != plan.identity
        || reply.operations.len() != plan.operations.len()
        || !plan
            .operations
            .iter()
            .all(|op| reply.operations.contains_key(&op.method.operation))
    {
        return Err("E_PLAN_BINDING: routes returned different receipt operations".into());
    }
    Ok(reply.operations)
}

fn read<T: serde::de::DeserializeOwned>(
    root: &Path,
    program: &Path,
    operation: &str,
    request: &(impl Serialize + Sync),
) -> Result<T, String> {
    let digest = crate::files::file_identity(program)?.0;
    // Each tool observation has its own registered bounds. Join the complete
    // preparation even when a plan must observe several tools in sequence.
    let mut command = crate::storage::command(&crate::observed::installed(root)?, program)?;
    command
        .arg(operation)
        .current_dir(root)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("start {}: {e}", program.display()))?;
    let stdin = child.stdin.take().unwrap();
    let (output, sent) = std::thread::scope(|scope| {
        let writer = scope.spawn(|| serde_json::to_writer(stdin, &request));
        (child.wait_with_output(), writer.join())
    });
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        return Err(if reason.is_empty() {
            format!("routes {operation} {}", output.status)
        } else {
            reason.strip_suffix('\n').unwrap_or(&reason).to_owned()
        });
    }
    sent.map_err(|_| "routes request writer panicked")?
        .map_err(|e| e.to_string())?;
    if crate::files::file_identity(program)?.0 != digest {
        return Err("E_PLAN_BINDING: routes executable changed".into());
    }
    crate::codec::decode(&output.stdout)
}

#[derive(Serialize)]
pub(super) struct SourceEndpoint<'a> {
    pub commit: &'a str,
    pub filemap: &'a Value,
    pub projects: &'a Value,
    pub artifacts: Vec<String>,
}

impl<'a> SourceEndpoint<'a> {
    pub fn scoped(commit: &'a str, snapshot: &'a Snapshot) -> Self {
        Self {
            commit,
            filemap: &snapshot.filemap,
            projects: &snapshot.registry,
            artifacts: snapshot.policy.artifacts.clone(),
        }
    }

    pub fn registered(
        root: &Path,
        commit: &'a str,
        config: &'a Value,
        filemap: &'a Value,
        projects: &'a Value,
    ) -> Result<Self, String> {
        Ok(Self {
            commit,
            filemap,
            projects,
            artifacts: crate::runtime_session::Session::query(
                root,
                "judge-evidence-v1",
                json!({"operation":"artifact-directories", "config":config}),
            )?,
        })
    }
}

/// The route planner owns proof admission. Keep its real Git observations in the
/// same sequence as the caller's facts, and accept them only for the same binding.
pub(super) fn source_plan(
    query: &crate::impact_query::Query<'_>,
    program: &str,
    config: &str,
    previous: Option<SourceEndpoint<'_>>,
    candidate: SourceEndpoint<'_>,
    affected: &BTreeSet<String>,
    methods: &BTreeMap<String, Vec<Method>>,
    operations: &[Operation],
    facts: &Value,
    evidence: &mut crate::source_observations::GitEvidence,
) -> Result<(), String> {
    let (binding, offset) = crate::source_observations::binding(facts)?;
    let git = offset.map(|_| json!({"config":config, "git_facts":binding}));
    let reply = query.read(
        program,
        "source-plan-v1",
        &json!({
            "git":git, "previous":previous, "candidate":candidate, "affected":affected,
            "methods":methods, "operations":operations
        }),
    )?;
    evidence.admit(
        previous.map(|e| e.commit),
        candidate.commit,
        &binding,
        offset,
        reply,
    )
}
