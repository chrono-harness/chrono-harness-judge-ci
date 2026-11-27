//! Collector side of the installed runner's retained-original reader.
#[cfg(not(verus_keep_ghost))]
use crate::originals::Original;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::io::{BufRead, Write};

#[cfg(not(verus_keep_ghost))]
pub(super) fn write_frame(output: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|e| e.to_string())
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn response<T: DeserializeOwned>(input: &mut impl BufRead) -> Result<T, String> {
    let mut line = Vec::new();
    input
        .read_until(b'\n', &mut line)
        .map_err(|e| e.to_string())?;
    if !crate::codec::complete_line(&line) {
        return Err("E_PROTOCOL: incomplete runner response".into());
    }
    serde_json::from_slice(&line).map_err(|e| format!("E_PROTOCOL: {e}"))
}

/// A length is admitted by the runner before the payload enters the command stream.
#[cfg(not(verus_keep_ghost))]
pub(super) fn transfer<T: DeserializeOwned>(
    input: &mut impl BufRead,
    output: &mut impl Write,
    operation: &Value,
    read: &mut impl FnMut(&Original) -> Result<Vec<u8>, String>,
) -> Result<T, String> {
    #[derive(Deserialize)]
    enum Reply<T> {
        #[serde(rename = "read_original")]
        Read(Original),
        #[serde(rename = "send_original")]
        Send(bool),
        Ok(T),
        Err(String),
    }
    write_frame(output, operation)?;
    loop {
        let original = match response(input)? {
            Reply::Read(original) => original,
            Reply::Ok(value) => return Ok(value),
            Reply::Err(error) => return Err(error),
            Reply::Send(_) => return Err("E_PROTOCOL: unexpected original transfer".into()),
        };
        let bytes = read(&original);
        write_frame(output, &bytes.as_ref().map(Vec::len))?;
        let acknowledgement = match response::<Reply<T>>(input)? {
            Reply::Err(error) => Err(error),
            Reply::Send(allowed) => Ok(allowed),
            _ => Ok(false),
        };
        let bytes = admitted_bytes(bytes, acknowledgement).map_err(|error| {
            error
                .unwrap_or_else(|| "E_PROTOCOL: invalid original length acknowledgement".to_owned())
        })?;
        output
            .write_all(&bytes)
            .and_then(|()| output.flush())
            .map_err(|e| e.to_string())?;
    }
}

use vstd::prelude::*;
verus! {
/// The peer error takes precedence. Never send original bytes without an affirmative ack.
pub fn admitted_bytes<T, E>(read: Result<T, E>, ack: Result<bool, E>)
    -> (accepted: Result<T,Option<E>>)
    ensures accepted == match ack {
        Err(error) => Err(Some(error)),
        Ok(allowed) => match read { Ok(bytes) if allowed => Ok(bytes), _ => Err(None) },
    },
{
    match (read, ack) {
        (_, Err(error)) => Err(Some(error)),
        (Ok(bytes), Ok(true)) => Ok(bytes),
        _ => Err(None),
    }
}
}
