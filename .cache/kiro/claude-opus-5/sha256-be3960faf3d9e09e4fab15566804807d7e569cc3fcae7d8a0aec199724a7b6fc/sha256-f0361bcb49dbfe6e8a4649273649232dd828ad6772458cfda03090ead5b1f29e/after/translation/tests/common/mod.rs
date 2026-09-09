//! Differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading` and exposes the same typed
//! function-pointer table for each, so every call in every test crosses a real
//! FFI boundary (exercising the `#[no_mangle]` export wrappers).
//!
//!  * C   `.so`: `c_src/build/libmujs.so`
//!  * Rust `.so`: `translation/target/{release,debug}/libmujs.so`
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::Library;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_short, c_uint, c_ushort, c_void};
use std::path::PathBuf;

pub type JS = *mut c_void; // js_State *
pub type Obj = *mut c_void; // js_Object *
pub type Prog = *mut c_void; // Reprog *
pub type Ast = *mut c_void; // js_Ast *
pub type Func = *mut c_void; // js_Function *
pub type Env = *mut c_void; // js_Environment *
pub type Prop = *mut c_void; // js_Property *
pub type Rune = c_int;

pub type CFunction = unsafe extern "C-unwind" fn(JS);
pub type Alloc = unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
pub type Panic = unsafe extern "C-unwind" fn(JS);
pub type Report = unsafe extern "C-unwind" fn(JS, *const c_char);
pub type Finalize = unsafe extern "C-unwind" fn(JS, *mut c_void);
pub type HasProperty = unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type PutProperty = unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int;
pub type DelProperty = unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) -> c_int;

pub const REG_MAXSUB: usize = 16;

/// `union js_Value` from `c_src/src/jsi.h` — 16 bytes, SysV class INTEGER,INTEGER
/// (the `char shrstr[16]` member forces INTEGER for both eightbytes).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JsValue(pub [u8; 16]);

/// Layout must match `struct Resub` in `c_src/src/regexp.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubPair; REG_MAXSUB],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResubPair {
    pub sp: *const c_char,
    pub ep: *const c_char,
}
impl Default for Resub {
    fn default() -> Self {
        Resub {
            nsub: 0,
            sub: [ResubPair {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

/* state flags */
pub const JS_STRICT: c_int = 1;
/* regexp (public) flags */
pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;
/* property attributes */
pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;
/* regexp.h flags */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;

pub const RUNEERROR: Rune = 0xFFFD;
pub const RUNEMAX: Rune = 0x10FFFF;

macro_rules! api {
    ($($field:ident : $t:ty = $sym:literal;)*) => {
        pub struct Api {
            pub lib: &'static Library,
            pub tag: &'static str,
            $(pub $field: $t,)*
        }
        impl Api {
            pub fn load(path: &std::path::Path, tag: &'static str) -> Api {
                let lib: &'static Library = Box::leak(Box::new(
                    unsafe { Library::new(path) }
                        .unwrap_or_else(|e| panic!("dlopen {}: {}", path.display(), e)),
                ));
                unsafe {
                    Api {
                        lib,
                        tag,
                        $($field: *lib
                            .get::<$t>(concat!($sym, "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", tag, $sym, e)),)*
                    }
                }
            }
        }
    };
}

api! {
    /* ---- allocator wrappers ---- */
    js_malloc: unsafe extern "C-unwind" fn(JS, c_int) -> *mut c_void = "js_malloc";
    js_realloc: unsafe extern "C-unwind" fn(JS, *mut c_void, c_int) -> *mut c_void = "js_realloc";
    js_free: unsafe extern "C-unwind" fn(JS, *mut c_void) = "js_free";
    js_strdup: unsafe extern "C-unwind" fn(JS, *const c_char) -> *mut c_char = "js_strdup";
    js_intern: unsafe extern "C-unwind" fn(JS, *const c_char) -> *const c_char = "js_intern";
    jsS_dumpstrings: unsafe extern "C-unwind" fn(JS) = "jsS_dumpstrings";
    jsS_freestrings: unsafe extern "C-unwind" fn(JS) = "jsS_freestrings";

    /* ---- utf ---- */
    jsU_chartorune: unsafe extern "C-unwind" fn(*mut Rune, *const c_char) -> c_int = "jsU_chartorune";
    jsU_runetochar: unsafe extern "C-unwind" fn(*mut c_char, *const Rune) -> c_int = "jsU_runetochar";
    jsU_runelen: unsafe extern "C-unwind" fn(c_int) -> c_int = "jsU_runelen";
    jsU_isalpharune: unsafe extern "C-unwind" fn(Rune) -> c_int = "jsU_isalpharune";
    jsU_islowerrune: unsafe extern "C-unwind" fn(Rune) -> c_int = "jsU_islowerrune";
    jsU_isupperrune: unsafe extern "C-unwind" fn(Rune) -> c_int = "jsU_isupperrune";
    jsU_tolowerrune: unsafe extern "C-unwind" fn(Rune) -> Rune = "jsU_tolowerrune";
    jsU_toupperrune: unsafe extern "C-unwind" fn(Rune) -> Rune = "jsU_toupperrune";
    jsU_tolowerrune_full: unsafe extern "C-unwind" fn(Rune) -> *const Rune = "jsU_tolowerrune_full";
    jsU_toupperrune_full: unsafe extern "C-unwind" fn(Rune) -> *const Rune = "jsU_toupperrune_full";
    js_utflen: unsafe extern "C-unwind" fn(*const c_char) -> c_int = "js_utflen";
    js_utfptrtoidx: unsafe extern "C-unwind" fn(*const c_char, *const c_char) -> c_int = "js_utfptrtoidx";
    js_runeat: unsafe extern "C-unwind" fn(JS, *const c_char, c_int) -> c_int = "js_runeat";

    /* ---- number formatting / parsing ---- */
    js_itoa: unsafe extern "C-unwind" fn(*mut c_char, c_int) -> *const c_char = "js_itoa";
    js_grisu2: unsafe extern "C-unwind" fn(c_double, *mut c_char, *mut c_int) -> c_int = "js_grisu2";
    js_fmtexp: unsafe extern "C-unwind" fn(*mut c_char, c_int) = "js_fmtexp";
    js_strtod: unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char) -> c_double = "js_strtod";
    js_strtol: unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char, c_int) -> c_double = "js_strtol";
    js_stringtofloat: unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char) -> c_double = "js_stringtofloat";
    jsV_numbertostring: unsafe extern "C-unwind" fn(JS, *mut c_char, c_double) -> *const c_char = "jsV_numbertostring";
    jsV_stringtonumber: unsafe extern "C-unwind" fn(JS, *const c_char) -> c_double = "jsV_stringtonumber";
    jsV_numbertointeger: unsafe extern "C-unwind" fn(c_double) -> c_int = "jsV_numbertointeger";
    jsV_numbertoint32: unsafe extern "C-unwind" fn(c_double) -> c_int = "jsV_numbertoint32";
    jsV_numbertouint32: unsafe extern "C-unwind" fn(c_double) -> c_uint = "jsV_numbertouint32";
    jsV_numbertoint16: unsafe extern "C-unwind" fn(c_double) -> c_short = "jsV_numbertoint16";
    jsV_numbertouint16: unsafe extern "C-unwind" fn(c_double) -> c_ushort = "jsV_numbertouint16";
    js_isarrayindex: unsafe extern "C-unwind" fn(JS, *const c_char, *mut c_int) -> c_int = "js_isarrayindex";

