use std::collections::HashMap;
use std::fs::Metadata;
use std::io;
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;
use std::str;

use crate::file;
use crate::hashing;
use crate::hex;

const SIGNATURE: &str = "DIRC";
const VERSION: [u8; 4] = [0, 0, 0, 2];

const BYTES_PER_U32: usize = 4;
const BYTES_PER_U16: usize = 2;
const BYTES_PER_PACKED_OID: usize = 20;

#[derive(Debug, PartialEq, Eq)]
pub struct Index {
    entries: HashMap<PathBuf, IndexEntry>,
}

fn to_be_u32(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() != 4 {
        return Err(format!("Expected 4 bytes, but got {:?}", bytes));
    }

    let mut result: u32 = 0;
    for (index, byte) in bytes.iter().enumerate() {
        result |= (*byte as u32) << (3 - index) * 8;
    }

    Ok(result)
}

fn to_be_u16(bytes: &[u8]) -> Result<u16, String> {
    if bytes.len() != 2 {
        return Err(format!("Expected 2 bytes, but got {:?}", bytes));
    }

    let mut result: u16 = 0;
    for (index, byte) in bytes.iter().enumerate() {
        result |= (*byte as u16) << (1 - index) * 8;
    }
    Ok(result)
}

impl Index {
    pub fn new() -> Index {
        Index {
            entries: HashMap::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Index, String> {
        let preamble_end = SIGNATURE.len() + VERSION.len();

        let num_entries = to_be_u32(&bytes[preamble_end..(preamble_end + 4)])?;
        let mut entries = HashMap::new();

        let mut position = preamble_end + 4;
        for _ in 0..num_entries {
            let (entry, consumed_bytes) = Index::parse_entry(&bytes[position..])?;
            position += consumed_bytes;
            entries.insert(PathBuf::from(&entry.path), entry);
        }

        Ok(Index { entries })
    }

    pub fn from_file(path: &PathBuf) -> io::Result<Index> {
        let index = if path.is_file() {
            let index_bytes = file::read_file(path)?;

            // TODO handle error from reading index
            Index::from_bytes(&index_bytes).ok().unwrap()
        } else {
            Index::new()
        };

        Ok(index)
    }

    fn parse_entry(bytes: &[u8]) -> Result<(IndexEntry, usize), String> {
        let mut position = 0;

        let ctime_seconds = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let ctime_nanoseconds = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let mtime_seconds = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let mtime_nanoseconds = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let dev = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let ino = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let mode = Mode::new(to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?);
        position += BYTES_PER_U32;
        let uid = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let gid = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let file_size = to_be_u32(&bytes[position..(position + BYTES_PER_U32)])?;
        position += BYTES_PER_U32;
        let object_id = hex::unhexlify(&bytes[position..(position + BYTES_PER_PACKED_OID)]);
        position += BYTES_PER_PACKED_OID;

        let path_size = to_be_u16(&bytes[position..(position + BYTES_PER_U16)])? as usize;
        position += BYTES_PER_U16;

        // TODO fix error handling of parsing path
        let path = std::str::from_utf8(&bytes[position..(position + path_size)])
            .ok()
            .unwrap();

        let entry = IndexEntry {
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
            path: PathBuf::from(path),
            object_id,
        };

        let unpadded_entry_size = position + path_size + 1;
        let entry_padding = if unpadded_entry_size % 8 != 0 {
            8 - unpadded_entry_size % 8
        } else {
            0
        };
        let entry_total_size = unpadded_entry_size + entry_padding;

        Ok((entry, entry_total_size))
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.insert(PathBuf::from(&entry.path), entry);
    }

    pub fn remove(&mut self, path: &PathBuf) -> Option<IndexEntry> {
        self.entries.remove(path)
    }

    pub fn get_entries(&self) -> Vec<&IndexEntry> {
        let mut entries: Vec<&IndexEntry> = self.entries.values().collect();
        entries.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));
        entries
    }

