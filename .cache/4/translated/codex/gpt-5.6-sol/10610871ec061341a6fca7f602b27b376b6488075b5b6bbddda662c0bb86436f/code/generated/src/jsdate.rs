extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn js_rangeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_call(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_defglobal(
        J: *mut js_State,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_getproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    );
    fn js_pushnull(J: *mut js_State);
    fn js_pushnumber(J: *mut js_State, v: ::core::ffi::c_double);
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_newcconstructor(
        J: *mut js_State,
        fun: js_CFunction,
        con: js_CFunction,
        name: *const ::core::ffi::c_char,
        length: ::core::ffi::c_int,
    );
    fn js_isdefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isnumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isstring(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscallable(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tonumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_double;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_gettop(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn __uflow(_: *mut FILE) -> ::core::ffi::c_int;
    fn __overflow(_: *mut FILE, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn strtoll(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    fn time(__timer: *mut time_t) -> time_t;
    fn mktime(__tp: *mut tm) -> time_t;
    fn gmtime(__timer: *const time_t) -> *mut tm;
    fn localtime(__timer: *const time_t) -> *mut tm;
    fn gettimeofday(
        __tv: *mut timeval,
        __tz: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn fabs(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn floor(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn fmod(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn js_toprimitive(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        hint: ::core::ffi::c_int,
    );
    fn js_toobject(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Object;
    fn js_pushobject(J: *mut js_State, v: *mut js_Object);
    fn jsV_newobject(
        J: *mut js_State,
        type_0: js_Class,
        prototype: *mut js_Object,
    ) -> *mut js_Object;
    fn jsB_propf(
        J: *mut js_State,
        name: *const ::core::ffi::c_char,
        cfun: js_CFunction,
        n: ::core::ffi::c_int,
    );
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: ::core::ffi::c_uint,
    pub fp_offset: ::core::ffi::c_uint,
    pub overflow_arg_area: *mut ::core::ffi::c_void,
    pub reg_save_area: *mut ::core::ffi::c_void,
}
pub type __jmp_buf = [::core::ffi::c_long; 8];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sigset_t {
    pub __val: [::core::ffi::c_ulong; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __jmp_buf_tag {
    pub __jmpbuf: __jmp_buf,
    pub __mask_was_saved: ::core::ffi::c_int,
    pub __saved_mask: __sigset_t,
}
pub type jmp_buf = [__jmp_buf_tag; 1];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_State {
    pub actx: *mut ::core::ffi::c_void,
    pub uctx: *mut ::core::ffi::c_void,
    pub alloc: js_Alloc,
    pub report: js_Report,
    pub panic: js_Panic,
    pub strings: *mut js_StringNode,
    pub default_strict: ::core::ffi::c_int,
    pub strict: ::core::ffi::c_int,
    pub filename: *const ::core::ffi::c_char,
    pub source: *const ::core::ffi::c_char,
    pub line: ::core::ffi::c_int,
    pub lexbuf: C2RustUnnamed_8,
    pub lexline: ::core::ffi::c_int,
    pub lexchar: ::core::ffi::c_int,
    pub lasttoken: ::core::ffi::c_int,
    pub newline: ::core::ffi::c_int,
    pub astdepth: ::core::ffi::c_int,
    pub lookahead: ::core::ffi::c_int,
    pub text: *const ::core::ffi::c_char,
    pub number: ::core::ffi::c_double,
    pub gcast: *mut js_Ast,
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
    pub seed: ::core::ffi::c_uint,
    pub scratch: [::core::ffi::c_char; 12],
    pub nextref: ::core::ffi::c_int,
    pub R: *mut js_Object,
    pub G: *mut js_Object,
    pub E: *mut js_Environment,
    pub GE: *mut js_Environment,
    pub top: ::core::ffi::c_int,
    pub bot: ::core::ffi::c_int,
    pub stack: *mut js_Value,
    pub gcmark: ::core::ffi::c_int,
    pub gccounter: ::core::ffi::c_uint,
    pub gcthresh: ::core::ffi::c_uint,
    pub gcenv: *mut js_Environment,
    pub gcfun: *mut js_Function,
    pub gcobj: *mut js_Object,
    pub gcstr: *mut js_String,
    pub gcroot: *mut js_Object,
    pub runlimit: ::core::ffi::c_int,
    pub memlimit: ::core::ffi::c_int,
    pub envtop: ::core::ffi::c_int,
    pub envstack: [*mut js_Environment; 1024],
    pub tracetop: ::core::ffi::c_int,
    pub trace: [js_StackTrace; 1024],
    pub trytop: ::core::ffi::c_int,
    pub trybuf: [js_Jumpbuf; 64],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Jumpbuf {
    pub buf: jmp_buf,
    pub E: *mut js_Environment,
    pub envtop: ::core::ffi::c_int,
    pub tracetop: ::core::ffi::c_int,
    pub top: ::core::ffi::c_int,
    pub bot: ::core::ffi::c_int,
    pub strict: ::core::ffi::c_int,
    pub pc: *mut js_Instruction,
}
pub type js_Instruction = ::core::ffi::c_ushort;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Environment {
    pub outer: *mut js_Environment,
    pub variables: *mut js_Object,
    pub gcnext: *mut js_Environment,
    pub gcmark: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Object {
    pub type_0: js_Class,
    pub extensible: ::core::ffi::c_int,
    pub properties: *mut js_Property,
    pub count: ::core::ffi::c_int,
    pub prototype: *mut js_Object,
    pub u: C2RustUnnamed,
    pub gcnext: *mut js_Object,
    pub gcroot: *mut js_Object,
    pub gcmark: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub boolean: ::core::ffi::c_int,
    pub number: ::core::ffi::c_double,
    pub s: C2RustUnnamed_7,
    pub a: C2RustUnnamed_4,
    pub f: C2RustUnnamed_3,
    pub c: C2RustUnnamed_2,
    pub r: js_Regexp,
    pub iter: C2RustUnnamed_1,
    pub user: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub tag: *const ::core::ffi::c_char,
    pub data: *mut ::core::ffi::c_void,
    pub has: js_HasProperty,
    pub put: js_Put,
    pub delete: js_Delete,
    pub finalize: js_Finalize,
}
pub type js_Finalize = Option<
    unsafe extern "C" fn(*mut js_State, *mut ::core::ffi::c_void) -> (),
>;
pub type js_Delete = Option<
    unsafe extern "C" fn(
        *mut js_State,
        *mut ::core::ffi::c_void,
        *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int,
>;
pub type js_Put = Option<
    unsafe extern "C" fn(
        *mut js_State,
        *mut ::core::ffi::c_void,
        *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int,
>;
pub type js_HasProperty = Option<
    unsafe extern "C" fn(
        *mut js_State,
        *mut ::core::ffi::c_void,
        *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub target: *mut js_Object,
    pub i: ::core::ffi::c_int,
    pub n: ::core::ffi::c_int,
    pub head: *mut js_Iterator,
    pub current: *mut js_Iterator,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Iterator {
    pub next: *mut js_Iterator,
    pub name: [::core::ffi::c_char; 1],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Regexp {
    pub prog: *mut ::core::ffi::c_void,
    pub source: *mut ::core::ffi::c_char,
    pub flags: ::core::ffi::c_ushort,
    pub last: ::core::ffi::c_ushort,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub name: *const ::core::ffi::c_char,
    pub function: js_CFunction,
    pub constructor: js_CFunction,
    pub length: ::core::ffi::c_int,
    pub data: *mut ::core::ffi::c_void,
    pub finalize: js_Finalize,
}
pub type js_CFunction = Option<unsafe extern "C" fn(*mut js_State) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub function: *mut js_Function,
    pub scope: *mut js_Environment,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Function {
    pub name: *const ::core::ffi::c_char,
    pub script: ::core::ffi::c_int,
    pub lightweight: ::core::ffi::c_int,
    pub strict: ::core::ffi::c_int,
    pub arguments: ::core::ffi::c_int,
    pub numparams: ::core::ffi::c_int,
    pub code: *mut js_Instruction,
    pub codecap: ::core::ffi::c_int,
    pub codelen: ::core::ffi::c_int,
    pub funtab: *mut *mut js_Function,
    pub funcap: ::core::ffi::c_int,
    pub funlen: ::core::ffi::c_int,
    pub vartab: *mut *const ::core::ffi::c_char,
    pub varcap: ::core::ffi::c_int,
    pub varlen: ::core::ffi::c_int,
    pub filename: *const ::core::ffi::c_char,
    pub line: ::core::ffi::c_int,
    pub lastline: ::core::ffi::c_int,
    pub gcnext: *mut js_Function,
    pub gcmark: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub length: ::core::ffi::c_int,
    pub simple: ::core::ffi::c_int,
    pub flat_length: ::core::ffi::c_int,
    pub flat_capacity: ::core::ffi::c_int,
    pub array: *mut js_Value,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union js_Value {
    pub t: C2RustUnnamed_6,
    pub u: C2RustUnnamed_5,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_5 {
    pub shrstr: [::core::ffi::c_char; 16],
    pub boolean: ::core::ffi::c_int,
    pub number: ::core::ffi::c_double,
    pub litstr: *const ::core::ffi::c_char,
    pub memstr: *mut js_String,
    pub object: *mut js_Object,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_String {
    pub gcnext: *mut js_String,
    pub gcmark: ::core::ffi::c_char,
    pub p: [::core::ffi::c_char; 1],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub pad: [::core::ffi::c_char; 15],
    pub type_0: ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub length: ::core::ffi::c_int,
    pub string: *mut ::core::ffi::c_char,
    pub shrstr: [::core::ffi::c_char; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Property {
    pub left: *mut js_Property,
    pub right: *mut js_Property,
    pub level: ::core::ffi::c_int,
    pub atts: ::core::ffi::c_int,
    pub value: js_Value,
    pub getter: *mut js_Object,
    pub setter: *mut js_Object,
    pub name: [::core::ffi::c_char; 1],
}
pub type js_Class = ::core::ffi::c_uint;
pub const JS_CUSERDATA: js_Class = 15;
pub const JS_CITERATOR: js_Class = 14;
pub const JS_CARGUMENTS: js_Class = 13;
pub const JS_CJSON: js_Class = 12;
pub const JS_CMATH: js_Class = 11;
pub const JS_CDATE: js_Class = 10;
pub const JS_CREGEXP: js_Class = 9;
pub const JS_CSTRING: js_Class = 8;
pub const JS_CNUMBER: js_Class = 7;
pub const JS_CBOOLEAN: js_Class = 6;
pub const JS_CERROR: js_Class = 5;
pub const JS_CCFUNCTION: js_Class = 4;
pub const JS_CSCRIPT: js_Class = 3;
pub const JS_CFUNCTION: js_Class = 2;
pub const JS_CARRAY: js_Class = 1;
pub const JS_COBJECT: js_Class = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_StackTrace {
    pub name: *const ::core::ffi::c_char,
    pub file: *const ::core::ffi::c_char,
    pub line: ::core::ffi::c_int,
    pub stack: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Ast {
    pub type_0: js_AstType,
    pub line: ::core::ffi::c_int,
    pub parent: *mut js_Ast,
    pub a: *mut js_Ast,
    pub b: *mut js_Ast,
    pub c: *mut js_Ast,
    pub d: *mut js_Ast,
    pub number: ::core::ffi::c_double,
    pub string: *const ::core::ffi::c_char,
    pub jumps: *mut js_JumpList,
    pub casejump: ::core::ffi::c_int,
    pub gcnext: *mut js_Ast,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_JumpList {
    pub type_0: js_AstType,
    pub inst: ::core::ffi::c_int,
    pub next: *mut js_JumpList,
}
pub type js_AstType = ::core::ffi::c_uint;
pub const STM_DEFAULT: js_AstType = 91;
pub const STM_CASE: js_AstType = 90;
pub const STM_LABEL: js_AstType = 89;
pub const STM_DEBUGGER: js_AstType = 88;
pub const STM_TRY: js_AstType = 87;
pub const STM_THROW: js_AstType = 86;
pub const STM_SWITCH: js_AstType = 85;
pub const STM_WITH: js_AstType = 84;
pub const STM_RETURN: js_AstType = 83;
pub const STM_BREAK: js_AstType = 82;
pub const STM_CONTINUE: js_AstType = 81;
pub const STM_FOR_IN_VAR: js_AstType = 80;
pub const STM_FOR_IN: js_AstType = 79;
pub const STM_FOR_VAR: js_AstType = 78;
pub const STM_FOR: js_AstType = 77;
pub const STM_WHILE: js_AstType = 76;
pub const STM_DO: js_AstType = 75;
pub const STM_IF: js_AstType = 74;
pub const STM_VAR: js_AstType = 73;
pub const STM_EMPTY: js_AstType = 72;
pub const STM_BLOCK: js_AstType = 71;
pub const EXP_VAR: js_AstType = 70;
pub const EXP_COMMA: js_AstType = 69;
pub const EXP_ASS_BITOR: js_AstType = 68;
pub const EXP_ASS_BITXOR: js_AstType = 67;
pub const EXP_ASS_BITAND: js_AstType = 66;
pub const EXP_ASS_USHR: js_AstType = 65;
pub const EXP_ASS_SHR: js_AstType = 64;
pub const EXP_ASS_SHL: js_AstType = 63;
pub const EXP_ASS_SUB: js_AstType = 62;
pub const EXP_ASS_ADD: js_AstType = 61;
pub const EXP_ASS_MOD: js_AstType = 60;
pub const EXP_ASS_DIV: js_AstType = 59;
pub const EXP_ASS_MUL: js_AstType = 58;
pub const EXP_ASS: js_AstType = 57;
pub const EXP_COND: js_AstType = 56;
pub const EXP_LOGOR: js_AstType = 55;
pub const EXP_LOGAND: js_AstType = 54;
pub const EXP_BITOR: js_AstType = 53;
pub const EXP_BITXOR: js_AstType = 52;
pub const EXP_BITAND: js_AstType = 51;
pub const EXP_EQ: js_AstType = 50;
pub const EXP_NE: js_AstType = 49;
pub const EXP_STRICTEQ: js_AstType = 48;
pub const EXP_STRICTNE: js_AstType = 47;
pub const EXP_LT: js_AstType = 46;
pub const EXP_GT: js_AstType = 45;
pub const EXP_LE: js_AstType = 44;
pub const EXP_GE: js_AstType = 43;
pub const EXP_INSTANCEOF: js_AstType = 42;
pub const EXP_IN: js_AstType = 41;
pub const EXP_SHL: js_AstType = 40;
pub const EXP_SHR: js_AstType = 39;
pub const EXP_USHR: js_AstType = 38;
pub const EXP_ADD: js_AstType = 37;
pub const EXP_SUB: js_AstType = 36;
pub const EXP_MUL: js_AstType = 35;
pub const EXP_DIV: js_AstType = 34;
pub const EXP_MOD: js_AstType = 33;
pub const EXP_LOGNOT: js_AstType = 32;
pub const EXP_BITNOT: js_AstType = 31;
pub const EXP_NEG: js_AstType = 30;
pub const EXP_POS: js_AstType = 29;
pub const EXP_PREDEC: js_AstType = 28;
pub const EXP_PREINC: js_AstType = 27;
pub const EXP_TYPEOF: js_AstType = 26;
pub const EXP_VOID: js_AstType = 25;
pub const EXP_DELETE: js_AstType = 24;
pub const EXP_POSTDEC: js_AstType = 23;
pub const EXP_POSTINC: js_AstType = 22;
pub const EXP_NEW: js_AstType = 21;
pub const EXP_CALL: js_AstType = 20;
pub const EXP_MEMBER: js_AstType = 19;
pub const EXP_INDEX: js_AstType = 18;
pub const EXP_FUN: js_AstType = 17;
pub const EXP_PROP_SET: js_AstType = 16;
pub const EXP_PROP_GET: js_AstType = 15;
pub const EXP_PROP_VAL: js_AstType = 14;
pub const EXP_OBJECT: js_AstType = 13;
pub const EXP_ARRAY: js_AstType = 12;
pub const EXP_THIS: js_AstType = 11;
pub const EXP_FALSE: js_AstType = 10;
pub const EXP_TRUE: js_AstType = 9;
pub const EXP_NULL: js_AstType = 8;
pub const EXP_ELISION: js_AstType = 7;
pub const EXP_REGEXP: js_AstType = 6;
pub const EXP_STRING: js_AstType = 5;
pub const EXP_NUMBER: js_AstType = 4;
pub const EXP_IDENTIFIER: js_AstType = 3;
pub const AST_IDENTIFIER: js_AstType = 2;
pub const AST_FUNDEC: js_AstType = 1;
pub const AST_LIST: js_AstType = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub text: *mut ::core::ffi::c_char,
    pub len: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
}
pub type js_Panic = Option<unsafe extern "C" fn(*mut js_State) -> ()>;
pub type js_Report = Option<
    unsafe extern "C" fn(*mut js_State, *const ::core::ffi::c_char) -> (),
>;
pub type js_Alloc = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        *mut ::core::ffi::c_void,
        ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void,
>;
pub type C2RustUnnamed_9 = ::core::ffi::c_uint;
pub const JS_DONTCONF: C2RustUnnamed_9 = 4;
pub const JS_DONTENUM: C2RustUnnamed_9 = 2;
pub const JS_READONLY: C2RustUnnamed_9 = 1;
pub type size_t = usize;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __time_t = ::core::ffi::c_long;
pub type __suseconds_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timeval {
    pub tv_sec: __time_t,
    pub tv_usec: __suseconds_t,
}
pub type __compar_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
pub type C2RustUnnamed_10 = ::core::ffi::c_uint;
pub const JS_HSTRING: C2RustUnnamed_10 = 2;
pub const JS_HNUMBER: C2RustUnnamed_10 = 1;
pub const JS_HNONE: C2RustUnnamed_10 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: ::core::ffi::c_int,
    pub tm_min: ::core::ffi::c_int,
    pub tm_hour: ::core::ffi::c_int,
    pub tm_mday: ::core::ffi::c_int,
    pub tm_mon: ::core::ffi::c_int,
    pub tm_year: ::core::ffi::c_int,
    pub tm_wday: ::core::ffi::c_int,
    pub tm_yday: ::core::ffi::c_int,
    pub tm_isdst: ::core::ffi::c_int,
    pub tm_gmtoff: ::core::ffi::c_long,
    pub tm_zone: *const ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const _IO_EOF_SEEN: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const _IO_ERR_SEEN: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn vprintf(
    mut __fmt: *const ::core::ffi::c_char,
    mut __arg: ::core::ffi::VaList,
) -> ::core::ffi::c_int {
    return vfprintf(stdout, __fmt, __arg.as_va_list());
}
#[inline]
unsafe extern "C" fn getchar() -> ::core::ffi::c_int {
    return getc(stdin);
}
#[inline]
unsafe extern "C" fn fgetc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh2 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh2 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh0 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh0 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh1 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh1 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn putchar(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return putc(__c, stdout);
}
#[inline]
unsafe extern "C" fn fputc_unlocked(
    mut __c: ::core::ffi::c_int,
    mut __stream: *mut FILE,
) -> ::core::ffi::c_int {
    return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end)
        as ::core::ffi::c_int as ::core::ffi::c_long != 0
    {
        __overflow(__stream, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
    } else {
        let fresh3 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh3 = __c as ::core::ffi::c_char;
        *fresh3 as ::core::ffi::c_uchar as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn putc_unlocked(
    mut __c: ::core::ffi::c_int,
    mut __stream: *mut FILE,
) -> ::core::ffi::c_int {
    return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end)
        as ::core::ffi::c_int as ::core::ffi::c_long != 0
    {
        __overflow(__stream, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
    } else {
        let fresh4 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh4 = __c as ::core::ffi::c_char;
        *fresh4 as ::core::ffi::c_uchar as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn putchar_unlocked(
    mut __c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return if ((*stdout)._IO_write_ptr >= (*stdout)._IO_write_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __overflow(stdout, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
    } else {
        let fresh5 = (*stdout)._IO_write_ptr;
        (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
        *fresh5 = __c as ::core::ffi::c_char;
        *fresh5 as ::core::ffi::c_uchar as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn feof_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
    return ((*__stream)._flags & _IO_EOF_SEEN != 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn ferror_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
    return ((*__stream)._flags & _IO_ERR_SEEN != 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn atoi(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn atol(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_long {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn atoll(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_longlong {
    return strtoll(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn __bswap_16(mut __bsx: __uint16_t) -> __uint16_t {
    return (__bsx as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
        & 0xff as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int) as __uint16_t;
}
#[inline]
unsafe extern "C" fn __bswap_32(mut __bsx: __uint32_t) -> __uint32_t {
    return (__bsx & 0xff000000 as __uint32_t) >> 24 as ::core::ffi::c_int
        | (__bsx & 0xff0000 as __uint32_t) >> 8 as ::core::ffi::c_int
        | (__bsx & 0xff00 as __uint32_t) << 8 as ::core::ffi::c_int
        | (__bsx & 0xff as __uint32_t) << 24 as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn __bswap_64(mut __bsx: __uint64_t) -> __uint64_t {
    return ((__bsx as ::core::ffi::c_ulonglong
        & 0xff00000000000000 as ::core::ffi::c_ulonglong) >> 56 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong
            & 0xff000000000000 as ::core::ffi::c_ulonglong) >> 40 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong
            & 0xff0000000000 as ::core::ffi::c_ulonglong) >> 24 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong & 0xff00000000 as ::core::ffi::c_ulonglong)
            >> 8 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong & 0xff000000 as ::core::ffi::c_ulonglong)
            << 8 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong & 0xff0000 as ::core::ffi::c_ulonglong)
            << 24 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong & 0xff00 as ::core::ffi::c_ulonglong)
            << 40 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong & 0xff as ::core::ffi::c_ulonglong)
            << 56 as ::core::ffi::c_int) as __uint64_t;
}
#[inline]
unsafe extern "C" fn __uint16_identity(mut __x: __uint16_t) -> __uint16_t {
    return __x;
}
#[inline]
unsafe extern "C" fn __uint32_identity(mut __x: __uint32_t) -> __uint32_t {
    return __x;
}
#[inline]
unsafe extern "C" fn __uint64_identity(mut __x: __uint64_t) -> __uint64_t {
    return __x;
}
#[inline]
unsafe extern "C" fn bsearch(
    mut __key: *const ::core::ffi::c_void,
    mut __base: *const ::core::ffi::c_void,
    mut __nmemb: size_t,
    mut __size: size_t,
    mut __compar: __compar_fn_t,
) -> *mut ::core::ffi::c_void {
    let mut __l: size_t = 0;
    let mut __u: size_t = 0;
    let mut __idx: size_t = 0;
    let mut __p: *const ::core::ffi::c_void = ::core::ptr::null::<::core::ffi::c_void>();
    let mut __comparison: ::core::ffi::c_int = 0;
    __l = 0 as size_t;
    __u = __nmemb;
    while __l < __u {
        __idx = __l.wrapping_add(__u).wrapping_div(2 as size_t);
        __p = (__base as *const ::core::ffi::c_char)
            .offset(__idx.wrapping_mul(__size) as isize) as *const ::core::ffi::c_void;
        __comparison = Some(__compar.expect("non-null function pointer"))
            .expect("non-null function pointer")(__key, __p);
        if __comparison < 0 as ::core::ffi::c_int {
            __u = __idx;
        } else if __comparison > 0 as ::core::ffi::c_int {
            __l = __idx.wrapping_add(1 as size_t);
        } else {
            return __p as *mut ::core::ffi::c_void
        }
    }
    return NULL;
}
#[inline]
unsafe extern "C" fn atof(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    return strtod(__nptr, NULL as *mut *mut ::core::ffi::c_char);
}
unsafe extern "C" fn Now() -> ::core::ffi::c_double {
    let mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
    gettimeofday(&raw mut tv, NULL_0);
    return floor(
        tv.tv_sec as ::core::ffi::c_double * 1000.0f64
            + tv.tv_usec as ::core::ffi::c_double / 1000.0f64,
    );
}
unsafe extern "C" fn LocalTZA() -> ::core::ffi::c_double {
    static mut once: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    static mut tza: ::core::ffi::c_double = 0 as ::core::ffi::c_int
        as ::core::ffi::c_double;
    if once != 0 {
        let mut now: time_t = time(::core::ptr::null_mut::<time_t>());
        let mut utc: time_t = mktime(gmtime(&raw mut now));
        let mut loc: time_t = mktime(localtime(&raw mut now));
        tza = ((loc as ::core::ffi::c_long - utc as ::core::ffi::c_long)
            * 1000 as ::core::ffi::c_long) as ::core::ffi::c_double;
        once = 0 as ::core::ffi::c_int;
    }
    return tza;
}
unsafe extern "C" fn DaylightSavingTA(
    mut t: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
}
pub const HoursPerDay: ::core::ffi::c_double = 24.0f64;
pub const MinutesPerDay: ::core::ffi::c_double = HoursPerDay * MinutesPerHour;
pub const MinutesPerHour: ::core::ffi::c_double = 60.0f64;
pub const SecondsPerDay: ::core::ffi::c_double = MinutesPerDay * SecondsPerMinute;
pub const SecondsPerHour: ::core::ffi::c_double = MinutesPerHour * SecondsPerMinute;
pub const SecondsPerMinute: ::core::ffi::c_double = 60.0f64;
pub const msPerDay: ::core::ffi::c_double = SecondsPerDay * msPerSecond;
pub const msPerHour: ::core::ffi::c_double = SecondsPerHour * msPerSecond;
pub const msPerMinute: ::core::ffi::c_double = SecondsPerMinute * msPerSecond;
pub const msPerSecond: ::core::ffi::c_double = 1000.0f64;
unsafe extern "C" fn pmod(
    mut x: ::core::ffi::c_double,
    mut y: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    x = fmod(x, y);
    if x < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        x += y;
    }
    return x;
}
unsafe extern "C" fn Day(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return floor(t / msPerDay) as ::core::ffi::c_int;
}
unsafe extern "C" fn TimeWithinDay(
    mut t: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return pmod(t, msPerDay);
}
unsafe extern "C" fn DaysInYear(mut y: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return if y % 4 as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        && (y % 100 as ::core::ffi::c_int != 0
            || y % 400 as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
    {
        366 as ::core::ffi::c_int
    } else {
        365 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn DayFromYear(mut y: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return ((365 as ::core::ffi::c_int * (y - 1970 as ::core::ffi::c_int))
        as ::core::ffi::c_double
        + floor((y - 1969 as ::core::ffi::c_int) as ::core::ffi::c_double / 4.0f64)
        - floor((y - 1901 as ::core::ffi::c_int) as ::core::ffi::c_double / 100.0f64)
        + floor((y - 1601 as ::core::ffi::c_int) as ::core::ffi::c_double / 400.0f64))
        as ::core::ffi::c_int;
}
unsafe extern "C" fn TimeFromYear(mut y: ::core::ffi::c_int) -> ::core::ffi::c_double {
    return DayFromYear(y) as ::core::ffi::c_double * msPerDay;
}
unsafe extern "C" fn YearFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    let mut y: ::core::ffi::c_int = (floor(t / (msPerDay * 365.2425f64))
        + 1970 as ::core::ffi::c_int as ::core::ffi::c_double) as ::core::ffi::c_int;
    let mut t2: ::core::ffi::c_double = TimeFromYear(y);
    if t2 > t {
        y -= 1;
    } else if t2 + msPerDay * DaysInYear(y) as ::core::ffi::c_double <= t {
        y += 1;
    }
    return y;
}
unsafe extern "C" fn InLeapYear(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return (DaysInYear(YearFromTime(t)) == 366 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn DayWithinYear(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return Day(t) - DayFromYear(YearFromTime(t));
}
unsafe extern "C" fn MonthFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    let mut day: ::core::ffi::c_int = DayWithinYear(t);
    let mut leap: ::core::ffi::c_int = InLeapYear(t);
    if day < 31 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if day < 59 as ::core::ffi::c_int + leap {
        return 1 as ::core::ffi::c_int;
    }
    if day < 90 as ::core::ffi::c_int + leap {
        return 2 as ::core::ffi::c_int;
    }
    if day < 120 as ::core::ffi::c_int + leap {
        return 3 as ::core::ffi::c_int;
    }
    if day < 151 as ::core::ffi::c_int + leap {
        return 4 as ::core::ffi::c_int;
    }
    if day < 181 as ::core::ffi::c_int + leap {
        return 5 as ::core::ffi::c_int;
    }
    if day < 212 as ::core::ffi::c_int + leap {
        return 6 as ::core::ffi::c_int;
    }
    if day < 243 as ::core::ffi::c_int + leap {
        return 7 as ::core::ffi::c_int;
    }
    if day < 273 as ::core::ffi::c_int + leap {
        return 8 as ::core::ffi::c_int;
    }
    if day < 304 as ::core::ffi::c_int + leap {
        return 9 as ::core::ffi::c_int;
    }
    if day < 334 as ::core::ffi::c_int + leap {
        return 10 as ::core::ffi::c_int;
    }
    return 11 as ::core::ffi::c_int;
}
unsafe extern "C" fn DateFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    let mut day: ::core::ffi::c_int = DayWithinYear(t);
    let mut leap: ::core::ffi::c_int = InLeapYear(t);
    match MonthFromTime(t) {
        0 => return day + 1 as ::core::ffi::c_int,
        1 => return day - 30 as ::core::ffi::c_int,
        2 => return day - 58 as ::core::ffi::c_int - leap,
        3 => return day - 89 as ::core::ffi::c_int - leap,
        4 => return day - 119 as ::core::ffi::c_int - leap,
        5 => return day - 150 as ::core::ffi::c_int - leap,
        6 => return day - 180 as ::core::ffi::c_int - leap,
        7 => return day - 211 as ::core::ffi::c_int - leap,
        8 => return day - 242 as ::core::ffi::c_int - leap,
        9 => return day - 272 as ::core::ffi::c_int - leap,
        10 => return day - 303 as ::core::ffi::c_int - leap,
        _ => return day - 333 as ::core::ffi::c_int - leap,
    };
}
unsafe extern "C" fn WeekDay(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return pmod(
        (Day(t) + 4 as ::core::ffi::c_int) as ::core::ffi::c_double,
        7 as ::core::ffi::c_int as ::core::ffi::c_double,
    ) as ::core::ffi::c_int;
}
unsafe extern "C" fn LocalTime(mut utc: ::core::ffi::c_double) -> ::core::ffi::c_double {
    return utc + LocalTZA() + DaylightSavingTA(utc);
}
unsafe extern "C" fn UTC(mut loc: ::core::ffi::c_double) -> ::core::ffi::c_double {
    return loc - LocalTZA() - DaylightSavingTA(loc - LocalTZA());
}
unsafe extern "C" fn HourFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return pmod(floor(t / msPerHour), HoursPerDay) as ::core::ffi::c_int;
}
unsafe extern "C" fn MinFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return pmod(floor(t / msPerMinute), MinutesPerHour) as ::core::ffi::c_int;
}
unsafe extern "C" fn SecFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return pmod(floor(t / msPerSecond), SecondsPerMinute) as ::core::ffi::c_int;
}
unsafe extern "C" fn msFromTime(mut t: ::core::ffi::c_double) -> ::core::ffi::c_int {
    return pmod(t, msPerSecond) as ::core::ffi::c_int;
}
unsafe extern "C" fn MakeTime(
    mut hour: ::core::ffi::c_double,
    mut min: ::core::ffi::c_double,
    mut sec: ::core::ffi::c_double,
    mut ms: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return ((hour * MinutesPerHour + min) * SecondsPerMinute + sec) * msPerSecond + ms;
}
unsafe extern "C" fn MakeDay(
    mut y: ::core::ffi::c_double,
    mut m: ::core::ffi::c_double,
    mut date: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    static mut firstDayOfMonth: [[::core::ffi::c_double; 12]; 2] = [
        [
            0 as ::core::ffi::c_int as ::core::ffi::c_double,
            31 as ::core::ffi::c_int as ::core::ffi::c_double,
            59 as ::core::ffi::c_int as ::core::ffi::c_double,
            90 as ::core::ffi::c_int as ::core::ffi::c_double,
            120 as ::core::ffi::c_int as ::core::ffi::c_double,
            151 as ::core::ffi::c_int as ::core::ffi::c_double,
            181 as ::core::ffi::c_int as ::core::ffi::c_double,
            212 as ::core::ffi::c_int as ::core::ffi::c_double,
            243 as ::core::ffi::c_int as ::core::ffi::c_double,
            273 as ::core::ffi::c_int as ::core::ffi::c_double,
            304 as ::core::ffi::c_int as ::core::ffi::c_double,
            334 as ::core::ffi::c_int as ::core::ffi::c_double,
        ],
        [
            0 as ::core::ffi::c_int as ::core::ffi::c_double,
            31 as ::core::ffi::c_int as ::core::ffi::c_double,
            60 as ::core::ffi::c_int as ::core::ffi::c_double,
            91 as ::core::ffi::c_int as ::core::ffi::c_double,
            121 as ::core::ffi::c_int as ::core::ffi::c_double,
            152 as ::core::ffi::c_int as ::core::ffi::c_double,
            182 as ::core::ffi::c_int as ::core::ffi::c_double,
            213 as ::core::ffi::c_int as ::core::ffi::c_double,
            244 as ::core::ffi::c_int as ::core::ffi::c_double,
            274 as ::core::ffi::c_int as ::core::ffi::c_double,
            305 as ::core::ffi::c_int as ::core::ffi::c_double,
            335 as ::core::ffi::c_int as ::core::ffi::c_double,
        ],
    ];
    let mut yd: ::core::ffi::c_double = 0.;
    let mut md: ::core::ffi::c_double = 0.;
    let mut im: ::core::ffi::c_int = 0;
    y += floor(m / 12 as ::core::ffi::c_int as ::core::ffi::c_double);
    m = pmod(m, 12 as ::core::ffi::c_int as ::core::ffi::c_double);
    im = m as ::core::ffi::c_int;
    if im < 0 as ::core::ffi::c_int || im >= 12 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    yd = floor(TimeFromYear(y as ::core::ffi::c_int) / msPerDay);
    md = firstDayOfMonth[(DaysInYear(y as ::core::ffi::c_int)
        == 366 as ::core::ffi::c_int) as ::core::ffi::c_int as usize][im as usize];
    return yd + md + date - 1 as ::core::ffi::c_int as ::core::ffi::c_double;
}
unsafe extern "C" fn MakeDate(
    mut day: ::core::ffi::c_double,
    mut time_0: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
    return day * msPerDay + time_0;
}
unsafe extern "C" fn TimeClip(mut t: ::core::ffi::c_double) -> ::core::ffi::c_double {
    if t.is_finite() as i32 == 0 {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if fabs(t) > 8.64e15f64 {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    return if t < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        -floor(-t)
    } else {
        floor(t)
    };
}
unsafe extern "C" fn toint(
    mut sp: *mut *const ::core::ffi::c_char,
    mut w: ::core::ffi::c_int,
    mut v: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = *sp;
    *v = 0 as ::core::ffi::c_int;
    loop {
        let fresh6 = w;
        w = w - 1;
        if !(fresh6 != 0) {
            break;
        }
        if (*s as ::core::ffi::c_int) < '0' as i32
            || *s as ::core::ffi::c_int > '9' as i32
        {
            return 0 as ::core::ffi::c_int;
        }
        let fresh7 = s;
        s = s.offset(1);
        *v = *v * 10 as ::core::ffi::c_int
            + (*fresh7 as ::core::ffi::c_int - '0' as i32);
    }
    *sp = s;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn parseDateTime(
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    let mut y: ::core::ffi::c_int = 1970 as ::core::ffi::c_int;
    let mut m: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut d: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut H: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut M: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut S: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut ms: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut tza: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut t: ::core::ffi::c_double = 0.;
    if toint(&raw mut s, 4 as ::core::ffi::c_int, &raw mut y) == 0 {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if *s as ::core::ffi::c_int == '-' as i32 {
        s = s.offset(1 as ::core::ffi::c_int as isize);
        if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut m) == 0 {
            return ::core::f32::NAN as ::core::ffi::c_double;
        }
        if *s as ::core::ffi::c_int == '-' as i32 {
            s = s.offset(1 as ::core::ffi::c_int as isize);
            if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut d) == 0 {
                return ::core::f32::NAN as ::core::ffi::c_double;
            }
        }
    }
    if *s as ::core::ffi::c_int == 'T' as i32 {
        s = s.offset(1 as ::core::ffi::c_int as isize);
        if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut H) == 0 {
            return ::core::f32::NAN as ::core::ffi::c_double;
        }
        if *s as ::core::ffi::c_int != ':' as i32 {
            return ::core::f32::NAN as ::core::ffi::c_double;
        }
        s = s.offset(1 as ::core::ffi::c_int as isize);
        if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut M) == 0 {
            return ::core::f32::NAN as ::core::ffi::c_double;
        }
        if *s as ::core::ffi::c_int == ':' as i32 {
            s = s.offset(1 as ::core::ffi::c_int as isize);
            if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut S) == 0 {
                return ::core::f32::NAN as ::core::ffi::c_double;
            }
            if *s as ::core::ffi::c_int == '.' as i32 {
                s = s.offset(1 as ::core::ffi::c_int as isize);
                if toint(&raw mut s, 3 as ::core::ffi::c_int, &raw mut ms) == 0 {
                    return ::core::f32::NAN as ::core::ffi::c_double;
                }
            }
        }
        if *s as ::core::ffi::c_int == 'Z' as i32 {
            s = s.offset(1 as ::core::ffi::c_int as isize);
            tza = 0 as ::core::ffi::c_int;
        } else if *s as ::core::ffi::c_int == '+' as i32
            || *s as ::core::ffi::c_int == '-' as i32
        {
            let mut tzh: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut tzm: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut tzs: ::core::ffi::c_int = if *s as ::core::ffi::c_int == '+' as i32 {
                1 as ::core::ffi::c_int
            } else {
                -(1 as ::core::ffi::c_int)
            };
            s = s.offset(1 as ::core::ffi::c_int as isize);
            if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut tzh) == 0 {
                return ::core::f32::NAN as ::core::ffi::c_double;
            }
            if *s as ::core::ffi::c_int == ':' as i32 {
                s = s.offset(1 as ::core::ffi::c_int as isize);
                if toint(&raw mut s, 2 as ::core::ffi::c_int, &raw mut tzm) == 0 {
                    return ::core::f32::NAN as ::core::ffi::c_double;
                }
            }
            if tzh > 23 as ::core::ffi::c_int || tzm > 59 as ::core::ffi::c_int {
                return ::core::f32::NAN as ::core::ffi::c_double;
            }
            tza = (tzs as ::core::ffi::c_double
                * (tzh as ::core::ffi::c_double * msPerHour
                    + tzm as ::core::ffi::c_double * msPerMinute)) as ::core::ffi::c_int;
        } else {
            tza = LocalTZA() as ::core::ffi::c_int;
        }
    }
    if *s != 0 {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if m < 1 as ::core::ffi::c_int || m > 12 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if d < 1 as ::core::ffi::c_int || d > 31 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if H < 0 as ::core::ffi::c_int || H > 24 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if M < 0 as ::core::ffi::c_int || M > 59 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if S < 0 as ::core::ffi::c_int || S > 59 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if ms < 0 as ::core::ffi::c_int || ms > 999 as ::core::ffi::c_int {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    if H == 24 as ::core::ffi::c_int
        && (M != 0 as ::core::ffi::c_int || S != 0 as ::core::ffi::c_int
            || ms != 0 as ::core::ffi::c_int)
    {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    t = MakeDate(
        MakeDay(
            y as ::core::ffi::c_double,
            (m - 1 as ::core::ffi::c_int) as ::core::ffi::c_double,
            d as ::core::ffi::c_double,
        ),
        MakeTime(
            H as ::core::ffi::c_double,
            M as ::core::ffi::c_double,
            S as ::core::ffi::c_double,
            ms as ::core::ffi::c_double,
        ),
    );
    return t - tza as ::core::ffi::c_double;
}
unsafe extern "C" fn fmtdate(
    mut buf: *mut ::core::ffi::c_char,
    mut t: ::core::ffi::c_double,
) -> *mut ::core::ffi::c_char {
    let mut y: ::core::ffi::c_int = YearFromTime(t);
    let mut m: ::core::ffi::c_int = MonthFromTime(t);
    let mut d: ::core::ffi::c_int = DateFromTime(t);
    if t.is_finite() as i32 == 0 {
        return b"Invalid Date\0" as *const u8 as *const ::core::ffi::c_char
            as *mut ::core::ffi::c_char;
    }
    sprintf(
        buf,
        b"%04d-%02d-%02d\0" as *const u8 as *const ::core::ffi::c_char,
        y,
        m + 1 as ::core::ffi::c_int,
        d,
    );
    return buf;
}
unsafe extern "C" fn fmttime(
    mut buf: *mut ::core::ffi::c_char,
    mut t: ::core::ffi::c_double,
    mut tza: ::core::ffi::c_double,
) -> *mut ::core::ffi::c_char {
    let mut H: ::core::ffi::c_int = HourFromTime(t);
    let mut M: ::core::ffi::c_int = MinFromTime(t);
    let mut S: ::core::ffi::c_int = SecFromTime(t);
    let mut ms: ::core::ffi::c_int = msFromTime(t);
    let mut tzh: ::core::ffi::c_int = HourFromTime(fabs(tza));
    let mut tzm: ::core::ffi::c_int = MinFromTime(fabs(tza));
    if t.is_finite() as i32 == 0 {
        return b"Invalid Date\0" as *const u8 as *const ::core::ffi::c_char
            as *mut ::core::ffi::c_char;
    }
    if tza == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        sprintf(
            buf,
            b"%02d:%02d:%02d.%03dZ\0" as *const u8 as *const ::core::ffi::c_char,
            H,
            M,
            S,
            ms,
        );
    } else if tza < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        sprintf(
            buf,
            b"%02d:%02d:%02d.%03d-%02d:%02d\0" as *const u8
                as *const ::core::ffi::c_char,
            H,
            M,
            S,
            ms,
            tzh,
            tzm,
        );
    } else {
        sprintf(
            buf,
            b"%02d:%02d:%02d.%03d+%02d:%02d\0" as *const u8
                as *const ::core::ffi::c_char,
            H,
            M,
            S,
            ms,
            tzh,
            tzm,
        );
    }
    return buf;
}
unsafe extern "C" fn fmtdatetime(
    mut buf: *mut ::core::ffi::c_char,
    mut t: ::core::ffi::c_double,
    mut tza: ::core::ffi::c_double,
) -> *mut ::core::ffi::c_char {
    let mut dbuf: [::core::ffi::c_char; 20] = [0; 20];
    let mut tbuf: [::core::ffi::c_char; 20] = [0; 20];
    if t.is_finite() as i32 == 0 {
        return b"Invalid Date\0" as *const u8 as *const ::core::ffi::c_char
            as *mut ::core::ffi::c_char;
    }
    fmtdate(&raw mut dbuf as *mut ::core::ffi::c_char, t);
    fmttime(&raw mut tbuf as *mut ::core::ffi::c_char, t, tza);
    sprintf(
        buf,
        b"%sT%s\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut dbuf as *mut ::core::ffi::c_char,
        &raw mut tbuf as *mut ::core::ffi::c_char,
    );
    return buf;
}
unsafe extern "C" fn js_todate(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_double {
    let mut self_0: *mut js_Object = js_toobject(J, idx);
    if (*self_0).type_0 as ::core::ffi::c_uint
        != JS_CDATE as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_typeerror(J, b"not a date\0" as *const u8 as *const ::core::ffi::c_char);
    }
    return (*self_0).u.number;
}
unsafe extern "C" fn js_setdate(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut t: ::core::ffi::c_double,
) {
    let mut self_0: *mut js_Object = js_toobject(J, idx);
    if (*self_0).type_0 as ::core::ffi::c_uint
        != JS_CDATE as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_typeerror(J, b"not a date\0" as *const u8 as *const ::core::ffi::c_char);
    }
    (*self_0).u.number = TimeClip(t);
    js_pushnumber(J, (*self_0).u.number);
}
unsafe extern "C" fn D_parse(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = parseDateTime(
        js_tostring(J, 1 as ::core::ffi::c_int),
    );
    js_pushnumber(J, t);
}
unsafe extern "C" fn D_UTC(mut J: *mut js_State) {
    let mut y: ::core::ffi::c_double = 0.;
    let mut m: ::core::ffi::c_double = 0.;
    let mut d: ::core::ffi::c_double = 0.;
    let mut H: ::core::ffi::c_double = 0.;
    let mut M: ::core::ffi::c_double = 0.;
    let mut S: ::core::ffi::c_double = 0.;
    let mut ms: ::core::ffi::c_double = 0.;
    let mut t: ::core::ffi::c_double = 0.;
    y = js_tonumber(J, 1 as ::core::ffi::c_int);
    if y < 100 as ::core::ffi::c_int as ::core::ffi::c_double {
        y += 1900 as ::core::ffi::c_int as ::core::ffi::c_double;
    }
    m = js_tonumber(J, 2 as ::core::ffi::c_int);
    d = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        1 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    H = if js_isdefined(J, 4 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 4 as ::core::ffi::c_int)
    } else {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    M = if js_isdefined(J, 5 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 5 as ::core::ffi::c_int)
    } else {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    S = if js_isdefined(J, 6 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 6 as ::core::ffi::c_int)
    } else {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    ms = if js_isdefined(J, 7 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 7 as ::core::ffi::c_int)
    } else {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    };
    t = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
    t = TimeClip(t);
    js_pushnumber(J, t);
}
unsafe extern "C" fn D_now(mut J: *mut js_State) {
    js_pushnumber(J, Now());
}
unsafe extern "C" fn jsB_Date(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    js_pushstring(
        J,
        fmtdatetime(
            &raw mut buf as *mut ::core::ffi::c_char,
            LocalTime(Now()),
            LocalTZA(),
        ),
    );
}
unsafe extern "C" fn jsB_new_Date(mut J: *mut js_State) {
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut t: ::core::ffi::c_double = 0.;
    if top == 1 as ::core::ffi::c_int {
        t = Now();
    } else if top == 2 as ::core::ffi::c_int {
        js_toprimitive(J, 1 as ::core::ffi::c_int, JS_HNONE as ::core::ffi::c_int);
        if js_isstring(J, 1 as ::core::ffi::c_int) != 0 {
            t = parseDateTime(js_tostring(J, 1 as ::core::ffi::c_int));
        } else {
            t = TimeClip(js_tonumber(J, 1 as ::core::ffi::c_int));
        }
    } else {
        let mut y: ::core::ffi::c_double = 0.;
        let mut m: ::core::ffi::c_double = 0.;
        let mut d: ::core::ffi::c_double = 0.;
        let mut H: ::core::ffi::c_double = 0.;
        let mut M: ::core::ffi::c_double = 0.;
        let mut S: ::core::ffi::c_double = 0.;
        let mut ms: ::core::ffi::c_double = 0.;
        y = js_tonumber(J, 1 as ::core::ffi::c_int);
        if y < 100 as ::core::ffi::c_int as ::core::ffi::c_double {
            y += 1900 as ::core::ffi::c_int as ::core::ffi::c_double;
        }
        m = js_tonumber(J, 2 as ::core::ffi::c_int);
        d = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
            js_tonumber(J, 3 as ::core::ffi::c_int)
        } else {
            1 as ::core::ffi::c_int as ::core::ffi::c_double
        };
        H = if js_isdefined(J, 4 as ::core::ffi::c_int) != 0 {
            js_tonumber(J, 4 as ::core::ffi::c_int)
        } else {
            0 as ::core::ffi::c_int as ::core::ffi::c_double
        };
        M = if js_isdefined(J, 5 as ::core::ffi::c_int) != 0 {
            js_tonumber(J, 5 as ::core::ffi::c_int)
        } else {
            0 as ::core::ffi::c_int as ::core::ffi::c_double
        };
        S = if js_isdefined(J, 6 as ::core::ffi::c_int) != 0 {
            js_tonumber(J, 6 as ::core::ffi::c_int)
        } else {
            0 as ::core::ffi::c_int as ::core::ffi::c_double
        };
        ms = if js_isdefined(J, 7 as ::core::ffi::c_int) != 0 {
            js_tonumber(J, 7 as ::core::ffi::c_int)
        } else {
            0 as ::core::ffi::c_int as ::core::ffi::c_double
        };
        t = MakeDate(MakeDay(y, m, d), MakeTime(H, M, S, ms));
        t = TimeClip(UTC(t));
    }
    obj = jsV_newobject(J, JS_CDATE, (*J).Date_prototype);
    (*obj).u.number = t;
    js_pushobject(J, obj);
}
unsafe extern "C" fn Dp_valueOf(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    js_pushnumber(J, t);
}
unsafe extern "C" fn Dp_toString(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    js_pushstring(
        J,
        fmtdatetime(&raw mut buf as *mut ::core::ffi::c_char, LocalTime(t), LocalTZA()),
    );
}
unsafe extern "C" fn Dp_toDateString(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    js_pushstring(J, fmtdate(&raw mut buf as *mut ::core::ffi::c_char, LocalTime(t)));
}
unsafe extern "C" fn Dp_toTimeString(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    js_pushstring(
        J,
        fmttime(&raw mut buf as *mut ::core::ffi::c_char, LocalTime(t), LocalTZA()),
    );
}
unsafe extern "C" fn Dp_toUTCString(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    js_pushstring(
        J,
        fmtdatetime(
            &raw mut buf as *mut ::core::ffi::c_char,
            t,
            0 as ::core::ffi::c_int as ::core::ffi::c_double,
        ),
    );
}
unsafe extern "C" fn Dp_toISOString(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 64] = [0; 64];
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_finite() as i32 == 0 {
        js_rangeerror(J, b"invalid date\0" as *const u8 as *const ::core::ffi::c_char);
    }
    js_pushstring(
        J,
        fmtdatetime(
            &raw mut buf as *mut ::core::ffi::c_char,
            t,
            0 as ::core::ffi::c_int as ::core::ffi::c_double,
        ),
    );
}
unsafe extern "C" fn Dp_getFullYear(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, YearFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getMonth(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, MonthFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getDate(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, DateFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getDay(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, WeekDay(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getHours(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, HourFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getMinutes(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, MinFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getSeconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, SecFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getMilliseconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, msFromTime(LocalTime(t)) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCFullYear(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, YearFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCMonth(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, MonthFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCDate(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, DateFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCDay(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, WeekDay(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCHours(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, HourFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCMinutes(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, MinFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCSeconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, SecFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getUTCMilliseconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, msFromTime(t) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Dp_getTimezoneOffset(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    if t.is_nan() as i32 != 0 {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, (t - LocalTime(t)) / msPerMinute);
    };
}
unsafe extern "C" fn Dp_setTime(mut J: *mut js_State) {
    js_setdate(J, 0 as ::core::ffi::c_int, js_tonumber(J, 1 as ::core::ffi::c_int));
}
unsafe extern "C" fn Dp_setMilliseconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MinFromTime(t) as ::core::ffi::c_double;
    let mut s: ::core::ffi::c_double = SecFromTime(t) as ::core::ffi::c_double;
    let mut ms: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms))),
    );
}
unsafe extern "C" fn Dp_setSeconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MinFromTime(t) as ::core::ffi::c_double;
    let mut s: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms))),
    );
}
unsafe extern "C" fn Dp_setMinutes(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut s: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        SecFromTime(t) as ::core::ffi::c_double
    };
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms))),
    );
}
unsafe extern "C" fn Dp_setHours(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut h: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut m: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        MinFromTime(t) as ::core::ffi::c_double
    };
    let mut s: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        SecFromTime(t) as ::core::ffi::c_double
    };
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 4 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 4 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms))),
    );
}
unsafe extern "C" fn Dp_setDate(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut y: ::core::ffi::c_double = YearFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MonthFromTime(t) as ::core::ffi::c_double;
    let mut d: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))),
    );
}
unsafe extern "C" fn Dp_setMonth(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut y: ::core::ffi::c_double = YearFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut d: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        DateFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))),
    );
}
unsafe extern "C" fn Dp_setFullYear(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = LocalTime(js_todate(J, 0 as ::core::ffi::c_int));
    let mut y: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut m: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        MonthFromTime(t) as ::core::ffi::c_double
    };
    let mut d: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        DateFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        UTC(MakeDate(MakeDay(y, m, d), TimeWithinDay(t))),
    );
}
unsafe extern "C" fn Dp_setUTCMilliseconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MinFromTime(t) as ::core::ffi::c_double;
    let mut s: ::core::ffi::c_double = SecFromTime(t) as ::core::ffi::c_double;
    let mut ms: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms)),
    );
}
unsafe extern "C" fn Dp_setUTCSeconds(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MinFromTime(t) as ::core::ffi::c_double;
    let mut s: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms)),
    );
}
unsafe extern "C" fn Dp_setUTCMinutes(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut h: ::core::ffi::c_double = HourFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut s: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        SecFromTime(t) as ::core::ffi::c_double
    };
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms)),
    );
}
unsafe extern "C" fn Dp_setUTCHours(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut h: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut m: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        HourFromTime(t) as ::core::ffi::c_double
    };
    let mut s: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        SecFromTime(t) as ::core::ffi::c_double
    };
    let mut ms: ::core::ffi::c_double = if js_isdefined(J, 4 as ::core::ffi::c_int) != 0
    {
        js_tonumber(J, 4 as ::core::ffi::c_int)
    } else {
        msFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(
        J,
        0 as ::core::ffi::c_int,
        MakeDate(Day(t) as ::core::ffi::c_double, MakeTime(h, m, s, ms)),
    );
}
unsafe extern "C" fn Dp_setUTCDate(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut y: ::core::ffi::c_double = YearFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = MonthFromTime(t) as ::core::ffi::c_double;
    let mut d: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    js_setdate(J, 0 as ::core::ffi::c_int, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}
unsafe extern "C" fn Dp_setUTCMonth(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut y: ::core::ffi::c_double = YearFromTime(t) as ::core::ffi::c_double;
    let mut m: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut d: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        DateFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(J, 0 as ::core::ffi::c_int, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}
unsafe extern "C" fn Dp_setUTCFullYear(mut J: *mut js_State) {
    let mut t: ::core::ffi::c_double = js_todate(J, 0 as ::core::ffi::c_int);
    let mut y: ::core::ffi::c_double = js_tonumber(J, 1 as ::core::ffi::c_int);
    let mut m: ::core::ffi::c_double = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 2 as ::core::ffi::c_int)
    } else {
        MonthFromTime(t) as ::core::ffi::c_double
    };
    let mut d: ::core::ffi::c_double = if js_isdefined(J, 3 as ::core::ffi::c_int) != 0 {
        js_tonumber(J, 3 as ::core::ffi::c_int)
    } else {
        DateFromTime(t) as ::core::ffi::c_double
    };
    js_setdate(J, 0 as ::core::ffi::c_int, MakeDate(MakeDay(y, m, d), TimeWithinDay(t)));
}
unsafe extern "C" fn Dp_toJSON(mut J: *mut js_State) {
    js_copy(J, 0 as ::core::ffi::c_int);
    js_toprimitive(J, -(1 as ::core::ffi::c_int), JS_HNUMBER as ::core::ffi::c_int);
    if js_isnumber(J, -(1 as ::core::ffi::c_int)) != 0
        && js_tonumber(J, -(1 as ::core::ffi::c_int)).is_finite() as i32 == 0
    {
        js_pushnull(J);
        return;
    }
    js_pop(J, 1 as ::core::ffi::c_int);
    js_getproperty(
        J,
        0 as ::core::ffi::c_int,
        b"toISOString\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_iscallable(J, -(1 as ::core::ffi::c_int)) == 0 {
        js_typeerror(
            J,
            b"this.toISOString is not a function\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    js_copy(J, 0 as ::core::ffi::c_int);
    js_call(J, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn jsB_initdate(mut J: *mut js_State) {
    (*(*J).Date_prototype).u.number = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    js_pushobject(J, (*J).Date_prototype);
    jsB_propf(
        J,
        b"Date.prototype.valueOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_valueOf as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toDateString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toDateString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toTimeString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toTimeString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toLocaleString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toLocaleDateString\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Dp_toDateString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toLocaleTimeString\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Dp_toTimeString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toUTCString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toUTCString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getTime\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_valueOf as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getFullYear\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getFullYear as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCFullYear\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCFullYear as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getMonth\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getMonth as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCMonth\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCMonth as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getDate\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getDate as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCDate\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCDate as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getDay\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getDay as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCDay\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCDay as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getHours\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getHours as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCHours\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCHours as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getMinutes\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getMinutes as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCMinutes\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCMinutes as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getSeconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getSeconds as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCSeconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getUTCSeconds as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getMilliseconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getMilliseconds as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getUTCMilliseconds\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Dp_getUTCMilliseconds as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.getTimezoneOffset\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_getTimezoneOffset as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setTime\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setTime as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setMilliseconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setMilliseconds as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCMilliseconds\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Dp_setUTCMilliseconds as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setSeconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setSeconds as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCSeconds\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCSeconds as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setMinutes\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setMinutes as unsafe extern "C" fn(*mut js_State) -> ()),
        3 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCMinutes\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCMinutes as unsafe extern "C" fn(*mut js_State) -> ()),
        3 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setHours\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setHours as unsafe extern "C" fn(*mut js_State) -> ()),
        4 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCHours\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCHours as unsafe extern "C" fn(*mut js_State) -> ()),
        4 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setDate\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setDate as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCDate\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCDate as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setMonth\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setMonth as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCMonth\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCMonth as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setFullYear\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setFullYear as unsafe extern "C" fn(*mut js_State) -> ()),
        3 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.setUTCFullYear\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_setUTCFullYear as unsafe extern "C" fn(*mut js_State) -> ()),
        3 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toISOString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toISOString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.prototype.toJSON\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Dp_toJSON as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    js_newcconstructor(
        J,
        Some(jsB_Date as unsafe extern "C" fn(*mut js_State) -> ()),
        Some(jsB_new_Date as unsafe extern "C" fn(*mut js_State) -> ()),
        b"Date\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.parse\0" as *const u8 as *const ::core::ffi::c_char,
        Some(D_parse as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.UTC\0" as *const u8 as *const ::core::ffi::c_char,
        Some(D_UTC as unsafe extern "C" fn(*mut js_State) -> ()),
        7 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Date.now\0" as *const u8 as *const ::core::ffi::c_char,
        Some(D_now as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    js_defglobal(
        J,
        b"Date\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
}
