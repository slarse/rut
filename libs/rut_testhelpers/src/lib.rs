use std::{
    ffi::OsString,
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
    add, cli, commit, diff, init, log,
    output::{Color, OutputWriter, Style},
    restore, rm, status,
    workspace::Repository,
};

use shlex;

pub fn run_command<S: Into<OsString> + Clone + From<&'static str>>(
    args: Vec<S>,
    repository: &Repository,
) -> io::Result<String> {
    let mut writer = CapturingOutputWriter {
        output: String::new(),
    };
    
    let has_rut = args.get(0).map(|arg| arg.to_owned().into() == "rut").unwrap_or(false);
    let complete_args = if has_rut {
        args
    } else {
        let mut complete_args = args.clone();
        complete_args.insert(0, "rut".into());
        complete_args
    };

    cli::run_command(complete_args, repository.worktree().root(), &mut writer)?;
    Ok(writer.output)
}

pub fn run_command_string<S: AsRef<str>>(args: S, repository: &Repository) -> io::Result<String> {
    let args = shlex::split(args.as_ref())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Failed to split arguments"))?;
    let result = run_command(args, repository)?;
    Ok(result)
}

pub fn rut_commit_with_output_capture(
    commit_message: &str,
    repository: &Repository,
) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    let options = commit::OptionsBuilder::default()
        .message(Some(commit_message.to_owned()))
        .build()
        .unwrap();
    commit::commit(&repository, &options, &mut output_writer)?;
    Ok(output_writer.output)
}

pub fn rut_commit(commit_message: &str, repository: &Repository) -> io::Result<String> {
    fs::write(&repository.git_dir().join("COMMIT_EDITMSG"), commit_message)?;
    let options = commit::OptionsBuilder::default()
        .message(Some(commit_message.to_owned()))
        .build()
        .unwrap();
    commit::commit(&repository, &options, &mut NoopOutputWriter)?;

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

    fn set_style(&mut self, _style: Style) -> io::Result<&mut dyn OutputWriter> {
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

    fn set_style(&mut self, _: Style) -> io::Result<&mut dyn OutputWriter> {
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

pub fn rut_restore(
    file: &Path,
    options: &restore::Options,
    repository: &Repository,
) -> io::Result<()> {
    restore::restore_worktree(file, options, repository)?;
    Ok(())
}

pub fn rut_diff_default(repository: &Repository) -> io::Result<String> {
    let options = Default::default();
    rut_diff(repository, &options)
}

pub fn rut_diff(repository: &Repository, options: &diff::Options) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    diff::diff_repository(repository, options, &mut output_writer)?;
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

pub fn commit_content(
    repository: &Repository,
    file: &Path,
    content: &str,
    commit_message: &str,
) -> io::Result<String> {
    fs::write(file, content)?;
    rut_add(file, repository);
    rut_commit(commit_message, repository)
}

pub fn rut_log_default(repository: &Repository) -> io::Result<String> {
    let options = log::OptionsBuilder::default().build().ok().unwrap();
    rut_log(repository, &options)
}

pub fn rut_log(repository: &Repository, options: &log::Options) -> io::Result<String> {
    let mut output_writer = CapturingOutputWriter {
        output: String::new(),
    };
    log::log(repository, options, &mut output_writer)?;
    Ok(output_writer.output)
}
