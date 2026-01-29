# secret-agent

A minimal CLI vault that keeps secrets out of agent traces.

## Problem

AI agents need to work with secrets (API keys, passwords, tokens) but:
- Secrets in prompts/responses leak into logs, traces, context windows
- Agents can be tricked via prompt injection to reveal secrets
- Most vaults are "retrieve and use" — the agent sees the value

## Solution

A CLI broker that agents call to **orchestrate** secrets, but never **handle** them:
- Agent references secrets by name
- Tool generates, stores, and injects actual values
- Agent only sees sanitized output

## Design Decisions

| Decision | Choice |
|----------|--------|
| Language | Rust (single binary, no runtime) |
| Scope | Single user, single machine |
| Key management | System keychain (macOS Keychain, Linux secret-service) |
| Secret types | Strings only |
| Sync (MVP) | `.env` files only |
| Exec | Shell subprocess only |
| Sanitization | `[REDACTED:SECRET_NAME]` |

## CLI Interface

### create
Generate a random secret. Agent never sees the value.

```bash
secret-agent create DB_PASS
secret-agent create DB_PASS --length 32
secret-agent create DB_PASS --length 64 --charset alphanumeric
secret-agent create API_KEY --charset hex
```

**Options:**
- `--length N` — Length of generated secret (default: 32)
- `--charset <type>` — One of: `alphanumeric`, `ascii`, `hex`, `base64` (default: `alphanumeric`)

**Output:**
```
Created secret: DB_PASS
```

### import
Import an existing secret. Value provided via stdin or interactive prompt (never CLI args).

```bash
# Interactive prompt (hidden input)
secret-agent import API_KEY

# From stdin
echo "sk-1234..." | secret-agent import API_KEY

# From file
secret-agent import API_KEY < /path/to/secret.txt
```

**Output:**
```
Imported secret: API_KEY
```

### exec
Execute a shell command with secrets injected. Output is sanitized.

```bash
secret-agent exec "curl -H 'Authorization: Bearer {{API_KEY}}' https://api.example.com"

secret-agent exec "psql postgres://{{DB_USER}}:{{DB_PASS}}@localhost/mydb -c 'SELECT 1'"
```

**Behavior:**
1. Parse command for `{{SECRET_NAME}}` placeholders
2. **Fail immediately** if any referenced secret doesn't exist
3. Replace placeholders with actual values
4. Execute command via shell
5. Sanitize stdout/stderr — replace any occurrence of secret values with `[REDACTED:SECRET_NAME]`
6. Return command's exit code

**Output:**
```
$ secret-agent exec "echo 'token is {{API_KEY}}'"
token is [REDACTED:API_KEY]
```

### inject
Write a secret into a file, replacing a placeholder.

```bash
# Replace placeholder in existing file
secret-agent inject DB_PASS --file config.yaml --placeholder '{{DB_PASS}}'

# Append to .env file
secret-agent inject DB_PASS --file .env --env-format
# Writes: DB_PASS=<value>
```

**Options:**
- `--file <path>` — Target file
- `--placeholder <string>` — String to replace (e.g., `{{DB_PASS}}`)
- `--env-format` — Append as `NAME=value` line (for .env files)

**Output:**
```
Injected DB_PASS into config.yaml
```

### env
Sync secrets to/from a `.env` file.

```bash
# Export secrets to .env file
secret-agent env export --file .env.local DB_PASS API_KEY
secret-agent env export --file .env.local --all

# Import secrets from .env file (each becomes a secret)
secret-agent env import --file .env.local
```

### list
List secret names (never values).

```bash
secret-agent list
```

**Output:**
```
NAME        CREATED
DB_PASS     2024-01-28 10:30:00
API_KEY     2024-01-27 15:45:00
DB_USER     2024-01-27 15:45:00
```

### delete
Delete a secret from the vault.

```bash
secret-agent delete DB_PASS
```

**Output:**
```
Deleted secret: DB_PASS
```

### get (restricted)
For debugging only. Requires explicit flag to prevent accidental exposure.

```bash
secret-agent get DB_PASS --unsafe-display
```

**Output:**
```
WARNING: Displaying secret value. Do not use in agent contexts.
hunter2
```

## Storage

### Location
```
~/.secret-agent/
└── vault.db          # SQLite with encrypted values
```

