//! Translation of jscompile.c

use crate::*;

/* jsC_error(J, node, fmt, ...) lives in varargs.rs; format locally and call the
 * non-variadic core with a msgbuf of exactly the same size as the C original. */
macro_rules! jsC_error {
    ($J:expr, $node:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut __b: [c_char; 256] = [0; 256];
        snprintf(__b.as_mut_ptr(), 256, cs!($fmt) $(, $a)*);
        crate::varargs::jsC_error_str($J, $node, __b.as_ptr())
    }};
}

static futurewords: [&str; 7] = [
    "class\0",
    "const\0",
    "enum\0",
    "export\0",
    "extends\0",
    "import\0",
    "super\0",
];

static strictfuturewords: [&str; 9] = [
    "implements\0",
    "interface\0",
    "let\0",
    "package\0",
    "private\0",
    "protected\0",
    "public\0",
    "static\0",
    "yield\0",
];

unsafe fn checkfutureword(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let mut list: [*const c_char; 7] = [null(); 7];
    let mut k: usize = 0;
    while k < 7 {
        list[k] = futurewords[k].as_ptr() as *const c_char;
        k += 1;
    }
    if jsY_findword((*exp).string, list.as_ptr(), 7) >= 0 {
        jsC_error!(
            J,
            exp,
            "'%s' is a future reserved word",
            (*exp).string
        );
    }
    if (*F).strict != 0 {
        let mut slist: [*const c_char; 9] = [null(); 9];
        let mut k: usize = 0;
        while k < 9 {
            slist[k] = strictfuturewords[k].as_ptr() as *const c_char;
            k += 1;
        }
        if jsY_findword((*exp).string, slist.as_ptr(), 9) >= 0 {
            jsC_error!(
                J,
                exp,
                "'%s' is a strict mode future reserved word",
                (*exp).string
            );
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
    let F: *mut js_Function =
        js_malloc(J, core::mem::size_of::<js_Function>() as c_int) as *mut js_Function;
    memset(
        F as *mut c_void,
        0,
        core::mem::size_of::<js_Function>(),
    );
    (*F).gcmark = 0;
    (*F).gcnext = (*J).gcfun;
    (*J).gcfun = F;
    (*J).gccounter += 1;

    (*F).filename = js_intern(J, (*J).filename);
    (*F).line = line;
    (*F).script = script;
    (*F).strict = default_strict;
    (*F).name = if !name.is_null() {
        (*name).string
    } else {
        cs!("")
    };

    cfunbody(J, F, name, params, body, is_fun_exp);

    F
}

/* Emit opcodes, constants and jumps */

unsafe fn emitraw(J: *mut js_State, F: *mut js_Function, value: c_int) {
    if value != (value as js_Instruction) as c_int {
        js_syntaxerror!(J, "integer overflow in instruction coding");
    }
    if (*F).codelen >= (*F).codecap {
        (*F).codecap = if (*F).codecap != 0 {
            (*F).codecap * 2
        } else {
            64
        };
        (*F).code = js_realloc(
            J,
            (*F).code as *mut c_void,
            ((*F).codecap as usize * core::mem::size_of::<js_Instruction>()) as c_int,
        ) as *mut js_Instruction;
    }
    *(*F).code.offset((*F).codelen as isize) = value as js_Instruction;
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
        (*F).funcap = if (*F).funcap != 0 {
            (*F).funcap * 2
        } else {
            16
        };
        (*F).funtab = js_realloc(
            J,
            (*F).funtab as *mut c_void,
            ((*F).funcap as usize * core::mem::size_of::<*mut js_Function>()) as c_int,
        ) as *mut *mut js_Function;
    }
    *(*F).funtab.offset((*F).funlen as isize) = value;
    let r = (*F).funlen;
    (*F).funlen += 1;
    r
}

unsafe fn addlocal(
    J: *mut js_State,
    F: *mut js_Function,
    ident: *mut js_Ast,
    reuse: c_int,
) -> c_int {
    let name: *const c_char = (*ident).string;
    if (*F).strict != 0 {
        if strcmp(name, cs!("arguments")) == 0 {
            jsC_error!(
                J,
                ident,
                "redefining 'arguments' is not allowed in strict mode"
            );
        }
        if strcmp(name, cs!("eval")) == 0 {
            jsC_error!(J, ident, "redefining 'eval' is not allowed in strict mode");
        }
    } else {
        if strcmp(name, cs!("eval")) == 0 {
            js_evalerror!(
                J,
                "%s:%d: invalid use of 'eval'",
                (*J).filename,
                (*ident).line
            );
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
                    jsC_error!(J, ident, "duplicate formal parameter '%s'", name);
                }
            }
            i += 1;
        }
    }
    if (*F).varlen >= (*F).varcap {
        (*F).varcap = if (*F).varcap != 0 {
            (*F).varcap * 2
        } else {
            16
        };
        (*F).vartab = js_realloc(
            J,
            (*F).vartab as *mut c_void,
            ((*F).varcap as usize * core::mem::size_of::<*const c_char>()) as c_int,
        ) as *mut *const c_char;
    }
    *(*F).vartab.offset((*F).varlen as isize) = name;
    (*F).varlen += 1;
    (*F).varlen
}

