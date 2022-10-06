use std::{fs, io, path::PathBuf};

use rut::{index::Index, workspace::Repository};

use rut_testhelpers::{
    assert_healthy_repo, create_temporary_directory, rut_add, rut_commit, rut_init, rut_rm,
};

#[test]
fn test_remove_file() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();

    let readme = workdir.join("README.md");
    let file_txt = workdir.join("file.txt");

    fs::write(&readme, "A README.")?;
    fs::write(&file_txt, "A file.")?;

    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);

    rut_add(repository.worktree().root(), &repository);
    rut_commit("Initial commit", &repository)?;

    // act
    rut_rm(&readme, &repository);

    // assert
    assert_healthy_repo(&repository.git_dir());
    let index = Index::from_file(&repository.index_file())?;
    let paths_in_index: Vec<&PathBuf> = index
        .get_entries()
        .iter()
        .map(|entry| &entry.path)
        .collect();

    let expected_paths: Vec<PathBuf> = ["file.txt"]
        .iter()
        .map(|path| PathBuf::from(path))
        .collect();

    assert_eq!(
        paths_in_index,
        expected_paths.iter().collect::<Vec<&PathBuf>>()
    );

    Ok(())
}
