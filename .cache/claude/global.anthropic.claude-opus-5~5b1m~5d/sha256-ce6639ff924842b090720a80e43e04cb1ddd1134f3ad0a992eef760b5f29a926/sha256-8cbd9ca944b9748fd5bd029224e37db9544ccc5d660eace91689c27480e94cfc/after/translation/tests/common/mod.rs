//! Differential test harness: loads the C libmujs.so and the Rust libmujs.so
//! through `libloading` and compares their behaviour through the FFI boundary.
//!
//! Every call goes through `dlsym`ed function pointers — the Rust library is
//! never called directly, so the `#[no_mangle]` export wrappers are tested too.

#![allow(dead_code, non_snake_case, non_camel_case_types, unused_imports)]

use std::ffi::{c_char, c_int, c_short, c_uint, c_ushort, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type JS = *mut c_void;
pub type Obj = *mut c_void;
pub type Prop = *mut c_void;
pub type Env = *mut c_void;
pub type Func = *mut c_void;
pub type Ast = *mut c_void;
pub type Str = *mut c_void;
pub type Buf = *mut c_void;
pub type Reprog = *mut c_void;

pub type Alloc = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
pub type Panic = unsafe extern "C" fn(JS);
pub type CFun = unsafe extern "C" fn(JS);
pub type Finalize = unsafe extern "C" fn(JS, *mut c_void);
pub type HasProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type PutProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type DelProp = unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type Report = unsafe extern "C" fn(JS, *const c_char);

/* --- js_Value: identical repr to the C union (16 bytes) ------------------- */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Value_t {
    pub pad: [c_char; 15],
    pub type_: c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_Value_u {
    pub shrstr: [c_char; 16],
    pub boolean: c_int,
    pub number: f64,
    pub litstr: *const c_char,
    pub memstr: *mut c_void,
    pub object: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_Value {
    pub t: js_Value_t,
    pub u: js_Value_u,
}

impl js_Value {
    pub fn zero() -> js_Value {
        js_Value { u: js_Value_u { shrstr: [0; 16] } }
    }
}

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
    pub sub: [ResubEnt; 16],
}

impl Resub {
    pub fn new() -> Resub {
        Resub { nsub: 0, sub: [ResubEnt { sp: std::ptr::null(), ep: std::ptr::null() }; 16] }
    }
}

/* --- constants from mujs.h ----------------------------------------------- */

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

/* --- the API table ------------------------------------------------------- */

macro_rules! api {
    ( $( $name:ident : $t:ty ),* $(,)? ) => {
        pub struct Api {
            pub which: &'static str,
            $( pub $name : $t, )*
        }
        impl Api {
            unsafe fn build(lib: &libloading::Library, which: &'static str) -> Api {
                Api {
                    which,
                    $( $name : {
                        let s: libloading::Symbol<$t> =
                            lib.get(concat!(stringify!($name), "\0").as_bytes())
                               .unwrap_or_else(|e| panic!("{}: {} missing: {}", which, stringify!($name), e));
                        *s
                    }, )*
                }
            }
        }
    };
}

api! {
    /* state */
    js_newstate: unsafe extern "C" fn(Option<Alloc>, *mut c_void, c_int) -> JS,
    js_freestate: unsafe extern "C" fn(JS),
    js_setcontext: unsafe extern "C" fn(JS, *mut c_void),
    js_getcontext: unsafe extern "C" fn(JS) -> *mut c_void,
    js_setreport: unsafe extern "C" fn(JS, Option<Report>),
    js_atpanic: unsafe extern "C" fn(JS, Option<Panic>) -> Option<Panic>,
    js_gc: unsafe extern "C" fn(JS, c_int),
    js_setlimit: unsafe extern "C" fn(JS, c_int, c_int),
    js_report: unsafe extern "C" fn(JS, *const c_char),

    /* run */
    js_dostring: unsafe extern "C" fn(JS, *const c_char) -> c_int,
    js_ploadstring: unsafe extern "C" fn(JS, *const c_char, *const c_char) -> c_int,
    js_pcall: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_pconstruct: unsafe extern "C" fn(JS, c_int) -> c_int,
    js_loadstring: unsafe extern "C" fn(JS, *const c_char, *const c_char),
    js_loadeval: unsafe extern "C" fn(JS, *const c_char, *const c_char),
    js_eval: unsafe extern "C" fn(JS),
    js_call: unsafe extern "C" fn(JS, c_int),
    js_construct: unsafe extern "C" fn(JS, c_int),
    js_savetry: unsafe extern "C" fn(JS) -> *mut c_void,
    js_savetrypc: unsafe extern "C" fn(JS, *mut c_void) -> *mut c_void,
    js_endtry: unsafe extern "C" fn(JS),
    js_throw: unsafe extern "C" fn(JS),
    js_trap: unsafe extern "C" fn(JS, c_int),

    /* errors */
    js_newerror: unsafe extern "C" fn(JS, *const c_char),
    js_newevalerror: unsafe extern "C" fn(JS, *const c_char),
    js_newrangeerror: unsafe extern "C" fn(JS, *const c_char),
    js_newreferenceerror: unsafe extern "C" fn(JS, *const c_char),
    js_newsyntaxerror: unsafe extern "C" fn(JS, *const c_char),
    js_newtypeerror: unsafe extern "C" fn(JS, *const c_char),
    js_newurierror: unsafe extern "C" fn(JS, *const c_char),
    js_error: unsafe extern "C" fn(JS, *const c_char, ...),
    js_evalerror: unsafe extern "C" fn(JS, *const c_char, ...),
    js_rangeerror: unsafe extern "C" fn(JS, *const c_char, ...),
    js_referenceerror: unsafe extern "C" fn(JS, *const c_char, ...),
    js_syntaxerror: unsafe extern "C" fn(JS, *const c_char, ...),
    js_typeerror: unsafe extern "C" fn(JS, *const c_char, ...),
    js_urierror: unsafe extern "C" fn(JS, *const c_char, ...),

    /* registry / globals */
    js_ref: unsafe extern "C" fn(JS) -> *const c_char,
    js_unref: unsafe extern "C" fn(JS, *const c_char),
    js_getregistry: unsafe extern "C" fn(JS, *const c_char),
    js_setregistry: unsafe extern "C" fn(JS, *const c_char),
    js_delregistry: unsafe extern "C" fn(JS, *const c_char),
    js_getglobal: unsafe extern "C" fn(JS, *const c_char),
    js_setglobal: unsafe extern "C" fn(JS, *const c_char),
    js_defglobal: unsafe extern "C" fn(JS, *const c_char, c_int),
    js_delglobal: unsafe extern "C" fn(JS, *const c_char),

    /* properties */
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

    /* push / new */
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
    js_pushvalue: unsafe extern "C" fn(JS, js_Value),
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
    js_newfunction: unsafe extern "C" fn(JS, Func, Env),
    js_newscript: unsafe extern "C" fn(JS, Func, Env),

    /* iterators */
    js_pushiterator: unsafe extern "C" fn(JS, c_int, c_int),
    js_nextiterator: unsafe extern "C" fn(JS, c_int) -> *const c_char,

    /* predicates */
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

    /* conversions */
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
    js_tovalue: unsafe extern "C" fn(JS, c_int) -> *mut js_Value,
    js_toprimitive: unsafe extern "C" fn(JS, c_int, c_int),
    js_toobject: unsafe extern "C" fn(JS, c_int) -> Obj,
    js_toregexp: unsafe extern "C" fn(JS, c_int) -> *mut c_void,

    /* stack */
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

    /* operators */
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

    /* memory / strings */
    js_malloc: unsafe extern "C" fn(JS, c_int) -> *mut c_void,
    js_realloc: unsafe extern "C" fn(JS, *mut c_void, c_int) -> *mut c_void,
    js_free: unsafe extern "C" fn(JS, *mut c_void),
    js_strdup: unsafe extern "C" fn(JS, *const c_char) -> *mut c_char,
    js_intern: unsafe extern "C" fn(JS, *const c_char) -> *const c_char,
    jsS_dumpstrings: unsafe extern "C" fn(JS),
    jsS_freestrings: unsafe extern "C" fn(JS),
    js_putc: unsafe extern "C" fn(JS, *mut Buf, c_int),
    js_puts: unsafe extern "C" fn(JS, *mut Buf, *const c_char),
    js_putm: unsafe extern "C" fn(JS, *mut Buf, *const c_char, *const c_char),

    /* numbers */
    js_fmtexp: unsafe extern "C" fn(*mut c_char, c_int),
    js_grisu2: unsafe extern "C" fn(f64, *mut c_char, *mut c_int) -> c_int,
    js_strtod: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
    js_strtol: unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> f64,
    js_itoa: unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char,
    js_stringtofloat: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,
    jsV_numbertointeger: unsafe extern "C" fn(f64) -> c_int,
    jsV_numbertoint32: unsafe extern "C" fn(f64) -> c_int,
    jsV_numbertouint32: unsafe extern "C" fn(f64) -> c_uint,
    jsV_numbertoint16: unsafe extern "C" fn(f64) -> c_short,
    jsV_numbertouint16: unsafe extern "C" fn(f64) -> c_ushort,
    jsV_numbertostring: unsafe extern "C" fn(JS, *mut c_char, f64) -> *const c_char,
    jsV_stringtonumber: unsafe extern "C" fn(JS, *const c_char) -> f64,

    /* values / objects (low level) */
    jsV_toboolean: unsafe extern "C" fn(JS, *mut js_Value) -> c_int,
    jsV_tonumber: unsafe extern "C" fn(JS, *mut js_Value) -> f64,
    jsV_tointeger: unsafe extern "C" fn(JS, *mut js_Value) -> f64,
    jsV_tostring: unsafe extern "C" fn(JS, *mut js_Value) -> *const c_char,
    jsV_toobject: unsafe extern "C" fn(JS, *mut js_Value) -> Obj,
    jsV_toprimitive: unsafe extern "C" fn(JS, *mut js_Value, c_int),
    jsV_newobject: unsafe extern "C" fn(JS, c_int, Obj) -> Obj,
    jsV_newmemstring: unsafe extern "C" fn(JS, *const c_char, c_int) -> Str,
    jsV_getownproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_getproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_getpropertyx: unsafe extern "C" fn(JS, Obj, *const c_char, *mut c_int) -> Prop,
    jsV_setproperty: unsafe extern "C" fn(JS, Obj, *const c_char) -> Prop,
    jsV_delproperty: unsafe extern "C" fn(JS, Obj, *const c_char),
    jsV_newiterator: unsafe extern "C" fn(JS, Obj, c_int) -> Obj,
    jsV_nextiterator: unsafe extern "C" fn(JS, Obj) -> *const c_char,
    jsV_resizearray: unsafe extern "C" fn(JS, Obj, c_int),
    jsR_newenvironment: unsafe extern "C" fn(JS, Obj, Env) -> Env,
    jsR_unflattenarray: unsafe extern "C" fn(JS, Obj),
    js_RegExp_prototype_exec: unsafe extern "C" fn(JS, *mut c_void, *const c_char),

    /* utf */
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
    js_utflen: unsafe extern "C" fn(*const c_char) -> c_int,
    js_utfptrtoidx: unsafe extern "C" fn(*const c_char, *const c_char) -> c_int,
    js_runeat: unsafe extern "C" fn(JS, *const c_char, c_int) -> c_int,

    /* lexer / parser / compiler */
    jsY_iswhite: unsafe extern "C" fn(c_int) -> c_int,
    jsY_isnewline: unsafe extern "C" fn(c_int) -> c_int,
    jsY_ishex: unsafe extern "C" fn(c_int) -> c_int,
    jsY_tohex: unsafe extern "C" fn(c_int) -> c_int,
    jsY_tokenstring: unsafe extern "C" fn(c_int) -> *const c_char,
    jsY_findword: unsafe extern "C" fn(*const c_char, *const *const c_char, c_int) -> c_int,
    jsY_initlex: unsafe extern "C" fn(JS, *const c_char, *const c_char),
    jsY_lex: unsafe extern "C" fn(JS) -> c_int,
    jsY_lexjson: unsafe extern "C" fn(JS) -> c_int,
    jsP_parse: unsafe extern "C" fn(JS, *const c_char, *const c_char) -> Ast,
    jsP_parsefunction: unsafe extern "C" fn(JS, *const c_char, *const c_char, *const c_char) -> Ast,
    jsP_freeparse: unsafe extern "C" fn(JS),
    jsC_compilefunction: unsafe extern "C" fn(JS, Ast) -> Func,
    jsC_compilescript: unsafe extern "C" fn(JS, Ast, c_int) -> Func,
    jsC_error: unsafe extern "C" fn(JS, Ast, *const c_char, ...),

    /* builtins */
    jsB_init: unsafe extern "C" fn(JS),
    jsB_initobject: unsafe extern "C" fn(JS),
    jsB_initarray: unsafe extern "C" fn(JS),
    jsB_initfunction: unsafe extern "C" fn(JS),
    jsB_initboolean: unsafe extern "C" fn(JS),
    jsB_initnumber: unsafe extern "C" fn(JS),
    jsB_initstring: unsafe extern "C" fn(JS),
    jsB_initregexp: unsafe extern "C" fn(JS),
    jsB_initerror: unsafe extern "C" fn(JS),
    jsB_initmath: unsafe extern "C" fn(JS),
    jsB_initjson: unsafe extern "C" fn(JS),
    jsB_initdate: unsafe extern "C" fn(JS),
    jsB_propf: unsafe extern "C" fn(JS, *const c_char, Option<CFun>, c_int),
    jsB_propn: unsafe extern "C" fn(JS, *const c_char, f64),
    jsB_props: unsafe extern "C" fn(JS, *const c_char, *const c_char),

    /* regexp */
    js_regcomp: unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> Reprog,
    js_regcompx: unsafe extern "C" fn(Option<Alloc>, *mut c_void, *const c_char, c_int, *mut *const c_char) -> Reprog,
    js_regexec: unsafe extern "C" fn(Reprog, *const c_char, *mut Resub, c_int) -> c_int,
    js_regfree: unsafe extern "C" fn(Reprog),
    js_regfreex: unsafe extern "C" fn(Option<Alloc>, *mut c_void, Reprog),
}

pub struct Libs {
    pub c: Api,
    pub r: Api,
    _libs: (libloading::Library, libloading::Library),
}

unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let rel = base.join("release/libmujs.so");
    let dbg = base.join("debug/libmujs.so");
    match (rel.exists(), dbg.exists()) {
        (true, true) => {
            let mr = std::fs::metadata(&rel).unwrap().modified().unwrap();
            let md = std::fs::metadata(&dbg).unwrap().modified().unwrap();
            if md > mr { dbg } else { rel }
        }
        (true, false) => rel,
        (false, true) => dbg,
        _ => panic!("no Rust libmujs.so built"),
    }
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_C_SO") {
        return PathBuf::from(p);
    }
    root().join("c_src/build/libmujs.so")
}

