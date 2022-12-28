use std::collections::HashMap;
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use walkdir::DirEntry;

use crate::file;
use crate::hex;
use crate::index::Index;
use crate::output::OutputWriter;
use crate::workspace::{Repository, Worktree};

pub fn status(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let worktree = repository.worktree();
    let unlocked_index = repository.load_index_unlocked()?;

    let tracked_paths = resolve_tracked_paths(&worktree, &unlocked_index);
    let modified_paths = get_modified_paths(&tracked_paths, &worktree, &unlocked_index);
    let untracked_paths = resolve_untracked_paths(&worktree, &unlocked_index);
    let staged_paths = resolve_modified_staged_paths(&repository, &unlocked_index)?;

    print_paths(modified_paths, " M", &worktree, writer)?;
    print_paths(staged_paths, "M ", &worktree, writer)?;
    print_paths(untracked_paths, "??", &worktree, writer)?;

    Ok(())
}

fn print_paths(
    mut paths: Vec<PathBuf>,
    prefix: &str,
    worktree: &Worktree,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    paths.sort_by(|lhs, rhs| lhs.cmp(rhs));
    for path in paths {
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

fn resolve_tracked_paths(worktree: &Worktree, index: &Index) -> Vec<PathBuf> {
    let tracked_paths_filter = |entry: &DirEntry| {
        if entry.path().is_dir() {
            return true;
        }

        let relative_path = worktree.relativize_path(entry.path());
        index.has_entry(&relative_path)
    };

    file::resolve_paths(worktree.root(), tracked_paths_filter)
}

fn resolve_untracked_paths(worktree: &Worktree, index: &Index) -> Vec<PathBuf> {
    let untracked_paths_filter = |entry: &DirEntry| {
        let relative_path = worktree.relativize_path(entry.path());
        let parent = relative_path.parent().unwrap();

        let parent_is_tracked =
            parent.to_str().unwrap() == "" || index.is_tracked_directory(parent);
        let is_path_tracked = || {
            if entry.path().is_dir() {
                index.is_tracked_directory(relative_path)
            } else {
                index.has_entry(relative_path)
            }
        };

        parent_is_tracked && !is_path_tracked()
    };

    file::resolve_paths(worktree.root(), untracked_paths_filter)
}

fn resolve_modified_staged_paths(
    repository: &Repository,
    index: &Index,
) -> io::Result<Vec<PathBuf>> {
    let head_commit_id_opt = repository.head_commit();
    if head_commit_id_opt.is_err() {
        return Ok(vec![]);
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

    let staged_paths_filter = |entry: &DirEntry| {
        if entry.path().is_dir() {
            return true;
        }

        let relative_path = repository.worktree().relativize_path(entry.path());
        let is_staged = index
            .get(&relative_path)
            .map(|index_entry| hex::to_hex_string(&index_entry.object_id))
            .map_or(false, |object_id| {
                path_to_id
                    .get(&relative_path)
                    .map_or(false, |id_in_commit| *id_in_commit != object_id)
            });

        is_staged
    };

    Ok(
        file::resolve_paths(repository.worktree().root(), staged_paths_filter)
            .into_iter()
            .filter(|path| path.is_file())
            .collect(),
    )
}

fn get_modified_paths(
    tracked_paths: &[PathBuf],
    worktree: &Worktree,
    index: &Index,
) -> Vec<PathBuf> {
    tracked_paths
        .into_iter()
        .filter(|path| {
            is_modified(&path, &worktree.relativize_path(&path), &index)
                .ok()
                .unwrap_or(false)
        })
        .map(|path| path.to_owned())
        .collect()
}

fn is_modified(absolute_path: &Path, tracked_path: &Path, index: &Index) -> io::Result<bool> {
    let is_modified = if let Some(index_entry) = index.get(tracked_path) {
        let metadata = fs::metadata(absolute_path)?;
        index_entry.mtime_seconds != metadata.st_mtime() as u32
            || index_entry.mtime_nanoseconds != metadata.st_mtime_nsec() as u32
    } else {
        false
    };
    Ok(is_modified)
}
