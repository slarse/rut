use std::{fs, io, path::PathBuf};

use crate::{
    file,
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
    workspace::{Database, Workspace},
};

static GITIGNORE: [&str; 1] = ["Cargo.lock"];

pub fn add(path: PathBuf, workspace: &Workspace, database: &Database) -> io::Result<()> {
    if GITIGNORE.contains(&path.to_str().expect("Path was bad UTF8")) {
        return Ok(());
    }

    let absolute_path = workspace.workdir().join(&path);
    let file_bytes = file::read_file(&absolute_path)?;
    let blob = Blob::new(file_bytes);
    database.store_object(&blob)?;

    let metadata = fs::metadata(&absolute_path)?;
    let entry = IndexEntry::new(path, blob.id(), &metadata);

    let index_file = workspace.git_dir().join("index");
    let mut index = if index_file.is_file() {
        let index_bytes = file::read_file(&index_file)?;

        // TODO handle error from reading index
        Index::from_bytes(&index_bytes).ok().unwrap()
    } else {
        Index::new()
    };

    index.add_entry(entry);

    fs::write(&index_file, index.as_vec())
}
