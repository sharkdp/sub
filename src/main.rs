use clap::{App, AppSettings, crate_description, crate_name, crate_version};

fn main() {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .global_setting(AppSettings::ColoredHelp);
    app.get_matches();

    println!("Hello, world!");
}
