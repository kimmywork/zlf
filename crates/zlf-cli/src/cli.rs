use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zlf", about = "zlf graph database CLI")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CliCommand {
    /// Read JSON-over-STDIO requests, one request per line.
    Stdio,
    /// Start the Prolog REPL.
    Repl {
        /// Optional database path. Defaults to config/env db_path.
        db_path: Option<String>,
    },
    /// Start the HTTP server.
    Serve {
        /// HTTP port.
        #[arg(default_value_t = 8520)]
        port: u16,
    },
}
