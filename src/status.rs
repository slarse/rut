use std::collections::{HashMap, HashSet};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use walkdir::DirEntry;

use crate::file;
use crate::hex;
use crate::index::Index;
use crate::objects::{Blob, GitObject};
use crate::output::OutputWriter;
use crate::refs::RefHandler;
use crate::workspace::{Repository, Worktree};

pub fn status(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let worktree = repository.worktree();
    let mut index_lockfile = repository.load_index()?;
    let index = index_lockfile.as_mut();
    let path_to_committed_id = resolve_committed_paths_and_ids(&repository)?;

    let tracked_paths = resolve_tracked_paths(&path_to_committed_id, &worktree, index);
    let modified_unstaged_paths =
        resolve_modified_unstaged_paths(&tracked_paths, &repository, index);
    let deleted_unstaged_paths = resolve_deleted_unstaged_paths(&tracked_paths);
    let untracked_paths = resolve_untracked_paths(&tracked_paths, &worktree, index);
    let (modified_staged_paths, created_staged_paths) =
        resolve_modified_and_created_staged_paths(&path_to_committed_id, &repository, index)?;
    let deleted_staged_paths =
        resolve_deleted_staged_paths(&path_to_committed_id, &worktree.root(), index);

    print_paths(" M", &modified_unstaged_paths, &worktree, writer)?;
    print_paths("M ", &modified_staged_paths, &worktree, writer)?;
    print_paths(" D", &deleted_unstaged_paths, &worktree, writer)?;
    print_paths("D ", &deleted_staged_paths, &worktree, writer)?;
    print_paths("A ", &created_staged_paths, &worktree, writer)?;
    print_paths("??", &untracked_paths, &worktree, writer)?;

    index_lockfile.write()
}

fn print_paths(
    prefix: &str,
    paths: &[PathBuf],
    worktree: &Worktree,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let mut sorted_paths = paths.iter().collect::<Vec<&PathBuf>>();
    sorted_paths.sort();
    for path in sorted_paths {
        let relative_path = worktree.relativize_path(&path);
        let suffix = if path.is_dir() { "/" } else { "" };
        let line = format!(
            "{} {}{}",
            prefix,
            relative_path.as_os_str().to_str().unwrap(),
            suffix
        );
        writer.write(line)?;
    }
    Ok(())
}

fn resolve_tracked_paths(
    path_to_committed_id: &HashMap<PathBuf, String>,
    worktree: &Worktree,
    index: &Index,
) -> Vec<PathBuf> {
    let root = worktree.root();

    let mut paths = index
        .get_entries()
        .iter()
        .map(|entry| root.join(&entry.path))
        .collect::<HashSet<PathBuf>>();
    let paths_in_last_commit = path_to_committed_id.keys().map(|path| root.join(path));

    paths.extend(paths_in_last_commit);
    paths.into_iter().collect()
}

fn resolve_untracked_paths(
    tracked_paths: &[PathBuf],
    worktree: &Worktree,
    index: &Index,
) -> Vec<PathBuf> {
    let tracked_path_set = tracked_paths
        .iter()
        .map(|path| path.as_path())
        .collect::<HashSet<_>>();

    let untracked_paths_filter = |entry: &DirEntry| {
        let relative_path = worktree.relativize_path(entry.path());
        let parent = relative_path.parent().unwrap();

        let parent_is_tracked =
            parent.to_str().unwrap() == "" || index.is_tracked_directory(parent);
        let is_path_tracked = || {
            if entry.path().is_dir() {
                index.is_tracked_directory(relative_path)
            } else {
                tracked_path_set.contains(entry.path())
            }
        };

        parent_is_tracked && !is_path_tracked()
    };

    file::resolve_paths(worktree.root(), untracked_paths_filter)
}

fn resolve_modified_and_created_staged_paths(
    path_to_committed_id: &HashMap<PathBuf, String>,
    repository: &Repository,
    index: &Index,
) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let staged_paths_filter = |entry: &DirEntry| {
        if entry.path().is_dir() {
            return true;
        }

        let relative_path = repository.worktree().relativize_path(entry.path());
        let is_staged = index.has_entry(relative_path);

        is_staged
    };

    let staged_paths: Vec<PathBuf> =
        file::resolve_paths(repository.worktree().root(), staged_paths_filter)
            .into_iter()
            .filter(|path| path.is_file())
            .collect();

    split_staged_paths_into_modified_and_created(
        &staged_paths,
        path_to_committed_id,
        repository,
        index,
    )
}

