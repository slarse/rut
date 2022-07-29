use std::iter::Peekable;
use std::path::Component;
use std::{fs, io, path::PathBuf};

use crate::file;
use crate::hex::to_hex_string;
use crate::index::{FileMode, Index, IndexEntry};
use crate::objects::{Author, Commit, GitObject, Tree, TreeEntry};
use crate::workspace::{Database, Workspace};

pub fn commit(workspace: &Workspace, database: &Database) -> io::Result<()> {
    let index_bytes = file::read_file(&workspace.git_dir().join("index"))?;

    // TODO handle index parse error
    let index = Index::from_bytes(&index_bytes).expect("Could not parse index");

    let (root_tree, containing_trees) = build_tree(&index.get_entries()[..]);
    for tree in containing_trees.iter() {
        database.store_object(tree)?;
    }
    database.store_object(&root_tree)?;

    let config = workspace.get_config();
    let author = Author {
        name: config.author_name,
        email: config.author_email,
    };
    let commit_msg = fs::read_to_string(workspace.git_dir().join("COMMIT_EDITMSG"))
        .expect("failed to read commit message");

    let parent_commit = fs::read_to_string(workspace.git_dir().join("HEAD")).ok();

    let commit = Commit {
        tree: &root_tree,
        author: &author,
        message: &commit_msg,
        parent: parent_commit.as_deref(),
    };

    database.store_object(&commit)?;

    let first_line = commit_msg
        .split("\n")
        .next()
        .expect("Not a single line in the commit message");

    let root_commit_notice = if parent_commit.is_some() {
        ""
    } else {
        "(root commit) "
    };

    println!(
        "[{}{}] {}",
        root_commit_notice,
        to_hex_string(&commit.short_id()),
        first_line
    );

    fs::write(
        workspace.git_dir().join("HEAD"),
        to_hex_string(&commit.id()),
    )?;

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
