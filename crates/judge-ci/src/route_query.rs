//! Read scoped operation declarations and tool observations from the host's routes release.
#[cfg(not(verus_keep_ghost))]
use super::route_wire::{Execution, Operation, Receipt};
#[cfg(not(verus_keep_ghost))]
use crate::Snapshot;
#[cfg(not(verus_keep_ghost))]
use crate::route_wire::Tool;
#[cfg(not(verus_keep_ghost))]
use crate::route_wire::{Method, Plan};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Stdio,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
pub(super) struct Graph<'a> {
    pub selected: &'a BTreeSet<String>,
    pub plans: &'a BTreeMap<String, Plan>,
    pub methods: &'a BTreeMap<String, Vec<Method>>,
    pub execute: &'a BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ordered {
    pub selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FullOperations {
    selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
    pub methods: BTreeMap<String, Vec<Method>>,
    pub required_units: Option<Vec<String>>,
}

/// Schedule and collect against the same interpreted endpoint, including retired bindings.
#[cfg(not(verus_keep_ghost))]
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
    selected_plans(response.selected.keys().eq(selected.iter()), true)
        .map_err(ReplyMismatch::message)?;
    for (test, plan) in &response.selected {
        let declared: Plan = serde_json::from_value(filemap["execution_plans"][test].clone())
            .map_err(|error| error.to_string())?;
        selected_plans(true, plan == &declared).map_err(ReplyMismatch::message)?;
    }
    let expected_units = execution_units
        .map(|block| {
            let units = serde_json::from_value(block["units"].clone())
                .map_err(|error| error.to_string())?;
            crate::units::required(root, &units, selected)
        })
        .transpose()?;
    required_units(&expected_units, &response.required_units).map_err(ReplyMismatch::message)?;
    Ok(response)
}

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
impl Graph<'_> {
    fn check_selection(&self, selected: &BTreeMap<String, Plan>) -> Result<(), ReplyMismatch> {
        let keys_match = selected.keys().eq(self.selected.iter());
        let plans_match = keys_match
            && selected
                .iter()
                .all(|(id, plan)| self.plans.get(id) == Some(plan));
        selected_plans(keys_match, plans_match)
    }

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
        self.check_selection(&result.selected)
            .map_err(ReplyMismatch::message)?;
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
        let root_matches = plan.root == root.canonicalize().map_err(|e| e.to_string())?;
        let selection_matches = root_matches && self.check_selection(&plan.selected).is_ok();
        prepared_plan(
            root_matches,
            selection_matches,
            plan.scheduling == snapshot.scheduling,
        )
        .map_err(ReplyMismatch::message)?;
        Ok(plan)
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanIdentity {
    plan: String,
}

#[cfg(not(verus_keep_ghost))]
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
    plan_identity(&plan.identity, &reply.plan).map_err(|error| error.message().into())
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptChecks {
    plan: String,
    operations: BTreeMap<String, Option<String>>,
}

/// One replay for all receipts. The caller still reports the first error in plan order.
#[cfg(not(verus_keep_ghost))]
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
    receipt_operations(
        &plan.identity,
        &reply.plan,
        &plan.operations,
        reply.operations.len(),
        |op| reply.operations.contains_key(&op.method.operation),
    )
    .map_err(ReplyMismatch::message)?;
    Ok(reply.operations)
}

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
pub(super) struct SourceEndpoint<'a> {
    pub commit: &'a str,
    pub filemap: &'a Value,
    pub projects: &'a Value,
    pub artifacts: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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

#[cfg(verus_keep_ghost)]
use super::verified_text::equal_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_collection::verified_text::equal_text;
use vstd::prelude::*;

#[cfg(not(verus_keep_ghost))]
impl ReplyMismatch {
    fn message(self) -> &'static str {
        match self {
            Self::Selection => "E_PLAN_BINDING: routes selected different test plans",
            Self::RequiredUnits => "E_UNIT_ASSIGNMENT: routes returned different required units",
            Self::PreparedPlan => "E_PLAN_BINDING: prepared plan differs from request",
            Self::PlanIdentity => "E_PLAN_IDENTITY: routes returned a different plan",
            Self::ReceiptOperations => {
                "E_PLAN_BINDING: routes returned different receipt operations"
            }
        }
    }
}

