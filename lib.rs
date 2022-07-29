pub mod cli {
    use crate::{
        commit, init,
        workspace::{Database, Workspace},
    };
    use std::env;
    use std::io;

    pub fn run_command(args: Vec<String>) -> io::Result<()> {
        let sliced_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
        let workdir = env::current_dir()?;
        let git_dir = workdir.join(".git");

        let workspace = Workspace::new(workdir);
        let database = Database::new(workspace.git_dir());

        match sliced_args[..] {
            ["init"] => {
                init::init(&git_dir)?;
            }
            ["commit"] => {
                commit::commit(&workspace, &database)?;
            }
            _ => panic!("unexpected command {:?}", sliced_args),
        };

        Ok(())
    }
}

pub mod workspace {
    use std::fs;
    use std::io;
    use std::io::prelude::*;
    use std::path::PathBuf;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use crate::config;
    use crate::config::Config;
    use crate::hex;
    use crate::objects::GitObject;

    static GITIGNORE: &'static [&str] = &["Cargo.lock"];

    pub struct Workspace {
        workdir: PathBuf,
    }

    impl Workspace {
        pub fn new(workdir: PathBuf) -> Workspace {
            Workspace { workdir }
        }

        pub fn workdir(&self) -> &PathBuf {
            &self.workdir
        }

        pub fn git_dir(&self) -> PathBuf {
            self.workdir.join(".git")
        }

        pub fn objects_dir(&self) -> PathBuf {
            self.git_dir().join("objects")
        }

        pub fn get_config(&self) -> Config {
            config::read_config().unwrap()
        }

        pub fn list_files(&self) -> io::Result<impl Iterator<Item = PathBuf>> {
            let file_paths = fs::read_dir(self.workdir())?
                .map(|res| res.map(|e| e.path()))
                .flatten()
                .filter(|path| path.is_file())
                .filter(|path| {
                    for ignored_file in GITIGNORE.iter() {
                        if path.ends_with(ignored_file) {
                            return false;
                        }
                    }
                    true
                });

            Ok(file_paths)
        }
    }

    pub struct Database {
        git_dir: PathBuf,
    }

    impl Database {
        pub fn new(git_dir: PathBuf) -> Database {
            Database { git_dir }
        }

        pub fn store_object<'a>(
            &self,
            git_object: &'a (impl GitObject<'a> + 'a),
        ) -> io::Result<()> {
            let object_id = git_object.id();
            let mut content = git_object.to_object_format();

            let dirname = hex::to_hex_string(&object_id[..2]);
            let filename = hex::to_hex_string(&object_id[2..]);
            let dirpath = self.git_dir.join("objects").join(dirname);
            fs::create_dir(&dirpath)?;

            let compressed_bytes = Database::compress(&mut content)?;
            fs::write(dirpath.join(filename), &compressed_bytes)?;

            Ok(())
        }

        fn compress(content: &mut Vec<u8>) -> io::Result<Vec<u8>> {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(content)?;
            let compressed_bytes = encoder.finish()?;
            Ok(compressed_bytes)
        }
    }
}

pub mod config {
    use std::env;

    pub struct Config {
        pub author_name: String,
        pub author_email: String,
    }

    pub fn read_config() -> Result<Config, env::VarError> {
        Ok(Config {
            author_name: env::var("GIT_AUTHOR_NAME")?,
            author_email: env::var("GIT_AUTHOR_EMAIL")?,
        })
    }
}

pub mod init {
    use std::{fs, io, path::PathBuf};

    pub fn init(git_dir: &PathBuf) -> io::Result<()> {
        for subdir in ["objects", "refs"] {
            fs::create_dir_all(git_dir.join(subdir))?;
        }
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main")?;
        println!("Initialized empty Rut repository in {:#?}", git_dir);
        Ok(())
    }
}

pub mod commit {
    use std::{fs, fs::File, io, io::Read};

    use crate::hex::to_hex_string;
    use crate::objects::{Author, Blob, Commit, GitObject, Tree, TreeEntry};
    use crate::workspace::{Database, Workspace};

    pub fn commit(workspace: &Workspace, database: &Database) -> io::Result<()> {
        let mut blobs = Vec::new();
        let mut tree_entries = Vec::new();

        let file_paths = workspace.list_files()?;
        for path in file_paths {
            let mut file = File::open(&path)?;
            let mut bytes: Vec<u8> = Vec::new();
            file.read_to_end(&mut bytes)?;

            let blob = Blob::new(bytes);
            let tree_entry = TreeEntry::new(&path, blob.id());

            blobs.push(blob);
            tree_entries.push(tree_entry);
        }

        for blob in blobs {
            database.store_object(&blob)?;
        }

        let root_tree = Tree::new(tree_entries);
        database.store_object(&root_tree)?;

        let config = workspace.get_config();
        let author = Author {
            name: config.author_name,
            email: config.author_email,
        };
        let commit_msg = fs::read_to_string(workspace.git_dir().join("COMMIT_EDITMSG"))
            .expect("failed to read commit message");

        let commit = Commit {
            tree: &root_tree,
            author: &author,
            message: &commit_msg,
        };

        database.store_object(&commit)?;
        let first_line = commit_msg.split("\n").next().expect("Not a single line in the commit message");
        println!("[(root-commit) {}] {}", to_hex_string(&commit.id()), first_line);

        fs::write(workspace.git_dir().join("HEAD"), to_hex_string(&commit.id()))?;

        Ok(())
    }
}

pub mod objects {
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
    }

    impl<'a> GitObject<'a> for Commit<'a> {
        fn id(&self) -> Vec<u8> {
            let object_format = self.to_object_format();
            let hash = hash(&object_format);
            unhexlify(&hash)
        }

        fn to_object_format(&self) -> Vec<u8> {
            let tree_string = hex::to_hex_string(&self.tree.id());
            let content = format!(
                "tree {}\nauthor {}\ncommitter {}\n\n{}",
                &tree_string, self.author, self.author, self.message
            );
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
}

pub mod hex {
    pub fn to_hex_string(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|byte| format!("{:x}", byte))
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn unhexlify(bytes: &[u8]) -> Vec<u8> {
        let mut unhexlified = Vec::new();
        for i in 0..bytes.len() {
            let compressed_bytes = bytes.get(i).unwrap();
            let left_byte = compressed_bytes >> 4;
            let right_byte = compressed_bytes & 0b00001111;
            unhexlified.push(left_byte);
            unhexlified.push(right_byte);
        }
        unhexlified
    }

    pub fn hexlify(bytes: &[u8]) -> Vec<u8> {
        let mut hexlified = Vec::new();
        for i in (0..bytes.len() - 1).step_by(2) {
            let left_byte = bytes.get(i).unwrap();
            let right_byte = bytes.get(i + 1).unwrap();
            let compressed = (left_byte << 4) | right_byte;
            hexlified.push(compressed);
        }

        hexlified
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fmt::Debug;

        #[test]
        fn hexlify_and_unhexlify_roundtrip_works() {
            let bytes = vec![0, 1, 2, 3, 4, 6];

            let hexlified = hexlify(&bytes);
            let unhexlified = unhexlify(&hexlified);

            assert_vectors_equal(&unhexlified.to_vec(), &bytes)
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
}
