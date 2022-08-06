use std::{fs, io, path::Path};

pub fn init<P: AsRef<Path>>(git_dir: P) -> io::Result<()> {
    for subdir in ["objects", "refs/heads"] {
        fs::create_dir_all(git_dir.as_ref().join(subdir))?;
    }
    fs::write(git_dir.as_ref().join("HEAD"), "ref: refs/heads/main")?;
    println!("Initialized empty Rut repository in {:#?}", git_dir.as_ref());
    Ok(())
}
