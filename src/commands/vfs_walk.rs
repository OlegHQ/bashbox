//! Shared helpers for virtual filesystem directory reads and path joining.

pub use crate::fs::{child_path, relative_child};

use crate::fs::{FileSystem, FsError};

/// Read a directory and return sorted entry names (stable, bash-friendly ordering).
pub async fn read_dir_sorted(fs: &dyn FileSystem, path: &str) -> Result<Vec<String>, FsError> {
    let mut names = fs.readdir(path).await?;
    names.sort();
    Ok(names)
}
