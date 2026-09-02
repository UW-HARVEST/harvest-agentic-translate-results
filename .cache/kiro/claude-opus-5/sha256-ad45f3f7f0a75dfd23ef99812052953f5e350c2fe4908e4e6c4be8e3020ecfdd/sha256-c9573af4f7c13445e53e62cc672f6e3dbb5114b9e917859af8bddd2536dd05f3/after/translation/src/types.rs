// Translation of c_src/src/jsi.h and c_src/include/mujs.h data structures
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use std::ffi::{c_char, c_double, c_int, c_short, c_uint, c_ushort, c_void};

/* ---- public typedefs (mujs.h) ---- */

pub type js_Alloc = Option<unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type js_Panic = Option<unsafe extern "C-unwind" fn(*mut js_State)>;
pub type js_CFunction = Option<unsafe extern "C-unwind" fn(*mut js_State)>;
pub type js_Finalize = Option<unsafe extern "C-unwind" fn(*mut js_State, *mut c_void)>;
pub type js_HasProperty =
    Option<unsafe extern "C-unwind" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Put = Option<unsafe extern "C-unwind" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Delete =
    Option<unsafe extern "C-unwind" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Report = Option<unsafe extern "C-unwind" fn(*mut js_State, *const c_char)>;

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

/* ---- limits ---- */

pub const JS_STACKSIZE: c_int = 4096;
pub const JS_ENVLIMIT: usize = 1024;
pub const JS_TRYLIMIT: usize = 64;
pub const JS_ARRAYLIMIT: c_int = 1 << 26;
pub const JS_GCFACTOR: f64 = 5.0;
pub const JS_ASTLIMIT: c_int = 400;
pub const JS_STRLIMIT: usize = 1 << 28;

pub type js_Instruction = c_ushort;

/* ---- hint to ToPrimitive() ---- */
pub const JS_HNONE: c_int = 0;
pub const JS_HNUMBER: c_int = 1;
pub const JS_HSTRING: c_int = 2;

/* ---- enum js_Type ---- */
pub const JS_TSHRSTR: c_int = 0;
pub const JS_TUNDEFINED: c_int = 1;
pub const JS_TNULL: c_int = 2;
pub const JS_TBOOLEAN: c_int = 3;
pub const JS_TNUMBER: c_int = 4;
pub const JS_TLITSTR: c_int = 5;
pub const JS_TMEMSTR: c_int = 6;
pub const JS_TOBJECT: c_int = 7;

/* ---- enum js_Class ---- */
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

/* ---- js_Value ---- */

