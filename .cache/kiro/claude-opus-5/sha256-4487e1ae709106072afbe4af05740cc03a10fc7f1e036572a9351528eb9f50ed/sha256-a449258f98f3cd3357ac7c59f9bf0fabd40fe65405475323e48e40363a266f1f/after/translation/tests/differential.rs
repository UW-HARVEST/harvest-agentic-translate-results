//! Differential tests: run the C program and the Rust program as subprocesses
//! and compare stdout, stderr and exit status byte for byte.
//!
//! Nothing here links the Rust code as a library; both sides are driven exactly
//! the way a shell would drive them.
//!
//! Two pairs of executables are compared:
//!
//! * `driver`  -- c_src/src/main.c vs translation/src/main.rs. This is the
//!   graded program. It takes no input, so there is exactly one invocation of
//!   it, checked under several stdout/stderr redirection layouts.
//!
//! * `probe`   -- tests/cprobe/probe.c vs src/bin/probe.rs. A second driver over
//!   the *same* library sources in c_src/src, added because main.c never reaches
//!   most branches in tree.c / hashmap.c (three of the five reachable error
//!   messages, the "(empty tree)" path, hashmap_clear, NULL data, strncpy
//!   truncation, tombstone reuse, the find_path length clamps, ...). Each
//!   scenario name is an input class; see ERRORS.md.
//!
//! The C side is compiled here with `cc` and no `-DNDEBUG`, matching the plain
//! `cmake .. && cmake --build .` build. That detail matters: CMake's Release
//! preset defines NDEBUG, which deletes every `assert(...)` in main.c -- and
//! because main.c performs almost all of its work *inside* asserts, an NDEBUG
//! build is a hollow program that prints "(empty tree)" and no stderr. c_src is
//! only ever read, never written.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------- paths

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_src() -> PathBuf {
    workspace_root().join("c_src")
}

/// A scratch directory next to the test binary for the compiled C artifacts.
fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .expect("test binary has a parent")
        .join("c-differential");
    std::fs::create_dir_all(&dir).expect("create artifact dir");
    dir
}

// ---------------------------------------------------------------- C build

fn compile_c(out_name: &str, first: PathBuf) -> PathBuf {
    let out = artifact_dir().join(out_name);
    let status = Command::new("cc")
        .arg("-std=c11")
        .arg("-I")
        .arg(c_src().join("include"))
        .arg("-o")
        .arg(&out)
        .arg(first)
        .arg(c_src().join("src").join("hashmap.c"))
        .arg(c_src().join("src").join("tree.c"))
        .status()
        .expect("failed to invoke cc; a C compiler is required for these tests");
    assert!(status.success(), "C compilation of {out_name} failed");
    assert!(out.is_file(), "cc produced no binary for {out_name}");
    out
}

/// The C driver: c_src/src/main.c plus the library.
fn c_driver() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| compile_c("c_driver", c_src().join("src").join("main.c")))
}

/// The C probe: tests/cprobe/probe.c plus the same library.
fn c_probe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        compile_c(
            "c_probe",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("cprobe")
                .join("probe.c"),
        )
    })
}

fn rust_driver() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn rust_probe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_probe"))
}

// ---------------------------------------------------------------- running

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Normal exit code, if the process was not killed by a signal.
    code: Option<i32>,
    /// Terminating signal, if any.
    signal: Option<i32>,
}

fn describe(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Run with stdout and stderr captured on separate pipes.
fn run(exe: &Path, args: &[&str]) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let out = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run with stdout and stderr pointing at the *same* file, which is how the
/// buffering difference between the two streams becomes observable: C's stdout
/// is block-buffered when redirected while stderr is unbuffered, so error lines
/// land ahead of output produced before them.
fn run_combined(exe: &Path, args: &[&str], tag: &str) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let path = artifact_dir().join(format!("combined-{tag}.out"));
    let file = std::fs::File::create(&path).expect("create combined output file");
    let err = file.try_clone().expect("dup file handle");
    let status = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(err))
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));
    let bytes = std::fs::read(&path).expect("read combined output");
    Run {
        stdout: bytes,
        stderr: Vec::new(),
        code: status.code(),
        signal: status.signal(),
    }
}

