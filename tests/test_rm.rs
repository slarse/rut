use std::{fs, io, path::PathBuf};

use rut::index::Index;

#[test]
fn test_remove_file() -> rut::Result<()> {
    // arrange
    let repository = rut_testhelpers::create_repository();
    let workdir = repository.worktree().root();

    let readme = workdir.join("README.md");
    let file_txt = workdir.join("file.txt");

    fs::write(&readme, "A README.")?;
    fs::write(file_txt, "A file.")?;

    rut_testhelpers::rut_add(repository.worktree().root(), &repository);
    rut_testhelpers::rut_commit("Initial commit", &repository)?;

    // act
    rut_testhelpers::run_command_string("rm README.md", &repository)?;

    // assert
    rut_testhelpers::assert_healthy_repo(&repository.git_dir());
    let index = Index::from_file(repository.index_file())?;
    let paths_in_index: Vec<&PathBuf> = index
        .get_entries()
        .iter()
        .map(|entry| &entry.path)
        .collect();

    let expected_paths: Vec<PathBuf> = ["file.txt"].iter().map(PathBuf::from).collect();

    assert_eq!(
        paths_in_index,
        expected_paths.iter().collect::<Vec<&PathBuf>>()
    );

    Ok(())
}
