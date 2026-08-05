//! Translated from jscompile.c — AST -> bytecode compiler.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::{js_free, js_malloc, js_realloc};
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// jsC_error variadic: rs_ impl called by shim (jsC_error_shim(J, node, fmt, ...)).
#[no_mangle]
pub unsafe extern "C-unwind" fn rs_jsC_error(J: *mut js_State, node: *mut c_void, msg: *const c_char) {
    let node = node as *mut js_Ast;
    let mut buf: [c_char; 512] = [0; 512];
    libc::snprintf(buf.as_mut_ptr(), 256, cstr!("%s:%d: "), (*J).filename, (*node).line);
    strcat(buf.as_mut_ptr(), msg);
    crate::jserror::js_newsyntaxerror(J, buf.as_ptr());
    crate::jsrun::js_throw(J);
}

macro_rules! jsC_error {
    ($J:expr, $node:expr, $($arg:tt)*) => {
        crate::jserror::jsC_error($J, $node as *mut c_void, $($arg)*)
    };
}

const SHRLEN_INSTR: usize = std::mem::size_of::<*const c_char>() / std::mem::size_of::<js_Instruction>();

struct Words7([*const c_char; 7]);
unsafe impl Sync for Words7 {}
struct Words9([*const c_char; 9]);
unsafe impl Sync for Words9 {}

static FUTUREWORDS: Words7 = Words7([
    b"class\0".as_ptr() as *const c_char,
    b"const\0".as_ptr() as *const c_char,
    b"enum\0".as_ptr() as *const c_char,
    b"export\0".as_ptr() as *const c_char,
    b"extends\0".as_ptr() as *const c_char,
    b"import\0".as_ptr() as *const c_char,
    b"super\0".as_ptr() as *const c_char,
]);

static STRICTFUTUREWORDS: Words9 = Words9([
    b"implements\0".as_ptr() as *const c_char,
    b"interface\0".as_ptr() as *const c_char,
    b"let\0".as_ptr() as *const c_char,
    b"package\0".as_ptr() as *const c_char,
    b"private\0".as_ptr() as *const c_char,
    b"protected\0".as_ptr() as *const c_char,
    b"public\0".as_ptr() as *const c_char,
    b"static\0".as_ptr() as *const c_char,
    b"yield\0".as_ptr() as *const c_char,
]);

struct SyncArr(*const *const c_char);
unsafe impl Sync for SyncArr {}
static FW: SyncArr = SyncArr(FUTUREWORDS.0.as_ptr());
static SFW: SyncArr = SyncArr(STRICTFUTUREWORDS.0.as_ptr());

unsafe fn checkfutureword(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    if crate::jslex::jsY_findword((*exp).string, FW.0, 7) >= 0 {
        jsC_error!(J, exp, cstr!("'%s' is a future reserved word"), (*exp).string);
    }
    if (*F).strict != 0 {
        if crate::jslex::jsY_findword((*exp).string, SFW.0, 9) >= 0 {
            jsC_error!(J, exp, cstr!("'%s' is a strict mode future reserved word"), (*exp).string);
        }
    }
}

unsafe fn newfun(J: *mut js_State, line: c_int, name: *mut js_Ast, params: *mut js_Ast, body: *mut js_Ast, script: c_int, default_strict: c_int, is_fun_exp: c_int) -> *mut js_Function {
    let F = js_malloc(J, std::mem::size_of::<js_Function>() as c_int) as *mut js_Function;
    libc::memset(F as *mut c_void, 0, std::mem::size_of::<js_Function>());
    (*F).gcmark = 0;
    (*F).gcnext = (*J).gcfun;
    (*J).gcfun = F;
    (*J).gccounter += 1;

    (*F).filename = crate::jsintern::js_intern(J, (*J).filename);
    (*F).line = line;
    (*F).script = script;
    (*F).strict = default_strict;
    (*F).name = if !name.is_null() { (*name).string } else { cstr!("") };

    cfunbody(J, F, name, params, body, is_fun_exp);

    F
}

/* Emit */
unsafe fn emitraw(J: *mut js_State, F: *mut js_Function, value: c_int) {
    if value != (value as js_Instruction) as c_int {
        crate::jserror::js_syntaxerror(J, cstr!("integer overflow in instruction coding"));
    }
    if (*F).codelen >= (*F).codecap {
        (*F).codecap = if (*F).codecap != 0 { (*F).codecap * 2 } else { 64 };
        (*F).code = js_realloc(J, (*F).code as *mut c_void, (*F).codecap * std::mem::size_of::<js_Instruction>() as c_int) as *mut js_Instruction;
    }
    *(*F).code.add((*F).codelen as usize) = value as js_Instruction;
    (*F).codelen += 1;
}

unsafe fn emit(J: *mut js_State, F: *mut js_Function, value: c_int) {
    emitraw(J, F, (*F).lastline);
    emitraw(J, F, value);
}

unsafe fn emitarg(J: *mut js_State, F: *mut js_Function, value: c_int) {
    emitraw(J, F, value);
}

unsafe fn emitline(J: *mut js_State, F: *mut js_Function, node: *mut js_Ast) {
    (*F).lastline = (*node).line;
}

unsafe fn addfunction(J: *mut js_State, F: *mut js_Function, value: *mut js_Function) -> c_int {
    if (*F).funlen >= (*F).funcap {
        (*F).funcap = if (*F).funcap != 0 { (*F).funcap * 2 } else { 16 };
        (*F).funtab = js_realloc(J, (*F).funtab as *mut c_void, (*F).funcap * std::mem::size_of::<*mut js_Function>() as c_int) as *mut *mut js_Function;
    }
    *(*F).funtab.add((*F).funlen as usize) = value;
    let r = (*F).funlen;
    (*F).funlen += 1;
    r
}