verus! {
pub enum ReplyMismatch {
    Selection,
    RequiredUnits,
    PreparedPlan,
    PlanIdentity,
    ReceiptOperations,
}

/// The native adapters compare keys from ordered collections and the typed plans they decoded.
pub fn selected_plans(keys_match: bool, plans_match: bool) -> (result: Result<(), ReplyMismatch>)
    ensures result == if keys_match && plans_match { Ok(()) } else { Err(ReplyMismatch::Selection) },
{
    if keys_match && plans_match { Ok(()) } else { Err(ReplyMismatch::Selection) }
}

/// A prepared reply must bind all three observations after the identity replay succeeds.
pub fn prepared_plan(root_matches: bool, selection_matches: bool, scheduling_matches: bool)
    -> (result: Result<(), ReplyMismatch>)
    ensures result == if root_matches && selection_matches && scheduling_matches {
        Ok(())
    } else {
        Err(ReplyMismatch::PreparedPlan)
    },
{
    if root_matches && selection_matches && scheduling_matches {
        Ok(())
    } else {
        Err(ReplyMismatch::PreparedPlan)
    }
}

pub open spec fn same_unit_sequence(want: Seq<String>, got: Seq<String>) -> bool {
    want.len() == got.len()
        && forall|i: int| 0 <= i < want.len()
            ==> (#[trigger] want[i])@ == got[i]@
}

fn unit_sequence_matches(want: &[String], got: &[String]) -> (matches: bool)
    ensures matches == same_unit_sequence(want@, got@),
{
    if want.len() != got.len() { return false; }
    let mut i = 0;
    while i < want.len()
        invariant i <= want.len(), want.len() == got.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] want@[j])@ == got@[j]@,
        decreases want.len() - i,
    {
        if !equal_text(want[i].as_str(), got[i].as_str()) {
            proof {
                assert(!same_unit_sequence(want@, got@)) by {
                    if same_unit_sequence(want@, got@) {
                        assert(want@[i as int]@ == got@[i as int]@);
                    }
                }
            }
            return false;
        }
        i += 1;
    }
    true
}

pub fn required_units(expected: &Option<Vec<String>>, actual: &Option<Vec<String>>)
    -> (result: Result<(), ReplyMismatch>)
    ensures result == if match (expected, actual) {
        (None, None) => true,
        (Some(want), Some(got)) => same_unit_sequence(want@, got@),
        _ => false,
    } { Ok(()) } else { Err(ReplyMismatch::RequiredUnits) },
{
    let matches = match (expected, actual) {
        (None, None) => true,
        (Some(want), Some(got)) => unit_sequence_matches(want.as_slice(), got.as_slice()),
        _ => false,
    };
    if matches { Ok(()) } else { Err(ReplyMismatch::RequiredUnits) }
}

pub fn plan_identity(expected: &str, actual: &str) -> (result: Result<(), ReplyMismatch>)
    ensures result == if expected@ == actual@ { Ok(()) } else { Err(ReplyMismatch::PlanIdentity) },
{
    if equal_text(expected, actual) { Ok(()) } else { Err(ReplyMismatch::PlanIdentity) }
}

/// Each replay must name the requested plan and have the same number of operation keys.
pub fn receipt_header(expected: &str, actual: &str, planned: usize, returned: usize) -> (bound: bool)
    ensures bound == (expected@ == actual@ && planned == returned),
{
    equal_text(expected, actual) && planned == returned
}

/// Check the header before looking up every planned operation in the reply map.
pub fn receipt_operations<T, F>(expected: &str, actual: &str, planned: &[T], returned: usize, present: F)
    -> (result: Result<(), ReplyMismatch>)
    where F: Fn(&T) -> bool,
    requires forall|row: &T| present.requires((row,)),
    ensures match result {
        Ok(()) => expected@ == actual@ && planned.len() == returned
            && forall|i: int| 0 <= i < planned.len()
                ==> #[trigger] present.ensures((&planned@[i],), true),
        Err(ReplyMismatch::ReceiptOperations) => expected@ != actual@ || planned.len() != returned
            || exists|i: int| 0 <= i < planned.len()
                && #[trigger] present.ensures((&planned@[i],), false),
        _ => false,
    },
{
    if !receipt_header(expected, actual, planned.len(), returned) {
        return Err(ReplyMismatch::ReceiptOperations);
    }
    let mut i = 0;
    while i < planned.len()
        invariant i <= planned.len(), forall|row: &T| present.requires((row,)),
            forall|j: int| 0 <= j < i ==> #[trigger] present.ensures((&planned@[j],), true),
        decreases planned.len() - i,
    {
        if !present(&planned[i]) {
            return Err(ReplyMismatch::ReceiptOperations);
        }
        i += 1;
    }
    Ok(())
}
}
