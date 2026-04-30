mod ai;
mod auth;
mod config;
mod effect;
mod model;
mod sim;
mod system;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::AiAgent;
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
    /// Commands for authentication.
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Commands for working with effects.
    Effect {
        #[command(subcommand)]
        command: EffectCommands,
    },
    /// Commands for working with simulations.
    Sim {
        #[command(subcommand)]
        command: SimCommands,
    },
    /// Commands for working with systems.
    System {
        #[command(subcommand)]
        command: SystemCommands,
    },
}

#[derive(Debug, Subcommand)]
enum AuthCommands {
    /// Save an API key for API authentication.
    Login {},
    /// Delete the API key saved for API authentication.
    Logout {},
}

#[derive(Debug, Subcommand)]
enum EffectCommands {
    /// Infer effects using an LLM.
    Infer {
        /// Show the agent's output (verbose mode)
        #[arg(short, long)]
        verbose: bool,

        /// Run the prompt in an agent subshell instead of outputting it
        #[arg(short, long)]
        agent: Option<AiAgent>,
    },
}

#[derive(Debug, Subcommand)]
enum SimCommands {
    /// Initialize rngo in the current application.
    Init {},
    /// Create a simulation and download the data.
    Run {
        /// The sim file to use for the simulation
        #[arg(short, long)]
        file: Option<String>,

        /// Stream the simulation data to stdout
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Debug, Subcommand)]
enum SystemCommands {
    /// Infer systems using an LLM - outputs an LLM skill document.
    Infer {
        /// Show the agent's output (verbose mode)
        #[arg(short, long)]
        verbose: bool,

        /// Run the prompt in an agent subshell instead of outputting it
        #[arg(short, long)]
        agent: Option<AiAgent>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Auth { command } => match command {
            AuthCommands::Login {} => auth::login().await,
            AuthCommands::Logout {} => auth::logout().await,
        },
        Commands::Effect { command } => match command {
            EffectCommands::Infer { verbose, agent } => effect::infer(agent, verbose).await,
        },
        Commands::System { command } => match command {
            SystemCommands::Infer { verbose, agent } => system::infer(agent, verbose).await,
        },
        Commands::Sim { command } => match command {
            SimCommands::Init {} => sim::init().await,
            SimCommands::Run { file, stdout } => sim::run(file, stdout).await,
        },
    }
}