unsafe fn addlocal(J: *mut js_State, F: *mut js_Function, ident: *mut js_Ast, reuse: c_int) -> c_int {
    let name = (*ident).string;
    if (*F).strict != 0 {
        if strcmp(name, cstr!("arguments")) == 0 {
            jsC_error!(J, ident, cstr!("redefining 'arguments' is not allowed in strict mode"));
        }
        if strcmp(name, cstr!("eval")) == 0 {
            jsC_error!(J, ident, cstr!("redefining 'eval' is not allowed in strict mode"));
        }
    } else {
        if strcmp(name, cstr!("eval")) == 0 {
            crate::jserror::js_evalerror(J, cstr!("%s:%d: invalid use of 'eval'"), (*J).filename, (*ident).line);
        }
    }
    if reuse != 0 || (*F).strict != 0 {
        let mut i = 0;
        while i < (*F).varlen {
            if strcmp(*(*F).vartab.add(i as usize), name) == 0 {
                if reuse != 0 {
                    return i + 1;
                }
                if (*F).strict != 0 {
                    jsC_error!(J, ident, cstr!("duplicate formal parameter '%s'"), name);
                }
            }
            i += 1;
        }
    }
    if (*F).varlen >= (*F).varcap {
        (*F).varcap = if (*F).varcap != 0 { (*F).varcap * 2 } else { 16 };
        (*F).vartab = js_realloc(J, (*F).vartab as *mut c_void, (*F).varcap * std::mem::size_of::<*const c_char>() as c_int) as *mut *const c_char;
    }
    *(*F).vartab.add((*F).varlen as usize) = name;
    (*F).varlen += 1;
    (*F).varlen
}

unsafe fn findlocal(J: *mut js_State, F: *mut js_Function, name: *const c_char) -> c_int {
    let mut i = (*F).varlen;
    while i > 0 {
        if strcmp(*(*F).vartab.add((i - 1) as usize), name) == 0 {
            return i;
        }
        i -= 1;
    }
    -1
}

unsafe fn emitfunction(J: *mut js_State, F: *mut js_Function, fun: *mut js_Function) {
    (*F).lightweight = 0;
    emit(J, F, OP_CLOSURE);
    let n = addfunction(J, F, fun);
    emitarg(J, F, n);
}

unsafe fn emitnumber(J: *mut js_State, F: *mut js_Function, num: f64) {
    if num == 0.0 {
        emit(J, F, OP_INTEGER);
        emitarg(J, F, 32768);
        if num.is_sign_negative() {
            emit(J, F, OP_NEG);
        }
    } else if num >= i16::MIN as f64 && num <= i16::MAX as f64 && num == (num as c_int) as f64 {
        emit(J, F, OP_INTEGER);
        emitarg(J, F, num as c_int + 32768);
    } else {
        const N: usize = std::mem::size_of::<f64>() / std::mem::size_of::<js_Instruction>();
        let mut x: [js_Instruction; N] = [0; N];
        emit(J, F, OP_NUMBER);
        libc::memcpy(x.as_mut_ptr() as *mut c_void, &num as *const f64 as *const c_void, std::mem::size_of::<f64>());
        let mut i = 0;
        while i < N {
            emitarg(J, F, x[i] as c_int);
            i += 1;
        }
    }
}

unsafe fn emitstring(J: *mut js_State, F: *mut js_Function, opcode: c_int, str: *const c_char) {
    const N: usize = std::mem::size_of::<*const c_char>() / std::mem::size_of::<js_Instruction>();
    let mut x: [js_Instruction; N] = [0; N];
    emit(J, F, opcode);
    libc::memcpy(x.as_mut_ptr() as *mut c_void, &str as *const *const c_char as *const c_void, std::mem::size_of::<*const c_char>());
    let mut i = 0;
    while i < N {
        emitarg(J, F, x[i] as c_int);
        i += 1;
    }
}

unsafe fn emitlocal(J: *mut js_State, F: *mut js_Function, oploc: c_int, opvar: c_int, ident: *mut js_Ast) {
    let is_arguments = (strcmp((*ident).string, cstr!("arguments")) == 0) as c_int;
    let is_eval = (strcmp((*ident).string, cstr!("eval")) == 0) as c_int;
    let i;

    if is_arguments != 0 {
        (*F).lightweight = 0;
        (*F).arguments = 1;
    }

    checkfutureword(J, F, ident);
    if (*F).strict != 0 && oploc == OP_SETLOCAL {
        if is_arguments != 0 {
            jsC_error!(J, ident, cstr!("'arguments' is read-only in strict mode"));
        }
        if is_eval != 0 {
            jsC_error!(J, ident, cstr!("'eval' is read-only in strict mode"));
        }
    }
    if is_eval != 0 {
        crate::jserror::js_evalerror(J, cstr!("%s:%d: invalid use of 'eval'"), (*J).filename, (*ident).line);
    }

    i = findlocal(J, F, (*ident).string);
    if i < 0 {
        emitstring(J, F, opvar, (*ident).string);
    } else {
        emit(J, F, oploc);
        emitarg(J, F, i);
    }
}

unsafe fn here(J: *mut js_State, F: *mut js_Function) -> c_int {
    (*F).codelen
}

unsafe fn emitjump(J: *mut js_State, F: *mut js_Function, opcode: c_int) -> c_int {
    let inst;
    emit(J, F, opcode);
    inst = (*F).codelen;
    emitarg(J, F, 0);
    inst
}

unsafe fn emitjumpto(J: *mut js_State, F: *mut js_Function, opcode: c_int, dest: c_int) {
    emit(J, F, opcode);
    if dest != (dest as js_Instruction) as c_int {
        crate::jserror::js_syntaxerror(J, cstr!("jump address integer overflow"));
    }
    emitarg(J, F, dest);
}

unsafe fn labelto(J: *mut js_State, F: *mut js_Function, inst: c_int, addr: c_int) {
    if addr != (addr as js_Instruction) as c_int {
        crate::jserror::js_syntaxerror(J, cstr!("jump address integer overflow"));
    }
    *(*F).code.add(inst as usize) = addr as js_Instruction;
}

unsafe fn label(J: *mut js_State, F: *mut js_Function, inst: c_int) {
    labelto(J, F, inst, (*F).codelen);
}

