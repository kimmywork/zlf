use anyhow::Result;
use std::io::{self, BufRead, Write};

mod embed_commands;
mod handler;
mod io_data;
mod protocol;
mod repl;
mod server;
mod state;
mod values;

use handler::handle_request;
use protocol::{Request, Response};
use repl::run_repl;
use server::serve_http;
use state::AppState;

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "repl" {
        return run_repl(args.get(2).map(String::as_str));
    }

    if args.len() > 1 && args[1] == "serve" {
        let port = args
            .get(2)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8520);
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(serve_http(port))?;
        return Ok(());
    }

    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(line) {
            Ok(request) => {
                let state = AppState::empty();
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(handle_request(request, &state))
            }
            Err(e) => Response::Error {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Invalid JSON: {}", e),
            },
        };

        let mut out = stdout.lock();
        serde_json::to_writer(&mut out, &response)?;
        writeln!(out)?;
        out.flush()?;
    }

    Ok(())
}
