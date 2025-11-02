use std::path::Path;
use std::{fmt::Display, str};

use crate::hashing;
use crate::hex;
use crate::index::FileMode;

pub trait GitObject<'a> {
    fn id(&'a self) -> &'a ObjectId;

    fn id_as_string(&'a self) -> String {
        self.id().to_string()
    }

    fn short_id(&'a self) -> Vec<u8> {
        self.id().bytes()[0..7].to_vec()
    }

    fn short_id_as_string(&'a self) -> String {
        self.id().to_short_string()
    }

    fn to_object_format(&self) -> Vec<u8>;
}

/// A Git object id is the sha1 hash of the object's content, which is represented as a 40 byte
/// hexadecimal string. This struct encapsulates this concept and provides some utility methods
/// related to common operations on object ids, such as finding out the filepath in the object
/// database.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ObjectId {
    bytes: Vec<u8>,
}

impl ObjectId {
    /// Turn a hexadecimal string into an ObjectId. This is the inverse of to_string().
    ///
    /// # Examples
    /// ```
    /// use rut::objects::ObjectId;
    ///
    /// let id = ObjectId::from_sha("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3").unwrap();
    /// assert_eq!(id.to_string(), "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
    /// ```
    pub fn from_sha(s: &str) -> Result<ObjectId, String> {
        let bytes = hex::from_hex_string(s).map_err(|e| e.to_string())?;
        Self::from_sha_bytes(&bytes)
    }

    /// Turn a string that is the utf8 encoded version of a sha1 hash into an ObjectId.
    ///
    /// # Examples
    /// ```
    /// use rut::objects::ObjectId;
    ///
    /// let bytes = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".as_bytes();
    /// let id = ObjectId::from_utf8_encoded_sha(bytes).unwrap();
    /// assert_eq!(id.to_string(), "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3");
    /// ```
    pub fn from_utf8_encoded_sha(bytes: &[u8]) -> Result<ObjectId, String> {
        let s = str::from_utf8(bytes).map_err(|e| e.to_string())?;
        Self::from_sha(s)
    }

    /// Turn bytes into an ObjectId. This is the inverse of bytes().
    ///
    /// # Examples
    /// ```
    /// use rut::objects::ObjectId;
    ///
    /// let bytes = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".as_bytes();
    /// let id = ObjectId::from_sha_bytes(bytes).unwrap();
    /// assert_eq!(id.bytes(), bytes);
    pub fn from_sha_bytes(bytes: &[u8]) -> Result<ObjectId, String> {
        let unhexlified_bytes = if bytes.len() == 20 {
            hex::unhexlify(bytes)
        } else if bytes.len() == 40 {
            bytes.to_vec()
        } else {
            return Err(
                "Object ID must be hexflified (20 bytes long) or in full (40 bytes long)"
                    .to_string(),
            );
        };

        Ok(ObjectId {
            bytes: unhexlified_bytes,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_short_string(&self) -> String {
        hex::to_hex_string(&self.bytes[0..7])
    }

    pub fn dirname(&self) -> String {
        hex::to_hex_string(&self.bytes[0..2])
    }

    pub fn filename(&self) -> String {
        hex::to_hex_string(&self.bytes[2..])
    }
}

impl Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex = hex::to_hex_string(&self.bytes);
        write!(f, "{}", hex)
    }
}

#[derive(Clone)]
pub struct Blob {
    bytes: Vec<u8>,
    id: ObjectId,
}

impl Blob {
    pub fn new(bytes: Vec<u8>) -> Blob {
        let object_format = to_object_format("blob", &bytes);
        let raw_id = &hashing::sha1_hash(&object_format);
        let id = ObjectId::from_sha_bytes(raw_id).unwrap();
        Blob { bytes, id }
    }

    pub fn with_hash(bytes: Vec<u8>, raw_id: &[u8]) -> Blob {
        let id = ObjectId::from_sha_bytes(raw_id).unwrap();
        Blob { bytes, id }
    }

    pub fn content(&self) -> &[u8] {
        &self.bytes
    }
}

impl<'a> GitObject<'a> for Blob {
    fn id(&'a self) -> &'a ObjectId {
        &self.id
    }

    fn to_object_format(&self) -> Vec<u8> {
        to_object_format("blob", &self.bytes)
    }
}

fn to_object_format(object_type: &str, bytes: &[u8]) -> Vec<u8> {
    let mut object_format = object_type.as_bytes().to_vec();
    let byte_count = format!(" {}", bytes.len());

    object_format.extend_from_slice(byte_count.as_bytes());
    object_format.push(0);
    object_format.extend_from_slice(bytes);
    object_format
}

#[derive(Debug, PartialEq, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub object_id: ObjectId,
    pub mode: FileMode,
}

impl TreeEntry {
    pub fn new(path: &Path, object_id: ObjectId, mode: FileMode) -> TreeEntry {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_owned())
            .unwrap();
        TreeEntry {
            name,
            object_id,
            mode,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tree {
    entries: Vec<TreeEntry>,
    id: ObjectId,
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id;
    }

    fn ne(&self, other: &Self) -> bool {
        return self.id != other.id;
    }
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        let mut mutable_entries = entries;
        mutable_entries.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let object_format = Self::to_object_format(&mutable_entries);
        let hash = hashing::sha1_hash(&object_format);
        let id = ObjectId::from_sha_bytes(&hash).unwrap();
        Self {
            entries: mutable_entries,
            id,
        }
    }

    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries[..]
    }

