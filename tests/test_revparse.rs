use std::io;

#[test]
fn test_parse_head() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let commit_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    let output = rut_testhelpers::run_command_string("rev-parse HEAD", &repository)?;

    // assert
    assert_eq!(output, format!("{}\n", commit_oid));

    Ok(())
}

#[test]
fn test_parse_head_parent() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let parent_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;
    rut_testhelpers::rut_commit("Second commit", &repository)?;

    // act
    let output = rut_testhelpers::run_command_string("rev-parse HEAD^", &repository)?;

    // assert
    assert_eq!(output, format!("{}\n", parent_oid));

    Ok(())
}

#[test]
fn test_parse_head_ancestor() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let commit_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;
    rut_testhelpers::rut_commit("Second commit", &repository)?;
    rut_testhelpers::rut_commit("Third commit", &repository)?;

    // act
    let output = rut_testhelpers::run_command_string("rev-parse HEAD~2", &repository)?;

    // assert
    assert_eq!(output, format!("{}\n", commit_oid));

    Ok(())
}

#[test]
fn test_parse_branch() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let initial_commit_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;
    rut_testhelpers::run_command_string("branch new-branch", &repository)?;
    let second_commit_oid = rut_testhelpers::rut_commit("Second commit", &repository)?;

    // act
    let main_oid = rut_testhelpers::run_command_string("rev-parse main", &repository)?;
    let new_branch_oid = rut_testhelpers::run_command_string("rev-parse new-branch", &repository)?;

    // assert
    assert_eq!(main_oid, format!("{}\n", second_commit_oid));
    assert_eq!(new_branch_oid, format!("{}\n", initial_commit_oid));

    Ok(())
}
