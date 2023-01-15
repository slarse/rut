use std::collections::{HashMap, HashSet};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use walkdir::DirEntry;

use crate::file;
use crate::hex;
use crate::index::Index;
use crate::objects::{Blob, GitObject};
use crate::output::{Color, OutputWriter};
use crate::refs::RefHandler;
use crate::workspace::{Repository, Worktree};

#[derive(Default, Builder, Debug)]
pub struct Options {
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Porcelain,
    HumanReadable,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::HumanReadable
    }
}

pub fn status(
    repository: &Repository,
    options: &Options,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let worktree = repository.worktree();
    let mut index_lockfile = repository.load_index()?;
    let index = index_lockfile.as_mut();
    let path_to_committed_id = resolve_committed_paths_and_ids(&repository)?;

    let tracked_paths = resolve_tracked_paths(&path_to_committed_id, &worktree, index);
    let untracked_paths = resolve_untracked(&tracked_paths, &worktree, index);

    let mut unstaged_changes = resolve_unstaged_changes(&tracked_paths, &repository, index);
    let mut staged_changes = resolve_staged_changes(&path_to_committed_id, &repository, index)?;

    match options.output_format {
        OutputFormat::HumanReadable => write_human_readable(
            &mut staged_changes,
            &mut unstaged_changes,
            &untracked_paths,
            &worktree,
            writer,
        )?,
        OutputFormat::Porcelain => {
            let mut all_changes = vec![]
                .into_iter()
                .chain(unstaged_changes)
                .chain(staged_changes)
                .collect::<Vec<_>>();

            write_porcelain(&mut all_changes, &untracked_paths, &worktree, writer)?
        }
    }

    index_lockfile.write()
}

struct Change {
    path: PathBuf,
    change_type: ChangeType,
    changed_in: ChangePlace,
}

impl Change {
    fn porcelain_format(&self) -> String {
        let character = self.change_type.to_char();
        let modification_shorthand = match self.changed_in {
            ChangePlace::Index => format!("{} ", character),
            ChangePlace::Worktree => format!(" {}", character),
        };
        format!("{} {}", modification_shorthand, self.path.display())
    }

    fn human_readable_format(&self) -> String {
        let modification_longform = match self.changed_in {
            ChangePlace::Index => match self.change_type {
                ChangeType::Modified => "modified",
                ChangeType::Deleted => "deleted",
                ChangeType::Created => "new file",
            },
            ChangePlace::Worktree => match self.change_type {
                ChangeType::Modified => "modified",
                ChangeType::Deleted => "deleted",
                ChangeType::Created => panic!("This should not happen"),
            },
        };
        format!("{}: {}", modification_longform, self.path.display())
    }
}

enum ChangePlace {
    Worktree,
    Index,
}

enum ChangeType {
    Modified,
    Deleted,
    Created,
}

impl ChangeType {
    fn to_char(&self) -> char {
        match self {
            ChangeType::Modified => 'M',
            ChangeType::Deleted => 'D',
            ChangeType::Created => 'A',
        }
    }
}

fn write_human_readable(
    staged_changes: &mut Vec<Change>,
    unstaged_changes: &mut Vec<Change>,
    untracked_paths: &[PathBuf],
    worktree: &Worktree,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    staged_changes.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));
    unstaged_changes.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));

    let mut written = false;
    if !staged_changes.is_empty() {
        writer.write("Changes to be committed:".to_string())?;

        writer.set_color(Color::Green)?;
        for change in staged_changes {
            writer.write(format!("\t{}", change.human_readable_format()))?;
        }
        writer.reset_formatting()?;

        written = true;
    }

    if !unstaged_changes.is_empty() {
        if written {
            writer.write("".to_string())?;
        }

        writer.write("Changes not staged for commit:".to_string())?;
        writer.set_color(Color::Red)?;
        for change in unstaged_changes {
            writer.write(format!("\t{}", change.human_readable_format()))?;
        }
        writer.reset_formatting()?;

        written = true;
    }

    if !untracked_paths.is_empty() {
        if written {
            writer.write("".to_string())?;
        }

        writer.write("Untracked files:".to_string())?;
        writer.set_color(Color::Red)?;
        print_paths("\t", untracked_paths, worktree, writer)?;
        writer.reset_formatting()?;
    }

    writer.write("".to_string())?;
    Ok(())
}

