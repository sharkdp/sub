use assert_cmd::prelude::*;
use std::process::Command;

fn sub() -> Command {
    Command::cargo_bin("sub").unwrap()
}

struct ReplacementTest {
    pattern: &'static str,
    replacement: &'static str,
    input: String,
}

impl ReplacementTest {
    pub fn new(pattern: &'static str, replacement: &'static str) -> Self {
        ReplacementTest {
            pattern,
            replacement,
            input: String::new(),
        }
    }

    pub fn for_input(&mut self, input: &str) -> &mut Self {
        self.input = input.into();
        self
    }

    pub fn expect_output(&mut self, output: &'static str) -> &mut Self {
        sub()
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
