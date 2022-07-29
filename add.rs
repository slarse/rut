use std::{fs, fs::File, io, io::Read, path::PathBuf};

use crate::{
    index::{Index, IndexEntry},
    objects::{Blob, GitObject},
};

pub fn add(path: PathBuf) -> io::Result<()> {
    let mut file = File::open(&path)?;
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes)?;

    let blob = Blob::new(bytes);
    let entry = IndexEntry::new(path, blob.id())?;

    let mut index = Index::new();
    index.add_entry(entry);

    fs::write(".git/index", index.as_vec())
}
