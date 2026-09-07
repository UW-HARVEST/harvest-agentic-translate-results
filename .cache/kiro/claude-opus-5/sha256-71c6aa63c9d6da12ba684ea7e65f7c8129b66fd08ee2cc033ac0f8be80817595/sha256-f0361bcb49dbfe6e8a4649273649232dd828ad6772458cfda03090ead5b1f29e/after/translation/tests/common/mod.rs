//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and compares their behaviour through the FFI boundary only.
//!
//! Nothing in here calls a Rust function directly — every call goes through a
//! `dlsym`'d symbol, exactly as an external C consumer would, so the
//! `#[no_mangle]` export wrappers are under test too.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::Library;

// ---------------------------------------------------------------------------
// libc bits we need for fd-level stdout capture. Declared directly so we don't
// need the `libc` crate.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// ComputeState — must match the C layout exactly (12 bytes, align 4).
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

impl ComputeState {
    pub fn bytes(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..4].copy_from_slice(&self.accumulator.to_ne_bytes());
        out[4..8].copy_from_slice(&self.operation_count.to_ne_bytes());
        out[8..12].copy_from_slice(&self.checksum.to_ne_bytes());
        out
    }
}

pub type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type OpFnRaw = Option<OpFn>;

// ---------------------------------------------------------------------------
// One loaded implementation (either the C one or the Rust one).
// ---------------------------------------------------------------------------
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub multiply_with_static: OpFn,
    pub add_with_static: OpFn,
    pub xor_operation: OpFn,
    pub shift_with_static: OpFn,
    pub get_operation: unsafe extern "C" fn(c_int) -> OpFnRaw,
    pub execute_operation:
        unsafe extern "C" fn(OpFnRaw, c_int, c_int, *const c_char) -> c_int,
    pub compute_checksum: unsafe extern "C" fn(*mut c_int, c_int) -> c_uint,
    pub init_state: unsafe extern "C" fn(*mut ComputeState, c_int),
    pub apply_operation: unsafe extern "C" fn(*mut ComputeState, c_int, OpFnRaw),
    pub checkshift: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name))
        };
        unsafe { *s.into_raw() }
    }};
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        // RTLD_NOW | RTLD_LOCAL so the two libraries' identically-named
        // symbols never shadow each other.
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let lib: Library = lib.into();

        Impl {
            name,
            multiply_with_static: sym!(lib, "multiply_with_static", OpFn),
            add_with_static: sym!(lib, "add_with_static", OpFn),
            xor_operation: sym!(lib, "xor_operation", OpFn),
            shift_with_static: sym!(lib, "shift_with_static", OpFn),
            get_operation: sym!(lib, "get_operation", unsafe extern "C" fn(c_int) -> OpFnRaw),
            execute_operation: sym!(
                lib,
                "execute_operation",
                unsafe extern "C" fn(OpFnRaw, c_int, c_int, *const c_char) -> c_int
            ),
            compute_checksum: sym!(
                lib,
                "compute_checksum",
                unsafe extern "C" fn(*mut c_int, c_int) -> c_uint
            ),
            init_state: sym!(lib, "init_state", unsafe extern "C" fn(*mut ComputeState, c_int)),
            apply_operation: sym!(
                lib,
                "apply_operation",
                unsafe extern "C" fn(*mut ComputeState, c_int, OpFnRaw)
            ),
            checkshift: sym!(
                lib,
                "checkshift",
                unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
            ),
            _lib: lib,
        }
    }

    /// The four leaf ops in `get_operation` index order.
    pub fn op_by_index(&self, i: usize) -> OpFn {
        match i {
            0 => self.multiply_with_static,
            1 => self.add_with_static,
            2 => self.xor_operation,
            3 => self.shift_with_static,
            _ => panic!("bad op index"),
        }
    }

    pub fn op_name(i: usize) -> &'static str {
        ["multiply_with_static", "add_with_static", "xor_operation", "shift_with_static"][i]
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {} — build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Allow explicitly targeting a profile's .so (used to verify the debug
    // build, where overflow checks are enabled).
    if let Ok(p) = std::env::var("CHECKSHIFT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "CHECKSHIFT_RUST_SO={} does not exist", p.display());
        return p;
    }
    // Prefer the profile the tests were built with, then fall back.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libcheckshift_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libcheckshift_lib.so not found under {} — run `cargo build --release` first",
        root.display()
    )
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level.
//
// Both libraries call the *process's* `printf`, so they share one `stdout`
// FILE. We flush, redirect fd 1 into a temp file, run the closure, flush
// again, restore fd 1, and read back the bytes. This is what makes the
// "byte-identical stdout" comparison meaningful across the FFI boundary.
// ---------------------------------------------------------------------------
/// Global lock serialising fd-1 redirection. MUST be non-generic (a `static`
/// inside a generic fn is instantiated per monomorphisation and would not
/// serialise different closures).
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = std::env::temp_dir().join(format!(
        "cs_cap_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let tmp_c = std::ffi::CString::new(tmp.to_str().unwrap()).unwrap();

    unsafe {
        fflush(std::ptr::null_mut()); // flush ALL streams
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(tmp_c.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open temp failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        let r = f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);

        lseek(fd, 0, 0 /* SEEK_SET */);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        unlink(tmp_c.as_ptr());
        (r, out)
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Assert two captured stdout buffers are byte-identical, showing only a small
/// window around the first difference (transcripts can be megabytes).
pub fn assert_stdout_eq(ctx: &str, c_out: &[u8], r_out: &[u8]) {
    if c_out != r_out {
        let at = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_out.len().min(r_out.len()));
        let lo = at.saturating_sub(160);
        let hi_c = (at + 160).min(c_out.len());
        let hi_r = (at + 160).min(r_out.len());
        panic!(
            "stdout mismatch [{ctx}]\n  first differing byte offset: {at}\n  C   len={} window: \"{}\"\n  Rust len={} window: \"{}\"",
            c_out.len(),
            show(&c_out[lo..hi_c]),
            r_out.len(),
            show(&r_out[lo..hi_r]),
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed → reproducible test inputs).
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5EED_C0FF_EE00_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Uniform in [lo, hi] inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> c_int {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
    /// A value biased towards interesting bit patterns / boundaries.
    pub fn spicy_i32(&mut self) -> c_int {
        match self.next_u64() % 8 {
            0 => 0,
            1 => -1,
            2 => i32::MAX,
            3 => i32::MIN,
            4 => self.range_i32(-16, 16),
            5 => (1i32) << (self.next_u64() % 32) as u32,
            6 => !((1i32) << (self.next_u64() % 32) as u32),
            _ => self.next_i32(),
        }
    }
}

pub const EDGES: [c_int; 12] = [
    0,
    1,
    -1,
    2,
    -2,
    100,
    -100,
    i32::MAX,
    i32::MIN,
    i32::MAX / 3,
    0x5555_5555u32 as i32,
    0xAAAA_AAAAu32 as i32,
];
