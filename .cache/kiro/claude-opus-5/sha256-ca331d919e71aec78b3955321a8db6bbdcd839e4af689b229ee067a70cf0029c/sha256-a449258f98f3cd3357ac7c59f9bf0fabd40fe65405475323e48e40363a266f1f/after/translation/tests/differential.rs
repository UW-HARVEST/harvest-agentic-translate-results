//! Differential tests: run the C program and the Rust program as subprocesses
//! and compare stdout, stderr and exit status byte for byte.
//!
//! Nothing here loads the Rust code as a library. Both programs are driven the
//! way a shell drives them, because that is how they are compared.
//!
//! ## What "input" means for this program
//!
//! `c_src/src/main.c` declares `int main(void)` and never touches `stdin`,
//! `argv`, the environment, the clock or the RNG (verified by grepping the C
//! source for `scanf`/`fgets`/`getchar`/`read`/`argv`/`argc`/`getenv`/`stdin`/
//! `time`/`rand`/`clock`). It is a fixed test driver: the "input" it branches on
//! is compiled in, and it is exercised in full on every run.
//!
//! Two consequences drive the design of this file:
//!
//! 1. The internal branch classes (empty tree, single node, `MAX_CHILDREN`
//!    saturation, duplicate id, hashmap resize/collision, subtree removal, root
//!    removal, path finding) are all reached by a single run. `landmarks`
//!    asserts each one actually shows up in the observed output, so the suite
//!    fails loudly if a future edit stops exercising one of them rather than
//!    silently comparing less.
//! 2. The axes that *can* vary between invocations are the process-level ones:
//!    stdin contents, argv, environment, working directory, and how stdout and
//!    stderr are wired up. Those are enumerated below. The stream-wiring cases
//!    are the ones with teeth: C's stdout is line buffered on a terminal and
//!    fully buffered otherwise, while stderr is unbuffered, so the *order* of
//!    bytes changes with the wiring, and `SIGPIPE` disposition changes the exit
//!    status.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two programs
// ---------------------------------------------------------------------------

/// The Rust binary under test, built by cargo for this integration test.
///
/// This is the `test`-profile build. `rust_release_binary` covers the
/// `--release` build separately, because that is the artifact a caller running
/// `cargo build --release` ends up comparing against the C program, and the two
/// profiles differ in ways that could matter (`debug_assertions`,
/// `overflow-checks`, and `panic = "abort"`).
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Build and locate the `--release` binary.
///
/// A separate `CARGO_TARGET_DIR` is used so this nested cargo invocation cannot
/// contend with the build lock held by the outer `cargo test`.
fn rust_release_binary() -> &'static Path {
    static REL: OnceLock<PathBuf> = OnceLock::new();

    REL.get_or_init(|| {
        let target = manifest_dir().join("target").join("release-check");
        let out = Command::new(env!("CARGO"))
            .args(["build", "--release", "--bin", "driver"])
            .current_dir(manifest_dir())
            .env("CARGO_TARGET_DIR", &target)
            .env_remove("RUSTFLAGS")
            .output()
            .expect("run cargo build --release");
        assert!(
            out.status.success(),
            "cargo build --release failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bin = target.join("release").join("driver");
        assert!(bin.is_file(), "release binary missing at {}", bin.display());
        bin
    })
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn c_source_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .join("c_src")
}

/// Build `c_src` with the documented commands and return the `driver` path.
///
/// The build type is deliberately left unset, exactly as the reference build
/// (`cmake .. && cmake --build .`) leaves it. That matters: `main.c` wraps calls
/// that have side effects inside `assert(...)`, so a `-DNDEBUG` build would drop
/// every `tree_add_node` call and print `(empty tree)`. Asserts-enabled is the
/// ground truth.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();

    C_BIN.get_or_init(|| {
        let src = c_source_dir();
        // Build outside c_src: that tree is read-only for this exercise.
        let build_dir = manifest_dir().join("target").join("c_build");
        fs::create_dir_all(&build_dir).expect("create C build dir");

        let configure = Command::new("cmake")
            .arg(&src)
            .current_dir(&build_dir)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .arg("--build")
            .arg(".")
            .current_dir(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let bin = build_dir.join("driver");
        assert!(bin.is_file(), "C driver missing at {}", bin.display());
        bin
    })
}

