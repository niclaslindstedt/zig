mod cli;

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, WorkflowCommand};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { workflow, prompt } => {
            zig_core::run::run_workflow(&workflow, prompt.as_deref())?;
        }
        Command::Workflow { command } => match command {
            WorkflowCommand::List => {
                zig_core::manage::list_workflows()?;
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
        },
        Command::Describe { prompt, output } => {
            let dest = output.unwrap_or_else(|| "workflow.zug".to_string());
            println!(
                "zig describe: generating '{dest}' from prompt '{prompt}' (not yet implemented)"
            );
        }
        Command::Validate { workflow } => {
            let path = Path::new(&workflow);
            let wf = zig_core::workflow::parser::parse_file(path)?;

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
