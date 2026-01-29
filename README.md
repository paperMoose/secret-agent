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
# Install
cargo install secret-agent

# Import a secret (interactive prompt, value never in shell history)
secret-agent import OPENAI_KEY

# Use it in commands
secret-agent exec "curl -H 'Authorization: Bearer {{OPENAI_KEY}}' https://api.openai.com/v1/models"

# Generate new secrets
secret-agent create DB_PASS --length 32

# Write secrets to .env files (agent never sees values)
secret-agent inject DB_PASS --file .env --env-format
```

## Status

🚧 **Under active development** — not yet ready for production use.

See [SPEC.md](./SPEC.md) for the full design.

## License

MIT
