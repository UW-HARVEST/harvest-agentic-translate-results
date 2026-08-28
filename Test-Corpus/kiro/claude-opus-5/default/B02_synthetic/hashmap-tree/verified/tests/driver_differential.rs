//! Phase A/B: differential tests of the two *executables*.
//!
//! `c_src/src/main.c` declares `int main(void)`: it reads nothing from stdin and
//! ignores argv entirely. The invocation classes below therefore cover the
//! observable surface of the program itself — the branch coverage of the
//! library code lives in `library_branches.rs`.

mod common;

use common::*;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn rust_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn diff_case(case: &str, args: &[&str], stdin_bytes: Option<&[u8]>) {
    let c = capture(&c_driver(), args, stdin_bytes);
    let r = capture(&rust_driver(), args, stdin_bytes);
    assert_same(case, &c, &r);
}

#[test]
fn no_arguments_no_stdin() {
    diff_case("bare invocation", &[], None);
}

#[test]
fn c_reference_was_built_with_assertions_enabled() {
    // main.c performs every mutation *inside* assert(), so a build that defines
    // NDEBUG (for example `cmake -DCMAKE_BUILD_TYPE=Release`) silently skips all
    // of them: the tree ends up empty, `tree_print` reports "(empty tree)" and
    // stderr stays silent. The Rust translation keeps the side effects, matching
    // the plain `cmake .. && cmake --build .` build the task specifies. Detect a
    // mis-built reference instead of reporting a translation bug.
    let c = capture(&c_driver(), &[], None);
    let text = String::from_utf8_lossy(&c.stdout).to_string();
    assert!(
        !text.contains("(empty tree)"),
        "the C driver appears to be built with NDEBUG (asserts compiled out); \
         remove c_src/build and rebuild with plain `cmake ..`"
    );
    assert!(
        text.contains("[10] ggc1"),
        "the C driver did not print the complex tree, so its assert() side \
         effects did not run"
    );
    assert!(
        !c.stderr.is_empty(),
        "the C driver produced no stderr, so its error paths did not run"
    );
}

#[test]
fn exit_status_is_zero_for_both() {
    // main() ends in `return 0`, and no assert() fires, so both programs must
    // exit 0 rather than merely printing the same text.
    let c = capture(&c_driver(), &[], None);
    let r = capture(&rust_driver(), &[], None);
    assert_eq!(c.code, Some(0), "C driver exit status");
    assert_eq!(r.code, Some(0), "Rust driver exit status");
    assert_eq!(c.signal, None);
    assert_eq!(r.signal, None);
}

#[test]
fn stderr_carries_the_two_expected_error_lines() {
    // test_tree_duplicate_id and test_tree_max_children are the only paths that
    // write to stderr. A stdout-only comparison would miss them entirely.
    let c = capture(&c_driver(), &[], None);
    let r = capture(&rust_driver(), &[], None);
    assert_same("stderr content", &c, &r);
    assert!(
        !c.stderr.is_empty(),
        "the C program is expected to emit diagnostics on stderr; \
         if this fires, the comparison above proves nothing"
    );
}

#[test]
fn ignores_argv() {
    diff_case("one argument", &["ignored"], None);
    diff_case("many arguments", &["a", "b", "c", "d"], None);
    diff_case("flag-looking arguments", &["-h", "--help", "--version"], None);
    diff_case("empty-string argument", &[""], None);
    diff_case("argument with spaces", &["a b\tc\nd"], None);
}

#[test]
fn ignores_stdin() {
    diff_case("empty stdin", &[], Some(b""));
    diff_case("single line on stdin", &[], Some(b"1\n"));
    diff_case("single token, no newline", &[], Some(b"1"));
    diff_case("multiple lines", &[], Some(b"3\n1 root\n2 child\n"));
    diff_case("binary stdin", &[], Some(&[0u8, 1, 2, 255, b'\n', 0]));
    diff_case("long stdin", &[], Some(&vec![b'x'; 128 * 1024]));
}

#[test]
fn broken_stdout_pipe_kills_both_the_same_way() {
    // A C program keeps SIGPIPE at its default disposition and dies with signal
    // 13 when the reader goes away; Rust's runtime ignores SIGPIPE unless the
    // program restores it. This asserts the exit status, not just the bytes.
    fn with_closed_reader(exe: &PathBuf) -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let mut child = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        // Drop the read end immediately so the first write to stdout fails.
        drop(child.stdout.take());
        let status = child.wait().expect("wait");
        (status.code(), status.signal())
    }

    let (cc, cs) = with_closed_reader(&c_driver());
    let (rc, rs) = with_closed_reader(&rust_driver());
    assert_eq!(
        (cc, cs),
        (rc, rs),
        "C exited (code {cc:?}, signal {cs:?}) but Rust exited (code {rc:?}, signal {rs:?})"
    );
}

#[test]
fn unwritable_stdout_does_not_change_the_exit_status() {
    // /dev/full accepts the open() and fails every write with ENOSPC. The C code
    // ignores printf's return value and main still returns 0, so the exit status
    // must stay 0 in both programs.
    fn to_dev_full(exe: &PathBuf) -> (Vec<u8>, Option<i32>) {
        let full = match fs::OpenOptions::new().write(true).open("/dev/full") {
            Ok(f) => f,
            Err(_) => return (Vec::new(), Some(0)), // platform without /dev/full
        };
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(full))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.stderr, out.status.code())
    }

    let (ce, cc) = to_dev_full(&c_driver());
    let (re, rc) = to_dev_full(&rust_driver());
    assert_eq!(ce, re, "stderr when stdout is unwritable");
    assert_eq!(cc, rc, "exit status when stdout is unwritable");
}

