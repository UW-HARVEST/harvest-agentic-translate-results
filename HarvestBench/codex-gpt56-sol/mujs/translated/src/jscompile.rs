extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn js_newsyntaxerror(J: *mut js_State, message: *const ::core::ffi::c_char);
    fn js_evalerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_syntaxerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_throw(J: *mut js_State) -> !;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn vsnprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
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
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_realloc(
        J: *mut js_State,
        ptr: *mut ::core::ffi::c_void,
        size: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_intern(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    fn jsV_numbertostring(
        J: *mut js_State,
        buf: *mut ::core::ffi::c_char,
        number: ::core::ffi::c_double,
    ) -> *const ::core::ffi::c_char;
    fn jsY_findword(
        s: *const ::core::ffi::c_char,
        list: *mut *const ::core::ffi::c_char,
        num: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
pub type __builtin_va_list = [__va_list_tag; 1];
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
pub type size_t = usize;
pub type va_list = __builtin_va_list;
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
pub type js_OpCode = ::core::ffi::c_uint;
pub const OP_RETURN: js_OpCode = 84;
pub const OP_JFALSE: js_OpCode = 83;
pub const OP_JTRUE: js_OpCode = 82;
pub const OP_JUMP: js_OpCode = 81;
pub const OP_DEBUGGER: js_OpCode = 80;
pub const OP_ENDWITH: js_OpCode = 79;
pub const OP_WITH: js_OpCode = 78;
pub const OP_ENDCATCH: js_OpCode = 77;
pub const OP_CATCH: js_OpCode = 76;
pub const OP_ENDTRY: js_OpCode = 75;
pub const OP_TRY: js_OpCode = 74;
pub const OP_THROW: js_OpCode = 73;
pub const OP_INSTANCEOF: js_OpCode = 72;
pub const OP_BITOR: js_OpCode = 71;
pub const OP_BITXOR: js_OpCode = 70;
pub const OP_BITAND: js_OpCode = 69;
pub const OP_JCASE: js_OpCode = 68;
pub const OP_STRICTNE: js_OpCode = 67;
pub const OP_STRICTEQ: js_OpCode = 66;
pub const OP_NE: js_OpCode = 65;
pub const OP_EQ: js_OpCode = 64;
pub const OP_GE: js_OpCode = 63;
pub const OP_LE: js_OpCode = 62;
pub const OP_GT: js_OpCode = 61;
pub const OP_LT: js_OpCode = 60;
pub const OP_USHR: js_OpCode = 59;
pub const OP_SHR: js_OpCode = 58;
pub const OP_SHL: js_OpCode = 57;
pub const OP_SUB: js_OpCode = 56;
pub const OP_ADD: js_OpCode = 55;
pub const OP_MOD: js_OpCode = 54;
pub const OP_DIV: js_OpCode = 53;
pub const OP_MUL: js_OpCode = 52;
pub const OP_POSTDEC: js_OpCode = 51;
pub const OP_POSTINC: js_OpCode = 50;
pub const OP_DEC: js_OpCode = 49;
pub const OP_INC: js_OpCode = 48;
pub const OP_LOGNOT: js_OpCode = 47;
pub const OP_BITNOT: js_OpCode = 46;
pub const OP_NEG: js_OpCode = 45;
pub const OP_POS: js_OpCode = 44;
pub const OP_TYPEOF: js_OpCode = 43;
pub const OP_NEW: js_OpCode = 42;
pub const OP_CALL: js_OpCode = 41;
pub const OP_EVAL: js_OpCode = 40;
pub const OP_NEXTITER: js_OpCode = 39;
pub const OP_ITERATOR: js_OpCode = 38;
pub const OP_DELPROP_S: js_OpCode = 37;
pub const OP_DELPROP: js_OpCode = 36;
pub const OP_SETPROP_S: js_OpCode = 35;
pub const OP_SETPROP: js_OpCode = 34;
pub const OP_GETPROP_S: js_OpCode = 33;
pub const OP_GETPROP: js_OpCode = 32;
pub const OP_INITSETTER: js_OpCode = 31;
pub const OP_INITGETTER: js_OpCode = 30;
pub const OP_INITPROP: js_OpCode = 29;
pub const OP_INITARRAY: js_OpCode = 28;
pub const OP_SKIPARRAY: js_OpCode = 27;
pub const OP_IN: js_OpCode = 26;
pub const OP_DELVAR: js_OpCode = 25;
pub const OP_SETVAR: js_OpCode = 24;
pub const OP_GETVAR: js_OpCode = 23;
pub const OP_HASVAR: js_OpCode = 22;
pub const OP_DELLOCAL: js_OpCode = 21;
pub const OP_SETLOCAL: js_OpCode = 20;
pub const OP_GETLOCAL: js_OpCode = 19;
pub const OP_CURRENT: js_OpCode = 18;
pub const OP_THIS: js_OpCode = 17;
pub const OP_FALSE: js_OpCode = 16;
pub const OP_TRUE: js_OpCode = 15;
pub const OP_NULL: js_OpCode = 14;
pub const OP_UNDEF: js_OpCode = 13;
pub const OP_NEWREGEXP: js_OpCode = 12;
pub const OP_NEWOBJECT: js_OpCode = 11;
pub const OP_NEWARRAY: js_OpCode = 10;
pub const OP_CLOSURE: js_OpCode = 9;
pub const OP_STRING: js_OpCode = 8;
pub const OP_NUMBER: js_OpCode = 7;
pub const OP_INTEGER: js_OpCode = 6;
pub const OP_ROT4: js_OpCode = 5;
pub const OP_ROT3: js_OpCode = 4;
pub const OP_ROT2: js_OpCode = 3;
pub const OP_DUP2: js_OpCode = 2;
pub const OP_DUP: js_OpCode = 1;
pub const OP_POP: js_OpCode = 0;
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_error(
    mut J: *mut js_State,
    mut node: *mut js_Ast,
    mut fmt: *const ::core::ffi::c_char,
    mut args: ...
) -> ! {
    let mut ap: ::core::ffi::VaList;
    let mut buf: [::core::ffi::c_char; 512] = [0; 512];
    let mut msgbuf: [::core::ffi::c_char; 256] = [0; 256];
    ap = args.clone();
    vsnprintf(
        &raw mut msgbuf as *mut ::core::ffi::c_char,
        256 as size_t,
        fmt,
        ap,
    );
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        256 as size_t,
        b"%s:%d: \0" as *const u8 as *const ::core::ffi::c_char,
        (*J).filename,
        (*node).line,
    );
    strcat(
        &raw mut buf as *mut ::core::ffi::c_char,
        &raw mut msgbuf as *mut ::core::ffi::c_char,
    );
    js_newsyntaxerror(J, &raw mut buf as *mut ::core::ffi::c_char);
    js_throw(J);
}
static mut futurewords: [*const ::core::ffi::c_char; 7] = [
    b"class\0" as *const u8 as *const ::core::ffi::c_char,
    b"const\0" as *const u8 as *const ::core::ffi::c_char,
    b"enum\0" as *const u8 as *const ::core::ffi::c_char,
    b"export\0" as *const u8 as *const ::core::ffi::c_char,
    b"extends\0" as *const u8 as *const ::core::ffi::c_char,
    b"import\0" as *const u8 as *const ::core::ffi::c_char,
    b"super\0" as *const u8 as *const ::core::ffi::c_char,
];
static mut strictfuturewords: [*const ::core::ffi::c_char; 9] = [
    b"implements\0" as *const u8 as *const ::core::ffi::c_char,
    b"interface\0" as *const u8 as *const ::core::ffi::c_char,
    b"let\0" as *const u8 as *const ::core::ffi::c_char,
    b"package\0" as *const u8 as *const ::core::ffi::c_char,
    b"private\0" as *const u8 as *const ::core::ffi::c_char,
    b"protected\0" as *const u8 as *const ::core::ffi::c_char,
    b"public\0" as *const u8 as *const ::core::ffi::c_char,
    b"static\0" as *const u8 as *const ::core::ffi::c_char,
    b"yield\0" as *const u8 as *const ::core::ffi::c_char,
];
unsafe extern "C" fn checkfutureword(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
) {
    if jsY_findword(
        (*exp).string,
        &raw mut futurewords as *mut *const ::core::ffi::c_char,
        (::core::mem::size_of::<[*const ::core::ffi::c_char; 7]>() as usize)
            .wrapping_div(::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
            as ::core::ffi::c_int,
    ) >= 0 as ::core::ffi::c_int
    {
        jsC_error(
            J,
            exp,
            b"'%s' is a future reserved word\0" as *const u8
                as *const ::core::ffi::c_char,
            (*exp).string,
        );
    }
    if (*F).strict != 0 {
        if jsY_findword(
            (*exp).string,
            &raw mut strictfuturewords as *mut *const ::core::ffi::c_char,
            (::core::mem::size_of::<[*const ::core::ffi::c_char; 9]>() as usize)
                .wrapping_div(
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as usize,
                ) as ::core::ffi::c_int,
        ) >= 0 as ::core::ffi::c_int
        {
            jsC_error(
                J,
                exp,
                b"'%s' is a strict mode future reserved word\0" as *const u8
                    as *const ::core::ffi::c_char,
                (*exp).string,
            );
        }
    }
}
unsafe extern "C" fn newfun(
    mut J: *mut js_State,
    mut line: ::core::ffi::c_int,
    mut name: *mut js_Ast,
    mut params: *mut js_Ast,
    mut body: *mut js_Ast,
    mut script: ::core::ffi::c_int,
    mut default_strict: ::core::ffi::c_int,
    mut is_fun_exp: ::core::ffi::c_int,
) -> *mut js_Function {
    let mut F: *mut js_Function = js_malloc(
        J,
        ::core::mem::size_of::<js_Function>() as ::core::ffi::c_int,
    ) as *mut js_Function;
    memset(
        F as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<js_Function>() as size_t,
    );
    (*F).gcmark = 0 as ::core::ffi::c_int;
    (*F).gcnext = (*J).gcfun;
    (*J).gcfun = F;
    (*J).gccounter = (*J).gccounter.wrapping_add(1);
    (*F).filename = js_intern(J, (*J).filename);
    (*F).line = line;
    (*F).script = script;
    (*F).strict = default_strict;
    (*F).name = if !name.is_null() {
        (*name).string
    } else {
        b"\0" as *const u8 as *const ::core::ffi::c_char
    };
    cfunbody(J, F, name, params, body, is_fun_exp);
    return F;
}
unsafe extern "C" fn emitraw(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut value: ::core::ffi::c_int,
) {
    if value != value as js_Instruction as ::core::ffi::c_int {
        js_syntaxerror(
            J,
            b"integer overflow in instruction coding\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if (*F).codelen >= (*F).codecap {
        (*F).codecap = if (*F).codecap != 0 {
            (*F).codecap * 2 as ::core::ffi::c_int
        } else {
            64 as ::core::ffi::c_int
        };
        (*F).code = js_realloc(
            J,
            (*F).code as *mut ::core::ffi::c_void,
            ((*F).codecap as usize)
                .wrapping_mul(::core::mem::size_of::<js_Instruction>() as usize)
                as ::core::ffi::c_int,
        ) as *mut js_Instruction;
    }
    let fresh6 = (*F).codelen;
    (*F).codelen = (*F).codelen + 1;
    *(*F).code.offset(fresh6 as isize) = value as js_Instruction;
}
unsafe extern "C" fn emit(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut value: ::core::ffi::c_int,
) {
    emitraw(J, F, (*F).lastline);
    emitraw(J, F, value);
}
unsafe extern "C" fn emitarg(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut value: ::core::ffi::c_int,
) {
    emitraw(J, F, value);
}
unsafe extern "C" fn emitline(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut node: *mut js_Ast,
) {
    (*F).lastline = (*node).line;
}
unsafe extern "C" fn addfunction(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut value: *mut js_Function,
) -> ::core::ffi::c_int {
    if (*F).funlen >= (*F).funcap {
        (*F).funcap = if (*F).funcap != 0 {
            (*F).funcap * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*F).funtab = js_realloc(
            J,
            (*F).funtab as *mut ::core::ffi::c_void,
            ((*F).funcap as usize)
                .wrapping_mul(::core::mem::size_of::<*mut js_Function>() as usize)
                as ::core::ffi::c_int,
        ) as *mut *mut js_Function;
    }
    let ref mut fresh8 = *(*F).funtab.offset((*F).funlen as isize);
    *fresh8 = value;
    let fresh9 = (*F).funlen;
    (*F).funlen = (*F).funlen + 1;
    return fresh9;
}
unsafe extern "C" fn addlocal(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut ident: *mut js_Ast,
    mut reuse: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut name: *const ::core::ffi::c_char = (*ident).string;
    if (*F).strict != 0 {
        if strcmp(name, b"arguments\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            jsC_error(
                J,
                ident,
                b"redefining 'arguments' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if strcmp(name, b"eval\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            jsC_error(
                J,
                ident,
                b"redefining 'eval' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    } else if strcmp(name, b"eval\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
        js_evalerror(
            J,
            b"%s:%d: invalid use of 'eval'\0" as *const u8 as *const ::core::ffi::c_char,
            (*J).filename,
            (*ident).line,
        );
    }
    if reuse != 0 || (*F).strict != 0 {
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while i < (*F).varlen {
            if strcmp(*(*F).vartab.offset(i as isize), name) == 0 {
                if reuse != 0 {
                    return i + 1 as ::core::ffi::c_int;
                }
                if (*F).strict != 0 {
                    jsC_error(
                        J,
                        ident,
                        b"duplicate formal parameter '%s'\0" as *const u8
                            as *const ::core::ffi::c_char,
                        name,
                    );
                }
            }
            i += 1;
        }
    }
    if (*F).varlen >= (*F).varcap {
        (*F).varcap = if (*F).varcap != 0 {
            (*F).varcap * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*F).vartab = js_realloc(
            J,
            (*F).vartab as *mut ::core::ffi::c_void,
            ((*F).varcap as usize)
                .wrapping_mul(
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as usize,
                ) as ::core::ffi::c_int,
        ) as *mut *const ::core::ffi::c_char;
    }
    let ref mut fresh10 = *(*F).vartab.offset((*F).varlen as isize);
    *fresh10 = name;
    (*F).varlen += 1;
    return (*F).varlen;
}
unsafe extern "C" fn findlocal(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    i = (*F).varlen;
    while i > 0 as ::core::ffi::c_int {
        if strcmp(*(*F).vartab.offset((i - 1 as ::core::ffi::c_int) as isize), name) == 0
        {
            return i;
        }
        i -= 1;
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn emitfunction(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut fun: *mut js_Function,
) {
    (*F).lightweight = 0 as ::core::ffi::c_int;
    emit(J, F, OP_CLOSURE as ::core::ffi::c_int);
    emitarg(J, F, addfunction(J, F, fun));
}
unsafe extern "C" fn emitnumber(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut num: ::core::ffi::c_double,
) {
    if num == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        emit(J, F, OP_INTEGER as ::core::ffi::c_int);
        emitarg(J, F, 32768 as ::core::ffi::c_int);
        if num.is_sign_negative() as ::core::ffi::c_int != 0 {
            emit(J, F, OP_NEG as ::core::ffi::c_int);
        }
    } else if num >= SHRT_MIN as ::core::ffi::c_double
        && num <= SHRT_MAX as ::core::ffi::c_double
        && num == num as ::core::ffi::c_int as ::core::ffi::c_double
    {
        emit(J, F, OP_INTEGER as ::core::ffi::c_int);
        emitarg(
            J,
            F,
            (num + 32768 as ::core::ffi::c_int as ::core::ffi::c_double)
                as ::core::ffi::c_int,
        );
    } else {
        let mut x: [js_Instruction; 4] = [0; 4];
        let mut i: size_t = 0;
        emit(J, F, OP_NUMBER as ::core::ffi::c_int);
        memcpy(
            &raw mut x as *mut js_Instruction as *mut ::core::ffi::c_void,
            &raw mut num as *const ::core::ffi::c_void,
            ::core::mem::size_of::<::core::ffi::c_double>() as size_t,
        );
        i = 0 as size_t;
        while i < N_0 {
            emitarg(J, F, x[i as usize] as ::core::ffi::c_int);
            i = i.wrapping_add(1);
        }
    };
}
pub const N_0: usize = (::core::mem::size_of::<::core::ffi::c_double>() as usize)
    .wrapping_div(::core::mem::size_of::<js_Instruction>() as usize);
unsafe extern "C" fn emitstring(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut opcode: ::core::ffi::c_int,
    mut str: *const ::core::ffi::c_char,
) {
    let mut x: [js_Instruction; 4] = [0; 4];
    let mut i: size_t = 0;
    emit(J, F, opcode);
    memcpy(
        &raw mut x as *mut js_Instruction as *mut ::core::ffi::c_void,
        &raw mut str as *const ::core::ffi::c_void,
        ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
    );
    i = 0 as size_t;
    while i < N {
        emitarg(J, F, x[i as usize] as ::core::ffi::c_int);
        i = i.wrapping_add(1);
    }
}
pub const N: usize = (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
    .wrapping_div(::core::mem::size_of::<js_Instruction>() as usize);
unsafe extern "C" fn emitlocal(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut oploc: ::core::ffi::c_int,
    mut opvar: ::core::ffi::c_int,
    mut ident: *mut js_Ast,
) {
    let mut is_arguments: ::core::ffi::c_int = (strcmp(
        (*ident).string,
        b"arguments\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0) as ::core::ffi::c_int;
    let mut is_eval: ::core::ffi::c_int = (strcmp(
        (*ident).string,
        b"eval\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0) as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0;
    if is_arguments != 0 {
        (*F).lightweight = 0 as ::core::ffi::c_int;
        (*F).arguments = 1 as ::core::ffi::c_int;
    }
    checkfutureword(J, F, ident);
    if (*F).strict != 0 && oploc == OP_SETLOCAL as ::core::ffi::c_int {
        if is_arguments != 0 {
            jsC_error(
                J,
                ident,
                b"'arguments' is read-only in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if is_eval != 0 {
            jsC_error(
                J,
                ident,
                b"'eval' is read-only in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    if is_eval != 0 {
        js_evalerror(
            J,
            b"%s:%d: invalid use of 'eval'\0" as *const u8 as *const ::core::ffi::c_char,
            (*J).filename,
            (*ident).line,
        );
    }
    i = findlocal(J, F, (*ident).string);
    if i < 0 as ::core::ffi::c_int {
        emitstring(J, F, opvar, (*ident).string);
    } else {
        emit(J, F, oploc);
        emitarg(J, F, i);
    };
}
unsafe extern "C" fn here(
    mut J: *mut js_State,
    mut F: *mut js_Function,
) -> ::core::ffi::c_int {
    return (*F).codelen;
}
unsafe extern "C" fn emitjump(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut opcode: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut inst: ::core::ffi::c_int = 0;
    emit(J, F, opcode);
    inst = (*F).codelen;
    emitarg(J, F, 0 as ::core::ffi::c_int);
    return inst;
}
unsafe extern "C" fn emitjumpto(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut opcode: ::core::ffi::c_int,
    mut dest: ::core::ffi::c_int,
) {
    emit(J, F, opcode);
    if dest != dest as js_Instruction as ::core::ffi::c_int {
        js_syntaxerror(
            J,
            b"jump address integer overflow\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    emitarg(J, F, dest);
}
unsafe extern "C" fn labelto(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut inst: ::core::ffi::c_int,
    mut addr: ::core::ffi::c_int,
) {
    if addr != addr as js_Instruction as ::core::ffi::c_int {
        js_syntaxerror(
            J,
            b"jump address integer overflow\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    *(*F).code.offset(inst as isize) = addr as js_Instruction;
}
unsafe extern "C" fn label(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut inst: ::core::ffi::c_int,
) {
    labelto(J, F, inst, (*F).codelen);
}
unsafe extern "C" fn ctypeof(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
) {
    if (*(*exp).a).type_0 as ::core::ffi::c_uint
        == EXP_IDENTIFIER as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        emitline(J, F, (*exp).a);
        emitlocal(
            J,
            F,
            OP_GETLOCAL as ::core::ffi::c_int,
            OP_HASVAR as ::core::ffi::c_int,
            (*exp).a,
        );
    } else {
        jsC_cexp(J, F, (*exp).a);
    }
    emitline(J, F, exp);
    emit(J, F, OP_TYPEOF as ::core::ffi::c_int);
}
unsafe extern "C" fn cunary(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
    mut opcode: ::core::ffi::c_int,
) {
    jsC_cexp(J, F, (*exp).a);
    emitline(J, F, exp);
    emit(J, F, opcode);
}
unsafe extern "C" fn cbinary(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
    mut opcode: ::core::ffi::c_int,
) {
    jsC_cexp(J, F, (*exp).a);
    jsC_cexp(J, F, (*exp).b);
    emitline(J, F, exp);
    emit(J, F, opcode);
}
unsafe extern "C" fn carray(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) {
    while !list.is_null() {
        emitline(J, F, (*list).a);
        if (*(*list).a).type_0 as ::core::ffi::c_uint
            == EXP_ELISION as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            emit(J, F, OP_SKIPARRAY as ::core::ffi::c_int);
        } else {
            jsC_cexp(J, F, (*list).a);
            emit(J, F, OP_INITARRAY as ::core::ffi::c_int);
        }
        list = (*list).b;
    }
}
unsafe extern "C" fn checkdup(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
    mut end: *mut js_Ast,
) {
    let mut nbuf: [::core::ffi::c_char; 32] = [0; 32];
    let mut sbuf: [::core::ffi::c_char; 32] = [0; 32];
    let mut needle: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut straw: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    if (*(*end).a).type_0 as ::core::ffi::c_uint
        == EXP_NUMBER as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        needle = jsV_numbertostring(
            J,
            &raw mut nbuf as *mut ::core::ffi::c_char,
            (*(*end).a).number,
        );
    } else {
        needle = (*(*end).a).string;
    }
    while (*list).a != end {
        if (*(*list).a).type_0 as ::core::ffi::c_uint
            == (*end).type_0 as ::core::ffi::c_uint
        {
            let mut prop: *mut js_Ast = (*(*list).a).a;
            if (*prop).type_0 as ::core::ffi::c_uint
                == EXP_NUMBER as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                straw = jsV_numbertostring(
                    J,
                    &raw mut sbuf as *mut ::core::ffi::c_char,
                    (*prop).number,
                );
            } else {
                straw = (*prop).string;
            }
            if strcmp(needle, straw) == 0 {
                jsC_error(
                    J,
                    list,
                    b"duplicate property '%s' in object literal\0" as *const u8
                        as *const ::core::ffi::c_char,
                    needle,
                );
            }
        }
        list = (*list).b;
    }
}
unsafe extern "C" fn cobject(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) {
    let mut head: *mut js_Ast = list;
    while !list.is_null() {
        let mut kv: *mut js_Ast = (*list).a;
        let mut prop: *mut js_Ast = (*kv).a;
        if (*prop).type_0 as ::core::ffi::c_uint
            == AST_IDENTIFIER as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*prop).type_0 as ::core::ffi::c_uint
                == EXP_STRING as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            emitline(J, F, prop);
            emitstring(J, F, OP_STRING as ::core::ffi::c_int, (*prop).string);
        } else if (*prop).type_0 as ::core::ffi::c_uint
            == EXP_NUMBER as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            emitline(J, F, prop);
            emitnumber(J, F, (*prop).number);
        } else {
            jsC_error(
                J,
                prop,
                b"invalid property name in object initializer\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if (*F).strict != 0 {
            checkdup(J, F, head, kv);
        }
        match (*kv).type_0 as ::core::ffi::c_uint {
            14 => {
                jsC_cexp(J, F, (*kv).b);
                emitline(J, F, kv);
                emit(J, F, OP_INITPROP as ::core::ffi::c_int);
            }
            15 => {
                emitfunction(
                    J,
                    F,
                    newfun(
                        J,
                        (*prop).line,
                        ::core::ptr::null_mut::<js_Ast>(),
                        ::core::ptr::null_mut::<js_Ast>(),
                        (*kv).c,
                        0 as ::core::ffi::c_int,
                        (*F).strict,
                        1 as ::core::ffi::c_int,
                    ),
                );
                emitline(J, F, kv);
                emit(J, F, OP_INITGETTER as ::core::ffi::c_int);
            }
            16 => {
                emitfunction(
                    J,
                    F,
                    newfun(
                        J,
                        (*prop).line,
                        ::core::ptr::null_mut::<js_Ast>(),
                        (*kv).b,
                        (*kv).c,
                        0 as ::core::ffi::c_int,
                        (*F).strict,
                        1 as ::core::ffi::c_int,
                    ),
                );
                emitline(J, F, kv);
                emit(J, F, OP_INITSETTER as ::core::ffi::c_int);
            }
            _ => {}
        }
        list = (*list).b;
    }
}
unsafe extern "C" fn cargs(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !list.is_null() {
        jsC_cexp(J, F, (*list).a);
        list = (*list).b;
        n += 1;
    }
    return n;
}
unsafe extern "C" fn cassign(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
) {
    let mut lhs: *mut js_Ast = (*exp).a;
    let mut rhs: *mut js_Ast = (*exp).b;
    match (*lhs).type_0 as ::core::ffi::c_uint {
        3 => {
            jsC_cexp(J, F, rhs);
            emitline(J, F, exp);
            emitlocal(
                J,
                F,
                OP_SETLOCAL as ::core::ffi::c_int,
                OP_SETVAR as ::core::ffi::c_int,
                lhs,
            );
        }
        18 => {
            jsC_cexp(J, F, (*lhs).a);
            jsC_cexp(J, F, (*lhs).b);
            jsC_cexp(J, F, rhs);
            emitline(J, F, exp);
            emit(J, F, OP_SETPROP as ::core::ffi::c_int);
        }
        19 => {
            jsC_cexp(J, F, (*lhs).a);
            jsC_cexp(J, F, rhs);
            emitline(J, F, exp);
            emitstring(J, F, OP_SETPROP_S as ::core::ffi::c_int, (*(*lhs).b).string);
        }
        _ => {
            jsC_error(
                J,
                lhs,
                b"invalid l-value in assignment\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn cassignforin(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut stm: *mut js_Ast,
) {
    let mut lhs: *mut js_Ast = (*stm).a;
    if (*stm).type_0 as ::core::ffi::c_uint
        == STM_FOR_IN_VAR as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if !(*lhs).b.is_null() {
            jsC_error(
                J,
                (*lhs).b,
                b"more than one loop variable in for-in statement\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        emitline(J, F, (*lhs).a);
        emitlocal(
            J,
            F,
            OP_SETLOCAL as ::core::ffi::c_int,
            OP_SETVAR as ::core::ffi::c_int,
            (*(*lhs).a).a,
        );
        emit(J, F, OP_POP as ::core::ffi::c_int);
        return;
    }
    match (*lhs).type_0 as ::core::ffi::c_uint {
        3 => {
            emitline(J, F, lhs);
            emitlocal(
                J,
                F,
                OP_SETLOCAL as ::core::ffi::c_int,
                OP_SETVAR as ::core::ffi::c_int,
                lhs,
            );
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        18 => {
            jsC_cexp(J, F, (*lhs).a);
            jsC_cexp(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT3 as ::core::ffi::c_int);
            emit(J, F, OP_SETPROP as ::core::ffi::c_int);
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        19 => {
            jsC_cexp(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT2 as ::core::ffi::c_int);
            emitstring(J, F, OP_SETPROP_S as ::core::ffi::c_int, (*(*lhs).b).string);
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        _ => {
            jsC_error(
                J,
                lhs,
                b"invalid l-value in for-in loop assignment\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn cassignop1(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut lhs: *mut js_Ast,
) {
    match (*lhs).type_0 as ::core::ffi::c_uint {
        3 => {
            emitline(J, F, lhs);
            emitlocal(
                J,
                F,
                OP_GETLOCAL as ::core::ffi::c_int,
                OP_GETVAR as ::core::ffi::c_int,
                lhs,
            );
        }
        18 => {
            jsC_cexp(J, F, (*lhs).a);
            jsC_cexp(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP2 as ::core::ffi::c_int);
            emit(J, F, OP_GETPROP as ::core::ffi::c_int);
        }
        19 => {
            jsC_cexp(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP as ::core::ffi::c_int);
            emitstring(J, F, OP_GETPROP_S as ::core::ffi::c_int, (*(*lhs).b).string);
        }
        _ => {
            jsC_error(
                J,
                lhs,
                b"invalid l-value in assignment\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn cassignop2(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut lhs: *mut js_Ast,
    mut postfix: ::core::ffi::c_int,
) {
    match (*lhs).type_0 as ::core::ffi::c_uint {
        3 => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT2 as ::core::ffi::c_int);
            }
            emitlocal(
                J,
                F,
                OP_SETLOCAL as ::core::ffi::c_int,
                OP_SETVAR as ::core::ffi::c_int,
                lhs,
            );
        }
        18 => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT4 as ::core::ffi::c_int);
            }
            emit(J, F, OP_SETPROP as ::core::ffi::c_int);
        }
        19 => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT3 as ::core::ffi::c_int);
            }
            emitstring(J, F, OP_SETPROP_S as ::core::ffi::c_int, (*(*lhs).b).string);
        }
        _ => {
            jsC_error(
                J,
                lhs,
                b"invalid l-value in assignment\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn cassignop(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
    mut opcode: ::core::ffi::c_int,
) {
    let mut lhs: *mut js_Ast = (*exp).a;
    let mut rhs: *mut js_Ast = (*exp).b;
    cassignop1(J, F, lhs);
    jsC_cexp(J, F, rhs);
    emitline(J, F, exp);
    emit(J, F, opcode);
    cassignop2(J, F, lhs, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn cdelete(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
) {
    let mut arg: *mut js_Ast = (*exp).a;
    match (*arg).type_0 as ::core::ffi::c_uint {
        3 => {
            if (*F).strict != 0 {
                jsC_error(
                    J,
                    exp,
                    b"delete on an unqualified name is not allowed in strict mode\0"
                        as *const u8 as *const ::core::ffi::c_char,
                );
            }
            emitline(J, F, exp);
            emitlocal(
                J,
                F,
                OP_DELLOCAL as ::core::ffi::c_int,
                OP_DELVAR as ::core::ffi::c_int,
                arg,
            );
        }
        18 => {
            jsC_cexp(J, F, (*arg).a);
            jsC_cexp(J, F, (*arg).b);
            emitline(J, F, exp);
            emit(J, F, OP_DELPROP as ::core::ffi::c_int);
        }
        19 => {
            jsC_cexp(J, F, (*arg).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_DELPROP_S as ::core::ffi::c_int, (*(*arg).b).string);
        }
        _ => {
            jsC_error(
                J,
                exp,
                b"invalid l-value in delete expression\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn ceval(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut fun: *mut js_Ast,
    mut args: *mut js_Ast,
) {
    let mut n: ::core::ffi::c_int = cargs(J, F, args);
    (*F).lightweight = 0 as ::core::ffi::c_int;
    (*F).arguments = 1 as ::core::ffi::c_int;
    if n == 0 as ::core::ffi::c_int {
        emit(J, F, OP_UNDEF as ::core::ffi::c_int);
    } else {
        loop {
            let fresh7 = n;
            n = n - 1;
            if !(fresh7 > 1 as ::core::ffi::c_int) {
                break;
            }
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
    }
    emit(J, F, OP_EVAL as ::core::ffi::c_int);
}
unsafe extern "C" fn ccall(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut fun: *mut js_Ast,
    mut args: *mut js_Ast,
) {
    let mut n: ::core::ffi::c_int = 0;
    let mut current_block_14: u64;
    match (*fun).type_0 as ::core::ffi::c_uint {
        18 => {
            jsC_cexp(J, F, (*fun).a);
            emit(J, F, OP_DUP as ::core::ffi::c_int);
            jsC_cexp(J, F, (*fun).b);
            emit(J, F, OP_GETPROP as ::core::ffi::c_int);
            emit(J, F, OP_ROT2 as ::core::ffi::c_int);
            current_block_14 = 11050875288958768710;
        }
        19 => {
            jsC_cexp(J, F, (*fun).a);
            emit(J, F, OP_DUP as ::core::ffi::c_int);
            emitstring(J, F, OP_GETPROP_S as ::core::ffi::c_int, (*(*fun).b).string);
            emit(J, F, OP_ROT2 as ::core::ffi::c_int);
            current_block_14 = 11050875288958768710;
        }
        3 => {
            if strcmp(
                (*fun).string,
                b"eval\0" as *const u8 as *const ::core::ffi::c_char,
            ) == 0
            {
                ceval(J, F, fun, args);
                return;
            }
            current_block_14 = 9611568049259025200;
        }
        _ => {
            current_block_14 = 9611568049259025200;
        }
    }
    match current_block_14 {
        9611568049259025200 => {
            jsC_cexp(J, F, fun);
            emit(J, F, OP_UNDEF as ::core::ffi::c_int);
        }
        _ => {}
    }
    n = cargs(J, F, args);
    emit(J, F, OP_CALL as ::core::ffi::c_int);
    emitarg(J, F, n);
}
unsafe extern "C" fn jsC_cexp(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut exp: *mut js_Ast,
) {
    let mut then: ::core::ffi::c_int = 0;
    let mut end: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    match (*exp).type_0 as ::core::ffi::c_uint {
        5 => {
            emitline(J, F, exp);
            emitstring(J, F, OP_STRING as ::core::ffi::c_int, (*exp).string);
        }
        4 => {
            emitline(J, F, exp);
            emitnumber(J, F, (*exp).number);
        }
        7 => {}
        8 => {
            emitline(J, F, exp);
            emit(J, F, OP_NULL as ::core::ffi::c_int);
        }
        9 => {
            emitline(J, F, exp);
            emit(J, F, OP_TRUE as ::core::ffi::c_int);
        }
        10 => {
            emitline(J, F, exp);
            emit(J, F, OP_FALSE as ::core::ffi::c_int);
        }
        11 => {
            emitline(J, F, exp);
            emit(J, F, OP_THIS as ::core::ffi::c_int);
        }
        6 => {
            emitline(J, F, exp);
            emitstring(J, F, OP_NEWREGEXP as ::core::ffi::c_int, (*exp).string);
            emitarg(J, F, (*exp).number as ::core::ffi::c_int);
        }
        13 => {
            emitline(J, F, exp);
            emit(J, F, OP_NEWOBJECT as ::core::ffi::c_int);
            cobject(J, F, (*exp).a);
        }
        12 => {
            emitline(J, F, exp);
            emit(J, F, OP_NEWARRAY as ::core::ffi::c_int);
            carray(J, F, (*exp).a);
        }
        17 => {
            emitline(J, F, exp);
            emitfunction(
                J,
                F,
                newfun(
                    J,
                    (*exp).line,
                    (*exp).a,
                    (*exp).b,
                    (*exp).c,
                    0 as ::core::ffi::c_int,
                    (*F).strict,
                    1 as ::core::ffi::c_int,
                ),
            );
        }
        3 => {
            emitline(J, F, exp);
            emitlocal(
                J,
                F,
                OP_GETLOCAL as ::core::ffi::c_int,
                OP_GETVAR as ::core::ffi::c_int,
                exp,
            );
        }
        18 => {
            jsC_cexp(J, F, (*exp).a);
            jsC_cexp(J, F, (*exp).b);
            emitline(J, F, exp);
            emit(J, F, OP_GETPROP as ::core::ffi::c_int);
        }
        19 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_GETPROP_S as ::core::ffi::c_int, (*(*exp).b).string);
        }
        20 => {
            ccall(J, F, (*exp).a, (*exp).b);
        }
        21 => {
            jsC_cexp(J, F, (*exp).a);
            n = cargs(J, F, (*exp).b);
            emitline(J, F, exp);
            emit(J, F, OP_NEW as ::core::ffi::c_int);
            emitarg(J, F, n);
        }
        24 => {
            cdelete(J, F, exp);
        }
        27 => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_INC as ::core::ffi::c_int);
            cassignop2(J, F, (*exp).a, 0 as ::core::ffi::c_int);
        }
        28 => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DEC as ::core::ffi::c_int);
            cassignop2(J, F, (*exp).a, 0 as ::core::ffi::c_int);
        }
        22 => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POSTINC as ::core::ffi::c_int);
            cassignop2(J, F, (*exp).a, 1 as ::core::ffi::c_int);
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        23 => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POSTDEC as ::core::ffi::c_int);
            cassignop2(J, F, (*exp).a, 1 as ::core::ffi::c_int);
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        25 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP as ::core::ffi::c_int);
            emit(J, F, OP_UNDEF as ::core::ffi::c_int);
        }
        26 => {
            ctypeof(J, F, exp);
        }
        29 => {
            cunary(J, F, exp, OP_POS as ::core::ffi::c_int);
        }
        30 => {
            cunary(J, F, exp, OP_NEG as ::core::ffi::c_int);
        }
        31 => {
            cunary(J, F, exp, OP_BITNOT as ::core::ffi::c_int);
        }
        32 => {
            cunary(J, F, exp, OP_LOGNOT as ::core::ffi::c_int);
        }
        53 => {
            cbinary(J, F, exp, OP_BITOR as ::core::ffi::c_int);
        }
        52 => {
            cbinary(J, F, exp, OP_BITXOR as ::core::ffi::c_int);
        }
        51 => {
            cbinary(J, F, exp, OP_BITAND as ::core::ffi::c_int);
        }
        50 => {
            cbinary(J, F, exp, OP_EQ as ::core::ffi::c_int);
        }
        49 => {
            cbinary(J, F, exp, OP_NE as ::core::ffi::c_int);
        }
        48 => {
            cbinary(J, F, exp, OP_STRICTEQ as ::core::ffi::c_int);
        }
        47 => {
            cbinary(J, F, exp, OP_STRICTNE as ::core::ffi::c_int);
        }
        46 => {
            cbinary(J, F, exp, OP_LT as ::core::ffi::c_int);
        }
        45 => {
            cbinary(J, F, exp, OP_GT as ::core::ffi::c_int);
        }
        44 => {
            cbinary(J, F, exp, OP_LE as ::core::ffi::c_int);
        }
        43 => {
            cbinary(J, F, exp, OP_GE as ::core::ffi::c_int);
        }
        42 => {
            cbinary(J, F, exp, OP_INSTANCEOF as ::core::ffi::c_int);
        }
        41 => {
            cbinary(J, F, exp, OP_IN as ::core::ffi::c_int);
        }
        40 => {
            cbinary(J, F, exp, OP_SHL as ::core::ffi::c_int);
        }
        39 => {
            cbinary(J, F, exp, OP_SHR as ::core::ffi::c_int);
        }
        38 => {
            cbinary(J, F, exp, OP_USHR as ::core::ffi::c_int);
        }
        37 => {
            cbinary(J, F, exp, OP_ADD as ::core::ffi::c_int);
        }
        36 => {
            cbinary(J, F, exp, OP_SUB as ::core::ffi::c_int);
        }
        35 => {
            cbinary(J, F, exp, OP_MUL as ::core::ffi::c_int);
        }
        34 => {
            cbinary(J, F, exp, OP_DIV as ::core::ffi::c_int);
        }
        33 => {
            cbinary(J, F, exp, OP_MOD as ::core::ffi::c_int);
        }
        57 => {
            cassign(J, F, exp);
        }
        58 => {
            cassignop(J, F, exp, OP_MUL as ::core::ffi::c_int);
        }
        59 => {
            cassignop(J, F, exp, OP_DIV as ::core::ffi::c_int);
        }
        60 => {
            cassignop(J, F, exp, OP_MOD as ::core::ffi::c_int);
        }
        61 => {
            cassignop(J, F, exp, OP_ADD as ::core::ffi::c_int);
        }
        62 => {
            cassignop(J, F, exp, OP_SUB as ::core::ffi::c_int);
        }
        63 => {
            cassignop(J, F, exp, OP_SHL as ::core::ffi::c_int);
        }
        64 => {
            cassignop(J, F, exp, OP_SHR as ::core::ffi::c_int);
        }
        65 => {
            cassignop(J, F, exp, OP_USHR as ::core::ffi::c_int);
        }
        66 => {
            cassignop(J, F, exp, OP_BITAND as ::core::ffi::c_int);
        }
        67 => {
            cassignop(J, F, exp, OP_BITXOR as ::core::ffi::c_int);
        }
        68 => {
            cassignop(J, F, exp, OP_BITOR as ::core::ffi::c_int);
        }
        69 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP as ::core::ffi::c_int);
            jsC_cexp(J, F, (*exp).b);
        }
        55 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP as ::core::ffi::c_int);
            end = emitjump(J, F, OP_JTRUE as ::core::ffi::c_int);
            emit(J, F, OP_POP as ::core::ffi::c_int);
            jsC_cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        54 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP as ::core::ffi::c_int);
            end = emitjump(J, F, OP_JFALSE as ::core::ffi::c_int);
            emit(J, F, OP_POP as ::core::ffi::c_int);
            jsC_cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        56 => {
            jsC_cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            then = emitjump(J, F, OP_JTRUE as ::core::ffi::c_int);
            jsC_cexp(J, F, (*exp).c);
            end = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
            label(J, F, then);
            jsC_cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        _ => {
            jsC_error(
                J,
                exp,
                b"unknown expression type\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    };
}
unsafe extern "C" fn addjump(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut type_0: js_AstType,
    mut target: *mut js_Ast,
    mut inst: ::core::ffi::c_int,
) {
    let mut jump: *mut js_JumpList = js_malloc(
        J,
        ::core::mem::size_of::<js_JumpList>() as ::core::ffi::c_int,
    ) as *mut js_JumpList;
    (*jump).type_0 = type_0;
    (*jump).inst = inst;
    (*jump).next = (*target).jumps;
    (*target).jumps = jump;
}
unsafe extern "C" fn labeljumps(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut stm: *mut js_Ast,
    mut baddr: ::core::ffi::c_int,
    mut caddr: ::core::ffi::c_int,
) {
    let mut jump: *mut js_JumpList = (*stm).jumps;
    while !jump.is_null() {
        let mut next: *mut js_JumpList = (*jump).next;
        if (*jump).type_0 as ::core::ffi::c_uint
            == STM_BREAK as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            labelto(J, F, (*jump).inst, baddr);
        }
        if (*jump).type_0 as ::core::ffi::c_uint
            == STM_CONTINUE as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            labelto(J, F, (*jump).inst, caddr);
        }
        js_free(J, jump as *mut ::core::ffi::c_void);
        jump = next;
    }
    (*stm).jumps = ::core::ptr::null_mut::<js_JumpList>();
}
unsafe extern "C" fn isloop(mut T: js_AstType) -> ::core::ffi::c_int {
    return (T as ::core::ffi::c_uint
        == STM_DO as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == STM_WHILE as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == STM_FOR as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == STM_FOR_VAR as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == STM_FOR_IN as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == STM_FOR_IN_VAR as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn isfun(mut T: js_AstType) -> ::core::ffi::c_int {
    return (T as ::core::ffi::c_uint
        == AST_FUNDEC as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == EXP_FUN as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == EXP_PROP_GET as ::core::ffi::c_int as ::core::ffi::c_uint
        || T as ::core::ffi::c_uint
            == EXP_PROP_SET as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn matchlabel(
    mut node: *mut js_Ast,
    mut label_0: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    while !node.is_null()
        && (*node).type_0 as ::core::ffi::c_uint
            == STM_LABEL as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp((*(*node).a).string, label_0) == 0 {
            return 1 as ::core::ffi::c_int;
        }
        node = (*node).parent;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn breaktarget(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut node: *mut js_Ast,
    mut label_0: *const ::core::ffi::c_char,
) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_0) != 0 {
            break;
        }
        if label_0.is_null() {
            if isloop((*node).type_0) != 0
                || (*node).type_0 as ::core::ffi::c_uint
                    == STM_SWITCH as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                return node;
            }
        } else if matchlabel((*node).parent, label_0) != 0 {
            return node
        }
        node = (*node).parent;
    }
    return ::core::ptr::null_mut::<js_Ast>();
}
unsafe extern "C" fn continuetarget(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut node: *mut js_Ast,
    mut label_0: *const ::core::ffi::c_char,
) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_0) != 0 {
            break;
        }
        if isloop((*node).type_0) != 0 {
            if label_0.is_null() {
                return node
            } else if matchlabel((*node).parent, label_0) != 0 {
                return node
            }
        }
        node = (*node).parent;
    }
    return ::core::ptr::null_mut::<js_Ast>();
}
unsafe extern "C" fn returntarget(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut node: *mut js_Ast,
) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_0) != 0 {
            return node;
        }
        node = (*node).parent;
    }
    return ::core::ptr::null_mut::<js_Ast>();
}
unsafe extern "C" fn cexit(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut T: js_AstType,
    mut node: *mut js_Ast,
    mut target: *mut js_Ast,
) {
    let mut prev: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    loop {
        prev = node;
        node = (*node).parent;
        match (*node).type_0 as ::core::ffi::c_uint {
            84 => {
                emitline(J, F, node);
                emit(J, F, OP_ENDWITH as ::core::ffi::c_int);
            }
            79 | 80 => {
                emitline(J, F, node);
                if (*F).script != 0 {
                    if T as ::core::ffi::c_uint
                        == STM_RETURN as ::core::ffi::c_int as ::core::ffi::c_uint
                        || T as ::core::ffi::c_uint
                            == STM_BREAK as ::core::ffi::c_int as ::core::ffi::c_uint
                        || T as ::core::ffi::c_uint
                            == STM_CONTINUE as ::core::ffi::c_int as ::core::ffi::c_uint
                            && target != node
                    {
                        emit(J, F, OP_ROT2 as ::core::ffi::c_int);
                        emit(J, F, OP_POP as ::core::ffi::c_int);
                    }
                    if T as ::core::ffi::c_uint
                        == STM_CONTINUE as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        emit(J, F, OP_ROT2 as ::core::ffi::c_int);
                    }
                } else {
                    if T as ::core::ffi::c_uint
                        == STM_RETURN as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        emit(J, F, OP_ROT2 as ::core::ffi::c_int);
                        emit(J, F, OP_POP as ::core::ffi::c_int);
                    }
                    if T as ::core::ffi::c_uint
                        == STM_BREAK as ::core::ffi::c_int as ::core::ffi::c_uint
                        || T as ::core::ffi::c_uint
                            == STM_CONTINUE as ::core::ffi::c_int as ::core::ffi::c_uint
                            && target != node
                    {
                        emit(J, F, OP_POP as ::core::ffi::c_int);
                    }
                }
            }
            87 => {
                emitline(J, F, node);
                if prev == (*node).a {
                    emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
                    if !(*node).d.is_null() {
                        cstm(J, F, (*node).d);
                    }
                }
                if prev == (*node).c {
                    if !(*node).d.is_null() {
                        emit(J, F, OP_ENDCATCH as ::core::ffi::c_int);
                        emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
                        cstm(J, F, (*node).d);
                    } else {
                        emit(J, F, OP_ENDCATCH as ::core::ffi::c_int);
                    }
                }
            }
            _ => {}
        }
        if !(node != target) {
            break;
        }
    };
}
unsafe extern "C" fn ctryfinally(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut trystm: *mut js_Ast,
    mut finallystm: *mut js_Ast,
) {
    let mut L1: ::core::ffi::c_int = 0;
    L1 = emitjump(J, F, OP_TRY as ::core::ffi::c_int);
    cstm(J, F, finallystm);
    emit(J, F, OP_THROW as ::core::ffi::c_int);
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
    cstm(J, F, finallystm);
}
unsafe extern "C" fn ctrycatch(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut trystm: *mut js_Ast,
    mut catchvar: *mut js_Ast,
    mut catchstm: *mut js_Ast,
) {
    let mut L1: ::core::ffi::c_int = 0;
    let mut L2: ::core::ffi::c_int = 0;
    L1 = emitjump(J, F, OP_TRY as ::core::ffi::c_int);
    checkfutureword(J, F, catchvar);
    if (*F).strict != 0 {
        if strcmp(
            (*catchvar).string,
            b"arguments\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            jsC_error(
                J,
                catchvar,
                b"redefining 'arguments' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if strcmp(
            (*catchvar).string,
            b"eval\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            jsC_error(
                J,
                catchvar,
                b"redefining 'eval' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    emitline(J, F, catchvar);
    emitstring(J, F, OP_CATCH as ::core::ffi::c_int, (*catchvar).string);
    cstm(J, F, catchstm);
    emit(J, F, OP_ENDCATCH as ::core::ffi::c_int);
    L2 = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
    label(J, F, L2);
}
unsafe extern "C" fn ctrycatchfinally(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut trystm: *mut js_Ast,
    mut catchvar: *mut js_Ast,
    mut catchstm: *mut js_Ast,
    mut finallystm: *mut js_Ast,
) {
    let mut L1: ::core::ffi::c_int = 0;
    let mut L2: ::core::ffi::c_int = 0;
    let mut L3: ::core::ffi::c_int = 0;
    L1 = emitjump(J, F, OP_TRY as ::core::ffi::c_int);
    L2 = emitjump(J, F, OP_TRY as ::core::ffi::c_int);
    cstm(J, F, finallystm);
    emit(J, F, OP_THROW as ::core::ffi::c_int);
    label(J, F, L2);
    if (*F).strict != 0 {
        checkfutureword(J, F, catchvar);
        if strcmp(
            (*catchvar).string,
            b"arguments\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            jsC_error(
                J,
                catchvar,
                b"redefining 'arguments' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if strcmp(
            (*catchvar).string,
            b"eval\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            jsC_error(
                J,
                catchvar,
                b"redefining 'eval' is not allowed in strict mode\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    emitline(J, F, catchvar);
    emitstring(J, F, OP_CATCH as ::core::ffi::c_int, (*catchvar).string);
    cstm(J, F, catchstm);
    emit(J, F, OP_ENDCATCH as ::core::ffi::c_int);
    emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
    L3 = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY as ::core::ffi::c_int);
    label(J, F, L3);
    cstm(J, F, finallystm);
}
unsafe extern "C" fn cswitch(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut ref_0: *mut js_Ast,
    mut head: *mut js_Ast,
) {
    let mut node: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut clause: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut def: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut end: ::core::ffi::c_int = 0;
    jsC_cexp(J, F, ref_0);
    node = head;
    while !node.is_null() {
        clause = (*node).a;
        if (*clause).type_0 as ::core::ffi::c_uint
            == STM_DEFAULT as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if !def.is_null() {
                jsC_error(
                    J,
                    clause,
                    b"more than one default label in switch\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            }
            def = clause;
        } else {
            jsC_cexp(J, F, (*clause).a);
            emitline(J, F, clause);
            (*clause).casejump = emitjump(J, F, OP_JCASE as ::core::ffi::c_int);
        }
        node = (*node).b;
    }
    emit(J, F, OP_POP as ::core::ffi::c_int);
    if !def.is_null() {
        emitline(J, F, def);
        (*def).casejump = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
        end = 0 as ::core::ffi::c_int;
    } else {
        end = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
    }
    node = head;
    while !node.is_null() {
        clause = (*node).a;
        label(J, F, (*clause).casejump);
        if (*clause).type_0 as ::core::ffi::c_uint
            == STM_DEFAULT as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            cstmlist(J, F, (*clause).a);
        } else {
            cstmlist(J, F, (*clause).b);
        }
        node = (*node).b;
    }
    if end != 0 {
        label(J, F, end);
    }
}
unsafe extern "C" fn cvarinit(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) {
    while !list.is_null() {
        let mut var: *mut js_Ast = (*list).a;
        if !(*var).b.is_null() {
            jsC_cexp(J, F, (*var).b);
            emitline(J, F, var);
            emitlocal(
                J,
                F,
                OP_SETLOCAL as ::core::ffi::c_int,
                OP_SETVAR as ::core::ffi::c_int,
                (*var).a,
            );
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        list = (*list).b;
    }
}
unsafe extern "C" fn cstm(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut stm: *mut js_Ast,
) {
    let mut target: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut loop_0: ::core::ffi::c_int = 0;
    let mut cont: ::core::ffi::c_int = 0;
    let mut then: ::core::ffi::c_int = 0;
    let mut end: ::core::ffi::c_int = 0;
    emitline(J, F, stm);
    match (*stm).type_0 as ::core::ffi::c_uint {
        1 => {}
        71 => {
            cstmlist(J, F, (*stm).a);
        }
        72 => {
            if (*F).script != 0 {
                emitline(J, F, stm);
                emit(J, F, OP_POP as ::core::ffi::c_int);
                emit(J, F, OP_UNDEF as ::core::ffi::c_int);
            }
        }
        73 => {
            cvarinit(J, F, (*stm).a);
        }
        74 => {
            if !(*stm).c.is_null() {
                jsC_cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                then = emitjump(J, F, OP_JTRUE as ::core::ffi::c_int);
                cstm(J, F, (*stm).c);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JUMP as ::core::ffi::c_int);
                label(J, F, then);
                cstm(J, F, (*stm).b);
                label(J, F, end);
            } else {
                jsC_cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JFALSE as ::core::ffi::c_int);
                cstm(J, F, (*stm).b);
                label(J, F, end);
            }
        }
        75 => {
            loop_0 = here(J, F);
            cstm(J, F, (*stm).a);
            cont = here(J, F);
            jsC_cexp(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JTRUE as ::core::ffi::c_int, loop_0);
            labeljumps(J, F, stm, here(J, F), cont);
        }
        76 => {
            loop_0 = here(J, F);
            jsC_cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            end = emitjump(J, F, OP_JFALSE as ::core::ffi::c_int);
            cstm(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP as ::core::ffi::c_int, loop_0);
            label(J, F, end);
            labeljumps(J, F, stm, here(J, F), loop_0);
        }
        77 | 78 => {
            if (*stm).type_0 as ::core::ffi::c_uint
                == STM_FOR_VAR as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                cvarinit(J, F, (*stm).a);
            } else if !(*stm).a.is_null() {
                jsC_cexp(J, F, (*stm).a);
                emit(J, F, OP_POP as ::core::ffi::c_int);
            }
            loop_0 = here(J, F);
            if !(*stm).b.is_null() {
                jsC_cexp(J, F, (*stm).b);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JFALSE as ::core::ffi::c_int);
            } else {
                end = 0 as ::core::ffi::c_int;
            }
            cstm(J, F, (*stm).d);
            cont = here(J, F);
            if !(*stm).c.is_null() {
                jsC_cexp(J, F, (*stm).c);
                emit(J, F, OP_POP as ::core::ffi::c_int);
            }
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP as ::core::ffi::c_int, loop_0);
            if end != 0 {
                label(J, F, end);
            }
            labeljumps(J, F, stm, here(J, F), cont);
        }
        79 | 80 => {
            jsC_cexp(J, F, (*stm).b);
            emitline(J, F, stm);
            emit(J, F, OP_ITERATOR as ::core::ffi::c_int);
            loop_0 = here(J, F);
            emitline(J, F, stm);
            emit(J, F, OP_NEXTITER as ::core::ffi::c_int);
            end = emitjump(J, F, OP_JFALSE as ::core::ffi::c_int);
            cassignforin(J, F, stm);
            if (*F).script != 0 {
                emit(J, F, OP_ROT2 as ::core::ffi::c_int);
                cstm(J, F, (*stm).c);
                emit(J, F, OP_ROT2 as ::core::ffi::c_int);
            } else {
                cstm(J, F, (*stm).c);
            }
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP as ::core::ffi::c_int, loop_0);
            label(J, F, end);
            labeljumps(J, F, stm, here(J, F), loop_0);
        }
        85 => {
            cswitch(J, F, (*stm).a, (*stm).b);
            labeljumps(J, F, stm, here(J, F), 0 as ::core::ffi::c_int);
        }
        89 => {
            cstm(J, F, (*stm).b);
            while (*stm).type_0 as ::core::ffi::c_uint
                == STM_LABEL as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                stm = (*stm).b;
            }
            if isloop((*stm).type_0) == 0
                && (*stm).type_0 as ::core::ffi::c_uint
                    != STM_SWITCH as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                labeljumps(J, F, stm, here(J, F), 0 as ::core::ffi::c_int);
            }
        }
        82 => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                target = breaktarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error(
                        J,
                        stm,
                        b"break label '%s' not found\0" as *const u8
                            as *const ::core::ffi::c_char,
                        (*(*stm).a).string,
                    );
                }
            } else {
                target = breaktarget(
                    J,
                    F,
                    (*stm).parent,
                    ::core::ptr::null::<::core::ffi::c_char>(),
                );
                if target.is_null() {
                    jsC_error(
                        J,
                        stm,
                        b"unlabelled break must be inside loop or switch\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
            }
            cexit(J, F, STM_BREAK, stm, target);
            emitline(J, F, stm);
            addjump(
                J,
                F,
                STM_BREAK,
                target,
                emitjump(J, F, OP_JUMP as ::core::ffi::c_int),
            );
        }
        81 => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                target = continuetarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error(
                        J,
                        stm,
                        b"continue label '%s' not found\0" as *const u8
                            as *const ::core::ffi::c_char,
                        (*(*stm).a).string,
                    );
                }
            } else {
                target = continuetarget(
                    J,
                    F,
                    (*stm).parent,
                    ::core::ptr::null::<::core::ffi::c_char>(),
                );
                if target.is_null() {
                    jsC_error(
                        J,
                        stm,
                        b"continue must be inside loop\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
            }
            cexit(J, F, STM_CONTINUE, stm, target);
            emitline(J, F, stm);
            addjump(
                J,
                F,
                STM_CONTINUE,
                target,
                emitjump(J, F, OP_JUMP as ::core::ffi::c_int),
            );
        }
        83 => {
            if !(*stm).a.is_null() {
                jsC_cexp(J, F, (*stm).a);
            } else {
                emit(J, F, OP_UNDEF as ::core::ffi::c_int);
            }
            target = returntarget(J, F, (*stm).parent);
            if target.is_null() {
                jsC_error(
                    J,
                    stm,
                    b"return not in function\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            }
            cexit(J, F, STM_RETURN, stm, target);
            emitline(J, F, stm);
            emit(J, F, OP_RETURN as ::core::ffi::c_int);
        }
        86 => {
            jsC_cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            emit(J, F, OP_THROW as ::core::ffi::c_int);
        }
        84 => {
            (*F).lightweight = 0 as ::core::ffi::c_int;
            if (*F).strict != 0 {
                jsC_error(
                    J,
                    (*stm).a,
                    b"'with' statements are not allowed in strict mode\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            }
            jsC_cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            emit(J, F, OP_WITH as ::core::ffi::c_int);
            cstm(J, F, (*stm).b);
            emitline(J, F, stm);
            emit(J, F, OP_ENDWITH as ::core::ffi::c_int);
        }
        87 => {
            emitline(J, F, stm);
            if !(*stm).b.is_null() && !(*stm).c.is_null() {
                (*F).lightweight = 0 as ::core::ffi::c_int;
                if !(*stm).d.is_null() {
                    ctrycatchfinally(J, F, (*stm).a, (*stm).b, (*stm).c, (*stm).d);
                } else {
                    ctrycatch(J, F, (*stm).a, (*stm).b, (*stm).c);
                }
            } else {
                ctryfinally(J, F, (*stm).a, (*stm).d);
            }
        }
        88 => {
            emitline(J, F, stm);
            emit(J, F, OP_DEBUGGER as ::core::ffi::c_int);
        }
        _ => {
            if (*F).script != 0 {
                emitline(J, F, stm);
                emit(J, F, OP_POP as ::core::ffi::c_int);
                jsC_cexp(J, F, stm);
            } else {
                jsC_cexp(J, F, stm);
                emitline(J, F, stm);
                emit(J, F, OP_POP as ::core::ffi::c_int);
            }
        }
    };
}
unsafe extern "C" fn cstmlist(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) {
    while !list.is_null() {
        cstm(J, F, (*list).a);
        list = (*list).b;
    }
}
unsafe extern "C" fn listlength(mut list: *mut js_Ast) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !list.is_null() {
        n += 1;
        list = (*list).b;
    }
    return n;
}
unsafe extern "C" fn cparams(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
    mut fname: *mut js_Ast,
) {
    (*F).numparams = listlength(list);
    while !list.is_null() {
        checkfutureword(J, F, (*list).a);
        addlocal(J, F, (*list).a, 0 as ::core::ffi::c_int);
        list = (*list).b;
    }
}
unsafe extern "C" fn cvardecs(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut node: *mut js_Ast,
) {
    if (*node).type_0 as ::core::ffi::c_uint
        == AST_LIST as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        while !node.is_null() {
            cvardecs(J, F, (*node).a);
            node = (*node).b;
        }
        return;
    }
    if isfun((*node).type_0) != 0 {
        return;
    }
    if (*node).type_0 as ::core::ffi::c_uint
        == EXP_VAR as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        checkfutureword(J, F, (*node).a);
        addlocal(J, F, (*node).a, 1 as ::core::ffi::c_int);
    }
    if !(*node).a.is_null() {
        cvardecs(J, F, (*node).a);
    }
    if !(*node).b.is_null() {
        cvardecs(J, F, (*node).b);
    }
    if !(*node).c.is_null() {
        cvardecs(J, F, (*node).c);
    }
    if !(*node).d.is_null() {
        cvardecs(J, F, (*node).d);
    }
}
unsafe extern "C" fn cfundecs(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut list: *mut js_Ast,
) {
    while !list.is_null() {
        let mut stm: *mut js_Ast = (*list).a;
        if (*stm).type_0 as ::core::ffi::c_uint
            == AST_FUNDEC as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            emitline(J, F, stm);
            emitfunction(
                J,
                F,
                newfun(
                    J,
                    (*stm).line,
                    (*stm).a,
                    (*stm).b,
                    (*stm).c,
                    0 as ::core::ffi::c_int,
                    (*F).strict,
                    0 as ::core::ffi::c_int,
                ),
            );
            emitline(J, F, stm);
            emit(J, F, OP_SETLOCAL as ::core::ffi::c_int);
            emitarg(J, F, addlocal(J, F, (*stm).a, 1 as ::core::ffi::c_int));
            emit(J, F, OP_POP as ::core::ffi::c_int);
        }
        list = (*list).b;
    }
}
unsafe extern "C" fn cfunbody(
    mut J: *mut js_State,
    mut F: *mut js_Function,
    mut name: *mut js_Ast,
    mut params: *mut js_Ast,
    mut body: *mut js_Ast,
    mut is_fun_exp: ::core::ffi::c_int,
) {
    (*F).lightweight = 1 as ::core::ffi::c_int;
    (*F).arguments = 0 as ::core::ffi::c_int;
    if (*F).script != 0 {
        (*F).lightweight = 0 as ::core::ffi::c_int;
    }
    if !body.is_null()
        && (*body).type_0 as ::core::ffi::c_uint
            == AST_LIST as ::core::ffi::c_int as ::core::ffi::c_uint
        && !(*body).a.is_null()
        && (*(*body).a).type_0 as ::core::ffi::c_uint
            == EXP_STRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(
            (*(*body).a).string,
            b"use strict\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            (*F).strict = 1 as ::core::ffi::c_int;
        }
    }
    (*F).lastline = (*F).line;
    cparams(J, F, params, name);
    if !body.is_null() {
        cvardecs(J, F, body);
        cfundecs(J, F, body);
    }
    if !name.is_null() {
        checkfutureword(J, F, name);
        if is_fun_exp != 0 {
            if findlocal(J, F, (*name).string) < 0 as ::core::ffi::c_int {
                emit(J, F, OP_CURRENT as ::core::ffi::c_int);
                emit(J, F, OP_SETLOCAL as ::core::ffi::c_int);
                emitarg(J, F, addlocal(J, F, name, 1 as ::core::ffi::c_int));
                emit(J, F, OP_POP as ::core::ffi::c_int);
            }
        }
    }
    if (*F).script != 0 {
        emit(J, F, OP_UNDEF as ::core::ffi::c_int);
        cstmlist(J, F, body);
        emit(J, F, OP_RETURN as ::core::ffi::c_int);
    } else {
        cstmlist(J, F, body);
        emit(J, F, OP_UNDEF as ::core::ffi::c_int);
        emit(J, F, OP_RETURN as ::core::ffi::c_int);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_compilefunction(
    mut J: *mut js_State,
    mut prog: *mut js_Ast,
) -> *mut js_Function {
    return newfun(
        J,
        (*prog).line,
        (*prog).a,
        (*prog).b,
        (*prog).c,
        0 as ::core::ffi::c_int,
        (*J).default_strict,
        1 as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_compilescript(
    mut J: *mut js_State,
    mut prog: *mut js_Ast,
    mut default_strict: ::core::ffi::c_int,
) -> *mut js_Function {
    return newfun(
        J,
        if !prog.is_null() { (*prog).line } else { 0 as ::core::ffi::c_int },
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        prog,
        1 as ::core::ffi::c_int,
        default_strict,
        0 as ::core::ffi::c_int,
    );
}
pub const SHRT_MAX: ::core::ffi::c_int = __SHRT_MAX__;
pub const SHRT_MIN: ::core::ffi::c_int = -__SHRT_MAX__ - 1 as ::core::ffi::c_int;
pub const __SHRT_MAX__: ::core::ffi::c_int = 32767 as ::core::ffi::c_int;
