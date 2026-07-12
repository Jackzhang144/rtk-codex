# CI/CD Flows

## PR Quality Gates (ci.yml)

Trigger: pull_request to develop or master

```
                          ┌──────────────────┐
                          │    PR opened      │
                          └────────┬─────────┘
                                   │
                          ┌────────▼─────────┐
                          │    fmt --all     │
                          └────────┬─────────┘
                                   │
                       ┌───────────▼──────────┐
                       │ clippy --all-targets │
                       └───┬───┬───┬───┬───┬──┘
                           │   │   │   │   │
           ┌───────────────┘   │   │   │   └────────────────┐
           │       ┌───────────┘   │   └───────────┐        │
           ▼       ▼              ▼               ▼        ▼
     ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌─────────┐ ┌──────────┐
     │ test     │ │ security │ │ semgrep   │ │benchmark│ │ doc      │
     │ ubuntu   │ │ cargo    │ │ AST-aware │ │ >=80%   │ │ review   │
     │ windows  │ │ audit    │ │ diff-only │ │ savings │ │ ai agent │
     │ macos    │ │ patterns │ │           │ │         │ │          │
     └────┬─────┘ └────┬─────┘ └─────┬─────┘ └────┬────┘ └────┬─────┘
          │            │             │             │            │
          └────────────┴─────────┬───┴─────────────┴────────────┘
                                 │
                      ┌──────────▼─────────┐
                      │  All must pass     │
                      │  to merge          │
                      └────────────────────┘

     + DCO check (independent, develop PRs only)
     + Dependabot (weekly: Cargo deps + GitHub Actions)
```

## Merge to develop — pre-release (cd.yml)

Trigger: push to develop | workflow_dispatch (not master) | Concurrency: cancel-in-progress

```
     ┌──────────────────┐
     │ push to develop   │
     │ OR dispatch       │
     └────────┬─────────┘
              │
     ┌────────▼──────────────────┐
     │ pre-release                │
     │ base = Cargo.toml X.Y.Z   │
     │ increment Codex B         │
     │ tag = dev-X.Y.Z-codex.    │
     │       A.B-rc.{run}        │
     └────────┬──────────────────┘
              │
     ┌────────▼──────────────────┐
     │ release.yml               │
     │ prerelease = true         │
     └────────┬──────────────────┘
              │
     ┌────────▼──────────────────┐
     │ Build                     │
     │ Apple Silicon tarball     │
     └────────┬──────────────────┘
              │
     ┌────────▼──────────────────┐
     │ GitHub Release            │
     │ (pre-release badge)       │
     │                           │
     │ Discord:  SKIPPED         │
     │ Homebrew: SKIPPED         │
     └──────────────────────────┘
```

## Merge to master — stable release (cd.yml)

Trigger: push to master (only) | Concurrency: never cancelled

```
     ┌──────────────────┐
     │ push to master    │
     └────────┬─────────┘
              │
     ┌────────▼──────────────────┐
     │ release-please            │
     │ analyze conventional      │
     │ commits                   │
     └────────┬──────────────────┘
              │
         ┌────┴────────────────┐
         │                     │
    no release           release created
         │                     │
         ▼                     ▼
  ┌──────────────┐    ┌───────────────────────┐
  │ create/update│    │ release.yml            │
  │ release PR   │    │ prerelease = false     │
  └──────────────┘    └───────────┬───────────┘
                                  │
                     ┌────────────▼────────────┐
                     │ Build                   │
                     │ Apple Silicon tarball   │
                     └────────────┬────────────┘
                                  │
                     ┌────────────▼────────────┐
                     │ GitHub Release           │
                     │ (stable, "Latest" badge) │
                     └──┬─────────┬─────────┬──┘
                        │         │         │
                        ▼         ▼         ▼
                    Discord   Homebrew   latest
                    notify    tap update  tag
```

## Manual release (release.yml)

Trigger: workflow_dispatch

```
     ┌────────────────────────┐
     │ workflow_dispatch       │
     │ inputs: tag, prerelease │
     └───────────┬────────────┘
                 │
     ┌───────────▼────────────┐
     │ Fork build pipeline     │
     │ Apple Silicon tarball   │
     └───────────┬────────────┘
                 │
          ┌──────┴──────┐
          │             │
   prerelease=false  prerelease=true
          │             │
          ▼             ▼
     Discord        pre-release
     Homebrew       badge only
     latest tag
```

Fork release tags use `vX.Y.Z-codex.A.B`, where `X.Y.Z` is the upstream RTK version from `Cargo.toml` and `A.B` is the Codex adaptation version. The develop CD path creates `dev-X.Y.Z-codex.A.B-rc.{run}` prereleases by incrementing B from the latest matching Codex tag. Select A deliberately when you publish a new adaptation generation; this avoids upstream merges or PR titles changing the fork version. Publish a stable fork release manually with `release.yml`, for example `v0.42.4-codex.2.2`.