unsafe fn findlocal(J: *mut js_State, F: *mut js_Function, name: *const c_char) -> c_int {
    let mut i: c_int = (*F).varlen;
    while i > 0 {
        if strcmp(*(*F).vartab.offset((i - 1) as isize), name) == 0 {
            return i;
        }
        i -= 1;
    }
    -1
}

unsafe fn emitfunction(J: *mut js_State, F: *mut js_Function, fun: *mut js_Function) {
    (*F).lightweight = 0;
    emit(J, F, OP_CLOSURE);
    let a = addfunction(J, F, fun);
    emitarg(J, F, a);
}

unsafe fn emitnumber(J: *mut js_State, F: *mut js_Function, num: f64) {
    if num == 0.0 {
        emit(J, F, OP_INTEGER);
        emitarg(J, F, 32768);
        if signbit(num) {
            emit(J, F, OP_NEG);
        }
    } else if num >= -32768.0 && num <= 32767.0 && num == (num as c_int) as f64 {
        emit(J, F, OP_INTEGER);
        emitarg(J, F, (num + 32768.0) as c_int);
    } else {
        const N: usize = core::mem::size_of::<f64>() / core::mem::size_of::<js_Instruction>();
        let mut x: [js_Instruction; N] = [0; N];
        emit(J, F, OP_NUMBER);
        memcpy(
            x.as_mut_ptr() as *mut c_void,
            addr_of!(num) as *const c_void,
            core::mem::size_of::<f64>(),
        );
        let mut i: usize = 0;
        while i < N {
            emitarg(J, F, x[i] as c_int);
            i += 1;
        }
    }
}

unsafe fn emitstring(
    J: *mut js_State,
    F: *mut js_Function,
    opcode: c_int,
    str_: *const c_char,
) {
    const N: usize =
        core::mem::size_of::<*const c_char>() / core::mem::size_of::<js_Instruction>();
    let mut x: [js_Instruction; N] = [0; N];
    emit(J, F, opcode);
    memcpy(
        x.as_mut_ptr() as *mut c_void,
        addr_of!(str_) as *const c_void,
        core::mem::size_of::<*const c_char>(),
    );
    let mut i: usize = 0;
    while i < N {
        emitarg(J, F, x[i] as c_int);
        i += 1;
    }
}

unsafe fn emitlocal(
    J: *mut js_State,
    F: *mut js_Function,
    oploc: c_int,
    opvar: c_int,
    ident: *mut js_Ast,
) {
    let is_arguments: c_int = (strcmp((*ident).string, cs!("arguments")) == 0) as c_int;
    let is_eval: c_int = (strcmp((*ident).string, cs!("eval")) == 0) as c_int;
    let i: c_int;

    if is_arguments != 0 {
        (*F).lightweight = 0;
        (*F).arguments = 1;
    }

    checkfutureword(J, F, ident);
    if (*F).strict != 0 && oploc == OP_SETLOCAL {
        if is_arguments != 0 {
            jsC_error!(J, ident, "'arguments' is read-only in strict mode");
        }
        if is_eval != 0 {
            jsC_error!(J, ident, "'eval' is read-only in strict mode");
        }
    }
    if is_eval != 0 {
        js_evalerror!(
            J,
            "%s:%d: invalid use of 'eval'",
            (*J).filename,
            (*ident).line
        );
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
    let inst: c_int;
    emit(J, F, opcode);
    inst = (*F).codelen;
    emitarg(J, F, 0);
    inst
}

unsafe fn emitjumpto(J: *mut js_State, F: *mut js_Function, opcode: c_int, dest: c_int) {
    emit(J, F, opcode);
    if dest != (dest as js_Instruction) as c_int {
        js_syntaxerror!(J, "jump address integer overflow");
    }
    emitarg(J, F, dest);
}

unsafe fn labelto(J: *mut js_State, F: *mut js_Function, inst: c_int, addr: c_int) {
    if addr != (addr as js_Instruction) as c_int {
        js_syntaxerror!(J, "jump address integer overflow");
    }
    *(*F).code.offset(inst as isize) = addr as js_Instruction;
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
        cexp_(J, F, (*exp).a);
    }
    emitline(J, F, exp);
    emit(J, F, OP_TYPEOF);
}

