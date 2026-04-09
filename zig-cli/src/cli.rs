use clap::{Parser, Subcommand, ValueEnum};

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

    /// Create a new workflow interactively with an AI agent
    Create {
        /// Workflow name
        name: Option<String>,

        /// Output file path (defaults to <name>.zug or workflow.zug)
        #[arg(long, short)]
        output: Option<String>,

        /// Orchestration pattern to use
        #[arg(long, short)]
        pattern: Option<Pattern>,
    },

    /// Describe a workflow to an agent and generate a .zug file
    Describe {
        /// Natural language description of the workflow
        prompt: String,

        /// Output file path (defaults to <name>.zug)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Validate a .zug workflow file
    Validate {
        /// Path to the .zug file to validate
        workflow: String,
    },

    /// List available workflows
    List,

    /// Initialize a new zig project in the current directory
    Init,
}

/// Orchestration pattern for workflow creation.
#[derive(Clone, Debug, ValueEnum)]
pub enum Pattern {
    /// Steps run in order, each feeding the next
    Sequential,
    /// Parallel independent steps, then synthesize
    FanOut,
    /// Generate, evaluate, iterate until quality threshold
    GeneratorCritic,
    /// Classify input, route to specialized handlers
    CoordinatorDispatcher,
    /// Break down into sub-tasks, delegate, synthesize
    HierarchicalDecomposition,
    /// Automated steps with human approval gates
    HumanInTheLoop,
    /// Agents collaborate via shared variables
    InterAgentCommunication,
}

impl Pattern {
    /// Return the kebab-case identifier used by zig-core.
    pub fn as_core_name(&self) -> &'static str {
        match self {
            Pattern::Sequential => "sequential",
            Pattern::FanOut => "fan-out",
            Pattern::GeneratorCritic => "generator-critic",
            Pattern::CoordinatorDispatcher => "coordinator-dispatcher",
            Pattern::HierarchicalDecomposition => "hierarchical-decomposition",
            Pattern::HumanInTheLoop => "human-in-the-loop",
            Pattern::InterAgentCommunication => "inter-agent-communication",
        }
    }
}
