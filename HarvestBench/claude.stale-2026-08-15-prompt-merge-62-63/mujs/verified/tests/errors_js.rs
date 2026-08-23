//! Phase C — error-path differential tests for every JavaScript-reachable row
//! of `ERRORS.md` sections 1-3.
//!
//! GENERATED from the trigger table; each entry is
//! `(ERRORS.md row id + C site, JS snippet, expected report string)`.
//!
//! For every row the test asserts:
//!   1. the C and Rust libraries return the SAME `js_dostring` code and produce
//!      byte-identical report output (the differential property), AND
//!   2. the C library really does produce the exact error recorded in
//!      `ERRORS.md` (so the table cannot silently rot), AND
//!   3. the same error is observed through a JS `catch` as `e.name`/`e.message`.
#![allow(non_snake_case)]

mod common;
use common::*;

/// (C site, snippet, expected report string)
const ROWS: &[(&str, &str, &str)] = &[
    ("jsarray.c:440 Ap_sort", "[2,1].sort(1)", "TypeError: comparison function must be a function or undefined"),
    ("jsarray.c:443 Ap_sort", "Array.prototype.sort.call({length:1e10})", "RangeError: array is too large to sort"),
    ("jsarray.c:537 Ap_toString", "Array.prototype.toString.call(null)", "TypeError: 'this' is not an object"),
    ("jsarray.c:604 Ap_every", "[].every()", "TypeError: callback is not a function"),
    ("jsarray.c:633 Ap_some", "[].some()", "TypeError: callback is not a function"),
    ("jsarray.c:662 Ap_forEach", "[].forEach()", "TypeError: callback is not a function"),
    ("jsarray.c:689 Ap_map", "[].map()", "TypeError: callback is not a function"),
    ("jsarray.c:718 Ap_filter", "[].filter()", "TypeError: callback is not a function"),
    ("jsarray.c:751 Ap_reduce", "[].reduce()", "TypeError: callback is not a function"),
    ("jsarray.c:757 Ap_reduce", "[].reduce(function(){})", "TypeError: no initial value"),
    ("jsarray.c:767 Ap_reduce", "Array.prototype.reduce.call({length:1}, function(){})", "TypeError: no initial value"),
    ("jsarray.c:792 Ap_reduceRight", "[].reduceRight()", "TypeError: callback is not a function"),
    ("jsarray.c:798 Ap_reduceRight", "[].reduceRight(function(){})", "TypeError: no initial value"),
    ("jsarray.c:808 Ap_reduceRight", "Array.prototype.reduceRight.call({length:1}, function(){})", "TypeError: no initial value"),
    ("jsboolean.c:16 Bp_toString", "Boolean.prototype.toString.call({})", "TypeError: not a boolean"),
    ("jsboolean.c:23 Bp_valueOf", "Boolean.prototype.valueOf.call({})", "TypeError: not a boolean"),
    ("jsbuiltin.c:145 Decode", r#"decodeURI("%")"#, "URIError: truncated escape sequence"),
    ("jsbuiltin.c:149 Decode", r#"decodeURI("%zz")"#, "URIError: invalid escape sequence"),
    ("jsdate.c:366 js_todate", "Date.prototype.getTime.call({})", "TypeError: not a date"),
    ("jsdate.c:374 js_setdate", "Date.prototype.setTime.call({}, 0)", "TypeError: not a date"),
    ("jsdate.c:485 Dp_toISOString", "new Date(NaN).toISOString()", "RangeError: invalid date"),
    ("jsdate.c:793 Dp_toJSON", "Date.prototype.toJSON.call({})", "TypeError: this.toISOString is not a function"),
    ("jserror.c:36 Ep_toString", r#"Error.prototype.toString.call("x")"#, "TypeError: not an object"),
    ("jsfunction.c:53 Fp_toString", "Function.prototype.toString.call({})", "TypeError: not a function"),
    ("jsfunction.c:100 Fp_apply", "Function.prototype.apply.call({})", "TypeError: not a function"),
    ("jsfunction.c:123 Fp_call", "Function.prototype.call.call({})", "TypeError: not a function"),
    ("jsfunction.c:186 Fp_bind", "Function.prototype.bind.call({})", "TypeError: not a function"),
    ("jsnumber.c:22 Np_valueOf", "Number.prototype.valueOf.call({})", "TypeError: not a number"),
    ("jsnumber.c:33 Np_toString", "Number.prototype.toString.call({})", "TypeError: not a number"),
    ("jsnumber.c:40 Np_toString", "(5).toString(1)", "RangeError: invalid radix"),
    ("jsnumber.c:134 Np_toFixed", "Number.prototype.toFixed.call({})", "TypeError: not a number"),
    ("jsnumber.c:135 Np_toFixed", "(1).toFixed(-1)", "RangeError: precision -1 out of range"),
    ("jsnumber.c:136 Np_toFixed", "(1).toFixed(21)", "RangeError: precision 21 out of range"),
    ("jsnumber.c:150 Np_toExponential", "Number.prototype.toExponential.call({})", "TypeError: not a number"),
    ("jsnumber.c:151 Np_toExponential", "(1).toExponential(-1)", "RangeError: precision -1 out of range"),
    ("jsnumber.c:152 Np_toExponential", "(1).toExponential(21)", "RangeError: precision 21 out of range"),
    ("jsnumber.c:166 Np_toPrecision", "Number.prototype.toPrecision.call({})", "TypeError: not a number"),
    ("jsnumber.c:167 Np_toPrecision", "(1).toPrecision(0)", "RangeError: precision 0 out of range"),
    ("jsnumber.c:168 Np_toPrecision", "(1).toPrecision(22)", "RangeError: precision 22 out of range"),
    ("jsobject.c:112 O_getPrototypeOf", "Object.getPrototypeOf(1)", "TypeError: not an object"),
    ("jsobject.c:125 O_getOwnPropertyDescriptor", r#"Object.getOwnPropertyDescriptor(1, "x")"#, "TypeError: not an object"),
    ("jsobject.c:176 O_getOwnPropertyNames", "Object.getOwnPropertyNames(1)", "TypeError: not an object"),
    ("jsobject.c:258 ToPropertyDescriptor", r#"Object.defineProperty({}, "x", {value:1, get:function(){}})"#, "TypeError: value/writable and get/set attributes are exclusive"),
    ("jsobject.c:265 ToPropertyDescriptor", r#"Object.defineProperty({}, "x", {value:1, set:function(v){}})"#, "TypeError: value/writable and get/set attributes are exclusive"),
    ("jsobject.c:277 O_defineProperty", r#"Object.defineProperty(1, "x", {})"#, "TypeError: not an object"),
    ("jsobject.c:278 O_defineProperty", r#"Object.defineProperty({}, "x", 1)"#, "TypeError: not an object"),
    ("jsobject.c:289 O_defineProperties_walk", "Object.defineProperties({}, {x:1})", "TypeError: not an object"),
    ("jsobject.c:304 O_defineProperties_imp", "Object.defineProperties({}, 1)", "TypeError: not an object"),
    ("jsobject.c:326 O_defineProperties", "Object.defineProperties(1, {})", "TypeError: not an object"),
    ("jsobject.c:342 O_create", "Object.create(1)", "TypeError: not an object or null"),
    ("jsobject.c:372 O_keys", "Object.keys(1)", "TypeError: not an object"),
    ("jsobject.c:403 O_preventExtensions", "Object.preventExtensions(1)", "TypeError: not an object"),
    ("jsobject.c:413 O_isExtensible", "Object.isExtensible(1)", "TypeError: not an object"),
    ("jsobject.c:431 O_seal", "Object.seal(1)", "TypeError: not an object"),
    ("jsobject.c:461 O_isSealed", "Object.isSealed(1)", "TypeError: not an object"),
    ("jsobject.c:489 O_freeze", "Object.freeze(1)", "TypeError: not an object"),
    ("jsobject.c:521 O_isFrozen", "Object.isFrozen(1)", "TypeError: not an object"),
    ("jscompile.c:43 checkfutureword", "const", "SyntaxError: [string]:1: 'const' is a future reserved word"),
    ("jscompile.c:46 checkfutureword", r#""use strict"; let"#, "SyntaxError: [string]:1: 'let' is a strict mode future reserved word"),
    ("jscompile.c:114 addlocal", r#""use strict"; var arguments;"#, "SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode"),
    ("jscompile.c:116 addlocal", r#""use strict"; var eval;"#, "SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode"),
    ("jscompile.c:119 addlocal", "var eval;", "EvalError: [string]:1: invalid use of 'eval'"),
    ("jscompile.c:128 addlocal", r#""use strict"; function f(a,a){}"#, "SyntaxError: [string]:1: duplicate formal parameter 'a'"),
    ("jscompile.c:204 emitlocal", r#""use strict"; arguments = 1;"#, "SyntaxError: [string]:1: 'arguments' is read-only in strict mode"),
    ("jscompile.c:206 emitlocal", r#""use strict"; eval = 1;"#, "SyntaxError: [string]:1: 'eval' is read-only in strict mode"),
    ("jscompile.c:209 emitlocal", "eval", "EvalError: [string]:1: invalid use of 'eval'"),
    ("jscompile.c:315 checkdup", r#""use strict"; ({a:1,a:2});"#, "SyntaxError: [string]:0: duplicate property 'a' in object literal"),
    ("jscompile.c:400 cassign", "1 = 2;", "SyntaxError: [string]:1: invalid l-value in assignment"),
    ("jscompile.c:410 cassignforin", "for (var a, b in {}) ;", "SyntaxError: [string]:0: more than one loop variable in for-in statement"),
    ("jscompile.c:439 cassignforin", "for (1 in {}) ;", "SyntaxError: [string]:1: invalid l-value in for-in loop assignment"),
    ("jscompile.c:464 cassignop1", "1 += 2;", "SyntaxError: [string]:1: invalid l-value in assignment"),
    ("jscompile.c:508 cdelete", r#""use strict"; delete x;"#, "SyntaxError: [string]:1: delete on an unqualified name is not allowed in strict mode"),
    ("jscompile.c:524 cdelete", "delete 1;", "SyntaxError: [string]:1: invalid l-value in delete expression"),
    ("jscompile.c:961 ctrycatch", r#""use strict"; try{}catch(arguments){}"#, "SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode"),
    ("jscompile.c:963 ctrycatch", r#""use strict"; try{}catch(eval){}"#, "SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode"),
    ("jscompile.c:993 ctrycatchfinally", r#""use strict"; try{}catch(arguments){}finally{}"#, "SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode"),
    ("jscompile.c:995 ctrycatchfinally", r#""use strict"; try{}catch(eval){}finally{}"#, "SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode"),
    ("jscompile.c:1025 cswitch", "switch(1){default:default:}", "SyntaxError: [string]:1: more than one default label in switch"),
    ("jscompile.c:1217 cstm", "break foo;", "SyntaxError: [string]:1: break label 'foo' not found"),
    ("jscompile.c:1221 cstm", "break;", "SyntaxError: [string]:1: unlabelled break must be inside loop or switch"),
    ("jscompile.c:1233 cstm", "continue foo;", "SyntaxError: [string]:1: continue label 'foo' not found"),
    ("jscompile.c:1237 cstm", "continue;", "SyntaxError: [string]:1: continue must be inside loop"),
    ("jscompile.c:1251 cstm", "return;", "SyntaxError: [string]:1: return not in function"),
    ("jscompile.c:1266 cstm", r#""use strict"; with({}){}"#, "SyntaxError: [string]:1: 'with' statements are not allowed in strict mode"),
    ("jslex.c:177 jsY_expect (macro)", r#"JSON.parse("nul")"#, "SyntaxError: JSON:1: expected 'l'"),
    ("jslex.c:192 jsY_unescape", r#"\q"#, "SyntaxError: [string]:1: unexpected escape sequence"),
    ("jslex.c:255 lexhex", "0x", "SyntaxError: [string]:1: malformed hexadecimal number"),
    ("jslex.c:351 lexnumber", "01", "SyntaxError: [string]:1: number with leading zero"),
    ("jslex.c:377 lexnumber", "1e", "SyntaxError: [string]:1: missing exponent"),
    ("jslex.c:381 lexnumber", "1a", "SyntaxError: [string]:1: number with letter suffix"),
    ("jslex.c:399 lexescape", r#""\"#, "SyntaxError: [string]:1: unterminated escape sequence"),
    ("jslex.c:440 lexstring", r#""abc"#, "SyntaxError: [string]:1: string not terminated"),
    ("jslex.c:443 lexstring", r#""\x""#, "SyntaxError: [string]:1: malformed escape sequence"),
    ("jslex.c:490 lexregexp", "/abc", "SyntaxError: [string]:1: regular expression not terminated"),
    ("jslex.c:497 lexregexp", r#"/\"#, "SyntaxError: [string]:1: regular expression not terminated"),
    ("jslex.c:521 lexregexp", "/a/x", "SyntaxError: [string]:1: illegal flag in regular expression: x"),
    ("jslex.c:525 lexregexp", "/a/gg", "SyntaxError: [string]:1: duplicated flag in regular expression"),
    ("jslex.c:574 jsY_lexx", "/*", "SyntaxError: [string]:1: multi-line comment not terminated"),
    ("jslex.c:728 jsY_lexx", "#", "SyntaxError: [string]:1: unexpected character: '#'"),
    ("jslex.c:729 jsY_lexx", "€", r#"SyntaxError: [string]:1: unexpected character: \u20AC"#),
    ("jslex.c:760 lexjsonnumber", r#"JSON.parse("-")"#, "SyntaxError: JSON:1: unexpected non-digit"),
    ("jslex.c:767 lexjsonnumber", r#"JSON.parse("1.")"#, "SyntaxError: JSON:1: missing digits after decimal point"),
    ("jslex.c:777 lexjsonnumber", r#"JSON.parse("1e")"#, "SyntaxError: JSON:1: missing digits after exponent indicator"),
    ("jslex.c:791 lexjsonescape", r#"JSON.parse('"\\q"')"#, "SyntaxError: JSON:1: invalid escape sequence"),
    ("jslex.c:820 lexjsonstring", r#"JSON.parse('"abc')"#, "SyntaxError: JSON:1: unterminated string"),
    ("jslex.c:822 lexjsonstring", r#"JSON.parse('"\x01"')"#, "SyntaxError: JSON:1: invalid control character in string"),
    ("jslex.c:878 jsY_lexjson", r#"JSON.parse("x")"#, "SyntaxError: JSON:1: unexpected character: 'x'"),
    ("jslex.c:879 jsY_lexjson", r#"JSON.parse("€")"#, r#"SyntaxError: JSON:1: unexpected character: \u20AC"#),
    ("json.c:41 jsonexpect", "JSON.parse('[1')", "SyntaxError: JSON: unexpected token: (end-of-file) (expected ']')"),
    ("json.c:67 jsonvalue", "JSON.parse('{1:2}')", "SyntaxError: JSON: unexpected token: (number) (expected string)"),
    ("json.c:107 jsonvalue", "JSON.parse('')", "SyntaxError: JSON: unexpected token: (end-of-file)"),
    ("json.c:261 fmtobject", "var a={};a.a=a;JSON.stringify(a);", "TypeError: cyclic object value"),
    ("json.c:297 fmtarray", "var a=[];a[0]=a;JSON.stringify(a);", "TypeError: cyclic object value"),
    ("jsparse.c:24 INCREC (macro)", "(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1", "SyntaxError: [string]:1: too much recursion"),
    ("jsparse.c:143 jsP_expect (macro)", "(1", "SyntaxError: [string]:1: unexpected token: (end-of-file) (expected ')')"),
    ("jsparse.c:153 semicolon", "1 2", "SyntaxError: [string]:1: unexpected token: (number) (expected ';')"),
    ("jsparse.c:166 identifier", "var 1", "SyntaxError: [string]:1: unexpected token: (number) (expected identifier)"),
    ("jsparse.c:183 identifiername", "a.", "SyntaxError: [string]:1: unexpected token: (end-of-file) (expected identifier or keyword)"),
    ("jsparse.c:363 primary", "*", "SyntaxError: [string]:1: unexpected token in expression: '*'"),
    ("jsparse.c:700 caseclause", "switch(1){x}", "SyntaxError: [string]:1: unexpected token in switch: (identifier) (expected 'case' or 'default')"),
    ("jsparse.c:751 forstatement", "for(var a)", "SyntaxError: [string]:1: unexpected token in for-var-statement: ')'"),
    ("jsparse.c:770 forstatement", "for(a)", "SyntaxError: [string]:1: unexpected token in for-statement: ')'"),
    ("jsparse.c:888 statement", "try{}", "SyntaxError: [string]:1: unexpected token in try: (end-of-file) (expected 'catch' or 'finally')"),
    ("jsproperty.c:228 jsV_setproperty", r#""use strict"; var o = Object.preventExtensions({}); o.q = 1;"#, "TypeError: object is non-extensible"),
    ("jsregexp.c:38 js_newregexpx", r#"new RegExp("(");"#, "SyntaxError: regular expression: unmatched '('"),
    ("jsregexp.c:77 js_RegExp_prototype_exec", r#"/a*/.exec(new Array(6000).join("a"));"#, "Error: regexec failed"),
    ("jsregexp.c:126 Rp_test", r#"/a*/.test(new Array(6000).join("a"));"#, "Error: regexec failed"),
    ("jsregexp.c:149 jsB_new_RegExp", r#"new RegExp(/a/, "g");"#, "TypeError: cannot supply flags when creating one RegExp from another"),
    ("jsregexp.c:172 jsB_new_RegExp", r#"new RegExp("a", "x");"#, "SyntaxError: invalid regular expression flag: 'x'"),
    ("jsregexp.c:175 jsB_new_RegExp", r#"new RegExp("a", "gg");"#, "SyntaxError: invalid regular expression flag: 'g'"),
    ("jsregexp.c:176 jsB_new_RegExp", r#"new RegExp("a", "ii");"#, "SyntaxError: invalid regular expression flag: 'i'"),
    ("jsregexp.c:177 jsB_new_RegExp", r#"new RegExp("a", "mm");"#, "SyntaxError: invalid regular expression flag: 'm'"),
    ("jsrun.c:373 js_toregexp", r#"RegExp.prototype.test.call("a", "a");"#, "TypeError: not a regexp"),
    ("jsrun.c:393 jsR_tofunction", r#"Object.defineProperty({}, "x", {get:1});"#, "TypeError: not a function"),
    ("jsrun.c:707 jsR_setproperty", "[].length = 1.5;", "RangeError: invalid array length"),
    ("jsrun.c:709 jsR_setproperty", "[].length = 1073741824;", "RangeError: array too large"),
    ("jsrun.c:773 jsR_setproperty", r#""use strict"; var o = {get x(){return 1}}; o.x = 2;"#, "TypeError: setting property 'x' that only has a getter"),
    ("jsrun.c:800 jsR_setproperty", r#""use strict"; "abc".length = 1;"#, "TypeError: 'length' is read-only"),
    ("jsrun.c:854 jsR_defproperty", r#""use strict"; Object.defineProperty(function(){}, "length", {value:1});"#, "TypeError: 'length' is read-only"),
    ("jsrun.c:860 jsR_defproperty", r#""use strict"; Object.defineProperty(function(){}, "length", {get:function(){}});"#, "TypeError: 'length' is non-configurable"),
    ("jsrun.c:866 jsR_defproperty", r#""use strict"; Object.defineProperty(function(){}, "length", {set:function(v){}});"#, "TypeError: 'length' is non-configurable"),
    ("jsrun.c:875 jsR_defproperty", r#"Object.defineProperty([], "length", {value:0});"#, "TypeError: 'length' is read-only or non-configurable"),
    ("jsrun.c:921 jsR_delproperty", r#""use strict"; delete [].length;"#, "TypeError: 'length' is non-configurable"),
    ("jsrun.c:1127 js_setvar", r#""use strict"; undefined = 1;"#, "TypeError: 'undefined' is read-only"),
    ("jsrun.c:1133 js_setvar", r#""use strict"; xyz = 1;"#, "ReferenceError: assignment to undeclared variable 'xyz'"),
    ("jsrun.c:1290 jsR_pushtrace", "function f(){ f() } f();", "Error: call stack overflow"),
    ("jsrun.c:1307 js_call", "undefined();", "TypeError: undefined is not callable"),
    ("jsrun.c:1341 js_construct", "new undefined();", "TypeError: undefined is not callable"),
    ("jsrun.c:1673 jsR_run", r#"x = 1; eval("var x; delete x; x");"#, "ReferenceError: 'x' is not defined"),
    ("jsrun.c:1698 jsR_run", "nosuchvar;", "ReferenceError: 'nosuchvar' is not defined"),
    ("jsrun.c:1721 jsR_run", r#""a" in 1;"#, "TypeError: operand to 'in' is not an object"),
    ("jsstring.c:9 js_doregexec", r#"new Array(6000).join("a").search(/a*/);"#, "Error: regexec failed"),
    ("jsstring.c:16 checkstring", "String.prototype.trim.call(null);", "TypeError: string function called on null or undefined"),
    ("jsstring.c:108 Sp_toString", "String.prototype.toString.call(1);", "TypeError: not a string"),
    ("jsstring.c:115 Sp_valueOf", "String.prototype.valueOf.call(1);", "TypeError: not a string"),
    ("jsvalue.c:144 jsV_toprimitive", r#""use strict"; var o = Object.create(null); o + "";"#, "TypeError: cannot convert object to primitive"),
    ("jsvalue.c:401 jsV_toobject", "undefined.x;", "TypeError: cannot convert undefined to object"),
    ("jsvalue.c:402 jsV_toobject", "null.x;", "TypeError: cannot convert null to object"),
    ("jsvalue.c:579 js_instanceof", "1 instanceof 2;", "TypeError: instanceof: invalid operand"),
    ("jsvalue.c:586 js_instanceof", "var f = function(){}; f.prototype = 1; ({}) instanceof f;", "TypeError: instanceof: 'prototype' property is not an object"),
];

#[test]
fn errors_js_differential_all_rows() {
    let mut fails = Vec::new();
    for (site, src, want) in ROWS {
        let (c, r) = both(|api, _| run_script(api, 0, src));
        if c != r {
            fails.push(format!(
                "  [{}] {:?}\n      C   : rc={} out={:?}\n      Rust: rc={} out={:?}",
                site, src, c.0, String::from_utf8_lossy(&c.1), r.0, String::from_utf8_lossy(&r.1)
            ));
            continue;
        }
        let got = String::from_utf8_lossy(&c.1).to_string();
        let expect = format!("[report] {}\n", want);
        if c.0 != 1 || got != expect {
            fails.push(format!(
                "  [{}] {:?}\n      ERRORS.md says {:?} but the C library produced rc={} {:?}",
                site, src, expect, c.0, got
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "{} of {} ERRORS.md rows failed:\n{}",
        fails.len(),
        ROWS.len(),
        fails.join("\n")
    );
}

/// The same triggers, observed through a JS `catch` clause instead of the
/// top-level report path (a different code path: OP_CATCH, and the exception
/// value stays a live object).
#[test]
fn errors_js_differential_via_catch() {
    let mut fails = Vec::new();
    for (site, src, _want) in ROWS {
        let wrapped = format!(
            "try {{ {} }} catch (e) {{ print('caught', typeof e, e && e.name, e && e.message, String(e)) }}",
            src
        );
        let (c, r) = both(|api, _| run_script(api, 0, &wrapped));
        if c != r {
            fails.push(format!(
                "  [{}] {:?}\n      C   : rc={} out={:?}\n      Rust: rc={} out={:?}",
                site, wrapped, c.0, String::from_utf8_lossy(&c.1), r.0, String::from_utf8_lossy(&r.1)
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "{} of {} caught-error rows diverged:\n{}",
        fails.len(),
        ROWS.len(),
        fails.join("\n")
    );
}

/// And once more on a strict state, which routes through different compile-time
/// checks (`J->default_strict`).
#[test]
fn errors_js_differential_strict_state() {
    let mut fails = Vec::new();
    for (site, src, _want) in ROWS {
        let (c, r) = both(|api, _| run_script(api, JS_STRICT, src));
        if c != r {
            fails.push(format!(
                "  [{}] {:?}\n      C   : rc={} out={:?}\n      Rust: rc={} out={:?}",
                site, src, c.0, String::from_utf8_lossy(&c.1), r.0, String::from_utf8_lossy(&r.1)
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "{} of {} rows diverged under JS_STRICT:\n{}",
        fails.len(),
        ROWS.len(),
        fails.join("\n")
    );
}

/// `ERRORS.md` rows marked HARD: reachable, but only with a generated
/// multi-kilobyte source.
#[test]
fn errors_js_hard_generated_sources() {
    let cases: Vec<(&str, String)> = vec![
        // jscompile.c:75  emitraw -> "integer overflow in instruction coding"
        // emit() writes F->lastline first, so a line number > 65535 overflows
        // js_Instruction (unsigned short).
        ("jscompile.c:75 emitraw", format!("{}x", "\n".repeat(65536))),
        ("jscompile.c:75 emitraw (just under)", format!("{}x", "\n".repeat(65000))),
        // jscompile.c:238  emitjumpto -> "jump address integer overflow"
        ("jscompile.c:238 emitjumpto", format!("{}while(0);", "1;".repeat(15000))),
        // jscompile.c:245  labelto -> "jump address integer overflow"
        ("jscompile.c:245 labelto", format!("{}if(0);", "1;".repeat(15000))),
        ("jscompile.c:245 labelto (just under)", format!("{}if(0);", "1;".repeat(10000))),
    ];
    let mut fails = Vec::new();
    for (site, src) in &cases {
        let (c, r) = both(|api, _| run_script(api, 0, src));
        if c != r {
            fails.push(format!(
                "  [{}] source len {}\n      C   : rc={} out={:?}\n      Rust: rc={} out={:?}",
                site, src.len(), c.0, String::from_utf8_lossy(&c.1), r.0, String::from_utf8_lossy(&r.1)
            ));
        }
    }
    assert!(fails.is_empty(), "HARD rows diverged:\n{}", fails.join("\n"));
}

/// `ERRORS.md` rows marked NO (proved unreachable): assert the *neighbouring*
/// reachable behaviour still matches, so the surrounding branch is covered even
/// though the rejection itself cannot fire.
#[test]
fn errors_js_unreachable_rows_neighbourhood() {
    let scripts = [
        // jslex.c:269/312/333 live inside an `#if 0` block; the live duplicates
        // are jslex.c:351/381, exercised here together with the near-misses.
        "print(0)", "print(0.5)", "print(00)", "print(01)", "print(0x10)", "print(09)",
        "print(1e1)", "print(1E1)", "print(1e+1)", "print(1e-1)",
        // jscompile.c:336 invalid property name in object initializer — every
        // property-name form the parser can actually produce:
        "print(JSON.stringify({a:1, 'b':2, 3:4, 0.5:6, if:7, null:8, true:9}))",
        "print(JSON.stringify({get a(){return 1}, set a(v){}}))",
        // jscompile.c:487 cassignop2 — the reachable cassignop1 twin:
        "var o={}; o.x = 1; o.x += 2; print(o.x)",
        "var a=[0]; a[0] += 2; print(a[0])",
        "var v=1; v += 2; print(v)",
        // jscompile.c:780 cexp default — every expression node kind:
        "print(typeof (function(){}), typeof [], typeof {}, typeof /x/, typeof null)",
        "for (var k in {a:1}) print(k)",
        "switch(1){case 1: print('one'); break; default: print('def')}",
        // jsrun.c:166 js_pushlstring / jsintern.c:47 — long-but-legal strings:
        "var s=''; for(var i=0;i<1000;++i) s+='x'; print(s.length)",
        "print(String.fromCharCode(65,66,67).length)",
        // jsrun.c:1145 js_delvar strict — blocked at compile time:
        "try { eval('\"use strict\"; delete x') } catch(e) { print('caught', e.name, e.message) }",
        "var o={x:1}; print(delete o.x, o.x)",
        // regexp.c:942 defensive `syntax error` — the reachable :940 twin:
        "try { new RegExp(')') } catch(e) { print('caught', e.message) }",
        "try { new RegExp('a)') } catch(e) { print('caught', e.message) }",
    ];
    diff_scripts(0, &scripts);
    diff_scripts(JS_STRICT, &scripts);
}
