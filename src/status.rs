use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use walkdir::DirEntry;

use crate::file;
use crate::index::Index;
use crate::output::OutputWriter;
use crate::workspace::{Repository, Worktree};

pub fn status(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let worktree = repository.worktree();
    let unlocked_index = repository.load_index_unlocked()?;

    let tracked_paths = resolve_tracked_paths(&worktree, &unlocked_index);
    let modified_paths = get_modified_paths(&tracked_paths, &worktree, &unlocked_index);
    let untracked_paths = resolve_untracked_paths(&worktree, &unlocked_index);

    print_paths(modified_paths, " M", &worktree, writer)?;
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
        let parent_is_tracked = parent.to_str().unwrap() == "" || index.has_entry(parent);
        parent_is_tracked && !index.has_entry(relative_path)
    };

    file::resolve_paths(worktree.root(), untracked_paths_filter)
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
