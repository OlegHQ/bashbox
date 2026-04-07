// src/commands/grep/mod.rs
use crate::commands::arg_helpers::wants_help;
use crate::commands::vfs_helpers::read_file_accumulate_errors;
use crate::commands::{Command, CommandContext, CommandResult};
use async_trait::async_trait;
use regex_lite::Regex;

pub struct GrepCommand;

struct GrepOptions {
    /// One or more patterns; multiple `-e` / `--regexp` are OR'd (GNU grep).
    pattern_parts: Vec<String>,
    ignore_case: bool,
    invert_match: bool,
    count_only: bool,
    files_with_matches: bool,
    files_without_matches: bool,
    line_number: bool,
    only_matching: bool,
    quiet: bool,
    fixed_strings: bool,
    max_count: Option<usize>,
    files: Vec<String>,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            pattern_parts: Vec::new(),
            ignore_case: false,
            invert_match: false,
            count_only: false,
            files_with_matches: false,
            files_without_matches: false,
            line_number: false,
            only_matching: false,
            quiet: false,
            fixed_strings: false,
            max_count: None,
            files: Vec::new(),
        }
    }
}

fn parse_grep_args(args: &[String]) -> Result<GrepOptions, String> {
    let mut opts = GrepOptions::default();
    let mut positional: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if (arg == "-e" || arg == "--regexp") && i + 1 < args.len() {
            i += 1;
            opts.pattern_parts.push(args[i].clone());
        } else if arg == "-i" || arg == "--ignore-case" {
            opts.ignore_case = true;
        } else if arg == "-v" || arg == "--invert-match" {
            opts.invert_match = true;
        } else if arg == "-c" || arg == "--count" {
            opts.count_only = true;
        } else if arg == "-l" || arg == "--files-with-matches" {
            opts.files_with_matches = true;
        } else if arg == "-L" || arg == "--files-without-match" {
            opts.files_without_matches = true;
        } else if arg == "-n" || arg == "--line-number" {
            opts.line_number = true;
        } else if arg == "-o" || arg == "--only-matching" {
            opts.only_matching = true;
        } else if arg == "-q" || arg == "--quiet" || arg == "--silent" {
            opts.quiet = true;
        } else if arg == "-F" || arg == "--fixed-strings" {
            opts.fixed_strings = true;
        } else if arg == "-E" || arg == "--extended-regexp" {
            // 默认就是扩展正则
        } else if arg == "-m" && i + 1 < args.len() {
            i += 1;
            opts.max_count = args[i].parse().ok();
        } else if let Some(n) = arg.strip_prefix("-m") {
            opts.max_count = n.parse().ok();
        } else if !arg.starts_with('-') {
            positional.push(arg.clone());
        }
        i += 1;
    }

    // First positional is the pattern unless `-e` already supplied (then all positional are files).
    if opts.pattern_parts.is_empty() {
        if positional.is_empty() {
            return Err("grep: no pattern specified".to_string());
        }
        opts.pattern_parts.push(positional.remove(0));
    }

    opts.files = positional;
    Ok(opts)
}

fn build_regex(opts: &GrepOptions) -> Result<Regex, String> {
    if opts.pattern_parts.is_empty() {
        return Err("grep: no pattern specified".to_string());
    }

    // OR multiple `-e` patterns using non-capturing groups so `|` inside one pattern stays local.
    let mut combined = String::new();
    for (idx, part) in opts.pattern_parts.iter().enumerate() {
        let body = if opts.fixed_strings {
            regex_lite::escape(part)
        } else {
            part.clone()
        };
        if opts.pattern_parts.len() == 1 && body.is_empty() {
            // GNU grep: empty pattern matches every line; `regex_lite` accepts `""`.
            combined = body;
            break;
        }
        if idx > 0 {
            combined.push('|');
        }
        combined.push_str("(?:");
        combined.push_str(&body);
        combined.push(')');
    }

    let mut pattern = combined;
    if opts.ignore_case {
        pattern = format!("(?i){}", pattern);
    }

    Regex::new(&pattern).map_err(|e| format!("grep: invalid pattern: {}", e))
}

