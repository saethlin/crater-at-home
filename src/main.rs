use clap::Parser;
use color_eyre::Result;
use miri_the_world::*;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    Run(run::Args),
    Render(render::Args),
    Upload(upload::Args),
}

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let args = Cli::parse();
    match args.command {
        Commands::Run(args) => run::run(args),
        Commands::Render(args) => render::run(args),
        Commands::Upload(args) => upload::run(args),
    }
}
