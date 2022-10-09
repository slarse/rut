use std::{fs, io};

use rut::workspace::Repository;

use rut_testhelpers;

#[test]
fn test_status_shows_untracked_file() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let untracked_file = workdir.join("file.txt");
    fs::write(untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "?? file.txt\n");

    Ok(())
}

#[test]
fn test_status_does_not_show_unmodified_tracked_file() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let committed_file = workdir.join("file.txt");
    fs::write(&committed_file, "content")?;
    rut_testhelpers::rut_add(&committed_file, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "");

    Ok(())
}

#[test]
fn test_status_shows_entire_directory_as_untracked() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let untracked_directory = workdir.join("untracked");
    let untracked_file = untracked_directory.join("file.txt");
    fs::create_dir(untracked_directory)?;
    fs::write(&untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "?? untracked/\n");

    Ok(())
}

#[test]
fn test_output_path_sorting() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let untracked_directory = workdir.join("dir");
    let untracked_file = untracked_directory.join("file.txt");
    let other_untracked_file = workdir.join("file.txt");
    fs::create_dir(untracked_directory)?;
    fs::write(&untracked_file, "content")?;
    fs::write(&other_untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "?? dir/\n?? file.txt\n");

    Ok(())
}
