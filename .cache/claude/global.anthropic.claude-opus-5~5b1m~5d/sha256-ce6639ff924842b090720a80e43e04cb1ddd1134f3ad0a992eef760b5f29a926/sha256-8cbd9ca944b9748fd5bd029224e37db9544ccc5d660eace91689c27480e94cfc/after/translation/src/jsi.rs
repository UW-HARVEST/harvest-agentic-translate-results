//! Translation of jsi.h + mujs.h: shared types, constants, libc bindings, helpers.

pub use core::ffi::{c_char, c_double, c_int, c_long, c_short, c_uint, c_ulong, c_ushort, c_void};
pub use core::ptr::{addr_of, addr_of_mut, null, null_mut};

/* ---------------------------------------------------------------- libc ---- */

extern "C" {
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn calloc(n: usize, m: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;
    pub fn exit(code: c_int) -> !;

    pub fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;

    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strcat(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char;
    pub fn strspn(s: *const c_char, a: *const c_char) -> usize;
    pub fn strtod(s: *const c_char, e: *mut *mut c_char) -> c_double;

    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut c_void, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: *mut c_void) -> c_int;
    pub fn putchar(c: c_int) -> c_int;
    pub fn fputs(s: *const c_char, f: *mut c_void) -> c_int;
    pub fn fputc(c: c_int, f: *mut c_void) -> c_int;

    pub fn time(t: *mut c_long) -> c_long;
    pub fn clock() -> c_long;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    pub fn localtime(t: *const c_long) -> *mut tm;
    pub fn gmtime(t: *const c_long) -> *mut tm;
    pub fn mktime(t: *mut tm) -> c_long;

    pub fn _setjmp(env: *mut c_void) -> c_int;
    pub fn longjmp(env: *mut c_void, val: c_int) -> !;

    pub static mut stdout: *mut c_void;
    pub static mut stderr: *mut c_void;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

/* math: use Rust intrinsics where they map 1:1 to libm */
extern "C" {
    pub fn floor(x: f64) -> f64;
    pub fn ceil(x: f64) -> f64;
    pub fn fmod(x: f64, y: f64) -> f64;
    pub fn pow(x: f64, y: f64) -> f64;
    pub fn sqrt(x: f64) -> f64;
    pub fn exp(x: f64) -> f64;
    pub fn log(x: f64) -> f64;
    pub fn sin(x: f64) -> f64;
    pub fn cos(x: f64) -> f64;
    pub fn tan(x: f64) -> f64;
    pub fn asin(x: f64) -> f64;
    pub fn acos(x: f64) -> f64;
    pub fn atan(x: f64) -> f64;
    pub fn atan2(y: f64, x: f64) -> f64;
    pub fn fabs(x: f64) -> f64;
}

#[inline]
pub fn isnan(x: f64) -> bool {
    x.is_nan()
}
#[inline]
pub fn isinf(x: f64) -> bool {
    x.is_infinite()
}
#[inline]
pub fn isfinite(x: f64) -> bool {
    x.is_finite()
}
#[inline]
pub fn signbit(x: f64) -> bool {
    x.is_sign_negative()
}

pub const INFINITY: f64 = f64::INFINITY;
pub const NAN: f64 = f64::NAN;
pub const INT_MAX: c_int = c_int::MAX;
pub const INT_MIN: c_int = c_int::MIN;
pub const DBL_MAX: f64 = f64::MAX;
pub const DBL_MIN: f64 = f64::MIN_POSITIVE;
pub const DBL_EPSILON: f64 = f64::EPSILON;

/// `cs!("text")` yields a NUL terminated `*const c_char` pointing to static memory.
#[macro_export]
macro_rules! cs {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const $crate::jsi::c_char
    };
}

/* ------------------------------------------------------------- limits ---- */

pub const JS_STACKSIZE: c_int = 4096;
pub const JS_ENVLIMIT: usize = 1024;
pub const JS_TRYLIMIT: c_int = 64;
pub const JS_ARRAYLIMIT: c_int = 1 << 26;
pub const JS_GCFACTOR: f64 = 5.0;
pub const JS_ASTLIMIT: c_int = 400;
pub const JS_STRLIMIT: usize = 1 << 28;

pub const JS_VERSION_MAJOR: c_int = 1;
pub const JS_VERSION_MINOR: c_int = 3;
pub const JS_VERSION_PATCH: c_int = 8;

/* state constructor flags */
pub const JS_STRICT: c_int = 1;
/* regexp flags */
pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;
/* property attributes */
pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;
/* js_type() */
pub const JS_ISUNDEFINED: c_int = 0;
pub const JS_ISNULL: c_int = 1;
pub const JS_ISBOOLEAN: c_int = 2;
pub const JS_ISNUMBER: c_int = 3;
pub const JS_ISSTRING: c_int = 4;
pub const JS_ISFUNCTION: c_int = 5;
pub const JS_ISOBJECT: c_int = 6;

/* ToPrimitive hints */
pub const JS_HNONE: c_int = 0;
pub const JS_HNUMBER: c_int = 1;
pub const JS_HSTRING: c_int = 2;

/* enum js_Type */
pub const JS_TSHRSTR: c_int = 0;
pub const JS_TUNDEFINED: c_int = 1;
pub const JS_TNULL: c_int = 2;
pub const JS_TBOOLEAN: c_int = 3;
pub const JS_TNUMBER: c_int = 4;
pub const JS_TLITSTR: c_int = 5;
pub const JS_TMEMSTR: c_int = 6;
pub const JS_TOBJECT: c_int = 7;

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

/* --------------------------------------------------------- regexp.h ------- */

pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;
pub const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub_pair {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [Resub_pair; REG_MAXSUB],
}

const _: () = assert!(core::mem::size_of::<Resub>() == 264);

/* -------------------------------------------------------- public types ---- */

pub type js_Alloc = Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type js_Panic = Option<unsafe extern "C" fn(*mut js_State)>;
pub type js_CFunction = Option<unsafe extern "C" fn(*mut js_State)>;
pub type js_Finalize = Option<unsafe extern "C" fn(*mut js_State, *mut c_void)>;
pub type js_HasProperty =
    Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Put = Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Delete =
    Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Report = Option<unsafe extern "C" fn(*mut js_State, *const c_char)>;

pub type js_Instruction = c_ushort;
pub type Rune = c_int;

/* ------------------------------------------------------------- values ---- */

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
    pub memstr: *mut js_String,
    pub object: *mut js_Object,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_Value {
    pub t: js_Value_t,
    pub u: js_Value_u,
}

impl js_Value {
    pub const fn zero() -> js_Value {
        js_Value {
            u: js_Value_u { shrstr: [0; 16] },
        }
    }
    pub const fn undefined() -> js_Value {
        js_Value {
            t: js_Value_t {
                pad: [0; 15],
                type_: JS_TUNDEFINED as c_char,
            },
        }
    }
}

#[repr(C)]
pub struct js_String {
    pub gcnext: *mut js_String,
    pub gcmark: c_char,
    pub p: [c_char; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Regexp {
    pub prog: *mut c_void,
    pub source: *mut c_char,
    pub flags: c_ushort,
    pub last: c_ushort,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_s {
    pub length: c_int,
    pub string: *mut c_char,
    pub shrstr: [c_char; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_a {
    pub length: c_int,
    pub simple: c_int,
    pub flat_length: c_int,
    pub flat_capacity: c_int,
    pub array: *mut js_Value,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_f {
    pub function: *mut js_Function,
    pub scope: *mut js_Environment,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_c {
    pub name: *const c_char,
    pub function: js_CFunction,
    pub constructor: js_CFunction,
    pub length: c_int,
    pub data: *mut c_void,
    pub finalize: js_Finalize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_iter {
    pub target: *mut js_Object,
    pub i: c_int,
    pub n: c_int,
    pub head: *mut js_Iterator,
    pub current: *mut js_Iterator,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Object_user {
    pub tag: *const c_char,
    pub data: *mut c_void,
    pub has: js_HasProperty,
    pub put: js_Put,
    pub delete: js_Delete,
    pub finalize: js_Finalize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_Object_u {
    pub boolean: c_int,
    pub number: f64,
    pub s: js_Object_s,
    pub a: js_Object_a,
    pub f: js_Object_f,
    pub c: js_Object_c,
    pub r: js_Regexp,
    pub iter: js_Object_iter,
    pub user: js_Object_user,
}

#[repr(C)]
pub struct js_Object {
    pub type_: c_int,
    pub extensible: c_int,
    pub properties: *mut js_Property,
    pub count: c_int,
    pub prototype: *mut js_Object,
    pub u: js_Object_u,
    pub gcnext: *mut js_Object,
    pub gcroot: *mut js_Object,
    pub gcmark: c_int,
}

#[repr(C)]
pub struct js_Property {
    pub left: *mut js_Property,
    pub right: *mut js_Property,
    pub level: c_int,
    pub atts: c_int,
    pub value: js_Value,
    pub getter: *mut js_Object,
    pub setter: *mut js_Object,
    pub name: [c_char; 1],
}

#[repr(C)]
pub struct js_Iterator {
    pub next: *mut js_Iterator,
    pub name: [c_char; 1],
}

#[repr(C)]
pub struct js_Environment {
    pub outer: *mut js_Environment,
    pub variables: *mut js_Object,
    pub gcnext: *mut js_Environment,
    pub gcmark: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_StackTrace {
    pub name: *const c_char,
    pub file: *const c_char,
    pub line: c_int,
    pub stack: c_int,
}

#[repr(C)]
pub struct js_Jumpbuf {
    pub buf: [u64; 25], /* jmp_buf: 200 bytes on x86-64 glibc */
    pub E: *mut js_Environment,
    pub envtop: c_int,
    pub tracetop: c_int,
    pub top: c_int,
    pub bot: c_int,
    pub strict: c_int,
    pub pc: *mut js_Instruction,
}

#[repr(C)]
pub struct js_Buffer {
    pub n: c_int,
    pub m: c_int,
    pub s: [c_char; 64],
}

#[repr(C)]
pub struct js_Function {
    pub name: *const c_char,
    pub script: c_int,
    pub lightweight: c_int,
    pub strict: c_int,
    pub arguments: c_int,
    pub numparams: c_int,

    pub code: *mut js_Instruction,
    pub codecap: c_int,
    pub codelen: c_int,

    pub funtab: *mut *mut js_Function,
    pub funcap: c_int,
    pub funlen: c_int,

    pub vartab: *mut *const c_char,
    pub varcap: c_int,
    pub varlen: c_int,

    pub filename: *const c_char,
    pub line: c_int,
    pub lastline: c_int,

    pub gcnext: *mut js_Function,
    pub gcmark: c_int,
}

#[repr(C)]
pub struct js_JumpList {
    pub type_: c_int,
    pub inst: c_int,
    pub next: *mut js_JumpList,
}

#[repr(C)]
pub struct js_Ast {
    pub type_: c_int,
    pub line: c_int,
    pub parent: *mut js_Ast,
    pub a: *mut js_Ast,
    pub b: *mut js_Ast,
    pub c: *mut js_Ast,
    pub d: *mut js_Ast,
    pub number: f64,
    pub string: *const c_char,
    pub jumps: *mut js_JumpList,
    pub casejump: c_int,
    pub gcnext: *mut js_Ast,
}

#[repr(C)]
pub struct js_StringNode {
    pub left: *mut js_StringNode,
    pub right: *mut js_StringNode,
    pub level: c_int,
    pub string: [c_char; 1],
}

#[repr(C)]
pub struct js_State_lexbuf {
    pub text: *mut c_char,
    pub len: c_int,
    pub cap: c_int,
}

#[repr(C)]
pub struct js_State {
    pub actx: *mut c_void,
    pub uctx: *mut c_void,
    pub alloc: js_Alloc,
    pub report: js_Report,
    pub panic: js_Panic,

    pub strings: *mut js_StringNode,

    pub default_strict: c_int,
    pub strict: c_int,

    /* parser input source */
    pub filename: *const c_char,
    pub source: *const c_char,
    pub line: c_int,

    /* lexer state */
    pub lexbuf: js_State_lexbuf,
    pub lexline: c_int,
    pub lexchar: c_int,
    pub lasttoken: c_int,
    pub newline: c_int,

    /* parser state */
    pub astdepth: c_int,
    pub lookahead: c_int,
    pub text: *const c_char,
    pub number: f64,
    pub gcast: *mut js_Ast,

    /* runtime environment */
    pub Object_prototype: *mut js_Object,
    pub Array_prototype: *mut js_Object,
    pub Function_prototype: *mut js_Object,
    pub Boolean_prototype: *mut js_Object,
    pub Number_prototype: *mut js_Object,
    pub String_prototype: *mut js_Object,
    pub RegExp_prototype: *mut js_Object,
    pub Date_prototype: *mut js_Object,

    pub Error_prototype: *mut js_Object,
    pub EvalError_prototype: *mut js_Object,
    pub RangeError_prototype: *mut js_Object,
    pub ReferenceError_prototype: *mut js_Object,
    pub SyntaxError_prototype: *mut js_Object,
    pub TypeError_prototype: *mut js_Object,
    pub URIError_prototype: *mut js_Object,

    pub seed: c_uint,

    pub scratch: [c_char; 12],

    pub nextref: c_int,
    pub R: *mut js_Object,
    pub G: *mut js_Object,
    pub E: *mut js_Environment,
    pub GE: *mut js_Environment,

    /* execution stack */
    pub top: c_int,
    pub bot: c_int,
    pub stack: *mut js_Value,

    /* garbage collector list */
    pub gcmark: c_int,
    pub gccounter: c_uint,
    pub gcthresh: c_uint,
    pub gcenv: *mut js_Environment,
    pub gcfun: *mut js_Function,
    pub gcobj: *mut js_Object,
    pub gcstr: *mut js_String,

    pub gcroot: *mut js_Object,

    pub runlimit: c_int,
    pub memlimit: c_int,

    pub envtop: c_int,
    pub envstack: [*mut js_Environment; JS_ENVLIMIT],

    pub tracetop: c_int,
    pub trace: [js_StackTrace; JS_ENVLIMIT],

    pub trytop: c_int,
    pub trybuf: [js_Jumpbuf; JS_TRYLIMIT as usize],
}

/* Layout assertions: must match the C library exactly. */
const _: () = assert!(core::mem::size_of::<js_Value>() == 16);
const _: () = assert!(core::mem::size_of::<js_Object>() == 104);
const _: () = assert!(core::mem::size_of::<js_Property>() == 64);
const _: () = assert!(core::mem::size_of::<js_String>() == 16);
const _: () = assert!(core::mem::size_of::<js_Iterator>() == 16);
const _: () = assert!(core::mem::size_of::<js_Environment>() == 32);
const _: () = assert!(core::mem::size_of::<js_Jumpbuf>() == 240);
const _: () = assert!(core::mem::size_of::<js_Buffer>() == 72);
const _: () = assert!(core::mem::size_of::<js_Function>() == 112);
const _: () = assert!(core::mem::size_of::<js_Regexp>() == 24);
const _: () = assert!(core::mem::size_of::<js_Ast>() == 88);
const _: () = assert!(core::mem::size_of::<js_StackTrace>() == 24);
const _: () = assert!(core::mem::size_of::<js_StringNode>() == 24);
const _: () = assert!(core::mem::size_of::<js_State>() == 48552);

/* --------------------------------------------------------- offsetofs ---- */

pub const OFF_VALUE_TYPE: c_int = 15; /* soffsetof(js_Value, t.type) */
pub const OFF_STRING_P: usize = 9; /* soffsetof(js_String, p) */
pub const OFF_BUFFER_S: c_int = 8; /* soffsetof(js_Buffer, s) */
pub const OFF_PROPERTY_NAME: usize = 56;
pub const OFF_ITERATOR_NAME: usize = 8;
pub const OFF_STRINGNODE_STRING: usize = 20;

/* ---------------------------------------------------- value accessors ---- */

#[inline(always)]
pub unsafe fn vtype(v: *const js_Value) -> c_int {
    (*v).t.type_ as c_int
}

#[inline(always)]
pub unsafe fn setvtype(v: *mut js_Value, t: c_int) {
    (*v).t.type_ = t as c_char;
}

/// JSV_ISSTRING(v)
#[inline(always)]
pub unsafe fn jsv_isstring(v: *const js_Value) -> bool {
    let t = vtype(v);
    t == JS_TSHRSTR || t == JS_TMEMSTR || t == JS_TLITSTR
}

/// JSV_TOSTRING(v)
#[inline(always)]
pub unsafe fn jsv_tostring_raw(v: *const js_Value) -> *const c_char {
    match vtype(v) {
        JS_TSHRSTR => addr_of!((*v).u.shrstr) as *const c_char,
        JS_TLITSTR => (*v).u.litstr,
        JS_TMEMSTR => addr_of!((*(*v).u.memstr).p) as *const c_char,
        _ => cs!(""),
    }
}

/// address of the `p` array of a js_String
#[inline(always)]
pub unsafe fn strp(s: *mut js_String) -> *mut c_char {
    addr_of_mut!((*s).p) as *mut c_char
}

/// address of the `shrstr` array of a js_Value
#[inline(always)]
pub unsafe fn shrstrp(v: *mut js_Value) -> *mut c_char {
    addr_of_mut!((*v).u.shrstr) as *mut c_char
}

/// address of the `name` array of a js_Property
#[inline(always)]
pub unsafe fn propname(p: *mut js_Property) -> *mut c_char {
    addr_of_mut!((*p).name) as *mut c_char
}

/// address of the `name` array of a js_Iterator
#[inline(always)]
pub unsafe fn itername(p: *mut js_Iterator) -> *mut c_char {
    addr_of_mut!((*p).name) as *mut c_char
}

/// address of the `string` array of a js_StringNode
#[inline(always)]
pub unsafe fn nodestring(p: *mut js_StringNode) -> *mut c_char {
    addr_of_mut!((*p).string) as *mut c_char
}

/* -------------------------------------------------------- error macros ---- */

/// Format a message like C's `vsnprintf(buf, 256, fmt, ...)` and raise it.
///
/// Usage: `js_throw_error!(js_typeerror_str, J, "'%s' is read-only", name)`
#[macro_export]
macro_rules! js_throw_error {
    ($f:path, $J:expr, $fmt:expr) => {{
        let mut __buf: [$crate::jsi::c_char; 256] = [0; 256];
        $crate::jsi::snprintf(__buf.as_mut_ptr(), 256, $crate::cs!($fmt));
        $f($J, __buf.as_ptr())
    }};
    ($f:path, $J:expr, $fmt:expr $(, $a:expr)+) => {{
        let mut __buf: [$crate::jsi::c_char; 256] = [0; 256];
        $crate::jsi::snprintf(__buf.as_mut_ptr(), 256, $crate::cs!($fmt) $(, $a)+);
        $f($J, __buf.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_error_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_evalerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_evalerror_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_rangeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_rangeerror_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_referenceerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_referenceerror_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_syntaxerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_syntaxerror_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_typeerror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_typeerror_str, $J, $fmt $(, $a)*)
    };
}
#[macro_export]
macro_rules! js_urierror {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {
        $crate::js_throw_error!($crate::jserror::js_urierror_str, $J, $fmt $(, $a)*)
    };
}

/// Volatile read of a local variable (mirrors C's `volatile` qualifier which
/// the original code uses to keep values alive across `longjmp`).
#[macro_export]
macro_rules! vol {
    ($x:expr) => {
        core::ptr::read_volatile(core::ptr::addr_of!($x))
    };
}

/// Volatile write of a local variable.
#[macro_export]
macro_rules! setvol {
    ($x:expr, $v:expr) => {
        core::ptr::write_volatile(core::ptr::addr_of_mut!($x), $v)
    };
}

/// `js_try(J)` -- returns nonzero when an exception jumped back here.
///
/// MUST be invoked directly in the frame that should be resumed.
#[macro_export]
macro_rules! js_try {
    ($J:expr) => {
        $crate::jsi::_setjmp($crate::jsrun::js_savetry($J) as *mut $crate::jsi::c_void)
    };
}

#[macro_export]
macro_rules! js_trypc {
    ($J:expr, $pc:expr) => {
        $crate::jsi::_setjmp($crate::jsrun::js_savetrypc($J, $pc) as *mut $crate::jsi::c_void)
    };
}
