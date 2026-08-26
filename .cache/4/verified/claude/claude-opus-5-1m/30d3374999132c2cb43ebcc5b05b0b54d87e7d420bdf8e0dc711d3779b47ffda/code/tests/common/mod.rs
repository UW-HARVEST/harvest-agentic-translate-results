//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* and driven exclusively
//! through their exported `main` symbol, so the Rust `#[no_mangle]` wrapper is
//! part of what is under test. Nothing from the Rust crate is called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used by the harness itself
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Locating / building the artifacts
// ---------------------------------------------------------------------------

/// `.../translated_rust`
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The cargo target directory the tests were built into (`target/debug` or
/// `target/release`).
pub fn target_dir() -> PathBuf {
    // current_exe is <target>/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// The Rust shared object built from `src/lib.rs`.
///
/// `cargo test --test <name>` does *not* rebuild the `cdylib` (no test target
/// depends on it), so guard against silently testing a stale artifact.
pub fn rust_so() -> PathBuf {
    let p = target_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "{} is missing - build the cdylib first (cargo build)",
        p.display()
    );
    assert_not_stale(&p, &["lib.rs", "imp.rs"]);
    p
}

/// Panic if `artifact` is older than any of the sources it is built from.
fn assert_not_stale(artifact: &Path, sources: &[&str]) {
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let built = mtime(artifact);
    for name in sources {
        let src = manifest_dir().join("src").join(name);
        assert!(
            mtime(&src) <= built,
            "{} is older than {} - run `cargo build` (or scripts/verify.sh) first",
            artifact.display(),
            src.display()
        );
    }
}

/// The C shared object. Compiled on demand from `c_src/src/main.c` (never
/// modifying anything under `c_src/`); the optimization level can be steered
/// with `CDIFF_CFLAGS` so the same tests can run against `-O0` and `-O2`.
pub fn c_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let opt = std::env::var("CDIFF_CFLAGS").unwrap_or_else(|_| "-O2".to_string());
            let out_dir = manifest_dir().join("target").join("cdiff");
            std::fs::create_dir_all(&out_dir).expect("mkdir target/cdiff");
            let out = out_dir.join(format!("libcdriver{}.so", opt.replace(['-', ' '], "_")));
            let src = manifest_dir().join("c_src").join("src").join("main.c");
            let status = Command::new("gcc")
                .args(["-shared", "-fPIC", "-std=c99", &opt])
                .arg("-o")
                .arg(&out)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(status.success(), "compiling {} failed", src.display());
            out
        })
        .clone()
}

/// The C executable produced by `c_src/CMakeLists.txt`. `CDIFF_C_EXE` selects a
/// different CMake build directory (used to test the optimized build too).
pub fn c_exe() -> PathBuf {
    let p = match std::env::var_os("CDIFF_C_EXE") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src").join("build").join("driver"),
    };
    assert!(
        p.exists(),
        "{} is missing - build it with cmake first (see SYMBOLS.md)",
        p.display()
    );
    p
}

/// The Rust executable produced by `[[bin]] driver`.
pub fn rust_exe() -> PathBuf {
    let p = target_dir().join("driver");
    assert!(
        p.exists(),
        "{} is missing - build it first (cargo build)",
        p.display()
    );
    assert_not_stale(&p, &["main.rs", "imp.rs"]);
    p
}

// ---------------------------------------------------------------------------
// The library under test
// ---------------------------------------------------------------------------

pub type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

pub struct Lib {
    lib: libloading::Library,
    pub name: &'static str,
}

impl Lib {
    pub fn open(path: &Path, name: &'static str) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        // Fail loudly right away if `main` is not exported.
        unsafe {
            lib.get::<MainFn>(b"main\0")
                .unwrap_or_else(|e| panic!("{} does not export `main`: {e}", path.display()));
        }
        Lib { lib, name }
    }

    /// Call the library's `main(argc, argv)`.
    pub unsafe fn call(&self, argc: c_int, argv: *mut *mut c_char) -> c_int {
        let f = self.lib.get::<MainFn>(b"main\0").expect("symbol main");
        f(argc, argv)
    }
}

