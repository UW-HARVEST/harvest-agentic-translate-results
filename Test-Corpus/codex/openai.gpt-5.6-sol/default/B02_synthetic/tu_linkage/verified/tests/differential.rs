use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run_driver(executable: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(executable)
        .arg0("driver")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", executable.display()));

    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(stdin)
        .expect("write stdin");
    child.wait_with_output().expect("wait for driver")
}

fn assert_case(name: &str, args: &[&str], stdin: &[u8]) {
    let c = run_driver(&c_driver(), args, stdin);
    let rust = run_driver(Path::new(env!("CARGO_BIN_EXE_driver")), args, stdin);

    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
    assert_eq!(rust.status, c.status, "{name}: exit status differs");
}

fn assert_owned_case(name: &str, args: &[String]) {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    assert_case(name, &borrowed, b"");
}

#[test]
fn cli_and_argument_parsing() {
    let cases: &[(&str, &[&str])] = &[
        ("empty input", &[]),
        ("single item", &["10"]),
        ("invalid argument", &["not-a-number"]),
        ("invalid before valid", &["bad", "10"]),
        ("empty argument parses as zero", &[""]),
        ("help", &["--help"]),
        ("help returns immediately", &["bad", "--help", "10"]),
        ("maximum i32", &["0", "2147483647"]),
        ("minimum i32", &["0", "-2147483648"]),
        (
            "long maximum truncates to int",
            &["0", "9223372036854775807"],
        ),
        (
            "long minimum truncates to int",
            &["0", "-9223372036854775808"],
        ),
        (
            "positive long overflow saturates",
            &["0", "999999999999999999999999"],
        ),
        (
            "negative long overflow saturates",
            &["0", "-999999999999999999999999"],
        ),
        ("leading C whitespace", &["\u{b}+10"]),
        ("trailing junk", &["10x"]),
        ("maximum opcode stops", &["10", "11"]),
    ];

    for &(name, args) in cases {
        assert_case(name, args, b"");
    }
}

#[test]
fn stdin_parsing_and_buffer_boundaries() {
    let mut split_at_fgets_limit = vec![b' '; 4094];
    split_at_fgets_limit.extend_from_slice(b"10\n");

    let cases: Vec<(&str, Vec<&str>, Vec<u8>)> = vec![
        ("empty stdin", vec!["--stdin"], vec![]),
        (
            "whitespace-only stdin",
            vec!["--stdin"],
            b" \t\r\n".to_vec(),
        ),
        (
            "stdin tokens cross newlines",
            vec!["--stdin"],
            b"0\n12\t0\r3 1\n".to_vec(),
        ),
        (
            "stdin invalid tokens are ignored",
            vec!["--stdin"],
            b"junk 10x 10\n".to_vec(),
        ),
        (
            "argv precedes stdin",
            vec!["0", "4", "--stdin"],
            b"3\n".to_vec(),
        ),
        (
            "vertical tab is not a stdin delimiter",
            vec!["--stdin"],
            b"0\x0b1 10\n".to_vec(),
        ),
        (
            "embedded nul hides rest of fgets chunk",
            vec!["--stdin"],
            b"0 7\0 0 8\n10\n".to_vec(),
        ),
        (
            "token splits at fgets buffer limit",
            vec!["--stdin"],
            split_at_fgets_limit,
        ),
    ];

    for (name, args, stdin) in cases {
        assert_case(name, &args, &stdin);
    }
}

