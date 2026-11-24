//! Collector side of the installed runner's retained-original reader.
use crate::originals::Original;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::io::{BufRead, Write};

pub(super) fn write_frame(output: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|e| e.to_string())
}
pub(super) fn response<T: DeserializeOwned>(input: &mut impl BufRead) -> Result<T, String> {
    let mut line = Vec::new();
    input
        .read_until(b'\n', &mut line)
        .map_err(|e| e.to_string())?;
    if line.last() != Some(&b'\n') {
        return Err("E_PROTOCOL: incomplete runner response".into());
    }
    serde_json::from_slice(&line).map_err(|e| format!("E_PROTOCOL: {e}"))
}

/// A length is admitted by the runner before the payload enters the command stream.
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
        match (bytes, response::<Reply<T>>(input)?) {
            (_, Reply::Err(error)) => return Err(error),
            (Ok(bytes), Reply::Send(true)) => {
                output
                    .write_all(&bytes)
                    .and_then(|()| output.flush())
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err("E_PROTOCOL: invalid original length acknowledgement".into()),
        }
    }
}
