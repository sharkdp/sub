use std::io;
use std::io::prelude::*;
use std::process;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

#[derive(Debug, Clone, Copy)]
enum SubError {
    FailedToWrite,
    InvalidUTF8,
}

impl SubError {
    pub fn message(self) -> String {
        use SubError::*;

        match self {
            FailedToWrite => "Output stream has been closed".into(),
            InvalidUTF8 => "Input contains invalid UTF-8".into(),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

fn run(pattern: &str, replacement: &str) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let input = stdin.lock();
    let mut output = stdout.lock();

    for line in input.lines() {
        let line = line.map_err(|_| SubError::InvalidUTF8)?;
        let new_line = line.replace(pattern, replacement);
        writeln!(output, "{}", new_line).map_err(|_| SubError::FailedToWrite)?;
    }

    Ok(())
}

fn main() {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("pattern")
                .required(true)
                .help("The search pattern that should be replaced"),
        )
        .arg(
            Arg::with_name("replacement")
                .required(true)
                .help("The string that should be substituted in"),
        );

    let matches = app.get_matches();

    let pattern = matches.value_of("pattern").expect("required argument");
    let replacement = matches.value_of("replacement").expect("required argument");

    let result = run(pattern, replacement);

    match result {
        Err(e) => {
            eprintln!("[sub error]: {}", e.message());
            process::exit(1);
        }
        Ok(_) => {}
    }
}
