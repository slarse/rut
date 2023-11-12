use rut::log;

use rut::objects::GitObject;
use rut::objects::ObjectId;

#[test]
fn test_log() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let commit_id = rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;

    // act
    let output = rut_testhelpers::run_command_string("log", &repository)?;

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
fn test_log_two_commits() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let first_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    let second_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let output = rut_testhelpers::run_command_string("log", &repository)?;

    // assert
    assert!(output.contains(&first_commit_id));
    assert!(output.contains(&second_commit_id));

    Ok(())
}

#[test]
fn test_log_two_commits_with_max_count_1() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let first_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    let second_commit_id =
        rut_testhelpers::commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let output = rut_testhelpers::run_command_string("log -n 1", &repository)?;

    // assert
    assert!(output.contains(&second_commit_id));
    assert!(!output.contains(&first_commit_id));
    Ok(())
}

#[test]
fn test_log_two_commits_with_oneline_formatting() -> rut::Result<()> {
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
    let output = rut_testhelpers::run_command_string("log --oneline", &repository)?;

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
