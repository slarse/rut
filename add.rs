use std::{fs, io, path::PathBuf};

use crate::index::IndexEntry;

pub fn add(path: PathBuf) -> io::Result<()> {
    let entry = IndexEntry::new(path)?;
    fs::write("test.binary", entry.as_vec())
}
