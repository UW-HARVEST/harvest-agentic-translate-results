use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(binary: &Path, args: &[&OsStr]) -> Output {
    Command::new(binary)
        .args(args.iter().copied())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_matches_c(case_name: &str, args: &[&str]) {
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    assert_os_args_match_c(case_name, &args);
}

fn assert_os_args_match_c(case_name: &str, args: &[&OsStr]) {
    let c = run(&c_binary(), args);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), args);

    assert_eq!(
        rust.stdout, c.stdout,
        "{case_name}: stdout differs for arguments {args:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case_name}: stderr differs for arguments {args:?}"
    );
    assert_eq!(
        rust.status, c.status,
        "{case_name}: exit status differs for arguments {args:?}"
    );
}

#[test]
fn argument_count_and_success_cases_match() {
    let cases: &[(&str, &[&str])] = &[
        ("no arguments", &[]),
        ("empty string", &[""]),
        ("single string argument", &["abcdef"]),
        ("empty range at string end", &["abcdef", "6"]),
        ("maximum argument count", &["abcdef", "1", "5"]),
        ("too many arguments", &["abcdef", "0", "1", "extra"]),
        ("long string", &["abcdefghijklmnopqrstuvwxyz", "5", "21"]),
    ];

    for (name, args) in cases {
        assert_matches_c(name, args);
    }
}

#[test]
fn non_utf8_string_bytes_match() {
    let string = OsString::from_vec(vec![b'a', 0x80, 0xff, b'z']);
    assert_os_args_match_c("non-UTF-8 string", &[string.as_os_str(), OsStr::new("1")]);
}

#[test]
fn second_argument_parsing_and_bounds_match() {
    let cases: &[(&str, &[&str])] = &[
        ("empty start", &["abcdef", ""]),
        ("nonnumeric start", &["abcdef", "abc"]),
        ("sign-only start", &["abcdef", "+"]),
        ("whitespace-only start", &["abcdef", " \t"]),
        ("partially numeric start", &["abcdef", "2junk"]),
        ("signed start", &["abcdef", "+2"]),
        ("leading-whitespace start", &["abcdef", "\t 2"]),
        ("start one past end", &["abcdef", "7"]),
        ("negative start", &["abcdef", "-1"]),
        ("maximum i32 start", &["abcdef", "2147483647"]),
        ("i32 truncating start", &["abcdef", "4294967297"]),
        (
            "positive long overflow start",
            &["abcdef", "999999999999999999999"],
        ),
        (
            "negative long overflow start",
            &["abcdef", "-999999999999999999999"],
        ),
    ];

    for (name, args) in cases {
        assert_matches_c(name, args);
    }
}

#[test]
fn third_argument_parsing_bounds_and_order_match() {
    let cases: &[(&str, &[&str])] = &[
        ("empty stop uses stale end pointer", &["abcdef", "0", ""]),
        (
            "nonnumeric stop uses stale end pointer",
            &["abcdef", "0", "abc"],
        ),
        (
            "sign-only stop uses stale end pointer",
            &["abcdef", "0", "-"],
        ),
        ("partially numeric stop", &["abcdef", "1", "4junk"]),
        ("signed stop", &["abcdef", "1", "+4"]),
        ("leading-whitespace stop", &["abcdef", "1", "\n 4"]),
        ("stop one past end", &["abcdef", "0", "7"]),
        (
            "negative stop checked before ordering",
            &["abcdef", "0", "-1"],
        ),
        ("stop equal to start", &["abcdef", "2", "2"]),
        ("stop before start", &["abcdef", "4", "2"]),
        (
            "positive long overflow stop",
            &["abcdef", "0", "999999999999999999999"],
        ),
        (
            "negative long overflow stop",
            &["abcdef", "0", "-999999999999999999999"],
        ),
    ];

    for (name, args) in cases {
        assert_matches_c(name, args);
    }
}