// ---------------------------------------------------------------------------
// Observations
// ---------------------------------------------------------------------------

/// The full wait status of a finished process: normal exit code *and* the
/// terminating signal. Comparing only `code()` would let a `SIGPIPE` death
/// (`None`) pass as equal to a different signal death.
#[derive(Debug, PartialEq, Eq)]
struct Status {
    code: Option<i32>,
    signal: Option<i32>,
}

impl Status {
    fn of(status: std::process::ExitStatus) -> Status {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        Status {
            code: status.code(),
            #[cfg(unix)]
            signal: status.signal(),
            #[cfg(not(unix))]
            signal: None,
        }
    }
}

/// Everything observable about one run.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Status,
}

impl Observed {
    fn from_output(out: Output) -> Observed {
        Observed {
            stdout: out.stdout,
            stderr: out.stderr,
            status: Status::of(out.status),
        }
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Assert two runs of the same scenario agree on all three observables.
fn assert_same(case: &str, c: &Observed, r: &Observed) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}",
        c.stdout.len(),
        show(&c.stdout),
        r.stdout.len(),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr differs\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}",
        c.stderr.len(),
        show(&c.stderr),
        r.stderr.len(),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "[{case}] exit status differs: C={:?} Rust={:?}",
        c.status, r.status
    );
}

/// Scratch directory for per-case temporary files.
fn scratch(case: &str) -> PathBuf {
    let dir = manifest_dir()
        .join("target")
        .join("difftest")
        .join(case.replace(['/', ' '], "_"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

// ---------------------------------------------------------------------------
// Runners, one per way of wiring the process up
// ---------------------------------------------------------------------------

/// Plain run: stdout and stderr on separate pipes. `stdin` is fed from a file so
/// that a large payload cannot deadlock against a program that never reads it.
fn run_plain(prog: &Path, args: &[&str], stdin_file: Option<&Path>, cwd: &Path, env: Env) -> Observed {
    let mut cmd = Command::new(prog);
    cmd.args(args).current_dir(cwd);
    match env {
        Env::Inherit => {}
        Env::Empty => {
            cmd.env_clear();
        }
        Env::Only(pairs) => {
            cmd.env_clear();
            for (k, v) in pairs {
                cmd.env(k, v);
            }
        }
    }
    cmd.stdin(match stdin_file {
        Some(p) => Stdio::from(File::open(p).expect("open stdin file")),
        None => Stdio::null(),
    });
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    Observed::from_output(cmd.output().expect("spawn program"))
}

enum Env {
    Inherit,
    Empty,
    Only(&'static [(&'static str, &'static str)]),
}

/// stdout and stderr each go to their own regular file (fully buffered stdout).
fn run_to_separate_files(prog: &Path, dir: &Path, tag: &str) -> Observed {
    let out_path = dir.join(format!("{tag}.out"));
    let err_path = dir.join(format!("{tag}.err"));
    let status = Command::new(prog)
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&out_path).unwrap()))
        .stderr(Stdio::from(File::create(&err_path).unwrap()))
        .status()
        .expect("spawn program");
    Observed {
        stdout: fs::read(&out_path).unwrap(),
        stderr: fs::read(&err_path).unwrap(),
        status: Status::of(status),
    }
}

/// Both streams share one regular file, the `>file 2>&1` shape. This is where
/// stdout's full buffering versus stderr's unbuffered writes decides the byte
/// order: all stderr text lands before the stdout text that logically precedes
/// it. The merged bytes are reported as `stdout` here; `stderr` is empty.
fn run_merged_to_file(prog: &Path, dir: &Path, tag: &str) -> Observed {
    let path = dir.join(format!("{tag}.both"));
    let file = File::create(&path).unwrap();
    let dup = file.try_clone().unwrap();
    let status = Command::new(prog)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(dup))
        .status()
        .expect("spawn program");
    Observed {
        stdout: fs::read(&path).unwrap(),
        stderr: Vec::new(),
        status: Status::of(status),
    }
}

