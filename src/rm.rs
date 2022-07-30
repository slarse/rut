use std::path::PathBuf;
use std::{fs, io};

use crate::{index::Index, workspace::Workspace};

pub fn rm(path: &PathBuf, workspace: &Workspace) -> io::Result<()> {
    let index_file = workspace.git_dir().join("index");
    let mut index = Index::from_file(&index_file)?;

    let absolute_path = workspace.workdir().join(&path);
    let relative_path = workspace.relativize_path(&absolute_path);
    index.remove(&relative_path);

    fs::write(&index_file, index.as_vec())
}
