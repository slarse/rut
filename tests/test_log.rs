use std::io;

use rut::log;

use rut::objects::GitObject;
use rut::objects::ObjectId;

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
        .load_commit(&ObjectId::from_sha(&commit_id).unwrap())?;
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

#[test]
fn test_log_two_commits_with_oneline_formatting() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let first_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit\nwith body")?;
    let second_commit_id = rut_testhelpers::commit_content(
        &repository,
        &file,
        "more content",
        "Second commit\nwith body",
    )?;

    // act
    let options = log::OptionsBuilder::default()
        .format(log::Format::Oneline)
        .build()
        .unwrap();
    let output = rut_testhelpers::rut_log(&repository, &options)?;

    // assert
    let first_commit = repository
        .database
        .load_commit(&ObjectId::from_sha(&first_commit_id).unwrap())?;
    let second_commit = repository
        .database
        .load_commit(&ObjectId::from_sha(&second_commit_id).unwrap())?;

    assert_eq!(
        output,
        format!(
            "{} (HEAD -> main) {}\n{} {}\n",
            second_commit.short_id_as_string(),
            second_commit.message.lines().next().unwrap(),
            first_commit.short_id_as_string(),
            first_commit.message.lines().next().unwrap(),
        ),
    );

    Ok(())
}
