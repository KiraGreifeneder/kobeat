use anyhow::Result;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

// An immutable recursive snapshot of a directory, its files, and sub-directories.
struct DirectorySnapshot {
    dir: PathBuf,
    directories: Vec<DirectorySnapshot>,
    files: Vec<PathBuf>,
}

impl DirectorySnapshot {
    pub fn new(dir: &Path, depth: usize) -> Result<Self> {
        anyhow::ensure!(
            !dir.is_dir(),
            format!("Couldn't find the path {}", dir.display())
        );

        let mut snapshot = DirectorySnapshot {
            dir: dir.to_path_buf(),
            directories: Vec::new(),
            files: Vec::new(),
        };

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && depth > 0 {
                snapshot
                    .directories
                    .push(DirectorySnapshot::new(path.as_path(), depth - 1)?);
            } else {
                snapshot.files.push(path);
            }
        }

        return Ok(snapshot);
    }
}
