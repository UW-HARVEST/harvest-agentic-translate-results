//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical arguments / stdin / environment, and require
//! byte-identical stdout, byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library: both programs are
//! driven exactly the way a shell drives them, because that is how they are
//! compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// ---------------------------------------------------------------- locating ---

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the C executable produced by `c_src/CMakeLists.txt`.
fn c_bin() -> PathBuf {
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let build = root.join("c_src").join("build");
    for candidate in [
        build.join("driver"),
        build.join("driver.exe"),
        build.join("Release").join("driver.exe"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    build.join("driver")
}

static BUILD_C: Once = Once::new();

/// Build the C program once per test binary, so the suite is self-contained.
fn ensure_c_built() {
    BUILD_C.call_once(|| {
        if c_bin().is_file() {
            return;
        }
        let root = manifest_dir().parent().expect("workspace root").to_path_buf();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        std::fs::create_dir_all(&build).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
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
            .expect("run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
    });
    assert!(
        c_bin().is_file(),
        "C executable missing at {}",
        c_bin().display()
    );
}

// ------------------------------------------------------------------ running ---

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    code: Option<i32>,
    /// Unix signal number when killed by one.
    signal: Option<i32>,
}

fn run(program: &Path, args: &[&str], stdin_bytes: &[u8], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

    // The programs never read stdin; write it anyway and tolerate EPIPE so a
    // non-reading child cannot deadlock or fail the test.
    {
        let mut sink = child.stdin.take().expect("stdin pipe");
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// The core assertion: identical stdout, stderr and exit status.
fn assert_same(case: &str, args: &[&str], stdin_bytes: &[u8], env: &[(&str, &str)]) -> Run {
    ensure_c_built();
    let c = run(&c_bin(), args, stdin_bytes, env);
    let r = run(&rust_bin(), args, stdin_bytes, env);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs\n  C:    {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr differs\n  C:    {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "[{case}] exit code differs (C {:?} vs Rust {:?})",
        c.code, r.code
    );
    assert_eq!(
        c.signal, r.signal,
        "[{case}] terminating signal differs (C {:?} vs Rust {:?})",
        c.signal, r.signal
    );
    c
}

/// The exact bytes `main` must produce, derived by reading the C source:
/// good() prints 0 then 2 (intSum is assigned), bad() prints 0 twice (the
/// `intOne + intTwo` statement's result is discarded, so intSum stays 0).
const EXPECTED: &[u8] =
    b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";

// -------------------------------------------------------------- happy path ---

#[test]
fn no_arguments_matches_and_has_expected_bytes() {
    let c = assert_same("no args", &[], b"", &[]);
    assert_eq!(
        c.stdout,
        EXPECTED,
        "C stdout drifted from the bytes read out of c_src/src/main.c:\n  got {}",
        show(&c.stdout)
    );
    assert!(c.stderr.is_empty(), "C wrote to stderr: {}", show(&c.stderr));
    assert_eq!(c.code, Some(0), "main() returns 0");
}

// ---------------------------------------------- argv classes (argc/argv are
// accepted by main but never inspected, so every arity must behave the same) ---

#[test]
fn single_argument() {
    assert_same("one arg", &["alpha"], b"", &[]);
}

#[test]
fn two_arguments() {
    assert_same("two args", &["alpha", "beta"], b"", &[]);
}

#[test]
fn empty_string_argument() {
    assert_same("empty arg", &[""], b"", &[]);
}

#[test]
fn arguments_that_look_like_flags() {
    assert_same("flag-ish args", &["-h", "--help", "-0", "--"], b"", &[]);
}

#[test]
fn argument_with_whitespace_newline_and_tab() {
    assert_same("whitespace arg", &["a b\tc\nd", "  "], b"", &[]);
}

#[test]
fn argument_with_format_specifiers() {
    // Guards against either program accidentally treating an argument as a
    // printf format string.
    assert_same("format-string arg", &["%s%d%n%%", "%99999999d"], b"", &[]);
}

#[test]
fn argument_with_non_ascii_bytes() {
    assert_same("utf-8 arg", &["ünïcødé…", "日本語"], b"", &[]);
}

#[test]
fn many_arguments() {
    let owned: Vec<String> = (0..512).map(|i| format!("arg{i}")).collect();
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_same("512 args", &args, b"", &[]);
}

#[test]
fn very_long_single_argument() {
    let long = "x".repeat(64 * 1024);
    assert_same("64KiB arg", &[long.as_str()], b"", &[]);
}

// ------------------------------------------------------------ stdin classes ---
// Neither program reads stdin; every one of these must be ignored identically.

#[test]
fn empty_stdin() {
    assert_same("empty stdin", &[], b"", &[]);
}

#[test]
fn stdin_with_a_single_line() {
    assert_same("one line stdin", &[], b"1\n", &[]);
}

#[test]
fn stdin_with_many_lines_and_no_trailing_newline() {
    assert_same("multiline stdin", &[], b"1\n2\n3\nno-trailing-newline", &[]);
}

#[test]
fn stdin_with_nul_and_binary_bytes() {
    assert_same("binary stdin", &[], &[0u8, 1, 2, 255, 254, b'\n', 0], &[]);
}

#[test]
fn stdin_from_dev_null() {
    ensure_c_built();
    let mut outs = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let out = Command::new(&prog)
            .stdin(Stdio::null())
            .output()
            .expect("run with /dev/null stdin");
        outs.push((out.stdout, out.stderr, out.status.code()));
    }
    assert_eq!(outs[0], outs[1], "/dev/null stdin: outputs differ");
    assert_eq!(outs[0].0, EXPECTED);
}

#[test]
fn stdin_is_left_completely_unconsumed() {
    // If one program read stdin and the other did not, the leftover bytes seen
    // by a following `cat` would differ. This pins the "reads nothing" contract.
    ensure_c_built();
    let payload = "LEFTOVER-1\nLEFTOVER-2\n";
    let mut results = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let script = format!("\"{}\"; cat", prog.display());
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .expect("write stdin");
        let out = child.wait_with_output().expect("wait sh");
        results.push((out.stdout, out.stderr, out.status.code()));
    }
    assert_eq!(results[0], results[1], "leftover stdin differs");
    let mut expected = EXPECTED.to_vec();
    expected.extend_from_slice(payload.as_bytes());
    assert_eq!(
        results[0].0, expected,
        "stdin must be passed through untouched"
    );
}

// --------------------------------------------------- environment / locale ----

#[test]
fn c_locale() {
    assert_same("LC_ALL=C", &[], b"", &[("LC_ALL", "C"), ("LANG", "C")]);
}

#[test]
fn utf8_locale() {
    assert_same(
        "LC_ALL=en_US.UTF-8",
        &[],
        b"",
        &[("LC_ALL", "en_US.UTF-8"), ("LANG", "en_US.UTF-8")],
    );
}

#[test]
fn numeric_locale_that_could_change_integer_grouping() {
    assert_same(
        "LC_NUMERIC=de_DE.UTF-8",
        &[],
        b"",
        &[("LC_NUMERIC", "de_DE.UTF-8")],
    );
}

// ------------------------------------------------ output destination classes --

#[test]
fn stdout_redirected_to_a_file_is_identical() {
    // stdio is fully buffered to a file and line buffered to a tty; the flushed
    // bytes must be the same either way, and in the same order.
    ensure_c_built();
    let dir = std::env::temp_dir().join(format!("driver-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let mut contents = Vec::new();
    for (name, prog) in [("c", c_bin()), ("rust", rust_bin())] {
        let path = dir.join(format!("{name}.out"));
        let file = std::fs::File::create(&path).expect("create out file");
        let status = Command::new(&prog)
            .stdout(file)
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .expect("run with file stdout");
        assert_eq!(status.status.code(), Some(0));
        assert!(status.stderr.is_empty());
        contents.push(std::fs::read(&path).expect("read out file"));
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(contents[0], contents[1], "file-redirected stdout differs");
    assert_eq!(contents[0], EXPECTED);
}

#[test]
fn stdout_and_stderr_merged_into_one_stream() {
    // Interleaving order is observable when 2>&1; both must emit stdout only.
    ensure_c_built();
    let mut outs = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let script = format!("\"{}\" 2>&1", prog.display());
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::null())
            .output()
            .expect("run with 2>&1");
        outs.push((out.stdout, out.status.code()));
    }
    assert_eq!(outs[0], outs[1], "merged stream differs");
    assert_eq!(outs[0].0, EXPECTED);
}

#[test]
fn closed_stdout_does_not_change_the_exit_status() {
    // printf's failure is ignored by the C; the Rust must not diverge (e.g. by
    // panicking on a failed write).
    ensure_c_built();
    let mut results = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let script = format!("exec 1>&-; exec \"{}\"", prog.display());
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::null())
            .output()
            .expect("run with closed stdout");
        #[cfg(unix)]
        let sig = {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal()
        };
        #[cfg(not(unix))]
        let sig: Option<i32> = None;
        results.push((out.status.code(), sig, out.stderr));
    }
    assert_eq!(
        results[0], results[1],
        "closed-stdout behavior differs (code, signal, stderr)"
    );
}

#[test]
fn early_closing_reader_on_the_pipe() {
    // `| head -c 3` can close the pipe while the writer is still going.
    ensure_c_built();
    let mut results = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let script = format!(
            "\"{}\" 2>/dev/null | head -c 3; exit ${{PIPESTATUS[0]:-0}}",
            prog.display()
        );
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::null())
            .output()
            .expect("run into head");
        results.push((out.stdout, out.status.code()));
    }
    assert_eq!(results[0], results[1], "broken-pipe behavior differs");
    assert_eq!(results[0].0, b"Cal".to_vec());
}