unsafe fn cunary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    cexp_(J, F, (*exp).a);
    emitline(J, F, exp);
    emit(J, F, opcode);
}

unsafe fn cbinary(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    cexp_(J, F, (*exp).a);
    cexp_(J, F, (*exp).b);
    emitline(J, F, exp);
    emit(J, F, opcode);
}

unsafe fn carray(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    while !list.is_null() {
        emitline(J, F, (*list).a);
        if (*(*list).a).type_ == EXP_ELISION {
            emit(J, F, OP_SKIPARRAY);
        } else {
            cexp_(J, F, (*list).a);
            emit(J, F, OP_INITARRAY);
        }
        list = (*list).b;
    }
}

unsafe fn checkdup(
    J: *mut js_State,
    F: *mut js_Function,
    mut list: *mut js_Ast,
    end: *mut js_Ast,
) {
    let mut nbuf: [c_char; 32] = [0; 32];
    let mut sbuf: [c_char; 32] = [0; 32];
    let needle: *const c_char;
    let mut straw: *const c_char;

    if (*(*end).a).type_ == EXP_NUMBER {
        needle = jsV_numbertostring(J, nbuf.as_mut_ptr(), (*(*end).a).number);
    } else {
        needle = (*(*end).a).string;
    }

    while (*list).a != end {
        if (*(*list).a).type_ == (*end).type_ {
            let prop: *mut js_Ast = (*(*list).a).a;
            if (*prop).type_ == EXP_NUMBER {
                straw = jsV_numbertostring(J, sbuf.as_mut_ptr(), (*prop).number);
            } else {
                straw = (*prop).string;
            }
            if strcmp(needle, straw) == 0 {
                jsC_error!(
                    J,
                    list,
                    "duplicate property '%s' in object literal",
                    needle
                );
            }
        }
        list = (*list).b;
    }
}

unsafe fn cobject(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) {
    let head: *mut js_Ast = list;

    while !list.is_null() {
        let kv: *mut js_Ast = (*list).a;
        let prop: *mut js_Ast = (*kv).a;

        if (*prop).type_ == AST_IDENTIFIER || (*prop).type_ == EXP_STRING {
            emitline(J, F, prop);
            emitstring(J, F, OP_STRING, (*prop).string);
        } else if (*prop).type_ == EXP_NUMBER {
            emitline(J, F, prop);
            emitnumber(J, F, (*prop).number);
        } else {
            jsC_error!(J, prop, "invalid property name in object initializer");
        }

        if (*F).strict != 0 {
            checkdup(J, F, head, kv);
        }

        match (*kv).type_ {
            EXP_PROP_VAL => {
                cexp_(J, F, (*kv).b);
                emitline(J, F, kv);
                emit(J, F, OP_INITPROP);
            }
            EXP_PROP_GET => {
                let fun = newfun(
                    J,
                    (*prop).line,
                    null_mut(),
                    null_mut(),
                    (*kv).c,
                    0,
                    (*F).strict,
                    1,
                );
                emitfunction(J, F, fun);
                emitline(J, F, kv);
                emit(J, F, OP_INITGETTER);
            }
            EXP_PROP_SET => {
                let fun = newfun(
                    J,
                    (*prop).line,
                    null_mut(),
                    (*kv).b,
                    (*kv).c,
                    0,
                    (*F).strict,
                    1,
                );
                emitfunction(J, F, fun);
                emitline(J, F, kv);
                emit(J, F, OP_INITSETTER);
            }
            _ => { /* impossible */ }
        }

        list = (*list).b;
    }
}

unsafe fn cargs(J: *mut js_State, F: *mut js_Function, mut list: *mut js_Ast) -> c_int {
    let mut n: c_int = 0;
    while !list.is_null() {
        cexp_(J, F, (*list).a);
        list = (*list).b;
        n += 1;
    }
    n
}

unsafe fn cassign(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let lhs: *mut js_Ast = (*exp).a;
    let rhs: *mut js_Ast = (*exp).b;
    match (*lhs).type_ {
        EXP_IDENTIFIER => {
            cexp_(J, F, rhs);
            emitline(J, F, exp);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
        }
        EXP_INDEX => {
            cexp_(J, F, (*lhs).a);
            cexp_(J, F, (*lhs).b);
            cexp_(J, F, rhs);
            emitline(J, F, exp);
            emit(J, F, OP_SETPROP);
        }
        EXP_MEMBER => {
            cexp_(J, F, (*lhs).a);
            cexp_(J, F, rhs);
            emitline(J, F, exp);
            emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
        }
        _ => {
            jsC_error!(J, lhs, "invalid l-value in assignment");
        }
    }
}

