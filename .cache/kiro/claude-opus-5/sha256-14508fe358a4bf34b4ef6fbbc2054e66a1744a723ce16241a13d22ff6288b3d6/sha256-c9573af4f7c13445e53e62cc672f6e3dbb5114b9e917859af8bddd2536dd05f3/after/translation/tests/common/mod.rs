//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through `dlsym`, so the `#[no_mangle]` export wrappers are part of
//! what is under test. No Rust function is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(ptr: *mut c_void) -> ();
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    match found.len() {
        0 => panic!(
            "no .so in {} — run: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        ),
        _ => found.remove(0),
    }
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libcharinbuf_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libcharinbuf_lib.so not found — run `cargo build --release` first");
}

/// Resolved path of the C shared object under test.
pub fn c_so() -> PathBuf {
    c_so_path()
}

/// Resolved path of the Rust shared object under test.
pub fn rust_so() -> PathBuf {
    rust_so_path()
}

/// Abort if the Rust `.so` is older than any file in `src/`.
///
/// `cargo test` does **not** rebuild a `cdylib` target, so without this check a
/// whole run can silently pass against a stale shared object. That failure mode
/// is invisible (everything looks green), so it is a hard error here.
fn assert_so_is_fresh(so: &PathBuf) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(&src) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            so_mtime >= t,
            "STALE SHARED OBJECT: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib — run `cargo build --release` first,\n\
             or use ./verify.sh, which does it for you.",
            so.display(),
            p.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Typed view over one loaded library
// ---------------------------------------------------------------------------

pub type IntFn = unsafe extern "C" fn(c_int) -> c_int;
pub type OpFnPtr = *const c_void;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub increment_counter: IntFn,
    pub decrement_counter: IntFn,
    pub multiply_counter: IntFn,
    pub reset_counter: IntFn,
    pub is_string_empty: unsafe extern "C" fn(*const c_char) -> c_int,
    pub find_char_in_buffer: unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char,
    pub create_buffer: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    pub validate_uint16_range: unsafe extern "C" fn(c_int) -> c_int,
    pub apply_operation: unsafe extern "C" fn(OpFnPtr, c_int) -> c_int,
    pub charinbuf: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {{
        let s: libloading::Symbol<_> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Lib {
            name,
            increment_counter: sym!(lib, "increment_counter"),
            decrement_counter: sym!(lib, "decrement_counter"),
            multiply_counter: sym!(lib, "multiply_counter"),
            reset_counter: sym!(lib, "reset_counter"),
            is_string_empty: sym!(lib, "is_string_empty"),
            find_char_in_buffer: sym!(lib, "find_char_in_buffer"),
            create_buffer: sym!(lib, "create_buffer"),
            validate_uint16_range: sym!(lib, "validate_uint16_range"),
            apply_operation: sym!(lib, "apply_operation"),
            charinbuf: sym!(lib, "charinbuf"),
            _lib: lib,
        }
    }

    /// Raw `dlsym` for the four mutators, as an opaque `operation_func` value to
    /// hand back to *this* library's `apply_operation`.
    pub fn op_ptr(&self, which: MutOp) -> OpFnPtr {
        let f: IntFn = match which {
            MutOp::Increment => self.increment_counter,
            MutOp::Decrement => self.decrement_counter,
            MutOp::Multiply => self.multiply_counter,
            MutOp::Reset => self.reset_counter,
        };
        f as *const c_void
    }

    pub fn call_mut(&self, which: MutOp, v: c_int) -> c_int {
        let f: IntFn = match which {
            MutOp::Increment => self.increment_counter,
            MutOp::Decrement => self.decrement_counter,
            MutOp::Multiply => self.multiply_counter,
            MutOp::Reset => self.reset_counter,
        };
        unsafe { f(v) }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MutOp {
    Increment,
    Decrement,
    Multiply,
    Reset,
}

impl MutOp {
    pub const ALL: [MutOp; 4] = [MutOp::Increment, MutOp::Decrement, MutOp::Multiply, MutOp::Reset];
    pub fn from_u32(x: u32) -> MutOp {
        MutOp::ALL[(x % 4) as usize]
    }
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
// Serialises stdout redirection *and* the per-library `static counter`.
static LOCK: Mutex<()> = Mutex::new(());

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let rs = rust_so_path();
        assert_so_is_fresh(&rs);
        Pair {
            c: Lib::open("C", &c_so_path()),
            r: Lib::open("Rust", &rs),
        }
    })
}

/// Acquire the global serialisation lock (poison-tolerant).
pub fn guard() -> MutexGuard<'static, ()> {
    match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected into a temp file; return `(f's value, bytes)`.
///
/// The caller must already hold [`guard`].
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::io::AsRawFd;

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "charinbuf-diff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("temp file");

    unsafe {
        // Drain anything libtest or we buffered in *Rust's* stdout first, so a
        // later flush cannot dump it into the captured file.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let out = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut buf).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
        (out, buf)
    }
}

/// Call `charinbuf` on both libraries and assert return value **and** stdout
/// bytes are identical.
pub fn diff_charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> (c_int, Vec<u8>) {
    let _g = guard();
    diff_charinbuf_locked(mode, value, opt1, opt2)
}

/// Same as [`diff_charinbuf`] but assumes the caller already holds [`guard`].
pub fn diff_charinbuf_locked(
    mode: c_int,
    value: c_int,
    opt1: c_int,
    opt2: c_int,
) -> (c_int, Vec<u8>) {
    let p = pair();
    let (rc_c, out_c) = capture(|| unsafe { (p.c.charinbuf)(mode, value, opt1, opt2) });
    let (rc_r, out_r) = capture(|| unsafe { (p.r.charinbuf)(mode, value, opt1, opt2) });
    assert_eq!(
        rc_c, rc_r,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) return mismatch: C={rc_c} Rust={rc_r}"
    );
    assert_eq!(
        show(&out_c),
        show(&out_r),
        "charinbuf({mode}, {value}, {opt1}, {opt2}) stdout mismatch"
    );
    assert_eq!(out_c, out_r, "charinbuf stdout byte mismatch (non-UTF8)");
    (rc_c, out_c)
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG (SplitMix64) — reproducible property-style inputs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as i32
    }
    /// Uniform in `[0, n)`; `n == 0` yields `0`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn range_i32(&mut self, lo: i32, hi_inclusive: i32) -> c_int {
        let span = (hi_inclusive as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
    /// A byte in `0x01..=0xFF` (never NUL).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    /// An `int` biased toward interesting values.
    pub fn interesting_i32(&mut self) -> c_int {
        const POOL: [c_int; 14] = [
            i32::MIN,
            i32::MIN + 1,
            -65537,
            -65536,
            -65535,
            -2,
            -1,
            0,
            1,
            2,
            65534,
            65535,
            65536,
            i32::MAX,
        ];
        match self.below(3) {
            0 => POOL[self.below(POOL.len() as u32) as usize],
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Small C-string helpers
// ---------------------------------------------------------------------------

/// A NUL-terminated byte buffer usable as `const char *`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Length of a NUL-terminated C string starting at `p`.
pub unsafe fn c_strlen(p: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    n
}

/// Bytes of a NUL-terminated C string (excluding the NUL).
pub unsafe fn c_bytes(p: *const c_char) -> Vec<u8> {
    let n = unsafe { c_strlen(p) };
    (0..n).map(|i| unsafe { *p.add(i) } as u8).collect()
}

pub unsafe fn c_free(p: *mut c_char) {
    unsafe { free(p as *mut c_void) }
}

pub const UINT16_MAX_C: c_uint = 65535;
