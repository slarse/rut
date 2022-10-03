use crate::workspace::Repository;
use std::io;
use std::path::Path;

pub fn rm<P: AsRef<Path>>(path: P, repository: &Repository) -> io::Result<()> {
    let mut index = repository.load_index()?;
    let worktree = repository.worktree();

    let absolute_path = worktree.root().join(path);
    let relative_path = worktree.relativize_path(absolute_path);
    index.as_mut().remove(&relative_path);

    index.write()
}
