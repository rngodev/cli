mod login;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rngo")]
#[command(about = "A fictional versioning CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Login { api_key: String },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Login { api_key } => login::login(api_key),
    }
}