    /* ---- regexp (low level) ---- */
    js_regcomp: unsafe extern "C-unwind" fn(*const c_char, c_int, *mut *const c_char) -> Prog = "js_regcomp";
    js_regcompx: unsafe extern "C-unwind" fn(Alloc, *mut c_void, *const c_char, c_int, *mut *const c_char) -> Prog = "js_regcompx";
    js_regexec: unsafe extern "C-unwind" fn(Prog, *const c_char, *mut Resub, c_int) -> c_int = "js_regexec";
    js_regfree: unsafe extern "C-unwind" fn(Prog) = "js_regfree";
    js_regfreex: unsafe extern "C-unwind" fn(Alloc, *mut c_void, Prog) = "js_regfreex";

    /* ---- state ---- */
    js_newstate: unsafe extern "C-unwind" fn(Option<Alloc>, *mut c_void, c_int) -> JS = "js_newstate";
    js_freestate: unsafe extern "C-unwind" fn(JS) = "js_freestate";
    js_setcontext: unsafe extern "C-unwind" fn(JS, *mut c_void) = "js_setcontext";
    js_getcontext: unsafe extern "C-unwind" fn(JS) -> *mut c_void = "js_getcontext";
    js_setreport: unsafe extern "C-unwind" fn(JS, Option<Report>) = "js_setreport";
    js_report: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_report";
    js_atpanic: unsafe extern "C-unwind" fn(JS, Option<Panic>) -> Option<Panic> = "js_atpanic";
    js_gc: unsafe extern "C-unwind" fn(JS, c_int) = "js_gc";
    js_setlimit: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_setlimit";

