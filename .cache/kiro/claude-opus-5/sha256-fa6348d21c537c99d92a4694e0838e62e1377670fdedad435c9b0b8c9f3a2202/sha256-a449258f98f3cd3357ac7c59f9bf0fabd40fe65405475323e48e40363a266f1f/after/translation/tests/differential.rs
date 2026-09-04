//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses with identical arguments and compare stdout, stderr and exit
//! status byte for byte.
//!
//! The Rust code is never called as a library. Both programs are driven
//! exactly the way a shell would drive them.
//!
//! Both programs print `argv[0]` in their usage message, so every invocation
//! forces the same `argv[0]` via `CommandExt::arg0` (the equivalent of
//! `exec -a driver ...`). That keeps stderr byte-identical without any
//! post-hoc normalisation.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// `argv[0]` handed to both programs.
const ARG0: &str = "./driver";

// ---------------------------------------------------------------------------
// Locating the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C program, building it with CMake if it is absent.
fn c_binary() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");

    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn cmake; is CMake installed?");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );

    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn cmake --build");
    assert!(
        compile.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    assert!(
        exe.is_file(),
        "C program was not produced at {}",
        exe.display()
    );
    exe
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

/// Spawn `bin` with `args` and `argv[0] == ARG0`, capturing everything.
fn spawn(bin: &Path, args: &[OsString]) -> std::process::Child {
    Command::new(bin)
        .arg0(ARG0)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()))
}

/// Render a byte string so mismatch reports stay readable.
fn show(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out
}

fn show_args(args: &[OsString]) -> String {
    let rendered: Vec<String> = args
        .iter()
        .map(|a| format!("'{}'", show(a.as_bytes())))
        .collect();
    format!("[{}]", rendered.join(", "))
}

