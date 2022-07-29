use std::{fs::File, io, io::Read, path::PathBuf};

pub fn read_file(path: &PathBuf) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
