use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Case {
    name: &'static str,
    input: &'static [u8],
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("C driver is missing; build it with cmake before running tests")
}

fn run(path: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", path.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to wait for child")
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases = [
        Case {
            name: "empty input",
            input: b"",
        },
        Case {
            name: "whitespace followed by eof",
            input: b" \t\r\n",
        },
        Case {
            name: "invalid token",
            input: b"not-an-integer\n",
        },
        Case {
            name: "sign without digits",
            input: b"+\n",
        },
        Case {
            name: "zero",
            input: b"0\n",
        },
        Case {
            name: "single positive item",
            input: b"1\n",
        },
        Case {
            name: "single negative item",
            input: b"-1\n",
        },
        Case {
            name: "maximum int",
            input: b"2147483647\n",
        },
        Case {
            name: "minimum int",
            input: b"-2147483648\n",
        },
        Case {
            name: "positive int overflow",
            input: b"2147483648\n",
        },
        Case {
            name: "negative int overflow",
            input: b"-2147483649\n",
        },
        Case {
            name: "positive overflow truncates to zero",
            input: b"4294967296\n",
        },
        Case {
            name: "negative overflow truncates to zero",
            input: b"-4294967296\n",
        },
        Case {
            name: "value exceeds long",
            input: b"9999999999999999999999999999999999999999\n",
        },
        Case {
            name: "negative value exceeds long",
            input: b"-9999999999999999999999999999999999999999\n",
        },
        Case {
            name: "leading whitespace across newlines",
            input: b"\n\n\t1\n",
        },
        Case {
            name: "numeric prefix",
            input: b"1abc\n",
        },
        Case {
            name: "only first item is read",
            input: b"0 1\n",
        },
        Case {
            name: "no trailing newline",
            input: b"7",
        },
        Case {
            name: "nul before integer",
            input: b"\0 1\n",
        },
    ];

    let c_driver = c_driver();
    let rust_driver = Path::new(env!("CARGO_BIN_EXE_driver"));

    for case in cases {
        let c = run(&c_driver, case.input);
        let rust = run(rust_driver, case.input);

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
            "{}: exit status differs\nC: {:?}\nRust: {:?}",
            case.name, c.status, rust.status
        );
    }
}
