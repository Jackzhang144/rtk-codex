//! Detects whether RTK hooks are installed and warns if they are outdated.

use super::constants::{
    CLAUDE_HOOK_COMMAND, CODEX_CONFIG_TOML, CODEX_HOOK_COMMAND, HOOKS_SUBDIR, PRE_TOOL_USE_KEY,
    REWRITE_HOOK_FILE, SETTINGS_JSON,
};
use super::init::{resolve_claude_dir, resolve_codex_dir};
use crate::core::constants::RTK_DATA_DIR;
use std::path::{Path, PathBuf};

const CURRENT_HOOK_VERSION: u8 = 3;
const WARN_INTERVAL_SECS: u64 = 24 * 3600;

/// Hook status for diagnostics and `rtk gain`.
#[derive(Debug, PartialEq, Clone)]
pub enum HookStatus {
    /// Hook is installed and up to date.
    Ok,
    /// Hook exists but is outdated or unreadable.
    Outdated,
    /// No hook file found (but Claude Code is installed).
    Missing,
}

/// Return the current hook status without printing anything.
/// Returns `Ok` if no Claude Code is detected (not applicable).
pub fn status() -> HookStatus {
    // Don't warn users who don't have Claude Code installed
    let claude_dir = resolve_claude_dir().ok();
    let codex_dir = resolve_codex_dir().ok();
    status_at(claude_dir.as_deref(), codex_dir.as_deref())
}

fn status_at(claude_dir: Option<&Path>, codex_dir: Option<&Path>) -> HookStatus {
    let Some(claude_dir) = claude_dir else {
        return HookStatus::Ok;
    };
    if !claude_dir.exists() {
        return HookStatus::Ok;
    }

    let claude_status = if binary_hook_registered(claude_dir) {
        // If old script file still exists alongside new command, report Outdated
        // (migration not complete — user should run `rtk init -g` to clean up)
        let old_hook = claude_dir.join(HOOKS_SUBDIR).join(REWRITE_HOOK_FILE);
        if old_hook.exists() {
            HookStatus::Outdated
        } else {
            HookStatus::Ok
        }
    } else {
        // Fall back to legacy script file check
        let Some(hook_path) = hook_installed_path() else {
            return suppress_if_codex_hook_present(HookStatus::Missing, codex_dir);
        };
        let Ok(content) = std::fs::read_to_string(&hook_path) else {
            return suppress_if_codex_hook_present(HookStatus::Outdated, codex_dir);
        };
        if parse_hook_version(&content) >= CURRENT_HOOK_VERSION {
            HookStatus::Ok
        } else {
            HookStatus::Outdated
        }
    };

    suppress_if_codex_hook_present(claude_status, codex_dir)
}

fn suppress_if_codex_hook_present(status: HookStatus, codex_dir: Option<&Path>) -> HookStatus {
    if status != HookStatus::Ok && codex_dir.is_some_and(codex_hook_registered_at) {
        HookStatus::Ok
    } else {
        status
    }
}

/// Check for the exact Codex PreToolUse command written by `rtk init --codex`.
fn codex_hook_registered_at(codex_dir: &Path) -> bool {
    let config_path = codex_dir.join(CODEX_CONFIG_TOML);
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return false;
    };
    let Some(root) = value.as_table() else {
        return false;
    };
    let Some(hooks) = root.get("hooks").and_then(|hooks| hooks.as_table()) else {
        return false;
    };
    let Some(pre_tool_use) = hooks
        .get(PRE_TOOL_USE_KEY)
        .and_then(|pre_tool_use| pre_tool_use.as_array())
    else {
        return false;
    };

    pre_tool_use
        .iter()
        .filter_map(|entry| entry.get("hooks")?.as_array())
        .flatten()
        .filter_map(|hook| hook.get("command")?.as_str())
        .any(|command| command == CODEX_HOOK_COMMAND)
}

/// Check if the native binary command is registered in settings.json
fn binary_hook_registered(claude_dir: &std::path::Path) -> bool {
    let settings_path = claude_dir.join(SETTINGS_JSON);
    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => return false,
    };
    let root: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let pre_tool_use = match root
        .get("hooks")
        .and_then(|h| h.get(PRE_TOOL_USE_KEY))
        .and_then(|p| p.as_array())
    {
        Some(arr) => arr,
        None => return false,
    };
    pre_tool_use
        .iter()
        .filter_map(|entry| entry.get("hooks")?.as_array())
        .flatten()
        .filter_map(|hook| hook.get("command")?.as_str())
        .any(|cmd| cmd == CLAUDE_HOOK_COMMAND)
}

