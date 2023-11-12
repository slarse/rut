use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::str;

use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::config;
use crate::config::Config;
use crate::file;
use crate::file::{LockFile, LockFileResource};
use crate::hex;
use crate::index::FileMode;
use crate::index::Index;
use crate::objects::Blob;
use crate::objects::{Author, Commit, GitObject, ObjectId, Tree, TreeEntry};

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
    ) -> io::Result<PathBuf> {
        let object_id = git_object.id();
        let content = git_object.to_object_format();

        let dirpath = self.git_dir.join("objects").join(object_id.dirname());
        fs::create_dir_all(&dirpath)?;

        let compressed_bytes = Database::compress(&content)?;
        let object_filepath = dirpath.join(object_id.filename());
        if !object_filepath.exists() {
            file::atomic_write(&object_filepath, &compressed_bytes)?;
        }

        Ok(object_filepath)
    }

    fn compress(content: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        let compressed_bytes = encoder.finish()?;
        Ok(compressed_bytes)
    }

    /// Find an object by a shortened commit id.
    /// TODO handle ambiguous short commit ids
    pub fn prefix_match(&self, short_commit_id: &str) -> Option<ObjectId> {
        let objects_dir = self.git_dir.join("objects");

        let prefix_dirs = objects_dir
            .read_dir()
            .ok()?
            .map(|entry| entry.ok())
            .flatten();

        for prefix_dir in prefix_dirs {
            let objects = objects_dir
                .join(prefix_dir.file_name())
                .read_dir()
                .ok()?
                .map(|entry| entry.ok())
                .flatten();

            for object in objects {
                let mut oid = prefix_dir.file_name();
                oid.push(object.file_name());
                let oid = oid.to_str()?;
                if oid.starts_with(short_commit_id) {
                    let object_id = ObjectId::from_sha(oid).ok()?;
                    return Some(object_id);
                }
            }
        }

        None
    }

    pub fn load_commit(&self, commit_id: &ObjectId) -> io::Result<Commit> {
        let content = self.load_data(commit_id)?;
        Ok(self.parse_commit(&mut content.into_iter()))
    }

    fn load_data(&self, object_id: &ObjectId) -> io::Result<Vec<u8>> {
        let object_path = self
            .git_dir
            .join("objects")
            .join(object_id.dirname())
            .join(object_id.filename());
        let data = Database::decompress(object_path)?;

        // TODO handle bad/unexpected object type
        let object_type: Vec<u8> = data
            .iter()
            .map(|byte| byte.to_owned())
            .take_while(|byte| byte != &b' ')
            .collect();

        let size_start = object_type.len() + 1;
        let size: Vec<u8> = data[size_start..]
            .iter()
            .map(|byte| byte.to_owned())
            .take_while(|byte| byte != &0)
            .collect();

        let content_start = size_start + size.len() + 1;
        let content = data[content_start..].to_owned();

        Ok(content)
    }

    fn parse_commit(&self, content: &mut impl Iterator<Item = u8>) -> Commit {
        let tree_line = next_line(content);
        let author_or_parent_line = next_line(content);

        let space = b' ';
        let is_not_space = |item: &u8| *item != space;

        let (parent_line, author_line) = {
            let line_start_bytes: Vec<u8> = author_or_parent_line
                .iter()
                .map(|byte| byte.to_owned())
                .take_while(is_not_space)
                .collect();
            let line_start = str::from_utf8(&line_start_bytes).unwrap();
            if line_start == "parent" {
                (Some(author_or_parent_line), next_line(content))
            } else if line_start == "author" {
                (None, author_or_parent_line)
            } else {
                panic!("failed to parse commit");
            }
        };

        let parent = self
            .parse_parent(parent_line.as_ref())
            .map(|parent| ObjectId::from_sha(&parent).unwrap());
        let (author_name, author_email, timestamp) = parse_author_details(&author_line);

        let tree_object_id_bytes: Vec<u8> = tree_line
            .into_iter()
            .skip_while(is_not_space)
            .skip(1)
            .collect();
        let tree_object_id = ObjectId::from_utf8_encoded_sha(&tree_object_id_bytes).unwrap();

        let _committer_line = next_line(content); // TODO handle committer line
        let _empty_line = next_line(content);
        let message_bytes: Vec<u8> = content.collect();

        let message = str::from_utf8(&message_bytes).unwrap().to_owned();

        let author = Author {
            name: author_name,
            email: author_email,
        };

        Commit::new(tree_object_id, author, message, parent, timestamp)
    }

    fn parse_parent(&self, parent_line: Option<&Vec<u8>>) -> Option<String> {
        parent_line.and_then(|parent_line| {
            let parent_oid_bytes: Vec<u8> = parent_line
                .iter()
                .map(|byte| byte.to_owned())
                .skip_while(|byte| *byte != b' ')
                .collect();
            str::from_utf8(&parent_oid_bytes)
                .ok()
                .map(|parent| parent.trim().to_owned())
        })
    }

    pub fn load_tree(&self, tree_id: &ObjectId) -> io::Result<Tree> {
        let content = self.load_data(tree_id)?;
        let tree_entries = parse_tree_entries(&mut content.into_iter());
        Ok(Tree::new(tree_entries))
    }

    pub fn load_blob(&self, blob_id: &ObjectId) -> io::Result<Blob> {
        let content = self.load_data(blob_id)?;
        // TODO fix Blob::with_hash
        Ok(Blob::new(content))
    }

    fn decompress<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut decoder = ZlibDecoder::new(reader);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn print_paths(&self, path: String, tree: &Tree) -> io::Result<()> {
        let mut accumulator = vec![];
        self.extract_paths_from_tree(path, tree, &mut accumulator)?;
        for (object_id, file_path) in accumulator {
            println!("{} {}", object_id, file_path);
        }
        Ok(())
    }

    pub fn extract_paths_from_tree(
        &self,
        base_path: String,
        tree: &Tree,
        accumulator: &mut Vec<(String, String)>,
    ) -> io::Result<()> {
        for tree_entry in tree.entries() {
            let next_path = if base_path.is_empty() {
                String::from(&tree_entry.name)
            } else {
                format!("{}/{}", &base_path, &tree_entry.name)
            };
            match tree_entry.mode {
                FileMode::Directory => {
                    let tree = self.load_tree(&tree_entry.object_id)?;
                    self.extract_paths_from_tree(next_path, &tree, accumulator)?;
                }
                _ => {
                    accumulator.push((tree_entry.object_id.to_string(), next_path));
                }
            }
        }

        Ok(())
    }
}

