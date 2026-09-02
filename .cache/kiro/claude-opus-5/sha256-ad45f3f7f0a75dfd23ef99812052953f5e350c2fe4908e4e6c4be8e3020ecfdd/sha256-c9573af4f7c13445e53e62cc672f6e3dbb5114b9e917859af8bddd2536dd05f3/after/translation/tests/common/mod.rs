//! Differential-test harness.
//!
//! Loads BOTH shared objects (the C reference build and the Rust translation)
//! with `libloading` and resolves every symbol under test.  Nothing in this
//! module calls a Rust function directly — every call goes through the `.so`
//! export table, exactly like an external C consumer.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_short, c_uint, c_ushort, c_void};
use std::path::PathBuf;

pub type JS = *mut c_void;
pub type Rune = c_int;
pub type CFun = Option<unsafe extern "C-unwind" fn(JS)>;
pub type ReportFn = Option<unsafe extern "C-unwind" fn(JS, *const c_char)>;
pub type FinalizeFn = Option<unsafe extern "C-unwind" fn(JS, *mut c_void)>;
pub type HasFn = Option<unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int>;
pub type PutFn = Option<unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int>;
pub type DelFn = Option<unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int>;
pub type AllocFn = Option<unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type RegAllocFn = Option<unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;

pub const REG_MAXSUB: usize = 16;

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
            sub: [ResubEnt {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

/* ------------------------------------------------------------------ */
/* Generated API struct                                                */
/* ------------------------------------------------------------------ */

macro_rules! decl_api {
    ( $( fn $name:ident ( $($at:ty),* $(,)? ) $(-> $rt:ty)? ; )* ) => {
        pub struct Api {
            pub which: &'static str,
            _lib: &'static libloading::Library,
            $( pub $name: unsafe extern "C-unwind" fn($($at),*) $(-> $rt)?, )*
        }
        impl Api {
            pub fn load(which: &'static str, path: &std::path::Path) -> Api {
                let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
                    libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {}: {}", path.display(), e))
                }));
                unsafe {
                    Api {
                        which,
                        _lib: lib,
                        $( $name: *lib
                            .get::<unsafe extern "C-unwind" fn($($at),*) $(-> $rt)?>(
                                concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("{} missing {}: {}", which, stringify!($name), e)), )*
                    }
                }
            }
        }
    }
}

