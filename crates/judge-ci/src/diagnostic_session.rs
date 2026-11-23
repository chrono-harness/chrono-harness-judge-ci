//! Stream events to the installed diagnostics command while the check owns its output sink.
use crate::trace_source::Source;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reply {
    Ready,
    Accepted,
    Write(String),
    Flush,
    Finished(Vec<Value>),
}

pub(super) fn failure(code: &str, message: impl Into<String>) -> Value {
    json!({"schema":"chrono-error/v1", "error":{
        "type":"chrono_judge_ci::diagnostic_session", "code":code,
        "message":message.into(), "context":{}, "source":null
    }})
}

struct Channel<I, O, W> {
    input: I,
    output: O,
    sink: W,
}

impl<I: Write, O: BufRead, W: Write> Channel<I, O, W> {
    fn send(&mut self, value: impl Serialize) -> io::Result<()> {
        serde_json::to_writer(&mut self.input, &value)?;
        self.input.write_all(b"\n")?;
        self.input.flush()
    }

    fn reply(&mut self) -> io::Result<Reply> {
        let mut line = String::new();
        loop {
            line.clear();
            if self.output.read_line(&mut line)? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "diagnostics closed stdout before its reply",
                ));
            }
            let result = match serde_json::from_str(&line)? {
                Reply::Write(record) => self.sink.write_all(record.as_bytes()),
                Reply::Flush => self.sink.flush(),
                reply => return Ok(reply),
            };
            let ack = match result {
                Ok(()) => json!("ok"),
                Err(e) => {
                    json!({"error":{"native_code":e.raw_os_error(), "kind":format!("{:?}", e.kind()), "custom_source":e.get_ref().is_some(), "message":e.to_string()}})
                }
            };
            self.send(ack)?;
        }
    }
}

pub(super) fn run<T>(
    root: &Path,
    program: &str,
    sink: impl Write + Send + 'static,
    errors: &mut Vec<Value>,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let start = || -> Result<_, String> {
        let path = chrono_harness::resolve_program(root, program, None)?;
        let mut command = Command::new(&path);
        command
            .current_dir(root)
            .arg("trace-events-v1")
            .arg(root)
            .arg("chrono-judge-ci")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        chrono_harness::process_fds::forward_command(&mut command, &[])?;
        command
            .spawn()
            .map_err(|e| format!("start {}: {e}", path.display()))
    };
    let mut child =
        start().inspect_err(|message| errors.push(failure("E_LOG_PROTOCOL", message)))?;
    let channel = Channel {
        input: BufWriter::new(child.stdin.take().unwrap()),
        output: BufReader::new(child.stdout.take().unwrap()),
        sink,
    };
    let stderr = child.stderr.take().unwrap();
    std::thread::scope(|scope| {
        let diagnostic = scope.spawn(|| io::read_to_string(stderr));
        let evaluation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session(channel, errors, work)
        }));
        let status = child.wait();
        let diagnostic = diagnostic.join().expect("diagnostics stderr reader");
        let (result, expected_status) =
            evaluation.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        match (status, diagnostic) {
            (Ok(status), Ok(text))
                if status.code() == expected_status
                    && text.is_empty()
                    && expected_status.is_some() =>
            {
                ()
            }
            (status, diagnostic) => errors.push(failure(
                "E_LOG_PROTOCOL",
                format!("diagnostics process {status:?}, stderr {diagnostic:?}"),
            )),
        }
        result
    })
}

fn session<I, O, W, T>(
    mut channel: Channel<I, O, W>,
    errors: &mut Vec<Value>,
    work: impl FnOnce() -> Result<T, String>,
) -> (Result<T, String>, Option<i32>)
where
    I: Write + Send + 'static,
    O: BufRead + Send + 'static,
    W: Write + Send + 'static,
{
    match channel.reply() {
        Ok(Reply::Ready) => (),
        Ok(Reply::Finished(records)) if !records.is_empty() => {
            let message = records[0]["error"]["message"]
                .as_str()
                .unwrap_or("diagnostics setup failed")
                .to_owned();
            errors.extend(records);
            return (Err(message), Some(2));
        }
        reply => {
            let message = reply
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "missing diagnostics ready reply".into());
            errors.push(failure("E_LOG_PROTOCOL", &message));
            return (Err(message), None);
        }
    }

    let state = Arc::new(Mutex::new((channel, Ok(None))));
    let writer = state.clone();
    let subscriber = Source::new(move |event| {
        let mut state = writer.lock().unwrap();
        let (channel, sent) = &mut *state;
        if matches!(sent, Ok(None)) {
            *sent = channel
                .send(json!({"event":event}))
                .and_then(|()| match channel.reply()? {
                    Reply::Accepted => Ok(None),
                    Reply::Finished(records) if !records.is_empty() => Ok(Some(records)),
                    _ => Err(io::Error::other("missing tracing event acknowledgment")),
                });
        }
    });
    let result = tracing::subscriber::with_default(subscriber, work);
    let mut state = state.lock().unwrap();
    let (channel, sent) = &mut *state;
    let finished = std::mem::replace(sent, Ok(None)).and_then(|terminal| {
        if let Some(records) = terminal {
            return Ok(records);
        }
        channel.send("finish")?;
        match channel.reply()? {
            Reply::Finished(records) => Ok(records),
            _ => Err(io::Error::other("missing diagnostics terminal reply")),
        }
    });
    let status = match finished {
        Ok(records) => {
            let status = if records.is_empty() { 0 } else { 2 };
            errors.extend(records);
            Some(status)
        }
        Err(e) => {
            errors.push(failure("E_LOG_PROTOCOL", e.to_string()));
            None
        }
    };
    (result, status)
}
