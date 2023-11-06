use std::io;

#[test]
fn test_create_valid_branch() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let commit_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    rut_testhelpers::run_command_string("branch new-branch", &repository)?;

    // assert
    rut_testhelpers::assert_file_contains(
        &repository.git_dir().join("refs/heads/new-branch"),
        &commit_oid,
    );

    Ok(())
}

#[test]
fn test_error_on_creating_duplicate_branch() -> io::Result<()> {
    // arrrange
    let repository = rut_testhelpers::create_repository();
    rut_testhelpers::rut_commit("Initial commit", &repository)?;
    rut_testhelpers::run_command_string("branch new-branch", &repository)?;

    // act
    let result = rut_testhelpers::run_command_string("branch new-branch", &repository);

    // assert
    match result {
        Ok(_) => panic!("should have failed to create duplicate branch"),
        Err(error) => {
            let message = error.to_string();
            let expected_message = "fatal: a branch named 'new-branch' already exists";
            assert_eq!(message, expected_message);
        }
    }

    Ok(())
}

#[test]
fn test_error_on_invalid_branch_name() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    let result = rut_testhelpers::run_command_string("branch ../../etc/passwd", &repository);

    match result {
        Ok(_) => panic!("expected error on invalid branch name"),
        Err(error) => {
            let message = error.to_string();
            let expected_message = "fatal: '../../etc/passwd' is not a valid branch name";
            assert_eq!(message, expected_message);
        }
    }

    Ok(())
}
