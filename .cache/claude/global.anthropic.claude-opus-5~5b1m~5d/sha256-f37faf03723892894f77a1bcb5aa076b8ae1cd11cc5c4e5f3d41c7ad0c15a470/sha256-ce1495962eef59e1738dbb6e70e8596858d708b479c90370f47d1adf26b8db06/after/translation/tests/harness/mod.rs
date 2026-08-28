//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the dynamic-symbol table, so the `#[no_mangle]`
//! `extern "C"` export wrappers are under test too. No Rust function from the
//! crate is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// `DataBlock` — must mirror `typedef struct { int id; double value;
// char label[20]; } DataBlock;`. Verified against the C ABI: size 40,
// align 8, offsets 0 / 8 / 16 (see `layout` test in phase_b_valid.rs).
// ---------------------------------------------------------------------------
pub const DATA_BLOCK_SIZE: usize = 40;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

/// A `DataBlock`-sized, `DataBlock`-aligned raw byte buffer. Using this instead
/// of the typed struct lets the tests observe the **padding** bytes, which
/// `memcpy(dest, src, sizeof(DataBlock))` copies as well.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawBlock(pub [u8; DATA_BLOCK_SIZE]);

impl RawBlock {
    pub fn zeroed() -> Self {
        RawBlock([0u8; DATA_BLOCK_SIZE])
    }
    pub fn filled(b: u8) -> Self {
        RawBlock([b; DATA_BLOCK_SIZE])
    }
    pub fn from_fields(id: c_int, value: c_double, label: &[u8]) -> Self {
        let mut r = RawBlock::zeroed();
        r.0[0..4].copy_from_slice(&id.to_ne_bytes());
        r.0[8..16].copy_from_slice(&value.to_ne_bytes());
        let n = label.len().min(20);
        r.0[16..16 + n].copy_from_slice(&label[..n]);
        r
    }
    pub fn id(&self) -> c_int {
        c_int::from_ne_bytes(self.0[0..4].try_into().unwrap())
    }
    pub fn value_bits(&self) -> u64 {
        u64::from_ne_bytes(self.0[8..16].try_into().unwrap())
    }
    pub fn label(&self) -> &[u8] {
        &self.0[16..36]
    }
}

impl std::fmt::Debug for RawBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawBlock({:02x?})", &self.0)
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// The C `.so`. Its name is derived by CMake from the parent directory name
/// (`cmake_path(GET parent FILENAME project_name)`), so it is discovered by
/// globbing rather than hard-coded.
fn c_so_path() -> PathBuf {
    // Allows pointing the suites at an alternative build of the *same*
    // `c_src/src/lib.c` (e.g. an -O2 one) without touching `c_src/`.
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFFTEST_C_SO={} does not exist", p.display());
        return p;
    }
    let dir = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `.so`, resolved relative to the running test executable
/// (`target/<profile>/deps/<test>`) so debug and release runs each pick up the
/// artifact from their own profile directory.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let p = profile_dir.join("liboverunder_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} — run `cargo build` first",
        p.display()
    );
    p
}

