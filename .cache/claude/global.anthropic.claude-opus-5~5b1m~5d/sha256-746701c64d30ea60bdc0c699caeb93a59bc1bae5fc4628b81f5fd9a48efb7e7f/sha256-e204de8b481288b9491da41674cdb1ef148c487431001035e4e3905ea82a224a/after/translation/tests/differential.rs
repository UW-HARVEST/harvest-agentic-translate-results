//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses on identical stdin, and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C program, building it with CMake on first use.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );
    assert!(exe.exists(), "C binary missing after build: {:?}", exe);
    exe
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

/// `fork(2)` copies the whole descriptor table, so a child spawned by another
/// test thread can inherit — and thereby keep alive — the pipe used by
/// `closed_stdout_pipe_matches_c_sigpipe_behaviour`, suppressing the `EPIPE`
/// that test depends on. Ordinary spawns take this lock for *read* (they stay
/// parallel with each other); the SIGPIPE test takes it for *write*, so no
/// `fork` can happen while its pipe exists.
static SPAWN_LOCK: std::sync::RwLock<()> = std::sync::RwLock::new(());

fn spawn_guard() -> std::sync::RwLockReadGuard<'static, ()> {
    SPAWN_LOCK.read().unwrap_or_else(|e| e.into_inner())
}

fn exclusive_spawn_guard() -> std::sync::RwLockWriteGuard<'static, ()> {
    SPAWN_LOCK.write().unwrap_or_else(|e| e.into_inner())
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = {
        let _g = spawn_guard();
        Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {:?}: {e}", exe))
    };

    {
        let mut si = child.stdin.take().expect("stdin pipe");
        // The child may exit before consuming everything; a write error here is
        // not a test failure.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 400 {
        s.push_str("...<truncated>");
    }
    s
}

/// Core assertion: identical stdout, stderr and exit status for one input.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  input: {}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  input: {}\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] exit status differs (code, signal)\n  input: {}",
        show(stdin_bytes)
    );
}

// ---------------------------------------------------------------------------
// Branch coverage of multi_stage(): the four outcomes reachable through
// scanf("%d %d %d", &x, &y, &z).
//
// Note `y` starts at 123 and is the *second* scanf target, so an input that
// supplies fewer than two integers leaves y == 123.
// ---------------------------------------------------------------------------

#[test]
fn all_stages_pass() {
    // x==1, y==2, z==3 -> "Ok!" / Result: 0
    assert_same("ok", b"1 2 3");
    assert_same("ok_trailing_newline", b"1 2 3\n");
    assert_same("ok_extra_tokens", b"1 2 3 4 5");
    assert_same("ok_multiple_spaces", b"1   2     3");
}

#[test]
fn stage1_x_not_one() {
    assert_same("x_zero", b"0 2 3");
    assert_same("x_two", b"2 2 3");
    assert_same("x_negative", b"-1 2 3");
    assert_same("x_large", b"1000000 2 3");
}

#[test]
fn stage2_y_not_two() {
    assert_same("y_five", b"1 5 3");
    assert_same("y_zero", b"1 0 3");
    assert_same("y_negative", b"1 -2 3");
    assert_same("y_default_123", b"1 123 3");
}

#[test]
fn stage3_z_not_three() {
    assert_same("z_five", b"1 2 5");
    assert_same("z_zero", b"1 2 0");
    assert_same("z_negative", b"1 2 -3");
}

// ---------------------------------------------------------------------------
// Partial and failed scanf conversions. scanf leaves the remaining
// destinations untouched, so x/z keep their initialisers (0) and y keeps 123.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input() {
    assert_same("empty", b"");
    assert_same("single_newline", b"\n");
    assert_same("spaces_only", b"     ");
    assert_same("mixed_whitespace_only", b" \t\n\r\x0b\x0c ");
}