/// `cargo test` does NOT rebuild the cdylib artifact, so a stale
/// `libmujs.so` would silently make every differential test vacuous.
/// Refuse to run if any Rust source is newer than the loaded library.
fn assert_fresh(rpath: &Path) {
    let so = match std::fs::metadata(rpath).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(&src) {
        for e in rd.flatten() {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map_or(true, |(n, _)| t > *n) {
                    newest = Some((t, e.path()));
                }
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            t <= so,
            "STALE {}: {} is newer. Run `cargo build --release` before `cargo test`.",
            rpath.display(),
            p.display()
        );
    }
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let cpath = c_so();
        let rpath = rust_so();
        assert!(cpath.exists(), "missing {}", cpath.display());
        assert!(rpath.exists(), "missing {}", rpath.display());
        assert_fresh(&rpath);
        /* The C library is linked without -lm (see c_src/CMakeLists.txt), so
         * pull libm into the global scope first. */
        let libm = libloading::os::unix::Library::open(
            Some("libm.so.6"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        );
        std::mem::forget(libm);
        let cl = libloading::Library::new(&cpath).expect("dlopen C lib");
        let rl = libloading::Library::new(&rpath).expect("dlopen Rust lib");
        let c = Api::build(&cl, "C");
        let r = Api::build(&rl, "RUST");
        Libs { c, r, _libs: (cl, rl) }
    })
}

