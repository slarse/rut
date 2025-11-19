use std::fs;

/// Plan is just a debug command to show the planned changes to the worktree.
#[test]
fn test_switch_plan() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let root = repository.worktree().root();

    let to_update = root.join("to_update.txt");
    let to_delete = root.join("to_delete.txt");
    fs::write(&to_update, "Some text")?;
    fs::write(&to_delete, "Some more text")?;

    rut_testhelpers::rut_add(&to_update, &repository);
    rut_testhelpers::rut_add(&to_delete, &repository);
    let initial_commit_id = rut_testhelpers::rut_commit("Initial commit", &repository)?;

    let new_file = root.join("new.txt");
    fs::write(&new_file, "Some new text")?;
    fs::write(&to_update, "Also some new text")?;
    rut_testhelpers::rut_add(&new_file, &repository);
    rut_testhelpers::rut_add(&to_update, &repository);
    rut_testhelpers::rut_rm(&to_delete, &repository);
    rut_testhelpers::rut_commit("Second commit", &repository)?;

    // act
    let output = rut_testhelpers::run_command_string(
        format!("switch --plan --detach {initial_commit_id}"),
        &repository,
    )?;

    // assert
    let expected_output = "DELETE new.txt
UPDATE to_update.txt
ADD to_delete.txt
";

    assert_eq!(output, expected_output);

    Ok(())
}

#[test]
fn test_switch_plan_empty() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let file = repository.worktree().root().join("file.txt");

    fs::write(&file, "text")?;
    rut_testhelpers::rut_add(&file, &repository);
    let current_commit_id = rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    let output = rut_testhelpers::run_command_string(
        format!("switch --plan --detach {current_commit_id}"),
        &repository,
    )?;

    // assert
    assert_eq!(output, "");

    Ok(())
}