    js_dostring: unsafe extern "C-unwind" fn(JS, *const c_char) -> c_int = "js_dostring";
    js_ploadstring: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) -> c_int = "js_ploadstring";
    js_pcall: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_pcall";
    js_pconstruct: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_pconstruct";
    js_loadstring: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) = "js_loadstring";
    js_loadeval: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) = "js_loadeval";
    js_eval: unsafe extern "C-unwind" fn(JS) = "js_eval";
    js_call: unsafe extern "C-unwind" fn(JS, c_int) = "js_call";
    js_construct: unsafe extern "C-unwind" fn(JS, c_int) = "js_construct";

    js_savetry: unsafe extern "C-unwind" fn(JS) -> *mut c_void = "js_savetry";
    js_endtry: unsafe extern "C-unwind" fn(JS) = "js_endtry";
    js_throw: unsafe extern "C-unwind" fn(JS) = "js_throw";

    /* ---- errors ---- */
    js_newerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newerror";
    js_newevalerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newevalerror";
    js_newrangeerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newrangeerror";
    js_newreferenceerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newreferenceerror";
    js_newsyntaxerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newsyntaxerror";
    js_newtypeerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newtypeerror";
    js_newurierror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newurierror";
    js_error: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_error";
    js_evalerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_evalerror";
    js_rangeerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_rangeerror";
    js_referenceerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_referenceerror";
    js_syntaxerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_syntaxerror";
    js_typeerror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_typeerror";
    js_urierror: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_urierror";

    /* ---- registry / refs ---- */
    js_ref: unsafe extern "C-unwind" fn(JS) -> *const c_char = "js_ref";
    js_unref: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_unref";
    js_getregistry: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_getregistry";
    js_setregistry: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_setregistry";
    js_delregistry: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_delregistry";

    /* ---- globals / properties ---- */
    js_getglobal: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_getglobal";
    js_setglobal: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_setglobal";
    js_defglobal: unsafe extern "C-unwind" fn(JS, *const c_char, c_int) = "js_defglobal";
    js_delglobal: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_delglobal";
    js_hasproperty: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) -> c_int = "js_hasproperty";
    js_getproperty: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) = "js_getproperty";
    js_setproperty: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) = "js_setproperty";
    js_defproperty: unsafe extern "C-unwind" fn(JS, c_int, *const c_char, c_int) = "js_defproperty";
    js_delproperty: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) = "js_delproperty";
    js_defaccessor: unsafe extern "C-unwind" fn(JS, c_int, *const c_char, c_int) = "js_defaccessor";
    js_getlength: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_getlength";
    js_setlength: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_setlength";
    js_hasindex: unsafe extern "C-unwind" fn(JS, c_int, c_int) -> c_int = "js_hasindex";
    js_getindex: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_getindex";
    js_setindex: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_setindex";
    js_delindex: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_delindex";

    /* ---- push / new ---- */
    js_currentfunction: unsafe extern "C-unwind" fn(JS) = "js_currentfunction";
    js_currentfunctiondata: unsafe extern "C-unwind" fn(JS) -> *mut c_void = "js_currentfunctiondata";
    js_pushglobal: unsafe extern "C-unwind" fn(JS) = "js_pushglobal";
    js_pushundefined: unsafe extern "C-unwind" fn(JS) = "js_pushundefined";
    js_pushnull: unsafe extern "C-unwind" fn(JS) = "js_pushnull";
    js_pushboolean: unsafe extern "C-unwind" fn(JS, c_int) = "js_pushboolean";
    js_pushnumber: unsafe extern "C-unwind" fn(JS, c_double) = "js_pushnumber";
    js_pushstring: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_pushstring";
    js_pushlstring: unsafe extern "C-unwind" fn(JS, *const c_char, c_int) = "js_pushlstring";
    js_pushliteral: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_pushliteral";
    js_newobjectx: unsafe extern "C-unwind" fn(JS) = "js_newobjectx";
    js_newobject: unsafe extern "C-unwind" fn(JS) = "js_newobject";
    js_newarray: unsafe extern "C-unwind" fn(JS) = "js_newarray";
    js_newboolean: unsafe extern "C-unwind" fn(JS, c_int) = "js_newboolean";
    js_newnumber: unsafe extern "C-unwind" fn(JS, c_double) = "js_newnumber";
    js_newstring: unsafe extern "C-unwind" fn(JS, *const c_char) = "js_newstring";
    js_newcfunction: unsafe extern "C-unwind" fn(JS, CFunction, *const c_char, c_int) = "js_newcfunction";
    js_newcfunctionx: unsafe extern "C-unwind" fn(JS, CFunction, *const c_char, c_int, *mut c_void, Option<Finalize>) = "js_newcfunctionx";
    js_newcconstructor: unsafe extern "C-unwind" fn(JS, CFunction, CFunction, *const c_char, c_int) = "js_newcconstructor";
    js_newuserdata: unsafe extern "C-unwind" fn(JS, *const c_char, *mut c_void, Option<Finalize>) = "js_newuserdata";
    js_newuserdatax: unsafe extern "C-unwind" fn(JS, *const c_char, *mut c_void, Option<HasProperty>, Option<PutProperty>, Option<DelProperty>, Option<Finalize>) = "js_newuserdatax";
    js_newregexp: unsafe extern "C-unwind" fn(JS, *const c_char, c_int) = "js_newregexp";

    js_pushiterator: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_pushiterator";
    js_nextiterator: unsafe extern "C-unwind" fn(JS, c_int) -> *const c_char = "js_nextiterator";

    /* ---- predicates ---- */
    js_isdefined: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isdefined";
    js_isundefined: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isundefined";
    js_isnull: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isnull";
    js_isboolean: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isboolean";
    js_isnumber: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isnumber";
    js_isstring: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isstring";
    js_isprimitive: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isprimitive";
    js_isobject: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isobject";
    js_isarray: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isarray";
    js_isregexp: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isregexp";
    js_iscoercible: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_iscoercible";
    js_iscallable: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_iscallable";
    js_isuserdata: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) -> c_int = "js_isuserdata";
    js_iserror: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_iserror";
    js_isnumberobject: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isnumberobject";
    js_isstringobject: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isstringobject";
    js_isbooleanobject: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isbooleanobject";
    js_isdateobject: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_isdateobject";

    /* ---- conversions ---- */
    js_toboolean: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_toboolean";
    js_tonumber: unsafe extern "C-unwind" fn(JS, c_int) -> c_double = "js_tonumber";
    js_tostring: unsafe extern "C-unwind" fn(JS, c_int) -> *const c_char = "js_tostring";
    js_touserdata: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) -> *mut c_void = "js_touserdata";
    js_trystring: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) -> *const c_char = "js_trystring";
    js_trynumber: unsafe extern "C-unwind" fn(JS, c_int, c_double) -> c_double = "js_trynumber";
    js_tryinteger: unsafe extern "C-unwind" fn(JS, c_int, c_int) -> c_int = "js_tryinteger";
    js_tryboolean: unsafe extern "C-unwind" fn(JS, c_int, c_int) -> c_int = "js_tryboolean";
    js_tointeger: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_tointeger";
    js_toint32: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_toint32";
    js_touint32: unsafe extern "C-unwind" fn(JS, c_int) -> c_uint = "js_touint32";
    js_toint16: unsafe extern "C-unwind" fn(JS, c_int) -> c_short = "js_toint16";
    js_touint16: unsafe extern "C-unwind" fn(JS, c_int) -> c_ushort = "js_touint16";

    /* ---- stack ---- */
    js_gettop: unsafe extern "C-unwind" fn(JS) -> c_int = "js_gettop";
    js_pop: unsafe extern "C-unwind" fn(JS, c_int) = "js_pop";
    js_rot: unsafe extern "C-unwind" fn(JS, c_int) = "js_rot";
    js_copy: unsafe extern "C-unwind" fn(JS, c_int) = "js_copy";
    js_remove: unsafe extern "C-unwind" fn(JS, c_int) = "js_remove";
    js_insert: unsafe extern "C-unwind" fn(JS, c_int) = "js_insert";
    js_replace: unsafe extern "C-unwind" fn(JS, c_int) = "js_replace";
    js_dup: unsafe extern "C-unwind" fn(JS) = "js_dup";
    js_dup2: unsafe extern "C-unwind" fn(JS) = "js_dup2";
    js_rot2: unsafe extern "C-unwind" fn(JS) = "js_rot2";
    js_rot3: unsafe extern "C-unwind" fn(JS) = "js_rot3";
    js_rot4: unsafe extern "C-unwind" fn(JS) = "js_rot4";
    js_rot2pop1: unsafe extern "C-unwind" fn(JS) = "js_rot2pop1";
    js_rot3pop2: unsafe extern "C-unwind" fn(JS) = "js_rot3pop2";

    /* ---- operators ---- */
    js_concat: unsafe extern "C-unwind" fn(JS) = "js_concat";
    js_compare: unsafe extern "C-unwind" fn(JS, *mut c_int) -> c_int = "js_compare";
    js_equal: unsafe extern "C-unwind" fn(JS) -> c_int = "js_equal";
    js_strictequal: unsafe extern "C-unwind" fn(JS) -> c_int = "js_strictequal";
    js_instanceof: unsafe extern "C-unwind" fn(JS) -> c_int = "js_instanceof";
    js_typeof: unsafe extern "C-unwind" fn(JS, c_int) -> *const c_char = "js_typeof";
    js_type: unsafe extern "C-unwind" fn(JS, c_int) -> c_int = "js_type";

    /* ---- repr ---- */
    js_repr: unsafe extern "C-unwind" fn(JS, c_int) = "js_repr";
    js_torepr: unsafe extern "C-unwind" fn(JS, c_int) -> *const c_char = "js_torepr";
    js_tryrepr: unsafe extern "C-unwind" fn(JS, c_int, *const c_char) -> *const c_char = "js_tryrepr";

    /* ---- lexer ---- */
    jsY_iswhite: unsafe extern "C-unwind" fn(c_int) -> c_int = "jsY_iswhite";
    jsY_isnewline: unsafe extern "C-unwind" fn(c_int) -> c_int = "jsY_isnewline";
    jsY_ishex: unsafe extern "C-unwind" fn(c_int) -> c_int = "jsY_ishex";
    jsY_tohex: unsafe extern "C-unwind" fn(c_int) -> c_int = "jsY_tohex";
    jsY_tokenstring: unsafe extern "C-unwind" fn(c_int) -> *const c_char = "jsY_tokenstring";
    jsY_findword: unsafe extern "C-unwind" fn(*const c_char, *const *const c_char, c_int) -> c_int = "jsY_findword";
    jsY_initlex: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) = "jsY_initlex";
    jsY_lex: unsafe extern "C-unwind" fn(JS) -> c_int = "jsY_lex";
    jsY_lexjson: unsafe extern "C-unwind" fn(JS) -> c_int = "jsY_lexjson";

    /* ---- parser / compiler ---- */
    jsP_parse: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) -> Ast = "jsP_parse";
    jsP_parsefunction: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char, *const c_char) -> Ast = "jsP_parsefunction";
    jsP_freeparse: unsafe extern "C-unwind" fn(JS) = "jsP_freeparse";
    jsC_compilescript: unsafe extern "C-unwind" fn(JS, Ast, c_int) -> Func = "jsC_compilescript";
    jsC_compilefunction: unsafe extern "C-unwind" fn(JS, Ast) -> Func = "jsC_compilefunction";
    js_newfunction: unsafe extern "C-unwind" fn(JS, Func, Env) = "js_newfunction";
    js_newscript: unsafe extern "C-unwind" fn(JS, Func, Env) = "js_newscript";
    js_newarguments: unsafe extern "C-unwind" fn(JS) = "js_newarguments";
    jsR_newenvironment: unsafe extern "C-unwind" fn(JS, Obj, Env) -> Env = "jsR_newenvironment";
    jsR_unflattenarray: unsafe extern "C-unwind" fn(JS, Obj) = "jsR_unflattenarray";

    /* ---- low-level value api ---- */
    js_tovalue: unsafe extern "C-unwind" fn(JS, c_int) -> *mut c_void = "js_tovalue";
    js_pushvalue: unsafe extern "C-unwind" fn(JS, JsValue) = "js_pushvalue";
    js_pushobject: unsafe extern "C-unwind" fn(JS, Obj) = "js_pushobject";
    js_toobject: unsafe extern "C-unwind" fn(JS, c_int) -> Obj = "js_toobject";
    js_toprimitive: unsafe extern "C-unwind" fn(JS, c_int, c_int) = "js_toprimitive";
    jsV_toboolean: unsafe extern "C-unwind" fn(JS, *mut c_void) -> c_int = "jsV_toboolean";
    jsV_tonumber: unsafe extern "C-unwind" fn(JS, *mut c_void) -> c_double = "jsV_tonumber";
    jsV_tointeger: unsafe extern "C-unwind" fn(JS, *mut c_void) -> c_double = "jsV_tointeger";
    jsV_tostring: unsafe extern "C-unwind" fn(JS, *mut c_void) -> *const c_char = "jsV_tostring";
    jsV_toobject: unsafe extern "C-unwind" fn(JS, *mut c_void) -> Obj = "jsV_toobject";
    jsV_toprimitive: unsafe extern "C-unwind" fn(JS, *mut c_void, c_int) = "jsV_toprimitive";
    jsV_newobject: unsafe extern "C-unwind" fn(JS, c_int, Obj) -> Obj = "jsV_newobject";
    jsV_newmemstring: unsafe extern "C-unwind" fn(JS, *const c_char, c_int) -> *mut c_void = "jsV_newmemstring";
    jsV_getownproperty: unsafe extern "C-unwind" fn(JS, Obj, *const c_char) -> Prop = "jsV_getownproperty";
    jsV_getproperty: unsafe extern "C-unwind" fn(JS, Obj, *const c_char) -> Prop = "jsV_getproperty";
    jsV_getpropertyx: unsafe extern "C-unwind" fn(JS, Obj, *const c_char, *mut c_int) -> Prop = "jsV_getpropertyx";
    jsV_setproperty: unsafe extern "C-unwind" fn(JS, Obj, *const c_char) -> Prop = "jsV_setproperty";
    jsV_delproperty: unsafe extern "C-unwind" fn(JS, Obj, *const c_char) = "jsV_delproperty";
    jsV_newiterator: unsafe extern "C-unwind" fn(JS, Obj, c_int) -> Obj = "jsV_newiterator";
    jsV_nextiterator: unsafe extern "C-unwind" fn(JS, Obj) -> *const c_char = "jsV_nextiterator";
    jsV_resizearray: unsafe extern "C-unwind" fn(JS, Obj, c_int) = "jsV_resizearray";
    js_toregexp: unsafe extern "C-unwind" fn(JS, c_int) -> *mut c_void = "js_toregexp";
    js_RegExp_prototype_exec: unsafe extern "C-unwind" fn(JS, *mut c_void, *const c_char) = "js_RegExp_prototype_exec";

    /* ---- stdout / buffer writers ---- */
    js_trap: unsafe extern "C-unwind" fn(JS, c_int) = "js_trap";
    js_putc: unsafe extern "C-unwind" fn(JS, *mut *mut c_void, c_int) = "js_putc";
    js_puts: unsafe extern "C-unwind" fn(JS, *mut *mut c_void, *const c_char) = "js_puts";
    js_putm: unsafe extern "C-unwind" fn(JS, *mut *mut c_void, *const c_char, *const c_char) = "js_putm";

    /* ---- builtins ---- */
    jsB_init: unsafe extern "C-unwind" fn(JS) = "jsB_init";
    jsB_initobject: unsafe extern "C-unwind" fn(JS) = "jsB_initobject";
    jsB_initarray: unsafe extern "C-unwind" fn(JS) = "jsB_initarray";
    jsB_initfunction: unsafe extern "C-unwind" fn(JS) = "jsB_initfunction";
    jsB_initboolean: unsafe extern "C-unwind" fn(JS) = "jsB_initboolean";
    jsB_initnumber: unsafe extern "C-unwind" fn(JS) = "jsB_initnumber";
    jsB_initstring: unsafe extern "C-unwind" fn(JS) = "jsB_initstring";
    jsB_initregexp: unsafe extern "C-unwind" fn(JS) = "jsB_initregexp";
    jsB_initerror: unsafe extern "C-unwind" fn(JS) = "jsB_initerror";
    jsB_initmath: unsafe extern "C-unwind" fn(JS) = "jsB_initmath";
    jsB_initjson: unsafe extern "C-unwind" fn(JS) = "jsB_initjson";
    jsB_initdate: unsafe extern "C-unwind" fn(JS) = "jsB_initdate";
    jsB_propf: unsafe extern "C-unwind" fn(JS, *const c_char, CFunction, c_int) = "jsB_propf";
    jsB_propn: unsafe extern "C-unwind" fn(JS, *const c_char, c_double) = "jsB_propn";
    jsB_props: unsafe extern "C-unwind" fn(JS, *const c_char, *const c_char) = "jsB_props";
}

fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <work>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so() -> PathBuf {
    let p = root().join("c_src/build/libmujs.so");
    assert!(p.exists(), "missing C .so at {} — build it first", p.display());
    p
}

pub fn rust_so() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libmujs.so");
        if p.exists() {
            return p;
        }
    }
    panic!("missing Rust libmujs.so under {}", base.display());
}

/// The pair of loaded libraries. `both()` returns a process-wide singleton.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn both() -> &'static Pair {
    PAIR.get_or_init(|| {
        // c_src/CMakeLists.txt does not link libm, so libmujs.so leaves ceil/floor/
        // fmod/... undefined. Make them globally visible before dlopen'ing it.
        // (We must not modify anything under c_src/.)
        use libloading::os::unix as unix_dl;
        let flags = unix_dl::RTLD_NOW | unix_dl::RTLD_GLOBAL;
        for name in ["libm.so.6", "libm.so"] {
            if let Ok(l) = unsafe { unix_dl::Library::open(Some(name), flags) } {
                std::mem::forget(l);
                break;
            }
        }
        Pair {
            c: Api::load(&c_so(), "C"),
            r: Api::load(&rust_so(), "RUST"),
        }
    })
}

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A `'static` C string.
///
/// Several entry points store the pointer **without copying** — `js_newcfunction`
/// / `js_newcfunctionx` / `js_newcconstructor` keep `obj->u.c.name`,
/// `js_newuserdata*` keeps `obj->u.user.tag`, `js_pushliteral` and `jsB_props`
/// keep the string itself (`jsvalue.c:493`, `jsbuiltin.c:26`). Passing a
/// temporary `CString` there would dangle, so those call sites use `sstr`.
pub fn sstr(s: &str) -> *const c_char {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static POOL: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);
    let mut g = POOL.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(HashMap::new);
    if let Some(p) = m.get(s) {
        return *p as *const c_char;
    }
    let leaked: &'static CStr = Box::leak(cs(s).into_boxed_c_str());
    let p = leaked.as_ptr();
    m.insert(s.to_string(), p as usize);
    p
}