decl_api! {
    /* ---- state ---- */
    fn js_newstate(AllocFn, *mut c_void, c_int) -> JS;
    fn js_freestate(JS);
    fn js_gc(JS, c_int);
    fn js_setlimit(JS, c_int, c_int);
    fn js_setcontext(JS, *mut c_void);
    fn js_getcontext(JS) -> *mut c_void;
    fn js_setreport(JS, ReportFn);
    fn js_report(JS, *const c_char);
    fn js_dostring(JS, *const c_char) -> c_int;
    fn js_ploadstring(JS, *const c_char, *const c_char) -> c_int;
    fn js_pcall(JS, c_int) -> c_int;
    fn js_pconstruct(JS, c_int) -> c_int;
    fn js_loadstring(JS, *const c_char, *const c_char);
    fn js_loadeval(JS, *const c_char, *const c_char);
    fn js_eval(JS);
    fn js_call(JS, c_int);
    fn js_construct(JS, c_int);
    fn js_throw(JS);
    fn js_endtry(JS);
    fn js_savetry(JS) -> *mut c_void;
    fn js_gettop(JS) -> c_int;

    /* ---- stack ---- */
    fn js_pop(JS, c_int);
    fn js_rot(JS, c_int);
    fn js_copy(JS, c_int);
    fn js_remove(JS, c_int);
    fn js_insert(JS, c_int);
    fn js_replace(JS, c_int);
    fn js_dup(JS);
    fn js_dup2(JS);
    fn js_rot2(JS);
    fn js_rot3(JS);
    fn js_rot4(JS);
    fn js_rot2pop1(JS);
    fn js_rot3pop2(JS);

    /* ---- push ---- */
    fn js_pushglobal(JS);
    fn js_pushundefined(JS);
    fn js_pushnull(JS);
    fn js_pushboolean(JS, c_int);
    fn js_pushnumber(JS, f64);
    fn js_pushstring(JS, *const c_char);
    fn js_pushlstring(JS, *const c_char, c_int);
    fn js_pushliteral(JS, *const c_char);

    /* ---- constructors ---- */
    fn js_newobjectx(JS);
    fn js_newobject(JS);
    fn js_newarray(JS);
    fn js_newboolean(JS, c_int);
    fn js_newnumber(JS, f64);
    fn js_newstring(JS, *const c_char);
    fn js_newcfunction(JS, CFun, *const c_char, c_int);
    fn js_newcfunctionx(JS, CFun, *const c_char, c_int, *mut c_void, FinalizeFn);
    fn js_newcconstructor(JS, CFun, CFun, *const c_char, c_int);
    fn js_newuserdata(JS, *const c_char, *mut c_void, FinalizeFn);
    fn js_newuserdatax(JS, *const c_char, *mut c_void, HasFn, PutFn, DelFn, FinalizeFn);
    fn js_newregexp(JS, *const c_char, c_int);

    /* ---- error objects (non-throwing) ---- */
    fn js_newerror(JS, *const c_char);
    fn js_newevalerror(JS, *const c_char);
    fn js_newrangeerror(JS, *const c_char);
    fn js_newreferenceerror(JS, *const c_char);
    fn js_newsyntaxerror(JS, *const c_char);
    fn js_newtypeerror(JS, *const c_char);
    fn js_newurierror(JS, *const c_char);

    /* ---- predicates ---- */
    fn js_isdefined(JS, c_int) -> c_int;
    fn js_isundefined(JS, c_int) -> c_int;
    fn js_isnull(JS, c_int) -> c_int;
    fn js_isboolean(JS, c_int) -> c_int;
    fn js_isnumber(JS, c_int) -> c_int;
    fn js_isstring(JS, c_int) -> c_int;
    fn js_isprimitive(JS, c_int) -> c_int;
    fn js_isobject(JS, c_int) -> c_int;
    fn js_isarray(JS, c_int) -> c_int;
    fn js_isregexp(JS, c_int) -> c_int;
    fn js_iscoercible(JS, c_int) -> c_int;
    fn js_iscallable(JS, c_int) -> c_int;
    fn js_isuserdata(JS, c_int, *const c_char) -> c_int;
    fn js_iserror(JS, c_int) -> c_int;
    fn js_isnumberobject(JS, c_int) -> c_int;
    fn js_isstringobject(JS, c_int) -> c_int;
    fn js_isbooleanobject(JS, c_int) -> c_int;
    fn js_isdateobject(JS, c_int) -> c_int;

    /* ---- conversions ---- */
    fn js_toboolean(JS, c_int) -> c_int;
    fn js_tonumber(JS, c_int) -> f64;
    fn js_tostring(JS, c_int) -> *const c_char;
    fn js_touserdata(JS, c_int, *const c_char) -> *mut c_void;
    fn js_trystring(JS, c_int, *const c_char) -> *const c_char;
    fn js_trynumber(JS, c_int, f64) -> f64;
    fn js_tryinteger(JS, c_int, c_int) -> c_int;
    fn js_tryboolean(JS, c_int, c_int) -> c_int;
    fn js_tointeger(JS, c_int) -> c_int;
    fn js_toint32(JS, c_int) -> c_int;
    fn js_touint32(JS, c_int) -> c_uint;
    fn js_toint16(JS, c_int) -> c_short;
    fn js_touint16(JS, c_int) -> c_ushort;
    fn js_typeof(JS, c_int) -> *const c_char;
    fn js_type(JS, c_int) -> c_int;
    fn js_repr(JS, c_int);
    fn js_torepr(JS, c_int) -> *const c_char;
    fn js_tryrepr(JS, c_int, *const c_char) -> *const c_char;
    fn js_toprimitive(JS, c_int, c_int);

    /* ---- properties ---- */
    fn js_hasproperty(JS, c_int, *const c_char) -> c_int;
    fn js_getproperty(JS, c_int, *const c_char);
    fn js_setproperty(JS, c_int, *const c_char);
    fn js_defproperty(JS, c_int, *const c_char, c_int);
    fn js_delproperty(JS, c_int, *const c_char);
    fn js_defaccessor(JS, c_int, *const c_char, c_int);
    fn js_getlength(JS, c_int) -> c_int;
    fn js_setlength(JS, c_int, c_int);
    fn js_hasindex(JS, c_int, c_int) -> c_int;
    fn js_getindex(JS, c_int, c_int);
    fn js_setindex(JS, c_int, c_int);
    fn js_delindex(JS, c_int, c_int);
    fn js_getglobal(JS, *const c_char);
    fn js_setglobal(JS, *const c_char);
    fn js_defglobal(JS, *const c_char, c_int);
    fn js_delglobal(JS, *const c_char);
    fn js_getregistry(JS, *const c_char);
    fn js_setregistry(JS, *const c_char);
    fn js_delregistry(JS, *const c_char);
    fn js_ref(JS) -> *const c_char;
    fn js_unref(JS, *const c_char);
    fn js_pushiterator(JS, c_int, c_int);
    fn js_nextiterator(JS, c_int) -> *const c_char;
    fn js_concat(JS);
    fn js_equal(JS) -> c_int;
    fn js_strictequal(JS) -> c_int;
    fn js_compare(JS, *mut c_int) -> c_int;
    fn js_instanceof(JS) -> c_int;
    fn js_currentfunction(JS);
    fn js_currentfunctiondata(JS) -> *mut c_void;

    /* ---- utf ---- */
    fn jsU_chartorune(*mut Rune, *const c_char) -> c_int;
    fn jsU_runetochar(*mut c_char, *const Rune) -> c_int;
    fn jsU_runelen(c_int) -> c_int;
    fn jsU_isalpharune(Rune) -> c_int;
    fn jsU_islowerrune(Rune) -> c_int;
    fn jsU_isupperrune(Rune) -> c_int;
    fn jsU_tolowerrune(Rune) -> Rune;
    fn jsU_toupperrune(Rune) -> Rune;
    fn jsU_tolowerrune_full(Rune) -> *const Rune;
    fn jsU_toupperrune_full(Rune) -> *const Rune;
    fn js_utflen(*const c_char) -> c_int;
    fn js_utfptrtoidx(*const c_char, *const c_char) -> c_int;

    /* ---- dtoa / numeric ---- */
    fn js_itoa(*mut c_char, c_int) -> *const c_char;
    fn js_fmtexp(*mut c_char, c_int);
    fn js_grisu2(f64, *mut c_char, *mut c_int) -> c_int;
    fn js_strtod(*const c_char, *mut *mut c_char) -> f64;
    fn js_strtol(*const c_char, *mut *mut c_char, c_int) -> f64;
    fn js_stringtofloat(*const c_char, *mut *mut c_char) -> f64;

    /* ---- value ---- */
    fn jsV_numbertointeger(f64) -> c_int;
    fn jsV_numbertoint32(f64) -> c_int;
    fn jsV_numbertouint32(f64) -> c_uint;
    fn jsV_numbertoint16(f64) -> c_short;
    fn jsV_numbertouint16(f64) -> c_ushort;
    fn jsV_numbertostring(JS, *mut c_char, f64) -> *const c_char;
    fn jsV_stringtonumber(JS, *const c_char) -> f64;
    fn js_isarrayindex(JS, *const c_char, *mut c_int) -> c_int;
    fn js_intern(JS, *const c_char) -> *const c_char;
    fn js_runeat(JS, *const c_char, c_int) -> c_int;

    /* ---- lexer helpers ---- */
    fn jsY_iswhite(c_int) -> c_int;
    fn jsY_isnewline(c_int) -> c_int;
    fn jsY_ishex(c_int) -> c_int;
    fn jsY_tohex(c_int) -> c_int;
    fn jsY_tokenstring(c_int) -> *const c_char;
    fn jsY_findword(*const c_char, *const *const c_char, c_int) -> c_int;

    /* ---- regexp ---- */
    fn js_regcomp(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
    fn js_regcompx(RegAllocFn, *mut c_void, *const c_char, c_int, *mut *const c_char) -> *mut c_void;
    fn js_regexec(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int;
    fn js_regfree(*mut c_void);
    fn js_regfreex(RegAllocFn, *mut c_void, *mut c_void);

    /* ---- debug ---- */
    fn js_trap(JS, c_int);
    fn jsS_dumpstrings(JS);
}

/* ------------------------------------------------------------------ */
/* Loading                                                             */
/* ------------------------------------------------------------------ */

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libmujs.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libmujs.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libmujs.so")
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        // The C CMake build does not link libm, so `floor`, `pow`, ... are
        // unresolved in libmujs.so.  Publish libm globally first so the
        // subsequent dlopen can bind them (this mirrors how a normal C program
        // linking mujs would supply -lm).
        unsafe {
            use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
            for name in ["libm.so.6", "libm.so", "libc.so.6"] {
                if let Ok(l) = Library::open(Some(name), RTLD_NOW | RTLD_GLOBAL) {
                    std::mem::forget(l);
                }
            }
        }
        Pair {
            c: Api::load("C", &c_so_path()),
            r: Api::load("RUST", &rust_so_path()),
        }
    })
}

