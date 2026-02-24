# secret-agent-cli

A CLI vault that keeps secrets out of AI agent traces. Secrets are encrypted at rest and never appear in agent context windows, logs, or tool outputs.

## Install

```bash
npm install -g secret-agent-cli
```

## Why use secret-agent

- Secrets never appear in agent context windows
- Traces are safe to log
- Prompt injection can't leak secrets you don't have
- Output is automatically sanitized (secrets replaced with `[REDACTED:NAME]`)

## Quick reference

### Run commands with secrets (preferred)

```bash
# Inject as environment variables
secret-agent exec --env API_KEY node app.js
secret-agent exec -e KEY1 -e KEY2 ./script.sh

# With bucket prefix (env var is just the name part)
secret-agent exec --env prod/API_KEY node app.js

# Rename: vault secret -> different env var name
secret-agent exec --env MY_SECRET:OPENAI_API_KEY python app.py

# Template secrets into command strings
secret-agent exec curl -H 'Authorization: Bearer {{API_KEY}}' https://api.example.com
```

### Import secrets

```bash
# From clipboard (clears after reading)
secret-agent import OPENAI_KEY --clipboard

# From stdin
echo "sk-..." | secret-agent import API_KEY

# Replace existing
echo "new_value" | secret-agent import EXISTING_KEY --replace

# Multiline (PEM files, certificates)
cat private_key.pem | secret-agent import TLS_KEY
```

### Generate secrets

```bash
secret-agent create API_KEY                     # 32-char alphanumeric
secret-agent create API_KEY --length 64         # Custom length
secret-agent create API_KEY --charset hex       # hex | base64 | ascii | alphanumeric
```

### List and delete

```bash
secret-agent list                    # All secrets
secret-agent list --bucket prod      # Only secrets in a bucket
secret-agent delete OLD_SECRET
```

### Buckets

```bash
secret-agent create prod/SUPABASE_KEY
secret-agent create dev/SUPABASE_KEY
secret-agent list --bucket prod
```

### Write secrets to files

```bash
# Append NAME=value to .env file
secret-agent inject DB_PASS --file .env --env-format

# Replace a placeholder in any file
secret-agent inject API_KEY --file config.json --placeholder __API_KEY__
```

### Bulk .env import/export

```bash
secret-agent env import --file .env.local
secret-agent env export --file .env API_KEY DB_PASS
secret-agent env export --file .env --all
```

### Copy to clipboard

```bash
# Put a secret in the user's clipboard without the agent seeing the value
secret-agent get API_KEY --clipboard
```

## Supported platforms

- macOS (x64, ARM64)
- Linux (x64)

## How it works

`npm install` downloads a pre-built Rust binary from [GitHub releases](https://github.com/paperMoose/secret-agent/releases). No Rust toolchain required.

For building from source, install via Cargo: `cargo install secret-agent`
