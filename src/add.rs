use std::{fs, io, path::PathBuf};

use crate::{
    file::{self, LockFile},
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
    workspace::{Database, Workspace},
};

static GITIGNORE: [&str; 1] = ["Cargo.lock"];

pub fn add(path: PathBuf, workspace: &Workspace, database: &Database) -> io::Result<()> {
    if GITIGNORE.contains(&path.to_str().expect("Path was bad UTF8")) {
        return Ok(());
    }
    let index_file_path = workspace.git_dir().join("index");
    let mut index_lockfile = LockFile::acquire(&index_file_path)?;

    let absolute_path = workspace.workdir().join(&path);
    let file_bytes = file::read_file(&absolute_path)?;
    let blob = Blob::new(file_bytes);
    database.store_object(&blob)?;

    let metadata = fs::metadata(&absolute_path)?;
    let entry = IndexEntry::new(path, blob.id(), &metadata);

    let mut index = if index_file_path.is_file() {
        let index_bytes = file::read_file(&index_file_path)?;

        // TODO handle error from reading index
        Index::from_bytes(&index_bytes).ok().unwrap()
    } else {
        Index::new()
    };

    index.add_entry(entry);

    index_lockfile.write(&index.as_vec())
}