/* Expressions */
unsafe fn ctypeof(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    if (*(*exp).a).type_ == EXP_IDENTIFIER {
        emitline(J, F, (*exp).a);
        emitlocal(J, F, OP_GETLOCAL, OP_HASVAR, (*exp).a);
    } else {
        cexp(J, F, (*exp).a);
    }
    emitline(J, F, exp);
    emit(J, F, OP_TYPEOF);
}

unsafe fn cunary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    cexp(J, F, (*exp).a);
    emitline(J, F, exp);
    emit(J, F, opcode);
}

unsafe fn cbinary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    cexp(J, F, (*exp).a);
    cexp(J, F, (*exp).b);
    emitline(J, F, exp);
    emit(J, F, opcode);
}

unsafe fn carray(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    while !list.is_null() {
        emitline(J, F, (*list).a);
        if (*(*list).a).type_ == EXP_ELISION {
            emit(J, F, OP_SKIPARRAY);
        } else {
            cexp(J, F, (*list).a);
            emit(J, F, OP_INITARRAY);
        }
        list = (*list).b;
    }
}

unsafe fn checkdup(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast, end: *mut js_Ast) {
    let mut nbuf: [c_char; 32] = [0; 32];
    let mut sbuf: [c_char; 32] = [0; 32];
    let needle;
    let mut straw;

    if (*(*end).a).type_ == EXP_NUMBER {
        needle = crate::jsvalue::jsV_numbertostring(J, nbuf.as_mut_ptr(), (*(*end).a).number);
    } else {
        needle = (*(*end).a).string;
    }

    while (*list).a != end {
        if (*(*list).a).type_ == (*end).type_ {
            let prop = (*(*list).a).a;
            if (*prop).type_ == EXP_NUMBER {
                straw = crate::jsvalue::jsV_numbertostring(J, sbuf.as_mut_ptr(), (*prop).number);
            } else {
                straw = (*prop).string;
            }
            if strcmp(needle, straw) == 0 {
                jsC_error!(J, list, cstr!("duplicate property '%s' in object literal"), needle);
            }
        }
        list = (*list).b;
    }
}

unsafe fn cobject(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    let head = list;

    while !list.is_null() {
        let kv = (*list).a;
        let prop = (*kv).a;

        if (*prop).type_ == AST_IDENTIFIER || (*prop).type_ == EXP_STRING {
            emitline(J, F, prop);
            emitstring(J, F, OP_STRING, (*prop).string);
        } else if (*prop).type_ == EXP_NUMBER {
            emitline(J, F, prop);
            emitnumber(J, F, (*prop).number);
        } else {
            jsC_error!(J, prop, cstr!("invalid property name in object initializer"));
        }

        if (*F).strict != 0 {
            checkdup(J, F, head, kv);
        }

        match (*kv).type_ {
            t if t == EXP_PROP_VAL => {
                cexp(J, F, (*kv).b);
                emitline(J, F, kv);
                emit(J, F, OP_INITPROP);
            }
            t if t == EXP_PROP_GET => {
                emitfunction(J, F, newfun(J, (*prop).line, std::ptr::null_mut(), std::ptr::null_mut(), (*kv).c, 0, (*F).strict, 1));
                emitline(J, F, kv);
                emit(J, F, OP_INITGETTER);
            }
            t if t == EXP_PROP_SET => {
                emitfunction(J, F, newfun(J, (*prop).line, std::ptr::null_mut(), (*kv).b, (*kv).c, 0, (*F).strict, 1));
                emitline(J, F, kv);
                emit(J, F, OP_INITSETTER);
            }
            _ => {}
        }

        list = (*list).b;
    }
}

unsafe fn cargs(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) -> c_int {
    let mut n = 0;
    while !list.is_null() {
        cexp(J, F, (*list).a);
        list = (*list).b;
        n += 1;
    }
    n
}

unsafe fn cassign(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let lhs = (*exp).a;
    let rhs = (*exp).b;
    match (*lhs).type_ {
        t if t == EXP_IDENTIFIER => {
            cexp(J, F, rhs);
            emitline(J, F, exp);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
        }
        t if t == EXP_INDEX => {
            cexp(J, F, (*lhs).a);
            cexp(J, F, (*lhs).b);
            cexp(J, F, rhs);
            emitline(J, F, exp);
            emit(J, F, OP_SETPROP);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*lhs).a);
            cexp(J, F, rhs);
            emitline(J, F, exp);
            emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
        }
        _ => {
            jsC_error!(J, lhs, cstr!("invalid l-value in assignment"));
        }
    }
}

unsafe fn cassignforin(J: *mut js_State, F: *mut js_Function, stm: *mut js_Ast) {
    let lhs = (*stm).a;

    if (*stm).type_ == STM_FOR_IN_VAR {
        if !(*lhs).b.is_null() {
            jsC_error!(J, (*lhs).b, cstr!("more than one loop variable in for-in statement"));
        }
        emitline(J, F, (*lhs).a);
        emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*(*lhs).a).a);
        emit(J, F, OP_POP);
        return;
    }

    match (*lhs).type_ {
        t if t == EXP_IDENTIFIER => {
            emitline(J, F, lhs);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
            emit(J, F, OP_POP);
        }
        t if t == EXP_INDEX => {
            cexp(J, F, (*lhs).a);
            cexp(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT3);
            emit(J, F, OP_SETPROP);
            emit(J, F, OP_POP);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT2);
            emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
            emit(J, F, OP_POP);
        }
        _ => {
            jsC_error!(J, lhs, cstr!("invalid l-value in for-in loop assignment"));
        }
    }
}

unsafe fn cassignop1(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast) {
    match (*lhs).type_ {
        t if t == EXP_IDENTIFIER => {
            emitline(J, F, lhs);
            emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, lhs);
        }
        t if t == EXP_INDEX => {
            cexp(J, F, (*lhs).a);
            cexp(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP2);
            emit(J, F, OP_GETPROP);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP);
            emitstring(J, F, OP_GETPROP_S, (*(*lhs).b).string);
        }
        _ => {
            jsC_error!(J, lhs, cstr!("invalid l-value in assignment"));
        }
    }
}

