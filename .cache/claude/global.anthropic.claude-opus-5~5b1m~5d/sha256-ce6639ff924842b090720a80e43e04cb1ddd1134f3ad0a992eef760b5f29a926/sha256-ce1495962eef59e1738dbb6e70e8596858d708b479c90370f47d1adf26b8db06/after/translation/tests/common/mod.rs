//! Differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls them only through
//! their exported C symbols, exactly as an external consumer would:
//!
//!   * C:    `c_src/build/libmujs.so`
//!   * Rust: `translation/target/<profile>/libmujs.so`
//!
//! Nothing in this file calls a Rust function of the crate directly.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_short, c_uint, c_ushort, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::Mutex;

pub type JS = *mut c_void;
pub type Obj = *mut c_void;
pub type Prop = *mut c_void;
pub type Prog = *mut c_void;
pub type CFun = unsafe extern "C" fn(JS);
pub type Alloc = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
pub type Finalize = unsafe extern "C" fn(JS, *mut c_void);
pub type HasProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type PutProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type DelProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type Report = unsafe extern "C" fn(JS, *const c_char);
pub type Panic = unsafe extern "C" fn(JS);

/* mujs.h constants */
pub const JS_STRICT: c_int = 1;
pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;
pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;
pub const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResubOne {
    pub sp: *const c_char,
    pub ep: *const c_char,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubOne; REG_MAXSUB],
}
impl Default for Resub {
    fn default() -> Self {
        Resub {
            nsub: 0,
            sub: [ResubOne {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

macro_rules! api {
    ( $( $name:ident : $t:ty ),* $(,)? ) => {
        pub struct Api {
            pub tag: &'static str,
            $( pub $name : $t, )*
            _lib: Library,
        }
        impl Api {
            pub fn load(tag: &'static str, path: &std::path::Path) -> Api {
                let lib = unsafe { Library::new(path) }
                    .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
                let a = Api {
                    tag,
                    $( $name : unsafe {
                        let s: Symbol<$t> = lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", path.display(), stringify!($name), e));
                        *s
                    }, )*
                    _lib: lib,
                };
                a
            }
        }
    };
}

api! {
    /* ---- state ---- */
    js_newstate: unsafe extern "C" fn(Option<Alloc>, *mut c_void, c_int) -> JS,
    js_freestate: unsafe extern "C" fn(JS),
    js_setcontext: unsafe extern "C" fn(JS, *mut c_void),
    js_getcontext: unsafe extern "C" fn(JS) -> *mut c_void,
    js_setreport: unsafe extern "C" fn(JS, Option<Report>),
    js_atpanic: unsafe extern "C" fn(JS, Option<Panic>) -> Option<Panic>,
    js_gc: unsafe extern "C" fn(JS, c_int),
    js_setlimit: unsafe extern "C" fn(JS, c_int, c_int),
    js_report: unsafe extern "C" fn(JS, *const c_char),
    js_trap: unsafe extern "C" fn(JS, c_int),

    /* ---- eval ---- */
    js_dostring: unsafe extern "C" fn(JS, *const c_char) -> c_int,
    js_ploadstring: unsafe extern "C" fn(JS, *const c_char, *const c_char) -> c_int,
    js_loadstring: unsafe extern "C" fn(JS, *const c_char, *const c_char),
    js_loadeval: unsafe extern "C" fn(JS, *const c_char, *const c_char),
    js_pcall: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_pconstruct: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_call: unsafe extern "C" fn(JS, c_int),
    js_construct: unsafe extern "C" fn(JS, c_int),
    js_eval: unsafe extern "C" fn(JS),

    /* ---- errors ---- */
    js_newerror: unsafe extern "C" fn(JS, *const c_char),
    js_newevalerror: unsafe extern "C" fn(JS, *const c_char),
    js_newrangeerror: unsafe extern "C" fn(JS, *const c_char),
    js_newreferenceerror: unsafe extern "C" fn(JS, *const c_char),
    js_newsyntaxerror: unsafe extern "C" fn(JS, *const c_char),
    js_newtypeerror: unsafe extern "C" fn(JS, *const c_char),
    js_newurierror: unsafe extern "C" fn(JS, *const c_char),
    js_throw: unsafe extern "C" fn(JS),
    js_savetry: unsafe extern "C" fn(JS) -> *mut c_void,
    js_savetrypc: unsafe extern "C" fn(JS, *mut c_void) -> *mut c_void,
    js_endtry: unsafe extern "C" fn(JS),

    /* ---- registry / refs / globals ---- */
    js_ref: unsafe extern "C" fn(JS) -> *const c_char,
    js_unref: unsafe extern "C" fn(JS, *const c_char),
    js_getregistry: unsafe extern "C" fn(JS, *const c_char),
    js_setregistry: unsafe extern "C" fn(JS, *const c_char),
    js_delregistry: unsafe extern "C" fn(JS, *const c_char),
    js_getglobal: unsafe extern "C" fn(JS, *const c_char),
    js_setglobal: unsafe extern "C" fn(JS, *const c_char),
    js_defglobal: unsafe extern "C" fn(JS, *const c_char, c_int),
    js_delglobal: unsafe extern "C" fn(JS, *const c_char),

    /* ---- properties ---- */
    js_hasproperty: unsafe extern "C" fn(JS, c_int, *const c_char) -> c_int,
    js_getproperty: unsafe extern "C" fn(JS, c_int, *const c_char),
    js_setproperty: unsafe extern "C" fn(JS, c_int, *const c_char),
    js_defproperty: unsafe extern "C" fn(JS, c_int, *const c_char, c_int),
    js_delproperty: unsafe extern "C" fn(JS, c_int, *const c_char),
    js_defaccessor: unsafe extern "C" fn(JS, c_int, *const c_char, c_int),
    js_getlength: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_setlength: unsafe extern "C" fn(JS, c_int, c_int),
    js_hasindex: unsafe extern "C" fn(JS, c_int, c_int) -> c_int,
    js_getindex: unsafe extern "C" fn(JS, c_int, c_int),
    js_setindex: unsafe extern "C" fn(JS, c_int, c_int),
    js_delindex: unsafe extern "C" fn(JS, c_int, c_int),

    /* ---- push / new ---- */
    js_currentfunction: unsafe extern "C" fn(JS),
    js_currentfunctiondata: unsafe extern "C" fn(JS) -> *mut c_void,
    js_pushglobal: unsafe extern "C" fn(JS),
    js_pushundefined: unsafe extern "C" fn(JS),
    js_pushnull: unsafe extern "C" fn(JS),
    js_pushboolean: unsafe extern "C" fn(JS, c_int),
    js_pushnumber: unsafe extern "C" fn(JS, f64),
    js_pushstring: unsafe extern "C" fn(JS, *const c_char),
    js_pushlstring: unsafe extern "C" fn(JS, *const c_char, c_int),
    js_pushliteral: unsafe extern "C" fn(JS, *const c_char),
    js_pushobject: unsafe extern "C" fn(JS, Obj),
    js_newobjectx: unsafe extern "C" fn(JS),
    js_newobject: unsafe extern "C" fn(JS),
    js_newarray: unsafe extern "C" fn(JS),
    js_newboolean: unsafe extern "C" fn(JS, c_int),
    js_newnumber: unsafe extern "C" fn(JS, f64),
    js_newstring: unsafe extern "C" fn(JS, *const c_char),
    js_newcfunction: unsafe extern "C" fn(JS, Option<CFun>, *const c_char, c_int),
    js_newcfunctionx: unsafe extern "C" fn(JS, Option<CFun>, *const c_char, c_int, *mut c_void, Option<Finalize>),
    js_newcconstructor: unsafe extern "C" fn(JS, Option<CFun>, Option<CFun>, *const c_char, c_int),
    js_newuserdata: unsafe extern "C" fn(JS, *const c_char, *mut c_void, Option<Finalize>),
    js_newuserdatax: unsafe extern "C" fn(JS, *const c_char, *mut c_void, Option<HasProp>, Option<PutProp>, Option<DelProp>, Option<Finalize>),
    js_newregexp: unsafe extern "C" fn(JS, *const c_char, c_int),
    js_newarguments: unsafe extern "C" fn(JS),

    /* ---- iterators ---- */
    js_pushiterator: unsafe extern "C" fn(JS, c_int, c_int),
    js_nextiterator: unsafe extern "C" fn(JS, c_int) -> *const c_char,

    /* ---- predicates ---- */
    js_isdefined: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isundefined: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isnull: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isboolean: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isnumber: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isstring: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isprimitive: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isobject: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isarray: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isregexp: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_iscoercible: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_iscallable: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isuserdata: unsafe extern "C" fn(JS, c_int, *const c_char) -> c_int,
    js_iserror: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isnumberobject: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isstringobject: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isbooleanobject: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isdateobject: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_isarrayindex: unsafe extern "C" fn(JS, *const c_char, *mut c_int) -> c_int,

    /* ---- conversions ---- */
    js_toboolean: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_tonumber: unsafe extern "C" fn(JS, c_int) -> f64,
    js_tostring: unsafe extern "C" fn(JS, c_int) -> *const c_char,
    js_touserdata: unsafe extern "C" fn(JS, c_int, *const c_char) -> *mut c_void,
    js_trystring: unsafe extern "C" fn(JS, c_int, *const c_char) -> *const c_char,
    js_trynumber: unsafe extern "C" fn(JS, c_int, f64) -> f64,
    js_tryinteger: unsafe extern "C" fn(JS, c_int, c_int) -> c_int,
    js_tryboolean: unsafe extern "C" fn(JS, c_int, c_int) -> c_int,
    js_tointeger: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_toint32: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_touint32: unsafe extern "C" fn(JS, c_int) -> c_uint,
    js_toint16: unsafe extern "C" fn(JS, c_int) -> c_short,
    js_touint16: unsafe extern "C" fn(JS, c_int) -> c_ushort,
    js_toobject: unsafe extern "C" fn(JS, c_int) -> Obj,
    js_toprimitive: unsafe extern "C" fn(JS, c_int, c_int),
    js_tovalue: unsafe extern "C" fn(JS, c_int) -> *mut c_void,
    js_toregexp: unsafe extern "C" fn(JS, c_int) -> *mut c_void,

    /* ---- stack ---- */
    js_gettop: unsafe extern "C" fn(JS) -> c_int,
    js_pop: unsafe extern "C" fn(JS, c_int),
    js_rot: unsafe extern "C" fn(JS, c_int),
    js_copy: unsafe extern "C" fn(JS, c_int),
    js_remove: unsafe extern "C" fn(JS, c_int),
    js_insert: unsafe extern "C" fn(JS, c_int),
    js_replace: unsafe extern "C" fn(JS, c_int),
    js_dup: unsafe extern "C" fn(JS),
    js_dup2: unsafe extern "C" fn(JS),
    js_rot2: unsafe extern "C" fn(JS),
    js_rot3: unsafe extern "C" fn(JS),
    js_rot4: unsafe extern "C" fn(JS),
    js_rot2pop1: unsafe extern "C" fn(JS),
    js_rot3pop2: unsafe extern "C" fn(JS),

    /* ---- operators ---- */
    js_concat: unsafe extern "C" fn(JS),
    js_compare: unsafe extern "C" fn(JS, *mut c_int) -> c_int,
    js_equal: unsafe extern "C" fn(JS) -> c_int,
    js_strictequal: unsafe extern "C" fn(JS) -> c_int,
    js_instanceof: unsafe extern "C" fn(JS) -> c_int,
    js_typeof: unsafe extern "C" fn(JS, c_int) -> *const c_char,
    js_type: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_repr: unsafe extern "C" fn(JS, c_int),
    js_torepr: unsafe extern "C" fn(JS, c_int) -> *const c_char,
    js_tryrepr: unsafe extern "C" fn(JS, c_int, *const c_char) -> *const c_char,

    /* ---- low level: numbers / strings ---- */
    js_itoa: unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char,
    js_strtod: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
    js_strtol: unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> f64,
    js_stringtofloat: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
    js_grisu2: unsafe extern "C" fn(f64, *mut c_char, *mut c_int) -> c_int,
    js_fmtexp: unsafe extern "C" fn(*mut c_char, c_int),
    js_utflen: unsafe extern "C" fn(*const c_char) -> c_int,
    js_utfptrtoidx: unsafe extern "C" fn(*const c_char, *const c_char) -> c_int,
    js_runeat: unsafe extern "C" fn(JS, *const c_char, c_int) -> c_int,
    js_intern: unsafe extern "C" fn(JS, *const c_char) -> *const c_char,
    js_strdup: unsafe extern "C" fn(JS, *const c_char) -> *mut c_char,
    js_malloc: unsafe extern "C" fn(JS, c_int) -> *mut c_void,
    js_realloc: unsafe extern "C" fn(JS, *mut c_void, c_int) -> *mut c_void,
    js_free: unsafe extern "C" fn(JS, *mut c_void),

    jsV_numbertostring: unsafe extern "C" fn(JS, *mut c_char, f64) -> *const c_char,
    jsV_stringtonumber: unsafe extern "C" fn(JS, *const c_char) -> f64,
    jsV_numbertointeger: unsafe extern "C" fn(f64) -> c_int,
    jsV_numbertoint32: unsafe extern "C" fn(f64) -> c_int,
    jsV_numbertouint32: unsafe extern "C" fn(f64) -> c_uint,
    jsV_numbertoint16: unsafe extern "C" fn(f64) -> c_short,
    jsV_numbertouint16: unsafe extern "C" fn(f64) -> c_ushort,
    jsV_newobject: unsafe extern "C" fn(JS, c_int, Obj) -> Obj,
    jsV_getownproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_getproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_getpropertyx: unsafe extern "C" fn(JS, Obj, *const c_char, *mut c_int) -> Prop,
    jsV_setproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_delproperty: unsafe extern "C" fn(JS, Obj, *const c_char),
    jsV_newiterator: unsafe extern "C" fn(JS, Obj, c_int) -> Obj,
    jsV_nextiterator: unsafe extern "C" fn(JS, Obj) -> *const c_char,
    jsV_resizearray: unsafe extern "C" fn(JS, Obj, c_int),
    jsV_newmemstring: unsafe extern "C" fn(JS, *const c_char, c_int) -> *mut c_void,
    jsV_toboolean: unsafe extern "C" fn(JS, *mut c_void) -> c_int,
    jsV_tonumber: unsafe extern "C" fn(JS, *mut c_void) -> f64,
    jsV_tointeger: unsafe extern "C" fn(JS, *mut c_void) -> f64,
    jsV_tostring: unsafe extern "C" fn(JS, *mut c_void) -> *const c_char,
    jsV_toobject: unsafe extern "C" fn(JS, *mut c_void) -> Obj,
    jsV_toprimitive: unsafe extern "C" fn(JS, *mut c_void, c_int),

    /* ---- low level: utf ---- */
    jsU_chartorune: unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int,
    jsU_runetochar: unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int,
    jsU_runelen: unsafe extern "C" fn(c_int) -> c_int,
    jsU_isalpharune: unsafe extern "C" fn(c_int) -> c_int,
    jsU_islowerrune: unsafe extern "C" fn(c_int) -> c_int,
    jsU_isupperrune: unsafe extern "C" fn(c_int) -> c_int,
    jsU_tolowerrune: unsafe extern "C" fn(c_int) -> c_int,
    jsU_toupperrune: unsafe extern "C" fn(c_int) -> c_int,
    jsU_tolowerrune_full: unsafe extern "C" fn(c_int) -> *const c_int,
    jsU_toupperrune_full: unsafe extern "C" fn(c_int) -> *const c_int,

    /* ---- low level: regexp ---- */
    js_regcomp: unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> Prog,
    js_regexec: unsafe extern "C" fn(Prog, *const c_char, *mut Resub, c_int) -> c_int,
    js_regfree: unsafe extern "C" fn(Prog),
    js_regcompx: unsafe extern "C" fn(Option<Alloc>, *mut c_void, *const c_char, c_int, *mut *const c_char) -> Prog,
    js_regfreex: unsafe extern "C" fn(Option<Alloc>, *mut c_void, Prog),

    /* ---- low level: lexer ---- */
    jsY_iswhite: unsafe extern "C" fn(c_int) -> c_int,
    jsY_isnewline: unsafe extern "C" fn(c_int) -> c_int,
    jsY_ishex: unsafe extern "C" fn(c_int) -> c_int,
    jsY_tohex: unsafe extern "C" fn(c_int) -> c_int,
    jsY_tokenstring: unsafe extern "C" fn(c_int) -> *const c_char,
    jsY_findword: unsafe extern "C" fn(*const c_char, *const *const c_char, c_int) -> c_int,

    /* ---- interning / gc ---- */
    jsS_dumpstrings: unsafe extern "C" fn(JS),
}

fn env_or(name: &str, default: PathBuf) -> PathBuf {
    match std::env::var_os(name) {
        Some(v) => PathBuf::from(v),
        None => default,
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so() -> PathBuf {
    env_or(
        "MUJS_C_SO",
        root().parent().unwrap().join("c_src/build/libmujs.so"),
    )
}

pub fn rust_so() -> PathBuf {
    if let Some(v) = std::env::var_os("MUJS_RUST_SO") {
        return PathBuf::from(v);
    }
    /* the test binary lives in target/<profile>/deps/, the cdylib in target/<profile>/ */
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().unwrap().parent().unwrap();
    let p = dir.join("libmujs.so");
    if p.exists() {
        return p;
    }
    root().join("target/release/libmujs.so")
}

/* Both libraries are loaded exactly once per test process. */
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        /* c_src/CMakeLists.txt does not link libm, so libmujs.so has undefined
         * math symbols (ceil, floor, fmod, ...). Put libm in the global scope
         * first so those lazy bindings resolve. Both mujs libraries themselves
         * stay RTLD_LOCAL, so they can never interpose on each other. */
        use libloading::os::unix as ul;
        let m = unsafe { ul::Library::open(Some("libm.so.6"), ul::RTLD_NOW | ul::RTLD_GLOBAL) }
            .expect("dlopen libm.so.6");
        std::mem::forget(m);
        Pair {
            c: Api::load("C", &c_so()),
            r: Api::load("RUST", &rust_so()),
        }
    })
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub unsafe fn rs(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    String::from_utf8_lossy(CStr::from_ptr(p).to_bytes()).into_owned()
}

/* -------------------------------------------------------------------------- */
/* native action trampoline: lets a test drive the raw C API inside js_pcall,  */
/* so that a thrown exception is caught instead of aborting the process.       */
/* -------------------------------------------------------------------------- */

thread_local! {
    static CUR: Cell<*const Api> = const { Cell::new(std::ptr::null()) };
    static ACT: Cell<Option<fn(&Api, JS)>> = const { Cell::new(None) };
    static OUT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Record an observation from inside a native action. The borrow is never held
/// across a call into the library, so a longjmp can not leave it locked.
pub fn emit(s: &str) {
    OUT.with(|o| {
        let mut b = o.borrow_mut();
        b.push_str(s);
        b.push('|');
    });
}

pub fn emit_num(x: f64) {
    emit(&format!("{:#x}", x.to_bits()));
}

pub unsafe fn emit_cstr(p: *const c_char) {
    emit(&format!("{:?}", rs(p)));
}

fn out_take() -> String {
    OUT.with(|o| std::mem::take(&mut *o.borrow_mut()))
}

/* Parameters for native actions (which must be plain `fn`s, no captures). */
thread_local! {
    static PI: Cell<[i64; 4]> = const { Cell::new([0; 4]) };
    static PF: Cell<[f64; 2]> = const { Cell::new([0.0; 2]) };
    static PS: std::cell::RefCell<[String; 2]> =
        const { std::cell::RefCell::new([String::new(), String::new()]) };
}

pub fn set_pi(i: usize, v: i64) {
    PI.with(|p| {
        let mut a = p.get();
        a[i] = v;
        p.set(a);
    })
}
pub fn pi(i: usize) -> i64 {
    PI.with(|p| p.get()[i])
}
pub fn pic(i: usize) -> c_int {
    pi(i) as c_int
}
pub fn set_pf(i: usize, v: f64) {
    PF.with(|p| {
        let mut a = p.get();
        a[i] = v;
        p.set(a);
    })
}
pub fn pf(i: usize) -> f64 {
    PF.with(|p| p.get()[i])
}
pub fn set_ps(i: usize, v: &str) {
    PS.with(|p| p.borrow_mut()[i] = v.to_string())
}
pub fn ps(i: usize) -> CString {
    cs(&PS.with(|p| p.borrow()[i].clone()))
}

pub fn cur() -> &'static Api {
    let p = CUR.with(|c| c.get());
    assert!(!p.is_null(), "no current Api");
    unsafe { &*p }
}

unsafe extern "C" fn trampoline(J: JS) {
    let f = ACT.with(|a| a.get()).expect("no action");
    f(cur(), J);
}

impl Api {
    /* Result of an evaluation, rendered identically for both libraries. */
    pub fn newstate(&self, flags: c_int) -> JS {
        let J = unsafe { (self.js_newstate)(None, std::ptr::null_mut(), flags) };
        assert!(!J.is_null(), "{}: js_newstate failed", self.tag);
        J
    }

    /// Evaluate `src` and render `rc` + the resulting value (or error) as a string.
    pub fn eval(&self, src: &str, flags: c_int) -> String {
        let J = self.newstate(flags);
        let s = self.eval_in(J, src);
        unsafe { (self.js_freestate)(J) };
        s
    }

    pub fn eval_in(&self, J: JS, src: &str) -> String {
        unsafe {
            let name = cs("test.js");
            let source = cs(src);
            let rc = (self.js_ploadstring)(J, name.as_ptr(), source.as_ptr());
            if rc != 0 {
                let e = cs("<tostring failed>");
                let msg = rs((self.js_trystring)(J, -1, e.as_ptr()));
                (self.js_pop)(J, 1);
                return format!("load-error({}) {}", rc, msg);
            }
            (self.js_pushundefined)(J);
            let rc = (self.js_pcall)(J, 0);
            let e = cs("<repr failed>");
            let out = if rc != 0 {
                let msg = rs((self.js_trystring)(J, -1, e.as_ptr()));
                format!("throw({}) {}", rc, msg)
            } else {
                format!("ok {}", rs((self.js_tryrepr)(J, -1, e.as_ptr())))
            };
            (self.js_pop)(J, 1);
            out
        }
    }

    /// Run a raw-C-API sequence inside `js_pcall` so throws are caught.
    /// Returns `(rc, rendered top-of-stack)`.
    pub fn run_native(&self, act: fn(&Api, JS), flags: c_int) -> String {
        let J = self.newstate(flags);
        let s = self.run_native_in(J, act);
        unsafe { (self.js_freestate)(J) };
        s
    }

    pub fn run_native_in(&self, J: JS, act: fn(&Api, JS)) -> String {
        CUR.with(|c| c.set(self as *const Api));
        ACT.with(|a| a.set(Some(act)));
        let _ = out_take();
        unsafe {
            let n = cs("native");
            (self.js_newcfunction)(J, Some(trampoline), n.as_ptr(), 0);
            (self.js_pushundefined)(J);
            let rc = (self.js_pcall)(J, 0);
            let e = cs("<repr failed>");
            let out = if rc != 0 {
                format!("throw({}) {}", rc, rs((self.js_trystring)(J, -1, e.as_ptr())))
            } else {
                format!("ok {}", rs((self.js_tryrepr)(J, -1, e.as_ptr())))
            };
            (self.js_pop)(J, 1);
            format!("emit[{}] top={} top-idx={}", out_take(), out, (self.js_gettop)(J))
        }
    }
}

/* -------------------------------------------------------------------------- */
/* isolated (sub-process) execution, for paths where the C library aborts,     */
/* panics or crashes: js_throw without a try, stack overflow past the panic    */
/* handler, out-of-range stack indices, ...                                    */
/* -------------------------------------------------------------------------- */

pub fn set_cur(a: &Api) {
    CUR.with(|c| c.set(a as *const Api));
}

/// Called at the top of the `isolated_child` test of a test binary. Returns
/// `true` when it ran as a child (the process then exits).
pub fn isolated_child_main(cases: &[(&str, fn(&Api, JS))]) -> bool {
    let case = match std::env::var("MUJS_CASE") {
        Ok(c) => c,
        Err(_) => return false,
    };
    let which = std::env::var("MUJS_LIB").unwrap_or_else(|_| "C".into());
    let flags: c_int = std::env::var("MUJS_FLAGS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let p = libs();
    let a = if which == "C" { &p.c } else { &p.r };
    let f = cases
        .iter()
        .find(|(n, _)| *n == case)
        .unwrap_or_else(|| panic!("unknown isolated case {:?}", case))
        .1;
    set_cur(a);
    /* cases named "nostate_*" create (or fail to create) the state themselves */
    let J = if case.starts_with("nostate_") {
        std::ptr::null_mut()
    } else {
        a.newstate(flags)
    };
    f(a, J);
    /* if we get here the operation did not abort */
    if !J.is_null() {
        println!("CASE-COMPLETED top={}", unsafe { (a.js_gettop)(J) });
        unsafe { (a.js_freestate)(J) };
    } else {
        println!("CASE-COMPLETED");
    }
    println!("STATE-FREED");
    unsafe {
        extern "C" {
            fn fflush(f: *mut c_void) -> c_int;
        }
        fflush(std::ptr::null_mut());
    }
    std::process::exit(0);
}

fn run_child(case: &str, which: &str, flags: c_int) -> String {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("isolated_child")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("MUJS_CASE", case)
        .env("MUJS_LIB", which)
        .env("MUJS_FLAGS", flags.to_string())
        .output()
        .expect("spawn child");
    let mut s = String::new();
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        s.push_str(&format!(
            "code={:?} signal={:?}\n",
            out.status.code(),
            out.status.signal()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    /* keep only the lines the library itself produced */
    for l in stdout.lines().chain(stderr.lines()) {
        /* libtest prints "test isolated_child ... " without a newline, so the
         * first line of real output is glued onto it: strip the prefix and keep
         * the payload instead of dropping the line. */
        let l = match l.find("isolated_child ... ") {
            Some(i) => &l[i + "isolated_child ... ".len()..],
            None => l,
        };
        if l.starts_with("running ")
            || l.starts_with("test ")
            || l.starts_with("---- ")
            || l.is_empty()
            || l.contains("test result")
            || l.contains("Finished")
            || l.contains("Running")
            || l.contains("note: ")
        {
            continue;
        }
        s.push_str(l);
        s.push('\n');
    }
    mask_ptrs(&s)
}

/// Run one isolated case in both libraries and assert identical
/// exit status / signal / output.
#[track_caller]
pub fn diff_isolated(case: &str, flags: c_int) {
    let c = run_child(case, "C", flags);
    let r = run_child(case, "RUST", flags);
    same(&format!("isolated {} flags={}", case, flags), &c, &r);
}

/* -------------------------------------------------------------------------- */
/* stdout capture (js_gc report, jsS_dumpstrings, js_trap all use libc stdio)  */
/* -------------------------------------------------------------------------- */

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

pub static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Capture everything the callee writes to fd 1.
pub fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let _g = CAPTURE_LOCK.lock().unwrap();
    let mut path = std::env::temp_dir();
    path.push(format!("mujs_cap_{}_{:?}.txt", std::process::id(), std::thread::current().id()));
    let cpath = cs(path.to_str().unwrap());
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        /* O_WRONLY|O_CREAT|O_TRUNC = 1|64|512 on linux */
        let fd = open(cpath.as_ptr(), 1 | 64 | 512, 0o644 as c_int);
        assert!(fd >= 0, "cannot open capture file");
        dup2(fd, 1);
        close(fd);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&data).into_owned()
}

/* -------------------------------------------------------------------------- */
/* deterministic PRNG (fixed seed, reproducible property-style testing)        */
/* -------------------------------------------------------------------------- */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        /* splitmix64 */
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    /// A double drawn from a distribution that covers the interesting classes.
    pub fn f64(&mut self) -> f64 {
        match self.below(12) {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => self.range_i64(-1000, 1000) as f64,
            6 => self.range_i64(i32::MIN as i64, i32::MAX as i64) as f64,
            7 => f64::from_bits(self.next_u64()),
            8 => self.range_i64(-1 << 53, 1 << 53) as f64,
            9 => (self.next_u64() as f64) / (self.next_u64() as f64 + 1.0),
            10 => f64::MIN_POSITIVE * (self.below(1000) as f64),
            _ => {
                let m = self.next_u64() as f64 / (u64::MAX as f64);
                let e = self.range_i64(-300, 300) as f64;
                m * 10f64.powf(e)
            }
        }
    }
    /// A random string with a mix of ASCII, multibyte UTF-8 and escapes.
    pub fn string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen as u64 + 1) as usize;
        let mut s = String::new();
        for _ in 0..n {
            let c = match self.below(10) {
                0..=4 => (b'a' + (self.below(26) as u8)) as char,
                5 => (b'0' + (self.below(10) as u8)) as char,
                6 => [' ', '\t', '\n', '"', '\'', '\\', '/', '<', '>', '&'][self.below(10) as usize],
                7 => char::from_u32(0x80 + self.u32() % 0x700).unwrap_or('\u{80}'),
                8 => char::from_u32(0x800 + self.u32() % 0xF000).unwrap_or('\u{800}'),
                _ => char::from_u32(0x10000 + self.u32() % 0xFFFFF).unwrap_or('\u{10000}'),
            };
            s.push(c);
        }
        s
    }
}

/* -------------------------------------------------------------------------- */

/* `js_torepr`/`js_tostring` REPLACE the value at `idx` with the converted
 * string (see jsrepr.c:268). These helpers observe a value without destroying
 * it, by converting a copy. */
pub unsafe fn repr_at(a: &Api, J: JS, idx: c_int) -> String {
    (a.js_copy)(J, idx);
    let e = cs("<REPR-ERR>");
    let s = rs((a.js_tryrepr)(J, -1, e.as_ptr()));
    (a.js_pop)(J, 1);
    s
}

pub unsafe fn str_at(a: &Api, J: JS, idx: c_int) -> String {
    (a.js_copy)(J, idx);
    let e = cs("<STR-ERR>");
    let s = rs((a.js_trystring)(J, -1, e.as_ptr()));
    (a.js_pop)(J, 1);
    s
}

/// Replace `0x...` hex addresses (js_ref names, js_trap dumps) with a marker:
/// heap addresses legitimately differ between the two libraries.
pub fn mask_ptrs(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '0' && it.peek() == Some(&'x') {
            it.next();
            let mut n = 0;
            while it.peek().map_or(false, |c| c.is_ascii_hexdigit()) {
                it.next();
                n += 1;
            }
            if n > 0 {
                out.push_str("0xPTR");
            } else {
                out.push_str("0x");
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Assert both libraries agree, with a readable diff.
#[track_caller]
pub fn same(label: &str, c: &str, r: &str) {
    if c != r {
        panic!(
            "DIVERGENCE [{}]\n  C   : {:?}\n  RUST: {:?}",
            label, c, r
        );
    }
}

/// Run the same JS source through both libraries and compare.
#[track_caller]
pub fn diff_eval(label: &str, src: &str, flags: c_int) {
    let p = libs();
    let c = p.c.eval(src, flags);
    let r = p.r.eval(src, flags);
    same(&format!("{} | flags={} | src={:?}", label, flags, src), &c, &r);
}

/// Run the same native action through both libraries and compare.
#[track_caller]
pub fn diff_native(label: &str, act: fn(&Api, JS), flags: c_int) {
    let p = libs();
    let c = p.c.run_native(act, flags);
    let r = p.r.run_native(act, flags);
    same(&format!("{} | flags={}", label, flags), &c, &r);
}
