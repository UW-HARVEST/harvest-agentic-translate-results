//! Shared differential-test harness.
//!
//! Loads BOTH the C `libjansson.so` and the Rust `libjansson.so` through
//! `libloading` and calls them purely through their exported C ABI, exactly as
//! an external consumer would. Rust functions are NEVER called directly, so the
//! `#[no_mangle]` export wrappers are exercised too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_longlong, c_void, CStr, CString};
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- ABI types

pub type json_int_t = c_longlong;

#[repr(C)]
pub struct json_t {
    pub type_: c_int,
    pub refcount: usize,
}

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

#[repr(C)]
#[derive(Clone)]
pub struct json_error_t {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; JSON_ERROR_SOURCE_LENGTH],
    pub text: [c_char; JSON_ERROR_TEXT_LENGTH],
}

impl json_error_t {
    pub fn new() -> Self {
        json_error_t { line: 0, column: 0, position: 0, source: [0; 80], text: [0; 160] }
    }
    /// Mirrors the `json_error_code()` static inline in jansson.h:
    /// `(enum json_error_code)e->text[JSON_ERROR_TEXT_LENGTH - 1]`
    pub fn code(&self) -> i32 {
        self.text[JSON_ERROR_TEXT_LENGTH - 1] as i32
    }
    pub fn text_str(&self) -> String {
        cstr_to_string(self.text.as_ptr())
    }
    pub fn source_str(&self) -> String {
        cstr_to_string(self.source.as_ptr())
    }
    /// Full comparable snapshot of the error struct.
    pub fn snapshot(&self) -> ErrSnap {
        ErrSnap {
            line: self.line,
            column: self.column,
            position: self.position,
            source: self.source_str(),
            text: self.text_str(),
            code: self.code(),
        }
    }
}

impl Default for json_error_t {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ErrSnap {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: String,
    pub text: String,
    pub code: i32,
}

// json_type
pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

// enum json_error_code
pub const JSON_ERROR_UNKNOWN: i32 = 0;
pub const JSON_ERROR_OUT_OF_MEMORY: i32 = 1;
pub const JSON_ERROR_STACK_OVERFLOW: i32 = 2;
pub const JSON_ERROR_CANNOT_OPEN_FILE: i32 = 3;
pub const JSON_ERROR_INVALID_ARGUMENT: i32 = 4;
pub const JSON_ERROR_INVALID_UTF8: i32 = 5;
pub const JSON_ERROR_PREMATURE_END_OF_INPUT: i32 = 6;
pub const JSON_ERROR_END_OF_INPUT_EXPECTED: i32 = 7;
pub const JSON_ERROR_INVALID_SYNTAX: i32 = 8;
pub const JSON_ERROR_INVALID_FORMAT: i32 = 9;
pub const JSON_ERROR_WRONG_TYPE: i32 = 10;
pub const JSON_ERROR_NULL_CHARACTER: i32 = 11;
pub const JSON_ERROR_NULL_VALUE: i32 = 12;
pub const JSON_ERROR_NULL_BYTE_IN_KEY: i32 = 13;
pub const JSON_ERROR_DUPLICATE_KEY: i32 = 14;
pub const JSON_ERROR_NUMERIC_OVERFLOW: i32 = 15;
pub const JSON_ERROR_ITEM_NOT_FOUND: i32 = 16;
pub const JSON_ERROR_INDEX_OUT_OF_RANGE: i32 = 17;

// decoder flags
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

// encoder flags
pub const JSON_MAX_INDENT: usize = 0x1F;
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;

pub fn json_indent(n: usize) -> usize {
    n & JSON_MAX_INDENT
}
pub fn json_real_precision(n: usize) -> usize {
    (n & 0x1F) << 11
}

// pack/unpack flags
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

// ---------------------------------------------------------------- lib loading

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_JANSSON_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    for cand in ["cbuild/libjansson.so", "c_src/build/libjansson.so"] {
        let p = root.join(cand);
        if p.exists() {
            return p;
        }
    }
    panic!("C libjansson.so not found; build it first (see CMakeLists.txt)");
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_JANSSON_SO") {
        return PathBuf::from(p);
    }
    // Pick the artifact matching THIS test binary's profile, deterministically.
    //
    // Do NOT pick "whichever file is newest": if anything rebuilds the other
    // profile concurrently, a newest-wins rule silently swaps which library is
    // under test (and can even dlopen a half-written file). Keying off
    // debug_assertions makes `cargo test` and `cargo test --release` each load
    // their own artifact.
    let root = manifest_dir();
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["target/debug/libjansson.so", "target/release/libjansson.so"]
    } else {
        ["target/release/libjansson.so", "target/debug/libjansson.so"]
    };
    for cand in order {
        let p = root.join(cand);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust libjansson.so not found under target/{{debug,release}}; \
         run `cargo build --release` first"
    )
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Fixed hash seed applied to BOTH libraries before any object is created.
///
/// `json_object_seed(0)` would pull entropy from /dev/urandom, making object
/// iteration order (and therefore `json_dumps` key order) differ between the two
/// libraries and between runs. Seeding both with the same non-zero value makes
/// every object-order-dependent comparison deterministic and reproducible.
/// Note the C only honours this while `hashtable_seed == 0`, i.e. it must happen
/// before the first hashtable use — hence doing it here at load time.
pub const FIXED_SEED: usize = 0x5eed_1234;

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("failed to dlopen C libjansson.so");
        let r = Library::new(rust_so_path()).expect("failed to dlopen Rust libjansson.so");
        type FnSeed = unsafe extern "C" fn(usize);
        let cs: Symbol<FnSeed> = sym(&c, "json_object_seed");
        cs(FIXED_SEED);
        let rs: Symbol<FnSeed> = sym(&r, "json_object_seed");
        rs(FIXED_SEED);
        Libs { c, r }
    })
}

