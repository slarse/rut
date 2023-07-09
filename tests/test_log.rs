use std::io;

use rut::hex;
use rut::log;

#[test]
fn test_log() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let commit_id = rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;

    // act
    let output = rut_testhelpers::rut_log(&repository)?;

    // assert
    let commit = repository.database.load_commit(&hex::from_hex_string(&commit_id).unwrap())?;
    let timestring = log::to_local_timestring(commit.timestamp).unwrap();
    let expected_output = format!("commit {} (HEAD -> main)
Author: {}
Date:   {}

    First commit
", commit_id, commit.author, timestring);
    assert_eq!(output, expected_output);

    Ok(())
}
