//! Read scoped operation declarations and tool observations from the host's routes release.
use crate::Snapshot;
use chrono_harness::observation::Tool;
use chrono_judge_registration::execution::{Method, Plan};
use chrono_judge_routes::{Execution, Operation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
    chrono_harness::resolve_program(root, program, None)
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

    pub fn scoped_order(
        &self,
        root: &Path,
        snapshot: &Snapshot,
        binding: Option<&Value>,
    ) -> Result<Ordered, String> {
        let program = installed(root, &snapshot.tools, "chrono-judge-routes")?;
        self.ordered(root, &program, binding)
    }

    pub fn prepare(
        &self,
        root: &Path,
        snapshot: &Snapshot,
        binding: Value,
        declarations: &Value,
        environment: &BTreeMap<String, String>,
        reused: &BTreeMap<String, Tool>,
    ) -> Result<Execution, String> {
        let program = installed(root, &snapshot.tools, "chrono-judge-routes")?;
        let request = json!({"graph":self,"binding":binding,"declarations":declarations,
            "environment":environment,"reused":reused,"scheduling":snapshot.scheduling});
        let plan: Execution = read(root, &program, "prepare-scoped-plan-v1", &request)?;
        plan.validate()?;
        if plan.binding != binding
            || plan.root != root.canonicalize().map_err(|e| e.to_string())?
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

fn read<T: serde::de::DeserializeOwned>(
    root: &Path,
    program: &Path,
    operation: &str,
    request: &Value,
) -> Result<T, String> {
    let digest = chrono_harness::file_identity(program)?.0;
    // Each tool observation has its own registered bounds. Join the complete
    // preparation even when a plan must observe several tools in sequence.
    let mut command = Command::new(program);
    command
        .arg(operation)
        .current_dir(root)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    chrono_harness::process_fds::forward_command(&mut command, &[])?;
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
    if chrono_harness::file_identity(program)?.0 != digest {
        return Err("E_PLAN_BINDING: routes executable changed".into());
    }
    chrono_harness::decode(&output.stdout)
}
