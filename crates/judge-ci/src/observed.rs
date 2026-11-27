//! Bounded original query bytes and exit status from the host process owner.
#[cfg(verus_keep_ghost)]
use super::verified_text::equal_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_collection::verified_text::equal_text;
use vstd::prelude::*;

verus! {
/// A data frame carries the permitted new stream length; a terminal carries its JSON length.
pub const STDOUT_TAG: u8 = 79; // ASCII O
pub const STDERR_TAG: u8 = 69; // ASCII E
pub const TERMINAL_TAG: u8 = 84; // ASCII T

pub enum ObservedFrame { Stdout(usize), Stderr(usize), Terminal(usize) }

/// Keep launcher diagnostics as their original bytes until the caller needs to display them.
pub enum ObservedFailure<E, D> { Transport(E), Launcher(D) }

pub fn finish_observation<T, E, D>(
    wait: Result<bool, E>, diagnostics: Result<D, E>, written: Result<(), E>, received: Result<T, E>,
) -> (result: Result<T, ObservedFailure<E, D>>)
    ensures result == match wait {
        Err(e) => Err(ObservedFailure::Transport(e)),
        Ok(success) => match diagnostics {
            Err(e) => Err(ObservedFailure::Transport(e)),
            Ok(bytes) => if !success { Err(ObservedFailure::Launcher(bytes)) }
                else { match written {
                    Err(e) => Err(ObservedFailure::Transport(e)),
                    Ok(()) => match received { Err(e) => Err(ObservedFailure::Transport(e)), Ok(p) => Ok(p) },
                } },
        },
    },
{
    let success = match wait { Err(e) => return Err(ObservedFailure::Transport(e)), Ok(s) => s };
    let bytes = match diagnostics { Err(e) => return Err(ObservedFailure::Transport(e)), Ok(d) => d };
    if !success { return Err(ObservedFailure::Launcher(bytes)); }
    if let Err(e) = written { return Err(ObservedFailure::Transport(e)); }
    match received { Err(e) => Err(ObservedFailure::Transport(e)), Ok(p) => Ok(p) }
}

pub fn observed_frame(tag: u8, length: usize, stdout: usize, stderr: usize, limit: usize)
    -> (result: Result<ObservedFrame, &'static str>)
    requires stdout <= limit, stderr <= limit,
    ensures match result {
        Ok(ObservedFrame::Stdout(end)) => tag == STDOUT_TAG && end as int == stdout + length && end <= limit,
        Ok(ObservedFrame::Stderr(end)) => tag == STDERR_TAG && end as int == stderr + length && end <= limit,
        Ok(ObservedFrame::Terminal(size)) => tag == TERMINAL_TAG && size == length && length <= 2097152,
        Err(e) => if tag == STDOUT_TAG || tag == STDERR_TAG {
            e@ == "observed output exceeds command bound"@
                && (if tag == STDOUT_TAG { stdout } else { stderr }) as int + length > limit
        } else if tag == TERMINAL_TAG {
            e@ == "oversize observed terminal record"@ && length > 2097152
        } else { e@ == "unknown observed command frame"@ }
    },
{
    match tag {
        STDOUT_TAG if length <= limit - stdout => Ok(ObservedFrame::Stdout(stdout + length)),
        STDERR_TAG if length <= limit - stderr => Ok(ObservedFrame::Stderr(stderr + length)),
        STDOUT_TAG | STDERR_TAG => Err("observed output exceeds command bound"),
        TERMINAL_TAG if length <= 2 * 1024 * 1024 => Ok(ObservedFrame::Terminal(length)),
        TERMINAL_TAG => Err("oversize observed terminal record"),
        _ => Err("unknown observed command frame"),
    }
}

/// The first byte after a terminal record must be EOF. The lengths and SHA-256 strings
/// observed at the reader must agree with those declared by the producer.
pub fn observed_stream_matches(
    trailing: usize, declared_out: usize, actual_out: usize, declared_err: usize, actual_err: usize,
    declared_out_sha: &str, actual_out_sha: &str, declared_err_sha: &str, actual_err_sha: &str,
) -> (ok: bool)
    ensures ok == (trailing == 0 && declared_out == actual_out && declared_err == actual_err
        && declared_out_sha@ == actual_out_sha@ && declared_err_sha@ == actual_err_sha@),
{
    trailing == 0 && declared_out == actual_out && declared_err == actual_err
        && equal_text(declared_out_sha, actual_out_sha)
        && equal_text(declared_err_sha, actual_err_sha)
}

pub fn observed_process_matches(launcher: Option<u32>, expected: u32, child: Option<u32>,
    digest: &str, expected_digest: &str) -> (ok: bool)
    ensures ok == (launcher == Some(expected) && child.is_some() && child != Some(0u32)
        && digest@ == expected_digest@),
{
    matches!(launcher, Some(pid) if pid == expected) && matches!(child, Some(pid) if pid != 0)
        && equal_text(digest, expected_digest)
}

}

#[cfg(not(verus_keep_ghost))]
use crate::files::sha256;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};

#[cfg(not(verus_keep_ghost))]
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
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
        let (stream, end) =
            match observed_frame(header[0], size, stdout.len(), stderr.len(), limit)? {
                ObservedFrame::Stdout(end) => (&mut stdout, end),
                ObservedFrame::Stderr(end) => (&mut stderr, end),
                ObservedFrame::Terminal(size) => {
                    let mut bytes = vec![0u8; size];
                    input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                    let mut p: Observation = crate::decode(&bytes)?;
                    let trailing = input.read(&mut [0]).map_err(|e| e.to_string())?;
                    if !observed_stream_matches(
                        trailing,
                        p.stdout_size,
                        stdout.len(),
                        p.stderr_size,
                        stderr.len(),
                        &p.stdout_sha256,
                        &sha256(&stdout),
                        &p.stderr_sha256,
                        &sha256(&stderr),
                    ) {
                        return Err("observed command stream identity differs".into());
                    }
                    p.stdout_bytes = stdout;
                    p.stderr_bytes = stderr;
                    return Ok(p);
                }
            };
        let offset = stream.len();
        stream.resize(end, 0);
        input
            .read_exact(&mut stream[offset..])
            .map_err(|e| e.to_string())?;
    }
}

/// Use the registered program with the source process's input, deadline and ownership descriptors.
#[cfg(not(verus_keep_ghost))]
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
    let wait = child
        .wait()
        .map(|status| status.success())
        .map_err(|e| e.to_string());
    let diagnostics = diagnostics
        .map_err(|_| "observed diagnostic reader panicked".to_owned())
        .and_then(|read| read.map_err(|e| e.to_string()));
    let written = sent
        .map_err(|_| "observed input writer panicked".to_owned())
        .and_then(|write| write);
    let p =
        finish_observation(wait, diagnostics, written, received).map_err(
            |failure| match failure {
                ObservedFailure::Transport(error) => error,
                ObservedFailure::Launcher(bytes) => {
                    String::from_utf8_lossy(&bytes).trim_end().into()
                }
            },
        )?;
    if !observed_process_matches(p.launcher_pid, launcher, p.child_pid, &p.sha256, digest) {
        return Err("observed process identity differs".into());
    }
    Ok(p)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn installed(root: &Path) -> Result<PathBuf, String> {
    let local = root.join(".chrono-harness/bin/chrono-harness");
    if local.is_file() {
        return Ok(local);
    }
    crate::files::resolve_program(root, "chrono-harness", None)
}
