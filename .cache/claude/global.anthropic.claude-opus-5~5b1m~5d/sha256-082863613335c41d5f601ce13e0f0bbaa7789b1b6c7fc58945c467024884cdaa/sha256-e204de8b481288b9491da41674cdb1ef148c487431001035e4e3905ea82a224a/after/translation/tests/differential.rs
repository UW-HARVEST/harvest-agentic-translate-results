// Differential tests: run the C binary and the Rust binary as subprocesses and
// require byte-identical stdout, byte-identical stderr and identical exit
// status for the same inputs.
//
// The Rust program is NEVER used as a library here; both programs are driven
// exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    // Cargo builds the `driver` bin target before running integration tests and
    // hands us its path, so this works under both `cargo test` and
    // `cargo test --release`.
    let candidate = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    assert!(
        candidate.is_file(),
        "Rust binary not found at {}",
        candidate.display()
    );
    candidate
}

/// Path to the C binary, building it with cmake if necessary.
///
/// Integration tests run as threads in a single process, so the build is done
/// exactly once behind a `OnceLock`. Letting each test invoke `cmake` in the
/// shared `c_src/build` directory concurrently makes the configure step clobber
/// its own temporary files and fail with "the C compiler is broken".
fn c_bin() -> PathBuf {
    static C_BIN: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    C_BIN.get_or_init(build_c_bin).clone()
}

