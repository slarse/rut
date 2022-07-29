use std::{io, fs};
use std::path::PathBuf;
use crate::file;

use crate::{index::Index, workspace::Workspace};

pub fn rm(path: &PathBuf, workspace: &Workspace) -> io::Result<()> {
    let index_file = workspace.git_dir().join("index");
    let mut index = if index_file.is_file() {
        let index_bytes = file::read_file(&index_file)?;

        // TODO handle error from reading index
        Index::from_bytes(&index_bytes).ok().unwrap()
    } else {
        Index::new()
    };

    index.remove(path);

    fs::write(&index_file, index.as_vec())
}
