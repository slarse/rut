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
fn test_status_does_not_show_unmodified_tracked_file_with_modified_mtime() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let committed_file = workdir.join("file.txt");
    fs::write(&committed_file, "content")?;
    rut_testhelpers::rut_add(&committed_file, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // write the file again to change the mtime (I couldn't find "touch" in the stdlib)
    fs::write(&committed_file, "content")?;

    let index_before = repository.load_index_unlocked()?;
    let index_entry_before_status = index_before.get("file.txt").unwrap();

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "");

    let index_after = repository.load_index_unlocked()?;
    let index_entry_after_status = index_after.get("file.txt").unwrap();
    assert_ne!(index_entry_before_status, index_entry_after_status);

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

#[test]
fn test_shows_modified_file() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let tracked_file = workdir.join("file.txt");
    fs::write(&tracked_file, "content")?;
    rut_testhelpers::rut_add(&tracked_file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&tracked_file, "CONTENT")?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, " M file.txt\n");

    Ok(())
}

#[test]
fn test_shows_modified_staged_file_in_subdirectory() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let directory = workdir.join("dir");
    fs::create_dir(&directory)?;
    let file = directory.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&file, "more content")?;
    rut_testhelpers::rut_add(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "M  dir/file.txt\n");

    Ok(())
}

#[test]
fn test_shows_newly_created_file_in_subdirectory() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let directory = workdir.join("dir");
    fs::create_dir(&directory)?;
    let file = directory.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "A  dir/file.txt\n");

    Ok(())
}

#[test]
fn test_shows_deleted_unstaged_file() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let file = workdir.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::remove_file(&file)?;

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, " D file.txt\n");

    Ok(())
}

#[test]
fn test_shows_deleted_staged_file() -> io::Result<()> {
    // arrange
    let workdir = rut_testhelpers::create_temporary_directory();
    let repository = Repository::from_worktree_root(&workdir);
    rut_testhelpers::rut_init(&repository);

    let file = workdir.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    rut_testhelpers::rut_rm(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status(&repository)?;

    // assert
    assert_eq!(output, "D  file.txt\n");

    Ok(())
}
