use std::fs;

use rut::status;

#[test]
fn test_status_shows_untracked_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let untracked_file = repository.worktree().root().join("file.txt");
    fs::write(untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "?? file.txt\n");

    Ok(())
}

#[test]
fn test_status_does_not_show_unmodified_tracked_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let committed_file = repository.worktree().root().join("file.txt");
    fs::write(&committed_file, "content")?;
    rut_testhelpers::rut_add(&committed_file, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "");

    Ok(())
}

#[test]
fn test_status_does_not_show_unmodified_tracked_file_with_modified_mtime() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let committed_file = repository.worktree().root().join("file.txt");
    fs::write(&committed_file, "content")?;
    rut_testhelpers::rut_add(&committed_file, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // write the file again to change the mtime (I couldn't find "touch" in the stdlib)
    fs::write(&committed_file, "content")?;

    let index_before = repository.load_index_unlocked()?;
    let index_entry_before_status = index_before.get("file.txt").unwrap();

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "");

    let index_after = repository.load_index_unlocked()?;
    let index_entry_after_status = index_after.get("file.txt").unwrap();
    assert_ne!(index_entry_before_status, index_entry_after_status);

    Ok(())
}

#[test]
fn test_status_shows_entire_directory_as_untracked() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let untracked_directory = repository.worktree().root().join("untracked");
    let untracked_file = untracked_directory.join("file.txt");
    fs::create_dir(untracked_directory)?;
    fs::write(untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "?? untracked/\n");

    Ok(())
}

#[test]
fn test_output_path_sorting() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let untracked_directory = workdir.join("dir");
    let untracked_file = untracked_directory.join("file.txt");
    let other_untracked_file = workdir.join("file.txt");
    fs::create_dir(untracked_directory)?;
    fs::write(untracked_file, "content")?;
    fs::write(other_untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "?? dir/\n?? file.txt\n");

    Ok(())
}

#[test]
fn test_shows_modified_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let tracked_file = repository.worktree().root().join("file.txt");
    fs::write(&tracked_file, "content")?;
    rut_testhelpers::rut_add(&tracked_file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&tracked_file, "CONTENT")?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, " M file.txt\n");

    Ok(())
}

#[test]
fn test_shows_modified_staged_file_in_subdirectory() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let directory = repository.worktree().root().join("dir");
    fs::create_dir(&directory)?;
    let file = directory.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&file, "more content")?;
    rut_testhelpers::rut_add(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "M  dir/file.txt\n");

    Ok(())
}

#[test]
fn test_shows_newly_created_file_in_subdirectory() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let directory = repository.worktree().root().join("dir");
    fs::create_dir(&directory)?;
    let file = directory.join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "A  dir/file.txt\n");

    Ok(())
}

#[test]
fn test_shows_deleted_unstaged_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::remove_file(&file)?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, " D file.txt\n");

    Ok(())
}

#[test]
fn test_shows_deleted_staged_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "content")?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    rut_testhelpers::rut_rm(&file, &repository);

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "D  file.txt\n");

    Ok(())
}

#[test]
fn test_human_readable_format() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let modified_file = workdir.join("modified.txt");
    fs::write(&modified_file, "content")?;
    rut_testhelpers::rut_add(&modified_file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    fs::write(&modified_file, "more content")?;

    let staged_file = workdir.join("staged.txt");
    fs::write(&staged_file, "content")?;
    rut_testhelpers::rut_add(&staged_file, &repository);

    let untracked_file = workdir.join("untracked.txt");
    fs::write(untracked_file, "content")?;

    let options = status::OptionsBuilder::default()
        .output_format(status::OutputFormat::HumanReadable)
        .build()
        .ok()
        .unwrap();

    // act
    let output = rut_testhelpers::rut_status(&repository, &options)?;

    assert_eq!(output, "Changes to be committed:\n\tnew file: staged.txt\n\nChanges not staged for commit:\n\tmodified: modified.txt\n\nUntracked files:\n\tuntracked.txt\n\n");

    Ok(())
}

#[test]
fn test_status_shows_untracked_file_in_tracked_directory() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let tracked_directory = workdir.join("tracked");
    let tracked_file = tracked_directory.join("file.txt");
    fs::create_dir(&tracked_directory)?;
    fs::write(tracked_file, "content")?;
    rut_testhelpers::rut_add(&tracked_directory, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    let untracked_file = tracked_directory.join("untracked.txt");
    fs::write(untracked_file, "content")?;

    // act
    let output = rut_testhelpers::rut_status_porcelain(&repository)?;

    // assert
    assert_eq!(output, "?? tracked/untracked.txt\n");

    Ok(())
}
