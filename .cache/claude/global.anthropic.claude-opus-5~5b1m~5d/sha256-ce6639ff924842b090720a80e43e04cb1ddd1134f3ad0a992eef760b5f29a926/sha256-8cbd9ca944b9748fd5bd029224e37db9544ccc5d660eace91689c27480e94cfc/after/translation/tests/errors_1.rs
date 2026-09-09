//! Error-path differential tests, ERRORS.md rows 1..64.
//!
//! Covers jsarray.c, jsboolean.c, jsbuiltin.c, jscompile.c, jsdate.c,
//! jsdtoa.c, jserror.c, jsfunction.c and jsintern.c.
//!
//! Every test drives EXACTLY the guarding condition of its ERRORS.md row and
//! makes the rejection observable:
//!   * runtime errors: the trigger is run inside a JS `try`/`catch` that prints
//!     `e.name + ": " + e.message`, and then the bare trigger is run through
//!     `js_dostring` so the default report (stderr) and the return code are
//!     compared as well;
//!   * compile time errors (jscompile.c) cannot be caught by a `try` in the
//!     same script, so the caught variant compiles the trigger with `eval()`;
//!   * errors reachable only through the C API are triggered by calling the
//!     exported function directly and letting the child die (the harness
//!     compares the exit status too).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static mut stdout: *mut c_void;
}

const IONBF: c_int = 2;

/* --- a `print` global, like dtest/driver.c ------------------------------- */

static mut CUR: *const Api = std::ptr::null();

unsafe extern "C" fn jsB_print(J: JS) {
    let api = &*CUR;
    let top = (api.js_gettop)(J);
    let mut i = 1;
    while i < top {
        let s = (api.js_tostring)(J, i);
        if i > 1 {
            putchar(b' ' as c_int);
        }
        printf(b"%s\0".as_ptr() as *const c_char, s);
        i += 1;
    }
    putchar(b'\n' as c_int);
    (api.js_pushundefined)(J);
}

/// Fresh state with `print` installed. Also makes stdout unbuffered so that
/// stdout/stderr interleaving is deterministic and nothing is lost when a
/// child aborts.
unsafe fn setup(api: &Api, row: c_int) -> JS {
    CUR = api as *const Api;
    setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
    printf(b"== row %d ==\n\0".as_ptr() as *const c_char, row);
    let J = newstate(api, 0);
    (api.js_newcfunction)(J, Some(jsB_print), b"print\0".as_ptr() as *const c_char, 1);
    (api.js_setglobal)(J, cs("print").as_ptr());
    J
}

/// Quote `s` as a JS double quoted string literal.
fn jsquote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Run `trigger` twice: once inside a try/catch printing name+message, once
/// raw so the default report and the js_dostring return code are compared.
/// `use_eval` wraps the trigger in `eval()` (needed for compile time errors).
unsafe fn case_impl(api: &Api, row: c_int, trigger: &str, use_eval: bool) {
    let J = setup(api, row);

    let caught = if use_eval {
        format!(
            "try {{ eval({}) }} catch (e) {{ print(e.name + \": \" + e.message) }}",
            jsquote(trigger)
        )
    } else {
        format!(
            "try {{ {} }} catch (e) {{ print(e.name + \": \" + e.message) }}",
            trigger
        )
    };
    let s = cs(&caught);
    let rc = (api.js_dostring)(J, s.as_ptr());
    p_int("caught_rc", rc);
    p_int("caught_top", (api.js_gettop)(J));

    p_line("-- raw --");
    let raw = cs(trigger);
    let rc2 = (api.js_dostring)(J, raw.as_ptr());
    p_int("raw_rc", rc2);
    p_int("raw_top", (api.js_gettop)(J));

    (api.js_freestate)(J);
}

/// Runtime (js_*error) trigger.
unsafe fn rt(api: &Api, row: c_int, trigger: &str) {
    case_impl(api, row, trigger, false);
}

