//! Shared differential-test harness.
//!
//! Loads BOTH the C `libmujs.so` and the Rust `libmujs.so` via `libloading`
//! and exposes symbol lookup helpers. Rust functions are NEVER called directly:
//! every call goes through the `.so`'s exported `#[no_mangle]` symbols, exactly
//! as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int, c_short, c_uint, c_ushort, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Rune = c_int;

/// Opaque `js_State *`.
pub type JsState = *mut c_void;
/// Opaque `Reprog *`.
pub type Reprog = *mut c_void;

pub const REG_MAXSUB: usize = 16;

/// Mirror of C `struct Resub` from `regexp.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResubSpan {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubSpan; REG_MAXSUB],
}

impl Default for Resub {
    fn default() -> Self {
        Resub {
            nsub: 0,
            sub: [ResubSpan {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

impl Resub {
    /// Normalise capture spans to `(start, end)` byte offsets relative to `base`
    /// so C and Rust results are comparable without comparing raw addresses.
    pub fn offsets(&self, base: *const c_char) -> Vec<Option<(isize, isize)>> {
        let n = self.nsub.clamp(0, REG_MAXSUB as c_int) as usize;
        (0..n)
            .map(|i| {
                let s = self.sub[i];
                if s.sp.is_null() || s.ep.is_null() {
                    None
                } else {
                    Some((
                        unsafe { s.sp.offset_from(base) },
                        unsafe { s.ep.offset_from(base) },
                    ))
                }
            })
            .collect()
    }
}

/// C `js_Alloc`: `void *(*)(void *memctx, void *ptr, int size)`.
pub type JsAlloc = extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
pub type JsReport = extern "C" fn(JsState, *const c_char);
pub type JsCFunction = extern "C" fn(JsState);
pub type JsFinalize = extern "C" fn(JsState, *mut c_void);
pub type JsHasProperty = extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int;
pub type JsPut = extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int;
pub type JsDelete = extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libmujs.so")
}

pub fn rust_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = manifest.join("target").join(profile).join("libmujs.so");
        if p.exists() {
            return p;
        }
    }
    manifest.join("target/release/libmujs.so")
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // The C CMakeLists.txt does not link libm, so `ceil`/`floor`/`fmod`
        // etc. are undefined in the C `.so`. Load libm with RTLD_GLOBAL first
        // so those symbols resolve. (c_src must not be modified.)
        #[cfg(unix)]
        {
            use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
            for name in ["libm.so.6", "libm.so"] {
                if let Ok(l) = unsafe { UnixLibrary::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                    std::mem::forget(l);
                    break;
                }
            }
        }
        let cp = c_so_path();
        let rp = rust_so_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("cannot load C .so at {}: {e}", cp.display()));
        let rust = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("cannot load Rust .so at {}: {e}", rp.display()));
        Libs { c, rust }
    })
}

/// Fetch a symbol of type `T` from a library, panicking with a clear message.
pub unsafe fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    unsafe {
        lib.get::<T>(&bytes)
            .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"))
    }
}

/// Both implementations of one symbol.
pub struct Pair<T> {
    pub c: T,
    pub rust: T,
}

/// Look up `name` in both libraries and return the two function pointers.
pub fn both_fn<T: Copy + 'static>(name: &str) -> Pair<T> {
    let l = libs();
    unsafe {
        let c: Symbol<'static, T> = sym(&l.c, name);
        let r: Symbol<'static, T> = sym(&l.rust, name);
        Pair { c: *c, rust: *r }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every property test is reproducible.
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
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.below((hi - lo) as u32) as i32)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A double drawn from the full bit space, mixed with "interesting" values.
    pub fn double(&mut self) -> f64 {
        const SPECIAL: &[f64] = &[
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            5e-324,
            2147483647.0,
            2147483648.0,
            -2147483648.0,
            -2147483649.0,
            4294967295.0,
            4294967296.0,
            65535.0,
            65536.0,
            32767.0,
            32768.0,
            -32768.0,
            -32769.0,
            9007199254740991.0,
            9007199254740992.0,
            1e21,
            1e-7,
            1e-6,
            123456789.0,
            0.1,
            1.5,
            2.5,
            -2.5,
            1e300,
            1e-300,
        ];
        match self.below(3) {
            0 => SPECIAL[self.below(SPECIAL.len() as u32) as usize],
            1 => {
                // small magnitude, few significant digits
                let m = self.range_i32(-100000, 100000) as f64;
                let e = self.range_i32(-12, 12);
                m * 10f64.powi(e)
            }
            _ => f64::from_bits(self.next_u64()),
        }
    }
    /// A finite positive double (for `js_grisu2`, which asserts `v > 0`).
    pub fn positive_double(&mut self) -> f64 {
        loop {
            let d = self.double();
            if d.is_finite() && d > 0.0 {
                return d;
            }
        }
    }
    /// Random ASCII-ish byte string of length `len` (no NUL).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (self.range_i32(1, 127)) as u8)
            .collect()
    }
    /// Random byte string of length `len` (no NUL) — may be invalid UTF-8.
    pub fn bytes_nonul(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.range_i32(1, 256)) as u8).collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