/// Both streams merged onto one *pipe* rather than a file. glibc picks its
/// buffer size from the target's `st_blksize`, so a pipe is a distinct case.
fn run_combined_pipe(exe: &Path, args: &[&str]) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    let mut merged = Vec::new();
    {
        use std::io::Read;
        let mut so = child.stdout.take().expect("stdout pipe");
        let mut se = child.stderr.take().expect("stderr pipe");
        let mut a = Vec::new();
        let mut b = Vec::new();
        so.read_to_end(&mut a).expect("read stdout");
        se.read_to_end(&mut b).expect("read stderr");
        merged.extend_from_slice(&b);
        merged.extend_from_slice(&a);
    }
    let status = child.wait().expect("wait");
    Run {
        stdout: merged,
        stderr: Vec::new(),
        code: status.code(),
        signal: status.signal(),
    }
}

/// Compare stdout, stderr and exit status of the two programs.
fn assert_identical(label: &str, c_exe: &Path, rust_exe: &Path, args: &[&str]) {
    let c = run(c_exe, args);
    let r = run(rust_exe, args);

    assert_eq!(
        describe(&c.stdout),
        describe(&r.stdout),
        "[{label}] stdout differs"
    );
    assert_eq!(c.stdout, r.stdout, "[{label}] stdout bytes differ");
    assert_eq!(
        describe(&c.stderr),
        describe(&r.stderr),
        "[{label}] stderr differs"
    );
    assert_eq!(c.stderr, r.stderr, "[{label}] stderr bytes differ");
    assert_eq!(c.code, r.code, "[{label}] exit code differs");
    assert_eq!(c.signal, r.signal, "[{label}] terminating signal differs");
}

/// Compare the interleaving of the two streams as well.
fn assert_identical_combined(label: &str, c_exe: &Path, rust_exe: &Path, args: &[&str]) {
    let c = run_combined(c_exe, args, &format!("c-{label}"));
    let r = run_combined(rust_exe, args, &format!("r-{label}"));
    assert_eq!(
        describe(&c.stdout),
        describe(&r.stdout),
        "[{label}] combined stdout+stderr differs (stream interleaving)"
    );
    assert_eq!(c.code, r.code, "[{label}] combined exit code differs");
    assert_eq!(c.signal, r.signal, "[{label}] combined signal differs");

    let cp = run_combined_pipe(c_exe, args);
    let rp = run_combined_pipe(rust_exe, args);
    assert_eq!(
        describe(&cp.stdout),
        describe(&rp.stdout),
        "[{label}] combined output over a pipe differs"
    );
    assert_eq!(cp.code, rp.code, "[{label}] piped exit code differs");
    assert_eq!(cp.signal, rp.signal, "[{label}] piped signal differs");
}

/// Every scenario the probe pair understands. Each is an input class taken from
/// a branch in c_src/src/tree.c or c_src/src/hashmap.c.
const SCENARIOS: &[&str] = &[
    // tree_print with has_root == 0, and queries against an empty tree
    "empty_print",
    // tree_add_node with data == NULL
    "null_data",
    // "Error: Parent node %lu not found"
    "parent_missing",
    // "Error: Node %lu not found"
    "remove_missing",
    // -1 returns from depth/height/descendants/find_path on an absent id
    "queries_missing",
    // the length clamp in tree_find_path, including max_length 0 and negative
    "find_path_clamp",
    // the 1000-entry temp_path bound in tree_find_path
    "find_path_deep",
    // strncpy truncation at MAX_DATA_LENGTH, empty and non-UTF-8 data
    "data_trunc",
    // tombstone reuse and the "update existing" path in hashmap_put
    "hashmap_reuse",
    // a stored NULL value: occupied but reported absent
    "hashmap_null_value",
    // hashmap_clear, never called by the C driver
    "hashmap_clear",
    // repeated hashmap_resize with tombstones counting toward the load factor
    "hashmap_resize",
    // %lu formatting at the extremes of uint64_t
    "big_ids",
    // root_id == 0, indistinguishable from "no parent"
    "zero_root",
    // has_root reset, then re-adding removed ids over tombstones
    "remove_root_readd",
    // the child_ids shifting loop in tree_remove_node
    "child_shift",
    // "Error: Parent has maximum children" at the MAX_CHILDREN boundary
    "max_children",
    // recursive tree_remove_subtree over a wide, deep tree
    "subtree_removal",
    // duplicate ids at the root and deeper
    "dup_and_reinsert",
    // stdout/stderr interleaving around an explicit fflush
    "interleaved",
    // recursion depth in get_height / count_descendants / remove_subtree
    "deep_recursion",
];

