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

        /// Additional context prompt injected into workflow steps
        prompt: Option<String>,
    },

    /// Manage workflows (list, show, create, delete)
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
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

    /// Initialize a new zig project in the current directory
    Init,

    /// Show manual pages for zig topics
    Man {
        /// Topic to display (e.g., run, zug, patterns). Omit to list all topics.
        topic: Option<String>,
    },

    /// Start an HTTP API server
    Serve {
        /// Port to listen on
        #[arg(long, short, default_value = "3000")]
        port: u16,

        /// Host/IP to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Bearer token for authentication (or set ZIG_SERVE_TOKEN env var)
        #[arg(long)]
        token: Option<String>,

        /// Graceful shutdown timeout in seconds
        #[arg(long, default_value = "30")]
        shutdown_timeout: u64,

        /// Enable TLS with auto-generated self-signed certificates
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate PEM file (implies --tls)
        #[arg(long)]
        tls_cert: Option<String>,

        /// Path to TLS private key PEM file (implies --tls)
        #[arg(long)]
        tls_key: Option<String>,

        /// Rate limit in requests per second (e.g., 100)
        #[arg(long)]
        rate_limit: Option<u64>,
    },

    /// Tail a running or completed zig session
    Listen {
        /// Session id (full UUID or unique prefix). Omit with --latest/--active.
        session_id: Option<String>,

        /// Tail the most recently started session
        #[arg(long, conflicts_with_all = ["session_id", "active"])]
        latest: bool,

        /// Tail the most recently active (still-running) session
        #[arg(long, conflicts_with_all = ["session_id", "latest"])]
        active: bool,
    },
}

/// Subcommands for `zig workflow`.
#[derive(Subcommand)]
pub enum WorkflowCommand {
    /// List available workflows
    List,

    /// Show details of a workflow
    Show {
        /// Name or path of the workflow to show
        workflow: String,
    },

    /// Delete a workflow file
    Delete {
        /// Name or path of the workflow to delete
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

    /// Pack a workflow directory into a .zug zip archive
    Pack {
        /// Path to directory containing the workflow and its prompt files
        path: String,

        /// Output file path (defaults to <workflow-name>.zug)
        #[arg(long, short)]
        output: Option<String>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_run_command() {
        let cli = Cli::try_parse_from(["zig", "run", "my-workflow"]).unwrap();
        match cli.command {
            Command::Run { workflow, prompt } => {
                assert_eq!(workflow, "my-workflow");
                assert!(prompt.is_none());
            }
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn parse_run_with_prompt() {
        let cli = Cli::try_parse_from(["zig", "run", "wf", "extra context"]).unwrap();
        match cli.command {
            Command::Run { prompt, .. } => assert_eq!(prompt.as_deref(), Some("extra context")),
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn parse_workflow_create_with_pattern() {
        let cli =
            Cli::try_parse_from(["zig", "workflow", "create", "my-wf", "--pattern", "fan-out"])
                .unwrap();
        match cli.command {
            Command::Workflow {
                command:
                    WorkflowCommand::Create {
                        name,
                        pattern,
                        output,
                    },
            } => {
                assert_eq!(name.as_deref(), Some("my-wf"));
                assert!(matches!(pattern, Some(Pattern::FanOut)));
                assert!(output.is_none());
            }
            _ => panic!("expected Workflow Create command"),
        }
    }

    #[test]
    fn parse_workflow_create_no_args() {
        let cli = Cli::try_parse_from(["zig", "workflow", "create"]).unwrap();
        match cli.command {
            Command::Workflow {
                command:
                    WorkflowCommand::Create {
                        name,
                        output,
                        pattern,
                    },
            } => {
                assert!(name.is_none());
                assert!(output.is_none());
                assert!(pattern.is_none());
            }
            _ => panic!("expected Workflow Create command"),
        }
    }

    #[test]
    fn parse_workflow_list() {
        let cli = Cli::try_parse_from(["zig", "workflow", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Workflow {
                command: WorkflowCommand::List
            }
        ));
    }

    #[test]
    fn parse_workflow_show() {
        let cli = Cli::try_parse_from(["zig", "workflow", "show", "my-wf"]).unwrap();
        match cli.command {
            Command::Workflow {
                command: WorkflowCommand::Show { workflow },
            } => assert_eq!(workflow, "my-wf"),
            _ => panic!("expected Workflow Show command"),
        }
    }

    #[test]
    fn parse_workflow_delete() {
        let cli = Cli::try_parse_from(["zig", "workflow", "delete", "my-wf"]).unwrap();
        match cli.command {
            Command::Workflow {
                command: WorkflowCommand::Delete { workflow },
            } => assert_eq!(workflow, "my-wf"),
            _ => panic!("expected Workflow Delete command"),
        }
    }

    #[test]
    fn parse_validate_command() {
        let cli = Cli::try_parse_from(["zig", "validate", "test.zug"]).unwrap();
        match cli.command {
            Command::Validate { workflow } => assert_eq!(workflow, "test.zug"),
            _ => panic!("expected Validate command"),
        }
    }

    #[test]
    fn parse_man_with_topic() {
        let cli = Cli::try_parse_from(["zig", "man", "zug"]).unwrap();
        match cli.command {
            Command::Man { topic } => assert_eq!(topic.as_deref(), Some("zug")),
            _ => panic!("expected Man command"),
        }
    }

    #[test]
    fn parse_man_without_topic() {
        let cli = Cli::try_parse_from(["zig", "man"]).unwrap();
        match cli.command {
            Command::Man { topic } => assert!(topic.is_none()),
            _ => panic!("expected Man command"),
        }
    }

    #[test]
    fn parse_listen_with_id() {
        let cli = Cli::try_parse_from(["zig", "listen", "abc123"]).unwrap();
        match cli.command {
            Command::Listen {
                session_id,
                latest,
                active,
            } => {
                assert_eq!(session_id.as_deref(), Some("abc123"));
                assert!(!latest);
                assert!(!active);
            }
            _ => panic!("expected Listen command"),
        }
    }

    #[test]
    fn parse_listen_latest() {
        let cli = Cli::try_parse_from(["zig", "listen", "--latest"]).unwrap();
        match cli.command {
            Command::Listen {
                session_id,
                latest,
                active,
            } => {
                assert!(session_id.is_none());
                assert!(latest);
                assert!(!active);
            }
            _ => panic!("expected Listen command"),
        }
    }

    #[test]
    fn parse_listen_active_conflicts_with_latest() {
        let result = Cli::try_parse_from(["zig", "listen", "--latest", "--active"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_global_flags() {
        let cli = Cli::try_parse_from(["zig", "--debug", "--quiet", "workflow", "list"]).unwrap();
        assert!(cli.debug);
        assert!(cli.quiet);
    }

    #[test]
    fn all_patterns_have_core_names() {
        let patterns = [
            Pattern::Sequential,
            Pattern::FanOut,
            Pattern::GeneratorCritic,
            Pattern::CoordinatorDispatcher,
            Pattern::HierarchicalDecomposition,
            Pattern::HumanInTheLoop,
            Pattern::InterAgentCommunication,
        ];
        for p in &patterns {
            let name = p.as_core_name();
            assert!(!name.is_empty());
            assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }
}