pub fn cstr_bytes(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// Read a NUL-terminated C string into owned bytes; `None` for NULL.
pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe { Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec()) }
}

pub fn show(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "<NULL>".to_string(),
        Some(v) => String::from_utf8_lossy(v).into_owned(),
    }
}

/// Bit-exact double comparison that treats all NaNs as equal (and distinguishes
/// +0 from -0, which the C code does observably via `1/x` and `String(x)`).
pub fn same_double(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

// ---------------------------------------------------------------------------
// A loaded MuJS API surface (one per .so). Only what the tests need.
// ---------------------------------------------------------------------------

macro_rules! api {
    ( $( $field:ident : $t:ty = $name:literal ),* $(,)? ) => {
        pub struct Api {
            $( pub $field: $t, )*
        }
        impl Api {
            pub fn load(lib: &Library) -> Api {
                unsafe {
                    Api {
                        $( $field: *sym::<$t>(lib, $name).into_raw(), )*
                    }
                }
            }
        }
    };
}

api! {
    // ---- state ----
    js_newstate: extern "C" fn(Option<JsAlloc>, *mut c_void, c_int) -> JsState = "js_newstate",
    js_freestate: extern "C" fn(JsState) = "js_freestate",
    js_setcontext: extern "C" fn(JsState, *mut c_void) = "js_setcontext",
    js_getcontext: extern "C" fn(JsState) -> *mut c_void = "js_getcontext",
    js_setreport: extern "C" fn(JsState, Option<JsReport>) = "js_setreport",
    js_gc: extern "C" fn(JsState, c_int) = "js_gc",
    js_setlimit: extern "C" fn(JsState, c_int, c_int) = "js_setlimit",
    js_dostring: extern "C" fn(JsState, *const c_char) -> c_int = "js_dostring",
    js_ploadstring: extern "C" fn(JsState, *const c_char, *const c_char) -> c_int = "js_ploadstring",
    js_pcall: extern "C" fn(JsState, c_int) -> c_int = "js_pcall",
    js_pconstruct: extern "C" fn(JsState, c_int) -> c_int = "js_pconstruct",

    // ---- push ----
    js_pushglobal: extern "C" fn(JsState) = "js_pushglobal",
    js_pushundefined: extern "C" fn(JsState) = "js_pushundefined",
    js_pushnull: extern "C" fn(JsState) = "js_pushnull",
    js_pushboolean: extern "C" fn(JsState, c_int) = "js_pushboolean",
    js_pushnumber: extern "C" fn(JsState, c_double) = "js_pushnumber",
    js_pushstring: extern "C" fn(JsState, *const c_char) = "js_pushstring",
    js_pushlstring: extern "C" fn(JsState, *const c_char, c_int) = "js_pushlstring",
    js_pushliteral: extern "C" fn(JsState, *const c_char) = "js_pushliteral",

    // ---- new ----
    js_newobject: extern "C" fn(JsState) = "js_newobject",
    js_newobjectx: extern "C" fn(JsState) = "js_newobjectx",
    js_newarray: extern "C" fn(JsState) = "js_newarray",
    js_newboolean: extern "C" fn(JsState, c_int) = "js_newboolean",
    js_newnumber: extern "C" fn(JsState, c_double) = "js_newnumber",
    js_newstring: extern "C" fn(JsState, *const c_char) = "js_newstring",
    js_newregexp: extern "C" fn(JsState, *const c_char, c_int) = "js_newregexp",
    js_newcfunction: extern "C" fn(JsState, JsCFunction, *const c_char, c_int) = "js_newcfunction",
    js_newcfunctionx: extern "C" fn(JsState, JsCFunction, *const c_char, c_int, *mut c_void, Option<JsFinalize>) = "js_newcfunctionx",
    js_newcconstructor: extern "C" fn(JsState, JsCFunction, JsCFunction, *const c_char, c_int) = "js_newcconstructor",
    js_newuserdata: extern "C" fn(JsState, *const c_char, *mut c_void, Option<JsFinalize>) = "js_newuserdata",
    js_newuserdatax: extern "C" fn(JsState, *const c_char, *mut c_void, Option<JsHasProperty>, Option<JsPut>, Option<JsDelete>, Option<JsFinalize>) = "js_newuserdatax",
    js_currentfunction: extern "C" fn(JsState) = "js_currentfunction",
    js_currentfunctiondata: extern "C" fn(JsState) -> *mut c_void = "js_currentfunctiondata",

    // ---- predicates ----
    js_isdefined: extern "C" fn(JsState, c_int) -> c_int = "js_isdefined",
    js_isundefined: extern "C" fn(JsState, c_int) -> c_int = "js_isundefined",
    js_isnull: extern "C" fn(JsState, c_int) -> c_int = "js_isnull",
    js_isboolean: extern "C" fn(JsState, c_int) -> c_int = "js_isboolean",
    js_isnumber: extern "C" fn(JsState, c_int) -> c_int = "js_isnumber",
    js_isstring: extern "C" fn(JsState, c_int) -> c_int = "js_isstring",
    js_isprimitive: extern "C" fn(JsState, c_int) -> c_int = "js_isprimitive",
    js_isobject: extern "C" fn(JsState, c_int) -> c_int = "js_isobject",
    js_isarray: extern "C" fn(JsState, c_int) -> c_int = "js_isarray",
    js_isregexp: extern "C" fn(JsState, c_int) -> c_int = "js_isregexp",
    js_iscoercible: extern "C" fn(JsState, c_int) -> c_int = "js_iscoercible",
    js_iscallable: extern "C" fn(JsState, c_int) -> c_int = "js_iscallable",
    js_isuserdata: extern "C" fn(JsState, c_int, *const c_char) -> c_int = "js_isuserdata",
    js_iserror: extern "C" fn(JsState, c_int) -> c_int = "js_iserror",
    js_isnumberobject: extern "C" fn(JsState, c_int) -> c_int = "js_isnumberobject",
    js_isstringobject: extern "C" fn(JsState, c_int) -> c_int = "js_isstringobject",
    js_isbooleanobject: extern "C" fn(JsState, c_int) -> c_int = "js_isbooleanobject",
    js_isdateobject: extern "C" fn(JsState, c_int) -> c_int = "js_isdateobject",

    // ---- conversions ----
    js_toboolean: extern "C" fn(JsState, c_int) -> c_int = "js_toboolean",
    js_tonumber: extern "C" fn(JsState, c_int) -> c_double = "js_tonumber",
    js_tostring: extern "C" fn(JsState, c_int) -> *const c_char = "js_tostring",
    js_touserdata: extern "C" fn(JsState, c_int, *const c_char) -> *mut c_void = "js_touserdata",
    js_trystring: extern "C" fn(JsState, c_int, *const c_char) -> *const c_char = "js_trystring",
    js_trynumber: extern "C" fn(JsState, c_int, c_double) -> c_double = "js_trynumber",
    js_tryinteger: extern "C" fn(JsState, c_int, c_int) -> c_int = "js_tryinteger",
    js_tryboolean: extern "C" fn(JsState, c_int, c_int) -> c_int = "js_tryboolean",
    js_tointeger: extern "C" fn(JsState, c_int) -> c_int = "js_tointeger",
    js_toint32: extern "C" fn(JsState, c_int) -> c_int = "js_toint32",
    js_touint32: extern "C" fn(JsState, c_int) -> c_uint = "js_touint32",
    js_toint16: extern "C" fn(JsState, c_int) -> c_short = "js_toint16",
    js_touint16: extern "C" fn(JsState, c_int) -> c_ushort = "js_touint16",

    // ---- stack ----
    js_gettop: extern "C" fn(JsState) -> c_int = "js_gettop",
    js_pop: extern "C" fn(JsState, c_int) = "js_pop",
    js_rot: extern "C" fn(JsState, c_int) = "js_rot",
    js_copy: extern "C" fn(JsState, c_int) = "js_copy",
    js_remove: extern "C" fn(JsState, c_int) = "js_remove",
    js_insert: extern "C" fn(JsState, c_int) = "js_insert",
    js_replace: extern "C" fn(JsState, c_int) = "js_replace",
    js_dup: extern "C" fn(JsState) = "js_dup",
    js_dup2: extern "C" fn(JsState) = "js_dup2",
    js_rot2: extern "C" fn(JsState) = "js_rot2",
    js_rot3: extern "C" fn(JsState) = "js_rot3",
    js_rot4: extern "C" fn(JsState) = "js_rot4",
    js_rot2pop1: extern "C" fn(JsState) = "js_rot2pop1",
    js_rot3pop2: extern "C" fn(JsState) = "js_rot3pop2",

    // ---- properties ----
    js_hasproperty: extern "C" fn(JsState, c_int, *const c_char) -> c_int = "js_hasproperty",
    js_getproperty: extern "C" fn(JsState, c_int, *const c_char) = "js_getproperty",
    js_setproperty: extern "C" fn(JsState, c_int, *const c_char) = "js_setproperty",
    js_defproperty: extern "C" fn(JsState, c_int, *const c_char, c_int) = "js_defproperty",
    js_delproperty: extern "C" fn(JsState, c_int, *const c_char) = "js_delproperty",
    js_defaccessor: extern "C" fn(JsState, c_int, *const c_char, c_int) = "js_defaccessor",
    js_getlength: extern "C" fn(JsState, c_int) -> c_int = "js_getlength",
    js_setlength: extern "C" fn(JsState, c_int, c_int) = "js_setlength",
    js_hasindex: extern "C" fn(JsState, c_int, c_int) -> c_int = "js_hasindex",
    js_getindex: extern "C" fn(JsState, c_int, c_int) = "js_getindex",
    js_setindex: extern "C" fn(JsState, c_int, c_int) = "js_setindex",
    js_delindex: extern "C" fn(JsState, c_int, c_int) = "js_delindex",
    js_getglobal: extern "C" fn(JsState, *const c_char) = "js_getglobal",
    js_setglobal: extern "C" fn(JsState, *const c_char) = "js_setglobal",
    js_defglobal: extern "C" fn(JsState, *const c_char, c_int) = "js_defglobal",
    js_delglobal: extern "C" fn(JsState, *const c_char) = "js_delglobal",
    js_getregistry: extern "C" fn(JsState, *const c_char) = "js_getregistry",
    js_setregistry: extern "C" fn(JsState, *const c_char) = "js_setregistry",
    js_delregistry: extern "C" fn(JsState, *const c_char) = "js_delregistry",
    js_ref: extern "C" fn(JsState) -> *const c_char = "js_ref",
    js_unref: extern "C" fn(JsState, *const c_char) = "js_unref",

    // ---- iterators ----
    js_pushiterator: extern "C" fn(JsState, c_int, c_int) = "js_pushiterator",
    js_nextiterator: extern "C" fn(JsState, c_int) -> *const c_char = "js_nextiterator",

    // ---- operators ----
    js_concat: extern "C" fn(JsState) = "js_concat",
    js_compare: extern "C" fn(JsState, *mut c_int) -> c_int = "js_compare",
    js_equal: extern "C" fn(JsState) -> c_int = "js_equal",
    js_strictequal: extern "C" fn(JsState) -> c_int = "js_strictequal",
    js_instanceof: extern "C" fn(JsState) -> c_int = "js_instanceof",
    js_typeof: extern "C" fn(JsState, c_int) -> *const c_char = "js_typeof",
    js_type: extern "C" fn(JsState, c_int) -> c_int = "js_type",

    // ---- repr ----
    js_repr: extern "C" fn(JsState, c_int) = "js_repr",
    js_torepr: extern "C" fn(JsState, c_int) -> *const c_char = "js_torepr",
    js_tryrepr: extern "C" fn(JsState, c_int, *const c_char) -> *const c_char = "js_tryrepr",
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn capi() -> &'static Api {
    C_API.get_or_init(|| Api::load(&libs().c))
}
pub fn rapi() -> &'static Api {
    R_API.get_or_init(|| Api::load(&libs().rust))
}

