mod infer;
mod init;
mod login;
mod logout;
mod sim;
mod systems;
pub mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rngo")]
#[command(
    about = "Data simulation CLI. See https://rngo.dev/docs/cli.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize rngo in the current application.
    Init {},
    /// Save an API key for API authentication.
    Login {},
    /// Delete the API key saved for API authentication.
    Logout {},
    /// Infer rngo entities using an LLM - see `rngo infer prompt`.
    Infer {
        #[command(subcommand)]
        command: InferCommands,
    },
    /// Commands for working with systems.
    Systems {
        #[command(subcommand)]
        command: SystemsCommands,
    },
    /// Create a simulation and download the data.
    Sim {
        /// The spec file to use for the simulation
        #[arg(short, long)]
        spec: Option<String>,

        /// Stream the simulation data to stdout
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Debug, Subcommand)]
enum InferCommands {
    /// Output an LLM prompt to infer rngo entites from the current application.
    Prompt {},
}

#[derive(Debug, Subcommand)]
enum SystemsCommands {
    /// Infer systems using an LLM - outputs an LLM skill document.
    Infer {
        /// Output the prompt instead of running Claude
        #[arg(long)]
        prompt: bool,

        /// Show Claude's output (verbose mode)
        #[arg(short, long)]
        verbose: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Init {} => init::init().await,
        Commands::Login {} => login::login().await,
        Commands::Logout {} => logout::logout().await,
        Commands::Infer { command } => match command {
            InferCommands::Prompt {} => infer::infer_prompt().await,
        },
        Commands::Systems { command } => match command {
            SystemsCommands::Infer { prompt, verbose } => {
                systems::infer_systems(prompt, verbose).await
            }
        },
        Commands::Sim { spec, stdout } => sim::sim(spec, stdout).await,
    }
}
