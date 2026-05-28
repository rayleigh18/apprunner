use anyhow::Result;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

use apprunner::app;
use apprunner::install;

#[derive(Parser)]
#[command(name = "apprunner", version, about = "Local app runner TUI", author)]
struct Cli {
    /// Uninstall apprunner (removes binary, completions, and data)
    #[arg(long)]
    uninstall: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.uninstall {
        return install::uninstall();
    }

    match cli.command {
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "apprunner", &mut std::io::stdout());
            Ok(())
        }
        None => app::run(),
    }
}
