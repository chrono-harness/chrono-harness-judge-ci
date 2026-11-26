//! Consume the installed project executor. Child operations keep their own command limits.
#[cfg(not(verus_keep_ghost))]
use crate::route_query::installed;
#[cfg(not(verus_keep_ghost))]
use crate::route_wire::{Execution, Receipt};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};
#[cfg(not(verus_keep_ghost))]
use tracing_tunnel::{TracingEvent, TracingEventReceiver};

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
struct RouteCommand {
    executable: PathBuf,
    sha256: String,
    timeout_seconds: u64,
    output_limit_bytes: usize,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
struct Request<'a> {
    plan: &'a Execution,
    routes: RouteCommand,
    diagnostics: Option<&'static str>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Operation {
    pub operation: String,
    pub status: String,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct Results {
    plan: String,
    selected: Vec<String>,
    pub executed: Vec<Operation>,
    pub blocked: Vec<Operation>,
    pub operation_state: Option<Value>,
    pub tests: BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
impl Results {
    fn validate(&self, plan: &Execution) -> Result<(), String> {
        if self.plan != plan.identity
            || !self.selected.iter().eq(plan.selected.keys())
            || !self.tests.keys().eq(plan.selected.keys())
        {
            return Err(
                "E_PLAN_BINDING: projects returned a different plan or test selection".into(),
            );
        }
        let mut missing: BTreeMap<_, _> = plan
            .operations
            .iter()
            .map(|op| {
                (
                    op.method.operation.as_str(),
                    op.method.output_contract.is_some(),
                )
            })
            .collect();
        let mut proof_receipt = None;
        let mut ordinary_receipt = None;
        for row in self.executed.iter().chain(&self.blocked) {
            let required = missing.remove(row.operation.as_str());
            if required.is_none()
                || row
                    .receipt
                    .as_ref()
                    .is_some_and(|r| r.operation != row.operation || r.plan != plan.identity)
            {
                return Err(format!(
                    "E_PLAN_BINDING: unexpected operation or receipt {}",
                    row.operation
                ));
            }
            if needs_receipt(&row.status, row.receipt.is_some()) {
                let gap = if required == Some(true) {
                    &mut proof_receipt
                } else {
                    &mut ordinary_receipt
                };
                gap.get_or_insert(row.operation.as_str());
            }
        }
        let gap = reply_gap(
            proof_receipt,
            ordinary_receipt,
            !missing.is_empty(),
            missing.values().any(|required| *required),
        );
        match gap {
            Some(ReplyGap::Receipt(name, proof)) => {
                return Err(crate::verified_collection::coverage_cause(
                    format!("E_RECEIPT_MISMATCH: no receipt for {name}"),
                    proof,
                ));
            }
            Some(ReplyGap::Operations(proof)) => {
                let names: BTreeSet<_> = missing.keys().copied().collect();
                return Err(crate::verified_collection::coverage_cause(
                    format!("E_PLAN_BINDING: missing project operations {names:?}"),
                    proof,
                ));
            }
            None => {}
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Reply {
    results: Results,
    diagnostic_error: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
fn maintenance_notice(line: &[u8]) -> bool {
    let Some(bytes) = line.strip_prefix(b"W_MAINTENANCE_ACTIVATION: ") else {
        return false;
    };
    let Ok(body) = crate::codec::json(bytes) else {
        return false;
    };
    let Some(fields) = body.as_object() else {
        return false;
    };
    if fields.keys().any(|name| {
        !matches!(
            name.as_str(),
            "schema"
                | "reason"
                | "accepted"
                | "round_completion"
                | "disposal"
                | "responsibility"
                | "activation_root"
                | "policy_sha256"
                | "socket"
                | "status"
                | "error"
        )
    }) || body["schema"] != "chrono-maintenance-request/v1"
        || body["responsibility"] != "existing-owner-state"
        || body["round_completion"] != "not-established"
        || body["disposal"] != "not-attempted"
        || !body["reason"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control))
    {
        return false;
    }
    for path in ["activation_root", "socket"] {
        if let Some(value) = fields.get(path) {
            if !value.is_null() && !value.as_str().is_some_and(|s| Path::new(s).is_absolute()) {
                return false;
            }
        }
    }
    if let Some(digest) = fields.get("policy_sha256") {
        if !digest.as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) {
            return false;
        }
    }
    match (body["status"].as_str(), body["accepted"].as_bool()) {
        (Some("requested"), Some(true)) => {
            ["activation_root", "policy_sha256", "socket"]
                .iter()
                .all(|key| fields.contains_key(*key))
                && !fields.contains_key("error")
        }
        (Some("wake-failed"), Some(false)) => body["error"].as_str().is_some_and(|s| !s.is_empty()),
        _ => false,
    }
}

/// Forward producer-owned wake receipts byte for byte. All other stderr
/// remains under the original strict project-diagnostic contract.
#[cfg(not(verus_keep_ghost))]
#[doc(hidden)]
pub fn receive_project_diagnostics(
    reader: impl Read,
    mut notifications: impl Write,
    errors: &mut Vec<String>,
) -> Result<Vec<u8>, String> {
    let mut traces =
        TracingEventReceiver::new(Default::default(), Default::default(), Default::default());
    let mut diagnostic = Vec::new();
    let mut line = Vec::new();
    let mut input = BufReader::new(reader);
    while input
        .read_until(b'\n', &mut line)
        .map_err(|err| err.to_string())?
        > 0
    {
        if let Some(bytes) = line.strip_prefix(b"chrono-trace-v1 ") {
            let event =
                serde_json::from_slice::<TracingEvent>(bytes).map_err(|error| error.to_string());
            if let Err(error) =
                event.and_then(|event| traces.try_receive(event).map_err(|e| e.to_string()))
            {
                errors.push(error);
            }
        } else if maintenance_notice(&line) {
            notifications
                .write_all(&line)
                .map_err(|error| error.to_string())?;
        } else {
            diagnostic.extend_from_slice(&line);
        }
        line.clear();
    }
    Ok(diagnostic)
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn execute(
    root: &Path,
    plan: &Execution,
    tools: &BTreeMap<String, Value>,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    errors: &mut Vec<String>,
) -> Result<Results, String> {
    let projects = installed(root, tools, "chrono-judge-projects")?;
    let routes = installed(root, tools, "chrono-judge-routes")?;
    let digest = crate::files::file_identity(&projects)?.0;
    let max_level = tracing::level_filters::LevelFilter::current();
    let diagnostics = max_level.into_level().map(|level| level.as_str());
    let request = Request {
        plan,
        routes: RouteCommand {
            sha256: crate::files::file_identity(&routes)?.0,
            executable: routes,
            timeout_seconds,
            output_limit_bytes,
        },
        diagnostics,
    };
    let mut command = crate::storage::command(&crate::observed::installed(root)?, &projects)?;
    command
        .arg("execute-plan-v1")
        .env_clear()
        .envs(&plan.environment)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("start {}: {e}", projects.display()))?;
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (sent, reply, diagnostic) = std::thread::scope(|scope| {
        let send = scope.spawn(move || {
            serde_json::to_writer(&mut stdin, &request).map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())
        });
        let read = scope.spawn(move || serde_json::from_reader::<_, Reply>(BufReader::new(stdout)));
        let diagnostic =
            receive_project_diagnostics(stderr, crate::diagnostic_session::Stderr, errors);
        (send.join(), read.join(), diagnostic)
    });
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostic = diagnostic.unwrap_or_else(|error| {
        errors.push(error);
        Vec::new()
    });
    if !status.success() {
        return Err(format!(
            "chrono-judge-projects execute-plan-v1 {status}: {}",
            String::from_utf8_lossy(&diagnostic)
        ));
    }
    sent.map_err(|_| "projects request writer panicked")??;
    let reply = reply
        .map_err(|_| "projects result reader panicked")?
        .map_err(|e| format!("invalid projects result: {e}"))?;
    reply.results.validate(plan)?;
    if crate::files::file_identity(&projects)?.0 != digest {
        return Err("E_PLAN_BINDING: projects program or result plan changed".into());
    }
    errors.extend(reply.diagnostic_error);
    if !diagnostic.is_empty() {
        errors.push(format!(
            "unexpected project execution diagnostic: {}",
            String::from_utf8_lossy(&diagnostic)
        ));
    }
    Ok(reply.results)
}

use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

verus! {

pub enum ReplyGap<'a> {
    Receipt(&'a str, bool),
    Operations(bool),
}

pub fn needs_receipt(status: &str, has_receipt: bool) -> (missing: bool)
    ensures missing == (status@ == "passed"@ && !has_receipt),
{
    <&str>::exec_eq(status, "passed") && !has_receipt
}

/// Select the original diagnosis after the plan and every returned identity match.
/// A missing proof stays visible when an ordinary operation also lacks its result.
pub fn reply_gap<'a>(
    proof_receipt: Option<&'a str>, ordinary_receipt: Option<&'a str>,
    missing_operations: bool, missing_proof: bool,
) -> (gap: Option<ReplyGap<'a>>)
    ensures match gap {
        Some(ReplyGap::Receipt(name, true)) => proof_receipt == Some(name),
        Some(ReplyGap::Operations(true)) => proof_receipt.is_none() && missing_proof,
        Some(ReplyGap::Receipt(name, false)) => proof_receipt.is_none() && !missing_proof
            && ordinary_receipt == Some(name),
        Some(ReplyGap::Operations(false)) => proof_receipt.is_none() && !missing_proof
            && ordinary_receipt.is_none() && missing_operations,
        None => proof_receipt.is_none() && ordinary_receipt.is_none()
            && !missing_operations && !missing_proof,
    },
{
    if let Some(name) = proof_receipt { return Some(ReplyGap::Receipt(name, true)); }
    if missing_proof { return Some(ReplyGap::Operations(true)); }
    if let Some(name) = ordinary_receipt { return Some(ReplyGap::Receipt(name, false)); }
    if missing_operations { Some(ReplyGap::Operations(false)) } else { None }
}

}
