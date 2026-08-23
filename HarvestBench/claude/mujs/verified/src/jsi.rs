//! Core types, constants and C library bindings shared by all modules.
//! Direct transliteration of mujs.h / jsi.h / utf.h / regexp.h.

pub use std::ffi::{c_char, c_double, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort,
                   c_void};
pub use std::ptr::{null, null_mut};

/* ---------------------------------------------------------------- limits */

pub const JS_STACKSIZE: c_int = 4096;
pub const JS_ENVLIMIT: usize = 1024;
pub const JS_TRYLIMIT: usize = 64;
pub const JS_ARRAYLIMIT: c_int = 1 << 26;
pub const JS_GCFACTOR: f64 = 5.0;
pub const JS_ASTLIMIT: c_int = 400;
pub const JS_STRLIMIT: c_int = 1 << 28;

pub const JS_VERSION_MAJOR: c_int = 1;
pub const JS_VERSION_MINOR: c_int = 3;
pub const JS_VERSION_PATCH: c_int = 8;

pub type js_Instruction = c_ushort;

/* State constructor flags */
pub const JS_STRICT: c_int = 1;

/* RegExp flags */
pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;

/* Property attribute flags */
pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;

/* enum for js_type() */
pub const JS_ISUNDEFINED: c_int = 0;
pub const JS_ISNULL: c_int = 1;
pub const JS_ISBOOLEAN: c_int = 2;
pub const JS_ISNUMBER: c_int = 3;
pub const JS_ISSTRING: c_int = 4;
pub const JS_ISFUNCTION: c_int = 5;
pub const JS_ISOBJECT: c_int = 6;

/* Hint to ToPrimitive() */
pub const JS_HNONE: c_int = 0;
pub const JS_HNUMBER: c_int = 1;
pub const JS_HSTRING: c_int = 2;

/* enum js_Type -- stored in js_Value.t.type (a char) */
pub const JS_TSHRSTR: c_char = 0;
pub const JS_TUNDEFINED: c_char = 1;
pub const JS_TNULL: c_char = 2;
pub const JS_TBOOLEAN: c_char = 3;
pub const JS_TNUMBER: c_char = 4;
pub const JS_TLITSTR: c_char = 5;
pub const JS_TMEMSTR: c_char = 6;
pub const JS_TOBJECT: c_char = 7;

/* enum js_Class -- stored in js_Object.type (an int) */
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

/* regexp.h flags */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;
pub const REG_MAXSUB: usize = 16;

/* utf.h */
pub type Rune = c_int;
pub const UTFmax: c_int = 4;
pub const Runesync: c_int = 0x80;
pub const Runeself: c_int = 0x80;
pub const Runeerror: c_int = 0xFFFD;
pub const Runemax: c_int = 0x10FFFF;

/* limits.h */
pub const INT_MIN: c_int = c_int::MIN;
pub const INT_MAX: c_int = c_int::MAX;
pub const UINT_MAX: c_uint = c_uint::MAX;
pub const CHAR_MAX: c_int = 127;
pub const DBL_MAX: f64 = f64::MAX;
pub const DBL_MIN: f64 = f64::MIN_POSITIVE;
pub const DBL_EPSILON: f64 = f64::EPSILON;
pub const INFINITY: f64 = f64::INFINITY;
pub const NAN: f64 = f64::NAN;

/* ---------------------------------------------------------------- callbacks */

pub type js_AllocFn = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
pub type js_Alloc = Option<js_AllocFn>;
pub type js_PanicFn = unsafe extern "C" fn(*mut js_State);
pub type js_Panic = Option<js_PanicFn>;
pub type js_CFunctionFn = unsafe extern "C" fn(*mut js_State);
pub type js_CFunction = Option<js_CFunctionFn>;
pub type js_FinalizeFn = unsafe extern "C" fn(*mut js_State, *mut c_void);
pub type js_Finalize = Option<js_FinalizeFn>;
pub type js_HasPropertyFn = unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int;
pub type js_HasProperty = Option<js_HasPropertyFn>;
pub type js_PutFn = unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int;
pub type js_Put = Option<js_PutFn>;
pub type js_DeleteFn = unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int;
pub type js_Delete = Option<js_DeleteFn>;
pub type js_ReportFn = unsafe extern "C" fn(*mut js_State, *const c_char);
pub type js_Report = Option<js_ReportFn>;

