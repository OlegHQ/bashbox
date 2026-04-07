//! CLI argument parsing for `sort`.

use super::comparator::{parse_key_spec, SortOptions};

pub(crate) fn parse_sort_args(args: &[String]) -> (SortOptions, Vec<String>) {
    let mut opts = SortOptions::default();
    let mut files: Vec<String> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--" {
            i += 1;
            while i < args.len() {
                files.push(args[i].clone());
                i += 1;
            }
            break;
        }

        if arg.starts_with("--") {
            match arg.as_str() {
                "--reverse" => opts.reverse = true,
                "--numeric-sort" => opts.numeric = true,
                "--unique" => opts.unique = true,
                "--ignore-case" => opts.ignore_case = true,
                "--human-numeric-sort" => opts.human_numeric = true,
                "--version-sort" => opts.version_sort = true,
                "--dictionary-order" => opts.dictionary_order = true,
                "--month-sort" => opts.month_sort = true,
                "--ignore-leading-blanks" => opts.ignore_leading_blanks = true,
                "--stable" => opts.stable = true,
                "--check" => opts.check = true,
                _ if arg.starts_with("--output=") => {
                    opts.output_file = Some(arg[9..].to_string());
                }
                _ if arg.starts_with("--key=") => {
                    opts.keys.push(parse_key_spec(&arg[6..]));
                }
                _ if arg.starts_with("--field-separator=") => {
                    let sep = &arg[18..];
                    if let Some(c) = sep.chars().next() {
                        opts.field_separator = Some(c);
                    }
                }
                _ => {}
            }
            i += 1;
            continue;
        }

        if arg.starts_with('-') && arg.len() > 1 {
            let chars: Vec<char> = arg[1..].chars().collect();
            let mut j = 0;
            let mut consumed_next = false;

            while j < chars.len() {
                match chars[j] {
                    'r' => opts.reverse = true,
                    'n' => opts.numeric = true,
                    'u' => opts.unique = true,
                    'f' => opts.ignore_case = true,
                    'h' => opts.human_numeric = true,
                    'V' => opts.version_sort = true,
                    'd' => opts.dictionary_order = true,
                    'M' => opts.month_sort = true,
                    'b' => opts.ignore_leading_blanks = true,
                    's' => opts.stable = true,
                    'c' => opts.check = true,
                    'o' => {
                        let rest: String = chars[j + 1..].iter().collect();
                        if !rest.is_empty() {
                            opts.output_file = Some(rest);
                        } else if i + 1 < args.len() {
                            i += 1;
                            opts.output_file = Some(args[i].clone());
                            consumed_next = true;
                        }
                        j = chars.len();
                        continue;
                    }
                    'k' => {
                        let rest: String = chars[j + 1..].iter().collect();
                        if !rest.is_empty() {
                            opts.keys.push(parse_key_spec(&rest));
                        } else if i + 1 < args.len() {
                            i += 1;
                            opts.keys.push(parse_key_spec(&args[i]));
                            consumed_next = true;
                        }
                        j = chars.len();
                        continue;
                    }
                    't' => {
                        let rest: String = chars[j + 1..].iter().collect();
                        if !rest.is_empty() {
                            opts.field_separator = rest.chars().next();
                        } else if i + 1 < args.len() {
                            i += 1;
                            opts.field_separator = args[i].chars().next();
                            consumed_next = true;
                        }
                        j = chars.len();
                        continue;
                    }
                    _ => {}
                }
                j += 1;
            }

            let _ = consumed_next;
            i += 1;
            continue;
        }

        files.push(arg.clone());
        i += 1;
    }

    (opts, files)
}
