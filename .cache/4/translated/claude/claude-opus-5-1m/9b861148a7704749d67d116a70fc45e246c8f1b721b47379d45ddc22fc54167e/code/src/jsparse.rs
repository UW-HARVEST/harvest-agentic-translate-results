//! Translation of `c_src/src/jsparse.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsi::*;
use crate::jsintern::js_intern;
use crate::jslex::*;
use crate::jsrun::{js_free, js_malloc, js_throw};
use core::ptr::{null, null_mut};

/* #define LIST(h) jsP_newnode(J, AST_LIST, 0, h, 0, 0, 0) */
macro_rules! LIST {
    ($J:expr, $h:expr) => {
        jsP_newnode($J, AST_LIST, 0, $h, null_mut(), null_mut(), null_mut())
    };
}

/* #define EXP0(x) / STM0(x) ... -- the node type is spelled out at the use site */
macro_rules! NODE0 {
    ($J:expr, $t:expr, $line:expr) => {
        jsP_newnode($J, $t, $line, null_mut(), null_mut(), null_mut(), null_mut())
    };
}
macro_rules! NODE1 {
    ($J:expr, $t:expr, $line:expr, $a:expr) => {
        jsP_newnode($J, $t, $line, $a, null_mut(), null_mut(), null_mut())
    };
}
macro_rules! NODE2 {
    ($J:expr, $t:expr, $line:expr, $a:expr, $b:expr) => {
        jsP_newnode($J, $t, $line, $a, $b, null_mut(), null_mut())
    };
}
macro_rules! NODE3 {
    ($J:expr, $t:expr, $line:expr, $a:expr, $b:expr, $c:expr) => {
        jsP_newnode($J, $t, $line, $a, $b, $c, null_mut())
    };
}
macro_rules! NODE4 {
    ($J:expr, $t:expr, $line:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {
        jsP_newnode($J, $t, $line, $a, $b, $c, $d)
    };
}

/* JS_NORETURN static void jsP_error(js_State *J, const char *fmt, ...) */
macro_rules! jsP_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut msgbuf = [0 as c_char; 256];
        let mut buf = [0 as c_char; 512];
        snprintf(msgbuf.as_mut_ptr(), 256, $fmt $(, $a)*);
        snprintf(buf.as_mut_ptr(), 256, c"%s:%d: ".as_ptr(), (*$J).filename, (*$J).lexline);
        strcat(buf.as_mut_ptr(), msgbuf.as_ptr());
        crate::jserror::js_newsyntaxerror($J, buf.as_ptr());
        crate::jsrun::js_throw($J)
    }};
}

/* static void jsP_warning(js_State *J, const char *fmt, ...) */
macro_rules! jsP_warning {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut msg = [0 as c_char; 256];
        let mut buf = [0 as c_char; 512];
        snprintf(msg.as_mut_ptr(), 256, $fmt $(, $a)*);
        snprintf(buf.as_mut_ptr(), 512, c"%s:%d: warning: %s".as_ptr(), (*$J).filename, (*$J).lexline, msg.as_ptr());
        crate::jsstate::js_report($J, buf.as_ptr());
    }};
}

/* #define INCREC() if (++J->astdepth > JS_ASTLIMIT) jsP_error(J, "too much recursion") */
macro_rules! INCREC {
    ($J:expr) => {{
        (*$J).astdepth += 1;
        if (*$J).astdepth > JS_ASTLIMIT {
            jsP_error!($J, c"too much recursion".as_ptr());
        }
    }};
}

/* #define DECREC() --J->astdepth */
macro_rules! DECREC {
    ($J:expr) => {
        (*$J).astdepth -= 1
    };
}

