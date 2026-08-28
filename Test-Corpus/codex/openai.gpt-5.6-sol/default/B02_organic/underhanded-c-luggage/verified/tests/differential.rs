use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static [u8],
}

fn run(binary: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");
    child.wait_with_output().expect("failed to wait for child")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("build the C program at c_src/build/driver before running tests")
}

fn assert_case(case: &Case) {
    let c = run(&c_binary(), case.args, case.input);
    let rust = run(
        Path::new(env!("CARGO_BIN_EXE_driver")),
        case.args,
        case.input,
    );

    assert_eq!(
        rust.stdout, c.stdout,
        "{}: stdout differs\nC: {:?}\nRust: {:?}",
        case.name, c.stdout, rust.stdout
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{}: stderr differs\nC: {:?}\nRust: {:?}",
        case.name, c.stderr, rust.stderr
    );
    assert_eq!(
        rust.status, c.status,
        "{}: exit status differs (C: {}, Rust: {})",
        case.name, c.status, rust.status
    );
}

#[test]
fn command_line_error_paths_match() {
    for case in [
        Case {
            name: "no arguments",
            args: &[],
            input: b"",
        },
        Case {
            name: "too few arguments",
            args: &["-", "-", "-"],
            input: b"1 BAG FLT AAA BBB comment\n",
        },
        Case {
            name: "too many arguments",
            args: &["-", "-", "-", "-", "extra"],
            input: b"1 BAG FLT AAA BBB comment\n",
        },
    ] {
        assert_case(&case);
    }
}

#[test]
fn input_boundaries_and_scanf_eof_paths_match() {
    for case in [
        Case {
            name: "empty input",
            args: &["-", "-", "-", "-"],
            input: b"",
        },
        Case {
            name: "whitespace-only input",
            args: &["-", "-", "-", "-"],
            input: b" \t\r\n\x0b\x0c",
        },
        Case {
            name: "EOF before luggage ID",
            args: &["-", "-", "-", "-"],
            input: b"1 ",
        },
        Case {
            name: "EOF before departure",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG FLT ",
        },
        Case {
            name: "EOF before comments",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG FLT AAA BBB",
        },
        Case {
            name: "single item with empty optional comments",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG FLT AAA BBB\n",
        },
        Case {
            name: "empty comments leave newline for next timestamp",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG1 FLT1 AAA BBB\n2 BAG2 FLT2 CCC DDD second\n",
        },
        Case {
            name: "scanf fields span newlines",
            args: &["-", "-", "-", "-"],
            input: b"2\nBAG\nFLT\nAAA\nBBB comment\n",
        },
        Case {
            name: "maximum-width fields and comment",
            args: &["-", "-", "-", "-"],
            input: b"4294967295 ABCD1234 FL1234 XYZ QRS12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        },
        Case {
            name: "field width overflow feeds following scans",
            args: &["-", "-", "-", "-"],
            input: b"1 ABCDEFGHIFLIGHT AAA BBB\n",
        },
        Case {
            name: "comment width ends where next timestamp begins",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG1 FLT1 AAA BBBxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx2 BAG2 FLT2 CCC DDD next\n",
        },
        Case {
            name: "embedded NUL truncates comments as strcpy does",
            args: &["-", "-", "-", "-"],
            input: b"3 BAG FLT AAA BBB before\0after\n",
        },
    ] {
        assert_case(&case);
    }
}

#[test]
fn timestamp_sign_and_overflow_match() {
    for case in [
        Case {
            name: "negative timestamp",
            args: &["-", "-", "-", "-"],
            input: b"-1 BAG FLT AAA BBB\n",
        },
        Case {
            name: "explicit positive timestamp",
            args: &["-", "-", "-", "-"],
            input: b"+7 BAG FLT AAA BBB\n",
        },
        Case {
            name: "one past u32 maximum",
            args: &["-", "-", "-", "-"],
            input: b"4294967296 BAG FLT AAA BBB\n",
        },
        Case {
            name: "signed-long maximum",
            args: &["-", "-", "-", "-"],
            input: b"9223372036854775807 BAG FLT AAA BBB\n",
        },
        Case {
            name: "signed-long minimum",
            args: &["-", "-", "-", "-"],
            input: b"-9223372036854775808 BAG FLT AAA BBB\n",
        },
        Case {
            name: "positive signed-long overflow",
            args: &["-", "-", "-", "-"],
            input: b"18446744073709551616 BAG FLT AAA BBB\n",
        },
        Case {
            name: "negative signed-long overflow",
            args: &["-", "-", "-", "-"],
            input: b"-18446744073709551616 BAG FLT AAA BBB\n",
        },
    ] {
        assert_case(&case);
    }
}

#[test]
fn sorting_and_equal_timestamp_insertion_match() {
    let case = Case {
        name: "out-of-order and equal timestamps",
        args: &["-", "-", "-", "-"],
        input: b"30 BAG3 FLT3 CCC DDD third\n10 BAG1 FLT1 AAA BBB first\n20 BAG2 FLT2 BBB CCC second-a\n20 BAG4 FLT4 DDD EEE second-b\n",
    };
    assert_case(&case);
}

#[test]
fn supersession_branches_match() {
    for case in [
        Case {
            name: "later matching departure supersedes",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG FLT1 AAA BBB old\n2 BAG FLT2 AAA CCC new\n",
        },
        Case {
            name: "equal timestamps preserve input order for supersession",
            args: &["-", "-", "-", "-"],
            input: b"1 BAG FLT1 AAA BBB old\n1 BAG FLT2 AAA CCC new\n",
        },
        Case {
            name: "unrelated luggage is skipped before superseding match",
            args: &["-", "-", "-", "-"],
            input:
                b"1 BAG FLT1 AAA BBB old\n2 OTHER FLT2 AAA CCC unrelated\n3 BAG FLT3 AAA DDD new\n",
        },
        Case {
            name: "first later luggage match with different departure stops search",
            args: &["-", "-", "-", "-"],
            input:
                b"1 BAG FLT1 AAA BBB first\n2 BAG FLT2 CCC DDD second\n3 BAG FLT3 AAA EEE third\n",
        },
        Case {
            name: "no later directive reaches null base case",
            args: &["-", "-", "-", "-"],
            input: b"1 ONLY ONE AAA BBB final\n",
        },
    ] {
        assert_case(&case);
    }
}

#[test]
fn exact_and_wildcard_filters_match() {
    let input = b"1 BAG1 FLT1 AAA BBB one\n2 BAG2 FLT2 CCC DDD two\n";
    for case in [
        Case {
            name: "all exact filters match",
            args: &["BAG1", "FLT1", "AAA", "BBB"],
            input,
        },
        Case {
            name: "hyphen-prefix wildcard",
            args: &["-anything", "-", "-", "-"],
            input,
        },
        Case {
            name: "luggage mismatch",
            args: &["NOPE", "-", "-", "-"],
            input,
        },
        Case {
            name: "flight mismatch",
            args: &["BAG1", "NOPE", "-", "-"],
            input,
        },
        Case {
            name: "departure mismatch",
            args: &["BAG1", "FLT1", "ZZZ", "-"],
            input,
        },
        Case {
            name: "arrival mismatch",
            args: &["BAG1", "FLT1", "AAA", "ZZZ"],
            input,
        },
    ] {
        assert_case(&case);
    }
}
