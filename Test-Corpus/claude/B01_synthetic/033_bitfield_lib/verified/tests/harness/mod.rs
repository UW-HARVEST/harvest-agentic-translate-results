//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading` and driven
//! only through their exported symbols — the Rust functions are never called
//! directly, so the `#[no_mangle] extern "C"` wrappers and the struct ABI are
//! part of what is under test.
//!
//! The library's only output channel is `printf` to `stdout`, so the harness
//! compares behaviour by redirecting file descriptor 1 to a temporary file
//! around a batch of calls and diffing the captured bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture and for the fault-behaviour tests
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which covers the
    /// `stdout` FILE shared by both `.so`s and by this process.
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Exported signatures under test
// ---------------------------------------------------------------------------

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
///
/// `b` is deliberately typed as `u32` rather than `bool`/`u8` here: a C `_Bool`
/// parameter occupies a full argument register, and an external caller can put
/// *any* 32-bit value there. Using one identical prototype for both libraries
/// means the raw ABI is what gets compared, including out-of-range `_Bool`
/// values that a Rust `bool` could not even represent.
pub type DriverFn = unsafe extern "C" fn(u32, u32, u32, i32);

/// `void print_foo(const foo_t *foo)`
pub type PrintFooFn = unsafe extern "C" fn(*const u8);

/// `foo_t` is 8 bytes with 4-byte alignment (verified against the C compiler);
/// this wrapper lets tests hand `print_foo` a correctly aligned raw image.
#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FooImage(pub [u8; 8]);

impl FooImage {
    /// Build the storage image the way the C `driver` does: `x` in bits 0..1,
    /// `y` in bits 2..4, `b` in bit 5, `z` little-endian at offset 4.
    pub fn new(storage: u8, z: i32) -> Self {
        let mut b = [0u8; 8];
        b[0] = storage;
        b[4..8].copy_from_slice(&z.to_le_bytes());
        Self(b)
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

/// Pack `(x, y, b)` into the bit-field storage byte, mirroring the C bit-field
/// assignment semantics (mask to the field width, no non-zero test on `b`).
pub fn pack_storage(x: u32, y: u32, b: u32, padding_bits: u8) -> u8 {
    ((x as u8) & 0x3) | (((y as u8) & 0x7) << 2) | (((b as u8) & 0x1) << 5) | (padding_bits & 0xC0)
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rs: Library,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build the C shared library with CMake if it is not already present.
fn ensure_c_so() -> PathBuf {
    let root = crate_root();
    let so = root.join("c_src/build/libdriver.so");
    if so.exists() {
        return so;
    }
    let build_dir = root.join("c_src/build");
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let ok = std::process::Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    assert!(
        ok && so.exists(),
        "failed to build the C shared library at {so:?}; build it manually with\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    so
}

/// Locate the Rust `cdylib`.
///
/// `cargo test` does *not* build the `cdylib` (the crate has no `rlib`, so the
/// test binaries never link it), and any `libdriver.so` left over in
/// `target/<profile>` from an earlier `cargo build` could easily be stale —
/// which would silently verify old code. So the harness builds the `cdylib`
/// itself, into a dedicated target directory.
///
/// A separate `--target-dir` matters: cargo's build lock is per target
/// directory, and the outer `cargo test` still holds the lock on the normal
/// one while the test binary runs.
///
/// * `DRIVER_RUST_SO` — use this exact `.so` and skip building (used to point
///   the suite at the release artifact).
/// * `DRIVER_BUILD_ARGS` — extra args for the nested build, e.g.
///   `--no-default-features --features foo`, so each feature combination can be
///   exercised.
fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={p:?} does not exist");
        return p;
    }

    let root = crate_root();
    let target_dir = root.join("target/dt-harness");
    let extra: Vec<String> = std::env::var("DRIVER_BUILD_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    let mut cmd = std::process::Command::new(
        std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
    );
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .args(&extra)
        // Do not inherit the outer `cargo test` invocation's environment hooks.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .env_remove("CARGO_TARGET_DIR");

    let out = cmd.output().expect("run nested cargo build for the cdylib");
    assert!(
        out.status.success(),
        "nested `cargo build` of the cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir.join("debug/libdriver.so");
    assert!(
        so.exists(),
        "Rust cdylib not found at {so:?} after a successful nested build"
    );
    so
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = ensure_c_so();
        let rs_path = rust_so();
        // Loaded with RTLD_LOCAL (libloading's default), so the two libraries'
        // identically named symbols cannot shadow one another: the C `driver`
        // keeps calling the C `print_foo` and vice-versa.
        let c = unsafe { Library::new(&c_path) }.expect("dlopen C .so");
        let rs = unsafe { Library::new(&rs_path) }.expect("dlopen Rust .so");
        Libs {
            c,
            rs,
            c_path,
            rs_path,
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0") }.expect("C driver symbol")
}
pub fn rs_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rs.get(b"driver\0") }.expect("Rust driver symbol")
}
pub fn c_print_foo() -> Symbol<'static, PrintFooFn> {
    unsafe { libs().c.get(b"print_foo\0") }.expect("C print_foo symbol")
}
pub fn rs_print_foo() -> Symbol<'static, PrintFooFn> {
    unsafe { libs().rs.get(b"print_foo\0") }.expect("Rust print_foo symbol")
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Outcome of running a call in a forked child.
#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child with fd 1 pointing at a temp file, and return
/// everything the child wrote plus how it terminated.
///
/// Forking (rather than redirecting fd 1 in-process) is deliberate:
///
/// * fd 1 is process-global, but libtest's main thread writes its own
///   `test ... ok` progress lines to fd 1 while worker threads run. An
///   in-process redirect captures those too, corrupting the comparison. A child
///   process has its own fd table and no libtest reporter, so only the calls
///   under test produce output.
/// * It also isolates faults, so an input that makes the library crash (e.g.
///   the unchecked NULL dereference) is observable as a signal instead of
///   taking the whole test binary down.
///
/// `f` must not print through Rust's `std` streams.
pub fn capture_with_outcome<F: FnOnce()>(f: F) -> (Vec<u8>, ChildOutcome) {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    // Flush before forking so already-buffered bytes are not duplicated into
    // the child's copy of the stdio buffers.
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child. Keep this as simple as possible: point fd 1 at the file, run
        // the calls, flush, and leave without running atexit handlers.
        if unsafe { dup2(fd, 1) } < 0 {
            unsafe { _exit(101) };
        }
        f();
        unsafe { fflush(std::ptr::null_mut()) };
        unsafe { _exit(0) };
    }

    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    // Decode the wait status without pulling in the libc crate.
    let outcome = if status & 0x7f == 0x7f {
        ChildOutcome::Signaled(-1) // stopped; not expected here
    } else if status & 0x7f != 0 {
        ChildOutcome::Signaled(status & 0x7f)
    } else {
        ChildOutcome::Exited((status >> 8) & 0xff)
    };
    (out, outcome)
}

