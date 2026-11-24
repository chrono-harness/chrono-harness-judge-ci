//! Consume the installed project executor. Child operations keep their own command limits.
use crate::route_query::installed;
use crate::route_wire::{Execution, Receipt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};
use tracing_tunnel::{TracingEvent, TracingEventReceiver};

#[derive(Serialize)]
struct RouteCommand {
    executable: PathBuf,
    sha256: String,
    timeout_seconds: u64,
    output_limit_bytes: usize,
}

#[derive(Serialize)]
struct Request<'a> {
    plan: &'a Execution,
    routes: RouteCommand,
    diagnostics: Option<&'static str>,
}

#[derive(Deserialize)]
pub(super) struct Operation {
    pub operation: String,
    pub status: String,
    pub error: Option<String>,
    pub receipt: Option<Receipt>,
}

#[derive(Deserialize)]
pub(super) struct Results {
    plan: String,
    selected: Vec<String>,
    pub executed: Vec<Operation>,
    pub blocked: Vec<Operation>,
    pub operation_state: Option<Value>,
    pub tests: BTreeMap<String, String>,
}

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
        let mut missing: BTreeSet<_> = plan
            .operations
            .iter()
            .map(|op| op.method.operation.as_str())
            .collect();
        for row in self.executed.iter().chain(&self.blocked) {
            if !missing.remove(row.operation.as_str())
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
            if row.status == "passed" && row.receipt.is_none() {
                return Err(format!(
                    "E_RECEIPT_MISMATCH: no receipt for {}",
                    row.operation
                ));
            }
        }
        if !missing.is_empty() {
            return Err(format!(
                "E_PLAN_BINDING: missing project operations {missing:?}"
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Reply {
    results: Results,
    diagnostic_error: Option<String>,
}

fn receive(reader: impl Read, errors: &mut Vec<String>) -> Result<Vec<u8>, String> {
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
        } else {
            diagnostic.extend_from_slice(&line);
        }
        line.clear();
    }
    Ok(diagnostic)
}

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
    let mut command = crate::observed::command(&crate::observed::installed(root)?, &projects);
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
        let diagnostic = receive(stderr, errors);
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
