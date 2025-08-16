mod infer;
mod init;
mod login;
mod logout;
mod sim;
pub mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rngo")]
#[command(about = "Data simulation CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a new rngo project in the current directory.
    Init {},
    /// Log into the rngo API.
    Login {},
    /// Log out of the rngo API.
    Logout {},
    Infer {
        #[command(subcommand)]
        command: InferCommands,
    },
    /// Creates a simulation and downloads the data.
    Sim {
        /// The spec file to use for the simulation
        #[arg(short, long)]
        spec: Option<String>,

        /// Stream the simulation data to stdout
        #[arg(long)]
        stream: bool,
    },
}

#[derive(Debug, Subcommand)]
enum InferCommands {
    Prompt {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Init {} => init::init().await,
        Commands::Login {} => login::login().await,
        Commands::Logout {} => logout::logout().await,
        Commands::Infer { .. } => infer::infer().await,
        Commands::Sim { spec, stream } => sim::sim(spec, stream).await,
    }
}
