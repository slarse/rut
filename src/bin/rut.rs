use std::env;

use rut::{
    cli::{self, StdoutWriter},
    output::OutputWriter,
};

pub fn main() {
    let exit_status = internal_main();
    std::process::exit(exit_status);
}

fn internal_main() -> i32 {
    let args: Vec<String> = env::args().collect();

    let mut writer = StdoutWriter::new(!args.contains(&"--help".to_string()));

    let workdir = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => panic!("Failed to get current directory: {:?}", err),
    };

    match cli::run_command(args, workdir, &mut writer) {
        Ok(_) => 0,
        Err(fatal @ rut::Error::Fatal(_, _)) => {
            writer
                .writeln(format!("{}", fatal))
                .expect("Failed to write to stdout");
            1
        }
        Err(rut::Error::Clap(err)) => err.exit(),
        err @ Err(_) => panic!("something went horribly wrong: {:?}", err),
    }
}