/// Read a NUL-terminated C string; `None` for NULL.
pub unsafe fn opt_str(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

pub unsafe fn s(p: *const c_char) -> String {
    match unsafe { opt_str(p) } {
        None => "<NULL>".to_string(),
        Some(b) => String::from_utf8_lossy(&b).into_owned(),
    }
}

/// Bit-exact double comparison (NaN == NaN, -0.0 != 0.0).
pub fn same_f64(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// xorshift64* — deterministic, identical across runs and platforms.
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
    /// uniform in [0, n)
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Random f64 drawn from a mix of "interesting" distributions.
    pub fn f64(&mut self) -> f64 {
        match self.below(8) {
            0 => f64::from_bits(self.next_u64()),           // any bit pattern
            1 => self.i32() as f64,                        // integral
            2 => (self.i32() as f64) / 1000.0,               // small fraction
            3 => self.next_u64() as f64,                     // large integral
            4 => {
                let e = (self.below(64) as i32) - 32;
                (self.i32() as f64) * 2f64.powi(e)
            }
            5 => {
                const SPECIAL: [f64; 16] = [
                    0.0, -0.0, 1.0, -1.0, 0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
                    1e21, 1e-7, 1e20, f64::MAX, f64::MIN_POSITIVE, 4294967296.0, 2147483648.0,
                    -2147483648.0,
                ];
                SPECIAL[self.below(16) as usize]
            }
            6 => (self.next_u32() as f64) / (self.next_u32().max(1) as f64),
            _ => {
                let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
                let e = (self.below(40) as i32) - 20;
                m * 10f64.powi(e)
            }
        }
    }
    /// Random byte string, may contain any byte except NUL.
    pub fn cstr_bytes(&mut self, maxlen: usize) -> Vec<u8> {
        let n = self.below(maxlen as u64 + 1) as usize;
        (0..n)
            .map(|_| {
                let b = (self.next_u32() & 0xFF) as u8;
                if b == 0 { 1 } else { b }
            })
            .collect()
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u64) as usize]
    }
}

/* ================================================================== */
/* Protected-call driver                                               */
/* ================================================================== */
//
// Almost every `js_*` entry point throws by `longjmp` (C) or by unwinding
// (Rust). An uncaught throw calls `J->panic()` and then `abort()`, so a test
// may never call a throwing entry point unprotected.
//
// The driver therefore registers the test body as a `js_CFunction` and invokes
// it with `js_pcall`, exactly like a real embedder would. Both libraries then
// protect the body with their own mechanism:
//   * C   — `setjmp` inside `js_pcall`; `longjmp` skips our frame;
//   * Rust — a try frame; the panic unwinds through our `extern "C-unwind"`
//            callback frame.
// Either way `js_pcall` returns 1 and leaves the exception on the stack, so the
// two sides are compared on (return code, error message, emitted trace).

use std::cell::{Cell, RefCell};
use std::fmt::Write as _;

thread_local! {
    static OUT: RefCell<String> = RefCell::new(String::new());
    static SLOT: Cell<Option<(*const Api, TaskFn)>> = const { Cell::new(None) };
}

pub type TaskFn = fn(&Api, JS);

/// Append one line to the current side's observation trace.
pub fn emit(line: String) {
    OUT.with(|o| {
        let mut b = o.borrow_mut();
        b.push_str(&line);
        b.push('\n');
    });
}

fn out_take() -> String {
    OUT.with(|o| std::mem::take(&mut *o.borrow_mut()))
}

/// The library currently being exercised. Callbacks registered from a task must
/// call back into the *same* library, so they resolve their API through this.
pub fn active_api() -> &'static Api {
    let (api, _) = SLOT.get().expect("no task installed");
    unsafe { &*api }
}

/// `extern "C-unwind"` so the Rust library's panic-based throw may pass through.
unsafe extern "C-unwind" fn trampoline(J: JS) {
    let (api, f) = SLOT.get().expect("no task installed");
    f(unsafe { &*api }, J);
}