/* ---------------------------------------------------------------- values */

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ValueT {
    pub pad: [c_char; 15],
    pub r#type: c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union js_ValueU {
    pub shrstr: [c_char; 16],
    pub boolean: c_int,
    pub number: f64,
    pub litstr: *const c_char,
    pub memstr: *mut js_String,
    pub object: *mut js_Object,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union js_Value {
    pub t: js_ValueT,
    pub u: js_ValueU,
}

impl js_Value {
    pub const fn zero() -> js_Value {
        js_Value { t: js_ValueT { pad: [0; 15], r#type: 0 } }
    }
    pub const fn undef() -> js_Value {
        js_Value { t: js_ValueT { pad: [0; 15], r#type: JS_TUNDEFINED } }
    }
}

#[repr(C)]
pub struct js_String {
    pub gcnext: *mut js_String,
    pub gcmark: c_char,
    pub p: [c_char; 1],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Regexp {
    pub prog: *mut c_void,
    pub source: *mut c_char,
    pub flags: c_ushort,
    pub last: c_ushort,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectS {
    pub length: c_int,
    pub string: *mut c_char,
    pub shrstr: [c_char; 16],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectA {
    pub length: c_int,
    pub simple: c_int,
    pub flat_length: c_int,
    pub flat_capacity: c_int,
    pub array: *mut js_Value,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectF {
    pub function: *mut js_Function,
    pub scope: *mut js_Environment,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectC {
    pub name: *const c_char,
    pub function: js_CFunction,
    pub constructor: js_CFunction,
    pub length: c_int,
    pub data: *mut c_void,
    pub finalize: js_Finalize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectIter {
    pub target: *mut js_Object,
    pub i: c_int,
    pub n: c_int,
    pub head: *mut js_Iterator,
    pub current: *mut js_Iterator,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_ObjectUser {
    pub tag: *const c_char,
    pub data: *mut c_void,
    pub has: js_HasProperty,
    pub put: js_Put,
    pub delete: js_Delete,
    pub finalize: js_Finalize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union js_ObjectU {
    pub boolean: c_int,
    pub number: f64,
    pub s: js_ObjectS,
    pub a: js_ObjectA,
    pub f: js_ObjectF,
    pub c: js_ObjectC,
    pub r: js_Regexp,
    pub iter: js_ObjectIter,
    pub user: js_ObjectUser,
}

#[repr(C)]
pub struct js_Object {
    pub r#type: c_int,
    pub extensible: c_int,
    pub properties: *mut js_Property,
    pub count: c_int,
    pub prototype: *mut js_Object,
    pub u: js_ObjectU,
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
pub struct js_StackTrace {
    pub name: *const c_char,
    pub file: *const c_char,
    pub line: c_int,
    pub stack: c_int,
}

/* jmp_buf: glibc x86_64 -- 200 bytes, 8 byte alignment */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jmp_buf {
    pub j: [c_long; 8],
    pub mask_was_saved: c_int,
    pub pad_: c_int,
    pub saved_mask: [c_ulong; 16],
}

#[repr(C)]
pub struct js_Jumpbuf {
    pub buf: jmp_buf,
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
pub struct js_Lexbuf {
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
    pub lexbuf: js_Lexbuf,
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

    /* environments on the call stack but currently not in scope */
    pub envtop: c_int,
    pub envstack: [*mut js_Environment; JS_ENVLIMIT],

    /* debug info stack trace */
    pub tracetop: c_int,
    pub trace: [js_StackTrace; JS_ENVLIMIT],

    /* exception stack */
    pub trytop: c_int,
    pub trybuf: [js_Jumpbuf; JS_TRYLIMIT],
}

/* ---------------------------------------------------------------- lexer tokens */

pub const TK_IDENTIFIER: c_int = 256;
pub const TK_NUMBER: c_int = 257;
pub const TK_STRING: c_int = 258;
pub const TK_REGEXP: c_int = 259;
pub const TK_LE: c_int = 260;
pub const TK_GE: c_int = 261;
pub const TK_EQ: c_int = 262;
pub const TK_NE: c_int = 263;
pub const TK_STRICTEQ: c_int = 264;
pub const TK_STRICTNE: c_int = 265;
pub const TK_SHL: c_int = 266;
pub const TK_SHR: c_int = 267;
pub const TK_USHR: c_int = 268;
pub const TK_AND: c_int = 269;
pub const TK_OR: c_int = 270;
pub const TK_ADD_ASS: c_int = 271;
pub const TK_SUB_ASS: c_int = 272;
pub const TK_MUL_ASS: c_int = 273;
pub const TK_DIV_ASS: c_int = 274;
pub const TK_MOD_ASS: c_int = 275;
pub const TK_SHL_ASS: c_int = 276;
pub const TK_SHR_ASS: c_int = 277;
pub const TK_USHR_ASS: c_int = 278;
pub const TK_AND_ASS: c_int = 279;
pub const TK_OR_ASS: c_int = 280;
pub const TK_XOR_ASS: c_int = 281;
pub const TK_INC: c_int = 282;
pub const TK_DEC: c_int = 283;
pub const TK_BREAK: c_int = 284;
pub const TK_CASE: c_int = 285;
pub const TK_CATCH: c_int = 286;
pub const TK_CONTINUE: c_int = 287;
pub const TK_DEBUGGER: c_int = 288;
pub const TK_DEFAULT: c_int = 289;
pub const TK_DELETE: c_int = 290;
pub const TK_DO: c_int = 291;
pub const TK_ELSE: c_int = 292;
pub const TK_FALSE: c_int = 293;
pub const TK_FINALLY: c_int = 294;
pub const TK_FOR: c_int = 295;
pub const TK_FUNCTION: c_int = 296;
pub const TK_IF: c_int = 297;
pub const TK_IN: c_int = 298;
pub const TK_INSTANCEOF: c_int = 299;
pub const TK_NEW: c_int = 300;
pub const TK_NULL: c_int = 301;
pub const TK_RETURN: c_int = 302;
pub const TK_SWITCH: c_int = 303;
pub const TK_THIS: c_int = 304;
pub const TK_THROW: c_int = 305;
pub const TK_TRUE: c_int = 306;
pub const TK_TRY: c_int = 307;
pub const TK_TYPEOF: c_int = 308;
pub const TK_VAR: c_int = 309;
pub const TK_VOID: c_int = 310;
pub const TK_WHILE: c_int = 311;
pub const TK_WITH: c_int = 312;

/* ---------------------------------------------------------------- ast */

pub const AST_LIST: c_int = 0;
pub const AST_FUNDEC: c_int = 1;
pub const AST_IDENTIFIER: c_int = 2;
pub const EXP_IDENTIFIER: c_int = 3;
pub const EXP_NUMBER: c_int = 4;
pub const EXP_STRING: c_int = 5;
pub const EXP_REGEXP: c_int = 6;
pub const EXP_ELISION: c_int = 7;
pub const EXP_NULL: c_int = 8;
pub const EXP_TRUE: c_int = 9;
pub const EXP_FALSE: c_int = 10;
pub const EXP_THIS: c_int = 11;
pub const EXP_ARRAY: c_int = 12;
pub const EXP_OBJECT: c_int = 13;
pub const EXP_PROP_VAL: c_int = 14;
pub const EXP_PROP_GET: c_int = 15;
pub const EXP_PROP_SET: c_int = 16;
pub const EXP_FUN: c_int = 17;
pub const EXP_INDEX: c_int = 18;
pub const EXP_MEMBER: c_int = 19;
pub const EXP_CALL: c_int = 20;
pub const EXP_NEW: c_int = 21;
pub const EXP_POSTINC: c_int = 22;
pub const EXP_POSTDEC: c_int = 23;
pub const EXP_DELETE: c_int = 24;
pub const EXP_VOID: c_int = 25;
pub const EXP_TYPEOF: c_int = 26;
pub const EXP_PREINC: c_int = 27;
pub const EXP_PREDEC: c_int = 28;
pub const EXP_POS: c_int = 29;
pub const EXP_NEG: c_int = 30;
pub const EXP_BITNOT: c_int = 31;
pub const EXP_LOGNOT: c_int = 32;
pub const EXP_MOD: c_int = 33;
pub const EXP_DIV: c_int = 34;
pub const EXP_MUL: c_int = 35;
pub const EXP_SUB: c_int = 36;
pub const EXP_ADD: c_int = 37;
pub const EXP_USHR: c_int = 38;
pub const EXP_SHR: c_int = 39;
pub const EXP_SHL: c_int = 40;
pub const EXP_IN: c_int = 41;
pub const EXP_INSTANCEOF: c_int = 42;
pub const EXP_GE: c_int = 43;
pub const EXP_LE: c_int = 44;
pub const EXP_GT: c_int = 45;
pub const EXP_LT: c_int = 46;
pub const EXP_STRICTNE: c_int = 47;
pub const EXP_STRICTEQ: c_int = 48;
pub const EXP_NE: c_int = 49;
pub const EXP_EQ: c_int = 50;
pub const EXP_BITAND: c_int = 51;
pub const EXP_BITXOR: c_int = 52;
pub const EXP_BITOR: c_int = 53;
pub const EXP_LOGAND: c_int = 54;
pub const EXP_LOGOR: c_int = 55;
pub const EXP_COND: c_int = 56;
pub const EXP_ASS: c_int = 57;
pub const EXP_ASS_MUL: c_int = 58;
pub const EXP_ASS_DIV: c_int = 59;
pub const EXP_ASS_MOD: c_int = 60;
pub const EXP_ASS_ADD: c_int = 61;
pub const EXP_ASS_SUB: c_int = 62;
pub const EXP_ASS_SHL: c_int = 63;
pub const EXP_ASS_SHR: c_int = 64;
pub const EXP_ASS_USHR: c_int = 65;
pub const EXP_ASS_BITAND: c_int = 66;
pub const EXP_ASS_BITXOR: c_int = 67;
pub const EXP_ASS_BITOR: c_int = 68;
pub const EXP_COMMA: c_int = 69;
pub const EXP_VAR: c_int = 70;
pub const STM_BLOCK: c_int = 71;
pub const STM_EMPTY: c_int = 72;
pub const STM_VAR: c_int = 73;
pub const STM_IF: c_int = 74;
pub const STM_DO: c_int = 75;
pub const STM_WHILE: c_int = 76;
pub const STM_FOR: c_int = 77;
pub const STM_FOR_VAR: c_int = 78;
pub const STM_FOR_IN: c_int = 79;
pub const STM_FOR_IN_VAR: c_int = 80;
pub const STM_CONTINUE: c_int = 81;
pub const STM_BREAK: c_int = 82;
pub const STM_RETURN: c_int = 83;
pub const STM_WITH: c_int = 84;
pub const STM_SWITCH: c_int = 85;
pub const STM_THROW: c_int = 86;
pub const STM_TRY: c_int = 87;
pub const STM_DEBUGGER: c_int = 88;
pub const STM_LABEL: c_int = 89;
pub const STM_CASE: c_int = 90;
pub const STM_DEFAULT: c_int = 91;

#[repr(C)]
pub struct js_JumpList {
    pub r#type: c_int,
    pub inst: c_int,
    pub next: *mut js_JumpList,
}

#[repr(C)]
pub struct js_Ast {
    pub r#type: c_int,
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

/* ---------------------------------------------------------------- opcodes */

pub const OP_POP: c_int = 0;
pub const OP_DUP: c_int = 1;
pub const OP_DUP2: c_int = 2;
pub const OP_ROT2: c_int = 3;
pub const OP_ROT3: c_int = 4;
pub const OP_ROT4: c_int = 5;
pub const OP_INTEGER: c_int = 6;
pub const OP_NUMBER: c_int = 7;
pub const OP_STRING: c_int = 8;
pub const OP_CLOSURE: c_int = 9;
pub const OP_NEWARRAY: c_int = 10;
pub const OP_NEWOBJECT: c_int = 11;
pub const OP_NEWREGEXP: c_int = 12;
pub const OP_UNDEF: c_int = 13;
pub const OP_NULL: c_int = 14;
pub const OP_TRUE: c_int = 15;
pub const OP_FALSE: c_int = 16;
pub const OP_THIS: c_int = 17;
pub const OP_CURRENT: c_int = 18;
pub const OP_GETLOCAL: c_int = 19;
pub const OP_SETLOCAL: c_int = 20;
pub const OP_DELLOCAL: c_int = 21;
pub const OP_HASVAR: c_int = 22;
pub const OP_GETVAR: c_int = 23;
pub const OP_SETVAR: c_int = 24;
pub const OP_DELVAR: c_int = 25;
pub const OP_IN: c_int = 26;
pub const OP_SKIPARRAY: c_int = 27;
pub const OP_INITARRAY: c_int = 28;
pub const OP_INITPROP: c_int = 29;
pub const OP_INITGETTER: c_int = 30;
pub const OP_INITSETTER: c_int = 31;
pub const OP_GETPROP: c_int = 32;
pub const OP_GETPROP_S: c_int = 33;
pub const OP_SETPROP: c_int = 34;
pub const OP_SETPROP_S: c_int = 35;
pub const OP_DELPROP: c_int = 36;
pub const OP_DELPROP_S: c_int = 37;
pub const OP_ITERATOR: c_int = 38;
pub const OP_NEXTITER: c_int = 39;
pub const OP_EVAL: c_int = 40;
pub const OP_CALL: c_int = 41;
pub const OP_NEW: c_int = 42;
pub const OP_TYPEOF: c_int = 43;
pub const OP_POS: c_int = 44;
pub const OP_NEG: c_int = 45;
pub const OP_BITNOT: c_int = 46;
pub const OP_LOGNOT: c_int = 47;
pub const OP_INC: c_int = 48;
pub const OP_DEC: c_int = 49;
pub const OP_POSTINC: c_int = 50;
pub const OP_POSTDEC: c_int = 51;
pub const OP_MUL: c_int = 52;
pub const OP_DIV: c_int = 53;
pub const OP_MOD: c_int = 54;
pub const OP_ADD: c_int = 55;
pub const OP_SUB: c_int = 56;
pub const OP_SHL: c_int = 57;
pub const OP_SHR: c_int = 58;
pub const OP_USHR: c_int = 59;
pub const OP_LT: c_int = 60;
pub const OP_GT: c_int = 61;
pub const OP_LE: c_int = 62;
pub const OP_GE: c_int = 63;
pub const OP_EQ: c_int = 64;
pub const OP_NE: c_int = 65;
pub const OP_STRICTEQ: c_int = 66;
pub const OP_STRICTNE: c_int = 67;
pub const OP_JCASE: c_int = 68;
pub const OP_BITAND: c_int = 69;
pub const OP_BITXOR: c_int = 70;
pub const OP_BITOR: c_int = 71;
pub const OP_INSTANCEOF: c_int = 72;
pub const OP_THROW: c_int = 73;
pub const OP_TRY: c_int = 74;
pub const OP_ENDTRY: c_int = 75;
pub const OP_CATCH: c_int = 76;
pub const OP_ENDCATCH: c_int = 77;
pub const OP_WITH: c_int = 78;
pub const OP_ENDWITH: c_int = 79;
pub const OP_DEBUGGER: c_int = 80;
pub const OP_JUMP: c_int = 81;
pub const OP_JTRUE: c_int = 82;
pub const OP_JFALSE: c_int = 83;
pub const OP_RETURN: c_int = 84;

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

/* opaque to most modules; defined in jsintern.rs / regexp.rs */
#[repr(C)]
pub struct js_StringNode {
    pub left: *mut js_StringNode,
    pub right: *mut js_StringNode,
    pub level: c_int,
    pub string: [c_char; 1],
}

/* Resub for regexp.c public API */
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Resub_sub {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [Resub_sub; REG_MAXSUB],
}

/* ---------------------------------------------------------------- offsets */

pub const SOFFSETOF_JS_VALUE_TYPE: c_int = 15; /* soffsetof(js_Value, t.type) */
pub const SOFFSETOF_JS_STRING_P: c_int = 9; /* soffsetof(js_String, p) */
pub const SOFFSETOF_JS_BUFFER_S: c_int = 8; /* soffsetof(js_Buffer, s) */
pub const SOFFSETOF_JS_PROPERTY_NAME: c_int = 56; /* soffsetof(js_Property, name) */
pub const SOFFSETOF_JS_ITERATOR_NAME: c_int = 8; /* soffsetof(js_Iterator, name) */
pub const SOFFSETOF_JS_STRINGNODE_STRING: c_int = 20; /* soffsetof(js_StringNode, string) */

/* ---------------------------------------------------------------- helpers */

/// Pointer to the flexible char array of a js_String.
#[inline(always)]
pub unsafe fn js_String_p(s: *mut js_String) -> *mut c_char {
    std::ptr::addr_of_mut!((*s).p) as *mut c_char
}

/// Pointer to the flexible char array of a js_Property.
#[inline(always)]
pub unsafe fn js_Property_name(p: *mut js_Property) -> *mut c_char {
    std::ptr::addr_of_mut!((*p).name) as *mut c_char
}

/// Pointer to the flexible char array of a js_Iterator.
#[inline(always)]
pub unsafe fn js_Iterator_name(p: *mut js_Iterator) -> *mut c_char {
    std::ptr::addr_of_mut!((*p).name) as *mut c_char
}

/// Pointer to the flexible char array of a js_StringNode.
#[inline(always)]
pub unsafe fn js_StringNode_string(p: *mut js_StringNode) -> *mut c_char {
    std::ptr::addr_of_mut!((*p).string) as *mut c_char
}

/// Pointer to the (growable) char array of a js_Buffer.
#[inline(always)]
pub unsafe fn js_Buffer_s(p: *mut js_Buffer) -> *mut c_char {
    std::ptr::addr_of_mut!((*p).s) as *mut c_char
}

/// Pointer to the short string storage of a js_Value.
#[inline(always)]
pub unsafe fn js_Value_shrstr(v: *mut js_Value) -> *mut c_char {
    std::ptr::addr_of_mut!((*v).u.shrstr) as *mut c_char
}

/// Pointer to the short string storage of a js_Object (JS_CSTRING).
#[inline(always)]
pub unsafe fn js_Object_shrstr(v: *mut js_Object) -> *mut c_char {
    std::ptr::addr_of_mut!((*v).u.s.shrstr) as *mut c_char
}

/// JSV_ISSTRING(v)
#[inline(always)]
pub unsafe fn JSV_ISSTRING(v: *const js_Value) -> bool {
    let t = (*v).t.r#type;
    t == JS_TSHRSTR || t == JS_TMEMSTR || t == JS_TLITSTR
}

/// JSV_TOSTRING(v)
#[inline(always)]
pub unsafe fn JSV_TOSTRING(v: *const js_Value) -> *const c_char {
    let t = (*v).t.r#type;
    if t == JS_TSHRSTR {
        std::ptr::addr_of!((*v).u.shrstr) as *const c_char
    } else if t == JS_TLITSTR {
        (*v).u.litstr
    } else if t == JS_TMEMSTR {
        js_String_p((*v).u.memstr)
    } else {
        c"".as_ptr()
    }
}

/* ------------------------------------------------------------------ *
 * C floating point -> integer conversions.
 *
 * Rust's `as` saturates (and maps NaN to 0), while C on x86-64 emits
 * cvttsd2si, which yields the "integer indefinite value" (INT_MIN of the
 * destination register width) for NaN and out-of-range values.  These helpers
 * reproduce the C/gcc behaviour exactly and must be used wherever the C code
 * converts a double that is not provably in range.
 * ------------------------------------------------------------------ */

/// C's `(int)x`
#[inline(always)]
pub fn d2i(x: f64) -> c_int {
    if x >= -2147483648.0 && x < 2147483648.0 {
        x as c_int
    } else {
        c_int::MIN
    }
}

/// C's `(long)x` / `(int64_t)x`
#[inline(always)]
pub fn d2i64(x: f64) -> i64 {
    if x >= -9223372036854775808.0 && x < 9223372036854775808.0 {
        x as i64
    } else {
        i64::MIN
    }
}

/// C's `(unsigned int)x` (gcc converts through a 64 bit register)
#[inline(always)]
pub fn d2u32(x: f64) -> c_uint {
    d2i64(x) as c_uint
}

/// C's `(unsigned short)x`
#[inline(always)]
pub fn d2u16(x: f64) -> c_ushort {
    d2i(x) as c_ushort
}

/// C's `(short)x`
#[inline(always)]
pub fn d2i16(x: f64) -> c_short {
    d2i(x) as c_short
}

/// C's `(uint64_t)x`
#[inline(always)]
pub fn d2u64(x: f64) -> u64 {
    if !(x >= 9223372036854775808.0) {
        d2i64(x) as u64
    } else {
        (d2i64(x - 9223372036854775808.0) as u64) ^ 0x8000000000000000
    }
}

/* volatile access, for locals that must survive longjmp (C's `volatile`) */
#[inline(always)]
pub unsafe fn vread<T: Copy>(p: *const T) -> T {
    std::ptr::read_volatile(p)
}
#[inline(always)]
pub unsafe fn vwrite<T: Copy>(p: *mut T, v: T) {
    std::ptr::write_volatile(p, v)
}

/* C library math predicates */
#[inline(always)]
pub fn isnan(x: f64) -> bool {
    x.is_nan()
}
#[inline(always)]
pub fn isinf(x: f64) -> bool {
    x.is_infinite()
}
#[inline(always)]
pub fn isfinite(x: f64) -> bool {
    x.is_finite()
}
#[inline(always)]
pub fn signbit(x: f64) -> bool {
    x.is_sign_negative()
}

/* ctype.h (C locale) */
#[inline(always)]
pub fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}
#[inline(always)]
pub fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}
#[inline(always)]
pub fn isupper(c: c_int) -> bool {
    c >= 'A' as c_int && c <= 'Z' as c_int
}
#[inline(always)]
pub fn islower(c: c_int) -> bool {
    c >= 'a' as c_int && c <= 'z' as c_int
}
#[inline(always)]
pub fn isspace(c: c_int) -> bool {
    c == ' ' as c_int || (c >= 0x09 && c <= 0x0d)
}
#[inline(always)]
pub fn toupper(c: c_int) -> c_int {
    if islower(c) {
        c - 32
    } else {
        c
    }
}
#[inline(always)]
pub fn tolower(c: c_int) -> c_int {
    if isupper(c) {
        c + 32
    } else {
        c
    }
}

/* ---------------------------------------------------------------- libc */

#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

#[repr(C)]
#[derive(Copy, Clone)]
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

pub type time_t = c_long;

extern "C" {
    pub static mut stdout: *mut FILE;
    pub static mut stderr: *mut FILE;

    pub fn malloc(n: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;

    pub fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strcat(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char;

    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: *mut VaListTag) -> c_int;
    pub fn fputs(s: *const c_char, f: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, f: *mut FILE) -> c_int;
    pub fn puts(s: *const c_char) -> c_int;
    pub fn putchar(c: c_int) -> c_int;

    pub fn floor(x: f64) -> f64;
    pub fn ceil(x: f64) -> f64;
    pub fn fabs(x: f64) -> f64;
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

    pub fn time(t: *mut time_t) -> time_t;
    pub fn mktime(t: *mut tm) -> time_t;
    pub fn localtime(t: *const time_t) -> *mut tm;
    pub fn gmtime(t: *const time_t) -> *mut tm;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;

    pub fn _setjmp(env: *mut jmp_buf) -> c_int;
    pub fn longjmp(env: *mut jmp_buf, val: c_int) -> !;
}

/* x86-64 SysV va_list */
#[repr(C)]
pub struct VaListTag {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}
