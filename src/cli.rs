use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "secret-agent")]
#[command(about = "A CLI vault that keeps secrets out of AI agent traces")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a new random secret
    Create {
        /// Name of the secret
        name: String,

        /// Length of the generated secret
        #[arg(short, long, default_value = "32")]
        length: usize,

        /// Character set: alphanumeric, ascii, hex, base64
        #[arg(short, long, default_value = "alphanumeric")]
        charset: String,
    },

    /// Import a secret from stdin or interactive prompt
    Import {
        /// Name of the secret
        name: String,
    },

    /// List all secret names
    List,

    /// Delete a secret
    Delete {
        /// Name of the secret to delete
        name: String,
    },

    /// Get a secret value (use with caution!)
    Get {
        /// Name of the secret
        name: String,

        /// Required flag to confirm you want to display the secret
        #[arg(long)]
        unsafe_display: bool,
    },

    /// Execute a command with secrets injected
    Exec {
        /// Command to execute (use {{SECRET_NAME}} for placeholders)
        command: String,
    },

    /// Inject a secret into a file
    Inject {
        /// Name of the secret
        name: String,

        /// Target file
        #[arg(short, long)]
        file: String,

        /// Placeholder string to replace
        #[arg(short, long)]
        placeholder: Option<String>,

        /// Append as NAME=value line (for .env files)
        #[arg(long)]
        env_format: bool,
    },

    /// Sync secrets with .env files
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
}

#[derive(Subcommand)]
pub enum EnvAction {
    /// Export secrets to a .env file
    Export {
        /// Target .env file
        #[arg(short, long)]
        file: String,

        /// Secret names to export (or --all)
        #[arg(required_unless_present = "all")]
        names: Vec<String>,

        /// Export all secrets
        #[arg(long)]
        all: bool,
    },

    /// Import secrets from a .env file
    Import {
        /// Source .env file
        #[arg(short, long)]
        file: String,
    },
}
