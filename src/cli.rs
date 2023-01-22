use std::io::Error;

use crate::diff;
use crate::output::{Color, OutputWriter};
use crate::{add, commit, init, rm, status, workspace::Repository};
use std::env;
use std::io;
use std::path::PathBuf;

pub fn run_command(args: Vec<String>) -> io::Result<()> {
    let sliced_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let workdir = env::current_dir()?;
    let git_dir = workdir.join(".git");

    let repository = Repository::from_worktree_root(workdir);
    let mut writer = StdoutWriter {};

    match sliced_args[..] {
        ["init"] => {
            init::init(&git_dir, &mut writer)?;
        }
        ["commit"] => {
            commit::commit(&repository, &mut writer)?;
        }
        ["add", path] => {
            add::add(resolve_path(path)?, &repository)?;
        }
        ["rm", path] => {
            rm::rm(resolve_path(path)?, &repository)?;
        }
        ["status"] => {
            let status_options = Default::default();
            status::status(&repository, &status_options, &mut writer)?;
        }
        ["status", "--porcelain"] => {
            let status_options = status::Options {
                output_format: status::OutputFormat::Porcelain,
            };
            status::status(&repository, &status_options, &mut writer)?;
        }
        ["diff"] => {
            diff::diff_repository(&repository, &mut writer)?;
        }
        _ => panic!("unexpected command {:?}", sliced_args),
    };

    Ok(())
}

pub struct StdoutWriter;

impl OutputWriter for StdoutWriter {
    fn write(&mut self, content: String) -> io::Result<&mut dyn OutputWriter> {
        print!("{}", content);
        Ok(self)
    }

    fn set_color(&mut self, color: Color) -> io::Result<&mut dyn OutputWriter> {
        let ansi_code = match color {
            Color::Red => "31",
            Color::Green => "32",
            Color::Cyan => "36",
        };
        print!("\x1b[{}m", ansi_code);
        Ok(self)
    }

    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter> {
        print!("\x1b[0m");
        Ok(self)
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
