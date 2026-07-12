use std::fs;
use std::process::Command;

fn rtk_init_in(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rtk"))
        .arg("init")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run rtk init")
}

#[test]
fn local_codex_uninstall_removes_local_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let install = rtk_init_in(temp.path(), &["--codex"]);
    assert!(install.status.success());

    let uninstall = rtk_init_in(temp.path(), &["--codex", "--uninstall"]);
    assert!(
        uninstall.status.success(),
        "{}",
        String::from_utf8_lossy(&uninstall.stderr)
    );

    let codex_dir = temp.path().join(".codex");
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(!config.contains("rtk hook codex"));
    assert!(!codex_dir.join("RTK.md").exists());
}

#[test]
fn codex_install_preserves_existing_toml_comments() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_dir = temp.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(
        codex_dir.join("config.toml"),
        "# keep this explanation\nmodel = \"gpt-5\" # keep inline comment\n",
    )
    .expect("write config.toml");

    let output = rtk_init_in(temp.path(), &["--codex"]);
    assert!(output.status.success());

    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# keep this explanation"));
    assert!(config.contains("model = \"gpt-5\" # keep inline comment"));

    let uninstall = rtk_init_in(temp.path(), &["--codex", "--uninstall"]);
    assert!(uninstall.status.success());
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# keep this explanation"));
    assert!(config.contains("model = \"gpt-5\" # keep inline comment"));
}

#[test]
fn codex_install_reminds_user_to_review_hook_trust() {
    let temp = tempfile::tempdir().expect("tempdir");

    let output = rtk_init_in(temp.path(), &["--codex"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/hooks"));
    assert!(stdout.contains("review and trust"));
}

#[test]
fn codex_install_warns_when_hooks_feature_is_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_dir = temp.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(codex_dir.join("config.toml"), "[features]\nhooks = false\n")
        .expect("write config.toml");

    let output = rtk_init_in(temp.path(), &["--codex"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hooks are disabled"));
    assert!(stdout.contains("hooks = false"));
}

#[test]
fn codex_install_merges_inline_pre_tool_use_array() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_dir = temp.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(
        codex_dir.join("config.toml"),
        "# existing hook\n[hooks]\nPreToolUse = [{ matcher = \"Bash\", hooks = [{ type = \"command\", command = \"echo existing\" }] }]\n",
    )
    .expect("write config.toml");

    let output = rtk_init_in(temp.path(), &["--codex"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# existing hook"));
    assert!(config.contains("echo existing"));
    assert!(config.contains("rtk hook codex"));

    let output = rtk_init_in(temp.path(), &["--codex", "--uninstall"]);
    assert!(output.status.success());
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# existing hook"));
    assert!(config.contains("echo existing"));
    assert!(!config.contains("rtk hook codex"));
}

#[test]
fn codex_install_merges_inline_hooks_in_pre_tool_use_table() {
    let temp = tempfile::tempdir().expect("tempdir");
    let codex_dir = temp.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(
        codex_dir.join("config.toml"),
        "# mixed representation\n[[hooks.PreToolUse]]\nmatcher = \"Bash\"\nhooks = [{ type = \"command\", command = \"echo existing\" }]\n",
    )
    .expect("write config.toml");

    let output = rtk_init_in(temp.path(), &["--codex"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# mixed representation"));
    assert!(config.contains("echo existing"));
    assert!(config.contains("rtk hook codex"));

    let output = rtk_init_in(temp.path(), &["--codex", "--uninstall"]);
    assert!(output.status.success());
    let config = fs::read_to_string(codex_dir.join("config.toml")).expect("config.toml");
    assert!(config.contains("# mixed representation"));
    assert!(config.contains("echo existing"));
    assert!(!config.contains("rtk hook codex"));
}
