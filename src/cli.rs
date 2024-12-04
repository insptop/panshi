use crate::app::AppTrait;
use crate::config::{resolve_dotenv_file, resolve_from_env, Environment, DEFAULT_ENVIRONMENT};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Specify the environment
    #[arg(short, long, global = true, help = &format!("Specify the environment [default: {}]", DEFAULT_ENVIRONMENT))]
    environment: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Start {},
}

pub fn main<T: AppTrait>() -> crate::error::Result<()> {
    let cli = Cli::parse();

    let dotenv = resolve_dotenv_file();
    let env: Environment = cli.environment.unwrap_or_else(resolve_from_env).into();

    let config = env.load_config().expect("Failed to load config");

    match cli.command {
        Commands::Start {} => {
            println!("Starting application");
        }
    }

    Ok(())
}
