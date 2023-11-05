use std::ffi::OsString;
use std::fmt::Debug;
use std::io::Error;

use crate::output::{Color, OutputWriter, Style};
use crate::{add, commit, diff, init, log, restore, rm, status, workspace::Repository};
use std::io;
use std::path::{Path, PathBuf};

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
    Log {
        #[arg(short = 'n', long)]
        max_count: Option<u32>,
        #[arg(long)]
        oneline: bool,
    },
}

pub fn run_command<P: AsRef<Path>, S: Into<OsString> + Clone>(
    args: Vec<S>,
    workdir: P,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let git_dir = workdir.as_ref().join(".git");

    let repository = Repository::from_worktree_root(workdir);

    let args = Args::parse_from(args);

    match args.action {
        Action::Init => {
            init::init(&git_dir, writer)?;
        }
        Action::Commit { message } => {
            let options = commit::OptionsBuilder::default()
                .message(message)
                .build()
                .unwrap();
            commit::commit(&repository, &options, writer)?;
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
            status::status(&repository, &options, writer)?;
        }
        Action::Diff { cached } => {
            let options = diff::OptionsBuilder::default()
                .cached(cached)
                .build()
                .unwrap();
            diff::diff_repository(&repository, &options, writer)?;
        }
        Action::Restore { path, source } => {
            let options = restore::OptionsBuilder::default()
                .source(source)
                .build()
                .unwrap();
            restore::restore_worktree(resolve_path(&path)?, &options, &repository)?;
        }
        Action::Log { max_count, oneline } => {
            let format = if oneline {
                log::Format::Oneline
            } else {
                log::Format::Default
            };

            let options = log::OptionsBuilder::default()
                .max_count(max_count)
                .format(format)
                .build()
                .unwrap();
            log::log(&repository, &options, writer)?;
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
