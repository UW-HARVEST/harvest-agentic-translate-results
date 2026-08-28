use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate has a parent")
        .join("c_src/build/driver")
}

fn run(program: &std::path::Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to launch {}: {error}", program.display()));
    child
        .stdin
        .take()
        .expect("child stdin is piped")
        .write_all(input)
        .expect("failed to write child stdin");
    child.wait_with_output().expect("failed to wait for child")
}

fn assert_equivalent(name: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(std::path::Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust.stdout,
        c.stdout,
        "{name}: stdout differs for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        rust.stderr,
        c.stderr,
        "{name}: stderr differs for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        rust.status,
        c.status,
        "{name}: exit status differs for input {:?}; C={:?}, Rust={:?}",
        String::from_utf8_lossy(input),
        c.status,
        rust.status
    );
}

fn invocation(operation: i32, flags: u32, input: &[u32], reference: &[u32]) -> Vec<u8> {
    let mut encoded = format!("{operation} {flags} {}", input.len());
    for byte in input {
        write!(encoded, " {byte}").unwrap();
    }
    write!(encoded, " {}", reference.len()).unwrap();
    for byte in reference {
        write!(encoded, " {byte}").unwrap();
    }
    encoded.push('\n');
    encoded.into_bytes()
}

fn c_bytes(value: &[u8]) -> Vec<u32> {
    value.iter().map(|&byte| u32::from(byte)).collect()
}

#[test]
fn input_and_validation_errors() {
    let cases: &[(&str, &[u8])] = &[
        ("missing operation", b""),
        ("invalid operation token", b"x"),
        ("missing flags", b"3"),
        ("invalid flags token", b"3 x"),
        ("missing input length", b"3 0"),
        ("invalid input length token", b"3 0 x"),
        ("input too long", b"3 0 1025"),
        ("negative input length", b"3 0 -1"),
        ("missing first input byte", b"3 0 1"),
        ("missing later input byte", b"3 0 2 65"),
        ("missing reference length", b"3 0 0"),
        ("invalid reference length token", b"3 0 0 x"),
        ("reference too long", b"3 0 0 1025"),
        ("missing first reference byte", b"3 0 0 1"),
        ("missing later reference byte", b"3 0 0 2 65"),
    ];

    for (name, input) in cases {
        assert_equivalent(name, input);
    }
}

#[test]
fn operation_zero_validates_tokens() {
    let cases = [
        (
            "matches reference",
            invocation(0, 0, &c_bytes(b"TOKEN\0"), &c_bytes(b"TOKEN\0")),
        ),
        (
            "accepts VALID fallback",
            invocation(0, 0, &c_bytes(b"VALID\0"), &c_bytes(b"other\0")),
        ),
        (
            "accepts OK fallback",
            invocation(0, 0, &c_bytes(b"OK\0"), &c_bytes(b"other\0")),
        ),
        (
            "rejects invalid token",
            invocation(0, 0, &c_bytes(b"BAD\0"), &c_bytes(b"other\0")),
        ),
    ];

    for (name, input) in cases {
        assert_equivalent(name, &input);
    }
}

#[test]
fn operation_one_parses_commands() {
    let mut cases = Vec::new();
    let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
    for (index, command) in commands.iter().enumerate() {
        cases.push((
            format!("command index {index}"),
            invocation(1, 0, &c_bytes(&[*command, b"\0"].concat()), &[]),
        ));
    }
    cases.extend([
        (
            "command followed by a space".to_owned(),
            invocation(1, 0, &c_bytes(b"START argument\0"), &[]),
        ),
        (
            "admin command".to_owned(),
            invocation(1, 0, &c_bytes(b"ADMIN\0"), &[]),
        ),
        (
            "unknown command".to_owned(),
            invocation(1, 0, &c_bytes(b"UNKNOWN\0"), &[]),
        ),
        (
            "command prefix without separator".to_owned(),
            invocation(1, 0, &c_bytes(b"STARTED\0"), &[]),
        ),
        (
            "empty command".to_owned(),
            invocation(1, 0, &c_bytes(b"\0"), &[]),
        ),
    ]);

    for (name, input) in cases {
        assert_equivalent(&name, &input);
    }
}

#[test]
fn operation_two_compares_prefixes() {
    let mut cases = vec![
        (
            "prefix match".to_owned(),
            invocation(2, 0, &c_bytes(b"prefix-tail\0"), &c_bytes(b"prefix\0")),
        ),
        (
            "prefix mismatch".to_owned(),
            invocation(2, 0, &c_bytes(b"other\0"), &c_bytes(b"prefix\0")),
        ),
        (
            "exact match".to_owned(),
            invocation(2, 1, &c_bytes(b"name\0"), &c_bytes(b"name\0")),
        ),
        (
            "exact mismatch".to_owned(),
            invocation(2, 1, &c_bytes(b"name-v3\0"), &c_bytes(b"name\0")),
        ),
    ];
    let suffixes: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
    for (index, suffix) in suffixes.iter().enumerate() {
        cases.push((
            format!("exact variation {index}"),
            invocation(
                2,
                1,
                &c_bytes(&[b"name", *suffix, b"\0"].concat()),
                &c_bytes(b"name\0"),
            ),
        ));
    }

    let long_reference = vec![b'a'; 70];
    let mut truncated_variation = vec![b'a'; 63];
    truncated_variation.push(0);
    let mut terminated_long_reference = long_reference;
    terminated_long_reference.push(0);
    cases.push((
        "variation construction truncates reference".to_owned(),
        invocation(
            2,
            1,
            &c_bytes(&truncated_variation),
            &c_bytes(&terminated_long_reference),
        ),
    ));

    for (name, input) in cases {
        assert_equivalent(&name, &input);
    }
}