/// The five exported symbols, resolved once per library.
pub struct Api {
    _lib: Library,
    pub name: &'static str,
    safe_double_to_int: unsafe extern "C" fn(c_double) -> c_int,
    process_with_fallthrough: unsafe extern "C" fn(c_int, c_int) -> c_int,
    copy_data_block: unsafe extern "C" fn(*mut c_void, *const c_void),
    handle_pointer_operations: unsafe extern "C" fn(c_int) -> c_int,
    overunder: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

impl Api {
    fn load(path: &PathBuf, name: &'static str) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let nul_terminated: &[u8] = $s;
                    let pretty = String::from_utf8_lossy(&nul_terminated[..nul_terminated.len() - 1])
                        .into_owned();
                    let s: Symbol<$t> = lib.get(nul_terminated).unwrap_or_else(|e| {
                        panic!("symbol {} missing from {}: {e}", pretty, path.display())
                    });
                    *s.into_raw()
                }};
            }
            let safe_double_to_int =
                sym!(unsafe extern "C" fn(c_double) -> c_int, b"safe_double_to_int\0");
            let process_with_fallthrough = sym!(
                unsafe extern "C" fn(c_int, c_int) -> c_int,
                b"process_with_fallthrough\0"
            );
            let copy_data_block = sym!(
                unsafe extern "C" fn(*mut c_void, *const c_void),
                b"copy_data_block\0"
            );
            let handle_pointer_operations = sym!(
                unsafe extern "C" fn(c_int) -> c_int,
                b"handle_pointer_operations\0"
            );
            let overunder = sym!(
                unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
                b"overunder\0"
            );
            Api {
                _lib: lib,
                name,
                safe_double_to_int,
                process_with_fallthrough,
                copy_data_block,
                handle_pointer_operations,
                overunder,
            }
        }
    }

    // -- safe wrappers around the FFI calls -------------------------------
    pub fn safe_double_to_int(&self, d: f64) -> c_int {
        unsafe { (self.safe_double_to_int)(d) }
    }
    pub fn process_with_fallthrough(&self, code: c_int, base: c_int) -> c_int {
        unsafe { (self.process_with_fallthrough)(code, base) }
    }
    pub fn handle_pointer_operations(&self, v: c_int) -> c_int {
        unsafe { (self.handle_pointer_operations)(v) }
    }
    pub fn overunder(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.overunder)(a, b, c, d) }
    }
    /// `copy_data_block` into a fresh destination, returning all 40 raw bytes
    /// (including padding) that the callee wrote.
    pub fn copy_block(&self, src: &RawBlock, dest_prefill: u8) -> RawBlock {
        let mut dest = RawBlock::filled(dest_prefill);
        unsafe {
            (self.copy_data_block)(
                dest.0.as_mut_ptr().cast::<c_void>(),
                src.0.as_ptr().cast::<c_void>(),
            );
        }
        dest
    }
    /// Self-copy: `dest == src`.
    pub fn copy_block_self(&self, block: &RawBlock) -> RawBlock {
        let mut b = *block;
        let p = b.0.as_mut_ptr().cast::<c_void>();
        unsafe {
            (self.copy_data_block)(p, p.cast_const());
        }
        b
    }
    /// Raw pointer form, for the NULL-pointer error rows.
    pub unsafe fn copy_data_block_raw(&self, dest: *mut c_void, src: *const c_void) {
        unsafe { (self.copy_data_block)(dest, src) }
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::load(&c_so_path(), "C"))
}
pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Api::load(&rust_so_path(), "Rust"))
}
/// `(c, rust)`
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds, reproducible runs.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Full-range `int`.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// `int` in `[-bound, bound]`.
    pub fn next_i32_bounded(&mut self, bound: i32) -> i32 {
        debug_assert!(bound > 0);
        let span = (bound as i64) * 2 + 1;
        ((self.next_u64() % span as u64) as i64 - bound as i64) as i32
    }
    /// Uniform `f64` in `[-1, 1)`.
    pub fn next_unit(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        m * 2.0 - 1.0
    }
    /// Completely arbitrary `f64` bit pattern (NaN / inf / subnormal / normal).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

// ---------------------------------------------------------------------------
// stdout capture — the C library's only other observable channel.
//
// `printf` in both `.so`s resolves to the *same* glibc `printf` in the same
// process, writing to fd 1. We redirect fd 1 to a temporary file around each
// call, flush, and read the bytes back. A global mutex serialises captures
// because fd 1 is process-wide.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Reusable fd-1 capturer. Keeps one temporary file open and rewinds it for
/// each call, so thousands of randomized captures stay cheap.
pub struct Capturer {
    file: std::fs::File,
    path: std::path::PathBuf,
    buf: Vec<u8>,
}

impl Capturer {
    pub fn new() -> Capturer {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "difftest-stdout-{}-{:?}.bin",
            std::process::id(),
            std::thread::current().id()
        ));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("create capture file");
        Capturer {
            file,
            path,
            buf: Vec::new(),
        }
    }

    /// Run `f`, returning everything it wrote to fd 1 plus `f`'s return value.
    pub fn run<R, F: FnOnce() -> R>(&mut self, f: F) -> (Vec<u8>, R) {
        use std::io::{Read, Seek, SeekFrom};
        use std::os::unix::io::AsRawFd;

        let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

        self.file.set_len(0).expect("truncate capture file");
        self.file.seek(SeekFrom::Start(0)).expect("rewind for write");

        let r = unsafe {
            // Flush anything already pending on the real stdout: both Rust's
            // own `LineWriter` and libc's `FILE*` buffers. Otherwise bytes
            // written *before* the redirect could be flushed *during* it and
            // pollute the captured library output.
            let _ = std::io::Write::flush(&mut std::io::stdout());
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(self.file.as_raw_fd(), 1) >= 0, "dup2 failed");

            let r = f();

            // Flush the redirected stdout *before* restoring fd 1, otherwise
            // buffered bytes would land on the terminal instead of the file.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
            r
        };

        self.file.seek(SeekFrom::Start(0)).expect("rewind for read");
        self.buf.clear();
        self.file
            .read_to_end(&mut self.buf)
            .expect("read capture file");
        (self.buf.clone(), r)
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// One-shot convenience wrapper around [`Capturer`].
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (Vec<u8>, R) {
    Capturer::new().run(f)
}