/// Run a closure against both APIs and compare the results.
pub fn both_apis() -> (&'static Api, &'static Api) {
    (capi(), rapi())
}

// ---------------------------------------------------------------------------
// `js_dostring` observation: captures return code + top-of-stack string + the
// text passed to a `js_setreport` callback.
// ---------------------------------------------------------------------------

/// Global sink used by the report trampolines below.
pub mod report_sink {
    use std::sync::Mutex;
    pub static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub fn take() -> Vec<String> {
        let mut g = LINES.lock().unwrap();
        std::mem::take(&mut *g)
    }
}

pub extern "C" fn report_trampoline(_j: JsState, msg: *const c_char) {
    let s = unsafe { read_cstr(msg) }
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_else(|| "<NULL>".into());
    report_sink::LINES.lock().unwrap().push(s);
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RunResult {
    pub rc: c_int,
    pub top: Option<String>,
    pub reports: Vec<String>,
    pub gettop: c_int,
}

/// The report sink is process-global, so runs must not interleave.
pub static RUN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Create a state, evaluate `src`, and observe the outcome.
///
/// `flags` is passed straight to `js_newstate` (so out-of-range values are
/// exercised too). `alloc` optionally supplies a custom allocator.
pub fn run_string(
    api: &Api,
    flags: c_int,
    alloc: Option<JsAlloc>,
    actx: *mut c_void,
    with_report: bool,
    src: &str,
) -> RunResult {
    let _guard = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = report_sink::take();
    let j = (api.js_newstate)(alloc, actx, flags);
    assert!(!j.is_null(), "js_newstate returned NULL");
    if with_report {
        (api.js_setreport)(j, Some(report_trampoline));
    }
    let csrc = cstr(src);
    let rc = (api.js_dostring)(j, csrc.as_ptr() as *const c_char);
    let top = if (api.js_gettop)(j) > 0 {
        unsafe { read_cstr((api.js_tostring)(j, -1)) }
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    } else {
        None
    };
    let gettop = (api.js_gettop)(j);
    let reports = report_sink::take();
    (api.js_freestate)(j);
    RunResult {
        rc,
        top,
        reports,
        gettop,
    }
}

/// The prologue every corpus program gets: `o()` records a typed rendering of a
/// value and `oj()` records its JSON form, both accumulated into the global
/// `__out` which the harness reads back after the run.
pub const PROLOGUE: &str = r#"
var __out = "";
function o(x) {
    try { __out += (typeof x) + ":" + String(x) + "|"; }
    catch (e) { __out += "THROW(" + String(e) + ")|"; }
}
function oj(x) {
    try { __out += String(JSON.stringify(x)) + "|"; }
    catch (e) { __out += "THROW(" + String(e) + ")|"; }
}
function ok(f) {
    try { o(f()); } catch (e) { __out += "THROW(" + String(e) + ")|"; }
}
"#;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProgResult {
    pub rc: c_int,
    /// value of the global `__out` after the run
    pub out: Option<String>,
    pub reports: Vec<String>,
    pub gettop: c_int,
}

