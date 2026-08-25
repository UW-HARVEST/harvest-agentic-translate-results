extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
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
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_itoa(
        buf: *mut ::core::ffi::c_char,
        a: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn jsV_numbertointeger(n: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn jsV_numbertostring(
        J: *mut js_State,
        buf: *mut ::core::ffi::c_char,
        number: ::core::ffi::c_double,
    ) -> *const ::core::ffi::c_char;
    fn jsV_stringtonumber(
        J: *mut js_State,
        string: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
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
pub type __compar_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
pub type js_Type = ::core::ffi::c_uint;
pub const JS_TOBJECT: js_Type = 7;
pub const JS_TMEMSTR: js_Type = 6;
pub const JS_TLITSTR: js_Type = 5;
pub const JS_TNUMBER: js_Type = 4;
pub const JS_TBOOLEAN: js_Type = 3;
pub const JS_TNULL: js_Type = 2;
pub const JS_TUNDEFINED: js_Type = 1;
pub const JS_TSHRSTR: js_Type = 0;
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
    return vfprintf(stdout, __fmt, __arg);
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
static mut sentinel: js_Property = unsafe {
    js_Property {
        left: &raw const sentinel as *mut js_Property,
        right: &raw const sentinel as *mut js_Property,
        level: 0 as ::core::ffi::c_int,
        atts: 0 as ::core::ffi::c_int,
        value: js_Value {
            t: C2RustUnnamed_6 {
                pad: [
                    0 as ::core::ffi::c_int as ::core::ffi::c_char,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
                type_0: JS_TUNDEFINED as ::core::ffi::c_int as ::core::ffi::c_char,
            },
        },
        getter: ::core::ptr::null::<js_Object>() as *mut js_Object,
        setter: ::core::ptr::null::<js_Object>() as *mut js_Object,
        name: ::core::mem::transmute::<[u8; 1], [::core::ffi::c_char; 1]>(*b"\0"),
    }
};
unsafe extern "C" fn newproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    let mut n: ::core::ffi::c_int = strlen(name).wrapping_add(1 as size_t)
        as ::core::ffi::c_int;
    let mut node: *mut js_Property = js_malloc(
        J,
        (56 as ::core::ffi::c_ulong).wrapping_add(n as ::core::ffi::c_ulong)
            as ::core::ffi::c_int,
    ) as *mut js_Property;
    (*node).right = &raw mut sentinel;
    (*node).left = (*node).right;
    (*node).level = 1 as ::core::ffi::c_int;
    (*node).atts = 0 as ::core::ffi::c_int;
    (*node).value.t.type_0 = JS_TUNDEFINED as ::core::ffi::c_int as ::core::ffi::c_char;
    (*node).value.u.number = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*node).getter = ::core::ptr::null_mut::<js_Object>();
    (*node).setter = ::core::ptr::null_mut::<js_Object>();
    memcpy(
        &raw mut (*node).name as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        name as *const ::core::ffi::c_void,
        n as size_t,
    );
    (*obj).count += 1;
    (*J).gccounter = (*J).gccounter.wrapping_add(1);
    return node;
}
unsafe extern "C" fn lookup(
    mut node: *mut js_Property,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    while node != &raw mut sentinel {
        let mut c: ::core::ffi::c_int = strcmp(
            name,
            &raw mut (*node).name as *mut ::core::ffi::c_char,
        );
        if c == 0 as ::core::ffi::c_int {
            return node
        } else if c < 0 as ::core::ffi::c_int {
            node = (*node).left;
        } else {
            node = (*node).right;
        }
    }
    return ::core::ptr::null_mut::<js_Property>();
}
unsafe extern "C" fn skew(mut node: *mut js_Property) -> *mut js_Property {
    if (*(*node).left).level == (*node).level {
        let mut temp: *mut js_Property = node;
        node = (*node).left;
        (*temp).left = (*node).right;
        (*node).right = temp;
    }
    return node;
}
unsafe extern "C" fn split(mut node: *mut js_Property) -> *mut js_Property {
    if (*(*(*node).right).right).level == (*node).level {
        let mut temp: *mut js_Property = node;
        node = (*node).right;
        (*temp).right = (*node).left;
        (*node).left = temp;
        (*node).level += 1;
    }
    return node;
}
unsafe extern "C" fn insert(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut node: *mut js_Property,
    mut name: *const ::core::ffi::c_char,
    mut result: *mut *mut js_Property,
) -> *mut js_Property {
    if node != &raw mut sentinel {
        let mut c: ::core::ffi::c_int = strcmp(
            name,
            &raw mut (*node).name as *mut ::core::ffi::c_char,
        );
        if c < 0 as ::core::ffi::c_int {
            (*node).left = insert(J, obj, (*node).left, name, result);
        } else if c > 0 as ::core::ffi::c_int {
            (*node).right = insert(J, obj, (*node).right, name, result);
        } else {
            *result = node;
            return *result;
        }
        node = skew(node);
        node = split(node);
        return node;
    }
    *result = newproperty(J, obj, name);
    return *result;
}
unsafe extern "C" fn freeproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut node: *mut js_Property,
) {
    js_free(J, node as *mut ::core::ffi::c_void);
    (*obj).count -= 1;
}
unsafe extern "C" fn unlinkproperty(
    mut node: *mut js_Property,
    mut name: *const ::core::ffi::c_char,
    mut garbage: *mut *mut js_Property,
) -> *mut js_Property {
    let mut temp: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut a: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut b: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    if node != &raw mut sentinel {
        let mut c: ::core::ffi::c_int = strcmp(
            name,
            &raw mut (*node).name as *mut ::core::ffi::c_char,
        );
        if c < 0 as ::core::ffi::c_int {
            (*node).left = unlinkproperty((*node).left, name, garbage);
        } else if c > 0 as ::core::ffi::c_int {
            (*node).right = unlinkproperty((*node).right, name, garbage);
        } else {
            *garbage = node;
            if (*node).left == &raw mut sentinel && (*node).right == &raw mut sentinel {
                return &raw mut sentinel
            } else if (*node).left == &raw mut sentinel {
                a = (*node).right;
                while (*a).left != &raw mut sentinel {
                    a = (*a).left;
                }
                b = unlinkproperty(
                    (*node).right,
                    &raw mut (*a).name as *mut ::core::ffi::c_char,
                    &raw mut temp,
                );
                (*temp).level = (*node).level;
                (*temp).left = (*node).left;
                (*temp).right = b;
                node = temp;
            } else {
                a = (*node).left;
                while (*a).right != &raw mut sentinel {
                    a = (*a).right;
                }
                b = unlinkproperty(
                    (*node).left,
                    &raw mut (*a).name as *mut ::core::ffi::c_char,
                    &raw mut temp,
                );
                (*temp).level = (*node).level;
                (*temp).left = b;
                (*temp).right = (*node).right;
                node = temp;
            }
        }
        if (*(*node).left).level < (*node).level - 1 as ::core::ffi::c_int
            || (*(*node).right).level < (*node).level - 1 as ::core::ffi::c_int
        {
            (*node).level -= 1;
            if (*(*node).right).level > (*node).level {
                (*(*node).right).level = (*node).level;
            }
            node = skew(node);
            (*node).right = skew((*node).right);
            (*(*node).right).right = skew((*(*node).right).right);
            node = split(node);
            (*node).right = split((*node).right);
        }
    }
    return node;
}
unsafe extern "C" fn deleteproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut tree: *mut js_Property,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    let mut garbage: *mut js_Property = &raw mut sentinel;
    tree = unlinkproperty(tree, name, &raw mut garbage);
    if garbage != &raw mut sentinel {
        freeproperty(J, obj, garbage);
    }
    return tree;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_newobject(
    mut J: *mut js_State,
    mut type_0: js_Class,
    mut prototype: *mut js_Object,
) -> *mut js_Object {
    let mut obj: *mut js_Object = js_malloc(
        J,
        ::core::mem::size_of::<js_Object>() as ::core::ffi::c_int,
    ) as *mut js_Object;
    memset(
        obj as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<js_Object>() as size_t,
    );
    (*obj).gcmark = 0 as ::core::ffi::c_int;
    (*obj).gcnext = (*J).gcobj;
    (*J).gcobj = obj;
    (*J).gccounter = (*J).gccounter.wrapping_add(1);
    (*obj).type_0 = type_0;
    (*obj).properties = &raw mut sentinel;
    (*obj).prototype = prototype;
    (*obj).extensible = 1 as ::core::ffi::c_int;
    return obj;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_getownproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    return lookup((*obj).properties, name);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_getpropertyx(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
    mut own: *mut ::core::ffi::c_int,
) -> *mut js_Property {
    *own = 1 as ::core::ffi::c_int;
    loop {
        let mut ref_0: *mut js_Property = lookup((*obj).properties, name);
        if !ref_0.is_null() {
            return ref_0;
        }
        obj = (*obj).prototype;
        *own = 0 as ::core::ffi::c_int;
        if obj.is_null() {
            break;
        }
    }
    return ::core::ptr::null_mut::<js_Property>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_getproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    loop {
        let mut ref_0: *mut js_Property = lookup((*obj).properties, name);
        if !ref_0.is_null() {
            return ref_0;
        }
        obj = (*obj).prototype;
        if obj.is_null() {
            break;
        }
    }
    return ::core::ptr::null_mut::<js_Property>();
}
unsafe extern "C" fn jsV_getenumproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    loop {
        let mut ref_0: *mut js_Property = lookup((*obj).properties, name);
        if !ref_0.is_null() && (*ref_0).atts & JS_DONTENUM as ::core::ffi::c_int == 0 {
            return ref_0;
        }
        obj = (*obj).prototype;
        if obj.is_null() {
            break;
        }
    }
    return ::core::ptr::null_mut::<js_Property>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_setproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> *mut js_Property {
    let mut result: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    if (*obj).extensible == 0 {
        result = lookup((*obj).properties, name);
        if (*J).strict != 0 && result.is_null() {
            js_typeerror(
                J,
                b"object is non-extensible\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        return result;
    }
    (*obj).properties = insert(J, obj, (*obj).properties, name, &raw mut result);
    return result;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_delproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) {
    (*obj).properties = deleteproperty(J, obj, (*obj).properties, name);
}
unsafe extern "C" fn itnewnode(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
    mut next: *mut js_Iterator,
) -> *mut js_Iterator {
    let mut n: ::core::ffi::c_int = strlen(name).wrapping_add(1 as size_t)
        as ::core::ffi::c_int;
    let mut node: *mut js_Iterator = js_malloc(
        J,
        (8 as ::core::ffi::c_ulong).wrapping_add(n as ::core::ffi::c_ulong)
            as ::core::ffi::c_int,
    ) as *mut js_Iterator;
    (*node).next = next;
    memcpy(
        &raw mut (*node).name as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        name as *const ::core::ffi::c_void,
        n as size_t,
    );
    return node;
}
unsafe extern "C" fn itwalk(
    mut J: *mut js_State,
    mut iter: *mut js_Iterator,
    mut prop: *mut js_Property,
    mut seen: *mut js_Object,
) -> *mut js_Iterator {
    if (*prop).right != &raw mut sentinel {
        iter = itwalk(J, iter, (*prop).right, seen);
    }
    if (*prop).atts & JS_DONTENUM as ::core::ffi::c_int == 0 {
        if seen.is_null()
            || jsV_getenumproperty(
                    J,
                    seen,
                    &raw mut (*prop).name as *mut ::core::ffi::c_char,
                )
                .is_null()
        {
            iter = itnewnode(J, &raw mut (*prop).name as *mut ::core::ffi::c_char, iter);
        }
    }
    if (*prop).left != &raw mut sentinel {
        iter = itwalk(J, iter, (*prop).left, seen);
    }
    return iter;
}
unsafe extern "C" fn itflatten(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
) -> *mut js_Iterator {
    let mut iter: *mut js_Iterator = ::core::ptr::null_mut::<js_Iterator>();
    if !(*obj).prototype.is_null() {
        iter = itflatten(J, (*obj).prototype);
    }
    if (*obj).properties != &raw mut sentinel {
        iter = itwalk(J, iter, (*obj).properties, (*obj).prototype);
    }
    return iter;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_newiterator(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut own: ::core::ffi::c_int,
) -> *mut js_Object {
    let mut io: *mut js_Object = jsV_newobject(
        J,
        JS_CITERATOR,
        ::core::ptr::null_mut::<js_Object>(),
    );
    (*io).u.iter.target = obj;
    (*io).u.iter.i = 0 as ::core::ffi::c_int;
    (*io).u.iter.n = 0 as ::core::ffi::c_int;
    if own != 0 {
        (*io).u.iter.head = ::core::ptr::null_mut::<js_Iterator>();
        if (*obj).properties != &raw mut sentinel {
            (*io).u.iter.head = itwalk(
                J,
                (*io).u.iter.head,
                (*obj).properties,
                ::core::ptr::null_mut::<js_Object>(),
            );
        }
    } else {
        (*io).u.iter.head = itflatten(J, obj);
    }
    (*io).u.iter.current = (*io).u.iter.head;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (*io).u.iter.n = (*obj).u.s.length;
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0
    {
        (*io).u.iter.n = (*obj).u.a.flat_length;
    }
    return io;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_nextiterator(
    mut J: *mut js_State,
    mut io: *mut js_Object,
) -> *const ::core::ffi::c_char {
    if (*io).type_0 as ::core::ffi::c_uint
        != JS_CITERATOR as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_typeerror(J, b"not an iterator\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*io).u.iter.i < (*io).u.iter.n {
        js_itoa(&raw mut (*J).scratch as *mut ::core::ffi::c_char, (*io).u.iter.i);
        (*io).u.iter.i += 1;
        return &raw mut (*J).scratch as *mut ::core::ffi::c_char;
    }
    while !(*io).u.iter.current.is_null() {
        let mut name: *const ::core::ffi::c_char = &raw mut (*(*io).u.iter.current).name
            as *mut ::core::ffi::c_char;
        (*io).u.iter.current = (*(*io).u.iter.current).next;
        if !jsV_getproperty(J, (*io).u.iter.target, name).is_null() {
            return name;
        }
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_resizearray(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut newlen: ::core::ffi::c_int,
) {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut k: ::core::ffi::c_int = 0;
    if newlen < (*obj).u.a.length {
        if (*obj).u.a.length > (*obj).count * 2 as ::core::ffi::c_int {
            let mut it: *mut js_Object = jsV_newiterator(
                J,
                obj,
                1 as ::core::ffi::c_int,
            );
            loop {
                s = jsV_nextiterator(J, it);
                if s.is_null() {
                    break;
                }
                k = jsV_numbertointeger(jsV_stringtonumber(J, s));
                if k >= newlen
                    && strcmp(
                        s,
                        jsV_numbertostring(
                            J,
                            &raw mut buf as *mut ::core::ffi::c_char,
                            k as ::core::ffi::c_double,
                        ),
                    ) == 0
                {
                    jsV_delproperty(J, obj, s);
                }
            }
        } else {
            k = newlen;
            while k < (*obj).u.a.length {
                jsV_delproperty(
                    J,
                    obj,
                    js_itoa(&raw mut buf as *mut ::core::ffi::c_char, k),
                );
                k += 1;
            }
        }
    }
    (*obj).u.a.length = newlen;
}
