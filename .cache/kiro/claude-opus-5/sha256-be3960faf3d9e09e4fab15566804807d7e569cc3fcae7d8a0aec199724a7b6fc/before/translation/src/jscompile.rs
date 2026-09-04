//! Translation of src/jscompile.c — the bytecode compiler.
//!
//! Faithful mechanical transliteration: raw pointers, same control flow, same
//! emitted bytecode, same order of checks and error messages.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;

use crate::jsintern::js_intern;
use crate::jslex::{jsY_findword, jsY_tokenstring};
use crate::jsrun::{js_free, js_malloc, js_realloc, js_throw};
use crate::jsvalue::jsV_numbertostring;

unsafe extern "C-unwind" {
    fn js_newsyntaxerror(J: *mut js_State, message: *const c_char);
}

/* #define cexp jsC_cexp -- collision with math.h (not relevant in Rust) */

/* SHRT_MIN / SHRT_MAX from <limits.h> */
const SHRT_MIN: c_int = -32768;
const SHRT_MAX: c_int = 32767;

/* ------------------------------------------------------------------ */
/* jsC_error                                                          */
/* ------------------------------------------------------------------ */

/* The exported variadic symbol `jsC_error` is provided by a naked-asm
 * trampoline in vararg.rs; it forwards the constructed va_list here. */
pub unsafe extern "C-unwind" fn jsC_error_v(
    J: *mut js_State,
    node: *mut js_Ast,
    fmt: *const c_char,
    ap: *mut c_void,
) -> ! {
    unsafe {
        let mut buf: [c_char; 512] = [0; 512];
        let mut msgbuf: [c_char; 256] = [0; 256];

        crate::vararg::vsnprintf(msgbuf.as_mut_ptr(), 256, fmt, ap);

        snprintf(
            buf.as_mut_ptr(),
            256,
            c"%s:%d: ".as_ptr(),
            (*J).filename,
            (*node).line,
        );
        strcat(buf.as_mut_ptr(), msgbuf.as_ptr());

        js_newsyntaxerror(J, buf.as_ptr());
        js_throw(J)
    }
}

/// The tail of `jsC_error` after the vsnprintf: takes the already-formatted
/// message body and produces the "file:line: message" error and throws.
unsafe fn jsC_error_msg(J: *mut js_State, node: *mut js_Ast, msg: *const c_char) -> ! {
    unsafe {
        let mut buf: [c_char; 512] = [0; 512];

        snprintf(
            buf.as_mut_ptr(),
            256,
            c"%s:%d: ".as_ptr(),
            (*J).filename,
            (*node).line,
        );
        strcat(buf.as_mut_ptr(), msg);

        js_newsyntaxerror(J, buf.as_ptr());
        js_throw(J)
    }
}

/// Internal call sites: format into a 256-byte buffer with snprintf then throw
/// via jsC_error_msg, exactly like the C jsC_error body.
macro_rules! jsC_error {
    ($J:expr, $n:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut b: [c_char; 256] = [0; 256];
        snprintf(b.as_mut_ptr(), 256, $fmt $(, $a)*);
        jsC_error_msg($J, $n, b.as_ptr())
    }};
}

/* ------------------------------------------------------------------ */

static mut futurewords: [*const c_char; 7] = [
    c"class".as_ptr(),
    c"const".as_ptr(),
    c"enum".as_ptr(),
    c"export".as_ptr(),
    c"extends".as_ptr(),
    c"import".as_ptr(),
    c"super".as_ptr(),
];

static mut strictfuturewords: [*const c_char; 9] = [
    c"implements".as_ptr(),
    c"interface".as_ptr(),
    c"let".as_ptr(),
    c"package".as_ptr(),
    c"private".as_ptr(),
    c"protected".as_ptr(),
    c"public".as_ptr(),
    c"static".as_ptr(),
    c"yield".as_ptr(),
];

unsafe fn checkfutureword(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    unsafe {
        if jsY_findword((*exp).string, (&raw const futurewords) as *const *const c_char, 7) >= 0 {
            jsC_error!(J, exp, c"'%s' is a future reserved word".as_ptr(), (*exp).string);
        }
        if (*F).strict != 0 {
            if jsY_findword(
                (*exp).string,
                (&raw const strictfuturewords) as *const *const c_char,
                9,
            ) >= 0
            {
                jsC_error!(
                    J,
                    exp,
                    c"'%s' is a strict mode future reserved word".as_ptr(),
                    (*exp).string
                );
            }
        }
    }
}

unsafe fn newfun(
    J: *mut js_State,
    line: c_int,
    name: *mut js_Ast,
    params: *mut js_Ast,
    body: *mut js_Ast,
    script: c_int,
    default_strict: c_int,
    is_fun_exp: c_int,
) -> *mut js_Function {
    unsafe {
        let F: *mut js_Function = js_malloc(J, core::mem::size_of::<js_Function>() as c_int) as *mut js_Function;
        memset(F as *mut c_void, 0, core::mem::size_of::<js_Function>());
        (*F).gcmark = 0;
        (*F).gcnext = (*J).gcfun;
        (*J).gcfun = F;
        (*J).gccounter += 1;

        (*F).filename = js_intern(J, (*J).filename);
        (*F).line = line;
        (*F).script = script;
        (*F).strict = default_strict;
        (*F).name = if !name.is_null() { (*name).string } else { c"".as_ptr() };

        cfunbody(J, F, name, params, body, is_fun_exp);

        F
    }
}

/* Emit opcodes, constants and jumps */

unsafe fn emitraw(J: *mut js_State, F: *mut js_Function, value: c_int) {
    unsafe {
        if value != (value as js_Instruction) as c_int {
            js_syntaxerror!(J, c"integer overflow in instruction coding".as_ptr());
        }
        if (*F).codelen >= (*F).codecap {
            (*F).codecap = if (*F).codecap != 0 { (*F).codecap * 2 } else { 64 };
            (*F).code = js_realloc(
                J,
                (*F).code as *mut c_void,
                (*F).codecap * core::mem::size_of::<js_Instruction>() as c_int,
            ) as *mut js_Instruction;
        }
        *(*F).code.offset((*F).codelen as isize) = value as js_Instruction;
        (*F).codelen += 1;
    }
}

unsafe fn emit(J: *mut js_State, F: *mut js_Function, value: c_int) {
    unsafe {
        emitraw(J, F, (*F).lastline);
        emitraw(J, F, value);
    }
}

unsafe fn emitarg(J: *mut js_State, F: *mut js_Function, value: c_int) {
    unsafe {
        emitraw(J, F, value);
    }
}

