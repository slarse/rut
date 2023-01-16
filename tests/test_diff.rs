use std::{fs, io};

use rut_testhelpers;

#[test]
fn test_diff_shows_modified_unstaged_files() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "First line\nSecond line\nThird line\n")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&file, "Second line\nThird line\nFourth line\n")?;

    // act
    let output = rut_testhelpers::rut_diff(&repository)?;

    // assert
    assert_eq!(
        output,
        "--- a/file.txt\n+++ b/file.txt\n-First line\n Second line\n Third line\n+Fourth line\n \n"
    );

    Ok(())
}
