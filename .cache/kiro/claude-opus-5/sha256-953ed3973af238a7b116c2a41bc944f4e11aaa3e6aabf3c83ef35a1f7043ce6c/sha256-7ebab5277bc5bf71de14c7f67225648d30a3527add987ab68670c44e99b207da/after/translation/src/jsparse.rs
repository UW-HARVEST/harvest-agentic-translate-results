#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jserror::js_newsyntaxerror;
use crate::jslex::{jsY_initlex, jsY_lex, jsY_tokenstring};
use crate::jsrun::{js_free, js_malloc, js_throw};
use crate::jsstate::js_report;
use crate::jsintern::js_intern;
use crate::types::*;
use std::ffi::{c_char, c_int, c_uint};
use std::ptr;

/*
 * Table of AST node names, generated from astnames.h.
 * Kept in the exact same strings and order as the C header so that the
 * dump functions (which index this table by AST node type) match byte for
 * byte.
 */
pub struct AstNames(pub [*const c_char; 92]);
unsafe impl Sync for AstNames {}

pub static astname: AstNames = AstNames([
	c"list".as_ptr(),
	c"fundec".as_ptr(),
	c"identifier".as_ptr(),
	c"exp_identifier".as_ptr(),
	c"exp_number".as_ptr(),
	c"exp_string".as_ptr(),
	c"exp_regexp".as_ptr(),
	c"exp_elision".as_ptr(),
	c"exp_null".as_ptr(),
	c"exp_true".as_ptr(),
	c"exp_false".as_ptr(),
	c"exp_this".as_ptr(),
	c"exp_array".as_ptr(),
	c"exp_object".as_ptr(),
	c"exp_prop_val".as_ptr(),
	c"exp_prop_get".as_ptr(),
	c"exp_prop_set".as_ptr(),
	c"exp_fun".as_ptr(),
	c"exp_index".as_ptr(),
	c"exp_member".as_ptr(),
	c"exp_call".as_ptr(),
	c"exp_new".as_ptr(),
	c"exp_postinc".as_ptr(),
	c"exp_postdec".as_ptr(),
	c"exp_delete".as_ptr(),
	c"exp_void".as_ptr(),
	c"exp_typeof".as_ptr(),
	c"exp_preinc".as_ptr(),
	c"exp_predec".as_ptr(),
	c"exp_pos".as_ptr(),
	c"exp_neg".as_ptr(),
	c"exp_bitnot".as_ptr(),
	c"exp_lognot".as_ptr(),
	c"exp_mod".as_ptr(),
	c"exp_div".as_ptr(),
	c"exp_mul".as_ptr(),
	c"exp_sub".as_ptr(),
	c"exp_add".as_ptr(),
	c"exp_ushr".as_ptr(),
	c"exp_shr".as_ptr(),
	c"exp_shl".as_ptr(),
	c"exp_in".as_ptr(),
	c"exp_instanceof".as_ptr(),
	c"exp_ge".as_ptr(),
	c"exp_le".as_ptr(),
	c"exp_gt".as_ptr(),
	c"exp_lt".as_ptr(),
	c"exp_strictne".as_ptr(),
	c"exp_stricteq".as_ptr(),
	c"exp_ne".as_ptr(),
	c"exp_eq".as_ptr(),
	c"exp_bitand".as_ptr(),
	c"exp_bitxor".as_ptr(),
	c"exp_bitor".as_ptr(),
	c"exp_logand".as_ptr(),
	c"exp_logor".as_ptr(),
	c"exp_cond".as_ptr(),
	c"exp_ass".as_ptr(),
	c"exp_ass_mul".as_ptr(),
	c"exp_ass_div".as_ptr(),
	c"exp_ass_mod".as_ptr(),
	c"exp_ass_add".as_ptr(),
	c"exp_ass_sub".as_ptr(),
	c"exp_ass_shl".as_ptr(),
	c"exp_ass_shr".as_ptr(),
	c"exp_ass_ushr".as_ptr(),
	c"exp_ass_bitand".as_ptr(),
	c"exp_ass_bitxor".as_ptr(),
	c"exp_ass_bitor".as_ptr(),
	c"exp_comma".as_ptr(),
	c"exp_var".as_ptr(),
	c"stm_block".as_ptr(),
	c"stm_empty".as_ptr(),
	c"stm_var".as_ptr(),
	c"stm_if".as_ptr(),
	c"stm_do".as_ptr(),
	c"stm_while".as_ptr(),
	c"stm_for".as_ptr(),
	c"stm_for_var".as_ptr(),
	c"stm_for_in".as_ptr(),
	c"stm_for_in_var".as_ptr(),
	c"stm_continue".as_ptr(),
	c"stm_break".as_ptr(),
	c"stm_return".as_ptr(),
	c"stm_with".as_ptr(),
	c"stm_switch".as_ptr(),
	c"stm_throw".as_ptr(),
	c"stm_try".as_ptr(),
	c"stm_debugger".as_ptr(),
	c"stm_label".as_ptr(),
	c"stm_case".as_ptr(),
	c"stm_default".as_ptr(),
]);

