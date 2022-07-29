use std::{fs, io, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::{
    file::{self, LockFile},
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
    workspace::{Database, Workspace},
};

static GITIGNORE: [&str; 2] = ["Cargo.lock", "target"];

pub fn add(path: PathBuf, workspace: &Workspace, database: &Database) -> io::Result<()> {
    if GITIGNORE.contains(&path.to_str().expect("Path was bad UTF8")) {
        return Ok(());
    }

    let absolute_path = workspace.workdir().join(&path);

    let index_file_path = workspace.git_dir().join("index");
    let mut index_lockfile = LockFile::acquire(&index_file_path)?;
    let mut index = Index::from_file(&index_file_path)?;

    for path in resolve_files(&absolute_path) {
        add_file(&path, &mut index, &workspace, &database)?;
    }

    index_lockfile.write(&index.as_vec())
}

fn add_file(
    absolute_path: &PathBuf,
    index: &mut Index,
    workspace: &Workspace,
    database: &Database,
) -> io::Result<()> {
    let file_bytes = file::read_file(&absolute_path)?;
    let blob = Blob::new(file_bytes);
    database.store_object(&blob)?;

    let metadata = fs::metadata(&absolute_path)?;

    let relative_path = workspace.relativize_path(absolute_path);
    let entry = IndexEntry::new(relative_path, blob.id(), &metadata);

    index.add_entry(entry);

    Ok(())
}

fn resolve_files(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_dir() {
        WalkDir::new(&path)
            .into_iter()
            .filter_entry(|entry| !(is_hidden(entry) || is_ignored(entry)))
            .flat_map(|maybe_entry| maybe_entry.map(|entry| PathBuf::from(entry.path())))
            .filter(|path| path.is_file())
            .collect()
    } else {
        vec![path.to_owned()]
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s != "." && s.starts_with("."))
        .unwrap_or(false)
}

fn is_ignored(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| GITIGNORE.contains(&s))
        .unwrap_or(false)
}