fn parse_author_details(author_line: &[u8]) -> (String, String, u64) {
    let line_as_str = str::from_utf8(author_line).unwrap();
    let mut chars = line_as_str.chars().skip_while(|chr| chr != &' ');
    let name: String = take_while(&mut chars, |chr| *chr != '<').iter().collect();
    let email: String = take_while(&mut chars, |chr| *chr != '>').iter().collect();

    let is_not_space = |chr: &char| *chr != ' ';
    take_while(&mut chars, is_not_space);
    let timestamp = take_while(&mut chars, is_not_space)
        .iter()
        .collect::<String>()
        .parse::<u64>()
        .unwrap_or(0);
    (name.trim().to_owned(), email.trim().to_owned(), timestamp)
}

fn parse_tree_entries(content: &mut impl Iterator<Item = u8>) -> Vec<TreeEntry> {
    let mut peekable_content = content.peekable();
    let mut entries = vec![];

    while peekable_content.peek().is_some() {
        let entry = parse_tree_entry(&mut peekable_content);
        entries.push(entry);
    }

    entries
}

fn parse_tree_entry(content: &mut impl Iterator<Item = u8>) -> TreeEntry {
    let mode_bytes = take_while(content, |byte: &u8| *byte != b' ');
    let name_bytes = take_while(content, |byte| *byte != 0);
    let raw_object_id = hex::unhexlify(&content.take(20).collect::<Vec<u8>>());
    let object_id = ObjectId::from_sha_bytes(&raw_object_id).unwrap();

    // TODO handle bad mode bytes
    let mode = match str::from_utf8(&mode_bytes).unwrap() {
        "40000" => FileMode::Directory,
        "100644" => FileMode::Regular,
        "100755" => FileMode::Executable,
        unknown_mode => panic!("Unknown mode: {}", unknown_mode),
    };

    // TODO handle bad name bytes
    let name = str::from_utf8(&name_bytes).unwrap().to_owned();

    TreeEntry {
        name,
        object_id,
        mode,
    }
}

