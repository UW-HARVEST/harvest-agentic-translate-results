//! Differential-test harness: loads BOTH the C `libmujs.so` and the Rust
//! `libmujs.so` with `libloading` and exposes every exported symbol as a
//! function pointer, so all calls cross the real FFI boundary.
#![allow(dead_code, non_snake_case, non_camel_case_types, improper_ctypes_definitions)]

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_short, c_uint, c_ushort, c_void};
use std::path::PathBuf;

pub type State = *mut c_void;
pub type Alloc = Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type CFunction = Option<unsafe extern "C-unwind" fn(State)>;
pub type Report = Option<unsafe extern "C-unwind" fn(State, *const c_char)>;
pub type Panic = Option<unsafe extern "C-unwind" fn(State)>;
pub type Finalize = Option<unsafe extern "C-unwind" fn(State, *mut c_void)>;
pub type HasProperty = Option<unsafe extern "C-unwind" fn(State, *mut c_void, *const c_char) -> c_int>;
pub type Put = Option<unsafe extern "C-unwind" fn(State, *mut c_void, *const c_char) -> c_int>;
pub type Delete = Option<unsafe extern "C-unwind" fn(State, *mut c_void, *const c_char) -> c_int>;

/* ---------- state-constructor flags / attributes / enums (mujs.h) ---------- */
pub const JS_STRICT: c_int = 1;
pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;
pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;

/* ---------- regexp.h ---------- */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;
pub const REG_MAXSUB: usize = 16;

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
            sub: [ResubSpan { sp: std::ptr::null(), ep: std::ptr::null() }; REG_MAXSUB],
        }
    }
}

/* ---------- js_Value (jsi.h): 16-byte union, passed by value ---------- */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsValueT {
    pub pad: [c_char; 15],
    pub type_: c_char,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union JsValueU {
    pub shrstr: [c_char; 16],
    pub boolean: c_int,
    pub number: c_double,
    pub litstr: *const c_char,
    pub memstr: *mut c_void,
    pub object: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union JsValue {
    pub t: JsValueT,
    pub u: JsValueU,
}
impl JsValue {
    pub fn zeroed() -> JsValue {
        JsValue { t: JsValueT { pad: [0; 15], type_: 0 } }
    }
    /// Byte-wise view, for byte-identical comparison of stack slots.
    pub fn bytes(&self) -> [u8; 16] {
        unsafe { std::mem::transmute_copy(self) }
    }
    pub fn tag(&self) -> i8 {
        unsafe { self.t.type_ as i8 }
    }
}
pub const JS_TSHRSTR: i8 = 0;
pub const JS_TUNDEFINED: i8 = 1;
pub const JS_TNULL: i8 = 2;
pub const JS_TBOOLEAN: i8 = 3;
pub const JS_TNUMBER: i8 = 4;
pub const JS_TLITSTR: i8 = 5;
pub const JS_TMEMSTR: i8 = 6;
pub const JS_TOBJECT: i8 = 7;

/* enum js_Class */
pub const JS_COBJECT: c_int = 0;
pub const JS_CARRAY: c_int = 1;
pub const JS_CFUNCTION: c_int = 2;
pub const JS_CSCRIPT: c_int = 3;
pub const JS_CCFUNCTION: c_int = 4;
pub const JS_CERROR: c_int = 5;
pub const JS_CBOOLEAN: c_int = 6;
pub const JS_CNUMBER: c_int = 7;
pub const JS_CSTRING: c_int = 8;
pub const JS_CREGEXP: c_int = 9;
pub const JS_CDATE: c_int = 10;
pub const JS_CMATH: c_int = 11;
pub const JS_CJSON: c_int = 12;
pub const JS_CARGUMENTS: c_int = 13;
pub const JS_CITERATOR: c_int = 14;
pub const JS_CUSERDATA: c_int = 15;

/* limits (jsi.h) */
pub const JS_STACKSIZE: c_int = 4096;
pub const JS_ENVLIMIT: c_int = 1024;
pub const JS_TRYLIMIT: c_int = 64;
pub const JS_ARRAYLIMIT: c_int = 1 << 26;
pub const JS_ASTLIMIT: c_int = 400;
pub const JS_STRLIMIT: c_int = 1 << 28;

/* token ids (jsi.h) — only what tests need */
pub const TK_IDENTIFIER: c_int = 256;
pub const TK_WITH: c_int = 256 + 3 + 24 + 30; // last keyword; computed in tests instead

/* ------------------------------------------------------------------ */
/*  The API table                                                      */
/* ------------------------------------------------------------------ */

macro_rules! api {
    ( $( $name:ident : fn ( $($at:ty),* ) $( -> $rt:ty )? ; )* ) => {
        pub struct Api {
            pub path: String,
            $( pub $name : unsafe extern "C-unwind" fn ( $($at),* ) $( -> $rt )? , )*
        }
        impl Api {
            pub fn open(path: &str) -> Api {
                let lib: &'static libloading::Library = Box::leak(Box::new(
                    unsafe { libloading::Library::new(path) }
                        .unwrap_or_else(|e| panic!("dlopen {}: {}", path, e)),
                ));
                unsafe {
                    Api {
                        path: path.to_string(),
                        $( $name : {
                            let s: libloading::Symbol<unsafe extern "C-unwind" fn ( $($at),* ) $( -> $rt )?> =
                                lib.get(concat!(stringify!($name), "\0").as_bytes())
                                   .unwrap_or_else(|e| panic!("dlsym {} in {}: {}", stringify!($name), path, e));
                            *s
                        }, )*
                    }
                }
            }
        }
    };
}

