use std::io::Error;

use crate::output::OutputWriter;
use crate::{add, commit, init, rm, workspace::Repository};
use std::env;
use std::io;
use std::path::PathBuf;

pub fn run_command(args: Vec<String>) -> io::Result<()> {
    let sliced_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let workdir = env::current_dir()?;
    let git_dir = workdir.join(".git");

    let repository = Repository::from_worktree_root(workdir);
    let writer = StdoutWriter {};

    match sliced_args[..] {
        ["init"] => {
            init::init(&git_dir, writer)?;
        }
        ["commit"] => {
            commit::commit(&repository, writer)?;
        }
        ["add", path] => {
            add::add(resolve_path(path)?, &repository)?;
        }
        ["rm", path] => {
            rm::rm(resolve_path(path)?, &repository)?;
        }
        _ => panic!("unexpected command {:?}", sliced_args),
    };

    Ok(())
}

pub struct StdoutWriter;

impl OutputWriter for StdoutWriter {
    fn write(&mut self, content: String) -> io::Result<()> {
        Ok(println!("{}", content))
    }
}

fn resolve_path(path: &str) -> io::Result<PathBuf> {
    let resolved = PathBuf::from(path);
    return if resolved.exists() {
        Ok(resolved)
    } else {
        let message = format!("pathspec {:?} did not match any files", resolved);
        Err(Error::new(io::ErrorKind::Other, message))
    };
}
