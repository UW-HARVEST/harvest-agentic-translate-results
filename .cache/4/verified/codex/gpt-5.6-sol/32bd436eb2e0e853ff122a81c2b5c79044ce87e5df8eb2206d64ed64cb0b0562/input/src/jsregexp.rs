extern "C" {
    pub type js_StringNode;
    pub type Reprog;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn js_savetry(J: *mut js_State) -> *mut ::core::ffi::c_void;
    fn js_endtry(J: *mut js_State);
    fn js_error(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_syntaxerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_throw(J: *mut js_State) -> !;
    fn js_defglobal(
        J: *mut js_State,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_setproperty(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
        name: *const ::core::ffi::c_char,
    );
    fn js_setindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_pushnull(J: *mut js_State);
    fn js_pushboolean(J: *mut js_State, v: ::core::ffi::c_int);
    fn js_pushnumber(J: *mut js_State, v: ::core::ffi::c_double);
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_pushlstring(
        J: *mut js_State,
        v: *const ::core::ffi::c_char,
        n: ::core::ffi::c_int,
    );
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
    fn js_isregexp(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
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
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn js_regcompx(
        alloc: Option<
            unsafe extern "C" fn(
                *mut ::core::ffi::c_void,
                *mut ::core::ffi::c_void,
                ::core::ffi::c_int,
            ) -> *mut ::core::ffi::c_void,
        >,
        ctx: *mut ::core::ffi::c_void,
        pattern: *const ::core::ffi::c_char,
        cflags: ::core::ffi::c_int,
        errorp: *mut *const ::core::ffi::c_char,
    ) -> *mut Reprog;
    fn js_regexec(
        prog: *mut Reprog,
        string: *const ::core::ffi::c_char,
        sub: *mut Resub,
        eflags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_strdup(
        J: *mut js_State,
        s: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn js_toregexp(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Regexp;
    fn js_utfptrtoidx(
        s: *const ::core::ffi::c_char,
        p: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
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
pub const JS_REGEXP_M: C2RustUnnamed_9 = 4;
pub const JS_REGEXP_I: C2RustUnnamed_9 = 2;
pub const JS_REGEXP_G: C2RustUnnamed_9 = 1;
pub type C2RustUnnamed_10 = ::core::ffi::c_uint;
pub const JS_DONTCONF: C2RustUnnamed_10 = 4;
pub const JS_DONTENUM: C2RustUnnamed_10 = 2;
pub const JS_READONLY: C2RustUnnamed_10 = 1;
pub const REG_NEWLINE: C2RustUnnamed_12 = 2;
pub const REG_ICASE: C2RustUnnamed_12 = 1;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_11 {
    pub sp: *const ::core::ffi::c_char,
    pub ep: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Resub {
    pub nsub: ::core::ffi::c_int,
    pub sub: [C2RustUnnamed_11; 16],
}
pub const REG_NOTBOL: C2RustUnnamed_12 = 4;
pub type C2RustUnnamed_12 = ::core::ffi::c_uint;
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
        let fresh4 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh4 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
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
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh3 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh3 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
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
        let fresh5 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh5 = __c as ::core::ffi::c_char;
        *fresh5 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh6 = (*__stream)._IO_write_ptr;
        (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
        *fresh6 = __c as ::core::ffi::c_char;
        *fresh6 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
        let fresh7 = (*stdout)._IO_write_ptr;
        (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
        *fresh7 = __c as ::core::ffi::c_char;
        *fresh7 as ::core::ffi::c_uchar as ::core::ffi::c_int
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
unsafe extern "C" fn escaperegexp(
    mut J: *mut js_State,
    mut pattern: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut copy: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    s = pattern;
    while *s != 0 {
        if *s as ::core::ffi::c_int == '/' as i32 {
            n += 1;
        }
        n += 1;
        s = s.offset(1);
    }
    p = js_malloc(J, n + 1 as ::core::ffi::c_int) as *mut ::core::ffi::c_char;
    copy = p;
    s = pattern;
    while *s != 0 {
        if *s as ::core::ffi::c_int == '/' as i32 {
            let fresh0 = p;
            p = p.offset(1);
            *fresh0 = '\\' as i32 as ::core::ffi::c_char;
        }
        let fresh1 = p;
        p = p.offset(1);
        *fresh1 = *s;
        s = s.offset(1);
    }
    *p = 0 as ::core::ffi::c_char;
    return copy;
}
unsafe extern "C" fn js_newregexpx(
    mut J: *mut js_State,
    mut pattern: *const ::core::ffi::c_char,
    mut flags: ::core::ffi::c_int,
    mut is_clone: ::core::ffi::c_int,
) {
    let mut error: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut obj: *mut js_Object = ::core::ptr::null_mut::<js_Object>();
    let mut prog: *mut Reprog = ::core::ptr::null_mut::<Reprog>();
    let mut opts: ::core::ffi::c_int = 0;
    obj = jsV_newobject(J, JS_CREGEXP, (*J).RegExp_prototype);
    opts = 0 as ::core::ffi::c_int;
    if flags & JS_REGEXP_I as ::core::ffi::c_int != 0 {
        opts |= REG_ICASE as ::core::ffi::c_int;
    }
    if flags & JS_REGEXP_M as ::core::ffi::c_int != 0 {
        opts |= REG_NEWLINE as ::core::ffi::c_int;
    }
    prog = js_regcompx(
        (*J).alloc
            as Option<
                unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *mut ::core::ffi::c_void,
                    ::core::ffi::c_int,
                ) -> *mut ::core::ffi::c_void,
            >,
        (*J).actx,
        pattern,
        opts,
        &raw mut error,
    );
    if prog.is_null() {
        js_syntaxerror(
            J,
            b"regular expression: %s\0" as *const u8 as *const ::core::ffi::c_char,
            error,
        );
    }
    (*obj).u.r.prog = prog as *mut ::core::ffi::c_void;
    (*obj).u.r.source = if is_clone != 0 {
        js_strdup(J, pattern)
    } else {
        escaperegexp(J, pattern)
    };
    (*obj).u.r.flags = flags as ::core::ffi::c_ushort;
    (*obj).u.r.last = 0 as ::core::ffi::c_ushort;
    js_pushobject(J, obj);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newregexp(
    mut J: *mut js_State,
    mut pattern: *const ::core::ffi::c_char,
    mut flags: ::core::ffi::c_int,
) {
    js_newregexpx(J, pattern, flags, 0 as ::core::ffi::c_int);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_RegExp_prototype_exec(
    mut J: *mut js_State,
    mut re: *mut js_Regexp,
    mut text: *const ::core::ffi::c_char,
) {
    let mut haystack: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut result: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut opts: ::core::ffi::c_int = 0;
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    haystack = text;
    opts = 0 as ::core::ffi::c_int;
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
        if (*re).last as size_t > strlen(haystack) {
            (*re).last = 0 as ::core::ffi::c_ushort;
            js_pushnull(J);
            return;
        }
        if (*re).last as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            haystack = text.offset((*re).last as ::core::ffi::c_int as isize);
            if (*re).flags as ::core::ffi::c_int & JS_REGEXP_M as ::core::ffi::c_int == 0
                || *haystack.offset(-(1 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int != '\n' as i32
            {
                opts |= REG_NOTBOL as ::core::ffi::c_int;
            }
        }
    }
    result = js_regexec((*re).prog as *mut Reprog, haystack, &raw mut m, opts);
    if result < 0 as ::core::ffi::c_int {
        js_error(J, b"regexec failed\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if result == 0 as ::core::ffi::c_int {
        js_newarray(J);
        js_pushstring(J, text);
        js_setproperty(
            J,
            -(2 as ::core::ffi::c_int),
            b"input\0" as *const u8 as *const ::core::ffi::c_char,
        );
        js_pushnumber(
            J,
            js_utfptrtoidx(text, m.sub[0 as ::core::ffi::c_int as usize].sp)
                as ::core::ffi::c_double,
        );
        js_setproperty(
            J,
            -(2 as ::core::ffi::c_int),
            b"index\0" as *const u8 as *const ::core::ffi::c_char,
        );
        i = 0 as ::core::ffi::c_int;
        while i < m.nsub {
            js_pushlstring(
                J,
                m.sub[i as usize].sp,
                m.sub[i as usize].ep.offset_from(m.sub[i as usize].sp)
                    as ::core::ffi::c_long as ::core::ffi::c_int,
            );
            js_setindex(J, -(2 as ::core::ffi::c_int), i);
            i += 1;
        }
        if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
            (*re).last = m.sub[0 as ::core::ffi::c_int as usize].ep.offset_from(text)
                as ::core::ffi::c_long as ::core::ffi::c_ushort;
        }
        return;
    }
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
        (*re).last = 0 as ::core::ffi::c_ushort;
    }
    js_pushnull(J);
}
unsafe extern "C" fn Rp_test(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut text: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut result: ::core::ffi::c_int = 0;
    let mut opts: ::core::ffi::c_int = 0;
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    re = js_toregexp(J, 0 as ::core::ffi::c_int);
    text = js_tostring(J, 1 as ::core::ffi::c_int);
    opts = 0 as ::core::ffi::c_int;
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
        if (*re).last as size_t > strlen(text) {
            (*re).last = 0 as ::core::ffi::c_ushort;
            js_pushboolean(J, 0 as ::core::ffi::c_int);
            return;
        }
        if (*re).last as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            text = text.offset((*re).last as ::core::ffi::c_int as isize);
            if (*re).flags as ::core::ffi::c_int & JS_REGEXP_M as ::core::ffi::c_int == 0
                || *text.offset(-(1 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int != '\n' as i32
            {
                opts |= REG_NOTBOL as ::core::ffi::c_int;
            }
        }
    }
    result = js_regexec((*re).prog as *mut Reprog, text, &raw mut m, opts);
    if result < 0 as ::core::ffi::c_int {
        js_error(J, b"regexec failed\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if result == 0 as ::core::ffi::c_int {
        if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
            (*re).last = ((*re).last as ::core::ffi::c_long
                + m.sub[0 as ::core::ffi::c_int as usize].ep.offset_from(text)
                    as ::core::ffi::c_long) as ::core::ffi::c_ushort;
        }
        js_pushboolean(J, 1 as ::core::ffi::c_int);
        return;
    }
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
        (*re).last = 0 as ::core::ffi::c_ushort;
    }
    js_pushboolean(J, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn jsB_new_RegExp(mut J: *mut js_State) {
    let mut old: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut pattern: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut flags: ::core::ffi::c_int = 0;
    let mut is_clone: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
            js_typeerror(
                J,
                b"cannot supply flags when creating one RegExp from another\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
        }
        old = js_toregexp(J, 1 as ::core::ffi::c_int);
        pattern = (*old).source;
        flags = (*old).flags as ::core::ffi::c_int;
        is_clone = 1 as ::core::ffi::c_int;
    } else if js_isundefined(J, 1 as ::core::ffi::c_int) != 0 {
        pattern = b"(?:)\0" as *const u8 as *const ::core::ffi::c_char;
        flags = 0 as ::core::ffi::c_int;
    } else {
        pattern = js_tostring(J, 1 as ::core::ffi::c_int);
        flags = 0 as ::core::ffi::c_int;
    }
    if strlen(pattern) == 0 as size_t {
        pattern = b"(?:)\0" as *const u8 as *const ::core::ffi::c_char;
    }
    if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        let mut s: *const ::core::ffi::c_char = js_tostring(J, 2 as ::core::ffi::c_int);
        let mut g: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut m: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while *s != 0 {
            if *s as ::core::ffi::c_int == 'g' as i32 {
                g += 1;
            } else if *s as ::core::ffi::c_int == 'i' as i32 {
                i += 1;
            } else if *s as ::core::ffi::c_int == 'm' as i32 {
                m += 1;
            } else {
                js_syntaxerror(
                    J,
                    b"invalid regular expression flag: '%c'\0" as *const u8
                        as *const ::core::ffi::c_char,
                    *s as ::core::ffi::c_int,
                );
            }
            s = s.offset(1);
        }
        if g > 1 as ::core::ffi::c_int {
            js_syntaxerror(
                J,
                b"invalid regular expression flag: 'g'\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if i > 1 as ::core::ffi::c_int {
            js_syntaxerror(
                J,
                b"invalid regular expression flag: 'i'\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if m > 1 as ::core::ffi::c_int {
            js_syntaxerror(
                J,
                b"invalid regular expression flag: 'm'\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if g != 0 {
            flags |= JS_REGEXP_G as ::core::ffi::c_int;
        }
        if i != 0 {
            flags |= JS_REGEXP_I as ::core::ffi::c_int;
        }
        if m != 0 {
            flags |= JS_REGEXP_M as ::core::ffi::c_int;
        }
    }
    js_newregexpx(J, pattern, flags, is_clone);
}
unsafe extern "C" fn jsB_RegExp(mut J: *mut js_State) {
    if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        return;
    }
    jsB_new_RegExp(J);
}
unsafe extern "C" fn Rp_toString(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut out: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    re = js_toregexp(J, 0 as ::core::ffi::c_int);
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, out as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    ::core::ptr::write_volatile(
        &mut out as *mut *mut ::core::ffi::c_char,
        js_malloc(
            J,
            strlen((*re).source).wrapping_add(6 as size_t) as ::core::ffi::c_int,
        ) as *mut ::core::ffi::c_char,
    );
    strcpy(out, b"/\0" as *const u8 as *const ::core::ffi::c_char);
    strcat(out, (*re).source);
    strcat(out, b"/\0" as *const u8 as *const ::core::ffi::c_char);
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0 {
        strcat(out, b"g\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_I as ::core::ffi::c_int != 0 {
        strcat(out, b"i\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_M as ::core::ffi::c_int != 0 {
        strcat(out, b"m\0" as *const u8 as *const ::core::ffi::c_char);
    }
    js_pop(J, 0 as ::core::ffi::c_int);
    js_pushstring(J, out);
    js_endtry(J);
    js_free(J, out as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Rp_exec(mut J: *mut js_State) {
    js_RegExp_prototype_exec(
        J,
        js_toregexp(J, 0 as ::core::ffi::c_int),
        js_tostring(J, 1 as ::core::ffi::c_int),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initregexp(mut J: *mut js_State) {
    js_pushobject(J, (*J).RegExp_prototype);
    jsB_propf(
        J,
        b"RegExp.prototype.toString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Rp_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"RegExp.prototype.test\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Rp_test as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"RegExp.prototype.exec\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Rp_exec as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    js_newcconstructor(
        J,
        Some(jsB_RegExp as unsafe extern "C" fn(*mut js_State) -> ()),
        Some(jsB_new_RegExp as unsafe extern "C" fn(*mut js_State) -> ()),
        b"RegExp\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
    );
    js_defglobal(
        J,
        b"RegExp\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
}