unsafe fn cassignop2(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast, postfix: c_int) {
    match (*lhs).type_ {
        t if t == EXP_IDENTIFIER => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT2);
            }
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
        }
        t if t == EXP_INDEX => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT4);
            }
            emit(J, F, OP_SETPROP);
        }
        t if t == EXP_MEMBER => {
            emitline(J, F, lhs);
            if postfix != 0 {
                emit(J, F, OP_ROT3);
            }
            emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
        }
        _ => {
            jsC_error!(J, lhs, cstr!("invalid l-value in assignment"));
        }
    }
}

unsafe fn cassignop(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    let lhs = (*exp).a;
    let rhs = (*exp).b;
    cassignop1(J, F, lhs);
    cexp(J, F, rhs);
    emitline(J, F, exp);
    emit(J, F, opcode);
    cassignop2(J, F, lhs, 0);
}

unsafe fn cdelete(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let arg = (*exp).a;
    match (*arg).type_ {
        t if t == EXP_IDENTIFIER => {
            if (*F).strict != 0 {
                jsC_error!(J, exp, cstr!("delete on an unqualified name is not allowed in strict mode"));
            }
            emitline(J, F, exp);
            emitlocal(J, F, OP_DELLOCAL, OP_DELVAR, arg);
        }
        t if t == EXP_INDEX => {
            cexp(J, F, (*arg).a);
            cexp(J, F, (*arg).b);
            emitline(J, F, exp);
            emit(J, F, OP_DELPROP);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*arg).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_DELPROP_S, (*(*arg).b).string);
        }
        _ => {
            jsC_error!(J, exp, cstr!("invalid l-value in delete expression"));
        }
    }
}

unsafe fn ceval(J: *mut js_State, F: *mut js_Function, _fun: *mut js_Ast, args: *mut js_Ast) {
    let mut n = cargs(J, F, args);
    (*F).lightweight = 0;
    (*F).arguments = 1;
    if n == 0 {
        emit(J, F, OP_UNDEF);
    } else {
        while n > 1 {
            emit(J, F, OP_POP);
            n -= 1;
        }
    }
    emit(J, F, OP_EVAL);
}

unsafe fn ccall(J: *mut js_State, F: *mut js_Function, fun: *mut js_Ast, args: *mut js_Ast) {
    let n;
    match (*fun).type_ {
        t if t == EXP_INDEX => {
            cexp(J, F, (*fun).a);
            emit(J, F, OP_DUP);
            cexp(J, F, (*fun).b);
            emit(J, F, OP_GETPROP);
            emit(J, F, OP_ROT2);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*fun).a);
            emit(J, F, OP_DUP);
            emitstring(J, F, OP_GETPROP_S, (*(*fun).b).string);
            emit(J, F, OP_ROT2);
        }
        t if t == EXP_IDENTIFIER => {
            if strcmp((*fun).string, cstr!("eval")) == 0 {
                ceval(J, F, fun, args);
                return;
            }
            cexp(J, F, fun);
            emit(J, F, OP_UNDEF);
        }
        _ => {
            cexp(J, F, fun);
            emit(J, F, OP_UNDEF);
        }
    }
    n = cargs(J, F, args);
    emit(J, F, OP_CALL);
    emitarg(J, F, n);
}