unsafe fn cassignforin(J: *mut js_State, F: *mut js_Function, stm: *mut js_Ast) {
    let lhs: *mut js_Ast = (*stm).a;

    if (*stm).type_ == STM_FOR_IN_VAR {
        if !(*lhs).b.is_null() {
            jsC_error!(
                J,
                (*lhs).b,
                "more than one loop variable in for-in statement"
            );
        }
        emitline(J, F, (*lhs).a);
        emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*(*lhs).a).a); /* list(var-init(ident)) */
        emit(J, F, OP_POP);
        return;
    }

    match (*lhs).type_ {
        EXP_IDENTIFIER => {
            emitline(J, F, lhs);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, lhs);
            emit(J, F, OP_POP);
        }
        EXP_INDEX => {
            cexp_(J, F, (*lhs).a);
            cexp_(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT3);
            emit(J, F, OP_SETPROP);
            emit(J, F, OP_POP);
        }
        EXP_MEMBER => {
            cexp_(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_ROT2);
            emitstring(J, F, OP_SETPROP_S, (*(*lhs).b).string);
            emit(J, F, OP_POP);
        }
        _ => {
            jsC_error!(J, lhs, "invalid l-value in for-in loop assignment");
        }
    }
}

unsafe fn cassignop1(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast) {
    match (*lhs).type_ {
        EXP_IDENTIFIER => {
            emitline(J, F, lhs);
            emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, lhs);
        }
        EXP_INDEX => {
            cexp_(J, F, (*lhs).a);
            cexp_(J, F, (*lhs).b);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP2);
            emit(J, F, OP_GETPROP);
        }
        EXP_MEMBER => {
            cexp_(J, F, (*lhs).a);
            emitline(J, F, lhs);
            emit(J, F, OP_DUP);
            emitstring(J, F, OP_GETPROP_S, (*(*lhs).b).string);
        }
        _ => {
            jsC_error!(J, lhs, "invalid l-value in assignment");
        }
    }
}

unsafe fn cassignop2(J: *mut js_State, F: *mut js_Function, lhs: *mut js_Ast, postfix: c_int) {
    match (*lhs).type_ {
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
            jsC_error!(J, lhs, "invalid l-value in assignment");
        }
    }
}

unsafe fn cassignop(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast, opcode: c_int) {
    let lhs: *mut js_Ast = (*exp).a;
    let rhs: *mut js_Ast = (*exp).b;
    cassignop1(J, F, lhs);
    cexp_(J, F, rhs);
    emitline(J, F, exp);
    emit(J, F, opcode);
    cassignop2(J, F, lhs, 0);
}

unsafe fn cdelete(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let arg: *mut js_Ast = (*exp).a;
    match (*arg).type_ {
        EXP_IDENTIFIER => {
            if (*F).strict != 0 {
                jsC_error!(
                    J,
                    exp,
                    "delete on an unqualified name is not allowed in strict mode"
                );
            }
            emitline(J, F, exp);
            emitlocal(J, F, OP_DELLOCAL, OP_DELVAR, arg);
        }
        EXP_INDEX => {
            cexp_(J, F, (*arg).a);
            cexp_(J, F, (*arg).b);
            emitline(J, F, exp);
            emit(J, F, OP_DELPROP);
        }
        EXP_MEMBER => {
            cexp_(J, F, (*arg).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_DELPROP_S, (*(*arg).b).string);
        }
        _ => {
            jsC_error!(J, exp, "invalid l-value in delete expression");
        }
    }
}

unsafe fn ceval(J: *mut js_State, F: *mut js_Function, fun: *mut js_Ast, args: *mut js_Ast) {
    let mut n: c_int = cargs(J, F, args);
    (*F).lightweight = 0;
    (*F).arguments = 1;
    if n == 0 {
        emit(J, F, OP_UNDEF);
    } else {
        while {
            let t = n;
            n -= 1;
            t
        } > 1
        {
            emit(J, F, OP_POP);
        }
    }
    emit(J, F, OP_EVAL);
}

unsafe fn ccall(J: *mut js_State, F: *mut js_Function, fun: *mut js_Ast, args: *mut js_Ast) {
    let n: c_int;
    match (*fun).type_ {
        EXP_INDEX => {
            cexp_(J, F, (*fun).a);
            emit(J, F, OP_DUP);
            cexp_(J, F, (*fun).b);
            emit(J, F, OP_GETPROP);
            emit(J, F, OP_ROT2);
        }
        EXP_MEMBER => {
            cexp_(J, F, (*fun).a);
            emit(J, F, OP_DUP);
            emitstring(J, F, OP_GETPROP_S, (*(*fun).b).string);
            emit(J, F, OP_ROT2);
        }
        _ => {
            /* EXP_IDENTIFIER falls through to default */
            if (*fun).type_ == EXP_IDENTIFIER && strcmp((*fun).string, cs!("eval")) == 0 {
                ceval(J, F, fun, args);
                return;
            }
            cexp_(J, F, fun);
            emit(J, F, OP_UNDEF);
        }
    }
    n = cargs(J, F, args);
    emit(J, F, OP_CALL);
    emitarg(J, F, n);
}

