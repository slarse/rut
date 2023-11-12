use crate::{output::OutputWriter, refs::Revision, workspace::Repository};

pub fn rev_parse(
    revision: &str,
    writer: &mut dyn OutputWriter,
    repository: &Repository,
) -> crate::Result<()> {
    let oid = Revision::parse(revision)?.resolve(repository)?;
    writer.writeln(oid.to_string())?;
    Ok(())
}
