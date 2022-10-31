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
use crate::index::Index;
use crate::objects::{Author, Commit, GitObject};

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
        let mut content = git_object.to_object_format();

        let dirname = hex::to_hex_string(&object_id[..2]);
        let filename = hex::to_hex_string(&object_id[2..]);
        let dirpath = self.git_dir.join("objects").join(dirname);
        fs::create_dir_all(&dirpath)?;

        let compressed_bytes = Database::compress(&mut content)?;
        let object_filepath = dirpath.join(&filename);
        if !object_filepath.exists() {
            file::atomic_write(&object_filepath, &compressed_bytes)?;
        }

        Ok(object_filepath)
    }

    fn compress(content: &mut Vec<u8>) -> io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        let compressed_bytes = encoder.finish()?;
        Ok(compressed_bytes)
    }

    pub fn load_commit(&self, commit_id: &[u8]) -> io::Result<Commit> {
        let dirname = hex::to_hex_string(&commit_id[..2]);
        let filename = hex::to_hex_string(&commit_id[2..]);
        let object_path = self.git_dir.join("objects").join(dirname).join(filename);
        let data = Database::decompress(object_path)?;

        let space = ' ' as u8;
        let object_type: Vec<u8> = data
            .iter()
            .map(|byte| byte.to_owned())
            .take_while(|byte| byte != &space)
            .collect();

        let size_start = object_type.len() + 1;
        let size: Vec<u8> = data[size_start..]
            .iter()
            .map(|byte| byte.to_owned())
            .take_while(|byte| byte != &0)
            .collect();

        let content_start = size_start + size.len() + 1;
        let mut content = data[content_start..].to_owned().into_iter();

        Ok(self.parse_commit(&mut content))
    }

    fn parse_commit(&self, content: &mut impl Iterator<Item = u8>) -> Commit {
        let tree_line = next_line(content);
        let author_or_parent_line = next_line(content);

        let space = ' ' as u8;
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

        let parent = self.parse_parent(parent_line.as_ref());
        let (author_name, author_email, timestamp) = parse_author_details(&author_line);

        let tree_object_id_bytes: Vec<u8> =
            tree_line.into_iter().skip_while(is_not_space).collect();
        let tree_object_id = str::from_utf8(&tree_object_id_bytes)
            .unwrap()
            .trim()
            .to_owned();

        let _committer_line = next_line(content); // TODO handle committer line
        let _empty_line = next_line(content);
        let message_bytes: Vec<u8> = content.collect();

        let message = str::from_utf8(&message_bytes).unwrap().to_owned();

        let author = Author {
            name: author_name,
            email: author_email,
        };

        Commit {
            tree: tree_object_id,
            author,
            message,
            parent,
            timestamp,
        }
    }

    fn parse_parent(&self, parent_line: Option<&Vec<u8>>) -> Option<String> {
        parent_line
            .map(|parent_line| {
                let parent_oid_bytes: Vec<u8> = parent_line
                    .iter()
                    .map(|byte| byte.to_owned())
                    .skip_while(|byte| *byte != (' ' as u8))
                    .collect();
                str::from_utf8(&parent_oid_bytes)
                    .ok()
                    .map(|parent| parent.trim().to_owned())
            })
            .flatten()
    }

    fn decompress<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut decoder = ZlibDecoder::new(reader);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
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

fn next_line(iter: &mut impl Iterator<Item = u8>) -> Vec<u8> {
    let is_not_newline = |item: &u8| *item != ('\n' as u8);
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
        let first_commit_oid = first_commit.id_as_string();
        let second_commit = create_commit(Some(first_commit_oid));

        database.store_object(&first_commit)?;
        database.store_object(&second_commit)?;

        // act
        let parsed_commit = database.load_commit(&second_commit.id())?;

        // assert
        assert_eq!(parsed_commit, second_commit);
        assert_eq!(parsed_commit.id_as_string(), second_commit.id_as_string());
        Ok(())
    }

    fn create_commit(parent: Option<String>) -> Commit {
        let tree_entry = TreeEntry {
            name: String::from("file.txt"),
            object_id: "ce013625030ba8dba906f756967f9e9ca394464a"
                .as_bytes()
                .to_owned(),
            mode: FileMode::Regular,
        };
        let tree = Tree::new(vec![tree_entry]).id_as_string();
        let author = Author {
            name: String::from("Full Name"),
            email: String::from("name@example.com"),
        };
        Commit {
            tree,
            author,
            message: String::from("Initial commit\n"),
            parent,
            timestamp: 1666811962,
        }
    }
}

pub struct Repository {
    pub database: Database,
    worktree: Worktree,
}

impl Repository {
    pub fn from_worktree_root<P: AsRef<Path>>(worktree_root: P) -> Repository {
        let database = Database::new(worktree_root.as_ref().join(".git"));
        let worktree = Worktree::new(worktree_root.as_ref().to_owned());
        Repository { database, worktree }
    }

    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    pub fn index_file(&self) -> PathBuf {
        self.git_dir().join("index")
    }

    pub fn load_index(&self) -> io::Result<LockFileResource<Index>> {
        let index_file_path = self.git_dir().join("index");
        let lockfile = LockFile::acquire(&index_file_path)?;
        let index = Index::from_file(&index_file_path)?;
        Ok(LockFileResource::new(lockfile, index))
    }

    pub fn load_index_unlocked(&self) -> io::Result<Index> {
        let index_file_path = self.git_dir().join("index");
        let index = Index::from_file(&index_file_path)?;
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

    /**
     * Absolute path to the root of the worktree.
     */
    pub fn root(&self) -> &Path {
        &self.root
    }

    /**
     * Return the path relative to the root of this worktree.
     */
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
