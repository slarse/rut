use std::io;

use chrono::{DateTime, Local};

use crate::hex;
use crate::output::OutputWriter;
use crate::refs::RefHandler;
use crate::workspace::Repository;

pub fn log(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let refs = RefHandler::new(repository);
    let head_commit_id = refs.head()?;
    let head_commit_id_hex = &hex::from_hex_string(&head_commit_id).unwrap();
    let head_commit = repository.database.load_commit(head_commit_id_hex)?;

    let timestamp_parse_error = io::Error::new(io::ErrorKind::Other, "Failed to parse timestamp");

    writer.writeln(format!(
        "commit {} (HEAD -> main)
Author: {}
Date:   {}

    {}",
        head_commit_id,
        head_commit.author,
        to_local_timestring(head_commit.timestamp).ok_or(timestamp_parse_error)?,
        head_commit.message
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
