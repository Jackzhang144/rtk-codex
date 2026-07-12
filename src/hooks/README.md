# Hook System

> See also [docs/contributing/TECHNICAL.md](../../docs/contributing/TECHNICAL.md) for the full architecture overview | [hooks/](../../hooks/README.md) for deployed hook artifacts

## Scope

The **lifecycle management** layer for LLM agent hooks: install, uninstall, verify integrity, audit usage, and manage trust. This component creates and maintains the hook artifacts that live in `hooks/` (root), but does **not** execute rewrite logic itself — that lives in `discover/registry`.

Owns: `rtk init` installation flows (5 agents via `AgentTarget` enum + 3 special modes: Gemini, Codex, OpenCode), SHA-256 integrity verification, hook version checking, audit log analysis, `rtk rewrite` CLI entry point, and TOML filter trust management.

Does **not** own: the deployed hook scripts themselves (that's `hooks/`), the rewrite pattern registry (that's `discover/`), or command filtering (that's `cmds/`).

Boundary notes:
- `rewrite_cmd.rs` is a thin CLI bridge — it exists to serve hooks (hooks call `rtk rewrite` as a subprocess) and delegates entirely to `discover/registry`.
- `trust.rs` gates project-local TOML filter execution. It lives here because the trust workflow is tied to hook-installed filter discovery, not to the core filter engine.

## Purpose
LLM agent integration layer that installs, validates, and executes Hooks for AI coding assistants. Most integrations intercept raw CLI commands (e.g., `git status`) and rewrite them to RTK equivalents (e.g., `rtk git status`). Codex instead combines `RTK.md` guidance with a non-authorizing Hook, so Codex invokes `rtk` directly while retaining its native approval behavior.

## Installation Modes

`rtk init` supports these installation flows:

| Mode | Command | Creates | Patches |
|------|---------|---------|----------|
| Default (global) | `rtk init -g` | Hook, SHA-256 hash, RTK.md | settings.json, CLAUDE.md |
| Hook only | `rtk init -g --hook-only` | Hook, SHA-256 hash | settings.json |
| Claude-MD (legacy) | `rtk init --claude-md` | 134-line RTK block | CLAUDE.md |
| Windsurf | `rtk init -g --agent windsurf` | `.windsurfrules` | -- |
| Cline | `rtk init --agent cline` | `.clinerules` | -- |
| Codex | `rtk init --codex` | `.codex/config.toml`, `.codex/RTK.md` (or global equivalents with `-g`) | config.toml |
| Cursor | `rtk init -g --agent cursor` | Cursor hook | hooks.json |
| Pi | `rtk init --agent pi` | `.pi/extensions/rtk.ts` | -- |
| Hermes | `rtk init --agent hermes` | Python plugin in `~/.hermes/plugins/rtk-rewrite/` | `config.yaml` `plugins.enabled` |

Codex writes its TOML configuration directly and does not accept `--auto-patch` or `--no-patch`. Both local and global installs have matching `--uninstall` flows.

## Integrity Verification

The integrity system prevents unauthorized hook modifications:

1. At install: `integrity::store_hash()` computes SHA-256 of the hook file, writes to `~/.claude/hooks/.rtk-hook.sha256` (read-only 0o444)
2. At runtime: `integrity::runtime_check()` re-computes hash and compares; blocks execution if tampered
3. On demand: `rtk verify` prints detailed verification status (PASS/FAIL/WARN/SKIP)

Five integrity states:
- **Verified**: Hash matches stored value
- **Tampered**: Hash mismatch (blocks execution)
- **NoBaseline**: Hook exists but no hash stored (old install)
- **NotInstalled**: No hook, no hash
- **OrphanedHash**: Hash file exists, hook missing

## PatchMode Behavior

Controls how `rtk init` modifies agent settings files:

| Mode | Flag | Behavior |
|------|------|----------|
| Ask (default) | -- | Prompts user `[y/N]`; defaults to No if stdin not terminal |
| Auto | `--auto-patch` | Patches without prompting; for CI/scripted installs |
| Skip | `--no-patch` | Prints manual instructions; user patches manually |

## Atomicity and Safety

All file operations use atomic writes (tempfile + rename) to prevent corruption on crash. Settings files are backed up to `.bak` before modification. All operations are idempotent -- running `rtk init` multiple times is safe.

## Permission Model

RTK enforces a permission precedence that matches Claude Code's least-privilege default:

```
Deny > Ask > Allow (explicit) > Default (ask)
```

Rules are loaded from all Claude Code `settings.json` files (project + global, including `.local` variants). Only `Bash(...)` rules are extracted; other scopes (Read, Write) are ignored. Codex does not reuse these rules because its approval model and Hook protocol differ.

| Verdict | Trigger | rewrite_cmd exit | Hook behavior |
|---------|---------|-----------------|---------------|
| Deny | `permissions.deny` rule matched | 2 | Passthrough — host tool handles denial |
| Ask | `permissions.ask` rule matched | 3 | Rewrite + let host tool prompt user |
| Allow | `permissions.allow` rule matched | 0 | Rewrite + auto-allow |
| Default | No rule matched | 3 | Rewrite + let host tool prompt user |

### Per-tool support

| Tool | ask support | Behavior on Default |
|------|------------|-------------------|
| Claude Code (rtk-rewrite.sh) | Yes | `permissionDecision: "ask"` — user prompted |
| Copilot VS Code (rtk hook copilot) | Yes | `permissionDecision: "ask"` — user prompted |
| Cursor (rtk hook cursor) | Ready | `permission: "ask",` — users will be prompted when Cursor enforces the permission; in the meantime, allow |
| Gemini CLI (rtk hook gemini) | No (allow/deny only) | allow (limitation — no ask mode in Gemini) |
| Copilot CLI (rtk hook copilot) | No updatedInput | deny-with-suggestion (unchanged) |
| Codex | No (`ask` is unsupported) | no decision; Codex keeps its native approval behavior |

Codex command rewriting requires `permissionDecision = "allow"` together with `updatedInput`, which also authorizes the rewritten call. RTK therefore does not rewrite from `PreToolUse` unless a future Codex-specific policy can independently prove the call safe. `PermissionRequest` cannot rewrite commands. Treat this Hook as a guardrail, not a complete security boundary, and review project hooks with `/hooks` before trusting them.

### Implementation

- `permissions.rs` — loads host-specific deny/ask/allow rules, evaluates precedence, returns `PermissionVerdict`; Codex currently returns `Default`
- `rewrite_cmd.rs` — maps verdict to exit code (consumed by shell hook)
- `hook_cmd.rs` — maps verdicts to host-specific JSON for Copilot, Gemini, Codex, Cursor, and Droid

## Exit Code Contract

Hook processors in `hook_cmd.rs` must return `Ok(())` on every path — success, no-match, parse error, and unexpected input. Returning `Err` propagates to `main()` and exits non-zero, which blocks the agent's command from executing. This violates the non-blocking guarantee documented in `hooks/README.md`.

## Adding New Functionality
To add support for a new AI coding agent: (1) add the hook installation logic to `init.rs` following the existing agent patterns, (2) if the agent requires a custom hook protocol (like Gemini's `BeforeTool`), add a processor function in `hook_cmd.rs`, (3) add the agent's Hook path or configuration to `hook_check.rs`, and (4) update `integrity.rs` when the integration installs a hashed Hook file. Test installation, uninstall, malformed input, and the exact host decision semantics; do not assume every host can safely combine rewriting with approval.
