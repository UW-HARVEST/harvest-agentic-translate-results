extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn js_report(J: *mut js_State, message: *const ::core::ffi::c_char);
    fn js_newsyntaxerror(J: *mut js_State, message: *const ::core::ffi::c_char);
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
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn ceil(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn floor(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn fmod(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_intern(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    fn jsY_tokenstring(token: ::core::ffi::c_int) -> *const ::core::ffi::c_char;
    fn jsY_initlex(
        J: *mut js_State,
        filename: *const ::core::ffi::c_char,
        source: *const ::core::ffi::c_char,
    );
    fn jsY_lex(J: *mut js_State) -> ::core::ffi::c_int;
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
pub type C2RustUnnamed_9 = ::core::ffi::c_uint;
pub const TK_WITH: C2RustUnnamed_9 = 312;
pub const TK_WHILE: C2RustUnnamed_9 = 311;
pub const TK_VOID: C2RustUnnamed_9 = 310;
pub const TK_VAR: C2RustUnnamed_9 = 309;
pub const TK_TYPEOF: C2RustUnnamed_9 = 308;
pub const TK_TRY: C2RustUnnamed_9 = 307;
pub const TK_TRUE: C2RustUnnamed_9 = 306;
pub const TK_THROW: C2RustUnnamed_9 = 305;
pub const TK_THIS: C2RustUnnamed_9 = 304;
pub const TK_SWITCH: C2RustUnnamed_9 = 303;
pub const TK_RETURN: C2RustUnnamed_9 = 302;
pub const TK_NULL: C2RustUnnamed_9 = 301;
pub const TK_NEW: C2RustUnnamed_9 = 300;
pub const TK_INSTANCEOF: C2RustUnnamed_9 = 299;
pub const TK_IN: C2RustUnnamed_9 = 298;
pub const TK_IF: C2RustUnnamed_9 = 297;
pub const TK_FUNCTION: C2RustUnnamed_9 = 296;
pub const TK_FOR: C2RustUnnamed_9 = 295;
pub const TK_FINALLY: C2RustUnnamed_9 = 294;
pub const TK_FALSE: C2RustUnnamed_9 = 293;
pub const TK_ELSE: C2RustUnnamed_9 = 292;
pub const TK_DO: C2RustUnnamed_9 = 291;
pub const TK_DELETE: C2RustUnnamed_9 = 290;
pub const TK_DEFAULT: C2RustUnnamed_9 = 289;
pub const TK_DEBUGGER: C2RustUnnamed_9 = 288;
pub const TK_CONTINUE: C2RustUnnamed_9 = 287;
pub const TK_CATCH: C2RustUnnamed_9 = 286;
pub const TK_CASE: C2RustUnnamed_9 = 285;
pub const TK_BREAK: C2RustUnnamed_9 = 284;
pub const TK_DEC: C2RustUnnamed_9 = 283;
pub const TK_INC: C2RustUnnamed_9 = 282;
pub const TK_XOR_ASS: C2RustUnnamed_9 = 281;
pub const TK_OR_ASS: C2RustUnnamed_9 = 280;
pub const TK_AND_ASS: C2RustUnnamed_9 = 279;
pub const TK_USHR_ASS: C2RustUnnamed_9 = 278;
pub const TK_SHR_ASS: C2RustUnnamed_9 = 277;
pub const TK_SHL_ASS: C2RustUnnamed_9 = 276;
pub const TK_MOD_ASS: C2RustUnnamed_9 = 275;
pub const TK_DIV_ASS: C2RustUnnamed_9 = 274;
pub const TK_MUL_ASS: C2RustUnnamed_9 = 273;
pub const TK_SUB_ASS: C2RustUnnamed_9 = 272;
pub const TK_ADD_ASS: C2RustUnnamed_9 = 271;
pub const TK_OR: C2RustUnnamed_9 = 270;
pub const TK_AND: C2RustUnnamed_9 = 269;
pub const TK_USHR: C2RustUnnamed_9 = 268;
pub const TK_SHR: C2RustUnnamed_9 = 267;
pub const TK_SHL: C2RustUnnamed_9 = 266;
pub const TK_STRICTNE: C2RustUnnamed_9 = 265;
pub const TK_STRICTEQ: C2RustUnnamed_9 = 264;
pub const TK_NE: C2RustUnnamed_9 = 263;
pub const TK_EQ: C2RustUnnamed_9 = 262;
pub const TK_GE: C2RustUnnamed_9 = 261;
pub const TK_LE: C2RustUnnamed_9 = 260;
pub const TK_REGEXP: C2RustUnnamed_9 = 259;
pub const TK_STRING: C2RustUnnamed_9 = 258;
pub const TK_NUMBER: C2RustUnnamed_9 = 257;
pub const TK_IDENTIFIER: C2RustUnnamed_9 = 256;
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
unsafe extern "C" fn jsP_error(
    mut J: *mut js_State,
    mut fmt: *const ::core::ffi::c_char,
    mut args: ...
) -> ! {
    let mut ap: ::core::ffi::VaListImpl;
    let mut buf: [::core::ffi::c_char; 512] = [0; 512];
    let mut msgbuf: [::core::ffi::c_char; 256] = [0; 256];
    ap = args.clone();
    vsnprintf(
        &raw mut msgbuf as *mut ::core::ffi::c_char,
        256 as size_t,
        fmt,
        ap.as_va_list(),
    );
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        256 as size_t,
        b"%s:%d: \0" as *const u8 as *const ::core::ffi::c_char,
        (*J).filename,
        (*J).lexline,
    );
    strcat(
        &raw mut buf as *mut ::core::ffi::c_char,
        &raw mut msgbuf as *mut ::core::ffi::c_char,
    );
    js_newsyntaxerror(J, &raw mut buf as *mut ::core::ffi::c_char);
    js_throw(J);
}
unsafe extern "C" fn jsP_warning(
    mut J: *mut js_State,
    mut fmt: *const ::core::ffi::c_char,
    mut args: ...
) {
    let mut ap: ::core::ffi::VaListImpl;
    let mut buf: [::core::ffi::c_char; 512] = [0; 512];
    let mut msg: [::core::ffi::c_char; 256] = [0; 256];
    ap = args.clone();
    vsnprintf(
        &raw mut msg as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
        fmt,
        ap.as_va_list(),
    );
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 512]>() as size_t,
        b"%s:%d: warning: %s\0" as *const u8 as *const ::core::ffi::c_char,
        (*J).filename,
        (*J).lexline,
        &raw mut msg as *mut ::core::ffi::c_char,
    );
    js_report(J, &raw mut buf as *mut ::core::ffi::c_char);
}
unsafe extern "C" fn jsP_newnode(
    mut J: *mut js_State,
    mut type_0: js_AstType,
    mut line: ::core::ffi::c_int,
    mut a: *mut js_Ast,
    mut b: *mut js_Ast,
    mut c: *mut js_Ast,
    mut d: *mut js_Ast,
) -> *mut js_Ast {
    let mut node: *mut js_Ast = js_malloc(
        J,
        ::core::mem::size_of::<js_Ast>() as ::core::ffi::c_int,
    ) as *mut js_Ast;
    (*node).type_0 = type_0;
    (*node).line = line;
    (*node).a = a;
    (*node).b = b;
    (*node).c = c;
    (*node).d = d;
    (*node).number = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    (*node).string = ::core::ptr::null::<::core::ffi::c_char>();
    (*node).jumps = ::core::ptr::null_mut::<js_JumpList>();
    (*node).casejump = 0 as ::core::ffi::c_int;
    (*node).parent = ::core::ptr::null_mut::<js_Ast>();
    if !a.is_null() {
        (*a).parent = node;
    }
    if !b.is_null() {
        (*b).parent = node;
    }
    if !c.is_null() {
        (*c).parent = node;
    }
    if !d.is_null() {
        (*d).parent = node;
    }
    (*node).gcnext = (*J).gcast;
    (*J).gcast = node;
    return node;
}
unsafe extern "C" fn jsP_list(mut head: *mut js_Ast) -> *mut js_Ast {
    let mut prev: *mut js_Ast = head;
    let mut node: *mut js_Ast = (*head).b;
    while !node.is_null() {
        (*node).parent = prev;
        prev = node;
        node = (*node).b;
    }
    return head;
}
unsafe extern "C" fn jsP_newstrnode(
    mut J: *mut js_State,
    mut type_0: js_AstType,
    mut s: *const ::core::ffi::c_char,
) -> *mut js_Ast {
    let mut node: *mut js_Ast = jsP_newnode(
        J,
        type_0,
        (*J).lexline,
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    (*node).string = js_intern(J, s);
    return node;
}
unsafe extern "C" fn jsP_newnumnode(
    mut J: *mut js_State,
    mut type_0: js_AstType,
    mut n: ::core::ffi::c_double,
) -> *mut js_Ast {
    let mut node: *mut js_Ast = jsP_newnode(
        J,
        type_0,
        (*J).lexline,
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    (*node).number = n;
    return node;
}
unsafe extern "C" fn jsP_freejumps(mut J: *mut js_State, mut node: *mut js_JumpList) {
    while !node.is_null() {
        let mut next: *mut js_JumpList = (*node).next;
        js_free(J, node as *mut ::core::ffi::c_void);
        node = next;
    }
}
#[no_mangle]
pub unsafe extern "C" fn jsP_freeparse(mut J: *mut js_State) {
    let mut node: *mut js_Ast = (*J).gcast;
    while !node.is_null() {
        let mut next: *mut js_Ast = (*node).gcnext;
        jsP_freejumps(J, (*node).jumps);
        js_free(J, node as *mut ::core::ffi::c_void);
        node = next;
    }
    (*J).gcast = ::core::ptr::null_mut::<js_Ast>();
}
unsafe extern "C" fn jsP_next(mut J: *mut js_State) {
    (*J).lookahead = jsY_lex(J);
}
unsafe extern "C" fn semicolon(mut J: *mut js_State) {
    if (*J).lookahead == ';' as i32 {
        jsP_next(J);
        return;
    }
    if (*J).newline != 0 || (*J).lookahead == '}' as i32
        || (*J).lookahead == 0 as ::core::ffi::c_int
    {
        return;
    }
    jsP_error(
        J,
        b"unexpected token: %s (expected ';')\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn identifier(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == TK_IDENTIFIER as ::core::ffi::c_int {
        a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error(
        J,
        b"unexpected token: %s (expected identifier)\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn identifieropt(mut J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER as ::core::ffi::c_int {
        return identifier(J);
    }
    return ::core::ptr::null_mut::<js_Ast>();
}
unsafe extern "C" fn identifiername(mut J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER as ::core::ffi::c_int
        || (*J).lookahead >= TK_BREAK as ::core::ffi::c_int
    {
        let mut a: *mut js_Ast = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error(
        J,
        b"unexpected token: %s (expected identifier or keyword)\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn arrayelement(mut J: *mut js_State) -> *mut js_Ast {
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if (*J).lookahead == ',' as i32 {
        return jsP_newnode(
            J,
            EXP_ELISION,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    return assignment(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn arrayliteral(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == ']' as i32 {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        arrayelement(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if (*J).lookahead != ']' as i32 {
            (*tail).b = jsP_newnode(
                J,
                AST_LIST,
                0 as ::core::ffi::c_int,
                arrayelement(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
            tail = (*tail).b;
        }
    }
    return jsP_list(head);
}
unsafe extern "C" fn propname(mut J: *mut js_State) -> *mut js_Ast {
    let mut name: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == TK_NUMBER as ::core::ffi::c_int {
        name = jsP_newnumnode(J, EXP_NUMBER, (*J).number);
        jsP_next(J);
    } else if (*J).lookahead == TK_STRING as ::core::ffi::c_int {
        name = jsP_newstrnode(J, EXP_STRING, (*J).text);
        jsP_next(J);
    } else {
        name = identifiername(J);
    }
    return name;
}
unsafe extern "C" fn propassign(mut J: *mut js_State) -> *mut js_Ast {
    let mut name: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut value: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut arg: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut body: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    name = propname(J);
    if (*J).lookahead != ':' as i32
        && (*name).type_0 as ::core::ffi::c_uint
            == AST_IDENTIFIER as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp((*name).string, b"get\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            name = propname(J);
            if if (*J).lookahead == '(' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring('(' as i32),
                );
            }
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
            body = funbody(J);
            return jsP_newnode(
                J,
                EXP_PROP_GET,
                line,
                name,
                ::core::ptr::null_mut::<js_Ast>(),
                body,
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
        if strcmp((*name).string, b"set\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            name = propname(J);
            if if (*J).lookahead == '(' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring('(' as i32),
                );
            }
            arg = identifier(J);
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
            body = funbody(J);
            return jsP_newnode(
                J,
                EXP_PROP_SET,
                line,
                name,
                jsP_newnode(
                    J,
                    AST_LIST,
                    0 as ::core::ffi::c_int,
                    arg,
                    ::core::ptr::null_mut::<js_Ast>(),
                    ::core::ptr::null_mut::<js_Ast>(),
                    ::core::ptr::null_mut::<js_Ast>(),
                ),
                body,
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    if if (*J).lookahead == ':' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(':' as i32),
        );
    }
    value = assignment(J, 0 as ::core::ffi::c_int);
    return jsP_newnode(
        J,
        EXP_PROP_VAL,
        line,
        name,
        value,
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
}
unsafe extern "C" fn objectliteral(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == '}' as i32 {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        propassign(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if (*J).lookahead == '}' as i32 {
            break;
        }
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            propassign(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn parameters(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == ')' as i32 {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        identifier(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            identifier(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn fundec(
    mut J: *mut js_State,
    mut line: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    a = identifier(J);
    if if (*J).lookahead == '(' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('(' as i32),
        );
    }
    b = parameters(J);
    if if (*J).lookahead == ')' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(')' as i32),
        );
    }
    c = funbody(J);
    return jsP_newnode(J, AST_FUNDEC, line, a, b, c, ::core::ptr::null_mut::<js_Ast>());
}
unsafe extern "C" fn funstm(
    mut J: *mut js_State,
    mut line: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    a = identifier(J);
    if if (*J).lookahead == '(' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('(' as i32),
        );
    }
    b = parameters(J);
    if if (*J).lookahead == ')' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(')' as i32),
        );
    }
    c = funbody(J);
    return jsP_newnode(
        J,
        STM_VAR,
        line,
        jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            jsP_newnode(
                J,
                EXP_VAR,
                line,
                a,
                jsP_newnode(
                    J,
                    EXP_FUN,
                    line,
                    a,
                    b,
                    c,
                    ::core::ptr::null_mut::<js_Ast>(),
                ),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            ),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        ),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
}
unsafe extern "C" fn funexp(
    mut J: *mut js_State,
    mut line: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    a = identifieropt(J);
    if if (*J).lookahead == '(' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('(' as i32),
        );
    }
    b = parameters(J);
    if if (*J).lookahead == ')' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(')' as i32),
        );
    }
    c = funbody(J);
    return jsP_newnode(J, EXP_FUN, line, a, b, c, ::core::ptr::null_mut::<js_Ast>());
}
unsafe extern "C" fn primary(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if (*J).lookahead == TK_IDENTIFIER as ::core::ffi::c_int {
        a = jsP_newstrnode(J, EXP_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_STRING as ::core::ffi::c_int {
        a = jsP_newstrnode(J, EXP_STRING, (*J).text);
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_REGEXP as ::core::ffi::c_int {
        a = jsP_newstrnode(J, EXP_REGEXP, (*J).text);
        (*a).number = (*J).number;
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_NUMBER as ::core::ffi::c_int {
        a = jsP_newnumnode(J, EXP_NUMBER, (*J).number);
        jsP_next(J);
        return a;
    }
    if if (*J).lookahead == TK_THIS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return jsP_newnode(
            J,
            EXP_THIS,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == TK_NULL as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return jsP_newnode(
            J,
            EXP_NULL,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == TK_TRUE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return jsP_newnode(
            J,
            EXP_TRUE,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == TK_FALSE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return jsP_newnode(
            J,
            EXP_FALSE,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == '{' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_OBJECT,
            line,
            objectliteral(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        if if (*J).lookahead == '}' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('}' as i32),
            );
        }
        return a;
    }
    if if (*J).lookahead == '[' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ARRAY,
            line,
            arrayliteral(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        if if (*J).lookahead == ']' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(']' as i32),
            );
        }
        return a;
    }
    if if (*J).lookahead == '(' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        return a;
    }
    jsP_error(
        J,
        b"unexpected token in expression: %s\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn arguments(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == ')' as i32 {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        assignment(J, 0 as ::core::ffi::c_int),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            assignment(J, 0 as ::core::ffi::c_int),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn newexp(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == TK_NEW as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = memberexp(J);
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            b = arguments(J);
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
            return jsP_newnode(
                J,
                EXP_NEW,
                line,
                a,
                b,
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
        return jsP_newnode(
            J,
            EXP_NEW,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == TK_FUNCTION as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return funexp(J, line);
    }
    return primary(J);
}
unsafe extern "C" fn memberexp(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = newexp(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == '.' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_MEMBER,
                line,
                a,
                identifiername(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(if (*J).lookahead == '[' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_INDEX,
                line,
                a,
                expression(J, 0 as ::core::ffi::c_int),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
            if if (*J).lookahead == ']' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(']' as i32),
                );
            }
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn callexp(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = newexp(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == '.' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_MEMBER,
                line,
                a,
                identifiername(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == '[' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_INDEX,
                line,
                a,
                expression(J, 0 as ::core::ffi::c_int),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
            if if (*J).lookahead == ']' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(']' as i32),
                );
            }
        } else {
            if !(if (*J).lookahead == '(' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_CALL,
                line,
                a,
                arguments(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn postfix(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = callexp(J);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if (*J).newline == 0
        && (if (*J).lookahead == TK_INC as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) != 0
    {
        return jsP_newnode(
            J,
            EXP_POSTINC,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if (*J).newline == 0
        && (if (*J).lookahead == TK_DEC as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) != 0
    {
        return jsP_newnode(
            J,
            EXP_POSTDEC,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    return a;
}
unsafe extern "C" fn unary(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    (*J).astdepth += 1;
    if (*J).astdepth > JS_ASTLIMIT {
        jsP_error(J, b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if if (*J).lookahead == TK_DELETE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_DELETE,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_VOID as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_VOID,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_TYPEOF as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_TYPEOF,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_INC as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_PREINC,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_DEC as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_PREDEC,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == '+' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_POS,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == '-' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_NEG,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == '~' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_BITNOT,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == '!' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_LOGNOT,
            line,
            unary(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else {
        a = postfix(J);
    }
    (*J).astdepth -= 1;
    return a;
}
unsafe extern "C" fn multiplicative(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = unary(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == '*' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_MUL,
                line,
                a,
                unary(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == '/' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_DIV,
                line,
                a,
                unary(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(if (*J).lookahead == '%' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_MOD,
                line,
                a,
                unary(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn additive(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = multiplicative(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == '+' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_ADD,
                line,
                a,
                multiplicative(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(if (*J).lookahead == '-' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_SUB,
                line,
                a,
                multiplicative(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn shift(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = additive(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == TK_SHL as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_SHL,
                line,
                a,
                additive(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_SHR as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_SHR,
                line,
                a,
                additive(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(if (*J).lookahead == TK_USHR as ::core::ffi::c_int {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_USHR,
                line,
                a,
                additive(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn relational(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = shift(J);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == '<' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_LT,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == '>' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_GT,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_LE as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_LE,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_GE as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_GE,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_INSTANCEOF as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_INSTANCEOF,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(notin == 0
                && (if (*J).lookahead == TK_IN as ::core::ffi::c_int {
                    jsP_next(J);
                    1 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_IN,
                line,
                a,
                shift(J),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn equality(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = relational(J, notin);
    let mut line: ::core::ffi::c_int = 0;
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    loop {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        line = (*J).lexline;
        if if (*J).lookahead == TK_EQ as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_EQ,
                line,
                a,
                relational(J, notin),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_NE as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_NE,
                line,
                a,
                relational(J, notin),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else if if (*J).lookahead == TK_STRICTEQ as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            a = jsP_newnode(
                J,
                EXP_STRICTEQ,
                line,
                a,
                relational(J, notin),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            if !(if (*J).lookahead == TK_STRICTNE as ::core::ffi::c_int {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0)
            {
                break;
            }
            a = jsP_newnode(
                J,
                EXP_STRICTNE,
                line,
                a,
                relational(J, notin),
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn bitand(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = equality(J, notin);
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    let mut line: ::core::ffi::c_int = (*J).lexline;
    while if (*J).lookahead == '&' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_BITAND,
            line,
            a,
            equality(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn bitxor(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = bitand(J, notin);
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    let mut line: ::core::ffi::c_int = (*J).lexline;
    while if (*J).lookahead == '^' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_BITXOR,
            line,
            a,
            bitand(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn bitor(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = bitxor(J, notin);
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    let mut line: ::core::ffi::c_int = (*J).lexline;
    while if (*J).lookahead == '|' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_BITOR,
            line,
            a,
            bitxor(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn logand(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = bitor(J, notin);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == TK_AND as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_LOGAND,
            line,
            a,
            logand(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        (*J).astdepth -= 1;
    }
    return a;
}
unsafe extern "C" fn logor(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = logand(J, notin);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == TK_OR as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_LOGOR,
            line,
            a,
            logor(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        (*J).astdepth -= 1;
    }
    return a;
}
unsafe extern "C" fn conditional(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = logor(J, notin);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == '?' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
        let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        b = assignment(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ':' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(':' as i32),
            );
        }
        c = assignment(J, notin);
        (*J).astdepth -= 1;
        return jsP_newnode(
            J,
            EXP_COND,
            line,
            a,
            b,
            c,
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    return a;
}
unsafe extern "C" fn assignment(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = conditional(J, notin);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    (*J).astdepth += 1;
    if (*J).astdepth > JS_ASTLIMIT {
        jsP_error(J, b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if if (*J).lookahead == '=' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_MUL_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_MUL,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_DIV_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_DIV,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_MOD_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_MOD,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_ADD_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_ADD,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_SUB_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_SUB,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_SHL_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_SHL,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_SHR_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_SHR,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_USHR_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_USHR,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_AND_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_BITAND,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_XOR_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_BITXOR,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_OR_ASS as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = jsP_newnode(
            J,
            EXP_ASS_BITOR,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    (*J).astdepth -= 1;
    return a;
}
unsafe extern "C" fn expression(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = assignment(J, notin);
    let mut SAVE: ::core::ffi::c_int = (*J).astdepth;
    let mut line: ::core::ffi::c_int = (*J).lexline;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*J).astdepth += 1;
        if (*J).astdepth > JS_ASTLIMIT {
            jsP_error(
                J,
                b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        a = jsP_newnode(
            J,
            EXP_COMMA,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    return a;
}
unsafe extern "C" fn vardec(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = identifier(J);
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == '=' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return jsP_newnode(
            J,
            EXP_VAR,
            line,
            a,
            assignment(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    return jsP_newnode(
        J,
        EXP_VAR,
        line,
        a,
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
}
unsafe extern "C" fn vardeclist(
    mut J: *mut js_State,
    mut notin: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        vardec(J, notin),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while if (*J).lookahead == ',' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            vardec(J, notin),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn statementlist(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == '}' as i32 || (*J).lookahead == TK_CASE as ::core::ffi::c_int
        || (*J).lookahead == TK_DEFAULT as ::core::ffi::c_int
    {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        statement(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while (*J).lookahead != '}' as i32 && (*J).lookahead != TK_CASE as ::core::ffi::c_int
        && (*J).lookahead != TK_DEFAULT as ::core::ffi::c_int
    {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            statement(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn caseclause(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == TK_CASE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ':' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(':' as i32),
            );
        }
        b = statementlist(J);
        return jsP_newnode(
            J,
            STM_CASE,
            line,
            a,
            b,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    if if (*J).lookahead == TK_DEFAULT as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lookahead == ':' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(':' as i32),
            );
        }
        a = statementlist(J);
        return jsP_newnode(
            J,
            STM_DEFAULT,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    jsP_error(
        J,
        b"unexpected token in switch: %s (expected 'case' or 'default')\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn caselist(mut J: *mut js_State) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == '}' as i32 {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        caseclause(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while (*J).lookahead != '}' as i32 {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            caseclause(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn block(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == '{' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('{' as i32),
        );
    }
    a = statementlist(J);
    if if (*J).lookahead == '}' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('}' as i32),
        );
    }
    return jsP_newnode(
        J,
        STM_BLOCK,
        line,
        a,
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
}
unsafe extern "C" fn forexpression(
    mut J: *mut js_State,
    mut end: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead != end {
        a = expression(J, 0 as ::core::ffi::c_int);
    }
    if if (*J).lookahead == end {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(end),
        );
    }
    return a;
}
unsafe extern "C" fn forstatement(
    mut J: *mut js_State,
    mut line: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut d: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if if (*J).lookahead == '(' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('(' as i32),
        );
    }
    if if (*J).lookahead == TK_VAR as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = vardeclist(J, 1 as ::core::ffi::c_int);
        if if (*J).lookahead == ';' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            b = forexpression(J, ';' as i32);
            c = forexpression(J, ')' as i32);
            d = statement(J);
            return jsP_newnode(J, STM_FOR_VAR, line, a, b, c, d);
        }
        if if (*J).lookahead == TK_IN as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            b = expression(J, 0 as ::core::ffi::c_int);
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
            c = statement(J);
            return jsP_newnode(
                J,
                STM_FOR_IN_VAR,
                line,
                a,
                b,
                c,
                ::core::ptr::null_mut::<js_Ast>(),
            );
        }
        jsP_error(
            J,
            b"unexpected token in for-var-statement: %s\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
        );
    }
    if (*J).lookahead != ';' as i32 {
        a = expression(J, 1 as ::core::ffi::c_int);
    } else {
        a = ::core::ptr::null_mut::<js_Ast>();
    }
    if if (*J).lookahead == ';' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        b = forexpression(J, ';' as i32);
        c = forexpression(J, ')' as i32);
        d = statement(J);
        return jsP_newnode(J, STM_FOR, line, a, b, c, d);
    }
    if if (*J).lookahead == TK_IN as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        b = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        c = statement(J);
        return jsP_newnode(
            J,
            STM_FOR_IN,
            line,
            a,
            b,
            c,
            ::core::ptr::null_mut::<js_Ast>(),
        );
    }
    jsP_error(
        J,
        b"unexpected token in for-statement: %s\0" as *const u8
            as *const ::core::ffi::c_char,
        jsY_tokenstring((*J).lookahead),
    );
}
unsafe extern "C" fn statement(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut b: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut c: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut d: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut stm: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = (*J).lexline;
    (*J).astdepth += 1;
    if (*J).astdepth > JS_ASTLIMIT {
        jsP_error(J, b"too much recursion\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*J).lookahead == '{' as i32 {
        stm = block(J);
    } else if if (*J).lookahead == TK_VAR as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = vardeclist(J, 0 as ::core::ffi::c_int);
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_VAR,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == ';' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        stm = jsP_newnode(
            J,
            STM_EMPTY,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_IF as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('(' as i32),
            );
        }
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        b = statement(J);
        if if (*J).lookahead == TK_ELSE as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            c = statement(J);
        } else {
            c = ::core::ptr::null_mut::<js_Ast>();
        }
        stm = jsP_newnode(J, STM_IF, line, a, b, c, ::core::ptr::null_mut::<js_Ast>());
    } else if if (*J).lookahead == TK_DO as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = statement(J);
        if if (*J).lookahead == TK_WHILE as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(TK_WHILE as ::core::ffi::c_int),
            );
        }
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('(' as i32),
            );
        }
        b = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_DO,
            line,
            a,
            b,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_WHILE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('(' as i32),
            );
        }
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        b = statement(J);
        stm = jsP_newnode(
            J,
            STM_WHILE,
            line,
            a,
            b,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_FOR as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        stm = forstatement(J, line);
    } else if if (*J).lookahead == TK_CONTINUE as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = identifieropt(J);
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_CONTINUE,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_BREAK as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = identifieropt(J);
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_BREAK,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_RETURN as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if (*J).lookahead != ';' as i32 && (*J).lookahead != '}' as i32
            && (*J).lookahead != 0 as ::core::ffi::c_int
        {
            a = expression(J, 0 as ::core::ffi::c_int);
        } else {
            a = ::core::ptr::null_mut::<js_Ast>();
        }
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_RETURN,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_WITH as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('(' as i32),
            );
        }
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        b = statement(J);
        stm = jsP_newnode(
            J,
            STM_WITH,
            line,
            a,
            b,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_SWITCH as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lookahead == '(' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('(' as i32),
            );
        }
        a = expression(J, 0 as ::core::ffi::c_int);
        if if (*J).lookahead == ')' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring(')' as i32),
            );
        }
        if if (*J).lookahead == '{' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('{' as i32),
            );
        }
        b = caselist(J);
        if if (*J).lookahead == '}' as i32 {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } == 0
        {
            jsP_error(
                J,
                b"unexpected token: %s (expected %s)\0" as *const u8
                    as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
                jsY_tokenstring('}' as i32),
            );
        }
        stm = jsP_newnode(
            J,
            STM_SWITCH,
            line,
            a,
            b,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_THROW as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = expression(J, 0 as ::core::ffi::c_int);
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_THROW,
            line,
            a,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_TRY as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        a = block(J);
        d = ::core::ptr::null_mut::<js_Ast>();
        c = d;
        b = c;
        if if (*J).lookahead == TK_CATCH as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            if if (*J).lookahead == '(' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring('(' as i32),
                );
            }
            b = identifier(J);
            if if (*J).lookahead == ')' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsP_error(
                    J,
                    b"unexpected token: %s (expected %s)\0" as *const u8
                        as *const ::core::ffi::c_char,
                    jsY_tokenstring((*J).lookahead),
                    jsY_tokenstring(')' as i32),
                );
            }
            c = block(J);
        }
        if if (*J).lookahead == TK_FINALLY as ::core::ffi::c_int {
            jsP_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            d = block(J);
        }
        if b.is_null() && d.is_null() {
            jsP_error(
                J,
                b"unexpected token in try: %s (expected 'catch' or 'finally')\0"
                    as *const u8 as *const ::core::ffi::c_char,
                jsY_tokenstring((*J).lookahead),
            );
        }
        stm = jsP_newnode(J, STM_TRY, line, a, b, c, d);
    } else if if (*J).lookahead == TK_DEBUGGER as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        semicolon(J);
        stm = jsP_newnode(
            J,
            STM_DEBUGGER,
            line,
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
    } else if if (*J).lookahead == TK_FUNCTION as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        jsP_warning(
            J,
            b"function statements are not standard\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        stm = funstm(J, line);
    } else if (*J).lookahead == TK_IDENTIFIER as ::core::ffi::c_int {
        a = expression(J, 0 as ::core::ffi::c_int);
        if (*a).type_0 as ::core::ffi::c_uint
            == EXP_IDENTIFIER as ::core::ffi::c_int as ::core::ffi::c_uint
            && (if (*J).lookahead == ':' as i32 {
                jsP_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            }) != 0
        {
            (*a).type_0 = AST_IDENTIFIER;
            b = statement(J);
            stm = jsP_newnode(
                J,
                STM_LABEL,
                line,
                a,
                b,
                ::core::ptr::null_mut::<js_Ast>(),
                ::core::ptr::null_mut::<js_Ast>(),
            );
        } else {
            semicolon(J);
            stm = a;
        }
    } else {
        stm = expression(J, 0 as ::core::ffi::c_int);
        semicolon(J);
    }
    (*J).astdepth -= 1;
    return stm;
}
unsafe extern "C" fn scriptelement(mut J: *mut js_State) -> *mut js_Ast {
    let mut line: ::core::ffi::c_int = (*J).lexline;
    if if (*J).lookahead == TK_FUNCTION as ::core::ffi::c_int {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return fundec(J, line);
    }
    return statement(J);
}
unsafe extern "C" fn script(
    mut J: *mut js_State,
    mut terminator: ::core::ffi::c_int,
) -> *mut js_Ast {
    let mut head: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut tail: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if (*J).lookahead == terminator {
        return ::core::ptr::null_mut::<js_Ast>();
    }
    tail = jsP_newnode(
        J,
        AST_LIST,
        0 as ::core::ffi::c_int,
        scriptelement(J),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
        ::core::ptr::null_mut::<js_Ast>(),
    );
    head = tail;
    while (*J).lookahead != terminator {
        (*tail).b = jsP_newnode(
            J,
            AST_LIST,
            0 as ::core::ffi::c_int,
            scriptelement(J),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
            ::core::ptr::null_mut::<js_Ast>(),
        );
        tail = (*tail).b;
    }
    return jsP_list(head);
}
unsafe extern "C" fn funbody(mut J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    if if (*J).lookahead == '{' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('{' as i32),
        );
    }
    a = script(J, '}' as i32);
    if if (*J).lookahead == '}' as i32 {
        jsP_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsP_error(
            J,
            b"unexpected token: %s (expected %s)\0" as *const u8
                as *const ::core::ffi::c_char,
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring('}' as i32),
        );
    }
    return a;
}
unsafe extern "C" fn toint32(mut d: ::core::ffi::c_double) -> ::core::ffi::c_int {
    let mut two32: ::core::ffi::c_double = 4294967296.0f64;
    let mut two31: ::core::ffi::c_double = 2147483648.0f64;
    if d.is_finite() as i32 == 0 || d == 0 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        return 0 as ::core::ffi::c_int;
    }
    d = fmod(d, two32);
    d = if d >= 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        floor(d)
    } else {
        ceil(d) + two32
    };
    if d >= two31 {
        return (d - two32) as ::core::ffi::c_int
    } else {
        return d as ::core::ffi::c_int
    };
}
unsafe extern "C" fn touint32(mut d: ::core::ffi::c_double) -> ::core::ffi::c_uint {
    return toint32(d) as ::core::ffi::c_uint;
}
unsafe extern "C" fn jsP_setnumnode(
    mut node: *mut js_Ast,
    mut x: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    (*node).type_0 = EXP_NUMBER;
    (*node).number = x;
    (*node).d = ::core::ptr::null_mut::<js_Ast>();
    (*node).c = (*node).d;
    (*node).b = (*node).c;
    (*node).a = (*node).b;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsP_foldconst(mut node: *mut js_Ast) -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_double = 0.;
    let mut y: ::core::ffi::c_double = 0.;
    let mut a: ::core::ffi::c_int = 0;
    let mut b: ::core::ffi::c_int = 0;
    if (*node).type_0 as ::core::ffi::c_uint
        == AST_LIST as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        while !node.is_null() {
            jsP_foldconst((*node).a);
            node = (*node).b;
        }
        return 0 as ::core::ffi::c_int;
    }
    if (*node).type_0 as ::core::ffi::c_uint
        == EXP_NUMBER as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as ::core::ffi::c_int;
    }
    a = if !(*node).a.is_null() {
        jsP_foldconst((*node).a)
    } else {
        0 as ::core::ffi::c_int
    };
    b = if !(*node).b.is_null() {
        jsP_foldconst((*node).b)
    } else {
        0 as ::core::ffi::c_int
    };
    if !(*node).c.is_null() {
        jsP_foldconst((*node).c);
    }
    if !(*node).d.is_null() {
        jsP_foldconst((*node).d);
    }
    if a != 0 {
        x = (*(*node).a).number;
        match (*node).type_0 as ::core::ffi::c_uint {
            30 => return jsP_setnumnode(node, -x),
            29 => return jsP_setnumnode(node, x),
            31 => return jsP_setnumnode(node, !toint32(x) as ::core::ffi::c_double),
            _ => {}
        }
        if b != 0 {
            y = (*(*node).b).number;
            match (*node).type_0 as ::core::ffi::c_uint {
                35 => return jsP_setnumnode(node, x * y),
                34 => return jsP_setnumnode(node, x / y),
                33 => return jsP_setnumnode(node, fmod(x, y)),
                37 => return jsP_setnumnode(node, x + y),
                36 => return jsP_setnumnode(node, x - y),
                40 => {
                    return jsP_setnumnode(
                        node,
                        (toint32(x) << (touint32(y) & 0x1f as ::core::ffi::c_uint))
                            as ::core::ffi::c_double,
                    );
                }
                39 => {
                    return jsP_setnumnode(
                        node,
                        (toint32(x) >> (touint32(y) & 0x1f as ::core::ffi::c_uint))
                            as ::core::ffi::c_double,
                    );
                }
                38 => {
                    return jsP_setnumnode(
                        node,
                        (touint32(x) >> (touint32(y) & 0x1f as ::core::ffi::c_uint))
                            as ::core::ffi::c_double,
                    );
                }
                51 => {
                    return jsP_setnumnode(
                        node,
                        (toint32(x) & toint32(y)) as ::core::ffi::c_double,
                    );
                }
                52 => {
                    return jsP_setnumnode(
                        node,
                        (toint32(x) ^ toint32(y)) as ::core::ffi::c_double,
                    );
                }
                53 => {
                    return jsP_setnumnode(
                        node,
                        (toint32(x) | toint32(y)) as ::core::ffi::c_double,
                    );
                }
                _ => {}
            }
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsP_parse(
    mut J: *mut js_State,
    mut filename: *const ::core::ffi::c_char,
    mut source: *const ::core::ffi::c_char,
) -> *mut js_Ast {
    let mut p: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    jsY_initlex(J, filename, source);
    jsP_next(J);
    (*J).astdepth = 0 as ::core::ffi::c_int;
    p = script(J, 0 as ::core::ffi::c_int);
    if !p.is_null() {
        jsP_foldconst(p);
    }
    return p;
}
#[no_mangle]
pub unsafe extern "C" fn jsP_parsefunction(
    mut J: *mut js_State,
    mut filename: *const ::core::ffi::c_char,
    mut params: *const ::core::ffi::c_char,
    mut body: *const ::core::ffi::c_char,
) -> *mut js_Ast {
    let mut p: *mut js_Ast = ::core::ptr::null_mut::<js_Ast>();
    let mut line: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !params.is_null() {
        jsY_initlex(J, filename, params);
        jsP_next(J);
        (*J).astdepth = 0 as ::core::ffi::c_int;
        p = parameters(J);
    }
    return jsP_newnode(
        J,
        EXP_FUN,
        line,
        ::core::ptr::null_mut::<js_Ast>(),
        p,
        jsP_parse(J, filename, body),
        ::core::ptr::null_mut::<js_Ast>(),
    );
}
pub const JS_ASTLIMIT: ::core::ffi::c_int = 400 as ::core::ffi::c_int;