fn write_porcelain(
    changes: &mut Vec<Change>,
    untracked_paths: &[PathBuf],
    worktree: &Worktree,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    changes.sort_by(|lhs, rhs| lhs.path.cmp(&rhs.path));
    for change in changes {
        writer.write(change.porcelain_format())?;
    }
    print_paths("?? ", &untracked_paths, &worktree, writer)?;
    Ok(())
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
            "{}{}{}",
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

fn resolve_untracked(
    tracked_paths: &[PathBuf],
    worktree: &Worktree,
    index: &Index,
) -> Vec<PathBuf> {
    let tracked_path_set = tracked_paths
        .iter()
        .map(|path| path.as_path())
        .collect::<HashSet<_>>();

    let untracked_directories = file::resolve_paths(worktree.root(), |entry| {
        if !entry.path().is_dir() {
            return false;
        }

        let relative_path = worktree.relativize_path(entry.path());
        let parent = relative_path.parent().unwrap();
        let parent_is_tracked =
            parent.to_str().unwrap() == "" || index.is_tracked_directory(parent);

        parent_is_tracked && !index.is_tracked_directory(relative_path)
    });

    let untracked_files = file::resolve_paths(worktree.root(), |entry| {
        if entry.path().is_dir() {
            return true;
        }

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
    }).into_iter().filter(|path| !path.is_dir());

    let mut untracked_paths = untracked_directories
        .into_iter()
        .chain(untracked_files)
        .collect::<Vec<_>>();
    untracked_paths.sort();

    untracked_paths
}

fn resolve_staged_changes(
    path_to_committed_id: &HashMap<PathBuf, String>,
    repository: &Repository,
    index: &mut Index,
) -> io::Result<Vec<Change>> {
    let mut staged_changes = resolve_staged_modifications(path_to_committed_id, repository, index)?;
    staged_changes.extend(resolve_staged_deletions(
        path_to_committed_id,
        repository.worktree(),
        index,
    ));
    Ok(staged_changes)
}

fn resolve_staged_modifications(
    path_to_committed_id: &HashMap<PathBuf, String>,
    repository: &Repository,
    index: &Index,
) -> io::Result<Vec<Change>> {
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

    classify_staged_changes(&staged_paths, path_to_committed_id, repository, index)
}

fn classify_staged_changes(
    staged_paths: &[PathBuf],
    path_to_committed_id: &HashMap<PathBuf, String>,
    repository: &Repository,
    index: &Index,
) -> io::Result<Vec<Change>> {
    let mut changes = vec![];

    for path in staged_paths {
        let relative_path = repository.worktree().relativize_path(path);
        match path_to_committed_id.get(&relative_path) {
            Some(committed_object_id) => {
                let indexed_object_id =
                    hex::to_hex_string(&index.get(&relative_path).unwrap().object_id);
                if *committed_object_id != indexed_object_id {
                    changes.push(Change {
                        path: relative_path.to_owned(),
                        change_type: ChangeType::Modified,
                        changed_in: ChangePlace::Index,
                    });
                }
            }
            None => changes.push(Change {
                path: relative_path.to_owned(),
                change_type: ChangeType::Created,
                changed_in: ChangePlace::Index,
            }),
        }
    }

    Ok(changes)
}

fn resolve_staged_deletions(
    path_to_committed_id: &HashMap<PathBuf, String>,
    worktree: &Worktree,
    index: &Index,
) -> Vec<Change> {
    path_to_committed_id
        .keys()
        .cloned()
        .filter(|path| !index.has_entry(path))
        .map(|path| worktree.root().join(path))
        .map(|path| Change {
            path: worktree.relativize_path(&path),
            change_type: ChangeType::Deleted,
            changed_in: ChangePlace::Index,
        })
        .collect()
}

fn resolve_unstaged_deletions<'a>(
    tracked_paths: &'a [PathBuf],
    worktree: &'a Worktree,
) -> impl Iterator<Item = Change> + 'a {
    tracked_paths
        .iter()
        .filter(|path| !path.exists())
        .map(|path| Change {
            path: worktree.relativize_path(&path),
            change_type: ChangeType::Deleted,
            changed_in: ChangePlace::Worktree,
        })
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

fn resolve_unstaged_changes(
    tracked_paths: &[PathBuf],
    repository: &Repository,
    index: &mut Index,
) -> Vec<Change> {
    resolve_unstaged_modifications(tracked_paths, repository, index)
        .chain(resolve_unstaged_deletions(
            tracked_paths,
            repository.worktree(),
        ))
        .collect()
}

fn resolve_unstaged_modifications<'a>(
    tracked_paths: &'a [PathBuf],
    repository: &'a Repository,
    index: &'a mut Index,
) -> impl Iterator<Item = Change> + 'a {
    let worktree = repository.worktree();
    tracked_paths
        .into_iter()
        .filter(|path| {
            is_modified(&path, &worktree.relativize_path(&path), index)
                .ok()
                .unwrap_or(false)
        })
        .map(|path| Change {
            path: repository.worktree().relativize_path(&path),
            change_type: ChangeType::Modified,
            changed_in: ChangePlace::Worktree,
        })
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
