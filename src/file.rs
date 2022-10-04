use std::{
    fs::{self, File, OpenOptions},
    io,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn read_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/**
 * Atomically write to a file by first writing to a temporary file and then renaming it to the
 * target file.
 */
pub fn atomic_write(path: &PathBuf, mut content: &[u8]) -> io::Result<()> {
    let mut buffer_file = PathBuf::from(path);
    let buffer_file_extension = format!(
        "{}.rut-tmp-buffer",
        buffer_file
            .extension()
            .map(|extension| extension.to_str())
            .flatten()
            .unwrap_or("ext")
    );
    buffer_file.set_extension(buffer_file_extension);

    fs::write(&buffer_file, &mut content)?;
    fs::rename(&buffer_file, &path)
}

/**
 * Struct that enables synchronized atomic writing to files. On acquiring with a lock with
 * [`LockFile::acquire`] an empty lockfile is created in the file system. You can then use
 * [`LockFile::write`] to write content to the lockfile.
 *
 * When the [`LockFile`] goes out of scope, the lockfile itself is renamed to the target file for
 * which the lock was acquired. Renames are atomic operations, so there is no risk that someone
 * reading the file without acquiring the lock gets a partially written result.
 */
pub struct LockFile {
    path: PathBuf,
    lockfile: File,
    lockfile_path: PathBuf,
    has_write: bool,
}

impl LockFile {
    pub fn acquire(path: &PathBuf) -> io::Result<LockFile> {
        let base_extension = String::from("lock");
        let lockfile_extension = match path.extension() {
            Some(ext) => format!("{:?}.{}", ext, base_extension),
            None => base_extension,
        };
        let mut lockfile_path = PathBuf::from(path);
        lockfile_path.set_extension(lockfile_extension);

        let lockfile_result = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lockfile_path);
        let lockfile = LockFile::handle_lockfile_create_failure(lockfile_result, &lockfile_path)?;

        Ok(LockFile {
            path: path.to_owned(),
            lockfile,
            lockfile_path,
            has_write: false,
        })
    }

    pub fn write(&mut self, mut text: &[u8]) -> io::Result<()> {
        self.has_write = true;
        self.lockfile.write_all(&mut text)
    }

    fn handle_lockfile_create_failure(
        result: Result<File, io::Error>,
        lockfile_path: &PathBuf,
    ) -> std::io::Result<File> {
        match result {
            ok @ Ok(_) => ok,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let message = format!(
                    "fatal: Unable to create '{}': File exists.",
                    lockfile_path.to_str().unwrap()
                );
                Err(io::Error::new(io::ErrorKind::AlreadyExists, message))
            }
            err => err,
        }
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let error_message = format!("Failed to commit changes for {:?}", self.lockfile);
        if self.has_write {
            fs::rename(&self.lockfile_path, &self.path).expect(&error_message);
        } else {
            fs::remove_file(&self.lockfile_path).expect(&error_message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Output};
    use std::{fs, str};

    #[test]
    fn test_cannot_acquire_two_locks_for_same_file() {
        let workdir = create_temporary_directory();
        let file = workdir.join("file.txt");
        fs::write(&file, "Hello").expect("Failed to write file");

        let first_lockfile = LockFile::acquire(&file);
        let second_lockfile = LockFile::acquire(&file);

        assert_eq!(first_lockfile.is_ok(), true);
        assert_eq!(second_lockfile.is_err(), true);
    }

    #[test]
    fn test_lock_is_released_on_exiting_scope() {
        let workdir = create_temporary_directory();
        let file = workdir.join("file.txt");
        fs::write(&file, "Hello").expect("Failed to write file");

        {
            // we create a lockfile here just to acquire the lock, which should be released when we
            // exit this block s.t. the second lockfile can be created
            let _first_lockfile = LockFile::acquire(&file);
        }
        let second_lockfile = LockFile::acquire(&file);

        assert_eq!(second_lockfile.is_ok(), true);
    }

    #[test]
    fn test_content_is_written_on_exiting_scope() {
        let workdir = create_temporary_directory();
        let file = workdir.join("file.txt");
        fs::write(&file, "Hello").expect("Failed to write file");
        let new_file_content = "This is the new content!";

        {
            let mut lockfile = LockFile::acquire(&file).expect("Failed to acquire lock");
            lockfile
                .write(&mut new_file_content.as_bytes())
                .expect("Failed to write to lockfile");
        }

        let content = fs::read_to_string(&file).expect("Failed to read file");

        assert_eq!(content, new_file_content);
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
}

pub trait AsVec<T> {
    fn as_vec(&self) -> Vec<T>;
}

/**
 * A resource backed by a lockfile. The final write is atomically transferred to the original file
 * when this struct is destroyed.
 *
 * Do note that any intermediate writes are simply discarded.
 */
pub struct LockFileResource<T: AsVec<u8>> {
    lockfile: LockFile,
    resource: T,
}

impl<T: AsVec<u8>> LockFileResource<T> {
    pub fn new(lockfile: LockFile, resource: T) -> LockFileResource<T> {
        LockFileResource { lockfile, resource }
    }

    /**
     * Write the resource to the lockfile. The final write to the lockfile are committed to the
     * original resource once this struct is destroyed.
     */
    pub fn write(&mut self) -> io::Result<()> {
        self.lockfile.write(&mut self.resource.as_vec())
    }
}

impl<'a, T: AsVec<u8>> AsMut<T> for LockFileResource<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.resource
    }
}
