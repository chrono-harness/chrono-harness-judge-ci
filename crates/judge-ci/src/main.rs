use std::io::{Read, Write};
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
            let request = chrono_harness::decode::<chrono_harness::Request>(&bytes)?;
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
    result.unwrap_or_else(|e| {
        let code = if query { "E_REQUIREMENTS" } else { "E_REQUEST" };
        chrono_harness::log_cli_error(&format!("{code}: {e}"));
        eprintln!("{code}: {e}");
        2
    })
}

fn write_response(output: &mut impl Write, response: &impl serde::Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *output, response).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|_| output.flush())
        .map_err(|e| e.to_string())
}

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::logged_main("chrono-judge-ci", run))
}
