# Codex CLI Hooks

> Part of [`hooks/`](../README.md) — see also [`src/hooks/`](../../src/hooks/README.md) for installation code

## Current behavior

Codex support combines:

- `RTK.md` prompt guidance that asks Codex to invoke `rtk <command>` directly.
- A native `PreToolUse` command (`rtk hook codex`) registered for Bash calls. Other tools are outside this Hook's coverage.

The Hook is deliberately non-authorizing. Codex requires `permissionDecision = "allow"` together with `updatedInput` to rewrite a command, and that response also approves the rewritten call. RTK has no independent Codex safety allowlist, so it does not use that response.

| Internal RTK decision | Codex Hook output |
|-----------------------|-------------------|
| Allow | No output; Codex keeps its native approval flow |
| Ask | No output; Codex does not currently support Hook `ask` |
| Default | No output; Codex keeps its native approval flow |
| Explicit deny | `permissionDecision: "deny"` |

Codex does not read or reuse Claude Code permission rules. RTK currently has no Codex-specific allowlist or deny-list configuration, so installed Hooks normally take the Default path. `PermissionRequest` is not used for rewriting because it cannot change tool input.

## Install and uninstall

```bash
rtk init --codex                 # current project: .codex/config.toml + .codex/RTK.md
rtk init -g --codex              # global: $CODEX_HOME or ~/.codex
rtk init --codex --uninstall     # remove project-scoped RTK artifacts
rtk init -g --codex --uninstall  # remove global RTK artifacts
```

Installation preserves existing `config.toml` comments, formatting, and unrelated Hooks. If `[features] hooks = false` is present, RTK reports that Hooks are disabled rather than claiming the integration is active.

After installation, run `/hooks` in Codex to review and trust project or plugin Hooks. Untrusted Hooks do not run.

## Failure behavior

The Hook fails open: empty, malformed, non-UTF-8, or oversized input produces no authorization decision and exits successfully. `PreToolUse` is a guardrail, not a complete security boundary; RTK still relies on Codex's own sandbox and approval policy.

See the [Codex Hooks documentation](https://learn.chatgpt.com/docs/hooks) for the host protocol and current limitations.
