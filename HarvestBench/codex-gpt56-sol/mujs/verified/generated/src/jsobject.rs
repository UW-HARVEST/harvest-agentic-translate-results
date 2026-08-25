extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_defglobal(
        J: *mut js_State,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_hasproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn js_defproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_defaccessor(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_getindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_setindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_pushundefined(J: *mut js_State);
    fn js_pushnull(J: *mut js_State);
    fn js_pushboolean(J: *mut js_State, v: ::core::ffi::c_int);
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_pushliteral(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_newobject(J: *mut js_State);
    fn js_newarray(J: *mut js_State);
    fn js_newcconstructor(
        J: *mut js_State,
        fun: js_CFunction,
        con: js_CFunction,
        name: *const ::core::ffi::c_char,
        length: ::core::ffi::c_int,
    );
    fn js_isdefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isundefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isnull(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isobject(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_toboolean(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
    fn js_concat(J: *mut js_State);
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
    fn js_isarrayindex(
        J: *mut js_State,
        str: *const ::core::ffi::c_char,
        idx: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_toobject(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Object;
    fn js_pushvalue(J: *mut js_State, v: js_Value);
    fn js_pushobject(J: *mut js_State, v: *mut js_Object);
    fn jsR_unflattenarray(J: *mut js_State, obj: *mut js_Object);
    fn js_itoa(
        buf: *mut ::core::ffi::c_char,
        a: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn jsV_newobject(
        J: *mut js_State,
        type_0: js_Class,
        prototype: *mut js_Object,
    ) -> *mut js_Object;
    fn jsV_getownproperty(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
    ) -> *mut js_Property;
    fn jsV_getproperty(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
    ) -> *mut js_Property;
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
unsafe extern "C" fn jsB_new_Object(mut J: *mut js_State) {
    if js_isundefined(J, 1 as ::core::ffi::c_int) != 0
        || js_isnull(J, 1 as ::core::ffi::c_int) != 0
    {
        js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1 as ::core::ffi::c_int));
    };
}
unsafe extern "C" fn jsB_Object(mut J: *mut js_State) {
    if js_isundefined(J, 1 as ::core::ffi::c_int) != 0
        || js_isnull(J, 1 as ::core::ffi::c_int) != 0
    {
        js_newobject(J);
    } else {
        js_pushobject(J, js_toobject(J, 1 as ::core::ffi::c_int));
    };
}
unsafe extern "C" fn Op_toString(mut J: *mut js_State) {
    if js_isundefined(J, 0 as ::core::ffi::c_int) != 0 {
        js_pushliteral(
            J,
            b"[object Undefined]\0" as *const u8 as *const ::core::ffi::c_char,
        );
    } else if js_isnull(J, 0 as ::core::ffi::c_int) != 0 {
        js_pushliteral(J, b"[object Null]\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
        match (*self_0).type_0 as ::core::ffi::c_uint {
            0 => {
                js_pushliteral(
                    J,
                    b"[object Object]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            1 => {
                js_pushliteral(
                    J,
                    b"[object Array]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            2 => {
                js_pushliteral(
                    J,
                    b"[object Function]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            3 => {
                js_pushliteral(
                    J,
                    b"[object Function]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            4 => {
                js_pushliteral(
                    J,
                    b"[object Function]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            5 => {
                js_pushliteral(
                    J,
                    b"[object Error]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            6 => {
                js_pushliteral(
                    J,
                    b"[object Boolean]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            7 => {
                js_pushliteral(
                    J,
                    b"[object Number]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            8 => {
                js_pushliteral(
                    J,
                    b"[object String]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            9 => {
                js_pushliteral(
                    J,
                    b"[object RegExp]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            10 => {
                js_pushliteral(
                    J,
                    b"[object Date]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            11 => {
                js_pushliteral(
                    J,
                    b"[object Math]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            12 => {
                js_pushliteral(
                    J,
                    b"[object JSON]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            13 => {
                js_pushliteral(
                    J,
                    b"[object Arguments]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            14 => {
                js_pushliteral(
                    J,
                    b"[object Iterator]\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            15 => {
                js_pushliteral(
                    J,
                    b"[object \0" as *const u8 as *const ::core::ffi::c_char,
                );
                js_pushliteral(J, (*self_0).u.user.tag);
                js_concat(J);
                js_pushliteral(J, b"]\0" as *const u8 as *const ::core::ffi::c_char);
                js_concat(J);
            }
            _ => {}
        }
    };
}
unsafe extern "C" fn Op_valueOf(mut J: *mut js_State) {
    js_copy(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Op_hasOwnProperty(mut J: *mut js_State) {
    let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    let mut name: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut k: ::core::ffi::c_int = 0;
    if (*self_0).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if js_isarrayindex(J, name, &raw mut k) != 0 && k >= 0 as ::core::ffi::c_int
            && k < (*self_0).u.s.length
        {
            js_pushboolean(J, 1 as ::core::ffi::c_int);
            return;
        }
    }
    if (*self_0).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*self_0).u.a.simple != 0
    {
        if js_isarrayindex(J, name, &raw mut k) != 0 && k >= 0 as ::core::ffi::c_int
            && k < (*self_0).u.a.flat_length
        {
            js_pushboolean(J, 1 as ::core::ffi::c_int);
            return;
        }
    }
    ref_0 = jsV_getownproperty(J, self_0, name);
    js_pushboolean(J, (ref_0 != NULL_0 as *mut js_Property) as ::core::ffi::c_int);
}
unsafe extern "C" fn Op_isPrototypeOf(mut J: *mut js_State) {
    let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    if js_isobject(J, 1 as ::core::ffi::c_int) != 0 {
        let mut V: *mut js_Object = js_toobject(J, 1 as ::core::ffi::c_int);
        loop {
            V = (*V).prototype;
            if V == self_0 {
                js_pushboolean(J, 1 as ::core::ffi::c_int);
                return;
            }
            if V.is_null() {
                break;
            }
        }
    }
    js_pushboolean(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Op_propertyIsEnumerable(mut J: *mut js_State) {
    let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    let mut name: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    let mut ref_0: *mut js_Property = jsV_getownproperty(J, self_0, name);
    js_pushboolean(
        J,
        (!ref_0.is_null() && (*ref_0).atts & JS_DONTENUM as ::core::ffi::c_int == 0)
            as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn O_getPrototypeOf(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    if !(*obj).prototype.is_null() {
        js_pushobject(J, (*obj).prototype);
    } else {
        js_pushnull(J);
    };
}
unsafe extern "C" fn O_getOwnPropertyDescriptor(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    ref_0 = jsV_getproperty(J, obj, js_tostring(J, 2 as ::core::ffi::c_int));
    if ref_0.is_null() {
        js_pushundefined(J);
    } else {
        js_newobject(J);
        if (*ref_0).getter.is_null() && (*ref_0).setter.is_null() {
            js_pushvalue(J, (*ref_0).value);
            js_defproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"value\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
            );
            js_pushboolean(
                J,
                ((*ref_0).atts & JS_READONLY as ::core::ffi::c_int == 0)
                    as ::core::ffi::c_int,
            );
            js_defproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"writable\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
            );
        } else {
            if !(*ref_0).getter.is_null() {
                js_pushobject(J, (*ref_0).getter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"get\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
            );
            if !(*ref_0).setter.is_null() {
                js_pushobject(J, (*ref_0).setter);
            } else {
                js_pushundefined(J);
            }
            js_defproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"set\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
            );
        }
        js_pushboolean(
            J,
            ((*ref_0).atts & JS_DONTENUM as ::core::ffi::c_int == 0)
                as ::core::ffi::c_int,
        );
        js_defproperty(
            J,
            -(2 as ::core::ffi::c_int),
            b"enumerable\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
        js_pushboolean(
            J,
            ((*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int == 0)
                as ::core::ffi::c_int,
        );
        js_defproperty(
            J,
            -(2 as ::core::ffi::c_int),
            b"configurable\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
    };
}
unsafe extern "C" fn O_getOwnPropertyNames_walk(
    mut J: *mut js_State,
    mut ref_0: *mut js_Property,
    mut i: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*(*ref_0).left).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*ref_0).left, i);
    }
    js_pushstring(J, &raw mut (*ref_0).name as *mut ::core::ffi::c_char);
    let fresh19 = i;
    i = i + 1;
    js_setindex(J, -(2 as ::core::ffi::c_int), fresh19);
    if (*(*ref_0).right).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*ref_0).right, i);
    }
    return i;
}
unsafe extern "C" fn O_getOwnPropertyNames(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut name: [::core::ffi::c_char; 32] = [0; 32];
    let mut k: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    js_newarray(J);
    if (*(*obj).properties).level != 0 {
        i = O_getOwnPropertyNames_walk(J, (*obj).properties, 0 as ::core::ffi::c_int);
    } else {
        i = 0 as ::core::ffi::c_int;
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_pushliteral(J, b"length\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh10 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh10);
        if (*obj).u.a.simple != 0 {
            k = 0 as ::core::ffi::c_int;
            while k < (*obj).u.a.flat_length {
                js_itoa(&raw mut name as *mut ::core::ffi::c_char, k);
                js_pushstring(J, &raw mut name as *mut ::core::ffi::c_char);
                let fresh11 = i;
                i = i + 1;
                js_setindex(J, -(2 as ::core::ffi::c_int), fresh11);
                k += 1;
            }
        }
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_pushliteral(J, b"length\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh12 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh12);
        k = 0 as ::core::ffi::c_int;
        while k < (*obj).u.s.length {
            js_itoa(&raw mut name as *mut ::core::ffi::c_char, k);
            js_pushstring(J, &raw mut name as *mut ::core::ffi::c_char);
            let fresh13 = i;
            i = i + 1;
            js_setindex(J, -(2 as ::core::ffi::c_int), fresh13);
            k += 1;
        }
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_pushliteral(J, b"source\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh14 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh14);
        js_pushliteral(J, b"global\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh15 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh15);
        js_pushliteral(J, b"ignoreCase\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh16 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh16);
        js_pushliteral(J, b"multiline\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh17 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh17);
        js_pushliteral(J, b"lastIndex\0" as *const u8 as *const ::core::ffi::c_char);
        let fresh18 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh18);
    }
}
unsafe extern "C" fn ToPropertyDescriptor(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
    mut desc: *mut js_Object,
) {
    let mut haswritable: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut hasvalue: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut enumerable: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut configurable: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut writable: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut atts: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    js_pushobject(J, obj);
    js_pushobject(J, desc);
    if js_hasproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"writable\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        haswritable = 1 as ::core::ffi::c_int;
        writable = js_toboolean(J, -(1 as ::core::ffi::c_int));
        js_pop(J, 1 as ::core::ffi::c_int);
    }
    if js_hasproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"enumerable\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        enumerable = js_toboolean(J, -(1 as ::core::ffi::c_int));
        js_pop(J, 1 as ::core::ffi::c_int);
    }
    if js_hasproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"configurable\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        configurable = js_toboolean(J, -(1 as ::core::ffi::c_int));
        js_pop(J, 1 as ::core::ffi::c_int);
    }
    if js_hasproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"value\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        hasvalue = 1 as ::core::ffi::c_int;
        js_defproperty(J, -(3 as ::core::ffi::c_int), name, 0 as ::core::ffi::c_int);
    }
    if writable == 0 {
        atts |= JS_READONLY as ::core::ffi::c_int;
    }
    if enumerable == 0 {
        atts |= JS_DONTENUM as ::core::ffi::c_int;
    }
    if configurable == 0 {
        atts |= JS_DONTCONF as ::core::ffi::c_int;
    }
    if js_hasproperty(
        J,
        -(1 as ::core::ffi::c_int),
        b"get\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        if haswritable != 0 || hasvalue != 0 {
            js_typeerror(
                J,
                b"value/writable and get/set attributes are exclusive\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    } else {
        js_pushundefined(J);
    }
    if js_hasproperty(
        J,
        -(2 as ::core::ffi::c_int),
        b"set\0" as *const u8 as *const ::core::ffi::c_char,
    ) != 0
    {
        if haswritable != 0 || hasvalue != 0 {
            js_typeerror(
                J,
                b"value/writable and get/set attributes are exclusive\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    } else {
        js_pushundefined(J);
    }
    js_defaccessor(J, -(4 as ::core::ffi::c_int), name, atts);
    js_pop(J, 2 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_defineProperty(mut J: *mut js_State) {
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if js_isobject(J, 3 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    ToPropertyDescriptor(
        J,
        js_toobject(J, 1 as ::core::ffi::c_int),
        js_tostring(J, 2 as ::core::ffi::c_int),
        js_toobject(J, 3 as ::core::ffi::c_int),
    );
    js_copy(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_defineProperties_walk(
    mut J: *mut js_State,
    mut ref_0: *mut js_Property,
    mut i: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*(*ref_0).left).level != 0 {
        i = O_defineProperties_walk(J, (*ref_0).left, i);
    }
    if (*ref_0).atts & JS_DONTENUM as ::core::ffi::c_int == 0 {
        if (*ref_0).value.t.type_0 as ::core::ffi::c_int
            != JS_TOBJECT as ::core::ffi::c_int
        {
            js_typeerror(
                J,
                b"not an object\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        js_pushstring(J, &raw mut (*ref_0).name as *mut ::core::ffi::c_char);
        let fresh9 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh9);
    }
    if (*(*ref_0).right).level != 0 {
        i = O_defineProperties_walk(J, (*ref_0).right, i);
    }
    return i;
}
unsafe extern "C" fn O_defineProperties_imp(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
) {
    let mut props: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut name: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut i: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    if js_isobject(J, 2 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    props = js_toobject(J, 2 as ::core::ffi::c_int);
    if (*(*props).properties).level != 0 {
        js_newarray(J);
        n = O_defineProperties_walk(J, (*props).properties, 0 as ::core::ffi::c_int);
        i = 0 as ::core::ffi::c_int;
        while i < n {
            js_getindex(J, -(1 as ::core::ffi::c_int), i);
            name = js_tostring(J, -(1 as ::core::ffi::c_int));
            if js_hasproperty(J, 2 as ::core::ffi::c_int, name) != 0 {
                ToPropertyDescriptor(
                    J,
                    obj,
                    name,
                    js_toobject(J, -(1 as ::core::ffi::c_int)),
                );
                js_pop(J, 1 as ::core::ffi::c_int);
            }
            js_pop(J, 1 as ::core::ffi::c_int);
            i += 1;
        }
        js_pop(J, 1 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn O_defineProperties(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    O_defineProperties_imp(J, obj);
    js_copy(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_create(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut proto: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) != 0 {
        proto = js_toobject(J, 1 as ::core::ffi::c_int);
    } else if js_isnull(J, 1 as ::core::ffi::c_int) != 0 {
        proto = ::core::ptr::null_mut::<js_Object>();
    } else {
        js_typeerror(
            J,
            b"not an object or null\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    obj = jsV_newobject(J, JS_COBJECT, proto);
    js_pushobject(J, obj);
    if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        O_defineProperties_imp(J, obj);
    }
}
unsafe extern "C" fn O_keys_walk(
    mut J: *mut js_State,
    mut ref_0: *mut js_Property,
    mut i: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*(*ref_0).left).level != 0 {
        i = O_keys_walk(J, (*ref_0).left, i);
    }
    if (*ref_0).atts & JS_DONTENUM as ::core::ffi::c_int == 0 {
        js_pushstring(J, &raw mut (*ref_0).name as *mut ::core::ffi::c_char);
        let fresh8 = i;
        i = i + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh8);
    }
    if (*(*ref_0).right).level != 0 {
        i = O_keys_walk(J, (*ref_0).right, i);
    }
    return i;
}
unsafe extern "C" fn O_keys(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut name: [::core::ffi::c_char; 32] = [0; 32];
    let mut i: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    js_newarray(J);
    if (*(*obj).properties).level != 0 {
        i = O_keys_walk(J, (*obj).properties, 0 as ::core::ffi::c_int);
    } else {
        i = 0 as ::core::ffi::c_int;
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        k = 0 as ::core::ffi::c_int;
        while k < (*obj).u.s.length {
            js_itoa(&raw mut name as *mut ::core::ffi::c_char, k);
            js_pushstring(J, &raw mut name as *mut ::core::ffi::c_char);
            let fresh6 = i;
            i = i + 1;
            js_setindex(J, -(2 as ::core::ffi::c_int), fresh6);
            k += 1;
        }
    }
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0
    {
        k = 0 as ::core::ffi::c_int;
        while k < (*obj).u.a.flat_length {
            js_itoa(&raw mut name as *mut ::core::ffi::c_char, k);
            js_pushstring(J, &raw mut name as *mut ::core::ffi::c_char);
            let fresh7 = i;
            i = i + 1;
            js_setindex(J, -(2 as ::core::ffi::c_int), fresh7);
            k += 1;
        }
    }
}
unsafe extern "C" fn O_preventExtensions(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0 as ::core::ffi::c_int;
    js_copy(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_isExtensible(mut J: *mut js_State) {
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    js_pushboolean(J, (*js_toobject(J, 1 as ::core::ffi::c_int)).extensible);
}
unsafe extern "C" fn O_seal_walk(mut J: *mut js_State, mut ref_0: *mut js_Property) {
    if (*(*ref_0).left).level != 0 {
        O_seal_walk(J, (*ref_0).left);
    }
    (*ref_0).atts |= JS_DONTCONF as ::core::ffi::c_int;
    if (*(*ref_0).right).level != 0 {
        O_seal_walk(J, (*ref_0).right);
    }
}
unsafe extern "C" fn O_seal(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0 as ::core::ffi::c_int;
    if (*(*obj).properties).level != 0 {
        O_seal_walk(J, (*obj).properties);
    }
    js_copy(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_isSealed_walk(
    mut J: *mut js_State,
    mut ref_0: *mut js_Property,
) -> ::core::ffi::c_int {
    if (*(*ref_0).left).level != 0 {
        if O_isSealed_walk(J, (*ref_0).left) == 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int == 0 {
        return 0 as ::core::ffi::c_int;
    }
    if (*(*ref_0).right).level != 0 {
        if O_isSealed_walk(J, (*ref_0).right) == 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn O_isSealed(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    if (*obj).extensible != 0 {
        js_pushboolean(J, 0 as ::core::ffi::c_int);
        return;
    }
    if (*(*obj).properties).level != 0 {
        js_pushboolean(J, O_isSealed_walk(J, (*obj).properties));
    } else {
        js_pushboolean(J, 1 as ::core::ffi::c_int);
    };
}
unsafe extern "C" fn O_freeze_walk(mut J: *mut js_State, mut ref_0: *mut js_Property) {
    if (*(*ref_0).left).level != 0 {
        O_freeze_walk(J, (*ref_0).left);
    }
    (*ref_0).atts
        |= JS_READONLY as ::core::ffi::c_int | JS_DONTCONF as ::core::ffi::c_int;
    if (*(*ref_0).right).level != 0 {
        O_freeze_walk(J, (*ref_0).right);
    }
}
unsafe extern "C" fn O_freeze(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    jsR_unflattenarray(J, obj);
    (*obj).extensible = 0 as ::core::ffi::c_int;
    if (*(*obj).properties).level != 0 {
        O_freeze_walk(J, (*obj).properties);
    }
    js_copy(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn O_isFrozen_walk(
    mut J: *mut js_State,
    mut ref_0: *mut js_Property,
) -> ::core::ffi::c_int {
    if (*(*ref_0).left).level != 0 {
        if O_isFrozen_walk(J, (*ref_0).left) == 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    if (*ref_0).atts & JS_READONLY as ::core::ffi::c_int == 0 {
        return 0 as ::core::ffi::c_int;
    }
    if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int == 0 {
        return 0 as ::core::ffi::c_int;
    }
    if (*(*ref_0).right).level != 0 {
        if O_isFrozen_walk(J, (*ref_0).right) == 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn O_isFrozen(mut J: *mut js_State) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_isobject(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(J, b"not an object\0" as *const u8 as *const ::core::ffi::c_char);
    }
    obj = js_toobject(J, 1 as ::core::ffi::c_int);
    if (*(*obj).properties).level != 0 {
        if O_isFrozen_walk(J, (*obj).properties) == 0 {
            js_pushboolean(J, 0 as ::core::ffi::c_int);
            return;
        }
    }
    js_pushboolean(J, ((*obj).extensible == 0) as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn jsB_initobject(mut J: *mut js_State) {
    js_pushobject(J, (*J).Object_prototype);
    jsB_propf(
        J,
        b"Object.prototype.toString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Op_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.prototype.toLocaleString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Op_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.prototype.valueOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Op_valueOf as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.prototype.hasOwnProperty\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Op_hasOwnProperty as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.prototype.isPrototypeOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Op_isPrototypeOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.prototype.propertyIsEnumerable\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Op_propertyIsEnumerable as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    js_newcconstructor(
        J,
        Some(jsB_Object as unsafe extern "C" fn(*mut js_State) -> ()),
        Some(jsB_new_Object as unsafe extern "C" fn(*mut js_State) -> ()),
        b"Object\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.getPrototypeOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_getPrototypeOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.getOwnPropertyDescriptor\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_getOwnPropertyDescriptor as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.getOwnPropertyNames\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_getOwnPropertyNames as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.create\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_create as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.defineProperty\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_defineProperty as unsafe extern "C" fn(*mut js_State) -> ()),
        3 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.defineProperties\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_defineProperties as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.seal\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_seal as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.freeze\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_freeze as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.preventExtensions\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_preventExtensions as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.isSealed\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_isSealed as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.isFrozen\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_isFrozen as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.isExtensible\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_isExtensible as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Object.keys\0" as *const u8 as *const ::core::ffi::c_char,
        Some(O_keys as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    js_defglobal(
        J,
        b"Object\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
}