/// Check if the installed hook is missing or outdated, warn once per day.
pub fn maybe_warn() {
    // Don't block startup — fail silently on any error
    let _ = check_and_warn();
}

/// Single source of truth: delegates to `status()` then rate-limits the warning.
fn check_and_warn() -> Option<()> {
    let warning = match status() {
        HookStatus::Ok => return Some(()),
        HookStatus::Missing => {
            "[rtk] /!\\ No hook installed — run `rtk init -g` for automatic token savings"
        }
        HookStatus::Outdated => "[rtk] /!\\ Hook outdated — run `rtk init -g` to update",
    };

    // Rate limit: warn once per day
    let marker = warn_marker_path()?;
    if let Ok(meta) = std::fs::metadata(&marker) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().map(|e| e.as_secs()).unwrap_or(u64::MAX) < WARN_INTERVAL_SECS {
                return Some(());
            }
        }
    }

    eprintln!("{}", warning);

    // Touch marker after warning is printed
    let _ = std::fs::create_dir_all(marker.parent()?);
    let _ = std::fs::write(&marker, b"");

    Some(())
}

pub fn parse_hook_version(content: &str) -> u8 {
    // Version tag must be in the first 5 lines (shebang + header convention)
    for line in content.lines().take(5) {
        if let Some(rest) = line.strip_prefix("# rtk-hook-version:") {
            if let Ok(v) = rest.trim().parse::<u8>() {
                return v;
            }
        }
    }
    0 // No version tag = version 0 (outdated)
}

