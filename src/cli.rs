use std::fmt::Debug;
use std::io::Error;

use crate::diff;
use crate::output::{Color, OutputWriter};
use crate::{add, commit, init, rm, status, workspace::Repository};
use std::env;
use std::io;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    Init,
    Commit {
        #[clap(short, long)]
        message: Option<String>,
    },
    Add {
        path: String,
    },
    Rm {
        path: String,
    },
    Status {
        #[clap(long)]
        porcelain: bool,
    },
    Diff {
        #[clap(long)]
        cached: bool,
    },
}

pub fn run_command(args: Vec<String>) -> io::Result<()> {
    let workdir = env::current_dir()?;
    let git_dir = workdir.join(".git");

    let repository = Repository::from_worktree_root(workdir);
    let mut writer = StdoutWriter {};

    let args = Args::parse_from(args);

    match args.action {
        Action::Init => {
            init::init(&git_dir, &mut writer)?;
        }
        Action::Commit { message } => {
            commit::commit(&repository, message.as_deref(), &mut writer)?;
        }
        Action::Add { path } => {
            add::add(resolve_path(&path)?, &repository)?;
        }
        Action::Rm { path } => {
            rm::rm(resolve_path(&path)?, &repository)?;
        }
        Action::Status { porcelain } => {
            let status_options = status::Options {
                output_format: if porcelain {
                    status::OutputFormat::Porcelain
                } else {
                    status::OutputFormat::HumanReadable
                },
            };
            status::status(&repository, &status_options, &mut writer)?;
        }
        Action::Diff { cached } => {
            let diff_options = diff::OptionsBuilder::default()
                .cached(cached)
                .build()
                .unwrap();
            diff::diff_repository(&repository, &diff_options, &mut writer)?;
        }
    }

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
