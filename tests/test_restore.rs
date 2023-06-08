use std::{fs, io, path::Path};

use rut::restore;
use rut::workspace::Repository;

#[test]
fn test_restores_unstaged_file_to_last_commit() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    commit_content(&repository, &file, "content", "First commit")?;
    fs::write(&file, "more content")?;

    // act
    let options = restore::OptionsBuilder::default().build().unwrap();
    rut_testhelpers::rut_restore(&file, &options, &repository)?;

    // assert
    let output = fs::read_to_string(&file)?;
    assert_eq!(output, "content");

    Ok(())
}

#[test]
fn test_restores_file_to_specified_commit() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file_relpath = "file.txt";
    let file = repository.worktree().root().join(file_relpath);
    let first_commit = commit_content(&repository, &file, "content", "First commit")?;
    commit_content(&repository, &file, "more content", "Second commit")?;

    // act
    let options = restore::OptionsBuilder::default()
        .source(first_commit)
        .build()
        .unwrap();
    rut_testhelpers::rut_restore(&file, &options, &repository)?;

    // assert
    let output = fs::read_to_string(&file)?;
    assert_eq!(output, "content");

    Ok(())
}

fn commit_content(
    repository: &Repository,
    file: &Path,
    content: &str,
    commit_message: &str,
) -> io::Result<String> {
    fs::write(file, content)?;
    rut_testhelpers::rut_add(file, repository);
    rut_testhelpers::rut_commit(commit_message, repository)
}