/* ------------------------------------------------------------------ */
/* Small helpers                                                       */
/* ------------------------------------------------------------------ */

pub fn cs(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap()
}

/// Raw bytes (may contain NUL) as a NUL-terminated buffer.
pub fn cbuf(b: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
    v.push(0);
    v
}

pub unsafe fn rstr(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

/// Bit-exact double comparison (NaN == NaN when both are NaN with same bits is
/// too strict; the C code produces canonical NaNs, so compare bit patterns but
/// treat any-NaN vs any-NaN as equal only if both are NaN and the payloads match
/// on the sign bit being irrelevant).
pub fn dbl_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

/// Deterministic xorshift PRNG so every test is reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" double: mixture of small ints, fractions and extremes.
    pub fn nice_f64(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => self.i32() as f64,
            6 => (self.i32() as f64) / 1000.0,
            7 => self.f64_bits(),
            8 => (self.next_u32() as f64) * 4294967296.0,
            _ => (self.next_u64() as f64) / (1u64 << 52) as f64,
        }
    }
}

/* ------------------------------------------------------------------ */
/* Report capture                                                      */
/* ------------------------------------------------------------------ */

use std::cell::RefCell;
thread_local! {
    pub static REPORTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub unsafe extern "C-unwind" fn report_cb(_j: JS, msg: *const c_char) {
    let s = unsafe { rstr(msg) };
    REPORTS.with(|r| r.borrow_mut().push(s));
}

pub fn take_reports() -> Vec<String> {
    REPORTS.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

/* ------------------------------------------------------------------ */
/* Script evaluation                                                   */
/* ------------------------------------------------------------------ */

pub const JS_STRICT: c_int = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EvalOut {
    pub rc: c_int,
    pub value: String,
    pub reports: Vec<String>,
    pub top: c_int,
}

/// Full end-to-end run: fresh state -> compile -> call -> stringify result.
/// `rc` distinguishes compile failure (1), runtime failure (2) and success (0).
pub unsafe fn eval(api: &Api, src: &str, flags: c_int) -> EvalOut {
    unsafe {
        let _ = take_reports();
        let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
        assert!(!j.is_null(), "{}: js_newstate failed", api.which);
        (api.js_setreport)(j, Some(report_cb));
        let fname = cs("[string]");
        let csrc = cs(src);
        let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
        if rc != 0 {
            rc = 1;
        } else {
            (api.js_pushundefined)(j);
            if (api.js_pcall)(j, 0) != 0 {
                rc = 2;
            }
        }
        let fallback = cs("<tostring threw>");
        let value = rstr((api.js_trystring)(j, -1, fallback.as_ptr()));
        let top = (api.js_gettop)(j);
        (api.js_freestate)(j);
        EvalOut {
            rc,
            value,
            reports: take_reports(),
            top,
        }
    }
}

/// Run `src` under both libraries and assert identical observable behaviour.
pub fn diff_eval(src: &str, flags: c_int) {
    let p = pair();
    let a = unsafe { eval(&p.c, src, flags) };
    let b = unsafe { eval(&p.r, src, flags) };
    assert_eq!(
        a, b,
        "\n--- divergence (flags={flags}) ---\nsource:\n{src}\nC   : {a:?}\nRUST: {b:?}\n"
    );
}

pub fn diff_eval_both_modes(src: &str) {
    diff_eval(src, 0);
    diff_eval(src, JS_STRICT);
}

/* ------------------------------------------------------------------ */
/* stdout / stderr capture (for js_gc(report), js_trap, ...)           */
/* ------------------------------------------------------------------ */

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Run `f` with fd 1 (and optionally fd 2) redirected into a temporary file and
/// return everything written.  Used to compare output that the C library prints
/// with `printf` directly.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, String) {
    const O_RDWR: c_int = 2;
    const O_CREAT: c_int = 64;
    const O_TRUNC: c_int = 512;
    let path = std::env::temp_dir().join(format!(
        "mujs-cap-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        let tmp = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp >= 0, "open temp capture file");
        dup2(tmp, 1);
        let r = f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(tmp);
        let out = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (r, out)
    }
}

/* ------------------------------------------------------------------ */
/* Value description helpers                                           */
/* ------------------------------------------------------------------ */

/// Full observable description of one stack slot, computed with the
/// *protected* accessors so a throwing getter cannot escape.
pub unsafe fn describe(api: &Api, j: JS, idx: c_int) -> String {
    unsafe {
        let errs = cs("<throw>");
        let mut s = String::new();
        s.push_str(&format!("type={} ", (api.js_type)(j, idx)));
        s.push_str(&format!("typeof={} ", rstr((api.js_typeof)(j, idx))));
        s.push_str(&format!(
            "str={:?} ",
            rstr((api.js_trystring)(j, idx, errs.as_ptr()))
        ));
        s.push_str(&format!("num={:?} ", (api.js_trynumber)(j, idx, -12345.0)));
        s.push_str(&format!("int={} ", (api.js_tryinteger)(j, idx, -1)));
        s.push_str(&format!("bool={} ", (api.js_tryboolean)(j, idx, -1)));
        s.push_str(&format!(
            "repr={:?} ",
            rstr((api.js_tryrepr)(j, idx, errs.as_ptr()))
        ));
        for (name, f) in [
            ("defined", api.js_isdefined),
            ("undefined", api.js_isundefined),
            ("null", api.js_isnull),
            ("boolean", api.js_isboolean),
            ("number", api.js_isnumber),
            ("string", api.js_isstring),
            ("primitive", api.js_isprimitive),
            ("object", api.js_isobject),
            ("array", api.js_isarray),
            ("regexp", api.js_isregexp),
            ("coercible", api.js_iscoercible),
            ("callable", api.js_iscallable),
            ("error", api.js_iserror),
            ("numobj", api.js_isnumberobject),
            ("strobj", api.js_isstringobject),
            ("boolobj", api.js_isbooleanobject),
            ("dateobj", api.js_isdateobject),
        ] {
            s.push_str(&format!("{name}={} ", f(j, idx)));
        }
        let tag = cs("tag");
        s.push_str(&format!(
            "userdata={} ",
            (api.js_isuserdata)(j, idx, tag.as_ptr())
        ));
        s
    }
}

pub unsafe fn snapshot(api: &Api, j: JS) -> Vec<String> {
    unsafe {
        let top = (api.js_gettop)(j);
        let mut v = vec![format!("top={top}")];
        for i in 0..top {
            v.push(describe(api, j, i));
        }
        v
    }
}

/* ------------------------------------------------------------------ */
/* Protected host callbacks                                            */
/*                                                                     */
/* Many low-level entry points throw JS exceptions (js_pop underflow,   */
/* js_remove/js_replace "stack error!", js_insert "not implemented      */
/* yet", ...).  A Rust test frame cannot use setjmp, so the work is     */
/* done inside a host C function invoked through js_pcall: the C        */
/* longjmp / the Rust unwind is then caught by the library itself.      */
/* ------------------------------------------------------------------ */

pub type Job = Box<dyn FnMut(&Api, JS)>;

thread_local! {
    static JOB: RefCell<Option<Job>> = const { RefCell::new(None) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn log(s: impl Into<String>) {
    LOG.with(|l| l.borrow_mut().push(s.into()));
}

unsafe extern "C-unwind" fn job_trampoline_c(j: JS) {
    unsafe { run_job(&pair().c, j) }
}
unsafe extern "C-unwind" fn job_trampoline_r(j: JS) {
    unsafe { run_job(&pair().r, j) }
}

unsafe fn run_job(api: &Api, j: JS) {
    let mut f = JOB.with(|p| p.borrow_mut().take());
    if let Some(f) = f.as_mut() {
        f(api, j);
    }
    JOB.with(|p| *p.borrow_mut() = f);
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProtectedOut {
    pub rc: c_int,
    pub result: String,
    pub log: Vec<String>,
    pub top: c_int,
    pub reports: Vec<String>,
}

/// Run `job` inside a host C function under `js_pcall` on a fresh state.
pub fn run_protected(
    api: &Api,
    which: usize,
    flags: c_int,
    job: impl FnMut(&Api, JS) + 'static,
) -> ProtectedOut {
    unsafe {
        JOB.with(|p| *p.borrow_mut() = Some(Box::new(job)));
        LOG.with(|l| l.borrow_mut().clear());
        let _ = take_reports();
        let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
        assert!(!j.is_null());
        (api.js_setreport)(j, Some(report_cb));
        let tramp = if which == 0 {
            job_trampoline_c
        } else {
            job_trampoline_r
        };
        (api.js_newcfunction)(j, Some(tramp), c"__job".as_ptr(), 0);
        (api.js_pushundefined)(j);
        let rc = (api.js_pcall)(j, 0);
        let fb = cs("<throw>");
        let result = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
        let top = (api.js_gettop)(j);
        (api.js_freestate)(j);
        JOB.with(|p| *p.borrow_mut() = None);
        ProtectedOut {
            rc,
            result,
            log: LOG.with(|l| l.borrow().clone()),
            top,
            reports: take_reports(),
        }
    }
}

/// Run the same job under both libraries and require identical outcomes.
pub fn diff_protected<F>(label: &str, flags: c_int, make: impl Fn() -> F)
where
    F: FnMut(&Api, JS) + 'static,
{
    let p = pair();
    let a = run_protected(&p.c, 0, flags, make());
    let b = run_protected(&p.r, 1, flags, make());
    assert_eq!(a, b, "{label}");
}

/// Run many scripts against ONE state (much faster than a fresh state per
/// script, and additionally exercises state reuse).  Results are per-script and
/// deterministic in both libraries, so leaked globals are not a problem.
pub unsafe fn eval_batch(api: &Api, srcs: &[String], flags: c_int) -> Vec<EvalOut> {
    unsafe {
        let _ = take_reports();
        let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
        assert!(!j.is_null(), "{}: js_newstate failed", api.which);
        (api.js_setreport)(j, Some(report_cb));
        let fname = cs("[string]");
        let fallback = cs("<tostring threw>");
        let mut out = Vec::with_capacity(srcs.len());
        for src in srcs {
            let csrc = cs(src);
            let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
            if rc != 0 {
                rc = 1;
            } else {
                (api.js_pushundefined)(j);
                if (api.js_pcall)(j, 0) != 0 {
                    rc = 2;
                }
            }
            let value = rstr((api.js_trystring)(j, -1, fallback.as_ptr()));
            let top = (api.js_gettop)(j);
            (api.js_pop)(j, 1);
            out.push(EvalOut {
                rc,
                value,
                reports: take_reports(),
                top,
            });
        }
        (api.js_freestate)(j);
        out
    }
}

/// Batched differential evaluation.  Reports the first divergence with the
/// script that produced it.
pub fn diff_eval_batch(label: &str, srcs: &[String], flags: c_int) {
    let p = pair();
    let a = unsafe { eval_batch(&p.c, srcs, flags) };
    let b = unsafe { eval_batch(&p.r, srcs, flags) };
    assert_eq!(a.len(), b.len());
    let mut diffs = 0;
    let mut first = String::new();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            diffs += 1;
            if diffs == 1 {
                first = format!(
                    "script #{i} {:?}\n  C   : {x:?}\n  RUST: {y:?}",
                    srcs[i]
                );
            }
        }
    }
    assert!(
        diffs == 0,
        "{label} (flags={flags}): {diffs}/{} scripts diverged; first:\n{first}",
        srcs.len()
    );
}