api! {
    /* --- utf.c --- */
    jsU_chartorune: fn(*mut c_int, *const c_char) -> c_int;
    jsU_runetochar: fn(*mut c_char, *const c_int) -> c_int;
    jsU_runelen: fn(c_int) -> c_int;
    jsU_isalpharune: fn(c_int) -> c_int;
    jsU_islowerrune: fn(c_int) -> c_int;
    jsU_isupperrune: fn(c_int) -> c_int;
    jsU_tolowerrune: fn(c_int) -> c_int;
    jsU_toupperrune: fn(c_int) -> c_int;
    jsU_tolowerrune_full: fn(c_int) -> *const c_int;
    jsU_toupperrune_full: fn(c_int) -> *const c_int;

    /* --- regexp.c --- */
    js_regcomp: fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
    js_regcompx: fn(Alloc, *mut c_void, *const c_char, c_int, *mut *const c_char) -> *mut c_void;
    js_regexec: fn(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int;
    js_regfree: fn(*mut c_void);
    js_regfreex: fn(Alloc, *mut c_void, *mut c_void);

    /* --- jsdtoa.c --- */
    js_strtod: fn(*const c_char, *mut *mut c_char) -> c_double;
    js_grisu2: fn(c_double, *mut c_char, *mut c_int) -> c_int;
    /* jsi.h:131  void js_fmtexp(char *p, int e);  -- returns void */
    js_fmtexp: fn(*mut c_char, c_int);

    /* --- jsvalue.c number/string helpers --- */
    /* jsi.h:468  const char *js_itoa(char *buf, int a);  -- NO radix parameter */
    js_itoa: fn(*mut c_char, c_int) -> *const c_char;
    js_strtol: fn(*const c_char, *mut *mut c_char, c_int) -> c_double;
    js_stringtofloat: fn(*const c_char, *mut *mut c_char) -> c_double;
    jsV_numbertostring: fn(State, *mut c_char, c_double) -> *const c_char;
    jsV_stringtonumber: fn(State, *const c_char) -> c_double;
    /* jsi.h:470  int jsV_numbertointeger(double n);  -- returns int */
    jsV_numbertointeger: fn(c_double) -> c_int;
    jsV_numbertoint32: fn(c_double) -> c_int;
    jsV_numbertouint32: fn(c_double) -> c_uint;
    jsV_numbertoint16: fn(c_double) -> c_short;
    jsV_numbertouint16: fn(c_double) -> c_ushort;

    /* --- jsvalue.c low level --- */
    jsV_toboolean: fn(State, *const JsValue) -> c_int;
    jsV_tonumber: fn(State, *const JsValue) -> c_double;
    jsV_tointeger: fn(State, *const JsValue) -> c_double;
    jsV_tostring: fn(State, *const JsValue) -> *const c_char;
    jsV_toobject: fn(State, *const JsValue) -> *mut c_void;
    jsV_toprimitive: fn(State, *mut JsValue, c_int);
    js_newarguments: fn(State);
    js_newfunction: fn(State, *mut c_void, *mut c_void);
    js_newscript: fn(State, *mut c_void, *mut c_void);
    js_compare: fn(State, *mut c_int) -> c_int;
    js_equal: fn(State) -> c_int;
    js_strictequal: fn(State) -> c_int;
    js_instanceof: fn(State) -> c_int;
    js_concat: fn(State);

    /* --- jsproperty.c --- */
    jsV_newobject: fn(State, c_int, *mut c_void) -> *mut c_void;
    jsV_getownproperty: fn(State, *mut c_void, *const c_char) -> *mut c_void;
    jsV_getproperty: fn(State, *mut c_void, *const c_char) -> *mut c_void;
    jsV_getpropertyx: fn(State, *mut c_void, *const c_char, *mut c_int) -> *mut c_void;
    jsV_setproperty: fn(State, *mut c_void, *const c_char) -> *mut c_void;
    jsV_delproperty: fn(State, *mut c_void, *const c_char);
    jsV_newiterator: fn(State, *mut c_void, c_int) -> *mut c_void;
    jsV_nextiterator: fn(State, *mut c_void) -> *const c_char;
    jsV_resizearray: fn(State, *mut c_void, c_int);

    /* --- jsintern.c --- */
    js_intern: fn(State, *const c_char) -> *const c_char;
    jsS_dumpstrings: fn(State);
    jsS_freestrings: fn(State);
    js_putc: fn(State, *mut *mut c_void, c_int);
    js_puts: fn(State, *mut *mut c_void, *const c_char);
    js_putm: fn(State, *mut *mut c_void, *const c_char, *const c_char);

    /* --- jslex.c --- */
    jsY_initlex: fn(State, *const c_char, *const c_char);
    jsY_lex: fn(State) -> c_int;
    jsY_lexjson: fn(State) -> c_int;
    jsY_findword: fn(*const c_char, *const *const c_char, c_int) -> c_int;
    jsY_iswhite: fn(c_int) -> c_int;
    jsY_isnewline: fn(c_int) -> c_int;
    jsY_ishex: fn(c_int) -> c_int;
    jsY_tohex: fn(c_int) -> c_int;
    jsY_tokenstring: fn(c_int) -> *const c_char;

    /* --- jsparse.c / jscompile.c --- */
    jsP_parse: fn(State, *const c_char, *const c_char) -> *mut c_void;
    jsP_parsefunction: fn(State, *const c_char, *const c_char, *const c_char) -> *mut c_void;
    jsP_freeparse: fn(State);
    jsC_compilefunction: fn(State, *mut c_void) -> *mut c_void;
    jsC_compilescript: fn(State, *mut c_void, c_int) -> *mut c_void;

    /* --- jsrun.c: memory --- */
    js_malloc: fn(State, c_int) -> *mut c_void;
    js_realloc: fn(State, *mut c_void, c_int) -> *mut c_void;
    js_free: fn(State, *mut c_void);
    js_strdup: fn(State, *const c_char) -> *mut c_char;
    jsV_newmemstring: fn(State, *const c_char, c_int) -> *mut c_void;
    jsR_newenvironment: fn(State, *mut c_void, *mut c_void) -> *mut c_void;
    jsR_unflattenarray: fn(State, *mut c_void);

    /* --- jsstate.c --- */
    js_newstate: fn(Alloc, *mut c_void, c_int) -> State;
    js_freestate: fn(State);
    js_setcontext: fn(State, *mut c_void);
    js_getcontext: fn(State) -> *mut c_void;
    js_setreport: fn(State, Report);
    js_atpanic: fn(State, Panic) -> Panic;
    js_report: fn(State, *const c_char);
    js_gc: fn(State, c_int);
    js_setlimit: fn(State, c_int, c_int);
    js_dostring: fn(State, *const c_char) -> c_int;
    js_ploadstring: fn(State, *const c_char, *const c_char) -> c_int;
    js_loadstring: fn(State, *const c_char, *const c_char);
    js_loadeval: fn(State, *const c_char, *const c_char);
    js_trystring: fn(State, c_int, *const c_char) -> *const c_char;
    js_trynumber: fn(State, c_int, c_double) -> c_double;
    js_tryinteger: fn(State, c_int, c_int) -> c_int;
    js_tryboolean: fn(State, c_int, c_int) -> c_int;

    /* --- jsrun.c: calls & exceptions --- */
    js_pcall: fn(State, c_int) -> c_int;
    js_pconstruct: fn(State, c_int) -> c_int;
    js_call: fn(State, c_int);
    js_construct: fn(State, c_int);
    js_eval: fn(State);
    js_savetry: fn(State) -> *mut c_void;
    js_savetrypc: fn(State, *mut c_void) -> *mut c_void;
    js_endtry: fn(State);
    js_throw: fn(State);
    js_trap: fn(State, c_int);

    /* --- jsrun.c: stack --- */
    js_gettop: fn(State) -> c_int;
    js_pop: fn(State, c_int);
    js_rot: fn(State, c_int);
    js_copy: fn(State, c_int);
    js_remove: fn(State, c_int);
    js_insert: fn(State, c_int);
    js_replace: fn(State, c_int);
    js_dup: fn(State);
    js_dup2: fn(State);
    js_rot2: fn(State);
    js_rot3: fn(State);
    js_rot4: fn(State);
    js_rot2pop1: fn(State);
    js_rot3pop2: fn(State);

    /* --- jsrun.c: push --- */
    js_pushvalue: fn(State, JsValue);
    js_pushobject: fn(State, *mut c_void);
    js_pushglobal: fn(State);
    js_pushundefined: fn(State);
    js_pushnull: fn(State);
    js_pushboolean: fn(State, c_int);
    js_pushnumber: fn(State, c_double);
    js_pushstring: fn(State, *const c_char);
    js_pushlstring: fn(State, *const c_char, c_int);
    js_pushliteral: fn(State, *const c_char);
    js_pushiterator: fn(State, c_int, c_int);
    js_nextiterator: fn(State, c_int) -> *const c_char;

    /* --- jsrun.c: new --- */
    js_newobject: fn(State);
    js_newobjectx: fn(State);
    js_newarray: fn(State);
    js_newboolean: fn(State, c_int);
    js_newnumber: fn(State, c_double);
    js_newstring: fn(State, *const c_char);
    js_newcfunction: fn(State, CFunction, *const c_char, c_int);
    js_newcfunctionx: fn(State, CFunction, *const c_char, c_int, *mut c_void, Finalize);
    js_newcconstructor: fn(State, CFunction, CFunction, *const c_char, c_int);
    js_newuserdata: fn(State, *const c_char, *mut c_void, Finalize);
    js_newuserdatax: fn(State, *const c_char, *mut c_void, HasProperty, Put, Delete, Finalize);
    js_newregexp: fn(State, *const c_char, c_int);

    /* --- jserror.c: js_new*error --- */
    js_newerror: fn(State, *const c_char);
    js_newevalerror: fn(State, *const c_char);
    js_newrangeerror: fn(State, *const c_char);
    js_newreferenceerror: fn(State, *const c_char);
    js_newsyntaxerror: fn(State, *const c_char);
    js_newtypeerror: fn(State, *const c_char);
    js_newurierror: fn(State, *const c_char);

    /* --- jsrun.c: properties --- */
    js_hasproperty: fn(State, c_int, *const c_char) -> c_int;
    js_getproperty: fn(State, c_int, *const c_char);
    js_setproperty: fn(State, c_int, *const c_char);
    js_defproperty: fn(State, c_int, *const c_char, c_int);
    js_delproperty: fn(State, c_int, *const c_char);
    js_defaccessor: fn(State, c_int, *const c_char, c_int);
    js_getglobal: fn(State, *const c_char);
    js_setglobal: fn(State, *const c_char);
    js_defglobal: fn(State, *const c_char, c_int);
    js_delglobal: fn(State, *const c_char);
    js_getregistry: fn(State, *const c_char);
    js_setregistry: fn(State, *const c_char);
    js_delregistry: fn(State, *const c_char);
    js_ref: fn(State) -> *const c_char;
    js_unref: fn(State, *const c_char);
    js_getlength: fn(State, c_int) -> c_int;
    js_setlength: fn(State, c_int, c_int);
    js_hasindex: fn(State, c_int, c_int) -> c_int;
    js_getindex: fn(State, c_int, c_int);
    js_setindex: fn(State, c_int, c_int);
    js_delindex: fn(State, c_int, c_int);
    js_currentfunction: fn(State);
    js_currentfunctiondata: fn(State) -> *mut c_void;

    /* --- jsrun.c: predicates & conversions --- */
    js_isdefined: fn(State, c_int) -> c_int;
    js_isundefined: fn(State, c_int) -> c_int;
    js_isnull: fn(State, c_int) -> c_int;
    js_isboolean: fn(State, c_int) -> c_int;
    js_isnumber: fn(State, c_int) -> c_int;
    js_isstring: fn(State, c_int) -> c_int;
    js_isprimitive: fn(State, c_int) -> c_int;
    js_isobject: fn(State, c_int) -> c_int;
    js_isarray: fn(State, c_int) -> c_int;
    js_isregexp: fn(State, c_int) -> c_int;
    js_iscoercible: fn(State, c_int) -> c_int;
    js_iscallable: fn(State, c_int) -> c_int;
    js_isuserdata: fn(State, c_int, *const c_char) -> c_int;
    js_iserror: fn(State, c_int) -> c_int;
    js_isnumberobject: fn(State, c_int) -> c_int;
    js_isstringobject: fn(State, c_int) -> c_int;
    js_isbooleanobject: fn(State, c_int) -> c_int;
    js_isdateobject: fn(State, c_int) -> c_int;
    js_isarrayindex: fn(State, *const c_char, *mut c_int) -> c_int;
    js_toboolean: fn(State, c_int) -> c_int;
    js_tonumber: fn(State, c_int) -> c_double;
    js_tostring: fn(State, c_int) -> *const c_char;
    js_touserdata: fn(State, c_int, *const c_char) -> *mut c_void;
    js_tointeger: fn(State, c_int) -> c_int;
    js_toint32: fn(State, c_int) -> c_int;
    js_touint32: fn(State, c_int) -> c_uint;
    js_toint16: fn(State, c_int) -> c_short;
    js_touint16: fn(State, c_int) -> c_ushort;
    js_tovalue: fn(State, c_int) -> *mut JsValue;
    js_toobject: fn(State, c_int) -> *mut c_void;
    js_toprimitive: fn(State, c_int, c_int);
    js_toregexp: fn(State, c_int) -> *mut c_void;
    js_typeof: fn(State, c_int) -> *const c_char;
    js_type: fn(State, c_int) -> c_int;
    js_utflen: fn(*const c_char) -> c_int;
    js_utfptrtoidx: fn(*const c_char, *const c_char) -> c_int;
    js_runeat: fn(State, *const c_char, c_int) -> c_int;

    /* --- jsrepr.c --- */
    js_repr: fn(State, c_int);
    js_torepr: fn(State, c_int) -> *const c_char;
    js_tryrepr: fn(State, c_int, *const c_char) -> *const c_char;

    /* --- jsbuiltin.c / init entry points --- */
    jsB_init: fn(State);
    jsB_initobject: fn(State);
    jsB_initarray: fn(State);
    jsB_initboolean: fn(State);
    jsB_initdate: fn(State);
    jsB_initerror: fn(State);
    jsB_initfunction: fn(State);
    jsB_initjson: fn(State);
    jsB_initmath: fn(State);
    jsB_initnumber: fn(State);
    jsB_initregexp: fn(State);
    jsB_initstring: fn(State);
    jsB_propf: fn(State, *const c_char, CFunction, c_int);
    jsB_propn: fn(State, *const c_char, c_double);
    jsB_props: fn(State, *const c_char, *const c_char);
    js_RegExp_prototype_exec: fn(State, *mut c_void, *const c_char);
}

/* ------------------------------------------------------------------ */
/*  Library discovery                                                  */
/* ------------------------------------------------------------------ */

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the freshly built Rust cdylib (target/<profile>).
///
/// `cargo test --test X` does not itself emit the cdylib, so the test suite
/// must be run after `cargo build` (see `run_tests.sh`). We look in the
/// profile directory that matches this test binary first.
fn rust_so_path() -> PathBuf {
    // current_exe = target/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().unwrap().to_path_buf(); // deps
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let p = dir.join("libmujs.so");
    if p.exists() {
        return p;
    }
    panic!(
        "Rust cdylib not found at {:?}.\n\
         `cargo test --test ...` does not build a cdylib-only lib target; run\n\
         `cargo build` (same profile) first, or use ./run_tests.sh",
        p
    );
}

fn c_so_path() -> PathBuf {
    let p = crate_root().join("c_src/build/libmujs.so");
    assert!(
        p.exists(),
        "C shared library not found at {:?}; build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p
    );
    p
}

/// `c_src/CMakeLists.txt` does not link `m`, so the C `libmujs.so` has
/// undefined references to `floor`, `fmod`, ... Load libm into the global scope
/// first so they can be resolved. (The two mujs libraries themselves are loaded
/// RTLD_LOCAL by libloading, so they never interpose on each other.)
fn preload_libm() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for name in ["libm.so.6", "libm.so"] {
            let n = CString::new(name).unwrap();
            let h = unsafe { libc::dlopen(n.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
            if !h.is_null() {
                return;
            }
        }
        // On glibc >= 2.34 the math symbols live in libc itself, which is
        // already global; nothing to do.
    });
}

