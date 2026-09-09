//! Error-path differential tests, ERRORS.md rows 65..120.
//!
//! Covered surfaces:
//!   * rows  65.. 90 — `jslex.c` lexer / JSON lexer `jsY_error` sites
//!   * rows  91..102 — `jsnumber.c` `Np_*` type/range errors
//!   * rows 103..120 — `jsobject.c` `O_*` type errors
//!
//! Every test drives the *exact* invalid input named in the row and prints
//! whatever the library reports, so that both libmujs.so builds must agree
//! byte for byte (and on exit status).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::ptr::null_mut;

/* ------------------------------------------------------------------------- */
/* plumbing                                                                  */
/* ------------------------------------------------------------------------- */

/// The Api currently under test — needed because a `js_CFunction` is a plain
/// `extern "C"` fn pointer and cannot capture. Same trick as `dtest/driver.c`,
/// which links against one library at a time.
static mut CUR: *const Api = std::ptr::null();

unsafe extern "C" fn jsB_print(J: JS) {
    let api = &*CUR;
    let top = (api.js_gettop)(J);
    let mut i = 1;
    while i < top {
        let s = (api.js_tostring)(J, i);
        if i > 1 {
            libc::putchar(b' ' as c_int);
        }
        libc::printf(b"%s\0".as_ptr() as *const c_char, s);
        i += 1;
    }
    libc::putchar(b'\n' as c_int);
    (api.js_pushundefined)(J);
}

/// Report hook: routed through stdout (printf) so that *all* of a child's
/// output shares one buffer and therefore one deterministic ordering.
unsafe extern "C" fn myreport(_J: JS, msg: *const c_char) {
    libc::printf(b"[report] %s\n\0".as_ptr() as *const c_char, msg);
}

unsafe fn mkstate(api: &Api) -> JS {
    CUR = api as *const Api;
    let J = newstate(api, 0);
    (api.js_setreport)(J, Some(myreport));
    (api.js_newcfunction)(J, Some(jsB_print), cs("print").as_ptr(), 0);
    (api.js_setglobal)(J, cs("print").as_ptr());
    J
}

unsafe fn hdr(row: c_int, what: &str) {
    libc::printf(
        b"== row %d %s\n\0".as_ptr() as *const c_char,
        row,
        cs(what).as_ptr(),
    );
}

