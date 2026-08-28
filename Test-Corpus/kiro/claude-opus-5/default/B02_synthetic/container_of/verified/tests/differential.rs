//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical arguments and require that stdout, stderr and
//! the exit status agree byte for byte / value for value.
//!
//! The Rust code is never linked as a library here. Both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

// ---------------------------------------------------------------------------
// Locating (and if necessary building) the two executables
// ---------------------------------------------------------------------------

/// The Rust binary under test. Cargo builds it for us and hands us the path.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // tests/ live in translation/, whose parent holds c_src/ and translation/.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The C binary, built through the project's own CMake setup if it is missing.
fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join(if cfg!(windows) { "driver.exe" } else { "driver" });

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("could not create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to invoke cmake; is it installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr),
            );

            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to invoke cmake --build");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(
            exe.exists(),
            "the C executable was not produced at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

/// How a process finished, in a form that compares equal only when the two
/// programs terminated in genuinely the same way. A normal exit with code 1
/// and a death by signal 1 are different outcomes and must not be conflated.
#[derive(Debug, PartialEq, Eq)]
enum Termination {
    Exited(i32),
    Signalled(i32),
}

fn termination(output: &Output) -> Termination {
    match output.status.code() {
        Some(code) => Termination::Exited(code),
        None => {
            #[cfg(unix)]
            {
                Termination::Signalled(
                    output
                        .status
                        .signal()
                        .expect("a process with no exit code must have been signalled"),
                )
            }
            #[cfg(not(unix))]
            {
                panic!("process terminated without an exit code on a non-unix target");
            }
        }
    }
}

fn run(program: &Path, args: &[OsString]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()))
}

/// Build an argument vector from raw bytes, so that arguments which are not
/// valid UTF-8 can be passed through untouched, just as a shell would.
fn args_from_bytes(raw: &[&[u8]]) -> Vec<OsString> {
    raw.iter()
        .map(|bytes| {
            #[cfg(unix)]
            {
                OsStr::from_bytes(bytes).to_os_string()
            }
            #[cfg(not(unix))]
            {
                OsString::from(String::from_utf8(bytes.to_vec()).expect("non-UTF-8 argument"))
            }
        })
        .collect()
}

/// The core assertion: same arguments in, same stdout, stderr and termination
/// out. Called by every test below.
fn assert_same(raw_args: &[&[u8]]) {
    let args = args_from_bytes(raw_args);

    let c = run(c_binary(), &args);
    let r = run(rust_binary(), &args);

    let pretty: Vec<String> = raw_args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect();
    let label = format!("argv = [{}]", pretty.join(", "));

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        termination(&c),
        termination(&r),
        "exit status differs for {label}",
    );
}

/// Convenience wrapper for the ordinary two-string-arguments case.
fn assert_same_str(args: &[&str]) {
    let raw: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
    assert_same(&raw);
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(c_binary(), &args_from_bytes(&[b"1", b"2"]));
    let r = run(rust_binary(), &args_from_bytes(&[b"1", b"2"]));
    assert_eq!(c.stdout, b"3\n", "the C program should print 3 for 1 and 2");
    assert_eq!(r.stdout, b"3\n", "the Rust program should print 3 for 1 and 2");
}

// ---------------------------------------------------------------------------
// Phase B: the ordinary paths
// ---------------------------------------------------------------------------

#[test]
fn small_positive_values() {
    assert_same_str(&["1", "2"]);
    assert_same_str(&["7", "0"]);
    assert_same_str(&["0", "7"]);
    assert_same_str(&["123", "456"]);
}

#[test]
fn zero_and_negative_values() {
    assert_same_str(&["0", "0"]);
    assert_same_str(&["-1", "-2"]);
    assert_same_str(&["-5", "5"]);
    assert_same_str(&["5", "-5"]);
    assert_same_str(&["-0", "-0"]);
}

#[test]
fn explicit_plus_sign_is_accepted() {
    assert_same_str(&["+42", "+7"]);
    assert_same_str(&["+0", "-0"]);
}

#[test]
fn leading_whitespace_is_skipped() {
    // atoi() skips every isspace() character, not just the space itself.
    assert_same_str(&["   42", "7"]);
    assert_same_str(&["\t42", "7"]);
    assert_same_str(&["\n42", "7"]);
    assert_same_str(&["\u{0b}42", "7"]);
    assert_same_str(&["\u{0c}42", "7"]);
    assert_same_str(&["\r42", "7"]);
    assert_same_str(&["\t\n\u{0b}\u{0c}\r 17", "0"]);
}

// ---------------------------------------------------------------------------
// Phase B/C: everything atoi() silently tolerates
// ---------------------------------------------------------------------------

#[test]
fn empty_arguments_parse_as_zero() {
    // The "empty input" class: present but empty argument strings.
    assert_same_str(&["", ""]);
    assert_same_str(&["", "5"]);
    assert_same_str(&["5", ""]);
}

#[test]
fn wholly_unparseable_arguments_parse_as_zero() {
    assert_same_str(&["abc", "def"]);
    assert_same_str(&["", "abc"]);
    assert_same_str(&[".5", "1e3"]);
    assert_same_str(&["-", "+"]);
    assert_same_str(&["+", "-"]);
    assert_same_str(&["  -  5", "0"]);
    assert_same_str(&["--5", "++5"]);
}

#[test]
fn parsing_stops_at_the_first_non_digit() {
    assert_same_str(&["12abc", "34xyz"]);
    assert_same_str(&["5 5", "0"]);
    assert_same_str(&["1,000", "2.999"]);
    // No hex, no octal: base is fixed at 10, so "0x10" stops at 'x' and
    // "010" is ten, not eight.
    assert_same_str(&["0x10", "010"]);
    assert_same_str(&["-0x1f", "0"]);
}

