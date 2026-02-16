# secret-agent

[![Crates.io](https://img.shields.io/crates/v/secret-agent.svg)](https://crates.io/crates/secret-agent)
[![CI](https://github.com/paperMoose/secret-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/paperMoose/secret-agent/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A CLI vault that keeps secrets out of AI agent traces.

## The Problem

AI agents are great at orchestrating tasks, but they have a fundamental security flaw: **everything they see ends up in logs, traces, and context windows**.

When an agent needs to call an API with a secret:
```bash
# Agent runs this
curl -H "Authorization: Bearer sk-1234567890" https://api.openai.com/v1/models
```

That secret is now:
- In the LLM's context window
- In your trace logs
- Potentially extractable via prompt injection
- Visible to anyone reviewing the conversation

Most secrets managers solve the wrong problem. They help you *retrieve* secrets securely, but then the agent *has* the secret. The agent becomes a liability.

## The Solution

`secret-agent` is a broker. The agent orchestrates, but never handles the actual secret values.

```bash
# Agent runs this instead
secret-agent exec "curl -H 'Authorization: Bearer {{OPENAI_KEY}}' https://api.openai.com/v1/models"
```

What happens:
1. `secret-agent` looks up `OPENAI_KEY` from its encrypted vault
2. Injects the real value into the command
3. Executes it
4. **Sanitizes the output** — if the secret somehow appears in stdout/stderr, it's replaced with `[REDACTED:OPENAI_KEY]`
5. Returns the sanitized output to the agent

The agent never sees `sk-1234567890`. It only knows the *name* `OPENAI_KEY`.

## Why This Matters

- **Prompt injection can't leak secrets** — the agent doesn't have them
- **Traces are safe to log** — secrets are redacted
- **No behavior change needed** — agent still orchestrates normally, just references secrets by name

## Quick Start

```bash
# Install from crates.io
cargo install secret-agent

# Import a secret (interactive prompt, value never in shell history)
secret-agent import OPENAI_KEY

# Import from clipboard (for agent workflows - agent never sees value)
secret-agent import OPENAI_KEY --clipboard

# Use it in commands
secret-agent exec --env OPENAI_KEY curl https://api.openai.com/v1/models
secret-agent exec "curl -H 'Authorization: Bearer {{OPENAI_KEY}}' https://api.openai.com/v1/models"

# Generate new secrets
secret-agent create DB_PASS --length 32

# Import PEM files, certificates, and key pairs
cat private_key.pem | secret-agent import TLS_KEY
secret-agent exec --env TLS_KEY my-deploy-script

# Copy a secret to clipboard (agent never sees value)
secret-agent get API_KEY --clipboard

# Write secrets to .env files (agent never sees values)
secret-agent inject DB_PASS --file .env --env-format

# Quiet mode for scripting
secret-agent -q create CI_TOKEN
```

## Setup

Add to your `~/.zshrc` or `~/.bashrc`:

```bash
export SECRET_AGENT_USE_FILE=1
```

This stores the master key in `~/.secret-agent/master.key` (chmod 600) instead of the system keychain, avoiding permission prompts.

## Platform Support

| Platform | Recommended Setup | Notes |
|----------|-------------------|-------|
| **macOS** | `SECRET_AGENT_USE_FILE=1` | Avoids Keychain permission prompts |
| **Linux (Desktop)** | `SECRET_AGENT_USE_FILE=1` | Or uses GNOME Keyring if available |
| **Linux (Headless)** | Auto-detected | File storage used automatically |
| **CI/Automation** | `SECRET_AGENT_PASSPHRASE` env var | Highest priority |

### Alternative: System Keychain

If you prefer system keychain (macOS Keychain, GNOME Keyring):
1. Don't set `SECRET_AGENT_USE_FILE`
2. On macOS: ad-hoc sign the binary to avoid repeated prompts:
   ```bash
   codesign -s - ~/.cargo/bin/secret-agent
   ```

## Commands

| Command | Description |
|---------|-------------|
| `create NAME` | Generate random secret (`--length`, `--charset`, `--force`) |
| `import NAME` | Import from stdin or `--clipboard` (`--replace` to overwrite). Supports multiline (PEM files, certs) |
| `list` | Show secret names (`--bucket` to filter) |
| `delete NAME` | Remove secret permanently |
| `get NAME --clipboard` | Copy to clipboard (agent never sees value, works on macOS + Linux) |
| `get NAME --unsafe-display` | Show value (debug only, not for agent use) |
| `exec --env KEY cmd` | Run with secrets as env vars + sanitized output (supports multiline) |
| `exec cmd {{KEY}}` | Run with secrets templated into command string (single-line only) |
| `inject NAME --file F` | Write to file (`--env-format`, `--placeholder`) |
| `env import --file F` | Bulk import from .env file |
| `env export --file F` | Bulk export to .env file (`--all` or specific names) |

Buckets: Use `bucket/name` syntax (e.g., `prod/API_KEY`) to organize secrets. Bucket prefix is stripped when injecting as env vars.

Global flags: `-q, --quiet` — Suppress informational output

## Claude Code Integration

To let Claude use secret-agent in all your projects, add this to your `~/.claude/CLAUDE.md`:

```markdown
## Secrets Management (secret-agent)

@~/git/secret-agent/CLAUDE.md
```

If you don't have the repo cloned, you can copy the usage reference from [CLAUDE.md](./CLAUDE.md) directly into your `~/.claude/CLAUDE.md` instead.

## License

MIT
