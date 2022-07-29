use std::env;

use rut::cli;

pub fn main() {
    let args: Vec<String> = env::args().collect();
    match cli::run_command(args) {
        Ok(_) => (),
        err @ Err(_) => panic!("something went horribly wrong: {:?}", err),
    }
}
