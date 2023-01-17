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

#[test]
fn test_diff_shows_context_lines() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "1\n2\n3\n4\n5\n6\n7\n8\n9")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&file, "1\n2\n3\n4\n6\n7\n8\n9")?;

    // act
    let output = rut_testhelpers::rut_diff(&repository)?;

    // assert
    assert_eq!(
        output,
        "--- a/file.txt\n+++ b/file.txt\n 2\n 3\n 4\n-5\n 6\n 7\n 8\n"
    );

    Ok(())
}
