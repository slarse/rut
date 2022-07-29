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
    println!(
        "[(root-commit) {}] {}",
        to_hex_string(&commit.id()),
        first_line
    );

    fs::write(
        workspace.git_dir().join("HEAD"),
        to_hex_string(&commit.id()),
    )?;

    Ok(())
}