unsafe fn emitline(J: *mut js_State, F: *mut js_Function, node: *mut js_Ast) {
    unsafe {
        (*F).lastline = (*node).line;
    }
}

unsafe fn addfunction(J: *mut js_State, F: *mut js_Function, value: *mut js_Function) -> c_int {
    unsafe {
        if (*F).funlen >= (*F).funcap {
            (*F).funcap = if (*F).funcap != 0 { (*F).funcap * 2 } else { 16 };
            (*F).funtab = js_realloc(
                J,
                (*F).funtab as *mut c_void,
                (*F).funcap * core::mem::size_of::<*mut js_Function>() as c_int,
            ) as *mut *mut js_Function;
        }
        *(*F).funtab.offset((*F).funlen as isize) = value;
        let r = (*F).funlen;
        (*F).funlen += 1;
        r
    }
}

unsafe fn addlocal(J: *mut js_State, F: *mut js_Function, ident: *mut js_Ast, reuse: c_int) -> c_int {
    unsafe {
        let name: *const c_char = (*ident).string;
        if (*F).strict != 0 {
            if strcmp(name, c"arguments".as_ptr()) == 0 {
                jsC_error!(J, ident, c"redefining 'arguments' is not allowed in strict mode".as_ptr());
            }
            if strcmp(name, c"eval".as_ptr()) == 0 {
                jsC_error!(J, ident, c"redefining 'eval' is not allowed in strict mode".as_ptr());
            }
        } else {
            if strcmp(name, c"eval".as_ptr()) == 0 {
                js_evalerror!(J, c"%s:%d: invalid use of 'eval'".as_ptr(), (*J).filename, (*ident).line);
            }
        }
        if reuse != 0 || (*F).strict != 0 {
            let mut i: c_int = 0;
            while i < (*F).varlen {
                if strcmp(*(*F).vartab.offset(i as isize), name) == 0 {
                    if reuse != 0 {
                        return i + 1;
                    }
                    if (*F).strict != 0 {
                        jsC_error!(J, ident, c"duplicate formal parameter '%s'".as_ptr(), name);
                    }
                }
                i += 1;
            }
        }
        if (*F).varlen >= (*F).varcap {
            (*F).varcap = if (*F).varcap != 0 { (*F).varcap * 2 } else { 16 };
            (*F).vartab = js_realloc(
                J,
                (*F).vartab as *mut c_void,
                (*F).varcap * core::mem::size_of::<*const c_char>() as c_int,
            ) as *mut *const c_char;
        }
        *(*F).vartab.offset((*F).varlen as isize) = name;
        (*F).varlen += 1;
        (*F).varlen
    }
}

unsafe fn findlocal(J: *mut js_State, F: *mut js_Function, name: *const c_char) -> c_int {
    unsafe {
        let mut i: c_int = (*F).varlen;
        while i > 0 {
            if strcmp(*(*F).vartab.offset((i - 1) as isize), name) == 0 {
                return i;
            }
            i -= 1;
        }
        -1
    }
}

unsafe fn emitfunction(J: *mut js_State, F: *mut js_Function, fun: *mut js_Function) {
    unsafe {
        (*F).lightweight = 0;
        emit(J, F, OP_CLOSURE);
        emitarg(J, F, addfunction(J, F, fun));
    }
}

unsafe fn emitnumber(J: *mut js_State, F: *mut js_Function, num: f64) {
    unsafe {
        if num == 0.0 {
            emit(J, F, OP_INTEGER);
            emitarg(J, F, 32768);
            if signbit(num) {
                emit(J, F, OP_NEG);
            }
        } else if num >= SHRT_MIN as f64 && num <= SHRT_MAX as f64 && num == (num as c_int) as f64 {
            emit(J, F, OP_INTEGER);
            emitarg(J, F, (num + 32768.0) as c_int);
        } else {
            /* N = sizeof(num) / sizeof(js_Instruction) */
            const N: usize =
                core::mem::size_of::<f64>() / core::mem::size_of::<js_Instruction>();
            let mut x: [js_Instruction; N] = [0; N];
            let mut i: usize;
            emit(J, F, OP_NUMBER);
            memcpy(
                x.as_mut_ptr() as *mut c_void,
                &num as *const f64 as *const c_void,
                core::mem::size_of::<f64>(),
            );
            i = 0;
            while i < N {
                emitarg(J, F, x[i] as c_int);
                i += 1;
            }
        }
    }
}

unsafe fn emitstring(J: *mut js_State, F: *mut js_Function, opcode: c_int, str: *const c_char) {
    unsafe {
        /* N = sizeof(str) / sizeof(js_Instruction) -- sizeof a pointer */
        const N: usize =
            core::mem::size_of::<*const c_char>() / core::mem::size_of::<js_Instruction>();
        let mut x: [js_Instruction; N] = [0; N];
        let mut i: usize;
        emit(J, F, opcode);
        memcpy(
            x.as_mut_ptr() as *mut c_void,
            &str as *const *const c_char as *const c_void,
            core::mem::size_of::<*const c_char>(),
        );
        i = 0;
        while i < N {
            emitarg(J, F, x[i] as c_int);
            i += 1;
        }
    }
}

unsafe fn emitlocal(J: *mut js_State, F: *mut js_Function, oploc: c_int, opvar: c_int, ident: *mut js_Ast) {
    unsafe {
        let is_arguments: c_int = (strcmp((*ident).string, c"arguments".as_ptr()) == 0) as c_int;
        let is_eval: c_int = (strcmp((*ident).string, c"eval".as_ptr()) == 0) as c_int;
        let i: c_int;

        if is_arguments != 0 {
            (*F).lightweight = 0;
            (*F).arguments = 1;
        }

        checkfutureword(J, F, ident);
        if (*F).strict != 0 && oploc == OP_SETLOCAL {
            if is_arguments != 0 {
                jsC_error!(J, ident, c"'arguments' is read-only in strict mode".as_ptr());
            }
            if is_eval != 0 {
                jsC_error!(J, ident, c"'eval' is read-only in strict mode".as_ptr());
            }
        }
        if is_eval != 0 {
            js_evalerror!(J, c"%s:%d: invalid use of 'eval'".as_ptr(), (*J).filename, (*ident).line);
        }

        i = findlocal(J, F, (*ident).string);
        if i < 0 {
            emitstring(J, F, opvar, (*ident).string);
        } else {
            emit(J, F, oploc);
            emitarg(J, F, i);
        }
    }
}

unsafe fn here(J: *mut js_State, F: *mut js_Function) -> c_int {
    unsafe { (*F).codelen }
}

