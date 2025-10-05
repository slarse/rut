use std::io;

use chrono::{Local, TimeZone};

use crate::objects::{Commit, GitObject};
use crate::output::{Color, OutputWriter, Style};
use crate::refs::RefHandler;
use crate::workspace::Repository;

#[derive(Debug, Clone, Default)]
pub enum Format {
    #[default]
    Default,
    Oneline,
}

#[derive(Default, Builder, Debug)]
pub struct Options {
    #[builder(default)]
    pub max_count: Option<u32>,

    #[builder(default)]
    pub format: Format,
}

pub fn log(
    repository: &Repository,
    options: &Options,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let refs = RefHandler::new(repository);
    let head_commit = repository.database.load_commit(&refs.head()?)?;

    let write_log = match options.format {
        Format::Oneline => write_log_message_oneline,
        Format::Default => write_log_message,
    };

    write_log(&head_commit, Some("main"), writer)?;

    let mut num_written_commits = 1;
    let max_count = options.max_count.unwrap_or(u32::MAX);

    let mut commit = head_commit;
    while commit.parent.is_some() && num_written_commits < max_count {
        commit = repository.database.load_commit(&commit.parent.unwrap())?;
        write_log(&commit, None, writer)?;
        num_written_commits += 1;
    }

    Ok(())
}

fn write_log_message_oneline(
    commit: &Commit,
    branch: Option<&str>,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    writer
        .set_color(Color::Brown)?
        .write(commit.short_id_as_string())?;

    if let Some(branch) = branch {
        write_branch(branch, writer)?
    }

    let first_line_of_message = commit.message.lines().next().unwrap();
    writer
        .reset_formatting()?
        .writeln(format!(" {}", first_line_of_message))?;
    Ok(())
}

fn write_log_message(
    commit: &Commit,
    branch: Option<&str>,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let timestamp_parse_error = io::Error::other("Failed to parse timestamp");

    writer
        .set_color(Color::Brown)?
        .write(format!("commit {}", commit.id_as_string()))?;

    if let Some(branch) = branch {
        write_branch(branch, writer)?
    }

    writer.reset_formatting()?.writeln(format!(
        "
Author: {}
Date:   {}

    {}",
        commit.author,
        to_local_timestring(commit.timestamp).ok_or(timestamp_parse_error)?,
        commit.message
    ))?;
    Ok(())
}

fn write_branch(branch: &str, writer: &mut dyn OutputWriter) -> io::Result<()> {
    writer
        .write(" (".to_string())?
        .set_color(Color::Cyan)?
        .set_style(Style::Bold)?
        .write("HEAD -> ".to_string())?
        .set_color(Color::Green)?
        .write(branch.to_string())?
        .set_color(Color::Brown)?
        .set_style(Style::Normal)?
        .write(")".to_string())?;
    Ok(())
}

pub fn to_local_timestring(timestamp: u64) -> Option<String> {
    let local_time = Local::now();
    let datetime =
        local_time
            .timezone()
            .from_utc_datetime(&chrono::DateTime::from_timestamp(
                timestamp as i64,
                0,
            )?.naive_utc());
    Some(datetime.format("%a %b%e %T %Y %z").to_string())
}
