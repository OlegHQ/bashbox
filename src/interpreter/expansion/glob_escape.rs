//! Glob Helper Functions
//!
//! Functions for handling glob patterns, escaping, and unescaping.

use regex_lite::Regex;
use std::path::Path;

/// Check if a string contains glob patterns, including extglob when enabled.
pub fn has_glob_pattern(value: &str, extglob: bool) -> bool {
    // Standard glob characters
    if value.chars().any(|c| c == '*' || c == '?' || c == '[') {
        return true;
    }
    // Extglob patterns: @(...), *(...), +(...), ?(...), !(...)
    if extglob {
        let extglob_re = Regex::new(r"[@*+?!]\(").unwrap();
        if extglob_re.is_match(value) {
            return true;
        }
    }
    false
}

/// Unescape a glob pattern - convert escaped glob chars to literal chars.
/// For example, [\]_ (escaped pattern) becomes [\\]_ (literal string).
///
/// This is used when we need to take a pattern that was built with escaped
/// glob characters and convert it back to a literal string (e.g., for
/// no-match fallback when nullglob is off).
///
/// Note: The input is expected to be a pattern string where backslashes escape
/// the following character. For patterns like "test\\[*" (user input: test\[*)
/// the output is "\\_" (with processed escapes), not [\\]_ (raw pattern).
pub fn unescape_glob_pattern(pattern: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            // Backslash escapes the next character - output just the escaped char
            result.push(chars[i + 1]);
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Escape glob metacharacters in a string for literal matching.
/// Includes extglob metacharacters: ( ) |
pub fn escape_glob_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '*' | '?' | '[' | ']' | '\\' | '(' | ')' | '|' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

/// Escape regex metacharacters in a string for literal matching.
/// Used when quoted patterns are used with =~ operator.
pub fn escape_regex_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' | '^' | '$' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

/// Per-field pathname expansion for already-split words (e.g. after `$@` / `$*` IFS split).
///
/// When no virtual filesystem is in play, this matches [`crate::interpreter::word_expansion::expand_word_with_glob`]
/// with `fs: None`: glob-active fields behave as if there were no matches (no host path scan).
pub fn split_and_glob_expand(
    words: &[String],
    _cwd: &Path,
    failglob: bool,
    nullglob: bool,
    noglob: bool,
    extglob: bool,
) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(words.len());
    for word in words {
        if noglob || !has_glob_pattern(word, extglob) {
            out.push(word.clone());
            continue;
        }
        if failglob {
            return Err(format!("no match: {word}"));
        }
        if nullglob {
            continue;
        }
        out.push(unescape_glob_pattern(word));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_glob_pattern() {
        assert!(has_glob_pattern("*.txt", false));
        assert!(has_glob_pattern("file?.rs", false));
        assert!(has_glob_pattern("[abc]", false));
        assert!(!has_glob_pattern("plain.txt", false));

        // Extglob patterns
        assert!(!has_glob_pattern("@(a|b)", false));
        assert!(has_glob_pattern("@(a|b)", true));
        assert!(has_glob_pattern("*(foo)", true));
        assert!(has_glob_pattern("+(bar)", true));
        assert!(has_glob_pattern("?(baz)", true));
        assert!(has_glob_pattern("!(qux)", true));
    }

    #[test]
    fn test_unescape_glob_pattern() {
        assert_eq!(unescape_glob_pattern(r"\*"), "*");
        assert_eq!(unescape_glob_pattern(r"\?\["), "?[");
        assert_eq!(unescape_glob_pattern(r"a\*b"), "a*b");
        assert_eq!(unescape_glob_pattern("plain"), "plain");
    }

    #[test]
    fn test_escape_glob_chars() {
        assert_eq!(escape_glob_chars("*"), r"\*");
        assert_eq!(escape_glob_chars("?"), r"\?");
        assert_eq!(escape_glob_chars("[a]"), r"\[a\]");
        assert_eq!(escape_glob_chars("plain"), "plain");
        assert_eq!(escape_glob_chars("a|b"), r"a\|b");
    }

    #[test]
    fn test_escape_regex_chars() {
        assert_eq!(escape_regex_chars("a.b"), r"a\.b");
        assert_eq!(escape_regex_chars("a*b"), r"a\*b");
        assert_eq!(escape_regex_chars("[a]"), r"\[a\]");
        assert_eq!(escape_regex_chars("^$"), r"\^\$");
        assert_eq!(escape_regex_chars("plain"), "plain");
    }

    #[test]
    fn split_and_glob_expand_noglob_passthrough() {
        let cwd = std::path::Path::new("/");
        let words = vec!["*.txt".to_string(), "plain".to_string()];
        let got = split_and_glob_expand(&words, cwd, false, false, true, false).unwrap();
        assert_eq!(got, words);
    }

    #[test]
    fn split_and_glob_expand_no_fs_failglob() {
        let cwd = std::path::Path::new("/");
        let words = vec!["*.nothing".to_string()];
        let err = split_and_glob_expand(&words, cwd, true, false, false, false).unwrap_err();
        assert_eq!(err, "no match: *.nothing");
    }

    #[test]
    fn split_and_glob_expand_no_fs_nullglob() {
        let cwd = std::path::Path::new("/");
        let words = vec!["a".to_string(), "*.nothing".to_string()];
        let got = split_and_glob_expand(&words, cwd, false, true, false, false).unwrap();
        assert_eq!(got, vec!["a".to_string()]);
    }

    #[test]
    fn split_and_glob_expand_no_fs_literal_fallback() {
        let cwd = std::path::Path::new("/");
        let words = vec![r"a\*b".to_string()];
        let got = split_and_glob_expand(&words, cwd, false, false, false, false).unwrap();
        assert_eq!(got, vec!["a*b".to_string()]);
    }
}