/// Capture the output of `f`, requiring that it completed normally.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let (out, outcome) = capture_with_outcome(f);
    assert_eq!(
        outcome,
        ChildOutcome::Exited(0),
        "the capture child terminated abnormally; captured {} bytes",
        out.len()
    );
    out
}

// ---------------------------------------------------------------------------
// Batch drivers: run a whole case list under one capture, then diff
// ---------------------------------------------------------------------------

pub fn run_driver_batch(f: &DriverFn, cases: &[(u32, u32, u32, i32)]) -> Vec<u8> {
    capture(|| {
        for &(x, y, b, z) in cases {
            unsafe { f(x, y, b, z) };
        }
    })
}

pub fn run_print_foo_batch(f: &PrintFooFn, cases: &[FooImage]) -> Vec<u8> {
    capture(|| {
        for img in cases {
            unsafe { f(img.as_ptr()) };
        }
    })
}

fn lines(b: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(b)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

/// Assert two captured outputs are byte-identical; on divergence report the
/// first differing line together with the input that produced it.
pub fn assert_same<D: std::fmt::Debug>(row: &str, cases: &[D], c_out: &[u8], rs_out: &[u8]) {
    if c_out == rs_out {
        let n = lines(c_out).len();
        assert_eq!(
            n,
            cases.len(),
            "{row}: expected one output line per case ({} cases) but got {n}",
            cases.len()
        );
        return;
    }
    let cl = lines(c_out);
    let rl = lines(rs_out);
    let mut msg = format!(
        "{row}: C and Rust output differ ({} vs {} lines, {} vs {} bytes)\n",
        cl.len(),
        rl.len(),
        c_out.len(),
        rs_out.len()
    );
    let mut shown = 0;
    for i in 0..cl.len().max(rl.len()) {
        let c = cl.get(i).map(String::as_str);
        let r = rl.get(i).map(String::as_str);
        if c != r {
            let case = cases
                .get(i)
                .map(|c| format!("{c:?}"))
                .unwrap_or_else(|| "<no case>".into());
            msg += &format!("  case[{i}] {case}\n    C  : {c:?}\n    Rust: {r:?}\n");
            shown += 1;
            if shown == 10 {
                msg += "  ... (further differences suppressed)\n";
                break;
            }
        }
    }
    panic!("{msg}");
}

/// Convenience for `driver` rows.
pub fn check_driver_row(row: &str, cases: &[(u32, u32, u32, i32)]) {
    let c = run_driver_batch(&*c_driver(), cases);
    let r = run_driver_batch(&*rs_driver(), cases);
    assert_same(row, cases, &c, &r);
}

/// Convenience for `print_foo` rows.
pub fn check_print_foo_row(row: &str, cases: &[FooImage]) {
    let c = run_print_foo_batch(&*c_print_foo(), cases);
    let r = run_print_foo_batch(&*rs_print_foo(), cases);
    assert_same(row, cases, &c, &r);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A value biased towards interesting boundaries.
    pub fn interesting_u32(&mut self) -> u32 {
        const POOL: [u32; 12] = [
            0,
            1,
            2,
            3,
            4,
            7,
            8,
            255,
            256,
            0x7FFF_FFFF,
            0x8000_0000,
            u32::MAX,
        ];
        if self.next_u32() & 1 == 0 {
            POOL[(self.below(POOL.len() as u32)) as usize]
        } else {
            self.next_u32()
        }
    }
    pub fn interesting_i32(&mut self) -> i32 {
        const POOL: [i32; 9] = [0, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
        if self.next_u32() & 1 == 0 {
            POOL[(self.below(POOL.len() as u32)) as usize]
        } else {
            self.next_i32()
        }
    }
}

// ---------------------------------------------------------------------------
// Fault-behaviour comparison (for the unchecked NULL dereference)
// ---------------------------------------------------------------------------

/// Run `f` in a forked child and report how the child terminated, discarding
/// its output. Used to compare crash behaviour (e.g. `SIGSEGV` on a NULL
/// dereference) without taking the test process down.
pub fn run_in_child<F: FnOnce()>(f: F) -> ChildOutcome {
    capture_with_outcome(f).1
}
