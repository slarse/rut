use std::{fs, path::PathBuf};

use rut::index::Index;

#[test]
fn test_add_directory() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let readme = workdir.join("README.md");
    let nested_dir = workdir.join("nested");
    let file_in_nested_dir = nested_dir.join("file.txt");

    fs::create_dir(&nested_dir)?;
    fs::write(readme, "A README.")?;
    fs::write(file_in_nested_dir, "A file.")?;

    // act
    rut_testhelpers::run_command_string("add .", &repository)?;

    // assert
    let index = Index::from_file(repository.git_dir().join("index"))?;
    let paths_in_index: Vec<&PathBuf> = index
        .get_entries()
        .iter()
        .map(|entry| &entry.path)
        .collect();

    let expected_paths: Vec<PathBuf> = ["README.md", "nested/file.txt"]
        .iter()
        .map(PathBuf::from)
        .collect();

    assert_eq!(
        paths_in_index,
        expected_paths.iter().collect::<Vec<&PathBuf>>()
    );

    Ok(())
}

#[test]
fn test_adding_file_when_index_is_locked() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let readme = repository.worktree().root().join("README.md");
    fs::write(&readme, "A README")?;

    let index_lockfile = repository.git_dir().join("index.lock");
    fs::write(&index_lockfile, ";")?;

    // act
    let add_result = rut_testhelpers::run_command_string("add README.md", &repository);

    // assert
    assert!(add_result.is_err());
    match add_result {
        Ok(_) => panic!("should have failed to add due to index lock"),
        Err(error) => {
            let message = error.to_string();
            let expected_message = format!(
                "fatal: Unable to create '{}': File exists.",
                index_lockfile.to_str().unwrap()
            );
            assert_eq!(message, expected_message);
        }
    }

    Ok(())
}

#[test]
fn test_add_removed_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let readme = repository.worktree().root().join("README.md");
    fs::write(&readme, "A README")?;

    rut_testhelpers::rut_add(&readme, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    fs::remove_file(&readme)?;

    // act
    rut_testhelpers::run_command_string("add README.md", &repository)?;

    // assert
    let index = repository.load_index_unlocked()?;
    let readme_relative_path = repository.worktree().relativize_path(&readme);
    let readme_entry = index.get(&readme_relative_path);

    assert!(readme_entry.is_none());

    Ok(())
}

#[test]
fn test_add_nonexisting_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let add_result =
        rut_testhelpers::run_command_string("add file/that/does/not/exist", &repository);

    match add_result {
        Ok(_) => panic!("should have failed to add nonexisting file"),
        Err(error) => {
            let message = error.to_string();
            let expected_message =
                "fatal: pathspec \"file/that/does/not/exist\" did not match any files";
            assert_eq!(message, expected_message);
        }
    };

    Ok(())
}
