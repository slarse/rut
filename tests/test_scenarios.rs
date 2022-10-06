use std::{
    fs,
    fs::File,
    io,
    io::Read,
    path::Path,
    path::PathBuf,
    process::{Command, Output},
    str,
};

use rut::{add, commit, index::Index, init, output::OutputWriter, rm, workspace::Repository};

#[test]
fn test_creating_commit_with_nested_directory() -> io::Result<()> {
    // arrange
    let expected_root_tree_id = "e7c2decd32b47cb6f7204971ea9cbbb629bfa35c";

    let workdir = create_temporary_directory();

    let readme = workdir.join("README.md");
    let nested_dir = workdir.join("nested");
    let file_in_nested_dir = nested_dir.join("file.txt");

    fs::create_dir(&nested_dir)?;
    fs::write(&readme, "A README.")?;
    fs::write(&file_in_nested_dir, "A file.")?;

    // act
    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);

    rut_add(&readme, &repository);
    rut_add(&file_in_nested_dir, &repository);

    commit("Initial commit", &repository)?;

    // assert
    assert_healthy_repo(&repository.git_dir());
    assert_is_root_tree(&repository, expected_root_tree_id);
    assert_file_contains(&repository.git_dir().join("HEAD"), "ref: refs/heads/main");

    Ok(())
}

#[test]
fn test_second_commit_gets_proper_parent() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();
    let readme = workdir.join("README.md");
    fs::write(&readme, "First commit content")?;

    // act
    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);

    rut_add(&readme, &repository);

    let first_commit_sha = commit("First commit", &repository)?;

    fs::write(&readme, "Second commit content")?;
    rut_add(&readme, &repository);
    let second_commit_sha = commit("Second commit", &repository)?;

    assert_ne!(first_commit_sha, second_commit_sha);

    assert_file_contains(
        &repository.git_dir().join("refs/heads/main"),
        &second_commit_sha,
    );

    let second_commit_content = git_cat_file(&repository.git_dir(), &second_commit_sha);
    assert_eq!(second_commit_content.contains(&first_commit_sha), true);

    Ok(())
}

#[test]
fn test_first_commit_denoted_as_root_commit_in_status_message() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();

    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);

    // act
    let first_commit_output = commit_with_output_capture("First commit", &repository)?;
    let second_commit_output = commit_with_output_capture("Second commit", &repository)?;

    assert!(first_commit_output.contains("(root commit)"));
    assert!(!second_commit_output.contains("(root commit)"));

    Ok(())
}

#[test]
fn test_add_directory() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();

    let readme = workdir.join("README.md");
    let nested_dir = workdir.join("nested");
    let file_in_nested_dir = nested_dir.join("file.txt");

    fs::create_dir(&nested_dir)?;
    fs::write(&readme, "A README.")?;
    fs::write(&file_in_nested_dir, "A file.")?;

    // act
    let repository = Repository::from_worktree_root(&workdir);
    rut_init(&repository);

    rut_add(&workdir, &repository);

    // assert
    let index = Index::from_file(&repository.git_dir().join("index"))?;
    let paths_in_index: Vec<&PathBuf> = index
        .get_entries()
        .iter()
        .map(|entry| &entry.path)
        .collect();

    let expected_paths: Vec<PathBuf> = ["README.md", "nested/file.txt"]
        .iter()
        .map(|path| PathBuf::from(path))
        .collect();

    assert_eq!(
        paths_in_index,
        expected_paths.iter().collect::<Vec<&PathBuf>>()
    );

    Ok(())
}

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
    commit("Initial commit", &repository)?;

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

#[test]
fn test_adding_file_when_index_is_locked() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();
    let readme = workdir.join("README.md");

    fs::write(&readme, "A README")?;

    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);
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

fn commit(commit_message: &str, repository: &Repository) -> io::Result<String> {
    fs::write(&repository.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    commit::commit(&repository, &mut NoopOutputWriter)?;
    Ok(get_head_commit(&repository.git_dir()))
}

fn commit_with_output_capture(commit_message: &str, repository: &Repository) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    fs::write(&repository.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    commit::commit(&repository, &mut output_writer)?;
    Ok(output_writer.output)
}

struct CapturingOutputWriter {
    output: String,
}

impl OutputWriter for CapturingOutputWriter {
    fn write(&mut self, content: String) -> io::Result<()> {
        Ok(self.output.push_str(content.as_str()))
    }
}

struct NoopOutputWriter;

impl OutputWriter for NoopOutputWriter {
    fn write(&mut self, _: String) -> io::Result<()> {
        Ok(())
    }
}

fn rut_add(path: &Path, repository: &Repository) {
    add::add(path.to_owned(), repository).expect("Failed to add file");
}

fn rut_rm(path: &PathBuf, repository: &Repository) {
    rm::rm(path, repository).expect("Failed to remove file");
}

fn rut_init(repository: &Repository) {
    init::init(repository.git_dir(), &mut NoopOutputWriter).expect("Failed to initialize repo");
}

fn get_head_commit(git_dir: &PathBuf) -> String {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "rev-parse", "HEAD"])
        .output()
        .expect("Failed running 'git rev-parse HEAD'");
    get_stdout(&output)
}

fn git_cat_file(git_dir: &PathBuf, reference: &str) -> String {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "cat-file", "-p", reference])
        .output()
        .expect("Failed running 'git cat-file -p HEAD'");
    assert_eq!(output.status.code().unwrap(), 0);
    get_stdout(&output)
}

fn assert_healthy_repo(git_dir: &PathBuf) {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "status"])
        .output()
        .expect("Failed running 'git status'");
    assert_eq!(output.status.code().unwrap(), 0);
}

fn assert_is_root_tree(repository: &Repository, root_tree_id: &str) {
    let root_tree_file = repository
        .objects_dir()
        .join(&root_tree_id[0..2])
        .join(&root_tree_id[2..]);

    assert_eq!(root_tree_file.is_file(), true);

    let git_dir = repository.git_dir();
    let stdout = git_cat_file(&git_dir, "HEAD");
    assert_eq!(stdout.contains(root_tree_id), true);
}

fn assert_file_contains(path: &PathBuf, expected_content: &str) {
    let mut file = File::open(path).ok().unwrap();
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes).expect("Failed to read file");
    let actual_content = str::from_utf8(&bytes).ok().unwrap();

    assert_eq!(actual_content, expected_content);
}

fn create_temporary_directory() -> PathBuf {
    let output = Command::new("mktemp")
        .args(["-d", "--tmpdir", "rut-test-XXXXXX"])
        .output()
        .expect("Failed running mktemp command");
    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = get_stdout(&output);
    PathBuf::from(stdout)
}

fn get_stdout(output: &Output) -> String {
    String::from(
        str::from_utf8(&output.stdout)
            .expect("Failed to decode process output")
            .trim_end_matches("\n"),
    )
}