// ----------------------------------------------------------------- argv[0] ---

#[test]
fn different_argv0_program_name() {
    // main ignores argv[0]; invoking through a differently named copy must not
    // change a byte of output.
    ensure_c_built();
    let dir = std::env::temp_dir().join(format!("driver-argv0-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let mut outs = Vec::new();
    for (name, prog) in [("c_renamed", c_bin()), ("rust_renamed", rust_bin())] {
        let dest = dir.join(name);
        std::fs::copy(&prog, &dest).expect("copy binary");
        // A concurrently running test thread can have inherited the write fd
        // for `dest` across its own fork, which makes exec fail with ETXTBSY.
        // That is a harness race, not a behavioral difference, so retry.
        let mut attempt = 0;
        let out = loop {
            match Command::new(&dest).stdin(Stdio::null()).output() {
                Ok(out) => break out,
                Err(e) if e.raw_os_error() == Some(26) && attempt < 100 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => panic!("run renamed binary: {e}"),
            }
        };
        outs.push((out.stdout, out.stderr, out.status.code()));
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(outs[0], outs[1], "renamed-binary output differs");
    assert_eq!(outs[0].0, EXPECTED);
}

// ------------------------------------------------------------- determinism ---

#[test]
fn repeated_runs_are_byte_stable() {
    ensure_c_built();
    let first = assert_same("repeat #0", &[], b"", &[]);
    for i in 1..8 {
        let again = assert_same(&format!("repeat #{i}"), &[], b"", &[]);
        assert_eq!(first.stdout, again.stdout, "run {i} stdout not stable");
        assert_eq!(first.stderr, again.stderr, "run {i} stderr not stable");
        assert_eq!(first.code, again.code, "run {i} exit code not stable");
    }
}

// ------------------------------------------- line-level structural checks ----

#[test]
fn line_by_line_structure_matches_the_c_source() {
    // Each printLine / printIntLine call site, in order, with its own line.
    ensure_c_built();
    let c = run(&c_bin(), &[], b"", &[]);
    let r = run(&rust_bin(), &[], b"", &[]);
    let c_text = String::from_utf8(c.stdout.clone()).expect("C stdout is utf-8");
    let r_text = String::from_utf8(r.stdout.clone()).expect("Rust stdout is utf-8");
    let expected = [
        "Calling good()...",
        "0",
        "2",
        "Finished good()",
        "Calling bad()...",
        "0",
        "0",
        "Finished bad()",
    ];
    for (i, want) in expected.iter().enumerate() {
        let c_line = c_text.lines().nth(i);
        let r_line = r_text.lines().nth(i);
        assert_eq!(c_line, Some(*want), "C line {i}");
        assert_eq!(r_line, Some(*want), "Rust line {i}");
    }
    assert_eq!(c_text.lines().count(), expected.len(), "C line count");
    assert_eq!(r_text.lines().count(), expected.len(), "Rust line count");
    // A trailing newline after the last line, and no extra blank line.
    assert!(c_text.ends_with("Finished bad()\n"));
    assert!(r_text.ends_with("Finished bad()\n"));
    assert!(!c_text.ends_with("\n\n"));
    assert!(!r_text.ends_with("\n\n"));
}

#[test]
fn bad_does_not_update_int_sum_but_good_does() {
    // Pins the deliberate C "bug" that must be preserved: bad() prints 0,0 and
    // good() prints 0,2. If the Rust ever "fixed" bad(), this fails.
    ensure_c_built();
    for prog in [c_bin(), rust_bin()] {
        let out = run(&prog, &[], b"", &[]);
        let text = String::from_utf8(out.stdout).expect("utf-8");
        let nums: Vec<&str> = text
            .lines()
            .filter(|l| l.chars().all(|c| c.is_ascii_digit()) && !l.is_empty())
            .collect();
        assert_eq!(
            nums,
            vec!["0", "2", "0", "0"],
            "{} printed the wrong integer sequence",
            prog.display()
        );
    }
}