/// Fetch an exported symbol, panicking with a useful message if absent.
pub unsafe fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    lib.get::<T>(name.as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{}` missing from .so: {}", name, e))
}

/// Run `f` against the C library and the Rust library and assert the results
/// are identical. `f` receives the `Library` handle and must drive the API only
/// through exported symbols.
#[track_caller]
pub fn diff<T: PartialEq + Debug>(label: &str, f: impl Fn(&Library) -> T) {
    let l = libs();
    let cv = f(&l.c);
    let rv = f(&l.r);
    assert_eq!(cv, rv, "C/Rust divergence in [{}]\n  C   = {:?}\n  Rust= {:?}", label, cv, rv);
}

/// Like `diff` but the closure also gets a per-iteration index, for
/// property-style randomized runs.
#[track_caller]
pub fn diff_n<T: PartialEq + Debug>(label: &str, n: u64, f: impl Fn(&Library, u64) -> T) {
    let l = libs();
    for i in 0..n {
        let cv = f(&l.c, i);
        let rv = f(&l.r, i);
        assert_eq!(
            cv, rv,
            "C/Rust divergence in [{}] iteration {}\n  C   = {:?}\n  Rust= {:?}",
            label, i, cv, rv
        );
    }
}

// ---------------------------------------------------------------- utilities

pub fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Owned NUL-terminated C string (keeps the CString alive for the call).
pub fn cs(s: &str) -> CString {
    CString::new(s).expect("interior NUL: use cs_bytes instead")
}

/// NUL-terminated buffer that MAY contain interior NUL bytes.
pub fn cs_bytes(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// Arbitrary double from random bits (may be NaN/inf).
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// Finite double in a "reasonable" range.
    pub fn f64_finite(&mut self) -> f64 {
        loop {
            let d = f64::from_bits(self.next_u64());
            if d.is_finite() {
                return d;
            }
        }
    }
    pub fn ascii_string(&mut self, maxlen: usize) -> String {
        let n = (self.below(maxlen as u64 + 1)) as usize;
        (0..n)
            .map(|_| {
                let c = 0x20u8 + (self.below(0x5f) as u8);
                c as char
            })
            .collect()
    }
    /// String drawn from a pool that includes escape-worthy and multi-byte UTF-8.
    pub fn utf8_string(&mut self, maxlen: usize) -> String {
        const POOL: &[&str] = &[
            "a", "Z", "0", " ", "\"", "\\", "/", "\u{8}", "\u{c}", "\n", "\r", "\t", "\u{1}",
            "\u{1f}", "\u{7f}", "é", "ß", "€", "中", "日", "\u{10348}", "𝄞", "😀", "\u{a0}",
            "\u{2028}", "~", "}", "{", "[", "]", ":", ",",
        ];
        let n = (self.below(maxlen as u64 + 1)) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(POOL[self.below(POOL.len() as u64) as usize]);
        }
        s
    }
}

// ---------------------------------------------------------------- fn types

pub type FnVoidPtr = unsafe extern "C" fn() -> *mut json_t;
pub type FnStr = unsafe extern "C" fn(*const c_char) -> *mut json_t;
pub type FnStrN = unsafe extern "C" fn(*const c_char, usize) -> *mut json_t;
pub type FnInt = unsafe extern "C" fn(json_int_t) -> *mut json_t;
pub type FnReal = unsafe extern "C" fn(c_double) -> *mut json_t;
pub type FnDelete = unsafe extern "C" fn(*mut json_t);
pub type FnDumps = unsafe extern "C" fn(*const json_t, usize) -> *mut c_char;
pub type FnDumpb = unsafe extern "C" fn(*const json_t, *mut c_char, usize, usize) -> usize;
pub type FnLoads = unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t;
pub type FnLoadb =
    unsafe extern "C" fn(*const c_char, usize, usize, *mut json_error_t) -> *mut json_t;
pub type FnSize = unsafe extern "C" fn(*const json_t) -> usize;
pub type FnObjGet = unsafe extern "C" fn(*const json_t, *const c_char) -> *mut json_t;
pub type FnObjGetN = unsafe extern "C" fn(*const json_t, *const c_char, usize) -> *mut json_t;
pub type FnObjSetNew = unsafe extern "C" fn(*mut json_t, *const c_char, *mut json_t) -> c_int;
pub type FnObjSetNNew =
    unsafe extern "C" fn(*mut json_t, *const c_char, usize, *mut json_t) -> c_int;
pub type FnObjDel = unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int;
pub type FnObjDelN = unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int;
pub type FnArrGet = unsafe extern "C" fn(*const json_t, usize) -> *mut json_t;
pub type FnArrSetNew = unsafe extern "C" fn(*mut json_t, usize, *mut json_t) -> c_int;
pub type FnArrAppendNew = unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int;
pub type FnArrRemove = unsafe extern "C" fn(*mut json_t, usize) -> c_int;
pub type FnIntVal = unsafe extern "C" fn(*const json_t) -> json_int_t;
pub type FnRealVal = unsafe extern "C" fn(*const json_t) -> c_double;
pub type FnStrVal = unsafe extern "C" fn(*const json_t) -> *const c_char;
pub type FnTwoJson = unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int;
pub type FnEqual = unsafe extern "C" fn(*const json_t, *const json_t) -> c_int;
pub type FnCopy = unsafe extern "C" fn(*mut json_t) -> *mut json_t;
pub type FnDeepCopy = unsafe extern "C" fn(*const json_t) -> *mut json_t;
pub type FnIter = unsafe extern "C" fn(*mut json_t) -> *mut c_void;
pub type FnIterNext = unsafe extern "C" fn(*mut json_t, *mut c_void) -> *mut c_void;
pub type FnIterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
pub type FnIterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnIterValue = unsafe extern "C" fn(*mut c_void) -> *mut json_t;

// ---------------------------------------------------------------- helpers

/// Mirrors the `json_decref` static inline from jansson.h (not exported).
///
/// The real inline would wrap on an over-decref (and then corrupt the heap). We
/// assert instead: an over-decref is always a bug in the TEST's ownership
/// bookkeeping, and silently wrapping makes it invisible in release builds while
/// panicking with "attempt to subtract with overflow" in debug ones.
#[track_caller]
pub unsafe fn decref(lib: &Library, j: *mut json_t) {
    if j.is_null() {
        return;
    }
    if (*j).refcount == usize::MAX {
        return; // singleton (true/false/null): incref/decref are no-ops
    }
    assert!(
        (*j).refcount > 0,
        "over-decref: json_t at {:p} already has refcount 0 (type={}). \
         The test released a reference it did not own — check for a format that \
         STEALS a reference ('o'/'O' semantics) or a container that already owns \
         this child.",
        j,
        (*j).type_
    );
    (*j).refcount -= 1;
    if (*j).refcount == 0 {
        let del: Symbol<FnDelete> = sym(lib, "json_delete");
        del(j);
    }
}

/// Mirrors the `json_incref` static inline from jansson.h (not exported).
pub unsafe fn incref(j: *mut json_t) -> *mut json_t {
    if !j.is_null() && (*j).refcount != usize::MAX {
        (*j).refcount += 1;
    }
    j
}

/// `json_dumps` + convert to an owned String, then free with the library's own
/// allocator-compatible `free` (jansson uses the configured free; default libc).
pub unsafe fn dumps_to_string(lib: &Library, j: *const json_t, flags: usize) -> Option<String> {
    let f: Symbol<FnDumps> = sym(lib, "json_dumps");
    let p = f(j, flags);
    if p.is_null() {
        return None;
    }
    let s = cstr_to_string(p);
    libc_free(p as *mut c_void);
    Some(s)
}

extern "C" {
    fn free(p: *mut c_void);
}

pub unsafe fn libc_free(p: *mut c_void) {
    free(p)
}

/// Parse `text` with `flags`, returning (dumped-round-trip, error snapshot).
/// This is the canonical "did both libraries behave identically" probe.
pub unsafe fn load_then_dump(
    lib: &Library,
    text: &[u8],
    load_flags: usize,
    dump_flags: usize,
) -> (Option<String>, ErrSnap) {
    let loads: Symbol<FnLoadb> = sym(lib, "json_loadb");
    let mut err = json_error_t::new();
    let j = loads(text.as_ptr() as *const c_char, text.len(), load_flags, &mut err);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, dump_flags);
    decref(lib, j);
    (out, err.snapshot())
}
