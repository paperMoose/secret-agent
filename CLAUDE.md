# secret-agent

A CLI vault that keeps secrets out of AI agent traces.

## Before Committing

Always run before pushing:
```bash
cargo test && cargo build --release
```

## Usage for AI Agents

### Run commands with secrets as environment variables (preferred)
```bash
# Inject secret as env var
secret-agent exec --env GEMINI_API_KEY node script.mjs

# Multiple secrets
secret-agent exec -e API_KEY -e DB_PASS ./deploy.sh

# Rename: vault secret → different env var name
secret-agent exec --env MY_SECRET:OPENAI_API_KEY python app.py
```

### Template secrets into command strings
```bash
secret-agent exec curl -H 'Authorization: Bearer {{API_KEY}}' https://api.example.com
```

### Create new secrets
```bash
secret-agent create DB_PASS --length 32
```

### Import secrets
```bash
# From clipboard (clears after reading)
secret-agent import OPENAI_KEY --clipboard

# From stdin
echo "sk-..." | secret-agent import API_KEY

# Replace existing secret
echo "new_value" | secret-agent import EXISTING_KEY --replace
```

### Write secrets to files
```bash
# Append NAME=value to .env file
secret-agent inject DB_PASS --file .env --env-format

# Append export NAME="value" for shell scripts
secret-agent inject DB_PASS --file env.sh --env-format --export
```

## Avoiding Keychain Prompts

If macOS keychain prompts block automation, set:
```bash
export SECRET_AGENT_USE_FILE=1
```
This uses `~/.secret-agent/master.key` instead of the system keychain.

## Why This Matters
- Secrets never appear in agent context windows
- Traces are safe to log
- Prompt injection can't leak secrets you don't have
- Output is automatically sanitized (secrets replaced with `[REDACTED:NAME]`)
