use std::io;
use std::io::prelude::*;

use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};

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

    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let new_line = line.replace(pattern, replacement);
        println!("{}", new_line);
    }
}
