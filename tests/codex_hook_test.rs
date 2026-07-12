use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_hook(name: &str, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rtk"))
        .args(["hook", name])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rtk hook codex");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write hook input");
    child.wait_with_output().expect("wait for hook")
}

fn run_codex_hook(input: &[u8]) -> Output {
    run_hook("codex", input)
}

#[test]
fn codex_hook_preserves_native_approval_by_default() {
    let output = run_codex_hook(br#"{"tool_name":"Bash","tool_input":{"command":"git status"}}"#);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn codex_hook_fails_open_when_input_exceeds_limit() {
    let output = run_codex_hook(&vec![b'x'; 1_048_577]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn codex_hook_fails_open_for_non_utf8_input() {
    let output = run_codex_hook(&[0xff, 0xfe]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn gemini_hook_fails_open_when_input_exceeds_limit() {
    let output = run_hook("gemini", &vec![b'x'; 1_048_577]);

    assert!(output.status.success());
}
