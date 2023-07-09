use std::fmt::Debug;
use std::io::Error;

use crate::output::{Color, OutputWriter, Style};
use crate::{add, commit, init, log, rm, status, workspace::Repository};
use crate::{diff, restore};
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
        #[arg(short, long)]
        message: Option<String>,
    },
    Add {
        path: String,
    },
    Rm {
        path: String,
    },
    Status {
        #[arg(long)]
        porcelain: bool,
    },
    Diff {
        #[arg(long)]
        cached: bool,
    },
    Restore {
        path: String,
        #[arg(long, default_value = "HEAD")]
        source: String,
    },
    Log,
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
            let options = commit::OptionsBuilder::default()
                .message(message)
                .build()
                .unwrap();
            commit::commit(&repository, &options, &mut writer)?;
        }
        Action::Add { path } => {
            add::add(resolve_path(&path)?, &repository)?;
        }
        Action::Rm { path } => {
            rm::rm(resolve_path(&path)?, &repository)?;
        }
        Action::Status { porcelain } => {
            let options = status::Options {
                output_format: if porcelain {
                    status::OutputFormat::Porcelain
                } else {
                    status::OutputFormat::HumanReadable
                },
            };
            status::status(&repository, &options, &mut writer)?;
        }
        Action::Diff { cached } => {
            let options = diff::OptionsBuilder::default()
                .cached(cached)
                .build()
                .unwrap();
            diff::diff_repository(&repository, &options, &mut writer)?;
        }
        Action::Restore { path, source } => {
            let options = restore::OptionsBuilder::default()
                .source(source)
                .build()
                .unwrap();
            restore::restore_worktree(resolve_path(&path)?, &options, &repository)?;
        }
        Action::Log => {
            log::log(&repository, &mut writer)?;
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
            Color::Brown => "38;5;130",
        };
        print_ansi_code(ansi_code);
        Ok(self)
    }

    fn set_style(&mut self, style: Style) -> io::Result<&mut dyn OutputWriter> {
        let ansi_code = match style {
            Style::Bold => "1",
            Style::Normal => "22",
        };
        print_ansi_code(ansi_code);
        Ok(self)
    }

    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter> {
        print!("\x1b[0m");
        Ok(self)
    }
}

fn print_ansi_code(ansi_code: &str) {
    print!("\x1b[{}m", ansi_code);
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
