//! Grep is in-tree (regex + VFS). Tests cover flags and filesystem wiring without duplicating every long/short flag pair.

use super::GrepCommand;
use crate::commands::test_utils::*;
use crate::commands::Command;

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_match() {
    let ctx = make_ctx_with_files(
        vec!["hello", "/test.txt"],
        vec![("/test.txt", "hello world\nfoo bar\nhello again\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("hello world"));
    assert!(result.stdout.contains("hello again"));
    assert!(!result.stdout.contains("foo bar"));
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_case() {
    let ctx = make_ctx_with_files(
        vec!["-i", "HELLO", "/test.txt"],
        vec![("/test.txt", "Hello World\nhello world\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("Hello World"));
    assert!(result.stdout.contains("hello world"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_long_flags_ignore_case_line_number() {
    let ctx = make_ctx_with_files(
        vec!["--ignore-case", "--line-number", "x", "/t.txt"],
        vec![("/t.txt", "X\ny\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout, "1:X\n");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invert() {
    let ctx = make_ctx_with_files(
        vec!["-v", "hello", "/test.txt"],
        vec![("/test.txt", "hello\nworld\nhello again\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "world");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_count() {
    let ctx = make_ctx_with_files(
        vec!["-c", "hello", "/test.txt"],
        vec![("/test.txt", "hello\nworld\nhello again\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "2");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_line_number() {
    let ctx = make_ctx_with_files(
        vec!["-n", "hello", "/test.txt"],
        vec![("/test.txt", "hello\nworld\nhello again\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("1:hello"));
    assert!(result.stdout.contains("3:hello again"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_match_exit_code() {
    let ctx = make_ctx_with_files(
        vec!["notfound", "/test.txt"],
        vec![("/test.txt", "hello world\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stdout.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_fixed_strings() {
    let ctx = make_ctx_with_files(
        vec!["-F", "a.b", "/test.txt"],
        vec![("/test.txt", "a.b\naXb\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "a.b");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_files_with_matches() {
    let ctx = make_ctx_with_files(
        vec!["-l", "needle", "/a.txt", "/b.txt"],
        vec![("/a.txt", "x\n"), ("/b.txt", "needle\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("/b.txt"));
    assert!(!result.stdout.contains("/a.txt"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_files_without_match() {
    let ctx = make_ctx_with_files(
        vec!["-L", "needle", "/a.txt", "/b.txt"],
        vec![("/a.txt", "none\n"), ("/b.txt", "needle\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("/a.txt"));
    assert!(!result.stdout.contains("/b.txt"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_only_matching() {
    let ctx = make_ctx_with_files(
        vec!["-o", "[0-9]+", "/t.txt"],
        vec![("/t.txt", "a 12 b 3\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "12\n3");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_quiet_finds_match() {
    let ctx = make_ctx_with_files(vec!["-q", "yes", "/t.txt"], vec![("/t.txt", "yes\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.is_empty());
    assert_eq!(result.exit_code, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_count() {
    let ctx = make_ctx_with_files(vec!["-m", "1", "a", "/t.txt"], vec![("/t.txt", "a\na\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    let lines: Vec<_> = result.stdout.lines().collect();
    assert_eq!(lines.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_e_single_pattern() {
    let ctx = make_ctx_with_files(vec!["-e", "^f", "/t.txt"], vec![("/t.txt", "foo\nbar\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout, "foo\n");
}

/// GNU grep: multiple `-e` / `--regexp` are OR'd.
#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_e_patterns_alternate() {
    let ctx = make_ctx_with_files(
        vec!["-e", "foo", "-e", "bar", "/t.txt"],
        vec![("/t.txt", "foo\nx\nbar\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout, "foo\nbar\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_extended_regexp() {
    let ctx =
        make_ctx_with_files(vec!["-E", "a+", "/t.txt"], vec![("/t.txt", "a\nbbb\naa\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("a"));
    assert!(result.stdout.contains("aa"));
    assert!(!result.stdout.contains("bbb"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_file_shows_prefix() {
    let ctx = make_ctx_with_files(
        vec!["x", "/a.txt", "/b.txt"],
        vec![("/a.txt", "x\n"), ("/b.txt", "y\n")],
    )
    .await;
    let result = GrepCommand.execute(ctx).await;
    assert!(result.stdout.contains("/a.txt:"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_single_file_no_prefix() {
    let ctx = make_ctx_with_files(vec!["line", "/only.txt"], vec![("/only.txt", "line\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "line");
    assert!(!result.stdout.contains("/only.txt:"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_file_stderr() {
    let ctx = make_ctx_with_files(vec!["a", "/nope.txt"], vec![]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.exit_code, 1);
    assert!(result.stderr.contains("No such file"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_stdin_when_no_files() {
    let ctx = make_ctx_with_stdin(vec!["pat"], "pat\nno\n");
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout.trim(), "pat");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_file() {
    let ctx = make_ctx_with_files(vec!["test", "/empty.txt"], vec![("/empty.txt", "")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout, "");
    assert_eq!(result.exit_code, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_pattern_matches_all() {
    let ctx =
        make_ctx_with_files(vec!["", "/test.txt"], vec![("/test.txt", "hello\nworld\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_eq!(result.stdout, "hello\nworld\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_pattern() {
    let ctx = make_ctx_with_files(vec!["[", "/t.txt"], vec![("/t.txt", "x\n")]).await;
    let result = GrepCommand.execute(ctx).await;
    assert_ne!(result.exit_code, 0);
    assert!(
        result.stderr.contains("invalid pattern"),
        "expected regex compile error, got stderr={:?}",
        result.stderr
    );
}