unsafe fn emitjump(J: *mut js_State, F: *mut js_Function, opcode: c_int) -> c_int {
    unsafe {
        let inst: c_int;
        emit(J, F, opcode);
        inst = (*F).codelen;
        emitarg(J, F, 0);
        inst
    }
}

unsafe fn emitjumpto(J: *mut js_State, F: *mut js_Function, opcode: c_int, dest: c_int) {
    unsafe {
        emit(J, F, opcode);
        if dest != (dest as js_Instruction) as c_int {
            js_syntaxerror!(J, c"jump address integer overflow".as_ptr());
        }
        emitarg(J, F, dest);
    }
}

unsafe fn labelto(J: *mut js_State, F: *mut js_Function, inst: c_int, addr: c_int) {
    unsafe {
        if addr != (addr as js_Instruction) as c_int {
            js_syntaxerror!(J, c"jump address integer overflow".as_ptr());
        }
        *(*F).code.offset(inst as isize) = addr as js_Instruction;
    }
}

unsafe fn label(J: *mut js_State, F: *mut js_Function, inst: c_int) {
    unsafe {
        labelto(J, F, inst, (*F).codelen);
    }
}

/* Expressions */

unsafe fn ctypeof(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    unsafe {
        if (*(*exp).a).ty == EXP_IDENTIFIER {
            emitline(J, F, (*exp).a);
            emitlocal(J, F, OP_GETLOCAL, OP_HASVAR, (*exp).a);
        } else {
            cexp(J, F, (*exp).a);
        }
        emitline(J, F, exp);
        emit(J, F, OP_TYPEOF);
    }
}

unsafe fn cunary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    unsafe {
        cexp(J, F, (*exp).a);
        emitline(J, F, exp);
        emit(J, F, opcode);
    }
}

unsafe fn cbinary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    unsafe {
        cexp(J, F, (*exp).a);
        cexp(J, F, (*exp).b);
        emitline(J, F, exp);
        emit(J, F, opcode);
    }
}

unsafe fn carray(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    unsafe {
        while !list.is_null() {
            emitline(J, F, (*list).a);
            if (*(*list).a).ty == EXP_ELISION {
                emit(J, F, OP_SKIPARRAY);
            } else {
                cexp(J, F, (*list).a);
                emit(J, F, OP_INITARRAY);
            }
            list = (*list).b;
        }
    }
}

unsafe fn checkdup(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast, end: *mut js_Ast) {
    unsafe {
        let mut nbuf: [c_char; 32] = [0; 32];
        let mut sbuf: [c_char; 32] = [0; 32];
        let needle: *const c_char;
        let mut straw: *const c_char;

        if (*(*end).a).ty == EXP_NUMBER {
            needle = jsV_numbertostring(J, nbuf.as_mut_ptr(), (*(*end).a).number);
        } else {
            needle = (*(*end).a).string;
        }

        while (*list).a != end {
            if (*(*list).a).ty == (*end).ty {
                let prop: *mut js_Ast = (*(*list).a).a;
                if (*prop).ty == EXP_NUMBER {
                    straw = jsV_numbertostring(J, sbuf.as_mut_ptr(), (*prop).number);
                } else {
                    straw = (*prop).string;
                }
                if strcmp(needle, straw) == 0 {
                    jsC_error!(J, list, c"duplicate property '%s' in object literal".as_ptr(), needle);
                }
            }
            list = (*list).b;
        }
    }
}

unsafe fn cobject(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    unsafe {
        let head: *mut js_Ast = list;

        while !list.is_null() {
            let kv: *mut js_Ast = (*list).a;
            let prop: *mut js_Ast = (*kv).a;

            if (*prop).ty == AST_IDENTIFIER || (*prop).ty == EXP_STRING {
                emitline(J, F, prop);
                emitstring(J, F, OP_STRING, (*prop).string);
            } else if (*prop).ty == EXP_NUMBER {
                emitline(J, F, prop);
                emitnumber(J, F, (*prop).number);
            } else {
                jsC_error!(J, prop, c"invalid property name in object initializer".as_ptr());
            }

            if (*F).strict != 0 {
                checkdup(J, F, head, kv);
            }

            /* switch (kv->type) */
            let t = (*kv).ty;
            if t == EXP_PROP_VAL {
                cexp(J, F, (*kv).b);
                emitline(J, F, kv);
                emit(J, F, OP_INITPROP);
            } else if t == EXP_PROP_GET {
                emitfunction(
                    J,
                    F,
                    newfun(J, (*prop).line, core::ptr::null_mut(), core::ptr::null_mut(), (*kv).c, 0, (*F).strict, 1),
                );
                emitline(J, F, kv);
                emit(J, F, OP_INITGETTER);
            } else if t == EXP_PROP_SET {
                emitfunction(
                    J,
                    F,
                    newfun(J, (*prop).line, core::ptr::null_mut(), (*kv).b, (*kv).c, 0, (*F).strict, 1),
                );
                emitline(J, F, kv);
                emit(J, F, OP_INITSETTER);
            } else {
                /* default: impossible */
            }

            list = (*list).b;
        }
    }
}

unsafe fn cargs(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) -> c_int {
    unsafe {
        let mut n: c_int = 0;
        while !list.is_null() {
            cexp(J, F, (*list).a);
            list = (*list).b;
            n += 1;
        }
        n
    }
}

unsafe fn cassign(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    unsafe {
        let lhs: *mut js_Ast = (*exp).a;
        let rhs: *mut js_Ast = (*exp).b;
        match (*lhs).ty {
            EXP_IDENTIFIER => {
                cexp(J, F, rhs);
                emitline(J, F, exp);
                emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
            }
            EXP_INDEX => {
                cexp(J, F, (*lhs).a);
                cexp(J, F, (*lhs).b);
                cexp(J, F, rhs);
                emitline(J, F, exp);
                emit(J, F, OP_SETPROP);
            }
            EXP_MEMBER => {
                cexp(J, F, (*lhs).a);
                cexp(J, F, rhs);
                emitline(J, F, exp);
                emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
            }
            _ => {
                jsC_error!(J, lhs, c"invalid l-value in assignment".as_ptr());
            }
        }
    }
}