/// The C and the Rust shared object, opened once per test process.
pub fn libs() -> &'static (Lib, Lib) {
    static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
    LIBS.get_or_init(|| {
        (
            Lib::open(&c_so(), "C"),
            Lib::open(&rust_so(), "Rust"),
        )
    })
}

// ---------------------------------------------------------------------------
// argv construction
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Layout {
    /// One contiguous block of NUL-separated strings: exactly what `execve`
    /// hands to `main`.
    Contiguous,
    /// Every string in its own allocation.
    Separate,
}

/// A C-style `argv`: `n` pointers to NUL-terminated strings plus a NULL
/// terminator. The backing storage is owned, so the pointers stay valid for as
/// long as the `Argv` lives.
pub struct Argv {
    _blocks: Vec<Vec<u8>>,
    ptrs: Vec<*mut c_char>,
    argc: c_int,
}

impl Argv {
    pub fn new(args: &[&[u8]], layout: Layout) -> Argv {
        for a in args {
            assert!(!a.contains(&0), "argv strings cannot contain NUL");
        }
        let (blocks, ptrs): (Vec<Vec<u8>>, Vec<*mut c_char>) = match layout {
            Layout::Separate => {
                let mut blocks: Vec<Vec<u8>> = args
                    .iter()
                    .map(|a| {
                        let mut v = Vec::with_capacity(a.len() + 1);
                        v.extend_from_slice(a);
                        v.push(0);
                        v
                    })
                    .collect();
                let ptrs = blocks
                    .iter_mut()
                    .map(|b| b.as_mut_ptr() as *mut c_char)
                    .collect();
                (blocks, ptrs)
            }
            Layout::Contiguous => {
                let mut block: Vec<u8> = Vec::new();
                let mut offsets: Vec<usize> = Vec::new();
                for a in args {
                    offsets.push(block.len());
                    block.extend_from_slice(a);
                    block.push(0);
                }
                let mut blocks = vec![block];
                let base = blocks[0].as_mut_ptr();
                let ptrs = offsets
                    .iter()
                    .map(|&o| unsafe { base.add(o) } as *mut c_char)
                    .collect();
                (blocks, ptrs)
            }
        };

        let argc = ptrs.len() as c_int;
        let mut ptrs = ptrs;
        ptrs.push(std::ptr::null_mut());
        Argv {
            _blocks: blocks,
            ptrs,
            argc,
        }
    }

    /// `argv[2] = "<num>"`, `argv[3] = argv[2] + k`: makes `argv[3]` point
    /// *into* `argv[2]`, the only way `end == argv[3]` can ever be true.
    pub fn aliased(prog: &[u8], string: &[u8], num: &[u8], k: usize) -> Argv {
        assert!(k <= num.len());
        let mut block: Vec<u8> = Vec::new();
        let prog_off = block.len();
        block.extend_from_slice(prog);
        block.push(0);
        let str_off = block.len();
        block.extend_from_slice(string);
        block.push(0);
        let num_off = block.len();
        block.extend_from_slice(num);
        block.push(0);

        let mut blocks = vec![block];
        let base = blocks[0].as_mut_ptr();
        let ptrs = vec![
            unsafe { base.add(prog_off) } as *mut c_char,
            unsafe { base.add(str_off) } as *mut c_char,
            unsafe { base.add(num_off) } as *mut c_char,
            unsafe { base.add(num_off + k) } as *mut c_char,
            std::ptr::null_mut(),
        ];
        Argv {
            _blocks: blocks,
            ptrs,
            argc: 4,
        }
    }

    /// Build a vector directly from caller-owned pointers (used to point into
    /// `mmap`ed pages). The last element must be NULL.
    pub fn from_raw_ptrs(ptrs: Vec<*mut c_char>, argc: c_int) -> Argv {
        assert_eq!(*ptrs.last().expect("non-empty"), std::ptr::null_mut());
        Argv {
            _blocks: Vec::new(),
            ptrs,
            argc,
        }
    }

    /// Replace one slot (used to inject NULL pointers).
    pub fn set(&mut self, i: usize, p: *mut c_char) {
        self.ptrs[i] = p;
    }

    pub fn argc(&self) -> c_int {
        self.argc
    }

    pub fn as_ptr(&mut self) -> *mut *mut c_char {
        self.ptrs.as_mut_ptr()
    }