unsafe fn cexp_(J: *mut js_State, F: *mut js_Function, exp: *mut js_Ast) {
    let then: c_int;
    let end: c_int;
    let n: c_int;

    match (*exp).type_ {
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
            let fun = newfun(
                J,
                (*exp).line,
                (*exp).a,
                (*exp).b,
                (*exp).c,
                0,
                (*F).strict,
                1,
            );
            emitfunction(J, F, fun);
        }

        EXP_IDENTIFIER => {
            emitline(J, F, exp);
            emitlocal(J, F, OP_GETLOCAL, OP_GETVAR, exp);
        }

        EXP_INDEX => {
            cexp_(J, F, (*exp).a);
            cexp_(J, F, (*exp).b);
            emitline(J, F, exp);
            emit(J, F, OP_GETPROP);
        }

        EXP_MEMBER => {
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            emitstring(J, F, OP_GETPROP_S, (*(*exp).b).string);
        }

        EXP_CALL => {
            ccall(J, F, (*exp).a, (*exp).b);
        }

        EXP_NEW => {
            cexp_(J, F, (*exp).a);
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
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP);
            emit(J, F, OP_UNDEF);
        }

        EXP_TYPEOF => ctypeof(J, F, exp),
        EXP_POS => cunary(J, F, exp, OP_POS),
        EXP_NEG => cunary(J, F, exp, OP_NEG),
        EXP_BITNOT => cunary(J, F, exp, OP_BITNOT),
        EXP_LOGNOT => cunary(J, F, exp, OP_LOGNOT),

        EXP_BITOR => cbinary(J, F, exp, OP_BITOR),
        EXP_BITXOR => cbinary(J, F, exp, OP_BITXOR),
        EXP_BITAND => cbinary(J, F, exp, OP_BITAND),
        EXP_EQ => cbinary(J, F, exp, OP_EQ),
        EXP_NE => cbinary(J, F, exp, OP_NE),
        EXP_STRICTEQ => cbinary(J, F, exp, OP_STRICTEQ),
        EXP_STRICTNE => cbinary(J, F, exp, OP_STRICTNE),
        EXP_LT => cbinary(J, F, exp, OP_LT),
        EXP_GT => cbinary(J, F, exp, OP_GT),
        EXP_LE => cbinary(J, F, exp, OP_LE),
        EXP_GE => cbinary(J, F, exp, OP_GE),
        EXP_INSTANCEOF => cbinary(J, F, exp, OP_INSTANCEOF),
        EXP_IN => cbinary(J, F, exp, OP_IN),
        EXP_SHL => cbinary(J, F, exp, OP_SHL),
        EXP_SHR => cbinary(J, F, exp, OP_SHR),
        EXP_USHR => cbinary(J, F, exp, OP_USHR),
        EXP_ADD => cbinary(J, F, exp, OP_ADD),
        EXP_SUB => cbinary(J, F, exp, OP_SUB),
        EXP_MUL => cbinary(J, F, exp, OP_MUL),
        EXP_DIV => cbinary(J, F, exp, OP_DIV),
        EXP_MOD => cbinary(J, F, exp, OP_MOD),

        EXP_ASS => cassign(J, F, exp),
        EXP_ASS_MUL => cassignop(J, F, exp, OP_MUL),
        EXP_ASS_DIV => cassignop(J, F, exp, OP_DIV),
        EXP_ASS_MOD => cassignop(J, F, exp, OP_MOD),
        EXP_ASS_ADD => cassignop(J, F, exp, OP_ADD),
        EXP_ASS_SUB => cassignop(J, F, exp, OP_SUB),
        EXP_ASS_SHL => cassignop(J, F, exp, OP_SHL),
        EXP_ASS_SHR => cassignop(J, F, exp, OP_SHR),
        EXP_ASS_USHR => cassignop(J, F, exp, OP_USHR),
        EXP_ASS_BITAND => cassignop(J, F, exp, OP_BITAND),
        EXP_ASS_BITXOR => cassignop(J, F, exp, OP_BITXOR),
        EXP_ASS_BITOR => cassignop(J, F, exp, OP_BITOR),

        EXP_COMMA => {
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_POP);
            cexp_(J, F, (*exp).b);
        }

        EXP_LOGOR => {
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP);
            end = emitjump(J, F, OP_JTRUE);
            emit(J, F, OP_POP);
            cexp_(J, F, (*exp).b);
            label(J, F, end);
        }

        EXP_LOGAND => {
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            emit(J, F, OP_DUP);
            end = emitjump(J, F, OP_JFALSE);
            emit(J, F, OP_POP);
            cexp_(J, F, (*exp).b);
            label(J, F, end);
        }

        EXP_COND => {
            cexp_(J, F, (*exp).a);
            emitline(J, F, exp);
            then = emitjump(J, F, OP_JTRUE);
            cexp_(J, F, (*exp).c);
            end = emitjump(J, F, OP_JUMP);
            label(J, F, then);
            cexp_(J, F, (*exp).b);
            label(J, F, end);
        }

        _ => {
            jsC_error!(J, exp, "unknown expression type");
        }
    }
}