#[async_trait]
impl Command for GrepCommand {
    fn name(&self) -> &'static str {
        "grep"
    }

    async fn execute(&self, ctx: CommandContext) -> CommandResult {
        if wants_help(&ctx.args) {
            return CommandResult::success(
                "Usage: grep [OPTION]... PATTERN [FILE]...\n\n\
                 Search for PATTERN in each FILE.\n\n\
                 Options:\n\
                   -E, --extended-regexp  PATTERN is an extended regular expression\n\
                   -F, --fixed-strings    PATTERN is a set of newline-separated strings\n\
                   -i, --ignore-case      ignore case distinctions\n\
                   -v, --invert-match     select non-matching lines\n\
                   -c, --count            print only a count of matching lines\n\
                   -l, --files-with-matches  print only names of FILEs with matches\n\
                   -L, --files-without-match  print only names of FILEs without matches\n\
                   -n, --line-number      print line number with output lines\n\
                   -o, --only-matching    show only the part of a line matching PATTERN\n\
                   -q, --quiet            suppress all normal output\n\
                   -m NUM, --max-count=NUM  stop after NUM matches\n\
                       --help             display this help and exit\n"
                    .to_string(),
            );
        }

        let opts = match parse_grep_args(&ctx.args) {
            Ok(o) => o,
            Err(e) => return CommandResult::error(format!("{}\n", e)),
        };

        let regex = match build_regex(&opts) {
            Ok(r) => r,
            Err(e) => return CommandResult::error(format!("{}\n", e)),
        };

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code = 1; // 默认没有匹配

        // 如果没有文件，从 stdin 读取
        let files = if opts.files.is_empty() {
            vec!["-".to_string()]
        } else {
            opts.files.clone()
        };

        let show_filename = files.len() > 1;

        for file in &files {
            let content = if file == "-" {
                ctx.stdin.clone()
            } else {
                match read_file_accumulate_errors(
                    ctx.fs.as_ref(),
                    &ctx.cwd,
                    "grep",
                    file,
                    &mut stderr,
                    &mut exit_code,
                )
                .await
                {
                    Some(c) => c,
                    None => continue,
                }
            };

            let lines: Vec<&str> = content.lines().collect();
            let mut file_matches = 0;
            let mut matched_lines: Vec<(usize, &str)> = Vec::new();

            for (line_num, line) in lines.iter().enumerate() {
                let matches = regex.is_match(line);
                let should_output = if opts.invert_match { !matches } else { matches };

                if should_output {
                    file_matches += 1;
                    matched_lines.push((line_num + 1, line));

                    if let Some(max) = opts.max_count {
                        if file_matches >= max {
                            break;
                        }
                    }
                }
            }

            // For `-L` / `--files-without-match`, success means at least one file had *no*
            // match (we print names). Do not set exit 0 here just because a file matched.
            if file_matches > 0 && !opts.files_without_matches {
                exit_code = 0;
            }

            if opts.quiet {
                if file_matches > 0 {
                    return CommandResult::with_exit_code(String::new(), stderr, 0);
                }
                continue;
            }

            if opts.files_with_matches {
                if file_matches > 0 {
                    stdout.push_str(&format!("{}\n", file));
                }
                continue;
            }

            if opts.files_without_matches {
                if file_matches == 0 {
                    stdout.push_str(&format!("{}\n", file));
                    exit_code = 0;
                }
                continue;
            }

            if opts.count_only {
                if show_filename {
                    stdout.push_str(&format!("{}:{}\n", file, file_matches));
                } else {
                    stdout.push_str(&format!("{}\n", file_matches));
                }
                continue;
            }

            // 输出匹配的行
            for (line_num, line) in matched_lines {
                let prefix = if show_filename {
                    if opts.line_number {
                        format!("{}:{}:", file, line_num)
                    } else {
                        format!("{}:", file)
                    }
                } else if opts.line_number {
                    format!("{}:", line_num)
                } else {
                    String::new()
                };

                if opts.only_matching {
                    for mat in regex.find_iter(line) {
                        stdout.push_str(&format!("{}{}\n", prefix, mat.as_str()));
                    }
                } else {
                    stdout.push_str(&format!("{}{}\n", prefix, line));
                }
            }
        }

        CommandResult::with_exit_code(stdout, stderr, exit_code)
    }
}

#[cfg(test)]
mod tests;
