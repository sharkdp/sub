use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile;

fn sub() -> Command {
    Command::cargo_bin("sub").unwrap()
}

struct ReplacementTest {
    pattern: &'static str,
    replacement: &'static str,
    input: String,
    args: Vec<String>,
}

impl ReplacementTest {
    pub fn new(pattern: &'static str, replacement: &'static str) -> Self {
        ReplacementTest {
            pattern,
            replacement,
            input: String::new(),
            args: vec![],
        }
    }

    pub fn arg(&mut self, argument: &str) -> &mut Self {
        self.args.push(argument.to_owned());
        self
    }

    pub fn for_input(&mut self, input: &str) -> &mut Self {
        self.input = input.into();
        self
    }

    pub fn expect_output(&mut self, output: &'static str) -> &mut Self {
        sub()
            .args(&self.args)
            .arg(self.pattern)
            .arg(self.replacement)
            .with_stdin()
            .buffer(self.input.as_str())
            .assert()
            .success()
            .stdout(output);
        self
    }
}

#[test]
fn basic_replacement() {
    ReplacementTest::new(r"foo", "bar")
        .for_input("foo other foo\nfoo\n")
        .expect_output("bar other bar\nbar\n");
}

#[test]
fn regex_replacement() {
    ReplacementTest::new(r"\bfoo\b", "bar")
        .for_input("foo, foo.\n")
        .expect_output("bar, bar.\n")
        .for_input("foobar\n")
        .expect_output("foobar\n")
        .for_input("myfoo\n")
        .expect_output("myfoo\n");
}

#[test]
fn capture_group_replacement() {
    ReplacementTest::new(r"foo([0-9]+)", "bar$1")
        .for_input("foo123\n")
        .expect_output("bar123\n")
        .for_input("foo\n")
        .expect_output("foo\n")
        .for_input("fooABC\n")
        .expect_output("fooABC\n");
}

#[test]
fn case_insensitive_replacement() {
    ReplacementTest::new(r"foo", "bar")
        .for_input("foo Foo")
        .expect_output("bar Foo");

    ReplacementTest::new(r"foo", "bar")
        .arg("--ignore-case")
        .for_input("foo Foo")
        .expect_output("bar bar");
}

#[test]
fn fails_for_non_utf8_input() {
    sub()
        .arg("dummy")
        .arg("dummy")
        .with_stdin()
        .buffer(b"\xC3\x28".as_ref())
        .assert()
        .failure()
        .code(1);
}

#[test]
fn no_trailing_newline() {
    ReplacementTest::new(r"foo", "bar")
        .for_input("foo other foo\nfoo")
        .expect_output("bar other bar\nbar");
}

#[test]
fn windows_newline() {
    ReplacementTest::new(r"foo", "bar")
        .for_input("foo other foo\r\nfoo\r\n")
        .expect_output("bar other bar\r\nbar\r\n");
}

fn get_tempfile() -> tempfile::NamedTempFile {
    tempfile::Builder::new()
        .prefix("sub_test")
        .suffix(".txt")
        .tempfile()
        .unwrap()
}

#[test]
fn reads_from_files() {
    let mut file1 = get_tempfile();
    file1.write_all(b"foo other foo\nfoo\n").unwrap();

    let mut file2 = get_tempfile();
    file2.write_all(b"more dummy text foo\n").unwrap();

    sub()
        .arg("foo")
        .arg("bar")
        .arg(file1.path())
        .arg(file2.path())
        .assert()
        .success()
        .stdout("bar other bar\nbar\nmore dummy text bar\n");
}

#[test]
fn fails_if_file_not_found() {
    let input_file = get_tempfile();

    let mut path_nonexistent = input_file.path().to_path_buf();
    path_nonexistent.push(".dummy");

    sub()
        .arg("foo")
        .arg("bar")
        .arg(path_nonexistent)
        .assert()
        .stderr(predicate::str::contains("Could not open"))
        .failure()
        .code(1);
}

#[test]
fn ignores_directory_arguments() {
    let dir = tempfile::Builder::new()
        .prefix("sub_test")
        .tempdir()
        .unwrap();

    sub()
        .arg("foo")
        .arg("bar")
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn modifies_files_in_place() {
    let mut file = get_tempfile();
    file.write_all(b"foo other foo\nfoo\n").unwrap();

    sub()
        .arg("--in-place")
        .arg("foo")
        .arg("bar")
        .arg(file.path())
        .assert()
        .success();

    let contents = fs::read_to_string(file.path()).unwrap();
    assert_eq!(contents, "bar other bar\nbar\n");
}