/* Patch break and continue statements */

unsafe fn addjump(
    J: *mut js_State,
    F: *mut js_Function,
    type_: c_int,
    target: *mut js_Ast,
    inst: c_int,
) {
    let jump: *mut js_JumpList =
        js_malloc(J, core::mem::size_of::<js_JumpList>() as c_int) as *mut js_JumpList;
    (*jump).type_ = type_;
    (*jump).inst = inst;
    (*jump).next = (*target).jumps;
    (*target).jumps = jump;
}

unsafe fn labeljumps(
    J: *mut js_State,
    F: *mut js_Function,
    stm: *mut js_Ast,
    baddr: c_int,
    caddr: c_int,
) {
    let mut jump: *mut js_JumpList = (*stm).jumps;
    while !jump.is_null() {
        let next: *mut js_JumpList = (*jump).next;
        if (*jump).type_ == STM_BREAK {
            labelto(J, F, (*jump).inst, baddr);
        }
        if (*jump).type_ == STM_CONTINUE {
            labelto(J, F, (*jump).inst, caddr);
        }
        js_free(J, jump as *mut c_void);
        jump = next;
    }
    (*stm).jumps = null_mut();
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
    while !node.is_null() && (*node).type_ == STM_LABEL {
        if strcmp((*(*node).a).string, label) == 0 {
            return 1;
        }
        node = (*node).parent;
    }
    0
}

unsafe fn breaktarget(
    J: *mut js_State,
    F: *mut js_Function,
    mut node: *mut js_Ast,
    label: *const c_char,
) -> *mut js_Ast {
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
    null_mut()
}

unsafe fn continuetarget(
    J: *mut js_State,
    F: *mut js_Function,
    mut node: *mut js_Ast,
    label: *const c_char,
) -> *mut js_Ast {
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
    null_mut()
}

unsafe fn returntarget(
    J: *mut js_State,
    F: *mut js_Function,
    mut node: *mut js_Ast,
) -> *mut js_Ast {
    while !node.is_null() {
        if isfun((*node).type_) != 0 {
            return node;
        }
        node = (*node).parent;
    }
    null_mut()
}

/* Emit code to rebalance stack and scopes during an abrupt exit */

unsafe fn cexit(
    J: *mut js_State,
    F: *mut js_Function,
    T: c_int,
    node0: *mut js_Ast,
    target: *mut js_Ast,
) {
    let mut prev: *mut js_Ast;
    let mut node: *mut js_Ast = node0;
    loop {
        prev = node;
        node = (*node).parent;
        match (*node).type_ {
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
            _ => { /* impossible */ }
        }
        if node == target {
            break;
        }
    }
}

/* Try/catch/finally */

