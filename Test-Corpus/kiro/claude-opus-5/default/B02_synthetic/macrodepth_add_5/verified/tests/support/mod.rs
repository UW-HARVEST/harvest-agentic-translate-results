// Shared differential-test harness.
//
// Both implementations are loaded as shared objects with `libloading` and
// driven only through their exported C symbols -- the Rust functions are never
// called directly, so the `#[no_mangle] extern "C"` wrappers, the exported
// data objects and their ELF placement are all under test.
//
//   C    : ../cbuild/lib/libmd_<op>_<repeat>.so   (gcc -shared on c_src/src/mdcore.c)
//   Rust : target/release/libdriver.so            (cargo build --release)
//
// The active configuration is taken from the same Cargo features the crate
// itself uses, with the same precedence as `src/mdmacros.rs`, so
// `cargo test --no-default-features --features <op>,repeat_<n>` automatically
// selects the matching C library.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

/* ------------------------------------------------------------------ */
/* Active configuration (mirrors the cfg precedence in mdmacros.rs)   */
/* ------------------------------------------------------------------ */

/// `OP` -- `sub` wins over `mul` wins over `add`; no OP feature => `add`,
/// mirroring `#ifndef OP / #define OP add`.
pub const OP: &str = if cfg!(feature = "sub") {
    "sub"
} else if cfg!(feature = "mul") {
    "mul"
} else {
    "add"
};

/// `REPEAT` -- same resolution order as `src/mdmacros.rs`; no REPEAT feature
/// => 5, mirroring `#ifndef REPEAT / #define REPEAT 5`.
pub const REPEAT: c_int = if cfg!(feature = "repeat_0") {
    0
} else if cfg!(feature = "repeat_1") {
    1
} else if cfg!(feature = "repeat_2") {
    2
} else if cfg!(feature = "repeat_3") {
    3
} else if cfg!(feature = "repeat_4") {
    4
} else if cfg!(feature = "repeat_6") {
    6
} else if cfg!(feature = "repeat_7") {
    7
} else {
    5
};

/// `INIT_FOR(OP)` -- only used to document expectations in error-path tests.
pub const INIT: c_int = if cfg!(feature = "sub") {
    0
} else if cfg!(feature = "mul") {
    1
} else {
    0
};

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_LIB") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .join("..")
        .join("cbuild")
        .join("lib")
        .join(format!("libmd_{OP}_{REPEAT}.so"))
}

pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_LIB") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    let release = base.join("release").join("libdriver.so");
    if release.exists() {
        release
    } else {
        base.join("debug").join("libdriver.so")
    }
}

pub fn c_exe_path() -> PathBuf {
    manifest_dir()
        .join("..")
        .join("cbuild")
        .join("exe")
        .join(format!("driver_{OP}_{REPEAT}"))
}

pub fn rust_exe_path() -> PathBuf {
    let base = manifest_dir().join("target");
    let release = base.join("release").join("driver");
    if release.exists() {
        release
    } else {
        base.join("debug").join("driver")
    }
}

/* ------------------------------------------------------------------ */
/* The two loaded libraries                                           */
/* ------------------------------------------------------------------ */

pub type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{name} shared object not found: {}\n\
             build the C libs with the gcc loop documented in SYMBOLS.md and the\n\
             Rust cdylib with `cargo build --release --no-default-features --features {OP},repeat_{REPEAT}`",
            path.display()
        );
        // Safety: loading a plain C ABI library with no initialisers beyond the
        // usual CRT ones.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Impl { name, lib }
    }

    /// A `T` (`int`,`int`) -> `int` function symbol.
    pub fn op(&self, sym: &str) -> OpFn {
        let mut n = sym.as_bytes().to_vec();
        n.push(0);
        let s: Symbol<OpFn> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {sym}: {e}", self.name));
        *s
    }

    /// A `T` (`int`) -> `int` function symbol.
    pub fn unary(&self, sym: &str) -> UnaryFn {
        let mut n = sym.as_bytes().to_vec();
        n.push(0);
        let s: Symbol<UnaryFn> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {sym}: {e}", self.name));
        *s
    }

    /// Address of the `G_OP` *data object* (not a function symbol).
    pub fn g_op_slot(&self) -> *mut OpFn {
        let s: Symbol<*mut OpFn> = unsafe { self.lib.get(b"G_OP\0") }
            .unwrap_or_else(|e| panic!("{}: missing data symbol G_OP: {e}", self.name));
        *s
    }

    /// Address of the `G_OP_NAME` *data object*.
    pub fn g_op_name_slot(&self) -> *mut *const c_char {
        let s: Symbol<*mut *const c_char> = unsafe { self.lib.get(b"G_OP_NAME\0") }
            .unwrap_or_else(|e| panic!("{}: missing data symbol G_OP_NAME: {e}", self.name));
        *s
    }

    pub fn g_op(&self) -> OpFn {
        unsafe { *self.g_op_slot() }
    }

    pub fn g_op_name(&self) -> Vec<u8> {
        unsafe { CStr::from_ptr(*self.g_op_name_slot()).to_bytes().to_vec() }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let p = Pair {
            c: Impl::load("C", &c_lib_path()),
            rust: Impl::load("Rust", &rust_lib_path()),
        };
        // Guard against a stale / mismatched Rust build silently passing:
        // both libraries must agree on the compiled-in OP name, and it must
        // be the OP this test binary was compiled for.
        let cn = p.c.g_op_name();
        let rn = p.rust.g_op_name();
        assert_eq!(
            String::from_utf8_lossy(&cn),
            OP,
            "C library {} does not match the tested OP",
            c_lib_path().display()
        );
        assert_eq!(
            String::from_utf8_lossy(&rn),
            OP,
            "Rust library {} was built for a different OP -- rebuild with \
             --no-default-features --features {OP},repeat_{REPEAT}",
            rust_lib_path().display()
        );
        p
    })
}

