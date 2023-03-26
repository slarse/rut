use std::{fs, io};

#[test]
fn test_restores_unstaged_file_to_last_commit() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&file, "more content")?;

    // act
    rut_testhelpers::rut_restore(&file, &repository)?;

    // assert
    let output = fs::read_to_string(&file)?;
    assert_eq!(output, "content");

    Ok(())
}