/// Build the C program with cmake if it is not already built.
fn build_c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.is_file() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run cmake (is cmake installed?)");
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
        .expect("failed to run cmake --build");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    assert!(
        bin.is_file(),
        "C binary still missing at {} after building",
        bin.display()
    );
    bin
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Portable, fully-observable description of how a process terminated.
fn status_string(o: &Output) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = o.status.signal() {
            return format!("signal({sig})");
        }
    }
    match o.status.code() {
        Some(c) => format!("exit({c})"),
        None => "unknown".to_string(),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Assert the two runs are indistinguishable on stdout, stderr and status.
fn assert_same(case: &str, c: &Output, r: &Output) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs\n  C: {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr differs\n  C: {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        status_string(c),
        status_string(r),
        "[{case}] exit status differs (C={}, Rust={})",
        status_string(c),
        status_string(r)
    );
}

/// Run one binary with the given argv tail and stdin bytes.
fn run(bin: &Path, args: &[&str], stdin_data: Option<&[u8]>) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(match stdin_data {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(data) = stdin_data {
        let mut sin = child.stdin.take().expect("piped stdin");
        // The programs never read stdin, so a write here can legitimately fail
        // with EPIPE once the child exits; that is not a test failure.
        let _ = sin.write_all(data);
        drop(sin);
    }

    child.wait_with_output().expect("wait_with_output")
}

/// The core differential check for a plain (argv, stdin) input.
fn check(case: &str, args: &[&str], stdin_data: Option<&[u8]>) {
    let c = run(&c_bin(), args, stdin_data);
    let r = run(&rust_bin(), args, stdin_data);
    assert_same(case, &c, &r);
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C program can be given
//
// c_src/src/main.c is:
//     int main() { printf("Hello World!\n"); return 0; }
//
// It declares no parameters, so argv is ignored; it never reads stdin; it has
// no conditionals and exactly one `return`. The reachable input classes are
// therefore about the *process environment* rather than parsed data: argv
// shapes, stdin shapes, and the writability of stdout.
// ---------------------------------------------------------------------------

#[test]
fn no_args_no_stdin() {
    check("no args, /dev/null stdin", &[], None);
}

#[test]
fn output_is_exactly_hello_world() {
    // Pin the C program's exact bytes so a formatting drift in either program
    // (spacing, capitalization, missing or extra trailing newline) is caught
    // even if both programs drifted together.
    let c = run(&c_bin(), &[], None);
    assert_eq!(c.stdout, b"Hello World!\n");
    assert!(c.stderr.is_empty());
    assert_eq!(status_string(&c), "exit(0)");

    let r = run(&rust_bin(), &[], None);
    assert_same("exact bytes", &c, &r);
}

#[test]
fn empty_stdin() {
    check("empty stdin (immediate EOF)", &[], Some(b""));
}

#[test]
fn single_line_of_stdin() {
    // A single "item": the program must not consume or echo it.
    check("one line on stdin", &[], Some(b"1\n"));
}

#[test]
fn stdin_without_trailing_newline() {
    check("stdin lacking trailing newline", &[], Some(b"42"));
}

#[test]
fn many_lines_of_stdin() {
    let mut data = Vec::new();
    for i in 0..1000 {
        data.extend_from_slice(format!("{i}\n").as_bytes());
    }
    check("1000 lines on stdin", &[], Some(&data));
}

#[test]
fn large_stdin_exceeding_pipe_buffer() {
    // > 64 KiB, so the writer cannot finish before the child exits. Both
    // programs must still exit 0 and print only the greeting.
    let data = vec![b'x'; 256 * 1024];
    check("256 KiB of stdin", &[], Some(&data));
}

#[test]
fn binary_and_non_utf8_stdin() {
    let data: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
    check("binary/non-UTF-8 stdin", &[], Some(&data));
}

#[test]
fn one_argument() {
    check("single argv element", &["foo"], None);
}

#[test]
fn many_arguments() {
    let args: Vec<String> = (0..64).map(|i| format!("arg{i}")).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    check("64 argv elements", &refs, None);
}

#[test]
fn flag_like_and_empty_and_weird_arguments() {
    // main() takes no parameters, so none of these can be interpreted.
    check("flag-like args", &["-h", "--help", "--version"], None);
    check("empty-string arg", &[""], None);
    check("dash arg", &["-"], None);
    check("spaces and newline in arg", &["a b", "c\nd"], None);
    check("non-numeric arg", &["not-a-number"], None);
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above: stdout that cannot be written
// ---------------------------------------------------------------------------

/// Run `sh -c <script>` with the binary path as $1, capturing stderr and status.
#[cfg(unix)]
fn run_via_sh(script: &str, bin: &Path, cwd: &Path) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(bin)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run sh helper")
}

#[cfg(unix)]
fn tempdir(tag: &str) -> PathBuf {
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = base.join(format!("difftest-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// stdout is the write end of a FIFO that has no reader at all, so the very
/// first write fails. C is killed by SIGPIPE; the Rust program must be too.
///
/// Determinism: `exec 4<>fifo` opens the FIFO read-write so that the
/// subsequent `exec 1>fifo` does not block, then fd 4 is closed *before* the
/// program is exec'd. No reader exists by the time the program writes.
#[test]
#[cfg(unix)]
fn stdout_is_pipe_with_no_reader_sigpipe() {
    let script = r#"
        cd "$(dirname "$0")" 2>/dev/null || true
        rm -f fifo
        mkfifo fifo || exit 111
        exec 4<>fifo
        exec 1>fifo
        exec 4>&-
        exec "$1"
    "#;

    let cdir = tempdir("sigpipe-c");
    let rdir = tempdir("sigpipe-r");
    let c = run_via_sh(script, &c_bin(), &cdir);
    let r = run_via_sh(script, &rust_bin(), &rdir);

    assert_ne!(
        status_string(&c),
        "exit(111)",
        "mkfifo failed; the SIGPIPE case did not actually run"
    );
    // Guard the premise: C really must die on SIGPIPE here. The script ends in
    // `exec`, so the program replaces the shell and the signal is observed
    // directly (rather than being reported by sh as 128+13).
    assert_eq!(
        status_string(&c),
        "signal(13)",
        "expected the C program to be killed by SIGPIPE, got {}",
        status_string(&c)
    );
    assert_same("stdout is a pipe with no reader", &c, &r);

    let _ = std::fs::remove_dir_all(&cdir);
    let _ = std::fs::remove_dir_all(&rdir);
}

/// stdout closed outright (fd 1 not open): the write fails with EBADF. C's
/// printf failure does not change main's return value, so the exit status is
/// still 0 and nothing is printed.
#[test]
#[cfg(unix)]
fn stdout_closed() {
    let script = r#"exec 1>&-; exec "$1""#;
    let cwd = repo_root();
    let c = run_via_sh(script, &c_bin(), &cwd);
    let r = run_via_sh(script, &rust_bin(), &cwd);
    assert_same("stdout closed (EBADF)", &c, &r);
}

/// stdin closed outright: neither program reads it, so nothing changes.
#[test]
#[cfg(unix)]
fn stdin_closed() {
    let script = r#"exec 0<&-; exec "$1""#;
    let cwd = repo_root();
    let c = run_via_sh(script, &c_bin(), &cwd);
    let r = run_via_sh(script, &rust_bin(), &cwd);
    assert_same("stdin closed (EBADF)", &c, &r);
}

/// stdout redirected to a regular file: exercises the fully-buffered stdout
/// path in C (flush happens at exit) and compares the resulting file bytes.
#[test]
#[cfg(unix)]
fn stdout_redirected_to_file() {
    let dir = tempdir("tofile");
    let script = r#"exec 1>out.txt; exec "$1""#;
    let c = run_via_sh(script, &c_bin(), &dir);
    let c_file = std::fs::read(dir.join("out.txt")).expect("read C output file");
    let r = run_via_sh(script, &rust_bin(), &dir);
    let r_file = std::fs::read(dir.join("out.txt")).expect("read Rust output file");

    assert_same("stdout to regular file", &c, &r);
    assert_eq!(
        c_file, r_file,
        "file contents differ\n  C: {}\n  Rust: {}",
        show(&c_file),
        show(&r_file)
    );
    assert_eq!(c_file, b"Hello World!\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// stderr redirected into the same stream as stdout: ordering/interleaving must
/// match (both programs write nothing to stderr).
#[test]
#[cfg(unix)]
fn stderr_merged_into_stdout() {
    let script = r#"exec 2>&1; exec "$1""#;
    let cwd = repo_root();
    let c = run_via_sh(script, &c_bin(), &cwd);
    let r = run_via_sh(script, &rust_bin(), &cwd);
    assert_same("stderr merged into stdout", &c, &r);
}

/// A reader that closes early after the program already wrote: the 13 bytes fit
/// in the pipe buffer, so both programs must exit cleanly.
#[test]
#[cfg(unix)]
fn stdout_piped_to_reader_that_ignores_input() {
    // POSIX sh has no PIPESTATUS, so the program's own status is stashed in a
    // file inside a private temp dir and used as this script's exit status.
    let script = r#"cd "$(dirname "$0")" 2>/dev/null || true
                    { "$1"; echo $? > st; } | cat > /dev/null
                    exit "$(cat st)""#;
    let cdir = tempdir("pipe-c");
    let rdir = tempdir("pipe-r");
    let c = run_via_sh(script, &c_bin(), &cdir);
    let r = run_via_sh(script, &rust_bin(), &rdir);
    assert_same("stdout piped to cat", &c, &r);
    assert_eq!(status_string(&c), "exit(0)");
    let _ = std::fs::remove_dir_all(&cdir);
    let _ = std::fs::remove_dir_all(&rdir);
}

/// `main()` declares no parameters, so even a hostile argv[0] (empty string or
/// a name unrelated to the executable) cannot change the behavior.
#[test]
#[cfg(unix)]
fn unusual_argv0() {
    use std::os::unix::process::CommandExt;

    let run_with_arg0 = |bin: &Path, arg0: &str| -> Output {
        Command::new(bin)
            .arg0(arg0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with custom arg0")
    };

    for arg0 in ["", "weird-name", "-bash", "/nonexistent/path/driver"] {
        let c = run_with_arg0(&c_bin(), arg0);
        let r = run_with_arg0(&rust_bin(), arg0);
        assert_same(&format!("argv[0]={arg0:?}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Environment-related input classes
// ---------------------------------------------------------------------------

fn run_with_env(bin: &Path, envs: &[(&str, &str)], clear: bool) -> Output {
    let mut cmd = Command::new(bin);
    if clear {
        cmd.env_clear();
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn with env")
}

#[test]
fn empty_environment() {
    let c = run_with_env(&c_bin(), &[], true);
    let r = run_with_env(&rust_bin(), &[], true);
    assert_same("cleared environment", &c, &r);
}

#[test]
fn locale_environment_does_not_change_output() {
    for loc in ["C", "POSIX", "en_US.UTF-8", "tr_TR.UTF-8", "de_DE.UTF-8"] {
        let envs = [("LC_ALL", loc), ("LANG", loc)];
        let c = run_with_env(&c_bin(), &envs, false);
        let r = run_with_env(&rust_bin(), &envs, false);
        assert_same(&format!("LC_ALL={loc}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Repeatability
// ---------------------------------------------------------------------------

#[test]
fn output_is_deterministic_across_runs() {
    let first = run(&c_bin(), &[], None);
    for i in 0..25 {
        let c = run(&c_bin(), &[], None);
        let r = run(&rust_bin(), &[], None);
        assert_same(&format!("repeat #{i}"), &c, &r);
        assert_eq!(c.stdout, first.stdout, "C output not deterministic");
    }
}
