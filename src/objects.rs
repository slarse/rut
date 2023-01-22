use std::{fmt::Display, path::PathBuf, str};

use crate::hashing;
use crate::hex;
use crate::index::FileMode;

pub trait GitObject<'a> {
    fn id(&'a self) -> Vec<u8>;

    fn id_as_string(&'a self) -> String {
        hex::to_hex_string(&self.id())
    }

    fn short_id(&'a self) -> Vec<u8> {
        self.id()[0..7].to_vec()
    }

    fn short_id_as_string(&'a self) -> String {
        to_short_id(&self.id())
    }

    fn to_object_format(&self) -> Vec<u8>;
}

pub fn to_short_id(id: &[u8]) -> String {
    hex::to_hex_string(&id[0..7])
}

pub struct Blob {
    bytes: Vec<u8>,
    id: Vec<u8>,
}

impl Blob {
    pub fn new(bytes: Vec<u8>) -> Blob {
        let object_format = to_object_format("blob", &bytes);
        let id = hashing::sha1_hash(&object_format);

        Blob {
            bytes,
            id: id.to_vec(),
        }
    }

    pub fn with_hash(bytes: Vec<u8>, id: &[u8]) -> Blob {
        Blob {
            bytes,
            id: Vec::from(id),
        }
    }

    pub fn content(&self) -> &[u8] {
        &self.bytes
    }
}

impl<'a> GitObject<'a> for Blob {
    fn id(&'a self) -> Vec<u8> {
        hex::unhexlify(&self.id[..])
    }

    fn to_object_format(&self) -> Vec<u8> {
        to_object_format("blob", &self.bytes)
    }
}

fn to_object_format(object_type: &str, bytes: &[u8]) -> Vec<u8> {
    let mut object_format: Vec<u8> = Vec::from(object_type.as_bytes().to_vec());
    let byte_count = format!(" {}", bytes.len());

    object_format.extend_from_slice(byte_count.as_bytes());
    object_format.push(0);
    object_format.extend_from_slice(bytes);
    object_format
}

#[derive(Debug, PartialEq)]
pub struct TreeEntry {
    pub name: String,
    pub object_id: Vec<u8>,
    pub mode: FileMode,
}

impl TreeEntry {
    pub fn new(path: &PathBuf, object_id: Vec<u8>, mode: FileMode) -> TreeEntry {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Some(name.to_owned()))
            .unwrap();
        TreeEntry {
            name,
            object_id,
            mode,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>) -> Tree {
        let mut mutable_entries = entries;
        mutable_entries.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        Tree {
            entries: mutable_entries,
        }
    }

    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries[..]
    }
}

impl<'a> GitObject<'a> for Tree {
    fn id(&'a self) -> Vec<u8> {
        let object_format = self.to_object_format();
        let hash = hashing::sha1_hash(&object_format);
        hex::unhexlify(&hash)
    }

    fn to_object_format(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in self.entries.iter() {
            let name_bytes = entry.name.as_bytes();

            let mode = match entry.mode {
                FileMode::Directory => "40000",
                FileMode::Regular => "100644",
                FileMode::Executable => "100755",
            };

            bytes.extend_from_slice(mode.as_bytes());
            bytes.extend_from_slice(" ".as_bytes());
            bytes.extend_from_slice(name_bytes);
            bytes.push(0);
            bytes.extend_from_slice(&hex::hexlify(&entry.object_id));
        }

        to_object_format("tree", &bytes)
    }
}

#[derive(Debug, PartialEq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = format!("{} <{}>", self.name, self.email);
        write!(f, "{}", result)
    }
}

#[derive(Debug, PartialEq)]
pub struct Commit {
    pub tree: String,
    pub author: Author,
    pub message: String,
    pub parent: Option<String>,
    pub timestamp: u64,
}

impl<'a> GitObject<'a> for Commit {
    fn id(&self) -> Vec<u8> {
        let object_format = self.to_object_format();
        let hash = hashing::sha1_hash(&object_format);
        hex::unhexlify(&hash)
    }

    fn to_object_format(&self) -> Vec<u8> {
        // TODO get timezone from system
        let timezone = "+0200";
        let author_with_timestamp = format!("{} {} {}", self.author, self.timestamp, timezone);

        let content = match &self.parent {
            Some(parent) => {
                format!(
                    "tree {}\nparent {}\nauthor {}\ncommitter {}\n\n{}",
                    self.tree, parent, author_with_timestamp, author_with_timestamp, self.message
                )
            }
            None => {
                format!(
                    "tree {}\nauthor {}\ncommitter {}\n\n{}",
                    self.tree, author_with_timestamp, author_with_timestamp, self.message
                )
            }
        };

        to_object_format("commit", &content.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;

    #[test]
    fn blob_computes_correct_id() {
        let content = "hello\n";
        let blob = Blob::new(content.as_bytes().to_vec());

        let blob_hex = hex::to_hex_string(&blob.id());

        assert_eq!(blob_hex, "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn blob_computes_correct_object_format() {
        let content = "hello\n";
        let expected_object_format = "blob 6\0hello\n";
        let blob = Blob::new(content.as_bytes().to_vec());

        assert_eq!(blob.to_object_format(), expected_object_format.as_bytes());
    }
}
