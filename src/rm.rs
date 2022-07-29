use std::path::PathBuf;
use std::{fs, io};

use crate::{index::Index, workspace::Workspace};

pub fn rm(path: &PathBuf, workspace: &Workspace) -> io::Result<()> {
    let index_file = workspace.git_dir().join("index");
    let mut index = Index::from_file(&index_file)?;

    index.remove(path);

    fs::write(&index_file, index.as_vec())
}
