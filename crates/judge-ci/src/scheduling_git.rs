//! Original fixed registry endpoints used for full scheduling.
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) values: BTreeMap<String, Value>,
    pub(crate) effective_path: String,
    #[serde(default)]
    pub(crate) originals: BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Inputs {
    pub(crate) previous: Snapshot,
    pub(crate) candidate: Snapshot,
    pub(crate) observation: Value,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn read(
    root: &Path,
    profile: &str,
    base: &str,
    candidate: &str,
) -> Result<Inputs, String> {
    let mut child = Command::new("chrono-harness")
        .current_dir(root)
        .arg("git-facts-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("start chrono-harness git-facts-v1: {e}"))?;
    let input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let result = collect(input, BufReader::new(output), profile, base, candidate);
    let status = child.wait().map_err(|e| e.to_string())?;
    let inputs = result?;
    if !status.success() {
        return Err(format!("chrono-harness git-facts-v1 {status}"));
    }
    Ok(inputs)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn collect(
    mut input: impl Write,
    mut output: impl BufRead,
    profile: &str,
    base: &str,
    candidate: &str,
) -> Result<Inputs, String> {
    #[derive(Deserialize)]
    struct Open {
        bound: bool,
    }

    let open: Open = exchange(
        &mut input,
        &mut output,
        json!({"mode":"config","config":profile,"base":base,"candidate":candidate}),
    )?;
    let verify = json!({"op":"verify-config","candidate":candidate});
    exchange::<()>(&mut input, &mut output, verify.clone())?;
    for endpoint in ["base", "candidate"] {
        exchange::<String>(
            &mut input,
            &mut output,
            json!({"op":"verify-tree","endpoint":endpoint}),
        )?;
    }
    let mut snapshot = |endpoint| {
        exchange(
            &mut input,
            &mut output,
            json!({"op":"registry-snapshot","endpoint":endpoint}),
        )
    };
    let previous = snapshot("base")?;
    let candidate = snapshot("candidate")?;
    exchange::<()>(&mut input, &mut output, verify)?;
    let observation: Value = exchange(&mut input, &mut output, json!({"op":"observation"}))?;
    drop(input);
    let trailing = !output.fill_buf().map_err(|e| e.to_string())?.is_empty();
    if let Some(error) = terminal_error(trailing, open.bound, observation.is_null()) {
        return Err(error.into());
    }
    Ok(Inputs {
        previous,
        candidate,
        observation,
    })
}

#[cfg(not(verus_keep_ghost))]
fn exchange<T: DeserializeOwned>(
    input: &mut impl Write,
    output: &mut impl BufRead,
    request: Value,
) -> Result<T, String> {
    serde_json::to_writer(&mut *input, &request).map_err(|e| e.to_string())?;
    input
        .write_all(b"\n")
        .and_then(|()| input.flush())
        .map_err(|e| e.to_string())?;
    let mut frame = Vec::new();
    output
        .read_until(b'\n', &mut frame)
        .map_err(|e| e.to_string())?;
    if !crate::codec::complete_line(&frame) {
        return Err("E_PROTOCOL: incomplete Git facts response".into());
    }
    serde_json::from_slice::<Result<T, String>>(&frame).map_err(|e| e.to_string())?
}

#[cfg(not(verus_keep_ghost))]
struct Connection {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}
#[cfg(not(verus_keep_ghost))]
impl Connection {
    fn close(&mut self) -> Result<(), String> {
        self.input.take();
        let mut trailing = Vec::new();
        self.output
            .read_to_end(&mut trailing)
            .map_err(|e| e.to_string())?;
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("chrono-harness git-facts-v1 {status}"));
        }
        if !trailing.is_empty() {
            return Err("E_PROTOCOL: trailing Git facts reply".into());
        }
        Ok(())
    }
    fn call<T: DeserializeOwned>(&mut self, frame: Value) -> Result<T, String> {
        exchange(
            self.input.as_mut().ok_or("Git facts input closed")?,
            &mut self.output,
            frame,
        )
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Connection {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.wait();
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Default)]
pub(crate) struct Reader {
    root: PathBuf,
    base: Option<String>,
    candidate: String,
    connection: Option<RefCell<Connection>>,
}
#[cfg(not(verus_keep_ghost))]
impl Reader {
    pub fn open(
        root: &Path,
        base: Option<&str>,
        candidate: &str,
        header: Value,
    ) -> Result<(Self, Value), String> {
        let local = root.join(".chrono-harness/bin/chrono-harness");
        let program = if local.is_file() {
            local.as_path()
        } else {
            Path::new("chrono-harness")
        };
        let mut command = Command::new(program);
        command
            .arg("exec-with-descriptors-v1")
            .arg(program)
            .arg("git-facts-v1")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| format!("start git-facts-v1: {e}"))?;
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut connection = Connection {
            child,
            input,
            output,
        };
        let header = connection.call(header)?;
        Ok((
            Self {
                root: root.into(),
                base: base.map(str::to_owned),
                candidate: candidate.into(),
                connection: Some(RefCell::new(connection)),
            },
            header,
        ))
    }
    pub(crate) fn call<T: DeserializeOwned>(&self, root: &Path, frame: Value) -> Result<T, String> {
        if self.root != root {
            return Err("E_GIT_FACTS: unexpected root".into());
        }
        self.connection
            .as_ref()
            .ok_or("Git facts session not open")?
            .borrow_mut()
            .call(frame)
    }
    fn endpoint(&self, oid: &str) -> Result<&'static str, String> {
        if oid == self.candidate {
            Ok("candidate")
        } else if Some(oid) == self.base.as_deref() {
            Ok("base")
        } else {
            Err("E_GIT_FACTS: unexpected commit".into())
        }
    }
    pub fn verify_config(&self, root: &Path, candidate: &str) -> Result<(), String> {
        self.call(root, json!({"op":"verify-config","candidate":candidate}))
    }
    pub fn verify_oid(&self, root: &Path, oid: &str) -> Result<String, String> {
        self.call(
            root,
            json!({"op":"verify-tree","endpoint":self.endpoint(oid)?}),
        )
    }
    pub fn registry_snapshot(
        &self,
        root: &Path,
        oid: &str,
        config: &str,
    ) -> Result<Snapshot, String> {
        self.call(
            root,
            json!({"op":"registry-at","endpoint":self.endpoint(oid)?,"config":config}),
        )
    }
    pub fn blob(&self, root: &Path, oid: &str, path: &str) -> Result<Vec<u8>, String> {
        self.endpoint(oid)?;
        self.call(root, json!({"op":"blob-bytes","commit":oid,"path":path}))
    }
    pub fn tree(&self, root: &Path, oid: &str) -> Result<BTreeMap<String, Entry>, String> {
        self.call(
            root,
            json!({"op":"tree-entries","endpoint":self.endpoint(oid)?,"retain_output_modes":oid == self.candidate}),
        )
    }
    pub fn parents(&self, root: &Path, oid: &str) -> Result<Vec<String>, String> {
        self.endpoint(oid)?;
        self.call(root, json!({"op":"parents","commit":oid}))
    }
    pub fn checkout<T: DeserializeOwned>(
        &self,
        root: &Path,
        artifacts: &[String],
    ) -> Result<T, String> {
        self.call(
            root,
            json!({"op":"checkout-excluding","artifacts":artifacts}),
        )
    }
    pub fn observation(&self) -> Result<Value, String> {
        match self.connection {
            None => Ok(Value::Null),
            Some(_) => self.call(&self.root, json!({"op":"observation"})),
        }
    }
    pub fn close(self) -> Result<(), String> {
        if let Some(connection) = self.connection {
            connection.into_inner().close()?;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Value, String> {
        let facts = self.observation();
        self.close()?;
        facts
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(crate) struct Entry {
    pub mode: String,
    pub kind: String,
    pub oid: String,
}

use vstd::prelude::*;
verus! {
pub fn terminal_error(trailing: bool, bound: bool, null_facts: bool) -> (error: Option<&'static str>)
    ensures error == if trailing { Some("E_PROTOCOL: trailing Git facts response") }
        else if bound == null_facts { Some("E_GIT_FACTS: session returned a different binding") }
        else { None },
{
    if trailing { Some("E_PROTOCOL: trailing Git facts response") }
    else if bound == null_facts { Some("E_GIT_FACTS: session returned a different binding") }
    else { None }
}
}
