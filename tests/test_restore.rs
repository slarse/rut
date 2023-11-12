use std::fs;

#[test]
fn test_restores_unstaged_file_to_last_commit() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    fs::write(&file, "more content")?;

    // act
    rut_testhelpers::run_command_string("restore file.txt", &repository)?;

    // assert
    let output = fs::read_to_string(&file)?;
    assert_eq!(output, "content");

    Ok(())
}

#[test]
fn test_restores_file_to_specified_commit() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file_relpath = "file.txt";
    let file = repository.worktree().root().join(file_relpath);
    let first_commit =
        rut_testhelpers::commit_content(&repository, &file, "content", "First commit")?;
    rut_testhelpers::commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let command = format!("restore --source={} {}", first_commit, file_relpath);
    rut_testhelpers::run_command_string(command, &repository)?;

    // assert
    let output = fs::read_to_string(&file)?;
    assert_eq!(output, "content");

    Ok(())
}
