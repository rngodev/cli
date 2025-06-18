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
    /// Log into the rngo API.
    Login {},
    /// Log out of the rngo API.
    Logout {},
    /// Creates a simulation and downloads the data.
    Sim {
        /// Path to the simulation spec file.
        spec_path: Option<String>,

        /// Stream the simulation data to stdout
        #[arg(short, long)]
        stream: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Login {} => login::login().await,
        Commands::Logout {} => logout::logout().await,
        Commands::Sim { spec_path, stream } => sim::sim(spec_path, stream).await,
    }
}
