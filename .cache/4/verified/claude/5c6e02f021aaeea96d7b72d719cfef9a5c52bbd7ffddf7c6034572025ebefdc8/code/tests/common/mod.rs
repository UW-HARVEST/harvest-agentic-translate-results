//! Shared support code for the C-vs-Rust differential tests.
//!
//! Both implementations are always reached through `dlopen`/`dlsym`
//! (`libloading`) on their shared objects — the Rust functions are never called
//! directly — so the `#[no_mangle] extern "C"` export wrappers in `src/lib.rs`
//! are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every C stdio stream, including the `stdout`
    /// buffer that the dlopened C shared object writes into (it shares the
    /// process' single libc instance with this test binary).
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Fixed seed so that every randomized row is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

// ---------------------------------------------------------------------------
// paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>` — derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    dir
}

/// Build the Rust `cdylib` and the `driver` binary for the profile this test
/// binary was compiled for.
///
/// This is **not** redundant: `cargo test` compiles the library only as an
/// `rlib` (the form the test targets link against) and never refreshes the
/// `cdylib`, so without this step the differential tests happily load a
/// `libdriver.so` left over from an earlier build and a regression in `src/`
/// would pass unnoticed. Verified by mutation testing.
///
/// `run_all.sh` exports `DRIVER_LIB_BUILD_ARGS` with the exact feature flags of
/// the combination under test so the rebuilt `.so` matches the test binary's
/// configuration.
fn ensure_rust_artifacts_built() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let profile_dir = target_profile_dir();
        let is_release = profile_dir
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);

        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.current_dir(manifest_dir())
            .args(["build", "--offline", "--lib", "--bins"]);
        if is_release {
            cmd.arg("--release");
        }
        if let Ok(extra) = std::env::var("DRIVER_LIB_BUILD_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }
        let out = cmd.output().expect("spawn cargo build --lib --bins");
        assert!(
            out.status.success(),
            "`cargo build --lib --bins` failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

/// Fail loudly if `artifact` predates any file in `src/` — a stale artifact
/// would make every differential row pass vacuously.
fn assert_not_stale(artifact: &Path) {
    let art = artifact
        .metadata()
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", artifact.display()));
    let src = manifest_dir().join("src");
    for entry in std::fs::read_dir(&src).expect("read_dir src") {
        let entry = entry.expect("dir entry");
        if entry.path().extension().map(|e| e != "rs").unwrap_or(true) {
            continue;
        }
        let m = entry
            .metadata()
            .and_then(|m| m.modified())
            .expect("stat source");
        assert!(
            art >= m,
            "{} is OLDER than {} — the artifact under test is stale, so the \
             differential results would be meaningless. Run `cargo build` (or \
             ./run_all.sh) first.",
            artifact.display(),
            entry.path().display()
        );
    }
}

/// The Rust `cdylib` under test.
pub fn rust_so_path() -> PathBuf {
    ensure_rust_artifacts_built();
    let p = target_profile_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "missing {} — build it with `cargo build` (crate-type = cdylib)",
        p.display()
    );
    assert_not_stale(&p);
    p
}

/// The `driver` executable produced from `src/main.rs`.
pub fn rust_exe_path() -> PathBuf {
    ensure_rust_artifacts_built();
    let p = target_profile_dir().join("driver");
    assert!(p.exists(), "missing {}", p.display());
    assert_not_stale(&p);
    p
}

pub fn c_source() -> PathBuf {
    manifest_dir().join("c_src/src/main.c")
}

fn c_build_dir() -> PathBuf {
    let d = manifest_dir().join("c_build");
    std::fs::create_dir_all(&d).expect("create c_build");
    d
}

/// Compile the *unmodified* `c_src/src/main.c` into a shared object.
///
/// `c_src/CMakeLists.txt` only declares `add_executable`, so the shared library
/// is produced directly by gcc using CMake's default (unoptimised) flags.
/// Nothing inside `c_src/` is modified — only build products under `c_build/`
/// are created. Written to a unique temporary name and then atomically renamed
/// so that concurrently running test binaries cannot observe a partial file.
pub fn build_c_so(opt: &str) -> PathBuf {
    let tag = if opt.is_empty() { "default" } else { &opt[1..] };
    let out = c_build_dir().join(format!("libc_driver_{tag}.so"));

    let fresh = match (out.metadata(), c_source().metadata()) {
        (Ok(o), Ok(s)) => match (o.modified(), s.modified()) {
            (Ok(om), Ok(sm)) => om >= sm,
            _ => false,
        },
        _ => false,
    };
    if fresh {
        return out;
    }

    let tmp = c_build_dir().join(format!(
        "libc_driver_{tag}.{}.{}.tmp.so",
        std::process::id(),
        next_unique()
    ));
    let mut cmd = Command::new("gcc");
    cmd.arg("-shared").arg("-fPIC");
    if !opt.is_empty() {
        cmd.arg(opt);
    }
    cmd.arg("-o").arg(&tmp).arg(c_source());
    let st = cmd.output().expect("failed to spawn gcc");
    assert!(
        st.status.success(),
        "gcc failed for {opt:?}: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    let _ = std::fs::rename(&tmp, &out);
    let _ = std::fs::remove_file(&tmp);
    assert!(out.exists(), "gcc produced no {}", out.display());
    out
}

/// The reference C shared object (CMake default flags: no `-O`).
pub fn c_so_path() -> PathBuf {
    build_c_so("")
}

/// Compile the unmodified C source into an executable (mirrors
/// `add_executable(driver src/main.c)`); prefers the CMake build output.
pub fn c_exe_path() -> PathBuf {
    let cmake_out = manifest_dir().join("c_src/build/driver");
    if cmake_out.exists() {
        return cmake_out;
    }
    let out = c_build_dir().join("driver_c");
    if out.exists() {
        return out;
    }
    let tmp = c_build_dir().join(format!("driver_c.{}.tmp", std::process::id()));
    let st = Command::new("gcc")
        .arg("-o")
        .arg(&tmp)
        .arg(c_source())
        .output()
        .expect("failed to spawn gcc");
    assert!(
        st.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    let _ = std::fs::rename(&tmp, &out);
    let _ = std::fs::remove_file(&tmp);
    out
}

static UNIQUE: AtomicU64 = AtomicU64::new(0);
fn next_unique() -> u64 {
    UNIQUE.fetch_add(1, Ordering::Relaxed)
}

pub fn temp_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cdiff-{prefix}-{}-{}",
        std::process::id(),
        next_unique()
    ))
}

