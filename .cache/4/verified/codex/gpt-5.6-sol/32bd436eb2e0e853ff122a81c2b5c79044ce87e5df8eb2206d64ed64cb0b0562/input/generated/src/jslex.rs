extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
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
    fn jsU_chartorune(
        rune: *mut Rune,
        str: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn jsU_runetochar(
        str: *mut ::core::ffi::c_char,
        rune: *const Rune,
    ) -> ::core::ffi::c_int;
    fn jsU_runelen(c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn jsU_isalpharune(c: Rune) -> ::core::ffi::c_int;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_realloc(
        J: *mut js_State,
        ptr: *mut ::core::ffi::c_void,
        size: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void;
    fn js_strtod(
        as_0: *const ::core::ffi::c_char,
        aas: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
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
pub type C2RustUnnamed_9 = ::core::ffi::c_uint;
pub const JS_REGEXP_M: C2RustUnnamed_9 = 4;
pub const JS_REGEXP_I: C2RustUnnamed_9 = 2;
pub const JS_REGEXP_G: C2RustUnnamed_9 = 1;
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
pub type C2RustUnnamed_10 = ::core::ffi::c_uint;
pub const TK_WITH: C2RustUnnamed_10 = 312;
pub const TK_WHILE: C2RustUnnamed_10 = 311;
pub const TK_VOID: C2RustUnnamed_10 = 310;
pub const TK_VAR: C2RustUnnamed_10 = 309;
pub const TK_TYPEOF: C2RustUnnamed_10 = 308;
pub const TK_TRY: C2RustUnnamed_10 = 307;
pub const TK_TRUE: C2RustUnnamed_10 = 306;
pub const TK_THROW: C2RustUnnamed_10 = 305;
pub const TK_THIS: C2RustUnnamed_10 = 304;
pub const TK_SWITCH: C2RustUnnamed_10 = 303;
pub const TK_RETURN: C2RustUnnamed_10 = 302;
pub const TK_NULL: C2RustUnnamed_10 = 301;
pub const TK_NEW: C2RustUnnamed_10 = 300;
pub const TK_INSTANCEOF: C2RustUnnamed_10 = 299;
pub const TK_IN: C2RustUnnamed_10 = 298;
pub const TK_IF: C2RustUnnamed_10 = 297;
pub const TK_FUNCTION: C2RustUnnamed_10 = 296;
pub const TK_FOR: C2RustUnnamed_10 = 295;
pub const TK_FINALLY: C2RustUnnamed_10 = 294;
pub const TK_FALSE: C2RustUnnamed_10 = 293;
pub const TK_ELSE: C2RustUnnamed_10 = 292;
pub const TK_DO: C2RustUnnamed_10 = 291;
pub const TK_DELETE: C2RustUnnamed_10 = 290;
pub const TK_DEFAULT: C2RustUnnamed_10 = 289;
pub const TK_DEBUGGER: C2RustUnnamed_10 = 288;
pub const TK_CONTINUE: C2RustUnnamed_10 = 287;
pub const TK_CATCH: C2RustUnnamed_10 = 286;
pub const TK_CASE: C2RustUnnamed_10 = 285;
pub const TK_BREAK: C2RustUnnamed_10 = 284;
pub const TK_DEC: C2RustUnnamed_10 = 283;
pub const TK_INC: C2RustUnnamed_10 = 282;
pub const TK_XOR_ASS: C2RustUnnamed_10 = 281;
pub const TK_OR_ASS: C2RustUnnamed_10 = 280;
pub const TK_AND_ASS: C2RustUnnamed_10 = 279;
pub const TK_USHR_ASS: C2RustUnnamed_10 = 278;
pub const TK_SHR_ASS: C2RustUnnamed_10 = 277;
pub const TK_SHL_ASS: C2RustUnnamed_10 = 276;
pub const TK_MOD_ASS: C2RustUnnamed_10 = 275;
pub const TK_DIV_ASS: C2RustUnnamed_10 = 274;
pub const TK_MUL_ASS: C2RustUnnamed_10 = 273;
pub const TK_SUB_ASS: C2RustUnnamed_10 = 272;
pub const TK_ADD_ASS: C2RustUnnamed_10 = 271;
pub const TK_OR: C2RustUnnamed_10 = 270;
pub const TK_AND: C2RustUnnamed_10 = 269;
pub const TK_USHR: C2RustUnnamed_10 = 268;
pub const TK_SHR: C2RustUnnamed_10 = 267;
pub const TK_SHL: C2RustUnnamed_10 = 266;
pub const TK_STRICTNE: C2RustUnnamed_10 = 265;
pub const TK_STRICTEQ: C2RustUnnamed_10 = 264;
pub const TK_NE: C2RustUnnamed_10 = 263;
pub const TK_EQ: C2RustUnnamed_10 = 262;
pub const TK_GE: C2RustUnnamed_10 = 261;
pub const TK_LE: C2RustUnnamed_10 = 260;
pub const TK_REGEXP: C2RustUnnamed_10 = 259;
pub const TK_STRING: C2RustUnnamed_10 = 258;
pub const TK_NUMBER: C2RustUnnamed_10 = 257;
pub const TK_IDENTIFIER: C2RustUnnamed_10 = 256;
pub type Rune = ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const _IO_EOF_SEEN: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const _IO_ERR_SEEN: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const EOF: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
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
unsafe extern "C" fn jsY_error(
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
static mut tokenstring: [*const ::core::ffi::c_char; 313] = [
    b"(end-of-file)\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x01'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x02'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x03'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x04'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x05'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x06'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x07'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x08'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x09'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0A'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0B'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0C'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0D'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0E'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x0F'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x10'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x11'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x12'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x13'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x14'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x15'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x16'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x17'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x18'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x19'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1A'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1B'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1C'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1D'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1E'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x1F'\0" as *const u8 as *const ::core::ffi::c_char,
    b"' '\0" as *const u8 as *const ::core::ffi::c_char,
    b"'!'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\"'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'#'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'$'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'%'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'&'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\''\0" as *const u8 as *const ::core::ffi::c_char,
    b"'('\0" as *const u8 as *const ::core::ffi::c_char,
    b"')'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'*'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'+'\0" as *const u8 as *const ::core::ffi::c_char,
    b"','\0" as *const u8 as *const ::core::ffi::c_char,
    b"'-'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'.'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'/'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'0'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'1'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'2'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'3'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'4'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'5'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'6'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'7'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'8'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'9'\0" as *const u8 as *const ::core::ffi::c_char,
    b"':'\0" as *const u8 as *const ::core::ffi::c_char,
    b"';'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'<'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'?'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'@'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'A'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'B'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'C'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'D'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'E'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'F'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'G'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'H'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'I'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'J'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'K'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'L'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'M'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'N'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'O'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'P'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'Q'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'R'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'S'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'T'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'U'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'V'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'W'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'X'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'Y'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'Z'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'['\0" as *const u8 as *const ::core::ffi::c_char,
    b"''\0" as *const u8 as *const ::core::ffi::c_char,
    b"']'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'^'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'_'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'`'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'a'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'b'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'c'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'d'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'e'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'f'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'g'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'h'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'i'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'j'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'k'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'l'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'m'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'n'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'o'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'p'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'q'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'r'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'s'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'t'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'u'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'v'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'w'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'x'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'y'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'z'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'{'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'|'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'}'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'~'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'\\x7F'\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    ::core::ptr::null::<::core::ffi::c_char>(),
    b"(identifier)\0" as *const u8 as *const ::core::ffi::c_char,
    b"(number)\0" as *const u8 as *const ::core::ffi::c_char,
    b"(string)\0" as *const u8 as *const ::core::ffi::c_char,
    b"(regexp)\0" as *const u8 as *const ::core::ffi::c_char,
    b"'<='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'=='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'!='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'==='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'!=='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'<<'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>>'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>>>'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'&&'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'||'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'+='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'-='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'*='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'/='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'%='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'<<='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>>='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'>>>='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'&='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'|='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'^='\0" as *const u8 as *const ::core::ffi::c_char,
    b"'++'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'--'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'break'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'case'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'catch'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'continue'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'debugger'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'default'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'delete'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'do'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'else'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'false'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'finally'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'for'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'function'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'if'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'in'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'instanceof'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'new'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'null'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'return'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'switch'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'this'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'throw'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'true'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'try'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'typeof'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'var'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'void'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'while'\0" as *const u8 as *const ::core::ffi::c_char,
    b"'with'\0" as *const u8 as *const ::core::ffi::c_char,
];
#[no_mangle]
pub unsafe extern "C" fn jsY_tokenstring(
    mut token: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    if token >= 0 as ::core::ffi::c_int
        && token
            < (::core::mem::size_of::<[*const ::core::ffi::c_char; 313]>() as usize)
                .wrapping_div(
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as usize,
                ) as ::core::ffi::c_int
    {
        if !tokenstring[token as usize].is_null() {
            return tokenstring[token as usize];
        }
    }
    return b"<unknown>\0" as *const u8 as *const ::core::ffi::c_char;
}
static mut keywords: [*const ::core::ffi::c_char; 29] = [
    b"break\0" as *const u8 as *const ::core::ffi::c_char,
    b"case\0" as *const u8 as *const ::core::ffi::c_char,
    b"catch\0" as *const u8 as *const ::core::ffi::c_char,
    b"continue\0" as *const u8 as *const ::core::ffi::c_char,
    b"debugger\0" as *const u8 as *const ::core::ffi::c_char,
    b"default\0" as *const u8 as *const ::core::ffi::c_char,
    b"delete\0" as *const u8 as *const ::core::ffi::c_char,
    b"do\0" as *const u8 as *const ::core::ffi::c_char,
    b"else\0" as *const u8 as *const ::core::ffi::c_char,
    b"false\0" as *const u8 as *const ::core::ffi::c_char,
    b"finally\0" as *const u8 as *const ::core::ffi::c_char,
    b"for\0" as *const u8 as *const ::core::ffi::c_char,
    b"function\0" as *const u8 as *const ::core::ffi::c_char,
    b"if\0" as *const u8 as *const ::core::ffi::c_char,
    b"in\0" as *const u8 as *const ::core::ffi::c_char,
    b"instanceof\0" as *const u8 as *const ::core::ffi::c_char,
    b"new\0" as *const u8 as *const ::core::ffi::c_char,
    b"null\0" as *const u8 as *const ::core::ffi::c_char,
    b"return\0" as *const u8 as *const ::core::ffi::c_char,
    b"switch\0" as *const u8 as *const ::core::ffi::c_char,
    b"this\0" as *const u8 as *const ::core::ffi::c_char,
    b"throw\0" as *const u8 as *const ::core::ffi::c_char,
    b"true\0" as *const u8 as *const ::core::ffi::c_char,
    b"try\0" as *const u8 as *const ::core::ffi::c_char,
    b"typeof\0" as *const u8 as *const ::core::ffi::c_char,
    b"var\0" as *const u8 as *const ::core::ffi::c_char,
    b"void\0" as *const u8 as *const ::core::ffi::c_char,
    b"while\0" as *const u8 as *const ::core::ffi::c_char,
    b"with\0" as *const u8 as *const ::core::ffi::c_char,
];
#[no_mangle]
pub unsafe extern "C" fn jsY_findword(
    mut s: *const ::core::ffi::c_char,
    mut list: *mut *const ::core::ffi::c_char,
    mut num: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut l: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut r: ::core::ffi::c_int = num - 1 as ::core::ffi::c_int;
    while l <= r {
        let mut m: ::core::ffi::c_int = l + r >> 1 as ::core::ffi::c_int;
        let mut c: ::core::ffi::c_int = strcmp(s, *list.offset(m as isize));
        if c < 0 as ::core::ffi::c_int {
            r = m - 1 as ::core::ffi::c_int;
        } else if c > 0 as ::core::ffi::c_int {
            l = m + 1 as ::core::ffi::c_int;
        } else {
            return m
        }
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn jsY_findkeyword(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = jsY_findword(
        s,
        &raw mut keywords as *mut *const ::core::ffi::c_char,
        (::core::mem::size_of::<[*const ::core::ffi::c_char; 29]>() as usize)
            .wrapping_div(::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
            as ::core::ffi::c_int,
    );
    if i >= 0 as ::core::ffi::c_int {
        (*J).text = keywords[i as usize];
        return TK_BREAK as ::core::ffi::c_int + i;
    }
    (*J).text = s;
    return TK_IDENTIFIER as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsY_iswhite(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c == 0x9 as ::core::ffi::c_int || c == 0xb as ::core::ffi::c_int
        || c == 0xc as ::core::ffi::c_int || c == 0x20 as ::core::ffi::c_int
        || c == 0xa0 as ::core::ffi::c_int || c == 0xfeff as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsY_isnewline(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c == 0xa as ::core::ffi::c_int || c == 0xd as ::core::ffi::c_int
        || c == 0x2028 as ::core::ffi::c_int || c == 0x2029 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn jsY_isidentifierstart(
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return (c >= 'a' as i32 && c <= 'z' as i32 || c >= 'A' as i32 && c <= 'Z' as i32
        || c == '$' as i32 || c == '_' as i32 || jsU_isalpharune(c as Rune) != 0)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn jsY_isidentifierpart(
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return (c >= '0' as i32 && c <= '9' as i32
        || (c >= 'a' as i32 && c <= 'z' as i32 || c >= 'A' as i32 && c <= 'Z' as i32)
        || c == '$' as i32 || c == '_' as i32 || jsU_isalpharune(c as Rune) != 0)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn jsY_isdec(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c >= '0' as i32 && c <= '9' as i32) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsY_ishex(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c >= '0' as i32 && c <= '9' as i32
        || (c >= 'a' as i32 && c <= 'f' as i32 || c >= 'A' as i32 && c <= 'F' as i32))
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsY_tohex(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if c >= '0' as i32 && c <= '9' as i32 {
        return c - '0' as i32;
    }
    if c >= 'a' as i32 && c <= 'f' as i32 {
        return c - 'a' as i32 + 0xa as ::core::ffi::c_int;
    }
    if c >= 'A' as i32 && c <= 'F' as i32 {
        return c - 'A' as i32 + 0xa as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsY_next(mut J: *mut js_State) {
    let mut c: Rune = 0;
    if *(*J).source as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        (*J).lexchar = EOF;
        return;
    }
    (*J).source = (*J).source.offset(jsU_chartorune(&raw mut c, (*J).source) as isize);
    if c == '\r' as i32 && *(*J).source as ::core::ffi::c_int == '\n' as i32 {
        (*J).source = (*J).source.offset(1);
    }
    if jsY_isnewline(c as ::core::ffi::c_int) != 0 {
        (*J).line += 1;
        c = '\n' as i32 as Rune;
    }
    (*J).lexchar = c as ::core::ffi::c_int;
}
unsafe extern "C" fn jsY_unescape(mut J: *mut js_State) {
    if if (*J).lexchar == '\\' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if if (*J).lexchar == 'u' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            if !(jsY_ishex((*J).lexchar) == 0) {
                x |= jsY_tohex((*J).lexchar) << 12 as ::core::ffi::c_int;
                jsY_next(J);
                if !(jsY_ishex((*J).lexchar) == 0) {
                    x |= jsY_tohex((*J).lexchar) << 8 as ::core::ffi::c_int;
                    jsY_next(J);
                    if !(jsY_ishex((*J).lexchar) == 0) {
                        x |= jsY_tohex((*J).lexchar) << 4 as ::core::ffi::c_int;
                        jsY_next(J);
                        if !(jsY_ishex((*J).lexchar) == 0) {
                            x |= jsY_tohex((*J).lexchar);
                            (*J).lexchar = x;
                            return;
                        }
                    }
                }
            }
        }
        jsY_error(
            J,
            b"unexpected escape sequence\0" as *const u8 as *const ::core::ffi::c_char,
        );
    } else {
        return;
    };
}
unsafe extern "C" fn textinit(mut J: *mut js_State) {
    if (*J).lexbuf.text.is_null() {
        (*J).lexbuf.cap = 4096 as ::core::ffi::c_int;
        (*J).lexbuf.text = js_malloc(J, (*J).lexbuf.cap) as *mut ::core::ffi::c_char;
    }
    (*J).lexbuf.len = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn textpush(mut J: *mut js_State, mut c: Rune) {
    let mut n: ::core::ffi::c_int = 0;
    let mut newcap: ::core::ffi::c_int = 0;
    if c == EOF {
        n = 1 as ::core::ffi::c_int;
    } else {
        n = jsU_runelen(c as ::core::ffi::c_int);
    }
    if (*J).lexbuf.len + n > (*J).lexbuf.cap {
        newcap = (*J).lexbuf.cap * 2 as ::core::ffi::c_int;
        (*J).lexbuf.text = js_realloc(
            J,
            (*J).lexbuf.text as *mut ::core::ffi::c_void,
            newcap,
        ) as *mut ::core::ffi::c_char;
        (*J).lexbuf.cap = newcap;
    }
    if c == EOF {
        let fresh6 = (*J).lexbuf.len;
        (*J).lexbuf.len = (*J).lexbuf.len + 1;
        *(*J).lexbuf.text.offset(fresh6 as isize) = 0 as ::core::ffi::c_char;
    } else {
        (*J).lexbuf.len
            += jsU_runetochar(
                (*J).lexbuf.text.offset((*J).lexbuf.len as isize),
                &raw mut c,
            );
    };
}
unsafe extern "C" fn textend(mut J: *mut js_State) -> *mut ::core::ffi::c_char {
    textpush(J, EOF);
    return (*J).lexbuf.text;
}
unsafe extern "C" fn lexlinecomment(mut J: *mut js_State) {
    while (*J).lexchar != EOF && (*J).lexchar != '\n' as i32 {
        jsY_next(J);
    }
}
unsafe extern "C" fn lexcomment(mut J: *mut js_State) -> ::core::ffi::c_int {
    while (*J).lexchar != EOF {
        if if (*J).lexchar == '*' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            while (*J).lexchar == '*' as i32 {
                jsY_next(J);
            }
            if if (*J).lexchar == '/' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0
            {
                return 0 as ::core::ffi::c_int;
            }
        } else {
            jsY_next(J);
        }
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn lexhex(mut J: *mut js_State) -> ::core::ffi::c_double {
    let mut n: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    if jsY_ishex((*J).lexchar) == 0 {
        jsY_error(
            J,
            b"malformed hexadecimal number\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    while jsY_ishex((*J).lexchar) != 0 {
        n = n * 16 as ::core::ffi::c_int as ::core::ffi::c_double
            + jsY_tohex((*J).lexchar) as ::core::ffi::c_double;
        jsY_next(J);
    }
    return n;
}
unsafe extern "C" fn lexnumber(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = (*J)
        .source
        .offset(-(1 as ::core::ffi::c_int as isize));
    if if (*J).lexchar == '0' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if (if (*J).lexchar == 'x' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) != 0
            || (if (*J).lexchar == 'X' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            }) != 0
        {
            (*J).number = lexhex(J);
            return TK_NUMBER as ::core::ffi::c_int;
        }
        if jsY_isdec((*J).lexchar) != 0 {
            jsY_error(
                J,
                b"number with leading zero\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        if if (*J).lexchar == '.' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    } else if if (*J).lexchar == '.' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if jsY_isdec((*J).lexchar) == 0 {
            return '.' as i32;
        }
        while jsY_isdec((*J).lexchar) != 0 {
            jsY_next(J);
        }
    } else {
        while jsY_isdec((*J).lexchar) != 0 {
            jsY_next(J);
        }
        if if (*J).lexchar == '.' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    }
    if (if (*J).lexchar == 'e' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) != 0
        || (if (*J).lexchar == 'E' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) != 0
    {
        if (*J).lexchar == '-' as i32 || (*J).lexchar == '+' as i32 {
            jsY_next(J);
        }
        if jsY_isdec((*J).lexchar) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        } else {
            jsY_error(
                J,
                b"missing exponent\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    }
    if jsY_isidentifierstart((*J).lexchar) != 0 {
        jsY_error(
            J,
            b"number with letter suffix\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    (*J).number = js_strtod(s, ::core::ptr::null_mut::<*mut ::core::ffi::c_char>());
    return TK_NUMBER as ::core::ffi::c_int;
}
unsafe extern "C" fn lexescape(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if if (*J).lexchar == '\n' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        return 0 as ::core::ffi::c_int;
    }
    match (*J).lexchar {
        EOF => {
            jsY_error(
                J,
                b"unterminated escape sequence\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        117 => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 12 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 8 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 4 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x as Rune);
        }
        120 => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 4 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x as Rune);
        }
        48 => {
            textpush(J, 0 as Rune);
            jsY_next(J);
        }
        92 => {
            textpush(J, '\\' as i32);
            jsY_next(J);
        }
        39 => {
            textpush(J, '\'' as i32);
            jsY_next(J);
        }
        34 => {
            textpush(J, '"' as i32);
            jsY_next(J);
        }
        98 => {
            textpush(J, '\u{8}' as i32);
            jsY_next(J);
        }
        102 => {
            textpush(J, '\u{c}' as i32);
            jsY_next(J);
        }
        110 => {
            textpush(J, '\n' as i32);
            jsY_next(J);
        }
        114 => {
            textpush(J, '\r' as i32);
            jsY_next(J);
        }
        116 => {
            textpush(J, '\t' as i32);
            jsY_next(J);
        }
        118 => {
            textpush(J, '\u{b}' as i32);
            jsY_next(J);
        }
        _ => {
            textpush(J, (*J).lexchar as Rune);
            jsY_next(J);
        }
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn lexstring(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut q: ::core::ffi::c_int = (*J).lexchar;
    jsY_next(J);
    textinit(J);
    while (*J).lexchar != q {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as i32 {
            jsY_error(
                J,
                b"string not terminated\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        if if (*J).lexchar == '\\' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            if lexescape(J) != 0 {
                jsY_error(
                    J,
                    b"malformed escape sequence\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
            }
        } else {
            textpush(J, (*J).lexchar as Rune);
            jsY_next(J);
        }
    }
    if if (*J).lexchar == q {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsY_error(J, b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char, q);
    }
    s = textend(J);
    (*J).text = s;
    return TK_STRING as ::core::ffi::c_int;
}
unsafe extern "C" fn isregexpcontext(
    mut last: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    match last {
        93 | 41 | 125 | 256 | 257 | 258 | 293 | 301 | 304 | 306 => {
            return 0 as ::core::ffi::c_int;
        }
        _ => return 1 as ::core::ffi::c_int,
    };
}
unsafe extern "C" fn lexregexp(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut g: ::core::ffi::c_int = 0;
    let mut m: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut flags: ::core::ffi::c_int = 0;
    let mut inclass: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    textinit(J);
    while (*J).lexchar != '/' as i32 || inclass != 0 {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as i32 {
            jsY_error(
                J,
                b"regular expression not terminated\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        } else if if (*J).lexchar == '\\' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            if if (*J).lexchar == '/' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0
            {
                textpush(J, '/' as i32);
            } else {
                textpush(J, '\\' as i32);
                if (*J).lexchar == EOF || (*J).lexchar == '\n' as i32 {
                    jsY_error(
                        J,
                        b"regular expression not terminated\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
                textpush(J, (*J).lexchar as Rune);
                jsY_next(J);
            }
        } else {
            if (*J).lexchar == '[' as i32 && inclass == 0 {
                inclass = 1 as ::core::ffi::c_int;
            }
            if (*J).lexchar == ']' as i32 && inclass != 0 {
                inclass = 0 as ::core::ffi::c_int;
            }
            textpush(J, (*J).lexchar as Rune);
            jsY_next(J);
        }
    }
    if if (*J).lexchar == '/' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsY_error(
            J,
            b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
            '/' as i32,
        );
    }
    s = textend(J);
    m = 0 as ::core::ffi::c_int;
    i = m;
    g = i;
    while jsY_isidentifierpart((*J).lexchar) != 0 {
        if if (*J).lexchar == 'g' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            g += 1;
        } else if if (*J).lexchar == 'i' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            i += 1;
        } else if if (*J).lexchar == 'm' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            m += 1;
        } else {
            jsY_error(
                J,
                b"illegal flag in regular expression: %c\0" as *const u8
                    as *const ::core::ffi::c_char,
                (*J).lexchar,
            );
        }
    }
    if g > 1 as ::core::ffi::c_int || i > 1 as ::core::ffi::c_int
        || m > 1 as ::core::ffi::c_int
    {
        jsY_error(
            J,
            b"duplicated flag in regular expression\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    (*J).text = s;
    flags = 0 as ::core::ffi::c_int;
    if g != 0 {
        flags |= JS_REGEXP_G as ::core::ffi::c_int;
    }
    if i != 0 {
        flags |= JS_REGEXP_I as ::core::ffi::c_int;
    }
    if m != 0 {
        flags |= JS_REGEXP_M as ::core::ffi::c_int;
    }
    (*J).number = flags as ::core::ffi::c_double;
    return TK_REGEXP as ::core::ffi::c_int;
}
unsafe extern "C" fn isnlthcontext(mut last: ::core::ffi::c_int) -> ::core::ffi::c_int {
    match last {
        284 | 287 | 302 | 305 => return 1 as ::core::ffi::c_int,
        _ => return 0 as ::core::ffi::c_int,
    };
}
unsafe extern "C" fn jsY_lexx(mut J: *mut js_State) -> ::core::ffi::c_int {
    (*J).newline = 0 as ::core::ffi::c_int;
    loop {
        (*J).lexline = (*J).line;
        while jsY_iswhite((*J).lexchar) != 0 {
            jsY_next(J);
        }
        if if (*J).lexchar == '\n' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            (*J).newline = 1 as ::core::ffi::c_int;
            if isnlthcontext((*J).lasttoken) != 0 {
                return ';' as i32;
            }
        } else if if (*J).lexchar == '/' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            if if (*J).lexchar == '/' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0
            {
                lexlinecomment(J);
            } else if if (*J).lexchar == '*' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0
            {
                if lexcomment(J) != 0 {
                    jsY_error(
                        J,
                        b"multi-line comment not terminated\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
            } else if isregexpcontext((*J).lasttoken) != 0 {
                return lexregexp(J)
            } else if if (*J).lexchar == '=' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } != 0
            {
                return TK_DIV_ASS as ::core::ffi::c_int
            } else {
                return '/' as i32
            }
        } else {
            if (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
                return lexnumber(J);
            }
            match (*J).lexchar {
                40 => {
                    jsY_next(J);
                    return '(' as i32;
                }
                41 => {
                    jsY_next(J);
                    return ')' as i32;
                }
                44 => {
                    jsY_next(J);
                    return ',' as i32;
                }
                58 => {
                    jsY_next(J);
                    return ':' as i32;
                }
                59 => {
                    jsY_next(J);
                    return ';' as i32;
                }
                63 => {
                    jsY_next(J);
                    return '?' as i32;
                }
                91 => {
                    jsY_next(J);
                    return '[' as i32;
                }
                93 => {
                    jsY_next(J);
                    return ']' as i32;
                }
                123 => {
                    jsY_next(J);
                    return '{' as i32;
                }
                125 => {
                    jsY_next(J);
                    return '}' as i32;
                }
                126 => {
                    jsY_next(J);
                    return '~' as i32;
                }
                39 | 34 => return lexstring(J),
                46 => return lexnumber(J),
                60 => {
                    jsY_next(J);
                    if if (*J).lexchar == '<' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        if if (*J).lexchar == '=' as i32 {
                            jsY_next(J);
                            1 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        } != 0
                        {
                            return TK_SHL_ASS as ::core::ffi::c_int;
                        }
                        return TK_SHL as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_LE as ::core::ffi::c_int;
                    }
                    return '<' as i32;
                }
                62 => {
                    jsY_next(J);
                    if if (*J).lexchar == '>' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        if if (*J).lexchar == '>' as i32 {
                            jsY_next(J);
                            1 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        } != 0
                        {
                            if if (*J).lexchar == '=' as i32 {
                                jsY_next(J);
                                1 as ::core::ffi::c_int
                            } else {
                                0 as ::core::ffi::c_int
                            } != 0
                            {
                                return TK_USHR_ASS as ::core::ffi::c_int;
                            }
                            return TK_USHR as ::core::ffi::c_int;
                        }
                        if if (*J).lexchar == '=' as i32 {
                            jsY_next(J);
                            1 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        } != 0
                        {
                            return TK_SHR_ASS as ::core::ffi::c_int;
                        }
                        return TK_SHR as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_GE as ::core::ffi::c_int;
                    }
                    return '>' as i32;
                }
                61 => {
                    jsY_next(J);
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        if if (*J).lexchar == '=' as i32 {
                            jsY_next(J);
                            1 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        } != 0
                        {
                            return TK_STRICTEQ as ::core::ffi::c_int;
                        }
                        return TK_EQ as ::core::ffi::c_int;
                    }
                    return '=' as i32;
                }
                33 => {
                    jsY_next(J);
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        if if (*J).lexchar == '=' as i32 {
                            jsY_next(J);
                            1 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        } != 0
                        {
                            return TK_STRICTNE as ::core::ffi::c_int;
                        }
                        return TK_NE as ::core::ffi::c_int;
                    }
                    return '!' as i32;
                }
                43 => {
                    jsY_next(J);
                    if if (*J).lexchar == '+' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_INC as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_ADD_ASS as ::core::ffi::c_int;
                    }
                    return '+' as i32;
                }
                45 => {
                    jsY_next(J);
                    if if (*J).lexchar == '-' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_DEC as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_SUB_ASS as ::core::ffi::c_int;
                    }
                    return '-' as i32;
                }
                42 => {
                    jsY_next(J);
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_MUL_ASS as ::core::ffi::c_int;
                    }
                    return '*' as i32;
                }
                37 => {
                    jsY_next(J);
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_MOD_ASS as ::core::ffi::c_int;
                    }
                    return '%' as i32;
                }
                38 => {
                    jsY_next(J);
                    if if (*J).lexchar == '&' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_AND as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_AND_ASS as ::core::ffi::c_int;
                    }
                    return '&' as i32;
                }
                124 => {
                    jsY_next(J);
                    if if (*J).lexchar == '|' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_OR as ::core::ffi::c_int;
                    }
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_OR_ASS as ::core::ffi::c_int;
                    }
                    return '|' as i32;
                }
                94 => {
                    jsY_next(J);
                    if if (*J).lexchar == '=' as i32 {
                        jsY_next(J);
                        1 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    } != 0
                    {
                        return TK_XOR_ASS as ::core::ffi::c_int;
                    }
                    return '^' as i32;
                }
                EOF => return 0 as ::core::ffi::c_int,
                _ => {}
            }
            jsY_unescape(J);
            if jsY_isidentifierstart((*J).lexchar) != 0 {
                textinit(J);
                textpush(J, (*J).lexchar as Rune);
                jsY_next(J);
                jsY_unescape(J);
                while jsY_isidentifierpart((*J).lexchar) != 0 {
                    textpush(J, (*J).lexchar as Rune);
                    jsY_next(J);
                    jsY_unescape(J);
                }
                textend(J);
                return jsY_findkeyword(J, (*J).lexbuf.text);
            }
            if (*J).lexchar >= 0x20 as ::core::ffi::c_int
                && (*J).lexchar <= 0x7e as ::core::ffi::c_int
            {
                jsY_error(
                    J,
                    b"unexpected character: '%c'\0" as *const u8
                        as *const ::core::ffi::c_char,
                    (*J).lexchar,
                );
            }
            jsY_error(
                J,
                b"unexpected character: \\u%04X\0" as *const u8
                    as *const ::core::ffi::c_char,
                (*J).lexchar,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn jsY_initlex(
    mut J: *mut js_State,
    mut filename: *const ::core::ffi::c_char,
    mut source: *const ::core::ffi::c_char,
) {
    (*J).filename = filename;
    (*J).source = source;
    (*J).line = 1 as ::core::ffi::c_int;
    (*J).lasttoken = 0 as ::core::ffi::c_int;
    jsY_next(J);
}
#[no_mangle]
pub unsafe extern "C" fn jsY_lex(mut J: *mut js_State) -> ::core::ffi::c_int {
    (*J).lasttoken = jsY_lexx(J);
    return (*J).lasttoken;
}
unsafe extern "C" fn lexjsonnumber(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = (*J)
        .source
        .offset(-(1 as ::core::ffi::c_int as isize));
    if (*J).lexchar == '-' as i32 {
        jsY_next(J);
    }
    if (*J).lexchar == '0' as i32 {
        jsY_next(J);
    } else if (*J).lexchar >= '1' as i32 && (*J).lexchar <= '9' as i32 {
        while (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
            jsY_next(J);
        }
    } else {
        jsY_error(
            J,
            b"unexpected non-digit\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if if (*J).lexchar == '.' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } != 0
    {
        if (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
            while (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
                jsY_next(J);
            }
        } else {
            jsY_error(
                J,
                b"missing digits after decimal point\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    if (if (*J).lexchar == 'e' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) != 0
        || (if (*J).lexchar == 'E' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) != 0
    {
        if (*J).lexchar == '-' as i32 || (*J).lexchar == '+' as i32 {
            jsY_next(J);
        }
        if (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
            while (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32 {
                jsY_next(J);
            }
        } else {
            jsY_error(
                J,
                b"missing digits after exponent indicator\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    (*J).number = js_strtod(s, ::core::ptr::null_mut::<*mut ::core::ffi::c_char>());
    return TK_NUMBER as ::core::ffi::c_int;
}
unsafe extern "C" fn lexjsonescape(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    match (*J).lexchar {
        117 => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 12 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 8 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar) << 4 as ::core::ffi::c_int;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1 as ::core::ffi::c_int
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x as Rune);
        }
        34 => {
            textpush(J, '"' as i32);
            jsY_next(J);
        }
        92 => {
            textpush(J, '\\' as i32);
            jsY_next(J);
        }
        47 => {
            textpush(J, '/' as i32);
            jsY_next(J);
        }
        98 => {
            textpush(J, '\u{8}' as i32);
            jsY_next(J);
        }
        102 => {
            textpush(J, '\u{c}' as i32);
            jsY_next(J);
        }
        110 => {
            textpush(J, '\n' as i32);
            jsY_next(J);
        }
        114 => {
            textpush(J, '\r' as i32);
            jsY_next(J);
        }
        116 => {
            textpush(J, '\t' as i32);
            jsY_next(J);
        }
        _ => {
            jsY_error(
                J,
                b"invalid escape sequence\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn lexjsonstring(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    textinit(J);
    while (*J).lexchar != '"' as i32 {
        if (*J).lexchar == EOF {
            jsY_error(
                J,
                b"unterminated string\0" as *const u8 as *const ::core::ffi::c_char,
            );
        } else if (*J).lexchar < 32 as ::core::ffi::c_int {
            jsY_error(
                J,
                b"invalid control character in string\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        } else if if (*J).lexchar == '\\' as i32 {
            jsY_next(J);
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        } != 0
        {
            lexjsonescape(J);
        } else {
            textpush(J, (*J).lexchar as Rune);
            jsY_next(J);
        }
    }
    if if (*J).lexchar == '"' as i32 {
        jsY_next(J);
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } == 0
    {
        jsY_error(
            J,
            b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
            '"' as i32,
        );
    }
    s = textend(J);
    (*J).text = s;
    return TK_STRING as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsY_lexjson(mut J: *mut js_State) -> ::core::ffi::c_int {
    (*J).lexline = (*J).line;
    while jsY_iswhite((*J).lexchar) != 0 || (*J).lexchar == '\n' as i32 {
        jsY_next(J);
    }
    if (*J).lexchar >= '0' as i32 && (*J).lexchar <= '9' as i32
        || (*J).lexchar == '-' as i32
    {
        return lexjsonnumber(J);
    }
    match (*J).lexchar {
        44 => {
            jsY_next(J);
            return ',' as i32;
        }
        58 => {
            jsY_next(J);
            return ':' as i32;
        }
        91 => {
            jsY_next(J);
            return '[' as i32;
        }
        93 => {
            jsY_next(J);
            return ']' as i32;
        }
        123 => {
            jsY_next(J);
            return '{' as i32;
        }
        125 => {
            jsY_next(J);
            return '}' as i32;
        }
        34 => {
            jsY_next(J);
            return lexjsonstring(J);
        }
        102 => {
            jsY_next(J);
            if if (*J).lexchar == 'a' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'a' as i32,
                );
            }
            if if (*J).lexchar == 'l' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'l' as i32,
                );
            }
            if if (*J).lexchar == 's' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    's' as i32,
                );
            }
            if if (*J).lexchar == 'e' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'e' as i32,
                );
            }
            return TK_FALSE as ::core::ffi::c_int;
        }
        110 => {
            jsY_next(J);
            if if (*J).lexchar == 'u' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'u' as i32,
                );
            }
            if if (*J).lexchar == 'l' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'l' as i32,
                );
            }
            if if (*J).lexchar == 'l' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'l' as i32,
                );
            }
            return TK_NULL as ::core::ffi::c_int;
        }
        116 => {
            jsY_next(J);
            if if (*J).lexchar == 'r' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'r' as i32,
                );
            }
            if if (*J).lexchar == 'u' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'u' as i32,
                );
            }
            if if (*J).lexchar == 'e' as i32 {
                jsY_next(J);
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            } == 0
            {
                jsY_error(
                    J,
                    b"expected '%c'\0" as *const u8 as *const ::core::ffi::c_char,
                    'e' as i32,
                );
            }
            return TK_TRUE as ::core::ffi::c_int;
        }
        EOF => return 0 as ::core::ffi::c_int,
        _ => {}
    }
    if (*J).lexchar >= 0x20 as ::core::ffi::c_int
        && (*J).lexchar <= 0x7e as ::core::ffi::c_int
    {
        jsY_error(
            J,
            b"unexpected character: '%c'\0" as *const u8 as *const ::core::ffi::c_char,
            (*J).lexchar,
        );
    }
    jsY_error(
        J,
        b"unexpected character: \\u%04X\0" as *const u8 as *const ::core::ffi::c_char,
        (*J).lexchar,
    );
}