#[test]
fn one_integer_only() {
    // x set, y stays 123 -> stage 2 error.
    assert_same("just_one", b"1");
    assert_same("just_one_nl", b"1\n");
    assert_same("just_one_not_1", b"7");
}

#[test]
fn two_integers_only() {
    // x and y set, z stays 0 -> stage 3 error.
    assert_same("two", b"1 2");
    assert_same("two_nl", b"1 2\n");
    assert_same("two_bad_y", b"1 9");
}

#[test]
fn matching_failure_on_each_conversion() {
    assert_same("first_alpha", b"abc");
    assert_same("first_alpha_then_ints", b"abc 2 3");
    assert_same("second_alpha", b"1 abc");
    assert_same("third_alpha", b"1 2 abc");
    assert_same("dash_only", b"-");
    assert_same("plus_only", b"+");
    assert_same("dash_then_alpha", b"-x 2 3");
    assert_same("plus_then_alpha", b"+x 2 3");
    assert_same("dash_space_int", b"1 - 3");
    assert_same("punctuation", b"!!! ??? ...");
}

#[test]
fn scanf_reads_across_newlines() {
    // "%d" skips any run of whitespace, including newlines, unlike fgets.
    assert_same("one_per_line", b"1\n2\n3\n");
    assert_same("leading_newlines", b"\n\n\n1 2 3");
    assert_same("blank_lines_between", b"1\n\n\n2\n\n\n3");
    assert_same("tabs", b"\t1\t\t2\t3");
    assert_same("carriage_returns", b"1\r2\r3");
    assert_same("vtab_formfeed", b"1\x0b2\x0c3");
    assert_same("split_after_two_lines", b"1\n2\n");
}

#[test]
fn numeric_prefix_parsing() {
    // %d is decimal only: "0x1" scans 0 and stops at 'x'.
    assert_same("hex_literal", b"0x1 2 3");
    assert_same("float", b"1.5 2 3");
    assert_same("exponent", b"1e5 2 3");
    assert_same("digits_then_alpha", b"1 2 3abc");
    assert_same("leading_zeros", b"0000001 0000002 0000003");
    assert_same("many_leading_zeros", b"000000000000000000000001 2 3");
    assert_same("negative_zero", b"-0 2 3");
    assert_same("plus_signs", b"+1 +2 +3");
    assert_same("underscore", b"1_2 2 3");
    assert_same("comma_separated", b"1,2,3");
}

#[test]
fn integer_limits_and_overflow() {
    assert_same("int_max", b"2147483647 2 3");
    assert_same("int_max_plus_one", b"2147483648 2 3");
    assert_same("int_min", b"-2147483648 2 3");
    assert_same("int_min_minus_one", b"-2147483649 2 3");
    assert_same("long_max", b"9223372036854775807 2 3");
    assert_same("long_max_plus_one", b"9223372036854775808 2 3");
    assert_same("long_min", b"-9223372036854775808 2 3");
    assert_same("long_min_minus_one", b"-9223372036854775809 2 3");
    assert_same("huge_positive", b"99999999999999999999999999 2 3");
    assert_same("huge_negative", b"-99999999999999999999999999 2 3");
    // Overflow in y and z positions too.
    assert_same("y_overflow", b"1 99999999999999999999 3");
    assert_same("z_overflow", b"1 2 99999999999999999999");
    // 4294967297 truncates to 1 in an int: exercises truncation, not saturation.
    assert_same("wraps_to_one", b"4294967297 4294967298 4294967299");
}

#[test]
fn non_ascii_and_nul_bytes() {
    assert_same("nul_between", b"1\x002 3");
    assert_same("nul_first", b"\x00 1 2 3");
    assert_same("high_byte_first", b"\xff 2 3");
    assert_same("utf8_second", b"1 \xc3\xa9 3");
    assert_same("all_high_bytes", b"\x80\x81\x82");
}

