use std::{fs, io, path::Path};

use crate::{
    file,
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
    workspace::Repository,
};

pub static GITIGNORE: [&str; 2] = ["Cargo.lock", "target"];

pub fn add<P: AsRef<Path>>(path: P, repository: &Repository) -> crate::Result<()> {
    if GITIGNORE.contains(&path.as_ref().to_str().expect("Path was bad UTF8")) {
        return Ok(());
    }

    let absolute_path = repository.worktree().root().join(&path);
    let mut index = repository.load_index()?;

    if absolute_path.try_exists().unwrap_or(false) {
        for path in file::resolve_files(&absolute_path) {
            add_file(&path, index.as_mut(), repository)?;
        }
    } else {
        let relative_path = repository.worktree().relativize_path(&absolute_path);
        match index.as_mut().remove(&relative_path) {
            Some(_) => (),
            None => {
                let message = format!("pathspec {:?} did not match any files", path.as_ref());
                return Err(crate::Error::Fatal(None, message));
            }
        }
    }

    Ok(index.write()?)
}

fn add_file(absolute_path: &Path, index: &mut Index, repository: &Repository) -> crate::Result<()> {
    let file_bytes = file::read_file(absolute_path)?;
    let blob = Blob::new(file_bytes);
    repository.database.store_object(&blob)?;

    let metadata = fs::metadata(absolute_path)?;

    let relative_path = repository.worktree().relativize_path(absolute_path);
    let entry = IndexEntry::new(relative_path, blob.id().clone(), &metadata);

    index.add_entry(entry);

    Ok(())
}
