use std::{
    fs,
    fs::File,
    io,
    io::Read,
    path::PathBuf,
    process::{Command, Output},
    str,
};

use rut::{add, commit, index::Index, init};

use rut::workspace::{Database, Workspace};

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
    let workspace = Workspace::new(workdir);
    init::init(&workspace.git_dir())?;
    let database = Database::new(workspace.git_dir());

    rut_add(&readme, &workspace, &database);
    rut_add(&file_in_nested_dir, &workspace, &database);

    commit("Initial commit", &workspace, &database)?;

    // assert
    assert_healthy_repo(&workspace.git_dir());
    assert_is_root_tree(&workspace, expected_root_tree_id);
    assert_file_contains(&workspace.git_dir().join("HEAD"), "ref: refs/heads/main");

    Ok(())
}

#[test]
fn test_second_commit_gets_proper_parent() -> io::Result<()> {
    // arrange
    let workdir = create_temporary_directory();
    let readme = workdir.join("README.md");
    fs::write(&readme, "First commit content")?;

    // act
    let workspace = Workspace::new(workdir);
    init::init(&workspace.git_dir())?;
    let database = Database::new(workspace.git_dir());

    rut_add(&readme, &workspace, &database);

    let first_commit_sha = commit("First commit", &workspace, &database)?;

    fs::write(&readme, "Second commit content")?;
    rut_add(&readme, &workspace, &database);
    let second_commit_sha = commit("Second commit", &workspace, &database)?;

    assert_ne!(first_commit_sha, second_commit_sha);

    assert_file_contains(
        &workspace.git_dir().join("refs/heads/main"),
        &second_commit_sha,
    );

    let second_commit_content = git_cat_file(&workspace.git_dir(), &second_commit_sha);
    assert_eq!(second_commit_content.contains(&first_commit_sha), true);

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
    let workspace = Workspace::new(workdir);
    init::init(&workspace.git_dir())?;
    let database = Database::new(workspace.git_dir());

    rut_add(workspace.workdir(), &workspace, &database);

    // assert
    let index = Index::from_file(&workspace.git_dir().join("index"))?;
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

fn commit(commit_message: &str, workspace: &Workspace, database: &Database) -> io::Result<String> {
    fs::write(&workspace.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    commit::commit(&workspace, &database)?;
    Ok(get_head_commit(&workspace.git_dir()))
}

fn rut_add(path: &PathBuf, workspace: &Workspace, database: &Database) {
    add::add(path.to_owned(), &workspace, &database).expect("Failed to add file");
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
        .args(["--git-dir", git_dir_arg, "show", "HEAD"])
        .output()
        .expect("Failed running 'git show HEAD'");
    assert_eq!(output.status.code().unwrap(), 0);
}

fn assert_is_root_tree(workspace: &Workspace, root_tree_id: &str) {
    let root_tree_file = workspace
        .objects_dir()
        .join(&root_tree_id[0..2])
        .join(&root_tree_id[2..]);

    assert_eq!(root_tree_file.is_file(), true);

    let git_dir = workspace.git_dir();
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
