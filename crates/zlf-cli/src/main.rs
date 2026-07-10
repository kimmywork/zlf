use anyhow::Result;
use clap::Parser;
use std::io::{self, BufRead, Write};

mod cli;
mod embed_commands;
mod handler;
mod io_data;
mod protocol;
mod repl;
mod server;
mod state;
mod values;

use cli::{Cli, CliCommand};
use handler::handle_request;
use protocol::{Request, Response};
use repl::run_repl;
use server::serve_http;
use state::AppState;

fn main() -> Result<()> {
    match Cli::parse().command.unwrap_or(CliCommand::Stdio) {
        CliCommand::Stdio => run_stdio(),
        CliCommand::Repl { db_path } => run_repl(db_path.as_deref()),
        CliCommand::Serve { port } => run_server(port),
    }
}

fn run_server(port: u16) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(serve_http(port))?;
    Ok(())
}

fn run_stdio() -> Result<()> {
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
