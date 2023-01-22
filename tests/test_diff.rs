use std::{fs, io, path::Path};

use rut::objects::{Blob, GitObject};

use rut_testhelpers;

#[test]
fn test_diff_shows_modified_unstaged_files() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "First line\nSecond line\nThird line\n")?;
    let old_blob = Blob::new(fs::read(&file)?);

    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&file, "Second line\nThird line\nFourth line\n")?;
    let new_blob = Blob::new(fs::read(&file)?);

    // act
    let output = rut_testhelpers::rut_diff(&repository)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_output = format!(
        "{}-First line\n Second line\n Third line\n+Fourth line\n \n",
        expected_header
    );

    assert_eq!(output, expected_output,);

    Ok(())
}

#[test]
fn test_diff_shows_context_lines() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "1\n2\n3\n4\n5\n6\n7\n8\n9")?;
    let old_blob = Blob::new(fs::read(&file)?);

    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&file, "1\n2\n3\n4\n6\n7\n8\n9")?;
    let new_blob = Blob::new(fs::read(&file)?);

    // act
    let output = rut_testhelpers::rut_diff(&repository)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_output = format!("{} 2\n 3\n 4\n-5\n 6\n 7\n 8\n", expected_header);
    assert_eq!(output, expected_output);

    Ok(())
}

fn create_expected_header<P: AsRef<Path>>(filepath: P, old_blob: &Blob, new_blob: &Blob) -> String {
    format!(
        "diff --git a/{} b/{}\nindex {}..{}\n--- a/{}\n+++ b/{}\n",
        filepath.as_ref().display(),
        filepath.as_ref().display(),
        &old_blob.short_id_as_string(),
        &new_blob.short_id_as_string(),
        filepath.as_ref().display(),
        filepath.as_ref().display(),
    )
}
