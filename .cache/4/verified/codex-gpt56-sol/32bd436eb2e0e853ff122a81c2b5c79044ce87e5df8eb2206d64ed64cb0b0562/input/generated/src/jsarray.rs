extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn js_savetry(J: *mut js_State) -> *mut ::core::ffi::c_void;
    fn js_endtry(J: *mut js_State);
    fn js_rangeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_throw(J: *mut js_State) -> !;
    fn js_call(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_getglobal(J: *mut js_State, name: *const ::core::ffi::c_char);
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
    fn js_setproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    );
    fn js_hasindex(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_getindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_setindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_delindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_pushundefined(J: *mut js_State);
    fn js_pushboolean(J: *mut js_State, v: ::core::ffi::c_int);
    fn js_pushnumber(J: *mut js_State, v: ::core::ffi::c_double);
    fn js_pushlstring(
        J: *mut js_State,
        v: *const ::core::ffi::c_char,
        n: ::core::ffi::c_int,
    );
    fn js_pushliteral(J: *mut js_State, v: *const ::core::ffi::c_char);
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
    fn js_isnumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isobject(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isarray(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscoercible(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscallable(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_toboolean(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tonumber(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_double;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_tointeger(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_gettop(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_rot(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
    fn js_rot2pop1(J: *mut js_State);
    fn js_strictequal(J: *mut js_State) -> ::core::ffi::c_int;
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
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_realloc(
        J: *mut js_State,
        ptr: *mut ::core::ffi::c_void,
        size: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_tovalue(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Value;
    fn js_toobject(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Object;
    fn js_pushvalue(J: *mut js_State, v: js_Value);
    fn js_pushobject(J: *mut js_State, v: *mut js_Object);
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
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const JS_STRLIMIT: ::core::ffi::c_int = (1 as ::core::ffi::c_int)
    << 28 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn js_getlength(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = 0;
    js_getproperty(J, idx, b"length\0" as *const u8 as *const ::core::ffi::c_char);
    len = js_tointeger(J, -(1 as ::core::ffi::c_int));
    js_pop(J, 1 as ::core::ffi::c_int);
    return len;
}
#[no_mangle]
pub unsafe extern "C" fn js_setlength(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut len: ::core::ffi::c_int,
) {
    js_pushnumber(J, len as ::core::ffi::c_double);
    js_setproperty(
        J,
        if idx < 0 as ::core::ffi::c_int { idx - 1 as ::core::ffi::c_int } else { idx },
        b"length\0" as *const u8 as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn jsB_new_Array(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    js_newarray(J);
    if top == 2 as ::core::ffi::c_int {
        if js_isnumber(J, 1 as ::core::ffi::c_int) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            js_setproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"length\0" as *const u8 as *const ::core::ffi::c_char,
            );
        } else {
            js_copy(J, 1 as ::core::ffi::c_int);
            js_setindex(J, -(2 as ::core::ffi::c_int), 0 as ::core::ffi::c_int);
        }
    } else {
        i = 1 as ::core::ffi::c_int;
        while i < top {
            js_copy(J, i);
            js_setindex(J, -(2 as ::core::ffi::c_int), i - 1 as ::core::ffi::c_int);
            i += 1;
        }
    };
}
unsafe extern "C" fn Ap_concat(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut n: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    js_newarray(J);
    n = 0 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < top {
        js_copy(J, i);
        if js_isarray(J, -(1 as ::core::ffi::c_int)) != 0 {
            len = js_getlength(J, -(1 as ::core::ffi::c_int));
            k = 0 as ::core::ffi::c_int;
            while k < len {
                if js_hasindex(J, -(1 as ::core::ffi::c_int), k) != 0 {
                    let fresh9 = n;
                    n = n + 1;
                    js_setindex(J, -(3 as ::core::ffi::c_int), fresh9);
                }
                k += 1;
            }
            js_pop(J, 1 as ::core::ffi::c_int);
        } else {
            let fresh10 = n;
            n = n + 1;
            js_setindex(J, -(2 as ::core::ffi::c_int), fresh10);
        }
        i += 1;
    }
}
unsafe extern "C" fn Ap_join_cycle(mut J: *mut js_State) -> ::core::ffi::c_int {
    let mut needle: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    let mut top: ::core::ffi::c_int = (*J).tracetop - 1 as ::core::ffi::c_int;
    while top > 0 as ::core::ffi::c_int {
        let mut stk: ::core::ffi::c_int = (*J).trace[top as usize].stack;
        let mut fun: *mut js_Value = (*J)
            .stack
            .offset((stk - 1 as ::core::ffi::c_int) as isize) as *mut js_Value;
        if (*fun).t.type_0 as ::core::ffi::c_int != JS_TOBJECT as ::core::ffi::c_int {
            return 0 as ::core::ffi::c_int;
        }
        if (*(*fun).u.object).type_0 as ::core::ffi::c_uint
            != JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            return 0 as ::core::ffi::c_int;
        }
        if (*(*fun).u.object).u.c.function
            == Some(Ap_join as unsafe extern "C" fn(*mut js_State) -> ())
        {
            let mut obj: *mut js_Value = (*J).stack.offset(stk as isize)
                as *mut js_Value;
            if (*obj).t.type_0 as ::core::ffi::c_int != JS_TOBJECT as ::core::ffi::c_int
            {
                return 0 as ::core::ffi::c_int;
            }
            if (*obj).u.object == needle {
                return 1 as ::core::ffi::c_int;
            }
        } else if (*(*fun).u.object).u.c.function
            == Some(Ap_toString as unsafe extern "C" fn(*mut js_State) -> ())
        {} else {
            return 0 as ::core::ffi::c_int
        }
        top -= 1;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn Ap_join(mut J: *mut js_State) {
    let mut out: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut r: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut sep: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut seplen: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    let mut rlen: ::core::ffi::c_int = 0;
    if Ap_join_cycle(J) != 0 {
        js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    if js_isdefined(J, 1 as ::core::ffi::c_int) != 0 {
        sep = js_tostring(J, 1 as ::core::ffi::c_int);
        seplen = strlen(sep) as ::core::ffi::c_int;
    } else {
        sep = b",\0" as *const u8 as *const ::core::ffi::c_char;
        seplen = 1 as ::core::ffi::c_int;
    }
    if len <= 0 as ::core::ffi::c_int {
        js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, out as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    n = 0 as ::core::ffi::c_int;
    k = 0 as ::core::ffi::c_int;
    while k < len {
        js_getindex(J, 0 as ::core::ffi::c_int, k);
        if js_iscoercible(J, -(1 as ::core::ffi::c_int)) != 0 {
            ::core::ptr::write_volatile(
                &mut r as *mut *const ::core::ffi::c_char,
                js_tostring(J, -(1 as ::core::ffi::c_int)),
            );
            rlen = strlen(r) as ::core::ffi::c_int;
        } else {
            rlen = 0 as ::core::ffi::c_int;
        }
        if k == 0 as ::core::ffi::c_int {
            ::core::ptr::write_volatile(
                &mut out as *mut *mut ::core::ffi::c_char,
                js_malloc(J, rlen + 1 as ::core::ffi::c_int) as *mut ::core::ffi::c_char,
            );
            if rlen > 0 as ::core::ffi::c_int {
                memcpy(
                    out as *mut ::core::ffi::c_void,
                    r as *const ::core::ffi::c_void,
                    rlen as size_t,
                );
                n += rlen;
            }
        } else {
            if n + seplen + rlen > JS_STRLIMIT {
                js_rangeerror(
                    J,
                    b"invalid string length\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            ::core::ptr::write_volatile(
                &mut out as *mut *mut ::core::ffi::c_char,
                js_realloc(
                    J,
                    out as *mut ::core::ffi::c_void,
                    n + seplen + rlen + 1 as ::core::ffi::c_int,
                ) as *mut ::core::ffi::c_char,
            );
            if seplen > 0 as ::core::ffi::c_int {
                memcpy(
                    out.offset(n as isize) as *mut ::core::ffi::c_void,
                    sep as *const ::core::ffi::c_void,
                    seplen as size_t,
                );
                n += seplen;
            }
            if rlen > 0 as ::core::ffi::c_int {
                memcpy(
                    out.offset(n as isize) as *mut ::core::ffi::c_void,
                    r as *const ::core::ffi::c_void,
                    rlen as size_t,
                );
                n += rlen;
            }
        }
        js_pop(J, 1 as ::core::ffi::c_int);
        k += 1;
    }
    js_pushlstring(J, out, n);
    js_endtry(J);
    js_free(J, out as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Ap_pop(mut J: *mut js_State) {
    let mut n: ::core::ffi::c_int = 0;
    n = js_getlength(J, 0 as ::core::ffi::c_int);
    if n > 0 as ::core::ffi::c_int {
        js_getindex(J, 0 as ::core::ffi::c_int, n - 1 as ::core::ffi::c_int);
        js_delindex(J, 0 as ::core::ffi::c_int, n - 1 as ::core::ffi::c_int);
        js_setlength(J, 0 as ::core::ffi::c_int, n - 1 as ::core::ffi::c_int);
    } else {
        js_setlength(J, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
        js_pushundefined(J);
    };
}
unsafe extern "C" fn Ap_push(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut n: ::core::ffi::c_int = 0;
    n = js_getlength(J, 0 as ::core::ffi::c_int);
    i = 1 as ::core::ffi::c_int;
    while i < top {
        js_copy(J, i);
        js_setindex(J, 0 as ::core::ffi::c_int, n);
        i += 1;
        n += 1;
    }
    js_setlength(J, 0 as ::core::ffi::c_int, n);
    js_pushnumber(J, n as ::core::ffi::c_double);
}
unsafe extern "C" fn Ap_reverse(mut J: *mut js_State) {
    let mut len: ::core::ffi::c_int = 0;
    let mut middle: ::core::ffi::c_int = 0;
    let mut lower: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    middle = len / 2 as ::core::ffi::c_int;
    lower = 0 as ::core::ffi::c_int;
    while lower != middle {
        let mut upper: ::core::ffi::c_int = len - lower - 1 as ::core::ffi::c_int;
        let mut haslower: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            lower,
        );
        let mut hasupper: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            upper,
        );
        if haslower != 0 && hasupper != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, lower);
            js_setindex(J, 0 as ::core::ffi::c_int, upper);
        } else if hasupper != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, lower);
            js_delindex(J, 0 as ::core::ffi::c_int, upper);
        } else if haslower != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, upper);
            js_delindex(J, 0 as ::core::ffi::c_int, lower);
        }
        lower += 1;
    }
    js_copy(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_shift(mut J: *mut js_State) {
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    if len == 0 as ::core::ffi::c_int {
        js_setlength(J, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
        js_pushundefined(J);
        return;
    }
    js_getindex(J, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
    k = 1 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, k - 1 as ::core::ffi::c_int);
        } else {
            js_delindex(J, 0 as ::core::ffi::c_int, k - 1 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_delindex(J, 0 as ::core::ffi::c_int, len - 1 as ::core::ffi::c_int);
    js_setlength(J, 0 as ::core::ffi::c_int, len - 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_slice(mut J: *mut js_State) {
    let mut len: ::core::ffi::c_int = 0;
    let mut s: ::core::ffi::c_int = 0;
    let mut e: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    let mut sv: ::core::ffi::c_double = 0.;
    let mut ev: ::core::ffi::c_double = 0.;
    js_newarray(J);
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    sv = js_tointeger(J, 1 as ::core::ffi::c_int) as ::core::ffi::c_double;
    ev = (if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        len
    }) as ::core::ffi::c_double;
    if sv < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        sv = sv + len as ::core::ffi::c_double;
    }
    if ev < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        ev = ev + len as ::core::ffi::c_double;
    }
    s = (if sv < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    } else if sv > len as ::core::ffi::c_double {
        len as ::core::ffi::c_double
    } else {
        sv
    }) as ::core::ffi::c_int;
    e = (if ev < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        0 as ::core::ffi::c_int as ::core::ffi::c_double
    } else if ev > len as ::core::ffi::c_double {
        len as ::core::ffi::c_double
    } else {
        ev
    }) as ::core::ffi::c_int;
    n = 0 as ::core::ffi::c_int;
    while s < e {
        if js_hasindex(J, 0 as ::core::ffi::c_int, s) != 0 {
            js_setindex(J, -(2 as ::core::ffi::c_int), n);
        }
        s += 1;
        n += 1;
    }
}
unsafe extern "C" fn Ap_sort_cmp(
    mut J: *mut js_State,
    mut idx_a: ::core::ffi::c_int,
    mut idx_b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut obj: *mut js_Object = (*js_tovalue(J, 0 as ::core::ffi::c_int)).u.object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let mut val_a: *mut js_Value = (*obj).u.a.array.offset(idx_a as isize)
            as *mut js_Value;
        let mut val_b: *mut js_Value = (*obj).u.a.array.offset(idx_b as isize)
            as *mut js_Value;
        let mut und_a: ::core::ffi::c_int = ((*val_a).t.type_0 as ::core::ffi::c_int
            == JS_TUNDEFINED as ::core::ffi::c_int) as ::core::ffi::c_int;
        let mut und_b: ::core::ffi::c_int = ((*val_b).t.type_0 as ::core::ffi::c_int
            == JS_TUNDEFINED as ::core::ffi::c_int) as ::core::ffi::c_int;
        if und_a != 0 {
            return und_b;
        }
        if und_b != 0 {
            return -(1 as ::core::ffi::c_int);
        }
        if js_iscallable(J, 1 as ::core::ffi::c_int) != 0 {
            let mut v: ::core::ffi::c_double = 0.;
            js_copy(J, 1 as ::core::ffi::c_int);
            js_pushundefined(J);
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            js_call(J, 2 as ::core::ffi::c_int);
            v = js_tonumber(J, -(1 as ::core::ffi::c_int));
            js_pop(J, 1 as ::core::ffi::c_int);
            if v.is_nan() as i32 != 0 {
                return 0 as ::core::ffi::c_int;
            }
            if v == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                return 0 as ::core::ffi::c_int;
            }
            return if v < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                -(1 as ::core::ffi::c_int)
            } else {
                1 as ::core::ffi::c_int
            };
        } else {
            let mut str_a: *const ::core::ffi::c_char = ::core::ptr::null::<
                ::core::ffi::c_char,
            >();
            let mut str_b: *const ::core::ffi::c_char = ::core::ptr::null::<
                ::core::ffi::c_char,
            >();
            let mut c: ::core::ffi::c_int = 0;
            js_pushvalue(J, *val_a);
            js_pushvalue(J, *val_b);
            str_a = js_tostring(J, -(2 as ::core::ffi::c_int));
            str_b = js_tostring(J, -(1 as ::core::ffi::c_int));
            c = strcmp(str_a, str_b);
            js_pop(J, 2 as ::core::ffi::c_int);
            return c;
        }
    } else {
        let mut und_a_0: ::core::ffi::c_int = 0;
        let mut und_b_0: ::core::ffi::c_int = 0;
        let mut has_a: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            idx_a,
        );
        let mut has_b: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            idx_b,
        );
        if has_a == 0 && has_b == 0 {
            return 0 as ::core::ffi::c_int;
        }
        if has_a != 0 && has_b == 0 {
            js_pop(J, 1 as ::core::ffi::c_int);
            return -(1 as ::core::ffi::c_int);
        }
        if has_a == 0 && has_b != 0 {
            js_pop(J, 1 as ::core::ffi::c_int);
            return 1 as ::core::ffi::c_int;
        }
        und_a_0 = js_isundefined(J, -(2 as ::core::ffi::c_int));
        und_b_0 = js_isundefined(J, -(1 as ::core::ffi::c_int));
        if und_a_0 != 0 {
            js_pop(J, 2 as ::core::ffi::c_int);
            return und_b_0;
        }
        if und_b_0 != 0 {
            js_pop(J, 2 as ::core::ffi::c_int);
            return -(1 as ::core::ffi::c_int);
        }
        if js_iscallable(J, 1 as ::core::ffi::c_int) != 0 {
            let mut v_0: ::core::ffi::c_double = 0.;
            js_copy(J, 1 as ::core::ffi::c_int);
            js_pushundefined(J);
            js_copy(J, -(4 as ::core::ffi::c_int));
            js_copy(J, -(4 as ::core::ffi::c_int));
            js_call(J, 2 as ::core::ffi::c_int);
            v_0 = js_tonumber(J, -(1 as ::core::ffi::c_int));
            js_pop(J, 3 as ::core::ffi::c_int);
            if v_0.is_nan() as i32 != 0 {
                return 0 as ::core::ffi::c_int;
            }
            if v_0 == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                return 0 as ::core::ffi::c_int;
            }
            return if v_0 < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                -(1 as ::core::ffi::c_int)
            } else {
                1 as ::core::ffi::c_int
            };
        } else {
            let mut str_a_0: *const ::core::ffi::c_char = js_tostring(
                J,
                -(2 as ::core::ffi::c_int),
            );
            let mut str_b_0: *const ::core::ffi::c_char = js_tostring(
                J,
                -(1 as ::core::ffi::c_int),
            );
            let mut c_0: ::core::ffi::c_int = strcmp(str_a_0, str_b_0);
            js_pop(J, 2 as ::core::ffi::c_int);
            return c_0;
        }
    };
}
unsafe extern "C" fn Ap_sort_swap(
    mut J: *mut js_State,
    mut idx_a: ::core::ffi::c_int,
    mut idx_b: ::core::ffi::c_int,
) {
    let mut obj: *mut js_Object = (*js_tovalue(J, 0 as ::core::ffi::c_int)).u.object;
    if (*obj).u.a.simple != 0 && idx_b < (*obj).u.a.flat_length {
        let mut tmp: js_Value = *(*obj).u.a.array.offset(idx_a as isize);
        *(*obj).u.a.array.offset(idx_a as isize) = *(*obj)
            .u
            .a
            .array
            .offset(idx_b as isize);
        *(*obj).u.a.array.offset(idx_b as isize) = tmp;
    } else {
        let mut has_a: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            idx_a,
        );
        let mut has_b: ::core::ffi::c_int = js_hasindex(
            J,
            0 as ::core::ffi::c_int,
            idx_b,
        );
        if has_a != 0 && has_b != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, idx_a);
            js_setindex(J, 0 as ::core::ffi::c_int, idx_b);
        } else if has_a != 0 && has_b == 0 {
            js_delindex(J, 0 as ::core::ffi::c_int, idx_a);
            js_setindex(J, 0 as ::core::ffi::c_int, idx_b);
        } else if has_a == 0 && has_b != 0 {
            js_delindex(J, 0 as ::core::ffi::c_int, idx_b);
            js_setindex(J, 0 as ::core::ffi::c_int, idx_a);
        }
    };
}
unsafe extern "C" fn Ap_sort_leaf(
    mut J: *mut js_State,
    mut i: ::core::ffi::c_int,
    mut end: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut j: ::core::ffi::c_int = i;
    let mut lc: ::core::ffi::c_int = (j << 1 as ::core::ffi::c_int)
        + 1 as ::core::ffi::c_int;
    let mut rc: ::core::ffi::c_int = (j << 1 as ::core::ffi::c_int)
        + 2 as ::core::ffi::c_int;
    while rc < end {
        if Ap_sort_cmp(J, lc, rc) <= 0 as ::core::ffi::c_int {
            j = rc;
        } else {
            j = lc;
        }
        lc = (j << 1 as ::core::ffi::c_int) + 1 as ::core::ffi::c_int;
        rc = (j << 1 as ::core::ffi::c_int) + 2 as ::core::ffi::c_int;
    }
    if lc < end {
        j = lc;
    }
    return j;
}
unsafe extern "C" fn Ap_sort_sift(
    mut J: *mut js_State,
    mut i: ::core::ffi::c_int,
    mut end: ::core::ffi::c_int,
) {
    let mut j: ::core::ffi::c_int = Ap_sort_leaf(J, i, end);
    while j > i && Ap_sort_cmp(J, i, j) > 0 as ::core::ffi::c_int {
        j = j - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int;
    }
    while j > i {
        Ap_sort_swap(J, i, j);
        j = j - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int;
    }
}
unsafe extern "C" fn Ap_sort_heapsort(mut J: *mut js_State, mut n: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0;
    i = n / 2 as ::core::ffi::c_int - 1 as ::core::ffi::c_int;
    while i >= 0 as ::core::ffi::c_int {
        Ap_sort_sift(J, i, n);
        i -= 1;
    }
    i = n - 1 as ::core::ffi::c_int;
    while i > 0 as ::core::ffi::c_int {
        Ap_sort_swap(J, 0 as ::core::ffi::c_int, i);
        Ap_sort_sift(J, 0 as ::core::ffi::c_int, i);
        i -= 1;
    }
}
unsafe extern "C" fn Ap_sort(mut J: *mut js_State) {
    let mut len: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    if len <= 1 as ::core::ffi::c_int {
        js_copy(J, 0 as ::core::ffi::c_int);
        return;
    }
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0
        && js_isundefined(J, 1 as ::core::ffi::c_int) == 0
    {
        js_typeerror(
            J,
            b"comparison function must be a function or undefined\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if len >= INT_MAX {
        js_rangeerror(
            J,
            b"array is too large to sort\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    Ap_sort_heapsort(J, len);
    js_copy(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_splice(mut J: *mut js_State) {
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut len: ::core::ffi::c_int = 0;
    let mut start: ::core::ffi::c_int = 0;
    let mut del: ::core::ffi::c_int = 0;
    let mut add: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    start = js_tointeger(J, 1 as ::core::ffi::c_int);
    if start < 0 as ::core::ffi::c_int {
        start = if len + start > 0 as ::core::ffi::c_int {
            len + start
        } else {
            0 as ::core::ffi::c_int
        };
    } else if start > len {
        start = len;
    }
    if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        del = js_tointeger(J, 2 as ::core::ffi::c_int);
    } else {
        del = len - start;
    }
    if del > len - start {
        del = len - start;
    }
    if del < 0 as ::core::ffi::c_int {
        del = 0 as ::core::ffi::c_int;
    }
    js_newarray(J);
    k = 0 as ::core::ffi::c_int;
    while k < del {
        if js_hasindex(J, 0 as ::core::ffi::c_int, start + k) != 0 {
            js_setindex(J, -(2 as ::core::ffi::c_int), k);
        }
        k += 1;
    }
    js_setlength(J, -(1 as ::core::ffi::c_int), del);
    add = top - 3 as ::core::ffi::c_int;
    if add < del {
        k = start;
        while k < len - del {
            if js_hasindex(J, 0 as ::core::ffi::c_int, k + del) != 0 {
                js_setindex(J, 0 as ::core::ffi::c_int, k + add);
            } else {
                js_delindex(J, 0 as ::core::ffi::c_int, k + add);
            }
            k += 1;
        }
        k = len;
        while k > len - del + add {
            js_delindex(J, 0 as ::core::ffi::c_int, k - 1 as ::core::ffi::c_int);
            k -= 1;
        }
    } else if add > del {
        k = len - del;
        while k > start {
            if js_hasindex(J, 0 as ::core::ffi::c_int, k + del - 1 as ::core::ffi::c_int)
                != 0
            {
                js_setindex(
                    J,
                    0 as ::core::ffi::c_int,
                    k + add - 1 as ::core::ffi::c_int,
                );
            } else {
                js_delindex(
                    J,
                    0 as ::core::ffi::c_int,
                    k + add - 1 as ::core::ffi::c_int,
                );
            }
            k -= 1;
        }
    }
    k = 0 as ::core::ffi::c_int;
    while k < add {
        js_copy(J, 3 as ::core::ffi::c_int + k);
        js_setindex(J, 0 as ::core::ffi::c_int, start + k);
        k += 1;
    }
    js_setlength(J, 0 as ::core::ffi::c_int, len - del + add);
}
unsafe extern "C" fn Ap_unshift(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = len;
    while k > 0 as ::core::ffi::c_int {
        let mut from: ::core::ffi::c_int = k - 1 as ::core::ffi::c_int;
        let mut to: ::core::ffi::c_int = k + top - 2 as ::core::ffi::c_int;
        if js_hasindex(J, 0 as ::core::ffi::c_int, from) != 0 {
            js_setindex(J, 0 as ::core::ffi::c_int, to);
        } else {
            js_delindex(J, 0 as ::core::ffi::c_int, to);
        }
        k -= 1;
    }
    i = 1 as ::core::ffi::c_int;
    while i < top {
        js_copy(J, i);
        js_setindex(J, 0 as ::core::ffi::c_int, i - 1 as ::core::ffi::c_int);
        i += 1;
    }
    js_setlength(J, 0 as ::core::ffi::c_int, len + top - 1 as ::core::ffi::c_int);
    js_pushnumber(J, (len + top - 1 as ::core::ffi::c_int) as ::core::ffi::c_double);
}
unsafe extern "C" fn Ap_toString(mut J: *mut js_State) {
    if js_iscoercible(J, 0 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"'this' is not an object\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    js_getproperty(
        J,
        0 as ::core::ffi::c_int,
        b"join\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_iscallable(J, -(1 as ::core::ffi::c_int)) == 0 {
        js_pop(J, 1 as ::core::ffi::c_int);
        js_getglobal(J, b"Object\0" as *const u8 as *const ::core::ffi::c_char);
        js_getproperty(
            J,
            -(1 as ::core::ffi::c_int),
            b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
        );
        js_rot2pop1(J);
        js_getproperty(
            J,
            -(1 as ::core::ffi::c_int),
            b"toString\0" as *const u8 as *const ::core::ffi::c_char,
        );
        js_rot2pop1(J);
    }
    js_copy(J, 0 as ::core::ffi::c_int);
    js_call(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_indexOf(mut J: *mut js_State) {
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    let mut from: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    from = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        0 as ::core::ffi::c_int
    };
    if from < 0 as ::core::ffi::c_int {
        from = len + from;
    }
    if from < 0 as ::core::ffi::c_int {
        from = 0 as ::core::ffi::c_int;
    }
    js_copy(J, 1 as ::core::ffi::c_int);
    k = from;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            if js_strictequal(J) != 0 {
                js_pushnumber(J, k as ::core::ffi::c_double);
                return;
            }
            js_pop(J, 1 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_pushnumber(J, -(1 as ::core::ffi::c_int) as ::core::ffi::c_double);
}
unsafe extern "C" fn Ap_lastIndexOf(mut J: *mut js_State) {
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    let mut from: ::core::ffi::c_int = 0;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    from = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        len - 1 as ::core::ffi::c_int
    };
    if from > len - 1 as ::core::ffi::c_int {
        from = len - 1 as ::core::ffi::c_int;
    }
    if from < 0 as ::core::ffi::c_int {
        from = len + from;
    }
    js_copy(J, 1 as ::core::ffi::c_int);
    k = from;
    while k >= 0 as ::core::ffi::c_int {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            if js_strictequal(J) != 0 {
                js_pushnumber(J, k as ::core::ffi::c_double);
                return;
            }
            js_pop(J, 1 as ::core::ffi::c_int);
        }
        k -= 1;
    }
    js_pushnumber(J, -(1 as ::core::ffi::c_int) as ::core::ffi::c_double);
}
unsafe extern "C" fn Ap_every(mut J: *mut js_State) {
    let mut hasthis: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            if hasthis != 0 {
                js_copy(J, 2 as ::core::ffi::c_int);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -(3 as ::core::ffi::c_int));
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 3 as ::core::ffi::c_int);
            if js_toboolean(J, -(1 as ::core::ffi::c_int)) == 0 {
                return;
            }
            js_pop(J, 2 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_pushboolean(J, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_some(mut J: *mut js_State) {
    let mut hasthis: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            if hasthis != 0 {
                js_copy(J, 2 as ::core::ffi::c_int);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -(3 as ::core::ffi::c_int));
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 3 as ::core::ffi::c_int);
            if js_toboolean(J, -(1 as ::core::ffi::c_int)) != 0 {
                return;
            }
            js_pop(J, 2 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_pushboolean(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn Ap_forEach(mut J: *mut js_State) {
    let mut hasthis: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            if hasthis != 0 {
                js_copy(J, 2 as ::core::ffi::c_int);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -(3 as ::core::ffi::c_int));
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 3 as ::core::ffi::c_int);
            js_pop(J, 2 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_pushundefined(J);
}
unsafe extern "C" fn Ap_map(mut J: *mut js_State) {
    let mut hasthis: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    js_newarray(J);
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            if hasthis != 0 {
                js_copy(J, 2 as ::core::ffi::c_int);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -(3 as ::core::ffi::c_int));
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 3 as ::core::ffi::c_int);
            js_setindex(J, -(3 as ::core::ffi::c_int), k);
            js_pop(J, 1 as ::core::ffi::c_int);
        }
        k += 1;
    }
    js_setlength(J, -(1 as ::core::ffi::c_int), len);
}
unsafe extern "C" fn Ap_filter(mut J: *mut js_State) {
    let mut hasthis: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut to: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    js_newarray(J);
    to = 0 as ::core::ffi::c_int;
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            if hasthis != 0 {
                js_copy(J, 2 as ::core::ffi::c_int);
            } else {
                js_pushundefined(J);
            }
            js_copy(J, -(3 as ::core::ffi::c_int));
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 3 as ::core::ffi::c_int);
            if js_toboolean(J, -(1 as ::core::ffi::c_int)) != 0 {
                js_pop(J, 1 as ::core::ffi::c_int);
                let fresh8 = to;
                to = to + 1;
                js_setindex(J, -(2 as ::core::ffi::c_int), fresh8);
            } else {
                js_pop(J, 2 as ::core::ffi::c_int);
            }
        }
        k += 1;
    }
}
unsafe extern "C" fn Ap_reduce(mut J: *mut js_State) {
    let mut hasinitial: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = 0 as ::core::ffi::c_int;
    if len == 0 as ::core::ffi::c_int && hasinitial == 0 {
        js_typeerror(
            J,
            b"no initial value\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if hasinitial != 0 {
        js_copy(J, 2 as ::core::ffi::c_int);
    } else {
        while k < len {
            let fresh7 = k;
            k = k + 1;
            if js_hasindex(J, 0 as ::core::ffi::c_int, fresh7) != 0 {
                break;
            }
        }
        if k == len {
            js_typeerror(
                J,
                b"no initial value\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    }
    while k < len {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            js_pushundefined(J);
            js_rot(J, 4 as ::core::ffi::c_int);
            js_rot(J, 4 as ::core::ffi::c_int);
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 4 as ::core::ffi::c_int);
        }
        k += 1;
    }
}
unsafe extern "C" fn Ap_reduceRight(mut J: *mut js_State) {
    let mut hasinitial: ::core::ffi::c_int = (js_gettop(J) >= 3 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    if js_iscallable(J, 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"callback is not a function\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    len = js_getlength(J, 0 as ::core::ffi::c_int);
    k = len - 1 as ::core::ffi::c_int;
    if len == 0 as ::core::ffi::c_int && hasinitial == 0 {
        js_typeerror(
            J,
            b"no initial value\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if hasinitial != 0 {
        js_copy(J, 2 as ::core::ffi::c_int);
    } else {
        while k >= 0 as ::core::ffi::c_int {
            let fresh6 = k;
            k = k - 1;
            if js_hasindex(J, 0 as ::core::ffi::c_int, fresh6) != 0 {
                break;
            }
        }
        if k < 0 as ::core::ffi::c_int {
            js_typeerror(
                J,
                b"no initial value\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    }
    while k >= 0 as ::core::ffi::c_int {
        if js_hasindex(J, 0 as ::core::ffi::c_int, k) != 0 {
            js_copy(J, 1 as ::core::ffi::c_int);
            js_pushundefined(J);
            js_rot(J, 4 as ::core::ffi::c_int);
            js_rot(J, 4 as ::core::ffi::c_int);
            js_pushnumber(J, k as ::core::ffi::c_double);
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 4 as ::core::ffi::c_int);
        }
        k -= 1;
    }
}
unsafe extern "C" fn A_isArray(mut J: *mut js_State) {
    if js_isobject(J, 1 as ::core::ffi::c_int) != 0 {
        let mut T: *mut js_Object = js_toobject(J, 1 as ::core::ffi::c_int);
        js_pushboolean(
            J,
            ((*T).type_0 as ::core::ffi::c_uint
                == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint)
                as ::core::ffi::c_int,
        );
    } else {
        js_pushboolean(J, 0 as ::core::ffi::c_int);
    };
}
#[no_mangle]
pub unsafe extern "C" fn jsB_initarray(mut J: *mut js_State) {
    js_pushobject(J, (*J).Array_prototype);
    jsB_propf(
        J,
        b"Array.prototype.toString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.concat\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_concat as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.join\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_join as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.pop\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_pop as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.push\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_push as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.reverse\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_reverse as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.shift\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_shift as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.slice\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_slice as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.sort\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_sort as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.splice\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_splice as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.unshift\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_unshift as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.indexOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_indexOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.lastIndexOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_lastIndexOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.every\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_every as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.some\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_some as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.forEach\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_forEach as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.map\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_map as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.filter\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_filter as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.reduce\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_reduce as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.prototype.reduceRight\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Ap_reduceRight as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    js_newcconstructor(
        J,
        Some(jsB_new_Array as unsafe extern "C" fn(*mut js_State) -> ()),
        Some(jsB_new_Array as unsafe extern "C" fn(*mut js_State) -> ()),
        b"Array\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"Array.isArray\0" as *const u8 as *const ::core::ffi::c_char,
        Some(A_isArray as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    js_defglobal(
        J,
        b"Array\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
