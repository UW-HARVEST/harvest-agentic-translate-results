//! Shared differential-test harness.
//!
//! Loads BOTH the C `libmujs.so` and the Rust `libmujs.so` via `libloading` and
//! exposes them as a pair, so every test calls both implementations strictly
//! through their exported C ABI symbols (never via direct Rust calls).
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_short, c_uint, c_ushort, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libmujs.so`
fn c_so_path() -> PathBuf {
    manifest_dir().parent().unwrap().join("c_src/build/libmujs.so")
}

/// `<workdir>/translation/target/<profile>/libmujs.so`, derived from the running
/// test executable so it works for both `debug` and `release` profiles.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libmujs.so
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libmujs.so");
    if p.exists() {
        return p;
    }
    // Fallbacks in case the layout differs.
    for prof in ["release", "debug"] {
        let q = manifest_dir().join("target").join(prof).join("libmujs.so");
        if q.exists() {
            return q;
        }
    }
    panic!("could not locate Rust libmujs.so (looked at {})", p.display());
}

// CMake links the C `libmujs.so` without `-lm`, so `floor`, `sqrt`, `fmod`,
// `ceil`, ... are left undefined in it and must be satisfied from the global
// symbol namespace of the loading process. We are not allowed to touch c_src/,
// so we fix it purely on the loading side: force libm into this test binary's
// link line (giving it a DT_NEEDED entry, hence global scope from startup) and
// additionally dlopen it with RTLD_GLOBAL as a belt-and-braces fallback.
#[link(name = "m")]
extern "C" {
    fn floor(x: f64) -> f64;
    fn ceil(x: f64) -> f64;
    fn sqrt(x: f64) -> f64;
    fn fmod(x: f64, y: f64) -> f64;
}

static LIBM_SINK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn ensure_libm() {
    // Genuinely call them so `--as-needed` cannot drop the libm dependency.
    let v = unsafe { floor(1.5) + ceil(1.5) + sqrt(4.0) + fmod(5.0, 3.0) };
    LIBM_SINK.store(v.to_bits(), std::sync::atomic::Ordering::Relaxed);

    let mut ok = false;
    let mut errs = Vec::new();
    for name in ["libm.so.6", "libm.so", "libc.so.6"] {
        match unsafe {
            libloading::os::unix::Library::open(
                Some(name),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
        } {
            Ok(lib) => {
                // Leak deliberately: it must stay resident and global forever.
                std::mem::forget(lib);
                ok = true;
            }
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    if !ok {
        eprintln!("warning: could not dlopen libm/libc globally: {}", errs.join("; "));
    }
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Both libraries, loaded once per test process.
///
/// `Library::new` uses `RTLD_LOCAL`, so the two libraries' identically-named
/// symbols do not collide: each resolves its own internal references.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        ensure_libm();

        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {} -- build it first", cp.display());
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", cp.display()));
        let rust = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rp.display()));
        Libs { c, rust }
    })
}

/// Look up an exported symbol in a library, panicking with a useful message.
pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get::<T>(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

/// Convenience: `(c_symbol, rust_symbol)` for the same name.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    (sym::<T>(&l.c, name), sym::<T>(&l.rust, name))
}

/// Assert both `.so`s export `name`.
pub fn assert_exports(name: &str) {
    let l = libs();
    unsafe { l.c.get::<*const c_void>(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("C .so does not export `{name}`: {e}"));
    unsafe { l.rust.get::<*const c_void>(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("Rust .so does not export `{name}`: {e}"));
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) -- fixed seeds for reproducibility
// ---------------------------------------------------------------------------

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Pick a random element.
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
    /// A `f64` from the raw bits (covers NaN/Inf/subnormal/huge naturally).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" finite f64 spanning many magnitudes.
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = match self.below(6) {
                0 => self.range_i64(-1000, 1000) as f64,
                1 => self.range_i64(i32::MIN as i64, i32::MAX as i64) as f64,
                2 => self.range_i64(i64::MIN / 2, i64::MAX / 2) as f64,
                3 => (self.next_u32() as f64) / (self.next_u32() as f64 + 1.0),
                4 => {
                    let m = self.range_i64(-(1 << 52), 1 << 52) as f64;
                    let e = self.range_i64(-300, 300) as i32;
                    m * 2f64.powi(e)
                }
                _ => f64::from_bits(self.next_u64()),
            };
            if v.is_finite() {
                return v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Byte / string helpers
// ---------------------------------------------------------------------------

/// NUL-terminated byte buffer usable as `const char *`.
pub fn cstr(s: &str) -> CString {
    CString::new(s.as_bytes().to_vec()).unwrap_or_else(|_| {
        // Contains an interior NUL: truncate at it, matching C string semantics.
        let cut = s.as_bytes().iter().position(|&b| b == 0).unwrap();
        CString::new(&s.as_bytes()[..cut]).unwrap()
    })
}

/// NUL-terminated buffer from raw bytes (allows arbitrary, non-UTF-8 content).
pub fn cbytes(b: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = b.iter().copied().take_while(|&x| x != 0).collect();
    v.push(0);
    v
}

pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Compare two f64 bit-for-bit (so `-0.0 != 0.0` and NaN payloads must match).
pub fn bits_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn fmt_f64(v: f64) -> String {
    format!("{v:?}(0x{:016x})", v.to_bits())
}

// ---------------------------------------------------------------------------
// mujs.h constants
// ---------------------------------------------------------------------------

pub const JS_STRICT: c_int = 1;

pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;

pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;

pub const JS_ISUNDEFINED: c_int = 0;
pub const JS_ISNULL: c_int = 1;
pub const JS_ISBOOLEAN: c_int = 2;
pub const JS_ISNUMBER: c_int = 3;
pub const JS_ISSTRING: c_int = 4;
pub const JS_ISFUNCTION: c_int = 5;
pub const JS_ISOBJECT: c_int = 6;

// regexp.h
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;
pub const REG_MAXSUB: usize = 16;

// jsi.h limits
pub const JS_STACKSIZE: c_int = 4096;
pub const JS_ENVLIMIT: c_int = 1024;
pub const JS_TRYLIMIT: c_int = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResubEnt {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubEnt; REG_MAXSUB],
}

impl Default for Resub {
    fn default() -> Self {
        Resub {
            nsub: 0,
            sub: [ResubEnt { sp: std::ptr::null(), ep: std::ptr::null() }; REG_MAXSUB],
        }
    }
}

impl Resub {
    /// Normalise into offsets relative to `base` so the two implementations'
    /// (necessarily different) pointer values can be compared.
    pub fn offsets(&self, base: *const c_char) -> Vec<Option<(isize, isize)>> {
        (0..REG_MAXSUB)
            .map(|i| {
                let e = self.sub[i];
                if e.sp.is_null() || e.ep.is_null() {
                    None
                } else {
                    Some((
                        unsafe { e.sp.offset_from(base) },
                        unsafe { e.ep.offset_from(base) },
                    ))
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Typed signatures for the exported symbols we exercise
// ---------------------------------------------------------------------------

pub type JsState = *mut c_void;
pub type Reprog = *mut c_void;

// utf.h
pub type FnChartorune = unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int;
pub type FnRunetochar = unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int;
pub type FnRunelen = unsafe extern "C" fn(c_int) -> c_int;
pub type FnRunePred = unsafe extern "C" fn(c_int) -> c_int;
pub type FnRuneMap = unsafe extern "C" fn(c_int) -> c_int;
pub type FnRuneMapFull = unsafe extern "C" fn(c_int) -> *const c_int;

// dtoa / number formatting
pub type FnGrisu2 = unsafe extern "C" fn(c_double, *mut c_char, *mut c_int) -> c_int;
pub type FnFmtexp = unsafe extern "C" fn(*mut c_char, c_int);
pub type FnItoa = unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char;
pub type FnStrtod = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;
pub type FnStrtol = unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> c_double;
pub type FnStringtofloat = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;

// number conversions
pub type FnNumToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnNumToUint = unsafe extern "C" fn(c_double) -> c_uint;
pub type FnNumToShort = unsafe extern "C" fn(c_double) -> c_short;
pub type FnNumToUshort = unsafe extern "C" fn(c_double) -> c_ushort;
pub type FnNumberToString = unsafe extern "C" fn(JsState, *mut c_char, c_double) -> *const c_char;
pub type FnStringToNumber = unsafe extern "C" fn(JsState, *const c_char) -> c_double;

// string / index helpers
pub type FnIsArrayIndex = unsafe extern "C" fn(JsState, *const c_char, *mut c_int) -> c_int;
pub type FnUtflen = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnUtfptrtoidx = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnRuneat = unsafe extern "C" fn(JsState, *const c_char, c_int) -> c_int;
pub type FnIntern = unsafe extern "C" fn(JsState, *const c_char) -> *const c_char;

// lexer helpers
pub type FnCharPred = unsafe extern "C" fn(c_int) -> c_int;
pub type FnTokenString = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnFindword = unsafe extern "C" fn(*const c_char, *const *const c_char, c_int) -> c_int;

// regexp
pub type FnRegcomp = unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> Reprog;
pub type FnRegcompx = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>,
    *mut c_void,
    *const c_char,
    c_int,
    *mut *const c_char,
) -> Reprog;
pub type FnRegexec = unsafe extern "C" fn(Reprog, *const c_char, *mut Resub, c_int) -> c_int;
pub type FnRegfree = unsafe extern "C" fn(Reprog);
pub type FnRegfreex = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>,
    *mut c_void,
    Reprog,
);

// core state API
pub type FnNewstate = unsafe extern "C" fn(*const c_void, *mut c_void, c_int) -> JsState;
pub type FnFreestate = unsafe extern "C" fn(JsState);
pub type FnDostring = unsafe extern "C" fn(JsState, *const c_char) -> c_int;
pub type FnPloadstring = unsafe extern "C" fn(JsState, *const c_char, *const c_char) -> c_int;
pub type FnPcall = unsafe extern "C" fn(JsState, c_int) -> c_int;
pub type FnPconstruct = unsafe extern "C" fn(JsState, c_int) -> c_int;
pub type FnVoid1 = unsafe extern "C" fn(JsState);
pub type FnVoidInt = unsafe extern "C" fn(JsState, c_int);
pub type FnVoidIntInt = unsafe extern "C" fn(JsState, c_int, c_int);
pub type FnVoidDouble = unsafe extern "C" fn(JsState, c_double);
pub type FnVoidStr = unsafe extern "C" fn(JsState, *const c_char);
pub type FnVoidStrInt = unsafe extern "C" fn(JsState, *const c_char, c_int);
pub type FnVoidLStr = unsafe extern "C" fn(JsState, *const c_char, c_int);
pub type FnIntArg = unsafe extern "C" fn(JsState, c_int) -> c_int;
pub type FnIntNoArg = unsafe extern "C" fn(JsState) -> c_int;
pub type FnDoubleArg = unsafe extern "C" fn(JsState, c_int) -> c_double;
pub type FnStrArg = unsafe extern "C" fn(JsState, c_int) -> *const c_char;
pub type FnUintArg = unsafe extern "C" fn(JsState, c_int) -> c_uint;
pub type FnShortArg = unsafe extern "C" fn(JsState, c_int) -> c_short;
pub type FnUshortArg = unsafe extern "C" fn(JsState, c_int) -> c_ushort;
pub type FnIdxStr = unsafe extern "C" fn(JsState, c_int, *const c_char) -> c_int;
pub type FnVoidIdxStr = unsafe extern "C" fn(JsState, c_int, *const c_char);
pub type FnVoidIdxStrInt = unsafe extern "C" fn(JsState, c_int, *const c_char, c_int);
pub type FnVoidIdxInt = unsafe extern "C" fn(JsState, c_int, c_int);
pub type FnTrystring = unsafe extern "C" fn(JsState, c_int, *const c_char) -> *const c_char;
pub type FnTrynumber = unsafe extern "C" fn(JsState, c_int, c_double) -> c_double;
pub type FnTryint = unsafe extern "C" fn(JsState, c_int, c_int) -> c_int;
pub type FnCompare = unsafe extern "C" fn(JsState, *mut c_int) -> c_int;
pub type FnSetlimit = unsafe extern "C" fn(JsState, c_int, c_int);
pub type FnGc = unsafe extern "C" fn(JsState, c_int);
pub type FnNewregexp = unsafe extern "C" fn(JsState, *const c_char, c_int);
pub type FnNextiterator = unsafe extern "C" fn(JsState, c_int) -> *const c_char;
pub type FnRef = unsafe extern "C" fn(JsState) -> *const c_char;
pub type FnUnref = unsafe extern "C" fn(JsState, *const c_char);
pub type FnSetcontext = unsafe extern "C" fn(JsState, *mut c_void);
pub type FnGetcontext = unsafe extern "C" fn(JsState) -> *mut c_void;
pub type FnSetreport = unsafe extern "C" fn(JsState, *const c_void);

// ---------------------------------------------------------------------------
// A single loaded mujs implementation, with the whole API bound.
// ---------------------------------------------------------------------------

/// Thin, per-implementation facade over the exported state API.
pub struct Impl {
    pub name: &'static str,
    pub lib: &'static Library,
}

impl Impl {
    pub fn c() -> Impl {
        Impl { name: "C", lib: &libs().c }
    }
    pub fn rust() -> Impl {
        Impl { name: "Rust", lib: &libs().rust }
    }
    pub fn both() -> (Impl, Impl) {
        (Impl::c(), Impl::rust())
    }

    pub fn f<T>(&self, name: &str) -> Symbol<'static, T> {
        sym::<T>(self.lib, name)
    }

    /// `js_newstate(NULL, NULL, flags)`
    pub fn newstate(&self, flags: c_int) -> JsState {
        let f = self.f::<FnNewstate>("js_newstate");
        let j = unsafe { f(std::ptr::null(), std::ptr::null_mut(), flags) };
        assert!(!j.is_null(), "{}: js_newstate returned NULL", self.name);
        j
    }
    pub fn freestate(&self, j: JsState) {
        let f = self.f::<FnFreestate>("js_freestate");
        unsafe { f(j) }
    }
    pub fn gettop(&self, j: JsState) -> c_int {
        let f = self.f::<FnIntNoArg>("js_gettop");
        unsafe { f(j) }
    }
    pub fn pop(&self, j: JsState, n: c_int) {
        let f = self.f::<FnVoidInt>("js_pop");
        unsafe { f(j, n) }
    }
    pub fn ty(&self, j: JsState, idx: c_int) -> c_int {
        let f = self.f::<FnIntArg>("js_type");
        unsafe { f(j, idx) }
    }
    pub fn typeof_(&self, j: JsState, idx: c_int) -> Vec<u8> {
        let f = self.f::<FnStrArg>("js_typeof");
        unsafe { read_cstr(f(j, idx)) }.unwrap_or_else(|| b"<null>".to_vec())
    }
    /// `js_trystring` -- never longjmps out, so it is safe across FFI.
    pub fn trystring(&self, j: JsState, idx: c_int) -> Vec<u8> {
        let f = self.f::<FnTrystring>("js_trystring");
        let err = cstr("<throw-in-tostring>");
        unsafe { read_cstr(f(j, idx, err.as_ptr())) }.unwrap_or_else(|| b"<null>".to_vec())
    }
    pub fn trynumber(&self, j: JsState, idx: c_int) -> f64 {
        let f = self.f::<FnTrynumber>("js_trynumber");
        unsafe { f(j, idx, f64::from_bits(0x7ff8_0000_dead_beef)) }
    }
    pub fn tryinteger(&self, j: JsState, idx: c_int) -> c_int {
        let f = self.f::<FnTryint>("js_tryinteger");
        unsafe { f(j, idx, -987654) }
    }
    pub fn tryboolean(&self, j: JsState, idx: c_int) -> c_int {
        let f = self.f::<FnTryint>("js_tryboolean");
        unsafe { f(j, idx, -5) }
    }
    pub fn tryrepr(&self, j: JsState, idx: c_int) -> Vec<u8> {
        let f = self.f::<FnTrystring>("js_tryrepr");
        let err = cstr("<throw-in-repr>");
        unsafe { read_cstr(f(j, idx, err.as_ptr())) }.unwrap_or_else(|| b"<null>".to_vec())
    }
    pub fn ploadstring(&self, j: JsState, file: &str, src: &[u8]) -> c_int {
        let f = self.f::<FnPloadstring>("js_ploadstring");
        let fnm = cstr(file);
        let s = cbytes(src);
        unsafe { f(j, fnm.as_ptr(), s.as_ptr() as *const c_char) }
    }
    pub fn pcall(&self, j: JsState, n: c_int) -> c_int {
        let f = self.f::<FnPcall>("js_pcall");
        unsafe { f(j, n) }
    }
    pub fn pconstruct(&self, j: JsState, n: c_int) -> c_int {
        let f = self.f::<FnPconstruct>("js_pconstruct");
        unsafe { f(j, n) }
    }
    pub fn pushundefined(&self, j: JsState) {
        let f = self.f::<FnVoid1>("js_pushundefined");
        unsafe { f(j) }
    }
    pub fn pushnull(&self, j: JsState) {
        let f = self.f::<FnVoid1>("js_pushnull");
        unsafe { f(j) }
    }
    pub fn pushglobal(&self, j: JsState) {
        let f = self.f::<FnVoid1>("js_pushglobal");
        unsafe { f(j) }
    }
    pub fn pushboolean(&self, j: JsState, v: c_int) {
        let f = self.f::<FnVoidInt>("js_pushboolean");
        unsafe { f(j, v) }
    }
    pub fn pushnumber(&self, j: JsState, v: f64) {
        let f = self.f::<FnVoidDouble>("js_pushnumber");
        unsafe { f(j, v) }
    }
    pub fn pushstring(&self, j: JsState, v: &[u8]) {
        let f = self.f::<FnVoidStr>("js_pushstring");
        let s = cbytes(v);
        unsafe { f(j, s.as_ptr() as *const c_char) }
    }
    pub fn pushlstring(&self, j: JsState, v: &[u8], n: c_int) {
        let f = self.f::<FnVoidLStr>("js_pushlstring");
        unsafe { f(j, v.as_ptr() as *const c_char, n) }
    }
    pub fn newobject(&self, j: JsState) {
        let f = self.f::<FnVoid1>("js_newobject");
        unsafe { f(j) }
    }
    pub fn newarray(&self, j: JsState) {
        let f = self.f::<FnVoid1>("js_newarray");
        unsafe { f(j) }
    }
    pub fn gc(&self, j: JsState, report: c_int) {
        let f = self.f::<FnGc>("js_gc");
        unsafe { f(j, report) }
    }
    pub fn setlimit(&self, j: JsState, run: c_int, mem: c_int) {
        let f = self.f::<FnSetlimit>("js_setlimit");
        unsafe { f(j, run, mem) }
    }

    /// Silence the default report handler (which writes to stderr) so test
    /// output stays readable and both impls behave the same way.
    pub fn mute_report(&self, j: JsState) {
        let f = self.f::<FnSetreport>("js_setreport");
        unsafe { f(j, std::ptr::null()) }
    }
}

// ---------------------------------------------------------------------------
// High-level script-evaluation observation
// ---------------------------------------------------------------------------

/// Everything observable about evaluating one script, for byte-exact comparison.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EvalOutcome {
    /// return code of `js_ploadstring`
    pub load_rc: c_int,
    /// return code of `js_pcall` (or -1 if load failed)
    pub call_rc: c_int,
    /// `js_type` of the top of stack after the operation
    pub top_type: c_int,
    /// `js_typeof` of the top of stack
    pub top_typeof: Vec<u8>,
    /// the result value / error, stringified via `js_trystring`
    pub value: Vec<u8>,
    /// stack depth after the operation, before cleanup
    pub top: c_int,
}

impl Impl {
    /// Compile and run `src` as a script, observing the full outcome.
    ///
    /// Uses only the protected (`js_p*`) entry points plus `js_trystring`, so no
    /// `longjmp` ever crosses back out into Rust.
    pub fn eval_script(&self, flags: c_int, src: &[u8]) -> EvalOutcome {
        let j = self.newstate(flags);
        self.mute_report(j);
        let out = self.eval_on(j, src);
        self.freestate(j);
        out
    }

    /// Same as `eval_script` but on a caller-supplied state (so tests can
    /// pre-configure the state, then run several scripts on it).
    pub fn eval_on(&self, j: JsState, src: &[u8]) -> EvalOutcome {
        let load_rc = self.ploadstring(j, "[string]", src);
        if load_rc != 0 {
            let out = EvalOutcome {
                load_rc,
                call_rc: -1,
                top_type: self.ty(j, -1),
                top_typeof: self.typeof_(j, -1),
                value: self.trystring(j, -1),
                top: self.gettop(j),
            };
            self.pop(j, 1);
            return out;
        }
        self.pushundefined(j);
        let call_rc = self.pcall(j, 0);
        let out = EvalOutcome {
            load_rc,
            call_rc,
            top_type: self.ty(j, -1),
            top_typeof: self.typeof_(j, -1),
            value: self.trystring(j, -1),
            top: self.gettop(j),
        };
        self.pop(j, 1);
        out
    }
}

// ---------------------------------------------------------------------------
// In-engine API probes
// ---------------------------------------------------------------------------

/// A probe is a sequence of stack-API calls executed *inside* the engine, as the
/// body of a `js_CFunction` invoked through `js_pcall`. That means a JS-level
/// throw (`js_error` -> `longjmp`) is caught by `js_pcall` instead of unwinding
/// through Rust, so the throwing C-API paths become differentially testable.
pub type ProbeFn = fn(&Impl, JsState);

thread_local! {
    static PROBE_FN: std::cell::Cell<Option<ProbeFn>> = const { std::cell::Cell::new(None) };
    static PROBE_IMPL: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

unsafe extern "C" fn probe_trampoline(j: JsState) {
    let f = PROBE_FN.with(|c| c.get()).expect("probe_trampoline with no probe installed");
    let imp = if PROBE_IMPL.with(|c| c.get()) == 0 { Impl::c() } else { Impl::rust() };
    f(&imp, j);
}

pub type FnNewcfunction =
    unsafe extern "C" fn(JsState, unsafe extern "C" fn(JsState), *const c_char, c_int);
pub type FnNewcfunctionx = unsafe extern "C" fn(
    JsState,
    unsafe extern "C" fn(JsState),
    *const c_char,
    c_int,
    *mut c_void,
    Option<unsafe extern "C" fn(JsState, *mut c_void)>,
);
pub type FnNewcconstructor = unsafe extern "C" fn(
    JsState,
    unsafe extern "C" fn(JsState),
    unsafe extern "C" fn(JsState),
    *const c_char,
    c_int,
);
pub type FnNewuserdata = unsafe extern "C" fn(
    JsState,
    *const c_char,
    *mut c_void,
    Option<unsafe extern "C" fn(JsState, *mut c_void)>,
);
pub type FnTouserdata = unsafe extern "C" fn(JsState, c_int, *const c_char) -> *mut c_void;
pub type FnIsuserdata = unsafe extern "C" fn(JsState, c_int, *const c_char) -> c_int;

impl Impl {
    /// Run `probe` inside a fresh engine and observe the outcome exactly the way
    /// `eval_script` does.
    pub fn run_probe(&self, flags: c_int, probe: ProbeFn) -> EvalOutcome {
        let j = self.newstate(flags);
        self.mute_report(j);
        let out = self.run_probe_on(j, probe);
        self.freestate(j);
        out
    }

    pub fn run_probe_on(&self, j: JsState, probe: ProbeFn) -> EvalOutcome {
        PROBE_FN.with(|c| c.set(Some(probe)));
        PROBE_IMPL.with(|c| c.set(if self.name == "C" { 0 } else { 1 }));
        let newcf = self.f::<FnNewcfunction>("js_newcfunction");
        let name = cstr("probe");
        unsafe { newcf(j, probe_trampoline, name.as_ptr(), 0) };
        self.pushundefined(j); // `this`
        let call_rc = self.pcall(j, 0);
        let out = EvalOutcome {
            load_rc: 0,
            call_rc,
            top_type: self.ty(j, -1),
            top_typeof: self.typeof_(j, -1),
            value: self.trystring(j, -1),
            top: self.gettop(j),
        };
        self.pop(j, 1);
        PROBE_FN.with(|c| c.set(None));
        out
    }
}

impl EvalOutcome {
    /// Human-readable one-liner (the derived Debug prints raw byte vectors).
    pub fn pretty(&self) -> String {
        format!(
            "load_rc={} call_rc={} type={} typeof={} top={} value={:?}",
            self.load_rc,
            self.call_rc,
            self.top_type,
            show(&self.top_typeof),
            self.top,
            show(&self.value)
        )
    }
}

/// First differing position between two byte strings, with context.
pub fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    let i = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    let lo = i.saturating_sub(60);
    let hi_a = (i + 60).min(a.len());
    let hi_b = (i + 60).min(b.len());
    format!(
        "first difference at byte {i} (lens {} vs {}):\n    C   ...{}...\n    Rust...{}...",
        a.len(),
        b.len(),
        show(&a[lo..hi_a]),
        show(&b[lo..hi_b])
    )
}

/// Run the same probe under both implementations and assert identical outcomes.
pub fn assert_probe_eq(flags: c_int, label: &str, probe: ProbeFn) {
    let (c, r) = Impl::both();
    let a = c.run_probe(flags, probe);
    let b = r.run_probe(flags, probe);
    if a != b {
        panic!(
            "probe divergence: {label} (flags={flags})\n  C   : {}\n  Rust: {}\n  {}",
            a.pretty(),
            b.pretty(),
            first_diff(&a.value, &b.value)
        );
    }
}

impl Batch {
    pub fn probe(&mut self, flags: c_int, label: &str, probe: ProbeFn) {
        let (c, r) = Impl::both();
        let a = c.run_probe(flags, probe);
        let b = r.run_probe(flags, probe);
        self.checked += 1;
        if a != b && self.failures.len() < 40 {
            self.failures
                .push(format!("  probe {label} flags={flags}\n      C   : {a:?}\n      Rust: {b:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Additional API surface used by the probes
// ---------------------------------------------------------------------------

impl Impl {
    pub fn copy(&self, j: JsState, idx: c_int) {
        unsafe { self.f::<FnVoidInt>("js_copy")(j, idx) }
    }
    pub fn remove(&self, j: JsState, idx: c_int) {
        unsafe { self.f::<FnVoidInt>("js_remove")(j, idx) }
    }
    pub fn replace(&self, j: JsState, idx: c_int) {
        unsafe { self.f::<FnVoidInt>("js_replace")(j, idx) }
    }
    pub fn dup(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_dup")(j) }
    }
    pub fn dup2(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_dup2")(j) }
    }
    pub fn rot(&self, j: JsState, n: c_int) {
        unsafe { self.f::<FnVoidInt>("js_rot")(j, n) }
    }
    pub fn rot2(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_rot2")(j) }
    }
    pub fn rot3(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_rot3")(j) }
    }
    pub fn rot4(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_rot4")(j) }
    }
    pub fn rot2pop1(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_rot2pop1")(j) }
    }
    pub fn rot3pop2(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_rot3pop2")(j) }
    }
    pub fn concat(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_concat")(j) }
    }
    pub fn equal(&self, j: JsState) -> c_int {
        unsafe { self.f::<FnIntNoArg>("js_equal")(j) }
    }
    pub fn strictequal(&self, j: JsState) -> c_int {
        unsafe { self.f::<FnIntNoArg>("js_strictequal")(j) }
    }
    pub fn instanceof(&self, j: JsState) -> c_int {
        unsafe { self.f::<FnIntNoArg>("js_instanceof")(j) }
    }
    pub fn compare(&self, j: JsState) -> (c_int, c_int) {
        let mut okay: c_int = -99;
        let rc = unsafe { self.f::<FnCompare>("js_compare")(j, &mut okay) };
        (rc, okay)
    }
    pub fn is(&self, j: JsState, which: &str, idx: c_int) -> c_int {
        unsafe { self.f::<FnIntArg>(which)(j, idx) }
    }
    pub fn toboolean(&self, j: JsState, idx: c_int) -> c_int {
        unsafe { self.f::<FnIntArg>("js_toboolean")(j, idx) }
    }
    pub fn tonumber(&self, j: JsState, idx: c_int) -> f64 {
        unsafe { self.f::<FnDoubleArg>("js_tonumber")(j, idx) }
    }
    pub fn tostring(&self, j: JsState, idx: c_int) -> Option<Vec<u8>> {
        unsafe { read_cstr(self.f::<FnStrArg>("js_tostring")(j, idx)) }
    }
    pub fn tointeger(&self, j: JsState, idx: c_int) -> c_int {
        unsafe { self.f::<FnIntArg>("js_tointeger")(j, idx) }
    }
    pub fn toint32(&self, j: JsState, idx: c_int) -> c_int {
        unsafe { self.f::<FnIntArg>("js_toint32")(j, idx) }
    }
    pub fn touint32(&self, j: JsState, idx: c_int) -> c_uint {
        unsafe { self.f::<FnUintArg>("js_touint32")(j, idx) }
    }
    pub fn toint16(&self, j: JsState, idx: c_int) -> c_short {
        unsafe { self.f::<FnShortArg>("js_toint16")(j, idx) }
    }
    pub fn touint16(&self, j: JsState, idx: c_int) -> c_ushort {
        unsafe { self.f::<FnUshortArg>("js_touint16")(j, idx) }
    }
    pub fn getproperty(&self, j: JsState, idx: c_int, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidIdxStr>("js_getproperty")(j, idx, n.as_ptr()) }
    }
    pub fn setproperty(&self, j: JsState, idx: c_int, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidIdxStr>("js_setproperty")(j, idx, n.as_ptr()) }
    }
    pub fn defproperty(&self, j: JsState, idx: c_int, name: &str, atts: c_int) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidIdxStrInt>("js_defproperty")(j, idx, n.as_ptr(), atts) }
    }
    pub fn defaccessor(&self, j: JsState, idx: c_int, name: &str, atts: c_int) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidIdxStrInt>("js_defaccessor")(j, idx, n.as_ptr(), atts) }
    }
    pub fn delproperty(&self, j: JsState, idx: c_int, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidIdxStr>("js_delproperty")(j, idx, n.as_ptr()) }
    }
    pub fn hasproperty(&self, j: JsState, idx: c_int, name: &str) -> c_int {
        let n = cstr(name);
        unsafe { self.f::<FnIdxStr>("js_hasproperty")(j, idx, n.as_ptr()) }
    }
    pub fn getglobal(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_getglobal")(j, n.as_ptr()) }
    }
    pub fn setglobal(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_setglobal")(j, n.as_ptr()) }
    }
    pub fn defglobal(&self, j: JsState, name: &str, atts: c_int) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStrInt>("js_defglobal")(j, n.as_ptr(), atts) }
    }
    pub fn delglobal(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_delglobal")(j, n.as_ptr()) }
    }
    pub fn getindex(&self, j: JsState, idx: c_int, i: c_int) {
        unsafe { self.f::<FnVoidIdxInt>("js_getindex")(j, idx, i) }
    }
    pub fn setindex(&self, j: JsState, idx: c_int, i: c_int) {
        unsafe { self.f::<FnVoidIdxInt>("js_setindex")(j, idx, i) }
    }
    pub fn delindex(&self, j: JsState, idx: c_int, i: c_int) {
        unsafe { self.f::<FnVoidIdxInt>("js_delindex")(j, idx, i) }
    }
    pub fn hasindex(&self, j: JsState, idx: c_int, i: c_int) -> c_int {
        unsafe { self.f::<FnIdxInt>("js_hasindex")(j, idx, i) }
    }
    pub fn getlength(&self, j: JsState, idx: c_int) -> c_int {
        unsafe { self.f::<FnIntArg>("js_getlength")(j, idx) }
    }
    pub fn setlength(&self, j: JsState, idx: c_int, len: c_int) {
        unsafe { self.f::<FnVoidIdxInt>("js_setlength")(j, idx, len) }
    }
    pub fn newregexp(&self, j: JsState, pat: &str, flags: c_int) {
        let p = cstr(pat);
        unsafe { self.f::<FnNewregexp>("js_newregexp")(j, p.as_ptr(), flags) }
    }
    pub fn newstring(&self, j: JsState, v: &str) {
        let s = cstr(v);
        unsafe { self.f::<FnVoidStr>("js_newstring")(j, s.as_ptr()) }
    }
    pub fn newnumber(&self, j: JsState, v: f64) {
        unsafe { self.f::<FnVoidDouble>("js_newnumber")(j, v) }
    }
    pub fn newboolean(&self, j: JsState, v: c_int) {
        unsafe { self.f::<FnVoidInt>("js_newboolean")(j, v) }
    }
    pub fn newobjectx(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_newobjectx")(j) }
    }
    pub fn pushiterator(&self, j: JsState, idx: c_int, own: c_int) {
        unsafe { self.f::<FnVoidIdxInt>("js_pushiterator")(j, idx, own) }
    }
    pub fn nextiterator(&self, j: JsState, idx: c_int) -> Option<Vec<u8>> {
        unsafe { read_cstr(self.f::<FnNextiterator>("js_nextiterator")(j, idx)) }
    }
    pub fn refstr(&self, j: JsState) -> Option<Vec<u8>> {
        unsafe { read_cstr(self.f::<FnRef>("js_ref")(j)) }
    }
    pub fn unref(&self, j: JsState, r: &[u8]) {
        let s = cbytes(r);
        unsafe { self.f::<FnUnref>("js_unref")(j, s.as_ptr() as *const c_char) }
    }
    pub fn getregistry(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_getregistry")(j, n.as_ptr()) }
    }
    pub fn setregistry(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_setregistry")(j, n.as_ptr()) }
    }
    pub fn delregistry(&self, j: JsState, name: &str) {
        let n = cstr(name);
        unsafe { self.f::<FnVoidStr>("js_delregistry")(j, n.as_ptr()) }
    }
    pub fn setcontext(&self, j: JsState, p: *mut c_void) {
        unsafe { self.f::<FnSetcontext>("js_setcontext")(j, p) }
    }
    pub fn getcontext(&self, j: JsState) -> *mut c_void {
        unsafe { self.f::<FnGetcontext>("js_getcontext")(j) }
    }
    pub fn currentfunction(&self, j: JsState) {
        unsafe { self.f::<FnVoid1>("js_currentfunction")(j) }
    }
    pub fn typeof_str(&self, j: JsState, idx: c_int) -> Vec<u8> {
        self.typeof_(j, idx)
    }
    pub fn repr(&self, j: JsState, idx: c_int) {
        unsafe { self.f::<FnVoidInt>("js_repr")(j, idx) }
    }
}

pub type FnIdxInt = unsafe extern "C" fn(JsState, c_int, c_int) -> c_int;

/// The complete picture of one stack slot, using only non-throwing accessors.
#[derive(Debug, PartialEq, Clone)]
pub struct SlotView {
    pub ty: c_int,
    pub tyof: Vec<u8>,
    pub isdefined: c_int,
    pub isundefined: c_int,
    pub isnull: c_int,
    pub isboolean: c_int,
    pub isnumber: c_int,
    pub isstring: c_int,
    pub isprimitive: c_int,
    pub isobject: c_int,
    pub isarray: c_int,
    pub isregexp: c_int,
    pub iscoercible: c_int,
    pub iscallable: c_int,
    pub iserror: c_int,
    pub isnumberobject: c_int,
    pub isstringobject: c_int,
    pub isbooleanobject: c_int,
    pub isdateobject: c_int,
    pub trybool: c_int,
    pub trynum_bits: u64,
    pub tryint: c_int,
    pub trystr: Vec<u8>,
    pub tryrepr: Vec<u8>,
}

impl Impl {
    /// Inspect a stack slot with every non-throwing predicate/accessor.
    pub fn view(&self, j: JsState, idx: c_int) -> SlotView {
        SlotView {
            ty: self.ty(j, idx),
            tyof: self.typeof_(j, idx),
            isdefined: self.is(j, "js_isdefined", idx),
            isundefined: self.is(j, "js_isundefined", idx),
            isnull: self.is(j, "js_isnull", idx),
            isboolean: self.is(j, "js_isboolean", idx),
            isnumber: self.is(j, "js_isnumber", idx),
            isstring: self.is(j, "js_isstring", idx),
            isprimitive: self.is(j, "js_isprimitive", idx),
            isobject: self.is(j, "js_isobject", idx),
            isarray: self.is(j, "js_isarray", idx),
            isregexp: self.is(j, "js_isregexp", idx),
            iscoercible: self.is(j, "js_iscoercible", idx),
            iscallable: self.is(j, "js_iscallable", idx),
            iserror: self.is(j, "js_iserror", idx),
            isnumberobject: self.is(j, "js_isnumberobject", idx),
            isstringobject: self.is(j, "js_isstringobject", idx),
            isbooleanobject: self.is(j, "js_isbooleanobject", idx),
            isdateobject: self.is(j, "js_isdateobject", idx),
            trybool: self.tryboolean(j, idx),
            trynum_bits: self.trynumber(j, idx).to_bits(),
            tryint: self.tryinteger(j, idx),
            trystr: self.trystring(j, idx),
            tryrepr: self.tryrepr(j, idx),
        }
    }
}

// ---------------------------------------------------------------------------
// Subprocess comparison, for paths that deliberately abort the process
// ---------------------------------------------------------------------------

/// Some C paths end in `abort()` by design (an uncaught `js_throw` reaches
/// `js_defaultpanic`, which reports and then falls through to `abort()`).
/// Those cannot be observed in-process, but they are still behaviour that must
/// match, so we run each implementation in its OWN subprocess and compare the
/// exit status, the killing signal, and the captured stderr.
#[derive(Debug, PartialEq, Eq)]
pub struct ProcOutcome {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stderr_tail: String,
    pub markers: Vec<String>,
}

/// Env var the child looks at to know which scenario + side to run.
pub const SUBPROC_ENV: &str = "MUJS_DIFF_SUBPROC";
pub const SUBPROC_SIDE: &str = "MUJS_DIFF_SIDE";

/// Are we the child? Returns `(scenario, side)`.
pub fn subproc_role() -> Option<(String, String)> {
    match (std::env::var(SUBPROC_ENV), std::env::var(SUBPROC_SIDE)) {
        (Ok(a), Ok(b)) => Some((a, b)),
        _ => None,
    }
}

/// Spawn this test binary again, running `runner_test_name`, with the scenario
/// and side selected via the environment.
pub fn run_subproc(runner_test_name: &str, scenario: &str, side: &str) -> ProcOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", runner_test_name, "--nocapture", "--test-threads=1"])
        .env(SUBPROC_ENV, scenario)
        .env(SUBPROC_SIDE, side)
        .output()
        .expect("spawn subprocess");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    // `MARK:` lines are the child's structured, comparable output.
    let markers: Vec<String> = stderr
        .lines()
        .chain(String::from_utf8_lossy(&out.stdout).lines())
        .filter(|l| l.starts_with("MARK:"))
        // Normalise away the child's own identity so only behaviour is compared.
        .map(|l| l.replace("side=c", "side=<impl>").replace("side=rust", "side=<impl>"))
        .collect();
    // Keep only the last few lines of stderr, normalised: it may contain
    // impl-specific noise (Rust panic messages, addresses).
    let tail: String = stderr
        .lines()
        .rev()
        .take(6)
        .filter(|l| {
            l.contains("uncaught exception")
                || l.contains("out of memory")
                || l.contains("stack overflow")
        })
        .map(|l| l.trim().to_string())
        .collect::<Vec<_>>()
        .join("|");
    ProcOutcome {
        exit_code: out.status.code(),
        signal: out.status.signal(),
        stderr_tail: tail,
        markers,
    }
}

/// Compare a deliberately-aborting scenario across the two implementations.
pub fn assert_subproc_eq(runner_test_name: &str, scenario: &str) {
    let c = run_subproc(runner_test_name, scenario, "c");
    let r = run_subproc(runner_test_name, scenario, "rust");
    assert_eq!(
        c, r,
        "subprocess outcome divergence for scenario {scenario:?}\n  C   : {c:?}\n  Rust: {r:?}"
    );
    eprintln!("scenario {scenario:?}: both impls -> {c:?}");
}

/// Print a comparable marker line from inside a child process.
#[macro_export]
macro_rules! mark {
    ($($arg:tt)*) => {
        eprintln!("MARK:{}", format!($($arg)*))
    };
}

/// Run one script under both implementations and assert identical observations.
pub fn assert_script_eq(flags: c_int, src: &str) {
    assert_script_eq_bytes(flags, src.as_bytes(), src);
}

pub fn assert_script_eq_bytes(flags: c_int, src: &[u8], label: &str) {
    let (c, r) = Impl::both();
    let a = c.eval_script(flags, src);
    let b = r.eval_script(flags, src);
    if a != b {
        panic!(
            "script divergence\n  flags = {flags}\n  source = {label:?}\n\
             \n  C   : load_rc={} call_rc={} type={} typeof={:?} top={} value={:?}\
             \n  Rust: load_rc={} call_rc={} type={} typeof={:?} top={} value={:?}",
            a.load_rc, a.call_rc, a.top_type, show(&a.top_typeof), a.top, show(&a.value),
            b.load_rc, b.call_rc, b.top_type, show(&b.top_typeof), b.top, show(&b.value),
        );
    }
}

/// Like `assert_script_eq` but reports every failure in a batch at the end.
#[derive(Default)]
pub struct Batch {
    pub failures: Vec<String>,
    pub checked: usize,
}

impl Batch {
    pub fn new() -> Batch {
        Batch::default()
    }
    pub fn check(&mut self, label: &str, cval: impl std::fmt::Debug, rval: impl std::fmt::Debug) {
        self.checked += 1;
        let cs = format!("{cval:?}");
        let rs = format!("{rval:?}");
        if cs != rs {
            if self.failures.len() < 40 {
                self.failures.push(format!("  {label}\n      C   : {cs}\n      Rust: {rs}"));
            }
        }
    }
    pub fn script(&mut self, flags: c_int, src: &str) {
        let (c, r) = Impl::both();
        let a = c.eval_script(flags, src.as_bytes());
        let b = r.eval_script(flags, src.as_bytes());
        self.checked += 1;
        if a != b {
            if self.failures.len() < 40 {
                self.failures.push(format!(
                    "  flags={flags} src={src:?}\n      C   : {a:?}\n      Rust: {b:?}"
                ));
            }
        }
    }
    pub fn finish(self, what: &str) {
        if !self.failures.is_empty() {
            panic!(
                "{}: {} of {} cases diverged (showing up to 40):\n{}",
                what,
                self.failures.len(),
                self.checked,
                self.failures.join("\n")
            );
        }
        assert!(self.checked > 0, "{what}: no cases were actually checked");
        eprintln!("{what}: {} cases matched", self.checked);
    }
}