fn resolve_deleted_staged_paths(
    path_to_committed_id: &HashMap<PathBuf, String>,
    worktree_root: &Path,
    index: &Index,
) -> Vec<PathBuf> {
    path_to_committed_id
        .keys()
        .cloned()
        .filter(|path| !index.has_entry(path))
        .map(|path| worktree_root.join(path))
        .collect()
}

fn resolve_deleted_unstaged_paths(tracked_paths: &[PathBuf]) -> Vec<PathBuf> {
    tracked_paths
        .iter()
        .filter(|path| !path.exists())
        .map(|path| path.to_owned())
        .collect()
}

fn resolve_committed_paths_and_ids(
    repository: &Repository,
) -> io::Result<HashMap<PathBuf, String>> {
    let head_commit_id_opt = RefHandler::new(&repository).head();
    if head_commit_id_opt.is_err() {
        return Ok(HashMap::new());
    }
    let head_commit_id = head_commit_id_opt.ok().unwrap();

    let commit = repository
        .database
        .load_commit(&hex::from_hex_string(&head_commit_id).unwrap())?;
    let tree = repository
        .database
        .load_tree(&hex::from_hex_string(&commit.tree).unwrap())?;

    let mut paths_in_head = vec![];
    repository
        .database
        .extract_paths_from_tree(String::from(""), &tree, &mut paths_in_head)?;
    let path_to_id: HashMap<PathBuf, String> = paths_in_head
        .into_iter()
        .map(|(id, path)| (PathBuf::from(path), id))
        .collect();

    Ok(path_to_id)
}

fn split_staged_paths_into_modified_and_created(
    staged_paths: &[PathBuf],
    path_to_committed_id: &HashMap<PathBuf, String>,
    repository: &Repository,
    index: &Index,
) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut modified_paths = vec![];
    let mut created_paths = vec![];

    for path in staged_paths {
        let relative_path = repository.worktree().relativize_path(path);
        match path_to_committed_id.get(&relative_path) {
            Some(committed_object_id) => {
                let indexed_object_id =
                    hex::to_hex_string(&index.get(&relative_path).unwrap().object_id);
                if *committed_object_id != indexed_object_id {
                    modified_paths.push(path.to_owned())
                }
            }
            None => created_paths.push(path.to_owned()),
        }
    }

    Ok((modified_paths, created_paths))
}

fn resolve_modified_unstaged_paths(
    tracked_paths: &[PathBuf],
    repository: &Repository,
    index: &mut Index,
) -> Vec<PathBuf> {
    let worktree = repository.worktree();
    tracked_paths
        .into_iter()
        .filter(|path| {
            is_modified(&path, &worktree.relativize_path(&path), index)
                .ok()
                .unwrap_or(false)
        })
        .map(|path| path.to_owned())
        .collect()
}

/**
 * Returns true if the file at the given path has been modified since the last commit.
 *
 * Side effect: Updates the index with new mtimes if they've been updatet without the content being
 * changed.
 */
fn is_modified(absolute_path: &Path, tracked_path: &Path, index: &mut Index) -> io::Result<bool> {
    let is_modified = if let Some(index_entry) = index.get_mut(tracked_path) {
        let metadata = fs::metadata(absolute_path)?;
        let mtimes_differ = index_entry.mtime_seconds != metadata.st_mtime() as u32
            || index_entry.mtime_nanoseconds != metadata.st_mtime_nsec() as u32;

        if mtimes_differ {
            let current_object_id = hash_as_blob(absolute_path)?;
            if current_object_id != index_entry.object_id {
                true
            } else {
                index_entry.mtime_seconds = metadata.st_mtime() as u32;
                index_entry.mtime_nanoseconds = metadata.st_mtime_nsec() as u32;
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(is_modified)
}

fn hash_as_blob(absolute_path: &Path) -> io::Result<Vec<u8>> {
    let content = file::read_file(absolute_path)?;
    let blob = Blob::new(content);
    Ok(blob.id())
}