/// Compile time (jsC_error / js_syntaxerror / js_evalerror) trigger.
unsafe fn ct(api: &Api, row: c_int, trigger: &str) {
    case_impl(api, row, trigger, true);
}

/* ===================== jsarray.c (rows 1..15) ============================ */

/* row 1: Ap_join, n + seplen + rlen > JS_STRLIMIT (1<<28).
 * 257 references to the same 1 MiB string joined with an empty separator:
 * n reaches exactly 1<<28 after 256 elements, the 257th overflows the limit. */
#[test]
fn err001_join_invalid_string_length() {
    diff("err001", |api| unsafe {
        rt(
            api,
            1,
            "var s = \"x\"; for (var i = 0; i < 20; ++i) s = s + s; \
             var a = []; for (var j = 0; j < 257; ++j) a[j] = s; \
             a.join(\"\")",
        )
    });
}

/* row 2: Ap_sort, comparison function neither callable nor undefined. */
#[test]
fn err002_sort_comparison_not_function() {
    diff("err002", |api| unsafe { rt(api, 2, "[3,1,2].sort(1)") });
}

/* row 3: Ap_sort, len >= INT_MAX. */
#[test]
fn err003_sort_array_too_large() {
    diff("err003", |api| unsafe {
        rt(api, 3, "Array.prototype.sort.call({length:2147483647})")
    });
}

/* row 4: Ap_toString, 'this' not coercible. */
#[test]
fn err004_tostring_this_not_object() {
    diff("err004", |api| unsafe {
        rt(api, 4, "Array.prototype.toString.call(null)")
    });
}

/* row 5: Ap_every, callback not callable. */
#[test]
fn err005_every_callback_not_function() {
    diff("err005", |api| unsafe { rt(api, 5, "[1].every(1)") });
}

/* row 6: Ap_some, callback not callable. */
#[test]
fn err006_some_callback_not_function() {
    diff("err006", |api| unsafe { rt(api, 6, "[1].some(1)") });
}

/* row 7: Ap_forEach, callback not callable. */
#[test]
fn err007_foreach_callback_not_function() {
    diff("err007", |api| unsafe { rt(api, 7, "[1].forEach(1)") });
}

/* row 8: Ap_map, callback not callable. */
#[test]
fn err008_map_callback_not_function() {
    diff("err008", |api| unsafe { rt(api, 8, "[1].map(1)") });
}

/* row 9: Ap_filter, callback not callable. */
#[test]
fn err009_filter_callback_not_function() {
    diff("err009", |api| unsafe { rt(api, 9, "[1].filter(1)") });
}

/* row 10: Ap_reduce, callback not callable. */
#[test]
fn err010_reduce_callback_not_function() {
    diff("err010", |api| unsafe { rt(api, 10, "[1].reduce(1)") });
}

/* row 11: Ap_reduce, len == 0 && !hasinitial. */
#[test]
fn err011_reduce_no_initial_value_empty() {
    diff("err011", |api| unsafe { rt(api, 11, "[].reduce(function(){})") });
}

/* row 12: Ap_reduce, k == len (length > 0 but no present index). */
#[test]
fn err012_reduce_no_initial_value_holes() {
    diff("err012", |api| unsafe {
        rt(api, 12, "Array.prototype.reduce.call({length:3}, function(){})")
    });
}

/* row 13: Ap_reduceRight, callback not callable. */
#[test]
fn err013_reduceright_callback_not_function() {
    diff("err013", |api| unsafe { rt(api, 13, "[1].reduceRight(1)") });
}

/* row 14: Ap_reduceRight, len == 0 && !hasinitial. */
#[test]
fn err014_reduceright_no_initial_value_empty() {
    diff("err014", |api| unsafe {
        rt(api, 14, "[].reduceRight(function(){})")
    });
}

/* row 15: Ap_reduceRight, k < 0 (length > 0 but no present index). */
#[test]
fn err015_reduceright_no_initial_value_holes() {
    diff("err015", |api| unsafe {
        rt(api, 15, "Array.prototype.reduceRight.call({length:3}, function(){})")
    });
}

