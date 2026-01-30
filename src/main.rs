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
    let quiet = cli.quiet;

    let result = match cli.command {
        Commands::Create {
            name,
            length,
            charset,
            force,
        } => commands::create::run(&name, length, &charset, force, quiet),

        Commands::Import { name, clipboard, replace } => commands::import::run(&name, clipboard, replace, quiet),

        Commands::List => commands::list::run(),

        Commands::Delete { name } => commands::delete::run(&name, quiet),

        Commands::Get {
            name,
            unsafe_display,
        } => commands::get::run(&name, unsafe_display),

        Commands::Exec { env_secrets, command } => match commands::exec::run(&env_secrets, &command) {
            Ok(exit_code) => std::process::exit(exit_code),
            Err(e) => {
                eprintln!("Error: {:#}", e);
                std::process::exit(1);
            }
        },

        Commands::Inject {
            name,
            file,
            placeholder,
            env_format,
        } => commands::inject::run(&name, &file, placeholder.as_deref(), env_format, quiet),

        Commands::Env { action } => match action {
            EnvAction::Export { file, names, all } => {
                commands::env::export(&file, &names, all, quiet)
            }
            EnvAction::Import { file } => commands::env::import(&file, quiet),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
