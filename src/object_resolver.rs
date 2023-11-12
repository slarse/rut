use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    objects::{Blob, Tree},
    refs::RefHandler,
    workspace::{Database, Repository},
};

pub struct ObjectResolver<'a> {
    trees: HashMap<PathBuf, Tree>,
    blobs: HashMap<PathBuf, Blob>,
    database: &'a Database,
}

impl<'a> ObjectResolver<'a> {
    pub fn new(root_tree: Tree, database: &'a Database) -> Self {
        let mut trees = HashMap::new();
        trees.insert(PathBuf::new(), root_tree);
        Self {
            trees,
            database,
            blobs: HashMap::new(),
        }
    }

    pub fn from_head_commit(repository: &'a Repository) -> crate::Result<Self> {
        ObjectResolver::from_reference("HEAD", repository)
    }

    pub fn from_reference(reference: &str, repository: &'a Repository) -> crate::Result<Self> {
        let commit_id = RefHandler::new(repository).deref(reference)?;
        let commit = repository.database.load_commit(&commit_id)?;
        let root_tree = repository.database.load_tree(&commit.tree)?;

        Ok(ObjectResolver::new(root_tree, &repository.database))
    }

    /// Find a blob by its path, relative to the root tree of this ObjectResolver.
    pub fn find_blob_by_path(&mut self, path: &Path) -> crate::Result<Blob> {
        if let Some(blob) = self.blobs.get(path) {
            return Ok(blob.clone());
        }

        let parent_path = self.resolve_closest_cached_tree_path(path);
        let remaining_path = path.strip_prefix(&parent_path).unwrap();

        self.find_blob_in_tree_(&parent_path, remaining_path)
    }

    fn resolve_closest_cached_tree_path(&self, path: &Path) -> PathBuf {
        if self.trees.contains_key(path) {
            return path.to_owned();
        }

        self.resolve_closest_cached_tree_path(path.parent().unwrap())
    }

    fn find_blob_in_tree_(
        &mut self,
        parent_path: &Path,
        remaining_path: &Path,
    ) -> crate::Result<Blob> {
        if remaining_path.components().count() <= 1 {
            return self.get_blob(&parent_path.join(remaining_path));
        }

        self.find_blob_in_subtree(parent_path, remaining_path)
    }

    /// Recursively find a blob in a subtree. Cache any trees found along the way.
    fn find_blob_in_subtree(
        &mut self,
        parent_path: &Path,
        remaining_path: &Path,
    ) -> crate::Result<Blob> {
        let mut path_components = remaining_path.iter().map(|p| p.to_str().unwrap());
        let root_component = path_components.next().unwrap();
        let current_path = parent_path.join(root_component);

        let mut curent_remaining_path = PathBuf::new();
        for component in path_components {
            curent_remaining_path = curent_remaining_path.join(component);
        }

        let parent_tree = self.trees.get(parent_path).unwrap();

        let tree_entry = parent_tree
            .entries()
            .iter()
            .find(|e| e.name == root_component)
            .unwrap();
        let current_tree = self.database.load_tree(&tree_entry.object_id).unwrap();

        self.trees.insert(current_path.clone(), current_tree);

        self.find_blob_in_tree_(&current_path, &curent_remaining_path)
    }

    /// Get a blob assuming its parent tree is already cached.
    fn get_blob(&mut self, blob_path: &Path) -> crate::Result<Blob> {
        let file_name = blob_path.file_name().unwrap().to_str().unwrap();
        let tree = &self.trees[blob_path.parent().unwrap()];

        for entry in tree.entries() {
            if entry.name == file_name {
                let committed_blob = self.database.load_blob(&entry.object_id).unwrap();
                self.blobs
                    .insert(blob_path.to_path_buf(), committed_blob.clone());
                return Ok(committed_blob);
            }
        }

        Err(crate::Error::Fatal(
            None,
            format!("pathspec '{}' did not match any files", blob_path.display()),
        ))
    }
}