unsafe fn cassignforin(J: *mut js_State, F: *mut js_Function, stm: *mut js_Ast) {
    unsafe {
        let lhs: *mut js_Ast = (*stm).a;

        if (*stm).ty == STM_FOR_IN_VAR {
            if !(*lhs).b.is_null() {
                jsC_error!(J, (*lhs).b, c"more than one loop variable in for-in statement".as_ptr());
            }
            emitline(J, F, (*lhs).a);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*(*lhs).a).a); /* list(var-init(ident)) */
            emit(J, F, OP_POP);
            return;
        }

        match (*lhs).ty {
            EXP_IDENTIFIER => {
                emitline(J, F, lhs);
                emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
                emit(J, F, OP_POP);
            }
            EXP_INDEX => {
                cexp(J, F, (*lhs).a);
                cexp(J, F, (*lhs).b);
                emitline(J, F, lhs);
                emit(J, F, OP_ROT3);
                emit(J, F, OP_SETPROP);
                emit(J, F, OP_POP);
            }
            EXP_MEMBER => {
                cexp(J, F, (*lhs).a);
                emitline(J, F, lhs);
                emit(J, F, OP_ROT2);
                emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
                emit(J, F, OP_POP);
            }
            _ => {
                jsC_error!(J, lhs, c"invalid l-value in for-in loop assignment".as_ptr());
            }
        }
    }
}

unsafe fn cassignop1(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast) {
    unsafe {
        match (*lhs).ty {
            EXP_IDENTIFIER => {
                emitline(J, F, lhs);
                emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, lhs);
            }
            EXP_INDEX => {
                cexp(J, F, (*lhs).a);
                cexp(J, F, (*lhs).b);
                emitline(J, F, lhs);
                emit(J, F, OP_DUP2);
                emit(J, F, OP_GETPROP);
            }
            EXP_MEMBER => {
                cexp(J, F, (*lhs).a);
                emitline(J, F, lhs);
                emit(J, F, OP_DUP);
                emitstring(J, F, OP_GETPROP_S, (*(*lhs).b).string);
            }
            _ => {
                jsC_error!(J, lhs, c"invalid l-value in assignment".as_ptr());
            }
        }
    }
}

unsafe fn cassignop2(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast, postfix: c_int) {
    unsafe {
        match (*lhs).ty {
            EXP_IDENTIFIER => {
                emitline(J, F, lhs);
                if postfix != 0 {
                    emit(J, F, OP_ROT2);
                }
                emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
            }
            EXP_INDEX => {
                emitline(J, F, lhs);
                if postfix != 0 {
                    emit(J, F, OP_ROT4);
                }
                emit(J, F, OP_SETPROP);
            }
            EXP_MEMBER => {
                emitline(J, F, lhs);
                if postfix != 0 {
                    emit(J, F, OP_ROT3);
                }
                emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
            }
            _ => {
                jsC_error!(J, lhs, c"invalid l-value in assignment".as_ptr());
            }
        }
    }
}

unsafe fn cassignop(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    unsafe {
        let lhs: *mut js_Ast = (*exp).a;
        let rhs: *mut js_Ast = (*exp).b;
        cassignop1(J, F, lhs);
        cexp(J, F, rhs);
        emitline(J, F, exp);
        emit(J, F, opcode);
        cassignop2(J, F, lhs, 0);
    }
}

unsafe fn cdelete(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    unsafe {
        let arg: *mut js_Ast = (*exp).a;
        match (*arg).ty {
            EXP_IDENTIFIER => {
                if (*F).strict != 0 {
                    jsC_error!(J, exp, c"delete on an unqualified name is not allowed in strict mode".as_ptr());
                }
                emitline(J, F, exp);
                emitlocal(J, F, OP_DELLOCAL, OP_DELVAR, arg);
            }
            EXP_INDEX => {
                cexp(J, F, (*arg).a);
                cexp(J, F, (*arg).b);
                emitline(J, F, exp);
                emit(J, F, OP_DELPROP);
            }
            EXP_MEMBER => {
                cexp(J, F, (*arg).a);
                emitline(J, F, exp);
                emitstring(J, F, OP_DELPROP_S, (*(*arg).b).string);
            }
            _ => {
                jsC_error!(J, exp, c"invalid l-value in delete expression".as_ptr());
            }
        }
    }
}

unsafe fn ceval(J: *mut js_State, F: *mut js_Function, fun: *mut js_Ast, args: *mut js_Ast) {
    unsafe {
        let mut n: c_int = cargs(J, F, args);
        (*F).lightweight = 0;
        (*F).arguments = 1;
        if n == 0 {
            emit(J, F, OP_UNDEF);
        } else {
            while {
                let old = n;
                n -= 1;
                old > 1
            } {
                emit(J, F, OP_POP);
            }
        }
        emit(J, F, OP_EVAL);
    }
}

unsafe fn ccall(J: *mut js_State, F: *mut js_Function, fun: *mut js_Ast, args: *mut js_Ast) {
    unsafe {
        {
            match (*fun).ty {
                EXP_INDEX => {
                    cexp(J, F, (*fun).a);
                    emit(J, F, OP_DUP);
                    cexp(J, F, (*fun).b);
                    emit(J, F, OP_GETPROP);
                    emit(J, F, OP_ROT2);
                }
                EXP_MEMBER => {
                    cexp(J, F, (*fun).a);
                    emit(J, F, OP_DUP);
                    emitstring(J, F, OP_GETPROP_S, (*(*fun).b).string);
                    emit(J, F, OP_ROT2);
                }
                EXP_IDENTIFIER => {
                    if strcmp((*fun).string, c"eval".as_ptr()) == 0 {
                        ceval(J, F, fun, args);
                        return;
                    }
                    /* fallthrough */
                    cexp(J, F, fun);
                    emit(J, F, OP_UNDEF);
                }
                _ => {
                    cexp(J, F, fun);
                    emit(J, F, OP_UNDEF);
                }
            }
        }
        let n = cargs(J, F, args);
        emit(J, F, OP_CALL);
        emitarg(J, F, n);
    }
}

