extern "C" {
    pub type js_StringNode;
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    pub type Reprog;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn js_savetry(J: *mut js_State) -> *mut ::core::ffi::c_void;
    fn js_endtry(J: *mut js_State);
    fn js_error(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_rangeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_typeerror(J: *mut js_State, fmt: *const ::core::ffi::c_char, ...) -> !;
    fn js_throw(J: *mut js_State) -> !;
    fn js_call(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_defglobal(
        J: *mut js_State,
        name: *const ::core::ffi::c_char,
        atts: ::core::ffi::c_int,
    );
    fn js_setindex(J: *mut js_State, idx: ::core::ffi::c_int, i: ::core::ffi::c_int);
    fn js_pushundefined(J: *mut js_State);
    fn js_pushnull(J: *mut js_State);
    fn js_pushnumber(J: *mut js_State, v: ::core::ffi::c_double);
    fn js_pushstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_pushlstring(
        J: *mut js_State,
        v: *const ::core::ffi::c_char,
        n: ::core::ffi::c_int,
    );
    fn js_pushliteral(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_newarray(J: *mut js_State);
    fn js_newstring(J: *mut js_State, v: *const ::core::ffi::c_char);
    fn js_newcconstructor(
        J: *mut js_State,
        fun: js_CFunction,
        con: js_CFunction,
        name: *const ::core::ffi::c_char,
        length: ::core::ffi::c_int,
    );
    fn js_newregexp(
        J: *mut js_State,
        pattern: *const ::core::ffi::c_char,
        flags: ::core::ffi::c_int,
    );
    fn js_isdefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isundefined(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_isregexp(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscoercible(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_iscallable(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_tostring(
        J: *mut js_State,
        idx: ::core::ffi::c_int,
    ) -> *const ::core::ffi::c_char;
    fn js_tointeger(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn js_touint32(J: *mut js_State, idx: ::core::ffi::c_int) -> ::core::ffi::c_uint;
    fn js_gettop(J: *mut js_State) -> ::core::ffi::c_int;
    fn js_pop(J: *mut js_State, n: ::core::ffi::c_int);
    fn js_copy(J: *mut js_State, idx: ::core::ffi::c_int);
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
    fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn jsU_chartorune(
        rune: *mut Rune,
        str: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn jsU_runetochar(
        str: *mut ::core::ffi::c_char,
        rune: *const Rune,
    ) -> ::core::ffi::c_int;
    fn jsU_runelen(c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn jsU_tolowerrune(c: Rune) -> Rune;
    fn jsU_toupperrune(c: Rune) -> Rune;
    fn jsU_tolowerrune_full(c: Rune) -> *const Rune;
    fn jsU_toupperrune_full(c: Rune) -> *const Rune;
    fn js_regexec(
        prog: *mut Reprog,
        string: *const ::core::ffi::c_char,
        sub: *mut Resub,
        eflags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn js_malloc(J: *mut js_State, size: ::core::ffi::c_int) -> *mut ::core::ffi::c_void;
    fn js_realloc(
        J: *mut js_State,
        ptr: *mut ::core::ffi::c_void,
        size: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_void;
    fn js_free(J: *mut js_State, ptr: *mut ::core::ffi::c_void);
    fn js_toregexp(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Regexp;
    fn js_RegExp_prototype_exec(
        J: *mut js_State,
        re: *mut js_Regexp,
        text: *const ::core::ffi::c_char,
    );
    fn js_putc(J: *mut js_State, sbp: *mut *mut js_Buffer, c: ::core::ffi::c_int);
    fn js_puts(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const ::core::ffi::c_char);
    fn js_putm(
        J: *mut js_State,
        sb: *mut *mut js_Buffer,
        s: *const ::core::ffi::c_char,
        e: *const ::core::ffi::c_char,
    );
    fn js_toobject(J: *mut js_State, idx: ::core::ffi::c_int) -> *mut js_Object;
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
pub const JS_REGEXP_M: C2RustUnnamed_9 = 4;
pub const JS_REGEXP_I: C2RustUnnamed_9 = 2;
pub const JS_REGEXP_G: C2RustUnnamed_9 = 1;
pub type C2RustUnnamed_10 = ::core::ffi::c_uint;
pub const JS_DONTCONF: C2RustUnnamed_10 = 4;
pub const JS_DONTENUM: C2RustUnnamed_10 = 2;
pub const JS_READONLY: C2RustUnnamed_10 = 1;
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
pub type Rune = ::core::ffi::c_int;
pub const Runeself: C2RustUnnamed_12 = 128;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct js_Buffer {
    pub n: ::core::ffi::c_int,
    pub m: ::core::ffi::c_int,
    pub s: [::core::ffi::c_char; 64],
}
pub const UTFmax: C2RustUnnamed_12 = 4;
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
pub const REG_NOTBOL: C2RustUnnamed_13 = 4;
pub type C2RustUnnamed_12 = ::core::ffi::c_uint;
pub const Runemax: C2RustUnnamed_12 = 1114111;
pub const Runeerror: C2RustUnnamed_12 = 65533;
pub const Runesync: C2RustUnnamed_12 = 128;
pub type C2RustUnnamed_13 = ::core::ffi::c_uint;
pub const REG_NEWLINE: C2RustUnnamed_13 = 2;
pub const REG_ICASE: C2RustUnnamed_13 = 1;
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
unsafe extern "C" fn js_doregexec(
    mut J: *mut js_State,
    mut prog: *mut Reprog,
    mut string: *const ::core::ffi::c_char,
    mut sub: *mut Resub,
    mut eflags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = js_regexec(prog, string, sub, eflags);
    if result < 0 as ::core::ffi::c_int {
        js_error(J, b"regexec failed\0" as *const u8 as *const ::core::ffi::c_char);
    }
    return result;
}
unsafe extern "C" fn checkstring(
    mut J: *mut js_State,
    mut idx: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    if js_iscoercible(J, idx) == 0 {
        js_typeerror(
            J,
            b"string function called on null or undefined\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    return js_tostring(J, idx);
}
#[no_mangle]
pub unsafe extern "C" fn js_runeat(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
    mut i: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut rune: Rune = EOF;
    while i >= 0 as ::core::ffi::c_int {
        rune = *(s as *mut ::core::ffi::c_uchar) as Rune;
        if rune < Runeself as ::core::ffi::c_int {
            if rune == 0 as ::core::ffi::c_int {
                return EOF;
            }
            s = s.offset(1);
            i -= 1;
        } else {
            s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
            if rune >= 0x10000 as ::core::ffi::c_int {
                i -= 2 as ::core::ffi::c_int;
            } else {
                i -= 1;
            }
        }
    }
    if rune >= 0x10000 as ::core::ffi::c_int {
        if i == -(2 as ::core::ffi::c_int) {
            return 0xd800 as ::core::ffi::c_int
                + (rune as ::core::ffi::c_int - 0x10000 as ::core::ffi::c_int
                    >> 10 as ::core::ffi::c_int)
        } else {
            return 0xdc00 as ::core::ffi::c_int
                + (rune as ::core::ffi::c_int - 0x10000 as ::core::ffi::c_int
                    & 0x3ff as ::core::ffi::c_int)
        }
    }
    return rune as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn js_utflen(
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut c: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    let mut rune: Rune = 0;
    n = 0 as ::core::ffi::c_int;
    loop {
        c = *(s as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int;
        if c < Runeself as ::core::ffi::c_int {
            if c == 0 as ::core::ffi::c_int {
                return n;
            }
            s = s.offset(1);
            n += 1;
        } else {
            s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
            if rune >= 0x10000 as ::core::ffi::c_int {
                n += 2 as ::core::ffi::c_int;
            } else {
                n += 1;
            }
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_utfptrtoidx(
    mut s: *const ::core::ffi::c_char,
    mut p: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut rune: Rune = 0;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while s < p {
        if (*(s as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int)
            < Runeself as ::core::ffi::c_int
        {
            s = s.offset(1);
            i += 1;
        } else {
            s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
            if rune >= 0x10000 as ::core::ffi::c_int {
                i += 2 as ::core::ffi::c_int;
            } else {
                i += 1 as ::core::ffi::c_int;
            }
        }
    }
    return i;
}
unsafe extern "C" fn jsB_new_String(mut J: *mut js_State) {
    js_newstring(
        J,
        if js_gettop(J) > 1 as ::core::ffi::c_int {
            js_tostring(J, 1 as ::core::ffi::c_int)
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
}
unsafe extern "C" fn jsB_String(mut J: *mut js_State) {
    js_pushstring(
        J,
        if js_gettop(J) > 1 as ::core::ffi::c_int {
            js_tostring(J, 1 as ::core::ffi::c_int)
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
}
unsafe extern "C" fn Sp_toString(mut J: *mut js_State) {
    let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    if (*self_0).type_0 as ::core::ffi::c_uint
        != JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_typeerror(J, b"not a string\0" as *const u8 as *const ::core::ffi::c_char);
    }
    js_pushstring(J, (*self_0).u.s.string);
}
unsafe extern "C" fn Sp_valueOf(mut J: *mut js_State) {
    let mut self_0: *mut js_Object = js_toobject(J, 0 as ::core::ffi::c_int);
    if (*self_0).type_0 as ::core::ffi::c_uint
        != JS_CSTRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        js_typeerror(J, b"not a string\0" as *const u8 as *const ::core::ffi::c_char);
    }
    js_pushstring(J, (*self_0).u.s.string);
}
unsafe extern "C" fn Sp_charAt(mut J: *mut js_State) {
    let mut buf: [::core::ffi::c_char; 5] = [0; 5];
    let mut s: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut pos: ::core::ffi::c_int = js_tointeger(J, 1 as ::core::ffi::c_int);
    let mut rune: Rune = js_runeat(J, s, pos) as Rune;
    if rune >= 0 as ::core::ffi::c_int {
        buf[jsU_runetochar(&raw mut buf as *mut ::core::ffi::c_char, &raw mut rune)
            as usize] = 0 as ::core::ffi::c_char;
        js_pushstring(J, &raw mut buf as *mut ::core::ffi::c_char);
    } else {
        js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn Sp_charCodeAt(mut J: *mut js_State) {
    let mut s: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut pos: ::core::ffi::c_int = js_tointeger(J, 1 as ::core::ffi::c_int);
    let mut rune: Rune = js_runeat(J, s, pos) as Rune;
    if rune >= 0 as ::core::ffi::c_int {
        js_pushnumber(J, rune as ::core::ffi::c_double);
    } else {
        js_pushnumber(J, ::core::f32::NAN as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Sp_concat(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut n: ::core::ffi::c_int = 0;
    let mut out: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    if top == 1 as ::core::ffi::c_int {
        return;
    }
    s = checkstring(J, 0 as ::core::ffi::c_int);
    n = (1 as size_t).wrapping_add(strlen(s)) as ::core::ffi::c_int;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, out as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    if n > JS_STRLIMIT {
        js_rangeerror(
            J,
            b"invalid string length\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    ::core::ptr::write_volatile(
        &mut out as *mut *mut ::core::ffi::c_char,
        js_malloc(J, n) as *mut ::core::ffi::c_char,
    );
    strcpy(out, s);
    i = 1 as ::core::ffi::c_int;
    while i < top {
        s = js_tostring(J, i);
        n = (n as ::core::ffi::c_ulong).wrapping_add(strlen(s) as ::core::ffi::c_ulong)
            as ::core::ffi::c_int as ::core::ffi::c_int;
        if n > JS_STRLIMIT {
            js_rangeerror(
                J,
                b"invalid string length\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        ::core::ptr::write_volatile(
            &mut out as *mut *mut ::core::ffi::c_char,
            js_realloc(J, out as *mut ::core::ffi::c_void, n) as *mut ::core::ffi::c_char,
        );
        strcat(out, s);
        i += 1;
    }
    js_pushstring(J, out);
    js_endtry(J);
    js_free(J, out as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_indexOf(mut J: *mut js_State) {
    let mut haystack: *const ::core::ffi::c_char = checkstring(
        J,
        0 as ::core::ffi::c_int,
    );
    let mut needle: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    let mut pos: ::core::ffi::c_int = js_tointeger(J, 2 as ::core::ffi::c_int);
    let mut len: ::core::ffi::c_int = strlen(needle) as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rune: Rune = 0;
    while *haystack != 0 {
        if k >= pos && strncmp(haystack, needle, len as size_t) == 0 {
            js_pushnumber(J, k as ::core::ffi::c_double);
            return;
        }
        haystack = haystack.offset(jsU_chartorune(&raw mut rune, haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, -(1 as ::core::ffi::c_int) as ::core::ffi::c_double);
}
unsafe extern "C" fn Sp_lastIndexOf(mut J: *mut js_State) {
    let mut haystack: *const ::core::ffi::c_char = checkstring(
        J,
        0 as ::core::ffi::c_int,
    );
    let mut needle: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    let mut pos: ::core::ffi::c_int = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        strlen(haystack) as ::core::ffi::c_int
    };
    let mut len: ::core::ffi::c_int = strlen(needle) as ::core::ffi::c_int;
    let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut last: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut rune: Rune = 0;
    while *haystack as ::core::ffi::c_int != 0 && k <= pos {
        if strncmp(haystack, needle, len as size_t) == 0 {
            last = k;
        }
        haystack = haystack.offset(jsU_chartorune(&raw mut rune, haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, last as ::core::ffi::c_double);
}
unsafe extern "C" fn Sp_localeCompare(mut J: *mut js_State) {
    let mut a: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut b: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    js_pushnumber(J, strcmp(a, b) as ::core::ffi::c_double);
}
unsafe extern "C" fn Sp_substring_imp(
    mut J: *mut js_State,
    mut s: *const ::core::ffi::c_char,
    mut a: ::core::ffi::c_int,
    mut n: ::core::ffi::c_int,
) {
    let mut head_rune: Rune = 0 as Rune;
    let mut tail_rune: Rune = 0 as Rune;
    let mut head: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut tail: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut i: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    let mut head_len: ::core::ffi::c_int = 0;
    let mut tail_len: ::core::ffi::c_int = 0;
    head = s;
    i = 0 as ::core::ffi::c_int;
    while i < a {
        head = head.offset(jsU_chartorune(&raw mut head_rune, head) as isize);
        if head_rune >= 0x10000 as ::core::ffi::c_int {
            i += 1;
        }
        i += 1;
    }
    tail = head;
    k = i - a;
    while k < n {
        tail = tail.offset(jsU_chartorune(&raw mut tail_rune, tail) as isize);
        if tail_rune >= 0x10000 as ::core::ffi::c_int {
            k += 1;
        }
        k += 1;
    }
    if i == a && k == n {
        js_pushlstring(
            J,
            head,
            tail.offset_from(head) as ::core::ffi::c_long as ::core::ffi::c_int,
        );
        return;
    }
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, p as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    p = js_malloc(
        J,
        (UTFmax as ::core::ffi::c_int as ::core::ffi::c_long
            + tail.offset_from(head) as ::core::ffi::c_long) as ::core::ffi::c_int,
    ) as *mut ::core::ffi::c_char;
    if i > a {
        head_rune = (0xdc00 as ::core::ffi::c_int
            + (head_rune as ::core::ffi::c_int - 0x10000 as ::core::ffi::c_int
                & 0x3ff as ::core::ffi::c_int)) as Rune;
        head_len = jsU_runetochar(p, &raw mut head_rune);
        memcpy(
            p.offset(head_len as isize) as *mut ::core::ffi::c_void,
            head as *const ::core::ffi::c_void,
            tail.offset_from(head) as ::core::ffi::c_long as size_t,
        );
        js_pushlstring(
            J,
            p,
            (head_len as ::core::ffi::c_long
                + tail.offset_from(head) as ::core::ffi::c_long) as ::core::ffi::c_int,
        );
    }
    if k > n {
        tail = tail.offset(-(jsU_runelen(tail_rune as ::core::ffi::c_int) as isize));
        memcpy(
            p as *mut ::core::ffi::c_void,
            head as *const ::core::ffi::c_void,
            tail.offset_from(head) as ::core::ffi::c_long as size_t,
        );
        tail_rune = (0xd800 as ::core::ffi::c_int
            + (tail_rune as ::core::ffi::c_int - 0x10000 as ::core::ffi::c_int
                >> 10 as ::core::ffi::c_int)) as Rune;
        tail_len = jsU_runetochar(
            p.offset(tail.offset_from(head) as ::core::ffi::c_long as isize),
            &raw mut tail_rune,
        );
        js_pushlstring(
            J,
            p,
            (tail.offset_from(head) as ::core::ffi::c_long
                + tail_len as ::core::ffi::c_long) as ::core::ffi::c_int,
        );
    }
    js_endtry(J);
    js_free(J, p as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_slice(mut J: *mut js_State) {
    let mut str: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut len: ::core::ffi::c_int = js_utflen(str);
    let mut s: ::core::ffi::c_int = js_tointeger(J, 1 as ::core::ffi::c_int);
    let mut e: ::core::ffi::c_int = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        len
    };
    s = if s < 0 as ::core::ffi::c_int { s + len } else { s };
    e = if e < 0 as ::core::ffi::c_int { e + len } else { e };
    s = if s < 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else if s > len {
        len
    } else {
        s
    };
    e = if e < 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else if e > len {
        len
    } else {
        e
    };
    if s < e {
        Sp_substring_imp(J, str, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str, e, s - e);
    } else {
        js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn Sp_substring(mut J: *mut js_State) {
    let mut str: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut len: ::core::ffi::c_int = js_utflen(str);
    let mut s: ::core::ffi::c_int = js_tointeger(J, 1 as ::core::ffi::c_int);
    let mut e: ::core::ffi::c_int = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        len
    };
    s = if s < 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else if s > len {
        len
    } else {
        s
    };
    e = if e < 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else if e > len {
        len
    } else {
        e
    };
    if s < e {
        Sp_substring_imp(J, str, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str, e, s - e);
    } else {
        js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
unsafe extern "C" fn Sp_toLowerCase(mut J: *mut js_State) {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut s0: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut dst: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut d: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut rune: Rune = 0;
    let mut full: *const Rune = ::core::ptr::null::<Rune>();
    let mut n: ::core::ffi::c_int = 0;
    n = 1 as ::core::ffi::c_int;
    s = s0;
    while *s != 0 {
        s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
        full = jsU_tolowerrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += jsU_runelen(*full);
                full = full.offset(1);
            }
        } else {
            rune = jsU_tolowerrune(rune);
            n += jsU_runelen(rune as ::core::ffi::c_int);
        }
    }
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, dst as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    ::core::ptr::write_volatile(
        &mut dst as *mut *mut ::core::ffi::c_char,
        js_malloc(J, n) as *mut ::core::ffi::c_char,
    );
    d = ::core::ptr::read_volatile::<
        *mut ::core::ffi::c_char,
    >(&dst as *const *mut ::core::ffi::c_char);
    s = s0;
    while *s != 0 {
        s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
        full = jsU_tolowerrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                d = d.offset(jsU_runetochar(d, full) as isize);
                full = full.offset(1);
            }
        } else {
            rune = jsU_tolowerrune(rune);
            d = d.offset(jsU_runetochar(d, &raw mut rune) as isize);
        }
    }
    *d = 0 as ::core::ffi::c_char;
    js_pushstring(J, dst);
    js_endtry(J);
    js_free(J, dst as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_toUpperCase(mut J: *mut js_State) {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut s0: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut dst: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<
        ::core::ffi::c_char,
    >();
    let mut d: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut full: *const Rune = ::core::ptr::null::<Rune>();
    let mut rune: Rune = 0;
    let mut n: ::core::ffi::c_int = 0;
    n = 1 as ::core::ffi::c_int;
    s = s0;
    while *s != 0 {
        s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
        full = jsU_toupperrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += jsU_runelen(*full);
                full = full.offset(1);
            }
        } else {
            rune = jsU_toupperrune(rune);
            n += jsU_runelen(rune as ::core::ffi::c_int);
        }
    }
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, dst as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    ::core::ptr::write_volatile(
        &mut dst as *mut *mut ::core::ffi::c_char,
        js_malloc(J, n) as *mut ::core::ffi::c_char,
    );
    d = ::core::ptr::read_volatile::<
        *mut ::core::ffi::c_char,
    >(&dst as *const *mut ::core::ffi::c_char);
    s = s0;
    while *s != 0 {
        s = s.offset(jsU_chartorune(&raw mut rune, s) as isize);
        full = jsU_toupperrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                d = d.offset(jsU_runetochar(d, full) as isize);
                full = full.offset(1);
            }
        } else {
            rune = jsU_toupperrune(rune);
            d = d.offset(jsU_runetochar(d, &raw mut rune) as isize);
        }
    }
    *d = 0 as ::core::ffi::c_char;
    js_pushstring(J, dst);
    js_endtry(J);
    js_free(J, dst as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn isbol(
    mut re: *mut js_Regexp,
    mut text: *const ::core::ffi::c_char,
    mut a: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return (a == text
        || (*re).flags as ::core::ffi::c_int & JS_REGEXP_M as ::core::ffi::c_int != 0
            && *a.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                == '\n' as i32) as ::core::ffi::c_int;
}
unsafe extern "C" fn istrim(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c == 0x9 as ::core::ffi::c_int || c == 0xb as ::core::ffi::c_int
        || c == 0xc as ::core::ffi::c_int || c == 0x20 as ::core::ffi::c_int
        || c == 0xa0 as ::core::ffi::c_int || c == 0xfeff as ::core::ffi::c_int
        || c == 0xa as ::core::ffi::c_int || c == 0xd as ::core::ffi::c_int
        || c == 0x2028 as ::core::ffi::c_int || c == 0x2029 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn Sp_trim(mut J: *mut js_State) {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut e: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    s = checkstring(J, 0 as ::core::ffi::c_int);
    while istrim(*s as ::core::ffi::c_int) != 0 {
        s = s.offset(1);
    }
    e = s.offset(strlen(s) as isize);
    while e > s
        && istrim(*e.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int)
            != 0
    {
        e = e.offset(-1);
    }
    js_pushlstring(J, s, e.offset_from(s) as ::core::ffi::c_long as ::core::ffi::c_int);
}
unsafe extern "C" fn S_fromCharCode(mut J: *mut js_State) {
    let mut i: ::core::ffi::c_int = 0;
    let mut top: ::core::ffi::c_int = js_gettop(J);
    let mut s: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: Rune = 0;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, s as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    p = js_malloc(
        J,
        (top - 1 as ::core::ffi::c_int) * UTFmax as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int,
    ) as *mut ::core::ffi::c_char;
    ::core::ptr::write_volatile(&mut s as *mut *mut ::core::ffi::c_char, p);
    i = 1 as ::core::ffi::c_int;
    while i < top {
        c = js_touint32(J, i) as Rune;
        p = p.offset(jsU_runetochar(p, &raw mut c) as isize);
        i += 1;
    }
    *p = 0 as ::core::ffi::c_char;
    js_pushstring(J, s);
    js_endtry(J);
    js_free(J, s as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_match(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut text: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut len: ::core::ffi::c_int = 0;
    let mut a: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut b: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut c: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut e: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    let mut rune: Rune = 0;
    text = checkstring(J, 0 as ::core::ffi::c_int);
    if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        js_copy(J, 1 as ::core::ffi::c_int);
    } else if js_isundefined(J, 1 as ::core::ffi::c_int) != 0 {
        js_newregexp(
            J,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
    } else {
        js_newregexp(
            J,
            js_tostring(J, 1 as ::core::ffi::c_int),
            0 as ::core::ffi::c_int,
        );
    }
    re = js_toregexp(J, -(1 as ::core::ffi::c_int));
    if (*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int == 0 {
        js_RegExp_prototype_exec(J, re, text);
        return;
    }
    (*re).last = 0 as ::core::ffi::c_ushort;
    js_newarray(J);
    len = 0 as ::core::ffi::c_int;
    a = text;
    e = text.offset(strlen(text) as isize);
    while a <= e {
        if js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            a,
            &raw mut m,
            if isbol(re, text, a) != 0 {
                0 as ::core::ffi::c_int
            } else {
                REG_NOTBOL as ::core::ffi::c_int
            },
        ) != 0
        {
            break;
        }
        b = m.sub[0 as ::core::ffi::c_int as usize].sp;
        c = m.sub[0 as ::core::ffi::c_int as usize].ep;
        js_pushlstring(
            J,
            b,
            c.offset_from(b) as ::core::ffi::c_long as ::core::ffi::c_int,
        );
        let fresh11 = len;
        len = len + 1;
        js_setindex(J, -(2 as ::core::ffi::c_int), fresh11);
        a = c;
        if c.offset_from(b) as ::core::ffi::c_long == 0 as ::core::ffi::c_long {
            a = a.offset(jsU_chartorune(&raw mut rune, a) as isize);
        }
    }
    if len == 0 as ::core::ffi::c_int {
        js_pop(J, 1 as ::core::ffi::c_int);
        js_pushnull(J);
    }
}
unsafe extern "C" fn Sp_search(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut text: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    text = checkstring(J, 0 as ::core::ffi::c_int);
    if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        js_copy(J, 1 as ::core::ffi::c_int);
    } else if js_isundefined(J, 1 as ::core::ffi::c_int) != 0 {
        js_newregexp(
            J,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        );
    } else {
        js_newregexp(
            J,
            js_tostring(J, 1 as ::core::ffi::c_int),
            0 as ::core::ffi::c_int,
        );
    }
    re = js_toregexp(J, -(1 as ::core::ffi::c_int));
    if js_doregexec(
        J,
        (*re).prog as *mut Reprog,
        text,
        &raw mut m,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        js_pushnumber(
            J,
            js_utfptrtoidx(text, m.sub[0 as ::core::ffi::c_int as usize].sp)
                as ::core::ffi::c_double,
        );
    } else {
        js_pushnumber(J, -(1 as ::core::ffi::c_int) as ::core::ffi::c_double);
    };
}
unsafe extern "C" fn Sp_replace_regexp(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut source: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut source0: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut r: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut sb: *mut js_Buffer = ::core::ptr::null_mut::<js_Buffer>();
    let mut n: ::core::ffi::c_int = 0;
    let mut x: ::core::ffi::c_int = 0;
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    source0 = checkstring(J, 0 as ::core::ffi::c_int);
    source = source0;
    re = js_toregexp(J, 1 as ::core::ffi::c_int);
    if js_doregexec(
        J,
        (*re).prog as *mut Reprog,
        source,
        &raw mut m,
        0 as ::core::ffi::c_int,
    ) != 0
    {
        js_copy(J, 0 as ::core::ffi::c_int);
        return;
    }
    (*re).last = 0 as ::core::ffi::c_ushort;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, sb as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    loop {
        s = m.sub[0 as ::core::ffi::c_int as usize].sp;
        n = m
            .sub[0 as ::core::ffi::c_int as usize]
            .ep
            .offset_from(m.sub[0 as ::core::ffi::c_int as usize].sp)
            as ::core::ffi::c_long as ::core::ffi::c_int;
        if js_iscallable(J, 2 as ::core::ffi::c_int) != 0 {
            js_copy(J, 2 as ::core::ffi::c_int);
            js_pushundefined(J);
            x = 0 as ::core::ffi::c_int;
            while !m.sub[x as usize].sp.is_null() {
                js_pushlstring(
                    J,
                    m.sub[x as usize].sp,
                    m.sub[x as usize].ep.offset_from(m.sub[x as usize].sp)
                        as ::core::ffi::c_long as ::core::ffi::c_int,
                );
                x += 1;
            }
            js_pushnumber(
                J,
                s.offset_from(source) as ::core::ffi::c_long as ::core::ffi::c_double,
            );
            js_copy(J, 0 as ::core::ffi::c_int);
            js_call(J, 2 as ::core::ffi::c_int + x);
            r = js_tostring(J, -(1 as ::core::ffi::c_int));
            js_putm(J, &raw mut sb, source, s);
            js_puts(J, &raw mut sb, r);
            js_pop(J, 1 as ::core::ffi::c_int);
        } else {
            r = js_tostring(J, 2 as ::core::ffi::c_int);
            js_putm(J, &raw mut sb, source, s);
            while *r != 0 {
                if *r as ::core::ffi::c_int == '$' as i32 {
                    let mut current_block_48: u64;
                    r = r.offset(1);
                    match *r as ::core::ffi::c_int {
                        0 => {
                            r = r.offset(-1);
                            current_block_48 = 12444791199452447874;
                        }
                        36 => {
                            current_block_48 = 12444791199452447874;
                        }
                        96 => {
                            js_putm(J, &raw mut sb, source0, s);
                            current_block_48 = 10150597327160359210;
                        }
                        39 => {
                            js_puts(J, &raw mut sb, s.offset(n as isize));
                            current_block_48 = 10150597327160359210;
                        }
                        38 => {
                            js_putm(J, &raw mut sb, s, s.offset(n as isize));
                            current_block_48 = 10150597327160359210;
                        }
                        48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                            x = *r as ::core::ffi::c_int - '0' as i32;
                            if *r.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int >= '0' as i32
                                && *r.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int <= '9' as i32
                            {
                                r = r.offset(1);
                                x = x * 10 as ::core::ffi::c_int + *r as ::core::ffi::c_int
                                    - '0' as i32;
                            }
                            if x > 0 as ::core::ffi::c_int && x < m.nsub {
                                js_putm(
                                    J,
                                    &raw mut sb,
                                    m.sub[x as usize].sp,
                                    m.sub[x as usize].ep,
                                );
                            } else {
                                js_putc(J, &raw mut sb, '$' as i32);
                                if x > 10 as ::core::ffi::c_int {
                                    js_putc(
                                        J,
                                        &raw mut sb,
                                        '0' as i32 + x / 10 as ::core::ffi::c_int,
                                    );
                                    js_putc(
                                        J,
                                        &raw mut sb,
                                        '0' as i32 + x % 10 as ::core::ffi::c_int,
                                    );
                                } else {
                                    js_putc(J, &raw mut sb, '0' as i32 + x);
                                }
                            }
                            current_block_48 = 10150597327160359210;
                        }
                        _ => {
                            js_putc(J, &raw mut sb, '$' as i32);
                            js_putc(J, &raw mut sb, *r as ::core::ffi::c_int);
                            current_block_48 = 10150597327160359210;
                        }
                    }
                    match current_block_48 {
                        12444791199452447874 => {
                            js_putc(J, &raw mut sb, '$' as i32);
                        }
                        _ => {}
                    }
                    r = r.offset(1);
                } else {
                    let fresh9 = r;
                    r = r.offset(1);
                    js_putc(J, &raw mut sb, *fresh9 as ::core::ffi::c_int);
                }
            }
        }
        if !((*re).flags as ::core::ffi::c_int & JS_REGEXP_G as ::core::ffi::c_int != 0)
        {
            break;
        }
        source = m.sub[0 as ::core::ffi::c_int as usize].ep;
        if n == 0 as ::core::ffi::c_int {
            if !(*source != 0) {
                break;
            }
            let fresh10 = source;
            source = source.offset(1);
            js_putc(J, &raw mut sb, *fresh10 as ::core::ffi::c_int);
        }
        if !(js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            source,
            &raw mut m,
            if isbol(re, source0, source) != 0 {
                0 as ::core::ffi::c_int
            } else {
                REG_NOTBOL as ::core::ffi::c_int
            },
        ) == 0)
        {
            break;
        }
    }
    js_puts(J, &raw mut sb, s.offset(n as isize));
    js_putc(J, &raw mut sb, 0 as ::core::ffi::c_int);
    js_pushstring(
        J,
        if !sb.is_null() {
            &raw mut (*sb).s as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_replace_string(mut J: *mut js_State) {
    let mut source: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut needle: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut r: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut sb: *mut js_Buffer = ::core::ptr::null_mut::<js_Buffer>();
    let mut n: ::core::ffi::c_int = 0;
    source = checkstring(J, 0 as ::core::ffi::c_int);
    needle = js_tostring(J, 1 as ::core::ffi::c_int);
    s = strstr(source, needle);
    if s.is_null() {
        js_copy(J, 0 as ::core::ffi::c_int);
        return;
    }
    n = strlen(needle) as ::core::ffi::c_int;
    if _setjmp(js_savetry(J) as *mut __jmp_buf_tag) != 0 {
        js_free(J, sb as *mut ::core::ffi::c_void);
        js_throw(J);
    }
    if js_iscallable(J, 2 as ::core::ffi::c_int) != 0 {
        js_copy(J, 2 as ::core::ffi::c_int);
        js_pushundefined(J);
        js_pushlstring(J, s, n);
        js_pushnumber(
            J,
            s.offset_from(source) as ::core::ffi::c_long as ::core::ffi::c_double,
        );
        js_copy(J, 0 as ::core::ffi::c_int);
        js_call(J, 3 as ::core::ffi::c_int);
        r = js_tostring(J, -(1 as ::core::ffi::c_int));
        js_putm(J, &raw mut sb, source, s);
        js_puts(J, &raw mut sb, r);
        js_puts(J, &raw mut sb, s.offset(n as isize));
        js_putc(J, &raw mut sb, 0 as ::core::ffi::c_int);
        js_pop(J, 1 as ::core::ffi::c_int);
    } else {
        r = js_tostring(J, 2 as ::core::ffi::c_int);
        js_putm(J, &raw mut sb, source, s);
        while *r != 0 {
            if *r as ::core::ffi::c_int == '$' as i32 {
                let mut current_block_33: u64;
                r = r.offset(1);
                match *r as ::core::ffi::c_int {
                    0 => {
                        r = r.offset(-1);
                        current_block_33 = 3335424463289723891;
                    }
                    36 => {
                        current_block_33 = 3335424463289723891;
                    }
                    38 => {
                        js_putm(J, &raw mut sb, s, s.offset(n as isize));
                        current_block_33 = 4488286894823169796;
                    }
                    96 => {
                        js_putm(J, &raw mut sb, source, s);
                        current_block_33 = 4488286894823169796;
                    }
                    39 => {
                        js_puts(J, &raw mut sb, s.offset(n as isize));
                        current_block_33 = 4488286894823169796;
                    }
                    _ => {
                        js_putc(J, &raw mut sb, '$' as i32);
                        js_putc(J, &raw mut sb, *r as ::core::ffi::c_int);
                        current_block_33 = 4488286894823169796;
                    }
                }
                match current_block_33 {
                    3335424463289723891 => {
                        js_putc(J, &raw mut sb, '$' as i32);
                    }
                    _ => {}
                }
                r = r.offset(1);
            } else {
                let fresh8 = r;
                r = r.offset(1);
                js_putc(J, &raw mut sb, *fresh8 as ::core::ffi::c_int);
            }
        }
        js_puts(J, &raw mut sb, s.offset(n as isize));
        js_putc(J, &raw mut sb, 0 as ::core::ffi::c_int);
    }
    js_pushstring(
        J,
        if !sb.is_null() {
            &raw mut (*sb).s as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn Sp_replace(mut J: *mut js_State) {
    if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        Sp_replace_regexp(J);
    } else {
        Sp_replace_string(J);
    };
}
unsafe extern "C" fn Sp_split_regexp(mut J: *mut js_State) {
    let mut re: *mut js_Regexp = ::core::ptr::null_mut::<js_Regexp>();
    let mut text: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    let mut limit: ::core::ffi::c_int = 0;
    let mut len: ::core::ffi::c_int = 0;
    let mut k: ::core::ffi::c_int = 0;
    let mut p: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut a: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut b: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut c: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut e: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut m: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed_11 {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    let mut rune: Rune = 0;
    text = checkstring(J, 0 as ::core::ffi::c_int);
    re = js_toregexp(J, 1 as ::core::ffi::c_int);
    limit = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0 {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        (1 as ::core::ffi::c_int) << 30 as ::core::ffi::c_int
    };
    js_newarray(J);
    len = 0 as ::core::ffi::c_int;
    if limit == 0 as ::core::ffi::c_int {
        return;
    }
    e = text.offset(strlen(text) as isize);
    if e == text {
        if js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            text,
            &raw mut m,
            0 as ::core::ffi::c_int,
        ) != 0
        {
            js_pushliteral(J, b"\0" as *const u8 as *const ::core::ffi::c_char);
            js_setindex(J, -(2 as ::core::ffi::c_int), 0 as ::core::ffi::c_int);
        }
        return;
    }
    a = text;
    p = a;
    while a < e {
        if js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            a,
            &raw mut m,
            if isbol(re, text, a) != 0 {
                0 as ::core::ffi::c_int
            } else {
                REG_NOTBOL as ::core::ffi::c_int
            },
        ) != 0
        {
            break;
        }
        b = m.sub[0 as ::core::ffi::c_int as usize].sp;
        c = m.sub[0 as ::core::ffi::c_int as usize].ep;
        if b == c && b == p {
            a = a.offset(jsU_chartorune(&raw mut rune, a) as isize);
        } else {
            if len == limit {
                return;
            }
            js_pushlstring(
                J,
                p,
                b.offset_from(p) as ::core::ffi::c_long as ::core::ffi::c_int,
            );
            let fresh6 = len;
            len = len + 1;
            js_setindex(J, -(2 as ::core::ffi::c_int), fresh6);
            k = 1 as ::core::ffi::c_int;
            while k < m.nsub {
                if len == limit {
                    return;
                }
                js_pushlstring(
                    J,
                    m.sub[k as usize].sp,
                    m.sub[k as usize].ep.offset_from(m.sub[k as usize].sp)
                        as ::core::ffi::c_long as ::core::ffi::c_int,
                );
                let fresh7 = len;
                len = len + 1;
                js_setindex(J, -(2 as ::core::ffi::c_int), fresh7);
                k += 1;
            }
            p = c;
            a = p;
        }
    }
    if len == limit {
        return;
    }
    js_pushstring(J, p);
    js_setindex(J, -(2 as ::core::ffi::c_int), len);
}
unsafe extern "C" fn Sp_split_string(mut J: *mut js_State) {
    let mut str: *const ::core::ffi::c_char = checkstring(J, 0 as ::core::ffi::c_int);
    let mut sep: *const ::core::ffi::c_char = js_tostring(J, 1 as ::core::ffi::c_int);
    let mut limit: ::core::ffi::c_int = if js_isdefined(J, 2 as ::core::ffi::c_int) != 0
    {
        js_tointeger(J, 2 as ::core::ffi::c_int)
    } else {
        (1 as ::core::ffi::c_int) << 30 as ::core::ffi::c_int
    };
    let mut i: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    js_newarray(J);
    if limit == 0 as ::core::ffi::c_int {
        return;
    }
    n = strlen(sep) as ::core::ffi::c_int;
    if n == 0 as ::core::ffi::c_int {
        let mut rune: Rune = 0;
        i = 0 as ::core::ffi::c_int;
        while *str as ::core::ffi::c_int != 0 && i < limit {
            n = jsU_chartorune(&raw mut rune, str);
            js_pushlstring(J, str, n);
            js_setindex(J, -(2 as ::core::ffi::c_int), i);
            str = str.offset(n as isize);
            i += 1;
        }
        return;
    }
    i = 0 as ::core::ffi::c_int;
    while !str.is_null() && i < limit {
        let mut s: *const ::core::ffi::c_char = strstr(str, sep);
        if !s.is_null() {
            js_pushlstring(
                J,
                str,
                s.offset_from(str) as ::core::ffi::c_long as ::core::ffi::c_int,
            );
            js_setindex(J, -(2 as ::core::ffi::c_int), i);
            str = s.offset(n as isize);
        } else {
            js_pushstring(J, str);
            js_setindex(J, -(2 as ::core::ffi::c_int), i);
            str = ::core::ptr::null::<::core::ffi::c_char>();
        }
        i += 1;
    }
}
unsafe extern "C" fn Sp_split(mut J: *mut js_State) {
    if js_isundefined(J, 1 as ::core::ffi::c_int) != 0 {
        js_newarray(J);
        js_pushstring(J, js_tostring(J, 0 as ::core::ffi::c_int));
        js_setindex(J, -(2 as ::core::ffi::c_int), 0 as ::core::ffi::c_int);
    } else if js_isregexp(J, 1 as ::core::ffi::c_int) != 0 {
        Sp_split_regexp(J);
    } else {
        Sp_split_string(J);
    };
}
#[no_mangle]
pub unsafe extern "C" fn jsB_initstring(mut J: *mut js_State) {
    (*(*J).String_prototype).u.s.shrstr[0 as ::core::ffi::c_int as usize] = 0
        as ::core::ffi::c_char;
    (*(*J).String_prototype).u.s.string = &raw mut (*(*J).String_prototype).u.s.shrstr
        as *mut ::core::ffi::c_char;
    (*(*J).String_prototype).u.s.length = 0 as ::core::ffi::c_int;
    js_pushobject(J, (*J).String_prototype);
    jsB_propf(
        J,
        b"String.prototype.toString\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_toString as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.valueOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_valueOf as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.charAt\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_charAt as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.charCodeAt\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_charCodeAt as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.concat\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_concat as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.indexOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_indexOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.lastIndexOf\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_lastIndexOf as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.localeCompare\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_localeCompare as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.match\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_match as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.replace\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_replace as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.search\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_search as unsafe extern "C" fn(*mut js_State) -> ()),
        1 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.slice\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_slice as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.split\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_split as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.substring\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_substring as unsafe extern "C" fn(*mut js_State) -> ()),
        2 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.toLowerCase\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_toLowerCase as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.toLocaleLowerCase\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Sp_toLowerCase as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.toUpperCase\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_toUpperCase as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.toLocaleUpperCase\0" as *const u8
            as *const ::core::ffi::c_char,
        Some(Sp_toUpperCase as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.prototype.trim\0" as *const u8 as *const ::core::ffi::c_char,
        Some(Sp_trim as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    js_newcconstructor(
        J,
        Some(jsB_String as unsafe extern "C" fn(*mut js_State) -> ()),
        Some(jsB_new_String as unsafe extern "C" fn(*mut js_State) -> ()),
        b"String\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    );
    jsB_propf(
        J,
        b"String.fromCharCode\0" as *const u8 as *const ::core::ffi::c_char,
        Some(S_fromCharCode as unsafe extern "C" fn(*mut js_State) -> ()),
        0 as ::core::ffi::c_int,
    );
    js_defglobal(
        J,
        b"String\0" as *const u8 as *const ::core::ffi::c_char,
        JS_DONTENUM as ::core::ffi::c_int,
    );
}
pub const JS_STRLIMIT: ::core::ffi::c_int = (1 as ::core::ffi::c_int)
    << 28 as ::core::ffi::c_int;
