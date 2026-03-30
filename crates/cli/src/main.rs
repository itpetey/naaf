//! CLI entry point.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "naaf")]
#[command(about = "OpenSpec Orchestrator CLI")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Run { prompt: String },
    List,
    Inspect { run_id: String },
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    match args.command {
        Command::Run { prompt } => {
            println!("Running with prompt: {}", prompt);
        }
        Command::List => {
            println!("Listing runs...");
        }
        Command::Inspect { run_id } => {
            println!("Inspecting run: {}", run_id);
        }
    }
}