/*
 * #define LIST(h)  jsP_newnode(J, AST_LIST, 0, h, 0, 0, 0)
 *
 * #define EXP0(x)  jsP_newnode(J, EXP_ ## x, line, 0, 0, 0, 0)  (etc.)
 *
 * These are expressed as helper macros below so the call sites read like the C.
 */

macro_rules! LIST {
	($J:expr, $h:expr) => {
		jsP_newnode($J, AST_LIST, 0, $h, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
	};
}

/* We cannot easily do token-pasting for EXP_/STM_ prefixes in a declarative
 * macro, so the call sites below pass the fully-qualified constant explicitly
 * via these thin helpers that mirror the arity of the C macros. */

unsafe fn expn(
	J: *mut js_State,
	ty: c_int,
	line: c_int,
	a: *mut js_Ast,
	b: *mut js_Ast,
	c: *mut js_Ast,
	d: *mut js_Ast,
) -> *mut js_Ast {
	unsafe { jsP_newnode(J, ty, line, a, b, c, d) }
}

/* Error/recursion helpers */

/// Raise a syntax error at the current parser position; `msg` is preformatted.
unsafe fn jsP_error_msg(J: *mut js_State, msg: *const c_char) -> ! {
	unsafe {
		let mut buf: [c_char; 512] = [0; 512];
		snprintf(
			buf.as_mut_ptr(),
			256,
			c"%s:%d: ".as_ptr(),
			(*J).filename,
			(*J).lexline,
		);
		strcat(buf.as_mut_ptr(), msg);
		js_newsyntaxerror(J, buf.as_ptr());
		js_throw(J)
	}
}

macro_rules! jsP_error {
	($J:expr, $fmt:literal) => {
		jsP_error_msg($J, $fmt.as_ptr())
	};
	($J:expr, $fmt:literal, $($a:expr),+) => {{
		let mut __m = [0 as c_char; 256];
		snprintf(__m.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
		jsP_error_msg($J, __m.as_ptr())
	}};
}

/* #define INCREC() if (++J->astdepth > JS_ASTLIMIT) jsP_error(J, "too much recursion") */
macro_rules! INCREC {
	($J:expr) => {{
		(*$J).astdepth += 1;
		if (*$J).astdepth > JS_ASTLIMIT {
			jsP_error!($J, c"too much recursion");
		}
	}};
}

/* #define DECREC() --J->astdepth */
macro_rules! DECREC {
	($J:expr) => {
		(*$J).astdepth -= 1
	};
}

unsafe fn jsP_warning(J: *mut js_State, msg: *const c_char) {
	/* jsP_warning is variadic in C; all call sites pass a plain format string
	 * with no extra arguments, so this thin wrapper suffices. */
	unsafe {
		let mut buf: [c_char; 512] = [0; 512];
		snprintf(
			buf.as_mut_ptr(),
			512,
			c"%s:%d: warning: %s".as_ptr(),
			(*J).filename,
			(*J).lexline,
			msg,
		);
		js_report(J, buf.as_ptr());
	}
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
	unsafe {
		let node: *mut js_Ast = js_malloc(J, std::mem::size_of::<js_Ast>() as c_int) as *mut js_Ast;

		(*node).type_ = type_;
		(*node).line = line;
		(*node).a = a;
		(*node).b = b;
		(*node).c = c;
		(*node).d = d;
		(*node).number = 0.0;
		(*node).string = ptr::null();
		(*node).jumps = ptr::null_mut();
		(*node).casejump = 0;

		(*node).parent = ptr::null_mut();
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
}

unsafe fn jsP_list(head: *mut js_Ast) -> *mut js_Ast {
	unsafe {
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
}

unsafe fn jsP_newstrnode(J: *mut js_State, type_: c_int, s: *const c_char) -> *mut js_Ast {
	unsafe {
		let node = jsP_newnode(
			J,
			type_,
			(*J).lexline,
			ptr::null_mut(),
			ptr::null_mut(),
			ptr::null_mut(),
			ptr::null_mut(),
		);
		(*node).string = js_intern(J, s);
		node
	}
}

unsafe fn jsP_newnumnode(J: *mut js_State, type_: c_int, n: f64) -> *mut js_Ast {
	unsafe {
		let node = jsP_newnode(
			J,
			type_,
			(*J).lexline,
			ptr::null_mut(),
			ptr::null_mut(),
			ptr::null_mut(),
			ptr::null_mut(),
		);
		(*node).number = n;
		node
	}
}

unsafe fn jsP_freejumps(J: *mut js_State, mut node: *mut js_JumpList) {
	unsafe {
		while !node.is_null() {
			let next = (*node).next;
			js_free(J, node as *mut _);
			node = next;
		}
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_freeparse(J: *mut js_State) {
	unsafe {
		let mut node = (*J).gcast;
		while !node.is_null() {
			let next = (*node).gcnext;
			jsP_freejumps(J, (*node).jumps);
			js_free(J, node as *mut _);
			node = next;
		}
		(*J).gcast = ptr::null_mut();
	}
}

/* Lookahead */

unsafe fn jsP_next(J: *mut js_State) {
	unsafe {
		(*J).lookahead = jsY_lex(J);
	}
}

/* #define jsP_accept(J,x) (J->lookahead == x ? (jsP_next(J), 1) : 0) */
unsafe fn jsP_accept(J: *mut js_State, x: c_int) -> c_int {
	unsafe {
		if (*J).lookahead == x {
			jsP_next(J);
			1
		} else {
			0
		}
	}
}

/* #define jsP_expect(J,x) if (!jsP_accept(J, x)) jsP_error(...) */
unsafe fn jsP_expect(J: *mut js_State, x: c_int) {
	unsafe {
		if jsP_accept(J, x) == 0 {
			jsP_error!(
				J,
				c"unexpected token: %s (expected %s)",
				jsY_tokenstring((*J).lookahead),
				jsY_tokenstring(x)
			);
		}
	}
}

unsafe fn semicolon(J: *mut js_State) {
	unsafe {
		if (*J).lookahead == ';' as c_int {
			jsP_next(J);
			return;
		}
		if (*J).newline != 0 || (*J).lookahead == '}' as c_int || (*J).lookahead == 0 {
			return;
		}
		jsP_error!(
			J,
			c"unexpected token: %s (expected ';')",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

/* Literals */

unsafe fn identifier(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == TK_IDENTIFIER {
			let a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
			jsP_next(J);
			return a;
		}
		jsP_error!(
			J,
			c"unexpected token: %s (expected identifier)",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

unsafe fn identifieropt(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == TK_IDENTIFIER {
			return identifier(J);
		}
		ptr::null_mut()
	}
}

unsafe fn identifiername(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == TK_IDENTIFIER || (*J).lookahead >= TK_BREAK {
			let a = jsP_newstrnode(J, AST_IDENTIFIER, (*J).text);
			jsP_next(J);
			return a;
		}
		jsP_error!(
			J,
			c"unexpected token: %s (expected identifier or keyword)",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

unsafe fn arrayelement(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;
		if (*J).lookahead == ',' as c_int {
			return expn(J, EXP_ELISION, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		assignment(J, 0)
	}
}

unsafe fn arrayliteral(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == ']' as c_int {
			return ptr::null_mut();
		}
		let ae = arrayelement(J);
		let head = LIST!(J, ae);
		let mut tail = head;
		while jsP_accept(J, ',' as c_int) != 0 {
			if (*J).lookahead != ']' as c_int {
				let ae = arrayelement(J);
				(*tail).b = LIST!(J, ae);
				tail = (*tail).b;
			}
		}
		jsP_list(head)
	}
}

unsafe fn propname(J: *mut js_State) -> *mut js_Ast {
	unsafe {
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
}

unsafe fn propassign(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;

		let mut name = propname(J);

		if (*J).lookahead != ':' as c_int && (*name).type_ == AST_IDENTIFIER {
			if strcmp((*name).string, c"get".as_ptr()) == 0 {
				name = propname(J);
				jsP_expect(J, '(' as c_int);
				jsP_expect(J, ')' as c_int);
				let body = funbody(J);
				return expn(J, EXP_PROP_GET, line, name, ptr::null_mut(), body, ptr::null_mut());
			}
			if strcmp((*name).string, c"set".as_ptr()) == 0 {
				name = propname(J);
				jsP_expect(J, '(' as c_int);
				let arg = identifier(J);
				jsP_expect(J, ')' as c_int);
				let body = funbody(J);
				return expn(J, EXP_PROP_SET, line, name, LIST!(J, arg), body, ptr::null_mut());
			}
		}

		jsP_expect(J, ':' as c_int);
		let value = assignment(J, 0);
		expn(J, EXP_PROP_VAL, line, name, value, ptr::null_mut(), ptr::null_mut())
	}
}

unsafe fn objectliteral(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == '}' as c_int {
			return ptr::null_mut();
		}
		let pa = propassign(J);
		let head = LIST!(J, pa);
		let mut tail = head;
		while jsP_accept(J, ',' as c_int) != 0 {
			if (*J).lookahead == '}' as c_int {
				break;
			}
			let pa = propassign(J);
			(*tail).b = LIST!(J, pa);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

/* Functions */

unsafe fn parameters(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == ')' as c_int {
			return ptr::null_mut();
		}
		let id = identifier(J);
		let head = LIST!(J, id);
		let mut tail = head;
		while jsP_accept(J, ',' as c_int) != 0 {
			let id = identifier(J);
			(*tail).b = LIST!(J, id);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn fundec(J: *mut js_State, line: c_int) -> *mut js_Ast {
	unsafe {
		let a = identifier(J);
		jsP_expect(J, '(' as c_int);
		let b = parameters(J);
		jsP_expect(J, ')' as c_int);
		let c = funbody(J);
		jsP_newnode(J, AST_FUNDEC, line, a, b, c, ptr::null_mut())
	}
}

unsafe fn funstm(J: *mut js_State, line: c_int) -> *mut js_Ast {
	unsafe {
		let a = identifier(J);
		jsP_expect(J, '(' as c_int);
		let b = parameters(J);
		jsP_expect(J, ')' as c_int);
		let c = funbody(J);
		/* rewrite function statement as "var X = function X() {}" */
		let fun = expn(J, EXP_FUN, line, a, b, c, ptr::null_mut());
		let var = expn(J, EXP_VAR, line, a, fun, ptr::null_mut(), ptr::null_mut());
		expn(J, STM_VAR, line, LIST!(J, var), ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
	}
}

unsafe fn funexp(J: *mut js_State, line: c_int) -> *mut js_Ast {
	unsafe {
		let a = identifieropt(J);
		jsP_expect(J, '(' as c_int);
		let b = parameters(J);
		jsP_expect(J, ')' as c_int);
		let c = funbody(J);
		expn(J, EXP_FUN, line, a, b, c, ptr::null_mut())
	}
}

/* Expressions */

unsafe fn primary(J: *mut js_State) -> *mut js_Ast {
	unsafe {
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

		if jsP_accept(J, TK_THIS) != 0 {
			return expn(J, EXP_THIS, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		if jsP_accept(J, TK_NULL) != 0 {
			return expn(J, EXP_NULL, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		if jsP_accept(J, TK_TRUE) != 0 {
			return expn(J, EXP_TRUE, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		if jsP_accept(J, TK_FALSE) != 0 {
			return expn(J, EXP_FALSE, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		if jsP_accept(J, '{' as c_int) != 0 {
			let obj = objectliteral(J);
			let a = expn(J, EXP_OBJECT, line, obj, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
			jsP_expect(J, '}' as c_int);
			return a;
		}
		if jsP_accept(J, '[' as c_int) != 0 {
			let arr = arrayliteral(J);
			let a = expn(J, EXP_ARRAY, line, arr, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
			jsP_expect(J, ']' as c_int);
			return a;
		}
		if jsP_accept(J, '(' as c_int) != 0 {
			let a = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			return a;
		}

		jsP_error!(
			J,
			c"unexpected token in expression: %s",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

unsafe fn arguments(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == ')' as c_int {
			return ptr::null_mut();
		}
		let e = assignment(J, 0);
		let head = LIST!(J, e);
		let mut tail = head;
		while jsP_accept(J, ',' as c_int) != 0 {
			let e = assignment(J, 0);
			(*tail).b = LIST!(J, e);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn newexp(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;

		if jsP_accept(J, TK_NEW) != 0 {
			let a = memberexp(J);
			if jsP_accept(J, '(' as c_int) != 0 {
				let b = arguments(J);
				jsP_expect(J, ')' as c_int);
				return expn(J, EXP_NEW, line, a, b, ptr::null_mut(), ptr::null_mut());
			}
			return expn(J, EXP_NEW, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}

		if jsP_accept(J, TK_FUNCTION) != 0 {
			return funexp(J, line);
		}

		primary(J)
	}
}

unsafe fn memberexp(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let mut a = newexp(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, '.' as c_int) != 0 {
				a = expn(J, EXP_MEMBER, line, a, identifiername(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '[' as c_int) != 0 {
				a = expn(J, EXP_INDEX, line, a, expression(J, 0), ptr::null_mut(), ptr::null_mut());
				jsP_expect(J, ']' as c_int);
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn callexp(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let mut a = newexp(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, '.' as c_int) != 0 {
				a = expn(J, EXP_MEMBER, line, a, identifiername(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '[' as c_int) != 0 {
				a = expn(J, EXP_INDEX, line, a, expression(J, 0), ptr::null_mut(), ptr::null_mut());
				jsP_expect(J, ']' as c_int);
				continue;
			}
			if jsP_accept(J, '(' as c_int) != 0 {
				a = expn(J, EXP_CALL, line, a, arguments(J), ptr::null_mut(), ptr::null_mut());
				jsP_expect(J, ')' as c_int);
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn postfix(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let a = callexp(J);
		let line = (*J).lexline;
		if (*J).newline == 0 && jsP_accept(J, TK_INC) != 0 {
			return expn(J, EXP_POSTINC, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		if (*J).newline == 0 && jsP_accept(J, TK_DEC) != 0 {
			return expn(J, EXP_POSTDEC, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		a
	}
}

unsafe fn unary(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let a;
		let line = (*J).lexline;
		INCREC!(J);
		if jsP_accept(J, TK_DELETE) != 0 {
			a = expn(J, EXP_DELETE, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_VOID) != 0 {
			a = expn(J, EXP_VOID, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_TYPEOF) != 0 {
			a = expn(J, EXP_TYPEOF, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_INC) != 0 {
			a = expn(J, EXP_PREINC, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_DEC) != 0 {
			a = expn(J, EXP_PREDEC, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, '+' as c_int) != 0 {
			a = expn(J, EXP_POS, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, '-' as c_int) != 0 {
			a = expn(J, EXP_NEG, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, '~' as c_int) != 0 {
			a = expn(J, EXP_BITNOT, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, '!' as c_int) != 0 {
			a = expn(J, EXP_LOGNOT, line, unary(J), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else {
			a = postfix(J);
		}
		DECREC!(J);
		a
	}
}

unsafe fn multiplicative(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let mut a = unary(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, '*' as c_int) != 0 {
				a = expn(J, EXP_MUL, line, a, unary(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '/' as c_int) != 0 {
				a = expn(J, EXP_DIV, line, a, unary(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '%' as c_int) != 0 {
				a = expn(J, EXP_MOD, line, a, unary(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn additive(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let mut a = multiplicative(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, '+' as c_int) != 0 {
				a = expn(J, EXP_ADD, line, a, multiplicative(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '-' as c_int) != 0 {
				a = expn(J, EXP_SUB, line, a, multiplicative(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn shift(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let mut a = additive(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, TK_SHL) != 0 {
				a = expn(J, EXP_SHL, line, a, additive(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_SHR) != 0 {
				a = expn(J, EXP_SHR, line, a, additive(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_USHR) != 0 {
				a = expn(J, EXP_USHR, line, a, additive(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn relational(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = shift(J);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, '<' as c_int) != 0 {
				a = expn(J, EXP_LT, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, '>' as c_int) != 0 {
				a = expn(J, EXP_GT, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_LE) != 0 {
				a = expn(J, EXP_LE, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_GE) != 0 {
				a = expn(J, EXP_GE, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_INSTANCEOF) != 0 {
				a = expn(J, EXP_INSTANCEOF, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if notin == 0 && jsP_accept(J, TK_IN) != 0 {
				a = expn(J, EXP_IN, line, a, shift(J), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn equality(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = relational(J, notin);
		#[allow(unused_variables)]
		let mut line;
		let SAVE = (*J).astdepth;
		loop {
			INCREC!(J);
			line = (*J).lexline;
			if jsP_accept(J, TK_EQ) != 0 {
				a = expn(J, EXP_EQ, line, a, relational(J, notin), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_NE) != 0 {
				a = expn(J, EXP_NE, line, a, relational(J, notin), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_STRICTEQ) != 0 {
				a = expn(J, EXP_STRICTEQ, line, a, relational(J, notin), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			if jsP_accept(J, TK_STRICTNE) != 0 {
				a = expn(J, EXP_STRICTNE, line, a, relational(J, notin), ptr::null_mut(), ptr::null_mut());
				continue;
			}
			break;
		}
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn bitand(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = equality(J, notin);
		let SAVE = (*J).astdepth;
		let mut line = (*J).lexline;
		while jsP_accept(J, '&' as c_int) != 0 {
			INCREC!(J);
			a = expn(J, EXP_BITAND, line, a, equality(J, notin), ptr::null_mut(), ptr::null_mut());
			line = (*J).lexline;
		}
		let _ = line;
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn bitxor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = bitand(J, notin);
		let SAVE = (*J).astdepth;
		let mut line = (*J).lexline;
		while jsP_accept(J, '^' as c_int) != 0 {
			INCREC!(J);
			a = expn(J, EXP_BITXOR, line, a, bitand(J, notin), ptr::null_mut(), ptr::null_mut());
			line = (*J).lexline;
		}
		let _ = line;
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn bitor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = bitxor(J, notin);
		let SAVE = (*J).astdepth;
		let mut line = (*J).lexline;
		while jsP_accept(J, '|' as c_int) != 0 {
			INCREC!(J);
			a = expn(J, EXP_BITOR, line, a, bitxor(J, notin), ptr::null_mut(), ptr::null_mut());
			line = (*J).lexline;
		}
		let _ = line;
		(*J).astdepth = SAVE;
		a
	}
}

unsafe fn logand(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = bitor(J, notin);
		let line = (*J).lexline;
		if jsP_accept(J, TK_AND) != 0 {
			INCREC!(J);
			a = expn(J, EXP_LOGAND, line, a, logand(J, notin), ptr::null_mut(), ptr::null_mut());
			DECREC!(J);
		}
		a
	}
}

unsafe fn logor(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = logand(J, notin);
		let line = (*J).lexline;
		if jsP_accept(J, TK_OR) != 0 {
			INCREC!(J);
			a = expn(J, EXP_LOGOR, line, a, logor(J, notin), ptr::null_mut(), ptr::null_mut());
			DECREC!(J);
		}
		a
	}
}

unsafe fn conditional(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let a = logor(J, notin);
		let line = (*J).lexline;
		if jsP_accept(J, '?' as c_int) != 0 {
			INCREC!(J);
			let b = assignment(J, 0);
			jsP_expect(J, ':' as c_int);
			let c = assignment(J, notin);
			DECREC!(J);
			return expn(J, EXP_COND, line, a, b, c, ptr::null_mut());
		}
		a
	}
}

unsafe fn assignment(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = conditional(J, notin);
		let line = (*J).lexline;
		INCREC!(J);
		if jsP_accept(J, '=' as c_int) != 0 {
			a = expn(J, EXP_ASS, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_MUL_ASS) != 0 {
			a = expn(J, EXP_ASS_MUL, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_DIV_ASS) != 0 {
			a = expn(J, EXP_ASS_DIV, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_MOD_ASS) != 0 {
			a = expn(J, EXP_ASS_MOD, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_ADD_ASS) != 0 {
			a = expn(J, EXP_ASS_ADD, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_SUB_ASS) != 0 {
			a = expn(J, EXP_ASS_SUB, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_SHL_ASS) != 0 {
			a = expn(J, EXP_ASS_SHL, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_SHR_ASS) != 0 {
			a = expn(J, EXP_ASS_SHR, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_USHR_ASS) != 0 {
			a = expn(J, EXP_ASS_USHR, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_AND_ASS) != 0 {
			a = expn(J, EXP_ASS_BITAND, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_XOR_ASS) != 0 {
			a = expn(J, EXP_ASS_BITXOR, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_OR_ASS) != 0 {
			a = expn(J, EXP_ASS_BITOR, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		}
		DECREC!(J);
		a
	}
}

unsafe fn expression(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let mut a = assignment(J, notin);
		let SAVE = (*J).astdepth;
		let mut line = (*J).lexline;
		while jsP_accept(J, ',' as c_int) != 0 {
			INCREC!(J);
			a = expn(J, EXP_COMMA, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
			line = (*J).lexline;
		}
		let _ = line;
		(*J).astdepth = SAVE;
		a
	}
}

/* Statements */

unsafe fn vardec(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let a = identifier(J);
		let line = (*J).lexline;
		if jsP_accept(J, '=' as c_int) != 0 {
			return expn(J, EXP_VAR, line, a, assignment(J, notin), ptr::null_mut(), ptr::null_mut());
		}
		expn(J, EXP_VAR, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
	}
}

unsafe fn vardeclist(J: *mut js_State, notin: c_int) -> *mut js_Ast {
	unsafe {
		let vd = vardec(J, notin);
		let head = LIST!(J, vd);
		let mut tail = head;
		while jsP_accept(J, ',' as c_int) != 0 {
			let vd = vardec(J, notin);
			(*tail).b = LIST!(J, vd);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn statementlist(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == '}' as c_int
			|| (*J).lookahead == TK_CASE
			|| (*J).lookahead == TK_DEFAULT
		{
			return ptr::null_mut();
		}
		let st = statement(J);
		let head = LIST!(J, st);
		let mut tail = head;
		while (*J).lookahead != '}' as c_int
			&& (*J).lookahead != TK_CASE
			&& (*J).lookahead != TK_DEFAULT
		{
			let st = statement(J);
			(*tail).b = LIST!(J, st);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn caseclause(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;

		if jsP_accept(J, TK_CASE) != 0 {
			let a = expression(J, 0);
			jsP_expect(J, ':' as c_int);
			let b = statementlist(J);
			return expn(J, STM_CASE, line, a, b, ptr::null_mut(), ptr::null_mut());
		}

		if jsP_accept(J, TK_DEFAULT) != 0 {
			jsP_expect(J, ':' as c_int);
			let a = statementlist(J);
			return expn(J, STM_DEFAULT, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}

		jsP_error!(
			J,
			c"unexpected token in switch: %s (expected 'case' or 'default')",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

unsafe fn caselist(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == '}' as c_int {
			return ptr::null_mut();
		}
		let cc = caseclause(J);
		let head = LIST!(J, cc);
		let mut tail = head;
		while (*J).lookahead != '}' as c_int {
			let cc = caseclause(J);
			(*tail).b = LIST!(J, cc);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn block(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;
		jsP_expect(J, '{' as c_int);
		let a = statementlist(J);
		jsP_expect(J, '}' as c_int);
		expn(J, STM_BLOCK, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
	}
}

unsafe fn forexpression(J: *mut js_State, end: c_int) -> *mut js_Ast {
	unsafe {
		let mut a: *mut js_Ast = ptr::null_mut();
		if (*J).lookahead != end {
			a = expression(J, 0);
		}
		jsP_expect(J, end);
		a
	}
}

unsafe fn forstatement(J: *mut js_State, line: c_int) -> *mut js_Ast {
	unsafe {
		let a;
		let b;
		let c;
		let d;
		jsP_expect(J, '(' as c_int);
		if jsP_accept(J, TK_VAR) != 0 {
			a = vardeclist(J, 1);
			if jsP_accept(J, ';' as c_int) != 0 {
				b = forexpression(J, ';' as c_int);
				c = forexpression(J, ')' as c_int);
				d = statement(J);
				return expn(J, STM_FOR_VAR, line, a, b, c, d);
			}
			if jsP_accept(J, TK_IN) != 0 {
				b = expression(J, 0);
				jsP_expect(J, ')' as c_int);
				c = statement(J);
				return expn(J, STM_FOR_IN_VAR, line, a, b, c, ptr::null_mut());
			}
			jsP_error!(
				J,
				c"unexpected token in for-var-statement: %s",
				jsY_tokenstring((*J).lookahead)
			);
		}

		let a2;
		if (*J).lookahead != ';' as c_int {
			a2 = expression(J, 1);
		} else {
			a2 = ptr::null_mut();
		}
		if jsP_accept(J, ';' as c_int) != 0 {
			let b = forexpression(J, ';' as c_int);
			let c = forexpression(J, ')' as c_int);
			let d = statement(J);
			return expn(J, STM_FOR, line, a2, b, c, d);
		}
		if jsP_accept(J, TK_IN) != 0 {
			let b = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			let c = statement(J);
			return expn(J, STM_FOR_IN, line, a2, b, c, ptr::null_mut());
		}
		jsP_error!(
			J,
			c"unexpected token in for-statement: %s",
			jsY_tokenstring((*J).lookahead)
		);
	}
}

unsafe fn statement(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let a: *mut js_Ast;
		let b: *mut js_Ast;
		let c: *mut js_Ast;
		#[allow(unused_assignments)]
		let stm;
		let line = (*J).lexline;

		INCREC!(J);

		if (*J).lookahead == '{' as c_int {
			stm = block(J);
		} else if jsP_accept(J, TK_VAR) != 0 {
			a = vardeclist(J, 0);
			semicolon(J);
			stm = expn(J, STM_VAR, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		}
		/* empty statement */
		else if jsP_accept(J, ';' as c_int) != 0 {
			stm = expn(J, STM_EMPTY, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_IF) != 0 {
			jsP_expect(J, '(' as c_int);
			a = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			b = statement(J);
			if jsP_accept(J, TK_ELSE) != 0 {
				c = statement(J);
			} else {
				c = ptr::null_mut();
			}
			stm = expn(J, STM_IF, line, a, b, c, ptr::null_mut());
		} else if jsP_accept(J, TK_DO) != 0 {
			a = statement(J);
			jsP_expect(J, TK_WHILE);
			jsP_expect(J, '(' as c_int);
			b = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			semicolon(J);
			stm = expn(J, STM_DO, line, a, b, ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_WHILE) != 0 {
			jsP_expect(J, '(' as c_int);
			a = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			b = statement(J);
			stm = expn(J, STM_WHILE, line, a, b, ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_FOR) != 0 {
			stm = forstatement(J, line);
		} else if jsP_accept(J, TK_CONTINUE) != 0 {
			a = identifieropt(J);
			semicolon(J);
			stm = expn(J, STM_CONTINUE, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_BREAK) != 0 {
			a = identifieropt(J);
			semicolon(J);
			stm = expn(J, STM_BREAK, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_RETURN) != 0 {
			if (*J).lookahead != ';' as c_int
				&& (*J).lookahead != '}' as c_int
				&& (*J).lookahead != 0
			{
				a = expression(J, 0);
			} else {
				a = ptr::null_mut();
			}
			semicolon(J);
			stm = expn(J, STM_RETURN, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_WITH) != 0 {
			jsP_expect(J, '(' as c_int);
			a = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			b = statement(J);
			stm = expn(J, STM_WITH, line, a, b, ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_SWITCH) != 0 {
			jsP_expect(J, '(' as c_int);
			a = expression(J, 0);
			jsP_expect(J, ')' as c_int);
			jsP_expect(J, '{' as c_int);
			b = caselist(J);
			jsP_expect(J, '}' as c_int);
			stm = expn(J, STM_SWITCH, line, a, b, ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_THROW) != 0 {
			a = expression(J, 0);
			semicolon(J);
			stm = expn(J, STM_THROW, line, a, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_TRY) != 0 {
			a = block(J);
			let mut bb: *mut js_Ast = ptr::null_mut();
			let mut cc: *mut js_Ast = ptr::null_mut();
			let mut dd: *mut js_Ast = ptr::null_mut();
			if jsP_accept(J, TK_CATCH) != 0 {
				jsP_expect(J, '(' as c_int);
				bb = identifier(J);
				jsP_expect(J, ')' as c_int);
				cc = block(J);
			}
			if jsP_accept(J, TK_FINALLY) != 0 {
				dd = block(J);
			}
			if bb.is_null() && dd.is_null() {
				jsP_error!(
					J,
					c"unexpected token in try: %s (expected 'catch' or 'finally')",
					jsY_tokenstring((*J).lookahead)
				);
			}
			stm = expn(J, STM_TRY, line, a, bb, cc, dd);
		} else if jsP_accept(J, TK_DEBUGGER) != 0 {
			semicolon(J);
			stm = expn(J, STM_DEBUGGER, line, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
		} else if jsP_accept(J, TK_FUNCTION) != 0 {
			jsP_warning(J, c"function statements are not standard".as_ptr());
			stm = funstm(J, line);
		}
		/* labelled statement or expression statement */
		else if (*J).lookahead == TK_IDENTIFIER {
			let ae = expression(J, 0);
			if (*ae).type_ == EXP_IDENTIFIER && jsP_accept(J, ':' as c_int) != 0 {
				(*ae).type_ = AST_IDENTIFIER;
				b = statement(J);
				stm = expn(J, STM_LABEL, line, ae, b, ptr::null_mut(), ptr::null_mut());
			} else {
				semicolon(J);
				stm = ae;
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
}

/* Program */

unsafe fn scriptelement(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		let line = (*J).lexline;
		if jsP_accept(J, TK_FUNCTION) != 0 {
			return fundec(J, line);
		}
		statement(J)
	}
}

unsafe fn script(J: *mut js_State, terminator: c_int) -> *mut js_Ast {
	unsafe {
		if (*J).lookahead == terminator {
			return ptr::null_mut();
		}
		let se = scriptelement(J);
		let head = LIST!(J, se);
		let mut tail = head;
		while (*J).lookahead != terminator {
			let se = scriptelement(J);
			(*tail).b = LIST!(J, se);
			tail = (*tail).b;
		}
		jsP_list(head)
	}
}

unsafe fn funbody(J: *mut js_State) -> *mut js_Ast {
	unsafe {
		jsP_expect(J, '{' as c_int);
		let a = script(J, '}' as c_int);
		jsP_expect(J, '}' as c_int);
		a
	}
}

/* Constant folding */

unsafe fn toint32(mut d: f64) -> c_int {
	unsafe {
		let two32 = 4294967296.0_f64;
		let two31 = 2147483648.0_f64;

		if !isfinite(d) || d == 0.0 {
			return 0;
		}

		d = fmod(d, two32);
		d = if d >= 0.0 { floor(d) } else { ceil(d) + two32 };
		if d >= two31 {
			(d - two32) as c_int
		} else {
			d as c_int
		}
	}
}

unsafe fn touint32(d: f64) -> c_uint {
	unsafe { toint32(d) as c_uint }
}

unsafe fn jsP_setnumnode(node: *mut js_Ast, x: f64) -> c_int {
	unsafe {
		(*node).type_ = EXP_NUMBER;
		(*node).number = x;
		(*node).a = ptr::null_mut();
		(*node).b = ptr::null_mut();
		(*node).c = ptr::null_mut();
		(*node).d = ptr::null_mut();
		1
	}
}

unsafe fn jsP_foldconst(node: *mut js_Ast) -> c_int {
	unsafe {
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
						let sh = touint32(y) & 0x1F;
						let v = (toint32(x) as u32).wrapping_shl(sh) as i32;
						return jsP_setnumnode(node, v as f64);
					}
					EXP_SHR => {
						let sh = touint32(y) & 0x1F;
						let v = toint32(x) >> sh;
						return jsP_setnumnode(node, v as f64);
					}
					EXP_USHR => {
						let sh = touint32(y) & 0x1F;
						let v = touint32(x) >> sh;
						return jsP_setnumnode(node, v as f64);
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
}

/* Main entry point */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_parse(
	J: *mut js_State,
	filename: *const c_char,
	source: *const c_char,
) -> *mut js_Ast {
	unsafe {
		jsY_initlex(J, filename, source);
		jsP_next(J);
		(*J).astdepth = 0;
		let p = script(J, 0);
		if !p.is_null() {
			jsP_foldconst(p);
		}
		p
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsP_parsefunction(
	J: *mut js_State,
	filename: *const c_char,
	params: *const c_char,
	body: *const c_char,
) -> *mut js_Ast {
	unsafe {
		let mut p: *mut js_Ast = ptr::null_mut();
		let line = 0;
		if !params.is_null() {
			jsY_initlex(J, filename, params);
			jsP_next(J);
			(*J).astdepth = 0;
			p = parameters(J);
		}
		expn(J, EXP_FUN, line, ptr::null_mut(), p, jsP_parse(J, filename, body), ptr::null_mut())
	}
}