unsafe fn cexp(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let then: c_int = 0;
    let end: c_int = 0;
    let n: c_int = 0;

    match (*exp).type_ {
        t if t == EXP_STRING => {
            emitline(J, F, exp);
            emitstring(J, F, OP_STRING, (*exp).string);
        }
        t if t == EXP_NUMBER => {
            emitline(J, F, exp);
            emitnumber(J, F, (*exp).number);
        }
        t if t == EXP_ELISION => {}
        t if t == EXP_NULL => {
            emitline(J, F, exp);
            emit(J, F, OP_NULL);
        }
        t if t == EXP_TRUE => {
            emitline(J, F, exp);
            emit(J, F, OP_TRUE);
        }
        t if t == EXP_FALSE => {
            emitline(J, F, exp);
            emit(J, F, OP_FALSE);
        }
        t if t == EXP_THIS => {
            emitline(J, F, exp);
            emit(J, F, OP_THIS);
        }
        t if t == EXP_REGEXP => {
            emitline(J, F, exp);
            emitstring(J, F, OP_NEWREGEXP, (*exp).string);
            emitarg(J, F, (*exp).number as c_int);
        }
        t if t == EXP_OBJECT => {
            emitline(J, F, exp);
            emit(J, F, OP_NEWOBJECT);
            cobject(J, F, (*exp).a);
        }
        t if t == EXP_ARRAY => {
            emitline(J, F, exp);
            emit(J, F, OP_NEWARRAY);
            carray(J, F, (*exp).a);
        }
        t if t == EXP_FUN => {
            emitline(J, F, exp);
            emitfunction(J, F, newfun(J, (*exp).line, (*exp).a, (*exp).b, (*exp).c, 0, (*F).strict, 1));
        }
        t if t == EXP_IDENTIFIER => {
            emitline(J, F, exp);
            emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, exp);
        }
        t if t == EXP_INDEX => {
            cexp(J, F, (*exp).a);
            cexp(J, F, (*exp).b);
            emitline(J, F, exp);
            emit(J, F, OP_GETPROP);
        }
        t if t == EXP_MEMBER => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_GETPROP_S, (*(*exp).b).string);
        }
        t if t == EXP_CALL => {
            ccall(J, F, (*exp).a, (*exp).b);
        }
        t if t == EXP_NEW => {
            cexp(J, F, (*exp).a);
            let n = cargs(J, F, (*exp).b);
            emitline(J, F, exp);
            emit(J, F, OP_NEW);
            emitarg(J, F, n);
        }
        t if t == EXP_DELETE => {
            cdelete(J, F, exp);
        }
        t if t == EXP_PREINC => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_INC);
            cassignop2(J, F, (*exp).a, 0);
        }
        t if t == EXP_PREDEC => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DEC);
            cassignop2(J, F, (*exp).a, 0);
        }
        t if t == EXP_POSTINC => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POSTINC);
            cassignop2(J, F, (*exp).a, 1);
            emit(J, F, OP_POP);
        }
        t if t == EXP_POSTDEC => {
            cassignop1(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POSTDEC);
            cassignop2(J, F, (*exp).a, 1);
            emit(J, F, OP_POP);
        }
        t if t == EXP_VOID => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP);
            emit(J, F, OP_UNDEF);
        }
        t if t == EXP_TYPEOF => ctypeof(J, F, exp),
        t if t == EXP_POS => cunary(J, F, exp, OP_POS),
        t if t == EXP_NEG => cunary(J, F, exp, OP_NEG),
        t if t == EXP_BITNOT => cunary(J, F, exp, OP_BITNOT),
        t if t == EXP_LOGNOT => cunary(J, F, exp, OP_LOGNOT),
        t if t == EXP_BITOR => cbinary(J, F, exp, OP_BITOR),
        t if t == EXP_BITXOR => cbinary(J, F, exp, OP_BITXOR),
        t if t == EXP_BITAND => cbinary(J, F, exp, OP_BITAND),
        t if t == EXP_EQ => cbinary(J, F, exp, OP_EQ),
        t if t == EXP_NE => cbinary(J, F, exp, OP_NE),
        t if t == EXP_STRICTEQ => cbinary(J, F, exp, OP_STRICTEQ),
        t if t == EXP_STRICTNE => cbinary(J, F, exp, OP_STRICTNE),
        t if t == EXP_LT => cbinary(J, F, exp, OP_LT),
        t if t == EXP_GT => cbinary(J, F, exp, OP_GT),
        t if t == EXP_LE => cbinary(J, F, exp, OP_LE),
        t if t == EXP_GE => cbinary(J, F, exp, OP_GE),
        t if t == EXP_INSTANCEOF => cbinary(J, F, exp, OP_INSTANCEOF),
        t if t == EXP_IN => cbinary(J, F, exp, OP_IN),
        t if t == EXP_SHL => cbinary(J, F, exp, OP_SHL),
        t if t == EXP_SHR => cbinary(J, F, exp, OP_SHR),
        t if t == EXP_USHR => cbinary(J, F, exp, OP_USHR),
        t if t == EXP_ADD => cbinary(J, F, exp, OP_ADD),
        t if t == EXP_SUB => cbinary(J, F, exp, OP_SUB),
        t if t == EXP_MUL => cbinary(J, F, exp, OP_MUL),
        t if t == EXP_DIV => cbinary(J, F, exp, OP_DIV),
        t if t == EXP_MOD => cbinary(J, F, exp, OP_MOD),
        t if t == EXP_ASS => cassign(J, F, exp),
        t if t == EXP_ASS_MUL => cassignop(J, F, exp, OP_MUL),
        t if t == EXP_ASS_DIV => cassignop(J, F, exp, OP_DIV),
        t if t == EXP_ASS_MOD => cassignop(J, F, exp, OP_MOD),
        t if t == EXP_ASS_ADD => cassignop(J, F, exp, OP_ADD),
        t if t == EXP_ASS_SUB => cassignop(J, F, exp, OP_SUB),
        t if t == EXP_ASS_SHL => cassignop(J, F, exp, OP_SHL),
        t if t == EXP_ASS_SHR => cassignop(J, F, exp, OP_SHR),
        t if t == EXP_ASS_USHR => cassignop(J, F, exp, OP_USHR),
        t if t == EXP_ASS_BITAND => cassignop(J, F, exp, OP_BITAND),
        t if t == EXP_ASS_BITXOR => cassignop(J, F, exp, OP_BITXOR),
        t if t == EXP_ASS_BITOR => cassignop(J, F, exp, OP_BITOR),
        t if t == EXP_COMMA => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP);
            cexp(J, F, (*exp).b);
        }
        t if t == EXP_LOGOR => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP);
            let end = emitjump(J, F, OP_JTRUE);
            emit(J, F, OP_POP);
            cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        t if t == EXP_LOGAND => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP);
            let end = emitjump(J, F, OP_JFALSE);
            emit(J, F, OP_POP);
            cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        t if t == EXP_COND => {
            cexp(J, F, (*exp).a);
            emitline(J, F, exp);
            let then = emitjump(J, F, OP_JTRUE);
            cexp(J, F, (*exp).c);
            let end = emitjump(J, F, OP_JUMP);
            label(J, F, then);
            cexp(J, F, (*exp).b);
            label(J, F, end);
        }
        _ => {
            let _ = (then, end, n);
            jsC_error!(J, exp, cstr!("unknown expression type"));
        }
    }
}

/* Patch break/continue */
unsafe fn addjump(J: *mut js_State, F: *mut js_Function, type_: c_int, target: *mut js_Ast, inst: c_int) {
    let jump = js_malloc(J, std::mem::size_of::<js_JumpList>() as c_int) as *mut js_JumpList;
    (*jump).type_ = type_;
    (*jump).inst = inst;
    (*jump).next = (*target).jumps;
    (*target).jumps = jump;
}

unsafe fn labeljumps(J: *mut js_State, F: *mut js_Function, stm: *mut js_Ast, baddr: c_int, caddr: c_int) {
    let mut jump = (*stm).jumps;
    while !jump.is_null() {
        let next = (*jump).next;
        if (*jump).type_ == STM_BREAK {
            labelto(J, F, (*jump).inst, baddr);
        }
        if (*jump).type_ == STM_CONTINUE {
            labelto(J, F, (*jump).inst, caddr);
        }
        js_free(J, jump as *mut c_void);
        jump = next;
    }
    (*stm).jumps = std::ptr::null_mut();
}

unsafe fn isloop(t: c_int) -> c_int {
    (t == STM_DO || t == STM_WHILE || t == STM_FOR || t == STM_FOR_VAR || t == STM_FOR_IN || t == STM_FOR_IN_VAR) as c_int
}