// ---------------------------------------------------------------- tests

/// Phase A: both programs must exist and be runnable.
#[test]
fn both_programs_build_and_run() {
    for exe in [c_driver(), rust_driver(), c_probe(), rust_probe()] {
        assert!(exe.is_file(), "{} is not a file", exe.display());
    }
    let c = run(c_driver(), &[]);
    let r = run(rust_driver(), &[]);
    assert!(!c.stdout.is_empty(), "C driver produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust driver produced no stdout");
}

/// Phase B: the graded program, no arguments, streams captured separately.
#[test]
fn driver_matches() {
    assert_identical("driver", c_driver(), rust_driver(), &[]);
}

/// The driver's own stderr must be exactly the two errors its asserts provoke.
/// This pins the NDEBUG trap described at the top of this file: an NDEBUG build
/// of the C side emits no stderr at all and would make the comparison vacuous.
#[test]
fn driver_emits_the_expected_error_lines() {
    let c = run(c_driver(), &[]);
    assert_eq!(
        describe(&c.stderr),
        "Error: Node with ID 2 already exists\nError: Parent has maximum children\n",
        "the C driver under test is not exercising its assert bodies"
    );
    let r = run(rust_driver(), &[]);
    assert_eq!(describe(&c.stderr), describe(&r.stderr));
    assert!(
        describe(&c.stdout).contains("All tests passed successfully!"),
        "the C driver did not reach the end of main"
    );
    // The complex-structure test prints a real tree, not "(empty tree)".
    assert!(
        describe(&c.stdout).contains("[10] ggc1"),
        "the C driver did not print the complex tree"
    );
}

/// The driver ignores argv, so extra arguments must change nothing.
#[test]
fn driver_ignores_arguments() {
    assert_identical("driver-argv", c_driver(), rust_driver(), &["foo", "bar"]);
}

/// Stream interleaving for the graded program.
#[test]
fn driver_matches_with_merged_streams() {
    assert_identical_combined("driver", c_driver(), rust_driver(), &[]);
}

/// Phase C: every enumerated input class, streams captured separately.
#[test]
fn probe_scenarios_match() {
    for s in SCENARIOS {
        assert_identical(s, c_probe(), rust_probe(), &[s]);
    }
}

/// Phase C: the same input classes, checking stream interleaving too.
#[test]
fn probe_scenarios_match_with_merged_streams() {
    for s in SCENARIOS {
        assert_identical_combined(s, c_probe(), rust_probe(), &[s]);
    }
}

/// The probes' own error paths: no argument, and an unrecognised scenario.
#[test]
fn probe_usage_errors_match() {
    assert_identical("probe-no-args", c_probe(), rust_probe(), &[]);
    assert_identical("probe-unknown", c_probe(), rust_probe(), &["nope"]);
}

/// Every scenario must actually produce output, otherwise a passing comparison
/// would prove nothing.
#[test]
fn probe_scenarios_are_not_vacuous() {
    for s in SCENARIOS {
        let c = run(c_probe(), &[s]);
        assert!(
            c.stdout.len() > 20,
            "scenario {s} produced almost no stdout ({} bytes)",
            c.stdout.len()
        );
        assert_eq!(c.code, Some(0), "scenario {s} did not exit 0");
    }
    // and the error-path scenarios must reach stderr
    for s in ["parent_missing", "remove_missing", "max_children", "big_ids"] {
        let c = run(c_probe(), &[s]);
        assert!(!c.stderr.is_empty(), "scenario {s} produced no stderr");
    }
}