#[test]
fn operation_three_finds_delimiters() {
    let cases = [
        ("empty input", invocation(3, 0, &[], &[])),
        ("single item found", invocation(3, 0, &[b':' as u32], &[])),
        (
            "reference delimiter found",
            invocation(3, 0, &c_bytes(b"a|b\0"), &c_bytes(b"|\0")),
        ),
        (
            "nul stops search",
            invocation(3, 0, &c_bytes(b"a\0:b"), &[]),
        ),
        (
            "delimiter absent",
            invocation(3, 0, &c_bytes(b"abc\0"), &c_bytes(b"|\0")),
        ),
        (
            "NONE special case",
            invocation(3, 0, &c_bytes(b"NONE\0"), &c_bytes(b"|\0")),
        ),
        (
            "EMPTY special case",
            invocation(3, 0, &c_bytes(b"EMPTY\0"), &[]),
        ),
        (
            "input and delimiter truncate to char",
            invocation(3, 0, &[300], &[300]),
        ),
    ];

    for (name, input) in cases {
        assert_equivalent(name, &input);
    }
}

#[test]
fn operation_four_matches_patterns() {
    let cases = [
        (
            "case sensitive exact",
            invocation(4, 2, &c_bytes(b"Pattern\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive wildcard both",
            invocation(4, 2, &c_bytes(b"*Pattern*\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive wildcard suffix",
            invocation(4, 2, &c_bytes(b"Pattern*\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive wildcard prefix",
            invocation(4, 2, &c_bytes(b"*Pattern\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive contains",
            invocation(4, 2, &c_bytes(b"xxPatternyy\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive mismatch",
            invocation(4, 2, &c_bytes(b"unrelated\0"), &c_bytes(b"Pattern\0")),
        ),
        (
            "case sensitive empty pattern",
            invocation(4, 2, &c_bytes(b"text\0"), &c_bytes(b"\0")),
        ),
        (
            "case sensitive longer pattern underflows scan bound",
            invocation(4, 2, &c_bytes(b"x\0"), &c_bytes(b"long\0")),
        ),
        (
            "case insensitive exact",
            invocation(4, 0, &c_bytes(b"same\0"), &c_bytes(b"same\0")),
        ),
        (
            "case insensitive prefix",
            invocation(4, 0, &c_bytes(b"same-tail\0"), &c_bytes(b"same\0")),
        ),
        (
            "case insensitive folded match",
            invocation(4, 0, &c_bytes(b"MiXeD\0"), &c_bytes(b"mixed\0")),
        ),
        (
            "case insensitive equal-length mismatch",
            invocation(4, 0, &c_bytes(b"abc\0"), &c_bytes(b"abd\0")),
        ),
        (
            "case insensitive unequal-length mismatch",
            invocation(4, 0, &c_bytes(b"abc\0"), &c_bytes(b"longer\0")),
        ),
    ];

    for (name, input) in cases {
        assert_equivalent(name, &input);
    }
}

#[test]
fn boundary_lengths_and_scanning_behavior() {
    let mut maximum_input = vec![u32::from(b'a'); 1024];
    maximum_input[1023] = u32::from(b'|');
    assert_equivalent(
        "maximum input length",
        &invocation(3, 0, &maximum_input, &[u32::from(b'|')]),
    );

    let mut maximum_reference = vec![0; 1024];
    maximum_reference[0] = u32::from(b':');
    assert_equivalent(
        "maximum reference length",
        &invocation(3, 0, &[u32::from(b':')], &maximum_reference),
    );

    assert_equivalent("scanf spans lines", b"3\n0\n3\n97\n58\n98\n0\n");
    assert_equivalent("trailing input is ignored", b"3 0 1 58 0 trailing garbage");
    assert_equivalent("negative unsigned values wrap", b"3 -1 1 -1 1 -1");
    assert_equivalent("signed operation wraps to i32", b"4294967297 0 1 0 0");
    assert_equivalent(
        "unsigned flag wraps to u32",
        b"2 4294967297 5 110 97 109 101 0 5 110 97 109 101 0",
    );
    assert_equivalent("unsigned bytes wrap to char", b"3 0 1 4294967297 1 1");
    assert_equivalent(
        "size conversion overflow saturates",
        b"3 0 184467440737095516160",
    );
    assert_equivalent("leading plus is accepted", b"+3 +0 +1 +58 +0");
}

#[test]
fn invalid_operations_return_minus_three() {
    assert_equivalent("negative operation", &invocation(-1, 0, &[], &[]));
    assert_equivalent("operation above switch", &invocation(5, 0, &[], &[]));
}
