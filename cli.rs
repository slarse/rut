use crate::{
    commit, init,
    workspace::{Database, Workspace},
};
use std::env;
use std::io;

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
        _ => panic!("unexpected command {:?}", sliced_args),
    };

    Ok(())
}