/// A write to a pipe with no reader kills the C program with SIGPIPE. The Rust
/// runtime installs SIG_IGN for SIGPIPE before main, which would turn that into
/// a clean exit 0; src/main.rs restores the default disposition.
#[test]
fn sigpipe_exit_status_matches() {
    fn status_with_no_reader(exe: &Path) -> i32 {
        let out = Command::new("bash")
            .arg("-c")
            .arg("\"$1\" 2>/dev/null | true; exit ${PIPESTATUS[0]}")
            .arg("bash")
            .arg(exe)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run bash");
        out.status.code().expect("bash reported no exit code")
    }
    let c = status_with_no_reader(c_driver());
    let r = status_with_no_reader(rust_driver());
    assert_eq!(
        c, r,
        "exit status after writing to a pipe with no reader differs \
         (C={c}, Rust={r}); 141 means killed by SIGPIPE"
    );
    assert_eq!(c, 141, "expected the C program to die from SIGPIPE");
}

/// The same check, driven through Rust rather than a shell, and applied to the
/// probe pair as well.
#[test]
fn sigpipe_signal_matches_without_a_shell() {
    fn kill_signal(exe: &Path, args: &[&str]) -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let mut child = Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        // Close the read end immediately; the child still has to do all of its
        // work before it flushes at exit.
        drop(child.stdout.take());
        let st = child.wait().expect("wait");
        (st.code(), st.signal())
    }
    assert_eq!(
        kill_signal(c_driver(), &[]),
        kill_signal(rust_driver(), &[]),
        "driver disagrees on the outcome of a closed stdout pipe"
    );
    assert_eq!(
        kill_signal(c_probe(), &["hashmap_resize"]),
        kill_signal(rust_probe(), &["hashmap_resize"]),
        "probe disagrees on the outcome of a closed stdout pipe"
    );
}

/// stdout closed outright (EBADF on every write) rather than a dead pipe.
#[test]
fn closed_stdout_matches() {
    use std::os::unix::process::ExitStatusExt;
    fn with_stdout_closed(exe: &Path) -> (Option<i32>, Option<i32>, Vec<u8>) {
        let out = Command::new("bash")
            .arg("-c")
            .arg("\"$1\" >&-")
            .arg("bash")
            .arg(exe)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run bash");
        (out.status.code(), out.status.signal(), out.stderr)
    }
    let c = with_stdout_closed(c_driver());
    let r = with_stdout_closed(rust_driver());
    assert_eq!(c.0, r.0, "exit code with stdout closed differs");
    assert_eq!(c.1, r.1, "signal with stdout closed differs");
    assert_eq!(
        describe(&c.2),
        describe(&r.2),
        "stderr with stdout closed differs"
    );
}

/// A full disk (/dev/full) makes every stdout write fail with ENOSPC. glibc's
/// exit ignores the error, so the status stays 0.
#[test]
fn full_device_stdout_matches() {
    use std::os::unix::process::ExitStatusExt;
    fn to_dev_full(exe: &Path, args: &[&str]) -> (Option<i32>, Option<i32>, Vec<u8>) {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full is required for this test");
        let out = Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(full))
            .stderr(Stdio::piped())
            .output()
            .expect("run");
        (out.status.code(), out.status.signal(), out.stderr)
    }
    let c = to_dev_full(c_driver(), &[]);
    let r = to_dev_full(rust_driver(), &[]);
    assert_eq!(c.0, r.0, "exit code writing to /dev/full differs");
    assert_eq!(c.1, r.1, "signal writing to /dev/full differs");
    assert_eq!(describe(&c.2), describe(&r.2), "stderr differs");

    let c = to_dev_full(c_probe(), &["hashmap_resize"]);
    let r = to_dev_full(rust_probe(), &["hashmap_resize"]);
    assert_eq!(c.0, r.0, "probe exit code writing to /dev/full differs");
    assert_eq!(c.1, r.1, "probe signal writing to /dev/full differs");
}