unsafe fn isfun(t: c_int) -> c_int {
    (t == AST_FUNDEC || t == EXP_FUN || t == EXP_PROP_GET || t == EXP_PROP_SET) as c_int
}

unsafe fn matchlabel(mut node: *mut js_Ast, label: *const c_char) -> c_int {
    while !node.is_null() && (*node).type_ == STM_LABEL {
        if strcmp((*(*node).a).string, label) == 0 {
            return 1;
        }
        node = (*node).parent;
    }
    0
}

unsafe fn breaktarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast, label: *const c_char) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_) != 0 {
            break;
        }
        if label.is_null() {
            if isloop((*node).type_) != 0 || (*node).type_ == STM_SWITCH {
                return node;
            }
        } else {
            if matchlabel((*node).parent, label) != 0 {
                return node;
            }
        }
        node = (*node).parent;
    }
    std::ptr::null_mut()
}

unsafe fn continuetarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast, label: *const c_char) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_) != 0 {
            break;
        }
        if isloop((*node).type_) != 0 {
            if label.is_null() {
                return node;
            } else if matchlabel((*node).parent, label) != 0 {
                return node;
            }
        }
        node = (*node).parent;
    }
    std::ptr::null_mut()
}

unsafe fn returntarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_) != 0 {
            return node;
        }
        node = (*node).parent;
    }
    std::ptr::null_mut()
}

unsafe fn cexit(J: *mut js_State, F: *mut js_Function, T: c_int, mut node: *mut js_Ast, target: *mut js_Ast) {
    let mut prev;
    loop {
        prev = node;
        node = (*node).parent;
        match (*node).type_ {
            t if t == STM_WITH => {
                emitline(J, F, node);
                emit(J, F, OP_ENDWITH);
            }
            t if t == STM_FOR_IN || t == STM_FOR_IN_VAR => {
                emitline(J, F, node);
                if (*F).script != 0 {
                    if T == STM_RETURN || T == STM_BREAK || (T == STM_CONTINUE && target != node) {
                        emit(J, F, OP_ROT2);
                        emit(J, F, OP_POP);
                    }
                    if T == STM_CONTINUE {
                        emit(J, F, OP_ROT2);
                    }
                } else {
                    if T == STM_RETURN {
                        emit(J, F, OP_ROT2);
                        emit(J, F, OP_POP);
                    }
                    if T == STM_BREAK || (T == STM_CONTINUE && target != node) {
                        emit(J, F, OP_POP);
                    }
                }
            }
            t if t == STM_TRY => {
                emitline(J, F, node);
                if prev == (*node).a {
                    emit(J, F, OP_ENDTRY);
                    if !(*node).d.is_null() {
                        cstm(J, F, (*node).d);
                    }
                }
                if prev == (*node).c {
                    if !(*node).d.is_null() {
                        emit(J, F, OP_ENDCATCH);
                        emit(J, F, OP_ENDTRY);
                        cstm(J, F, (*node).d);
                    } else {
                        emit(J, F, OP_ENDCATCH);
                    }
                }
            }
            _ => {}
        }
        if node == target {
            break;
        }
    }
}

/* Try/catch/finally */
unsafe fn ctryfinally(J: *mut js_State, F: *mut js_Function, trystm: *mut js_Ast, finallystm: *mut js_Ast) {
    let L1;
    L1 = emitjump(J, F, OP_TRY);
    {
        cstm(J, F, finallystm);
        emit(J, F, OP_THROW);
    }
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY);
    cstm(J, F, finallystm);
}

unsafe fn ctrycatch(J: *mut js_State, F: *mut js_Function, trystm: *mut js_Ast, catchvar: *mut js_Ast, catchstm: *mut js_Ast) {
    let L1;
    let L2;
    L1 = emitjump(J, F, OP_TRY);
    {
        checkfutureword(J, F, catchvar);
        if (*F).strict != 0 {
            if strcmp((*catchvar).string, cstr!("arguments")) == 0 {
                jsC_error!(J, catchvar, cstr!("redefining 'arguments' is not allowed in strict mode"));
            }
            if strcmp((*catchvar).string, cstr!("eval")) == 0 {
                jsC_error!(J, catchvar, cstr!("redefining 'eval' is not allowed in strict mode"));
            }
        }
        emitline(J, F, catchvar);
        emitstring(J, F, OP_CATCH, (*catchvar).string);
        cstm(J, F, catchstm);
        emit(J, F, OP_ENDCATCH);
        L2 = emitjump(J, F, OP_JUMP);
    }
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY);
    label(J, F, L2);
}

unsafe fn ctrycatchfinally(J: *mut js_State, F: *mut js_Function, trystm: *mut js_Ast, catchvar: *mut js_Ast, catchstm: *mut js_Ast, finallystm: *mut js_Ast) {
    let L1;
    let L2;
    let L3;
    L1 = emitjump(J, F, OP_TRY);
    {
        L2 = emitjump(J, F, OP_TRY);
        {
            cstm(J, F, finallystm);
            emit(J, F, OP_THROW);
        }
        label(J, F, L2);
        if (*F).strict != 0 {
            checkfutureword(J, F, catchvar);
            if strcmp((*catchvar).string, cstr!("arguments")) == 0 {
                jsC_error!(J, catchvar, cstr!("redefining 'arguments' is not allowed in strict mode"));
            }
            if strcmp((*catchvar).string, cstr!("eval")) == 0 {
                jsC_error!(J, catchvar, cstr!("redefining 'eval' is not allowed in strict mode"));
            }
        }
        emitline(J, F, catchvar);
        emitstring(J, F, OP_CATCH, (*catchvar).string);
        cstm(J, F, catchstm);
        emit(J, F, OP_ENDCATCH);
        emit(J, F, OP_ENDTRY);
        L3 = emitjump(J, F, OP_JUMP);
    }
    label(J, F, L1);
    cstm(J, F, trystm);
    emit(J, F, OP_ENDTRY);
    label(J, F, L3);
    cstm(J, F, finallystm);
}

