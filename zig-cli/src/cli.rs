use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "zig",
    about = "Orchestration CLI for AI coding agents — describe, share, and run workflows powered by zag",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable debug logging
    #[arg(long, short, global = true)]
    pub debug: bool,

    /// Suppress all output except errors
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Execute a .zug workflow file
    Run {
        /// Name or path of the workflow to run
        workflow: String,
    },

    /// Describe a workflow to an agent and generate a .zug file
    Describe {
        /// Natural language description of the workflow
        prompt: String,

        /// Output file path (defaults to <name>.zug)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// List available workflows
    List,

    /// Initialize a new zig project in the current directory
    Init,
}
