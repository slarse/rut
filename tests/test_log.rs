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
    let output = rut_testhelpers::rut_log_default(&repository)?;

    // assert
    let commit = repository
        .database
        .load_commit(&hex::from_hex_string(&commit_id).unwrap())?;
    let timestring = log::to_local_timestring(commit.timestamp).unwrap();
    let expected_output = format!(
        "commit {} (HEAD -> main)
Author: {}
Date:   {}

    First commit
",
        commit_id, commit.author, timestring
    );
    assert_eq!(output, expected_output);

    Ok(())
}

#[test]
fn test_log_two_commits() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let first_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    let second_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let output = rut_testhelpers::rut_log_default(&repository)?;

    // assert
    assert!(output.contains(&first_commit_id));
    assert!(output.contains(&second_commit_id));

    Ok(())
}

#[test]
fn test_log_two_commits_with_max_count_1() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let first_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    let second_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let options = log::OptionsBuilder::default()
        .max_count(Some(1))
        .build()
        .unwrap();
    let output = rut_testhelpers::rut_log(&repository, &options)?;

    // assert
    assert!(output.contains(&second_commit_id));
    assert!(!output.contains(&first_commit_id));
    Ok(())
}