/// Both streams share one pipe (`prog 2>&1 | reader`). `Command` cannot make
/// stderr the *same* pipe object as stdout, so this goes through a shell.
fn run_merged_to_pipe(prog: &Path) -> Observed {
    let out = Command::new("sh")
        .arg("-c")
        .arg(r#"exec "$1" 2>&1"#)
        .arg("sh")
        .arg(prog)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn via sh");
    Observed::from_output(out)
}

/// Run under a shell fragment. `$1` is the program. Whatever the shell writes on
/// its own stdout/stderr is captured, and the shell's status mirrors the
/// program's (including `128 + signal`).
fn run_in_shell(prog: &Path, script: &str) -> Observed {
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg("bash")
        .arg(prog)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn via bash");
    Observed::from_output(out)
}

/// stdout wired to the given special file, stderr captured.
fn run_stdout_to_device(prog: &Path, device: &str) -> Observed {
    let dev = OpenOptions::new()
        .write(true)
        .open(device)
        .unwrap_or_else(|e| panic!("open {device}: {e}"));
    let child = Command::new(prog)
        .stdin(Stdio::null())
        .stdout(Stdio::from(dev))
        .stderr(Stdio::piped())
        .output()
        .expect("spawn program");
    Observed::from_output(child)
}

// ---------------------------------------------------------------------------
// Phase A: both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_programs_are_runnable() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing: {}", c.display());
    assert!(r.is_file(), "Rust binary missing: {}", r.display());

    let cwd = manifest_dir();
    let co = run_plain(c, &[], None, cwd, Env::Inherit);
    let ro = run_plain(r, &[], None, cwd, Env::Inherit);

    assert!(!co.stdout.is_empty(), "C produced no stdout; nothing to compare");
    assert_same("baseline", &co, &ro);
}

// ---------------------------------------------------------------------------
// Phase B: the branches the C program takes, observed end to end
// ---------------------------------------------------------------------------

/// The single run drives every branch class in the C source. Assert each one is
/// visible in the output, so the comparison cannot quietly shrink.
#[test]
fn landmarks_prove_every_branch_class_is_exercised() {
    let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Inherit);
    let r = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Inherit);
    assert_same("landmarks", &c, &r);

    let out = String::from_utf8(c.stdout.clone()).expect("stdout is UTF-8");
    let err = String::from_utf8(c.stderr.clone()).expect("stderr is UTF-8");

    // Non-ASCII framing: the box-drawing banner and the check/cross marks must
    // survive as the same bytes, not as `?` or replacement characters.
    for needle in [
        "╔════════════════════════════════════════╗",
        "║  TREE WITH HASHMAP ID MAPPING TESTS   ║",
        "╚════════════════════════════════════════╝",
        "✓ PASS: test_hashmap_basic",
    ] {
        assert!(out.contains(needle), "stdout missing {needle:?}");
    }

    // One `✓ PASS:` line per test function in main.c, and no `✗ FAIL:` line.
    assert_eq!(out.matches("✓ PASS:").count(), 14, "expected 14 PASS lines");
    assert!(!out.contains("✗ FAIL:"), "unexpected FAIL line");

    // Branch classes reached on stdout.
    let stdout_classes = [
        // hashmap: put/get/update/remove/contains, and resize under 100 keys
        "=== Testing Hashmap Basic Operations ===",
        "=== Testing Hashmap Collisions ===",
        // empty tree, single node (root), sibling children
        "=== Testing Tree Creation ===",
        "=== Testing Tree Add Root ===",
        "=== Testing Tree Add Children ===",
        // depth/height recursion
        "=== Testing Tree Deep Hierarchy ===",
        // tree_print: indentation and `[%lu] %s` formatting
        "=== Testing Tree Complex Structure ===",
        // removal: leaf, interior subtree, root
        "=== Testing Tree Remove Leaf ===",
        "=== Testing Tree Remove Subtree ===",
        "=== Testing Tree Remove Root ===",
        // queries
        "=== Testing Tree Count Descendants ===",
        "=== Testing Tree Find Path ===",
        // error paths
        "=== Testing Tree Duplicate ID ===",
        "=== Testing Tree Max Children ===",
    ];
    for needle in stdout_classes {
        assert!(out.contains(needle), "stdout missing branch class {needle:?}");
    }

    // `tree_print` output: nested indentation, ids and payloads.
    for line in [
        "[1] root",
        "  [2] child1",
        "    [5] gc1",
        "    [6] gc2",
        "  [3] child2",
        "    [7] gc3",
        "      [10] ggc1",
        "  [4] child3",
        "    [8] gc4",
        "    [9] gc5",
    ] {
        assert!(out.contains(line), "tree_print output missing {line:?}");
    }
    // The `!tree->has_root` arm of tree_print is never reached by this driver.
    assert!(!out.contains("(empty tree)"));

    // Both `fprintf(stderr, ...)` error paths the driver reaches, in order.
    assert_eq!(
        err,
        "Error: Node with ID 2 already exists\nError: Parent has maximum children\n",
        "stderr does not match the two expected error paths"
    );

    // Exit status: `return 0` from main, no signal.
    assert_eq!(
        c.status,
        Status {
            code: Some(0),
            signal: None
        }
    );
}