/// Call `overunder` on both libraries, capturing each one's stdout, and assert
/// that the return value **and** the printed bytes match exactly.
#[track_caller]
pub fn diff_overunder(cap: &mut Capturer, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let (capi, rapi) = both();
    let (c_out, c_ret) = cap.run(|| capi.overunder(a, b, c, d));
    let (r_out, r_ret) = cap.run(|| rapi.overunder(a, b, c, d));
    let ctx = format!("overunder({a}, {b}, {c}, {d})");
    assert_int_eq(&ctx, c_ret, r_ret);
    assert_bytes_eq(&ctx, &c_out, &r_out);
    c_ret
}

// ---------------------------------------------------------------------------
// Fork helper — for inputs where the C code faults (NULL dereference).
// The child performs the call and `_exit(0)`s if it somehow survives, so the
// wait status distinguishes "died with signal N" from "returned normally".
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(c_int),
    Signalled(c_int),
}

/// Fork and run `f` in the child. Returns how the child terminated.
pub fn run_in_child<F: FnOnce()>(f: F) -> ChildOutcome {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS
        let term_sig = status & 0x7f;
        if term_sig != 0 && term_sig != 0x7f {
            ChildOutcome::Signalled(term_sig)
        } else {
            ChildOutcome::Exited((status >> 8) & 0xff)
        }
    }
}

// ---------------------------------------------------------------------------
// Sequential suite runner (`harness = false`).
//
// These suites redirect the process-wide fd 1 to capture the library's
// `printf` output. libtest's default harness runs tests on several threads and
// writes its own progress text to fd 1, which would land inside a capture and
// produce spurious "divergences". Running the cases sequentially from a plain
// `main` makes each capture exclusive and the results reproducible.
// ---------------------------------------------------------------------------

pub type Case = (&'static str, fn());

pub fn run_suite(suite: &str, cases: &[Case]) -> ! {
    use std::io::Write;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    if args.iter().any(|a| a == "--list") {
        for (name, _) in cases {
            println!("{name}: test");
        }
        std::process::exit(0);
    }

    // Fail loudly rather than silently skipping if a name is misspelled.
    for f in &filters {
        assert!(
            cases.iter().any(|(n, _)| n.contains(f)),
            "filter {f:?} matches no case in suite {suite}"
        );
    }

    println!("\nrunning suite `{suite}` ({} cases, sequential)", cases.len());
    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let start = std::time::Instant::now();

    for (name, f) in cases {
        if !filters.is_empty() && !filters.iter().any(|flt| name.contains(flt)) {
            skipped += 1;
            continue;
        }
        print!("test {name} ... ");
        // Flush *now*, so no partial line is left in Rust's buffer while the
        // case redirects fd 1.
        let _ = std::io::stdout().flush();
        let t = std::time::Instant::now();
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => {
                passed += 1;
                println!("ok ({:.2?})", t.elapsed());
            }
            Err(_) => {
                failed.push(name);
                println!("FAILED ({:.2?})", t.elapsed());
            }
        }
        let _ = std::io::stdout().flush();
    }

    println!(
        "\nsuite `{suite}`: {} passed; {} failed; {} filtered out; finished in {:.2?}",
        passed,
        failed.len(),
        skipped,
        start.elapsed()
    );
    if !failed.is_empty() {
        println!("failures:");
        for f in &failed {
            println!("    {f}");
        }
        let _ = std::io::stdout().flush();
        std::process::exit(1);
    }
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_int_eq(ctx: &str, c: c_int, rust: c_int) {
    assert_eq!(
        c, rust,
        "DIVERGENCE [{ctx}]: C returned {c} ({c:#010x}), Rust returned {rust} ({rust:#010x})"
    );
}

#[track_caller]
pub fn assert_bytes_eq(ctx: &str, c: &[u8], rust: &[u8]) {
    if c != rust {
        panic!(
            "DIVERGENCE [{ctx}]:\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}",
            c.len(),
            String::from_utf8_lossy(c),
            rust.len(),
            String::from_utf8_lossy(rust),
        );
    }
}
