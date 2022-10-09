use std::io;

use walkdir::DirEntry;

use crate::file;
use crate::output::OutputWriter;
use crate::workspace::Repository;

pub fn status(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let worktree = repository.worktree();
    let unlocked_index = repository.load_index_unlocked()?;

    let directory_entry_filter = |entry: &DirEntry| {
        let relative_path = worktree.relativize_path(entry.path());
        let parent = relative_path.parent().unwrap();
        let parent_is_tracked = parent.to_str().unwrap() == "" || unlocked_index.has_entry(parent);
        parent_is_tracked && !unlocked_index.has_entry(relative_path)
    };

    let mut paths = file::resolve_paths(repository.worktree().root(), directory_entry_filter);
    paths.sort_by(|lhs, rhs| lhs.cmp(rhs));

    for path in paths {
        let relative_path = worktree.relativize_path(&path);
        let suffix = if path.is_dir() { "/" } else { "" };
        let line = format!("?? {}{}", relative_path.as_os_str().to_str().unwrap(), suffix);
        writer.write(line)?;
    }

    Ok(())
}
