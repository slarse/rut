use std::{io, path::Path};

use crate::{file, object_resolver::ObjectResolver, workspace::Repository};

#[derive(Default, Builder, Debug)]
pub struct Options {
    #[builder(default = "String::from(\"HEAD\")")]
    pub source: String,
}

/// Restores a file in the working directory to its state in the latest commit.
///
/// Given a file path and a reference to the repository, this function will retrieve the
/// file's content from the latest commit and overwrite the current file in the working
/// directory with the retrieved content.
///
/// This is useful for discarding local changes made to a file that have not been staged.
///
/// # Arguments
///
/// * `file`: A reference to the `Path` of the file to be restored.
/// * `repository`: A reference to the `Repository` containing the file.
///
/// # Returns
///
/// * `io::Result<()>`: A result indicating success or failure. In case of success, the
///   working directory file is overwritten with the content from the latest commit.
pub fn restore_worktree<P: AsRef<Path>>(file: P, options: &Options, repository: &Repository) -> io::Result<()> {
    let mut object_cache = ObjectResolver::from_reference(&options.source, repository)?;

    let absolute_path = repository.worktree().root().join(file.as_ref());
    let relative_path = repository.worktree().relativize_path(&absolute_path);
    let blob = object_cache.find_blob_by_path(&relative_path)?;

    let content = blob.content().to_vec();
    file::atomic_write(&absolute_path, &content)?;

    Ok(())
}