#[test]
fn leading_zeros_are_insignificant() {
    assert_same_str(&["0000000000000000042", "0"]);
    assert_same_str(&["007", "-007"]);
}

// ---------------------------------------------------------------------------
// Phase C: int truncation and long saturation inside atoi()
// ---------------------------------------------------------------------------

#[test]
fn int_boundary_values() {
    assert_same_str(&["2147483647", "0"]);
    assert_same_str(&["-2147483648", "0"]);
    assert_same_str(&["0", "2147483647"]);
    assert_same_str(&["0", "-2147483648"]);
}

#[test]
fn values_beyond_int_are_truncated() {
    // atoi() is (int)strtol(...): these fit in a long and are then cut down.
    assert_same_str(&["2147483648", "0"]);
    assert_same_str(&["-2147483649", "0"]);
    assert_same_str(&["4294967295", "0"]);
    assert_same_str(&["4294967296", "0"]);
    assert_same_str(&["4294967297", "0"]);
    assert_same_str(&["8589934592", "0"]);
}

#[test]
fn long_boundary_values() {
    assert_same_str(&["9223372036854775807", "0"]);
    assert_same_str(&["-9223372036854775808", "0"]);
}

#[test]
fn values_beyond_long_saturate_then_truncate() {
    // strtol clamps to LONG_MAX / LONG_MIN, and the cast to int then keeps
    // the low 32 bits: -1 for the positive side, 0 for the negative side.
    assert_same_str(&["9223372036854775808", "0"]);
    assert_same_str(&["-9223372036854775809", "0"]);
    assert_same_str(&["18446744073709551616", "0"]);
    assert_same_str(&["99999999999999999999", "0"]);
    assert_same_str(&["-99999999999999999999", "0"]);
}

#[test]
fn very_long_digit_strings() {
    // Far past any integer width, and long enough to exercise the
    // "keep consuming digits after saturating" path.
    let many_nines = "9".repeat(400);
    let many_negative_nines = format!("-{many_nines}");
    let padded = format!("{}5", "0".repeat(500));
    assert_same_str(&[&many_nines, "0"]);
    assert_same_str(&[&many_negative_nines, "0"]);
    assert_same_str(&[&padded, "0"]);
}

// ---------------------------------------------------------------------------
// Phase C: the addition itself
// ---------------------------------------------------------------------------

#[test]
fn addition_overflows_exactly_as_the_c_does() {
    assert_same_str(&["2147483647", "1"]);
    assert_same_str(&["2147483647", "2147483647"]);
    assert_same_str(&["-2147483648", "-1"]);
    assert_same_str(&["-2147483648", "-2147483648"]);
    assert_same_str(&["1073741824", "1073741824"]);
    assert_same_str(&["-1073741824", "-1073741825"]);
    assert_same_str(&["2147483647", "-2147483648"]);
}

// ---------------------------------------------------------------------------
// Phase C: argv shape -- the paths that never print anything
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_dies_the_same_way() {
    // argv[1] is NULL, atoi() dereferences it: killed by SIGSEGV with no
    // output at all. This is the case that distinguishes a status-checking
    // test from a stdout-only one.
    assert_same(&[]);
}

#[test]
fn one_argument_dies_the_same_way() {
    // argv[1] parses fine, argv[2] is NULL.
    assert_same(&[b"5"]);
    assert_same(&[b"abc"]);
    assert_same(&[b""]);
}

#[test]
fn no_output_is_produced_on_the_crashing_paths() {
    for raw in [Vec::<&[u8]>::new(), vec![b"5" as &[u8]]] {
        let args = args_from_bytes(&raw);
        let c = run(c_binary(), &args);
        assert!(c.stdout.is_empty(), "the C program printed before crashing");
        let r = run(rust_binary(), &args);
        assert!(r.stdout.is_empty(), "the Rust program printed before crashing");
        assert_eq!(termination(&c), termination(&r));
        // Guard against the crash being silently downgraded to a clean exit.
        assert_ne!(
            termination(&c),
            Termination::Exited(0),
            "the C program is expected to crash here"
        );
    }
}

#[test]
fn extra_arguments_are_ignored() {
    assert_same_str(&["1", "2", "3"]);
    assert_same_str(&["1", "2", "ignored", "also-ignored", ""]);
}

#[test]
fn arguments_that_are_not_valid_utf8() {
    // argv is a byte string in C; nothing here decodes it.
    assert_same(&[b"a\xffb", b"7"]);
    assert_same(&[b"\xff\xfe", b"\x80"]);
    assert_same(&[b"12\xff34", b"7"]);
    assert_same(&[b"caf\xc3\xa9", b"7"]);
}

// ---------------------------------------------------------------------------
// A broad sweep, in case the hand-picked cases missed a class
// ---------------------------------------------------------------------------

#[test]
fn randomised_sweep() {
    // A small deterministic PRNG keeps this dependency-free and reproducible.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let fragments = ["", "0", "-", "+", " ", "\t", "abc", "0x1f", "007", "  "];

    for _ in 0..250 {
        let make = |r: u64| -> String {
            match r % 4 {
                0 => (r as i32).to_string(),
                1 => (r as i64).to_string(),
                2 => fragments[(r as usize / 7) % fragments.len()].to_string(),
                _ => format!(
                    "{}{}{}",
                    fragments[(r as usize) % fragments.len()],
                    r as i32 % 1_000_001,
                    fragments[(r as usize / 13) % fragments.len()]
                ),
            }
        };
        let a = make(next());
        let b = make(next());
        assert_same_str(&[&a, &b]);
    }
}
