use std::env;

use rut::cli::{self, StdoutWriter};

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let mut writer = StdoutWriter {};

    let workdir = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => panic!("Failed to get current directory: {:?}", err),
    };

    match cli::run_command(args, workdir, &mut writer) {
        Ok(_) => (),
        err @ Err(_) => panic!("something went horribly wrong: {:?}", err),
    }
}
