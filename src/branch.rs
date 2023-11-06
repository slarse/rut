use std::io::{self, ErrorKind};

use crate::{refs::RefHandler, workspace::Repository};

#[derive(Default, Builder, Debug)]
pub struct Options {
    pub name: Option<String>,
}

pub fn branch(options: &Options, repository: &Repository) -> io::Result<()> {
    if let Some(name) = &options.name {
        let refs = RefHandler::new(repository);
        let head = refs.head()?;
        let result = refs.create_ref(&name, &head);

        match result {
            Ok(_) => (),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let message = format!("fatal: a branch named '{}' already exists", name);
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
            err => return err,
        }
    }

    Ok(())
}
