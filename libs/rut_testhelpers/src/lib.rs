use std::{
    fs,
    fs::File,
    io,
    io::Read,
    path::Path,
    path::PathBuf,
    process::{Command, Output},
    str, thread,
};

use rut::{
    add, commit, diff, init,
    output::{Color, OutputWriter},
    rm, status,
    workspace::Repository,
};

pub fn rut_commit_with_output_capture(
    commit_message: &str,
    repository: &Repository,
) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    fs::write(&repository.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    commit::commit(&repository, &mut output_writer)?;
    Ok(output_writer.output)
}

pub fn rut_commit(commit_message: &str, repository: &Repository) -> io::Result<String> {
    fs::write(&repository.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    commit::commit(&repository, &mut NoopOutputWriter)?;

    // sleep a little to ensure that we get a strict "happens-after" relationship the commit
    // and anything that follows it
    thread::sleep(std::time::Duration::from_millis(10));
    Ok(get_head_commit(&repository.git_dir()))
}

fn get_head_commit(git_dir: &PathBuf) -> String {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "rev-parse", "HEAD"])
        .output()
        .expect("Failed running 'git rev-parse HEAD'");
    get_stdout(&output)
}

fn get_stdout(output: &Output) -> String {
    String::from(
        str::from_utf8(&output.stdout)
            .expect("Failed to decode process output")
            .trim_end_matches("\n"),
    )
}

pub fn git_cat_file(git_dir: &PathBuf, reference: &str) -> String {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "cat-file", "-p", reference])
        .output()
        .expect("Failed running 'git cat-file -p HEAD'");
    assert_eq!(output.status.code().unwrap(), 0);
    get_stdout(&output)
}

struct CapturingOutputWriter {
    output: String,
}

impl OutputWriter for CapturingOutputWriter {
    fn write(&mut self, content: String) -> io::Result<&mut dyn OutputWriter> {
        self.output.push_str(content.as_str());
        Ok(self)
    }

    fn set_color(&mut self, _color: Color) -> io::Result<&mut dyn OutputWriter> {
        Ok(self)
    }

    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter> {
        Ok(self)
    }
}

struct NoopOutputWriter;

impl OutputWriter for NoopOutputWriter {
    fn write(&mut self, _: String) -> io::Result<&mut dyn OutputWriter> {
        Ok(self)
    }

    fn set_color(&mut self, _: Color) -> io::Result<&mut dyn OutputWriter> {
        Ok(self)
    }

    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter> {
        Ok(self)
    }
}

pub fn rut_add(path: &Path, repository: &Repository) {
    add::add(path.to_owned(), repository).expect("Failed to add file");
}

pub fn rut_rm(path: &PathBuf, repository: &Repository) {
    rm::rm(path, repository).expect("Failed to remove file");
}

pub fn rut_init(repository: &Repository) {
    init::init(repository.git_dir(), &mut NoopOutputWriter).expect("Failed to initialize repo");
}

pub fn rut_status_porcelain(repository: &Repository) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    let options = status::OptionsBuilder::default()
        .output_format(status::OutputFormat::Porcelain)
        .build()
        .ok()
        .unwrap();
    status::status(repository, &options, &mut output_writer)?;
    Ok(output_writer.output)
}

pub fn rut_status(repository: &Repository, options: &status::Options) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    status::status(repository, options, &mut output_writer)?;
    Ok(output_writer.output)
}

pub fn rut_diff(repository: &Repository) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    diff::diff_repository(repository, &mut output_writer)?;
    Ok(output_writer.output)
}

pub fn assert_healthy_repo(git_dir: &PathBuf) {
    let git_dir_arg = git_dir.as_os_str().to_str().unwrap();
    let output = Command::new("git")
        .args(["--git-dir", git_dir_arg, "status"])
        .output()
        .expect("Failed running 'git status'");
    assert_eq!(output.status.code().unwrap(), 0);
}

pub fn create_repository() -> Repository {
    let workdir = create_temporary_directory();
    let repository = Repository::from_worktree_root(workdir);
    rut_init(&repository);
    repository
}

pub fn create_temporary_directory() -> PathBuf {
    let output = Command::new("mktemp")
        .args(["-d", "--tmpdir", "rut-test-XXXXXX"])
        .output()
        .expect("Failed running mktemp command");
    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = get_stdout(&output);
    PathBuf::from(stdout)
}

pub fn assert_file_contains(path: &PathBuf, expected_content: &str) {
    let mut file = File::open(path).ok().unwrap();
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes).expect("Failed to read file");
    let actual_content = str::from_utf8(&bytes).ok().unwrap();

    assert_eq!(actual_content, expected_content);
}