fn next_line(iter: &mut impl Iterator<Item = u8>) -> Vec<u8> {
    let is_not_newline = |item: &u8| *item != b'\n';
    take_while(iter, is_not_newline)
}

fn take_while<T>(iter: &mut impl Iterator<Item = T>, predicate: fn(&T) -> bool) -> Vec<T> {
    let mut result = vec![];
    for item in iter {
        if !predicate(&item) {
            break;
        }
        result.push(item);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        index::FileMode,
        objects::{Author, Tree, TreeEntry},
    };
    use rut_testhelpers;

    #[test]
    fn test_load_empty_tree() -> io::Result<()> {
        // arrange
        let workdir = rut_testhelpers::create_temporary_directory();
        let database = Database::new(workdir);

        let empty_tree = Tree::new(vec![]);
        database.store_object(&empty_tree)?;

        // act
        let parsed_tree = database.load_tree(&empty_tree.id())?;

        // assert
        assert_eq!(parsed_tree, empty_tree);
        assert_eq!(parsed_tree.id_as_string(), empty_tree.id_as_string());

        Ok(())
    }

    #[test]
    fn test_load_single_entry_tree() -> io::Result<()> {
        // arrange
        let workdir = rut_testhelpers::create_temporary_directory();
        let database = Database::new(workdir);

        let entry = TreeEntry {
            name: String::from("file.txt"),
            object_id: ObjectId::from_sha("097711d5840f84b87f5567843471e886f5733d9a").unwrap(),
            mode: FileMode::Regular,
        };
        let tree = Tree::new(vec![entry]);
        database.store_object(&tree)?;

        // act
        let parsed_tree = database.load_tree(&tree.id())?;

        // assert
        assert_eq!(parsed_tree, tree);
        assert_eq!(parsed_tree.id_as_string(), tree.id_as_string());

        Ok(())
    }

    #[test]
    fn test_load_multiple_entry_tree() -> io::Result<()> {
        // arrange
        let workdir = rut_testhelpers::create_temporary_directory();
        let database = Database::new(workdir);

        let regular_file_entry = TreeEntry {
            name: String::from("file.txt"),
            object_id: ObjectId::from_sha("097711d5840f84b87f5567843471e886f5733d9a").unwrap(),
            mode: FileMode::Regular,
        };
        let executable_file_entry = TreeEntry {
            name: String::from("other_file.txt"),
            object_id: ObjectId::from_sha("097711d5840f84b87f5567843471e886f5733d9a").unwrap(),
            mode: FileMode::Executable,
        };
        let dir_entry = TreeEntry {
            name: String::from("libs"),
            object_id: ObjectId::from_sha("a2db0a195a522272a018af06515a439bb5ec5ceb").unwrap(),
            mode: FileMode::Directory,
        };

        let tree = Tree::new(vec![regular_file_entry, executable_file_entry, dir_entry]);
        database.store_object(&tree)?;

        // act
        let parsed_tree = database.load_tree(&tree.id())?;

        // assert
        assert_eq!(parsed_tree, tree);
        assert_eq!(parsed_tree.id_as_string(), tree.id_as_string());

        Ok(())
    }

    #[test]
    fn test_parse_without_parent() -> io::Result<()> {
        // arrange
        let workdir = rut_testhelpers::create_temporary_directory();
        let database = Database::new(workdir);

        let commit = create_commit(None);
        database.store_object(&commit)?;

        // act
        let parsed_commit = database.load_commit(&commit.id())?;

        // assert
        assert_eq!(parsed_commit, commit);
        assert_eq!(parsed_commit.id_as_string(), commit.id_as_string());

        Ok(())
    }

    #[test]
    fn test_parse_commit_with_parent() -> io::Result<()> {
        // arrange
        let workdir = rut_testhelpers::create_temporary_directory();
        let database = Database::new(workdir);

        let first_commit = create_commit(None);
        let second_commit = create_commit(Some(first_commit.id().clone()));

        database.store_object(&first_commit)?;
        database.store_object(&second_commit)?;

        // act
        let parsed_commit = database.load_commit(&second_commit.id())?;

        // assert
        assert_eq!(parsed_commit, second_commit);
        assert_eq!(parsed_commit.id_as_string(), second_commit.id_as_string());
        Ok(())
    }

    fn create_commit(parent: Option<ObjectId>) -> Commit {
        let tree_entry = TreeEntry {
            name: String::from("file.txt"),
            object_id: ObjectId::from_sha("ce013625030ba8dba906f756967f9e9ca394464a").unwrap(),
            mode: FileMode::Regular,
        };
        let tree = Tree::new(vec![tree_entry]);
        let author = Author {
            name: String::from("Full Name"),
            email: String::from("name@example.com"),
        };
        Commit::new(
            tree.id().clone(),
            author,
            String::from("Initial commit\n"),
            parent,
            1666811962,
        )
    }
}