/* ===================== jsboolean.c (rows 16..17) ========================= */

/* row 16: Bp_toString on a non-Boolean object. */
#[test]
fn err016_boolean_tostring_not_a_boolean() {
    diff("err016", |api| unsafe {
        rt(api, 16, "Boolean.prototype.toString.call({})")
    });
}

/* row 17: Bp_valueOf on a non-Boolean object. */
#[test]
fn err017_boolean_valueof_not_a_boolean() {
    diff("err017", |api| unsafe {
        rt(api, 17, "Boolean.prototype.valueOf.call({})")
    });
}

/* ===================== jsbuiltin.c (rows 18..19) ========================= */

/* row 18: Decode, escape sequence truncated at end of string. */
#[test]
fn err018_decode_truncated_escape_sequence() {
    diff("err018", |api| unsafe { rt(api, 18, "decodeURI(\"%\")") });
}

/* row 19: Decode, non hex digits after '%'. */
#[test]
fn err019_decode_invalid_escape_sequence() {
    diff("err019", |api| unsafe { rt(api, 19, "decodeURI(\"%zz\")") });
}

/* ===================== jscompile.c (rows 20..52) ========================= */

/* row 20: checkfutureword, future reserved word used as an identifier. */
#[test]
fn err020_future_reserved_word() {
    diff("err020", |api| unsafe { ct(api, 20, "class;") });
}

/* row 21: checkfutureword, strict mode future reserved word. */
#[test]
fn err021_strict_future_reserved_word() {
    diff("err021", |api| unsafe { ct(api, 21, "\"use strict\"; interface;") });
}

/* row 22: emitraw, value does not fit in js_Instruction (unsigned short).
 * The line number is emitted before every opcode, so a statement on line
 * 70001 overflows the instruction coding. */
#[test]
fn err022_instruction_coding_overflow() {
    diff("err022", |api| unsafe {
        let mut src = String::new();
        for _ in 0..70000 {
            src.push('\n');
        }
        src.push_str("var a = 1;");
        ct(api, 22, &src)
    });
}

/* row 23: addlocal, 'arguments' declared in strict mode. */
#[test]
fn err023_strict_redefine_arguments() {
    diff("err023", |api| unsafe { ct(api, 23, "\"use strict\"; var arguments;") });
}

/* row 24: addlocal, 'eval' declared in strict mode. */
#[test]
fn err024_strict_redefine_eval() {
    diff("err024", |api| unsafe { ct(api, 24, "\"use strict\"; var eval;") });
}

/* row 25: addlocal, 'eval' declared in non strict mode -> js_evalerror. */
#[test]
fn err025_var_eval_evalerror() {
    diff("err025", |api| unsafe { ct(api, 25, "var eval;") });
}

/* row 26: addlocal, duplicate formal parameter in strict mode. */
#[test]
fn err026_duplicate_formal_parameter() {
    diff("err026", |api| unsafe {
        ct(api, 26, "\"use strict\"; function f(a,a){}")
    });
}

/* row 27: emitlocal, assignment to 'arguments' in strict mode. */
#[test]
fn err027_arguments_read_only_in_strict_mode() {
    diff("err027", |api| unsafe { ct(api, 27, "\"use strict\"; arguments = 1;") });
}

/* row 28: emitlocal, assignment to 'eval' in strict mode. */
#[test]
fn err028_eval_read_only_in_strict_mode() {
    diff("err028", |api| unsafe { ct(api, 28, "\"use strict\"; eval = 1;") });
}

/* row 29: emitlocal, assignment to 'eval' in non strict mode -> js_evalerror. */
#[test]
fn err029_assign_eval_evalerror() {
    diff("err029", |api| unsafe { ct(api, 29, "eval = 1;") });
}

/* row 30: emitjumpto, backward jump target beyond 65535.
 * ~20000 statements of filler push the top of the while loop past the
 * js_Instruction range; emitjumpto(OP_JUMP, loop) is reached before the
 * forward label of the loop, so this is the emitjumpto site. */
