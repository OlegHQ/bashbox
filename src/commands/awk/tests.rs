//! Adapter tests: `-F` / `-v` / escapes, VFS file IO, errors — not full awk-rs language coverage.

use super::AwkCommand;
use crate::commands::test_utils::*;
use crate::commands::types::CommandContext;
use crate::commands::Command;
use crate::fs::FileSystem;
use std::collections::HashMap;
use std::sync::Arc;

fn make_ctx(args: Vec<&str>, stdin: &str) -> CommandContext {
    make_ctx_with_stdin(args, stdin)
}

fn make_ctx_with_env(args: Vec<&str>, stdin: &str, env: HashMap<String, String>) -> CommandContext {
    make_ctx_with_stdin_and_env(args, stdin, env)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_print_stdin() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["{ print }"], "line1\nline2\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "line1\nline2\n");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_field_separator_flag() {
    let cmd = AwkCommand;
    let ctx = make_ctx(
        vec!["-F:", "{ print $1 }"],
        "root:x:0:0\nuser:x:1000:1000\n",
    );
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "root\nuser\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_field_separator_escapes() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["-F", "\\t", "{ print $1, $2 }"], "a\tb\tc\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "a b\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_preset_variable_v() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["-v", "x=10", "{ print x }"], "line\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "10\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_preset_variable_escapes() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["-v", "x=a\\tb", "BEGIN { print x }"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "a\tb\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_begin_main_end() {
    let cmd = AwkCommand;
    let ctx = make_ctx(
        vec!["BEGIN { print \"start\" } { print } END { print \"end\" }"],
        "middle\n",
    );
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "start\nmiddle\nend\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_argc_argv_wiring() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["BEGIN { print ARGC, ARGV[0], ARGV[1] }"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "1 awk \n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_input() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["{ print }"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_files_vfs() {
    let fs = Arc::new(InMemoryFs::new());
    fs.write_file("/file1.txt", b"a\nb\n").await.unwrap();
    fs.write_file("/file2.txt", b"c\nd\n").await.unwrap();
    let cmd = AwkCommand;
    let ctx = CommandContext {
        args: vec![
            "{ print }".to_string(),
            "/file1.txt".to_string(),
            "/file2.txt".to_string(),
        ],
        stdin: String::new(),
        cwd: "/".to_string(),
        env: HashMap::new(),
        fs,
        exec_fn: None,
        fetch_fn: None,
    };
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "a\nb\nc\nd\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_program_error() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec![], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stderr.contains("missing program"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_parse_error() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["{ print "], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(!result.stderr.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_file_not_found() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["{ print }", "nonexistent.txt"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stderr.contains("No such file"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_help_flag() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["--help"], "");
    let result = cmd.execute(ctx).await;
    assert!(result.stdout.contains("Usage:"));
    assert!(result.stdout.contains("awk"));
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_option() {
    let cmd = AwkCommand;
    let ctx = make_ctx(vec!["-x", "{ print }"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stderr.contains("unknown option"));
}

/// awk-rs 0.1 ignores custom ORS — see KNOWN_LIMITATIONS.md
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_ors_known_issue() {
    let cmd = AwkCommand;
    let ctx = make_ctx(
        vec!["BEGIN { FS=\":\"; OFS=\"-\"; ORS=\"|\" } { print $1, $2 }"],
        "a:b\nx:y\n",
    );
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "a-b|x-y|");
}

/// ENVIRON should reflect sandbox ctx.env — see KNOWN_LIMITATIONS.md
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_environ_sandbox_known_issue() {
    let mut env = HashMap::new();
    env.insert("MY_VAR".to_string(), "hello".to_string());
    let cmd = AwkCommand;
    let ctx = make_ctx_with_env(vec!["BEGIN { print ENVIRON[\"MY_VAR\"] }"], "", env);
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "hello\n");
}