/// stdin is never read, so anything on it must be ignored identically: empty,
/// one line, no trailing newline, NUL bytes, and a payload far larger than a
/// pipe buffer. (`scanf` would consume across newlines and `fgets` would stop
/// at one; this program does neither, and these cases pin that down.)
#[test]
fn stdin_contents_are_ignored_identically() {
    let dir = scratch("stdin");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("single_line", b"1\n".to_vec()),
        ("single_item_no_newline", b"42".to_vec()),
        ("two_lines", b"hello\nworld\n".to_vec()),
        ("blank_lines", b"\n\n\n".to_vec()),
        ("whitespace_only", b"   \t  \n".to_vec()),
        ("nul_bytes", vec![0u8, 0, b'a', 0, b'\n']),
        ("binary", (0u8..=255).collect()),
        ("negative_numbers", b"-1 -2147483648 -9223372036854775808\n".to_vec()),
        ("huge_numbers", b"99999999999999999999999999\n".to_vec()),
        ("long_line", vec![b'x'; 100_000]),
        ("large_payload", b"0123456789\n".repeat(100_000)),
    ];

    for (name, bytes) in cases {
        let path = dir.join(name);
        File::create(&path).unwrap().write_all(&bytes).unwrap();
        let c = run_plain(c_binary(), &[], Some(&path), manifest_dir(), Env::Inherit);
        let r = run_plain(rust_binary(), &[], Some(&path), manifest_dir(), Env::Inherit);
        assert_same(&format!("stdin/{name}"), &c, &r);
    }
}

/// stdin as a pipe rather than a regular file: a program that probed `isatty`
/// or read a byte would show up here.
#[test]
fn stdin_as_pipe_and_as_closed_fd() {
    // Pipe with data the program must not consume.
    for prog_pair in [(c_binary(), rust_binary())] {
        let mut results = Vec::new();
        for prog in [prog_pair.0, prog_pair.1] {
            let mut child = Command::new(prog)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            let mut sink = child.stdin.take().unwrap();
            // Ignore write errors: the program never reads, so the pipe may
            // fill or be closed under us. Neither is our concern.
            let _ = sink.write_all(b"5\nalpha\nbeta\n");
            drop(sink);
            results.push(Observed::from_output(child.wait_with_output().unwrap()));
        }
        assert_same("stdin/pipe", &results[0], &results[1]);
    }

    // stdin closed outright.
    let c = run_in_shell(c_binary(), r#"exec "$1" <&-"#);
    let r = run_in_shell(rust_binary(), r#"exec "$1" <&-"#);
    assert_same("stdin/closed", &c, &r);
}

/// `main(void)` ignores argv; confirm both agree for none, one, many, empty and
/// non-UTF-8 arguments.
#[test]
fn argv_is_ignored_identically() {
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec![""],
        vec!["-h"],
        vec!["--help"],
        vec!["0"],
        vec!["a", "b", "c"],
        vec!["--", "-1", "999999999999999999999"],
        vec!["\u{2713}", "café", "日本語"],
    ];
    for (i, args) in cases.iter().enumerate() {
        let c = run_plain(c_binary(), args, None, manifest_dir(), Env::Inherit);
        let r = run_plain(rust_binary(), args, None, manifest_dir(), Env::Inherit);
        assert_same(&format!("argv/{i}:{args:?}"), &c, &r);
    }

    // A great many arguments, and one very long argument.
    let many: Vec<String> = (0..500).map(|i| i.to_string()).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let c = run_plain(c_binary(), &many, None, manifest_dir(), Env::Inherit);
    let r = run_plain(rust_binary(), &many, None, manifest_dir(), Env::Inherit);
    assert_same("argv/many", &c, &r);

    let long = "z".repeat(60_000);
    let c = run_plain(c_binary(), &[&long], None, manifest_dir(), Env::Inherit);
    let r = run_plain(rust_binary(), &[&long], None, manifest_dir(), Env::Inherit);
    assert_same("argv/long", &c, &r);
}

/// The UTF-8 banner is emitted as raw bytes by `printf`, so no locale may change
/// it, and an empty environment must not change it either.
#[test]
fn environment_does_not_change_output() {
    const LOCALES: &[&[(&str, &str)]] = &[
        &[("LC_ALL", "C")],
        &[("LC_ALL", "POSIX")],
        &[("LC_ALL", "C.UTF-8")],
        &[("LC_ALL", "en_US.UTF-8"), ("LANG", "en_US.UTF-8")],
        &[("LANG", "de_DE.ISO-8859-1")],
        &[("TERM", "dumb"), ("COLUMNS", "20")],
        &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
    ];

    let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Empty);
    let r = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Empty);
    assert_same("env/empty", &c, &r);

    for pairs in LOCALES {
        let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Only(pairs));
        let r = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Only(pairs));
        assert_same(&format!("env/{pairs:?}"), &c, &r);
    }
}

