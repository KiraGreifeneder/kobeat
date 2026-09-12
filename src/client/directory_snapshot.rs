use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// An immutable, recursive snapshot of a directory, its files, and its
/// sub-directories.
///
/// Nodes are stored flat and keyed by their own path, so a node's parent is
/// found with [`Path::parent`] plus a map lookup. That avoids a
/// self-referential struct (which cannot be expressed with `Box`) while still
/// giving cheap upward navigation without re-reading the filesystem.
pub struct DirectoryTree {
    root: PathBuf,
    nodes: HashMap<PathBuf, DirectoryNode>,
}

/// A single scanned directory. `subdirs` holds the paths of child directories,
/// which double as keys back into [`DirectoryTree`].
pub struct DirectoryNode {
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub subdirs: Vec<PathBuf>,
}

impl DirectoryTree {
    /// Scans `root`, descending at most `depth` levels of sub-directories.
    pub fn scan(root: &Path, depth: usize) -> Result<Self> {
        anyhow::ensure!(root.is_dir(), "couldn't find the path {}", root.display());

        let root = root.to_path_buf();
        let mut nodes = HashMap::new();
        let mut pending = vec![(root.clone(), depth)];

        while let Some((dir, remaining)) = pending.pop() {
            let mut node = DirectoryNode {
                dir: dir.clone(),
                files: Vec::new(),
                subdirs: Vec::new(),
            };

            let entries = fs::read_dir(&dir)
                .with_context(|| format!("failed to read directory {}", dir.display()))?;

            for entry in entries {
                let path = entry?.path();

                if path.is_dir() {
                    // Directories beyond the depth limit are skipped entirely
                    // rather than being recorded as playable files.
                    if remaining > 0 {
                        node.subdirs.push(path.clone());
                        pending.push((path, remaining - 1));
                    }
                } else {
                    node.files.push(path);
                }
            }

            node.subdirs.sort();
            node.files.sort();
            nodes.insert(dir, node);
        }

        Ok(Self { root, nodes })
    }

    pub fn root_path(&self) -> &Path {
        &self.root.as_path()
    }

    pub fn root(&self) -> &DirectoryNode {
        self.node(&self.root)
            .expect("the root directory is always scanned")
    }

    pub fn node(&self, dir: &Path) -> Option<&DirectoryNode> {
        self.nodes.get(dir)
    }

    /// The parent of `dir`, or `None` at the root of the scanned tree.
    ///
    /// Self-limiting: the root's parent was never scanned, so it is absent from
    /// the map and the lookup simply misses.
    pub fn parent(&self, dir: &Path) -> Option<&DirectoryNode> {
        dir.parent().and_then(|parent| self.nodes.get(parent))
    }

    pub fn children(&self, dir: &Path) -> Vec<&DirectoryNode> {
        self.node(dir)
            .into_iter()
            .flat_map(|node| node.subdirs.iter())
            .filter_map(|path| self.nodes.get(path.as_path()))
            .collect()
    }

    /// Scanned ancestors of `dir`, ordered root first, for UI breadcrumbs.
    pub fn ancestors(&self, dir: &Path) -> Vec<&DirectoryNode> {
        let mut crumbs: Vec<&DirectoryNode> = dir
            .ancestors()
            .filter_map(|path| self.nodes.get(path))
            .collect();
        crumbs.reverse();
        crumbs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kobeat-tree-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after the unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn parent_links_back_without_rescanning() -> Result<()> {
        let root = temp_dir("parent");
        let child = root.join("albums");
        fs::create_dir_all(&child)?;
        File::create(child.join("song.mp3"))?;

        let tree = DirectoryTree::scan(&root, 3)?;

        assert_eq!(
            tree.parent(&child).map(|node| node.dir.as_path()),
            Some(root.as_path())
        );
        assert!(tree.parent(&root).is_none(), "root has no scanned parent");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn directories_beyond_depth_are_not_recorded_as_files() -> Result<()> {
        let root = temp_dir("depth");
        fs::create_dir_all(root.join("too-deep"))?;

        let tree = DirectoryTree::scan(&root, 0)?;
        let node = tree.root();

        assert!(node.files.is_empty(), "a directory was misfiled as a file");
        assert!(node.subdirs.is_empty(), "depth limit was not respected");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn scan_rejects_missing_root() {
        let missing = std::env::temp_dir().join("kobeat-tree-definitely-missing");
        assert!(DirectoryTree::scan(&missing, 1).is_err());
    }
}
