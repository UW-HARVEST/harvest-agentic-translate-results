//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls every function through the FFI boundary only.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SiphashFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

/// Locate the C shared object built by `c_src/CMakeLists.txt`.
fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no C .so found in {:?}; build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build
    );
    candidates.sort();
    candidates.remove(0)
}

/// Locate the Rust cdylib. `cargo test` puts it next to the test binary's
/// parent directory (`target/<profile>/`).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test bin>  ->  target/<profile>/
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.file_name().map(|s| s == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidates = [
        dir.join("libsiphash_lib.so"),
        manifest_dir().join("target/release/libsiphash_lib.so"),
        manifest_dir().join("target/debug/libsiphash_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib libsiphash_lib.so not found; searched {:?}", candidates);
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

impl Libs {
    pub fn c_hash(&self) -> Symbol<'_, HashBytesFn> {
        unsafe { self.c.get(b"stbds_hash_bytes\0").expect("C stbds_hash_bytes") }
    }
    pub fn rs_hash(&self) -> Symbol<'_, HashBytesFn> {
        unsafe { self.rs.get(b"stbds_hash_bytes\0").expect("Rust stbds_hash_bytes") }
    }
    pub fn c_siphash(&self) -> Symbol<'_, SiphashFn> {
        unsafe { self.c.get(b"siphash\0").expect("C siphash") }
    }
    pub fn rs_siphash(&self) -> Symbol<'_, SiphashFn> {
        unsafe { self.rs.get(b"siphash\0").expect("Rust siphash") }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        eprintln!("C  .so: {}", cp.display());
        eprintln!("RS .so: {}", rp.display());
        unsafe {
            Libs {
                c: Library::new(&cp).expect("load C .so"),
                rs: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

/// Assert that both `.so`s compute the same `stbds_hash_bytes` value.
#[track_caller]
pub fn assert_hash_eq(buf: &mut [u8], len: usize, seed: usize, ctx: &str) -> usize {
    let l = libs();
    let ch = l.c_hash();
    let rh = l.rs_hash();
    let p = buf.as_mut_ptr() as *mut c_void;
    let c_val = unsafe { ch(p, len, seed) };
    let r_val = unsafe { rh(p, len, seed) };
    assert_eq!(
        c_val, r_val,
        "stbds_hash_bytes divergence [{}]: len={} seed={:#018x} C={:#018x} RS={:#018x}\n\
         first {} bytes = {:02x?}",
        ctx,
        len,
        seed,
        c_val,
        r_val,
        len.min(buf.len()),
        &buf[..len.min(buf.len())]
    );
    c_val
}

/// Same but with an explicitly-provided raw pointer (for null / unaligned tests).
#[track_caller]
pub fn assert_hash_eq_raw(p: *mut c_void, len: usize, seed: usize, ctx: &str) -> usize {
    let l = libs();
    let ch = l.c_hash();
    let rh = l.rs_hash();
    let c_val = unsafe { ch(p, len, seed) };
    let r_val = unsafe { rh(p, len, seed) };
    assert_eq!(
        c_val, r_val,
        "stbds_hash_bytes divergence [{}]: p={:p} len={} seed={:#018x} C={:#018x} RS={:#018x}",
        ctx, p, len, seed, c_val, r_val
    );
    c_val
}

// ---------------------------------------------------------------------------
// stdout capture via the `siphash_driver` child process.
//
// Capturing fd 1 in-process is not reliable under `cargo test`: libtest's own
// progress output goes to the same fd from other threads and lands inside the
// capture.  Instead we spawn `siphash_driver <so> <init>`, which dlopen()s the
// given library, calls `siphash`, and writes NOTHING else to stdout.  The
// child's stdout is therefore exactly the bytes the library printf'd.
// ---------------------------------------------------------------------------

fn driver_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_siphash_driver"))
}

/// Run `siphash(init)` from the shared object at `so` inside a fresh process
/// and return its raw stdout bytes.
pub fn driver_stdout(so: &std::path::Path, init: c_int) -> Vec<u8> {
    let out = std::process::Command::new(driver_path())
        .arg(so)
        .arg(init.to_string())
        .output()
        .expect("spawn siphash_driver");
    assert!(
        out.status.success(),
        "siphash_driver {:?} {} failed: status={:?} stderr={}",
        so,
        init,
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

pub fn c_so() -> PathBuf {
    c_so_path()
}
pub fn rust_so() -> PathBuf {
    rust_so_path()
}

/// Capture stdout of `siphash(init)` from both libraries and compare bytes.
#[track_caller]
pub fn assert_siphash_stdout_eq(init: c_int) {
    let c_out = driver_stdout(&c_so(), init);
    let r_out = driver_stdout(&rust_so(), init);
    if c_out != r_out {
        let c_s = String::from_utf8_lossy(&c_out);
        let r_s = String::from_utf8_lossy(&r_out);
        let first_diff = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c_out.len().min(r_out.len()));
        let cl: Vec<&str> = c_s.lines().collect();
        let rl: Vec<&str> = r_s.lines().collect();
        let mut detail = String::new();
        for i in 0..cl.len().max(rl.len()) {
            let a = cl.get(i).copied().unwrap_or("<missing>");
            let b = rl.get(i).copied().unwrap_or("<missing>");
            if a != b {
                detail.push_str(&format!("  line {}:\n    C : {}\n    RS: {}\n", i, a, b));
            }
        }
        panic!(
            "siphash({}) stdout divergence: C={} bytes, RS={} bytes, first differing byte at {}\n{}",
            init,
            c_out.len(),
            r_out.len(),
            first_diff,
            detail
        );
    }
    assert!(!c_out.is_empty(), "siphash({}) produced no output", init);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = (self.next_u64() >> 24) as u8;
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}

pub const SEED: u64 = 0x5eed_1234_abcd_0001;
pub const N: usize = 256;
