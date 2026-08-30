use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must have a parent directory")
        .join("c_src/build/driver")
}

fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin must be piped")
        .write_all(input)
        .expect("failed to write child stdin");
    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches(label: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{label}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{label}: stderr differs");
    assert_eq!(rust.status, c.status, "{label}: exit status differs");
}

#[test]
fn scanf_input_classes_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("single item", b"5\n"),
        ("malformed first item", b"not-an-integer\n"),
        ("malformed after selector", b"1 not-a-float\n"),
        (
            "scanf crosses newlines",
            b"1\n0.125 -0.25\n1.5 0\n8 256\n511 2.0\n0.5 1.0\n6\n",
        ),
        (
            "trailing input is ignored",
            b"0 0.25 0.5 0.75 0 0 0 0 0 0 0 0 trailing bytes\n",
        ),
    ];

    for &(label, input) in cases {
        assert_matches(label, input);
    }

    let tokens = [
        "1", "0.125", "-0.25", "1.5", "0", "8", "256", "511", "2.0", "0.5", "1.0", "6",
    ];
    for consumed in 0..=tokens.len() {
        let input = tokens[..consumed].join(" ");
        assert_matches(
            &format!("EOF after {consumed} conversions"),
            input.as_bytes(),
        );
    }
    for malformed_at in 0..tokens.len() {
        let mut malformed = tokens;
        malformed[malformed_at] = "invalid";
        let input = malformed.join(" ");
        assert_matches(
            &format!("malformed conversion {}", malformed_at + 1),
            input.as_bytes(),
        );
    }
}

#[test]
fn every_selector_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("noise3", b"0 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n"),
        ("seeded noise3", b"1 0.125 0.25 0.5 8 16 32 37 2 0.5 1 4\n"),
        ("ridge noise", b"2 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n"),
        ("fbm noise", b"3 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n"),
        ("turbulence noise", b"4 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n"),
        (
            "non-power-of-two wrapping",
            b"5 0.125 0.25 0.5 3 5 7 37 2 0.5 1 4\n",
        ),
        (
            "selector below range",
            b"-1 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n",
        ),
        (
            "selector above range",
            b"6 0.125 0.25 0.5 0 0 0 0 2 0.5 1 4\n",
        ),
    ];

    for &(label, input) in cases {
        assert_matches(label, input);
    }
}

#[test]
fn floor_wrap_and_seed_boundaries_match() {
    let cases: &[(&str, &[u8])] = &[
        (
            "negative non-integer coordinates",
            b"0 -0.125 -1.25 -255.5 0 0 0 0 0 0 0 0\n",
        ),
        (
            "negative integer coordinates",
            b"0 -1 -2 -256 0 0 0 0 0 0 0 0\n",
        ),
        (
            "minimum power-of-two wrap",
            b"1 0.125 1.25 2.5 1 1 1 255 0 0 0 0\n",
        ),
        (
            "maximum supported wrap",
            b"1 255.75 255.5 255.25 256 256 256 255 0 0 0 0\n",
        ),
        (
            "seed truncates at 256",
            b"1 0.125 0.25 0.5 0 0 0 256 0 0 0 0\n",
        ),
        (
            "negative seed truncates",
            b"1 0.125 0.25 0.5 0 0 0 -1 0 0 0 0\n",
        ),
        (
            "nonpow2 zero wraps use 256",
            b"5 255.75 255.5 255.25 0 0 0 255 0 0 0 0\n",
        ),
        (
            "nonpow2 maximum wraps",
            b"5 255.75 255.5 255.25 256 256 256 255 0 0 0 0\n",
        ),
        (
            "nonpow2 negative-coordinate corrections",
            b"5 -0.125 -1.25 -6.5 3 5 7 511 0 0 0 0\n",
        ),
        (
            "nonpow2 upper wrap boundaries",
            b"5 2.75 4.75 6.75 3 5 7 37 0 0 0 0\n",
        ),
    ];

    for &(label, input) in cases {
        assert_matches(label, input);
    }
}

#[test]
fn octave_loop_boundaries_match() {
    let cases: &[(&str, &[u8])] = &[
        (
            "ridge negative octaves",
            b"2 0.125 0.25 0.5 0 0 0 0 2 0.5 1 -1\n",
        ),
        (
            "ridge zero octaves",
            b"2 0.125 0.25 0.5 0 0 0 0 2 0.5 1 0\n",
        ),
        (
            "ridge single octave",
            b"2 0.125 0.25 0.5 0 0 0 0 2 0.5 1 1\n",
        ),
        (
            "ridge multiple octaves",
            b"2 -0.125 0.25 -0.5 0 0 0 0 2 0.5 1 6\n",
        ),
        ("fbm zero octaves", b"3 0.125 0.25 0.5 0 0 0 0 2 0.5 0 0\n"),
        ("fbm single octave", b"3 0.125 0.25 0.5 0 0 0 0 2 0.5 0 1\n"),
        (
            "fbm multiple octaves",
            b"3 -0.125 0.25 -0.5 0 0 0 0 2 0.5 0 6\n",
        ),
        (
            "turbulence zero octaves",
            b"4 0.125 0.25 0.5 0 0 0 0 2 0.5 0 0\n",
        ),
        (
            "turbulence single octave",
            b"4 0.125 0.25 0.5 0 0 0 0 2 0.5 0 1\n",
        ),
        (
            "turbulence multiple octaves",
            b"4 -0.125 0.25 -0.5 0 0 0 0 2 0.5 0 6\n",
        ),
        (
            "octave seed wraps after 255",
            b"3 0.125 0.25 0.5 0 0 0 0 1 0.25 0 256\n",
        ),
    ];

    for &(label, input) in cases {
        assert_matches(label, input);
    }
}
