mod cli;

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, ResourcesCommand, WorkflowCommand};
use zig_core::resources_manage::{ResourceScope, ResourceTarget};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            workflow,
            prompt,
            no_resources,
        } => {
            zig_core::run::run_workflow(&workflow, prompt.as_deref(), no_resources)?;
        }
        Command::Resources { command } => match command {
            ResourcesCommand::List {
                workflow,
                global,
                cwd,
            } => {
                let scope = ResourceScope::from_flags(global, cwd);
                zig_core::resources_manage::list_resources(workflow.as_deref(), scope)?;
            }
            ResourcesCommand::Add {
                file,
                workflow,
                global,
                cwd,
                name,
            } => {
                let target = ResourceTarget::from_flags(workflow.as_deref(), global, cwd)?;
                zig_core::resources_manage::add_resource(&file, target, name.as_deref())?;
            }
            ResourcesCommand::Remove {
                name,
                workflow,
                global,
                cwd,
            } => {
                let target = ResourceTarget::from_flags(workflow.as_deref(), global, cwd)?;
                zig_core::resources_manage::remove_resource(&name, target)?;
            }
            ResourcesCommand::Show { name, workflow } => {
                zig_core::resources_manage::show_resource(&name, workflow.as_deref())?;
            }
            ResourcesCommand::Where { workflow } => {
                zig_core::resources_manage::print_search_paths(workflow.as_deref())?;
            }
        },
        Command::Workflow { command } => match command {
            WorkflowCommand::List { json } => {
                if json {
                    let infos = zig_core::manage::get_workflow_list()?;
                    println!("{}", serde_json::to_string_pretty(&infos)?);
                } else {
                    zig_core::manage::list_workflows()?;
                }
            }
            WorkflowCommand::Show { workflow } => {
                zig_core::manage::show_workflow(&workflow)?;
            }
            WorkflowCommand::Delete { workflow } => {
                zig_core::manage::delete_workflow(&workflow)?;
            }
            WorkflowCommand::Create {
                name,
                output,
                pattern,
            } => {
                zig_core::create::run_create(
                    name.as_deref(),
                    output.as_deref(),
                    pattern.as_ref().map(|p| p.as_core_name()),
                )?;
            }
            WorkflowCommand::Pack { path, output } => {
                zig_core::pack::pack(&path, output.as_deref())?;
            }
        },
        Command::Describe { prompt, output } => {
            let dest = output.unwrap_or_else(|| "workflow.zug".to_string());
            println!(
                "zig describe: generating '{dest}' from prompt '{prompt}' (not yet implemented)"
            );
        }
        Command::Validate { workflow } => {
            let path = Path::new(&workflow);
            let (wf, _source) = zig_core::workflow::parser::parse_workflow(path)?;

            match zig_core::workflow::validate::validate(&wf) {
                Ok(()) => {
                    println!(
                        "workflow '{}' is valid ({} steps)",
                        wf.workflow.name,
                        wf.steps.len()
                    );
                }
                Err(errors) => {
                    eprintln!(
                        "workflow '{}' has {} error(s):",
                        wf.workflow.name,
                        errors.len()
                    );
                    for e in &errors {
                        eprintln!("  - {e}");
                    }
                    std::process::exit(1);
                }
            }
        }
        Command::Init => {
            println!("zig init: initializing project (not yet implemented)");
        }
        Command::Serve {
            port,
            host,
            token,
            shutdown_timeout,
            tls,
            tls_cert,
            tls_key,
            rate_limit,
            web,
        } => {
            // Precedence: CLI flag > env var > ~/.zig/serve.toml > default.
            let file = zig_serve::config::FileConfig::load();
            let s = &file.server;

            let host = host
                .or_else(|| s.host.clone())
                .unwrap_or_else(|| "127.0.0.1".into());
            let port = port.or(s.port).unwrap_or(3000);
            let token = token
                .or_else(|| std::env::var("ZIG_SERVE_TOKEN").ok())
                .or_else(|| s.token.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let shutdown_timeout = shutdown_timeout.or(s.shutdown_timeout).unwrap_or(30);
            let tls_cert = tls_cert.or_else(|| s.tls_cert.clone());
            let tls_key = tls_key.or_else(|| s.tls_key.clone());
            let tls = tls || s.tls || tls_cert.is_some();
            let rate_limit = rate_limit.or(s.rate_limit);
            let web = web
                || s.web
                || matches!(std::env::var("ZIG_SERVE_WEB").as_deref(), Ok("1" | "true"));

            let config = zig_serve::config::ServeConfig {
                host,
                port,
                token,
                shutdown_timeout: std::time::Duration::from_secs(shutdown_timeout),
                tls,
                tls_cert,
                tls_key,
                rate_limit,
                web,
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(zig_serve::start_server(config))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Command::Listen {
            session_id,
            latest,
            active,
        } => {
            let selector = if let Some(id) = session_id {
                zig_core::listen::SessionSelector::Id(id)
            } else if active {
                zig_core::listen::SessionSelector::Active
            } else if latest {
                zig_core::listen::SessionSelector::Latest
            } else {
                // Default to --latest when no flag is provided.
                zig_core::listen::SessionSelector::Latest
            };
            zig_core::listen::listen(selector, zig_core::listen::ListenOptions::default())?;
        }
        Command::Man { topic } => {
            if let Some(topic) = topic {
                match zig_core::man::get(&topic) {
                    Some(content) => print!("{content}"),
                    None => {
                        eprintln!("unknown manpage topic: '{topic}'\n");
                        eprintln!("{}", zig_core::man::list_topics());
                        std::process::exit(1);
                    }
                }
            } else {
                println!("{}", zig_core::man::list_topics());
            }
        }
    }

    Ok(())
}