/// The report callback used by `js_dostring`; both libraries share it.
unsafe extern "C-unwind" fn report_cb(_J: JS, msg: *const c_char) {
    emit(format!("REPORT {}", unsafe { s(msg) }));
}

/// Allocation-counting allocator (CONFIGS row 24). Counts are *not* compared
/// between libraries (their internal structures differ); the point is to prove
/// the caller-supplied `js_Alloc` hook is honoured identically.
unsafe extern "C-unwind" fn counting_alloc(ctx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    unsafe extern "C" {
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        fn free(p: *mut c_void);
    }
    unsafe {
        if !ctx.is_null() {
            *(ctx as *mut u64) += 1;
        }
        if size == 0 {
            free(ptr);
            std::ptr::null_mut()
        } else {
            realloc(ptr, size as usize)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    pub flags: c_int,
    pub custom_alloc: bool,
    pub runlimit: c_int,
    pub memlimit: c_int,
    pub with_report: bool,
}
impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            flags: 0,
            custom_alloc: false,
            runlimit: 0,
            memlimit: 0,
            with_report: true,
        }
    }
}
impl Cfg {
    pub fn strict() -> Cfg {
        Cfg { flags: JS_STRICT, ..Cfg::default() }
    }
    pub fn alloc() -> Cfg {
        Cfg { custom_alloc: true, ..Cfg::default() }
    }
    pub fn limits(runlimit: c_int, memlimit: c_int) -> Cfg {
        Cfg { runlimit, memlimit, ..Cfg::default() }
    }
}

/// Run `task` on one library under `cfg`; returns the full observation string.
pub fn run_side(api: &Api, cfg: Cfg, task: TaskFn) -> String {
    let _ = out_take();
    let mut counter: u64 = 0;
    let J = unsafe {
        if cfg.custom_alloc {
            (api.js_newstate)(
                Some(counting_alloc),
                &mut counter as *mut u64 as *mut c_void,
                cfg.flags,
            )
        } else {
            (api.js_newstate)(None, std::ptr::null_mut(), cfg.flags)
        }
    };
    if J.is_null() {
        return "js_newstate -> NULL\n".to_string();
    }
    unsafe {
        if cfg.with_report {
            (api.js_setreport)(J, Some(report_cb));
        }
        if cfg.runlimit != 0 || cfg.memlimit != 0 {
            (api.js_setlimit)(J, cfg.runlimit, cfg.memlimit);
        }
        SLOT.set(Some((api as *const Api, task)));
        (api.js_newcfunction)(J, trampoline, sstr("__task"), 0);
        (api.js_pushnull)(J); // this
        let rc = (api.js_pcall)(J, 0);
        let sentinel = cs("<unrepresentable>");
        let msg = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
        (api.js_pop)(J, 1);
        let top = (api.js_gettop)(J);
        let mut trace = out_take();
        let _ = write!(trace, "RC {rc}\nTOP {top}\nRESULT {msg}\n");
        (api.js_freestate)(J);
        trace
    }
}

/// Run `task` on BOTH libraries and require byte-identical observations.
pub fn diff(label: &str, cfg: Cfg, task: TaskFn) {
    let p = both();
    let a = run_side(&p.c, cfg, task);
    let b = run_side(&p.r, cfg, task);
    if a != b {
        panic!("{}", render_diff(label, &format!("{cfg:?}"), &a, &b));
    }
}

/// Run `task` on both libraries under every state-level configuration.
pub fn diff_all_flags(label: &str, task: TaskFn) {
    diff(&format!("{label} [flags=0]"), Cfg::default(), task);
    diff(&format!("{label} [JS_STRICT]"), Cfg::strict(), task);
    diff(&format!("{label} [custom alloc]"), Cfg::alloc(), task);
}

pub fn render_diff(label: &str, cfg: &str, a: &str, b: &str) -> String {
    let mut m = format!("DIVERGENCE in {label}  ({cfg})\n");
    let al: Vec<&str> = a.lines().collect();
    let bl: Vec<&str> = b.lines().collect();
    let mut shown = 0;
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or("<missing>");
        let y = bl.get(i).copied().unwrap_or("<missing>");
        if x != y {
            let _ = write!(m, "line {}:\n  C    : {x}\n  RUST : {y}\n", i + 1);
            shown += 1;
            if shown >= 12 {
                let _ = write!(m, "... (more differences suppressed)\n");
                break;
            }
        }
    }
    m
}

/* ---------------- helpers usable from inside a task ---------------- */

/// Dump every observable fact about stack slot `idx`.
pub unsafe fn dump_slot(api: &Api, J: JS, idx: c_int) -> String {
    unsafe {
        let sentinel = cs("<throw>");
        let t = (api.js_type)(J, idx);
        let ty = s((api.js_typeof)(J, idx));
        let b = (api.js_tryboolean)(J, idx, -1);
        let n = (api.js_trynumber)(J, idx, -12345.5);
        let st = s((api.js_trystring)(J, idx, sentinel.as_ptr()));
        let i = (api.js_tryinteger)(J, idx, -999);
        let rp = s((api.js_tryrepr)(J, idx, sentinel.as_ptr()));
        format!(
            "type={t} typeof={ty} bool={b} numbits={:#x} int={i} str={st:?} repr={rp:?} \
             def={} undef={} null={} isbool={} isnum={} isstr={} prim={} obj={} arr={} re={} \
             coerce={} call={} err={} numo={} stro={} boolo={} dateo={}",
            n.to_bits(),
            (api.js_isdefined)(J, idx),
            (api.js_isundefined)(J, idx),
            (api.js_isnull)(J, idx),
            (api.js_isboolean)(J, idx),
            (api.js_isnumber)(J, idx),
            (api.js_isstring)(J, idx),
            (api.js_isprimitive)(J, idx),
            (api.js_isobject)(J, idx),
            (api.js_isarray)(J, idx),
            (api.js_isregexp)(J, idx),
            (api.js_iscoercible)(J, idx),
            (api.js_iscallable)(J, idx),
            (api.js_iserror)(J, idx),
            (api.js_isnumberobject)(J, idx),
            (api.js_isstringobject)(J, idx),
            (api.js_isbooleanobject)(J, idx),
            (api.js_isdateobject)(J, idx),
        )
    }
}

