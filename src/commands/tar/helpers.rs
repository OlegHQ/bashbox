//! VFS traversal, path formatting, and archive loading for `tar`.

use std::sync::Arc;

use crate::commands::types::{CommandContext, CommandResult};
use crate::commands::vfs_walk::relative_child;
use crate::fs::types::FileSystem;
use crate::shell::pattern_utils;

use super::archive::{decompress_gzip, is_gzip, parse_archive, TarEntry};

/// Check if a path matches any exclude pattern.
pub(crate) fn matches_exclude(path: &str, patterns: &[String]) -> bool {
    let basename = if let Some(pos) = path.rfind('/') {
        &path[pos + 1..]
    } else {
        path
    };

    for pattern in patterns {
        if pattern_utils::matches_shell_glob(pattern, path) {
            return true;
        }
        let with_slash = format!("{}/", pattern);
        if pattern_utils::matches_shell_glob(&with_slash, path) || path.starts_with(&with_slash) {
            return true;
        }
        if !pattern.contains('/') && pattern_utils::matches_shell_glob(pattern, basename) {
            return true;
        }
    }
    false
}

/// Strip N leading path components.
pub(crate) fn strip_components(path: &str, count: usize) -> String {
    if count == 0 {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() <= count {
        return String::new();
    }
    parts[count..].join("/")
}

/// Format file mode for verbose output (like ls -l).
pub(crate) fn format_mode(mode: u32, is_dir: bool) -> String {
    let prefix = if is_dir { 'd' } else { '-' };
    let perms = [
        if mode & 0o400 != 0 { 'r' } else { '-' },
        if mode & 0o200 != 0 { 'w' } else { '-' },
        if mode & 0o100 != 0 { 'x' } else { '-' },
        if mode & 0o040 != 0 { 'r' } else { '-' },
        if mode & 0o020 != 0 { 'w' } else { '-' },
        if mode & 0o010 != 0 { 'x' } else { '-' },
        if mode & 0o004 != 0 { 'r' } else { '-' },
        if mode & 0o002 != 0 { 'w' } else { '-' },
        if mode & 0o001 != 0 { 'x' } else { '-' },
    ];
    let mut s = String::with_capacity(10);
    s.push(prefix);
    for c in &perms {
        s.push(*c);
    }
    s
}

/// Format a unix timestamp for verbose output.
pub(crate) fn format_mtime(mtime: u64) -> String {
    let secs = mtime;
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    let mut y = 1970i64;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let month_days = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0usize;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining_days < md {
            month = i;
            break;
        }
        remaining_days -= md;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        y,
        month + 1,
        day,
        hours,
        minutes
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Convert SystemTime to unix timestamp (seconds since epoch).
pub(crate) fn system_time_to_unix(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Recursively collect files from the virtual filesystem.
pub(crate) async fn collect_files(
    fs: &Arc<dyn FileSystem>,
    base_path: &str,
    relative_path: &str,
    exclude: &[String],
    entries: &mut Vec<TarEntry>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let full_path = fs.resolve_path(base_path, relative_path);

    if matches_exclude(relative_path, exclude) {
        return errors;
    }

    let stat = match fs.stat(&full_path).await {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!("tar: {}: {}", relative_path, e));
            return errors;
        }
    };

    if stat.is_directory {
        entries.push(TarEntry {
            path: relative_path.to_string(),
            content: Vec::new(),
            mode: stat.mode,
            size: 0,
            mtime: system_time_to_unix(stat.mtime),
            is_directory: true,
            is_symlink: false,
            link_target: String::new(),
        });

        let sorted_items =
            match crate::commands::vfs_walk::read_dir_sorted(fs.as_ref(), &full_path).await {
                Ok(items) => items,
                Err(e) => {
                    errors.push(format!("tar: {}: {}", relative_path, e));
                    return errors;
                }
            };

        for item in sorted_items {
            let child_rel = relative_child(relative_path, &item);
            let child_errors =
                Box::pin(collect_files(fs, base_path, &child_rel, exclude, entries)).await;
            errors.extend(child_errors);
        }
    } else if stat.is_file {
        let content = match fs.read_file_buffer(&full_path).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("tar: {}: {}", relative_path, e));
                return errors;
            }
        };
        entries.push(TarEntry {
            path: relative_path.to_string(),
            content: content.clone(),
            mode: stat.mode,
            size: content.len() as u64,
            mtime: system_time_to_unix(stat.mtime),
            is_directory: false,
            is_symlink: false,
            link_target: String::new(),
        });
    }

    errors
}

/// Read and decompress an archive from file or stdin.
pub(crate) async fn read_archive(
    ctx: &CommandContext,
    file: &Option<String>,
    use_gzip: bool,
) -> Result<Vec<TarEntry>, CommandResult> {
    let archive_data = if let Some(ref f) = file {
        if f == "-" {
            ctx.stdin.chars().map(|c| c as u8).collect::<Vec<u8>>()
        } else {
            let archive_path = ctx.fs.resolve_path(&ctx.cwd, f);
            match ctx.fs.read_file_buffer(&archive_path).await {
                Ok(data) => data,
                Err(_) => {
                    return Err(CommandResult::with_exit_code(
                        String::new(),
                        format!("tar: {}: Cannot open: No such file or directory\n", f),
                        2,
                    ));
                }
            }
        }
    } else {
        ctx.stdin.chars().map(|c| c as u8).collect::<Vec<u8>>()
    };

    let data = if use_gzip || is_gzip(&archive_data) {
        match decompress_gzip(&archive_data) {
            Ok(d) => d,
            Err(e) => {
                return Err(CommandResult::with_exit_code(
                    String::new(),
                    format!("tar: gzip decompression error: {}\n", e),
                    2,
                ));
            }
        }
    } else {
        archive_data
    };

    match parse_archive(&data) {
        Ok(entries) => Ok(entries),
        Err(e) => Err(CommandResult::with_exit_code(
            String::new(),
            format!("tar: {}\n", e),
            2,
        )),
    }
}