unsafe fn cexp(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    unsafe {
        let mut then: c_int;
        let mut end: c_int;
        let mut n: c_int;

        match (*exp).ty {
            EXP_STRING => {
                emitline(J, F, exp);
                emitstring(J, F, OP_STRING, (*exp).string);
            }
            EXP_NUMBER => {
                emitline(J, F, exp);
                emitnumber(J, F, (*exp).number);
            }
            EXP_ELISION => {}
            EXP_NULL => {
                emitline(J, F, exp);
                emit(J, F, OP_NULL);
            }
            EXP_TRUE => {
                emitline(J, F, exp);
                emit(J, F, OP_TRUE);
            }
            EXP_FALSE => {
                emitline(J, F, exp);
                emit(J, F, OP_FALSE);
            }
            EXP_THIS => {
                emitline(J, F, exp);
                emit(J, F, OP_THIS);
            }

            EXP_REGEXP => {
                emitline(J, F, exp);
                emitstring(J, F, OP_NEWREGEXP, (*exp).string);
                emitarg(J, F, (*exp).number as c_int);
            }

            EXP_OBJECT => {
                emitline(J, F, exp);
                emit(J, F, OP_NEWOBJECT);
                cobject(J, F, (*exp).a);
            }

            EXP_ARRAY => {
                emitline(J, F, exp);
                emit(J, F, OP_NEWARRAY);
                carray(J, F, (*exp).a);
            }

            EXP_FUN => {
                emitline(J, F, exp);
                emitfunction(
                    J,
                    F,
                    newfun(J, (*exp).line, (*exp).a, (*exp).b, (*exp).c, 0, (*F).strict, 1),
                );
            }

            EXP_IDENTIFIER => {
                emitline(J, F, exp);
                emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, exp);
            }

            EXP_INDEX => {
                cexp(J, F, (*exp).a);
                cexp(J, F, (*exp).b);
                emitline(J, F, exp);
                emit(J, F, OP_GETPROP);
            }

            EXP_MEMBER => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                emitstring(J, F, OP_GETPROP_S, (*(*exp).b).string);
            }

            EXP_CALL => {
                ccall(J, F, (*exp).a, (*exp).b);
            }

            EXP_NEW => {
                cexp(J, F, (*exp).a);
                n = cargs(J, F, (*exp).b);
                emitline(J, F, exp);
                emit(J, F, OP_NEW);
                emitarg(J, F, n);
            }

            EXP_DELETE => {
                cdelete(J, F, exp);
            }

            EXP_PREINC => {
                cassignop1(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_INC);
                cassignop2(J, F, (*exp).a, 0);
            }

            EXP_PREDEC => {
                cassignop1(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_DEC);
                cassignop2(J, F, (*exp).a, 0);
            }

            EXP_POSTINC => {
                cassignop1(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_POSTINC);
                cassignop2(J, F, (*exp).a, 1);
                emit(J, F, OP_POP);
            }

            EXP_POSTDEC => {
                cassignop1(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_POSTDEC);
                cassignop2(J, F, (*exp).a, 1);
                emit(J, F, OP_POP);
            }

            EXP_VOID => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_POP);
                emit(J, F, OP_UNDEF);
            }

            EXP_TYPEOF => {
                ctypeof(J, F, exp);
            }
            EXP_POS => {
                cunary(J, F, exp, OP_POS);
            }
            EXP_NEG => {
                cunary(J, F, exp, OP_NEG);
            }
            EXP_BITNOT => {
                cunary(J, F, exp, OP_BITNOT);
            }
            EXP_LOGNOT => {
                cunary(J, F, exp, OP_LOGNOT);
            }

            EXP_BITOR => {
                cbinary(J, F, exp, OP_BITOR);
            }
            EXP_BITXOR => {
                cbinary(J, F, exp, OP_BITXOR);
            }
            EXP_BITAND => {
                cbinary(J, F, exp, OP_BITAND);
            }
            EXP_EQ => {
                cbinary(J, F, exp, OP_EQ);
            }
            EXP_NE => {
                cbinary(J, F, exp, OP_NE);
            }
            EXP_STRICTEQ => {
                cbinary(J, F, exp, OP_STRICTEQ);
            }
            EXP_STRICTNE => {
                cbinary(J, F, exp, OP_STRICTNE);
            }
            EXP_LT => {
                cbinary(J, F, exp, OP_LT);
            }
            EXP_GT => {
                cbinary(J, F, exp, OP_GT);
            }
            EXP_LE => {
                cbinary(J, F, exp, OP_LE);
            }
            EXP_GE => {
                cbinary(J, F, exp, OP_GE);
            }
            EXP_INSTANCEOF => {
                cbinary(J, F, exp, OP_INSTANCEOF);
            }
            EXP_IN => {
                cbinary(J, F, exp, OP_IN);
            }
            EXP_SHL => {
                cbinary(J, F, exp, OP_SHL);
            }
            EXP_SHR => {
                cbinary(J, F, exp, OP_SHR);
            }
            EXP_USHR => {
                cbinary(J, F, exp, OP_USHR);
            }
            EXP_ADD => {
                cbinary(J, F, exp, OP_ADD);
            }
            EXP_SUB => {
                cbinary(J, F, exp, OP_SUB);
            }
            EXP_MUL => {
                cbinary(J, F, exp, OP_MUL);
            }
            EXP_DIV => {
                cbinary(J, F, exp, OP_DIV);
            }
            EXP_MOD => {
                cbinary(J, F, exp, OP_MOD);
            }

            EXP_ASS => {
                cassign(J, F, exp);
            }
            EXP_ASS_MUL => {
                cassignop(J, F, exp, OP_MUL);
            }
            EXP_ASS_DIV => {
                cassignop(J, F, exp, OP_DIV);
            }
            EXP_ASS_MOD => {
                cassignop(J, F, exp, OP_MOD);
            }
            EXP_ASS_ADD => {
                cassignop(J, F, exp, OP_ADD);
            }
            EXP_ASS_SUB => {
                cassignop(J, F, exp, OP_SUB);
            }
            EXP_ASS_SHL => {
                cassignop(J, F, exp, OP_SHL);
            }
            EXP_ASS_SHR => {
                cassignop(J, F, exp, OP_SHR);
            }
            EXP_ASS_USHR => {
                cassignop(J, F, exp, OP_USHR);
            }
            EXP_ASS_BITAND => {
                cassignop(J, F, exp, OP_BITAND);
            }
            EXP_ASS_BITXOR => {
                cassignop(J, F, exp, OP_BITXOR);
            }
            EXP_ASS_BITOR => {
                cassignop(J, F, exp, OP_BITOR);
            }

            EXP_COMMA => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_POP);
                cexp(J, F, (*exp).b);
            }

            EXP_LOGOR => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_DUP);
                end = emitjump(J, F, OP_JTRUE);
                emit(J, F, OP_POP);
                cexp(J, F, (*exp).b);
                label(J, F, end);
            }

            EXP_LOGAND => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                emit(J, F, OP_DUP);
                end = emitjump(J, F, OP_JFALSE);
                emit(J, F, OP_POP);
                cexp(J, F, (*exp).b);
                label(J, F, end);
            }

            EXP_COND => {
                cexp(J, F, (*exp).a);
                emitline(J, F, exp);
                then = emitjump(J, F, OP_JTRUE);
                cexp(J, F, (*exp).c);
                end = emitjump(J, F, OP_JUMP);
                label(J, F, then);
                cexp(J, F, (*exp).b);
                label(J, F, end);
            }

            _ => {
                jsC_error!(J, exp, c"unknown expression type".as_ptr());
            }
        }
    }
}

/* Patch break and continue statements */