/// Output must not depend on the locale, which could otherwise alter numeric
/// formatting or the treatment of the non-ASCII box-drawing characters.
#[test]
fn locale_does_not_change_output() {
    fn run_with_locale(exe: &Path, args: &[&str], locale: &str) -> Vec<u8> {
        let out = Command::new(exe)
            .args(args)
            .env("LC_ALL", locale)
            .env("LANG", locale)
            .stdin(Stdio::null())
            .output()
            .expect("run");
        out.stdout
    }
    for locale in ["C", "C.UTF-8", "en_US.UTF-8", "tr_TR.UTF-8", "de_DE.UTF-8"] {
        let c = run_with_locale(c_driver(), &[], locale);
        let r = run_with_locale(rust_driver(), &[], locale);
        assert_eq!(
            describe(&c),
            describe(&r),
            "driver output differs under LC_ALL={locale}"
        );
        let c = run_with_locale(c_probe(), &["big_ids"], locale);
        let r = run_with_locale(rust_probe(), &["big_ids"], locale);
        assert_eq!(
            describe(&c),
            describe(&r),
            "probe output differs under LC_ALL={locale}"
        );
    }
}

/// Repeated runs must be byte-identical: nothing may depend on allocation
/// addresses, hash seeding or iteration order. The C hashmap's slot layout is
/// dumped in full by the probe, so this also pins the FNV-1a hash and the
/// linear-probing order.
#[test]
fn output_is_deterministic_across_runs() {
    let first = run(rust_driver(), &[]);
    for _ in 0..5 {
        let again = run(rust_driver(), &[]);
        assert!(
            first.stdout == again.stdout && first.stderr == again.stderr,
            "Rust driver output is not reproducible across runs"
        );
    }
    let c_first = run(c_probe(), &["hashmap_resize"]);
    for _ in 0..3 {
        let again = run(c_probe(), &["hashmap_resize"]);
        assert!(
            c_first.stdout == again.stdout,
            "C probe output is not reproducible across runs"
        );
    }
    let r_first = run(rust_probe(), &["hashmap_resize"]);
    assert_eq!(
        describe(&c_first.stdout),
        describe(&r_first.stdout),
        "hashmap slot layout differs"
    );
}

/// Guard rail for this whole suite: the comparison must be able to fail. A
/// deliberately altered copy of tree.c (built outside c_src) has to be caught.
/// Without this, a harness that silently compared nothing would look green.
#[test]
fn harness_detects_a_planted_difference() {
    let original = std::fs::read_to_string(c_src().join("src").join("tree.c")).expect("read tree.c");
    assert!(
        original.contains("(empty tree)"),
        "tree.c no longer contains the string this test perturbs"
    );
    let mutated = original.replace("(empty tree)", "(EMPTY TREE)");
    assert_ne!(mutated, original, "mutation did not change anything");

    let dir = artifact_dir();
    let mutated_src = dir.join("tree_mutated.c");
    std::fs::write(&mutated_src, mutated).expect("write mutated copy");

    let out = dir.join("c_probe_mutated");
    let status = Command::new("cc")
        .arg("-std=c11")
        .arg("-I")
        .arg(c_src().join("include"))
        .arg("-o")
        .arg(&out)
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("cprobe")
                .join("probe.c"),
        )
        .arg(c_src().join("src").join("hashmap.c"))
        .arg(&mutated_src)
        .status()
        .expect("cc");
    assert!(status.success(), "failed to build the mutated probe");

    let mutated_run = run(&out, &["empty_print"]);
    let rust_run = run(rust_probe(), &["empty_print"]);
    assert_ne!(
        mutated_run.stdout, rust_run.stdout,
        "the harness cannot tell a modified C program from the Rust one, \
         so its passing results are meaningless"
    );

    // c_src itself must be untouched by all of the above.
    let after = std::fs::read_to_string(c_src().join("src").join("tree.c")).expect("re-read");
    assert_eq!(after, original, "c_src/src/tree.c was modified");
}

/// c_src must be byte-identical before and after the suite runs.
#[test]
fn c_src_is_not_modified() {
    for rel in [
        "src/main.c",
        "src/tree.c",
        "src/hashmap.c",
        "include/tree.h",
        "include/hashmap.h",
        "CMakeLists.txt",
    ] {
        let p = c_src().join(rel);
        assert!(p.is_file(), "{} is missing", p.display());
        let meta = std::fs::metadata(&p).expect("metadata");
        assert!(meta.len() > 0, "{} is empty", p.display());
    }
    // No build products may have been written into the source tree by us.
    let probe_c = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cprobe")
        .join("probe.c");
    assert!(
        probe_c.starts_with(env!("CARGO_MANIFEST_DIR")),
        "the C probe must live inside translation/, not c_src/"
    );
}
