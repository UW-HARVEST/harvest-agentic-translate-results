//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses and compare stdout (byte for byte), stderr (byte for byte)
//! and the exit status (both exit code *and* terminating signal).
//!
//! The Rust code is never used as a library here — only the built binary is
//! driven, the same way a shell would drive it, because that is how the two
//! programs are compared.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary. Cargo hands this to integration tests directly,
/// so it is always the binary built for the current profile.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake on first use if it is absent.
/// Only `c_src/build/` is ever touched; the C sources are never modified.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");

        // cmake generators put the executable in one of these places.
        let candidates = [
            build.join("driver"),
            build.join("Release").join("driver"),
            build.join("Debug").join("driver"),
        ];
        if let Some(found) = candidates.iter().find(|p| p.is_file()) {
            return found.clone();
        }

        std::fs::create_dir_all(&build).expect("cannot create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to spawn cmake (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to spawn cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        candidates
            .iter()
            .find(|p| p.is_file())
            .cloned()
            .expect("cmake reported success but no `driver` executable was produced")
    })
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl Outcome {
    fn status_text(&self) -> String {
        match (self.code, self.signal) {
            (Some(c), _) => format!("exited with code {c}"),
            (None, Some(s)) => format!("killed by signal {s}"),
            (None, None) => "unknown termination".to_string(),
        }
    }
}

/// Run `bin` with `args`, feeding it `stdin_bytes`, and capture everything.
fn run(bin: &Path, args: &[OsString], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program may die (SIGSEGV) before draining stdin; a broken pipe
        // here is expected and must not fail the test.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?} (not utf-8)"),
    }
}

fn args_of(raw: &[&[u8]]) -> Vec<OsString> {
    raw.iter()
        .map(|b| OsStr::from_bytes(b).to_os_string())
        .collect()
}

/// The core assertion: identical stdout, stderr and exit status.
fn assert_same_raw(label: &str, raw_args: &[&[u8]], stdin_bytes: &[u8]) {
    let args = args_of(raw_args);
    let c = run(c_bin(), &args, stdin_bytes);
    let r = run(&rust_bin(), &args, stdin_bytes);

    let pretty: Vec<String> = raw_args.iter().map(|a| show(a)).collect();
    let ctx = format!("case `{label}` with args [{}]", pretty.join(", "));

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {ctx}\n  C:    {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {ctx}\n  C:    {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {ctx}\n  C:    {}\n  Rust: {}",
        c.status_text(),
        r.status_text()
    );
}

fn assert_same(label: &str, args: &[&str]) {
    let raw: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
    assert_same_raw(label, &raw, b"");
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = c_bin();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    let r = rust_bin();
    assert!(r.is_file(), "Rust binary missing at {}", r.display());

    // A trivial invocation must succeed for both.
    for bin in [c, r.as_path()] {
        let o = run(bin, &args_of(&[b"1", b"2"]), b"");
        assert_eq!(o.code, Some(0), "{} did not exit 0", bin.display());
        assert_eq!(o.stdout, b"3\n", "{} printed {}", bin.display(), show(&o.stdout));
    }
}

// ---------------------------------------------------------------------------
// Phase B: the value paths.
//
// `main` has no conditionals, so the branching all lives in the two `atoi`
// calls (glibc strtol) plus the signed addition. Each group below is a
// distinct behaviour class of that code.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_small_numbers() {
    assert_same("positive pair", &["1", "2"]);
    assert_same("both zero", &["0", "0"]);
    assert_same("both negative", &["-1", "-2"]);
    assert_same("mixed signs", &["10", "-4"]);
    assert_same("single item then zero", &["7", "0"]);
}

#[test]
fn signed_addition_overflow_wraps() {
    // t.a + t.b overflows `int`; must wrap exactly as the C build does.
    assert_same("INT_MAX + 1", &["2147483647", "1"]);
    assert_same("INT_MAX + INT_MAX", &["2147483647", "2147483647"]);
    assert_same("INT_MIN + -1", &["-2147483648", "-1"]);
    assert_same("INT_MIN + INT_MIN", &["-2147483648", "-2147483648"]);
    assert_same("INT_MAX + INT_MIN", &["2147483647", "-2147483648"]);
}

#[test]
fn int_boundaries_exactly() {
    assert_same("INT_MAX and 0", &["2147483647", "0"]);
    assert_same("INT_MIN and 0", &["-2147483648", "0"]);
}

