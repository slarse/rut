use std::{fs, io, path::PathBuf};

pub fn init(git_dir: &PathBuf) -> io::Result<()> {
    for subdir in ["objects", "refs"] {
        fs::create_dir_all(git_dir.join(subdir))?;
    }
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main")?;
    println!("Initialized empty Rut repository in {:#?}", git_dir);
    Ok(())
}