unsafe fn addjump(J: *mut js_State, F: *mut js_Function, ty: c_int, target: *mut js_Ast, inst: c_int) {
    unsafe {
        let jump: *mut js_JumpList = js_malloc(J, core::mem::size_of::<js_JumpList>() as c_int) as *mut js_JumpList;
        (*jump).ty = ty;
        (*jump).inst = inst;
        (*jump).next = (*target).jumps;
        (*target).jumps = jump;
    }
}

unsafe fn labeljumps(J: *mut js_State, F: *mut js_Function, stm: *mut js_Ast, baddr: c_int, caddr: c_int) {
    unsafe {
        let mut jump: *mut js_JumpList = (*stm).jumps;
        while !jump.is_null() {
            let next: *mut js_JumpList = (*jump).next;
            if (*jump).ty == STM_BREAK {
                labelto(J, F, (*jump).inst, baddr);
            }
            if (*jump).ty == STM_CONTINUE {
                labelto(J, F, (*jump).inst, caddr);
            }
            js_free(J, jump as *mut c_void);
            jump = next;
        }
        (*stm).jumps = core::ptr::null_mut();
    }
}

unsafe fn isloop(T: c_int) -> c_int {
    (T == STM_DO
        || T == STM_WHILE
        || T == STM_FOR
        || T == STM_FOR_VAR
        || T == STM_FOR_IN
        || T == STM_FOR_IN_VAR) as c_int
}

unsafe fn isfun(T: c_int) -> c_int {
    (T == AST_FUNDEC || T == EXP_FUN || T == EXP_PROP_GET || T == EXP_PROP_SET) as c_int
}

unsafe fn matchlabel(mut node: *mut js_Ast, label: *const c_char) -> c_int {
    unsafe {
        while !node.is_null() && (*node).ty == STM_LABEL {
            if strcmp((*(*node).a).string, label) == 0 {
                return 1;
            }
            node = (*node).parent;
        }
        0
    }
}

unsafe fn breaktarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast, label: *const c_char) -> *mut js_Ast {
    unsafe {
        while !node.is_null() {
            if isfun((*node).ty) != 0 {
                break;
            }
            if label.is_null() {
                if isloop((*node).ty) != 0 || (*node).ty == STM_SWITCH {
                    return node;
                }
            } else {
                if matchlabel((*node).parent, label) != 0 {
                    return node;
                }
            }
            node = (*node).parent;
        }
        core::ptr::null_mut()
    }
}

unsafe fn continuetarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast, label: *const c_char) -> *mut js_Ast {
    unsafe {
        while !node.is_null() {
            if isfun((*node).ty) != 0 {
                break;
            }
            if isloop((*node).ty) != 0 {
                if label.is_null() {
                    return node;
                } else if matchlabel((*node).parent, label) != 0 {
                    return node;
                }
            }
            node = (*node).parent;
        }
        core::ptr::null_mut()
    }
}

unsafe fn returntarget(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast) -> *mut js_Ast {
    unsafe {
        while !node.is_null() {
            if isfun((*node).ty) != 0 {
                return node;
            }
            node = (*node).parent;
        }
        core::ptr::null_mut()
    }
}

/* Emit code to rebalance stack and scopes during an abrupt exit */

unsafe fn cexit(J: *mut js_State, F: *mut js_Function, T: c_int, mut node: *mut js_Ast, target: *mut js_Ast) {
    unsafe {
        let mut prev: *mut js_Ast;
        loop {
            prev = node;
            node = (*node).parent;
            match (*node).ty {
                STM_WITH => {
                    emitline(J, F, node);
                    emit(J, F, OP_ENDWITH);
                }
                STM_FOR_IN | STM_FOR_IN_VAR => {
                    emitline(J, F, node);
                    /* pop the iterator if leaving the loop */
                    if (*F).script != 0 {
                        if T == STM_RETURN
                            || T == STM_BREAK
                            || (T == STM_CONTINUE && target != node)
                        {
                            /* pop the iterator, save the return or exp value */
                            emit(J, F, OP_ROT2);
                            emit(J, F, OP_POP);
                        }
                        if T == STM_CONTINUE {
                            emit(J, F, OP_ROT2); /* put the iterator back on top */
                        }
                    } else {
                        if T == STM_RETURN {
                            /* pop the iterator, save the return value */
                            emit(J, F, OP_ROT2);
                            emit(J, F, OP_POP);
                        }
                        if T == STM_BREAK || (T == STM_CONTINUE && target != node) {
                            emit(J, F, OP_POP); /* pop the iterator */
                        }
                    }
                }
                STM_TRY => {
                    emitline(J, F, node);
                    /* came from try block */
                    if prev == (*node).a {
                        emit(J, F, OP_ENDTRY);
                        if !(*node).d.is_null() {
                            cstm(J, F, (*node).d); /* finally */
                        }
                    }
                    /* came from catch block */
                    if prev == (*node).c {
                        /* ... with finally */
                        if !(*node).d.is_null() {
                            emit(J, F, OP_ENDCATCH);
                            emit(J, F, OP_ENDTRY);
                            cstm(J, F, (*node).d); /* finally */
                        } else {
                            emit(J, F, OP_ENDCATCH);
                        }
                    }
                }
                _ => {
                    /* impossible */
                }
            }
            if node == target {
                break;
            }
        }
    }
}

/* Try/catch/finally */

unsafe fn ctryfinally(J: *mut js_State, F: *mut js_Function, trystm: *mut js_Ast, finallystm: *mut js_Ast) {
    unsafe {
        let L1: c_int;
        L1 = emitjump(J, F, OP_TRY);
        {
            /* if we get here, we have caught an exception in the try block */
            cstm(J, F, finallystm); /* inline finally block */
            emit(J, F, OP_THROW); /* rethrow exception */
        }
        label(J, F, L1);
        cstm(J, F, trystm);
        emit(J, F, OP_ENDTRY);
        cstm(J, F, finallystm);
    }
}

unsafe fn ctrycatch(J: *mut js_State, F: *mut js_Function, trystm: *mut js_Ast, catchvar: *mut js_Ast, catchstm: *mut js_Ast) {
    unsafe {
        let L1: c_int;
        let L2: c_int;
        L1 = emitjump(J, F, OP_TRY);
        {
            /* if we get here, we have caught an exception in the try block */
            checkfutureword(J, F, catchvar);
            if (*F).strict != 0 {
                if strcmp((*catchvar).string, c"arguments".as_ptr()) == 0 {
                    jsC_error!(J, catchvar, c"redefining 'arguments' is not allowed in strict mode".as_ptr());
                }
                if strcmp((*catchvar).string, c"eval".as_ptr()) == 0 {
                    jsC_error!(J, catchvar, c"redefining 'eval' is not allowed in strict mode".as_ptr());
                }
            }
            emitline(J, F, catchvar);
            emitstring(J, F, OP_CATCH, (*catchvar).string);
            cstm(J, F, catchstm);
            emit(J, F, OP_ENDCATCH);
            L2 = emitjump(J, F, OP_JUMP); /* skip past the try block */
        }
        label(J, F, L1);
        cstm(J, F, trystm);
        emit(J, F, OP_ENDTRY);
        label(J, F, L2);
    }
}

