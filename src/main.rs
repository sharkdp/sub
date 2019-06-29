use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use std::error::Error;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

use regex::RegexBuilder;

use tempfile;

use atty;

#[derive(Debug)]
enum SubError {
    FailedToWrite(io::Error),
    InvalidUTF8(io::Error),
    RegexError(regex::Error),
    CouldNotOpenFile(io::Error),
    CouldNotCreateTempFile(io::Error),
    CouldNotModifyInplace(io::Error),
    CouldNotReadMetadata(io::Error),
    CouldNotSetPermissions(io::Error),
}

impl Error for SubError {

    fn description(&self) -> &str {
        match self {
            SubError::FailedToWrite(e) => e.description(),
            SubError::InvalidUTF8(_) => "Input contains invalid UTF-8",
            SubError::RegexError(e) => e.description(),
            SubError::CouldNotOpenFile(_) => "Could not open file",
            SubError::CouldNotCreateTempFile(_) => "Failed to create temporary file",
            SubError::CouldNotModifyInplace(_) => "Could not modify the file",
            SubError::CouldNotReadMetadata(_) => "Could not read metadata",
            SubError::CouldNotSetPermissions(_) => "Could not set permissions",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match self {
            SubError::FailedToWrite(e) => Some(e),
            SubError::InvalidUTF8(e) => Some(e),
            SubError::RegexError(e) => Some(e),
            SubError::CouldNotOpenFile(e) => Some(e),
            SubError::CouldNotCreateTempFile(e) => Some(e),
            SubError::CouldNotModifyInplace(e) => Some(e),
            SubError::CouldNotReadMetadata(e) => Some(e),
            SubError::CouldNotSetPermissions(e) => Some(e),
        }
    }
}

impl fmt::Display for SubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use SubError::*;

        match self {
            FailedToWrite(e) => write!(f, "Output stream has been closed: {}", e),
            InvalidUTF8(e) => write!(f, "Input contains invalid UTF-8: {}", e),
            RegexError(e) => write!(f, "{}", e),
            CouldNotOpenFile(e) => write!(f, "Could not open file '{}'", e),
            CouldNotCreateTempFile(e) => write!(f, "Failed to create temporary file: {}", e),
            CouldNotModifyInplace(e) => write!(
                f,
                "Could not modify the file in-place: {}",
                //path.to_string_lossy(),
                e
            ),
            CouldNotReadMetadata(e) => write!(
                f,
                "Could not read metadata from file '{}'",
                e
            ),
            CouldNotSetPermissions(e) => write!(
                f,
                "Could not set permissions of file '{}'",
                e
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
    whole_word: bool,
    match_pattern: Option<&'a str>,
    ignore_case: bool,
    inputs: Vec<Input<'a>>,
}

impl<'a> Sub<'a> {
    fn replace(&self, reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
        let pattern = if self.whole_word {
            r"\b".to_string() + self.pattern + r"\b"
        } else {
            self.pattern.into()
        };
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(self.ignore_case)
            .build()
            .map_err(SubError::RegexError)?;

        let match_re = self
            .match_pattern
            .map(|match_pattern| {
                RegexBuilder::new(&match_pattern)
                    .case_insensitive(self.ignore_case)
                    .build()
                    .map_err(SubError::RegexError)
            })
            .transpose()?;

        let mut line_buffer = String::new();
        loop {
            line_buffer.clear();
            let num_bytes = reader
                .read_line(&mut line_buffer)
                .map_err(SubError::InvalidUTF8)?;
            if num_bytes == 0 {
                break;
            }

            let new_line = if match_re
                .as_ref()
                .map_or(true, |match_re| match_re.is_match(&line_buffer))
            {
                re.replace_all(&line_buffer, self.replacement)
            } else {
                Cow::from(&line_buffer)
            };
            write!(writer, "{}", new_line).map_err(SubError::FailedToWrite)?;
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
                        File::open(path).map_err(SubError::CouldNotOpenFile)?;

                    Box::new(io::BufReader::new(file))
                }
            };

            if self.in_place {
                if let Input::File(path) = input {
                    let output_file = tempfile::Builder::new()
                        .prefix("sub_")
                        .tempfile()
                        .map_err(SubError::CouldNotCreateTempFile)?;
                    let mut writer = io::BufWriter::new(&output_file);
                    self.replace(&mut reader, &mut writer)?;

                    drop(writer); // close the input file

                    let perms = fs::metadata(path)
                        .map_err(SubError::CouldNotReadMetadata)?
                        .permissions();

                    fs::set_permissions(output_file.as_ref(), perms).map_err(SubError::CouldNotSetPermissions)?;

                    fs::copy(output_file.as_ref(), &path)
                        .map_err(SubError::CouldNotModifyInplace)?;
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
        .global_setting(AppSettings::UnifiedHelpMessage)
        .arg(
            Arg::with_name("in-place")
                .long("in-place")
                .short("i")
                .requires("file")
                .help("Edit files in place"),
        )
        .arg(
            Arg::with_name("whole-word")
                .long("whole-word")
                .short("w")
                .help("Only match the pattern on whole words"),
        )
        .arg(
            Arg::with_name("match")
                .long("match")
                .short("m")
                .takes_value(true)
                .value_name("pattern")
                .help("Only substitute on lines that match the pattern"),
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
        whole_word: matches.is_present("whole-word"),
        match_pattern: matches.value_of("match"),
        ignore_case: matches.is_present("ignore-case"),
        inputs: matches
            .values_of_os("file")
            .map_or(vec![Input::StdIn], |vs| vs.map(Input::File).collect()),
    };
    let result = sub.run();

    match result {
        Ok(_) | Err(SubError::FailedToWrite(_)) => {}
        Err(e) => {
            eprintln!("[sub error]: {}", e);
            process::exit(1);
        }
    }
}