/// offsetof(js_Value, t.type) == 15: the maximum length of a short string
pub const SHRSTR_MAX: c_int = 15;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ValueT {
    pub pad: [c_char; 15],
    pub type_: c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_ValueU {
    pub shrstr: [c_char; 16],
    pub boolean: c_int,
    pub number: c_double,
    pub litstr: *const c_char,
    pub memstr: *mut js_String,
    pub object: *mut js_Object,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_Value {
    pub t: js_ValueT,
    pub u: js_ValueU,
}

impl js_Value {
    #[inline]
    pub const fn undef() -> js_Value {
        js_Value {
            t: js_ValueT {
                pad: [0; 15],
                type_: JS_TUNDEFINED as c_char,
            },
        }
    }
    #[inline]
    pub fn ty(&self) -> c_int {
        unsafe { self.t.type_ as c_int }
    }
    #[inline]
    pub fn set_ty(&mut self, t: c_int) {
        self.t.type_ = t as c_char;
    }
    #[inline]
    pub fn num(&self) -> f64 {
        unsafe { self.u.number }
    }
    #[inline]
    pub fn boolean(&self) -> c_int {
        unsafe { self.u.boolean }
    }
    #[inline]
    pub fn litstr(&self) -> *const c_char {
        unsafe { self.u.litstr }
    }
    #[inline]
    pub fn memstr(&self) -> *mut js_String {
        unsafe { self.u.memstr }
    }
    #[inline]
    pub fn object(&self) -> *mut js_Object {
        unsafe { self.u.object }
    }
    #[inline]
    pub fn shrstr_ptr(&self) -> *const c_char {
        (&raw const self.u.shrstr) as *const c_char
    }
    #[inline]
    pub fn shrstr_mut(&mut self) -> *mut c_char {
        (&raw mut self.u.shrstr) as *mut c_char
    }
    #[inline]
    pub fn is_string(&self) -> bool {
        let t = self.ty();
        t == JS_TSHRSTR || t == JS_TMEMSTR || t == JS_TLITSTR
    }
    /// JSV_TOSTRING(v)
    #[inline]
    pub unsafe fn to_str_ptr(&self) -> *const c_char {
        unsafe {
            match self.ty() {
                JS_TSHRSTR => self.shrstr_ptr(),
                JS_TLITSTR => self.u.litstr,
                JS_TMEMSTR => (&raw mut (*self.u.memstr).p) as *const c_char,
                _ => c"".as_ptr(),
            }
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
pub struct js_ObjectS {
    pub length: c_int,
    pub string: *mut c_char,
    pub shrstr: [c_char; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ObjectA {
    pub length: c_int,
    pub simple: c_int,
    pub flat_length: c_int,
    pub flat_capacity: c_int,
    pub array: *mut js_Value,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ObjectF {
    pub function: *mut js_Function,
    pub scope: *mut js_Environment,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ObjectC {
    pub name: *const c_char,
    pub function: js_CFunction,
    pub constructor: js_CFunction,
    pub length: c_int,
    pub data: *mut c_void,
    pub finalize: js_Finalize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ObjectIter {
    pub target: *mut js_Object,
    pub i: c_int,
    pub n: c_int,
    pub head: *mut js_Iterator,
    pub current: *mut js_Iterator,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_ObjectUser {
    pub tag: *const c_char,
    pub data: *mut c_void,
    pub has: js_HasProperty,
    pub put: js_Put,
    pub delete: js_Delete,
    pub finalize: js_Finalize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union js_ObjectU {
    pub boolean: c_int,
    pub number: c_double,
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
    pub type_: c_int, /* enum js_Class */
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
#[derive(Clone, Copy)]
pub struct js_StackTrace {
    pub name: *const c_char,
    pub file: *const c_char,
    pub line: c_int,
    pub stack: c_int,
}

/* jmp_buf is 200 bytes on glibc/x86-64; reserve extra space and 16-byte alignment. */
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct JmpBuf(pub [u64; 40]);

/// How a try frame unwinds: TRY_INTERNAL uses a Rust panic, TRY_EXTERNAL longjmp().
pub const TRY_INTERNAL: c_int = 0;
pub const TRY_EXTERNAL: c_int = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct js_Jumpbuf {
    pub buf: JmpBuf,
    pub E: *mut js_Environment,
    pub envtop: c_int,
    pub tracetop: c_int,
    pub top: c_int,
    pub bot: c_int,
    pub strict: c_int,
    pub pc: *mut js_Instruction,
    pub kind: c_int,
}

/* String buffer */
#[repr(C)]
pub struct js_Buffer {
    pub n: c_int,
    pub m: c_int,
    pub s: [c_char; 64],
}

#[repr(C)]
pub struct LexBuf {
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
    pub lexbuf: LexBuf,
    pub lexline: c_int,
    pub lexchar: c_int,
    pub lasttoken: c_int,
    pub newline: c_int,

    /* parser state */
    pub astdepth: c_int,
    pub lookahead: c_int,
    pub text: *const c_char,
    pub number: c_double,
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
    pub trybuf: [js_Jumpbuf; JS_TRYLIMIT],
}

/* ---- interned strings (jsintern.c) ---- */

#[repr(C)]
pub struct js_StringNode {
    pub left: *mut js_StringNode,
    pub right: *mut js_StringNode,
    pub level: c_int,
    pub string: [c_char; 1],
}

/* ---- lexer tokens ---- */

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

/* ---- enum js_AstType ---- */

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
    pub number: c_double,
    pub string: *const c_char,
    pub jumps: *mut js_JumpList,
    pub casejump: c_int,
    pub gcnext: *mut js_Ast,
}

/* ---- enum js_OpCode ---- */

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

/* Aliases used by translated code for short int types */
pub type Short = c_short;