    /// Copy of the backing storage, so a test can prove that neither
    /// implementation writes through `argv` (the C `main` only reads it).
    pub fn snapshot(&self) -> Vec<Vec<u8>> {
        self._blocks.clone()
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<u64> {
    static L: OnceLock<Mutex<u64>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(0))
}

/// Run `f` with file descriptor 1 redirected into a temporary file and return
/// what was written to it. Both implementations write to stdout (the C through
/// `printf`, the Rust through `std::io::Stdout`), so `fflush(NULL)` is issued
/// afterwards to drain libc's buffer before the fd is restored.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let lock = capture_lock();
    let mut counter = lock.lock().unwrap_or_else(|e| e.into_inner());
    *counter += 1;
    let path = std::env::temp_dir().join(format!(
        "cdiff-{}-{}-{}.out",
        std::process::id(),
        *counter,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    let file = std::fs::File::create(&path).expect("create capture file");

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let r = f();

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    drop(counter);
    (r, bytes)
}

// ---------------------------------------------------------------------------
// The differential check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub status: c_int,
    pub stdout: Vec<u8>,
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

fn show_args(args: &[&[u8]]) -> String {
    args.iter()
        .map(|a| format!("\"{}\"", show(a)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Call `main` in both shared objects with the very same `argv` (identical
/// bytes *and* identical addresses) and assert the results are byte-identical.
pub fn assert_same(args: &[&[u8]], layout: Layout) -> Outcome {
    assert_same_argc(args, layout, None)
}

/// Like [`assert_same`] but with an explicit `argc`, which lets the tests pass
/// values that do not match the number of strings in the vector (`0`, negative,
/// out-of-range - a C `int` parameter accepts any of them).
pub fn assert_same_argc(args: &[&[u8]], layout: Layout, argc: Option<c_int>) -> Outcome {
    let (c_lib, rust_lib) = libs();
    let mut argv = Argv::new(args, layout);
    let argc = argc.unwrap_or_else(|| argv.argc());
    let before = argv.snapshot();
    let p = argv.as_ptr();

    let (c_status, c_out) = capture_stdout(|| unsafe { c_lib.call(argc, p) });
    let after_c = argv.snapshot();
    let (r_status, r_out) = capture_stdout(|| unsafe { rust_lib.call(argc, p) });
    let after_rust = argv.snapshot();

    assert_eq!(after_c, before, "the C main must not modify argv");
    assert_eq!(after_rust, before, "the Rust main must not modify argv");

    if c_status != r_status || c_out != r_out {
        panic!(
            "DIVERGENCE argc={argc} layout={layout:?} argv=[{}]\n  C   : status={c_status} stdout=\"{}\"\n  Rust: status={r_status} stdout=\"{}\"",
            show_args(args),
            show(&c_out),
            show(&r_out)
        );
    }
    Outcome {
        status: c_status,
        stdout: c_out,
    }
}

/// Same, for a caller-built (possibly aliasing / NULL-containing) vector.
pub fn assert_same_argv(argv: &mut Argv, argc: c_int, what: &str) -> Outcome {
    let (c_lib, rust_lib) = libs();
    let p = argv.as_ptr();
    let (c_status, c_out) = capture_stdout(|| unsafe { c_lib.call(argc, p) });
    let (r_status, r_out) = capture_stdout(|| unsafe { rust_lib.call(argc, p) });
    if c_status != r_status || c_out != r_out {
        panic!(
            "DIVERGENCE ({what}) argc={argc}\n  C   : status={c_status} stdout=\"{}\"\n  Rust: status={r_status} stdout=\"{}\"",
            show(&c_out),
            show(&r_out)
        );
    }
    Outcome {
        status: c_status,
        stdout: c_out,
    }
}

/// Run `f` in a forked child and report how the child terminated:
/// `Ok(exit_code)` or `Err(signal)`.
pub fn fork_and_run(f: impl FnOnce()) -> Result<i32, i32> {
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    if status & 0x7f == 0 {
        Ok((status >> 8) & 0xff)
    } else {
        Err(status & 0x7f)
    }
}

// ---------------------------------------------------------------------------
// Process-level (EP2) differential check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcOutcome {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run_exe(exe: &Path, args: &[Vec<u8>]) -> ProcOutcome {
    use std::os::unix::ffi::OsStrExt;
    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(std::ffi::OsStr::from_bytes(a));
    }
    let out = cmd.output().expect("spawn");
    ProcOutcome {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Run the C and the Rust *executable* with the same arguments and compare
/// stdout, stderr and exit status.
pub fn assert_same_cli(args: &[Vec<u8>]) -> ProcOutcome {
    let c = run_exe(&c_exe(), args);
    let r = run_exe(&rust_exe(), args);
    if c != r {
        let shown: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
        panic!(
            "CLI DIVERGENCE argv=[{}]\n  C   : code={:?} stdout=\"{}\" stderr=\"{}\"\n  Rust: code={:?} stdout=\"{}\" stderr=\"{}\"",
            show_args(&shown),
            c.code,
            show(&c.stdout),
            show(&c.stderr),
            r.code,
            show(&r.stdout),
            show(&r.stderr)
        );
    }
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) - fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

/// Interior mutability keeps the call sites readable:
/// `random_bytes(&rng, rng.range(1, 8))` needs two simultaneous borrows.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(std::cell::Cell::new(seed | 1))
    }
    pub fn next_u64(&self) -> u64 {
        let mut x = self.0.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform-ish value in `lo..=hi_inclusive`.
    pub fn range(&self, lo: u64, hi_inclusive: u64) -> u64 {
        assert!(lo <= hi_inclusive);
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn pick<'a, T: Sized>(&self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    pub fn bool(&self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

/// Random byte string of length `len`, never containing NUL.
pub fn random_bytes(rng: &Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.range(1, 255) as u8).collect()
}

/// Random printable-ASCII string of length `len`.
pub fn random_ascii(rng: &Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect()
}

/// A random `argv[1]` covering the shape axis (S).
pub fn random_string_shape(rng: &Rng) -> Vec<u8> {
    match rng.below(6) {
        0 => Vec::new(),
        1 => random_ascii(rng, 1),
        2 => random_ascii(rng, rng.range(1, 32) as usize),
        3 => random_bytes(rng, rng.range(1, 64) as usize),
        4 => {
            // whitespace / newline heavy
            let alpha = b" \t\n\r\x0b\x0cab";
            (0..rng.range(1, 24))
                .map(|_| *rng.pick(alpha))
                .collect()
        }
        _ => random_bytes(rng, rng.range(64, 512) as usize),
    }
}

/// Render `v` as a decimal string with a random (but strtol-compatible)
/// decoration: leading C whitespace, explicit sign, leading zeros.
pub fn decorate_number(rng: &Rng, v: i64) -> Vec<u8> {
    let mut s: Vec<u8> = Vec::new();
    if rng.bool() {
        let ws = b" \t\n\r\x0b\x0c";
        for _ in 0..rng.below(3) {
            s.push(*rng.pick(ws));
        }
    }
    let neg = v < 0;
    let digits = format!("{}", (v as i128).abs());
    if neg {
        s.push(b'-');
    } else if rng.bool() {
        s.push(b'+');
    }
    for _ in 0..rng.below(3) {
        s.push(b'0');
    }
    s.extend_from_slice(digits.as_bytes());
    s
}

/// Junk that `strtol` stops at (never a digit, never a leading sign).
pub fn random_junk(rng: &Rng) -> Vec<u8> {
    let alpha = b"abcxyzXYZ.,;:_/*%$#@!()[]{}<>?~^&|'\"`\\ \t";
    (0..rng.range(1, 5)).map(|_| *rng.pick(alpha)).collect()
}

/// A string on which `strtol(_, _, 10)` performs no conversion at all.
pub fn no_conversion_string(rng: &Rng) -> Vec<u8> {
    let choices: &[&[u8]] = &[
        b"", b"abc", b"-", b"+", b" ", b"\t", b"  \t\n", b"x9", b".", b"--1", b"++1", b"-+2",
        b"e5", b"/3", b":7", b"#", b"0x", b" - 1", b"one",
    ];
    let mut v = rng.pick(choices).to_vec();
    if v == b"0x" {
        // "0x" *does* convert (to 0) in base 10; keep it honest.
        v = b"abc".to_vec();
    }
    v
}
