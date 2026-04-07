//! LOC heatmap for just-bash (`src/` + `tests/`).
//!
//! Run from crate root:
//!   cargo run --bin loc-heatmap
//! Snapshot:
//!   cargo run --bin loc-heatmap > scripts/LOC_HEATMAP.txt

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

fn main() -> io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if !root.join("Cargo.toml").exists() {
        eprintln!("Expected Cargo.toml at {}", root.display());
        std::process::exit(1);
    }

    let mut by_prefix: HashMap<String, Vec<(PathBuf, u32, u32, u32)>> = HashMap::new();
    let mut grand_total = 0u32;
    let mut grand_test = 0u32;
    let mut grand_prod = 0u32;

    for base in [&root.join("src"), &root.join("tests")] {
        if !base.is_dir() {
            continue;
        }
        let mut paths = Vec::new();
        collect_rs_files(base, &mut paths);
        paths.sort();
        for path in paths {
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let (total, test_lines, prod_lines) = line_stats(&text);
            let Ok(rel) = path.strip_prefix(&root) else {
                continue;
            };
            let (t_lines, p_lines) =
                if rel.components().next() == Some(Component::Normal(OsStr::new("tests"))) {
                    (total, 0)
                } else {
                    (test_lines, prod_lines)
                };
            grand_total += total;
            grand_test += t_lines;
            grand_prod += p_lines;
            let prefix = prefix_for(rel);
            by_prefix
                .entry(prefix)
                .or_default()
                .push((rel.to_path_buf(), total, t_lines, p_lines));
        }
    }

    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "just-bash LOC heatmap (cfg(test) mod blocks in src; tests/*.rs counted as all test)"
    )?;
    writeln!(stdout, "{}", "=".repeat(72))?;
    writeln!(
        stdout,
        "{:<28} {:>8} {:>10} {:>10}",
        "scope", "total", "non-test", "test-ish"
    )?;
    writeln!(stdout, "{}", "-".repeat(72))?;

    let mut keys: Vec<_> = by_prefix.keys().cloned().collect();
    keys.sort();
    for prefix in &keys {
        let rows = &by_prefix[prefix];
        let t: u32 = rows.iter().map(|r| r.1).sum();
        let tt: u32 = rows.iter().map(|r| r.2).sum();
        let tp: u32 = rows.iter().map(|r| r.3).sum();
        if prefix == "tests" {
            writeln!(
                stdout,
                "{prefix:<28} {t:>8} {:>10} {t:>10}  (integration tests dir)",
                0
            )?;
            continue;
        }
        writeln!(stdout, "{prefix:<28} {t:>8} {tp:>10} {tt:>10}")?;
    }

    writeln!(stdout, "{}", "-".repeat(72))?;
    writeln!(
        stdout,
        "{:<28} {:>8} {:>10} {:>10}",
        "ALL (src+tests)", grand_total, grand_prod, grand_test
    )?;
    writeln!(stdout)?;
    writeln!(
        stdout,
        "Largest files (total lines, non-test, test-ish, path)"
    )?;
    writeln!(stdout, "{}", "-".repeat(72))?;

    let mut all_rows: Vec<_> = by_prefix.values().flatten().collect();
    all_rows.sort_by(|a, b| b.1.cmp(&a.1));
    for (rel, total, tt, tp) in all_rows.into_iter().take(35) {
        writeln!(
            stdout,
            "{:>6} {:>6} {:>6}  {}",
            total,
            tp,
            tt,
            rel.display()
        )?;
    }

    Ok(())
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension() == Some(OsStr::new("rs")) {
            out.push(p);
        }
    }
}

fn prefix_for(rel: &Path) -> String {
    let parts: Vec<_> = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy()),
            _ => None,
        })
        .collect();
    if parts.len() >= 2 && parts[0] == "src" {
        format!("src/{}", parts[1])
    } else {
        parts.first().cloned().unwrap_or_default().into_owned()
    }
}

fn strip_line_comment(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut prev = b' ';
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if (ch == b'"' || ch == b'\'') && prev != b'\\' {
            in_string = !in_string;
        }
        if !in_string && ch == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            return String::from_utf8_lossy(&bytes[..i]).into_owned();
        }
        prev = ch;
        i += 1;
    }
    line.to_string()
}

fn cfg_test_regions(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let n = lines.len();
    let mut i = 0;
    while i < n {
        if lines[i].trim() == "#[cfg(test)]" {
            let mut j = i + 1;
            while j < n && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= n {
                break;
            }
            let mod_line = lines[j].trim();
            if let Some(open_idx) = mod_line.find('{') {
                if mod_line.starts_with("mod ") {
                    let after = &mod_line[open_idx..];
                    let mut depth = after.matches('{').count() as i32;
                    depth -= after.matches('}').count() as i32;
                    let start = j;
                    j += 1;
                    while j < n && depth > 0 {
                        let s = strip_line_comment(lines[j]);
                        depth += s.matches('{').count() as i32;
                        depth -= s.matches('}').count() as i32;
                        j += 1;
                    }
                    regions.push((start, j));
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
    regions
}

fn line_stats(text: &str) -> (u32, u32, u32) {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len() as u32;
    let regions = cfg_test_regions(&lines);
    let mut in_test: HashSet<usize> = HashSet::new();
    for (start, end) in regions {
        for li in start..end {
            in_test.insert(li);
        }
    }
    let test_lines = in_test.len() as u32;
    let prod_lines = total.saturating_sub(test_lines);
    (total, test_lines, prod_lines)
}