unsafe fn ctryfinally(
    J: *mut js_State,
    F: *mut js_Function,
    trystm: *mut js_Ast,
    finallystm: *mut js_Ast,
) {
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

unsafe fn ctrycatch(
    J: *mut js_State,
    F: *mut js_Function,
    trystm: *mut js_Ast,
    catchvar: *mut js_Ast,
    catchstm: *mut js_Ast,
) {
    let L1: c_int;
    let L2: c_int;
    L1 = emitjump(J, F, OP_TRY);
    {
        /* if we get here, we have caught an exception in the try block */
        checkfutureword(J, F, catchvar);
        if (*F).strict != 0 {
            if strcmp((*catchvar).string, cs!("arguments")) == 0 {
                jsC_error!(
                    J,
                    catchvar,
                    "redefining 'arguments' is not allowed in strict mode"
                );
            }
            if strcmp((*catchvar).string, cs!("eval")) == 0 {
                jsC_error!(
                    J,
                    catchvar,
                    "redefining 'eval' is not allowed in strict mode"
                );
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

unsafe fn ctrycatchfinally(
    J: *mut js_State,
    F: *mut js_Function,
    trystm: *mut js_Ast,
    catchvar: *mut js_Ast,
    catchstm: *mut js_Ast,
    finallystm: *mut js_Ast,
) {
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
            if strcmp((*catchvar).string, cs!("arguments")) == 0 {
                jsC_error!(
                    J,
                    catchvar,
                    "redefining 'arguments' is not allowed in strict mode"
                );
            }
            if strcmp((*catchvar).string, cs!("eval")) == 0 {
                jsC_error!(
                    J,
                    catchvar,
                    "redefining 'eval' is not allowed in strict mode"
                );
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

/* Switch */

unsafe fn cswitch(J: *mut js_State, F: *mut js_Function, ref_: *mut js_Ast, head: *mut js_Ast) {
    let mut node: *mut js_Ast;
    let mut clause: *mut js_Ast;
    let mut def: *mut js_Ast = null_mut();
    let end: c_int;

    cexp_(J, F, ref_);

    /* emit an if-else chain of tests for the case clause expressions */
    node = head;
    while !node.is_null() {
        clause = (*node).a;
        if (*clause).type_ == STM_DEFAULT {
            if !def.is_null() {
                jsC_error!(J, clause, "more than one default label in switch");
            }
            def = clause;
        } else {
            cexp_(J, F, (*clause).a);
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
        let var: *mut js_Ast = (*list).a;
        if !(*var).b.is_null() {
            cexp_(J, F, (*var).b);
            emitline(J, F, var);
            emitlocal(J, F, OP_SETLOCAL, OP_SETVAR, (*var).a);
            emit(J, F, OP_POP);
        }
        list = (*list).b;
    }
}

unsafe fn cstm(J: *mut js_State, F: *mut js_Function, stm0: *mut js_Ast) {
    let mut stm: *mut js_Ast = stm0;
    let target: *mut js_Ast;
    let loop_: c_int;
    let cont: c_int;
    let then: c_int;
    let mut end: c_int;

    emitline(J, F, stm);

    match (*stm).type_ {
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
                cexp_(J, F, (*stm).a);
                emitline(J, F, stm);
                then = emitjump(J, F, OP_JTRUE);
                cstm(J, F, (*stm).c);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JUMP);
                label(J, F, then);
                cstm(J, F, (*stm).b);
                label(J, F, end);
            } else {
                cexp_(J, F, (*stm).a);
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
            cexp_(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JTRUE, loop_);
            let h = here(J, F);
            labeljumps(J, F, stm, h, cont);
        }

        STM_WHILE => {
            loop_ = here(J, F);
            cexp_(J, F, (*stm).a);
            emitline(J, F, stm);
            end = emitjump(J, F, OP_JFALSE);
            cstm(J, F, (*stm).b);
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP, loop_);
            label(J, F, end);
            let h = here(J, F);
            labeljumps(J, F, stm, h, loop_);
        }

        STM_FOR | STM_FOR_VAR => {
            if (*stm).type_ == STM_FOR_VAR {
                cvarinit(J, F, (*stm).a);
            } else {
                if !(*stm).a.is_null() {
                    cexp_(J, F, (*stm).a);
                    emit(J, F, OP_POP);
                }
            }
            loop_ = here(J, F);
            if !(*stm).b.is_null() {
                cexp_(J, F, (*stm).b);
                emitline(J, F, stm);
                end = emitjump(J, F, OP_JFALSE);
            } else {
                end = 0;
            }
            cstm(J, F, (*stm).d);
            cont = here(J, F);
            if !(*stm).c.is_null() {
                cexp_(J, F, (*stm).c);
                emit(J, F, OP_POP);
            }
            emitline(J, F, stm);
            emitjumpto(J, F, OP_JUMP, loop_);
            if end != 0 {
                label(J, F, end);
            }
            let h = here(J, F);
            labeljumps(J, F, stm, h, cont);
        }

        STM_FOR_IN | STM_FOR_IN_VAR => {
            cexp_(J, F, (*stm).b);
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
            let h = here(J, F);
            labeljumps(J, F, stm, h, loop_);
        }

        STM_SWITCH => {
            cswitch(J, F, (*stm).a, (*stm).b);
            let h = here(J, F);
            labeljumps(J, F, stm, h, 0);
        }

        STM_LABEL => {
            cstm(J, F, (*stm).b);
            /* skip consecutive labels */
            while (*stm).type_ == STM_LABEL {
                stm = (*stm).b;
            }
            /* loops and switches have already been labelled */
            if isloop((*stm).type_) == 0 && (*stm).type_ != STM_SWITCH {
                let h = here(J, F);
                labeljumps(J, F, stm, h, 0);
            }
        }

        STM_BREAK => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                target = breaktarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error!(J, stm, "break label '%s' not found", (*(*stm).a).string);
                }
            } else {
                target = breaktarget(J, F, (*stm).parent, null());
                if target.is_null() {
                    jsC_error!(J, stm, "unlabelled break must be inside loop or switch");
                }
            }
            cexit(J, F, STM_BREAK, stm, target);
            emitline(J, F, stm);
            let j = emitjump(J, F, OP_JUMP);
            addjump(J, F, STM_BREAK, target, j);
        }

        STM_CONTINUE => {
            if !(*stm).a.is_null() {
                checkfutureword(J, F, (*stm).a);
                target = continuetarget(J, F, (*stm).parent, (*(*stm).a).string);
                if target.is_null() {
                    jsC_error!(J, stm, "continue label '%s' not found", (*(*stm).a).string);
                }
            } else {
                target = continuetarget(J, F, (*stm).parent, null());
                if target.is_null() {
                    jsC_error!(J, stm, "continue must be inside loop");
                }
            }
            cexit(J, F, STM_CONTINUE, stm, target);
            emitline(J, F, stm);
            let j = emitjump(J, F, OP_JUMP);
            addjump(J, F, STM_CONTINUE, target, j);
        }

        STM_RETURN => {
            if !(*stm).a.is_null() {
                cexp_(J, F, (*stm).a);
            } else {
                emit(J, F, OP_UNDEF);
            }
            target = returntarget(J, F, (*stm).parent);
            if target.is_null() {
                jsC_error!(J, stm, "return not in function");
            }
            cexit(J, F, STM_RETURN, stm, target);
            emitline(J, F, stm);
            emit(J, F, OP_RETURN);
        }

        STM_THROW => {
            cexp_(J, F, (*stm).a);
            emitline(J, F, stm);
            emit(J, F, OP_THROW);
        }

        STM_WITH => {
            (*F).lightweight = 0;
            if (*F).strict != 0 {
                jsC_error!(
                    J,
                    (*stm).a,
                    "'with' statements are not allowed in strict mode"
                );
            }
            cexp_(J, F, (*stm).a);
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
                cexp_(J, F, stm);
            } else {
                cexp_(J, F, stm);
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
    let mut n: c_int = 0;
    while !list.is_null() {
        n += 1;
        list = (*list).b;
    }
    n
}

unsafe fn cparams(
    J: *mut js_State,
    F: *mut js_Function,
    mut list: *mut js_Ast,
    fname: *mut js_Ast,
) {
    (*F).numparams = listlength(list);
    while !list.is_null() {
        checkfutureword(J, F, (*list).a);
        addlocal(J, F, (*list).a, 0);
        list = (*list).b;
    }
}

unsafe fn cvardecs(J: *mut js_State, F: *mut js_Function, node0: *mut js_Ast) {
    let mut node: *mut js_Ast = node0;
    if (*node).type_ == AST_LIST {
        while !node.is_null() {
            cvardecs(J, F, (*node).a);
            node = (*node).b;
        }
        return;
    }

    if isfun((*node).type_) != 0 {
        return; /* stop at inner functions */
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
        let stm: *mut js_Ast = (*list).a;
        if (*stm).type_ == AST_FUNDEC {
            emitline(J, F, stm);
            let fun = newfun(
                J,
                (*stm).line,
                (*stm).a,
                (*stm).b,
                (*stm).c,
                0,
                (*F).strict,
                0,
            );
            emitfunction(J, F, fun);
            emitline(J, F, stm);
            emit(J, F, OP_SETLOCAL);
            let a = addlocal(J, F, (*stm).a, 1);
            emitarg(J, F, a);
            emit(J, F, OP_POP);
        }
        list = (*list).b;
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
    (*F).lightweight = 1;
    (*F).arguments = 0;

    if (*F).script != 0 {
        (*F).lightweight = 0;
    }

    /* Check if first statement is 'use strict': */
    if !body.is_null()
        && (*body).type_ == AST_LIST
        && !(*body).a.is_null()
        && (*(*body).a).type_ == EXP_STRING
    {
        if strcmp((*(*body).a).string, cs!("use strict")) == 0 {
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
                let a = addlocal(J, F, name, 1);
                emitarg(J, F, a);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_compilefunction(
    J: *mut js_State,
    prog: *mut js_Ast,
) -> *mut js_Function {
    newfun(
        J,
        (*prog).line,
        (*prog).a,
        (*prog).b,
        (*prog).c,
        0,
        (*J).default_strict,
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsC_compilescript(
    J: *mut js_State,
    prog: *mut js_Ast,
    default_strict: c_int,
) -> *mut js_Function {
    newfun(
        J,
        if !prog.is_null() { (*prog).line } else { 0 },
        null_mut(),
        null_mut(),
        prog,
        1,
        default_strict,
        0,
    )
}
