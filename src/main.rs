use clap::{Parser, Subcommand};

use chainsawman_api::{cli, http};

#[derive(Debug, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Api,
    Scraper,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match &cli.command {
        Command::Api => http::handler::run().await,
        Command::Scraper => cli::handler::run().await,
    }
}