#[test]
fn atoi_rejects_non_numeric_input_silently() {
    // No diagnostic, no non-zero exit: atoi just yields 0.
    assert_same("letters", &["abc", "def"]);
    assert_same("punctuation", &["!!", "??"]);
    assert_same("hex looking", &["0x10", "0x10"]);
    assert_same("exponent notation", &["1e5", "2E5"]);
    assert_same("double sign", &["--5", "-+5"]);
    assert_same("leading dot", &[".5", "5."]);
    assert_same("empty strings", &["", ""]);
    assert_same("sign only", &["+", "-"]);
    assert_same("whitespace only", &[" ", " "]);
}

#[test]
fn atoi_skips_leading_whitespace_and_stops_at_first_non_digit() {
    assert_same("leading spaces", &["  12", "  -34"]);
    assert_same("trailing garbage", &["12abc", "34xyz"]);
    assert_same("explicit plus", &["+5", "-5"]);
    assert_same("tab and newline prefix", &["\t\n5", "3"]);
    assert_same("vertical tab and form feed", &["\u{0b}5", "\u{0c}6"]);
    assert_same("carriage returns", &["\r5", "5\r"]);
    assert_same("space between sign and digits", &[" +  5", "5"]);
    assert_same("negative zero", &["   -0", "-0"]);
    assert_same("leading zeros are decimal not octal", &["010", "010"]);
    assert_same("many leading zeros", &["000000000000000000005", "3"]);
    assert_same("internal space", &["1 2", "3"]);
}

#[test]
fn atoi_truncates_long_to_int() {
    assert_same("fits in long not int", &["99999999999", "1"]);
    assert_same("2^32", &["4294967296", "0"]);
    assert_same("2^32 + 1", &["4294967297", "0"]);
    assert_same("INT_MAX + 1 as text", &["2147483648", "0"]);
    assert_same("INT_MIN - 1 as text", &["-2147483649", "0"]);
}

#[test]
fn atoi_saturates_at_long_boundaries_then_truncates() {
    assert_same("LONG_MAX", &["9223372036854775807", "0"]);
    assert_same("LONG_MAX + 1", &["9223372036854775808", "0"]);
    assert_same("LONG_MIN", &["-9223372036854775808", "0"]);
    assert_same("LONG_MIN - 1", &["-9223372036854775809", "0"]);
    assert_same("twenty nines", &["99999999999999999999", "0"]);
    assert_same("twenty nines negative", &["-99999999999999999999", "0"]);
    assert_same("both saturating", &["99999999999999999999", "99999999999999999999"]);
}

#[test]
fn atoi_handles_very_long_digit_strings() {
    let long_nines = "9".repeat(400);
    let padded = format!("{}5", "0".repeat(400));
    assert_same("400 nines", &[&long_nines, "1"]);
    assert_same("400 zeros then 5", &[&padded, "1"]);
    assert_same("400 nines negative", &[&format!("-{long_nines}"), "1"]);
}

#[test]
fn arguments_are_raw_bytes_not_utf8() {
    // argv is a byte string in C; invalid UTF-8 must not change behaviour.
    assert_same_raw("invalid utf-8", &[b"\xff\xfe", b"\x80"], b"");
    assert_same_raw("digit then invalid byte", &[b"5\xff", b"\xff5"], b"");
    assert_same_raw("embedded high bytes", &[b"\xc3\x28" , b"12\xf0"], b"");
}

// ---------------------------------------------------------------------------
// Phase C: paths the happy path never reaches.
// ---------------------------------------------------------------------------

#[test]
fn extra_arguments_are_ignored() {
    assert_same("three args", &["1", "2", "3"]);
    assert_same("five args", &["1", "2", "3", "4", "5"]);
    assert_same("extra args are garbage", &["4", "5", "not-a-number"]);
}

#[test]
fn missing_second_argument_dereferences_null() {
    // `atoi(argv[2])` reads through the NULL terminator of argv:
    // the process dies from SIGSEGV with no output on either stream.
    assert_same("only argv[1]", &["7"]);
}

#[test]
fn missing_both_arguments_dereferences_null() {
    // `atoi(argv[1])` is reached first and faults there.
    assert_same("no arguments at all", &[]);
}

#[test]
fn null_deref_cases_really_die_by_signal_with_no_output() {
    // Pin down *what* the shared behaviour is, so an accidental
    // "both exit 0 printing nothing" cannot pass the comparison above.
    for args in [vec![], vec!["7"]] {
        let raw: Vec<&[u8]> = args.iter().map(|s: &&str| s.as_bytes()).collect();
        let osargs = args_of(&raw);
        for bin in [c_bin().to_path_buf(), rust_bin()] {
            let o = run(&bin, &osargs, b"");
            assert_eq!(
                o.code,
                None,
                "{} with {args:?} should not exit normally, got {}",
                bin.display(),
                o.status_text()
            );
            assert_eq!(
                o.signal,
                Some(11),
                "{} with {args:?} should die from SIGSEGV, got {}",
                bin.display(),
                o.status_text()
            );
            assert!(o.stdout.is_empty(), "{} wrote stdout", bin.display());
            assert!(o.stderr.is_empty(), "{} wrote stderr", bin.display());
        }
    }
}