    pub fn as_vec(&self) -> Vec<u8> {
        let signature = SIGNATURE.as_bytes();
        let num_entries = (self.entries.len() as u32).to_be_bytes();

        let mut index: Vec<u8> = Vec::new();
        index.extend_from_slice(signature);
        index.extend_from_slice(&VERSION);
        index.extend_from_slice(&num_entries);

        let entries = self.get_entries();

        for entry in entries {
            index.extend(entry.as_vec());
        }

        let index_checksum = hashing::sha1_hash(&index);
        index.extend_from_slice(&index_checksum);

        index
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexEntry {
    pub ctime_seconds: u32,
    pub ctime_nanoseconds: u32,
    pub mtime_seconds: u32,
    pub mtime_nanoseconds: u32,
    pub dev: u32,
    pub ino: u32,
    mode: Mode,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub path: PathBuf,
    pub object_id: Vec<u8>,
}

impl IndexEntry {
    pub fn new(path: PathBuf, object_id: Vec<u8>, metadata: &Metadata) -> IndexEntry {
        let ctime_seconds = metadata.st_ctime() as u32;
        let ctime_nanoseconds = metadata.st_ctime_nsec() as u32;
        let mtime_seconds = metadata.st_mtime() as u32;
        let mtime_nanoseconds = metadata.st_mtime_nsec() as u32;
        let dev = metadata.st_dev() as u32;
        let ino = metadata.st_ino() as u32;
        let mode = Mode::new(metadata.st_mode());
        let uid = metadata.st_uid() as u32;
        let gid = metadata.st_gid() as u32;
        let file_size = metadata.st_size() as u32;

        IndexEntry {
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
        }
    }

    pub fn as_vec(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        add_all(self.ctime_seconds, &mut bytes);
        add_all(self.ctime_nanoseconds, &mut bytes);
        add_all(self.mtime_seconds, &mut bytes);
        add_all(self.mtime_nanoseconds, &mut bytes);
        add_all(self.dev, &mut bytes);
        add_all(self.ino, &mut bytes);
        add_all(self.mode.raw_mode, &mut bytes);
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

    pub fn file_mode(&self) -> FileMode {
        self.mode.file_mode
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum FileMode {
    Directory,
    Executable,
    Regular,
}

#[derive(Eq, PartialEq, Debug)]
struct Mode {
    file_mode: FileMode,
    raw_mode: u32,
}

impl Mode {
    fn new(actual_mode: u32) -> Mode {
        let world_executable_bits = 0o700 as u32;
        if actual_mode & world_executable_bits == world_executable_bits {
            Mode {
                file_mode: FileMode::Executable,
                raw_mode: 0o100755,
            }
        } else {
            Mode {
                file_mode: FileMode::Regular,
                raw_mode: 0o100644,
            }
        }
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
    fn test_to_be_u32() {
        let expected: u32 = 99999;
        let bytes = expected.to_be_bytes();

        let actual = to_be_u32(&bytes).ok().unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_to_be_u32_error_on_bad_byte_count() {
        let bytes: [u8; 3] = [0, 1, 2];

        let error = to_be_u32(&bytes).err().unwrap();

        assert_eq!(error, "Expected 4 bytes, but got [0, 1, 2]");
    }

    #[test]
    fn test_single_entry_index_round_trip() {
        let entry = create_entry("Cargo.toml");

        let mut index = Index::new();
        index.add_entry(entry);
        let index_bytes = index.as_vec();

        let index_from_bytes = Index::from_bytes(&index_bytes).ok().unwrap();

        assert_eq!(index_from_bytes, index);
    }

    #[test]
    fn test_dual_entry_index_round_trip() {
        let first_entry = create_entry("Cargo.toml");
        let second_entry = create_entry("README.md");

        let mut index = Index::new();
        index.add_entry(first_entry);
        index.add_entry(second_entry);

        let index_bytes = index.as_vec();
        let index_from_bytes = Index::from_bytes(&index_bytes).ok().unwrap();

        assert_eq!(index_from_bytes, index);
    }

    #[test]
    fn test_as_vec() {
        let object_id = vec![1, 2];
        let hexlified_object_id = hex::hexlify(&object_id).get(0).unwrap().to_owned();
        let entry = IndexEntry {
            ctime_seconds: 1657658046,
            ctime_nanoseconds: 444900053,
            mtime_seconds: 1657658046,
            mtime_nanoseconds: 444900053,
            dev: 65026,
            ino: 3831260,
            mode: Mode::new(33188),
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
            hexlified_object_id,
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

    fn create_entry(path: &str) -> IndexEntry {
        let object_id: Vec<u8> = (0..10).cycle().take(40).collect();
        IndexEntry {
            ctime_seconds: 1657658046,
            ctime_nanoseconds: 444900053,
            mtime_seconds: 1657658046,
            mtime_nanoseconds: 444900053,
            dev: 65026,
            ino: 3831260,
            mode: Mode::new(33188),
            uid: 1000,
            gid: 985,
            file_size: 262,
            path: PathBuf::from(path),
            object_id,
        }
    }

    fn assert_vectors_equal<T: Debug + Eq>(actual: &Vec<T>, expected: &Vec<T>) {
        if actual.len() != expected.len() {
            panic!(
                "expected vector has length {}, but actual vector has length {}",
                expected.len(),
                actual.len()
            );
        }

        for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
            if actual != expected {
                panic!(
                    "mismatching characters at index {}, expected={:?}, actual={:?}",
                    index, expected, actual
                );
            }
        }
    }
}