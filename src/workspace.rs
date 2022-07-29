use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::config;
use crate::config::Config;
use crate::hex;
use crate::objects::GitObject;

pub struct Workspace {
    workdir: PathBuf,
}

impl Workspace {
    pub fn new(workdir: PathBuf) -> Workspace {
        Workspace { workdir }
    }

    pub fn workdir(&self) -> &PathBuf {
        &self.workdir
    }

    pub fn git_dir(&self) -> PathBuf {
        self.workdir.join(".git")
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.git_dir().join("objects")
    }

    pub fn get_config(&self) -> Config {
        config::read_config().unwrap()
    }
}

pub struct Database {
    git_dir: PathBuf,
}

impl Database {
    pub fn new(git_dir: PathBuf) -> Database {
        Database { git_dir }
    }

    pub fn store_object<'a>(&self, git_object: &'a (impl GitObject<'a> + 'a)) -> io::Result<()> {
        let object_id = git_object.id();
        let mut content = git_object.to_object_format();

        let dirname = hex::to_hex_string(&object_id[..2]);
        let filename = hex::to_hex_string(&object_id[2..]);
        let dirpath = self.git_dir.join("objects").join(dirname);
        fs::create_dir_all(&dirpath)?;

        let compressed_bytes = Database::compress(&mut content)?;
        if let Some(mut file) = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dirpath.join(&filename))
            .ok()
        {
            file.write_all(&compressed_bytes)?;
        }

        Ok(())
    }

    fn compress(content: &mut Vec<u8>) -> io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        let compressed_bytes = encoder.finish()?;
        Ok(compressed_bytes)
    }
}
