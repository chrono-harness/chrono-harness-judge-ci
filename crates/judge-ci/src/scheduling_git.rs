//! Original fixed registry endpoints used for full scheduling.
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
};

#[derive(Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) values: BTreeMap<String, Value>,
    pub(crate) effective_path: String,
}

pub(crate) struct Inputs {
    pub(crate) previous: Snapshot,
    pub(crate) candidate: Snapshot,
    pub(crate) observation: Value,
}

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

pub(crate) fn collect(
    mut input: impl Write,
    mut output: impl BufRead,
    profile: &str,
    base: &str,
    candidate: &str,
) -> Result<Inputs, String> {
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
        if frame.last() != Some(&b'\n') {
            return Err("E_PROTOCOL: incomplete Git facts response".into());
        }
        serde_json::from_slice::<Result<T, String>>(&frame).map_err(|e| e.to_string())?
    }
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
    if !output.fill_buf().map_err(|e| e.to_string())?.is_empty() {
        return Err("E_PROTOCOL: trailing Git facts response".into());
    }
    if open.bound == observation.is_null() {
        return Err("E_GIT_FACTS: session returned a different binding".into());
    }
    Ok(Inputs {
        previous,
        candidate,
        observation,
    })
}