### Encryption
- Each secret value encrypted with `age` (https://age-encryption.org/)
- Master key stored in system keychain:
  - macOS: Keychain Access
  - Linux: libsecret / GNOME Keyring / KWallet
  - Fallback: prompt for passphrase

### Schema
```sql
CREATE TABLE secrets (
    name TEXT PRIMARY KEY,
    encrypted_value BLOB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Stores: schema_version, created_at, etc.
```

## Output Sanitization

All output from `exec` is sanitized before returning to the agent:

```bash
# If API_KEY = "sk-12345" and command somehow outputs it:
$ secret-agent exec "some-command --verbose"
Connecting with token sk-12345...  # ← actual output
Connecting with token [REDACTED:API_KEY]...  # ← what agent sees
```

### Rules
1. Exact match replacement for all secrets used in the command
2. Check both stdout and stderr
3. Also check for common encodings:
   - Base64 encoded value
   - URL encoded value

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Secret `{{FOO}}` not found | Exit 1, error message: `Error: secret 'FOO' not found` |
| Command fails | Pass through exit code, sanitized output |
| Keychain unavailable | Prompt for passphrase fallback |
| Vault doesn't exist | Create on first `create` or `import` |

## Security Model

### Protected Against
- Secrets appearing in agent traces / LLM context
- Shell history exposure (values never in CLI args)
- `ps` exposure (values never in process args)
- Accidental logging of secret values

### NOT Protected Against
- Direct file access to vault (encrypted, but attacker with file access + keychain access wins)
- Commands that write secrets to files (agent could then read those files)
- Keyloggers / compromised system

### Threat Model
Primary threat: **accidental exposure via agent traces**

The tool ensures the agent can orchestrate operations involving secrets without ever having the secret value in its context. Even if the agent is compromised via prompt injection, it cannot reveal secrets it doesn't have.

## Implementation

### Rust Crates
- `clap` — CLI parsing
- `age` — Encryption
- `rusqlite` — SQLite
- `keyring` — System keychain access
- `rand` — Secret generation
- `regex` — Placeholder parsing

### Project Structure
```
src/
├── main.rs           # CLI entry point
├── cli.rs            # Clap command definitions
├── vault.rs          # Vault operations (CRUD)
├── crypto.rs         # age encryption/decryption
├── keychain.rs       # System keychain integration
├── exec.rs           # Command execution + sanitization
├── inject.rs         # File injection
└── env.rs            # .env file sync
```

### MVP Scope
- [x] `create` — generate random secret
- [x] `import` — import from stdin/prompt
- [x] `list` — list secret names
- [x] `delete` — remove secret
- [x] `exec` — execute with injection + sanitization
- [x] `inject` — write to file
- [x] `env export/import` — .env sync
- [x] System keychain integration
- [x] age encryption

### Post-MVP
- [ ] MCP server mode (`secret-agent mcp`)
- [ ] Cloud sync (GCloud, AWS, Vault)
- [ ] Secret rotation / expiry
- [ ] Audit logging

## Usage Examples

### Agent Creating a Database Password
```bash
# Agent: "Create a secure database password"
$ secret-agent create DB_PASS --length 32
Created secret: DB_PASS

# Agent: "Add it to the .env file"
$ secret-agent inject DB_PASS --file .env --env-format
Injected DB_PASS into .env
```

### Agent Making Authenticated API Call
```bash
# Agent: "Call the OpenAI API"
$ secret-agent exec "curl -s https://api.openai.com/v1/models -H 'Authorization: Bearer {{OPENAI_KEY}}'"
{
  "data": [...]
}
```

### Agent Running Database Migration
```bash
$ secret-agent exec "psql postgres://{{DB_USER}}:{{DB_PASS}}@localhost/mydb -f migrations/001.sql"
CREATE TABLE
INSERT 0 5
```

### Importing Existing Secrets
```bash
# Human imports secrets (one-time setup)
$ secret-agent import OPENAI_KEY
Enter secret value: <hidden input>
Imported secret: OPENAI_KEY

# Or from .env file
$ secret-agent env import --file .env.production
Imported 5 secrets: DB_HOST, DB_USER, DB_PASS, API_KEY, WEBHOOK_SECRET
```
