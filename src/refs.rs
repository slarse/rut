use std::fmt;
use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use regex::Regex;

use crate::file;
use crate::hex;
use crate::objects::ObjectId;
use crate::workspace::Repository;

pub struct RefHandler<'a> {
    repository: &'a Repository,
}

const SHA1_SIZE: usize = 40;

const INVALID_BRANCH_NAME_PATTERN: &str =
    r"^\.|\/\.|\.\.|^\/|\/$|\.lock$|@\{|[\x00-\x20*:?\[\\^~\x7F]";

const PARENT_PATTERN: &str = r"^(.*)\^$";
const ANCESTOR_PATTERN: &str = r"^(.*)~(\d+)$";

impl<'a> RefHandler<'a> {
    pub fn new(repository: &Repository) -> RefHandler {
        RefHandler { repository }
    }

    /// Dereference a reference to an object id.
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

        ObjectId::from_sha(&result).map_err(|err| Error::new(io::ErrorKind::Other, err.to_string()))
    }

    pub fn write_ref(&self, ref_name: &str, object_id: &ObjectId) -> crate::Result<()> {
        let ref_path = self.get_ref_path(ref_name)?;
        let hex_string = hex::to_hex_string(object_id.bytes());
        Ok(file::atomic_write(&ref_path, hex_string.as_bytes())?)
    }

    pub fn create_ref(&self, ref_name: &str, object_id: &ObjectId) -> crate::Result<()> {
        let ref_path = self.get_ref_path(ref_name)?;
        let hex_string = hex::to_hex_string(object_id.bytes());
        let result = file::create_file(&ref_path, hex_string.as_bytes());

        match result {
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let message = format!("a branch named '{}' already exists", ref_name);
                return Err(crate::Error::Fatal(Some(Box::new(error)), message));
            }
            other => return Ok(other?),
        }
    }

    fn get_ref_path(&self, ref_name: &str) -> crate::Result<PathBuf> {
        let re = Regex::new(INVALID_BRANCH_NAME_PATTERN).unwrap();
        if re.is_match(ref_name) {
            let message = format!("'{}' is not a valid branch name", ref_name);
            return Err(crate::Error::Fatal(None, message));
        }
        Ok(self.repository.git_dir().join("refs/heads/").join(ref_name))
    }

    /// Convenience method to get the object id of the current HEAD.
    pub fn head(&self) -> io::Result<ObjectId> {
        let head = self.repository.head()?;
        self.deref(&head)
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseRevisionError {
    InvalidFormat(String),
    // You can add more specific error types as needed
}

impl From<ParseRevisionError> for std::io::Error {
    fn from(error: ParseRevisionError) -> Self {
        io::Error::new(io::ErrorKind::Other, error.to_string())
    }
}

impl fmt::Display for ParseRevisionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseRevisionError::InvalidFormat(input) => {
                write!(f, "Invalid revision format: {}", input)
            }
        }
    }
}

impl std::error::Error for ParseRevisionError {}

#[derive(Debug, PartialEq)]
pub enum Revision {
    Reference(String),
    Parent(Box<Revision>),
    Ancestor(Box<Revision>, u32),
}

impl Revision {
    ///
    /// Parse a revision from a string.
    ///
    /// # Examples:
    ///
    /// ```
    /// use rut::refs::Revision;
    ///
    /// let revision = Revision::parse("HEAD").unwrap();
    /// assert_eq!(revision, Revision::Reference("HEAD".to_owned()));
    ///
    /// let parent_revision = Revision::parse("HEAD^").unwrap();
    /// assert_eq!(
    ///   parent_revision,
    ///   Revision::Parent(Box::new(Revision::Reference("HEAD".to_owned())))
    /// );
    ///
    /// let ancestor_revision = Revision::parse("HEAD~3").unwrap();
    /// assert_eq!(
    ///   ancestor_revision,
    ///   Revision::Ancestor(Box::new(Revision::Reference("HEAD".to_owned())), 3)
    /// );
    /// ```
    ///
    pub fn parse(s: &str) -> Result<Revision, ParseRevisionError> {
        let invalid_regex = Regex::new(INVALID_BRANCH_NAME_PATTERN).unwrap();
        let parent_regex = Regex::new(PARENT_PATTERN).unwrap();
        let ancestor_regex = Regex::new(ANCESTOR_PATTERN).unwrap();
        let err = ParseRevisionError::InvalidFormat(s.to_owned());

        if let Some(group) = parent_regex.captures(s).map(|g| g.get(1)).flatten() {
            let nested_rev = Revision::parse(group.as_str())?;
            Ok(Revision::Parent(Box::new(nested_rev)))
        } else if let Some(matches) = ancestor_regex.captures(s) {
            let nested_rev = Revision::parse(matches.get(1).unwrap().as_str())?;
            let count = matches
                .get(2)
                .unwrap()
                .as_str()
                .parse::<u32>()
                .map_err(|_| err)?;
            Ok(Revision::Ancestor(Box::new(nested_rev), count))
        } else if !invalid_regex.is_match(s) {
            Ok(Revision::Reference(s.to_owned()))
        } else {
            Err(err)
        }
    }

    pub fn resolve(&self, repository: &Repository) -> io::Result<ObjectId> {
        let refs = RefHandler::new(repository);

        let err = |revision: &Revision| {
            Error::new(
                ErrorKind::Other,
                format!(
                    "fatal: ambiguous argument '{:?}': unknown revision",
                    revision
                ),
            )
        };

        match self {
            Revision::Reference(reference) => refs.deref(reference),
            Revision::Parent(revision) => {
                let oid = revision.resolve(repository)?;
                let commit = repository.database.load_commit(&oid)?;
                commit.parent.ok_or_else(|| err(revision))
            }
            Revision::Ancestor(revision, count) => {
                let oid = revision.resolve(repository)?;
                let commit = repository.database.load_commit(&oid)?;
                let mut parent_oid = commit.parent.ok_or_else(|| err(revision))?;

                for _ in 1..*count {
                    let parent_commit = repository.database.load_commit(&parent_oid)?;
                    parent_oid = parent_commit.parent.ok_or_else(|| err(revision))?;
                }

                Ok(parent_oid)
            }
        }
    }
}

impl FromStr for Revision {
    type Err = ParseRevisionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Revision::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_parent_revision() {
        let revision = Revision::parse("HEAD^").unwrap();
        assert_eq!(
            revision,
            Revision::Parent(Box::new(Revision::Reference("HEAD".to_owned())))
        );
    }

    #[test]
    fn test_parse_ancestor_revision() {
        let revision = Revision::parse("HEAD~3").unwrap();
        assert_eq!(
            revision,
            Revision::Ancestor(Box::new(Revision::Reference("HEAD".to_owned())), 3)
        );
    }

    #[test]
    fn test_parse_error() {
        let parsed = Revision::parse("/HEAD~3");
        assert_eq!(
            parsed,
            Err(ParseRevisionError::InvalidFormat("/HEAD".to_owned()))
        );
    }
}