fn hook_installed_path() -> Option<PathBuf> {
    let claude_dir = resolve_claude_dir().ok()?;
    let path = claude_dir.join(HOOKS_SUBDIR).join(REWRITE_HOOK_FILE);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn warn_marker_path() -> Option<PathBuf> {
    let data_dir = dirs::data_local_dir()?.join(RTK_DATA_DIR);
    Some(data_dir.join(".hook_warn_last"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::constants::{
        CODEX_CONFIG_TOML, CODEX_DIR, CODEX_HOOK_COMMAND, CONFIG_DIR, CURSOR_DIR, GEMINI_DIR,
        GEMINI_HOOK_FILE, HERMES_DIR, HERMES_PLUGINS_SUBDIR, HERMES_PLUGIN_MANIFEST_FILE,
        HERMES_PLUGIN_NAME, OPENCODE_PLUGIN_FILE, OPENCODE_SUBDIR, PLUGIN_SUBDIR, PRE_TOOL_USE_KEY,
    };

    /// Check if the RTK Codex hook command is present in a parsed config.toml table.
    fn codex_hook_in_config(root: &toml::value::Table, hook_command: &str) -> bool {
        let Some(hooks) = root.get("hooks").and_then(|h| h.as_table()) else {
            return false;
        };
        let Some(ptu) = hooks.get(PRE_TOOL_USE_KEY).and_then(|p| p.as_array()) else {
            return false;
        };
        ptu.iter()
            .filter_map(|entry| entry.get("hooks")?.as_array())
            .flatten()
            .filter_map(|hook| hook.get("command")?.as_str())
            .any(|cmd| cmd == hook_command)
    }

    fn other_integration_installed(home: &std::path::Path) -> bool {
        let paths = [
            home.join(CONFIG_DIR)
                .join(OPENCODE_SUBDIR)
                .join(PLUGIN_SUBDIR)
                .join(OPENCODE_PLUGIN_FILE),
            home.join(CURSOR_DIR)
                .join(HOOKS_SUBDIR)
                .join(REWRITE_HOOK_FILE),
            home.join(GEMINI_DIR)
                .join(HOOKS_SUBDIR)
                .join(GEMINI_HOOK_FILE),
            home.join(HERMES_DIR)
                .join(HERMES_PLUGINS_SUBDIR)
                .join(HERMES_PLUGIN_NAME)
                .join(HERMES_PLUGIN_MANIFEST_FILE),
        ];
        if paths.iter().any(|p| p.exists()) {
            return true;
        }
        // Codex: parse config.toml and check for RTK hook command
        // (file-existence alone is insufficient — config.toml may exist
        // without an RTK PreToolUse hook entry; string matching would
        // false-positive on comments containing "rtk hook codex")
        let codex_config = home.join(CODEX_DIR).join(CODEX_CONFIG_TOML);
        if let Ok(content) = std::fs::read_to_string(&codex_config) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(root) = val.as_table() {
                    if codex_hook_in_config(root, CODEX_HOOK_COMMAND) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[test]
    fn test_parse_hook_version_present() {
        let content = "#!/usr/bin/env bash\n# rtk-hook-version: 2\n# some comment\n";
        assert_eq!(parse_hook_version(content), 2);
    }

    #[test]
    fn test_parse_hook_version_missing() {
        let content = "#!/usr/bin/env bash\n# old hook without version\n";
        assert_eq!(parse_hook_version(content), 0);
    }

    #[test]
    fn test_parse_hook_version_future() {
        let content = "#!/usr/bin/env bash\n# rtk-hook-version: 5\n";
        assert_eq!(parse_hook_version(content), 5);
    }

    #[test]
    fn test_parse_hook_version_no_tag() {
        assert_eq!(parse_hook_version("no version here"), 0);
        assert_eq!(parse_hook_version(""), 0);
    }

    #[test]
    fn test_hook_status_enum() {
        assert_ne!(HookStatus::Ok, HookStatus::Missing);
        assert_ne!(HookStatus::Outdated, HookStatus::Missing);
        assert_eq!(HookStatus::Ok, HookStatus::Ok);
        // Clone works
        let s = HookStatus::Missing;
        assert_eq!(s.clone(), HookStatus::Missing);
    }

    #[test]
    fn test_other_integration_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(!other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_opencode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp
            .path()
            .join(CONFIG_DIR)
            .join(OPENCODE_SUBDIR)
            .join(PLUGIN_SUBDIR)
            .join(OPENCODE_PLUGIN_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"plugin").unwrap();
        assert!(other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_cursor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp
            .path()
            .join(CURSOR_DIR)
            .join(HOOKS_SUBDIR)
            .join(REWRITE_HOOK_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"hook").unwrap();
        assert!(other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_codex() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(CODEX_DIR).join(CODEX_CONFIG_TOML);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("[hooks]\nPreToolUse = [\n  {{ matcher = \"Bash\", hooks = [\n    {{ type = \"command\", command = \"{}\" }}\n  ]}}\n]\n", CODEX_HOOK_COMMAND)).unwrap();
        assert!(other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_gemini() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp
            .path()
            .join(GEMINI_DIR)
            .join(HOOKS_SUBDIR)
            .join(GEMINI_HOOK_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"hook").unwrap();
        assert!(other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_hermes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp
            .path()
            .join(HERMES_DIR)
            .join(HERMES_PLUGINS_SUBDIR)
            .join(HERMES_PLUGIN_NAME)
            .join(HERMES_PLUGIN_MANIFEST_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"plugin").unwrap();
        assert!(other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_other_integration_empty_dirs_not_enough() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join(CURSOR_DIR).join(HOOKS_SUBDIR)).unwrap();
        std::fs::create_dir_all(tmp.path().join(CODEX_DIR)).unwrap();
        std::fs::create_dir_all(tmp.path().join(GEMINI_DIR)).unwrap();
        std::fs::create_dir_all(
            tmp.path()
                .join(HERMES_DIR)
                .join(HERMES_PLUGINS_SUBDIR)
                .join(HERMES_PLUGIN_NAME),
        )
        .unwrap();
        assert!(!other_integration_installed(tmp.path()));
    }

    #[test]
    fn test_status_ignores_missing_claude_hook_when_codex_is_installed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();

        let codex_config = tmp.path().join(CODEX_DIR).join(CODEX_CONFIG_TOML);
        std::fs::create_dir_all(codex_config.parent().unwrap()).unwrap();
        std::fs::write(
            &codex_config,
            format!(
                "[hooks]\nPreToolUse = [\n  {{ matcher = \"Bash\", hooks = [\n    {{ type = \"command\", command = \"{}\" }}\n  ]}}\n]\n",
                CODEX_HOOK_COMMAND
            ),
        )
        .unwrap();

        assert_eq!(
            status_at(Some(&claude_dir), codex_config.parent()),
            HookStatus::Ok
        );
    }

    #[test]
    fn test_status_returns_valid_variant() {
        // Skip on machines without Claude Code
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };
        let claude_dir = home.join(".claude");
        if !claude_dir.exists() {
            assert_eq!(status(), HookStatus::Ok);
            return;
        }
        // With .claude dir present, status must be one of the valid variants
        let s = status();
        assert!(
            s == HookStatus::Ok || s == HookStatus::Outdated || s == HookStatus::Missing,
            "Expected valid HookStatus variant, got {:?}",
            s
        );
    }
}
