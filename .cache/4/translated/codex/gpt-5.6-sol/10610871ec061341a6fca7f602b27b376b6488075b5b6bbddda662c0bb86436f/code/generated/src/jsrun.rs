extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn longjmp(__env: *mut __jmp_buf_tag, __val: ::core::ffi::c_int) -> !;
    fn js_gc(J: *mut js_State, report: ::core::ffi::c_int);
    fn js_error(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_rangeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_referenceerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_getlength(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_setlength(J: *mut js_State, idx: ::core::ffi::c_int, len: ::core::ffi::c_int);
    fn js_newobject(J: *mut js_State);
    fn js_newarray(J: *mut js_State);
    fn js_newregexp(
        J: *mut js_State,
        pattern: *const ::core::ffi::c_char,
        flags: ::core::ffi::c_int,
    );
    fn js_concat(J: *mut js_State);
    fn js_compare(J: *mut js_State, okay: *mut ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_equal(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_strictequal(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_instanceof(J: *mut js_State) -> ::core::ffi::c_int;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
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
    fn abort() -> !;
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
    fn jsU_runetochar(
        str: *mut ::core::ffi::c_char,
        rune: *const Rune,
    ) -> ::core::ffi::c_int;
    fn fmod(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn js_intern(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    fn js_newarguments(J: *mut js_State);
    fn js_newfunction(
        J: *mut js_State,
        function: *mut js_Function,
        scope: *mut js_Environment,
    );
    fn js_loadeval(
        J: *mut js_State,
        filename: *const ::core::ffi::c_char,
        source: *const ::core::ffi::c_char,
    );
    fn js_runeat(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
        i: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn jsV_toboolean(J: *mut js_State, v: *mut js_Value) -> ::core::ffi::c_int;
    fn jsV_tonumber(J: *mut js_State, v: *mut js_Value) -> ::core::ffi::c_double;
    fn jsV_tointeger(J: *mut js_State, v: *mut js_Value) -> ::core::ffi::c_double;
    fn jsV_tostring(J: *mut js_State, v: *mut js_Value) -> *const ::core::ffi::c_char;
    fn jsV_toobject(J: *mut js_State, v: *mut js_Value) -> *mut js_Object;
    fn jsV_toprimitive(
        J: *mut js_State,
        v: *mut js_Value,
        preferred: ::core::ffi::c_int,
    );
    fn js_itoa(
        buf: *mut ::core::ffi::c_char,
        a: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn jsV_numbertointeger(n: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn jsV_numbertoint32(n: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn jsV_numbertouint32(n: ::core::ffi::c_double) -> ::core::ffi::c_uint;
    fn jsV_numbertoint16(n: ::core::ffi::c_double) -> ::core::ffi::c_short;
    fn jsV_numbertouint16(n: ::core::ffi::c_double) -> ::core::ffi::c_ushort;
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
    fn jsV_getpropertyx(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
        own: *mut ::core::ffi::c_int,
    ) -> *mut js_Property;
    fn jsV_getproperty(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
    ) -> *mut js_Property;
    fn jsV_setproperty(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
    ) -> *mut js_Property;
    fn jsV_delproperty(
        J: *mut js_State,
        obj: *mut js_Object,
        name: *const ::core::ffi::c_char,
    );
    fn jsV_newiterator(
        J: *mut js_State,
        obj: *mut js_Object,
        own: ::core::ffi::c_int,
    ) -> *mut js_Object;
    fn jsV_nextiterator(
        J: *mut js_State,
        iter: *mut js_Object,
    ) -> *const ::core::ffi::c_char;
    fn jsV_resizearray(
        J: *mut js_State,
        obj: *mut js_Object,
        newlen: ::core::ffi::c_int,
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
pub const JS_TUNDEFINED: js_Type = 1;
pub const JS_TLITSTR: js_Type = 5;
pub const OP_RETURN: js_OpCode = 84;
pub const OP_JFALSE: js_OpCode = 83;
pub const OP_JTRUE: js_OpCode = 82;
pub const OP_JUMP: js_OpCode = 81;
pub type FILE = _IO_FILE;
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
pub type size_t = usize;
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
pub type __off_t = ::core::ffi::c_long;
pub const JS_TOBJECT: js_Type = 7;
pub const JS_TMEMSTR: js_Type = 6;
pub const JS_TSHRSTR: js_Type = 0;
pub const JS_TNUMBER: js_Type = 4;
pub const JS_TBOOLEAN: js_Type = 3;
pub const JS_TNULL: js_Type = 2;
pub const OP_DEBUGGER: js_OpCode = 80;
pub const OP_ENDWITH: js_OpCode = 79;
pub const OP_WITH: js_OpCode = 78;
pub const OP_ENDCATCH: js_OpCode = 77;
pub const JS_READONLY: C2RustUnnamed_10 = 1;
pub const OP_CATCH: js_OpCode = 76;
pub const OP_ENDTRY: js_OpCode = 75;
pub const OP_TRY: js_OpCode = 74;
pub const OP_THROW: js_OpCode = 73;
pub const OP_BITOR: js_OpCode = 71;
pub const OP_BITXOR: js_OpCode = 70;
pub const OP_BITAND: js_OpCode = 69;
pub const OP_JCASE: js_OpCode = 68;
pub const OP_STRICTNE: js_OpCode = 67;
pub const OP_STRICTEQ: js_OpCode = 66;
pub const OP_NE: js_OpCode = 65;
pub const OP_EQ: js_OpCode = 64;
pub const OP_INSTANCEOF: js_OpCode = 72;
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
pub const JS_REGEXP_M: C2RustUnnamed_9 = 4;
pub const JS_REGEXP_I: C2RustUnnamed_9 = 2;
pub const JS_REGEXP_G: C2RustUnnamed_9 = 1;
pub type Rune = ::core::ffi::c_int;
pub const OP_NEW: js_OpCode = 42;
pub const OP_CALL: js_OpCode = 41;
pub type js_Type = ::core::ffi::c_uint;
pub const OP_EVAL: js_OpCode = 40;
pub const OP_NEXTITER: js_OpCode = 39;
pub const OP_ITERATOR: js_OpCode = 38;
pub const JS_DONTCONF: C2RustUnnamed_10 = 4;
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
pub const OP_HASVAR: js_OpCode = 22;
pub const OP_GETVAR: js_OpCode = 23;
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
pub const OP_NEWARRAY: js_OpCode = 10;
pub const OP_NEWOBJECT: js_OpCode = 11;
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
pub type js_OpCode = ::core::ffi::c_uint;
pub const JS_DONTENUM: C2RustUnnamed_10 = 2;
pub type C2RustUnnamed_9 = ::core::ffi::c_uint;
pub type C2RustUnnamed_10 = ::core::ffi::c_uint;
pub type C2RustUnnamed_11 = ::core::ffi::c_uint;
pub const JS_ISOBJECT: C2RustUnnamed_11 = 6;
pub const JS_ISFUNCTION: C2RustUnnamed_11 = 5;
pub const JS_ISSTRING: C2RustUnnamed_11 = 4;
pub const JS_ISNUMBER: C2RustUnnamed_11 = 3;
pub const JS_ISBOOLEAN: C2RustUnnamed_11 = 2;
pub const JS_ISNULL: C2RustUnnamed_11 = 1;
pub const JS_ISUNDEFINED: C2RustUnnamed_11 = 0;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
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
        let fresh38 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh38 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh36 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh36 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh37 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh37 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
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
        let fresh39 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh39 = __c as ::core::ffi::c_char;
        *fresh39 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh40 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh40 = __c as ::core::ffi::c_char;
        *fresh40 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh41 = (*stdout)._IO_write_ptr;
        (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
        *fresh41 = __c as ::core::ffi::c_char;
        *fresh41 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
pub const JS_STACKSIZE: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const JS_ENVLIMIT: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
pub const JS_TRYLIMIT: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const JS_ARRAYLIMIT: ::core::ffi::c_int = (1 as ::core::ffi::c_int)
    << 26 as ::core::ffi::c_int;
pub const JS_STRLIMIT: ::core::ffi::c_int = (1 as ::core::ffi::c_int)
    << 28 as ::core::ffi::c_int;
unsafe extern "C" fn js_trystackoverflow(mut J: *mut js_State) {
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TLITSTR as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh22 = (*(*J).stack.offset((*J).top as isize)).u.litstr;
    *fresh22 = b"exception stack overflow\0" as *const u8 as *const ::core::ffi::c_char;
    (*J).top += 1;
    js_throw(J);
}
unsafe extern "C" fn js_stackoverflow(mut J: *mut js_State) {
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TLITSTR as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh0 = (*(*J).stack.offset((*J).top as isize)).u.litstr;
    *fresh0 = b"stack overflow\0" as *const u8 as *const ::core::ffi::c_char;
    (*J).top += 1;
    js_throw(J);
}
unsafe extern "C" fn js_outofmemory(mut J: *mut js_State) {
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TLITSTR as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh18 = (*(*J).stack.offset((*J).top as isize)).u.litstr;
    *fresh18 = b"out of memory\0" as *const u8 as *const ::core::ffi::c_char;
    (*J).top += 1;
    js_throw(J);
}
unsafe extern "C" fn js_runlimit(mut J: *mut js_State) {
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TLITSTR as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh29 = (*(*J).stack.offset((*J).top as isize)).u.litstr;
    *fresh29 = b"script ran too long\0" as *const u8 as *const ::core::ffi::c_char;
    (*J).top += 1;
    js_throw(J);
}
#[no_mangle]
pub unsafe extern "C" fn js_setlimit(
    mut J: *mut js_State,
    mut runlimit: ::core::ffi::c_int,
    mut memlimit: ::core::ffi::c_int,
) {
    (*J).runlimit = runlimit;
    (*J).memlimit = memlimit;
}
#[no_mangle]
pub unsafe extern "C" fn js_malloc(
    mut J: *mut js_State,
    mut size: ::core::ffi::c_int,
) -> *mut ::core::ffi::c_void {
    let mut ptr: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
        ::core::ffi::c_void,
    >();
    if (*J).memlimit > 0 as ::core::ffi::c_int {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = (*J).alloc.expect("non-null function pointer")((*J).actx, NULL, size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    return ptr;
}
#[no_mangle]
pub unsafe extern "C" fn js_realloc(
    mut J: *mut js_State,
    mut ptr: *mut ::core::ffi::c_void,
    mut size: ::core::ffi::c_int,
) -> *mut ::core::ffi::c_void {
    if (*J).memlimit > 0 as ::core::ffi::c_int {
        if size >= (*J).memlimit {
            js_outofmemory(J);
        }
        (*J).memlimit -= size;
    }
    ptr = (*J).alloc.expect("non-null function pointer")((*J).actx, ptr, size);
    if ptr.is_null() {
        js_outofmemory(J);
    }
    return ptr;
}
#[no_mangle]
pub unsafe extern "C" fn js_strdup(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut n: ::core::ffi::c_int = strlen(s).wrapping_add(1 as size_t)
        as ::core::ffi::c_int;
    let mut p: *mut ::core::ffi::c_char = js_malloc(J, n) as *mut ::core::ffi::c_char;
    memcpy(p as *mut ::core::ffi::c_void, s as *const ::core::ffi::c_void, n as size_t);
    return p;
}
#[no_mangle]
pub unsafe extern "C" fn js_free(
    mut J: *mut js_State,
    mut ptr: *mut ::core::ffi::c_void,
) {
    (*J)
        .alloc
        .expect("non-null function pointer")((*J).actx, ptr, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn jsV_newmemstring(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
    mut n: ::core::ffi::c_int,
) -> *mut js_String {
    let mut v: *mut js_String = js_malloc(
        J,
        9 as ::core::ffi::c_ulong as ::core::ffi::c_int + n + 1 as ::core::ffi::c_int,
    ) as *mut js_String;
    memcpy(
        &raw mut (*v).p as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        s as *const ::core::ffi::c_void,
        n as size_t,
    );
    *(&raw mut (*v).p as *mut ::core::ffi::c_char).offset(n as isize) = 0
        as ::core::ffi::c_char;
    (*v).gcmark = 0 as ::core::ffi::c_char;
    (*v).gcnext = (*J).gcstr;
    (*J).gcstr = v;
    (*J).gccounter = (*J).gccounter.wrapping_add(1);
    return v;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushvalue(mut J: *mut js_State, mut v: js_Value) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    *(*J).stack.offset((*J).top as isize) = v;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushundefined(mut J: *mut js_State) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TUNDEFINED
        as ::core::ffi::c_int as ::core::ffi::c_char;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushnull(mut J: *mut js_State) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TNULL as ::core::ffi::c_int
        as ::core::ffi::c_char;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushboolean(
    mut J: *mut js_State,
    mut v: ::core::ffi::c_int,
) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TBOOLEAN as ::core::ffi::c_int
        as ::core::ffi::c_char;
    (*(*J).stack.offset((*J).top as isize)).u.boolean = (v != 0) as ::core::ffi::c_int;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushnumber(
    mut J: *mut js_State,
    mut v: ::core::ffi::c_double,
) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TNUMBER as ::core::ffi::c_int
        as ::core::ffi::c_char;
    (*(*J).stack.offset((*J).top as isize)).u.number = v;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushstring(
    mut J: *mut js_State,
    mut v: *const ::core::ffi::c_char,
) {
    let mut n: size_t = strlen(v);
    if n > JS_STRLIMIT as size_t {
        js_rangeerror(
            J,
            b"invalid string length\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    if n <= 15 as ::core::ffi::c_ulong as ::core::ffi::c_int as size_t {
        let mut s: *mut ::core::ffi::c_char = &raw mut (*(*J)
            .stack
            .offset((*J).top as isize))
            .u
            .shrstr as *mut ::core::ffi::c_char;
        loop {
            let fresh25 = n;
            n = n.wrapping_sub(1);
            if !(fresh25 != 0) {
                break;
            }
            let fresh26 = v;
            v = v.offset(1);
            let fresh27 = s;
            s = s.offset(1);
            *fresh27 = *fresh26;
        }
        *s = 0 as ::core::ffi::c_char;
        (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TSHRSTR
            as ::core::ffi::c_int as ::core::ffi::c_char;
    } else {
        (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TMEMSTR
            as ::core::ffi::c_int as ::core::ffi::c_char;
        let ref mut fresh28 = (*(*J).stack.offset((*J).top as isize)).u.memstr;
        *fresh28 = jsV_newmemstring(J, v, n as ::core::ffi::c_int);
    }
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushlstring(
    mut J: *mut js_State,
    mut v: *const ::core::ffi::c_char,
    mut n: ::core::ffi::c_int,
) {
    if n > JS_STRLIMIT {
        js_rangeerror(
            J,
            b"invalid string length\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    if n <= 15 as ::core::ffi::c_ulong as ::core::ffi::c_int {
        let mut s: *mut ::core::ffi::c_char = &raw mut (*(*J)
            .stack
            .offset((*J).top as isize))
            .u
            .shrstr as *mut ::core::ffi::c_char;
        loop {
            let fresh32 = n;
            n = n - 1;
            if !(fresh32 != 0) {
                break;
            }
            let fresh33 = v;
            v = v.offset(1);
            let fresh34 = s;
            s = s.offset(1);
            *fresh34 = *fresh33;
        }
        *s = 0 as ::core::ffi::c_char;
        (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TSHRSTR
            as ::core::ffi::c_int as ::core::ffi::c_char;
    } else {
        (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TMEMSTR
            as ::core::ffi::c_int as ::core::ffi::c_char;
        let ref mut fresh35 = (*(*J).stack.offset((*J).top as isize)).u.memstr;
        *fresh35 = jsV_newmemstring(J, v, n);
    }
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushliteral(
    mut J: *mut js_State,
    mut v: *const ::core::ffi::c_char,
) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TLITSTR as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh24 = (*(*J).stack.offset((*J).top as isize)).u.litstr;
    *fresh24 = v;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushobject(mut J: *mut js_State, mut v: *mut js_Object) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TOBJECT as ::core::ffi::c_int
        as ::core::ffi::c_char;
    let ref mut fresh19 = (*(*J).stack.offset((*J).top as isize)).u.object;
    *fresh19 = v;
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_pushglobal(mut J: *mut js_State) {
    js_pushobject(J, (*J).G);
}
#[no_mangle]
pub unsafe extern "C" fn js_currentfunction(mut J: *mut js_State) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    if (*J).bot > 0 as ::core::ffi::c_int {
        *(*J).stack.offset((*J).top as isize) = *(*J)
            .stack
            .offset(((*J).bot - 1 as ::core::ffi::c_int) as isize);
    } else {
        (*(*J).stack.offset((*J).top as isize)).t.type_0 = JS_TUNDEFINED
            as ::core::ffi::c_int as ::core::ffi::c_char;
    }
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_currentfunctiondata(
    mut J: *mut js_State,
) -> *mut ::core::ffi::c_void {
    if (*J).bot > 0 as ::core::ffi::c_int {
        return (*(*(*J).stack.offset(((*J).bot - 1 as ::core::ffi::c_int) as isize))
            .u
            .object)
            .u
            .c
            .data;
    }
    return NULL;
}
unsafe extern "C" fn stackidx(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *mut js_Value {
    static mut undefined: js_Value = js_Value {
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
    };
    idx = if idx < 0 as ::core::ffi::c_int { (*J).top + idx } else { (*J).bot + idx };
    if idx < 0 as ::core::ffi::c_int || idx >= (*J).top {
        return &raw mut undefined;
    }
    return (*J).stack.offset(idx as isize);
}
#[no_mangle]
pub unsafe extern "C" fn js_tovalue(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *mut js_Value {
    return stackidx(J, idx);
}
#[no_mangle]
pub unsafe extern "C" fn js_isdefined(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        != JS_TUNDEFINED as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isundefined(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        == JS_TUNDEFINED as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isnull(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        == JS_TNULL as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isboolean(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        == JS_TBOOLEAN as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isnumber(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        == JS_TNUMBER as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isstring(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut t: js_Type = (*stackidx(J, idx)).t.type_0 as js_Type;
    return (t as ::core::ffi::c_uint
        == JS_TSHRSTR as ::core::ffi::c_int as ::core::ffi::c_uint
        || t as ::core::ffi::c_uint
            == JS_TLITSTR as ::core::ffi::c_int as ::core::ffi::c_uint
        || t as ::core::ffi::c_uint
            == JS_TMEMSTR as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isprimitive(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        != JS_TOBJECT as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isobject(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*stackidx(J, idx)).t.type_0 as ::core::ffi::c_int
        == JS_TOBJECT as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_iscoercible(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    return ((*v).t.type_0 as ::core::ffi::c_int != JS_TUNDEFINED as ::core::ffi::c_int
        && (*v).t.type_0 as ::core::ffi::c_int != JS_TNULL as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_iscallable(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int {
        return ((*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*(*v).u.object).type_0 as ::core::ffi::c_uint
                == JS_CSCRIPT as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*(*v).u.object).type_0 as ::core::ffi::c_uint
                == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint)
            as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isarray(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    return ((*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isregexp(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    return ((*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_isuserdata(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut tag: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return (strcmp(tag, (*(*v).u.object).u.user.tag) == 0) as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_iserror(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    return ((*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CERROR as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_typeof(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    let mut v: *mut js_Value = stackidx(J, idx);
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => return b"undefined\0" as *const u8 as *const ::core::ffi::c_char,
        2 => return b"object\0" as *const u8 as *const ::core::ffi::c_char,
        3 => return b"boolean\0" as *const u8 as *const ::core::ffi::c_char,
        4 => return b"number\0" as *const u8 as *const ::core::ffi::c_char,
        5 => return b"string\0" as *const u8 as *const ::core::ffi::c_char,
        6 => return b"string\0" as *const u8 as *const ::core::ffi::c_char,
        7 => {
            if (*(*v).u.object).type_0 as ::core::ffi::c_uint
                == JS_CFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
                || (*(*v).u.object).type_0 as ::core::ffi::c_uint
                    == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                return b"function\0" as *const u8 as *const ::core::ffi::c_char;
            }
            return b"object\0" as *const u8 as *const ::core::ffi::c_char;
        }
        0 | _ => return b"string\0" as *const u8 as *const ::core::ffi::c_char,
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_type(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => return JS_ISUNDEFINED as ::core::ffi::c_int,
        2 => return JS_ISNULL as ::core::ffi::c_int,
        3 => return JS_ISBOOLEAN as ::core::ffi::c_int,
        4 => return JS_ISNUMBER as ::core::ffi::c_int,
        5 => return JS_ISSTRING as ::core::ffi::c_int,
        6 => return JS_ISSTRING as ::core::ffi::c_int,
        7 => {
            if (*(*v).u.object).type_0 as ::core::ffi::c_uint
                == JS_CFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
                || (*(*v).u.object).type_0 as ::core::ffi::c_uint
                    == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                return JS_ISFUNCTION as ::core::ffi::c_int;
            }
            return JS_ISOBJECT as ::core::ffi::c_int;
        }
        0 | _ => return JS_ISSTRING as ::core::ffi::c_int,
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_toboolean(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return jsV_toboolean(J, stackidx(J, idx));
}
#[no_mangle]
pub unsafe extern "C" fn js_tonumber(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_double {
    return jsV_tonumber(J, stackidx(J, idx));
}
#[no_mangle]
pub unsafe extern "C" fn js_tointeger(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return jsV_numbertointeger(jsV_tonumber(J, stackidx(J, idx)));
}
#[no_mangle]
pub unsafe extern "C" fn js_toint32(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return jsV_numbertoint32(jsV_tonumber(J, stackidx(J, idx)));
}
#[no_mangle]
pub unsafe extern "C" fn js_touint32(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    return jsV_numbertouint32(jsV_tonumber(J, stackidx(J, idx)));
}
#[no_mangle]
pub unsafe extern "C" fn js_toint16(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_short {
    return jsV_numbertoint16(jsV_tonumber(J, stackidx(J, idx)));
}
#[no_mangle]
pub unsafe extern "C" fn js_touint16(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> ::core::ffi::c_ushort {
    return jsV_numbertouint16(jsV_tonumber(J, stackidx(J, idx)));
}
#[no_mangle]
pub unsafe extern "C" fn js_tostring(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    return jsV_tostring(J, stackidx(J, idx));
}
#[no_mangle]
pub unsafe extern "C" fn js_toobject(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *mut js_Object {
    return jsV_toobject(J, stackidx(J, idx));
}
#[no_mangle]
pub unsafe extern "C" fn js_toprimitive(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut hint: ::core::ffi::c_int,
) {
    jsV_toprimitive(J, stackidx(J, idx), hint);
}
#[no_mangle]
pub unsafe extern "C" fn js_toregexp(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *mut js_Regexp {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return &raw mut (*(*v).u.object).u.r;
    }
    js_typeerror(J, b"not a regexp\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn js_touserdata(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut tag: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_void {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int
        && (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(tag, (*(*v).u.object).u.user.tag) == 0 {
            return (*(*v).u.object).u.user.data;
        }
    }
    js_typeerror(J, b"not a %s\0" as *const u8 as *const ::core::ffi::c_char, tag);
}
unsafe extern "C" fn jsR_tofunction(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *mut js_Object {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TUNDEFINED as ::core::ffi::c_int
        || (*v).t.type_0 as ::core::ffi::c_int == JS_TNULL as ::core::ffi::c_int
    {
        return ::core::ptr::null_mut::<js_Object>();
    }
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TOBJECT as ::core::ffi::c_int {
        if (*(*v).u.object).type_0 as ::core::ffi::c_uint
            == JS_CFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
            || (*(*v).u.object).type_0 as ::core::ffi::c_uint
                == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            return (*v).u.object;
        }
    }
    js_typeerror(J, b"not a function\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn js_gettop(mut J: *mut js_State) -> ::core::ffi::c_int {
    return (*J).top - (*J).bot;
}
#[no_mangle]
pub unsafe extern "C" fn js_pop(mut J: *mut js_State, mut n: ::core::ffi::c_int) {
    (*J).top -= n;
    if (*J).top < (*J).bot {
        (*J).top = (*J).bot;
        js_error(J, b"stack underflow!\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
#[no_mangle]
pub unsafe extern "C" fn js_remove(mut J: *mut js_State, mut idx: ::core::ffi::c_int) {
    idx = if idx < 0 as ::core::ffi::c_int { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        js_error(J, b"stack error!\0" as *const u8 as *const ::core::ffi::c_char);
    }
    while idx < (*J).top - 1 as ::core::ffi::c_int {
        *(*J).stack.offset(idx as isize) = *(*J)
            .stack
            .offset((idx + 1 as ::core::ffi::c_int) as isize);
        idx += 1;
    }
    (*J).top -= 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_insert(mut J: *mut js_State, mut idx: ::core::ffi::c_int) {
    js_error(J, b"not implemented yet\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn js_replace(mut J: *mut js_State, mut idx: ::core::ffi::c_int) {
    idx = if idx < 0 as ::core::ffi::c_int { (*J).top + idx } else { (*J).bot + idx };
    if idx < (*J).bot || idx >= (*J).top {
        js_error(J, b"stack error!\0" as *const u8 as *const ::core::ffi::c_char);
    }
    (*J).top -= 1;
    *(*J).stack.offset(idx as isize) = *(*J).stack.offset((*J).top as isize);
}
#[no_mangle]
pub unsafe extern "C" fn js_copy(mut J: *mut js_State, mut idx: ::core::ffi::c_int) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    *(*J).stack.offset((*J).top as isize) = *stackidx(J, idx);
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_dup(mut J: *mut js_State) {
    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    *(*J).stack.offset((*J).top as isize) = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    (*J).top += 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_dup2(mut J: *mut js_State) {
    if (*J).top + 2 as ::core::ffi::c_int >= JS_STACKSIZE {
        js_stackoverflow(J);
    }
    *(*J).stack.offset((*J).top as isize) = *(*J)
        .stack
        .offset(((*J).top - 2 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top + 1 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    (*J).top += 2 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot2(mut J: *mut js_State) {
    let mut tmp: js_Value = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 1 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 2 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 2 as ::core::ffi::c_int) as isize) = tmp;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot3(mut J: *mut js_State) {
    let mut tmp: js_Value = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 1 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 2 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 2 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 3 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 3 as ::core::ffi::c_int) as isize) = tmp;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot4(mut J: *mut js_State) {
    let mut tmp: js_Value = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 1 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 2 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 2 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 3 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 3 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 4 as ::core::ffi::c_int) as isize);
    *(*J).stack.offset(((*J).top - 4 as ::core::ffi::c_int) as isize) = tmp;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot2pop1(mut J: *mut js_State) {
    *(*J).stack.offset(((*J).top - 2 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    (*J).top -= 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot3pop2(mut J: *mut js_State) {
    *(*J).stack.offset(((*J).top - 3 as ::core::ffi::c_int) as isize) = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    (*J).top -= 2 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_rot(mut J: *mut js_State, mut n: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0;
    let mut tmp: js_Value = *(*J)
        .stack
        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
    i = 1 as ::core::ffi::c_int;
    while i < n {
        *(*J).stack.offset(((*J).top - i) as isize) = *(*J)
            .stack
            .offset(((*J).top - i - 1 as ::core::ffi::c_int) as isize);
        i += 1;
    }
    *(*J).stack.offset(((*J).top - i) as isize) = tmp;
}
#[no_mangle]
pub unsafe extern "C" fn js_isarrayindex(
    mut J: *mut js_State,
    mut p: *const ::core::ffi::c_char,
    mut idx: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        == 0 as ::core::ffi::c_int
    {
        return 0 as ::core::ffi::c_int;
    }
    if *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '0' as i32 {
        return if *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
        {
            *idx = 0 as ::core::ffi::c_int;
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        };
    }
    while *p != 0 {
        let fresh20 = p;
        p = p.offset(1);
        let mut c: ::core::ffi::c_int = *fresh20 as ::core::ffi::c_int;
        if c >= '0' as i32 && c <= '9' as i32 {
            if n >= INT_MAX / 10 as ::core::ffi::c_int {
                return 0 as ::core::ffi::c_int;
            }
            n = n * 10 as ::core::ffi::c_int + (c - '0' as i32);
        } else {
            return 0 as ::core::ffi::c_int
        }
    }
    *idx = n;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn js_pushrune(mut J: *mut js_State, mut rune: Rune) {
    let mut buf: [::core::ffi::c_char; 5] = [0; 5];
    if rune >= 0 as ::core::ffi::c_int {
        buf[jsU_runetochar(&raw mut buf as *mut ::core::ffi::c_char, &raw mut rune)
            as usize] = 0 as ::core::ffi::c_char;
        js_pushstring(J, &raw mut buf as *mut ::core::ffi::c_char);
    } else {
        js_pushundefined(J);
    };
}
#[no_mangle]
pub unsafe extern "C" fn jsR_unflattenarray(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
) {
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0
    {
        let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
        let mut i: ::core::ffi::c_int = 0;
        let mut name: [::core::ffi::c_char; 32] = [0; 32];
        if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
            (*obj).properties = ::core::ptr::null_mut::<js_Property>();
            js_throw(J);
        }
        i = 0 as ::core::ffi::c_int;
        while i < (*obj).u.a.flat_length {
            js_itoa(&raw mut name as *mut ::core::ffi::c_char, i);
            ref_0 = jsV_setproperty(J, obj, &raw mut name as *mut ::core::ffi::c_char);
            (*ref_0).value = *(*obj).u.a.array.offset(i as isize);
            i += 1;
        }
        js_free(J, (*obj).u.a.array as *mut ::core::ffi::c_void);
        (*obj).u.a.simple = 0 as ::core::ffi::c_int;
        (*obj).u.a.flat_length = 0 as ::core::ffi::c_int;
        (*obj).u.a.flat_capacity = 0 as ::core::ffi::c_int;
        (*obj).u.a.array = ::core::ptr::null_mut::<js_Value>();
        js_endtry(J);
    }
}
unsafe extern "C" fn jsR_hasproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut k: ::core::ffi::c_int = 0;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushnumber(J, (*obj).u.a.length as ::core::ffi::c_double);
            return 1 as ::core::ffi::c_int;
        }
        if (*obj).u.a.simple != 0 {
            if js_isarrayindex(J, name, &raw mut k) != 0 {
                if k >= 0 as ::core::ffi::c_int && k < (*obj).u.a.flat_length {
                    js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
                    return 1 as ::core::ffi::c_int;
                }
                return 0 as ::core::ffi::c_int;
            }
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushnumber(J, (*obj).u.s.length as ::core::ffi::c_double);
            return 1 as ::core::ffi::c_int;
        }
        if js_isarrayindex(J, name, &raw mut k) != 0 {
            if k >= 0 as ::core::ffi::c_int && k < (*obj).u.s.length {
                js_pushrune(J, js_runeat(J, (*obj).u.s.string, k) as Rune);
                return 1 as ::core::ffi::c_int;
            }
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"source\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushstring(J, (*obj).u.r.source);
            return 1 as ::core::ffi::c_int;
        }
        if strcmp(name, b"global\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushboolean(
                J,
                (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_G as ::core::ffi::c_int,
            );
            return 1 as ::core::ffi::c_int;
        }
        if strcmp(name, b"ignoreCase\0" as *const u8 as *const ::core::ffi::c_char) == 0
        {
            js_pushboolean(
                J,
                (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_I as ::core::ffi::c_int,
            );
            return 1 as ::core::ffi::c_int;
        }
        if strcmp(name, b"multiline\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushboolean(
                J,
                (*obj).u.r.flags as ::core::ffi::c_int
                    & JS_REGEXP_M as ::core::ffi::c_int,
            );
            return 1 as ::core::ffi::c_int;
        }
        if strcmp(name, b"lastIndex\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            js_pushnumber(J, (*obj).u.r.last as ::core::ffi::c_double);
            return 1 as ::core::ffi::c_int;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if (*obj).u.user.has.is_some()
            && (*obj)
                .u
                .user
                .has
                .expect("non-null function pointer")(J, (*obj).u.user.data, name) != 0
        {
            return 1 as ::core::ffi::c_int;
        }
    }
    ref_0 = jsV_getproperty(J, obj, name);
    if !ref_0.is_null() {
        if !(*ref_0).getter.is_null() {
            js_pushobject(J, (*ref_0).getter);
            js_pushobject(J, obj);
            js_call(J, 0 as ::core::ffi::c_int);
        } else {
            js_pushvalue(J, (*ref_0).value);
        }
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsR_getproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) {
    if jsR_hasproperty(J, obj, name) == 0 {
        js_pushundefined(J);
    }
}
unsafe extern "C" fn jsR_hasindex(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut k: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0
    {
        if k >= 0 as ::core::ffi::c_int && k < (*obj).u.a.flat_length {
            js_pushvalue(J, *(*obj).u.a.array.offset(k as isize));
            return 1 as ::core::ffi::c_int;
        }
        return 0 as ::core::ffi::c_int;
    }
    return jsR_hasproperty(J, obj, js_itoa(&raw mut buf as *mut ::core::ffi::c_char, k));
}
unsafe extern "C" fn jsR_getindex(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut k: ::core::ffi::c_int,
) {
    if jsR_hasindex(J, obj, k) == 0 {
        js_pushundefined(J);
    }
}
unsafe extern "C" fn jsR_setarrayindex(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut k: ::core::ffi::c_int,
    mut value: *mut js_Value,
) {
    let mut newlen: ::core::ffi::c_int = k + 1 as ::core::ffi::c_int;
    if newlen > JS_ARRAYLIMIT {
        js_rangeerror(
            J,
            b"array too large\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if newlen > (*obj).u.a.flat_length {
        if newlen > (*obj).u.a.flat_capacity {
            let mut newcap: ::core::ffi::c_int = (*obj).u.a.flat_capacity;
            if newcap == 0 as ::core::ffi::c_int {
                newcap = 8 as ::core::ffi::c_int;
            }
            while newcap < newlen {
                newcap <<= 1 as ::core::ffi::c_int;
            }
            (*obj).u.a.array = js_realloc(
                J,
                (*obj).u.a.array as *mut ::core::ffi::c_void,
                (newcap as usize)
                    .wrapping_mul(::core::mem::size_of::<js_Value>() as usize)
                    as ::core::ffi::c_int,
            ) as *mut js_Value;
            (*obj).u.a.flat_capacity = newcap;
        }
        (*obj).u.a.flat_length = newlen;
    }
    if newlen > (*obj).u.a.length {
        (*obj).u.a.length = newlen;
    }
    *(*obj).u.a.array.offset(k as isize) = *value;
}
unsafe extern "C" fn jsR_setproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
    mut transient: ::core::ffi::c_int,
) {
    let mut current_block: u64;
    let mut value: *mut js_Value = stackidx(J, -(1 as ::core::ffi::c_int));
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut k: ::core::ffi::c_int = 0;
    let mut own: ::core::ffi::c_int = 0;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            let mut rawlen: ::core::ffi::c_double = jsV_tonumber(J, value);
            let mut newlen: ::core::ffi::c_int = jsV_numbertointeger(rawlen);
            if newlen as ::core::ffi::c_double != rawlen
                || newlen < 0 as ::core::ffi::c_int
            {
                js_rangeerror(
                    J,
                    b"invalid array length\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            if newlen > JS_ARRAYLIMIT {
                js_rangeerror(
                    J,
                    b"array too large\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            if (*obj).u.a.simple != 0 {
                (*obj).u.a.length = newlen;
                if newlen <= (*obj).u.a.flat_length {
                    (*obj).u.a.flat_length = newlen;
                }
            } else {
                jsV_resizearray(J, obj, newlen);
            }
            return;
        }
        if js_isarrayindex(J, name, &raw mut k) != 0 {
            if (*obj).u.a.simple != 0 {
                if k >= 0 as ::core::ffi::c_int && k <= (*obj).u.a.flat_length {
                    jsR_setarrayindex(J, obj, k, value);
                } else {
                    jsR_unflattenarray(J, obj);
                    if (*obj).u.a.length < k + 1 as ::core::ffi::c_int {
                        (*obj).u.a.length = k + 1 as ::core::ffi::c_int;
                    }
                }
            } else if (*obj).u.a.length < k + 1 as ::core::ffi::c_int {
                (*obj).u.a.length = k + 1 as ::core::ffi::c_int;
            }
        }
        current_block = 4567019141635105728;
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 14334464195658854988;
        } else if js_isarrayindex(J, name, &raw mut k) != 0 {
            if k >= 0 as ::core::ffi::c_int && k < (*obj).u.s.length {
                current_block = 14334464195658854988;
            } else {
                current_block = 4567019141635105728;
            }
        } else {
            current_block = 4567019141635105728;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"source\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 14334464195658854988;
        } else if strcmp(name, b"global\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 14334464195658854988;
        } else if strcmp(
            name,
            b"ignoreCase\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            current_block = 14334464195658854988;
        } else if strcmp(name, b"multiline\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 14334464195658854988;
        } else {
            if strcmp(name, b"lastIndex\0" as *const u8 as *const ::core::ffi::c_char)
                == 0
            {
                (*obj).u.r.last = jsV_tointeger(J, value) as ::core::ffi::c_ushort;
                return;
            }
            current_block = 4567019141635105728;
        }
    } else {
        if (*obj).type_0 as ::core::ffi::c_uint
            == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if (*obj).u.user.put.is_some()
                && (*obj)
                    .u
                    .user
                    .put
                    .expect("non-null function pointer")(J, (*obj).u.user.data, name)
                    != 0
            {
                return;
            }
        }
        current_block = 4567019141635105728;
    }
    match current_block {
        4567019141635105728 => {
            ref_0 = jsV_getpropertyx(J, obj, name, &raw mut own);
            if !ref_0.is_null() {
                if !(*ref_0).setter.is_null() {
                    js_pushobject(J, (*ref_0).setter);
                    js_pushobject(J, obj);
                    js_pushvalue(J, *value);
                    js_call(J, 1 as ::core::ffi::c_int);
                    js_pop(J, 1 as ::core::ffi::c_int);
                    return;
                } else {
                    if (*J).strict != 0 {
                        if !(*ref_0).getter.is_null() {
                            js_typeerror(
                                J,
                                b"setting property '%s' that only has a getter\0"
                                    as *const u8 as *const ::core::ffi::c_char,
                                name,
                            );
                        }
                    }
                    if (*ref_0).atts & JS_READONLY as ::core::ffi::c_int != 0 {
                        current_block = 14334464195658854988;
                    } else {
                        current_block = 13826291924415791078;
                    }
                }
            } else {
                current_block = 13826291924415791078;
            }
            match current_block {
                14334464195658854988 => {}
                _ => {
                    if ref_0.is_null() || own == 0 {
                        if transient != 0 {
                            if (*J).strict != 0 {
                                js_typeerror(
                                    J,
                                    b"cannot create property '%s' on transient object\0"
                                        as *const u8 as *const ::core::ffi::c_char,
                                    name,
                                );
                            }
                            return;
                        }
                        ref_0 = jsV_setproperty(J, obj, name);
                    }
                    if !ref_0.is_null() {
                        if (*ref_0).atts & JS_READONLY as ::core::ffi::c_int == 0 {
                            (*ref_0).value = *value;
                            current_block = 1623252117315916725;
                        } else {
                            current_block = 14334464195658854988;
                        }
                    } else {
                        current_block = 1623252117315916725;
                    }
                    match current_block {
                        14334464195658854988 => {}
                        _ => return,
                    }
                }
            }
        }
        _ => {}
    }
    if (*J).strict != 0 {
        js_typeerror(
            J,
            b"'%s' is read-only\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
    }
}
unsafe extern "C" fn jsR_setindex(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut k: ::core::ffi::c_int,
    mut transient: ::core::ffi::c_int,
) {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0 && k >= 0 as ::core::ffi::c_int
        && k <= (*obj).u.a.flat_length
    {
        jsR_setarrayindex(J, obj, k, stackidx(J, -(1 as ::core::ffi::c_int)));
    } else {
        jsR_setproperty(
            J,
            obj,
            js_itoa(&raw mut buf as *mut ::core::ffi::c_char, k),
            transient,
        );
    };
}
unsafe extern "C" fn jsR_defproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
    mut atts: ::core::ffi::c_int,
    mut value: *mut js_Value,
    mut getter: *mut js_Object,
    mut setter: *mut js_Object,
    mut throw: ::core::ffi::c_int,
) {
    let mut current_block: u64;
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut k: ::core::ffi::c_int = 0;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 17835093897775171461;
        } else {
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
            current_block = 2370887241019905314;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 17835093897775171461;
        } else if js_isarrayindex(J, name, &raw mut k) != 0 {
            if k >= 0 as ::core::ffi::c_int && k < (*obj).u.s.length {
                current_block = 17835093897775171461;
            } else {
                current_block = 2370887241019905314;
            }
        } else {
            current_block = 2370887241019905314;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"source\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 17835093897775171461;
        } else if strcmp(name, b"global\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 17835093897775171461;
        } else if strcmp(
            name,
            b"ignoreCase\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            current_block = 17835093897775171461;
        } else if strcmp(name, b"multiline\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 17835093897775171461;
        } else if strcmp(name, b"lastIndex\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 17835093897775171461;
        } else {
            current_block = 2370887241019905314;
        }
    } else {
        if (*obj).type_0 as ::core::ffi::c_uint
            == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if (*obj).u.user.put.is_some()
                && (*obj)
                    .u
                    .user
                    .put
                    .expect("non-null function pointer")(J, (*obj).u.user.data, name)
                    != 0
            {
                return;
            }
        }
        current_block = 2370887241019905314;
    }
    match current_block {
        17835093897775171461 => {
            if (*J).strict != 0 || throw != 0 {
                js_typeerror(
                    J,
                    b"'%s' is read-only or non-configurable\0" as *const u8
                        as *const ::core::ffi::c_char,
                    name,
                );
            }
            return;
        }
        _ => {
            ref_0 = jsV_setproperty(J, obj, name);
            if !ref_0.is_null() {
                if !value.is_null() {
                    if (*ref_0).atts & JS_READONLY as ::core::ffi::c_int == 0 {
                        (*ref_0).value = *value;
                    } else if (*J).strict != 0 {
                        js_typeerror(
                            J,
                            b"'%s' is read-only\0" as *const u8
                                as *const ::core::ffi::c_char,
                            name,
                        );
                    }
                }
                if !getter.is_null() {
                    if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int == 0 {
                        (*ref_0).getter = getter;
                    } else if (*J).strict != 0 {
                        js_typeerror(
                            J,
                            b"'%s' is non-configurable\0" as *const u8
                                as *const ::core::ffi::c_char,
                            name,
                        );
                    }
                }
                if !setter.is_null() {
                    if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int == 0 {
                        (*ref_0).setter = setter;
                    } else if (*J).strict != 0 {
                        js_typeerror(
                            J,
                            b"'%s' is non-configurable\0" as *const u8
                                as *const ::core::ffi::c_char,
                            name,
                        );
                    }
                }
                (*ref_0).atts |= atts;
            }
            return;
        }
    };
}
unsafe extern "C" fn jsR_delproperty(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut ref_0: *mut js_Property = ::core::ptr::null_mut::<js_Property>();
    let mut k: ::core::ffi::c_int = 0;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 4371238883303246385;
        } else {
            if (*obj).u.a.simple != 0 {
                jsR_unflattenarray(J, obj);
            }
            current_block = 2370887241019905314;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"length\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 4371238883303246385;
        } else if js_isarrayindex(J, name, &raw mut k) != 0 {
            if k >= 0 as ::core::ffi::c_int && k < (*obj).u.s.length {
                current_block = 4371238883303246385;
            } else {
                current_block = 2370887241019905314;
            }
        } else {
            current_block = 2370887241019905314;
        }
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CREGEXP as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if strcmp(name, b"source\0" as *const u8 as *const ::core::ffi::c_char) == 0 {
            current_block = 4371238883303246385;
        } else if strcmp(name, b"global\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 4371238883303246385;
        } else if strcmp(
            name,
            b"ignoreCase\0" as *const u8 as *const ::core::ffi::c_char,
        ) == 0
        {
            current_block = 4371238883303246385;
        } else if strcmp(name, b"multiline\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 4371238883303246385;
        } else if strcmp(name, b"lastIndex\0" as *const u8 as *const ::core::ffi::c_char)
            == 0
        {
            current_block = 4371238883303246385;
        } else {
            current_block = 2370887241019905314;
        }
    } else {
        if (*obj).type_0 as ::core::ffi::c_uint
            == JS_CUSERDATA as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if (*obj).u.user.delete.is_some()
                && (*obj)
                    .u
                    .user
                    .delete
                    .expect("non-null function pointer")(J, (*obj).u.user.data, name)
                    != 0
            {
                return 1 as ::core::ffi::c_int;
            }
        }
        current_block = 2370887241019905314;
    }
    match current_block {
        2370887241019905314 => {
            ref_0 = jsV_getownproperty(J, obj, name);
            if !ref_0.is_null() {
                if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int != 0 {
                    current_block = 4371238883303246385;
                } else {
                    jsV_delproperty(J, obj, name);
                    current_block = 14576567515993809846;
                }
            } else {
                current_block = 14576567515993809846;
            }
            match current_block {
                4371238883303246385 => {}
                _ => return 1 as ::core::ffi::c_int,
            }
        }
        _ => {}
    }
    if (*J).strict != 0 {
        js_typeerror(
            J,
            b"'%s' is non-configurable\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsR_delindex(
    mut J: *mut js_State,
    mut obj: *mut js_Object,
    mut k: ::core::ffi::c_int,
) {
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.a.simple != 0
        && k == (*obj).u.a.flat_length - 1 as ::core::ffi::c_int
    {
        (*obj).u.a.flat_length = k;
    } else {
        jsR_delproperty(J, obj, js_itoa(&raw mut buf as *mut ::core::ffi::c_char, k));
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_ref(mut J: *mut js_State) -> *const ::core::ffi::c_char {
    let mut v: *mut js_Value = stackidx(J, -(1 as ::core::ffi::c_int));
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut buf: [::core::ffi::c_char; 32] = [0; 32];
    match (*v).t.type_0 as ::core::ffi::c_int {
        1 => {
            s = b"_Undefined\0" as *const u8 as *const ::core::ffi::c_char;
        }
        2 => {
            s = b"_Null\0" as *const u8 as *const ::core::ffi::c_char;
        }
        3 => {
            s = if (*v).u.boolean != 0 {
                b"_True\0" as *const u8 as *const ::core::ffi::c_char
            } else {
                b"_False\0" as *const u8 as *const ::core::ffi::c_char
            };
        }
        7 => {
            sprintf(
                &raw mut buf as *mut ::core::ffi::c_char,
                b"%p\0" as *const u8 as *const ::core::ffi::c_char,
                (*v).u.object as *mut ::core::ffi::c_void,
            );
            s = js_intern(J, &raw mut buf as *mut ::core::ffi::c_char);
        }
        _ => {
            let fresh31 = (*J).nextref;
            (*J).nextref = (*J).nextref + 1;
            sprintf(
                &raw mut buf as *mut ::core::ffi::c_char,
                b"%d\0" as *const u8 as *const ::core::ffi::c_char,
                fresh31,
            );
            s = js_intern(J, &raw mut buf as *mut ::core::ffi::c_char);
        }
    }
    js_setregistry(J, s);
    return s;
}
#[no_mangle]
pub unsafe extern "C" fn js_unref(
    mut J: *mut js_State,
    mut ref_0: *const ::core::ffi::c_char,
) {
    js_delregistry(J, ref_0);
}
#[no_mangle]
pub unsafe extern "C" fn js_getregistry(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_getproperty(J, (*J).R, name);
}
#[no_mangle]
pub unsafe extern "C" fn js_setregistry(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_setproperty(J, (*J).R, name, 0 as ::core::ffi::c_int);
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_delregistry(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_delproperty(J, (*J).R, name);
}
#[no_mangle]
pub unsafe extern "C" fn js_getglobal(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_getproperty(J, (*J).G, name);
}
#[no_mangle]
pub unsafe extern "C" fn js_setglobal(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_setproperty(J, (*J).G, name, 0 as ::core::ffi::c_int);
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_defglobal(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
    mut atts: ::core::ffi::c_int,
) {
    jsR_defproperty(
        J,
        (*J).G,
        name,
        atts,
        stackidx(J, -(1 as ::core::ffi::c_int)),
        ::core::ptr::null_mut::<js_Object>(),
        ::core::ptr::null_mut::<js_Object>(),
        0 as ::core::ffi::c_int,
    );
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_delglobal(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_delproperty(J, (*J).G, name);
}
#[no_mangle]
pub unsafe extern "C" fn js_getproperty(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_getproperty(J, js_toobject(J, idx), name);
}
#[no_mangle]
pub unsafe extern "C" fn js_setproperty(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_setproperty(
        J,
        js_toobject(J, idx),
        name,
        (js_isobject(J, idx) == 0) as ::core::ffi::c_int,
    );
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_defproperty(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
    mut atts: ::core::ffi::c_int,
) {
    jsR_defproperty(
        J,
        js_toobject(J, idx),
        name,
        atts,
        stackidx(J, -(1 as ::core::ffi::c_int)),
        ::core::ptr::null_mut::<js_Object>(),
        ::core::ptr::null_mut::<js_Object>(),
        1 as ::core::ffi::c_int,
    );
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_delproperty(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
) {
    jsR_delproperty(J, js_toobject(J, idx), name);
}
#[no_mangle]
pub unsafe extern "C" fn js_defaccessor(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
    mut atts: ::core::ffi::c_int,
) {
    jsR_defproperty(
        J,
        js_toobject(J, idx),
        name,
        atts,
        ::core::ptr::null_mut::<js_Value>(),
        jsR_tofunction(J, -(2 as ::core::ffi::c_int)),
        jsR_tofunction(J, -(1 as ::core::ffi::c_int)),
        1 as ::core::ffi::c_int,
    );
    js_pop(J, 2 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_hasproperty(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return jsR_hasproperty(J, js_toobject(J, idx), name);
}
#[no_mangle]
pub unsafe extern "C" fn js_getindex(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut i: ::core::ffi::c_int,
) {
    jsR_getindex(J, js_toobject(J, idx), i);
}
#[no_mangle]
pub unsafe extern "C" fn js_hasindex(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut i: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return jsR_hasindex(J, js_toobject(J, idx), i);
}
#[no_mangle]
pub unsafe extern "C" fn js_setindex(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut i: ::core::ffi::c_int,
) {
    jsR_setindex(
        J,
        js_toobject(J, idx),
        i,
        (js_isobject(J, idx) == 0) as ::core::ffi::c_int,
    );
    js_pop(J, 1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_delindex(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut i: ::core::ffi::c_int,
) {
    jsR_delindex(J, js_toobject(J, idx), i);
}
#[no_mangle]
pub unsafe extern "C" fn js_pushiterator(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut own: ::core::ffi::c_int,
) {
    js_pushobject(J, jsV_newiterator(J, js_toobject(J, idx), own));
}
#[no_mangle]
pub unsafe extern "C" fn js_nextiterator(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    return jsV_nextiterator(J, js_toobject(J, idx));
}
#[no_mangle]
pub unsafe extern "C" fn jsR_newenvironment(
    mut J: *mut js_State,
    mut vars: *mut js_Object,
    mut outer: *mut js_Environment,
) -> *mut js_Environment {
    let mut E: *mut js_Environment = js_malloc(
        J,
        ::core::mem::size_of::<js_Environment>() as ::core::ffi::c_int,
    ) as *mut js_Environment;
    (*E).gcmark = 0 as ::core::ffi::c_int;
    (*E).gcnext = (*J).gcenv;
    (*J).gcenv = E;
    (*J).gccounter = (*J).gccounter.wrapping_add(1);
    (*E).outer = outer;
    (*E).variables = vars;
    return E;
}
unsafe extern "C" fn js_initvar(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
    mut idx: ::core::ffi::c_int,
) {
    jsR_defproperty(
        J,
        (*(*J).E).variables,
        name,
        JS_DONTENUM as ::core::ffi::c_int | JS_DONTCONF as ::core::ffi::c_int,
        stackidx(J, idx),
        ::core::ptr::null_mut::<js_Object>(),
        ::core::ptr::null_mut::<js_Object>(),
        0 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn js_hasvar(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut E: *mut js_Environment = (*J).E;
    loop {
        let mut ref_0: *mut js_Property = jsV_getproperty(J, (*E).variables, name);
        if !ref_0.is_null() {
            if !(*ref_0).getter.is_null() {
                js_pushobject(J, (*ref_0).getter);
                js_pushobject(J, (*E).variables);
                js_call(J, 0 as ::core::ffi::c_int);
            } else {
                js_pushvalue(J, (*ref_0).value);
            }
            return 1 as ::core::ffi::c_int;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn js_setvar(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) {
    let mut E: *mut js_Environment = (*J).E;
    loop {
        let mut ref_0: *mut js_Property = jsV_getproperty(J, (*E).variables, name);
        if !ref_0.is_null() {
            if !(*ref_0).setter.is_null() {
                js_pushobject(J, (*ref_0).setter);
                js_pushobject(J, (*E).variables);
                js_copy(J, -(3 as ::core::ffi::c_int));
                js_call(J, 1 as ::core::ffi::c_int);
                js_pop(J, 1 as ::core::ffi::c_int);
                return;
            }
            if (*ref_0).atts & JS_READONLY as ::core::ffi::c_int == 0 {
                (*ref_0).value = *stackidx(J, -(1 as ::core::ffi::c_int));
            } else if (*J).strict != 0 {
                js_typeerror(
                    J,
                    b"'%s' is read-only\0" as *const u8 as *const ::core::ffi::c_char,
                    name,
                );
            }
            return;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    if (*J).strict != 0 {
        js_referenceerror(
            J,
            b"assignment to undeclared variable '%s'\0" as *const u8
                as *const ::core::ffi::c_char,
            name,
        );
    }
    jsR_setproperty(J, (*J).G, name, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn js_delvar(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut E: *mut js_Environment = (*J).E;
    loop {
        let mut ref_0: *mut js_Property = jsV_getownproperty(J, (*E).variables, name);
        if !ref_0.is_null() {
            if (*ref_0).atts & JS_DONTCONF as ::core::ffi::c_int != 0 {
                if (*J).strict != 0 {
                    js_typeerror(
                        J,
                        b"'%s' is non-configurable\0" as *const u8
                            as *const ::core::ffi::c_char,
                        name,
                    );
                }
                return 0 as ::core::ffi::c_int;
            }
            jsV_delproperty(J, (*E).variables, name);
            return 1 as ::core::ffi::c_int;
        }
        E = (*E).outer;
        if E.is_null() {
            break;
        }
    }
    return jsR_delproperty(J, (*J).G, name);
}
unsafe extern "C" fn jsR_savescope(mut J: *mut js_State, mut newE: *mut js_Environment) {
    if (*J).envtop + 1 as ::core::ffi::c_int >= JS_ENVLIMIT {
        js_stackoverflow(J);
    }
    let fresh30 = (*J).envtop;
    (*J).envtop = (*J).envtop + 1;
    (*J).envstack[fresh30 as usize] = (*J).E;
    (*J).E = newE;
}
unsafe extern "C" fn jsR_restorescope(mut J: *mut js_State) {
    (*J).envtop -= 1;
    (*J).E = (*J).envstack[(*J).envtop as usize];
}
unsafe extern "C" fn jsR_calllwfunction(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
    mut F: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    let mut v: js_Value = js_Value {
        t: C2RustUnnamed_6 {
            pad: [0; 15],
            type_0: 0,
        },
    };
    let mut i: ::core::ffi::c_int = 0;
    jsR_savescope(J, scope);
    if n > (*F).numparams {
        js_pop(J, n - (*F).numparams);
        n = (*F).numparams;
    }
    i = n;
    while i < (*F).varlen {
        js_pushundefined(J);
        i += 1;
    }
    jsR_run(J, F);
    v = *stackidx(J, -(1 as ::core::ffi::c_int));
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);
    jsR_restorescope(J);
}
unsafe extern "C" fn jsR_callfunction(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
    mut F: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    let mut v: js_Value = js_Value {
        t: C2RustUnnamed_6 {
            pad: [0; 15],
            type_0: 0,
        },
    };
    let mut i: ::core::ffi::c_int = 0;
    scope = jsR_newenvironment(
        J,
        jsV_newobject(J, JS_COBJECT, ::core::ptr::null_mut::<js_Object>()),
        scope,
    );
    jsR_savescope(J, scope);
    if (*F).arguments != 0 {
        js_newarguments(J);
        if (*J).strict == 0 {
            js_currentfunction(J);
            js_defproperty(
                J,
                -(2 as ::core::ffi::c_int),
                b"callee\0" as *const u8 as *const ::core::ffi::c_char,
                JS_DONTENUM as ::core::ffi::c_int,
            );
        }
        js_pushnumber(J, n as ::core::ffi::c_double);
        js_defproperty(
            J,
            -(2 as ::core::ffi::c_int),
            b"length\0" as *const u8 as *const ::core::ffi::c_char,
            JS_DONTENUM as ::core::ffi::c_int,
        );
        i = 0 as ::core::ffi::c_int;
        while i < n {
            js_copy(J, i + 1 as ::core::ffi::c_int);
            js_setindex(J, -(2 as ::core::ffi::c_int), i);
            i += 1;
        }
        js_initvar(
            J,
            b"arguments\0" as *const u8 as *const ::core::ffi::c_char,
            -(1 as ::core::ffi::c_int),
        );
        js_pop(J, 1 as ::core::ffi::c_int);
    }
    i = 0 as ::core::ffi::c_int;
    while i < n && i < (*F).numparams {
        js_initvar(J, *(*F).vartab.offset(i as isize), i + 1 as ::core::ffi::c_int);
        i += 1;
    }
    js_pop(J, n);
    while i < (*F).varlen {
        js_pushundefined(J);
        js_initvar(J, *(*F).vartab.offset(i as isize), -(1 as ::core::ffi::c_int));
        js_pop(J, 1 as ::core::ffi::c_int);
        i += 1;
    }
    jsR_run(J, F);
    v = *stackidx(J, -(1 as ::core::ffi::c_int));
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);
    jsR_restorescope(J);
}
unsafe extern "C" fn jsR_callscript(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
    mut F: *mut js_Function,
    mut scope: *mut js_Environment,
) {
    let mut v: js_Value = js_Value {
        t: C2RustUnnamed_6 {
            pad: [0; 15],
            type_0: 0,
        },
    };
    let mut i: ::core::ffi::c_int = 0;
    if !scope.is_null() {
        jsR_savescope(J, scope);
    }
    js_pop(J, n);
    i = 0 as ::core::ffi::c_int;
    while i < (*F).varlen {
        if js_hasvar(J, *(*F).vartab.offset(i as isize)) == 0 {
            js_pushundefined(J);
            js_initvar(J, *(*F).vartab.offset(i as isize), -(1 as ::core::ffi::c_int));
            js_pop(J, 1 as ::core::ffi::c_int);
        }
        i += 1;
    }
    jsR_run(J, F);
    v = *stackidx(J, -(1 as ::core::ffi::c_int));
    (*J).bot -= 1;
    (*J).top = (*J).bot;
    js_pushvalue(J, v);
    if !scope.is_null() {
        jsR_restorescope(J);
    }
}
unsafe extern "C" fn jsR_callcfunction(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
    mut min: ::core::ffi::c_int,
    mut F: js_CFunction,
) {
    let mut save_top: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut v: js_Value = js_Value {
        t: C2RustUnnamed_6 {
            pad: [0; 15],
            type_0: 0,
        },
    };
    i = n;
    while i < min {
        js_pushundefined(J);
        i += 1;
    }
    save_top = (*J).top;
    F.expect("non-null function pointer")(J);
    if (*J).top > save_top {
        v = *stackidx(J, -(1 as ::core::ffi::c_int));
        (*J).bot -= 1;
        (*J).top = (*J).bot;
        js_pushvalue(J, v);
    } else {
        (*J).bot -= 1;
        (*J).top = (*J).bot;
        js_pushundefined(J);
    };
}
unsafe extern "C" fn jsR_pushtrace(
    mut J: *mut js_State,
    mut name: *const ::core::ffi::c_char,
    mut file: *const ::core::ffi::c_char,
    mut line: ::core::ffi::c_int,
) {
    if (*J).tracetop + 1 as ::core::ffi::c_int == JS_ENVLIMIT {
        js_error(J, b"call stack overflow\0" as *const u8 as *const ::core::ffi::c_char);
    }
    (*J).tracetop += 1;
    (*J).trace[(*J).tracetop as usize].stack = (*J).bot;
    (*J).trace[(*J).tracetop as usize].name = name;
    (*J).trace[(*J).tracetop as usize].file = file;
    (*J).trace[(*J).tracetop as usize].line = line;
}
#[no_mangle]
pub unsafe extern "C" fn js_call(mut J: *mut js_State, mut n: ::core::ffi::c_int) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut savebot: ::core::ffi::c_int = 0;
    if n < 0 as ::core::ffi::c_int {
        js_rangeerror(
            J,
            b"number of arguments cannot be negative\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if js_iscallable(J, -n - 2 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"%s is not callable\0" as *const u8 as *const ::core::ffi::c_char,
            js_typeof(J, -n - 2 as ::core::ffi::c_int),
        );
    }
    obj = js_toobject(J, -n - 2 as ::core::ffi::c_int);
    savebot = (*J).bot;
    (*J).bot = (*J).top - n - 1 as ::core::ffi::c_int;
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        jsR_pushtrace(
            J,
            (*(*obj).u.f.function).name,
            (*(*obj).u.f.function).filename,
            (*(*obj).u.f.function).line,
        );
        if (*(*obj).u.f.function).lightweight != 0 {
            jsR_calllwfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        } else {
            jsR_callfunction(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        }
        (*J).tracetop -= 1;
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CSCRIPT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        jsR_pushtrace(
            J,
            (*(*obj).u.f.function).name,
            (*(*obj).u.f.function).filename,
            (*(*obj).u.f.function).line,
        );
        jsR_callscript(J, n, (*obj).u.f.function, (*obj).u.f.scope);
        (*J).tracetop -= 1;
    } else if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        jsR_pushtrace(
            J,
            (*obj).u.c.name,
            b"native\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.function);
        (*J).tracetop -= 1;
    }
    (*J).bot = savebot;
}
#[no_mangle]
pub unsafe extern "C" fn js_construct(mut J: *mut js_State, mut n: ::core::ffi::c_int) {
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut prototype: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut newobj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    if js_iscallable(J, -n - 1 as ::core::ffi::c_int) == 0 {
        js_typeerror(
            J,
            b"%s is not callable\0" as *const u8 as *const ::core::ffi::c_char,
            js_typeof(J, -n - 1 as ::core::ffi::c_int),
        );
    }
    obj = js_toobject(J, -n - 1 as ::core::ffi::c_int);
    if (*obj).type_0 as ::core::ffi::c_uint
        == JS_CCFUNCTION as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*obj).u.c.constructor.is_some()
    {
        let mut savebot: ::core::ffi::c_int = (*J).bot;
        js_pushnull(J);
        if n > 0 as ::core::ffi::c_int {
            js_rot(J, n + 1 as ::core::ffi::c_int);
        }
        (*J).bot = (*J).top - n - 1 as ::core::ffi::c_int;
        jsR_pushtrace(
            J,
            (*obj).u.c.name,
            b"native\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
        jsR_callcfunction(J, n, (*obj).u.c.length, (*obj).u.c.constructor);
        (*J).tracetop -= 1;
        (*J).bot = savebot;
        return;
    }
    js_getproperty(
        J,
        -n - 1 as ::core::ffi::c_int,
        b"prototype\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if js_isobject(J, -(1 as ::core::ffi::c_int)) != 0 {
        prototype = js_toobject(J, -(1 as ::core::ffi::c_int));
    } else {
        prototype = (*J).Object_prototype;
    }
    js_pop(J, 1 as ::core::ffi::c_int);
    newobj = jsV_newobject(J, JS_COBJECT, prototype);
    js_pushobject(J, newobj);
    if n > 0 as ::core::ffi::c_int {
        js_rot(J, n + 1 as ::core::ffi::c_int);
    }
    js_pushobject(J, newobj);
    js_rot(J, n + 3 as ::core::ffi::c_int);
    js_call(J, n);
    if js_isobject(J, -(1 as ::core::ffi::c_int)) == 0 {
        js_pop(J, 1 as ::core::ffi::c_int);
    } else {
        js_rot2pop1(J);
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_eval(mut J: *mut js_State) {
    if js_isstring(J, -(1 as ::core::ffi::c_int)) == 0 {
        return;
    }
    js_loadeval(
        J,
        b"(eval)\0" as *const u8 as *const ::core::ffi::c_char,
        js_tostring(J, -(1 as ::core::ffi::c_int)),
    );
    js_rot2pop1(J);
    js_copy(J, 0 as ::core::ffi::c_int);
    js_call(J, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn js_pconstruct(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut savetop: ::core::ffi::c_int = (*J).top - n - 2 as ::core::ffi::c_int;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        *(*J).stack.offset(savetop as isize) = *(*J)
            .stack
            .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
        (*J).top = savetop + 1 as ::core::ffi::c_int;
        return 1 as ::core::ffi::c_int;
    }
    js_construct(J, n);
    js_endtry(J);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_pcall(
    mut J: *mut js_State,
    mut n: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut savetop: ::core::ffi::c_int = (*J).top - n - 2 as ::core::ffi::c_int;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        *(*J).stack.offset(savetop as isize) = *(*J)
            .stack
            .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
        (*J).top = savetop + 1 as ::core::ffi::c_int;
        return 1 as ::core::ffi::c_int;
    }
    js_call(J, n);
    js_endtry(J);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_savetrypc(
    mut J: *mut js_State,
    mut pc: *mut js_Instruction,
) -> *mut ::core::ffi::c_void {
    if (*J).trytop == JS_TRYLIMIT {
        js_trystackoverflow(J);
    }
    (*J).trybuf[(*J).trytop as usize].E = (*J).E;
    (*J).trybuf[(*J).trytop as usize].envtop = (*J).envtop;
    (*J).trybuf[(*J).trytop as usize].tracetop = (*J).tracetop;
    (*J).trybuf[(*J).trytop as usize].top = (*J).top;
    (*J).trybuf[(*J).trytop as usize].bot = (*J).bot;
    (*J).trybuf[(*J).trytop as usize].strict = (*J).strict;
    (*J).trybuf[(*J).trytop as usize].pc = pc;
    let fresh23 = (*J).trytop;
    (*J).trytop = (*J).trytop + 1;
    return &raw mut (*(&raw mut (*J).trybuf as *mut js_Jumpbuf).offset(fresh23 as isize))
        .buf as *mut __jmp_buf_tag as *mut ::core::ffi::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn js_savetry(mut J: *mut js_State) -> *mut ::core::ffi::c_void {
    if (*J).trytop == JS_TRYLIMIT {
        js_trystackoverflow(J);
    }
    (*J).trybuf[(*J).trytop as usize].E = (*J).E;
    (*J).trybuf[(*J).trytop as usize].envtop = (*J).envtop;
    (*J).trybuf[(*J).trytop as usize].tracetop = (*J).tracetop;
    (*J).trybuf[(*J).trytop as usize].top = (*J).top;
    (*J).trybuf[(*J).trytop as usize].bot = (*J).bot;
    (*J).trybuf[(*J).trytop as usize].strict = (*J).strict;
    (*J).trybuf[(*J).trytop as usize].pc = ::core::ptr::null_mut::<js_Instruction>();
    let fresh21 = (*J).trytop;
    (*J).trytop = (*J).trytop + 1;
    return &raw mut (*(&raw mut (*J).trybuf as *mut js_Jumpbuf).offset(fresh21 as isize))
        .buf as *mut __jmp_buf_tag as *mut ::core::ffi::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn js_endtry(mut J: *mut js_State) {
    if (*J).trytop == 0 as ::core::ffi::c_int {
        js_error(
            J,
            b"endtry: exception stack underflow\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    (*J).trytop -= 1;
}
#[no_mangle]
pub unsafe extern "C" fn js_throw(mut J: *mut js_State) -> ! {
    if (*J).trytop > 0 as ::core::ffi::c_int {
        let mut v: js_Value = *stackidx(J, -(1 as ::core::ffi::c_int));
        (*J).trytop -= 1;
        (*J).E = (*J).trybuf[(*J).trytop as usize].E;
        (*J).envtop = (*J).trybuf[(*J).trytop as usize].envtop;
        (*J).tracetop = (*J).trybuf[(*J).trytop as usize].tracetop;
        (*J).top = (*J).trybuf[(*J).trytop as usize].top;
        (*J).bot = (*J).trybuf[(*J).trytop as usize].bot;
        (*J).strict = (*J).trybuf[(*J).trytop as usize].strict;
        js_pushvalue(J, v);
        longjmp(
            &raw mut (*(&raw mut (*J).trybuf as *mut js_Jumpbuf)
                .offset((*J).trytop as isize))
                .buf as *mut __jmp_buf_tag,
            1 as ::core::ffi::c_int,
        );
    }
    if (*J).panic.is_some() {
        (*J).panic.expect("non-null function pointer")(J);
    }
    abort();
}
unsafe extern "C" fn js_dumpvalue(mut J: *mut js_State, mut v: js_Value) {
    match v.t.type_0 as ::core::ffi::c_int {
        1 => {
            printf(b"undefined\0" as *const u8 as *const ::core::ffi::c_char);
        }
        2 => {
            printf(b"null\0" as *const u8 as *const ::core::ffi::c_char);
        }
        3 => {
            printf(
                if v.u.boolean != 0 {
                    b"true\0" as *const u8 as *const ::core::ffi::c_char
                } else {
                    b"false\0" as *const u8 as *const ::core::ffi::c_char
                },
            );
        }
        4 => {
            printf(b"%.9g\0" as *const u8 as *const ::core::ffi::c_char, v.u.number);
        }
        0 => {
            printf(
                b"'%s'\0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut v.u.shrstr as *mut ::core::ffi::c_char,
            );
        }
        5 => {
            printf(b"'%s'\0" as *const u8 as *const ::core::ffi::c_char, v.u.litstr);
        }
        6 => {
            printf(
                b"'%s'\0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut (*v.u.memstr).p as *mut ::core::ffi::c_char,
            );
        }
        7 => {
            if v.u.object == (*J).G {
                printf(b"[Global]\0" as *const u8 as *const ::core::ffi::c_char);
            } else {
                match (*v.u.object).type_0 as ::core::ffi::c_uint {
                    0 => {
                        printf(
                            b"[Object %p]\0" as *const u8 as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                        );
                    }
                    1 => {
                        printf(
                            b"[Array %p]\0" as *const u8 as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                        );
                    }
                    2 => {
                        printf(
                            b"[Function %p, %s, %s:%d]\0" as *const u8
                                as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                            (*(*v.u.object).u.f.function).name,
                            (*(*v.u.object).u.f.function).filename,
                            (*(*v.u.object).u.f.function).line,
                        );
                    }
                    3 => {
                        printf(
                            b"[Script %s]\0" as *const u8 as *const ::core::ffi::c_char,
                            (*(*v.u.object).u.f.function).filename,
                        );
                    }
                    4 => {
                        printf(
                            b"[CFunction %s]\0" as *const u8
                                as *const ::core::ffi::c_char,
                            (*v.u.object).u.c.name,
                        );
                    }
                    6 => {
                        printf(
                            b"[Boolean %d]\0" as *const u8 as *const ::core::ffi::c_char,
                            (*v.u.object).u.boolean,
                        );
                    }
                    7 => {
                        printf(
                            b"[Number %g]\0" as *const u8 as *const ::core::ffi::c_char,
                            (*v.u.object).u.number,
                        );
                    }
                    8 => {
                        printf(
                            b"[String'%s']\0" as *const u8 as *const ::core::ffi::c_char,
                            (*v.u.object).u.s.string,
                        );
                    }
                    5 => {
                        printf(b"[Error]\0" as *const u8 as *const ::core::ffi::c_char);
                    }
                    13 => {
                        printf(
                            b"[Arguments %p]\0" as *const u8
                                as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                        );
                    }
                    14 => {
                        printf(
                            b"[Iterator %p]\0" as *const u8
                                as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                        );
                    }
                    15 => {
                        printf(
                            b"[Userdata %s %p]\0" as *const u8
                                as *const ::core::ffi::c_char,
                            (*v.u.object).u.user.tag,
                            (*v.u.object).u.user.data,
                        );
                    }
                    _ => {
                        printf(
                            b"[Object %p]\0" as *const u8 as *const ::core::ffi::c_char,
                            v.u.object as *mut ::core::ffi::c_void,
                        );
                    }
                }
            }
        }
        _ => {}
    };
}
unsafe extern "C" fn js_stacktrace(mut J: *mut js_State) {
    let mut n: ::core::ffi::c_int = 0;
    printf(b"stack trace:\n\0" as *const u8 as *const ::core::ffi::c_char);
    n = (*J).tracetop;
    while n >= 0 as ::core::ffi::c_int {
        let mut name: *const ::core::ffi::c_char = (*J).trace[n as usize].name;
        let mut file: *const ::core::ffi::c_char = (*J).trace[n as usize].file;
        let mut line: ::core::ffi::c_int = (*J).trace[n as usize].line;
        if line > 0 as ::core::ffi::c_int {
            if *name.offset(0 as ::core::ffi::c_int as isize) != 0 {
                printf(
                    b"\tat %s (%s:%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                    name,
                    file,
                    line,
                );
            } else {
                printf(
                    b"\tat %s:%d\n\0" as *const u8 as *const ::core::ffi::c_char,
                    file,
                    line,
                );
            }
        } else {
            printf(
                b"\tat %s (%s)\n\0" as *const u8 as *const ::core::ffi::c_char,
                name,
                file,
            );
        }
        n -= 1;
    }
}
unsafe extern "C" fn js_dumpstack(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    printf(b"stack {\n\0" as *const u8 as *const ::core::ffi::c_char);
    i = 0 as ::core::ffi::c_int;
    while i < (*J).top {
        putchar(if i == (*J).bot { '>' as i32 } else { ' ' as i32 });
        printf(b"%4d: \0" as *const u8 as *const ::core::ffi::c_char, i);
        js_dumpvalue(J, *(*J).stack.offset(i as isize));
        putchar('\n' as i32);
        i += 1;
    }
    printf(b"}\n\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn js_trap(mut J: *mut js_State, mut pc: ::core::ffi::c_int) {
    js_dumpstack(J);
    js_stacktrace(J);
}
unsafe extern "C" fn jsR_isindex(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
    mut k: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut v: *mut js_Value = stackidx(J, idx);
    if (*v).t.type_0 as ::core::ffi::c_int == JS_TNUMBER as ::core::ffi::c_int {
        *k = (*v).u.number as ::core::ffi::c_int;
        return (*k as ::core::ffi::c_double == (*v).u.number
            && *k >= 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsR_run(mut J: *mut js_State, mut F: *mut js_Function) {
    let mut FT: *mut *mut js_Function = (*F).funtab;
    let mut VT: *mut *const ::core::ffi::c_char = if !(*F).vartab.is_null() {
        (*F).vartab.offset(-(1 as ::core::ffi::c_int as isize))
    } else {
        ::core::ptr::null_mut::<*const ::core::ffi::c_char>()
    };
    let mut lightweight: ::core::ffi::c_int = (*F).lightweight;
    let mut pcstart: *mut js_Instruction = (*F).code;
    let mut pc: *mut js_Instruction = (*F).code;
    let mut opcode: js_OpCode = OP_POP;
    let mut offset: ::core::ffi::c_int = 0;
    let mut savestrict: ::core::ffi::c_int = 0;
    let mut str: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut x: ::core::ffi::c_double = 0.;
    let mut y: ::core::ffi::c_double = 0.;
    let mut ux: ::core::ffi::c_uint = 0;
    let mut uy: ::core::ffi::c_uint = 0;
    let mut ix: ::core::ffi::c_int = 0;
    let mut iy: ::core::ffi::c_int = 0;
    let mut okay: ::core::ffi::c_int = 0;
    let mut b: ::core::ffi::c_int = 0;
    let mut transient: ::core::ffi::c_int = 0;
    savestrict = (*J).strict;
    (*J).strict = (*F).strict;
    loop {
        if (*J).runlimit > 0 as ::core::ffi::c_int {
            if (*J).runlimit == 1 as ::core::ffi::c_int {
                js_runlimit(J);
            }
            (*J).runlimit -= 1;
        }
        if (*J).gccounter > (*J).gcthresh {
            js_gc(J, 0 as ::core::ffi::c_int);
        }
        let fresh1 = pc;
        pc = pc.offset(1);
        (*J).trace[(*J).tracetop as usize].line = *fresh1 as ::core::ffi::c_int;
        let fresh2 = pc;
        pc = pc.offset(1);
        opcode = *fresh2 as js_OpCode;
        match opcode as ::core::ffi::c_uint {
            0 => {
                js_pop(J, 1 as ::core::ffi::c_int);
            }
            1 => {
                js_dup(J);
            }
            2 => {
                js_dup2(J);
            }
            3 => {
                js_rot2(J);
            }
            4 => {
                js_rot3(J);
            }
            5 => {
                js_rot4(J);
            }
            6 => {
                let fresh3 = pc;
                pc = pc.offset(1);
                js_pushnumber(
                    J,
                    (*fresh3 as ::core::ffi::c_int - 32768 as ::core::ffi::c_int)
                        as ::core::ffi::c_double,
                );
            }
            7 => {
                memcpy(
                    &raw mut x as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<::core::ffi::c_double>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<::core::ffi::c_double>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                js_pushnumber(J, x);
            }
            8 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                js_pushliteral(J, str);
            }
            9 => {
                let fresh4 = pc;
                pc = pc.offset(1);
                js_newfunction(J, *FT.offset(*fresh4 as isize), (*J).E);
            }
            11 => {
                js_newobject(J);
            }
            10 => {
                js_newarray(J);
            }
            12 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                let fresh5 = pc;
                pc = pc.offset(1);
                js_newregexp(J, str, *fresh5 as ::core::ffi::c_int);
            }
            13 => {
                js_pushundefined(J);
            }
            14 => {
                js_pushnull(J);
            }
            15 => {
                js_pushboolean(J, 1 as ::core::ffi::c_int);
            }
            16 => {
                js_pushboolean(J, 0 as ::core::ffi::c_int);
            }
            17 => {
                if (*J).strict != 0 {
                    js_copy(J, 0 as ::core::ffi::c_int);
                } else if js_iscoercible(J, 0 as ::core::ffi::c_int) != 0 {
                    js_copy(J, 0 as ::core::ffi::c_int);
                } else {
                    js_pushglobal(J);
                }
            }
            18 => {
                js_currentfunction(J);
            }
            19 => {
                if lightweight != 0 {
                    if (*J).top + 1 as ::core::ffi::c_int >= JS_STACKSIZE {
                        js_stackoverflow(J);
                    }
                    let fresh6 = pc;
                    pc = pc.offset(1);
                    let fresh7 = (*J).top;
                    (*J).top = (*J).top + 1;
                    *(*J).stack.offset(fresh7 as isize) = *(*J)
                        .stack
                        .offset(((*J).bot + *fresh6 as ::core::ffi::c_int) as isize);
                } else {
                    let fresh8 = pc;
                    pc = pc.offset(1);
                    str = *VT.offset(*fresh8 as isize);
                    if js_hasvar(J, str) == 0 {
                        js_referenceerror(
                            J,
                            b"'%s' is not defined\0" as *const u8
                                as *const ::core::ffi::c_char,
                            str,
                        );
                    }
                }
            }
            20 => {
                if lightweight != 0 {
                    let fresh9 = pc;
                    pc = pc.offset(1);
                    *(*J)
                        .stack
                        .offset(((*J).bot + *fresh9 as ::core::ffi::c_int) as isize) = *(*J)
                        .stack
                        .offset(((*J).top - 1 as ::core::ffi::c_int) as isize);
                } else {
                    let fresh10 = pc;
                    pc = pc.offset(1);
                    js_setvar(J, *VT.offset(*fresh10 as isize));
                }
            }
            21 => {
                if lightweight != 0 {
                    pc = pc.offset(1);
                    js_pushboolean(J, 0 as ::core::ffi::c_int);
                } else {
                    let fresh11 = pc;
                    pc = pc.offset(1);
                    b = js_delvar(J, *VT.offset(*fresh11 as isize));
                    js_pushboolean(J, b);
                }
            }
            23 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                if js_hasvar(J, str) == 0 {
                    js_referenceerror(
                        J,
                        b"'%s' is not defined\0" as *const u8
                            as *const ::core::ffi::c_char,
                        str,
                    );
                }
            }
            22 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                if js_hasvar(J, str) == 0 {
                    js_pushundefined(J);
                }
            }
            24 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                js_setvar(J, str);
            }
            25 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                b = js_delvar(J, str);
                js_pushboolean(J, b);
            }
            26 => {
                str = js_tostring(J, -(2 as ::core::ffi::c_int));
                if js_isobject(J, -(1 as ::core::ffi::c_int)) == 0 {
                    js_typeerror(
                        J,
                        b"operand to 'in' is not an object\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
                b = js_hasproperty(J, -(1 as ::core::ffi::c_int), str);
                js_pop(J, 2 as ::core::ffi::c_int + b);
                js_pushboolean(J, b);
            }
            27 => {
                js_setlength(
                    J,
                    -(1 as ::core::ffi::c_int),
                    js_getlength(J, -(1 as ::core::ffi::c_int)) + 1 as ::core::ffi::c_int,
                );
            }
            28 => {
                js_setindex(
                    J,
                    -(2 as ::core::ffi::c_int),
                    js_getlength(J, -(2 as ::core::ffi::c_int)),
                );
            }
            29 => {
                obj = js_toobject(J, -(3 as ::core::ffi::c_int));
                str = js_tostring(J, -(2 as ::core::ffi::c_int));
                jsR_setproperty(J, obj, str, 0 as ::core::ffi::c_int);
                js_pop(J, 2 as ::core::ffi::c_int);
            }
            30 => {
                obj = js_toobject(J, -(3 as ::core::ffi::c_int));
                str = js_tostring(J, -(2 as ::core::ffi::c_int));
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0 as ::core::ffi::c_int,
                    ::core::ptr::null_mut::<js_Value>(),
                    jsR_tofunction(J, -(1 as ::core::ffi::c_int)),
                    ::core::ptr::null_mut::<js_Object>(),
                    0 as ::core::ffi::c_int,
                );
                js_pop(J, 2 as ::core::ffi::c_int);
            }
            31 => {
                obj = js_toobject(J, -(3 as ::core::ffi::c_int));
                str = js_tostring(J, -(2 as ::core::ffi::c_int));
                jsR_defproperty(
                    J,
                    obj,
                    str,
                    0 as ::core::ffi::c_int,
                    ::core::ptr::null_mut::<js_Value>(),
                    ::core::ptr::null_mut::<js_Object>(),
                    jsR_tofunction(J, -(1 as ::core::ffi::c_int)),
                    0 as ::core::ffi::c_int,
                );
                js_pop(J, 2 as ::core::ffi::c_int);
            }
            32 => {
                if jsR_isindex(J, -(1 as ::core::ffi::c_int), &raw mut ix) != 0 {
                    obj = js_toobject(J, -(2 as ::core::ffi::c_int));
                    jsR_getindex(J, obj, ix);
                } else {
                    str = js_tostring(J, -(1 as ::core::ffi::c_int));
                    obj = js_toobject(J, -(2 as ::core::ffi::c_int));
                    jsR_getproperty(J, obj, str);
                }
                js_rot3pop2(J);
            }
            33 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                obj = js_toobject(J, -(1 as ::core::ffi::c_int));
                jsR_getproperty(J, obj, str);
                js_rot2pop1(J);
            }
            34 => {
                if jsR_isindex(J, -(2 as ::core::ffi::c_int), &raw mut ix) != 0 {
                    obj = js_toobject(J, -(3 as ::core::ffi::c_int));
                    transient = (js_isobject(J, -(3 as ::core::ffi::c_int)) == 0)
                        as ::core::ffi::c_int;
                    jsR_setindex(J, obj, ix, transient);
                } else {
                    str = js_tostring(J, -(2 as ::core::ffi::c_int));
                    obj = js_toobject(J, -(3 as ::core::ffi::c_int));
                    transient = (js_isobject(J, -(3 as ::core::ffi::c_int)) == 0)
                        as ::core::ffi::c_int;
                    jsR_setproperty(J, obj, str, transient);
                }
                js_rot3pop2(J);
            }
            35 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                obj = js_toobject(J, -(2 as ::core::ffi::c_int));
                transient = (js_isobject(J, -(2 as ::core::ffi::c_int)) == 0)
                    as ::core::ffi::c_int;
                jsR_setproperty(J, obj, str, transient);
                js_rot2pop1(J);
            }
            36 => {
                str = js_tostring(J, -(1 as ::core::ffi::c_int));
                obj = js_toobject(J, -(2 as ::core::ffi::c_int));
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, b);
            }
            37 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                obj = js_toobject(J, -(1 as ::core::ffi::c_int));
                b = jsR_delproperty(J, obj, str);
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushboolean(J, b);
            }
            38 => {
                if js_iscoercible(J, -(1 as ::core::ffi::c_int)) != 0 {
                    obj = jsV_newiterator(
                        J,
                        js_toobject(J, -(1 as ::core::ffi::c_int)),
                        0 as ::core::ffi::c_int,
                    );
                    js_pop(J, 1 as ::core::ffi::c_int);
                    js_pushobject(J, obj);
                }
            }
            39 => {
                if js_isobject(J, -(1 as ::core::ffi::c_int)) != 0 {
                    obj = js_toobject(J, -(1 as ::core::ffi::c_int));
                    str = jsV_nextiterator(J, obj);
                    if !str.is_null() {
                        js_pushstring(J, str);
                        js_pushboolean(J, 1 as ::core::ffi::c_int);
                    } else {
                        js_pop(J, 1 as ::core::ffi::c_int);
                        js_pushboolean(J, 0 as ::core::ffi::c_int);
                    }
                } else {
                    js_pop(J, 1 as ::core::ffi::c_int);
                    js_pushboolean(J, 0 as ::core::ffi::c_int);
                }
            }
            40 => {
                js_eval(J);
            }
            41 => {
                let fresh12 = pc;
                pc = pc.offset(1);
                js_call(J, *fresh12 as ::core::ffi::c_int);
            }
            42 => {
                let fresh13 = pc;
                pc = pc.offset(1);
                js_construct(J, *fresh13 as ::core::ffi::c_int);
            }
            43 => {
                str = js_typeof(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushliteral(J, str);
            }
            44 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, x);
            }
            45 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, -x);
            }
            46 => {
                ix = js_toint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, !ix as ::core::ffi::c_double);
            }
            47 => {
                b = js_toboolean(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushboolean(J, (b == 0) as ::core::ffi::c_int);
            }
            48 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, x + 1 as ::core::ffi::c_int as ::core::ffi::c_double);
            }
            49 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, x - 1 as ::core::ffi::c_int as ::core::ffi::c_double);
            }
            50 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, x + 1 as ::core::ffi::c_int as ::core::ffi::c_double);
                js_pushnumber(J, x);
            }
            51 => {
                x = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                js_pushnumber(J, x - 1 as ::core::ffi::c_int as ::core::ffi::c_double);
                js_pushnumber(J, x);
            }
            52 => {
                x = js_tonumber(J, -(2 as ::core::ffi::c_int));
                y = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, x * y);
            }
            53 => {
                x = js_tonumber(J, -(2 as ::core::ffi::c_int));
                y = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, x / y);
            }
            54 => {
                x = js_tonumber(J, -(2 as ::core::ffi::c_int));
                y = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, fmod(x, y));
            }
            55 => {
                js_concat(J);
            }
            56 => {
                x = js_tonumber(J, -(2 as ::core::ffi::c_int));
                y = js_tonumber(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, x - y);
            }
            57 => {
                ix = js_toint32(J, -(2 as ::core::ffi::c_int));
                uy = js_touint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(
                    J,
                    (ix << (uy & 0x1f as ::core::ffi::c_uint)) as ::core::ffi::c_double,
                );
            }
            58 => {
                ix = js_toint32(J, -(2 as ::core::ffi::c_int));
                uy = js_touint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(
                    J,
                    (ix >> (uy & 0x1f as ::core::ffi::c_uint)) as ::core::ffi::c_double,
                );
            }
            59 => {
                ux = js_touint32(J, -(2 as ::core::ffi::c_int));
                uy = js_touint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(
                    J,
                    (ux >> (uy & 0x1f as ::core::ffi::c_uint)) as ::core::ffi::c_double,
                );
            }
            60 => {
                b = js_compare(J, &raw mut okay);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(
                    J,
                    (okay != 0 && b < 0 as ::core::ffi::c_int) as ::core::ffi::c_int,
                );
            }
            61 => {
                b = js_compare(J, &raw mut okay);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(
                    J,
                    (okay != 0 && b > 0 as ::core::ffi::c_int) as ::core::ffi::c_int,
                );
            }
            62 => {
                b = js_compare(J, &raw mut okay);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(
                    J,
                    (okay != 0 && b <= 0 as ::core::ffi::c_int) as ::core::ffi::c_int,
                );
            }
            63 => {
                b = js_compare(J, &raw mut okay);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(
                    J,
                    (okay != 0 && b >= 0 as ::core::ffi::c_int) as ::core::ffi::c_int,
                );
            }
            72 => {
                b = js_instanceof(J);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, b);
            }
            64 => {
                b = js_equal(J);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, b);
            }
            65 => {
                b = js_equal(J);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, (b == 0) as ::core::ffi::c_int);
            }
            66 => {
                b = js_strictequal(J);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, b);
            }
            67 => {
                b = js_strictequal(J);
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushboolean(J, (b == 0) as ::core::ffi::c_int);
            }
            68 => {
                let fresh14 = pc;
                pc = pc.offset(1);
                offset = *fresh14 as ::core::ffi::c_int;
                b = js_strictequal(J);
                if b != 0 {
                    js_pop(J, 2 as ::core::ffi::c_int);
                    pc = pcstart.offset(offset as isize);
                } else {
                    js_pop(J, 1 as ::core::ffi::c_int);
                }
            }
            69 => {
                ix = js_toint32(J, -(2 as ::core::ffi::c_int));
                iy = js_toint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, (ix & iy) as ::core::ffi::c_double);
            }
            70 => {
                ix = js_toint32(J, -(2 as ::core::ffi::c_int));
                iy = js_toint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, (ix ^ iy) as ::core::ffi::c_double);
            }
            71 => {
                ix = js_toint32(J, -(2 as ::core::ffi::c_int));
                iy = js_toint32(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 2 as ::core::ffi::c_int);
                js_pushnumber(J, (ix | iy) as ::core::ffi::c_double);
            }
            73 => {
                js_throw(J);
            }
            74 => {
                let fresh15 = pc;
                pc = pc.offset(1);
                offset = *fresh15 as ::core::ffi::c_int;
                if _setjmp(js_savetrypc(J, pc) as *mut __jmp_buf_tag) != 0 {
                    pc = (*J).trybuf[(*J).trytop as usize].pc;
                } else {
                    pc = pcstart.offset(offset as isize);
                }
            }
            75 => {
                js_endtry(J);
            }
            76 => {
                memcpy(
                    &raw mut str as *mut ::core::ffi::c_void,
                    pc as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<*const ::core::ffi::c_char>() as size_t,
                );
                pc = pc
                    .offset(
                        (::core::mem::size_of::<*const ::core::ffi::c_char>() as usize)
                            .wrapping_div(
                                ::core::mem::size_of::<js_Instruction>() as usize,
                            ) as isize,
                    );
                obj = jsV_newobject(J, JS_COBJECT, ::core::ptr::null_mut::<js_Object>());
                js_pushobject(J, obj);
                js_rot2(J);
                js_setproperty(J, -(2 as ::core::ffi::c_int), str);
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1 as ::core::ffi::c_int);
            }
            77 => {
                (*J).E = (*(*J).E).outer;
            }
            78 => {
                obj = js_toobject(J, -(1 as ::core::ffi::c_int));
                (*J).E = jsR_newenvironment(J, obj, (*J).E);
                js_pop(J, 1 as ::core::ffi::c_int);
            }
            79 => {
                (*J).E = (*(*J).E).outer;
            }
            80 => {
                js_trap(
                    J,
                    pc.offset_from(pcstart) as ::core::ffi::c_long as ::core::ffi::c_int
                        - 1 as ::core::ffi::c_int,
                );
            }
            81 => {
                pc = pcstart.offset(*pc as ::core::ffi::c_int as isize);
            }
            82 => {
                let fresh16 = pc;
                pc = pc.offset(1);
                offset = *fresh16 as ::core::ffi::c_int;
                b = js_toboolean(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                if b != 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }
            83 => {
                let fresh17 = pc;
                pc = pc.offset(1);
                offset = *fresh17 as ::core::ffi::c_int;
                b = js_toboolean(J, -(1 as ::core::ffi::c_int));
                js_pop(J, 1 as ::core::ffi::c_int);
                if b == 0 {
                    pc = pcstart.offset(offset as isize);
                }
            }
            84 => {
                (*J).strict = savestrict;
                return;
            }
            _ => {}
        }
    };
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
