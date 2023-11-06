use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str;

use crate::file;
use crate::hex;
use crate::objects::ObjectId;
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
    pub fn deref(&self, reference: &str) -> io::Result<ObjectId> {
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

        ObjectId::from_sha(&result)
            .map_err(|parse_error| Error::new(io::ErrorKind::Other, parse_error))
    }

    pub fn create_ref(&self, ref_name: &str, object_id: &ObjectId) -> io::Result<()> {
        let ref_path = self.get_ref_path(ref_name);
        let hex_string = hex::to_hex_string(object_id.bytes());
        let result = file::create_file(&ref_path, hex_string.as_bytes());

        match result {
            ok @ Ok(_) => ok,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let message = format!("fatal: a branch named '{}' already exists", ref_name);
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            err => err,
        }
    }

    fn get_ref_path(&self, ref_name: &str) -> PathBuf {
        // TODO validate ref_name, this is currently a security hole because ref_name
        // could be e.g. `../../etc/passwd`
        self.repository.git_dir().join("refs/heads/").join(ref_name)
    }

    /**
     * Convenience method to get the current head commit.
     */
    pub fn head(&self) -> io::Result<ObjectId> {
        let head = self.repository.head()?;
        self.deref(&head)
    }
}
