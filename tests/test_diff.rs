use std::{fs, io, path::Path, thread};

use rut::objects::{Blob, GitObject};

use rut_testhelpers;

#[test]
fn test_diff_shows_modified_unstaged_files() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let file = repository.worktree().root().join("file.txt");
    fs::write(&file, "First line\nSecond line\nThird line")?;
    let old_blob = Blob::new(fs::read(&file)?);

    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;

    fs::write(&file, "Second line\nThird line\nFourth line")?;
    let new_blob = Blob::new(fs::read(&file)?);

    // act
    let output = rut_testhelpers::rut_diff_default(&repository)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_chunk_header = "@@ -1,3 +1,3 @@";
    let expected_output = format!(
        "{}{}\n-First line\n Second line\n Third line\n+Fourth line\n",
        expected_header, expected_chunk_header,
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
    let output = rut_testhelpers::rut_diff_default(&repository)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_chunk_header = "@@ -2,7 +2,6 @@";
    let expected_output = format!(
        "{}{}\n 2\n 3\n 4\n-5\n 6\n 7\n 8\n",
        expected_header, expected_chunk_header
    );
    assert_eq!(output, expected_output);

    Ok(())
}

#[test]
fn test_diff_omits_final_empty_line() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let initial_content = "1\n";
    fs::write(&file, initial_content)?;
    rut_testhelpers::rut_add(&file, &repository);
    let old_blob = Blob::new(initial_content.as_bytes().to_vec());

    wait_for_new_timestamp();
    let new_content = "1\n2\n";
    fs::write(&file, new_content)?;
    let new_blob = Blob::new(new_content.as_bytes().to_vec());

    // act
    let output = rut_testhelpers::rut_diff_default(&repository)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_chunk_header = "@@ -1 +1,2 @@";
    let expected_output = format!("{}{}\n 1\n+2\n", expected_header, expected_chunk_header);

    assert_eq!(output, expected_output);

    Ok(())
}

#[test]
fn test_diff_cached_shows_staged_changes() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let file = repository.worktree().root().join("file.txt");
    let initial_content = "1\n";
    fs::write(&file, initial_content)?;
    rut_testhelpers::rut_add(&file, &repository);
    rut_testhelpers::rut_commit("First commit", &repository)?;
    let old_blob = Blob::new(initial_content.as_bytes().to_vec());

    wait_for_new_timestamp();
    let new_content = "1\n2\n";
    fs::write(&file, new_content)?;
    let new_blob = Blob::new(new_content.as_bytes().to_vec());
    rut_testhelpers::rut_add(&file, &repository);

    // act
    let options = rut::diff::OptionsBuilder::default()
        .cached(true)
        .build()
        .ok()
        .unwrap();
    let output = rut_testhelpers::rut_diff(&repository, &options)?;

    // assert
    let expected_header = create_expected_header(
        repository.worktree().relativize_path(&file),
        &old_blob,
        &new_blob,
    );
    let expected_chunk_header = "@@ -1 +1,2 @@";
    let expected_output = format!("{}{}\n 1\n+2\n", expected_header, expected_chunk_header);

    assert_eq!(output, expected_output);

    Ok(())
}

/**
 * When writing tiny files in tests, there may not be enough time between writes to make for
 * different timestamps between the writes. We therefore need to sleep a tiny amount before
 * making a new write where there is a necessity to have it happen "strictly after" a previous
 * write to the same file.
 */
fn wait_for_new_timestamp() {
    thread::sleep(std::time::Duration::from_millis(10));
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
