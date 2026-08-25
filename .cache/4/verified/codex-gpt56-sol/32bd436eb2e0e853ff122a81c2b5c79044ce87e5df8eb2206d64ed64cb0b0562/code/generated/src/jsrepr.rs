extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn js_savetry(J: *mut js_State) -> *mut ::core::ffi::c_void;
    fn js_endtry(J: *mut js_State);
    fn js_throw(J: *mut js_State) -> !;
    fn js_hasproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn js_getproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    );
    fn js_getlength(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_hasindex(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_pushiterator(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        own: ::core::ffi::c_int,
    );
    fn js_nextiterator(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_isundefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isnull(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isboolean(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isnumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isstring(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isobject(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_toboolean(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tonumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_double;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_gettop(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
    fn js_replace(J: *mut js_State, idx: ::core::ffi::c_int);
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
    fn jsU_chartorune(
        rune: *mut Rune,
        str: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: ::core::ffi::c_int);
    fn js_puts(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const ::core::ffi::c_char);
    fn js_toobject(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Object;
    fn jsV_numbertostring(
        J: *mut js_State,
        buf: *mut ::core::ffi::c_char,
        number: ::core::ffi::c_double,
    ) -> *const ::core::ffi::c_char;
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
pub const JS_REGEXP_M: C2RustUnnamed_9 = 4;
pub const JS_REGEXP_I: C2RustUnnamed_9 = 2;
pub const JS_REGEXP_G: C2RustUnnamed_9 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Buffer {
    pub n: ::core::ffi::c_int,
    pub m: ::core::ffi::c_int,
    pub s: [::core::ffi::c_char; 64],
}
pub type Rune = ::core::ffi::c_int;
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
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
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
        let fresh3 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh3 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh1 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh1 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh2 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh2 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
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
        let fresh4 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh4 = __c as ::core::ffi::c_char;
        *fresh4 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh5 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh5 = __c as ::core::ffi::c_char;
        *fresh5 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh6 = (*stdout)._IO_write_ptr;
        (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
        *fresh6 = __c as ::core::ffi::c_char;
        *fresh6 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        NULL_0 as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn atol(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_long {
    return strtol(
        __nptr,
        NULL_0 as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn atoll(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_longlong {
    return strtoll(
        __nptr,
        NULL_0 as *mut *mut ::core::ffi::c_char,
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
    return NULL_0;
}
#[inline]
unsafe extern "C" fn atof(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    return strtod(__nptr, NULL_0 as *mut *mut ::core::ffi::c_char);
}
unsafe extern "C" fn reprnum(
    mut J: *mut js_State,
    mut sb: *mut *mut js_Buffer,
    mut n: ::core::ffi::c_double,
) {
    let mut buf: [::core::ffi::c_char; 40] = [0; 40];
    if n == 0 as ::core::ffi::c_int as ::core::ffi::c_double
        && n.is_sign_negative() as ::core::ffi::c_int != 0
    {
        js_puts(J, sb, b"-0\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        js_puts(
            J,
            sb,
            jsV_numbertostring(J, &raw mut buf as *mut ::core::ffi::c_char, n),
        );
    };
}
unsafe extern "C" fn reprstr(
    mut J: *mut js_State,
    mut sb: *mut *mut js_Buffer,
    mut s: *const ::core::ffi::c_char,
) {
    static mut HEX: *const ::core::ffi::c_char = b"0123456789ABCDEF\0" as *const u8
        as *const ::core::ffi::c_char;
    let mut i: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    let mut c: Rune = 0;
    js_putc(J, sb, '"' as i32);
    while *s != 0 {
        n = jsU_chartorune(&raw mut c, s);
        match c {
            34 => {
                js_puts(J, sb, b"\\\"\0" as *const u8 as *const ::core::ffi::c_char);
            }
            92 => {
                js_puts(J, sb, b"\\\\\0" as *const u8 as *const ::core::ffi::c_char);
            }
            8 => {
                js_puts(J, sb, b"\\b\0" as *const u8 as *const ::core::ffi::c_char);
            }
            12 => {
                js_puts(J, sb, b"\\f\0" as *const u8 as *const ::core::ffi::c_char);
            }
            10 => {
                js_puts(J, sb, b"\\n\0" as *const u8 as *const ::core::ffi::c_char);
            }
            13 => {
                js_puts(J, sb, b"\\r\0" as *const u8 as *const ::core::ffi::c_char);
            }
            9 => {
                js_puts(J, sb, b"\\t\0" as *const u8 as *const ::core::ffi::c_char);
            }
            _ => {
                if c < ' ' as i32 {
                    js_putc(J, sb, '\\' as i32);
                    js_putc(J, sb, 'x' as i32);
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int >> 4 as ::core::ffi::c_int
                                    & 15 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int,
                    );
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int & 15 as ::core::ffi::c_int)
                                    as isize,
                            ) as ::core::ffi::c_int,
                    );
                } else if c < 128 as ::core::ffi::c_int {
                    js_putc(J, sb, c as ::core::ffi::c_int);
                } else if c < 0x10000 as ::core::ffi::c_int {
                    js_putc(J, sb, '\\' as i32);
                    js_putc(J, sb, 'u' as i32);
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int >> 12 as ::core::ffi::c_int
                                    & 15 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int,
                    );
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                                    & 15 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int,
                    );
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int >> 4 as ::core::ffi::c_int
                                    & 15 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int,
                    );
                    js_putc(
                        J,
                        sb,
                        *HEX
                            .offset(
                                (c as ::core::ffi::c_int & 15 as ::core::ffi::c_int)
                                    as isize,
                            ) as ::core::ffi::c_int,
                    );
                } else {
                    i = 0 as ::core::ffi::c_int;
                    while i < n {
                        js_putc(J, sb, *s.offset(i as isize) as ::core::ffi::c_int);
                        i += 1;
                    }
                }
            }
        }
        s = s.offset(n as isize);
    }
    js_putc(J, sb, '"' as i32);
}
unsafe extern "C" fn reprident(
    mut J: *mut js_State,
    mut sb: *mut *mut js_Buffer,
    mut name: *const ::core::ffi::c_char,
) {
    let mut p: *const ::core::ffi::c_char = name;
    if *p as ::core::ffi::c_int >= '0' as i32 && *p as ::core::ffi::c_int <= '9' as i32 {
        while *p as ::core::ffi::c_int >= '0' as i32
            && *p as ::core::ffi::c_int <= '9' as i32
        {
            p = p.offset(1);
        }
    } else if *p as ::core::ffi::c_int >= 'a' as i32
        && *p as ::core::ffi::c_int <= 'z' as i32
        || *p as ::core::ffi::c_int >= 'A' as i32
            && *p as ::core::ffi::c_int <= 'Z' as i32
        || *p as ::core::ffi::c_int == '_' as i32
    {
        while *p as ::core::ffi::c_int >= '0' as i32
            && *p as ::core::ffi::c_int <= '9' as i32
            || (*p as ::core::ffi::c_int >= 'a' as i32
                && *p as ::core::ffi::c_int <= 'z' as i32
                || *p as ::core::ffi::c_int >= 'A' as i32
                    && *p as ::core::ffi::c_int <= 'Z' as i32)
            || *p as ::core::ffi::c_int == '_' as i32
        {
            p = p.offset(1);
        }
    }
    if p > name && *p as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        js_puts(J, sb, name);
    } else {
        reprstr(J, sb, name);
    };
}
unsafe extern "C" fn reprobject(mut J: *mut js_State, mut sb: *mut *mut js_Buffer) {
    let mut key: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut i: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    n = js_gettop(J) - 1 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -(1 as ::core::ffi::c_int)) {
                js_puts(J, sb, b"{}\0" as *const u8 as *const ::core::ffi::c_char);
                return;
            }
        }
        i += 1;
    }
    n = 0 as ::core::ffi::c_int;
    js_putc(J, sb, '{' as i32);
    js_pushiterator(J, -(1 as ::core::ffi::c_int), 1 as ::core::ffi::c_int);
    loop {
        key = js_nextiterator(J, -(1 as ::core::ffi::c_int));
        if key.is_null() {
            break;
        }
        let fresh0 = n;
        n = n + 1;
        if fresh0 > 0 as ::core::ffi::c_int {
            js_puts(J, sb, b", \0" as *const u8 as *const ::core::ffi::c_char);
        }
        reprident(J, sb, key);
        js_puts(J, sb, b": \0" as *const u8 as *const ::core::ffi::c_char);
        js_getproperty(J, -(2 as ::core::ffi::c_int), key);
        reprvalue(J, sb);
        js_pop(J, 1 as ::core::ffi::c_int);
    }
    js_pop(J, 1 as ::core::ffi::c_int);
    js_putc(J, sb, '}' as i32);
}
unsafe extern "C" fn reprarray(mut J: *mut js_State, mut sb: *mut *mut js_Buffer) {
    let mut n: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    n = js_gettop(J) - 1 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -(1 as ::core::ffi::c_int)) {
                js_puts(J, sb, b"[]\0" as *const u8 as *const ::core::ffi::c_char);
                return;
            }
        }
        i += 1;
    }
    js_putc(J, sb, '[' as i32);
    n = js_getlength(J, -(1 as ::core::ffi::c_int));
    i = 0 as ::core::ffi::c_int;
    while i < n {
        if i > 0 as ::core::ffi::c_int {
            js_puts(J, sb, b", \0" as *const u8 as *const ::core::ffi::c_char);
        }
        if js_hasindex(J, -(1 as ::core::ffi::c_int), i) != 0 {
            reprvalue(J, sb);
            js_pop(J, 1 as ::core::ffi::c_int);
        }
        i += 1;
    }
    js_putc(J, sb, ']' as i32);
}
unsafe extern "C" fn reprfun(
    mut J: *mut js_State,
    mut sb: *mut *mut js_Buffer,
    mut fun: *mut js_Function,
) {
    let mut i: ::core::ffi::c_int = 0;
    js_puts(J, sb, b"function \0" as *const u8 as *const ::core::ffi::c_char);
    js_puts(J, sb, (*fun).name);
    js_putc(J, sb, '(' as i32);
    i = 0 as ::core::ffi::c_int;
    while i < (*fun).numparams {
        if i > 0 as ::core::ffi::c_int {
            js_puts(J, sb, b", \0" as *const u8 as *const ::core::ffi::c_char);
        }
        js_puts(J, sb, *(*fun).vartab.offset(i as isize));
        i += 1;
    }
    js_puts(J, sb, b") { [byte code] }\0" as *const u8 as *const ::core::ffi::c_char);
}
unsafe extern "C" fn reprvalue(mut J: *mut js_State, mut sb: *mut *mut js_Buffer) {
    if js_isundefined(J, -(1 as ::core::ffi::c_int)) != 0 {
        js_puts(J, sb, b"undefined\0" as *const u8 as *const ::core::ffi::c_char);
    } else if js_isnull(J, -(1 as ::core::ffi::c_int)) != 0 {
        js_puts(J, sb, b"null\0" as *const u8 as *const ::core::ffi::c_char);
    } else if js_isboolean(J, -(1 as ::core::ffi::c_int)) != 0 {
        js_puts(
            J,
            sb,
            if js_toboolean(J, -(1 as ::core::ffi::c_int)) != 0 {
                b"true\0" as *const u8 as *const ::core::ffi::c_char
            } else {
                b"false\0" as *const u8 as *const ::core::ffi::c_char
            },
        );
    } else if js_isnumber(J, -(1 as ::core::ffi::c_int)) != 0 {
        reprnum(J, sb, js_tonumber(J, -(1 as ::core::ffi::c_int)));
    } else if js_isstring(J, -(1 as ::core::ffi::c_int)) != 0 {
        reprstr(J, sb, js_tostring(J, -(1 as ::core::ffi::c_int)));
    } else if js_isobject(J, -(1 as ::core::ffi::c_int)) != 0 {
        let mut obj: *mut js_Object = js_toobject(J, -(1 as ::core::ffi::c_int));
        match (*obj).type_0 as ::core::ffi::c_uint {
            1 => {
                reprarray(J, sb);
            }
            2 | 3 => {
                reprfun(J, sb, (*obj).u.f.function);
            }
            4 => {
                js_puts(
                    J,
                    sb,
                    b"function \0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_puts(J, sb, (*obj).u.c.name);
                js_puts(
                    J,
                    sb,
                    b"() { [native code] }\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            6 => {
                js_puts(
                    J,
                    sb,
                    b"(new Boolean(\0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_puts(
                    J,
                    sb,
                    if (*obj).u.boolean != 0 {
                        b"true\0" as *const u8 as *const ::core::ffi::c_char
                    } else {
                        b"false\0" as *const u8 as *const ::core::ffi::c_char
                    },
                );
                js_puts(J, sb, b"))\0" as *const u8 as *const ::core::ffi::c_char);
            }
            7 => {
                js_puts(
                    J,
                    sb,
                    b"(new Number(\0" as *const u8 as *const ::core::ffi::c_char,
                );
                reprnum(J, sb, (*obj).u.number);
                js_puts(J, sb, b"))\0" as *const u8 as *const ::core::ffi::c_char);
            }
            8 => {
                js_puts(
                    J,
                    sb,
                    b"(new String(\0" as *const u8 as *const ::core::ffi::c_char,
                );
                reprstr(J, sb, (*obj).u.s.string);
                js_puts(J, sb, b"))\0" as *const u8 as *const ::core::ffi::c_char);
            }
            9 => {
                js_putc(J, sb, '/' as i32);
                js_puts(J, sb, (*obj).u.r.source);
                js_putc(J, sb, '/' as i32);
                if (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_G as ::core::ffi::c_int != 0
                {
                    js_putc(J, sb, 'g' as i32);
                }
                if (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_I as ::core::ffi::c_int != 0
                {
                    js_putc(J, sb, 'i' as i32);
                }
                if (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_M as ::core::ffi::c_int != 0
                {
                    js_putc(J, sb, 'm' as i32);
                }
            }
            10 => {
                let mut buf: [::core::ffi::c_char; 40] = [0; 40];
                js_puts(
                    J,
                    sb,
                    b"(new Date(\0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_puts(
                    J,
                    sb,
                    jsV_numbertostring(
                        J,
                        &raw mut buf as *mut ::core::ffi::c_char,
                        (*obj).u.number,
                    ),
                );
                js_puts(J, sb, b"))\0" as *const u8 as *const ::core::ffi::c_char);
            }
            5 => {
                js_puts(J, sb, b"(new \0" as *const u8 as *const ::core::ffi::c_char);
                js_getproperty(
                    J,
                    -(1 as ::core::ffi::c_int),
                    b"name\0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_puts(J, sb, js_tostring(J, -(1 as ::core::ffi::c_int)));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_putc(J, sb, '(' as i32);
                if js_hasproperty(
                    J,
                    -(1 as ::core::ffi::c_int),
                    b"message\0" as *const u8 as *const ::core::ffi::c_char,
                ) != 0
                {
                    reprvalue(J, sb);
                    js_pop(J, 1 as ::core::ffi::c_int);
                }
                js_puts(J, sb, b"))\0" as *const u8 as *const ::core::ffi::c_char);
            }
            11 => {
                js_puts(J, sb, b"Math\0" as *const u8 as *const ::core::ffi::c_char);
            }
            12 => {
                js_puts(J, sb, b"JSON\0" as *const u8 as *const ::core::ffi::c_char);
            }
            14 => {
                js_puts(
                    J,
                    sb,
                    b"[iterator \0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            15 => {
                js_puts(
                    J,
                    sb,
                    b"[userdata \0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_puts(J, sb, (*obj).u.user.tag);
                js_putc(J, sb, ']' as i32);
            }
            _ => {
                reprobject(J, sb);
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn js_repr(mut J: *mut js_State, mut idx: ::core::ffi::c_int) {
    let mut sb: *mut js_Buffer = ::core::ptr::null_mut::<js_Buffer>();
    let mut savebot: ::core::ffi::c_int = 0;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, sb as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    js_copy(J, idx);
    savebot = (*J).bot;
    (*J).bot = (*J).top - 1 as ::core::ffi::c_int;
    reprvalue(J, &raw mut sb);
    (*J).bot = savebot;
    js_pop(J, 1 as ::core::ffi::c_int);
    js_putc(J, &raw mut sb, 0 as ::core::ffi::c_int);
    js_pushstring(
        J,
        if !sb.is_null() {
            &raw mut (*sb).s as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
        } else {
            b"undefined\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn js_torepr(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    js_repr(J, idx);
    js_replace(
        J,
        if idx < 0 as ::core::ffi::c_int { idx - 1 as ::core::ffi::c_int } else { idx },
    );
    return js_tostring(J, idx);
}
#[no_mangle]
pub unsafe extern "C" fn js_tryrepr(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut error: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_pop(J, 1 as ::core::ffi::c_int);
        return error;
    }
    s = js_torepr(J, idx);
    js_endtry(J);
    return s;
}