pub struct Repository {
    pub database: Database,
    worktree: Worktree,
}

impl Repository {
    pub fn from_worktree_root<P: AsRef<Path>>(worktree_root: P) -> Repository {
        let database = Database::new(worktree_root.as_ref().join(".git"));
        let worktree = Worktree::new(worktree_root.as_ref());
        Repository { database, worktree }
    }

    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    pub fn index_file(&self) -> PathBuf {
        self.git_dir().join("index")
    }

    pub fn load_index(&self) -> crate::Result<LockFileResource<Index>> {
        let index_file_path = self.git_dir().join("index");
        let lockfile = LockFile::acquire(&index_file_path)?;
        let index = Index::from_file(&index_file_path)?;
        Ok(LockFileResource::new(lockfile, index))
    }

    pub fn load_index_unlocked(&self) -> io::Result<Index> {
        let index_file_path = self.git_dir().join("index");
        let index = Index::from_file(index_file_path)?;
        Ok(index)
    }

    pub fn git_dir(&self) -> PathBuf {
        self.worktree.root().join(".git")
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.git_dir().join("objects")
    }

    pub fn config(&self) -> Config {
        config::read_config().unwrap()
    }

    pub fn head(&self) -> io::Result<String> {
        let head_file = self.git_dir().join("HEAD");
        let head_content = fs::read_to_string(head_file)?;
        let trimmed_head_content = head_content.trim();
        Ok(trimmed_head_content
            .trim_start_matches("ref: refs/heads/")
            .to_owned())
    }
}

pub struct Worktree {
    root: PathBuf,
}

impl Worktree {
    pub fn new<P: AsRef<Path>>(root: P) -> Worktree {
        Worktree {
            root: root.as_ref().to_owned(),
        }
    }

    /// Absolute path to the root of the worktree.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the path relative to the root of this worktree.
    pub fn relativize_path<P: AsRef<Path>>(&self, absolute_path: P) -> PathBuf {
        let relative_path = absolute_path
            .as_ref()
            .strip_prefix(&self.root)
            .expect("Bad path");
        if relative_path.as_os_str() == "" {
            PathBuf::from(".")
        } else {
            PathBuf::from(relative_path)
        }
    }
}