thread_local! {
    static LIBS: (Api, Api) = {
        preload_libm();
        (
            Api::open(c_so_path().to_str().unwrap()),
            Api::open(rust_so_path().to_str().unwrap()),
        )
    };
}

/// Run `f` once for the C library and once for the Rust library.
/// Returns `(c_result, rust_result)`.
pub fn both<T>(mut f: impl FnMut(&Api, Side) -> T) -> (T, T) {
    LIBS.with(|(c, r)| (f(c, Side::C), f(r, Side::Rust)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    C,
    Rust,
}
impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::C => "C",
            Side::Rust => "Rust",
        }
    }
}

/// Assert the two results are equal, with a descriptive message.
#[track_caller]
pub fn same<T: PartialEq + std::fmt::Debug>(what: &str, (c, r): (T, T)) {
    assert_eq!(c, r, "DIVERGENCE in {}: C={:?} Rust={:?}", what, c, r);
}

/* ------------------------------------------------------------------ */
/*  Helpers                                                           */
/* ------------------------------------------------------------------ */

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Read a NUL-terminated C string as a lossless byte vector (may be non-UTF8).
pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub unsafe fn cstr_string(p: *const c_char) -> Option<String> {
    cstr_bytes(p).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/* ---------- report / print capture ---------- */

thread_local! {
    static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    /// js_tostring of the library currently executing, for the print callback.
    static TOSTRING: RefCell<Option<unsafe extern "C-unwind" fn(State, c_int) -> *const c_char>> =
        RefCell::new(None);
    static TRYSTRING: RefCell<Option<unsafe extern "C-unwind" fn(State, c_int, *const c_char) -> *const c_char>> =
        RefCell::new(None);
    static GETTOP: RefCell<Option<unsafe extern "C-unwind" fn(State) -> c_int>> = RefCell::new(None);
    static PUSHUNDEF: RefCell<Option<unsafe extern "C-unwind" fn(State)>> = RefCell::new(None);
}

fn out_push(bytes: &[u8]) {
    OUT.with(|o| o.borrow_mut().extend_from_slice(bytes));
}
pub fn out_take() -> Vec<u8> {
    OUT.with(|o| std::mem::take(&mut *o.borrow_mut()))
}
pub fn out_clear() {
    OUT.with(|o| o.borrow_mut().clear());
}

pub unsafe extern "C-unwind" fn report_cb(_J: State, msg: *const c_char) {
    out_push(b"[report] ");
    if !msg.is_null() {
        out_push(CStr::from_ptr(msg).to_bytes());
    } else {
        out_push(b"(null)");
    }
    out_push(b"\n");
}

pub unsafe extern "C-unwind" fn panic_cb(_J: State) {
    out_push(b"[panic]\n");
}

/// A `print`-style cfunction: joins all arguments with a space using the
/// *protected* `js_trystring` (so it can never throw out of the callback).
pub unsafe extern "C-unwind" fn print_cb(J: State) {
    let gettop = GETTOP.with(|g| *g.borrow()).unwrap();
    let trystring = TRYSTRING.with(|g| *g.borrow()).unwrap();
    let pushundef = PUSHUNDEF.with(|g| *g.borrow()).unwrap();
    let top = gettop(J);
    let err = b"(unprintable)\0";
    let mut line: Vec<u8> = Vec::new();
    for i in 1..top {
        if i > 1 {
            line.push(b' ');
        }
        let p = trystring(J, i, err.as_ptr() as *const c_char);
        if !p.is_null() {
            line.extend_from_slice(CStr::from_ptr(p).to_bytes());
        }
    }
    line.push(b'\n');
    out_push(&line);
    pushundef(J);
}

/// Install the thread-local trampoline slots for `api` (needed by `print_cb`).
pub fn bind_callbacks(api: &Api) {
    TOSTRING.with(|s| *s.borrow_mut() = Some(api.js_tostring));
    TRYSTRING.with(|s| *s.borrow_mut() = Some(api.js_trystring));
    GETTOP.with(|s| *s.borrow_mut() = Some(api.js_gettop));
    PUSHUNDEF.with(|s| *s.borrow_mut() = Some(api.js_pushundefined));
}

static PRINT_NAME: &[u8] = b"print\0";

/// Create a state with a report callback and a global `print` function bound.
pub unsafe fn new_state(api: &Api, flags: c_int) -> State {
    bind_callbacks(api);
    let J = (api.js_newstate)(None, std::ptr::null_mut(), flags);
    assert!(!J.is_null(), "js_newstate returned NULL in {}", api.path);
    (api.js_setreport)(J, Some(report_cb));
    (api.js_atpanic)(J, Some(panic_cb));
    // NOTE: js_newcfunctionx stores `name` as a bare pointer without copying
    // (`obj->u.c.name = name;` in jsvalue.c), so the name must outlive the
    // state. A `CString` temporary would be freed immediately and
    // `Function.prototype.toString` would then read freed memory.
    (api.js_newcfunction)(J, Some(print_cb), PRINT_NAME.as_ptr() as *const c_char, 1);
    (api.js_setglobal)(J, PRINT_NAME.as_ptr() as *const c_char);
    J
}

/// Run one script through `js_dostring` and return
/// `(return code, captured output bytes)`.
pub fn run_script(api: &Api, flags: c_int, src: &str) -> (c_int, Vec<u8>) {
    unsafe {
        out_clear();
        let J = new_state(api, flags);
        let rc = (api.js_dostring)(J, cs(src).as_ptr());
        (api.js_freestate)(J);
        (rc, out_take())
    }
}

/// Differential helper: run the same script in both libraries and assert the
/// return code and all captured output are byte-identical.
#[track_caller]
pub fn diff_script(flags: c_int, src: &str) {
    let (c, r) = both(|api, _| run_script(api, flags, src));
    if c != r {
        panic!(
            "DIVERGENCE for script (flags={}):\n---8<--- {}\n--->8---\n C  rc={} out={:?}\n Rust rc={} out={:?}",
            flags,
            src,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

/// Same, but for a whole list of scripts; reports every divergence at once.
#[track_caller]
pub fn diff_scripts(flags: c_int, scripts: &[&str]) {
    let mut fails = Vec::new();
    for (i, s) in scripts.iter().enumerate() {
        let (c, r) = both(|api, _| run_script(api, flags, s));
        if c != r {
            fails.push(format!(
                "  [{}] {:?}\n      C   : rc={} out={:?}\n      Rust: rc={} out={:?}",
                i,
                s,
                c.0,
                String::from_utf8_lossy(&c.1),
                r.0,
                String::from_utf8_lossy(&r.1)
            ));
        }
    }
    if !fails.is_empty() {
        panic!(
            "{} of {} scripts diverged (flags={}):\n{}",
            fails.len(),
            scripts.len(),
            flags,
            fails.join("\n")
        );
    }
}

/// Run a script under both non-strict and strict states.
#[track_caller]
pub fn diff_scripts_both_modes(scripts: &[&str]) {
    diff_scripts(0, scripts);
    diff_scripts(JS_STRICT, scripts);
}

/* ---------- deterministic RNG (xorshift64*) for property tests ---------- */

pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            lo
        } else {
            lo + (self.next_u64() % ((hi - lo) as u64)) as i64
        }
    }
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "interesting" finite-ish double.
    pub fn nice_f64(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => self.range(-1000, 1000) as f64,
            6 => self.range(i32::MIN as i64, i32::MAX as i64) as f64,
            7 => self.f64_bits(),
            8 => (self.next_u32() as f64) / 4096.0,
            _ => (self.range(-1_000_000, 1_000_000) as f64) / 1000.0,
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
}

/// Format a double so that all bits are visible in diagnostics.
pub fn dbg_f64(x: f64) -> String {
    format!("{:?}/{:#018x}", x, x.to_bits())
}
