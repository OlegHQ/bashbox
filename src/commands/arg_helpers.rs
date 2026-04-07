//! Shared argument and flag handling for external commands.

/// True if any argument is exactly `--help` (common GNU-style idiom).
pub fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help")
}

/// POSIX / GNU style: `{cmd}: invalid option -- '{flag}'\n`
pub fn invalid_option(cmd: &str, bad_flag: &str) -> String {
    format!("{}: invalid option -- '{}'\n", cmd, bad_flag)
}

/// Same as [`invalid_option`] but **no** trailing newline (for layering: `format!("{}\n", msg)` or `Err(msg)`).
pub fn invalid_option_line(cmd: &str, bad_flag: &str) -> String {
    format!("{}: invalid option -- '{}'", cmd, bad_flag)
}

/// If no file operands were parsed, default to stdin (`-`).
pub fn files_or_stdin(files: Vec<String>) -> Vec<String> {
    if files.is_empty() {
        vec!["-".to_string()]
    } else {
        files
    }
}
