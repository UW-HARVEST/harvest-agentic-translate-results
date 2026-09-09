//! Error-path differential tests for ERRORS.md rows 121..190
//! (json.c, jsparse.c, jsproperty.c, jsregexp.c, jsrun.c, jsstate.c,
//!  jsstring.c, jsvalue.c).
//!
//! Every test drives the C libmujs.so and the Rust libmujs.so through the
//! exact same invalid input and compares stdout+stderr bytes and exit status.

#![allow(dead_code, non_snake_case, unused_unsafe)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn memset(p: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

/* ------------------------------------------------------------------------- */
/* harness plumbing                                                          */
/* ------------------------------------------------------------------------- */

/// The Api currently under test; set at the top of every closure so the
/// `js_CFunction` callbacks below can reach the right library (this is the
/// equivalent of dtest/driver.c linking against one .so at a time).
static mut CUR: *const Api = null();

unsafe fn api() -> &'static Api {
    &*CUR
}

unsafe extern "C" fn cf_print(J: JS) {
    let a = api();
    let top = (a.js_gettop)(J);
    let mut i = 1;
    while i < top {
        let s = (a.js_tostring)(J, i);
        if i > 1 {
            putchar(b' ' as c_int);
        }
        printf(b"%s\0".as_ptr() as *const c_char, s);
        i += 1;
    }
    putchar(b'\n' as c_int);
    (a.js_pushundefined)(J);
}

unsafe fn state(a: &Api, strict: bool) -> JS {
    CUR = a as *const Api;
    let J = newstate(a, if strict { JS_STRICT } else { 0 });
    let n = cs("print");
    (a.js_newcfunction)(J, Some(cf_print), n.as_ptr(), 0);
    (a.js_setglobal)(J, n.as_ptr());
    J
}

unsafe fn banner(row: u32) {
    printf(b"--- row %d ---\n\0".as_ptr() as *const c_char, row as c_int);
    fflush(null_mut());
}

unsafe fn dostring(a: &Api, J: JS, label: &str, src: &str) {
    let s = cs(src);
    fflush(null_mut());
    let rc = (a.js_dostring)(J, s.as_ptr());
    fflush(null_mut());
    printf(
        b"%s: rc=%d top=%d\n\0".as_ptr() as *const c_char,
        cs(label).as_ptr(),
        rc,
        (a.js_gettop)(J),
    );
    fflush(null_mut());
}

/// Run `trigger` twice: once wrapped in try/catch (so the thrown error class
/// and message are printed), once raw (so the default uncaught-error report is
/// compared too).
unsafe fn jsrow(a: &Api, row: u32, strict: bool, trigger: &str) {
    banner(row);
    let J = state(a, strict);
    let wrapped = format!(
        "try {{ {} }} catch (e) {{ print(e.name + ': ' + e.message) }}",
        trigger
    );
    dostring(a, J, "caught", &wrapped);
    dostring(a, J, "raw", trigger);
    (a.js_freestate)(J);
}

/// Same, but the invalid input is produced by a C callback (`bad`) that misuses
/// the public C API directly.
unsafe fn capirow(a: &Api, row: u32, strict: bool, f: CFun, call: &str) {
    banner(row);
    let J = state(a, strict);
    let n = cs("bad");
    (a.js_newcfunction)(J, Some(f), n.as_ptr(), 0);
    (a.js_setglobal)(J, n.as_ptr());
    let wrapped = format!(
        "try {{ {}; print('no error') }} catch (e) {{ print(e.name + ': ' + e.message) }}",
        call
    );
    dostring(a, J, "caught", &wrapped);
    dostring(a, J, "raw", call);
    (a.js_freestate)(J);
}

/// Compile-time (parser) errors: report both the js_ploadstring return code and
/// the error object left on the stack, and the js_dostring default report.
unsafe fn parserow(a: &Api, row: u32, src: &str) {
    banner(row);
    let J = state(a, false);
    let fname = cs("errors_3.js");
    let s = cs(src);
    let rc = (a.js_ploadstring)(J, fname.as_ptr(), s.as_ptr());
    fflush(null_mut());
    p_int("ploadstring_rc", rc);
    p_str("stack_top", (a.js_tostring)(J, -1));
    (a.js_pop)(J, 1);
    p_int("top", (a.js_gettop)(J));
    dostring(a, J, "dostring", src);
    (a.js_freestate)(J);
}

