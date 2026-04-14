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
    /// Execute a .zwf/.zwfz workflow file
    Run {
        /// Name or path of the workflow to run
        workflow: String,

        /// Additional context prompt injected into workflow steps
        prompt: Option<String>,

        /// Disable the `<resources>` block injected into each step's system prompt
        #[arg(long)]
        no_resources: bool,

        /// Disable the `<memory>` block injected into each step's system prompt
        #[arg(long)]
        no_memory: bool,
    },

    /// Manage workflows (list, show, create, delete)
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },

    /// Manage knowledge / reference files (list, add, remove, show)
    Resources {
        #[command(subcommand)]
        command: ResourcesCommand,
    },

    /// Manage memory scratch pad files (add, update, delete, show, list, search)
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },

    /// Describe a workflow to an agent and generate a .zwf file
    Describe {
        /// Natural language description of the workflow
        prompt: String,

        /// Output file path (defaults to <name>.zwf)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Validate a .zwf/.zwfz workflow file
    Validate {
        /// Path to the .zwf or .zwfz file to validate
        workflow: String,
    },

    /// Initialize a new zig project in the current directory
    Init,

    /// Show manual pages for zig topics
    Man {
        /// Topic to display (e.g., run, zwf, patterns). Omit to list all topics.
        topic: Option<String>,
    },

    /// Start an HTTP API server
    ///
    /// Settings can also be stored in ~/.zig/serve.toml under a [server]
    /// section. Precedence: CLI flag > env var > config file > default.
    Serve {
        /// Port to listen on (default: 3000)
        #[arg(long, short)]
        port: Option<u16>,

        /// Host/IP to bind to (default: 127.0.0.1)
        #[arg(long)]
        host: Option<String>,

        /// Bearer token for authentication (or set ZIG_SERVE_TOKEN env var)
        #[arg(long)]
        token: Option<String>,

        /// Graceful shutdown timeout in seconds (default: 30)
        #[arg(long)]
        shutdown_timeout: Option<u64>,

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

        /// Serve the built-in React web UI from `/` alongside the API
        #[arg(long)]
        web: bool,
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
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

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

        /// Output file path (defaults to <name>.zwf or workflow.zwf)
        #[arg(long, short)]
        output: Option<String>,

        /// Orchestration pattern to use
        #[arg(long, short)]
        pattern: Option<Pattern>,
    },

    /// Update an existing workflow interactively with an AI agent
    Update {
        /// Name or path of the workflow to update
        workflow: String,
    },

    /// Pack a workflow directory into a .zwfz zip archive
    Pack {
        /// Path to directory containing the workflow and its prompt files
        path: String,

        /// Output file path (defaults to <workflow-name>.zwfz)
        #[arg(long, short)]
        output: Option<String>,
    },
}

/// Subcommands for `zig resources`.
#[derive(Subcommand)]
pub enum ResourcesCommand {
    /// List discovered resources from all tiers (or a single tier with a flag)
    List {
        /// Restrict the listing to a specific named workflow's global tier
        #[arg(long)]
        workflow: Option<String>,

        /// Show only the global tiers (~/.zig/resources)
        #[arg(long)]
        global: bool,

        /// Show only the project tier (./.zig/resources, walking up to git root)
        #[arg(long, conflicts_with = "global")]
        cwd: bool,
    },

    /// Add a resource file to one of the tiers
    Add {
        /// Path to the file to register as a resource
        file: String,

        /// Target a specific named workflow's global tier
        #[arg(long)]
        workflow: Option<String>,

        /// Place the resource in the global tier (~/.zig/resources)
        #[arg(long, conflicts_with = "cwd")]
        global: bool,

        /// Place the resource in the project tier (./.zig/resources)
        #[arg(long, conflicts_with = "global")]
        cwd: bool,

        /// Custom name to register the resource under (defaults to the file's basename)
        #[arg(long)]
        name: Option<String>,
    },

    /// Remove a resource by name from one of the tiers
    Remove {
        /// Name of the resource to remove (matches the registered file name)
        name: String,

        /// Target a specific named workflow's global tier
        #[arg(long)]
        workflow: Option<String>,

        /// Remove from the global tier (~/.zig/resources)
        #[arg(long, conflicts_with = "cwd")]
        global: bool,

        /// Remove from the project tier (./.zig/resources)
        #[arg(long, conflicts_with = "global")]
        cwd: bool,
    },

    /// Print the absolute path and contents of a resource by name
    Show {
        /// Name of the resource to show
        name: String,

        /// Restrict the search to a specific named workflow's global tier
        #[arg(long)]
        workflow: Option<String>,
    },