/// Restore the stack to an absolute depth recorded with `js_gettop`.
///
/// Needed because `js_pcall`/`js_pconstruct` unwind the stack to the *callee's*
/// base on failure, which is one slot below where a naive `js_pop(J, 1)` would
/// assume — popping blindly then walks off the frame and eventually re-enters
/// the trampoline.
pub unsafe fn restore_top(api: &Api, J: JS, base: c_int) {
    unsafe {
        let t = (api.js_gettop)(J);
        if t > base {
            (api.js_pop)(J, t - base);
        }
    }
}

/// Load + call `src`, emitting the (return code, result, exception) triple.
/// Uses only the protected entry points, so it is safe outside a task too.
pub unsafe fn eval(api: &Api, J: JS, tag: &str, src: &str) {
    unsafe {
        let f = cs("test.js");
        let c = match std::ffi::CString::new(src) {
            Ok(c) => c,
            Err(_) => return,
        };
        let sentinel = cs("<throw>");
        let rc = (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr());
        if rc != 0 {
            let m = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
            (api.js_pop)(J, 1);
            emit(format!("{tag}: LOAD rc=1 err={m:?}"));
            return;
        }
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        let m = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
        let r = s((api.js_tryrepr)(J, -1, sentinel.as_ptr()));
        let ty = (api.js_type)(J, -1);
        (api.js_pop)(J, 1);
        emit(format!("{tag}: rc={rc} type={ty} str={m:?} repr={r:?}"));
    }
}

/// Corpus of small JS programs shared by several test files.
pub fn js_snippets() -> Vec<&'static str> {
    vec![
        "1+1",
        "'a'+'b'",
        "var x = 1; x += 2; x",
        "(function(){return 42})()",
        "[1,2,3].join('-')",
        "[3,1,2].sort().join(',')",
        "[1,2,3].map(function(v){return v*2}).join(',')",
        "[1,2,3].reduce(function(a,b){return a+b})",
        "[].reduce(function(a,b){return a+b})",
        "'abc'.toUpperCase()",
        "'ABC'.toLowerCase()",
        "'a,b,c'.split(',').length",
        "'abc'.replace(/b/,'X')",
        "'aaa'.replace(/a/g,'b')",
        "/(\\d+)-(\\d+)/.exec('12-34').join('|')",
        "JSON.stringify({a:1,b:[1,2,{c:null}]})",
        "JSON.parse('{\"a\":[1,2,3]}').a[1]",
        "JSON.stringify([1,2],null,2)",
        "JSON.stringify({a:1},null,'\\t')",
        "JSON.stringify({a:1,b:2},['a'])",
        "JSON.parse('[1,2]',function(k,v){return v})+''",
        "JSON.stringify({toJSON:function(){return 5}})",
        "JSON.stringify(undefined)+''",
        "JSON.stringify([undefined,function(){}])",
        "Math.floor(3.7)+Math.ceil(3.2)",
        "Math.max(1,2,3)+Math.min(1,2,3)",
        "Math.max()+','+Math.min()",
        "Math.pow(2,10)",
        "Math.round(-0.5)+','+Math.round(0.5)+','+Math.round(2.5)",
        "Math.abs(-3)+Math.sign===undefined",
        "(123.456).toFixed(2)",
        "(1e21).toFixed(2)",
        "(0.000001).toExponential(3)",
        "(123.456).toPrecision(4)",
        "(255).toString(16)",
        "(255).toString(2)",
        "(-255).toString(36)",
        "parseInt('0x1f')",
        "parseInt('08')",
        "parseInt('10',2)",
        "parseFloat('3.14abc')",
        "String(NaN)+String(Infinity)+String(-0)",
        "typeof undefined + typeof null + typeof 1 + typeof 'a' + typeof {} + typeof function(){}",
        "var o = {}; o.a = 1; Object.keys(o).join()",
        "Object.getOwnPropertyNames([1,2]).join(',')",
        "var a = []; a[5]=1; a.length",
        "var a = [1,2,3]; delete a[1]; a.join(',')",
        "var s = ''; for (var i=0;i<5;i++) s+=i; s",
        "var s=''; for (var k in {a:1,b:2}) s+=k; s",
        "try { null.x } catch (e) { e.name + ': ' + e.message }",
        "try { undefinedVariable } catch (e) { e.name }",
        "try { (1)() } catch (e) { e.name }",
        "try { throw 'x' } catch (e) { e }",
        "try { throw {a:1} } catch (e) { e.a } finally { }",
        "(function f(n){return n<2?1:n*f(n-1)})(10)",
        "new Date(0).toISOString()",
        "new Date(1234567890123).getUTCFullYear()",
        "Date.UTC(2000,0,1)",
        "new Date('2001-02-03T04:05:06Z').getTime()",
        "new Date(NaN).toString()",
        "encodeURIComponent('a b/c?d=e&f')",
        "decodeURIComponent('a%20b')",
        "escape('a b\\u00e9')",
        "unescape('%u00e9')",
        "'\\u00e9\\u20ac'.length",
        "'\\u00e9'.charCodeAt(0)",
        "String.fromCharCode(233,8364)",
        "'abc'.charAt(1)+'abc'.charAt(9)",
        "'abcdef'.substring(1,3)+'abcdef'.slice(-2)",
        "[1,[2,[3,[4]]]].toString()",
        "var a=[1,2,3]; a.length=1; a.join(',')",
        "Array.prototype.concat.call([1],[2],3)+''",
        "(function(){return arguments.length})(1,2,3)",
        "(function(a,b){return a+b}).length",
        "new (function(){this.x=1})().x",
        "var o={get x(){return 7}}; o.x",
        "var o={set x(v){this.y=v}}; o.x=3; o.y",
        "[1,2,3].indexOf(2)+[1,2,3].lastIndexOf(3)",
        "Boolean('')+''+Boolean('a')",
        "null == undefined",
        "NaN != NaN",
        "0 == '0'",
        "0 === '0'",
        "'10' > '9'",
        "10 > 9",
        "[] + {}",
        "({}) + ''",
        "1/0 + ' ' + -1/0 + ' ' + 0/0",
        "(-0).toString() + (1/-0)",
        "void 0",
        "with ({a:5}) { a }",
        "switch(2){case 1: 'a'; break; case 2: 'b'; break; default: 'c'}",
        "do { var i = 1 } while (false); i",
        "eval('1+2')",
        "(function(){ return eval('var q=9; q') })()",
        "new Function('a','return a*2')(21)",
        "RegExp('a+').test('caaat')",
        "/a/.source",
        "'aaa'.match(/a/g).length",
        "'a1b2'.split(/\\d/).join('|')",
        "'abc'.search(/b/)",
        "/(a)(b)?/.exec('a')+''",
        "var re=/a/g; re.exec('aa').index + ',' + re.lastIndex",
        "Object.defineProperty({}, 'x', {value:1}).x",
        "var o={}; Object.defineProperty(o,'x',{get:function(){return 3}}); o.x",
        "JSON.stringify(Object.getOwnPropertyDescriptor({x:1},'x'))",
        "Object.keys(Object.freeze({a:1})).length",
        "var o=Object.seal({a:1}); o.b=2; Object.keys(o).join()",
        "Object.create(null) instanceof Object",
        "Object.getPrototypeOf([])===Array.prototype",
        "[1,2,3].every(function(v){return v>0})",
        "[1,2,3].some(function(v){return v>2})",
        "[1,2,3].filter(function(v){return v%2}).join()",
        "[1,2,3].forEach(function(v){}) === undefined",
        "[1,2,3].reduceRight(function(a,b){return a+''+b})",
        "[1,2,3].slice(1).join()+[1,2,3].splice(1,1).join()",
        "[1,2,3].reverse().join()",
        "[1,2,3].concat([4]).join()",
        "[3,20,100].sort().join()",
        "[3,20,100].sort(function(a,b){return a-b}).join()",
        "'  x  '.trim()+'|'",
        "'abc'.indexOf('b')+'abc'.lastIndexOf('c')",
        "'abc'.concat('d','e')",
        "'abc'.split('').join('.')",
        "'a-b-c'.split('-',2).join()",
        "'abc'.localeCompare('abd')",
        "'\\ud83d\\ude00'.length",
    ]
}