#[test]
fn stdin_that_cannot_be_read() {
    // A descriptor that is open but unreadable (a directory) would make any
    // `fgets`/`scanf` fail. Neither program reads stdin, so both must be
    // unaffected — including the exit status.
    fn with_dir_stdin(exe: &PathBuf) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let dir = fs::File::open(repo_root()).expect("open repo root as a file");
        let out = Command::new(exe)
            .stdin(Stdio::from(dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.stdout, out.stderr, out.status.code())
    }

    let (co, ce, cc) = with_dir_stdin(&c_driver());
    let (ro, re, rc) = with_dir_stdin(&rust_driver());
    assert_eq!(co, ro, "stdout with unreadable stdin");
    assert_eq!(ce, re, "stderr with unreadable stdin");
    assert_eq!(cc, rc, "exit status with unreadable stdin");
}

/// Run a program with stdout and stderr pointing at the *same* file, which is
/// what a shell's `2>&1` does. This is sensitive to C stdio buffering: stdout is
/// fully buffered when it is not a terminal, so every stderr line lands before
/// the stdout block.
fn capture_merged(exe: &PathBuf, tag: &str) -> (Vec<u8>, Option<i32>) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("cdiff");
    fs::create_dir_all(&dir).expect("create target/cdiff");
    let path = dir.join(format!("merged-{tag}.txt"));
    let file = fs::File::create(&path).expect("create merged output file");
    let dup = file.try_clone().expect("clone file handle");

    let status = Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(dup))
        .status()
        .expect("spawn");

    (fs::read(&path).expect("read merged output"), status.code())
}

#[test]
fn merged_streams_interleave_identically() {
    let (c_bytes, c_code) = capture_merged(&c_driver(), "c");
    let (r_bytes, r_code) = capture_merged(&rust_driver(), "rust");
    assert_eq!(c_code, r_code, "exit status with merged streams");
    assert_eq!(
        String::from_utf8_lossy(&c_bytes),
        String::from_utf8_lossy(&r_bytes),
        "merged 2>&1 output must match byte for byte"
    );
    assert_eq!(c_bytes, r_bytes, "merged 2>&1 output must match byte for byte");
}

#[test]
fn output_is_deterministic_across_runs() {
    // The hashmap's probe order depends on FNV-1a of the key, not on any
    // address, so repeated runs must be identical for both programs.
    let first_c = capture(&c_driver(), &[], None).stdout;
    let first_r = capture(&rust_driver(), &[], None).stdout;
    for i in 0..3 {
        let c = capture(&c_driver(), &[], None);
        let r = capture(&rust_driver(), &[], None);
        assert_eq!(c.stdout, first_c, "C run {i} not deterministic");
        assert_eq!(r.stdout, first_r, "Rust run {i} not deterministic");
        assert_same(&format!("repeat run {i}"), &c, &r);
    }
}

#[test]
fn environment_does_not_change_output() {
    fn with_env(exe: &PathBuf, vars: &[(&str, &str)]) -> Captured {
        let mut cmd = Command::new(exe);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in vars {
            cmd.env(k, v);
        }
        let out = cmd.output().expect("spawn");
        Captured {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    out.status.signal()
                }
                #[cfg(not(unix))]
                {
                    None
                }
            },
        }
    }

    let vars = [
        ("LC_ALL", "C"),
        ("LANG", "C"),
        ("RUST_BACKTRACE", "1"),
        ("COLUMNS", "40"),
    ];
    let c = with_env(&c_driver(), &vars);
    let r = with_env(&rust_driver(), &vars);
    assert_same("LC_ALL=C environment", &c, &r);

    let vars = [("LC_ALL", "en_US.UTF-8"), ("LANG", "en_US.UTF-8")];
    let c = with_env(&c_driver(), &vars);
    let r = with_env(&rust_driver(), &vars);
    assert_same("UTF-8 locale environment", &c, &r);
}

#[test]
fn stdout_to_regular_file_matches() {
    // Redirecting to a file rather than a pipe changes glibc's buffering mode
    // decision; the emitted bytes must still be identical.
    fn to_file(exe: &PathBuf, tag: &str) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("cdiff");
        fs::create_dir_all(&dir).unwrap();
        let op = dir.join(format!("file-{tag}.out"));
        let ep = dir.join(format!("file-{tag}.err"));
        let status = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(fs::File::create(&op).unwrap()))
            .stderr(Stdio::from(fs::File::create(&ep).unwrap()))
            .status()
            .expect("spawn");
        (fs::read(&op).unwrap(), fs::read(&ep).unwrap(), status.code())
    }

    let (co, ce, cc) = to_file(&c_driver(), "c");
    let (ro, re, rc) = to_file(&rust_driver(), "rust");
    assert_eq!(
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
        "stdout written to a file"
    );
    assert_eq!(co, ro, "stdout written to a file");
    assert_eq!(ce, re, "stderr written to a file");
    assert_eq!(cc, rc, "exit status when writing to files");
}

#[test]
fn utf8_box_drawing_and_check_marks_are_exact() {
    // The banner and the PASS/FAIL markers are multi-byte UTF-8 literals; a
    // transcription slip there would be invisible in a "looks right" review.
    let c = capture(&c_driver(), &[], None);
    let r = capture(&rust_driver(), &[], None);
    assert_eq!(c.stdout, r.stdout);
    let text = String::from_utf8(c.stdout).expect("C stdout is UTF-8");
    assert!(text.contains('╔') && text.contains('╝'), "banner present");
    assert_eq!(
        text.matches("✓ PASS: ").count(),
        14,
        "all fourteen tests report PASS"
    );
    assert!(!text.contains("✗ FAIL"), "no test reports FAIL");
}
