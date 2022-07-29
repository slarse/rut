use std::fs;
use std::io::Result;
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;

use crate::hashing;
use crate::hex;

pub struct Index {
    entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new() -> Index {
        Index { entries: vec![] }
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.push(entry)
    }

    pub fn as_vec(&self) -> Vec<u8> {
        let signature = "DIRC".as_bytes();
        let version: [u8; 4] = [0, 0, 0, 2];
        let num_entries = (self.entries.len() as u32).to_be_bytes();

        let mut index: Vec<u8> = Vec::new();
        index.extend_from_slice(signature);
        index.extend_from_slice(&version);
        index.extend_from_slice(&num_entries);

        for entry in &self.entries {
            index.extend(entry.as_vec());
        }

        let index_checksum = hashing::sha1_hash(&index);
        index.extend_from_slice(&index_checksum);

        index
    }
}

pub struct IndexEntry {
    ctime_seconds: u32,
    ctime_nanoseconds: u32,
    mtime_seconds: u32,
    mtime_nanoseconds: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    file_size: u32,
    path: PathBuf,
    object_id: Vec<u8>,
}

impl IndexEntry {
    pub fn new(path: PathBuf, object_id: Vec<u8>) -> Result<IndexEntry> {
        let metadata = fs::metadata(&path)?;

        let ctime_seconds = metadata.st_ctime() as u32;
        let ctime_nanoseconds = metadata.st_ctime_nsec() as u32;
        let mtime_seconds = metadata.st_mtime() as u32;
        let mtime_nanoseconds = metadata.st_mtime_nsec() as u32;
        let dev = metadata.st_dev() as u32;
        let ino = metadata.st_ino() as u32;
        let mode = metadata.st_mode() as u32;
        let uid = metadata.st_uid() as u32;
        let gid = metadata.st_gid() as u32;
        let file_size = metadata.st_size() as u32;

        Ok(IndexEntry {
            ctime_seconds,
            ctime_nanoseconds,
            mtime_seconds,
            mtime_nanoseconds,
            dev,
            ino,
            mode,
            uid,
            gid,
            file_size,
            path,
            object_id,
        })
    }

    pub fn as_vec(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        add_all(self.ctime_seconds, &mut bytes);
        add_all(self.ctime_nanoseconds, &mut bytes);
        add_all(self.mtime_seconds, &mut bytes);
        add_all(self.mtime_nanoseconds, &mut bytes);
        add_all(self.dev, &mut bytes);
        add_all(self.ino, &mut bytes);
        add_all(self.mode, &mut bytes);
        add_all(self.uid, &mut bytes);
        add_all(self.gid, &mut bytes);
        add_all(self.file_size, &mut bytes);
        hex::hexlify(&self.object_id)
            .into_iter()
            .for_each(|byte| bytes.push(byte));

        let path_bytes = self.path.to_str().unwrap().as_bytes().to_vec();
        let path_length = (path_bytes.len() as u16).to_be_bytes().to_vec();
        path_length.into_iter().for_each(|byte| bytes.push(byte));
        path_bytes.into_iter().for_each(|byte| bytes.push(byte));
        bytes.push(0);

        pad_to_block_size(&mut bytes);

        bytes
    }
}

fn pad_to_block_size(bytes: &mut Vec<u8>) {
    let block_size = 8;
    while bytes.len() % block_size != 0 {
        bytes.push(0);
    }
}

fn add_all(value: u32, bytes: &mut Vec<u8>) {
    value
        .to_be_bytes()
        .into_iter()
        .for_each(|byte| bytes.push(byte));
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    #[test]
    fn test_as_vec() {
        let object_id_byte: u8 = 123;
        let object_id: Vec<u8> = vec![object_id_byte];
        let entry = IndexEntry {
            ctime_seconds: 1657658046,
            ctime_nanoseconds: 444900053,
            mtime_seconds: 1657658046,
            mtime_nanoseconds: 444900053,
            dev: 65026,
            ino: 3831260,
            mode: 33188,
            uid: 1000,
            gid: 985,
            file_size: 262,
            path: PathBuf::from("Cargo.toml"),
            object_id,
        };

        let expected_vec: Vec<u8> = vec![
            98,
            205,
            218,
            190,
            26,
            132,
            162,
            213,
            98,
            205,
            218,
            190,
            26,
            132,
            162,
            213,
            0,
            0,
            254,
            2,
            0,
            58,
            117,
            220,
            0,
            0,
            129,
            164,
            0,
            0,
            3,
            232,
            0,
            0,
            3,
            217,
            0,
            0,
            1,
            6,
            object_id_byte,
            0,
            10,
            67,
            97,
            114,
            103,
            111,
            46,
            116,
            111,
            109,
            108,
            0,
            0,
            0,
        ];

        assert_vectors_equal(&entry.as_vec(), &expected_vec);
    }

    fn assert_vectors_equal<T: Debug + Eq>(actual: &Vec<T>, expected: &Vec<T>) {
        if actual.len() != expected.len() {
            panic!(
                "expected vector has length {}, but actual vector has length {}",
                expected.len(),
                actual.len()
            );
        }

        for (actual, expected) in actual.iter().zip(expected.iter()) {
            if actual != expected {
                panic!(
                    "mismatching characters, expected={:?}, actual={:?}",
                    expected, actual
                );
            }
        }
    }
}