/* ================================================================== */
/* Subprocess driver — for configurations where the C library aborts    */
/* ================================================================== */
//
// `js_throw()` with `trytop == 0` calls `J->panic()` and then `abort()`
// (`jsrun.c:1479-1481`). Some configurations reach that before any protected
// frame exists — e.g. a `memlimit` so small that `js_newcfunction` itself runs
// out of memory. Such a case cannot be asserted in-process, so the two
// libraries are compared in *child processes*: same exit status, same signal,
// same trace on stdout.
//
// A test file opts in by adding
//     #[test] fn worker() { common::maybe_worker(my_task); }
// and calling `diff_subproc("worker", case, cfg)` from its real tests.

thread_local! {
    static CASE: RefCell<String> = RefCell::new(String::new());
}

pub fn case() -> String {
    CASE.with(|c| c.borrow().clone())
}
fn set_case(v: &str) {
    CASE.with(|c| *c.borrow_mut() = v.to_string());
}

/// Called from the `worker` test. Returns immediately in a normal test run.
pub fn maybe_worker(task: TaskFn) {
    let side = match std::env::var("MUJS_SIDE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let case = std::env::var("MUJS_CASE").unwrap_or_default();
    let cfg = Cfg {
        flags: std::env::var("MUJS_FLAGS").unwrap_or_default().parse().unwrap_or(0),
        custom_alloc: std::env::var("MUJS_ALLOC").as_deref() == Ok("1"),
        runlimit: std::env::var("MUJS_RUNLIMIT").unwrap_or_default().parse().unwrap_or(0),
        memlimit: std::env::var("MUJS_MEMLIMIT").unwrap_or_default().parse().unwrap_or(0),
        with_report: true,
    };
    set_case(&case);
    let p = both();
    let api = if side == "c" { &p.c } else { &p.r };
    let trace = run_side(api, cfg, task);
    print!("{trace}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// Strip libtest's `test <name> ... ` progress prefix from a captured line.
pub fn strip_libtest_prefix(l: &str) -> &str {
    if let Some(rest) = l.strip_prefix("test ") {
        if let Some(k) = rest.find(" ... ") {
            return &rest[k + 5..];
        }
        if rest.ends_with(" ... ok") || rest.ends_with(" ... FAILED") {
            return "";
        }
    }
    l
}

fn spawn_side(worker_test: &str, side: &str, case_: &str, cfg: Cfg) -> String {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([worker_test, "--exact", "--nocapture", "--test-threads=1"])
        .env("MUJS_SIDE", side)
        .env("MUJS_CASE", case_)
        .env("MUJS_FLAGS", cfg.flags.to_string())
        .env("MUJS_ALLOC", if cfg.custom_alloc { "1" } else { "0" })
        .env("MUJS_RUNLIMIT", cfg.runlimit.to_string())
        .env("MUJS_MEMLIMIT", cfg.memlimit.to_string())
        .env("TZ", "UTC")
        .output()
        .expect("spawn worker");
    // Keep only the lines the worker itself printed. libtest emits
    // "test <name> ... " with no trailing newline, so the worker's first line is
    // glued to it; strip that prefix instead of dropping the line.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body: String = stdout
        .lines()
        .map(strip_libtest_prefix)
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("running ")
                && !l.starts_with("test result")
                && *l != "ok"
                && !l.trim_start().starts_with("Finished")
                && !l.trim_start().starts_with("Running")
        })
        .map(|l| format!("{l}\n"))
        .collect();
    use std::os::unix::process::ExitStatusExt;
    format!(
        "exit_code={:?} signal={:?}\n{}",
        out.status.code(),
        out.status.signal(),
        body
    )
}

/// Compare the two libraries in child processes (status + signal + trace).
pub fn diff_subproc(worker_test: &str, case_: &str, cfg: Cfg) {
    let a = spawn_side(worker_test, "c", case_, cfg);
    let b = spawn_side(worker_test, "rust", case_, cfg);
    if a != b {
        panic!("{}", render_diff(case_, &format!("{cfg:?}"), &a, &b));
    }
}

/// Same as `diff_subproc` but also returns the (identical) child output, so a
/// coverage gate can fold in messages that only a crashing child can produce.
pub fn diff_subproc_output(worker_test: &str, case_: &str, cfg: Cfg) -> String {
    let a = spawn_side(worker_test, "c", case_, cfg);
    let b = spawn_side(worker_test, "rust", case_, cfg);
    if a != b {
        panic!("{}", render_diff(case_, &format!("{cfg:?}"), &a, &b));
    }
    a
}

/* ---------------- stdout capture (for js_trap / jsS_dumpstrings) ---- */

pub static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

/// Run `f` with the process' fd 1 redirected to a temp file; return what was
/// written. Used for the entry points that print straight to `stdout`.
pub fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "mujs-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("temp file");
    use std::os::unix::io::AsRawFd;
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        dup2(file.as_raw_fd(), 1);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);
    let out = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}
