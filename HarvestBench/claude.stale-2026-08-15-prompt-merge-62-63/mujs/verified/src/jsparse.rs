//! Translated from jsparse.c — the parser + constant folding.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::{js_free, js_malloc};
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// jsP_error / jsP_warning variadics: rs_ impls called by shim.
#[no_mangle]
pub unsafe extern "C-unwind" fn rs_jsP_error(J: *mut js_State, msg: *const c_char) {
    let mut buf: [c_char; 512] = [0; 512];
    libc::snprintf(buf.as_mut_ptr(), 256, cstr!("%s:%d: "), (*J).filename, (*J).lexline);
    strcat(buf.as_mut_ptr(), msg);
    crate::jserror::js_newsyntaxerror(J, buf.as_ptr());
    crate::jsrun::js_throw(J);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn rs_jsP_warning(J: *mut js_State, msg: *const c_char) {
    let mut buf: [c_char; 512] = [0; 512];
    libc::snprintf(buf.as_mut_ptr(), 512, cstr!("%s:%d: warning: %s"), (*J).filename, (*J).lexline, msg);
    crate::jsstate::js_report(J, buf.as_ptr());
}

macro_rules! jsP_error {
    ($J:expr, $($arg:tt)*) => {
        crate::jserror::jsP_error_shim($J, $($arg)*)
    };
}
macro_rules! jsP_warning {
    ($J:expr, $($arg:tt)*) => {
        crate::jserror::jsP_warning_shim($J, $($arg)*)
    };
}

/* Recursion guards */
macro_rules! INCREC {
    ($J:expr) => {{
        (*$J).astdepth += 1;
        if (*$J).astdepth > JS_ASTLIMIT {
            jsP_error!($J, cstr!("too much recursion"));
        }
    }};
}
macro_rules! DECREC {
    ($J:expr) => {
        (*$J).astdepth -= 1
    };
}

unsafe fn jsP_newnode(J: *mut js_State, type_: c_int, line: c_int, a: *mut js_Ast, b: *mut js_Ast, c: *mut js_Ast, d: *mut js_Ast) -> *mut js_Ast {
    let node = js_malloc(J, std::mem::size_of::<js_Ast>() as c_int) as *mut js_Ast;

    (*node).type_ = type_;
    (*node).line = line;
    (*node).a = a;
    (*node).b = b;
    (*node).c = c;
    (*node).d = d;
    (*node).number = 0.0;
    (*node).string = std::ptr::null();
    (*node).jumps = std::ptr::null_mut();
    (*node).casejump = 0;

    (*node).parent = std::ptr::null_mut();
    if !a.is_null() {
        (*a).parent = node;
    }
    if !b.is_null() {
        (*b).parent = node;
    }
    if !c.is_null() {
        (*c).parent = node;
    }
    if !d.is_null() {
        (*d).parent = node;
    }

    (*node).gcnext = (*J).gcast;
    (*J).gcast = node;

    node
}

macro_rules! LIST {
    ($J:expr, $h:expr) => {
        jsP_newnode($J, AST_LIST, 0, $h, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    };
}
macro_rules! EXP0 { ($J:expr,$line:expr,$x:expr) => { jsP_newnode($J, $x, $line, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! EXP1 { ($J:expr,$line:expr,$x:expr,$a:expr) => { jsP_newnode($J, $x, $line, $a, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! EXP2 { ($J:expr,$line:expr,$x:expr,$a:expr,$b:expr) => { jsP_newnode($J, $x, $line, $a, $b, std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! EXP3 { ($J:expr,$line:expr,$x:expr,$a:expr,$b:expr,$c:expr) => { jsP_newnode($J, $x, $line, $a, $b, $c, std::ptr::null_mut()) }; }
macro_rules! STM0 { ($J:expr,$line:expr,$x:expr) => { jsP_newnode($J, $x, $line, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! STM1 { ($J:expr,$line:expr,$x:expr,$a:expr) => { jsP_newnode($J, $x, $line, $a, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! STM2 { ($J:expr,$line:expr,$x:expr,$a:expr,$b:expr) => { jsP_newnode($J, $x, $line, $a, $b, std::ptr::null_mut(), std::ptr::null_mut()) }; }
macro_rules! STM3 { ($J:expr,$line:expr,$x:expr,$a:expr,$b:expr,$c:expr) => { jsP_newnode($J, $x, $line, $a, $b, $c, std::ptr::null_mut()) }; }
macro_rules! STM4 { ($J:expr,$line:expr,$x:expr,$a:expr,$b:expr,$c:expr,$d:expr) => { jsP_newnode($J, $x, $line, $a, $b, $c, $d) }; }

unsafe fn jsP_list(head: *mut js_Ast) -> *mut js_Ast {
    let mut prev = head;
    let mut node = (*head).b;
    while !node.is_null() {
        (*node).parent = prev;
        prev = node;
        node = (*node).b;
    }
    head
}

unsafe fn jsP_newstrnode(J: *mut js_State, type_: c_int, s: *const c_char) -> *mut js_Ast {
    let node = jsP_newnode(J, type_, (*J).lexline, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
    (*node).string = crate::jsintern::js_intern(J, s);
    node
}

unsafe fn jsP_newnumnode(J: *mut js_State, type_: c_int, n: f64) -> *mut js_Ast {
    let node = jsP_newnode(J, type_, (*J).lexline, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
    (*node).number = n;
    node
}

unsafe fn jsP_freejumps(J: *mut js_State, mut node: *mut js_JumpList) {
    while !node.is_null() {
        let next = (*node).next;
        js_free(J, node as *mut c_void);
        node = next;
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsP_freeparse(J: *mut js_State) {
    let mut node = (*J).gcast;
    while !node.is_null() {
        let next = (*node).gcnext;
        jsP_freejumps(J, (*node).jumps);
        js_free(J, node as *mut c_void);
        node = next;
    }
    (*J).gcast = std::ptr::null_mut();
}

/* Lookahead */
unsafe fn jsP_next(J: *mut js_State) {
    (*J).lookahead = crate::jslex::jsY_lex(J);
}

macro_rules! jsP_accept {
    ($J:expr, $x:expr) => {
        if (*$J).lookahead == $x {
            jsP_next($J);
            1
        } else {
            0
        }
    };
}

macro_rules! jsP_expect {
    ($J:expr, $x:expr) => {
        if jsP_accept!($J, $x) == 0 {
            jsP_error!(
                $J,
                cstr!("unexpected token: %s (expected %s)"),
                crate::jslex::jsY_tokenstring((*$J).lookahead),
                crate::jslex::jsY_tokenstring($x)
            );
        }
    };
}

unsafe fn semicolon(J: *mut js_State) {
    if (*J).lookahead == ';' as c_int {
        jsP_next(J);
        return;
    }
    if (*J).newline != 0 || (*J).lookahead == '}' as c_int || (*J).lookahead == 0 {
        return;
    }
    jsP_error!(J, cstr!("unexpected token: %s (expected ';')"), crate::jslex::jsY_tokenstring((*J).lookahead));
}

/* Literals */
unsafe fn identifier(J: *mut js_State) -> *mut js_Ast {
    let a;
    if (*J).lookahead == TK_IDENTIFIER {
        a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error!(J, cstr!("unexpected token: %s (expected identifier)"), crate::jslex::jsY_tokenstring((*J).lookahead));
    unreachable!()
}

unsafe fn identifieropt(J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER {
        return identifier(J);
    }
    std::ptr::null_mut()
}

unsafe fn identifiername(J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER || (*J).lookahead >= TK_BREAK {
        let a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error!(J, cstr!("unexpected token: %s (expected identifier or keyword)"), crate::jslex::jsY_tokenstring((*J).lookahead));
    unreachable!()
}

unsafe fn arrayelement(J: *mut js_State) -> *mut js_Ast {
    let line = (*J).lexline;
    if (*J).lookahead == ',' as c_int {
        return EXP0!(J, line, EXP_ELISION);
    }
    assignment(J, 0)
}

unsafe fn arrayliteral(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == ']' as c_int {
        return std::ptr::null_mut();
    }
    head = LIST!(J, arrayelement(J));
    tail = head;
    while jsP_accept!(J, ',' as c_int) != 0 {
        if (*J).lookahead != ']' as c_int {
            (*tail).b = LIST!(J, arrayelement(J));
            tail = (*tail).b;
        }
    }
    jsP_list(head)
}

unsafe fn propname(J: *mut js_State) -> *mut js_Ast {
    let name;
    if (*J).lookahead == TK_NUMBER {
        name = jsP_newnumnode(J, EXP_NUMBER, (*J).number);
        jsP_next(J);
    } else if (*J).lookahead == TK_STRING {
        name = jsP_newstrnode(J, EXP_STRING, (*J).text);
        jsP_next(J);
    } else {
        name = identifiername(J);
    }
    name
}

unsafe fn propassign(J: *mut js_State) -> *mut js_Ast {
    let mut name;
    let value;
    let arg;
    let body;
    let line = (*J).lexline;

    name = propname(J);

    if (*J).lookahead != ':' as c_int && (*name).type_ == AST_IDENTIFIER {
        if strcmp((*name).string, cstr!("get")) == 0 {
            name = propname(J);
            jsP_expect!(J, '(' as c_int);
            jsP_expect!(J, ')' as c_int);
            body = funbody(J);
            return EXP3!(J, line, EXP_PROP_GET, name, std::ptr::null_mut(), body);
        }
        if strcmp((*name).string, cstr!("set")) == 0 {
            name = propname(J);
            jsP_expect!(J, '(' as c_int);
            arg = identifier(J);
            jsP_expect!(J, ')' as c_int);
            body = funbody(J);
            return EXP3!(J, line, EXP_PROP_SET, name, LIST!(J, arg), body);
        }
    }

    jsP_expect!(J, ':' as c_int);
    value = assignment(J, 0);
    EXP2!(J, line, EXP_PROP_VAL, name, value)
}

unsafe fn objectliteral(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == '}' as c_int {
        return std::ptr::null_mut();
    }
    head = LIST!(J, propassign(J));
    tail = head;
    while jsP_accept!(J, ',' as c_int) != 0 {
        if (*J).lookahead == '}' as c_int {
            break;
        }
        (*tail).b = LIST!(J, propassign(J));
        tail = (*tail).b;
    }
    jsP_list(head)
}

/* Functions */
unsafe fn parameters(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == ')' as c_int {
        return std::ptr::null_mut();
    }
    head = LIST!(J, identifier(J));
    tail = head;
    while jsP_accept!(J, ',' as c_int) != 0 {
        (*tail).b = LIST!(J, identifier(J));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn fundec(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a;
    let b;
    let c;
    a = identifier(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    jsP_newnode(J, AST_FUNDEC, line, a, b, c, std::ptr::null_mut())
}

unsafe fn funstm(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a;
    let b;
    let c;
    a = identifier(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    // rewrite: var X = function X() {}
    STM1!(J, line, STM_VAR, LIST!(J, EXP2!(J, line, EXP_VAR, a, EXP3!(J, line, EXP_FUN, a, b, c))))
}

unsafe fn funexp(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a;
    let b;
    let c;
    a = identifieropt(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    EXP3!(J, line, EXP_FUN, a, b, c)
}

/* Expressions */
unsafe fn primary(J: *mut js_State) -> *mut js_Ast {
    let a;
    let line = (*J).lexline;

    if (*J).lookahead == TK_IDENTIFIER {
        a = jsP_newstrnode(J, EXP_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_STRING {
        a = jsP_newstrnode(J, EXP_STRING, (*J).text);
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_REGEXP {
        a = jsP_newstrnode(J, EXP_REGEXP, (*J).text);
        (*a).number = (*J).number;
        jsP_next(J);
        return a;
    }
    if (*J).lookahead == TK_NUMBER {
        a = jsP_newnumnode(J, EXP_NUMBER, (*J).number);
        jsP_next(J);
        return a;
    }

    if jsP_accept!(J, TK_THIS) != 0 {
        return EXP0!(J, line, EXP_THIS);
    }
    if jsP_accept!(J, TK_NULL) != 0 {
        return EXP0!(J, line, EXP_NULL);
    }
    if jsP_accept!(J, TK_TRUE) != 0 {
        return EXP0!(J, line, EXP_TRUE);
    }
    if jsP_accept!(J, TK_FALSE) != 0 {
        return EXP0!(J, line, EXP_FALSE);
    }
    if jsP_accept!(J, '{' as c_int) != 0 {
        let a = EXP1!(J, line, EXP_OBJECT, objectliteral(J));
        jsP_expect!(J, '}' as c_int);
        return a;
    }
    if jsP_accept!(J, '[' as c_int) != 0 {
        let a = EXP1!(J, line, EXP_ARRAY, arrayliteral(J));
        jsP_expect!(J, ']' as c_int);
        return a;
    }
    if jsP_accept!(J, '(' as c_int) != 0 {
        let a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        return a;
    }

    jsP_error!(J, cstr!("unexpected token in expression: %s"), crate::jslex::jsY_tokenstring((*J).lookahead));
    unreachable!()
}

unsafe fn arguments(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == ')' as c_int {
        return std::ptr::null_mut();
    }
    head = LIST!(J, assignment(J, 0));
    tail = head;
    while jsP_accept!(J, ',' as c_int) != 0 {
        (*tail).b = LIST!(J, assignment(J, 0));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn newexp(J: *mut js_State) -> *mut js_Ast {
    let a;
    let b;
    let line = (*J).lexline;

    if jsP_accept!(J, TK_NEW) != 0 {
        a = memberexp(J);
        if jsP_accept!(J, '(' as c_int) != 0 {
            b = arguments(J);
            jsP_expect!(J, ')' as c_int);
            return EXP2!(J, line, EXP_NEW, a, b);
        }
        return EXP1!(J, line, EXP_NEW, a);
    }

    if jsP_accept!(J, TK_FUNCTION) != 0 {
        return funexp(J, line);
    }

    primary(J)
}

unsafe fn memberexp(J: *mut js_State) -> *mut js_Ast {
    let mut a = newexp(J);
    let mut line;
    let SAVE = (*J).astdepth;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, '.' as c_int) != 0 {
            a = EXP2!(J, line, EXP_MEMBER, a, identifiername(J));
            continue;
        }
        if jsP_accept!(J, '[' as c_int) != 0 {
            a = EXP2!(J, line, EXP_INDEX, a, expression(J, 0));
            jsP_expect!(J, ']' as c_int);
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn callexp(J: *mut js_State) -> *mut js_Ast {
    let mut a = newexp(J);
    let mut line;
    let SAVE = (*J).astdepth;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, '.' as c_int) != 0 {
            a = EXP2!(J, line, EXP_MEMBER, a, identifiername(J));
            continue;
        }
        if jsP_accept!(J, '[' as c_int) != 0 {
            a = EXP2!(J, line, EXP_INDEX, a, expression(J, 0));
            jsP_expect!(J, ']' as c_int);
            continue;
        }
        if jsP_accept!(J, '(' as c_int) != 0 {
            a = EXP2!(J, line, EXP_CALL, a, arguments(J));
            jsP_expect!(J, ')' as c_int);
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn postfix(J: *mut js_State) -> *mut js_Ast {
    let a = callexp(J);
    let line = (*J).lexline;
    if (*J).newline == 0 && jsP_accept!(J, TK_INC) != 0 {
        return EXP1!(J, line, EXP_POSTINC, a);
    }
    if (*J).newline == 0 && jsP_accept!(J, TK_DEC) != 0 {
        return EXP1!(J, line, EXP_POSTDEC, a);
    }
    a
}

unsafe fn unary(J: *mut js_State) -> *mut js_Ast {
    let a;
    let line = (*J).lexline;
    INCREC!(J);
    if jsP_accept!(J, TK_DELETE) != 0 {
        a = EXP1!(J, line, EXP_DELETE, unary(J));
    } else if jsP_accept!(J, TK_VOID) != 0 {
        a = EXP1!(J, line, EXP_VOID, unary(J));
    } else if jsP_accept!(J, TK_TYPEOF) != 0 {
        a = EXP1!(J, line, EXP_TYPEOF, unary(J));
    } else if jsP_accept!(J, TK_INC) != 0 {
        a = EXP1!(J, line, EXP_PREINC, unary(J));
    } else if jsP_accept!(J, TK_DEC) != 0 {
        a = EXP1!(J, line, EXP_PREDEC, unary(J));
    } else if jsP_accept!(J, '+' as c_int) != 0 {
        a = EXP1!(J, line, EXP_POS, unary(J));
    } else if jsP_accept!(J, '-' as c_int) != 0 {
        a = EXP1!(J, line, EXP_NEG, unary(J));
    } else if jsP_accept!(J, '~' as c_int) != 0 {
        a = EXP1!(J, line, EXP_BITNOT, unary(J));
    } else if jsP_accept!(J, '!' as c_int) != 0 {
        a = EXP1!(J, line, EXP_LOGNOT, unary(J));
    } else {
        a = postfix(J);
    }
    DECREC!(J);
    a
}

macro_rules! binop_loop {
    ($J:expr, $sub:ident, $notin:expr, $($tok:expr => $exp:expr),+ $(,)?) => {{
        let mut a = $sub($J, $notin);
        let SAVE = (*$J).astdepth;
        let mut line;
        loop {
            INCREC!($J);
            line = (*$J).lexline;
            let mut matched = false;
            $(
                if !matched && jsP_accept!($J, $tok) != 0 {
                    a = EXP2!($J, line, $exp, a, $sub($J, $notin));
                    matched = true;
                }
            )+
            if !matched { break; }
        }
        (*$J).astdepth = SAVE;
        a
    }};
}

unsafe fn multiplicative(J: *mut js_State) -> *mut js_Ast {
    let mut a = unary(J);
    let SAVE = (*J).astdepth;
    let mut line;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, '*' as c_int) != 0 {
            a = EXP2!(J, line, EXP_MUL, a, unary(J));
            continue;
        }
        if jsP_accept!(J, '/' as c_int) != 0 {
            a = EXP2!(J, line, EXP_DIV, a, unary(J));
            continue;
        }
        if jsP_accept!(J, '%' as c_int) != 0 {
            a = EXP2!(J, line, EXP_MOD, a, unary(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn additive(J: *mut js_State) -> *mut js_Ast {
    let mut a = multiplicative(J);
    let SAVE = (*J).astdepth;
    let mut line;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, '+' as c_int) != 0 {
            a = EXP2!(J, line, EXP_ADD, a, multiplicative(J));
            continue;
        }
        if jsP_accept!(J, '-' as c_int) != 0 {
            a = EXP2!(J, line, EXP_SUB, a, multiplicative(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn shift(J: *mut js_State) -> *mut js_Ast {
    let mut a = additive(J);
    let SAVE = (*J).astdepth;
    let mut line;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, TK_SHL) != 0 {
            a = EXP2!(J, line, EXP_SHL, a, additive(J));
            continue;
        }
        if jsP_accept!(J, TK_SHR) != 0 {
            a = EXP2!(J, line, EXP_SHR, a, additive(J));
            continue;
        }
        if jsP_accept!(J, TK_USHR) != 0 {
            a = EXP2!(J, line, EXP_USHR, a, additive(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn relational(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = shift(J);
    let SAVE = (*J).astdepth;
    let mut line;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, '<' as c_int) != 0 {
            a = EXP2!(J, line, EXP_LT, a, shift(J));
            continue;
        }
        if jsP_accept!(J, '>' as c_int) != 0 {
            a = EXP2!(J, line, EXP_GT, a, shift(J));
            continue;
        }
        if jsP_accept!(J, TK_LE) != 0 {
            a = EXP2!(J, line, EXP_LE, a, shift(J));
            continue;
        }
        if jsP_accept!(J, TK_GE) != 0 {
            a = EXP2!(J, line, EXP_GE, a, shift(J));
            continue;
        }
        if jsP_accept!(J, TK_INSTANCEOF) != 0 {
            a = EXP2!(J, line, EXP_INSTANCEOF, a, shift(J));
            continue;
        }
        if notin == 0 && jsP_accept!(J, TK_IN) != 0 {
            a = EXP2!(J, line, EXP_IN, a, shift(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn equality(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = relational(J, notin);
    let SAVE = (*J).astdepth;
    let mut line;
    loop {
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept!(J, TK_EQ) != 0 {
            a = EXP2!(J, line, EXP_EQ, a, relational(J, notin));
            continue;
        }
        if jsP_accept!(J, TK_NE) != 0 {
            a = EXP2!(J, line, EXP_NE, a, relational(J, notin));
            continue;
        }
        if jsP_accept!(J, TK_STRICTEQ) != 0 {
            a = EXP2!(J, line, EXP_STRICTEQ, a, relational(J, notin));
            continue;
        }
        if jsP_accept!(J, TK_STRICTNE) != 0 {
            a = EXP2!(J, line, EXP_STRICTNE, a, relational(J, notin));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn bitand(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = equality(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept!(J, '&' as c_int) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_BITAND, a, equality(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn bitxor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitand(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept!(J, '^' as c_int) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_BITXOR, a, bitand(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn bitor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitxor(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept!(J, '|' as c_int) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_BITOR, a, bitxor(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn logand(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitor(J, notin);
    let line = (*J).lexline;
    if jsP_accept!(J, TK_AND) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_LOGAND, a, logand(J, notin));
        DECREC!(J);
    }
    a
}

unsafe fn logor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = logand(J, notin);
    let line = (*J).lexline;
    if jsP_accept!(J, TK_OR) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_LOGOR, a, logor(J, notin));
        DECREC!(J);
    }
    a
}

unsafe fn conditional(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let a = logor(J, notin);
    let line = (*J).lexline;
    if jsP_accept!(J, '?' as c_int) != 0 {
        let b;
        let c;
        INCREC!(J);
        b = assignment(J, 0);
        jsP_expect!(J, ':' as c_int);
        c = assignment(J, notin);
        DECREC!(J);
        return EXP3!(J, line, EXP_COND, a, b, c);
    }
    a
}

unsafe fn assignment(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = conditional(J, notin);
    let line = (*J).lexline;
    INCREC!(J);
    if jsP_accept!(J, '=' as c_int) != 0 {
        a = EXP2!(J, line, EXP_ASS, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_MUL_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_MUL, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_DIV_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_DIV, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_MOD_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_MOD, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_ADD_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_ADD, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_SUB_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_SUB, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_SHL_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_SHL, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_SHR_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_SHR, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_USHR_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_USHR, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_AND_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_BITAND, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_XOR_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_BITXOR, a, assignment(J, notin));
    } else if jsP_accept!(J, TK_OR_ASS) != 0 {
        a = EXP2!(J, line, EXP_ASS_BITOR, a, assignment(J, notin));
    }
    DECREC!(J);
    a
}

unsafe fn expression(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = assignment(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    let _ = line;
    while jsP_accept!(J, ',' as c_int) != 0 {
        INCREC!(J);
        a = EXP2!(J, line, EXP_COMMA, a, assignment(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

/* Statements */
unsafe fn vardec(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let a = identifier(J);
    let line = (*J).lexline;
    if jsP_accept!(J, '=' as c_int) != 0 {
        return EXP2!(J, line, EXP_VAR, a, assignment(J, notin));
    }
    EXP1!(J, line, EXP_VAR, a)
}

unsafe fn vardeclist(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let head;
    let mut tail;
    head = LIST!(J, vardec(J, notin));
    tail = head;
    while jsP_accept!(J, ',' as c_int) != 0 {
        (*tail).b = LIST!(J, vardec(J, notin));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn statementlist(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == '}' as c_int || (*J).lookahead == TK_CASE || (*J).lookahead == TK_DEFAULT {
        return std::ptr::null_mut();
    }
    head = LIST!(J, statement(J));
    tail = head;
    while (*J).lookahead != '}' as c_int && (*J).lookahead != TK_CASE && (*J).lookahead != TK_DEFAULT {
        (*tail).b = LIST!(J, statement(J));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn caseclause(J: *mut js_State) -> *mut js_Ast {
    let a;
    let b;
    let line = (*J).lexline;

    if jsP_accept!(J, TK_CASE) != 0 {
        a = expression(J, 0);
        jsP_expect!(J, ':' as c_int);
        b = statementlist(J);
        return STM2!(J, line, STM_CASE, a, b);
    }

    if jsP_accept!(J, TK_DEFAULT) != 0 {
        jsP_expect!(J, ':' as c_int);
        a = statementlist(J);
        return STM1!(J, line, STM_DEFAULT, a);
    }

    jsP_error!(J, cstr!("unexpected token in switch: %s (expected 'case' or 'default')"), crate::jslex::jsY_tokenstring((*J).lookahead));
    unreachable!()
}

unsafe fn caselist(J: *mut js_State) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == '}' as c_int {
        return std::ptr::null_mut();
    }
    head = LIST!(J, caseclause(J));
    tail = head;
    while (*J).lookahead != '}' as c_int {
        (*tail).b = LIST!(J, caseclause(J));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn block(J: *mut js_State) -> *mut js_Ast {
    let a;
    let line = (*J).lexline;
    jsP_expect!(J, '{' as c_int);
    a = statementlist(J);
    jsP_expect!(J, '}' as c_int);
    STM1!(J, line, STM_BLOCK, a)
}

unsafe fn forexpression(J: *mut js_State, end: c_int) -> *mut js_Ast {
    let mut a: *mut js_Ast = std::ptr::null_mut();
    if (*J).lookahead != end {
        a = expression(J, 0);
    }
    jsP_expect!(J, end);
    a
}

unsafe fn forstatement(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let mut a;
    let b;
    let c;
    let d: *mut js_Ast;
    jsP_expect!(J, '(' as c_int);
    if jsP_accept!(J, TK_VAR) != 0 {
        a = vardeclist(J, 1);
        if jsP_accept!(J, ';' as c_int) != 0 {
            let b = forexpression(J, ';' as c_int);
            let c = forexpression(J, ')' as c_int);
            let d = statement(J);
            return STM4!(J, line, STM_FOR_VAR, a, b, c, d);
        }
        if jsP_accept!(J, TK_IN) != 0 {
            let b = expression(J, 0);
            jsP_expect!(J, ')' as c_int);
            let c = statement(J);
            return STM3!(J, line, STM_FOR_IN_VAR, a, b, c);
        }
        jsP_error!(J, cstr!("unexpected token in for-var-statement: %s"), crate::jslex::jsY_tokenstring((*J).lookahead));
    }

    if (*J).lookahead != ';' as c_int {
        a = expression(J, 1);
    } else {
        a = std::ptr::null_mut();
    }
    if jsP_accept!(J, ';' as c_int) != 0 {
        b = forexpression(J, ';' as c_int);
        c = forexpression(J, ')' as c_int);
        d = statement(J);
        return STM4!(J, line, STM_FOR, a, b, c, d);
    }
    if jsP_accept!(J, TK_IN) != 0 {
        let b = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        let c = statement(J);
        return STM3!(J, line, STM_FOR_IN, a, b, c);
    }
    jsP_error!(J, cstr!("unexpected token in for-statement: %s"), crate::jslex::jsY_tokenstring((*J).lookahead));
    unreachable!()
}

unsafe fn statement(J: *mut js_State) -> *mut js_Ast {
    let a;
    let b;
    let c;
    #[allow(unused_variables)]
    let d: *mut js_Ast;
    let stm;
    let line = (*J).lexline;

    INCREC!(J);

    if (*J).lookahead == '{' as c_int {
        stm = block(J);
    } else if jsP_accept!(J, TK_VAR) != 0 {
        let a = vardeclist(J, 0);
        semicolon(J);
        stm = STM1!(J, line, STM_VAR, a);
    } else if jsP_accept!(J, ';' as c_int) != 0 {
        stm = STM0!(J, line, STM_EMPTY);
    } else if jsP_accept!(J, TK_IF) != 0 {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        if jsP_accept!(J, TK_ELSE) != 0 {
            c = statement(J);
        } else {
            c = std::ptr::null_mut();
        }
        stm = STM3!(J, line, STM_IF, a, b, c);
    } else if jsP_accept!(J, TK_DO) != 0 {
        a = statement(J);
        jsP_expect!(J, TK_WHILE);
        jsP_expect!(J, '(' as c_int);
        b = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        semicolon(J);
        stm = STM2!(J, line, STM_DO, a, b);
    } else if jsP_accept!(J, TK_WHILE) != 0 {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        stm = STM2!(J, line, STM_WHILE, a, b);
    } else if jsP_accept!(J, TK_FOR) != 0 {
        stm = forstatement(J, line);
    } else if jsP_accept!(J, TK_CONTINUE) != 0 {
        a = identifieropt(J);
        semicolon(J);
        stm = STM1!(J, line, STM_CONTINUE, a);
    } else if jsP_accept!(J, TK_BREAK) != 0 {
        a = identifieropt(J);
        semicolon(J);
        stm = STM1!(J, line, STM_BREAK, a);
    } else if jsP_accept!(J, TK_RETURN) != 0 {
        if (*J).lookahead != ';' as c_int && (*J).lookahead != '}' as c_int && (*J).lookahead != 0 {
            a = expression(J, 0);
        } else {
            a = std::ptr::null_mut();
        }
        semicolon(J);
        stm = STM1!(J, line, STM_RETURN, a);
    } else if jsP_accept!(J, TK_WITH) != 0 {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        stm = STM2!(J, line, STM_WITH, a, b);
    } else if jsP_accept!(J, TK_SWITCH) != 0 {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        jsP_expect!(J, '{' as c_int);
        b = caselist(J);
        jsP_expect!(J, '}' as c_int);
        stm = STM2!(J, line, STM_SWITCH, a, b);
    } else if jsP_accept!(J, TK_THROW) != 0 {
        a = expression(J, 0);
        semicolon(J);
        stm = STM1!(J, line, STM_THROW, a);
    } else if jsP_accept!(J, TK_TRY) != 0 {
        let mut bb: *mut js_Ast = std::ptr::null_mut();
        let mut cc: *mut js_Ast = std::ptr::null_mut();
        let mut dd: *mut js_Ast = std::ptr::null_mut();
        let aa = block(J);
        if jsP_accept!(J, TK_CATCH) != 0 {
            jsP_expect!(J, '(' as c_int);
            bb = identifier(J);
            jsP_expect!(J, ')' as c_int);
            cc = block(J);
        }
        if jsP_accept!(J, TK_FINALLY) != 0 {
            dd = block(J);
        }
        if bb.is_null() && dd.is_null() {
            jsP_error!(J, cstr!("unexpected token in try: %s (expected 'catch' or 'finally')"), crate::jslex::jsY_tokenstring((*J).lookahead));
        }
        stm = STM4!(J, line, STM_TRY, aa, bb, cc, dd);
    } else if jsP_accept!(J, TK_DEBUGGER) != 0 {
        semicolon(J);
        stm = STM0!(J, line, STM_DEBUGGER);
    } else if jsP_accept!(J, TK_FUNCTION) != 0 {
        jsP_warning!(J, cstr!("function statements are not standard"));
        stm = funstm(J, line);
    } else if (*J).lookahead == TK_IDENTIFIER {
        let aa = expression(J, 0);
        if (*aa).type_ == EXP_IDENTIFIER && jsP_accept!(J, ':' as c_int) != 0 {
            (*aa).type_ = AST_IDENTIFIER;
            let bb = statement(J);
            stm = STM2!(J, line, STM_LABEL, aa, bb);
        } else {
            semicolon(J);
            stm = aa;
        }
    } else {
        stm = expression(J, 0);
        semicolon(J);
    }

    DECREC!(J);
    stm
}

/* Program */
unsafe fn scriptelement(J: *mut js_State) -> *mut js_Ast {
    let line = (*J).lexline;
    if jsP_accept!(J, TK_FUNCTION) != 0 {
        return fundec(J, line);
    }
    statement(J)
}

unsafe fn script(J: *mut js_State, terminator: c_int) -> *mut js_Ast {
    let head;
    let mut tail;
    if (*J).lookahead == terminator {
        return std::ptr::null_mut();
    }
    head = LIST!(J, scriptelement(J));
    tail = head;
    while (*J).lookahead != terminator {
        (*tail).b = LIST!(J, scriptelement(J));
        tail = (*tail).b;
    }
    jsP_list(head)
}

unsafe fn funbody(J: *mut js_State) -> *mut js_Ast {
    let a;
    jsP_expect!(J, '{' as c_int);
    a = script(J, '}' as c_int);
    jsP_expect!(J, '}' as c_int);
    a
}

/* Constant folding */
unsafe fn toint32(d: f64) -> c_int {
    let two32 = 4294967296.0;
    let two31 = 2147483648.0;

    if !d.is_finite() || d == 0.0 {
        return 0;
    }

    let mut d = d % two32;
    d = if d >= 0.0 { d.floor() } else { d.ceil() + two32 };
    // jsparse.c:968/970 `if (d >= two31) return d - two32; else return d;` — implicit
    // double -> `int` (return type `int`), but provably in range: non-finite d returned above,
    // and after the fmod/floor/ceil normalisation d is an integer in [0, 2^32], so the taken
    // branch yields [-2^31, 0) resp. [0, 2^31). Plain `as` is identical to C.
    if d >= two31 {
        (d - two32) as c_int
    } else {
        d as c_int
    }
}

unsafe fn touint32(d: f64) -> c_uint {
    toint32(d) as c_uint
}
use std::os::raw::c_uint;

unsafe fn jsP_setnumnode(node: *mut js_Ast, x: f64) -> c_int {
    (*node).type_ = EXP_NUMBER;
    (*node).number = x;
    (*node).a = std::ptr::null_mut();
    (*node).b = std::ptr::null_mut();
    (*node).c = std::ptr::null_mut();
    (*node).d = std::ptr::null_mut();
    1
}

unsafe fn jsP_foldconst(node: *mut js_Ast) -> c_int {
    let x;
    let y;
    let a;
    let b;

    if (*node).type_ == AST_LIST {
        let mut n = node;
        while !n.is_null() {
            jsP_foldconst((*n).a);
            n = (*n).b;
        }
        return 0;
    }

    if (*node).type_ == EXP_NUMBER {
        return 1;
    }

    a = if !(*node).a.is_null() { jsP_foldconst((*node).a) } else { 0 };
    b = if !(*node).b.is_null() { jsP_foldconst((*node).b) } else { 0 };
    if !(*node).c.is_null() {
        jsP_foldconst((*node).c);
    }
    if !(*node).d.is_null() {
        jsP_foldconst((*node).d);
    }

    if a != 0 {
        x = (*(*node).a).number;
        match (*node).type_ {
            t if t == EXP_NEG => return jsP_setnumnode(node, -x),
            t if t == EXP_POS => return jsP_setnumnode(node, x),
            t if t == EXP_BITNOT => return jsP_setnumnode(node, !toint32(x) as f64),
            _ => {}
        }

        if b != 0 {
            y = (*(*node).b).number;
            match (*node).type_ {
                t if t == EXP_MUL => return jsP_setnumnode(node, x * y),
                t if t == EXP_DIV => return jsP_setnumnode(node, x / y),
                t if t == EXP_MOD => return jsP_setnumnode(node, x % y),
                t if t == EXP_ADD => return jsP_setnumnode(node, x + y),
                t if t == EXP_SUB => return jsP_setnumnode(node, x - y),
                t if t == EXP_SHL => return jsP_setnumnode(node, (toint32(x).wrapping_shl(touint32(y) & 0x1F)) as f64),
                t if t == EXP_SHR => return jsP_setnumnode(node, (toint32(x) >> (touint32(y) & 0x1F)) as f64),
                t if t == EXP_USHR => return jsP_setnumnode(node, (touint32(x) >> (touint32(y) & 0x1F)) as f64),
                t if t == EXP_BITAND => return jsP_setnumnode(node, (toint32(x) & toint32(y)) as f64),
                t if t == EXP_BITXOR => return jsP_setnumnode(node, (toint32(x) ^ toint32(y)) as f64),
                t if t == EXP_BITOR => return jsP_setnumnode(node, (toint32(x) | toint32(y)) as f64),
                _ => {}
            }
        }
    }

    0
}

/* Main entry point */
#[no_mangle]
pub unsafe extern "C-unwind" fn jsP_parse(J: *mut js_State, filename: *const c_char, source: *const c_char) -> *mut js_Ast {
    let p;

    crate::jslex::jsY_initlex(J, filename, source);
    jsP_next(J);
    (*J).astdepth = 0;
    p = script(J, 0);
    if !p.is_null() {
        jsP_foldconst(p);
    }

    p
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsP_parsefunction(J: *mut js_State, filename: *const c_char, params: *const c_char, body: *const c_char) -> *mut js_Ast {
    let mut p: *mut js_Ast = std::ptr::null_mut();
    let line = 0;
    if !params.is_null() {
        crate::jslex::jsY_initlex(J, filename, params);
        jsP_next(J);
        (*J).astdepth = 0;
        p = parameters(J);
    }
    EXP3!(J, line, EXP_FUN, std::ptr::null_mut(), p, jsP_parse(J, filename, body))
}