#[test]
fn err030_emitjumpto_address_overflow() {
    diff("err030", |api| unsafe {
        let mut src = String::new();
        for _ in 0..20000 {
            src.push_str("x;");
        }
        src.push_str("while (x) { y; }");
        ct(api, 30, &src)
    });
}

/* row 31: labelto, forward jump address beyond 65535. The only jump in this
 * program is the JFALSE of the `if`, patched by label()/labelto() after the
 * ~20000 statement body. */
#[test]
fn err031_labelto_address_overflow() {
    diff("err031", |api| unsafe {
        let mut src = String::from("if (1) {");
        for _ in 0..20000 {
            src.push_str("x;");
        }
        src.push('}');
        ct(api, 31, &src)
    });
}

/* row 32: checkdup, duplicate property in an object literal (strict only). */
#[test]
fn err032_duplicate_property_in_object_literal() {
    diff("err032", |api| unsafe {
        ct(api, 32, "\"use strict\"; ({a:1,a:2});")
    });
}

/* row 33: cobject, "invalid property name in object initializer".
 * Only reachable if the parser produces a property name node that is not
 * AST_IDENTIFIER / EXP_STRING / EXP_NUMBER; propname() (jsparse.c:207) can
 * only produce those three, so the branch is dead. Nearest reachable trigger
 * is an invalid property name in an object initializer, which is rejected one
 * step earlier by identifiername() in the parser. */
#[test]
fn err033_invalid_property_name_in_object_initializer() {
    diff("err033", |api| unsafe { ct(api, 33, "({+:1});") });
}

/* row 34: cassign, invalid l-value. */
#[test]
fn err034_invalid_lvalue_in_assignment() {
    diff("err034", |api| unsafe { ct(api, 34, "1 = 2;") });
}

/* row 35: cassignforin, more than one loop variable. */
#[test]
fn err035_more_than_one_loop_variable() {
    diff("err035", |api| unsafe { ct(api, 35, "for (var a, b in x);") });
}

/* row 36: cassignforin, invalid l-value in for-in. */
#[test]
fn err036_invalid_lvalue_in_forin() {
    diff("err036", |api| unsafe { ct(api, 36, "for (1 in x);") });
}

/* row 37: cassignop1, invalid l-value in compound assignment. */
#[test]
fn err037_invalid_lvalue_in_assignop1() {
    diff("err037", |api| unsafe { ct(api, 37, "1 += 2;") });
}

/* row 38: cassignop2, invalid l-value. cassignop2 is only ever called after
 * cassignop1 on the *same* lhs node (cassignop / EXP_POSTINC / EXP_PREINC),
 * so any lhs that reaches the default of cassignop2 has already been rejected
 * by the identical check in cassignop1 (jscompile.c:464). Nearest reachable
 * trigger is a postfix increment of a non l-value, which errors with the same
 * message at the cassignop1 site. */
#[test]
fn err038_invalid_lvalue_in_assignop2() {
    diff("err038", |api| unsafe { ct(api, 38, "1++;") });
}

/* row 39: cdelete, delete of an unqualified name in strict mode. */
#[test]
fn err039_strict_delete_unqualified_name() {
    diff("err039", |api| unsafe { ct(api, 39, "\"use strict\"; delete x;") });
}

/* row 40: cdelete, invalid l-value in delete. */
#[test]
fn err040_invalid_lvalue_in_delete() {
    diff("err040", |api| unsafe { ct(api, 40, "delete 1;") });
}

/* row 41: cexp, "unknown expression type". Every EXP_* node type the parser
 * can produce in an expression position has a case in the cexp switch (only
 * EXP_PROP_VAL/GET/SET and the AST_/STM_ types are missing, and those are
 * never passed to cexp), so the default is dead. Nearest reachable trigger is
 * an expression the parser refuses to build at all. */
#[test]
fn err041_unknown_expression_type() {
    diff("err041", |api| unsafe { ct(api, 41, "var x = *;") });
}

