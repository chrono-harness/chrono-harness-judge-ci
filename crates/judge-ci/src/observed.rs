//! Bounded original query bytes and exit status from the host process owner.
use crate::files::sha256;
use serde::{Deserialize, Serialize};

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[derive(Deserialize)]
pub(crate) struct Observation {
    pub sha256: String,
    pub exit_code: i32,
    pub failure: Option<String>,
    launcher_pid: Option<u32>,
    child_pid: Option<u32>,
    stdout_size: usize,
    stderr_size: usize,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    #[serde(skip)]
    pub stdout_bytes: Vec<u8>,
    #[serde(skip)]
    pub stderr_bytes: Vec<u8>,
}

fn receive(mut input: impl Read, limit: usize) -> Result<Observation, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    loop {
        let mut header = [0u8; 9];
        input
            .read_exact(&mut header)
            .map_err(|e| format!("incomplete observed command: {e}"))?;
        let size = usize::try_from(u64::from_be_bytes(header[1..].try_into().unwrap()))
            .map_err(|_| "observed length overflow")?;
        let stream = match header[0] {
            b'O' => &mut stdout,
            b'E' => &mut stderr,
            b'T' => {
                if size > 2 * 1024 * 1024 {
                    return Err("oversize observed terminal record".into());
                }
                let mut bytes = vec![0u8; size];
                input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                let mut p: Observation = crate::decode(&bytes)?;
                if input.read(&mut [0]).map_err(|e| e.to_string())? != 0
                    || p.stdout_size != stdout.len()
                    || p.stderr_size != stderr.len()
                    || p.stdout_sha256 != sha256(&stdout)
                    || p.stderr_sha256 != sha256(&stderr)
                {
                    return Err("observed command stream identity differs".into());
                }
                p.stdout_bytes = stdout;
                p.stderr_bytes = stderr;
                return Ok(p);
            }
            _ => return Err("unknown observed command frame".into()),
        };
        if size > limit.saturating_sub(stream.len()) {
            return Err("observed output exceeds command bound".into());
        }
        let offset = stream.len();
        stream.resize(offset + size, 0);
        input
            .read_exact(&mut stream[offset..])
            .map_err(|e| e.to_string())?;
    }
}

/// Use the registered program with the source process's input, deadline and ownership descriptors.
pub(crate) fn run(
    root: &Path,
    runner: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
) -> Result<Observation, String> {
    let mut child = crate::storage::command(runner, runner)?
        .arg("observed-command-v1")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let launcher = child.id();
    let mut dest = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let mut errors = child.stderr.take().unwrap();
    let (received, sent, diagnostics) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || -> Result<(), String> {
            serde_json::to_writer(
                &mut dest,
                &serde_json::json!({"command":spec,"expected_sha256":digest,"timeout":timeout}),
            )
            .map_err(|e| e.to_string())?;
            dest.write_all(b"\n")
                .and_then(|_| dest.write_all(input))
                .map_err(|e| e.to_string())
        });
        let diagnostic = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut data = Vec::new();
            errors.read_to_end(&mut data)?;
            Ok(data)
        });
        let received = receive(output, spec.output_limit_bytes);
        (received, writer.join(), diagnostic.join())
    });
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostics = diagnostics
        .map_err(|_| "observed diagnostic reader panicked")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&diagnostics).trim_end().into());
    }
    sent.map_err(|_| "observed input writer panicked")??;
    let p = received?;
    if p.launcher_pid != Some(launcher)
        || p.child_pid.is_none_or(|pid| pid == 0)
        || p.sha256 != digest
    {
        return Err("observed process identity differs".into());
    }
    Ok(p)
}

pub(crate) fn installed(root: &Path) -> Result<PathBuf, String> {
    let local = root.join(".chrono-harness/bin/chrono-harness");
    if local.is_file() {
        return Ok(local);
    }
    crate::files::resolve_program(root, "chrono-harness", None)
}