// ---------------------------------------------------------------------------
// libloading handles
// ---------------------------------------------------------------------------

/// A dlopen'ed pair of implementations plus the typed entry points.
pub struct Pair {
    pub c: libloading::Library,
    pub rust: libloading::Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        Pair::load_with(&c_so_path())
    }

    pub fn load_with(c_path: &Path) -> Pair {
        let rust_path = rust_so_path();
        unsafe {
            Pair {
                c: libloading::Library::new(c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rust: libloading::Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display())),
                c_path: c_path.to_path_buf(),
                rust_path,
            }
        }
    }

    pub fn print_line(&self, which: Which) -> libloading::Symbol<'_, unsafe extern "C" fn(*const c_char)> {
        unsafe { self.lib(which).get(b"printLine\0").expect("dlsym printLine") }
    }

    pub fn print_int_line(&self, which: Which) -> libloading::Symbol<'_, unsafe extern "C" fn(c_int)> {
        unsafe {
            self.lib(which)
                .get(b"printIntLine\0")
                .expect("dlsym printIntLine")
        }
    }

    pub fn bad(&self, which: Which) -> libloading::Symbol<'_, unsafe extern "C" fn()> {
        unsafe { self.lib(which).get(b"bad\0").expect("dlsym bad") }
    }

    pub fn good(&self, which: Which) -> libloading::Symbol<'_, unsafe extern "C" fn()> {
        unsafe { self.lib(which).get(b"good\0").expect("dlsym good") }
    }

    pub fn main_fn(&self, which: Which) -> libloading::Symbol<'_, unsafe extern "C" fn() -> c_int> {
        unsafe { self.lib(which).get(b"main\0").expect("dlsym main") }
    }

    fn lib(&self, which: Which) -> &libloading::Library {
        match which {
            Which::C => &self.c,
            Which::Rust => &self.rust,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

impl Which {
    pub const BOTH: [Which; 2] = [Which::C, Which::Rust];
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with file descriptor 1 redirected to a temporary file and return the
/// bytes it produced.
///
/// * The dlopened C object buffers through libc stdio, so `fflush(NULL)` is
///   issued before the descriptor is restored.
/// * The dlopened Rust cdylib carries its own copy of `std`, whose `stdout` is
///   an unconditional `LineWriter`, so each emitted line reaches fd 1 as soon
///   as the trailing `\n` is written; `c_main` additionally flushes explicitly.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = temp_path("out");
    let file = File::create(&path).expect("create capture file");

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    data
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn range_usize(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below((hi_inclusive - lo + 1) as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    /// Random bytes, never containing NUL (so the result is a valid C string).
    pub fn c_string_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let b = (self.next_u32() & 0xFF) as u8;
                if b == 0 {
                    b'Z'
                } else {
                    b
                }
            })
            .collect()
    }

    pub fn ascii_printable(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| 0x20u8 + (self.below(95) as u8))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// diff helpers
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(160) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 160 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 160));
    }
    s
}

