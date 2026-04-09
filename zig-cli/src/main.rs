mod cli;

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { workflow, prompt } => {
            zig_core::run::run_workflow(&workflow, prompt.as_deref())?;
        }
        Command::Create { name, output } => {
            zig_core::create::run_create(name.as_deref(), output.as_deref())?;
        }
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
        Command::List => {
            println!("zig list: listing available workflows (not yet implemented)");
        }
        Command::Init => {
            println!("zig init: initializing project (not yet implemented)");
        }
    }

    Ok(())
}
