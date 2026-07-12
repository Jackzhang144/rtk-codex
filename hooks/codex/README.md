# Codex CLI Hooks

> Part of [`hooks/`](../README.md) — see also [`src/hooks/`](../../src/hooks/README.md) for installation code

## Specifics

- Prompt-level guidance via `RTK.md`, plus a native `PreToolUse` guardrail
- The guardrail does not rewrite or approve commands by default, preserving Codex's own approval policy
- Local install writes `.codex/config.toml`; global install uses `$CODEX_HOME` or `~/.codex/`
- Run `/hooks` in Codex after installation to review and trust the hook
