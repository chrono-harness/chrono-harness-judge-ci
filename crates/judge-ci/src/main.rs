use vstd::prelude::*;

verus! {

pub fn finish<E, F>(result: Result<u8, E>, diagnostic: F) -> (exit: u8)
    where F: FnOnce(E) -> bool,
    requires result is Err ==> diagnostic.requires((result.unwrap_err(),)),
    ensures
        exit == match result { Ok(status) => status, Err(_) => 2u8 },
        result is Err ==> exists|written: bool|
            #[trigger] diagnostic.ensures((result.unwrap_err(),), written),
{
    match result {
        Ok(status) => status,
        Err(error) => {
            let _ = diagnostic(error);
            2
        }
    }
}
}

#[cfg(not(verus_keep_ghost))]
use std::io::{Read, Write};

#[cfg(not(verus_keep_ghost))]
pub fn write_diagnostic(output: &mut impl Write, code: &str, message: &str) -> bool {
    writeln!(output, "{code}: {message}").is_ok()
}

#[cfg(not(verus_keep_ghost))]
fn run() -> u8 {
    let mut bytes = Vec::new();
    let input = std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            if bytes.len() > 1024 * 1024 {
                Err("request too large".into())
            } else {
                Ok(bytes)
            }
        });
    let query = std::env::args_os().skip(1).eq(["requirements-v1"]);
    let stdout = std::io::stdout();
    let mut output = std::io::BufWriter::new(stdout.lock());
    let result = input.and_then(|bytes| {
        if query {
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let response = chrono_judge_ci::requirements::query(&root, &bytes)?;
            write_response(&mut output, &response)?;
            Ok(0)
        } else {
            let request = chrono_judge_ci::codec::decode::<chrono_judge_ci::wire::Request>(&bytes)?;
            let response = chrono_judge_ci::judge(&request);
            let failed = matches!(
                response.status,
                chrono_judge_ci::verified_collection::result_types::Status::Failed
                    | chrono_judge_ci::verified_collection::result_types::Status::Blocked
            );
            write_response(&mut output, &response)?;
            Ok(u8::from(failed))
        }
    });
    finish(result, |e| {
        let code = if query { "E_REQUIREMENTS" } else { "E_REQUEST" };
        tracing::error!(event_id="cli.diagnostic", message=%format!("{code}: {e}"));
        write_diagnostic(&mut std::io::stderr().lock(), code, &e)
    })
}

#[cfg(not(verus_keep_ghost))]
fn write_response(output: &mut impl Write, response: &impl serde::Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *output, response).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|_| output.flush())
        .map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_judge_ci::cli(run))
}
