use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "secret-agent")]
#[command(about = "A CLI vault that keeps secrets out of AI agent traces")]
#[command(long_about = "A CLI vault that keeps secrets out of AI agent traces.

Secrets are encrypted and stored locally. When you run commands through
secret-agent, it injects secrets and sanitizes output so sensitive values
never appear in logs or AI context windows.")]
#[command(version)]
#[command(after_help = "Examples:
  secret-agent create API_KEY                      Generate a random secret
  secret-agent import GITHUB_TOKEN --clipboard     Import from clipboard
  secret-agent exec --env API_KEY -- curl ...      Run command with secret as env var
  secret-agent list                                Show all stored secrets")]
pub struct Cli {
    /// Suppress informational output (for scripting)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate and store a new random secret
    #[command(after_help = "Examples:
  secret-agent create DB_PASSWORD                  32-char alphanumeric (default)
  secret-agent create DB_PASSWORD -l 64            64-char alphanumeric
  secret-agent create DB_PASSWORD -c hex           Hex characters only
  secret-agent create DB_PASSWORD --force          Overwrite existing secret")]
    Create {
        /// Name of the secret (e.g., API_KEY, DB_PASSWORD)
        name: String,

        /// Length of the generated secret (default: 32)
        #[arg(short, long, default_value = "32")]
        length: usize,

        /// Character set to use: alphanumeric, ascii, hex, or base64
        #[arg(short, long, default_value = "alphanumeric")]
        charset: String,

        /// Overwrite if the secret already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Import a secret value from clipboard or stdin
    #[command(after_help = "Examples:
  secret-agent import API_KEY --clipboard    Read from clipboard (clears after)
  echo 'value' | secret-agent import KEY     Read from stdin
  secret-agent import KEY                    Interactive prompt (hidden input)
  secret-agent import KEY --replace          Replace existing secret")]
    Import {
        /// Name to store the secret under
        name: String,

        /// Read secret from clipboard instead of stdin (clears clipboard after)
        #[arg(long)]
        clipboard: bool,

        /// Replace if the secret already exists
        #[arg(short, long)]
        replace: bool,
    },

    /// List all stored secret names (values are never shown)
    List,

    /// Permanently delete a secret from the vault
    Delete {
        /// Name of the secret to delete
        name: String,
    },

    /// Display a secret value (requires explicit --unsafe-display flag)
    #[command(after_help = "WARNING: This displays the secret in plaintext.
Do not use in AI agent contexts or logged sessions.

Example:
  secret-agent get API_KEY --unsafe-display")]
    Get {
        /// Name of the secret to retrieve
        name: String,

        /// Required safety flag - confirms you want to display the secret in plaintext
        #[arg(long)]
        unsafe_display: bool,
    },

    /// Run a command with secrets injected as environment variables
    #[command(after_help = "Secrets can be injected two ways:

1. As environment variables (recommended):
   secret-agent exec --env API_KEY -- node app.js
   secret-agent exec -e KEY1 -e KEY2 -- ./script.sh
   secret-agent exec --env VAULT_NAME:ENV_VAR -- cmd    # rename

2. As placeholders in the command string:
   secret-agent exec -- curl -H 'Auth: {{API_KEY}}' https://...

Output is automatically sanitized - any secret values in stdout/stderr
are replaced with [REDACTED:NAME] so they never leak to logs or agents.")]
    Exec {
        /// Inject a secret as an environment variable.
        /// Use SECRET_NAME to inject with the same name, or
        /// SECRET_NAME:ENV_VAR to use a different env var name.
        /// Can be repeated: -e KEY1 -e KEY2
        #[arg(short, long = "env", value_name = "SECRET[:VAR]")]
        env_secrets: Vec<String>,

        /// The command and arguments to execute.
        /// Use {{SECRET_NAME}} to inject secrets directly into the command string.
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Write a secret into a file (replaces placeholder or appends)
    #[command(after_help = "Examples:
  secret-agent inject API_KEY -f .env --env-format     Append API_KEY=value to .env
  secret-agent inject KEY -f config.json -p __KEY__    Replace __KEY__ placeholder")]
    Inject {
        /// Name of the secret to inject
        name: String,

        /// Target file path
        #[arg(short, long)]
        file: String,

        /// String to find and replace with the secret value
        #[arg(short, long)]
        placeholder: Option<String>,

        /// Append as NAME=value line (for .env files)
        #[arg(long)]
        env_format: bool,
    },

    /// Bulk import/export secrets to .env files
    #[command(after_help = "Examples:
  secret-agent env import -f .env.local              Import all vars from file
  secret-agent env export -f .env API_KEY DB_PASS    Export specific secrets
  secret-agent env export -f .env --all              Export all secrets")]
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
}

#[derive(Subcommand)]
pub enum EnvAction {
    /// Write secrets to a .env file
    Export {
        /// Target .env file to write
        #[arg(short, long)]
        file: String,

        /// Names of secrets to export
        #[arg(required_unless_present = "all")]
        names: Vec<String>,

        /// Export all secrets from the vault
        #[arg(long)]
        all: bool,
    },

    /// Read secrets from a .env file into the vault
    Import {
        /// Source .env file to read
        #[arg(short, long)]
        file: String,
    },
}
