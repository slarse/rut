use std::{fmt::Display, path::PathBuf, str};

use crate::hex::{self, unhexlify};
use sha1::{Digest, Sha1};

pub trait GitObject<'a> {
    fn id(&'a self) -> Vec<u8>;

    fn to_object_format(&self) -> Vec<u8>;
}

pub struct Blob {
    bytes: Vec<u8>,
    id: Vec<u8>,
}

impl Blob {
    pub fn new(bytes: Vec<u8>) -> Blob {
        let object_format = to_object_format("blob", &bytes);
        let id = hash(&object_format);

        Blob {
            bytes,
            id: id.to_vec(),
        }
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

pub struct TreeEntry {
    pub name: String,
    pub object_id: Vec<u8>,
}

impl TreeEntry {
    pub fn new(path: &PathBuf, object_id: Vec<u8>) -> TreeEntry {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Some(name.to_owned()))
            .unwrap();
        TreeEntry { name, object_id }
    }
}

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
}

impl<'a> GitObject<'a> for Tree {
    fn id(&'a self) -> Vec<u8> {
        let object_format = self.to_object_format();
        let hash = hash(&object_format);
        unhexlify(&hash)
    }

    fn to_object_format(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mode = "100644 ".as_bytes();

        for entry in self.entries.iter() {
            let name_bytes = entry.name.as_bytes();

            bytes.extend_from_slice(mode);
            bytes.extend_from_slice(name_bytes);
            bytes.push(0);
            bytes.extend_from_slice(&hex::hexlify(&entry.object_id));
        }

        to_object_format("tree", &bytes)
    }
}

fn hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

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

pub struct Commit<'a> {
    pub tree: &'a Tree,
    pub author: &'a Author,
    pub message: &'a str,
    pub parent: Option<&'a str>,
}

impl<'a> GitObject<'a> for Commit<'a> {
    fn id(&self) -> Vec<u8> {
        let object_format = self.to_object_format();
        let hash = hash(&object_format);
        unhexlify(&hash)
    }

    fn to_object_format(&self) -> Vec<u8> {
        let tree_string = hex::to_hex_string(&self.tree.id());

        let content = match self.parent {
            Some(parent) => {
                format!(
                    "tree {}\nparent {}\nauthor {}\ncommitter {}\n\n{}",
                    &tree_string, parent, self.author, self.author, self.message
                )
            }
            None => {
                format!(
                    "tree {}\nauthor {}\ncommitter {}\n\n{}",
                    &tree_string, self.author, self.author, self.message
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
