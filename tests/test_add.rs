use std::{fs, io, path::PathBuf};

use rut::{add, index::Index};

#[test]
fn test_add_directory() -> io::Result<()> {
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
    rut_testhelpers::rut_add(workdir, &repository);

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
fn test_adding_file_when_index_is_locked() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let readme = repository.worktree().root().join("README.md");
    fs::write(&readme, "A README")?;

    let index_lockfile = repository.git_dir().join("index.lock");
    fs::write(&index_lockfile, ";")?;

    // act
    let add_result = add::add(readme, &repository);

    // assert
    assert!(add_result.is_err());
    match add_result {
        Ok(()) => panic!("should have failed to add due to index lock"),
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
fn test_add_removed_file() -> io::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();

    let readme = repository.worktree().root().join("README.md");
    fs::write(&readme, "A README")?;

    rut_testhelpers::rut_add(&readme, &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    fs::remove_file(&readme)?;

    // act
    add::add(&readme, &repository)?;

    // assert
    let index = repository.load_index_unlocked()?;
    let readme_relative_path = repository.worktree().relativize_path(&readme);
    let readme_entry = index.get(&readme_relative_path);

    assert!(readme_entry.is_none());

    Ok(())
}