    /// Print the directories the collector would search for the current cwd
    Where {
        /// Print directories for a specific workflow name (affects ~/.zig/resources/<name>)
        #[arg(long)]
        workflow: Option<String>,
    },
}

/// Subcommands for `zig memory`.
#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Add a file to the memory scratch pad
    Add {
        /// Path to the file to add
        path: String,

        /// Target a specific named workflow's memory tier
        #[arg(long)]
        workflow: Option<String>,

        /// Associate this memory with a specific step (metadata only)
        #[arg(long)]
        step: Option<String>,

        /// Custom display name (defaults to the file's basename)
        #[arg(long)]
        name: Option<String>,

        /// Description of the memory entry
        #[arg(long)]
        description: Option<String>,

        /// Comma-separated tags
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// Update metadata for a memory entry
    Update {
        /// Numeric ID of the memory entry
        id: u64,

        /// Restrict search to a specific workflow tier
        #[arg(long)]
        workflow: Option<String>,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// New comma-separated tags (replaces existing)
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// Delete a memory entry and its file
    Delete {
        /// Numeric ID of the memory entry
        id: u64,

        /// Restrict search to a specific workflow tier
        #[arg(long)]
        workflow: Option<String>,
    },

    /// Show metadata and contents of a memory entry
    Show {
        /// Numeric ID of the memory entry
        id: u64,

        /// Restrict search to a specific workflow tier
        #[arg(long)]
        workflow: Option<String>,
    },

    /// List all memory entries
    List {
        /// Filter by workflow name
        #[arg(long)]
        workflow: Option<String>,
    },

    /// Search across memory files
    Search {
        /// Search query string
        query: String,

        /// Result granularity: sentence, paragraph, section, file
        #[arg(long, default_value = "sentence")]
        scope: String,

        /// Filter by workflow name
        #[arg(long)]
        workflow: Option<String>,
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
            Command::Run {
                workflow,
                prompt,
                no_resources,
                no_memory,
            } => {
                assert_eq!(workflow, "my-workflow");
                assert!(prompt.is_none());
                assert!(!no_resources);
                assert!(!no_memory);
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
    fn parse_run_with_no_resources() {
        let cli = Cli::try_parse_from(["zig", "run", "wf", "--no-resources"]).unwrap();
        match cli.command {
            Command::Run { no_resources, .. } => assert!(no_resources),
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn parse_resources_list() {
        let cli = Cli::try_parse_from(["zig", "resources", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Resources {
                command: ResourcesCommand::List { .. }
            }
        ));
    }

    #[test]
    fn parse_resources_add_with_workflow_and_global() {
        let cli = Cli::try_parse_from([
            "zig",
            "resources",
            "add",
            "./cv.md",
            "--workflow",
            "cover-letter",
            "--global",
        ])
        .unwrap();
        match cli.command {
            Command::Resources {
                command:
                    ResourcesCommand::Add {
                        file,
                        workflow,
                        global,
                        cwd,
                        name,
                    },
            } => {
                assert_eq!(file, "./cv.md");
                assert_eq!(workflow.as_deref(), Some("cover-letter"));
                assert!(global);
                assert!(!cwd);
                assert!(name.is_none());
            }
            _ => panic!("expected Resources Add command"),
        }
    }

    #[test]
    fn parse_resources_add_global_and_cwd_conflict() {
        let result =
            Cli::try_parse_from(["zig", "resources", "add", "./cv.md", "--global", "--cwd"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_resources_where() {
        let cli = Cli::try_parse_from(["zig", "resources", "where"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Resources {
                command: ResourcesCommand::Where { .. }
            }
        ));
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
                command: WorkflowCommand::List { json: false }
            }
        ));
    }

    #[test]
    fn parse_workflow_list_json() {
        let cli = Cli::try_parse_from(["zig", "workflow", "list", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Workflow {
                command: WorkflowCommand::List { json: true }
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
        let cli = Cli::try_parse_from(["zig", "validate", "test.zwf"]).unwrap();
        match cli.command {
            Command::Validate { workflow } => assert_eq!(workflow, "test.zwf"),
            _ => panic!("expected Validate command"),
        }
    }

    #[test]
    fn parse_workflow_update() {
        let cli = Cli::try_parse_from(["zig", "workflow", "update", "my-wf"]).unwrap();
        match cli.command {
            Command::Workflow {
                command: WorkflowCommand::Update { workflow },
            } => assert_eq!(workflow, "my-wf"),
            _ => panic!("expected Workflow Update command"),
        }
    }

    #[test]
    fn parse_man_with_topic() {
        let cli = Cli::try_parse_from(["zig", "man", "zwf"]).unwrap();
        match cli.command {
            Command::Man { topic } => assert_eq!(topic.as_deref(), Some("zwf")),
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
    fn parse_serve_with_web_flag() {
        let cli = Cli::try_parse_from(["zig", "serve", "--web"]).unwrap();
        match cli.command {
            Command::Serve { web, .. } => assert!(web),
            _ => panic!("expected Serve command"),
        }
    }

    #[test]
    fn parse_serve_without_web_flag_defaults_false() {
        let cli = Cli::try_parse_from(["zig", "serve"]).unwrap();
        match cli.command {
            Command::Serve { web, .. } => assert!(!web),
            _ => panic!("expected Serve command"),
        }
    }

    #[test]
    fn parse_run_with_no_memory() {
        let cli = Cli::try_parse_from(["zig", "run", "wf", "--no-memory"]).unwrap();
        match cli.command {
            Command::Run { no_memory, .. } => assert!(no_memory),
            _ => panic!("expected Run command"),
        }
    }

    #[test]
    fn parse_memory_add() {
        let cli = Cli::try_parse_from([
            "zig",
            "memory",
            "add",
            "./notes.md",
            "--workflow",
            "my-wf",
            "--step",
            "analysis",
            "--description",
            "Architecture notes",
            "--tags",
            "arch,design",
        ])
        .unwrap();
        match cli.command {
            Command::Memory {
                command:
                    MemoryCommand::Add {
                        path,
                        workflow,
                        step,
                        description,
                        tags,
                        ..
                    },
            } => {
                assert_eq!(path, "./notes.md");
                assert_eq!(workflow.as_deref(), Some("my-wf"));
                assert_eq!(step.as_deref(), Some("analysis"));
                assert_eq!(description.as_deref(), Some("Architecture notes"));
                assert_eq!(tags, vec!["arch", "design"]);
            }
            _ => panic!("expected Memory Add command"),
        }
    }

    #[test]
    fn parse_memory_update() {
        let cli = Cli::try_parse_from([
            "zig",
            "memory",
            "update",
            "42",
            "--description",
            "Updated desc",
            "--tags",
            "new",
        ])
        .unwrap();
        match cli.command {
            Command::Memory {
                command:
                    MemoryCommand::Update {
                        id,
                        description,
                        tags,
                        ..
                    },
            } => {
                assert_eq!(id, 42);
                assert_eq!(description.as_deref(), Some("Updated desc"));
                assert_eq!(tags, Some(vec!["new".to_string()]));
            }
            _ => panic!("expected Memory Update command"),
        }
    }

    #[test]
    fn parse_memory_delete() {
        let cli = Cli::try_parse_from(["zig", "memory", "delete", "3"]).unwrap();
        match cli.command {
            Command::Memory {
                command: MemoryCommand::Delete { id, .. },
            } => assert_eq!(id, 3),
            _ => panic!("expected Memory Delete command"),
        }
    }

    #[test]
    fn parse_memory_show() {
        let cli = Cli::try_parse_from(["zig", "memory", "show", "5"]).unwrap();
        match cli.command {
            Command::Memory {
                command: MemoryCommand::Show { id, .. },
            } => assert_eq!(id, 5),
            _ => panic!("expected Memory Show command"),
        }
    }

    #[test]
    fn parse_memory_list() {
        let cli = Cli::try_parse_from(["zig", "memory", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Memory {
                command: MemoryCommand::List { .. }
            }
        ));
    }

    #[test]
    fn parse_memory_search() {
        let cli = Cli::try_parse_from([
            "zig",
            "memory",
            "search",
            "architecture",
            "--scope",
            "section",
        ])
        .unwrap();
        match cli.command {
            Command::Memory {
                command: MemoryCommand::Search { query, scope, .. },
            } => {
                assert_eq!(query, "architecture");
                assert_eq!(scope, "section");
            }
            _ => panic!("expected Memory Search command"),
        }
    }

    #[test]
    fn parse_memory_search_default_scope() {
        let cli = Cli::try_parse_from(["zig", "memory", "search", "test"]).unwrap();
        match cli.command {
            Command::Memory {
                command: MemoryCommand::Search { scope, .. },
            } => assert_eq!(scope, "sentence"),
            _ => panic!("expected Memory Search command"),
        }
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
