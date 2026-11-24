//! Blocking JSON lines connection to a single native runner capability.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::Path,
    process::{Child, ChildStdout, Stdio},
};

pub(crate) struct Session {
    child: Child,
    pub(crate) socket: Option<UnixStream>,
    output: BufReader<ChildStdout>,
    name: &'static str,
}
impl Session {
    pub fn open(root: &Path, program: &Path, name: &'static str) -> Result<Self, String> {
        let (socket, input) = UnixStream::pair().map_err(|e| e.to_string())?;
        let mut command = crate::storage::command(program, program)?;
        command
            .arg(name)
            .current_dir(root)
            .stdin(Stdio::from(OwnedFd::from(input)))
            .stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| format!("{} {name}: {e}", program.display()))?;
        let output = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            socket: Some(socket),
            output,
            name,
        })
    }
    pub(crate) fn query<T: DeserializeOwned>(
        root: &Path,
        capability: &'static str,
        input: Value,
    ) -> Result<T, String> {
        let runner = crate::observed::installed(root)?;
        let mut session = Self::open(root, &runner, capability)?;
        let response = session.call(input)?;
        session.close()?;
        Ok(response)
    }

    pub(crate) fn selected_config(root: &Path, path: &str) -> Result<Value, String> {
        Self::one(root, "selected-host-config-v1", &path)
    }
    pub fn one<T: DeserializeOwned>(
        root: &Path,
        operation: &str,
        input: &impl serde::Serialize,
    ) -> Result<T, String> {
        let encoded = serde_json::to_vec(input).map_err(|e|e.to_string())?;
        let program = crate::observed::installed(root)?;
        let mut child = crate::storage::command(&program, &program)?
            .arg(operation)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().unwrap();
        // Drain output while sending large requests even when the peer exits early.
        let (sent, output) = std::thread::scope(|scope| {
            let writer = scope.spawn(move || {
                let mut stdin = stdin;
                stdin.write_all(&encoded)
            });
            let output = child.wait_with_output().map_err(|e| e.to_string());
            Ok::<_, String>((
                writer.join().map_err(|_| "runner input writer panicked")?,
                output?,
            ))
        })?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim_end().into());
        }
        sent.map_err(|e| e.to_string())?;
        crate::codec::decode(&output.stdout)
    }

    pub fn call<T: DeserializeOwned>(&mut self, command: Value) -> Result<T, String> {
        let stdin = self.socket.as_mut().unwrap();
        serde_json::to_writer(&mut *stdin, &command).map_err(|e| e.to_string())?;
        stdin
            .write_all(b"\n")
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string())?;
        let mut line = Vec::new();
        self.output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if line.last() != Some(&b'\n') {
            return Err("E_PROTOCOL: incomplete runner response".into());
        }
        serde_json::from_slice::<Result<T, String>>(&line)
            .map_err(|e| format!("E_PROTOCOL: {e}"))?
    }
    pub fn close(mut self) -> Result<(), String> {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{} exited {status}", self.name))
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        let _ = self.child.wait();
    }
}