/// No path is opened, so the working directory must be irrelevant.
#[test]
fn working_directory_does_not_change_output() {
    for cwd in [Path::new("/"), Path::new("/tmp"), manifest_dir(), &c_source_dir()] {
        let c = run_plain(c_binary(), &[], None, cwd, Env::Inherit);
        let r = run_plain(rust_binary(), &[], None, cwd, Env::Inherit);
        assert_same(&format!("cwd/{}", cwd.display()), &c, &r);
    }
}

/// Output is fully deterministic across repeated runs, and identical between the
/// two programs each time. A hash-order or address-dependent difference would
/// surface here.
#[test]
fn repeated_runs_are_byte_identical() {
    let mut c_runs = Vec::new();
    let mut r_runs = Vec::new();
    for i in 0..5 {
        let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Inherit);
        let r = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Inherit);
        assert_same(&format!("repeat/{i}"), &c, &r);
        c_runs.push(c);
        r_runs.push(r);
    }
    for i in 1..c_runs.len() {
        assert_eq!(c_runs[0], c_runs[i], "C run {i} differs from C run 0");
        assert_eq!(r_runs[0], r_runs[i], "Rust run {i} differs from Rust run 0");
    }
}

// ---------------------------------------------------------------------------
// Phase C: stream wiring, buffering order and signal disposition
// ---------------------------------------------------------------------------

/// Separate regular files: stdout is fully buffered in both cases, so this is
/// the same content as the piped baseline but reached through a different glibc
/// buffering decision.
#[test]
fn separate_regular_files_match() {
    let dir = scratch("separate_files");
    let c = run_to_separate_files(c_binary(), &dir, "c");
    let r = run_to_separate_files(rust_binary(), &dir, "r");
    assert_same("streams/separate_files", &c, &r);
}

/// `prog >file 2>&1`: the interleaving is decided by buffering. glibc holds the
/// 1499 bytes of stdout until `exit`, so both stderr lines appear *first* in the
/// merged file even though they are printed in the middle of the run. A Rust
/// translation using line-buffered `println!` would put them in a different
/// place, so this case is the one that pins the buffering emulation down.
#[test]
fn merged_into_one_file_matches_including_order() {
    let dir = scratch("merged_file");
    let c = run_merged_to_file(c_binary(), &dir, "c");
    let r = run_merged_to_file(rust_binary(), &dir, "r");
    assert_same("streams/merged_file", &c, &r);

    let merged = String::from_utf8(c.stdout.clone()).unwrap();
    let first_err = merged
        .find("Error: Node with ID 2 already exists")
        .expect("stderr line present in merged output");
    let banner = merged.find("TREE WITH HASHMAP ID MAPPING TESTS").expect("banner");
    assert!(
        first_err < banner,
        "expected fully buffered stdout to land after unbuffered stderr"
    );
}

