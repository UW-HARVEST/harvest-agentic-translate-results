use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/driver")
}

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("stdin pipe was not created")
        .write_all(input)
        .expect("failed to write process input");

    child.wait_with_output().expect("failed to collect output")
}

fn assert_same(case: &str, input: &[u8]) {
    let expected = run(&c_binary(), input);
    let actual = run(&rust_binary(), input);

    assert_eq!(
        actual.stdout, expected.stdout,
        "{case}: stdout differs for input {input:?}"
    );
    assert_eq!(
        actual.stderr, expected.stderr,
        "{case}: stderr differs for input {input:?}"
    );
    assert_eq!(
        actual.status, expected.status,
        "{case}: exit status differs for input {input:?}"
    );
}

#[test]
fn input_reading_and_top_level_errors_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("missing parameter", b"0\n"),
        ("missing decision string", b"0\n0\n"),
        ("empty decision string", b"0\n0\n\n"),
        ("single decision", b"0\n0\ny\n"),
        ("two decisions", b"1\n0\nyy\n"),
        ("invalid operation", b"99\n0\ny\n"),
        ("decision without trailing newline", b"0\n0\nyyy"),
        ("invalid boolean characters", b"0\n0\ny?Y\n"),
        ("atoi stops at non-digit", b"garbage\n0\nyyy\n"),
        ("atoi accepts whitespace and signs", b"  +1suffix\n  -0junk\nyyy\n"),
        ("atoi positive overflow", b"999999999999999999999999999\n0\nyyy\n"),
        ("atoi negative overflow", b"-999999999999999999999999999\n0\nyyy\n"),
        ("nul terminates operation", b"1\0ignored\n0\nyyy\n"),
        ("leading nul makes decision empty", b"0\n0\n\0ignored\n"),
        ("nul terminates decisions", b"0\n0\ny\0yy\n"),
        ("carriage return remains in decision", b"3\n0\ny\r\n"),
    ];

    for (case, input) in cases {
        assert_same(case, input);
    }
}

#[test]
fn every_permission_combination_matches() {
    for bits in 0_u8..8 {
        let decisions = [
            if bits & 4 != 0 { b'y' } else { b'n' },
            if bits & 2 != 0 { b'y' } else { b'n' },
            if bits & 1 != 0 { b'y' } else { b'n' },
        ];
        let mut input = b"0\n0\n".to_vec();
        input.extend_from_slice(&decisions);
        input.push(b'\n');
        assert_same(&format!("permission combination {bits:03b}"), &input);
    }
}

#[test]
fn every_logic_operator_and_boolean_combination_matches() {
    for logic_op in 0_u8..=3 {
        for bits in 0_u8..8 {
            let decisions = [
                if bits & 4 != 0 { b'y' } else { b'n' },
                if bits & 2 != 0 { b'y' } else { b'n' },
                if bits & 1 != 0 { b'y' } else { b'n' },
            ];
            let mut input = format!("1\n{logic_op}\n").into_bytes();
            input.extend_from_slice(&decisions);
            input.push(b'\n');
            assert_same(
                &format!("logic operator {logic_op}, combination {bits:03b}"),
                &input,
            );
        }
    }

    assert_same("invalid logic operator", b"1\n4\nyyy\n");
}

#[test]
fn every_flag_configuration_branch_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("all false", b"2\n0\nnnn\n"),
        ("all true", b"2\n0\nyyy\n"),
        ("exactly one true", b"2\n0\nnynnn\n"),
        ("exactly one false", b"2\n0\nyynyy\n"),
        ("alternating", b"2\n0\nynyn\n"),
        ("three consecutive true", b"2\n0\nnyyynnn\n"),
        ("true count fallback", b"2\n0\nyynnyn\n"),
    ];

    for (case, input) in cases {
        assert_same(case, input);
    }

    let mut first_32_true = b"2\n0\n".to_vec();
    first_32_true.extend(std::iter::repeat_n(b'y', 32));
    first_32_true.extend(std::iter::repeat_n(b'n', 991));
    first_32_true.push(b'\n');
    assert_same("1023 decisions capped at first 32", &first_32_true);

    let mut first_32_false = b"2\n0\n".to_vec();
    first_32_false.extend(std::iter::repeat_n(b'n', 32));
    first_32_false.extend(std::iter::repeat_n(b'y', 991));
    first_32_false.push(b'\n');
    assert_same("decisions after first 32 are ignored", &first_32_false);
}

#[test]
fn every_reachable_sequence_validation_branch_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("sequence must start true", b"3\n0\nn\n"),
        ("sequence must end false", b"3\n0\nyy\n"),
        ("four consecutive values", b"3\n0\nyyyyn\n"),
        ("short all same", b"3\n0\ny\n"),
        ("short all different", b"3\n0\nyn\n"),
        ("short transition fallback", b"3\n0\nyyn\n"),
        ("medium few transitions", b"3\n0\nyyynnn\n"),
        ("medium many transitions", b"3\n0\nynyn\n"),
        ("medium transition fallback", b"3\n0\nyyynn\n"),
        ("medium upper boundary", b"3\n0\nynynynynyn\n"),
        ("long many transitions", b"3\n0\nynynynynynn\n"),
        ("long transition fallback", b"3\n0\nyyynnyyynnn\n"),
    ];

    for (case, input) in cases {
        assert_same(case, input);
    }

    let mut maximum = b"3\n0\nyy".to_vec();
    for index in 0..1021 {
        maximum.push(if index % 2 == 0 { b'n' } else { b'y' });
    }
    maximum.push(b'\n');
    assert_same("maximum 1023-byte sequence", &maximum);
}