/// malloc a NUL terminated buffer of exactly `len` 'x' characters.
unsafe fn bigstr(len: usize) -> *const c_char {
    let p = malloc(len + 1) as *mut c_char;
    assert!(!p.is_null(), "malloc failed");
    memset(p as *mut c_void, b'x' as c_int, len);
    *p.add(len) = 0;
    p as *const c_char
}

/// 8192 'a' characters: deep enough to blow REG_MAXREC (4096) in regexp.c's
/// recursive `match()` for the pattern /a*b/.
const LONG_A: &str = "var s='a'; while (s.length < 6000) s += s;";

/* ------------------------------------------------------------------------- */
/* json.c                                                                    */
/* ------------------------------------------------------------------------- */

/* row 121: jsonvalue (json.c:67) -- object key is not a string */
#[test]
fn err121_json_expected_string() {
    diff("err121", |a| unsafe {
        jsrow(a, 121, false, "JSON.parse('{1:2}')");
    });
}

/* row 122: jsonvalue (json.c:107) -- token cannot start a JSON value */
#[test]
fn err122_json_unexpected_token() {
    diff("err122", |a| unsafe {
        jsrow(a, 122, false, "JSON.parse(']')");
    });
}

/* row 123: fmtobject (json.c:261) -- cyclic object value */
#[test]
fn err123_json_cyclic_object() {
    diff("err123", |a| unsafe {
        jsrow(a, 123, false, "var o = {}; o.self = o; JSON.stringify(o)");
    });
}

/* row 124: fmtarray (json.c:297) -- cyclic object value through an array */
#[test]
fn err124_json_cyclic_array() {
    diff("err124", |a| unsafe {
        jsrow(a, 124, false, "var q = []; q[0] = q; JSON.stringify(q)");
    });
}

/* ------------------------------------------------------------------------- */
/* jsparse.c                                                                 */
/* ------------------------------------------------------------------------- */

/* row 125: semicolon (jsparse.c:153) -- missing ';' on the same line */
#[test]
fn err125_parse_expected_semicolon() {
    diff("err125", |a| unsafe {
        parserow(a, 125, "var a = 1 var b = 2");
    });
}

/* row 126: identifier (jsparse.c:166) */
#[test]
fn err126_parse_expected_identifier() {
    diff("err126", |a| unsafe {
        parserow(a, 126, "var 1;");
    });
}

/* row 127: identifiername (jsparse.c:183) -- after '.' */
#[test]
fn err127_parse_expected_identifier_or_keyword() {
    diff("err127", |a| unsafe {
        parserow(a, 127, "x.'y';");
    });
}

/* row 128: primary (jsparse.c:363) */
#[test]
fn err128_parse_unexpected_in_expression() {
    diff("err128", |a| unsafe {
        parserow(a, 128, "var x = ;");
    });
}

/* row 129: caseclause (jsparse.c:700) */
#[test]
fn err129_parse_unexpected_in_switch() {
    diff("err129", |a| unsafe {
        parserow(a, 129, "switch (x) { 1: break; }");
    });
}

/* row 130: forstatement (jsparse.c:751) -- for-var-statement */
#[test]
fn err130_parse_unexpected_in_for_var() {
    diff("err130", |a| unsafe {
        parserow(a, 130, "for (var i = 0) ;");
    });
}

/* row 131: forstatement (jsparse.c:770) */
#[test]
fn err131_parse_unexpected_in_for() {
    diff("err131", |a| unsafe {
        parserow(a, 131, "for (i = 0) ;");
    });
}

/* row 132: statement (jsparse.c:888) -- try without catch or finally */
#[test]
fn err132_parse_try_without_catch() {
    diff("err132", |a| unsafe {
        parserow(a, 132, "try { }");
    });
}

/* ------------------------------------------------------------------------- */
/* jsproperty.c                                                              */
/* ------------------------------------------------------------------------- */

/* row 133: jsV_setproperty (jsproperty.c:228) -- object is non-extensible */
#[test]
fn err133_object_non_extensible() {
    diff("err133", |a| unsafe {
        jsrow(
            a,
            133,
            true,
            "var o = {}; Object.preventExtensions(o); o.brandnew = 1",
        );
    });
}

/* row 134: jsV_nextiterator (jsproperty.c:303) -- not an iterator */
unsafe extern "C" fn cf134(J: JS) {
    let a = api();
    (a.js_newobject)(J);
    (a.js_nextiterator)(J, -1);
}