    fn to_object_format(entries: &[TreeEntry]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in entries.iter() {
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
            bytes.extend_from_slice(&hex::hexlify(entry.object_id.bytes()));
        }

        to_object_format("tree", &bytes)
    }
}

impl<'a> GitObject<'a> for Tree {
    fn id(&'a self) -> &'a ObjectId {
        &self.id
    }

    fn to_object_format(&self) -> Vec<u8> {
        Self::to_object_format(&self.entries)
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
    pub tree: ObjectId,
    pub author: Author,
    pub message: String,
    pub parent: Option<ObjectId>,
    pub timestamp: u64,
    pub offset: String,
    id: ObjectId,
}

impl Commit {
    pub fn new(
        tree: ObjectId,
        author: Author,
        message: String,
        parent: Option<ObjectId>,
        timestamp: u64,
        offset: String,
    ) -> Self {
        let object_format = Self::to_object_format(
            &tree,
            &author,
            &message,
            parent.as_ref(),
            timestamp,
            &offset,
        );
        let hash = hashing::sha1_hash(&object_format);
        let id = ObjectId::from_sha_bytes(&hash).unwrap();
        Self {
            tree,
            author,
            message,
            parent,
            timestamp,
            offset,
            id,
        }
    }

    fn to_object_format(
        tree: &ObjectId,
        author: &Author,
        message: &str,
        parent: Option<&ObjectId>,
        timestamp: u64,
        offset: &str,
    ) -> Vec<u8> {
        let author_with_timestamp = format!("{} {} {}", author, timestamp, offset);

        let content = match &parent {
            Some(parent) => {
                format!(
                    "tree {}\nparent {}\nauthor {}\ncommitter {}\n\n{}",
                    tree, parent, author_with_timestamp, author_with_timestamp, message
                )
            }
            None => {
                format!(
                    "tree {}\nauthor {}\ncommitter {}\n\n{}",
                    tree, author_with_timestamp, author_with_timestamp, message
                )
            }
        };

        to_object_format("commit", content.as_bytes())
    }
}

impl<'a> GitObject<'a> for Commit {
    fn id(&self) -> &ObjectId {
        &self.id
    }

    fn to_object_format(&self) -> Vec<u8> {
        Self::to_object_format(
            &self.tree,
            &self.author,
            &self.message,
            self.parent.as_ref(),
            self.timestamp,
            &self.offset,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod blob_tests {
        use super::*;

        #[test]
        fn blob_computes_correct_id() {
            let content = "hello\n";
            let blob = Blob::new(content.as_bytes().to_vec());

            assert_eq!(
                blob.id().to_string(),
                "ce013625030ba8dba906f756967f9e9ca394464a"
            );
        }

        #[test]
        fn blob_computes_correct_object_format() {
            let content = "hello\n";
            let expected_object_format = "blob 6\0hello\n";
            let blob = Blob::new(content.as_bytes().to_vec());

            assert_eq!(blob.to_object_format(), expected_object_format.as_bytes());
        }
    }

    mod objectid_tests {
        use super::*;

        use std::num::ParseIntError;

        #[test]
        fn from_sha_error_on_invalid_length() {
            let hash = "c";
            let result = ObjectId::from_sha(hash);

            assert!(result.is_err());
        }

        #[test]
        fn from_sha_error_on_non_hex_characters() {
            let hash = "xe013625030ba8dba906f756967f9e9ca394464a";
            let result = ObjectId::from_sha(hash);

            assert!(result.is_err());
        }

        #[test]
        fn from_sha_bytes_accepts_hexlified_bytes() -> Result<(), ParseIntError> {
            let hash = "ce013625030ba8dba906f756967f9e9ca394464a";
            let bytes = hex::from_hex_string(hash)?;
            let hexlified_bytes = hex::hexlify(&bytes);
            let result = ObjectId::from_sha_bytes(&hexlified_bytes);

            assert_eq!(result.unwrap().to_string(), hash);

            Ok(())
        }
    }
}