#[test]
fn buffer_refill_boundaries() {
    // The Rust scanner reads stdin in 4096-byte chunks; make tokens and
    // whitespace runs straddle those boundaries.
    for pad in [4093usize, 4094, 4095, 4096, 4097, 8191, 8192, 8193] {
        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"11 2 3");
        assert_same(&format!("pad_{pad}_token"), &input);

        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"1 2 3");
        assert_same(&format!("pad_{pad}_ok"), &input);
    }
    // A single token whose digits span several chunks.
    let mut input = vec![b'0'; 10000];
    input.extend_from_slice(b"1 2 3");
    assert_same("ten_thousand_leading_zeros", &input);
}

#[test]
fn very_large_inputs() {
    let mut input = vec![b'\n'; 100_000];
    input.extend_from_slice(b"1 2 3");
    assert_same("100k_newlines", &input);

    let mut input = vec![b'9'; 1_000_000];
    input.extend_from_slice(b" 2 3");
    assert_same("one_million_digits", &input);

    // Huge trailing garbage after a complete conversion: scanf stops early.
    let mut input = b"1 2 3 ".to_vec();
    input.extend(std::iter::repeat_n(b'x', 500_000));
    assert_same("trailing_garbage_500k", &input);
}

// ---------------------------------------------------------------------------
// Environmental error paths. The C code ignores printf failures and always
// `return 0`s, and keeps libc's default SIGPIPE disposition.
// ---------------------------------------------------------------------------

/// Deterministic pseudo-random sweep over the byte alphabet that actually
/// matters to `scanf("%d %d %d", ...)`, to catch parsing divergences no
/// hand-written case anticipated.
#[test]
fn randomized_sweep() {
    const ALPHABET: &[u8] = b"0123456789      \n\n\t-+.xeE,abFF\x00\xff\r\x0b\x0c";
    let mut state: u64 = 0x243F_6A88_85A3_08D3; // fixed seed: reproducible
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..600u32 {
        let len = (next() % 24) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
        }
        assert_same(&format!("random_{case}"), &input);
    }
}

/// Same idea, but biased towards well-formed integer triples so the successful
/// and near-miss branches of `multi_stage` are hit repeatedly.
#[test]
fn randomized_integer_triples() {
    let mut state: u64 = 0x13198A2E_03707344;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let interesting: [i64; 12] = [
        0,
        1,
        2,
        3,
        -1,
        -3,
        123,
        2147483647,
        -2147483648,
        2147483648,
        4294967297,
        9223372036854775807,
    ];
    let separators = [" ", "\n", "\t", "  ", "\n\n", " \t ", "\r\n"];

    for case in 0..400u32 {
        let pick = |n: u64| interesting[(n % interesting.len() as u64) as usize];
        let sep = |n: u64| separators[(n % separators.len() as u64) as usize];
        let input = format!(
            "{}{}{}{}{}",
            pick(next()),
            sep(next()),
            pick(next()),
            sep(next()),
            pick(next())
        );
        assert_same(&format!("triple_{case}"), input.as_bytes());
    }
}

/// Minimal libc bindings so these tests need no extra dependencies.
mod sys {
    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    extern "C" {
        fn pipe2(fds: *mut i32, flags: i32) -> i32;
    }
    const O_CLOEXEC: i32 = 0o2000000;

    /// Creates a pipe whose ends are close-on-exec, so unrelated children that
    /// happen to fork cannot keep either end open past their `exec`.
    pub fn make_pipe() -> (i32, i32) {
        let mut fds = [0i32; 2];
        let rc = unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) };
        if rc == 0 {
            return (fds[0], fds[1]);
        }
        let rc = unsafe { pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        (fds[0], fds[1])
    }

    pub fn close_fd(fd: i32) {
        unsafe { close(fd) };
    }
}

/// (exit code, terminating signal, stderr) — the observable result when stdout
/// is not a plain pipe we can capture.
type SideEffects = (Option<i32>, Option<i32>, Vec<u8>);