/// Run `PROLOGUE + src` and read back the global `__out`.
pub fn run_program(api: &Api, flags: c_int, alloc: Option<JsAlloc>, src: &str) -> ProgResult {
    let _guard = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = report_sink::take();
    let j = (api.js_newstate)(alloc, std::ptr::null_mut(), flags);
    assert!(!j.is_null(), "js_newstate returned NULL");
    (api.js_setreport)(j, Some(report_trampoline));
    let full = format!("{PROLOGUE}\n{src}\n");
    let csrc = cstr(&full);
    let rc = (api.js_dostring)(j, csrc.as_ptr() as *const c_char);

    // Read the accumulator without letting a throw escape: `__out` is always a
    // string, so `js_tostring` on it cannot throw.
    let name = cstr("__out");
    let mut out = None;
    let before = (api.js_gettop)(j);
    (api.js_getglobal)(j, name.as_ptr() as *const c_char);
    if (api.js_isstring)(j, -1) != 0 {
        out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
            .map(|b| String::from_utf8_lossy(&b).into_owned());
    }
    (api.js_pop)(j, 1);
    let gettop = (api.js_gettop)(j) - before;
    let reports = report_sink::take();
    (api.js_freestate)(j);
    ProgResult {
        rc,
        out,
        reports,
        gettop,
    }
}

/// Differential program run: assert C and Rust agree exactly.
pub fn assert_same_program(flags: c_int, label: &str, src: &str) {
    let (c, r) = both_apis();
    let a = run_program(c, flags, None, src);
    let b = run_program(r, flags, None, src);
    if a != b {
        panic!(
            "DIVERGENCE [{label}] flags={flags}\n--- source ---\n{src}\n--- C ---\nrc={} gettop={}\nout={:?}\nreports={:?}\n--- RUST ---\nrc={} gettop={}\nout={:?}\nreports={:?}",
            a.rc, a.gettop, a.out, a.reports, b.rc, b.gettop, b.out, b.reports
        );
    }
}

/// Differential `js_dostring`: assert C and Rust agree exactly.
pub fn assert_same_run(flags: c_int, src: &str) {
    let (c, r) = both_apis();
    let a = run_string(c, flags, None, std::ptr::null_mut(), true, src);
    let b = run_string(r, flags, None, std::ptr::null_mut(), true, src);
    assert_eq!(
        a, b,
        "divergence for flags={flags} source:\n{src}\nC={a:?}\nRUST={b:?}"
    );
}
