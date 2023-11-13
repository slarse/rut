#[test]
fn test_parse_head() -> rut::Result<()> {
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
fn test_parse_head_parent() -> rut::Result<()> {
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
fn test_parse_head_ancestor() -> rut::Result<()> {
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
fn test_parse_branch() -> rut::Result<()> {
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

#[test]
fn test_parse_short_commit_id() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let commit_oid = rut_testhelpers::rut_commit("Initial commit", &repository)?;
    let short_commit_oid = &commit_oid[..7];

    // act
    let command = format!("rev-parse {}", short_commit_oid);
    let output = rut_testhelpers::run_command_string(command, &repository)?;

    // assert
    assert_eq!(output, format!("{}\n", &commit_oid));

    Ok(())
}

#[test]
fn test_error_on_ambiguous_id() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    // object hashes of a file with just a "b" and a file with just an "f" both start on 6
    rut_testhelpers::rut_commit("b", &repository)?;
    rut_testhelpers::rut_commit("f", &repository)?;

    // act
    let result = rut_testhelpers::run_command_string("rev-parse 6", &repository);

    // assert
    match result {
        Ok(_) => panic!("expected error on ambiguous id"),
        Err(error) => {
            let message = error.to_string();
            let expected_message =
                "fatal: ambiguous argument '6': unknown revision or path not in the working tree.";
            assert_eq!(message, expected_message);
        }
    }

    Ok(())
}
