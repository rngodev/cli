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
    Login {},
    Logout {},
    Sim { spec_path: Option<String> },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Login {} => login::login().await,
        Commands::Logout {} => logout::logout().await,
        Commands::Sim { spec_path } => sim::sim(spec_path).await,
    }
}
