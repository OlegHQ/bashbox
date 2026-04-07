//! Canonical path normalization for the virtual filesystem.
//!
//! Resolves `.`, `..`, duplicate slashes, and trailing slashes without
//! touching the real filesystem (no symlink resolution).

/// Normalize a path by resolving `.`, `..`, duplicate slashes, and trailing
/// slashes.
///
/// All results are absolute (rooted at `/`).  Relative input is treated as
/// though it were already under `/`.  Empty input maps to `"/"`.
/// `..` past the root is silently absorbed.
///
/// No symlink resolution is performed — this is purely lexical.
pub fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/".to_string();
    }

    let mut components: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            c => {
                components.push(c);
            }
        }
    }

    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

/// Join an absolute directory path with a single entry name (`/a` + `b` → `/a/b`).
/// Trailing slashes on `parent` are trimmed; root `/` stays special-cased.
pub fn child_path(parent: &str, entry_name: &str) -> String {
    let base = if parent == "/" {
        "/"
    } else {
        parent.trim_end_matches('/')
    };
    if base == "/" {
        format!("/{}", entry_name)
    } else {
        format!("{}/{}", base, entry_name)
    }
}

/// Join a relative path prefix (possibly empty) with a name (`a/b` + `c` → `a/b/c`).
pub fn relative_child(parent_rel: &str, name: &str) -> String {
    if parent_rel.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", parent_rel, name)
    }
}

/// If `path` is absolute, return it; otherwise join under `cwd` (trailing slashes on `cwd` trimmed).
pub fn resolve_under_cwd(cwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        let base = cwd.trim_end_matches('/');
        if base.is_empty() {
            child_path("/", path)
        } else {
            relative_child(base, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(normalize_path(""), "/");
    }

    #[test]
    fn test_root() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_absolute_simple() {
        assert_eq!(normalize_path("/foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_trailing_slash() {
        assert_eq!(normalize_path("/foo/bar/"), "/foo/bar");
    }

    #[test]
    fn test_relative_becomes_absolute() {
        assert_eq!(normalize_path("foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_dot_removal() {
        assert_eq!(normalize_path("/foo/./bar"), "/foo/bar");
    }

    #[test]
    fn test_dotdot_resolution() {
        assert_eq!(normalize_path("/foo/../bar"), "/bar");
        assert_eq!(normalize_path("/foo/bar/.."), "/foo");
        assert_eq!(normalize_path("/foo/bar/../baz"), "/foo/baz");
    }

    #[test]
    fn test_dotdot_at_root() {
        assert_eq!(normalize_path("/foo/bar/../.."), "/");
        assert_eq!(normalize_path("/../.."), "/");
    }

    #[test]
    fn test_double_slash() {
        assert_eq!(normalize_path("/foo//bar"), "/foo/bar");
    }

    #[test]
    fn child_path_root() {
        assert_eq!(child_path("/", "x"), "/x");
    }

    #[test]
    fn child_path_absolute() {
        assert_eq!(child_path("/a/b", "c"), "/a/b/c");
    }

    #[test]
    fn child_path_trailing_slash_on_parent() {
        assert_eq!(child_path("/a/b/", "c"), "/a/b/c");
    }

    #[test]
    fn relative_child_empty_parent() {
        assert_eq!(relative_child("", "foo"), "foo");
    }

    #[test]
    fn relative_child_join() {
        assert_eq!(relative_child("a", "b"), "a/b");
    }

    #[test]
    fn resolve_under_cwd_absolute() {
        assert_eq!(resolve_under_cwd("/any", "/foo"), "/foo");
    }

    #[test]
    fn resolve_under_cwd_from_root_cwd() {
        assert_eq!(resolve_under_cwd("/", "x"), "/x");
    }

    #[test]
    fn resolve_under_cwd_nested() {
        assert_eq!(resolve_under_cwd("/app", "y"), "/app/y");
    }
}