#[test]
fn every_engine_error_return() {
    let cases: &[(&str, &[&str])] = &[
        ("push missing immediate rc1", &["0"]),
        ("add empty stack rc2", &["1"]),
        ("add one item consumes it rc2", &["0", "7", "1"]),
        ("multiply empty stack rc3", &["2"]),
        ("multiply one item consumes it rc3", &["0", "7", "2"]),
        ("drop empty stack rc4", &["4"]),
        ("jump missing offset rc5", &["6"]),
        ("jump missing condition rc6", &["6", "0"]),
        ("jump past end rc7", &["0", "1", "6", "1"]),
        (
            "negative jump becomes too large rc7",
            &["0", "1", "6", "-1"],
        ),
        ("repeat missing count rc8", &["7"]),
        ("repeat missing instruction rc9", &["7", "1"]),
        ("stream missing count rc10", &["9"]),
        ("stream negative count rc11", &["9", "-1"]),
        ("stream count exceeds stack rc11", &["9", "1"]),
        ("unknown opcode rc99", &["11"]),
        ("negative unknown opcode rc99", &["-1"]),
    ];

    for &(name, args) in cases {
        assert_case(name, args, b"");
    }
}

#[test]
fn successful_opcode_and_control_flow_branches() {
    let cases: &[(&str, &[&str])] = &[
        ("push", &["0", "42"]),
        ("add", &["0", "2", "0", "3", "1"]),
        ("add wraps", &["0", "2147483647", "0", "1", "1"]),
        ("multiply", &["0", "-3", "0", "7", "2"]),
        (
            "multiply wraps",
            &["0", "2147483647", "0", "2147483647", "2"],
        ),
        ("duplicate empty stack default", &["3"]),
        ("duplicate nonempty stack", &["0", "4", "3"]),
        ("drop", &["0", "4", "4"]),
        ("classify bucket zero", &["0", "-1", "5"]),
        ("classify bucket one and default", &["0", "0", "5"]),
        ("classify bucket two", &["0", "7", "5"]),
        ("classify bucket three", &["0", "-20", "5"]),
        ("classify bucket four", &["0", "2", "5"]),
        ("jump false", &["0", "0", "6", "1", "3"]),
        ("jump true with zero offset", &["0", "1", "6", "0", "3"]),
        (
            "jump true skips instruction",
            &["0", "1", "6", "1", "11", "3"],
        ),
        ("repeat negative count", &["7", "-1", "3"]),
        ("repeat zero count", &["7", "0", "3"]),
        ("repeat successful instruction", &["7", "3", "3"]),
        ("repeat instruction error", &["7", "2", "0", "3"]),
        ("alternate classify", &["0", "8", "8"]),
        ("stream zero items", &["9", "0"]),
        ("stop returns before unknown opcode", &["10", "11"]),
    ];

    for &(name, args) in cases {
        assert_case(name, args, b"");
    }
}

#[test]
fn stream_processing_and_partial_second_pop() {
    let cases: &[(&str, &[&str])] = &[
        (
            "one stream item with partial second pop",
            &["0", "5", "9", "1"],
        ),
        (
            "two stream items with no second-pop values",
            &["0", "-2", "0", "7", "9", "2"],
        ),
        (
            "stream second pop overwrites temporary values",
            &["0", "1", "0", "2", "0", "3", "0", "4", "9", "2"],
        ),
        (
            "stream varied target branches",
            &[
                "0", "-5", "0", "0", "0", "1", "0", "3", "0", "5", "0", "7", "0", "9", "0", "10",
                "9", "4",
            ],
        ),
    ];

    for &(name, args) in cases {
        assert_case(name, args, b"");
    }
}

#[test]
fn classifier_and_stream_branch_sweeps() {
    let values = (-32..=32).chain([i32::MIN, i32::MAX]);

    for value in values {
        let value = value.to_string();
        assert_owned_case(
            &format!("stateful classifier sweep at {value}"),
            &[
                "0".into(),
                value.clone(),
                "5".into(),
                "4".into(),
                "5".into(),
                "4".into(),
                "8".into(),
            ],
        );
        assert_owned_case(
            &format!("stream sweep at {value}"),
            &["0".into(), value, "9".into(), "1".into()],
        );
    }
}

#[test]
fn vectors_grow_beyond_initial_capacity() {
    let mut args = Vec::new();
    for value in 0..40 {
        args.push("0".into());
        args.push(value.to_string());
    }
    args.push("9".into());
    args.push("20".into());
    assert_owned_case("code stack and trace vectors grow", &args);
}