/* row 42: ctrycatch, catch variable named 'arguments' in strict mode. */
#[test]
fn err042_trycatch_arguments_in_strict_mode() {
    diff("err042", |api| unsafe {
        ct(api, 42, "\"use strict\"; try {} catch (arguments) {}")
    });
}

/* row 43: ctrycatch, catch variable named 'eval' in strict mode. */
#[test]
fn err043_trycatch_eval_in_strict_mode() {
    diff("err043", |api| unsafe {
        ct(api, 43, "\"use strict\"; try {} catch (eval) {}")
    });
}

/* row 44: ctrycatchfinally, catch variable 'arguments' in strict mode. */
#[test]
fn err044_trycatchfinally_arguments_in_strict_mode() {
    diff("err044", |api| unsafe {
        ct(api, 44, "\"use strict\"; try {} catch (arguments) {} finally {}")
    });
}

/* row 45: ctrycatchfinally, catch variable 'eval' in strict mode. */
#[test]
fn err045_trycatchfinally_eval_in_strict_mode() {
    diff("err045", |api| unsafe {
        ct(api, 45, "\"use strict\"; try {} catch (eval) {} finally {}")
    });
}

/* row 46: cswitch, more than one default label. */
#[test]
fn err046_more_than_one_default_label() {
    diff("err046", |api| unsafe {
        ct(api, 46, "switch (1) { default: ; default: ; }")
    });
}

/* row 47: cstm STM_BREAK, labelled break target not found. */
#[test]
fn err047_break_label_not_found() {
    diff("err047", |api| unsafe { ct(api, 47, "break foo;") });
}

/* row 48: cstm STM_BREAK, unlabelled break outside loop/switch. */
#[test]
fn err048_unlabelled_break() {
    diff("err048", |api| unsafe { ct(api, 48, "break;") });
}

/* row 49: cstm STM_CONTINUE, labelled continue target not found. */
#[test]
fn err049_continue_label_not_found() {
    diff("err049", |api| unsafe { ct(api, 49, "continue foo;") });
}

/* row 50: cstm STM_CONTINUE, continue outside loop. */
#[test]
fn err050_continue_must_be_inside_loop() {
    diff("err050", |api| unsafe { ct(api, 50, "continue;") });
}

/* row 51: cstm STM_RETURN, return outside function. */
#[test]
fn err051_return_not_in_function() {
    diff("err051", |api| unsafe { ct(api, 51, "return 1;") });
}

/* row 52: cstm STM_WITH, 'with' in strict mode. */
#[test]
fn err052_with_in_strict_mode() {
    diff("err052", |api| unsafe { ct(api, 52, "\"use strict\"; with (x) {}") });
}

/* ===================== jsdate.c (rows 53..56) ============================ */

/* row 53: js_todate on a non-Date object. */
#[test]
fn err053_todate_not_a_date() {
    diff("err053", |api| unsafe {
        rt(api, 53, "Date.prototype.getTime.call({})")
    });
}

/* row 54: js_setdate on a non-Date object. */
#[test]
fn err054_setdate_not_a_date() {
    diff("err054", |api| unsafe {
        rt(api, 54, "Date.prototype.setTime.call({}, 0)")
    });
}

/* row 55: Dp_toISOString on a non finite time value. */
#[test]
fn err055_toisostring_invalid_date() {
    diff("err055", |api| unsafe { rt(api, 55, "new Date(NaN).toISOString()") });
}

/* row 56: Dp_toJSON, this.toISOString not callable. */
#[test]
fn err056_tojson_toisostring_not_a_function() {
    diff("err056", |api| unsafe {
        rt(api, 56, "Date.prototype.toJSON.call({})")
    });
}

/* ===================== jsdtoa.c (rows 57..58) ============================ */

/* row 57: minus(), assert(x.e == y.e). normalized_boundaries() always forces
 * mi.e = pl.e and multiply() adds the same exponent to both, so the two
 * exponents handed to minus() are always equal and this assert cannot fail.
 * Nearest reachable trigger is js_grisu2(0.0), which reaches minus() with
 * equal exponents and aborts on the *second* assert (row 58). */