unsafe fn ctrycatchfinally(
    J: *mut js_State,
    F: *mut js_Function,
    trystm: *mut js_Ast,
    catchvar: *mut js_Ast,
    catchstm: *mut js_Ast,
    finallystm: *mut js_Ast,
) {
    unsafe {
        let L1: c_int;
        let L2: c_int;
        let L3: c_int;
        L1 = emitjump(J, F, OP_TRY);
        {
            /* if we get here, we have caught an exception in the try block */
            L2 = emitjump(J, F, OP_TRY);
            {
                /* if we get here, we have caught an exception in the catch block */
                cstm(J, F, finallystm); /* inline finally block */
                emit(J, F, OP_THROW); /* rethrow exception */
            }
            label(J, F, L2);
            if (*F).strict != 0 {
                checkfutureword(J, F, catchvar);
                if strcmp((*catchvar).string, c"arguments".as_ptr()) == 0 {
                    jsC_error!(J, catchvar, c"redefining 'arguments' is not allowed in strict mode".as_ptr());
                }
                if strcmp((*catchvar).string, c"eval".as_ptr()) == 0 {
                    jsC_error!(J, catchvar, c"redefining 'eval' is not allowed in strict mode".as_ptr());
                }
            }
            emitline(J, F, catchvar);
            emitstring(J, F, OP_CATCH, (*catchvar).string);
            cstm(J, F, catchstm);
            emit(J, F, OP_ENDCATCH);
            emit(J, F, OP_ENDTRY);
            L3 = emitjump(J, F, OP_JUMP); /* skip past the try block to the finally block */
        }
        label(J, F, L1);
        cstm(J, F, trystm);
        emit(J, F, OP_ENDTRY);
        label(J, F, L3);
        cstm(J, F, finallystm);
    }
}

/* Switch */