fn run_with_stdout(exe: &Path, stdout: Stdio, stdin_bytes: &[u8]) -> SideEffects {
    let mut child = {
        let _g = spawn_guard();
        Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"))
    };
    {
        let mut si = child.stdin.take().expect("stdin pipe");
        let _ = si.write_all(stdin_bytes);
    }
    let out = child.wait_with_output().expect("wait");
    (out.status.code(), out.status.signal(), out.stderr)
}

#[test]
fn stdout_to_dev_null_matches() {
    let c = run_with_stdout(&c_bin(), Stdio::null(), b"1 2 3");
    let r = run_with_stdout(&rust_bin(), Stdio::null(), b"1 2 3");
    assert_eq!(c, r, "stdout=/dev/null diverged: C={c:?} Rust={r:?}");
}

#[test]
fn stdout_write_error_is_ignored_like_c() {
    // /dev/full accepts an open() but fails every write() with ENOSPC. The C
    // program never checks printf's result and still exits 0.
    let full = Path::new("/dev/full");
    assert!(
        full.exists(),
        "/dev/full is required to exercise the write-error path on this platform"
    );

    let mut results = Vec::new();
    for exe in [c_bin(), rust_bin()] {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open(full)
            .expect("open /dev/full");
        results.push(run_with_stdout(&exe, Stdio::from(sink), b"1 2 3"));
    }
    assert_eq!(
        results[0], results[1],
        "/dev/full diverged: C={:?} Rust={:?}",
        results[0], results[1]
    );
    assert_eq!(results[0].0, Some(0), "C is expected to still exit 0");
}

#[test]
fn closed_stdout_pipe_matches_c_sigpipe_behaviour() {
    // Hand the child the write end of a pipe whose read end we close at once.
    // libc's default SIGPIPE disposition kills the C program with signal 13;
    // the Rust program must die identically rather than panicking on EPIPE.
    use std::os::unix::io::FromRawFd;

    // Resolve the binaries before taking the lock (c_bin() may spawn cmake).
    let (cb, rb) = (c_bin(), rust_bin());
    let mut results = Vec::new();
    // Exclusive: no other thread may fork while these pipes exist.
    let _excl = exclusive_spawn_guard();
    for exe in [cb, rb] {
        let (reader, writer) = sys::make_pipe();
        let child_stdout = unsafe { Stdio::from_raw_fd(writer) };
        // Close the read end before the child writes anything.
        sys::close_fd(reader);
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(child_stdout)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"));
        {
            let mut si = child.stdin.take().expect("stdin pipe");
            let _ = si.write_all(b"1 2 3");
        }
        let out = child.wait_with_output().expect("wait");
        results.push((out.status.code(), out.status.signal(), out.stderr));
    }
    assert_eq!(
        results[0], results[1],
        "closed-pipe stdout diverged: C={:?} Rust={:?}",
        results[0], results[1]
    );
    assert_eq!(
        results[0].1,
        Some(13),
        "C is expected to be killed by SIGPIPE, got {:?}",
        results[0]
    );
}

#[test]
fn stdin_closed_behaves_like_eof() {
    use std::os::unix::io::FromRawFd;

    let (cb, rb) = (c_bin(), rust_bin());
    let mut results = Vec::new();
    // Exclusive: a fork elsewhere could inherit the write end and stop the
    // child from ever seeing EOF.
    let _excl = exclusive_spawn_guard();
    for exe in [cb, rb] {
        // An immediately-closed stdin pipe: scanf sees EOF straight away.
        let (reader, writer) = sys::make_pipe();
        sys::close_fd(writer);
        let out = Command::new(&exe)
            .stdin(unsafe { Stdio::from_raw_fd(reader) })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run");
        results.push((
            out.stdout,
            out.stderr,
            out.status.code(),
            out.status.signal(),
        ));
    }
    assert_eq!(
        results[0], results[1],
        "closed stdin diverged: C={:?} Rust={:?}",
        results[0], results[1]
    );
}