/* --- libc bits used by the harness --------------------------------------- */

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
    fn fputs(s: *const c_char, f: *mut c_void) -> c_int;
    fn puts(s: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` in a forked child with stdout+stderr redirected to a temp file.
/// Returns the captured bytes and the raw wait status.
pub fn capture<F: FnOnce()>(tag: &str, f: F) -> (Vec<u8>, c_int) {
    capture_ex(tag, true, f)
}

/// Like `capture`, but when `merge_stderr` is false the child's stderr is sent
/// to /dev/null and only stdout is captured. Needed for the `assert()` abort
/// paths: the C library is built with assertions live, and glibc's
/// `__assert_fail` prints a message containing the absolute C source path and
/// the host program name, which no translation can reproduce. Exit status and
/// stdout are still compared.
pub fn capture_ex<F: FnOnce()>(tag: &str, merge_stderr: bool, f: F) -> (Vec<u8>, c_int) {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "mujs_diff_{}_{}_{}.out",
        std::process::id(),
        tag.replace(['/', ' ', '.'], "_"),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        if pid == 0 {
            let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
            if fd < 0 {
                _exit(120);
            }
            dup2(fd, 1);
            if merge_stderr {
                dup2(fd, 2);
            } else {
                let devnull = open(cs("/dev/null").as_ptr(), O_RDWR, 0o600 as c_int);
                if devnull >= 0 {
                    dup2(devnull, 2);
                    close(devnull);
                }
            }
            close(fd);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        assert!(pid > 0, "fork failed");
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);
        let data = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (data, status)
    }
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Run the same closure against the C API and the Rust API in separate
/// processes and assert that output bytes and exit status are identical.
pub fn diff<F: Fn(&Api)>(tag: &str, f: F) {
    diff_ex(tag, true, f)
}

/// Same as `diff` but compares stdout only (stderr discarded). Use ONLY for
/// C `assert()` abort paths, whose glibc diagnostic embeds the absolute C
/// source path and the host program name and therefore can never match.
pub fn diff_stdout<F: Fn(&Api)>(tag: &str, f: F) {
    diff_ex(tag, false, f)
}

pub fn diff_ex<F: Fn(&Api)>(tag: &str, merge_stderr: bool, f: F) {
    let l = libs();
    let (oc, sc) = capture_ex(&format!("c_{}", tag), merge_stderr, || f(&l.c));
    let (or, sr) = capture_ex(&format!("r_{}", tag), merge_stderr, || f(&l.r));
    if oc != or || sc != sr {
        let sc_ = decode_status(sc);
        let sr_ = decode_status(sr);
        let a = String::from_utf8_lossy(&oc);
        let b = String::from_utf8_lossy(&or);
        let mut msg = format!(
            "DIVERGENCE in `{}`\n  C status: {}\n  R status: {}\n",
            tag, sc_, sr_
        );
        let al: Vec<&str> = a.lines().collect();
        let bl: Vec<&str> = b.lines().collect();
        let mut shown = 0;
        for i in 0..al.len().max(bl.len()) {
            let x = al.get(i).copied().unwrap_or("<missing>");
            let y = bl.get(i).copied().unwrap_or("<missing>");
            if x != y {
                msg += &format!("  line {}:\n    C: {}\n    R: {}\n", i + 1, x, y);
                shown += 1;
                if shown >= 12 {
                    msg += "  ...\n";
                    break;
                }
            }
        }
        if shown == 0 && oc != or {
            msg += &format!("  bytes differ (C {} bytes, R {} bytes)\n", oc.len(), or.len());
        }
        panic!("{}", msg);
    }
}

pub fn decode_status(s: c_int) -> String {
    if s & 0x7f == 0 {
        format!("exit {}", (s >> 8) & 0xff)
    } else {
        format!("signal {}", s & 0x7f)
    }
}

/* --- printing helpers (libc, so bytes match the library's own output) ----- */

pub fn cs(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

pub unsafe fn p_str(label: &str, s: *const c_char) {
    let l = cs(label);
    if s.is_null() {
        printf(cs("%s=<null>\n").as_ptr(), l.as_ptr());
    } else {
        printf(cs("%s=%s\n").as_ptr(), l.as_ptr(), s);
    }
}

pub unsafe fn p_int(label: &str, v: c_int) {
    printf(cs("%s=%d\n").as_ptr(), cs(label).as_ptr(), v);
}

pub unsafe fn p_uint(label: &str, v: c_uint) {
    printf(cs("%s=%u\n").as_ptr(), cs(label).as_ptr(), v);
}

pub unsafe fn p_num(label: &str, v: f64) {
    printf(cs("%s=%.17g\n").as_ptr(), cs(label).as_ptr(), v);
}

pub unsafe fn p_line(s: &str) {
    puts(cs(s).as_ptr());
}

pub unsafe fn p_ptr_nonnull(label: &str, p: *const c_void) {
    printf(cs("%s=%s\n").as_ptr(), cs(label).as_ptr(),
        cs(if p.is_null() { "null" } else { "nonnull" }).as_ptr());
}

/* --- convenience: fresh state with builtins ------------------------------ */

pub unsafe fn newstate(api: &Api, flags: c_int) -> JS {
    let J = (api.js_newstate)(None, std::ptr::null_mut(), flags);
    assert!(!J.is_null());
    J
}

/// Deterministic xorshift PRNG so both runs see identical inputs.
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
    /// Random double covering the whole bit pattern space (incl. NaN/Inf).
    pub fn bits_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// "Reasonable" random double in a wide but finite range.
    pub fn nice_f64(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let e = (self.next_u32() % 60) as i32 - 30;
        let s = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        s * m * 10f64.powi(e)
    }
    pub fn range(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