/* Switch */
unsafe fn cswitch(J: *mut js_State, F: *mut js_Function, r#ref: *mut js_Ast, head: *mut js_Ast) {
    let mut node;
    let mut clause;
    let mut def: *mut js_Ast = std::ptr::null_mut();
    let end;

    cexp(J, F, r#ref);

    node = head;
    while !node.is_null() {
        clause = (*node).a;
        if (*clause).type_ == STM_DEFAULT {
            if !def.is_null() {
                jsC_error!(J, clause, cstr!("more than one default label in switch"));
            }
            def = clause;
        } else {
            cexp(J, F, (*clause).a);
            emitline(J, F, clause);
            (*clause).casejump = emitjump(J, F, OP_JCASE);
        }
        node = (*node).b;
    }
    emit(J, F, OP_POP);
    if !def.is_null() {
        emitline(J, F, def);
        (*def).casejump = emitjump(J, F, OP_JUMP);
        end = 0;
    } else {
        end = emitjump(J, F, OP_JUMP);
    }

    node = head;
    while !node.is_null() {
        clause = (*node).a;
        label(J, F, (*clause).casejump);
        if (*clause).type_ == STM_DEFAULT {
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

/* Statements */
unsafe fn cvarinit(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    while !list.is_null() {
        let var = (*list).a;
        if !(*var).b.is_null() {
            cexp(J, F, (*var).b);
            emitline(J, F, var);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*var).a);
            emit(J, F, OP_POP);
        }
        list = (*list).b;
    }
}

unsafe fn cstm(J: *mut js_State, F: *mut js_Function, mut stm: *mut js_Ast) {
    let target: *mut js_Ast = std::ptr::null_mut();
    let loop_: c_int = 0;
    let cont: c_int = 0;
    let then: c_int = 0;
    let end: c_int = 0;

    emitline(J, F, stm);

    match (*stm).type_ {
        t if t == AST_FUNDEC => {}
        t if t == STM_BLOCK => {
            cstmlist(J, F, (*stm).a);
        }
        t if t == STM_EMPTY => {
            if (*F).script != 0 {
                emitline(J, F, stm);
                emit(J, F, OP_POP);
                emit(J, F, OP_UNDEF);
            }
        }
        t if t == STM_VAR => {
            cvarinit(J, F, (*stm).a);
        }
        t if t == STM_IF => {
            if !(*stm).c.is_null() {
                cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                let then = emitjump(J, F, OP_JTRUE);
                cstm(J, F, (*stm).c);
                emitline(J, F, stm);
                let end = emitjump(J, F, OP_JUMP);
                label(J, F, then);
                cstm(J, F, (*stm).b);
                label(J, F, end);
            } else {
                cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                let end = emitjump(J, F, OP_JFALSE);
                cstm(J, F, (*stm).b);
                label(J, F, end);
            }
        }
        t if t == STM_DO => {
            let loop_ = here(J, F);
            cstm(J, F, (*stm).a);
            let cont = here(J, F);
            cexp(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JTRUE, loop_);
            labeljumps(J, F, stm, here(J, F), cont);
        }
        t if t == STM_WHILE => {
            let loop_ = here(J, F);
            cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            let end = emitjump(J, F, OP_JFALSE);
            cstm(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP, loop_);
            label(J, F, end);
            labeljumps(J, F, stm, here(J, F), loop_);
        }
        t if t == STM_FOR || t == STM_FOR_VAR => {
            if (*stm).type_ == STM_FOR_VAR {
                cvarinit(J, F, (*stm).a);
            } else {
                if !(*stm).a.is_null() {
                    cexp(J, F, (*stm).a);
                    emit(J, F, OP_POP);
                }
            }
            let loop_ = here(J, F);
            let end;
            if !(*stm).b.is_null() {
                cexp(J, F, (*stm).b);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JFALSE);
            } else {
                end = 0;
            }
            cstm(J, F, (*stm).d);
            let cont = here(J, F);
            if !(*stm).c.is_null() {
                cexp(J, F, (*stm).c);
                emit(J, F, OP_POP);
            }
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP, loop_);
            if end != 0 {
                label(J, F, end);
            }
            labeljumps(J, F, stm, here(J, F), cont);
        }
        t if t == STM_FOR_IN || t == STM_FOR_IN_VAR => {
            cexp(J, F, (*stm).b);
            emitline(J, F, stm);
            emit(J, F, OP_ITERATOR);
            let loop_ = here(J, F);
            {
                emitline(J, F, stm);
                emit(J, F, OP_NEXTITER);
                let end = emitjump(J, F, OP_JFALSE);
                cassignforin(J, F, stm);
                if (*F).script != 0 {
                    emit(J, F, OP_ROT2);
                    cstm(J, F, (*stm).c);
                    emit(J, F, OP_ROT2);
                } else {
                    cstm(J, F, (*stm).c);
                }
                emitline(J, F, stm);
                emitjumpto(J, F, OP_JUMP, loop_);
                label(J, F, end);
            }
            labeljumps(J, F, stm, here(J, F), loop_);
        }
        t if t == STM_SWITCH => {
            cswitch(J, F, (*stm).a, (*stm).b);
            labeljumps(J, F, stm, here(J, F), 0);
        }
        t if t == STM_LABEL => {
            cstm(J, F, (*stm).b);
            while (*stm).type_ == STM_LABEL {
                stm = (*stm).b;
            }
            if isloop((*stm).type_) == 0 && (*stm).type_ != STM_SWITCH {
                labeljumps(J, F, stm, here(J, F), 0);
            }
        }
        t if t == STM_BREAK => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                let target = breaktarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error!(J, stm, cstr!("break label '%s' not found"), (*(*stm).a).string);
                }
                cexit(J, F, STM_BREAK, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_BREAK, target, emitjump(J, F, OP_JUMP));
            } else {
                let target = breaktarget(J, F, (*stm).parent, std::ptr::null());
                if target.is_null() {
                    jsC_error!(J, stm, cstr!("unlabelled break must be inside loop or switch"));
                }
                cexit(J, F, STM_BREAK, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_BREAK, target, emitjump(J, F, OP_JUMP));
            }
        }
        t if t == STM_CONTINUE => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                let target = continuetarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error!(J, stm, cstr!("continue label '%s' not found"), (*(*stm).a).string);
                }
                cexit(J, F, STM_CONTINUE, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_CONTINUE, target, emitjump(J, F, OP_JUMP));
            } else {
                let target = continuetarget(J, F, (*stm).parent, std::ptr::null());
                if target.is_null() {
                    jsC_error!(J, stm, cstr!("continue must be inside loop"));
                }
                cexit(J, F, STM_CONTINUE, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_CONTINUE, target, emitjump(J, F, OP_JUMP));
            }
        }
        t if t == STM_RETURN => {
            if !(*stm).a.is_null() {
                cexp(J, F, (*stm).a);
            } else {
                emit(J, F, OP_UNDEF);
            }
            let target = returntarget(J, F, (*stm).parent);
            if target.is_null() {
                jsC_error!(J, stm, cstr!("return not in function"));
            }
            cexit(J, F, STM_RETURN, stm, target);
            emitline(J, F, stm);
            emit(J, F, OP_RETURN);
        }
        t if t == STM_THROW => {
            cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            emit(J, F, OP_THROW);
        }
        t if t == STM_WITH => {
            (*F).lightweight = 0;
            if (*F).strict != 0 {
                jsC_error!(J, (*stm).a, cstr!("'with' statements are not allowed in strict mode"));
            }
            cexp(J, F, (*stm).a);
            emitline(J, F, stm);
            emit(J, F, OP_WITH);
            cstm(J, F, (*stm).b);
            emitline(J, F, stm);
            emit(J, F, OP_ENDWITH);
        }
        t if t == STM_TRY => {
            emitline(J, F, stm);
            if !(*stm).b.is_null() && !(*stm).c.is_null() {
                (*F).lightweight = 0;
                if !(*stm).d.is_null() {
                    ctrycatchfinally(J, F, (*stm).a, (*stm).b, (*stm).c, (*stm).d);
                } else {
                    ctrycatch(J, F, (*stm).a, (*stm).b, (*stm).c);
                }
            } else {
                ctryfinally(J, F, (*stm).a, (*stm).d);
            }
        }
        t if t == STM_DEBUGGER => {
            emitline(J, F, stm);
            emit(J, F, OP_DEBUGGER);
        }
        _ => {
            let _ = (target, loop_, cont, then, end);
            if (*F).script != 0 {
                emitline(J, F, stm);
                emit(J, F, OP_POP);
                cexp(J, F, stm);
            } else {
                cexp(J, F, stm);
                emitline(J, F, stm);
                emit(J, F, OP_POP);
            }
        }
    }
}

