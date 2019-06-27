use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

use regex::RegexBuilder;

use tempfile;

use atty;

#[derive(Debug)]
enum SubError {
    FailedToWrite,
    InvalidUTF8,
    RegexError(regex::Error),
    CouldNotOpenFile(OsString),
    CouldNotCreateTempFile,
    CouldNotModifyInplace(OsString, io::Error),
    CouldNotReadMetadata(OsString),
    CouldNotSetPermissions(OsString),
}

impl SubError {
    pub fn message(self) -> String {
        use SubError::*;

        match self {
            FailedToWrite => "Output stream has been closed".into(),
            InvalidUTF8 => "Input contains invalid UTF-8".into(),
            RegexError(e) => format!("{}", e),
            CouldNotOpenFile(path) => format!("Could not open file '{}'", path.to_string_lossy()),
            CouldNotCreateTempFile => "Failed to create temporary file".into(),
            CouldNotModifyInplace(path, io_error) => format!(
                "Could not modify the file '{}' in-place: {}",
                path.to_string_lossy(),
                io_error
            ),
            CouldNotReadMetadata(path) => format!(
                "Could not read metadata from file '{}'",
                path.to_string_lossy()
            ),
            CouldNotSetPermissions(path) => format!(
                "Could not set permissions of file '{}'",
                path.to_string_lossy()
            ),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

#[derive(Debug, Clone)]
enum Input<'a> {
    StdIn,
    File(&'a OsStr),
}

#[derive(Debug, Clone)]
struct Sub<'a> {
    pattern: &'a str,
    replacement: &'a str,
    in_place: bool,
    ignore_case: bool,
    inputs: Vec<Input<'a>>,
}

impl<'a> Sub<'a> {
    fn replace(&self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let re = RegexBuilder::new(self.pattern)
            .case_insensitive(self.ignore_case)
            .build()
            .map_err(SubError::RegexError)?;

        let mut line_buffer = String::new();
        loop {
            line_buffer.clear();
            let num_bytes = reader
                .read_line(&mut line_buffer)
                .map_err(|_| SubError::InvalidUTF8)?;
            if num_bytes == 0 {
                break;
            }

            let new_line = re.replace_all(&line_buffer, self.replacement);
            write!(writer, "{}", new_line).map_err(|_| SubError::FailedToWrite)?;
        }

        Ok(())
    }

    fn run(&self) -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        for input in &self.inputs {
            let mut reader: Box<dyn BufRead> = match input {
                Input::StdIn => Box::new(stdin.lock()),
                Input::File(path) => {
                    if Path::new(path).is_dir() {
                        eprintln!(
                            "[sub warning]: '{}' is a directory.",
                            path.to_string_lossy()
                        );
                        continue;
                    }

                    let file =
                        File::open(path).map_err(|_| SubError::CouldNotOpenFile(path.into()))?;

                    Box::new(io::BufReader::new(file))
                }
            };

            if self.in_place {
                if let Input::File(path) = input {
                    let output_file = tempfile::Builder::new()
                        .prefix("sub_")
                        .tempfile()
                        .map_err(|_| SubError::CouldNotCreateTempFile)?;
                    let mut writer = io::BufWriter::new(&output_file);
                    self.replace(&mut reader, &mut writer)?;

                    drop(writer); // close the input file

                    let perms = fs::metadata(path)
                        .map_err(|_| SubError::CouldNotReadMetadata(path.into()))?
                        .permissions();

                    fs::set_permissions(output_file.as_ref(), perms).map_err(|_| {
                        SubError::CouldNotSetPermissions(output_file.as_ref().into())
                    })?;

                    fs::copy(output_file.as_ref(), &path)
                        .map_err(|e| SubError::CouldNotModifyInplace(path.into(), e))?;
                } else {
                    unreachable!();
                }
            } else {
                if atty::is(atty::Stream::Stdout) {
                    let mut writer = stdout.lock();
                    self.replace(&mut reader, &mut writer)?;
                } else {
                    let mut writer = io::BufWriter::new(stdout.lock());
                    self.replace(&mut reader, &mut writer)?;
                };
            };
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
            Arg::with_name("in-place")
                .long("in-place")
                .short("i")
                .requires("file")
                .help("Edit files in place"),
        )
        .arg(
            Arg::with_name("ignore-case")
                .long("ignore-case")
                .short("I")
                .help("Use case-insensitive search"),
        )
        .arg(
            Arg::with_name("pattern")
                .required(true)
                .help("The search pattern that should be replaced"),
        )
        .arg(
            Arg::with_name("replacement")
                .required(true)
                .help("The string that should be substituted in"),
        )
        .arg(
            Arg::with_name("file")
                .multiple(true)
                .help("Input files to perform the substitution on."),
        );

    let matches = app.get_matches();

    let sub = Sub {
        pattern: matches.value_of("pattern").expect("required argument"),
        replacement: matches.value_of("replacement").expect("required argument"),
        in_place: matches.is_present("in-place"),
        ignore_case: matches.is_present("ignore-case"),
        inputs: matches
            .values_of_os("file")
            .map_or(vec![Input::StdIn], |vs| vs.map(Input::File).collect()),
    };
    let result = sub.run();

    match result {
        Ok(_) | Err(SubError::FailedToWrite) => {}
        Err(e) => {
            eprintln!("[sub error]: {}", e.message());
            process::exit(1);
        }
    }
}
