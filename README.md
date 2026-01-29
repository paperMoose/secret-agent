# secret-agent

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
# Install (from source for now)
cargo install --path .

# Import a secret (interactive prompt, value never in shell history)
secret-agent import OPENAI_KEY

# Use it in commands
secret-agent exec "curl -H 'Authorization: Bearer {{OPENAI_KEY}}' https://api.openai.com/v1/models"

# Generate new secrets
secret-agent create DB_PASS --length 32

# Overwrite existing secret
secret-agent create DB_PASS --length 32 --force

# Write secrets to .env files (agent never sees values)
secret-agent inject DB_PASS --file .env --env-format

# Quiet mode for scripting
secret-agent -q create CI_TOKEN
```

## Platform Support

| Platform | Master Key Storage | Notes |
|----------|-------------------|-------|
| **macOS** | Keychain Access | Touch ID supported if enabled in System Settings |
| **Linux (Desktop)** | GNOME Keyring / KWallet | Via Secret Service API |
| **Linux (Headless)** | `~/.secret-agent/master.key` | File with 600 permissions |
| **CI/Automation** | `SECRET_AGENT_PASSPHRASE` env var | Highest priority |

### Authentication Priority

1. `SECRET_AGENT_PASSPHRASE` environment variable (for CI/scripts)
2. System keychain (macOS Keychain, Linux Secret Service)
3. File-based key at `~/.secret-agent/master.key` (headless fallback)
4. Interactive passphrase prompt (last resort)

### macOS Touch ID

To require Touch ID for vault access:
1. Open **Keychain Access**
2. Find the `secret-agent` entry
3. Get Info → Access Control → Require Touch ID

### Linux Headless Servers

On servers without D-Bus/Secret Service, the master key is stored in `~/.secret-agent/master.key` with `chmod 600` permissions. The tool detects headless environments automatically.

For extra security on servers, set the passphrase via environment:
```bash
export SECRET_AGENT_PASSPHRASE="your-secure-passphrase"
```

## Commands

| Command | Description |
|---------|-------------|
| `create NAME` | Generate random secret |
| `create NAME --force` | Overwrite existing secret |
| `import NAME` | Import from stdin/prompt |
| `list` | Show secret names |
| `delete NAME` | Remove secret |
| `get NAME --unsafe-display` | Show value (debug only) |
| `exec "cmd {{NAME}}"` | Run with injection + sanitization |
| `inject NAME --file F` | Write to file |
| `env export/import` | Sync with .env files |

Global flags:
- `-q, --quiet` — Suppress informational output

## Status

🚧 **Under active development** — not yet ready for production use.

See [SPEC.md](./SPEC.md) for the full design.

## License

MIT
