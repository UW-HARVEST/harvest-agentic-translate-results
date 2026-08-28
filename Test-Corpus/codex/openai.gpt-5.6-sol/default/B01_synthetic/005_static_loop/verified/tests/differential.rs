use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(program: &Path, arguments: &[&[u8]]) -> Output {
    Command::new(program)
        .args(arguments.iter().map(|argument| OsStr::from_bytes(argument)))
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()))
}

fn assert_matches_c(case: &str, arguments: &[&[u8]]) {
    let c = run(&c_driver(), arguments);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), arguments);

    assert_eq!(
        rust.stdout, c.stdout,
        "{case}: stdout differs\nC: {:?}\nRust: {:?}",
        c.stdout, rust.stdout
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case}: stderr differs\nC: {:?}\nRust: {:?}",
        c.stderr, rust.stderr
    );
    assert_eq!(
        rust.status, c.status,
        "{case}: exit status differs\nC: {}\nRust: {}",
        c.status, rust.status
    );
}

#[test]
fn differential_argument_matrix() {
    let cases: &[(&str, &[&[u8]])] = &[
        ("no argument", &[]),
        ("too many arguments", &[b"1", b"2"]),
        ("empty argument", &[b""]),
        ("C whitespace only", &[b" \t\n\r\x0b\x0c"]),
        ("plus sign only", &[b"+"]),
        ("minus sign only", &[b"-"]),
        ("nonnumeric argument", &[b"abc"]),
        ("zero", &[b"0"]),
        ("single positive integer", &[b"1"]),
        ("negative integer", &[b"-7"]),
        ("leading whitespace and plus", &[b" \t+12"]),
        ("numeric prefix with trailing text", &[b"123garbage"]),
        ("i32 maximum", &[b"2147483647"]),
        ("i32 minimum", &[b"-2147483648"]),
        ("one above i32 maximum", &[b"2147483648"]),
        ("one below i32 minimum", &[b"-2147483649"]),
        ("long maximum", &[b"9223372036854775807"]),
        ("long minimum", &[b"-9223372036854775808"]),
        ("positive strtol overflow", &[b"9223372036854775808"]),
        ("negative strtol overflow", &[b"-9223372036854775809"]),
        (
            "large positive strtol overflow",
            &[b"999999999999999999999999999999999999"],
        ),
        (
            "large negative strtol overflow",
            &[b"-999999999999999999999999999999999999"],
        ),
        ("non-UTF-8 trailing byte after digits", &[b"17\xff"]),
        ("non-UTF-8 first byte", &[b"\xff17"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}