/// Compare two `Output`s on all three observable channels.
///
/// Returns `Err(description)` on any difference rather than panicking, so a
/// single test can report every failing input at once.
fn compare(args: &[OsString], c: &Output, r: &Output) -> Result<(), String> {
    let mut problems = Vec::new();

    if c.stdout != r.stdout {
        problems.push(format!(
            "  stdout differs:\n    C:    \"{}\"\n    Rust: \"{}\"",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "  stderr differs:\n    C:    \"{}\"\n    Rust: \"{}\"",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    // Compare the full status: exit code when exited normally, signal
    // otherwise. `ExitStatus`'s Debug covers both.
    if c.status.code() != r.status.code() {
        problems.push(format!(
            "  exit status differs:\n    C:    {:?} (code {:?})\n    Rust: {:?} (code {:?})",
            c.status,
            c.status.code(),
            r.status,
            r.status.code()
        ));
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "argv = {}\n{}",
            show_args(args),
            problems.join("\n")
        ))
    }
}

/// Run every case, C and Rust side by side, all cases concurrently.
///
/// The happy path is genuinely expensive (2000 passes over a 256K-element
/// array), so cases are launched in parallel instead of one after another.
fn run_cases(cases: &[Vec<OsString>]) {
    let c = c_binary();
    let r = rust_binary();

    // Launch every child up-front; each case is an independent process pair.
    let mut running: Vec<(&Vec<OsString>, std::process::Child, std::process::Child)> =
        Vec::with_capacity(cases.len());
    for args in cases {
        let cc = spawn(&c, args);
        let rc = spawn(&r, args);
        running.push((args, cc, rc));
    }

    let mut failures = Vec::new();
    for (args, cc, rc) in running {
        let c_out = cc.wait_with_output().expect("wait on C program");
        let r_out = rc.wait_with_output().expect("wait on Rust program");
        if let Err(msg) = compare(args, &c_out, &r_out) {
            failures.push(msg);
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} of {} input(s) diverged:\n\n{}\n",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}

fn args_of<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    items
        .into_iter()
        .map(|s| s.as_ref().to_os_string())
        .collect()
}

/// A single argument built from raw bytes (may be invalid UTF-8).
fn raw(bytes: &[u8]) -> Vec<OsString> {
    vec![OsString::from_vec(bytes.to_vec())]
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "missing C binary at {}", c.display());
    assert!(r.is_file(), "missing Rust binary at {}", r.display());

    // Cheapest invocation that exercises a full code path in both.
    let no_args: Vec<OsString> = Vec::new();
    let c_out = spawn(&c, &no_args).wait_with_output().unwrap();
    let r_out = spawn(&r, &no_args).wait_with_output().unwrap();
    compare(&no_args, &c_out, &r_out).unwrap();
    assert_eq!(c_out.status.code(), Some(1));
    assert_eq!(c_out.stderr, b"Usage: ./driver <seed>\n");
}

// ---------------------------------------------------------------------------
// Phase B/C: `argc != 2`
// ---------------------------------------------------------------------------

#[test]
fn argc_branch() {
    let cases: Vec<Vec<OsString>> = vec![
        // argc == 1
        args_of::<[&str; 0], &str>([]),
        // argc == 3
        args_of(["1", "2"]),
        args_of(["1", ""]),
        args_of(["", ""]),
        // argc == 4 and beyond
        args_of(["1", "2", "3"]),
        args_of(["7", "junk", "more", "and-more"]),
        // A valid seed is still rejected when it is not the only argument,
        // and the seed is never parsed: no "Invalid seed" for the bad one.
        args_of(["notanumber", "alsobad"]),
    ];
    run_cases(&cases);
}

/// Both programs echo `argv[0]` verbatim in the usage message.
#[test]
fn usage_message_echoes_argv0() {
    let c = c_binary();
    let r = rust_binary();
    let no_args: Vec<OsString> = Vec::new();

    for arg0 in ["driver", "/weird/path/to/driver", "x", ""] {
        let spawn_with = |bin: &Path| {
            Command::new(bin)
                .arg0(arg0)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap()
        };
        let c_out = spawn_with(&c);
        let r_out = spawn_with(&r);
        compare(&no_args, &c_out, &r_out)
            .unwrap_or_else(|e| panic!("argv[0] = {arg0:?}\n{e}"));
        assert_eq!(
            c_out.stderr,
            format!("Usage: {arg0} <seed>\n").into_bytes(),
            "argv[0] = {arg0:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: `strtoul` rejection paths (`*endptr != '\0'`)
// ---------------------------------------------------------------------------

#[test]
fn trailing_garbage_branch() {
    let mut cases: Vec<Vec<OsString>> = vec![
        // No conversion at all: endptr == nptr, first byte is not NUL.
        args_of(["abc"]),
        args_of(["x"]),
        args_of([" "]),
        args_of(["   "]),
        args_of(["\t"]),
        args_of(["\n"]),
        args_of(["\r"]),
        args_of(["\u{b}"]),
        args_of(["\u{c}"]),
        args_of(["+"]),
        args_of(["-"]),
        args_of(["++1"]),
        args_of(["--1"]),
        args_of(["+-1"]),
        args_of(["-+1"]),
        args_of([" -"]),
        args_of(["."]),
        args_of(["e5"]),
        // Digits followed by junk.
        args_of(["5x"]),
        args_of(["12."]),
        args_of(["1 "]),
        args_of(["1\t"]),
        args_of(["1\n"]),
        args_of(["0x10"]),
        args_of(["10abc"]),
        args_of(["1_000"]),
        args_of(["1,000"]),
        args_of(["1-"]),
        args_of(["1+1"]),
        args_of(["-1x"]),
        args_of(["+1x"]),
        args_of(["  42  "]),
        args_of(["4294967295 "]),
        // Base-10 only: base-8/base-16 prefixes are not special.
        args_of(["0b101"]),
        args_of(["0o17"]),
        // Non-ASCII digits are not digits.
        args_of(["\u{661}\u{662}"]),
    ];
    // Raw bytes that are not valid UTF-8: the error message must echo them
    // through unchanged.
    cases.push(raw(b"\xff"));
    cases.push(raw(b"\xff\xfe"));
    cases.push(raw(b"12\xff"));
    cases.push(raw(b"\xc3"));
    cases.push(raw(b"\x80\x80"));
    run_cases(&cases);
}

// ---------------------------------------------------------------------------
// Phase B/C: `errno != 0` (ERANGE) and `temp_seed > UINT_MAX`
// ---------------------------------------------------------------------------

#[test]
fn out_of_range_branch() {
    let cases: Vec<Vec<OsString>> = vec![
        // Just past UINT_MAX.
        args_of(["4294967296"]),
        args_of(["4294967297"]),
        args_of(["429496729500"]),
        args_of(["9999999999"]),
        // ULONG_MAX exactly: strtoul succeeds without ERANGE, so this is
        // rejected purely by `temp_seed > UINT_MAX`.
        args_of(["18446744073709551615"]),
        args_of(["+18446744073709551615"]),
        // Just past ULONG_MAX: ERANGE, strtoul returns ULONG_MAX.
        args_of(["18446744073709551616"]),
        args_of(["99999999999999999999999999"]),
        args_of([
            "1000000000000000000000000000000000000000000000000000000000000000",
        ]),
        // Leading zeros do not create an overflow.
        args_of(["00000000000000000000000000000000004294967296"]),
        // Negative inputs are wrapped by strtoul, then range-checked.
        args_of(["-1"]),
        args_of(["-2"]),
        args_of(["-7"]),
        args_of(["-4294967295"]),
        args_of(["-4294967296"]),
        // Negative with ERANGE: strtoul returns ULONG_MAX (not negated).
        args_of(["-18446744073709551616"]),
        args_of(["-99999999999999999999999999"]),
        // Wraps to exactly 4294967296, i.e. UINT_MAX + 1.
        args_of(["-18446744069414584320"]),
    ];
    run_cases(&cases);
}

/// A digit string far longer than any integer: ERANGE, and the whole argument
/// is echoed back in the error message.
#[test]
fn very_long_argument() {
    let mut cases: Vec<Vec<OsString>> = Vec::new();
    cases.push(args_of([&"9".repeat(100_000)]));
    cases.push(args_of([&format!("-{}", "9".repeat(100_000))]));
    // 100k leading zeros followed by an out-of-range value: no ERANGE from the
    // zeros, rejected by the UINT_MAX check.
    cases.push(args_of([&format!("{}4294967296", "0".repeat(100_000))]));
    // Long run of non-digits.
    cases.push(args_of([&"z".repeat(100_000)]));
    run_cases(&cases);
}

// ---------------------------------------------------------------------------
// Phase B/C: the accepting path (expensive)
// ---------------------------------------------------------------------------

/// Every input that reaches `srand` + the 2000-pass workload.
///
/// This is the slow test: a single run touches 256K ints 200000 times each.
/// All process pairs are launched concurrently so wall-clock time is roughly
/// one run, not `n` runs.
#[test]
fn accepted_seeds_produce_identical_output() {
    let cases: Vec<Vec<OsString>> = vec![
        // Empty argument: strtoul performs no conversion, so endptr is reset
        // to nptr, `*endptr` is the terminating NUL, and seed 0 is accepted.
        args_of([""]),
        // seed 0 (glibc's srand forces the internal seed to 1)
        args_of(["0"]),
        args_of(["-0"]),
        args_of(["+0"]),
        args_of(["0000"]),
        // seed 1
        args_of(["1"]),
        // Negating ULONG_MAX yields 1, which is in range: accepted.
        args_of(["-18446744073709551615"]),
        // Small seeds, sign and leading-whitespace/zero forms.
        args_of(["7"]),
        args_of(["+7"]),
        args_of(["007"]),
        args_of(["\t9"]),
        args_of([" 12"]),
        args_of(["12"]),
        // The rest of C's `isspace` set, which `strtoul` also skips. These are
        // only discriminating on the accepting path: if the leading byte were
        // not skipped the program would reject the argument instead.
        args_of(["\n9"]),
        args_of(["\r9"]),
        args_of(["\u{b}9"]),
        args_of(["\u{c}9"]),
        args_of([" \t\n\u{b}\u{c}\r9"]),
        args_of([" \t\n\u{b}\u{c}\r-18446744073709551615"]),
        // Seed with the high bit set (> INT_MAX).
        args_of(["2147483648"]),
        // The maximum seed the code accepts.
        args_of(["4294967295"]),
        // -18446744069414584321 negates to 4294967295 (UINT_MAX): accepted.
        args_of(["-18446744069414584321"]),
    ];
    run_cases(&cases);
}