/// `prog 2>&1 | reader`: both streams share a pipe.
#[test]
fn merged_into_one_pipe_matches_including_order() {
    let c = run_merged_to_pipe(c_binary());
    let r = run_merged_to_pipe(rust_binary());
    assert_same("streams/merged_pipe", &c, &r);
}

/// Discarding one stream must not perturb the other.
#[test]
fn discarded_streams_match() {
    let c = run_stdout_to_device(c_binary(), "/dev/null");
    let r = run_stdout_to_device(rust_binary(), "/dev/null");
    assert_same("streams/stdout_devnull", &c, &r);

    let c = run_in_shell(c_binary(), r#"exec "$1" 2>/dev/null"#);
    let r = run_in_shell(rust_binary(), r#"exec "$1" 2>/dev/null"#);
    assert_same("streams/stderr_devnull", &c, &r);
}

/// Closed descriptors. C's `printf` fails with `EBADF` and the return value is
/// ignored, so the program still exits 0; the Rust side must do the same rather
/// than panicking on a write error.
#[test]
fn closed_descriptors_match() {
    let c = run_in_shell(c_binary(), r#"exec "$1" >&-"#);
    let r = run_in_shell(rust_binary(), r#"exec "$1" >&-"#);
    assert_same("streams/stdout_closed", &c, &r);

    let c = run_in_shell(c_binary(), r#"exec "$1" 2>&-"#);
    let r = run_in_shell(rust_binary(), r#"exec "$1" 2>&-"#);
    assert_same("streams/stderr_closed", &c, &r);

    let c = run_in_shell(c_binary(), r#"exec "$1" >&- 2>&-"#);
    let r = run_in_shell(rust_binary(), r#"exec "$1" >&- 2>&-"#);
    assert_same("streams/both_closed", &c, &r);
}

/// `/dev/full` makes every write fail with `ENOSPC`. C ignores `printf`'s return
/// value and exits 0; the Rust program must not turn the error into a panic or a
/// non-zero status.
#[test]
fn write_errors_are_ignored_identically() {
    let c = run_stdout_to_device(c_binary(), "/dev/full");
    let r = run_stdout_to_device(rust_binary(), "/dev/full");
    assert_same("streams/dev_full", &c, &r);
}

/// Writing to a pipe whose reader is already gone. A C program starts with
/// `SIGPIPE` at `SIG_DFL` and is killed by the signal (bash reports 141); the
/// Rust runtime installs `SIG_IGN` before `main` runs, which would swallow the
/// failed write and exit 0 instead. `src/main.rs` restores `SIG_DFL` for exactly
/// this reason.
///
/// `| true` is used rather than bash process substitution: with `>(exec true)`
/// the reader's exit races the program's exit-time flush, and the C program wins
/// that race often enough to make the case flaky. With a pipeline the read end
/// is reliably gone by the time either program flushes, so the expected status
/// can be asserted outright.
#[test]
fn sigpipe_disposition_matches() {
    let cases = [
        // stdout is the reader-less pipe; stderr is discarded.
        (
            "stdout",
            r#""$1" 2>/dev/null | true; exit ${PIPESTATUS[0]}"#,
            141,
        ),
        // stderr is the reader-less pipe; stdout is discarded. This one dies on
        // the *first* `fprintf(stderr, ...)`, mid-run, because stderr is
        // unbuffered -- so it also proves stderr is not being buffered.
        (
            "stderr",
            r#""$1" 2>&1 >/dev/null | true; exit ${PIPESTATUS[0]}"#,
            141,
        ),
        // Both streams on the reader-less pipe.
        (
            "merged",
            r#""$1" 2>&1 | true; exit ${PIPESTATUS[0]}"#,
            141,
        ),
    ];

    for (name, script, expected_code) in cases {
        let c = run_in_shell(c_binary(), script);
        let r = run_in_shell(rust_binary(), script);
        assert_same(&format!("signals/sigpipe_{name}"), &c, &r);
        assert_eq!(
            c.status.code,
            Some(expected_code),
            "signals/sigpipe_{name}: expected the C program to die from SIGPIPE"
        );
    }
}

/// A pipeline whose reader consumes only a prefix, and one that consumes
/// everything. Both must agree.
#[test]
fn partial_and_full_pipe_readers_match() {
    for (name, script) in [
        ("head_1", r#""$1" 2>&1 | head -c 1; exit ${PIPESTATUS[0]}"#),
        ("head_100", r#""$1" 2>&1 | head -c 100; exit ${PIPESTATUS[0]}"#),
        ("head_1_line", r#""$1" 2>&1 | head -n 1; exit ${PIPESTATUS[0]}"#),
        ("full_cat", r#""$1" 2>&1 | cat; exit ${PIPESTATUS[0]}"#),
        ("wc", r#""$1" 2>&1 | wc -c; exit ${PIPESTATUS[0]}"#),
    ] {
        let c = run_in_shell(c_binary(), script);
        let r = run_in_shell(rust_binary(), script);
        assert_same(&format!("pipe/{name}"), &c, &r);
    }
}

/// Under a pseudo-terminal glibc switches stdout to line buffering, which
/// interleaves stdout and stderr differently from the fully buffered case.
#[test]
fn pseudo_terminal_matches() {
    let probe = Command::new("script").arg("--version").output();
    let have_script = matches!(probe, Ok(o) if o.status.success());
    assert!(
        have_script,
        "util-linux `script` is required to exercise the line-buffered \
         (terminal) stdout path"
    );

    let run = |prog: &Path| -> Observed {
        let out = Command::new("script")
            .args(["-qec", &prog.display().to_string(), "/dev/null"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn script");
        Observed::from_output(out)
    };

    let c = run(c_binary());
    let r = run(rust_binary());
    assert_same("streams/pty", &c, &r);

    // On a terminal the stderr lines must appear where they are printed, i.e.
    // interleaved, not hoisted to the front as in the fully buffered case.
    let merged = String::from_utf8_lossy(&c.stdout).into_owned();
    let banner = merged.find("TREE WITH HASHMAP ID MAPPING TESTS");
    let first_err = merged.find("Error: Node with ID 2 already exists");
    if let (Some(banner), Some(err)) = (banner, first_err) {
        assert!(
            banner < err,
            "line-buffered stdout should precede the mid-run stderr line"
        );
    }
}

/// stdout to an append-mode file that already has content: the program must not
/// truncate or seek, so the pre-existing bytes stay put.
#[test]
fn append_mode_stdout_matches() {
    let dir = scratch("append");
    let run = |prog: &Path, tag: &str| -> Observed {
        let path = dir.join(format!("{tag}.out"));
        fs::write(&path, b"PREEXISTING\n").unwrap();
        let f = OpenOptions::new().append(true).open(&path).unwrap();
        let out = Command::new(prog)
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        Observed {
            stdout: fs::read(&path).unwrap(),
            stderr: out.stderr,
            status: Status::of(out.status),
        }
    };
    let c = run(c_binary(), "c");
    let r = run(rust_binary(), "r");
    assert_same("streams/append", &c, &r);
    assert!(c.stdout.starts_with(b"PREEXISTING\n"));
}

/// Both programs run concurrently many times over: catches any dependence on
/// shared state, timing or process id.
#[test]
fn concurrent_runs_match() {
    let handles: Vec<_> = (0..8)
        .map(|i| {
            std::thread::spawn(move || {
                let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Inherit);
                let r = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Inherit);
                assert_same(&format!("concurrent/{i}"), &c, &r);
            })
        })
        .collect();
    for h in handles {
        h.join().expect("concurrent case failed");
    }
}

/// The reference C build must leave `NDEBUG` unset. `main.c` puts calls with
/// side effects inside `assert(...)`, so a `-DNDEBUG` build is a different
/// program: every `tree_add_node` disappears and `tree_print` reports
/// `(empty tree)`. This test documents that the binary being compared against
/// really is the asserts-enabled one.
#[test]
fn reference_c_build_has_asserts_enabled() {
    let c = run_plain(c_binary(), &[], None, manifest_dir(), Env::Inherit);
    let out = String::from_utf8_lossy(&c.stdout);
    assert!(
        out.contains("[1] root") && !out.contains("(empty tree)"),
        "the C binary under test looks like an NDEBUG build; configure it with \
         `cmake ..` and no CMAKE_BUILD_TYPE"
    );
}

/// The whole matrix again against the `--release` build, which is the artifact
/// `cargo build --release` produces and therefore the one actually compared
/// against the C program. `panic = "abort"` and the absence of
/// `debug_assertions`/`overflow-checks` make it a genuinely different binary
/// from the one `CARGO_BIN_EXE_driver` points at.
#[test]
fn release_profile_matches_across_the_matrix() {
    let c = c_binary();
    let r = rust_release_binary();
    let dir = scratch("release");

    // Baseline, separate pipes.
    assert_same(
        "release/baseline",
        &run_plain(c, &[], None, manifest_dir(), Env::Inherit),
        &run_plain(r, &[], None, manifest_dir(), Env::Inherit),
    );

    // argv and env ignored.
    assert_same(
        "release/argv",
        &run_plain(c, &["a", "b"], None, manifest_dir(), Env::Inherit),
        &run_plain(r, &["a", "b"], None, manifest_dir(), Env::Inherit),
    );
    assert_same(
        "release/env_empty",
        &run_plain(c, &[], None, manifest_dir(), Env::Empty),
        &run_plain(r, &[], None, manifest_dir(), Env::Empty),
    );

    // stdin ignored, including a payload larger than a pipe buffer.
    let stdin_path = dir.join("stdin");
    File::create(&stdin_path)
        .unwrap()
        .write_all(&b"0123456789\n".repeat(100_000))
        .unwrap();
    assert_same(
        "release/stdin",
        &run_plain(c, &[], Some(&stdin_path), manifest_dir(), Env::Inherit),
        &run_plain(r, &[], Some(&stdin_path), manifest_dir(), Env::Inherit),
    );

    // Regular files, separate and merged (buffering order).
    assert_same(
        "release/separate_files",
        &run_to_separate_files(c, &dir, "c_sep"),
        &run_to_separate_files(r, &dir, "r_sep"),
    );
    assert_same(
        "release/merged_file",
        &run_merged_to_file(c, &dir, "c_merged"),
        &run_merged_to_file(r, &dir, "r_merged"),
    );
    assert_same(
        "release/merged_pipe",
        &run_merged_to_pipe(c),
        &run_merged_to_pipe(r),
    );

    // Devices and closed descriptors.
    assert_same(
        "release/devnull",
        &run_stdout_to_device(c, "/dev/null"),
        &run_stdout_to_device(r, "/dev/null"),
    );
    assert_same(
        "release/dev_full",
        &run_stdout_to_device(c, "/dev/full"),
        &run_stdout_to_device(r, "/dev/full"),
    );
    for script in [
        r#"exec "$1" >&-"#,
        r#"exec "$1" 2>&-"#,
        r#"exec "$1" >&- 2>&-"#,
        r#"exec "$1" <&-"#,
    ] {
        assert_same(
            &format!("release/shell:{script}"),
            &run_in_shell(c, script),
            &run_in_shell(r, script),
        );
    }

    // SIGPIPE disposition.
    for (name, script) in [
        ("stdout", r#""$1" 2>/dev/null | true; exit ${PIPESTATUS[0]}"#),
        ("stderr", r#""$1" 2>&1 >/dev/null | true; exit ${PIPESTATUS[0]}"#),
        ("merged", r#""$1" 2>&1 | true; exit ${PIPESTATUS[0]}"#),
    ] {
        let co = run_in_shell(c, script);
        let ro = run_in_shell(r, script);
        assert_same(&format!("release/sigpipe_{name}"), &co, &ro);
        assert_eq!(co.status.code, Some(141), "release/sigpipe_{name}");
    }

    // Line-buffered (terminal) stdout.
    let pty = |prog: &Path| -> Observed {
        Observed::from_output(
            Command::new("script")
                .args(["-qec", &prog.display().to_string(), "/dev/null"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("spawn script"),
        )
    };
    assert_same("release/pty", &pty(c), &pty(r));
}

/// The release build must be byte-for-byte equivalent in behaviour to the test
/// build, so the debug-profile matrix above genuinely covers the shipped binary.
#[test]
fn release_and_test_profile_behave_identically() {
    let a = run_plain(rust_binary(), &[], None, manifest_dir(), Env::Inherit);
    let b = run_plain(rust_release_binary(), &[], None, manifest_dir(), Env::Inherit);
    assert_eq!(a, b, "debug and release Rust builds disagree");
}
