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
        let encoded = serde_json::to_vec(input).map_err(|e| e.to_string())?;
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

    fn write_frame(&mut self, value: &impl serde::Serialize) -> Result<(), String> {
        let input = self.socket.as_mut().ok_or("closed runner session")?;
        serde_json::to_writer(&mut *input, value).map_err(|e| e.to_string())?;
        input
            .write_all(b"\n")
            .and_then(|()| input.flush())
            .map_err(|e| e.to_string())
    }
    fn response<T: DeserializeOwned>(&mut self) -> Result<T, String> {
        let mut line = Vec::new();
        self.output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if line.last() != Some(&b'\n') {
            return Err("E_PROTOCOL: incomplete runner response".into());
        }
        serde_json::from_slice(&line).map_err(|e| format!("E_PROTOCOL: {e}"))
    }
    pub fn call<T: DeserializeOwned>(&mut self, command: Value) -> Result<T, String> {
        self.write_frame(&command)?;
        self.response::<Result<T, String>>()?
    }

    /// Supply only the originals requested by the owner. The runner admits the
    /// byte count before a binary payload can enter the command stream.
    pub(crate) fn originals<T: DeserializeOwned>(
        &mut self,
        operation: &Value,
        read: &mut impl FnMut(&crate::originals::Original) -> Result<Vec<u8>, String>,
    ) -> Result<T, String> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Reply<T> {
            Read {
                read_original: crate::originals::Original,
            },
            Send {
                send_original: bool,
            },
            Complete(Result<T, String>),
        }
        self.write_frame(operation)?;
        let mut pending: Option<Vec<u8>> = None;
        loop {
            match self.response::<Reply<T>>()? {
                Reply::Complete(result) => return result,
                Reply::Read { read_original } if pending.is_none() => {
                    let bytes = read(&read_original);
                    self.write_frame(&bytes.as_ref().map(Vec::len))?;
                    pending = bytes.ok();
                }
                Reply::Send {
                    send_original: true,
                } if pending.is_some() => {
                    let bytes = pending.take().unwrap();
                    let input = self.socket.as_mut().ok_or("closed runner session")?;
                    input
                        .write_all(&bytes)
                        .and_then(|()| input.flush())
                        .map_err(|e| e.to_string())?;
                }
                _ => return Err("E_PROTOCOL: invalid original transfer order".into()),
            }
        }
    }
    pub fn close(mut self) -> Result<(), String> {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        std::io::copy(&mut self.output, &mut std::io::sink()).map_err(|e| e.to_string())?;
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
        let _ = std::io::copy(&mut self.output, &mut std::io::sink());
        let _ = self.child.wait();
    }
}
