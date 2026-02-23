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

    if !quiet && !matches!(cli.command, Commands::Setup { .. }) && !commands::setup::is_configured()
    {
        eprintln!("Tip: run `secret-agent setup` to configure Claude Code integration");
        eprintln!();
    }

    let result = match cli.command {
        Commands::Create {
            name,
            length,
            charset,
            force,
        } => commands::create::run(&name, length, &charset, force, quiet),

        Commands::Import {
            name,
            clipboard,
            replace,
        } => commands::import::run(&name, clipboard, replace, quiet),

        Commands::List { bucket } => commands::list::run(bucket.as_deref()),

        Commands::Delete { name } => commands::delete::run(&name, quiet),

        Commands::Get {
            name,
            clipboard,
            unsafe_display,
        } => commands::get::run(&name, clipboard, unsafe_display, quiet),

        Commands::Exec {
            env_secrets,
            command,
        } => match commands::exec::run(&env_secrets, &command) {
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
            export,
        } => commands::inject::run(
            &name,
            &file,
            placeholder.as_deref(),
            env_format,
            export,
            quiet,
        ),

        Commands::Env { action } => match action {
            EnvAction::Export { file, names, all } => {
                commands::env::export(&file, &names, all, quiet)
            }
            EnvAction::Import { file } => commands::env::import(&file, quiet),
        },

        Commands::Setup { print } => commands::setup::run(print, quiet),
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
