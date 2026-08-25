extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn js_savetry(J: *mut js_State) -> *mut ::core::ffi::c_void;
    fn js_endtry(J: *mut js_State);
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_throw(J: *mut js_State) -> !;
    fn js_call(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_getproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    );
    fn js_defproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_pushnumber(J: *mut js_State, v: ::core::ffi::c_double);
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_isstring(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isprimitive(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isobject(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscallable(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tonumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_double;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
    fn js_rot2(J: *mut js_State);
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
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn ceil(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn floor(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn fmod(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_strdup(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn js_fmtexp(p: *mut ::core::ffi::c_char, e: ::core::ffi::c_int);
    fn js_grisu2(
        v: ::core::ffi::c_double,
        buffer: *mut ::core::ffi::c_char,
        K: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_strtod(
        as_0: *const ::core::ffi::c_char,
        aas: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn js_utflen(s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn jsV_newmemstring(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
        n: ::core::ffi::c_int,
    ) -> *mut js_String;
    fn js_tovalue(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Value;
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
    fn jsY_iswhite(c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn jsY_isnewline(c: ::core::ffi::c_int) -> ::core::ffi::c_int;
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
pub const JS_HNONE: C2RustUnnamed_10 = 0;
pub const JS_HNUMBER: C2RustUnnamed_10 = 1;
pub const JS_TMEMSTR: js_Type = 6;
pub const JS_TLITSTR: js_Type = 5;
pub const JS_TSHRSTR: js_Type = 0;
pub const JS_HSTRING: C2RustUnnamed_10 = 2;
pub const JS_TOBJECT: js_Type = 7;
pub const JS_TNUMBER: js_Type = 4;
pub const JS_TBOOLEAN: js_Type = 3;
pub const JS_TNULL: js_Type = 2;
pub const JS_TUNDEFINED: js_Type = 1;
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
pub type js_Type = ::core::ffi::c_uint;
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
        let fresh6 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh6 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh4 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh4 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh5 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh5 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
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
        let fresh7 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh7 = __c as ::core::ffi::c_char;
        *fresh7 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh8 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh8 = __c as ::core::ffi::c_char;
        *fresh8 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh9 = (*stdout)._IO_write_ptr;
        (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
        *fresh9 = __c as ::core::ffi::c_char;
        *fresh9 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_strtol(
    mut s: *const ::core::ffi::c_char,
    mut p: *mut *mut ::core::ffi::c_char,
    mut base: ::core::ffi::c_int,
) -> ::core::ffi::c_double {
    static mut table: [::core::ffi::c_uchar; 256] = [
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        1 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        2 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        3 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        4 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        5 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        6 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        7 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        8 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        9 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        10 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        11 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        12 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        13 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        14 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        15 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        16 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        17 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        18 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        19 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        20 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        21 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        22 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        23 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        24 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        25 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        26 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        27 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        28 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        29 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        30 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        31 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        32 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        33 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        34 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        35 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        10 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        11 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        12 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        13 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        14 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        15 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        16 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        17 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        18 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        19 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        20 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        21 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        22 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        23 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        24 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        25 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        26 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        27 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        28 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        29 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        30 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        31 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        32 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        33 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        34 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        35 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        80 as ::core::ffi::c_int as ::core::ffi::c_uchar,
    ];
    let mut x: ::core::ffi::c_double = 0.;
    let mut c: ::core::ffi::c_uchar = 0;
    if base == 10 as ::core::ffi::c_int {
        x = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        let fresh0 = s;
        s = s.offset(1);
        c = *fresh0 as ::core::ffi::c_uchar;
        while 0 as ::core::ffi::c_int <= c as ::core::ffi::c_int - '0' as i32
            && (c as ::core::ffi::c_int - '0' as i32) < 10 as ::core::ffi::c_int
        {
            x = x * 10 as ::core::ffi::c_int as ::core::ffi::c_double
                + (c as ::core::ffi::c_int - '0' as i32) as ::core::ffi::c_double;
            let fresh1 = s;
            s = s.offset(1);
            c = *fresh1 as ::core::ffi::c_uchar;
        }
    } else {
        x = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        let fresh2 = s;
        s = s.offset(1);
        c = *fresh2 as ::core::ffi::c_uchar;
        while (table[c as usize] as ::core::ffi::c_int) < base {
            x = x * base as ::core::ffi::c_double
                + table[c as usize] as ::core::ffi::c_int as ::core::ffi::c_double;
            let fresh3 = s;
            s = s.offset(1);
            c = *fresh3 as ::core::ffi::c_uchar;
        }
    }
    if !p.is_null() {
        *p = (s as *mut ::core::ffi::c_char).offset(-(1 as ::core::ffi::c_int as isize));
    }
    return x;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertointeger(
    mut n: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    if n == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        return 0 as ::core::ffi::c_int;
    }
    if n.is_nan() as i32 != 0 {
        return 0 as ::core::ffi::c_int;
    }
    n = if n < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        -floor(-n)
    } else {
        floor(n)
    };
    if n < INT_MIN as ::core::ffi::c_double {
        return INT_MIN;
    }
    if n > INT_MAX as ::core::ffi::c_double {
        return INT_MAX;
    }
    return n as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertoint32(
    mut n: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut two32: ::core::ffi::c_double = 4294967296.0f64;
    let mut two31: ::core::ffi::c_double = 2147483648.0f64;
    if n.is_finite() as i32 == 0 || n == 0 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        return 0 as ::core::ffi::c_int;
    }
    n = fmod(n, two32);
    n = if n >= 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        floor(n)
    } else {
        ceil(n) + two32
    };
    if n >= two31 {
        return (n - two32) as ::core::ffi::c_int
    } else {
        return n as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertouint32(
    mut n: ::core::ffi::c_double,
) -> ::core::ffi::c_uint {
    return jsV_numbertoint32(n) as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertoint16(
    mut n: ::core::ffi::c_double,
) -> ::core::ffi::c_short {
    return jsV_numbertoint32(n) as ::core::ffi::c_short;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertouint16(
    mut n: ::core::ffi::c_double,
) -> ::core::ffi::c_ushort {
    return jsV_numbertoint32(n) as ::core::ffi::c_ushort;
}
unsafe extern "C" fn jsV_toString(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
) -> ::core::ffi::c_int {
    js_pushobject(J, obj);
    js_getproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"toString\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_iscallable(J, -(1 as ::core::ffi::c_int)) != 0 {
        js_rot2(J);
        js_call(J, 0 as ::core::ffi::c_int);
        if js_isprimitive(J, -(1 as ::core::ffi::c_int)) != 0 {
            return 1 as ::core::ffi::c_int;
        }
        js_pop(J, 1 as ::core::ffi::c_int);
        return 0 as ::core::ffi::c_int;
    }
    js_pop(J, 2 as ::core::ffi::c_int);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsV_valueOf(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
) -> ::core::ffi::c_int {
    js_pushobject(J, obj);
    js_getproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"valueOf\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_iscallable(J, -(1 as ::core::ffi::c_int)) != 0 {
        js_rot2(J);
        js_call(J, 0 as ::core::ffi::c_int);
        if js_isprimitive(J, -(1 as ::core::ffi::c_int)) != 0 {
            return 1 as ::core::ffi::c_int;
        }
        js_pop(J, 1 as ::core::ffi::c_int);
        return 0 as ::core::ffi::c_int;
    }
    js_pop(J, 2 as ::core::ffi::c_int);
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toprimitive(
    mut J: *mut js_State,
    mut v: *mut js_Value,
    mut preferred: ::core::ffi::c_int,
) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if (*v).t.type_0 as ::core::ffi::c_int != JS_TOBJECT as ::core::ffi::c_int {
        return;
    }
    obj = (*v).u.object;
    if preferred == JS_HNONE as ::core::ffi::c_int {
        preferred = if (*obj).type_0 as ::core::ffi::c_uint
            == JS_CDATE as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            JS_HSTRING as ::core::ffi::c_int
        } else {
            JS_HNUMBER as ::core::ffi::c_int
        };
    }
    if preferred == JS_HSTRING as ::core::ffi::c_int {
        if jsV_toString(J, obj) != 0 || jsV_valueOf(J, obj) != 0 {
            *v = *js_tovalue(J, -(1 as ::core::ffi::c_int));
            js_pop(J, 1 as ::core::ffi::c_int);
            return;
        }
    } else if jsV_valueOf(J, obj) != 0 || jsV_toString(J, obj) != 0 {
        *v = *js_tovalue(J, -(1 as ::core::ffi::c_int));
        js_pop(J, 1 as ::core::ffi::c_int);
        return;
    }
    if (*J).strict != 0 {
        js_typeerror(
            J,
            b"cannot convert object to primitive\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    (*v).t.type_0 = JS_TLITSTR as ::core::ffi::c_int as ::core::ffi::c_char;
    (*v).u.litstr = b"[object]\0" as *const u8 as *const ::core::ffi::c_char;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toboolean(
    mut J: *mut js_State,
    mut v: *mut js_Value,
) -> ::core::ffi::c_int {
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => return 0 as ::core::ffi::c_int,
        2 => return 0 as ::core::ffi::c_int,
        3 => return (*v).u.boolean,
        4 => {
            return ((*v).u.number != 0 as ::core::ffi::c_int as ::core::ffi::c_double
                && (*v).u.number.is_nan() as i32 == 0) as ::core::ffi::c_int;
        }
        5 => {
            return (*(*v).u.litstr.offset(0 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
        }
        6 => {
            return (*(&raw mut (*(*v).u.memstr).p as *mut ::core::ffi::c_char)
                .offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
        }
        7 => return 1 as ::core::ffi::c_int,
        0 | _ => {
            return ((*v).u.shrstr[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_itoa(
    mut out: *mut ::core::ffi::c_char,
    mut v: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    let mut s: *mut ::core::ffi::c_char = out;
    let mut a: ::core::ffi::c_uint = 0;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if v < 0 as ::core::ffi::c_int {
        a = (v as ::core::ffi::c_uint).wrapping_neg();
        let fresh33 = s;
        s = s.offset(1);
        *fresh33 = '-' as i32 as ::core::ffi::c_char;
    } else {
        a = v as ::core::ffi::c_uint;
    }
    while a != 0 {
        let fresh34 = i;
        i = i + 1;
        buf[fresh34 as usize] = a
            .wrapping_rem(10 as ::core::ffi::c_uint)
            .wrapping_add('0' as i32 as ::core::ffi::c_uint) as ::core::ffi::c_char;
        a = a.wrapping_div(10 as ::core::ffi::c_uint);
    }
    if i == 0 as ::core::ffi::c_int {
        let fresh35 = i;
        i = i + 1;
        buf[fresh35 as usize] = '0' as i32 as ::core::ffi::c_char;
    }
    while i > 0 as ::core::ffi::c_int {
        i -= 1;
        let fresh36 = s;
        s = s.offset(1);
        *fresh36 = buf[i as usize];
    }
    *s = 0 as ::core::ffi::c_char;
    return out;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_stringtofloat(
    mut s: *const ::core::ffi::c_char,
    mut ep: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut n: ::core::ffi::c_double = 0.;
    let mut e: *const ::core::ffi::c_char = s;
    let mut isflt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if *e as ::core::ffi::c_int == '+' as i32 || *e as ::core::ffi::c_int == '-' as i32 {
        e = e.offset(1);
    }
    while *e as ::core::ffi::c_int >= '0' as i32
        && *e as ::core::ffi::c_int <= '9' as i32
    {
        e = e.offset(1);
    }
    if *e as ::core::ffi::c_int == '.' as i32 {
        e = e.offset(1);
        isflt = 1 as ::core::ffi::c_int;
    }
    while *e as ::core::ffi::c_int >= '0' as i32
        && *e as ::core::ffi::c_int <= '9' as i32
    {
        e = e.offset(1);
    }
    if *e as ::core::ffi::c_int == 'e' as i32 || *e as ::core::ffi::c_int == 'E' as i32 {
        e = e.offset(1);
        if *e as ::core::ffi::c_int == '+' as i32
            || *e as ::core::ffi::c_int == '-' as i32
        {
            e = e.offset(1);
        }
        while *e as ::core::ffi::c_int >= '0' as i32
            && *e as ::core::ffi::c_int <= '9' as i32
        {
            e = e.offset(1);
        }
        isflt = 1 as ::core::ffi::c_int;
    }
    if isflt != 0 {
        n = js_strtod(s, &raw mut end);
    } else if *s as ::core::ffi::c_int == '-' as i32 {
        n = -js_strtol(
            s.offset(1 as ::core::ffi::c_int as isize),
            &raw mut end,
            10 as ::core::ffi::c_int,
        );
    } else if *s as ::core::ffi::c_int == '+' as i32 {
        n = js_strtol(
            s.offset(1 as ::core::ffi::c_int as isize),
            &raw mut end,
            10 as ::core::ffi::c_int,
        );
    } else {
        n = js_strtol(s, &raw mut end, 10 as ::core::ffi::c_int);
    }
    if end == e as *mut ::core::ffi::c_char {
        *ep = e as *mut ::core::ffi::c_char;
        return n;
    }
    *ep = s as *mut ::core::ffi::c_char;
    return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_stringtonumber(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    let mut e: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut n: ::core::ffi::c_double = 0.;
    while jsY_iswhite(*s as ::core::ffi::c_int) != 0
        || jsY_isnewline(*s as ::core::ffi::c_int) != 0
    {
        s = s.offset(1);
    }
    if *s.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '0' as i32
        && (*s.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 'x' as i32
            || *s.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                == 'X' as i32)
        && *s.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            != 0 as ::core::ffi::c_int
    {
        n = js_strtol(
            s.offset(2 as ::core::ffi::c_int as isize),
            &raw mut e,
            16 as ::core::ffi::c_int,
        );
    } else if strncmp(
        s,
        b"Infinity\0" as *const u8 as *const ::core::ffi::c_char,
        8 as size_t,
    ) == 0
    {
        n = ::core::f32::INFINITY as ::core::ffi::c_double;
        e = (s as *mut ::core::ffi::c_char).offset(8 as ::core::ffi::c_int as isize);
    } else if strncmp(
        s,
        b"+Infinity\0" as *const u8 as *const ::core::ffi::c_char,
        9 as size_t,
    ) == 0
    {
        n = ::core::f32::INFINITY as ::core::ffi::c_double;
        e = (s as *mut ::core::ffi::c_char).offset(9 as ::core::ffi::c_int as isize);
    } else if strncmp(
        s,
        b"-Infinity\0" as *const u8 as *const ::core::ffi::c_char,
        9 as size_t,
    ) == 0
    {
        n = -::core::f32::INFINITY as ::core::ffi::c_double;
        e = (s as *mut ::core::ffi::c_char).offset(9 as ::core::ffi::c_int as isize);
    } else {
        n = js_stringtofloat(s, &raw mut e);
    }
    while jsY_iswhite(*e as ::core::ffi::c_int) != 0
        || jsY_isnewline(*e as ::core::ffi::c_int) != 0
    {
        e = e.offset(1);
    }
    if *e != 0 {
        return ::core::f32::NAN as ::core::ffi::c_double;
    }
    return n;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tonumber(
    mut J: *mut js_State,
    mut v: *mut js_Value,
) -> ::core::ffi::c_double {
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => return ::core::f32::NAN as ::core::ffi::c_double,
        2 => return 0 as ::core::ffi::c_int as ::core::ffi::c_double,
        3 => return (*v).u.boolean as ::core::ffi::c_double,
        4 => return (*v).u.number,
        5 => return jsV_stringtonumber(J, (*v).u.litstr),
        6 => {
            return jsV_stringtonumber(
                J,
                &raw mut (*(*v).u.memstr).p as *mut ::core::ffi::c_char,
            );
        }
        7 => {
            jsV_toprimitive(J, v, JS_HNUMBER as ::core::ffi::c_int);
            return jsV_tonumber(J, v);
        }
        0 | _ => {
            return jsV_stringtonumber(
                J,
                &raw mut (*v).u.shrstr as *mut ::core::ffi::c_char,
            );
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tointeger(
    mut J: *mut js_State,
    mut v: *mut js_Value,
) -> ::core::ffi::c_double {
    return jsV_numbertointeger(jsV_tonumber(J, v)) as ::core::ffi::c_double;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertostring(
    mut J: *mut js_State,
    mut buf: *mut ::core::ffi::c_char,
    mut f: ::core::ffi::c_double,
) -> *const ::core::ffi::c_char {
    let mut digits: [::core::ffi::c_char; 32] = [0; 32];
    let mut p: *mut ::core::ffi::c_char = buf as *mut ::core::ffi::c_char;
    let mut s: *mut ::core::ffi::c_char = &raw mut digits as *mut ::core::ffi::c_char;
    let mut exp: ::core::ffi::c_int = 0;
    let mut ndigits: ::core::ffi::c_int = 0;
    let mut point: ::core::ffi::c_int = 0;
    if f == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        return b"0\0" as *const u8 as *const ::core::ffi::c_char;
    }
    if f.is_nan() as i32 != 0 {
        return b"NaN\0" as *const u8 as *const ::core::ffi::c_char;
    }
    if if f.is_infinite() { if f.is_sign_positive() { 1 } else { -1 } } else { 0 } != 0 {
        return if f < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
            b"-Infinity\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"Infinity\0" as *const u8 as *const ::core::ffi::c_char
        };
    }
    if f >= INT_MIN as ::core::ffi::c_double && f <= INT_MAX as ::core::ffi::c_double {
        let mut i: ::core::ffi::c_int = f as ::core::ffi::c_int;
        if i as ::core::ffi::c_double == f {
            return js_itoa(buf as *mut ::core::ffi::c_char, i);
        }
    }
    ndigits = js_grisu2(f, &raw mut digits as *mut ::core::ffi::c_char, &raw mut exp);
    point = ndigits + exp;
    if f.is_sign_negative() as ::core::ffi::c_int != 0 {
        let fresh13 = p;
        p = p.offset(1);
        *fresh13 = '-' as i32 as ::core::ffi::c_char;
    }
    if point < -(5 as ::core::ffi::c_int) || point > 21 as ::core::ffi::c_int {
        let fresh14 = s;
        s = s.offset(1);
        let fresh15 = p;
        p = p.offset(1);
        *fresh15 = *fresh14;
        if ndigits > 1 as ::core::ffi::c_int {
            let mut n: ::core::ffi::c_int = ndigits - 1 as ::core::ffi::c_int;
            let fresh16 = p;
            p = p.offset(1);
            *fresh16 = '.' as i32 as ::core::ffi::c_char;
            loop {
                let fresh17 = n;
                n = n - 1;
                if !(fresh17 != 0) {
                    break;
                }
                let fresh18 = s;
                s = s.offset(1);
                let fresh19 = p;
                p = p.offset(1);
                *fresh19 = *fresh18;
            }
        }
        js_fmtexp(p, point - 1 as ::core::ffi::c_int);
    } else if point <= 0 as ::core::ffi::c_int {
        let fresh20 = p;
        p = p.offset(1);
        *fresh20 = '0' as i32 as ::core::ffi::c_char;
        let fresh21 = p;
        p = p.offset(1);
        *fresh21 = '.' as i32 as ::core::ffi::c_char;
        loop {
            let fresh22 = point;
            point = point + 1;
            if !(fresh22 < 0 as ::core::ffi::c_int) {
                break;
            }
            let fresh23 = p;
            p = p.offset(1);
            *fresh23 = '0' as i32 as ::core::ffi::c_char;
        }
        loop {
            let fresh24 = ndigits;
            ndigits = ndigits - 1;
            if !(fresh24 > 0 as ::core::ffi::c_int) {
                break;
            }
            let fresh25 = s;
            s = s.offset(1);
            let fresh26 = p;
            p = p.offset(1);
            *fresh26 = *fresh25;
        }
        *p = 0 as ::core::ffi::c_char;
    } else {
        loop {
            let fresh27 = ndigits;
            ndigits = ndigits - 1;
            if !(fresh27 > 0 as ::core::ffi::c_int) {
                break;
            }
            let fresh28 = s;
            s = s.offset(1);
            let fresh29 = p;
            p = p.offset(1);
            *fresh29 = *fresh28;
            point -= 1;
            if point == 0 as ::core::ffi::c_int && ndigits > 0 as ::core::ffi::c_int {
                let fresh30 = p;
                p = p.offset(1);
                *fresh30 = '.' as i32 as ::core::ffi::c_char;
            }
        }
        loop {
            let fresh31 = point;
            point = point - 1;
            if !(fresh31 > 0 as ::core::ffi::c_int) {
                break;
            }
            let fresh32 = p;
            p = p.offset(1);
            *fresh32 = '0' as i32 as ::core::ffi::c_char;
        }
        *p = 0 as ::core::ffi::c_char;
    }
    return buf as *const ::core::ffi::c_char;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tostring(
    mut J: *mut js_State,
    mut v: *mut js_Value,
) -> *const ::core::ffi::c_char {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    let mut p: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => return b"undefined\0" as *const u8 as *const ::core::ffi::c_char,
        2 => return b"null\0" as *const u8 as *const ::core::ffi::c_char,
        3 => {
            return if (*v).u.boolean != 0 {
                b"true\0" as *const u8 as *const ::core::ffi::c_char
            } else {
                b"false\0" as *const u8 as *const ::core::ffi::c_char
            };
        }
        5 => return (*v).u.litstr,
        6 => return &raw mut (*(*v).u.memstr).p as *mut ::core::ffi::c_char,
        4 => {
            p = jsV_numbertostring(
                J,
                &raw mut buf as *mut ::core::ffi::c_char,
                (*v).u.number,
            );
            if p
                == &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
            {
                let mut n: ::core::ffi::c_int = strlen(p) as ::core::ffi::c_int;
                if n <= 15 as ::core::ffi::c_ulong as ::core::ffi::c_int {
                    let mut s: *mut ::core::ffi::c_char = &raw mut (*v).u.shrstr
                        as *mut ::core::ffi::c_char;
                    loop {
                        let fresh10 = n;
                        n = n - 1;
                        if !(fresh10 != 0) {
                            break;
                        }
                        let fresh11 = p;
                        p = p.offset(1);
                        let fresh12 = s;
                        s = s.offset(1);
                        *fresh12 = *fresh11;
                    }
                    *s = 0 as ::core::ffi::c_char;
                    (*v).t.type_0 = JS_TSHRSTR as ::core::ffi::c_int
                        as ::core::ffi::c_char;
                    return &raw mut (*v).u.shrstr as *mut ::core::ffi::c_char;
                } else {
                    (*v).u.memstr = jsV_newmemstring(J, p, n);
                    (*v).t.type_0 = JS_TMEMSTR as ::core::ffi::c_int
                        as ::core::ffi::c_char;
                    return &raw mut (*(*v).u.memstr).p as *mut ::core::ffi::c_char;
                }
            }
            return p;
        }
        7 => {
            jsV_toprimitive(J, v, JS_HSTRING as ::core::ffi::c_int);
            return jsV_tostring(J, v);
        }
        0 | _ => return &raw mut (*v).u.shrstr as *mut ::core::ffi::c_char,
    };
}
unsafe extern "C" fn jsV_newboolean(
    mut J: *mut js_State,
    mut v: ::core::ffi::c_int,
) -> *mut js_Object {
    let mut obj: *mut js_Object = jsV_newobject(J, JS_CBOOLEAN, (*J).Boolean_prototype);
    (*obj).u.boolean = v;
    return obj;
}
unsafe extern "C" fn jsV_newnumber(
    mut J: *mut js_State,
    mut v: ::core::ffi::c_double,
) -> *mut js_Object {
    let mut obj: *mut js_Object = jsV_newobject(J, JS_CNUMBER, (*J).Number_prototype);
    (*obj).u.number = v;
    return obj;
}
unsafe extern "C" fn jsV_newstring(
    mut J: *mut js_State,
    mut v: *const ::core::ffi::c_char,
) -> *mut js_Object {
    let mut obj: *mut js_Object = jsV_newobject(J, JS_CSTRING, (*J).String_prototype);
    let mut n: size_t = strlen(v);
    if n < ::core::mem::size_of::<[::core::ffi::c_char; 16]>() as usize {
        (*obj).u.s.string = &raw mut (*obj).u.s.shrstr as *mut ::core::ffi::c_char;
        memcpy(
            &raw mut (*obj).u.s.shrstr as *mut ::core::ffi::c_char
                as *mut ::core::ffi::c_void,
            v as *const ::core::ffi::c_void,
            n.wrapping_add(1 as size_t),
        );
    } else {
        (*obj).u.s.string = js_strdup(J, v);
    }
    (*obj).u.s.length = js_utflen(v);
    return obj;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toobject(
    mut J: *mut js_State,
    mut v: *mut js_Value,
) -> *mut js_Object {
    let mut o: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    match (*v).t.type_0 as ::core::ffi::c_int {
        2 => {
            js_typeerror(
                J,
                b"cannot convert null to object\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        7 => return (*v).u.object,
        0 => {
            o = jsV_newstring(J, &raw mut (*v).u.shrstr as *mut ::core::ffi::c_char);
        }
        5 => {
            o = jsV_newstring(J, (*v).u.litstr);
        }
        6 => {
            o = jsV_newstring(
                J,
                &raw mut (*(*v).u.memstr).p as *mut ::core::ffi::c_char,
            );
        }
        3 => {
            o = jsV_newboolean(J, (*v).u.boolean);
        }
        4 => {
            o = jsV_newnumber(J, (*v).u.number);
        }
        1 | _ => {
            js_typeerror(
                J,
                b"cannot convert undefined to object\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    (*v).t.type_0 = JS_TOBJECT as ::core::ffi::c_int as ::core::ffi::c_char;
    (*v).u.object = o;
    return o;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newobjectx(mut J: *mut js_State) {
    let mut prototype: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, -(1 as ::core::ffi::c_int)) != 0 {
        prototype = js_toobject(J, -(1 as ::core::ffi::c_int));
    }
    js_pop(J, 1 as ::core::ffi::c_int);
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, prototype));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newobject(mut J: *mut js_State) {
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, (*J).Object_prototype));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newarguments(mut J: *mut js_State) {
    js_pushobject(J, jsV_newobject(J, JS_CARGUMENTS, (*J).Object_prototype));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newarray(mut J: *mut js_State) {
    let mut obj: *mut js_Object = jsV_newobject(J, JS_CARRAY, (*J).Array_prototype);
    (*obj).u.a.simple = 1 as ::core::ffi::c_int;
    js_pushobject(J, obj);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newboolean(mut J: *mut js_State, mut v: ::core::ffi::c_int) {
    js_pushobject(J, jsV_newboolean(J, v));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newnumber(
    mut J: *mut js_State,
    mut v: ::core::ffi::c_double,
) {
    js_pushobject(J, jsV_newnumber(J, v));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newstring(
    mut J: *mut js_State,
    mut v: *const ::core::ffi::c_char,
) {
    js_pushobject(J, jsV_newstring(J, v));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newfunction(
    mut J: *mut js_State,
    mut fun: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    let mut obj: *mut js_Object = jsV_newobject(
        J,
        JS_CFUNCTION,
        (*J).Function_prototype,
    );
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
    js_pushnumber(J, (*fun).numparams as ::core::ffi::c_double);
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"length\0" as *const u8 as *const ::core::ffi::c_char,
        JS_READONLY as ::core::ffi::c_int | JS_DONTENUM as ::core::ffi::c_int
            | JS_DONTCONF as ::core::ffi::c_int,
    );
    js_newobject(J);
    js_copy(J, -(2 as ::core::ffi::c_int));
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"constructor\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int | JS_DONTCONF as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newscript(
    mut J: *mut js_State,
    mut fun: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    let mut obj: *mut js_Object = jsV_newobject(
        J,
        JS_CSCRIPT,
        ::core::ptr::null_mut::<js_Object>(),
    );
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcfunctionx(
    mut J: *mut js_State,
    mut cfun: js_CFunction,
    mut name: *const ::core::ffi::c_char,
    mut length: ::core::ffi::c_int,
    mut data: *mut ::core::ffi::c_void,
    mut finalize: js_Finalize,
) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        if finalize.is_some() {
            finalize.expect("non-null function pointer")(J, data);
        }
        js_throw(J);
    }
    obj = jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = None;
    (*obj).u.c.length = length;
    (*obj).u.c.data = data;
    (*obj).u.c.finalize = finalize;
    js_endtry(J);
    js_pushobject(J, obj);
    js_pushnumber(J, length as ::core::ffi::c_double);
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"length\0" as *const u8 as *const ::core::ffi::c_char,
        JS_READONLY as ::core::ffi::c_int | JS_DONTENUM as ::core::ffi::c_int
            | JS_DONTCONF as ::core::ffi::c_int,
    );
    js_newobject(J);
    js_copy(J, -(2 as ::core::ffi::c_int));
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"constructor\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int | JS_DONTCONF as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcfunction(
    mut J: *mut js_State,
    mut cfun: js_CFunction,
    mut name: *const ::core::ffi::c_char,
    mut length: ::core::ffi::c_int,
) {
    js_newcfunctionx(J, cfun, name, length, NULL, None);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcconstructor(
    mut J: *mut js_State,
    mut cfun: js_CFunction,
    mut ccon: js_CFunction,
    mut name: *const ::core::ffi::c_char,
    mut length: ::core::ffi::c_int,
) {
    let mut obj: *mut js_Object = jsV_newobject(
        J,
        JS_CCFUNCTION,
        (*J).Function_prototype,
    );
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = ccon;
    (*obj).u.c.length = length;
    js_pushobject(J, obj);
    js_pushnumber(J, length as ::core::ffi::c_double);
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"length\0" as *const u8 as *const ::core::ffi::c_char,
        JS_READONLY as ::core::ffi::c_int | JS_DONTENUM as ::core::ffi::c_int
            | JS_DONTCONF as ::core::ffi::c_int,
    );
    js_rot2(J);
    js_copy(J, -(2 as ::core::ffi::c_int));
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"constructor\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
    js_defproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int | JS_DONTCONF as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newuserdatax(
    mut J: *mut js_State,
    mut tag: *const ::core::ffi::c_char,
    mut data: *mut ::core::ffi::c_void,
    mut has: js_HasProperty,
    mut put: js_Put,
    mut delete: js_Delete,
    mut finalize: js_Finalize,
) {
    let mut prototype: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, -(1 as ::core::ffi::c_int)) != 0 {
        prototype = js_toobject(J, -(1 as ::core::ffi::c_int));
    }
    js_pop(J, 1 as ::core::ffi::c_int);
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        if finalize.is_some() {
            finalize.expect("non-null function pointer")(J, data);
        }
        js_throw(J);
    }
    obj = jsV_newobject(J, JS_CUSERDATA, prototype);
    (*obj).u.user.tag = tag;
    (*obj).u.user.data = data;
    (*obj).u.user.has = has;
    (*obj).u.user.put = put;
    (*obj).u.user.delete = delete;
    (*obj).u.user.finalize = finalize;
    js_endtry(J);
    js_pushobject(J, obj);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newuserdata(
    mut J: *mut js_State,
    mut tag: *const ::core::ffi::c_char,
    mut data: *mut ::core::ffi::c_void,
    mut finalize: js_Finalize,
) {
    js_newuserdatax(J, tag, data, None, None, None, finalize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_instanceof(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut O: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut V: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_iscallable(J, -(1 as ::core::ffi::c_int)) == 0 {
        js_typeerror(
            J,
            b"instanceof: invalid operand\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if js_isobject(J, -(2 as ::core::ffi::c_int)) == 0 {
        return 0 as ::core::ffi::c_int;
    }
    js_getproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_isobject(J, -(1 as ::core::ffi::c_int)) == 0 {
        js_typeerror(
            J,
            b"instanceof: 'prototype' property is not an object\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    O = js_toobject(J, -(1 as ::core::ffi::c_int));
    js_pop(J, 1 as ::core::ffi::c_int);
    V = js_toobject(J, -(2 as ::core::ffi::c_int));
    while !V.is_null() {
        V = (*V).prototype;
        if O == V {
            return 1 as ::core::ffi::c_int;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_concat(mut J: *mut js_State) {
    js_toprimitive(J, -(2 as ::core::ffi::c_int), JS_HNONE as ::core::ffi::c_int);
    js_toprimitive(J, -(1 as ::core::ffi::c_int), JS_HNONE as ::core::ffi::c_int);
    if js_isstring(J, -(2 as ::core::ffi::c_int)) != 0
        || js_isstring(J, -(1 as ::core::ffi::c_int)) != 0
    {
        let mut sa: *const ::core::ffi::c_char = js_tostring(
            J,
            -(2 as ::core::ffi::c_int),
        );
        let mut sb: *const ::core::ffi::c_char = js_tostring(
            J,
            -(1 as ::core::ffi::c_int),
        );
        let mut sab: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
            ::core::ffi::c_char,
        >();
        if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
            js_free(J, sab as *mut ::core::ffi::c_void);
            js_throw(J);
        }
        ::core::ptr::write_volatile(
            &mut sab as *mut *mut ::core::ffi::c_char,
            js_malloc(
                J,
                strlen(sa).wrapping_add(strlen(sb)).wrapping_add(1 as size_t)
                    as ::core::ffi::c_int,
            ) as *mut ::core::ffi::c_char,
        );
        strcpy(sab, sa);
        strcat(sab, sb);
        js_pop(J, 2 as ::core::ffi::c_int);
        js_pushstring(J, sab);
        js_endtry(J);
        js_free(J, sab as *mut ::core::ffi::c_void);
    } else {
        let mut x: ::core::ffi::c_double = js_tonumber(J, -(2 as ::core::ffi::c_int));
        let mut y: ::core::ffi::c_double = js_tonumber(J, -(1 as ::core::ffi::c_int));
        js_pop(J, 2 as ::core::ffi::c_int);
        js_pushnumber(J, x + y);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_compare(
    mut J: *mut js_State,
    mut okay: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    js_toprimitive(J, -(2 as ::core::ffi::c_int), JS_HNUMBER as ::core::ffi::c_int);
    js_toprimitive(J, -(1 as ::core::ffi::c_int), JS_HNUMBER as ::core::ffi::c_int);
    *okay = 1 as ::core::ffi::c_int;
    if js_isstring(J, -(2 as ::core::ffi::c_int)) != 0
        && js_isstring(J, -(1 as ::core::ffi::c_int)) != 0
    {
        return strcmp(
            js_tostring(J, -(2 as ::core::ffi::c_int)),
            js_tostring(J, -(1 as ::core::ffi::c_int)),
        )
    } else {
        let mut x: ::core::ffi::c_double = js_tonumber(J, -(2 as ::core::ffi::c_int));
        let mut y: ::core::ffi::c_double = js_tonumber(J, -(1 as ::core::ffi::c_int));
        if x.is_nan() as i32 != 0 || y.is_nan() as i32 != 0 {
            *okay = 0 as ::core::ffi::c_int;
        }
        return if x < y {
            -(1 as ::core::ffi::c_int)
        } else if x > y {
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        };
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_equal(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut x: *mut js_Value = js_tovalue(J, -(2 as ::core::ffi::c_int));
    let mut y: *mut js_Value = js_tovalue(J, -(1 as ::core::ffi::c_int));
    loop {
        if ((*x).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TMEMSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TLITSTR as ::core::ffi::c_int)
            && ((*y).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
                || (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TMEMSTR as ::core::ffi::c_int
                || (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TLITSTR as ::core::ffi::c_int)
        {
            return (strcmp(
                if (*x).t.type_0 as ::core::ffi::c_int
                    == JS_TSHRSTR as ::core::ffi::c_int
                {
                    &raw mut (*x).u.shrstr as *mut ::core::ffi::c_char
                        as *const ::core::ffi::c_char
                } else if (*x).t.type_0 as ::core::ffi::c_int
                    == JS_TLITSTR as ::core::ffi::c_int
                {
                    (*x).u.litstr
                } else if (*x).t.type_0 as ::core::ffi::c_int
                    == JS_TMEMSTR as ::core::ffi::c_int
                {
                    &raw mut (*(*x).u.memstr).p as *mut ::core::ffi::c_char
                        as *const ::core::ffi::c_char
                } else {
                    b"\0" as *const u8 as *const ::core::ffi::c_char
                },
                if (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TSHRSTR as ::core::ffi::c_int
                {
                    &raw mut (*y).u.shrstr as *mut ::core::ffi::c_char
                        as *const ::core::ffi::c_char
                } else if (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TLITSTR as ::core::ffi::c_int
                {
                    (*y).u.litstr
                } else if (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TMEMSTR as ::core::ffi::c_int
                {
                    &raw mut (*(*y).u.memstr).p as *mut ::core::ffi::c_char
                        as *const ::core::ffi::c_char
                } else {
                    b"\0" as *const u8 as *const ::core::ffi::c_char
                },
            ) == 0) as ::core::ffi::c_int;
        }
        if (*x).t.type_0 as ::core::ffi::c_int == (*y).t.type_0 as ::core::ffi::c_int {
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TUNDEFINED as ::core::ffi::c_int
            {
                return 1 as ::core::ffi::c_int;
            }
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TNULL as ::core::ffi::c_int {
                return 1 as ::core::ffi::c_int;
            }
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int {
                return ((*x).u.number == (*y).u.number) as ::core::ffi::c_int;
            }
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TBOOLEAN as ::core::ffi::c_int {
                return ((*x).u.boolean == (*y).u.boolean) as ::core::ffi::c_int;
            }
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int {
                return ((*x).u.object == (*y).u.object) as ::core::ffi::c_int;
            }
            return 0 as ::core::ffi::c_int;
        }
        if (*x).t.type_0 as ::core::ffi::c_int == JS_TNULL as ::core::ffi::c_int
            && (*y).t.type_0 as ::core::ffi::c_int == JS_TUNDEFINED as ::core::ffi::c_int
        {
            return 1 as ::core::ffi::c_int;
        }
        if (*x).t.type_0 as ::core::ffi::c_int == JS_TUNDEFINED as ::core::ffi::c_int
            && (*y).t.type_0 as ::core::ffi::c_int == JS_TNULL as ::core::ffi::c_int
        {
            return 1 as ::core::ffi::c_int;
        }
        if (*x).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int
            && ((*y).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
                || (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TMEMSTR as ::core::ffi::c_int
                || (*y).t.type_0 as ::core::ffi::c_int
                    == JS_TLITSTR as ::core::ffi::c_int)
        {
            return ((*x).u.number == jsV_tonumber(J, y)) as ::core::ffi::c_int;
        }
        if ((*x).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TMEMSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TLITSTR as ::core::ffi::c_int)
            && (*y).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int
        {
            return (jsV_tonumber(J, x) == (*y).u.number) as ::core::ffi::c_int;
        }
        if (*x).t.type_0 as ::core::ffi::c_int == JS_TBOOLEAN as ::core::ffi::c_int {
            (*x).t.type_0 = JS_TNUMBER as ::core::ffi::c_int as ::core::ffi::c_char;
            (*x).u.number = (if (*x).u.boolean != 0 {
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            }) as ::core::ffi::c_double;
        } else if (*y).t.type_0 as ::core::ffi::c_int
            == JS_TBOOLEAN as ::core::ffi::c_int
        {
            (*y).t.type_0 = JS_TNUMBER as ::core::ffi::c_int as ::core::ffi::c_char;
            (*y).u.number = (if (*y).u.boolean != 0 {
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            }) as ::core::ffi::c_double;
        } else if ((*x).t.type_0 as ::core::ffi::c_int
            == JS_TSHRSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TMEMSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TLITSTR as ::core::ffi::c_int
            || (*x).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int)
            && (*y).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        {
            jsV_toprimitive(J, y, JS_HNONE as ::core::ffi::c_int);
        } else {
            if !((*x).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
                && ((*y).t.type_0 as ::core::ffi::c_int
                    == JS_TSHRSTR as ::core::ffi::c_int
                    || (*y).t.type_0 as ::core::ffi::c_int
                        == JS_TMEMSTR as ::core::ffi::c_int
                    || (*y).t.type_0 as ::core::ffi::c_int
                        == JS_TLITSTR as ::core::ffi::c_int
                    || (*y).t.type_0 as ::core::ffi::c_int
                        == JS_TNUMBER as ::core::ffi::c_int))
            {
                break;
            }
            jsV_toprimitive(J, x, JS_HNONE as ::core::ffi::c_int);
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_strictequal(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut x: *mut js_Value = js_tovalue(J, -(2 as ::core::ffi::c_int));
    let mut y: *mut js_Value = js_tovalue(J, -(1 as ::core::ffi::c_int));
    if ((*x).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
        || (*x).t.type_0 as ::core::ffi::c_int == JS_TMEMSTR as ::core::ffi::c_int
        || (*x).t.type_0 as ::core::ffi::c_int == JS_TLITSTR as ::core::ffi::c_int)
        && ((*y).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int
            || (*y).t.type_0 as ::core::ffi::c_int == JS_TMEMSTR as ::core::ffi::c_int
            || (*y).t.type_0 as ::core::ffi::c_int == JS_TLITSTR as ::core::ffi::c_int)
    {
        return (strcmp(
            if (*x).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int {
                &raw mut (*x).u.shrstr as *mut ::core::ffi::c_char
                    as *const ::core::ffi::c_char
            } else if (*x).t.type_0 as ::core::ffi::c_int
                == JS_TLITSTR as ::core::ffi::c_int
            {
                (*x).u.litstr
            } else if (*x).t.type_0 as ::core::ffi::c_int
                == JS_TMEMSTR as ::core::ffi::c_int
            {
                &raw mut (*(*x).u.memstr).p as *mut ::core::ffi::c_char
                    as *const ::core::ffi::c_char
            } else {
                b"\0" as *const u8 as *const ::core::ffi::c_char
            },
            if (*y).t.type_0 as ::core::ffi::c_int == JS_TSHRSTR as ::core::ffi::c_int {
                &raw mut (*y).u.shrstr as *mut ::core::ffi::c_char
                    as *const ::core::ffi::c_char
            } else if (*y).t.type_0 as ::core::ffi::c_int
                == JS_TLITSTR as ::core::ffi::c_int
            {
                (*y).u.litstr
            } else if (*y).t.type_0 as ::core::ffi::c_int
                == JS_TMEMSTR as ::core::ffi::c_int
            {
                &raw mut (*(*y).u.memstr).p as *mut ::core::ffi::c_char
                    as *const ::core::ffi::c_char
            } else {
                b"\0" as *const u8 as *const ::core::ffi::c_char
            },
        ) == 0) as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int != (*y).t.type_0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int == JS_TUNDEFINED as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int == JS_TNULL as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int {
        return ((*x).u.number == (*y).u.number) as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int == JS_TBOOLEAN as ::core::ffi::c_int {
        return ((*x).u.boolean == (*y).u.boolean) as ::core::ffi::c_int;
    }
    if (*x).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int {
        return ((*x).u.object == (*y).u.object) as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
