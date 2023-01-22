use std::iter::Peekable;
use std::path::Component;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io, path::PathBuf};

use crate::hex::to_hex_string;
use crate::index::{FileMode, Index, IndexEntry};
use crate::objects::{Author, Commit, GitObject, Tree, TreeEntry};
use crate::output::OutputWriter;
use crate::refs::RefHandler;
use crate::workspace::Repository;

pub fn commit(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let mut index = repository.load_index()?;

    let head_ref = repository.head().expect("HEAD does not exist");
    let commit = create_commit(&repository, index.as_mut(), &head_ref)?;
    repository.database.store_object(&commit)?;

    fs::write(repository.git_dir().join(&head_ref), commit.id_as_string())?;

    write_commit_status(&commit, writer)?;

    Ok(())
}

pub fn create_commit<'a>(
    repository: &'a Repository,
    index: &'a mut Index,
    head_ref: &'a str,
) -> io::Result<Commit> {
    let (root_tree, containing_trees) = build_tree(&index.get_entries()[..]);
    for tree in containing_trees.iter() {
        repository.database.store_object(tree)?;
    }
    repository.database.store_object(&root_tree)?;

    let ref_handler = RefHandler::new(&repository);
    let parent_commit = ref_handler.deref(&head_ref).ok();
    Ok(create_commit_with_tree(
        root_tree.id_as_string(),
        parent_commit,
        &repository,
    ))
}

fn create_commit_with_tree<'a>(
    tree: String,
    parent: Option<String>,
    repository: &'a Repository,
) -> Commit {
    let config = repository.config();
    let author = Author {
        name: config.author_name,
        email: config.author_email,
    };
    let message = fs::read_to_string(repository.git_dir().join("COMMIT_EDITMSG"))
        .expect("failed to read commit message");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Commit {
        tree,
        author,
        message,
        parent,
        timestamp,
    }
}

fn write_commit_status(commit: &Commit, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let first_line = commit
        .message
        .split("\n")
        .next()
        .expect("Not a single line in the commit message");

    let root_commit_notice = commit.parent.to_owned().map_or("(root commit) ", |_| "");

    let message = format!(
        "[{}{}] {}",
        root_commit_notice,
        to_hex_string(&commit.short_id()),
        first_line,
    );
    writer.writeln(message)?;
    Ok(())
}

fn build_tree(entries: &[&IndexEntry]) -> (Tree, Vec<Tree>) {
    let tmp_entries = entries.iter().map(|entry| TmpEntry {
        path: PathBuf::from(&entry.path),
        object_id: entry.object_id[..].to_vec(),
        file_mode: entry.file_mode(),
    });

    build_tree_from_tmp_entries(tmp_entries)
}

#[derive(Debug)]
struct TmpEntry {
    path: PathBuf,
    object_id: Vec<u8>,
    file_mode: FileMode,
}

fn build_tree_from_tmp_entries(entries: impl Iterator<Item = TmpEntry>) -> (Tree, Vec<Tree>) {
    let mut entry_iter = entries.peekable();
    let mut tree_entries = Vec::new();

    let mut trees: Vec<Tree> = Vec::new();

    while let Some(entry) = entry_iter.next() {
        let tree_entry = if entry.path.parent() == Some(&PathBuf::from("")) {
            TreeEntry::new(&entry.path, entry.object_id, entry.file_mode)
        } else {
            let prefix = entry.path.components().next().unwrap();
            let mut entries_for_tree = vec![TmpEntry {
                path: PathBuf::from(&entry.path),
                object_id: entry.object_id,
                file_mode: entry.file_mode,
            }];

            while let Some(entry) = next_if_prefixed_with(&prefix, &mut entry_iter) {
                entries_for_tree.push(entry);
            }

            let tmp_entries: Vec<TmpEntry> = entries_for_tree
                .iter()
                .map(|entry| {
                    let mut path = PathBuf::new();
                    let object_id = entry.object_id[..].to_vec();
                    entry
                        .path
                        .components()
                        .skip(1)
                        .map(|c| c.as_os_str())
                        .for_each(|path_part| path.push(path_part));
                    let file_mode = entry.file_mode;
                    TmpEntry {
                        path,
                        object_id,
                        file_mode,
                    }
                })
                .collect();
            let (root_tree, containing_trees) =
                build_tree_from_tmp_entries(tmp_entries.into_iter());
            let tree_entry = TreeEntry::new(
                &PathBuf::from(prefix.as_os_str()),
                root_tree.id(),
                FileMode::Directory,
            );

            trees.push(root_tree);
            trees.extend(containing_trees);

            tree_entry
        };

        tree_entries.push(tree_entry);
    }

    (Tree::new(tree_entries), trees)
}

fn next_if_prefixed_with(
    prefix: &Component,
    entries: &mut Peekable<impl Iterator<Item = TmpEntry>>,
) -> Option<TmpEntry> {
    if let Some(entry_peek) = entries.peek() {
        let peek_prefix = entry_peek.path.components().next().unwrap();
        return if peek_prefix == *prefix {
            Some(entries.next().unwrap())
        } else {
            None
        };
    }

    None
}