unsafe fn cstmlist(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    while !list.is_null() {
        cstm(J, F, (*list).a);
        list = (*list).b;
    }
}

/* Declarations and programs */
unsafe fn listlength(mut list: *mut js_Ast) -> c_int {
    let mut n = 0;
    while !list.is_null() {
        n += 1;
        list = (*list).b;
    }
    n
}

unsafe fn cparams(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast, _fname: *mut js_Ast) {
    (*F).numparams = listlength(list);
    while !list.is_null() {
        checkfutureword(J, F, (*list).a);
        addlocal(J, F, (*list).a, 0);
        list = (*list).b;
    }
}

unsafe fn cvardecs(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast) {
    if (*node).type_ == AST_LIST {
        while !node.is_null() {
            cvardecs(J, F, (*node).a);
            node = (*node).b;
        }
        return;
    }

    if isfun((*node).type_) != 0 {
        return;
    }

    if (*node).type_ == EXP_VAR {
        checkfutureword(J, F, (*node).a);
        addlocal(J, F, (*node).a, 1);
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

unsafe fn cfundecs(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    while !list.is_null() {
        let stm = (*list).a;
        if (*stm).type_ == AST_FUNDEC {
            emitline(J, F, stm);
            emitfunction(J, F, newfun(J, (*stm).line, (*stm).a, (*stm).b, (*stm).c, 0, (*F).strict, 0));
            emitline(J, F, stm);
            emit(J, F, OP_SETLOCAL);
            let n = addlocal(J, F, (*stm).a, 1);
            emitarg(J, F, n);
            emit(J, F, OP_POP);
        }
        list = (*list).b;
    }
}

unsafe fn cfunbody(J: *mut js_State, F: *mut js_Function, name: *mut js_Ast, params: *mut js_Ast, body: *mut js_Ast, is_fun_exp: c_int) {
    (*F).lightweight = 1;
    (*F).arguments = 0;

    if (*F).script != 0 {
        (*F).lightweight = 0;
    }

    if !body.is_null() && (*body).type_ == AST_LIST && !(*body).a.is_null() && (*(*body).a).type_ == EXP_STRING {
        if strcmp((*(*body).a).string, cstr!("use strict")) == 0 {
            (*F).strict = 1;
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
            if findlocal(J, F, (*name).string) < 0 {
                emit(J, F, OP_CURRENT);
                emit(J, F, OP_SETLOCAL);
                let n = addlocal(J, F, name, 1);
                emitarg(J, F, n);
                emit(J, F, OP_POP);
            }
        }
    }

    if (*F).script != 0 {
        emit(J, F, OP_UNDEF);
        cstmlist(J, F, body);
        emit(J, F, OP_RETURN);
    } else {
        cstmlist(J, F, body);
        emit(J, F, OP_UNDEF);
        emit(J, F, OP_RETURN);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsC_compilefunction(J: *mut js_State, prog: *mut js_Ast) -> *mut js_Function {
    newfun(J, (*prog).line, (*prog).a, (*prog).b, (*prog).c, 0, (*J).default_strict, 1)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsC_compilescript(J: *mut js_State, prog: *mut js_Ast, default_strict: c_int) -> *mut js_Function {
    newfun(J, if !prog.is_null() { (*prog).line } else { 0 }, std::ptr::null_mut(), std::ptr::null_mut(), prog, 1, default_strict, 0)
}