unsafe fn cswitch(J: *mut js_State, F: *mut js_Function, ref_: *mut js_Ast, head: *mut js_Ast) {
    unsafe {
        let mut node: *mut js_Ast;
        let mut clause: *mut js_Ast;
        let mut def: *mut js_Ast = core::ptr::null_mut();
        let mut end: c_int;

        cexp(J, F, ref_);

        /* emit an if-else chain of tests for the case clause expressions */
        node = head;
        while !node.is_null() {
            clause = (*node).a;
            if (*clause).ty == STM_DEFAULT {
                if !def.is_null() {
                    jsC_error!(J, clause, c"more than one default label in switch".as_ptr());
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

        /* emit the case clause bodies */
        node = head;
        while !node.is_null() {
            clause = (*node).a;
            label(J, F, (*clause).casejump);
            if (*clause).ty == STM_DEFAULT {
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
}

/* Statements */

unsafe fn cvarinit(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    unsafe {
        while !list.is_null() {
            let var: *mut js_Ast = (*list).a;
            if !(*var).b.is_null() {
                cexp(J, F, (*var).b);
                emitline(J, F, var);
                emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*var).a);
                emit(J, F, OP_POP);
            }
            list = (*list).b;
        }
    }
}

unsafe fn cstm(J: *mut js_State, F: *mut js_Function, mut stm: *mut js_Ast) {
    unsafe {
        let mut target: *mut js_Ast;
        let mut loop_: c_int;
        let mut cont: c_int;
        let mut then: c_int;
        let mut end: c_int;

        emitline(J, F, stm);

        match (*stm).ty {
            AST_FUNDEC => {}

            STM_BLOCK => {
                cstmlist(J, F, (*stm).a);
            }

            STM_EMPTY => {
                if (*F).script != 0 {
                    emitline(J, F, stm);
                    emit(J, F, OP_POP);
                    emit(J, F, OP_UNDEF);
                }
            }

            STM_VAR => {
                cvarinit(J, F, (*stm).a);
            }

            STM_IF => {
                if !(*stm).c.is_null() {
                    cexp(J, F, (*stm).a);
                    emitline(J, F, stm);
                    then = emitjump(J, F, OP_JTRUE);
                    cstm(J, F, (*stm).c);
                    emitline(J, F, stm);
                    end = emitjump(J, F, OP_JUMP);
                    label(J, F, then);
                    cstm(J, F, (*stm).b);
                    label(J, F, end);
                } else {
                    cexp(J, F, (*stm).a);
                    emitline(J, F, stm);
                    end = emitjump(J, F, OP_JFALSE);
                    cstm(J, F, (*stm).b);
                    label(J, F, end);
                }
            }

            STM_DO => {
                loop_ = here(J, F);
                cstm(J, F, (*stm).a);
                cont = here(J, F);
                cexp(J, F, (*stm).b);
                emitline(J, F, stm);
                emitjumpto(J, F, OP_JTRUE, loop_);
                labeljumps(J, F, stm, here(J, F), cont);
            }

            STM_WHILE => {
                loop_ = here(J, F);
                cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JFALSE);
                cstm(J, F, (*stm).b);
                emitline(J, F, stm);
                emitjumpto(J, F, OP_JUMP, loop_);
                label(J, F, end);
                labeljumps(J, F, stm, here(J, F), loop_);
            }

            STM_FOR | STM_FOR_VAR => {
                if (*stm).ty == STM_FOR_VAR {
                    cvarinit(J, F, (*stm).a);
                } else {
                    if !(*stm).a.is_null() {
                        cexp(J, F, (*stm).a);
                        emit(J, F, OP_POP);
                    }
                }
                loop_ = here(J, F);
                if !(*stm).b.is_null() {
                    cexp(J, F, (*stm).b);
                    emitline(J, F, stm);
                    end = emitjump(J, F, OP_JFALSE);
                } else {
                    end = 0;
                }
                cstm(J, F, (*stm).d);
                cont = here(J, F);
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

            STM_FOR_IN | STM_FOR_IN_VAR => {
                cexp(J, F, (*stm).b);
                emitline(J, F, stm);
                emit(J, F, OP_ITERATOR);
                loop_ = here(J, F);
                {
                    emitline(J, F, stm);
                    emit(J, F, OP_NEXTITER);
                    end = emitjump(J, F, OP_JFALSE);
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
                }
                label(J, F, end);
                labeljumps(J, F, stm, here(J, F), loop_);
            }

            STM_SWITCH => {
                cswitch(J, F, (*stm).a, (*stm).b);
                labeljumps(J, F, stm, here(J, F), 0);
            }

            STM_LABEL => {
                cstm(J, F, (*stm).b);
                /* skip consecutive labels */
                while (*stm).ty == STM_LABEL {
                    stm = (*stm).b;
                }
                /* loops and switches have already been labelled */
                if isloop((*stm).ty) == 0 && (*stm).ty != STM_SWITCH {
                    labeljumps(J, F, stm, here(J, F), 0);
                }
            }

            STM_BREAK => {
                if !(*stm).a.is_null() {
                    checkfutureword(J, F, (*stm).a);
                    target = breaktarget(J, F, (*stm).parent, (*(*stm).a).string);
                    if target.is_null() {
                        jsC_error!(J, stm, c"break label '%s' not found".as_ptr(), (*(*stm).a).string);
                    }
                } else {
                    target = breaktarget(J, F, (*stm).parent, core::ptr::null());
                    if target.is_null() {
                        jsC_error!(J, stm, c"unlabelled break must be inside loop or switch".as_ptr());
                    }
                }
                cexit(J, F, STM_BREAK, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_BREAK, target, emitjump(J, F, OP_JUMP));
            }

            STM_CONTINUE => {
                if !(*stm).a.is_null() {
                    checkfutureword(J, F, (*stm).a);
                    target = continuetarget(J, F, (*stm).parent, (*(*stm).a).string);
                    if target.is_null() {
                        jsC_error!(J, stm, c"continue label '%s' not found".as_ptr(), (*(*stm).a).string);
                    }
                } else {
                    target = continuetarget(J, F, (*stm).parent, core::ptr::null());
                    if target.is_null() {
                        jsC_error!(J, stm, c"continue must be inside loop".as_ptr());
                    }
                }
                cexit(J, F, STM_CONTINUE, stm, target);
                emitline(J, F, stm);
                addjump(J, F, STM_CONTINUE, target, emitjump(J, F, OP_JUMP));
            }

            STM_RETURN => {
                if !(*stm).a.is_null() {
                    cexp(J, F, (*stm).a);
                } else {
                    emit(J, F, OP_UNDEF);
                }
                target = returntarget(J, F, (*stm).parent);
                if target.is_null() {
                    jsC_error!(J, stm, c"return not in function".as_ptr());
                }
                cexit(J, F, STM_RETURN, stm, target);
                emitline(J, F, stm);
                emit(J, F, OP_RETURN);
            }

            STM_THROW => {
                cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                emit(J, F, OP_THROW);
            }

            STM_WITH => {
                (*F).lightweight = 0;
                if (*F).strict != 0 {
                    jsC_error!(J, (*stm).a, c"'with' statements are not allowed in strict mode".as_ptr());
                }
                cexp(J, F, (*stm).a);
                emitline(J, F, stm);
                emit(J, F, OP_WITH);
                cstm(J, F, (*stm).b);
                emitline(J, F, stm);
                emit(J, F, OP_ENDWITH);
            }

            STM_TRY => {
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

            STM_DEBUGGER => {
                emitline(J, F, stm);
                emit(J, F, OP_DEBUGGER);
            }

            _ => {
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
}

unsafe fn cstmlist(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    unsafe {
        while !list.is_null() {
            cstm(J, F, (*list).a);
            list = (*list).b;
        }
    }
}

/* Declarations and programs */

unsafe fn listlength(mut list: *mut js_Ast) -> c_int {
    unsafe {
        let mut n: c_int = 0;
        while !list.is_null() {
            n += 1;
            list = (*list).b;
        }
        n
    }
}

unsafe fn cparams(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast, fname: *mut js_Ast) {
    unsafe {
        (*F).numparams = listlength(list);
        while !list.is_null() {
            checkfutureword(J, F, (*list).a);
            addlocal(J, F, (*list).a, 0);
            list = (*list).b;
        }
    }
}

unsafe fn cvardecs(J: *mut js_State, F: *mut js_Function, mut node: *mut js_Ast) {
    unsafe {
        if (*node).ty == AST_LIST {
            while !node.is_null() {
                cvardecs(J, F, (*node).a);
                node = (*node).b;
            }
            return;
        }

        if isfun((*node).ty) != 0 {
            return; /* stop at inner functions */
        }

        if (*node).ty == EXP_VAR {
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
}

unsafe fn cfundecs(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    unsafe {
        while !list.is_null() {
            let stm: *mut js_Ast = (*list).a;
            if (*stm).ty == AST_FUNDEC {
                emitline(J, F, stm);
                emitfunction(
                    J,
                    F,
                    newfun(J, (*stm).line, (*stm).a, (*stm).b, (*stm).c, 0, (*F).strict, 0),
                );
                emitline(J, F, stm);
                emit(J, F, OP_SETLOCAL);
                emitarg(J, F, addlocal(J, F, (*stm).a, 1));
                emit(J, F, OP_POP);
            }
            list = (*list).b;
        }
    }
}

unsafe fn cfunbody(
    J: *mut js_State,
    F: *mut js_Function,
    name: *mut js_Ast,
    params: *mut js_Ast,
    body: *mut js_Ast,
    is_fun_exp: c_int,
) {
    unsafe {
        (*F).lightweight = 1;
        (*F).arguments = 0;

        if (*F).script != 0 {
            (*F).lightweight = 0;
        }

        /* Check if first statement is 'use strict': */
        if !body.is_null()
            && (*body).ty == AST_LIST
            && !(*body).a.is_null()
            && (*(*body).a).ty == EXP_STRING
        {
            if strcmp((*(*body).a).string, c"use strict".as_ptr()) == 0 {
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
                    /* TODO: make this binding immutable! */
                    emit(J, F, OP_CURRENT);
                    emit(J, F, OP_SETLOCAL);
                    emitarg(J, F, addlocal(J, F, name, 1));
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsC_compilefunction(J: *mut js_State, prog: *mut js_Ast) -> *mut js_Function {
    unsafe { newfun(J, (*prog).line, (*prog).a, (*prog).b, (*prog).c, 0, (*J).default_strict, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsC_compilescript(
    J: *mut js_State,
    prog: *mut js_Ast,
    default_strict: c_int,
) -> *mut js_Function {
    unsafe {
        newfun(
            J,
            if !prog.is_null() { (*prog).line } else { 0 },
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            prog,
            1,
            default_strict,
            0,
        )
    }
}
