mod cli;
mod commands;
mod crypto;
mod error;
mod keychain;
mod sanitize;
mod secret_gen;
mod vault;

use clap::Parser;
use cli::{Cli, Commands, EnvAction};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Create {
            name,
            length,
            charset,
        } => commands::create::run(&name, length, &charset),

        Commands::Import { name } => commands::import::run(&name),

        Commands::List => commands::list::run(),

        Commands::Delete { name } => commands::delete::run(&name),

        Commands::Get {
            name,
            unsafe_display,
        } => commands::get::run(&name, unsafe_display),

        Commands::Exec { command } => {
            match commands::exec::run(&command) {
                Ok(exit_code) => std::process::exit(exit_code),
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Inject {
            name,
            file,
            placeholder,
            env_format,
        } => commands::inject::run(&name, &file, placeholder.as_deref(), env_format),

        Commands::Env { action } => match action {
            EnvAction::Export { file, names, all } => {
                commands::env::export(&file, &names, all)
            }
            EnvAction::Import { file } => commands::env::import(&file),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
