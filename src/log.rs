use std::io;

use chrono::{DateTime, Local};

use crate::hex;
use crate::objects::{Commit, GitObject};
use crate::output::{Color, OutputWriter, Style};
use crate::refs::RefHandler;
use crate::workspace::Repository;

#[derive(Default, Builder, Debug)]
pub struct Options {
    #[builder(default)]
    pub max_count: Option<u32>,
}

pub fn log(
    repository: &Repository,
    options: &Options,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let refs = RefHandler::new(repository);
    let head_commit_id = refs.head()?;
    let head_commit_id_hex = &hex::from_hex_string(&head_commit_id).unwrap();
    let head_commit = repository.database.load_commit(head_commit_id_hex)?;

    write_log_message(&head_commit, Some("main"), writer)?;

    let mut num_written_commits = 1;
    let max_count = options.max_count.unwrap_or(u32::MAX);

    let mut commit = head_commit;
    while commit.parent.is_some() && num_written_commits < max_count {
        commit = repository
            .database
            .load_commit(&hex::from_hex_string(&commit.parent.unwrap()).unwrap())?;
        write_log_message(&commit, None, writer)?;
        num_written_commits += 1;
    }

    Ok(())
}

fn write_log_message(
    commit: &Commit,
    branch: Option<&str>,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let timestamp_parse_error = io::Error::new(io::ErrorKind::Other, "Failed to parse timestamp");

    writer
        .set_color(Color::Brown)?
        .write(format!("commit {}", commit.id_as_string()))?;

    if let Some(branch) = branch {
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

pub fn to_local_timestring(timestamp: u64) -> Option<String> {
    let local_time = Local::now();
    let datetime = DateTime::<Local>::from_utc(
        chrono::NaiveDateTime::from_timestamp_opt(timestamp as i64, 0)?,
        local_time.offset().to_owned(),
    );
    Some(datetime.format("%a %b%e %T %Y %z").to_string())
}
