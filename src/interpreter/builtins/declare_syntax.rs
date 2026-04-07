//! Shared regex patterns for `declare`, `local`, and related builtins.

use regex_lite::Regex;
use std::sync::OnceLock;

pub fn valid_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap())
}

pub fn valid_target_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(\[.+\])?$").unwrap())
}

/// Prefix of a bash name at the start of a string (for subscript parsing).
pub fn name_prefix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*").unwrap())
}

pub fn array_assign_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)=\((.*)\)$").unwrap())
}

pub fn array_append_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\+=\((.*)\)$").unwrap())
}

pub fn append_scalar_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\+=(.*)$").unwrap())
}

pub fn index_assign_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\[([^\]]+)\]=(.*)$").unwrap())
}

pub fn keyed_elem_prefix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[[^\]]+\]=").unwrap())
}

pub fn keyed_elem_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[([^\]]+)\]=(.*)$").unwrap())
}
