use std::{fs, io, path::Path};

use crate::output::OutputWriter;

pub fn init<P: AsRef<Path>>(git_dir: P, writer: impl OutputWriter) -> io::Result<()> {
    for subdir in ["objects", "refs/heads"] {
        fs::create_dir_all(git_dir.as_ref().join(subdir))?;
    }
    fs::write(git_dir.as_ref().join("HEAD"), "ref: refs/heads/main")?;

    write_init_message(git_dir.as_ref(), writer)
}

fn write_init_message(git_dir: &Path, mut writer: impl OutputWriter) -> io::Result<()> {
    let message = format!("Initialized empty Rut repository in {:#?}", git_dir);
    writer.write(message)
}
