use std::io::Error;

use crate::{
    add, commit, init, rm,
    workspace::{Database, Workspace},
};
use std::env;
use std::io;
use std::path::PathBuf;

pub fn run_command(args: Vec<String>) -> io::Result<()> {
    let sliced_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let workdir = env::current_dir()?;
    let git_dir = workdir.join(".git");

    let workspace = Workspace::new(workdir);
    let database = Database::new(workspace.git_dir());

    match sliced_args[..] {
        ["init"] => {
            init::init(&git_dir)?;
        }
        ["commit"] => {
            commit::commit(&workspace, &database)?;
        }
        ["add", path] => {
            add::add(resolve_path(path)?, &workspace, &database)?;
        }
        ["rm", path] => {
            rm::rm(resolve_path(path)?, &workspace)?;
        }
        _ => panic!("unexpected command {:?}", sliced_args),
    };

    Ok(())
}

fn resolve_path(path: &str) -> io::Result<PathBuf> {
    let resolved = PathBuf::from(path);
    return if resolved.exists() {
        Ok(resolved)
    } else {
        let message = format!("pathspec {:?} did not match any files", resolved);
        Err(Error::new(io::ErrorKind::Other, message))
    }
}
