use std::io;
use std::io::prelude::*;
use std::process;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

use regex::RegexBuilder;

#[derive(Debug, Clone)]
enum SubError {
    FailedToWrite,
    InvalidUTF8,
    RegexError(regex::Error),
}

impl SubError {
    pub fn message(self) -> String {
        use SubError::*;

        match self {
            FailedToWrite => "Output stream has been closed".into(),
            InvalidUTF8 => "Input contains invalid UTF-8".into(),
            RegexError(e) => format!("{}", e),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

#[derive(Debug, Clone)]
struct Sub<'a> {
    pattern: &'a str,
    replacement: &'a str,
}

impl<'a> Sub<'a> {
    fn run(&self) -> Result<()> {
        let re = RegexBuilder::new(self.pattern)
            .build()
            .map_err(SubError::RegexError)?;

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut input = stdin.lock();
        let mut output = stdout.lock();

        let mut line_buffer = String::new();
        loop {
            line_buffer.clear();
            let num_bytes = input
                .read_line(&mut line_buffer)
                .map_err(|_| SubError::InvalidUTF8)?;
            if num_bytes == 0 {
                break;
            }

            let new_line = re.replace_all(&line_buffer, self.replacement);
            write!(output, "{}", new_line).map_err(|_| SubError::FailedToWrite)?;
        }

        Ok(())
    }
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

    let sub = Sub {
        pattern: matches.value_of("pattern").expect("required argument"),
        replacement: matches.value_of("replacement").expect("required argument"),
    };
    let result = sub.run();

    match result {
        Err(e) => {
            eprintln!("[sub error]: {}", e.message());
            process::exit(1);
        }
        Ok(_) => {}
    }
}