#[test]
fn err057_minus_assert_exponent() {
    diff_stdout("err057", |api| unsafe {
        setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
        printf(b"== row %d ==\n\0".as_ptr() as *const c_char, 57);
        let mut buf = [0 as c_char; 64];
        let mut k: c_int = 0;
        let n = (api.js_grisu2)(0.0, buf.as_mut_ptr(), &mut k);
        /* not reached: the assert in minus() aborts first */
        p_int("n", n);
        p_int("k", k);
    });
}

/* row 58: minus(), assert(x.f >= y.f). For 0.0 the two normalized boundaries
 * have the same significand, then Wm.f++ / Wp.f-- makes Wm.f == Wp.f + 2, so
 * minus(Wp, Wm) trips assert(x.f >= y.f) -> SIGABRT. */
#[test]
fn err058_minus_assert_significand() {
    diff_stdout("err058", |api| unsafe {
        setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
        printf(b"== row %d ==\n\0".as_ptr() as *const c_char, 58);
        let mut buf = [0 as c_char; 64];
        let mut k: c_int = 0;
        let n = (api.js_grisu2)(0.0, buf.as_mut_ptr(), &mut k);
        /* not reached in C: assertion failure aborts the process */
        p_int("n", n);
        p_int("k", k);
        p_str("digits", buf.as_ptr());
    });
}

/* ===================== jserror.c (row 59) ================================ */

/* row 59: Ep_toString with a non object 'this'. */
#[test]
fn err059_error_tostring_not_an_object() {
    diff("err059", |api| unsafe {
        rt(api, 59, "Error.prototype.toString.call(1)")
    });
}

/* ===================== jsfunction.c (rows 60..63) ======================== */

/* row 60: Fp_toString on a non callable. */
#[test]
fn err060_function_tostring_not_a_function() {
    diff("err060", |api| unsafe {
        rt(api, 60, "Function.prototype.toString.call({})")
    });
}

/* row 61: Fp_apply on a non callable. */
#[test]
fn err061_apply_not_a_function() {
    diff("err061", |api| unsafe {
        rt(api, 61, "Function.prototype.apply.call({}, null, [])")
    });
}

/* row 62: Fp_call on a non callable. */
#[test]
fn err062_call_not_a_function() {
    diff("err062", |api| unsafe {
        rt(api, 62, "Function.prototype.call.call({})")
    });
}

/* row 63: Fp_bind on a non callable. */
#[test]
fn err063_bind_not_a_function() {
    diff("err063", |api| unsafe {
        rt(api, 63, "Function.prototype.bind.call({})")
    });
}

/* ===================== jsintern.c (row 64) ============================== */

/* row 64: jsS_newstringnode, strlen(string) > JS_STRLIMIT. Not reachable from
 * JavaScript without a >256 MiB source file, so js_intern() (the exported
 * entry point of the intern table) is called directly with a string of
 * 2^28 + 1 characters. The js_rangeerror is thrown with no try buffer
 * installed, so the panic handler reports and abort()s. */
#[test]
fn err064_intern_invalid_string_length() {
    diff("err064", |api| unsafe {
        setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
        printf(b"== row %d ==\n\0".as_ptr() as *const c_char, 64);
        let J = newstate(api, 0);
        let n: c_int = (1 << 28) + 2; /* 2^28 + 1 characters plus NUL */
        let p = (api.js_malloc)(J, n) as *mut c_char;
        p_ptr_nonnull("buf", p as *const c_void);
        std::ptr::write_bytes(p as *mut u8, b'x', (n - 1) as usize);
        *p.add((n - 1) as usize) = 0;
        let s = (api.js_intern)(J, p as *const c_char);
        /* not reached: js_rangeerror -> js_throw -> panic -> abort */
        p_ptr_nonnull("interned", s as *const c_void);
        (api.js_free)(J, p as *mut c_void);
        (api.js_freestate)(J);
    });
}
