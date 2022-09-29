use crate::{file::LockFile, workspace::Repository};
use std::io;
use std::path::Path;

use crate::index::Index;

pub fn rm<P: AsRef<Path>>(path: P, repository: &Repository) -> io::Result<()> {
    let index_file_path = repository.git_dir().join("index");
    let mut index_lockfile = LockFile::acquire(&index_file_path)?;
    let mut index = Index::from_file(&index_file_path)?;

    let absolute_path = repository.workspace.workdir().join(path);
    let relative_path = repository.workspace.relativize_path(absolute_path);
    index.remove(&relative_path);

    index_lockfile.write(&index.as_vec())
}
