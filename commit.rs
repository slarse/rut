use std::{fs, io};

use crate::file;
use crate::hex::to_hex_string;
use crate::index::Index;
use crate::objects::{Author, Commit, GitObject, Tree, TreeEntry};
use crate::workspace::{Database, Workspace};

pub fn commit(workspace: &Workspace, database: &Database) -> io::Result<()> {
    let index_bytes = file::read_file(&workspace.git_dir().join("index"))?;

    // TODO handle index parse error
    let index = Index::from_bytes(&index_bytes).expect("Could not parse index");

    let tree_entries = index
        .get_entries()
        .iter()
        .map(|entry| TreeEntry::new(&entry.path, entry.object_id[..].to_vec()))
        .collect();

    let root_tree = Tree::new(tree_entries);
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
        to_hex_string(&commit.id()),
        first_line
    );

    fs::write(
        workspace.git_dir().join("HEAD"),
        to_hex_string(&commit.id()),
    )?;

    Ok(())
}
