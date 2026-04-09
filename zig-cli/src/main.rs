mod cli;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { workflow } => {
            println!("zig run: executing workflow '{workflow}' (not yet implemented)");
        }
        Command::Describe { prompt, output } => {
            let dest = output.unwrap_or_else(|| "workflow.zug".to_string());
            println!(
                "zig describe: generating '{dest}' from prompt '{prompt}' (not yet implemented)"
            );
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