/* ------------------------------------------------------------------ */
/* stdout capture (the C code uses printf, the Rust code println!)    */
/* ------------------------------------------------------------------ */

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// fd 1 redirection is process-global, so every capture is serialised.
fn capture_lock() -> MutexGuard<'static, CaptureState> {
    static L: OnceLock<Mutex<CaptureState>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(CaptureState::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub struct CaptureState {
    file: std::fs::File,
}

impl CaptureState {
    fn new() -> CaptureState {
        let path = std::env::temp_dir().join(format!(
            "driver_diff_{}_{}_{}.out",
            OP,
            REPEAT,
            std::process::id()
        ));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("temp capture file");
        let _ = std::fs::remove_file(&path); // unlinked; the handle keeps it alive
        CaptureState { file }
    }
}

/// Run `f` with fd 1 pointing at a scratch file and return everything it wrote.
///
/// `fflush(NULL)` drains glibc's `stdout` FILE buffer (shared with the C `.so`,
/// which is fully buffered because fd 1 is a regular file here). The Rust `.so`
/// has its own `std` with a `LineWriter`, which flushes on every `\n`.
pub fn with_stdout_capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut st = capture_lock();
    unsafe {
        fflush(std::ptr::null_mut());
    }
    st.file.set_len(0).unwrap();
    st.file.seek(SeekFrom::Start(0)).unwrap();

    let fd = {
        use std::os::unix::io::AsRawFd;
        st.file.as_raw_fd()
    };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 failed");

    let r = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    let mut buf = Vec::new();
    st.file.seek(SeekFrom::Start(0)).unwrap();
    st.file.read_to_end(&mut buf).unwrap();
    st.file.set_len(0).unwrap();
    (r, buf)
}

/// Call the same symbol in both libraries and assert the return value *and*
/// the bytes written to stdout are identical.
pub fn diff_op(sym: &str, a: c_int, b: c_int) {
    let p = pair();
    let cf = p.c.op(sym);
    let rf = p.rust.op(sym);
    let (cr, cout) = with_stdout_capture(|| unsafe { cf(a, b) });
    let (rr, rout) = with_stdout_capture(|| unsafe { rf(a, b) });
    assert_eq!(
        cr, rr,
        "{sym}({a}, {b}) return value: C={cr} Rust={rr}  [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{sym}({a}, {b}) stdout  [OP={OP} REPEAT={REPEAT}]"
    );
}

/// Same, for the one-argument entry point.
pub fn diff_unary(sym: &str, n: c_int) {
    let p = pair();
    let cf = p.c.unary(sym);
    let rf = p.rust.unary(sym);
    let (cr, cout) = with_stdout_capture(|| unsafe { cf(n) });
    let (rr, rout) = with_stdout_capture(|| unsafe { rf(n) });
    assert_eq!(
        cr, rr,
        "{sym}({n}) return value: C={cr} Rust={rr}  [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{sym}({n}) stdout  [OP={OP} REPEAT={REPEAT}]"
    );
}

/// Call through the `G_OP` global function pointer in both libraries.
pub fn diff_g_op(a: c_int, b: c_int) {
    let p = pair();
    let cf = p.c.g_op();
    let rf = p.rust.g_op();
    let cr = unsafe { cf(a, b) };
    let rr = unsafe { rf(a, b) };
    assert_eq!(
        cr, rr,
        "G_OP({a}, {b}): C={cr} Rust={rr}  [OP={OP} REPEAT={REPEAT}]"
    );
}

/* ------------------------------------------------------------------ */
/* Deterministic input generation                                     */
/* ------------------------------------------------------------------ */

/// SplitMix64 -- fixed seed, so every run uses the same inputs.
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
    /// A value biased towards interesting magnitudes: full-range, 16-bit,
    /// 8-bit, tiny, and exact boundary values all appear.
    pub fn next_i32(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 8) as u32 as i32,             // full 32-bit range
            1 => ((r >> 8) as u16 as i32) - 32768,   // 16-bit signed-ish
            2 => ((r >> 8) as u8 as i32) - 128,      // 8-bit
            3 => ((r >> 8) % 11) as i32 - 5,         // tiny
            4 => BOUNDARY[(r >> 8) as usize % BOUNDARY.len()],
            5 => i32::MAX - ((r >> 8) % 4) as i32,
            6 => i32::MIN + ((r >> 8) % 4) as i32,
            _ => (r >> 32) as u32 as i32,
        }
    }
}

/// Every boundary value the arithmetic in `op_*` / `STEP_*` can pivot on.
pub const BOUNDARY: [c_int; 17] = [
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    65535,
    -65536,
    65536,
    46340,  // floor(sqrt(INT_MAX)) -- last non-overflowing square
    46341,  // first overflowing square
    -46341,
];

/// The `n` values `DISPATCH_REP` distinguishes: `case 0..=6` and `default`.
pub const DISPATCH_IN_RANGE: [c_int; 7] = [0, 1, 2, 3, 4, 5, 6];
pub const DISPATCH_OUT_OF_RANGE: [c_int; 10] =
    [7, 8, 9, 100, -1, -2, -7, i32::MIN, i32::MAX, i32::MIN + 1];
