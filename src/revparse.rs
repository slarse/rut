use std::io;

use crate::{output::OutputWriter, workspace::Repository, refs::Revision};

pub fn rev_parse(
    revision: &str,
    writer: &mut dyn OutputWriter,
    repository: &Repository,
) -> io::Result<()> {
    let revision = Revision::parse(revision).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            err.to_string(),
        )
    })?;
    let oid = revision.resolve(repository)?;
    writer.writeln(oid.to_string())?;
    Ok(())
}