/// (a) uncaught: `js_dostring` — the installed report receives the message.
unsafe fn dostring(api: &Api, src: &str) {
    let J = mkstate(api);
    let rc = (api.js_dostring)(J, cs(src).as_ptr());
    p_int("rc", rc);
    p_int("top", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

/// (b) `js_ploadstring` — returns 1 and leaves the error object on the stack,
/// so the lexer/parser message can be printed directly.
unsafe fn ploadstring(api: &Api, src: &str) {
    let J = mkstate(api);
    let rc = (api.js_ploadstring)(J, cs("[file]").as_ptr(), cs(src).as_ptr());
    p_int("prc", rc);
    if rc != 0 {
        /* read `name` first: js_tostring() converts the slot in place. */
        (api.js_getproperty)(J, -1, cs("name").as_ptr());
        p_str("pname", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
        p_str("perr", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
    }
    p_int("top", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

/// (c) C-API level: compile then `js_pcall` — the thrown object comes back on
/// the stack, no JS `try` needed.
unsafe fn pcallstring(api: &Api, src: &str) {
    let J = mkstate(api);
    let lrc = (api.js_ploadstring)(J, cs("[expr]").as_ptr(), cs(src).as_ptr());
    p_int("loadrc", lrc);
    if lrc == 0 {
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("pcallrc", rc);
        p_str("pcallres", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
    } else {
        p_str("loaderr", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
    }
    p_int("top", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

/// (d) inside JavaScript `try`/`catch`, printing `e.name + ": " + e.message`.
unsafe fn caught(api: &Api, expr: &str) {
    let src = format!(
        "try {{ {} }} catch (e) {{ print(e.name + \": \" + e.message) }}",
        expr
    );
    let J = mkstate(api);
    let rc = (api.js_dostring)(J, cs(&src).as_ptr());
    p_int("rc", rc);
    p_int("top", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

/// Lexer rows: the malformed token is rejected at load time, so `try`/`catch`
/// in the same script cannot see it — drive `js_dostring` and `js_ploadstring`.
unsafe fn lexrow(api: &Api, row: c_int, src: &str) {
    hdr(row, "dostring");
    dostring(api, src);
    hdr(row, "ploadstring");
    ploadstring(api, src);
}

/// Runtime rows: uncaught + `js_pcall` + JS `try`/`catch`.
unsafe fn rtrow(api: &Api, row: c_int, expr: &str) {
    hdr(row, "dostring");
    dostring(api, expr);
    hdr(row, "pcall");
    pcallstring(api, expr);
    hdr(row, "caught");
    caught(api, expr);
}

/* ------------------------------------------------------------------------- */
/* jslex.c — rows 65..82                                                     */
/* ------------------------------------------------------------------------- */

/// row 65: `jsY_unescape` (jslex.c:192) — "unexpected escape sequence".
/// A `\` in identifier position that is not `\uXXXX`.
/// Also driven through the raw lexer (`jsY_initlex` + `jsY_lex`): the throw
/// escapes the library with no try frame, so `js_throw` -> panic -> abort().
#[test]
fn err065_unexpected_escape_sequence() {
    diff("err065", |api| unsafe {
        lexrow(api, 65, "var \\q = 1;");
        // and a \u with too few hex digits
        hdr(65, "dostring-u");
        dostring(api, "var \\u12 = 1;");
        hdr(65, "ploadstring-u");
        ploadstring(api, "var \\u12 = 1;");
    });
    diff("err065_rawlex", |api| unsafe {
        hdr(65, "rawlex(aborts)");
        let J = mkstate(api);
        libc::fflush(null_mut());
        (api.jsY_initlex)(J, cs("[lex]").as_ptr(), cs("var \\q = 1;").as_ptr());
        loop {
            let t = (api.jsY_lex)(J);
            p_int("tok", t);
            libc::fflush(null_mut());
            if t == 0 {
                break;
            }
        }
        (api.js_freestate)(J);
    });
}

/// row 66: `lexhex` (jslex.c:255) — "malformed hexadecimal number".
#[test]
fn err066_malformed_hexadecimal_number() {
    diff("err066", |api| unsafe {
        lexrow(api, 66, "var a = 0x;");
        hdr(66, "dostring-Z");
        dostring(api, "var a = 0xZ;");
        hdr(66, "ploadstring-Z");
        ploadstring(api, "var a = 0xX;");
    });
}

/// row 67: `lexinteger` (jslex.c:269) — "malformed number".
// row 67: unreachable directly; lexinteger()/lexfraction()/lexexponent() live
// inside the `#if 0` block of jslex.c (lines 263..339) and are not compiled in
// (the live lexnumber() at line 341 scans digits inline and hands the text to
// js_strtod, so "malformed number" can never be produced). Nearest reachable
// trigger for a malformed numeric literal is lexhex()'s
// "malformed hexadecimal number" (jslex.c:255) plus the live
// "missing exponent" path (jslex.c:377).
#[test]
fn err067_malformed_number_unreachable() {
    diff("err067", |api| unsafe {
        lexrow(api, 67, "var a = 0X;");
        hdr(67, "nearest-live-exponent");
        lexrow(api, 67, "var a = 1e+;");
    });
}

/// row 68: `lexnumber` (jslex.c:312) — "number with leading zero".
// row 68: this exact site is in the `#if 0` copy of lexnumber (jslex.c:263..339)
// and is not compiled in; the live copy raises the identical message at
// jslex.c:351, which is what the input below reaches (see also row 70).
#[test]
fn err068_number_with_leading_zero_dead_copy() {
    diff("err068", |api| unsafe {
        lexrow(api, 68, "var a = 01;");
    });
}

/// row 69: `lexnumber` (jslex.c:333) — "number with letter suffix".
// row 69: `#if 0` copy again (jslex.c:263..339); the live copy raises the
// identical message at jslex.c:381 which the input below reaches (row 72).
#[test]
fn err069_number_with_letter_suffix_dead_copy() {
    diff("err069", |api| unsafe {
        lexrow(api, 69, "var a = 1abc;");
    });
}

/// row 70: `lexnumber` (jslex.c:351) — "number with leading zero".
#[test]
fn err070_number_with_leading_zero() {
    diff("err070", |api| unsafe {
        lexrow(api, 70, "var a = 0777;");
        hdr(70, "09");
        lexrow(api, 70, "var a = 09;");
    });
}

/// row 71: `lexnumber` (jslex.c:377) — "missing exponent".
#[test]
fn err071_missing_exponent() {
    diff("err071", |api| unsafe {
        lexrow(api, 71, "var a = 1e;");
        hdr(71, "E-minus");
        lexrow(api, 71, "var a = 2E-;");
        hdr(71, "dot-e");
        lexrow(api, 71, "var a = 1.5e+;");
    });
}

/// row 72: `lexnumber` (jslex.c:381) — "number with letter suffix".
#[test]
fn err072_number_with_letter_suffix() {
    diff("err072", |api| unsafe {
        lexrow(api, 72, "var a = 3g;");
        hdr(72, "hex-suffix");
        lexrow(api, 72, "var a = 1.5$;");
    });
}

/// row 73: `lexescape` (jslex.c:399) — "unterminated escape sequence".
/// String literal whose last character is a backslash, then EOF.
#[test]
fn err073_unterminated_escape_sequence() {
    diff("err073", |api| unsafe {
        lexrow(api, 73, "var a = \"abc\\");
    });
}

/// row 74: `lexstring` (jslex.c:440) — "string not terminated".
#[test]
fn err074_string_not_terminated() {
    diff("err074", |api| unsafe {
        hdr(74, "eof");
        lexrow(api, 74, "var a = \"abc");
        hdr(74, "newline");
        lexrow(api, 74, "var a = 'abc\nvar b = 2;\n");
    });
}

/// row 75: `lexstring` (jslex.c:443) — "malformed escape sequence".
/// `lexescape` returns 1 on a bad `\x`/`\u` hex digit.
#[test]
fn err075_malformed_escape_sequence() {
    diff("err075", |api| unsafe {
        hdr(75, "bad-x");
        lexrow(api, 75, "var a = \"\\xZZ\";");
        hdr(75, "bad-u");
        lexrow(api, 75, "var a = \"\\u12G4\";");
    });
}

/// row 76: `lexregexp` (jslex.c:490) — "regular expression not terminated".
#[test]
fn err076_regexp_not_terminated_body() {
    diff("err076", |api| unsafe {
        hdr(76, "eof");
        lexrow(api, 76, "var a = /abc");
        hdr(76, "newline");
        lexrow(api, 76, "var a = /abc\nvar b = 2;\n");
    });
}

/// row 77: `lexregexp` (jslex.c:497) — "regular expression not terminated"
/// (EOF/newline directly after an escaping backslash).
#[test]
fn err077_regexp_not_terminated_escape() {
    diff("err077", |api| unsafe {
        hdr(77, "eof");
        lexrow(api, 77, "var a = /abc\\");
        hdr(77, "newline");
        lexrow(api, 77, "var a = /abc\\\nvar b = 2;\n");
    });
}

/// row 78: `lexregexp` (jslex.c:521) — "illegal flag in regular expression: %c".
#[test]
fn err078_illegal_regexp_flag() {
    diff("err078", |api| unsafe {
        lexrow(api, 78, "var a = /abc/x;");
        hdr(78, "flag-y");
        lexrow(api, 78, "var a = /abc/gy;");
    });
}

/// row 79: `lexregexp` (jslex.c:525) — "duplicated flag in regular expression".
#[test]
fn err079_duplicated_regexp_flag() {
    diff("err079", |api| unsafe {
        lexrow(api, 79, "var a = /abc/gg;");
        hdr(79, "ii");
        lexrow(api, 79, "var a = /abc/ii;");
        hdr(79, "mm");
        lexrow(api, 79, "var a = /abc/mgm;");
    });
}

/// row 80: `jsY_lexx` (jslex.c:574) — "multi-line comment not terminated".
#[test]
fn err080_comment_not_terminated() {
    diff("err080", |api| unsafe {
        lexrow(api, 80, "var a = 1; /* not closed");
        hdr(80, "star-eof");
        lexrow(api, 80, "var a = 1; /* not closed *");
    });
}

/// row 81: `jsY_lexx` (jslex.c:728) — "unexpected character: '%c'".
#[test]
fn err081_unexpected_character_printable() {
    diff("err081", |api| unsafe {
        lexrow(api, 81, "var a = @;");
        hdr(81, "hash");
        lexrow(api, 81, "var a = #;");
        hdr(81, "backtick");
        lexrow(api, 81, "var a = `x`;");
    });
}

/// row 82: `jsY_lexx` (jslex.c:729) — "unexpected character: \\u%04X".
/// 0x01 is neither whitespace, newline nor an identifier start.
#[test]
fn err082_unexpected_character_escaped() {
    diff("err082", |api| unsafe {
        lexrow(api, 82, "var a = \u{1};");
        hdr(82, "u007F");
        lexrow(api, 82, "var a = \u{7f};");
        hdr(82, "non-ascii");
        lexrow(api, 82, "var a = \u{2044};");
    });
}

/* ------------------------------------------------------------------------- */
/* jslex.c JSON lexer — rows 83..90 (reached through JSON.parse)             */
/* ------------------------------------------------------------------------- */

/// row 83: `lexjsonnumber` (jslex.c:760) — "unexpected non-digit".
#[test]
fn err083_json_unexpected_non_digit() {
    diff("err083", |api| unsafe {
        rtrow(api, 83, "JSON.parse('-x')");
        hdr(83, "lone-minus");
        rtrow(api, 83, "JSON.parse('-')");
    });
}

/// row 84: `lexjsonnumber` (jslex.c:767) — "missing digits after decimal point".
#[test]
fn err084_json_missing_digits_after_point() {
    diff("err084", |api| unsafe {
        rtrow(api, 84, "JSON.parse('1.')");
        hdr(84, "1-dot-x");
        rtrow(api, 84, "JSON.parse('1.x')");
    });
}

/// row 85: `lexjsonnumber` (jslex.c:777) — "missing digits after exponent indicator".
#[test]
fn err085_json_missing_digits_after_exponent() {
    diff("err085", |api| unsafe {
        rtrow(api, 85, "JSON.parse('1e')");
        hdr(85, "1e-plus");
        rtrow(api, 85, "JSON.parse('1e+')");
    });
}

/// row 86: `lexjsonescape` (jslex.c:791) — "invalid escape sequence".
#[test]
fn err086_json_invalid_escape_sequence() {
    diff("err086", |api| unsafe {
        rtrow(api, 86, "JSON.parse('\"\\\\q\"')");
        hdr(86, "backslash-x");
        rtrow(api, 86, "JSON.parse('\"\\\\x41\"')");
    });
}

/// row 87: `lexjsonstring` (jslex.c:820) — "unterminated string".
#[test]
fn err087_json_unterminated_string() {
    diff("err087", |api| unsafe {
        rtrow(api, 87, "JSON.parse('\"abc')");
        hdr(87, "empty-quote");
        rtrow(api, 87, "JSON.parse('\"')");
    });
}

/// row 88: `lexjsonstring` (jslex.c:822) — "invalid control character in string".
#[test]
fn err088_json_control_character_in_string() {
    diff("err088", |api| unsafe {
        rtrow(api, 88, "JSON.parse('\"a\\u0001b\"')");
        hdr(88, "tab");
        rtrow(api, 88, "JSON.parse('\"a\\u0009b\"')");
    });
}

/// row 89: `jsY_lexjson` (jslex.c:878) — "unexpected character: '%c'".
#[test]
fn err089_json_unexpected_character_printable() {
    diff("err089", |api| unsafe {
        rtrow(api, 89, "JSON.parse('@')");
        hdr(89, "quote-single");
        rtrow(api, 89, "JSON.parse(\"'a'\")");
    });
}

/// row 90: `jsY_lexjson` (jslex.c:879) — "unexpected character: \\u%04X".
#[test]
fn err090_json_unexpected_character_escaped() {
    diff("err090", |api| unsafe {
        rtrow(api, 90, "JSON.parse('\\u0001')");
        hdr(90, "non-ascii");
        rtrow(api, 90, "JSON.parse('\\u2044')");
    });
}

/* ------------------------------------------------------------------------- */
/* jsnumber.c — rows 91..102                                                 */
/* ------------------------------------------------------------------------- */

/// row 91: `Np_valueOf` (jsnumber.c:22) — "not a number".
#[test]
fn err091_number_valueof_not_a_number() {
    diff("err091", |api| unsafe {
        rtrow(api, 91, "Number.prototype.valueOf.call({})");
        hdr(91, "on-string");
        rtrow(api, 91, "Number.prototype.valueOf.call('x')");
    });
}

/// row 92: `Np_toString` (jsnumber.c:33) — "not a number".
#[test]
fn err092_number_tostring_not_a_number() {
    diff("err092", |api| unsafe {
        rtrow(api, 92, "Number.prototype.toString.call({})");
        hdr(92, "with-radix");
        rtrow(api, 92, "Number.prototype.toString.call([], 16)");
    });
}

/// row 93: `Np_toString` (jsnumber.c:40) — "invalid radix".
#[test]
fn err093_number_tostring_invalid_radix() {
    diff("err093", |api| unsafe {
        rtrow(api, 93, "(5).toString(1)");
        hdr(93, "radix-37");
        rtrow(api, 93, "(5).toString(37)");
        hdr(93, "radix-0");
        rtrow(api, 93, "(5).toString(0)");
    });
}

/// row 94: `Np_toFixed` (jsnumber.c:134) — "not a number".
#[test]
fn err094_tofixed_not_a_number() {
    diff("err094", |api| unsafe {
        rtrow(api, 94, "Number.prototype.toFixed.call({}, 2)");
    });
}

/// row 95: `Np_toFixed` (jsnumber.c:135) — "precision %d out of range" (width < 0).
#[test]
fn err095_tofixed_precision_negative() {
    diff("err095", |api| unsafe {
        rtrow(api, 95, "(1.5).toFixed(-1)");
        hdr(95, "minus-100");
        rtrow(api, 95, "(1.5).toFixed(-100)");
    });
}

/// row 96: `Np_toFixed` (jsnumber.c:136) — "precision %d out of range" (width > 20).
#[test]
fn err096_tofixed_precision_too_big() {
    diff("err096", |api| unsafe {
        rtrow(api, 96, "(1.5).toFixed(21)");
        hdr(96, "1000");
        rtrow(api, 96, "(1.5).toFixed(1000)");
    });
}

/// row 97: `Np_toExponential` (jsnumber.c:150) — "not a number".
#[test]
fn err097_toexponential_not_a_number() {
    diff("err097", |api| unsafe {
        rtrow(api, 97, "Number.prototype.toExponential.call({}, 2)");
    });
}

/// row 98: `Np_toExponential` (jsnumber.c:151) — "precision %d out of range" (< 0).
#[test]
fn err098_toexponential_precision_negative() {
    diff("err098", |api| unsafe {
        rtrow(api, 98, "(1.5).toExponential(-1)");
        hdr(98, "minus-7");
        rtrow(api, 98, "(1.5).toExponential(-7)");
    });
}

/// row 99: `Np_toExponential` (jsnumber.c:152) — "precision %d out of range" (> 20).
#[test]
fn err099_toexponential_precision_too_big() {
    diff("err099", |api| unsafe {
        rtrow(api, 99, "(1.5).toExponential(21)");
        hdr(99, "99");
        rtrow(api, 99, "(1.5).toExponential(99)");
    });
}

/// row 100: `Np_toPrecision` (jsnumber.c:166) — "not a number".
#[test]
fn err100_toprecision_not_a_number() {
    diff("err100", |api| unsafe {
        rtrow(api, 100, "Number.prototype.toPrecision.call({}, 2)");
    });
}

/// row 101: `Np_toPrecision` (jsnumber.c:167) — "precision %d out of range" (< 1).
#[test]
fn err101_toprecision_precision_too_small() {
    diff("err101", |api| unsafe {
        rtrow(api, 101, "(1.5).toPrecision(0)");
        hdr(101, "minus-3");
        rtrow(api, 101, "(1.5).toPrecision(-3)");
    });
}

/// row 102: `Np_toPrecision` (jsnumber.c:168) — "precision %d out of range" (> 21).
#[test]
fn err102_toprecision_precision_too_big() {
    diff("err102", |api| unsafe {
        rtrow(api, 102, "(1.5).toPrecision(22)");
        hdr(102, "500");
        rtrow(api, 102, "(1.5).toPrecision(500)");
    });
}

/* ------------------------------------------------------------------------- */
/* jsobject.c — rows 103..120                                                */
/* ------------------------------------------------------------------------- */

/// row 103: `O_getPrototypeOf` (jsobject.c:112) — "not an object".
#[test]
fn err103_getprototypeof_not_an_object() {
    diff("err103", |api| unsafe {
        rtrow(api, 103, "Object.getPrototypeOf(42)");
        hdr(103, "undefined");
        rtrow(api, 103, "Object.getPrototypeOf()");
        hdr(103, "string-primitive");
        rtrow(api, 103, "Object.getPrototypeOf('x')");
    });
}

/// row 104: `O_getOwnPropertyDescriptor` (jsobject.c:125) — "not an object".
#[test]
fn err104_getownpropertydescriptor_not_an_object() {
    diff("err104", |api| unsafe {
        rtrow(api, 104, "Object.getOwnPropertyDescriptor(42, 'x')");
        hdr(104, "null");
        rtrow(api, 104, "Object.getOwnPropertyDescriptor(null, 'x')");
    });
}

/// row 105: `O_getOwnPropertyNames` (jsobject.c:176) — "not an object".
#[test]
fn err105_getownpropertynames_not_an_object() {
    diff("err105", |api| unsafe {
        rtrow(api, 105, "Object.getOwnPropertyNames(42)");
        hdr(105, "boolean");
        rtrow(api, 105, "Object.getOwnPropertyNames(true)");
    });
}

/// row 106: `ToPropertyDescriptor` (jsobject.c:258) —
/// "value/writable and get/set attributes are exclusive" (the `get` branch).
#[test]
fn err106_descriptor_value_and_get_exclusive() {
    diff("err106", |api| unsafe {
        rtrow(
            api,
            106,
            "Object.defineProperty({}, 'x', { value: 1, get: function () { return 1 } })",
        );
        hdr(106, "writable+get");
        rtrow(
            api,
            106,
            "Object.defineProperty({}, 'x', { writable: true, get: function () { return 1 } })",
        );
    });
}

/// row 107: `ToPropertyDescriptor` (jsobject.c:265) —
/// "value/writable and get/set attributes are exclusive" (the `set` branch,
/// i.e. no `get` present so the check at :258 is skipped).
#[test]
fn err107_descriptor_value_and_set_exclusive() {
    diff("err107", |api| unsafe {
        rtrow(
            api,
            107,
            "Object.defineProperty({}, 'x', { value: 1, set: function (v) {} })",
        );
        hdr(107, "writable+set");
        rtrow(
            api,
            107,
            "Object.defineProperty({}, 'x', { writable: false, set: function (v) {} })",
        );
    });
}

/// row 108: `O_defineProperty` (jsobject.c:277) — "not an object" (target).
#[test]
fn err108_defineproperty_target_not_an_object() {
    diff("err108", |api| unsafe {
        rtrow(api, 108, "Object.defineProperty(42, 'x', { value: 1 })");
        hdr(108, "no-args");
        rtrow(api, 108, "Object.defineProperty()");
    });
}

/// row 109: `O_defineProperty` (jsobject.c:278) — "not an object" (descriptor).
#[test]
fn err109_defineproperty_descriptor_not_an_object() {
    diff("err109", |api| unsafe {
        rtrow(api, 109, "Object.defineProperty({}, 'x', 42)");
        hdr(109, "missing-descriptor");
        rtrow(api, 109, "Object.defineProperty({}, 'x')");
    });
}

/// row 110: `O_defineProperties_walk` (jsobject.c:289) — "not an object"
/// (an enumerable property of the descriptor map is not an object).
#[test]
fn err110_defineproperties_walk_not_an_object() {
    diff("err110", |api| unsafe {
        rtrow(api, 110, "Object.defineProperties({}, { x: 1 })");
        hdr(110, "string-value");
        rtrow(api, 110, "Object.defineProperties({}, { a: {value:1}, b: 'no' })");
        hdr(110, "via-Object.create");
        rtrow(api, 110, "Object.create({}, { x: 1 })");
    });
}

/// row 111: `O_defineProperties_imp` (jsobject.c:304) — "not an object".
#[test]
fn err111_defineproperties_imp_not_an_object() {
    diff("err111", |api| unsafe {
        rtrow(api, 111, "Object.defineProperties({}, 42)");
        hdr(111, "via-Object.create");
        rtrow(api, 111, "Object.create({}, 42)");
    });
}

/// row 112: `O_defineProperties` (jsobject.c:326) — "not an object" (target).
#[test]
fn err112_defineproperties_target_not_an_object() {
    diff("err112", |api| unsafe {
        rtrow(api, 112, "Object.defineProperties(42, {})");
        hdr(112, "no-args");
        rtrow(api, 112, "Object.defineProperties()");
    });
}

/// row 113: `O_create` (jsobject.c:342) — "not an object or null".
#[test]
fn err113_create_not_an_object_or_null() {
    diff("err113", |api| unsafe {
        rtrow(api, 113, "Object.create(42)");
        hdr(113, "undefined");
        rtrow(api, 113, "Object.create()");
        hdr(113, "string");
        rtrow(api, 113, "Object.create('x')");
    });
}

/// row 114: `O_keys` (jsobject.c:372) — "not an object".
#[test]
fn err114_keys_not_an_object() {
    diff("err114", |api| unsafe {
        rtrow(api, 114, "Object.keys(42)");
        hdr(114, "null");
        rtrow(api, 114, "Object.keys(null)");
    });
}

/// row 115: `O_preventExtensions` (jsobject.c:403) — "not an object".
#[test]
fn err115_preventextensions_not_an_object() {
    diff("err115", |api| unsafe {
        rtrow(api, 115, "Object.preventExtensions(42)");
        hdr(115, "undefined");
        rtrow(api, 115, "Object.preventExtensions()");
    });
}

/// row 116: `O_isExtensible` (jsobject.c:413) — "not an object".
#[test]
fn err116_isextensible_not_an_object() {
    diff("err116", |api| unsafe {
        rtrow(api, 116, "Object.isExtensible(42)");
        hdr(116, "string");
        rtrow(api, 116, "Object.isExtensible('x')");
    });
}

/// row 117: `O_seal` (jsobject.c:431) — "not an object".
#[test]
fn err117_seal_not_an_object() {
    diff("err117", |api| unsafe {
        rtrow(api, 117, "Object.seal(42)");
        hdr(117, "null");
        rtrow(api, 117, "Object.seal(null)");
    });
}

/// row 118: `O_isSealed` (jsobject.c:461) — "not an object".
#[test]
fn err118_issealed_not_an_object() {
    diff("err118", |api| unsafe {
        rtrow(api, 118, "Object.isSealed(42)");
        hdr(118, "undefined");
        rtrow(api, 118, "Object.isSealed()");
    });
}

/// row 119: `O_freeze` (jsobject.c:489) — "not an object".
#[test]
fn err119_freeze_not_an_object() {
    diff("err119", |api| unsafe {
        rtrow(api, 119, "Object.freeze(42)");
        hdr(119, "boolean");
        rtrow(api, 119, "Object.freeze(false)");
    });
}

/// row 120: `O_isFrozen` (jsobject.c:521) — "not an object".
#[test]
fn err120_isfrozen_not_an_object() {
    diff("err120", |api| unsafe {
        rtrow(api, 120, "Object.isFrozen(42)");
        hdr(120, "null");
        rtrow(api, 120, "Object.isFrozen(null)");
    });
}

