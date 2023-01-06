use std::{fs, io};

use rut::workspace::Repository;

use rut_testhelpers;

#[test]
fn test_first_commit_denoted_as_root_commit_in_status_message() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    // act
    let first_commit_output =
        rut_testhelpers::rut_commit_with_output_capture("First commit", &repository)?;
    let second_commit_output =
        rut_testhelpers::rut_commit_with_output_capture("Second commit", &repository)?;

    assert!(first_commit_output.contains("(root commit)"));
    assert!(!second_commit_output.contains("(root commit)"));

    Ok(())
}

#[test]
fn test_creating_commit_with_nested_directory() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let expected_root_tree_id = "e7c2decd32b47cb6f7204971ea9cbbb629bfa35c";

    let readme = workdir.join("README.md");
    let nested_dir = workdir.join("nested");
    let file_in_nested_dir = nested_dir.join("file.txt");

    fs::create_dir(&nested_dir)?;
    fs::write(&readme, "A README.")?;
    fs::write(&file_in_nested_dir, "A file.")?;

    // act
    rut_testhelpers::rut_add(&readme, &repository);
    rut_testhelpers::rut_add(&file_in_nested_dir, &repository);

    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // assert
    rut_testhelpers::assert_healthy_repo(&repository.git_dir());
    assert_is_root_tree(&repository, expected_root_tree_id);
    rut_testhelpers::assert_file_contains(
        &repository.git_dir().join("HEAD"),
        "ref: refs/heads/main",
    );

    Ok(())
}

#[test]
fn test_second_commit_gets_proper_parent() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let readme = repository.worktree().root().join("README.md");
    fs::write(&readme, "First commit content")?;

    // act
    rut_testhelpers::rut_add(&readme, &repository);

    let first_commit_sha = rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&readme, "Second commit content")?;
    rut_testhelpers::rut_add(&readme, &repository);
    let second_commit_sha = rut_testhelpers::rut_commit("Second commit", &repository)?;

    assert_ne!(first_commit_sha, second_commit_sha);

    rut_testhelpers::assert_file_contains(
        &repository.git_dir().join("refs/heads/main"),
        &second_commit_sha,
    );

    let second_commit_content =
        rut_testhelpers::git_cat_file(&repository.git_dir(), &second_commit_sha);
    assert_eq!(second_commit_content.contains(&first_commit_sha), true);

    Ok(())
}

fn assert_is_root_tree(repository: &Repository, root_tree_id: &str) {
    let root_tree_file = repository
        .objects_dir()
        .join(&root_tree_id[0..2])
        .join(&root_tree_id[2..]);

    assert!(root_tree_file.is_file());

    let git_dir = repository.git_dir();
    let stdout = rut_testhelpers::git_cat_file(&git_dir, "HEAD");
    assert_eq!(stdout.contains(root_tree_id), true);
}
