//! Adapter tests: CLI, VFS (`-f`, `-i`, stdin), BRE/ERE entry — not full sed-rs coverage.

use super::SedCommand;
use crate::commands::test_utils::*;
use crate::commands::types::CommandContext;
use crate::commands::Command;
use crate::fs::FileSystem;
use std::sync::Arc;

fn make_ctx(args: Vec<&str>, stdin: &str) -> CommandContext {
    make_ctx_with_stdin(args, stdin)
}

fn make_ctx_with_fs(args: Vec<&str>, stdin: &str, fs: Arc<InMemoryFs>) -> CommandContext {
    make_ctx_with_stdin_and_fs(args, stdin, fs)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_substitution() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["s/world/rust/"], "hello world\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "hello rust\n");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_expressions() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["-e", "s/a/x/", "-e", "s/b/y/"], "ab\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "xy\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_in_place_editing() {
    let fs = Arc::new(InMemoryFs::new());
    fs.write_file("/test.txt", b"old content\n").await.unwrap();
    let cmd = SedCommand;
    let ctx = make_ctx_with_fs(vec!["-i", "s/old/new/", "/test.txt"], "", fs.clone());
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 0);
    let content = fs.read_file("/test.txt").await.unwrap();
    assert_eq!(content, "new content\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_stdin_marker() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["s/a/b/", "-"], "aaa\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "baa\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_files() {
    let fs = Arc::new(InMemoryFs::new());
    fs.write_file("/file1.txt", b"a\n").await.unwrap();
    fs.write_file("/file2.txt", b"b\n").await.unwrap();
    let cmd = SedCommand;
    let ctx = make_ctx_with_fs(vec!["s/./X/", "/file1.txt", "/file2.txt"], "", fs);
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "X\nX\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_input_file_not_found() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["s/a/b/", "nonexistent"], "");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert_eq!(
        result.stderr,
        "sed: nonexistent: No such file or directory\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_script_error() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec![], "hello\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stderr.contains("no script"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_help_flag() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["--help"], "");
    let result = cmd.execute(ctx).await;
    assert!(result.stdout.contains("Usage:"));
    assert!(result.stdout.contains("sed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_script_from_file_vfs() {
    let fs = Arc::new(InMemoryFs::new());
    fs.write_file("/script.sed", b"s/old/new/").await.unwrap();
    let cmd = SedCommand;
    let ctx = make_ctx_with_fs(vec!["-f", "/script.sed"], "old text\n", fs);
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "new text\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_script_file_missing() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["-f", "/no/such/script.sed"], "x\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert_eq!(
        result.stderr,
        "sed: couldn't open file /no/such/script.sed: No such file or directory\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_bre_preprocessed() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["s/\\(foo\\)/[\\1]/"], "foo\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "[foo]\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ere_flag() {
    let cmd = SedCommand;
    let ctx = make_ctx(vec!["-E", "s/(foo)/[\\1]/"], "foo\n");
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "[foo]\n");
}

/// sed-rs uses the real filesystem for `r` — cannot read from InMemoryFs.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_read_file_command() {
    let fs = Arc::new(InMemoryFs::new());
    fs.write_file("/append.txt", b"appended").await.unwrap();
    let cmd = SedCommand;
    let ctx = make_ctx_with_fs(vec!["r /append.txt"], "line\n", fs);
    let result = cmd.execute(ctx).await;
    assert_eq!(result.stdout, "line\nappended\n");
}

/// sed-rs uses the real filesystem for `w` — cannot write to InMemoryFs.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_write_file_command() {
    let fs = Arc::new(InMemoryFs::new());
    let cmd = SedCommand;
    let ctx = make_ctx_with_fs(vec!["w /output.txt"], "hello\n", fs.clone());
    let result = cmd.execute(ctx).await;
    assert_eq!(result.exit_code, 0);
    let content = fs.read_file("/output.txt").await.unwrap();
    assert_eq!(content, "hello\n");
}