unsafe fn jsP_newnode(
    J: *mut js_State,
    type_: c_int,
    line: c_int,
    a: *mut js_Ast,
    b: *mut js_Ast,
    c: *mut js_Ast,
    d: *mut js_Ast,
) -> *mut js_Ast {
    let node = js_malloc(J, core::mem::size_of::<js_Ast>() as c_int) as *mut js_Ast;

    (*node).type_ = type_;
    (*node).line = line;
    (*node).a = a;
    (*node).b = b;
    (*node).c = c;
    (*node).d = d;
    (*node).number = 0.0;
    (*node).string = null();
    (*node).jumps = null_mut();
    (*node).casejump = 0;

    (*node).parent = null_mut();
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

unsafe fn jsP_list(head: *mut js_Ast) -> *mut js_Ast {
    /* set parent pointers in list nodes */
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
    let node = jsP_newnode(
        J,
        type_,
        (*J).lexline,
        null_mut(),
        null_mut(),
        null_mut(),
        null_mut(),
    );
    (*node).string = js_intern(J, s);
    node
}

unsafe fn jsP_newnumnode(J: *mut js_State, type_: c_int, n: f64) -> *mut js_Ast {
    let node = jsP_newnode(
        J,
        type_,
        (*J).lexline,
        null_mut(),
        null_mut(),
        null_mut(),
        null_mut(),
    );
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_freeparse(J: *mut js_State) {
    let mut node = (*J).gcast;
    while !node.is_null() {
        let next = (*node).gcnext;
        jsP_freejumps(J, (*node).jumps);
        js_free(J, node as *mut c_void);
        node = next;
    }
    (*J).gcast = null_mut();
}

/* Lookahead */

unsafe fn jsP_next(J: *mut js_State) {
    (*J).lookahead = jsY_lex(J);
}

/* #define jsP_accept(J,x) (J->lookahead == x ? (jsP_next(J), 1) : 0) */
#[inline]
unsafe fn jsP_accept(J: *mut js_State, x: c_int) -> bool {
    if (*J).lookahead == x {
        jsP_next(J);
        true
    } else {
        false
    }
}

/* #define jsP_expect(J,x) if (!jsP_accept(J, x)) jsP_error(...) */
macro_rules! jsP_expect {
    ($J:expr, $x:expr) => {
        if !jsP_accept($J, $x) {
            jsP_error!(
                $J,
                c"unexpected token: %s (expected %s)".as_ptr(),
                jsY_tokenstring((*$J).lookahead),
                jsY_tokenstring($x)
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
    jsP_error!(
        J,
        c"unexpected token: %s (expected ';')".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

/* Literals */

unsafe fn identifier(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    if (*J).lookahead == TK_IDENTIFIER {
        a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error!(
        J,
        c"unexpected token: %s (expected identifier)".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

unsafe fn identifieropt(J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER {
        return identifier(J);
    }
    null_mut()
}

unsafe fn identifiername(J: *mut js_State) -> *mut js_Ast {
    if (*J).lookahead == TK_IDENTIFIER || (*J).lookahead >= TK_BREAK {
        let a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
        jsP_next(J);
        return a;
    }
    jsP_error!(
        J,
        c"unexpected token: %s (expected identifier or keyword)".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

unsafe fn arrayelement(J: *mut js_State) -> *mut js_Ast {
    let line = (*J).lexline;
    if (*J).lookahead == ',' as c_int {
        return NODE0!(J, EXP_ELISION, line);
    }
    assignment(J, 0)
}

unsafe fn arrayliteral(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == ']' as c_int {
        return null_mut();
    }
    head = LIST!(J, arrayelement(J));
    tail = head;
    while jsP_accept(J, ',' as c_int) {
        if (*J).lookahead != ']' as c_int {
            let n = LIST!(J, arrayelement(J));
            (*tail).b = n;
            tail = n;
        }
    }
    jsP_list(head)
}

unsafe fn propname(J: *mut js_State) -> *mut js_Ast {
    let name: *mut js_Ast;
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
    let mut name: *mut js_Ast;
    let value: *mut js_Ast;
    let arg: *mut js_Ast;
    let body: *mut js_Ast;
    let line = (*J).lexline;

    name = propname(J);

    if (*J).lookahead != ':' as c_int && (*name).type_ == AST_IDENTIFIER {
        if streq((*name).string, c"get".as_ptr()) {
            name = propname(J);
            jsP_expect!(J, '(' as c_int);
            jsP_expect!(J, ')' as c_int);
            body = funbody(J);
            return NODE3!(J, EXP_PROP_GET, line, name, null_mut(), body);
        }
        if streq((*name).string, c"set".as_ptr()) {
            name = propname(J);
            jsP_expect!(J, '(' as c_int);
            arg = identifier(J);
            jsP_expect!(J, ')' as c_int);
            body = funbody(J);
            return NODE3!(J, EXP_PROP_SET, line, name, LIST!(J, arg), body);
        }
    }

    jsP_expect!(J, ':' as c_int);
    value = assignment(J, 0);
    NODE2!(J, EXP_PROP_VAL, line, name, value)
}

unsafe fn objectliteral(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == '}' as c_int {
        return null_mut();
    }
    head = LIST!(J, propassign(J));
    tail = head;
    while jsP_accept(J, ',' as c_int) {
        if (*J).lookahead == '}' as c_int {
            break;
        }
        let n = LIST!(J, propassign(J));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

/* Functions */

unsafe fn parameters(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == ')' as c_int {
        return null_mut();
    }
    head = LIST!(J, identifier(J));
    tail = head;
    while jsP_accept(J, ',' as c_int) {
        let n = LIST!(J, identifier(J));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn fundec(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a: *mut js_Ast;
    let b: *mut js_Ast;
    let c: *mut js_Ast;
    a = identifier(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    jsP_newnode(J, AST_FUNDEC, line, a, b, c, null_mut())
}

unsafe fn funstm(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a: *mut js_Ast;
    let b: *mut js_Ast;
    let c: *mut js_Ast;
    a = identifier(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    /* rewrite function statement as "var X = function X() {}" */
    NODE1!(
        J,
        STM_VAR,
        line,
        LIST!(
            J,
            NODE2!(J, EXP_VAR, line, a, NODE3!(J, EXP_FUN, line, a, b, c))
        )
    )
}

unsafe fn funexp(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let a: *mut js_Ast;
    let b: *mut js_Ast;
    let c: *mut js_Ast;
    a = identifieropt(J);
    jsP_expect!(J, '(' as c_int);
    b = parameters(J);
    jsP_expect!(J, ')' as c_int);
    c = funbody(J);
    NODE3!(J, EXP_FUN, line, a, b, c)
}

/* Expressions */

unsafe fn primary(J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast;
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

    if jsP_accept(J, TK_THIS) {
        return NODE0!(J, EXP_THIS, line);
    }
    if jsP_accept(J, TK_NULL) {
        return NODE0!(J, EXP_NULL, line);
    }
    if jsP_accept(J, TK_TRUE) {
        return NODE0!(J, EXP_TRUE, line);
    }
    if jsP_accept(J, TK_FALSE) {
        return NODE0!(J, EXP_FALSE, line);
    }
    if jsP_accept(J, '{' as c_int) {
        a = NODE1!(J, EXP_OBJECT, line, objectliteral(J));
        jsP_expect!(J, '}' as c_int);
        return a;
    }
    if jsP_accept(J, '[' as c_int) {
        a = NODE1!(J, EXP_ARRAY, line, arrayliteral(J));
        jsP_expect!(J, ']' as c_int);
        return a;
    }
    if jsP_accept(J, '(' as c_int) {
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        return a;
    }

    jsP_error!(
        J,
        c"unexpected token in expression: %s".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

unsafe fn arguments(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == ')' as c_int {
        return null_mut();
    }
    head = LIST!(J, assignment(J, 0));
    tail = head;
    while jsP_accept(J, ',' as c_int) {
        let n = LIST!(J, assignment(J, 0));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn newexp(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    let b: *mut js_Ast;
    let line = (*J).lexline;

    if jsP_accept(J, TK_NEW) {
        a = memberexp(J);
        if jsP_accept(J, '(' as c_int) {
            b = arguments(J);
            jsP_expect!(J, ')' as c_int);
            return NODE2!(J, EXP_NEW, line, a, b);
        }
        return NODE1!(J, EXP_NEW, line, a);
    }

    if jsP_accept(J, TK_FUNCTION) {
        return funexp(J, line);
    }

    primary(J)
}

unsafe fn memberexp(J: *mut js_State) -> *mut js_Ast {
    let mut a = newexp(J);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, '.' as c_int) {
            a = NODE2!(J, EXP_MEMBER, line, a, identifiername(J));
            continue;
        }
        if jsP_accept(J, '[' as c_int) {
            a = NODE2!(J, EXP_INDEX, line, a, expression(J, 0));
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
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, '.' as c_int) {
            a = NODE2!(J, EXP_MEMBER, line, a, identifiername(J));
            continue;
        }
        if jsP_accept(J, '[' as c_int) {
            a = NODE2!(J, EXP_INDEX, line, a, expression(J, 0));
            jsP_expect!(J, ']' as c_int);
            continue;
        }
        if jsP_accept(J, '(' as c_int) {
            a = NODE2!(J, EXP_CALL, line, a, arguments(J));
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
    if (*J).newline == 0 && jsP_accept(J, TK_INC) {
        return NODE1!(J, EXP_POSTINC, line, a);
    }
    if (*J).newline == 0 && jsP_accept(J, TK_DEC) {
        return NODE1!(J, EXP_POSTDEC, line, a);
    }
    a
}

unsafe fn unary(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    let line = (*J).lexline;
    INCREC!(J);
    if jsP_accept(J, TK_DELETE) {
        a = NODE1!(J, EXP_DELETE, line, unary(J));
    } else if jsP_accept(J, TK_VOID) {
        a = NODE1!(J, EXP_VOID, line, unary(J));
    } else if jsP_accept(J, TK_TYPEOF) {
        a = NODE1!(J, EXP_TYPEOF, line, unary(J));
    } else if jsP_accept(J, TK_INC) {
        a = NODE1!(J, EXP_PREINC, line, unary(J));
    } else if jsP_accept(J, TK_DEC) {
        a = NODE1!(J, EXP_PREDEC, line, unary(J));
    } else if jsP_accept(J, '+' as c_int) {
        a = NODE1!(J, EXP_POS, line, unary(J));
    } else if jsP_accept(J, '-' as c_int) {
        a = NODE1!(J, EXP_NEG, line, unary(J));
    } else if jsP_accept(J, '~' as c_int) {
        a = NODE1!(J, EXP_BITNOT, line, unary(J));
    } else if jsP_accept(J, '!' as c_int) {
        a = NODE1!(J, EXP_LOGNOT, line, unary(J));
    } else {
        a = postfix(J);
    }
    DECREC!(J);
    a
}

unsafe fn multiplicative(J: *mut js_State) -> *mut js_Ast {
    let mut a = unary(J);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, '*' as c_int) {
            a = NODE2!(J, EXP_MUL, line, a, unary(J));
            continue;
        }
        if jsP_accept(J, '/' as c_int) {
            a = NODE2!(J, EXP_DIV, line, a, unary(J));
            continue;
        }
        if jsP_accept(J, '%' as c_int) {
            a = NODE2!(J, EXP_MOD, line, a, unary(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn additive(J: *mut js_State) -> *mut js_Ast {
    let mut a = multiplicative(J);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, '+' as c_int) {
            a = NODE2!(J, EXP_ADD, line, a, multiplicative(J));
            continue;
        }
        if jsP_accept(J, '-' as c_int) {
            a = NODE2!(J, EXP_SUB, line, a, multiplicative(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn shift(J: *mut js_State) -> *mut js_Ast {
    let mut a = additive(J);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, TK_SHL) {
            a = NODE2!(J, EXP_SHL, line, a, additive(J));
            continue;
        }
        if jsP_accept(J, TK_SHR) {
            a = NODE2!(J, EXP_SHR, line, a, additive(J));
            continue;
        }
        if jsP_accept(J, TK_USHR) {
            a = NODE2!(J, EXP_USHR, line, a, additive(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn relational(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = shift(J);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, '<' as c_int) {
            a = NODE2!(J, EXP_LT, line, a, shift(J));
            continue;
        }
        if jsP_accept(J, '>' as c_int) {
            a = NODE2!(J, EXP_GT, line, a, shift(J));
            continue;
        }
        if jsP_accept(J, TK_LE) {
            a = NODE2!(J, EXP_LE, line, a, shift(J));
            continue;
        }
        if jsP_accept(J, TK_GE) {
            a = NODE2!(J, EXP_GE, line, a, shift(J));
            continue;
        }
        if jsP_accept(J, TK_INSTANCEOF) {
            a = NODE2!(J, EXP_INSTANCEOF, line, a, shift(J));
            continue;
        }
        if notin == 0 && jsP_accept(J, TK_IN) {
            a = NODE2!(J, EXP_IN, line, a, shift(J));
            continue;
        }
        break;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn equality(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = relational(J, notin);
    let mut line: c_int;
    let SAVE = (*J).astdepth;
    loop {
        /* loop: */
        INCREC!(J);
        line = (*J).lexline;
        if jsP_accept(J, TK_EQ) {
            a = NODE2!(J, EXP_EQ, line, a, relational(J, notin));
            continue;
        }
        if jsP_accept(J, TK_NE) {
            a = NODE2!(J, EXP_NE, line, a, relational(J, notin));
            continue;
        }
        if jsP_accept(J, TK_STRICTEQ) {
            a = NODE2!(J, EXP_STRICTEQ, line, a, relational(J, notin));
            continue;
        }
        if jsP_accept(J, TK_STRICTNE) {
            a = NODE2!(J, EXP_STRICTNE, line, a, relational(J, notin));
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
    while jsP_accept(J, '&' as c_int) {
        INCREC!(J);
        a = NODE2!(J, EXP_BITAND, line, a, equality(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn bitxor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitand(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept(J, '^' as c_int) {
        INCREC!(J);
        a = NODE2!(J, EXP_BITXOR, line, a, bitand(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn bitor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitxor(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept(J, '|' as c_int) {
        INCREC!(J);
        a = NODE2!(J, EXP_BITOR, line, a, bitxor(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

unsafe fn logand(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = bitor(J, notin);
    let line = (*J).lexline;
    if jsP_accept(J, TK_AND) {
        INCREC!(J);
        a = NODE2!(J, EXP_LOGAND, line, a, logand(J, notin));
        DECREC!(J);
    }
    a
}

unsafe fn logor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = logand(J, notin);
    let line = (*J).lexline;
    if jsP_accept(J, TK_OR) {
        INCREC!(J);
        a = NODE2!(J, EXP_LOGOR, line, a, logor(J, notin));
        DECREC!(J);
    }
    a
}

unsafe fn conditional(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let a = logor(J, notin);
    let line = (*J).lexline;
    if jsP_accept(J, '?' as c_int) {
        let b: *mut js_Ast;
        let c: *mut js_Ast;
        INCREC!(J);
        b = assignment(J, 0);
        jsP_expect!(J, ':' as c_int);
        c = assignment(J, notin);
        DECREC!(J);
        return NODE3!(J, EXP_COND, line, a, b, c);
    }
    a
}

unsafe fn assignment(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = conditional(J, notin);
    let line = (*J).lexline;
    INCREC!(J);
    if jsP_accept(J, '=' as c_int) {
        a = NODE2!(J, EXP_ASS, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_MUL_ASS) {
        a = NODE2!(J, EXP_ASS_MUL, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_DIV_ASS) {
        a = NODE2!(J, EXP_ASS_DIV, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_MOD_ASS) {
        a = NODE2!(J, EXP_ASS_MOD, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_ADD_ASS) {
        a = NODE2!(J, EXP_ASS_ADD, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_SUB_ASS) {
        a = NODE2!(J, EXP_ASS_SUB, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_SHL_ASS) {
        a = NODE2!(J, EXP_ASS_SHL, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_SHR_ASS) {
        a = NODE2!(J, EXP_ASS_SHR, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_USHR_ASS) {
        a = NODE2!(J, EXP_ASS_USHR, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_AND_ASS) {
        a = NODE2!(J, EXP_ASS_BITAND, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_XOR_ASS) {
        a = NODE2!(J, EXP_ASS_BITXOR, line, a, assignment(J, notin));
    } else if jsP_accept(J, TK_OR_ASS) {
        a = NODE2!(J, EXP_ASS_BITOR, line, a, assignment(J, notin));
    }
    DECREC!(J);
    a
}

unsafe fn expression(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let mut a = assignment(J, notin);
    let SAVE = (*J).astdepth;
    let mut line = (*J).lexline;
    while jsP_accept(J, ',' as c_int) {
        INCREC!(J);
        a = NODE2!(J, EXP_COMMA, line, a, assignment(J, notin));
        line = (*J).lexline;
    }
    (*J).astdepth = SAVE;
    a
}

/* Statements */

unsafe fn vardec(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let a = identifier(J);
    let line = (*J).lexline;
    if jsP_accept(J, '=' as c_int) {
        return NODE2!(J, EXP_VAR, line, a, assignment(J, notin));
    }
    NODE1!(J, EXP_VAR, line, a)
}

unsafe fn vardeclist(J: *mut js_State, notin: c_int) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    head = LIST!(J, vardec(J, notin));
    tail = head;
    while jsP_accept(J, ',' as c_int) {
        let n = LIST!(J, vardec(J, notin));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn statementlist(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == '}' as c_int
        || (*J).lookahead == TK_CASE
        || (*J).lookahead == TK_DEFAULT
    {
        return null_mut();
    }
    head = LIST!(J, statement(J));
    tail = head;
    while (*J).lookahead != '}' as c_int
        && (*J).lookahead != TK_CASE
        && (*J).lookahead != TK_DEFAULT
    {
        let n = LIST!(J, statement(J));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn caseclause(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    let b: *mut js_Ast;
    let line = (*J).lexline;

    if jsP_accept(J, TK_CASE) {
        a = expression(J, 0);
        jsP_expect!(J, ':' as c_int);
        b = statementlist(J);
        return NODE2!(J, STM_CASE, line, a, b);
    }

    if jsP_accept(J, TK_DEFAULT) {
        jsP_expect!(J, ':' as c_int);
        a = statementlist(J);
        return NODE1!(J, STM_DEFAULT, line, a);
    }

    jsP_error!(
        J,
        c"unexpected token in switch: %s (expected 'case' or 'default')".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

unsafe fn caselist(J: *mut js_State) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == '}' as c_int {
        return null_mut();
    }
    head = LIST!(J, caseclause(J));
    tail = head;
    while (*J).lookahead != '}' as c_int {
        let n = LIST!(J, caseclause(J));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn block(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    let line = (*J).lexline;
    jsP_expect!(J, '{' as c_int);
    a = statementlist(J);
    jsP_expect!(J, '}' as c_int);
    NODE1!(J, STM_BLOCK, line, a)
}

unsafe fn forexpression(J: *mut js_State, end: c_int) -> *mut js_Ast {
    let mut a: *mut js_Ast = null_mut();
    if (*J).lookahead != end {
        a = expression(J, 0);
    }
    jsP_expect!(J, end);
    a
}

unsafe fn forstatement(J: *mut js_State, line: c_int) -> *mut js_Ast {
    let mut a: *mut js_Ast;
    let b: *mut js_Ast;
    let c: *mut js_Ast;
    let d: *mut js_Ast;
    jsP_expect!(J, '(' as c_int);
    if jsP_accept(J, TK_VAR) {
        a = vardeclist(J, 1);
        if jsP_accept(J, ';' as c_int) {
            b = forexpression(J, ';' as c_int);
            c = forexpression(J, ')' as c_int);
            d = statement(J);
            return NODE4!(J, STM_FOR_VAR, line, a, b, c, d);
        }
        if jsP_accept(J, TK_IN) {
            b = expression(J, 0);
            jsP_expect!(J, ')' as c_int);
            c = statement(J);
            return NODE3!(J, STM_FOR_IN_VAR, line, a, b, c);
        }
        jsP_error!(
            J,
            c"unexpected token in for-var-statement: %s".as_ptr(),
            jsY_tokenstring((*J).lookahead)
        );
    }

    if (*J).lookahead != ';' as c_int {
        a = expression(J, 1);
    } else {
        a = null_mut();
    }
    if jsP_accept(J, ';' as c_int) {
        b = forexpression(J, ';' as c_int);
        c = forexpression(J, ')' as c_int);
        d = statement(J);
        return NODE4!(J, STM_FOR, line, a, b, c, d);
    }
    if jsP_accept(J, TK_IN) {
        b = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        c = statement(J);
        return NODE3!(J, STM_FOR_IN, line, a, b, c);
    }
    jsP_error!(
        J,
        c"unexpected token in for-statement: %s".as_ptr(),
        jsY_tokenstring((*J).lookahead)
    );
}

unsafe fn statement(J: *mut js_State) -> *mut js_Ast {
    let mut a: *mut js_Ast = null_mut();
    let mut b: *mut js_Ast = null_mut();
    let mut c: *mut js_Ast = null_mut();
    let mut d: *mut js_Ast = null_mut();
    let stm: *mut js_Ast;
    let line = (*J).lexline;

    INCREC!(J);

    if (*J).lookahead == '{' as c_int {
        stm = block(J);
    }
    else if jsP_accept(J, TK_VAR) {
        a = vardeclist(J, 0);
        semicolon(J);
        stm = NODE1!(J, STM_VAR, line, a);
    }
    /* empty statement */
    else if jsP_accept(J, ';' as c_int) {
        stm = NODE0!(J, STM_EMPTY, line);
    }
    else if jsP_accept(J, TK_IF) {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        if jsP_accept(J, TK_ELSE) {
            c = statement(J);
        } else {
            c = null_mut();
        }
        stm = NODE3!(J, STM_IF, line, a, b, c);
    }
    else if jsP_accept(J, TK_DO) {
        a = statement(J);
        jsP_expect!(J, TK_WHILE);
        jsP_expect!(J, '(' as c_int);
        b = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        semicolon(J);
        stm = NODE2!(J, STM_DO, line, a, b);
    }
    else if jsP_accept(J, TK_WHILE) {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        stm = NODE2!(J, STM_WHILE, line, a, b);
    }
    else if jsP_accept(J, TK_FOR) {
        stm = forstatement(J, line);
    }
    else if jsP_accept(J, TK_CONTINUE) {
        a = identifieropt(J);
        semicolon(J);
        stm = NODE1!(J, STM_CONTINUE, line, a);
    }
    else if jsP_accept(J, TK_BREAK) {
        a = identifieropt(J);
        semicolon(J);
        stm = NODE1!(J, STM_BREAK, line, a);
    }
    else if jsP_accept(J, TK_RETURN) {
        if (*J).lookahead != ';' as c_int && (*J).lookahead != '}' as c_int && (*J).lookahead != 0 {
            a = expression(J, 0);
        } else {
            a = null_mut();
        }
        semicolon(J);
        stm = NODE1!(J, STM_RETURN, line, a);
    }
    else if jsP_accept(J, TK_WITH) {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        b = statement(J);
        stm = NODE2!(J, STM_WITH, line, a, b);
    }
    else if jsP_accept(J, TK_SWITCH) {
        jsP_expect!(J, '(' as c_int);
        a = expression(J, 0);
        jsP_expect!(J, ')' as c_int);
        jsP_expect!(J, '{' as c_int);
        b = caselist(J);
        jsP_expect!(J, '}' as c_int);
        stm = NODE2!(J, STM_SWITCH, line, a, b);
    }
    else if jsP_accept(J, TK_THROW) {
        a = expression(J, 0);
        semicolon(J);
        stm = NODE1!(J, STM_THROW, line, a);
    }
    else if jsP_accept(J, TK_TRY) {
        a = block(J);
        d = null_mut();
        c = d;
        b = c;
        if jsP_accept(J, TK_CATCH) {
            jsP_expect!(J, '(' as c_int);
            b = identifier(J);
            jsP_expect!(J, ')' as c_int);
            c = block(J);
        }
        if jsP_accept(J, TK_FINALLY) {
            d = block(J);
        }
        if b.is_null() && d.is_null() {
            jsP_error!(
                J,
                c"unexpected token in try: %s (expected 'catch' or 'finally')".as_ptr(),
                jsY_tokenstring((*J).lookahead)
            );
        }
        stm = NODE4!(J, STM_TRY, line, a, b, c, d);
    }
    else if jsP_accept(J, TK_DEBUGGER) {
        semicolon(J);
        stm = NODE0!(J, STM_DEBUGGER, line);
    }
    else if jsP_accept(J, TK_FUNCTION) {
        jsP_warning!(J, c"function statements are not standard".as_ptr());
        stm = funstm(J, line);
    }
    /* labelled statement or expression statement */
    else if (*J).lookahead == TK_IDENTIFIER {
        a = expression(J, 0);
        if (*a).type_ == EXP_IDENTIFIER && jsP_accept(J, ':' as c_int) {
            (*a).type_ = AST_IDENTIFIER;
            b = statement(J);
            stm = NODE2!(J, STM_LABEL, line, a, b);
        } else {
            semicolon(J);
            stm = a;
        }
    }
    /* expression statement */
    else {
        stm = expression(J, 0);
        semicolon(J);
    }

    DECREC!(J);
    stm
}

/* Program */

unsafe fn scriptelement(J: *mut js_State) -> *mut js_Ast {
    let line = (*J).lexline;
    if jsP_accept(J, TK_FUNCTION) {
        return fundec(J, line);
    }
    statement(J)
}

unsafe fn script(J: *mut js_State, terminator: c_int) -> *mut js_Ast {
    let head: *mut js_Ast;
    let mut tail: *mut js_Ast;
    if (*J).lookahead == terminator {
        return null_mut();
    }
    head = LIST!(J, scriptelement(J));
    tail = head;
    while (*J).lookahead != terminator {
        let n = LIST!(J, scriptelement(J));
        (*tail).b = n;
        tail = n;
    }
    jsP_list(head)
}

unsafe fn funbody(J: *mut js_State) -> *mut js_Ast {
    let a: *mut js_Ast;
    jsP_expect!(J, '{' as c_int);
    a = script(J, '}' as c_int);
    jsP_expect!(J, '}' as c_int);
    a
}

/* Constant folding */

unsafe fn toint32(mut d: f64) -> c_int {
    let two32: f64 = 4294967296.0;
    let two31: f64 = 2147483648.0;

    if !isfinite(d) || d == 0.0 {
        return 0;
    }

    d = fmod(d, two32);
    d = if d >= 0.0 { floor(d) } else { ceil(d) + two32 };
    if d >= two31 {
        cvt_i32(d - two32)
    } else {
        cvt_i32(d)
    }
}

unsafe fn touint32(d: f64) -> c_uint {
    toint32(d) as c_uint
}

unsafe fn jsP_setnumnode(node: *mut js_Ast, x: f64) -> c_int {
    (*node).type_ = EXP_NUMBER;
    (*node).number = x;
    (*node).d = null_mut();
    (*node).c = null_mut();
    (*node).b = null_mut();
    (*node).a = null_mut();
    1
}

unsafe fn jsP_foldconst(mut node: *mut js_Ast) -> c_int {
    let x: f64;
    let y: f64;
    let a: c_int;
    let b: c_int;

    if (*node).type_ == AST_LIST {
        while !node.is_null() {
            jsP_foldconst((*node).a);
            node = (*node).b;
        }
        return 0;
    }

    if (*node).type_ == EXP_NUMBER {
        return 1;
    }

    a = if !(*node).a.is_null() {
        jsP_foldconst((*node).a)
    } else {
        0
    };
    b = if !(*node).b.is_null() {
        jsP_foldconst((*node).b)
    } else {
        0
    };
    if !(*node).c.is_null() {
        jsP_foldconst((*node).c);
    }
    if !(*node).d.is_null() {
        jsP_foldconst((*node).d);
    }

    if a != 0 {
        x = (*(*node).a).number;
        match (*node).type_ {
            EXP_NEG => return jsP_setnumnode(node, -x),
            EXP_POS => return jsP_setnumnode(node, x),
            EXP_BITNOT => return jsP_setnumnode(node, (!toint32(x)) as f64),
            _ => {}
        }

        if b != 0 {
            y = (*(*node).b).number;
            match (*node).type_ {
                EXP_MUL => return jsP_setnumnode(node, x * y),
                EXP_DIV => return jsP_setnumnode(node, x / y),
                EXP_MOD => return jsP_setnumnode(node, fmod(x, y)),
                EXP_ADD => return jsP_setnumnode(node, x + y),
                EXP_SUB => return jsP_setnumnode(node, x - y),
                EXP_SHL => {
                    return jsP_setnumnode(
                        node,
                        (((toint32(x) as c_uint) << (touint32(y) & 0x1F)) as c_int) as f64,
                    )
                }
                EXP_SHR => {
                    return jsP_setnumnode(node, (toint32(x) >> (touint32(y) & 0x1F)) as f64)
                }
                EXP_USHR => {
                    return jsP_setnumnode(node, (touint32(x) >> (touint32(y) & 0x1F)) as f64)
                }
                EXP_BITAND => return jsP_setnumnode(node, (toint32(x) & toint32(y)) as f64),
                EXP_BITXOR => return jsP_setnumnode(node, (toint32(x) ^ toint32(y)) as f64),
                EXP_BITOR => return jsP_setnumnode(node, (toint32(x) | toint32(y)) as f64),
                _ => {}
            }
        }
    }

    0
}

/* Main entry point */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_parse(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) -> *mut js_Ast {
    let p: *mut js_Ast;

    jsY_initlex(J, filename, source);
    jsP_next(J);
    (*J).astdepth = 0;
    p = script(J, 0);
    if !p.is_null() {
        jsP_foldconst(p);
    }

    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_parsefunction(
    J: *mut js_State,
    filename: *const c_char,
    params: *const c_char,
    body: *const c_char,
) -> *mut js_Ast {
    let mut p: *mut js_Ast = null_mut();
    let line: c_int = 0;
    if !params.is_null() {
        jsY_initlex(J, filename, params);
        jsP_next(J);
        (*J).astdepth = 0;
        p = parameters(J);
    }
    NODE3!(
        J,
        EXP_FUN,
        line,
        null_mut(),
        p,
        jsP_parse(J, filename, body)
    )
}
