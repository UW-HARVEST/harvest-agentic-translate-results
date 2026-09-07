//! Shared differential-testing harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls `driver` only via
//! the `.so` exports — never a direct Rust call — so the `#[no_mangle]`
//! `extern "C"` wrapper is on the critical path of every assertion.
//!
//! `driver` returns `void` and communicates purely through `stdout`, so the
//! observable output is captured at the *file descriptor* level (`dup`/`dup2`
//! on fd 1). That is the only way to see what the C `.so` wrote, since it
//! `printf`s into the process-wide glibc `stdout` FILE stream. Both `.so`s and
//! this test binary resolve against the same `libc.so.6`, hence the same
//! `stdout` object.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// ABI of the single public entry point: `void driver(float x)`.
pub type DriverFn = unsafe extern "C" fn(f32);

unsafe extern "C" {
    /// `fflush(NULL)` flushes every open output stream, which drains the
    /// buffered `stdout` of both loaded `.so`s.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

pub struct Api {
    pub c_driver: DriverFn,
    pub r_driver: DriverFn,
}

static API: OnceLock<Api> = OnceLock::new();

/// fd 1 is process-global, so only one capture window may be open at a time.
static FD_LOCK: Mutex<()> = Mutex::new(());

fn target_dir() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // the test executable itself
    if p.file_name().and_then(|s| s.to_str()) == Some("deps") {
        p.pop();
    }
    p
}

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    target_dir().join("libdriver.so")
}

/// Loads both `.so`s once and resolves `driver` from each.
pub fn api() -> &'static Api {
    API.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.is_file(),
            "C shared library not found at {cp:?}; build it with \
             `cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`"
        );
        assert!(
            rp.is_file(),
            "Rust shared library not found at {rp:?}; build it with `cargo build`"
        );
        unsafe {
            // Leaked so the `Library` handles outlive the extracted symbols.
            let c: &'static Library = Box::leak(Box::new(
                Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}")),
            ));
            let r: &'static Library = Box::leak(Box::new(
                Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}")),
            ));
            let cf: Symbol<DriverFn> = c
                .get(b"driver\0")
                .expect("symbol `driver` missing from the C .so");
            let rf: Symbol<DriverFn> = r
                .get(b"driver\0")
                .expect("symbol `driver` missing from the Rust .so");
            Api {
                c_driver: *cf,
                r_driver: *rf,
            }
        }
    })
}

/// Runs `f` with fd 1 redirected to a temporary file and returns every byte
/// written to it.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Drain anything already buffered so it lands on the real stdout, not in
    // our capture file.
    unsafe { fflush(std::ptr::null_mut()) };

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n));

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// The reference output the C is expected to produce for one call: the object
/// representation of the `f32`, in memory order, as lowercase zero-padded hex,
/// then `\n`. Used as an independent cross-check so a test cannot pass merely
/// because both sides are broken the same way.
pub fn expected_line(bits: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(9);
    for b in bits.to_le_bytes() {
        v.extend_from_slice(format!("{b:02x}").as_bytes());
    }
    v.push(b'\n');
    v
}

/// CONFIGS.md row 17: every emitted line is exactly 8 chars from `[0-9a-f]`
/// plus a newline — no uppercase, no `0x` prefix, no separators.
pub fn assert_line_shape(line: &[u8], row: &str, idx: usize) {
    assert_eq!(
        line.len(),
        9,
        "{row}: line {idx} has length {} not 9: {:?}",
        line.len(),
        String::from_utf8_lossy(line)
    );
    assert_eq!(line[8], b'\n', "{row}: line {idx} does not end with \\n");
    for (j, c) in line[..8].iter().enumerate() {
        assert!(
            c.is_ascii_digit() || (b'a'..=b'f').contains(c),
            "{row}: line {idx} char {j} = {:?} is not lowercase hex",
            *c as char
        );
    }
}

/// Drives a whole batch of inputs through BOTH `.so`s and compares the captured
/// stdout byte-for-byte, reporting the first divergent input if they differ.
pub fn run_batch(row: &str, bits: &[u32]) {
    let api = api();

    let c_out = capture(|| {
        for &b in bits {
            unsafe { (api.c_driver)(f32::from_bits(b)) };
        }
    });
    let r_out = capture(|| {
        for &b in bits {
            unsafe { (api.r_driver)(f32::from_bits(b)) };
        }
    });

    // Byte-for-byte equality of the whole stream.
    if c_out != r_out {
        // Locate the first divergence and name the responsible input.
        let lines = bits.len();
        for i in 0..lines {
            let lo = i * 9;
            let c = c_out.get(lo..lo + 9);
            let r = r_out.get(lo..lo + 9);
            if c != r {
                panic!(
                    "{row}: divergence at call {i}, input bits 0x{:08x} (f32 {:?})\n  \
                     C   : {:?}\n  Rust: {:?}\n  expected: {:?}\n  \
                     (C stream {} bytes, Rust stream {} bytes)",
                    bits[i],
                    f32::from_bits(bits[i]),
                    c.map(String::from_utf8_lossy),
                    r.map(String::from_utf8_lossy),
                    String::from_utf8_lossy(&expected_line(bits[i])),
                    c_out.len(),
                    r_out.len()
                );
            }
        }
        panic!(
            "{row}: streams differ in length only: C {} bytes vs Rust {} bytes",
            c_out.len(),
            r_out.len()
        );
    }

    // Total length: exactly 9 bytes per call, no state carried between calls.
    assert_eq!(
        c_out.len(),
        bits.len() * 9,
        "{row}: expected {} bytes for {} calls, got {}",
        bits.len() * 9,
        bits.len(),
        c_out.len()
    );

    // Per-line shape invariant + independent expected value.
    for (i, &b) in bits.iter().enumerate() {
        let line = &c_out[i * 9..i * 9 + 9];
        assert_line_shape(line, row, i);
        assert_eq!(
            line,
            &expected_line(b)[..],
            "{row}: call {i} input 0x{b:08x}: output {:?} does not match the \
             independently computed object representation {:?}",
            String::from_utf8_lossy(line),
            String::from_utf8_lossy(&expected_line(b))
        );
    }
}

/// Deterministic xorshift64* PRNG — fixed seed so every row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