#[track_caller]
pub fn assert_same(row: &str, case: &str, c_out: &[u8], rust_out: &[u8]) {
    assert!(
        c_out == rust_out,
        "[{row}] divergence for {case}\n  C    ({:>5} bytes): {}\n  Rust ({:>5} bytes): {}",
        c_out.len(),
        show(c_out),
        rust_out.len(),
        show(rust_out)
    );
}

// ---------------------------------------------------------------------------
// stdin corpus for the scanf("%d") pipeline
// ---------------------------------------------------------------------------

/// Every stdin shape the C `scanf("%d", &x)` + `if (x)` pipeline distinguishes
/// (CONFIGS.md rows 20-36). Returned as `(row, case, stdin bytes)` so that the
/// `main` differential tests, the executable comparison and the scanf probe can
/// all replay the identical corpus.
pub fn stdin_corpus() -> Vec<(&'static str, String, Vec<u8>)> {
    let mut v: Vec<(&'static str, String, Vec<u8>)> = Vec::new();
    let mut push = |row: &'static str, case: String, bytes: Vec<u8>| v.push((row, case, bytes));

    // Row 20 / ERRORS 9 — empty stdin, immediate EOF.
    push("row20/ERR9", "empty".into(), b"".to_vec());

    // Row 21 / ERRORS 10 — whitespace only (the six isspace bytes).
    for (name, s) in [
        ("space", " "),
        ("tab", "\t"),
        ("newline", "\n"),
        ("vtab", "\x0b"),
        ("formfeed", "\x0c"),
        ("cr", "\r"),
        ("all-ws", " \t\n\x0b\x0c\r"),
        ("many-nl", "\n\n\n\n"),
        ("spaces", "        "),
    ] {
        push("row21/ERR10", format!("ws-only {name}"), s.as_bytes().to_vec());
    }
    {
        let mut rng = Rng::new(SEED ^ 0x21);
        let ws = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
        for i in 0..16 {
            let len = rng.range_usize(1, 12);
            let s: Vec<u8> = (0..len).map(|_| *rng.pick(&ws)).collect();
            push("row21/ERR10", format!("random ws #{i}"), s);
        }
    }

    // Row 22 — leading whitespace then a non-zero number.
    for (name, s) in [
        ("nl-tab-sp-7", "\n\t 7"),
        ("sp-1", " 1"),
        ("many-ws-42", "  \t\n\n  42"),
        ("cr-neg", "\r-5"),
    ] {
        push("row22", format!("leading ws {name}"), s.as_bytes().to_vec());
    }

    // Row 23 — zero in its various spellings.
    for s in ["0", "-0", "+0", "000", "0000000000", "-000", "+000", "0\n"] {
        push("row23", format!("zero {s:?}"), s.as_bytes().to_vec());
    }

    // Row 24 — plain non-zero digits.
    for s in ["1", "2", "9", "42", "12345", "999999999", "2147483646"] {
        push("row24", format!("nonzero {s:?}"), s.as_bytes().to_vec());
    }
    {
        let mut rng = Rng::new(SEED ^ 0x24);
        for i in 0..24 {
            let nd = rng.range_usize(1, 9);
            let mut s = vec![b'1' + rng.below(9) as u8];
            for _ in 1..nd {
                s.push(b'0' + rng.below(10) as u8);
            }
            push("row24", format!("random digits #{i}"), s);
        }
    }

    // Row 25 — explicit sign then non-zero digits.
    for s in ["+1", "-1", "+42", "-42", "+2147483647", "-2147483647"] {
        push("row25", format!("signed {s:?}"), s.as_bytes().to_vec());
    }

    // Row 26 / ERRORS 12 — sign with no digits: matching failure.
    for s in ["-", "+", "-x", "+x", "- 5", "+ 5", "--5", "++5", "-+5", "+-5", "-\n"] {
        push("row26/ERR12", format!("sign-only {s:?}"), s.as_bytes().to_vec());
    }

    // Row 27 — leading zeros, and "0x" (%d is base 10, conversion stops at 'x').
    for s in ["0007", "00000001", "0x10", "0X10", "0b1", "0o7", "010", "-0007"] {
        push("row27", format!("prefixed {s:?}"), s.as_bytes().to_vec());
    }

    // Row 28 — digits then trailing garbage.
    for s in [
        "5abc", "12,34", "7\n\n", "3.14", "1e5", "42;", "8 9", "0abc", "0.0", "1/2",
    ] {
        push("row28", format!("trailing {s:?}"), s.as_bytes().to_vec());
    }

    // Row 29 / ERRORS 11 — non-numeric leading data: matching failure.
    for s in [
        "abc", ".5", "e1", "x", "!", "#0", "\"1\"", "nan", "inf", "NULL", "()", "[]", "\x7f",
    ] {
        push("row29/ERR11", format!("garbage {s:?}"), s.as_bytes().to_vec());
    }
    {
        let mut rng = Rng::new(SEED ^ 0x29);
        for i in 0..24 {
            let len = rng.range_usize(1, 10);
            let s: Vec<u8> = (0..len)
                .map(|_| loop {
                    let b = (rng.next_u32() & 0x7F) as u8;
                    if b != 0 && !b.is_ascii_digit() && b != b'+' && b != b'-' && !b.is_ascii_whitespace() {
                        return b;
                    }
                })
                .collect();
            push("row29/ERR11", format!("random garbage #{i}"), s);
        }
    }

    // Row 30 / ERRORS 13 — int boundaries as text (truncation of the long).
    for s in [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "8589934592",
        "1099511627776",
    ] {
        push("row30/ERR13", format!("int-boundary {s:?}"), s.as_bytes().to_vec());
    }

    // Row 31 / ERRORS 14-15 — long boundaries: strtol saturation, then truncation.
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
    ] {
        push("row31/ERR14-15", format!("long-boundary {s:?}"), s.as_bytes().to_vec());
    }

    // Row 32 — very long digit runs.
    {
        let mut rng = Rng::new(SEED ^ 0x32);
        for nd in [19usize, 20, 21, 30, 100, 400] {
            push("row32", format!("nines x {nd}"), vec![b'9'; nd]);
            push(
                "row32",
                format!("-nines x {nd}"),
                [b"-".to_vec(), vec![b'9'; nd]].concat(),
            );
            push("row32", format!("zeros x {nd}"), vec![b'0'; nd]);
            push(
                "row32",
                format!("leading zeros then 1 x {nd}"),
                [vec![b'0'; nd], b"1".to_vec()].concat(),
            );
            for i in 0..3 {
                let mut s = vec![b'1' + rng.below(9) as u8];
                for _ in 1..nd {
                    s.push(b'0' + rng.below(10) as u8);
                }
                push("row32", format!("random {nd} digits #{i}"), s.clone());
                push(
                    "row32",
                    format!("random -{nd} digits #{i}"),
                    [b"-".to_vec(), s].concat(),
                );
            }
        }
    }

    // Row 33 — several numbers on the line; only the first conversion happens.
    for s in ["3 4 5", "0 1", "1 0", "0\n1\n", "1\n0\n", "-1 -2", "0 abc"] {
        push("row33", format!("multi {s:?}"), s.as_bytes().to_vec());
    }

    // Row 34 / ERRORS 16 — trailing newline present/absent, embedded NUL.
    push("row34", "no trailing newline \"7\"".into(), b"7".to_vec());
    push("row34", "trailing newline \"7\\n\"".into(), b"7\n".to_vec());
    push("row34", "crlf \"7\\r\\n\"".into(), b"7\r\n".to_vec());
    push("row34", "NUL first".into(), b"\x000".to_vec());
    push("row34", "NUL after digit".into(), b"7\x00".to_vec());
    push("row34", "NUL only".into(), b"\x00".to_vec());
    push("row34", "high bytes".into(), b"\xff\xfe".to_vec());
    push("row34", "utf8 digits".into(), "٧".as_bytes().to_vec());

    // Row 35 — randomized byte fuzz over a digit/sign/space/garbage alphabet.
    {
        let mut rng = Rng::new(SEED ^ 0x35);
        let alphabet: Vec<u8> = b"0123456789+- \t\n\r\x0b\x0cabcXx.,;e%\xff\x01".to_vec();
        for i in 0..512 {
            let len = rng.range_usize(0, 24);
            let s: Vec<u8> = (0..len).map(|_| *rng.pick(&alphabet)).collect();
            push("row35", format!("fuzz #{i}"), s);
        }
    }

    // Row 36 — randomized decimal fuzz across the i64 / i128 ranges.
    {
        let mut rng = Rng::new(SEED ^ 0x36);
        for i in 0..128 {
            let v = rng.next_u64() as i64;
            push("row36", format!("i64 fuzz #{i} = {v}"), v.to_string().into_bytes());
        }
        for i in 0..128 {
            let hi = rng.next_u64() as u128;
            let lo = rng.next_u64() as u128;
            let v = ((hi << 64) | lo) as i128;
            push("row36", format!("i128 fuzz #{i} = {v}"), v.to_string().into_bytes());
        }
    }

    v
}

// ---------------------------------------------------------------------------
// hostile stdout descriptors (for the process-termination rows)
// ---------------------------------------------------------------------------

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
}

/// The write end of a pipe whose read end has already been closed: the first
/// write raises `SIGPIPE` (or fails with `EPIPE` if `SIGPIPE` is ignored).
pub fn orphan_pipe_write_fd() -> c_int {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    assert_eq!(unsafe { close(fds[0]) }, 0, "close(read end) failed");
    fds[1]
}

/// Close an arbitrary descriptor (thin wrapper so test files can reach `close`).
pub fn close_fd(fd: c_int) {
    unsafe { close(fd) };
}
