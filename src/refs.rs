use std::fs;
use std::io;
use std::io::Error;
use std::str;

use crate::workspace::Repository;

pub struct RefHandler<'a> {
    repository: &'a Repository,
}

const SHA1_SIZE: usize = 40;

impl<'a> RefHandler<'a> {
    pub fn new(repository: &Repository) -> RefHandler {
        RefHandler { repository }
    }

    /**
     * Dereference a Git ref.
     */
    pub fn deref(&self, reference: &str) -> io::Result<String> {
        if reference == "HEAD" {
            return self.head();
        }

        let trimmed_reference = reference.trim().trim_start_matches("refs/heads/");
        let ref_file = self
            .repository
            .git_dir()
            .join("refs/heads/")
            .join(trimmed_reference);

        let result = if reference.len() == SHA1_SIZE {
            reference.to_owned()
        } else if ref_file.is_file() {
            fs::read_to_string(&ref_file).map(|content| content.trim().to_owned())?
        } else {
            let message = format!("Could not dereference ref {}", reference);
            return Err(Error::new(io::ErrorKind::Other, message));
        };

        Ok(result)
    }

    /**
     * Convenience method to get the current head commit.
     */
    pub fn head(&self) -> io::Result<String> {
        let head = self.repository.head()?;
        self.deref(&head)
    }
}