#[test]
fn stdin_is_never_read() {
    // The C program takes its input purely from argv; whatever is on stdin
    // must not affect the output of either program.
    assert_same_raw("stdin has numbers", &[b"1", b"2"], b"999 999\n");
    assert_same_raw("stdin is large", &[b"1", b"2"], &vec![b'x'; 64 * 1024]);
    assert_same_raw("stdin is binary", &[b"-5", b"5"], b"\x00\x01\x02\xff");
}

#[test]
fn stdout_is_exactly_one_line_with_trailing_newline() {
    // printf("%d\n", ...) — no padding, no extra whitespace.
    for (args, expected) in [
        (["1", "2"], "3\n"),
        (["-1", "-2"], "-3\n"),
        (["0", "0"], "0\n"),
    ] {
        let osargs = args_of(&[args[0].as_bytes(), args[1].as_bytes()]);
        for bin in [c_bin().to_path_buf(), rust_bin()] {
            let o = run(&bin, &osargs, b"");
            assert_eq!(
                o.stdout,
                expected.as_bytes(),
                "{} with {args:?} printed {}",
                bin.display(),
                show(&o.stdout)
            );
            assert!(o.stderr.is_empty(), "{} wrote to stderr", bin.display());
            assert_eq!(o.code, Some(0), "{} exit status", bin.display());
        }
    }
}

#[test]
fn container_of_recovers_the_same_struct_for_both_members() {
    // find_container_of_a(&t.a) and find_container_of_b(&t.b) must both
    // resolve back to `t`, so the printed value is a + b and nothing else
    // (a wrong offset would read the neighbouring member or padding).
    for a in [0i64, 1, -1, 12345, -12345, 2147483647, -2147483648] {
        for b in [0i64, 1, -1, 777, -777, 2147483647, -2147483648] {
            let sa = a.to_string();
            let sb = b.to_string();
            let expected = format!("{}\n", (a as i32).wrapping_add(b as i32));
            let osargs = args_of(&[sa.as_bytes(), sb.as_bytes()]);

            let c = run(c_bin(), &osargs, b"");
            let r = run(&rust_bin(), &osargs, b"");
            assert_eq!(
                c.stdout,
                expected.as_bytes(),
                "C printed {} for {a} + {b}",
                show(&c.stdout)
            );
            assert_eq!(c.stdout, r.stdout, "mismatch for {a} + {b}");
            assert_eq!(c.stderr, r.stderr, "stderr mismatch for {a} + {b}");
            assert_eq!((c.code, c.signal), (r.code, r.signal), "status for {a} + {b}");
        }
    }
}

#[test]
fn exhaustive_sweep_over_representative_inputs() {
    // Cross product of one representative string per behaviour class, in both
    // argument positions, so ordering effects cannot hide.
    const CLASSES: &[&str] = &[
        "0",
        "1",
        "-1",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "9223372036854775807",
        "-9223372036854775808",
        "99999999999999999999",
        "-99999999999999999999",
        "99999999999",
        "",
        " ",
        "+",
        "-",
        "abc",
        "12abc",
        "  12",
        "\t7",
        "0x10",
        "010",
        "1e5",
        "--5",
        ".5",
    ];

    for x in CLASSES {
        for y in CLASSES {
            let osargs = args_of(&[x.as_bytes(), y.as_bytes()]);
            let c = run(c_bin(), &osargs, b"");
            let r = run(&rust_bin(), &osargs, b"");
            assert_eq!(
                c.stdout,
                r.stdout,
                "stdout differs for [{}, {}]\n  C:    {}\n  Rust: {}",
                show(x.as_bytes()),
                show(y.as_bytes()),
                show(&c.stdout),
                show(&r.stdout)
            );
            assert_eq!(
                c.stderr,
                r.stderr,
                "stderr differs for [{}, {}]",
                show(x.as_bytes()),
                show(y.as_bytes())
            );
            assert_eq!(
                (c.code, c.signal),
                (r.code, r.signal),
                "exit status differs for [{}, {}]\n  C:    {}\n  Rust: {}",
                show(x.as_bytes()),
                show(y.as_bytes()),
                c.status_text(),
                r.status_text()
            );
        }
    }
}