#[test]
fn err134_not_an_iterator() {
    diff("err134", |a| unsafe {
        capirow(a, 134, false, cf134, "bad()");
    });
}

/* row 135: jsV_resizearray (jsproperty.c:325) -- assert(!obj->u.a.simple)
 * The C library is built with assertions, so this aborts with SIGABRT. */
unsafe extern "C" fn cf135(J: JS) {
    let a = api();
    (a.js_newarray)(J);
    (a.js_pushnumber)(J, 10.0);
    (a.js_setindex)(J, -2, 0);
    (a.js_pushnumber)(J, 20.0);
    (a.js_setindex)(J, -2, 1);
    /* the array is still "simple" (flat) here */
    let obj = (a.js_toobject)(J, -1);
    (a.jsV_resizearray)(J, obj, 0);
}

#[test]
fn err135_resizearray_assert_simple() {
    diff_stdout("err135", |a| unsafe {
        banner(135);
        let J = state(a, false);
        let n = cs("bad");
        (a.js_newcfunction)(J, Some(cf135), n.as_ptr(), 0);
        (a.js_setglobal)(J, n.as_ptr());
        dostring(a, J, "raw", "bad()");
        p_line("after: should be unreachable (assert aborts)");
        (a.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* jsregexp.c                                                                */
/* ------------------------------------------------------------------------- */

/* row 136: js_newregexpx (jsregexp.c:38) -- regcomp failure */
#[test]
fn err136_regexp_compile_error() {
    diff("err136", |a| unsafe {
        jsrow(a, 136, false, "new RegExp('(')");
    });
}

/* row 137: js_RegExp_prototype_exec (jsregexp.c:77) -- regexec failed */
#[test]
fn err137_regexec_failed_exec() {
    diff("err137", |a| unsafe {
        jsrow(
            a,
            137,
            false,
            &format!("{} /a*b/.exec(s)", LONG_A),
        );
    });
}

/* row 138: Rp_test (jsregexp.c:126) -- regexec failed */
#[test]
fn err138_regexec_failed_test() {
    diff("err138", |a| unsafe {
        jsrow(
            a,
            138,
            false,
            &format!("{} /a*b/.test(s)", LONG_A),
        );
    });
}

/* row 139: jsB_new_RegExp (jsregexp.c:149) */
#[test]
fn err139_regexp_flags_from_regexp() {
    diff("err139", |a| unsafe {
        jsrow(a, 139, false, "new RegExp(/abc/, 'g')");
    });
}

/* row 140: jsB_new_RegExp (jsregexp.c:172) -- unknown flag */
#[test]
fn err140_regexp_invalid_flag_char() {
    diff("err140", |a| unsafe {
        jsrow(a, 140, false, "new RegExp('a', 'x')");
    });
}

/* row 141: jsB_new_RegExp (jsregexp.c:175) -- duplicate 'g' */
#[test]
fn err141_regexp_duplicate_g() {
    diff("err141", |a| unsafe {
        jsrow(a, 141, false, "new RegExp('a', 'gg')");
    });
}

/* row 142: jsB_new_RegExp (jsregexp.c:176) -- duplicate 'i' */
#[test]
fn err142_regexp_duplicate_i() {
    diff("err142", |a| unsafe {
        jsrow(a, 142, false, "new RegExp('a', 'ii')");
    });
}

/* row 143: jsB_new_RegExp (jsregexp.c:177) -- duplicate 'm' */
#[test]
fn err143_regexp_duplicate_m() {
    diff("err143", |a| unsafe {
        jsrow(a, 143, false, "new RegExp('a', 'mm')");
    });
}

/* ------------------------------------------------------------------------- */
/* jsrun.c                                                                   */
/* ------------------------------------------------------------------------- */

/* row 144: js_pushstring (jsrun.c:149) -- strlen(v) > JS_STRLIMIT (1<<28) */
unsafe extern "C" fn cf144(J: JS) {
    let a = api();
    let p = bigstr((1usize << 28) + 1);
    (a.js_pushstring)(J, p);
}

#[test]
fn err144_pushstring_strlimit() {
    diff("err144", |a| unsafe {
        capirow(a, 144, false, cf144, "bad()");
    });
}

/* row 145: js_pushlstring (jsrun.c:166) -- n > JS_STRLIMIT */
unsafe extern "C" fn cf145(J: JS) {
    let a = api();
    (a.js_pushlstring)(J, b"x\0".as_ptr() as *const c_char, (1 << 28) + 1);
}

#[test]
fn err145_pushlstring_strlimit() {
    diff("err145", |a| unsafe {
        capirow(a, 145, false, cf145, "bad()");
    });
}

/* row 146: js_toregexp (jsrun.c:373) -- not a regexp */
unsafe extern "C" fn cf146(J: JS) {
    let a = api();
    (a.js_pushnumber)(J, 42.0);
    (a.js_toregexp)(J, -1);
}

#[test]
fn err146_toregexp_not_a_regexp() {
    diff("err146", |a| unsafe {
        capirow(a, 146, false, cf146, "bad()");
    });
}

/* row 147: js_touserdata (jsrun.c:382) -- wrong tag */
unsafe extern "C" fn cf147(J: JS) {
    let a = api();
    (a.js_pushnull)(J); /* prototype slot consumed by js_newuserdata */
    (a.js_newuserdata)(
        J,
        b"Foo\0".as_ptr() as *const c_char,
        0x1234 as *mut c_void,
        None,
    );
    (a.js_touserdata)(J, -1, b"Bar\0".as_ptr() as *const c_char);
}

#[test]
fn err147_touserdata_wrong_tag() {
    diff("err147", |a| unsafe {
        capirow(a, 147, false, cf147, "bad()");
    });
}

/* row 148: jsR_tofunction (jsrun.c:393) -- reached through js_defaccessor */
#[test]
fn err148_tofunction_not_a_function() {
    diff("err148", |a| unsafe {
        jsrow(
            a,
            148,
            false,
            "Object.defineProperty({}, 'x', { get: 1 })",
        );
    });
}

/* row 149: js_pop (jsrun.c:408) -- stack underflow */
unsafe extern "C" fn cf149(J: JS) {
    let a = api();
    (a.js_pop)(J, 1000);
}

#[test]
fn err149_pop_stack_underflow() {
    diff("err149", |a| unsafe {
        capirow(a, 149, false, cf149, "bad()");
    });
}

/* row 150: js_remove (jsrun.c:416) -- stack error */
unsafe extern "C" fn cf150(J: JS) {
    let a = api();
    (a.js_remove)(J, 100);
}

#[test]
fn err150_remove_stack_error() {
    diff("err150", |a| unsafe {
        capirow(a, 150, false, cf150, "bad()");
    });
}

/* row 151: js_insert (jsrun.c:424) -- not implemented yet */
unsafe extern "C" fn cf151(J: JS) {
    let a = api();
    (a.js_insert)(J, 0);
}

#[test]
fn err151_insert_not_implemented() {
    diff("err151", |a| unsafe {
        capirow(a, 151, false, cf151, "bad()");
    });
}

/* row 152: js_replace (jsrun.c:431) -- stack error */
unsafe extern "C" fn cf152(J: JS) {
    let a = api();
    (a.js_replace)(J, 100);
}

#[test]
fn err152_replace_stack_error() {
    diff("err152", |a| unsafe {
        capirow(a, 152, false, cf152, "bad()");
    });
}

/* row 153: jsR_setarrayindex (jsrun.c:673) assert(obj->u.a.simple)
 * // row 153: unreachable directly; jsR_setarrayindex is `static` and both callers
 * (jsR_setproperty / jsR_setindex) only reach it when obj->u.a.simple is
 * already true.  Nearest reachable trigger is the flat-array fast path itself,
 * exercised here on a simple array plus the non-simple (unflattened) fallback. */
#[test]
fn err153_setarrayindex_assert_simple_unreachable() {
    diff("err153", |a| unsafe {
        jsrow(
            a,
            153,
            false,
            "var v = [1,2,3]; v[1] = 9; Object.defineProperty(v, '5', {value:1}); v[0] = 8; \
             print(v.length + ' ' + v[0] + ' ' + v[1])",
        );
    });
}

/* row 154: jsR_setarrayindex (jsrun.c:674) assert(k >= 0)
 * // row 154: unreachable directly; js_isarrayindex()/jsR_isindex() never yield a negative
 * index, so the assert cannot fire.  Nearest reachable trigger is a negative
 * "index" which is handled as an ordinary named property instead. */
#[test]
fn err154_setarrayindex_assert_nonneg_unreachable() {
    diff("err154", |a| unsafe {
        jsrow(
            a,
            154,
            false,
            "var v = []; v[-1] = 7; print(v.length + ' ' + v[-1] + ' ' + v['-1'])",
        );
    });
}

/* row 155: jsR_setarrayindex (jsrun.c:676) -- newlen > JS_ARRAYLIMIT (1<<26)
 * Reaching the check inside jsR_setarrayindex needs a *flat* array whose
 * flat_length is already JS_ARRAYLIMIT (that is 1G of flat array data), so we
 * use the equivalent, cheap JS_ARRAYLIMIT check in jsR_setproperty instead
 * (row 158) and additionally show the flat fast path near the limit.
 * // row 155: unreachable directly (needs a 1 GiB flat array); nearest
 * // reachable trigger is the identical "array too large" check on a.length. */
#[test]
fn err155_setarrayindex_array_too_large_proxy() {
    diff("err155", |a| unsafe {
        jsrow(
            a,
            155,
            false,
            "var v = []; v[0] = 1; v.length = 67108864; v[1] = 2; print(v.length)",
        );
    });
}

/* row 156: jsR_setarrayindex (jsrun.c:678) assert(newlen == flat_length + 1)
 * // row 156: unreachable directly; callers guarantee k <= flat_length, so newlen is at
 * most flat_length + 1.  Nearest reachable trigger is appending exactly one
 * past the end (the only case that grows a flat array). */
#[test]
fn err156_setarrayindex_assert_append_unreachable() {
    diff("err156", |a| unsafe {
        jsrow(
            a,
            156,
            false,
            "var v = []; v[0] = 1; v[1] = 2; v[3] = 4; print(v.length + ' ' + v[3])",
        );
    });
}

/* row 157: jsR_setproperty (jsrun.c:707) -- invalid array length */
#[test]
fn err157_invalid_array_length() {
    diff("err157", |a| unsafe {
        jsrow(a, 157, false, "var v = []; v.length = -1");
    });
}

/* row 158: jsR_setproperty (jsrun.c:709) -- array too large */
#[test]
fn err158_array_too_large() {
    diff("err158", |a| unsafe {
        jsrow(a, 158, false, "var v = []; v.length = 134217728");
    });
}

/* row 159: jsR_setproperty (jsrun.c:773) -- setter-less accessor */
#[test]
fn err159_only_has_a_getter() {
    diff("err159", |a| unsafe {
        jsrow(
            a,
            159,
            true,
            "var o = {}; Object.defineProperty(o, 'x', { get: function () { return 1 } }); o.x = 2",
        );
    });
}

/* row 160: jsR_setproperty (jsrun.c:783) -- transient object.
 * NOT reachable through OP_SETPROP/OP_SETPROP_S: those do
 *   obj = js_toobject(J, -2); transient = !js_isobject(J, -2);
 * and jsV_toobject() rewrites the stack slot in place, so `transient` is
 * always 0 there (the JS path below silently succeeds).  The only way in is
 * js_setproperty(), whose two arguments
 *   jsR_setproperty(J, js_toobject(J, idx), name, !js_isobject(J, idx))
 * are evaluated right-to-left by the C compiler. */
unsafe extern "C" fn cf160(J: JS) {
    let a = api();
    (a.js_pushstring)(J, b"abc\0".as_ptr() as *const c_char);
    (a.js_pushnumber)(J, 1.0);
    (a.js_setproperty)(J, -2, b"zzz\0".as_ptr() as *const c_char);
}

#[test]
fn err160_transient_object() {
    diff("err160", |a| unsafe {
        jsrow(a, 160, true, "var s = 'abc'; s.zzz = 1; print('js path: ' + s.zzz)");
        capirow(a, 160, true, cf160, "bad()");
    });
}

/* row 161: jsR_setproperty (jsrun.c:800) -- read-only */
#[test]
fn err161_setproperty_read_only() {
    diff("err161", |a| unsafe {
        jsrow(a, 161, true, "var s = 'abc'; s.length = 5");
    });
}

/* row 162: jsR_defproperty (jsrun.c:854) -- read-only */
#[test]
fn err162_defproperty_read_only() {
    diff("err162", |a| unsafe {
        jsrow(
            a,
            162,
            true,
            "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
             Object.defineProperty(o, 'x', { value: 2 })",
        );
    });
}

/* row 163: jsR_defproperty (jsrun.c:860) -- non-configurable getter */
#[test]
fn err163_defproperty_getter_non_configurable() {
    diff("err163", |a| unsafe {
        jsrow(
            a,
            163,
            true,
            "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
             Object.defineProperty(o, 'x', { get: function () { return 2 } })",
        );
    });
}

/* row 164: jsR_defproperty (jsrun.c:866) -- non-configurable setter */
#[test]
fn err164_defproperty_setter_non_configurable() {
    diff("err164", |a| unsafe {
        jsrow(
            a,
            164,
            true,
            "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
             Object.defineProperty(o, 'x', { set: function (v) { } })",
        );
    });
}

/* row 165: jsR_defproperty (jsrun.c:875) -- read-only or non-configurable
 * (throw == 1 from js_defproperty, so it fires in non-strict mode too) */
#[test]
fn err165_defproperty_array_length() {
    diff("err165", |a| unsafe {
        jsrow(
            a,
            165,
            false,
            "Object.defineProperty([], 'length', { value: 1 })",
        );
    });
}

/* row 166: jsR_delproperty (jsrun.c:921) -- non-configurable */
#[test]
fn err166_delproperty_non_configurable() {
    diff("err166", |a| unsafe {
        jsrow(a, 166, true, "var v = [1]; delete v.length");
    });
}

/* row 167: js_setvar (jsrun.c:1127) -- read-only variable */
#[test]
fn err167_setvar_read_only() {
    diff("err167", |a| unsafe {
        jsrow(a, 167, true, "undefined = 1");
    });
}

/* row 168: js_setvar (jsrun.c:1133) -- assignment to undeclared variable */
#[test]
fn err168_setvar_undeclared() {
    diff("err168", |a| unsafe {
        jsrow(a, 168, true, "notDeclaredAnywhere = 1");
    });
}

/* row 169: js_delvar (jsrun.c:1145) -- '%s' is non-configurable
 * // row 169: unreachable directly; `delete <identifier>` is rejected at
 * // compile time in strict mode (jscompile.c cdelete) and J->strict always
 * // mirrors the running function's strict flag, so js_delvar can never see
 * // J->strict != 0.  Nearest reachable triggers shown: the compile-time
 * // rejection in strict mode, and the silent non-strict path. */
#[test]
fn err169_delvar_non_configurable_unreachable() {
    diff("err169", |a| unsafe {
        banner(169);
        let J = state(a, true);
        dostring(a, J, "strict_compile", "delete undefined;");
        (a.js_freestate)(J);
        let J2 = state(a, false);
        dostring(
            a,
            J2,
            "nonstrict_runtime",
            "print(delete undefined); print(typeof undefined)",
        );
        (a.js_freestate)(J2);
    });
}

/* row 170: jsR_pushtrace (jsrun.c:1290) -- call stack overflow */
#[test]
fn err170_call_stack_overflow() {
    diff("err170", |a| unsafe {
        jsrow(a, 170, false, "function rec() { return rec() } rec()");
    });
}

/* row 171: js_call (jsrun.c:1304) -- negative argument count */
unsafe extern "C" fn cf171(J: JS) {
    let a = api();
    (a.js_pushundefined)(J);
    (a.js_pushundefined)(J);
    (a.js_call)(J, -1);
}

#[test]
fn err171_call_negative_argc() {
    diff("err171", |a| unsafe {
        capirow(a, 171, false, cf171, "bad()");
    });
}

/* row 172: js_call (jsrun.c:1307) -- not callable */
#[test]
fn err172_call_not_callable() {
    diff("err172", |a| unsafe {
        jsrow(a, 172, false, "var x = 1; x()");
    });
}

/* row 173: js_construct (jsrun.c:1341) -- not callable */
#[test]
fn err173_construct_not_callable() {
    diff("err173", |a| unsafe {
        jsrow(a, 173, false, "var x = 1; new x()");
    });
}

/* row 174: js_endtry (jsrun.c:1461) -- exception stack underflow.
 * There is no try frame at all, so the thrown Error reaches js_atpanic's
 * default handler ("uncaught exception") and abort()s. */
#[test]
fn err174_endtry_underflow() {
    diff("err174", |a| unsafe {
        banner(174);
        let J = state(a, false);
        p_int("gettop_before", (a.js_gettop)(J));
        fflush(null_mut());
        (a.js_endtry)(J);
        p_line("after: should be unreachable (panic aborts)");
        (a.js_freestate)(J);
    });
}

/* row 175: jsR_run OP_GETLOCAL (jsrun.c:1673) -- '%s' is not defined
 * // row 175: unreachable directly; OP_GETLOCAL is only emitted for names in
 * // the function's own vartab, and jsR_callfunction js_initvar()s every one of
 * // them before jsR_run starts, so js_hasvar always succeeds.  Nearest
 * // reachable trigger is the identical message from OP_GETVAR inside a
 * // non-lightweight (try/catch bearing) function. */
#[test]
fn err175_getlocal_not_defined_unreachable() {
    diff("err175", |a| unsafe {
        jsrow(
            a,
            175,
            false,
            "function heavy() { try { return missingLocalName } finally { } } heavy()",
        );
    });
}

/* row 176: jsR_run OP_GETVAR (jsrun.c:1698) -- '%s' is not defined */
#[test]
fn err176_getvar_not_defined() {
    diff("err176", |a| unsafe {
        jsrow(a, 176, false, "noSuchGlobalName");
    });
}

/* row 177: jsR_run OP_IN (jsrun.c:1721) -- operand to 'in' is not an object */
#[test]
fn err177_in_operand_not_object() {
    diff("err177", |a| unsafe {
        jsrow(a, 177, false, "'a' in 'b'");
    });
}

/* ------------------------------------------------------------------------- */
/* jsstate.c                                                                 */
/* ------------------------------------------------------------------------- */

/* row 178: js_newstate (jsstate.c:191) assert(sizeof(js_Value) == 16)
 * // row 178: unreachable directly; the condition is a compile-time property
 * // of the struct layout, so the assert can never fire at run time.  Nearest
 * // reachable jsstate.c error paths shown instead: js_newstate succeeding and
 * // js_ploadstring returning 1, plus the JS_TRYLIMIT (64) js_ptry path. */
unsafe extern "C" fn cf178(J: JS) {
    let a = api();
    let rc = (a.js_ploadstring)(
        J,
        b"deep.js\0".as_ptr() as *const c_char,
        b"1+1\0".as_ptr() as *const c_char,
    );
    printf(b"ptry_ploadstring_rc=%d\n\0".as_ptr() as *const c_char, rc);
    p_str("ptry_stack_top", (a.js_tostring)(J, -1));
    (a.js_pop)(J, 1);
    (a.js_pushundefined)(J);
}

#[test]
fn err178_newstate_value_size_assert_unreachable() {
    diff("err178", |a| unsafe {
        banner(178);
        let J = state(a, false);
        p_ptr_nonnull("newstate", J);
        let fname = cs("proxy.js");
        let bad = cs("var = ;");
        let rc = (a.js_ploadstring)(J, fname.as_ptr(), bad.as_ptr());
        p_int("ploadstring_rc", rc);
        p_str("ploadstring_err", (a.js_tostring)(J, -1));
        (a.js_pop)(J, 1);

        /* JS_TRYLIMIT = 64: js_dostring's own js_try is #1, so 63 nested JS
         * try statements bring J->trytop to exactly 64 and js_ptry fires. */
        let n = cs("bad");
        (a.js_newcfunction)(J, Some(cf178), n.as_ptr(), 0);
        (a.js_setglobal)(J, n.as_ptr());
        let mut src = String::new();
        for _ in 0..63 {
            src.push_str("try{");
        }
        src.push_str("bad();");
        for _ in 0..63 {
            src.push_str("}catch(e){print('caught '+e)}");
        }
        dostring(a, J, "trylimit", &src);
        (a.js_freestate)(J);
    });
}

/* row 179: js_newstate (jsstate.c:192) assert(soffsetof(js_Value, t.type) == 15)
 * // row 179: unreachable directly (compile-time layout property).  Nearest
 * // reachable jsstate.c paths: js_pcall and js_pconstruct returning 1. */
#[test]
fn err179_newstate_type_offset_assert_unreachable() {
    diff("err179", |a| unsafe {
        banner(179);
        let J = state(a, false);
        /* js_pcall on a non-callable */
        (a.js_pushnumber)(J, 1.0);
        (a.js_pushundefined)(J);
        let rc = (a.js_pcall)(J, 0);
        p_int("pcall_rc", rc);
        p_str("pcall_err", (a.js_tostring)(J, -1));
        (a.js_pop)(J, 1);
        p_int("top_after_pcall", (a.js_gettop)(J));
        /* js_pconstruct on a non-callable.  js_pconstruct computes
         * savetop = TOP - n - 2, so keep one spare slot below the callee. */
        (a.js_pushundefined)(J);
        (a.js_pushnumber)(J, 1.0);
        let rc2 = (a.js_pconstruct)(J, 0);
        p_int("pconstruct_rc", rc2);
        p_str("pconstruct_err", (a.js_tostring)(J, -1));
        (a.js_pop)(J, 1);
        p_int("top_after_pconstruct", (a.js_gettop)(J));
        fflush(null_mut());
        (a.js_freestate)(J);
    });
}

/* ------------------------------------------------------------------------- */
/* jsstring.c                                                                */
/* ------------------------------------------------------------------------- */

/* row 180: js_doregexec (jsstring.c:9) -- regexec failed */
#[test]
fn err180_doregexec_failed() {
    diff("err180", |a| unsafe {
        jsrow(a, 180, false, &format!("{} s.search(/a*b/)", LONG_A));
    });
}

/* row 181: checkstring (jsstring.c:16) -- called on null or undefined */
#[test]
fn err181_checkstring_non_coercible() {
    diff("err181", |a| unsafe {
        jsrow(
            a,
            181,
            false,
            "String.prototype.charAt.call(undefined, 0)",
        );
    });
}

/* row 182: Sp_toString (jsstring.c:108) -- not a string */
#[test]
fn err182_sp_tostring_not_a_string() {
    diff("err182", |a| unsafe {
        jsrow(a, 182, false, "String.prototype.toString.call(1)");
    });
}

/* row 183: Sp_valueOf (jsstring.c:115) -- not a string */
#[test]
fn err183_sp_valueof_not_a_string() {
    diff("err183", |a| unsafe {
        jsrow(a, 183, false, "String.prototype.valueOf.call(1)");
    });
}

/* row 184: Sp_concat (jsstring.c:163) -- 1 + strlen(this) > JS_STRLIMIT.
 * `this` is pushed with js_pushliteral (no length check there) so exactly
 * JS_STRLIMIT bytes reach Sp_concat's first check. */
unsafe extern "C" fn cf184(J: JS) {
    let a = api();
    let big = bigstr(1usize << 28);
    (a.js_copy)(J, 1); /* String.prototype.concat */
    (a.js_pushliteral)(J, big); /* this */
    (a.js_pushliteral)(J, b"y\0".as_ptr() as *const c_char);
    (a.js_call)(J, 1);
}

#[test]
fn err184_concat_strlimit_first() {
    diff("err184", |a| unsafe {
        capirow(a, 184, false, cf184, "bad(String.prototype.concat)");
    });
}

/* row 185: Sp_concat (jsstring.c:171) -- limit crossed while appending args */
unsafe extern "C" fn cf185(J: JS) {
    let a = api();
    let big = bigstr((1usize << 28) - 1);
    (a.js_copy)(J, 1); /* String.prototype.concat */
    (a.js_pushliteral)(J, big); /* this: n == JS_STRLIMIT, passes */
    (a.js_pushliteral)(J, b"yy\0".as_ptr() as *const c_char);
    (a.js_call)(J, 1);
}

#[test]
fn err185_concat_strlimit_loop() {
    diff("err185", |a| unsafe {
        capirow(a, 185, false, cf185, "bad(String.prototype.concat)");
    });
}

/* ------------------------------------------------------------------------- */
/* jsvalue.c                                                                 */
/* ------------------------------------------------------------------------- */

/* row 186: jsV_toprimitive (jsvalue.c:144) -- cannot convert object to primitive */
#[test]
fn err186_toprimitive_strict() {
    diff("err186", |a| unsafe {
        jsrow(
            a,
            186,
            true,
            "var o = { valueOf: 1, toString: 2 }; '' + o",
        );
    });
}

/* row 187: jsV_toobject (jsvalue.c:401) -- undefined */
#[test]
fn err187_toobject_undefined() {
    diff("err187", |a| unsafe {
        jsrow(a, 187, false, "undefined.x");
    });
}

/* row 188: jsV_toobject (jsvalue.c:402) -- null */
#[test]
fn err188_toobject_null() {
    diff("err188", |a| unsafe {
        jsrow(a, 188, false, "null.x");
    });
}

/* row 189: js_instanceof (jsvalue.c:579) -- invalid operand */
#[test]
fn err189_instanceof_invalid_operand() {
    diff("err189", |a| unsafe {
        jsrow(a, 189, false, "1 instanceof 2");
    });
}

/* row 190: js_instanceof (jsvalue.c:586) -- 'prototype' is not an object */
#[test]
fn err190_instanceof_prototype_not_object() {
    diff("err190", |a| unsafe {
        jsrow(
            a,
            190,
            false,
            "function F() { } F.prototype = 1; ({}) instanceof F",
        );
    });
}
