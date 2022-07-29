use std::{fs, fs::File, io, io::Read, path::PathBuf};

use crate::{
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
};

pub fn add(path: PathBuf) -> io::Result<()> {
    let file_bytes = read_file(&path)?;
    let blob = Blob::new(file_bytes);
    let entry = IndexEntry::new(path, blob.id())?;

    let index_bytes = read_file(&PathBuf::from(".git/index"))?;

    // TODO handle error from reading index
    let mut index = Index::from_bytes(&index_bytes).ok().unwrap();

    index.add_entry(entry);

    fs::write(".git/index", index.as_vec())
}

fn read_file(path: &PathBuf) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
