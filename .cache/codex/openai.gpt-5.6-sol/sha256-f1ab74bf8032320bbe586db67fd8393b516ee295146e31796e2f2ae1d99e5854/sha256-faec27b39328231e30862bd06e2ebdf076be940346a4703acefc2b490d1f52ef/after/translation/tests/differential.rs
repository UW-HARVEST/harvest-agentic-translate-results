use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::{env, fs};

fn run_with_input(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

#[test]
fn matches_c_for_all_input_classes() {
    let c_driver = c_driver();
    assert!(
        fs::metadata(&c_driver)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false),
        "build the C reference first: {}",
        c_driver.display()
    );

    let rust_driver = Path::new(env!("CARGO_BIN_EXE_driver"));
    let mut cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty input", b"".to_vec()),
        ("whitespace-only input", b" \t\n\x0b\x0c\r".to_vec()),
        ("invalid first byte", b"x".to_vec()),
        ("plus sign only", b"+".to_vec()),
        ("minus sign only", b"-".to_vec()),
        ("sign followed by invalid byte", b"-x".to_vec()),
        ("sign followed by whitespace", b"+ 1".to_vec()),
        ("whitespace before sign", b"\n\t-8".to_vec()),
        ("NUL before any digit", b"\0123".to_vec()),
        ("single zero", b"0".to_vec()),
        ("single item", b"1".to_vec()),
        ("negative item", b"-1".to_vec()),
        ("explicit plus sign", b"+42".to_vec()),
        ("all leading whitespace forms", b" \t\n\x0b\x0c\r7".to_vec()),
        ("scanf reads across newlines", b"\n\n123\n".to_vec()),
        ("decimal with leading zeroes", b"000000008".to_vec()),
        ("trailing invalid input", b"12xyz".to_vec()),
        ("NUL after digits", b"12\0xyz".to_vec()),
        ("second item ignored", b"4 999".to_vec()),
        ("maximum int", b"2147483647".to_vec()),
        ("minimum int", b"-2147483648".to_vec()),
        ("one above maximum int", b"2147483648".to_vec()),
        ("one below minimum int", b"-2147483649".to_vec()),
        ("maximum long", b"9223372036854775807".to_vec()),
        ("minimum long", b"-9223372036854775808".to_vec()),
        ("positive long overflow", b"9223372036854775808".to_vec()),
        ("negative long overflow", b"-9223372036854775809".to_vec()),
        ("multiplication upper boundary", b"1073741823".to_vec()),
        ("multiplication upper overflow", b"1073741824".to_vec()),
        ("multiplication lower boundary", b"-1073741824".to_vec()),
        ("multiplication lower overflow", b"-1073741825".to_vec()),
        ("addition upper boundary", b"1073741673".to_vec()),
        ("addition upper overflow", b"1073741674".to_vec()),
    ];
    cases.push(("very long positive integer", vec![b'9'; 512]));
    cases.push((
        "very long negative integer",
        [b"-".as_slice(), vec![b'9'; 512].as_slice()].concat(),
    ));

    for (name, input) in cases {
        let expected = run_with_input(&c_driver, &input);
        let actual = run_with_input(rust_driver, &input);

        assert_eq!(
            actual.status, expected.status,
            "{name}: exit status differs for input {input:?}"
        );
        assert_eq!(
            actual.stdout, expected.stdout,
            "{name}: stdout differs for input {input:?}"
        );
        assert_eq!(
            actual.stderr, expected.stderr,
            "{name}: stderr differs for input {input:?}"
        );
    }
}
