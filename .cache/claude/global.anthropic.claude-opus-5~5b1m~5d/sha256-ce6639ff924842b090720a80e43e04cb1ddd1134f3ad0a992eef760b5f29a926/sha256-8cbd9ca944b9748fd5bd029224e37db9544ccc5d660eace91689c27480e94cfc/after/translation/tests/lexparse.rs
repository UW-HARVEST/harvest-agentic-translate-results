//! Differential tests for CONFIGS.md rows 64-68 and 76-78:
//! the low level lexer / parser / compiler pipeline, the interned string
//! table, the raw allocator wrappers and the js_Buffer helpers.
//!
//! Every case runs in its own forked child pair (see `common::diff`), so a
//! case whose input makes the library throw (which, uncaught, ends the
//! process through `js_atpanic`) does not hide the cases after it.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

extern "C" {
    fn _exit(code: c_int) -> !;
    fn fflush(f: *mut c_void) -> c_int;
}

/* ------------------------------------------------------------------ */
/* uncaught exceptions: print the message, then leave deterministically */

static mut PANIC_API: *const Api = null();

unsafe extern "C" fn panic_report(J: JS) {
    let api = &*PANIC_API;
    let dflt = cs("<no-message>");
    let msg = (api.js_trystring)(J, -1, dflt.as_ptr());
    p_str("PANIC", msg);
    fflush(null_mut());
    _exit(9);
}

/// Fresh state whose uncaught-exception path is observable and deterministic.
unsafe fn st(api: &Api, flags: c_int) -> JS {
    PANIC_API = api as *const Api;
    let J = newstate(api, flags);
    (api.js_atpanic)(J, Some(panic_report));
    J
}

/* ================================================================== */
/* row 67: jsY_initlex + jsY_lex                                       */
/* ================================================================== */

unsafe fn lex_tokens(api: &Api, src: &str, json: bool) {
    let J = st(api, 0);
    let file = cs("lex.js");
    let source = cs(src);
    (api.jsY_initlex)(J, file.as_ptr(), source.as_ptr());
    let mut i = 0;
    loop {
        let t = if json { (api.jsY_lexjson)(J) } else { (api.jsY_lex)(J) };
        p_int(&format!("tok{}", i), t);
        p_str(&format!("txt{}", i), (api.jsY_tokenstring)(t));
        if t == 0 {
            break;
        }
        i += 1;
        if i >= 4000 {
            p_line("TRUNCATED");
            break;
        }
    }
    p_int("ntok", i);
    (api.js_freestate)(J);
}

/// A deterministic soup of `count` tokens picked from `toks`.
fn soup(seed: u64, count: usize, toks: &[&str]) -> String {
    let mut r = Rng::new(seed);
    let mut s = String::new();
    for _ in 0..count {
        s.push_str(toks[r.range(toks.len() as u32) as usize]);
        s.push(' ');
    }
    s
}

fn lex_corpus() -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();

    v.push(("empty", String::new()));
    v.push(("ws_only", " \t\u{b}\u{c}\u{a0}\u{feff}  ".to_string()));
    v.push(("bom_first", "\u{feff}var a = 1;".to_string()));
    v.push(("weird_ws", "\u{feff}a\u{a0}b\tc\u{b}d\u{c}e".to_string()));

    /* every single-character punctuator */
    v.push(("punct1", "()[]{},;:?~.".to_string()));
    /* every multi-character operator and compound assignment */
    v.push((
        "ops",
        "< <= << <<= > >= >> >>= >>> >>>= = == === ! != !== \
         + ++ += - -- -= * *= % %= & && &= | || |= ^ ^= ~ ?"
            .to_string(),
    ));
    v.push(("ops_tight", "a<b<=c<<d<<=e>f>=g>>h>>=i>>>j>>>=k".to_string()));
    v.push(("ops_tight2", "a=b==c===d!e!=f!==g+h++i+=j-k--l-=m*n*=o%p%=q".to_string()));
    v.push(("ops_tight3", "a&b&&c&=d|e||f|=g^h^=i".to_string()));

    /* all keywords */
    v.push((
        "keywords",
        "break case catch continue debugger default delete do else false \
         finally for function if in instanceof new null return switch this \
         throw true try typeof var void while with"
            .to_string(),
    ));
    /* future reserved words (plain identifiers to the lexer) */
    v.push((
        "futurewords",
        "abstract boolean byte char class const double enum export extends \
         final float goto implements import int interface long native \
         package private protected public short static super synchronized \
         throws transient volatile let yield"
            .to_string(),
    ));

    /* identifiers */
    v.push(("idents", "$ _ $$ __ a A z Z a1 _9 $x9 abc$_123".to_string()));
    v.push(("idents_unicode", "\u{3c0} \u{fc}n\u{ef}code caf\u{e9} \u{5d0}".to_string()));
    v.push(("idents_escape", r"\u0041BC a\u0062c \u0069f \u0024x \u005f".to_string()));

    /* numbers */
    v.push((
        "numbers",
        "0 1 42 100 3.14 0.5 .5 5. 0. 1e3 1E3 1e+3 1e-3 1.5e10 1.5E-10 \
         0x0 0x1f 0X1F 0xdeadBEEF 0xFFFFFFFF 9007199254740993 1e309 \
         0.000001 1e21"
            .to_string(),
    ));
    v.push(("number_dot", ". .. 1 .1 .".to_string()));

    /* strings: every escape */
    let mut s = String::new();
    s.push_str(r"'a\nb' ");
    s.push_str(r"'a\tb' ");
    s.push_str(r"'\x41' ");
    s.push_str(r"'A' ");
    s.push_str(r"'\0' ");
    s.push_str(r"'\\' ");
    s.push_str(r"'\'' ");
    s.push_str(r"'\b\f\n\r\t\v' ");
    s.push_str(r"'\u00e9\u0041\uFFFF' ");
    s.push_str(r"'\q\ \-' ");
    s.push_str(r#""c\td" "#);
    s.push_str(r#""\"" "#);
    s.push_str(r#""\u0041\x42\\" "#);
    s.push_str("'' \"\" ");
    s.push_str("'line\\\ncont' ");
    s.push_str("\"dq\\\ncont\" ");
    s.push_str("'utf8 \u{e9}\u{4e2d}\u{1f600}' ");
    v.push(("strings", s));

    /* regexp literals and the div/regexp wart */
    v.push((
        "regexp",
        r"x = /ab+c/gi; y = /a\/b/m; z = /[/]/; w = /x/; q = /a/g;".to_string(),
    ));
    v.push((
        "regexp_ctx",
        r"a / b; 1 / 2; a /= b; (x) / y; [1] / 2; {} /re/; typeof /t/i; return /r/gm; case /c/:"
            .to_string(),
    ));
    v.push(("regexp_class", r"= /[a-z/\]]+/i".to_string()));
    v.push(("regexp_escapes", r"= /\\\/\n[\]]/".to_string()));
    v.push(("div_ass", "a /= 2; b / 2; c/d/e".to_string()));

    /* comments */
    v.push((
        "comments",
        "// one\nx // two\n/* a */ y /** b **/ z /*\nmulti\n*/ w /***/ v // end".to_string(),
    ));
    v.push(("comment_only", "/* just a comment */".to_string()));
    v.push(("linecomment_eof", "x // no newline at eof".to_string()));

    /* line terminators */
    v.push(("lineterm", "a\nb\rc\r\nd\u{2028}e\u{2029}f\n".to_string()));
    v.push(("crlf_mix", "1\r\n2\r3\n4\u{2028}5".to_string()));
    /* the line counter is only observable through an error message */
    v.push(("lineterm_error", "a\nb\r\nc\u{2028}d\u{2029}@".to_string()));

    /* newline-restricted contexts */
    v.push((
        "nlth",
        "return\n1;\nbreak\nx;\ncontinue\ny;\nthrow\nz;\nvar\nq;".to_string(),
    ));

    /* generated soup */
    v.push((
        "soup300",
        soup(
            0x1234_5678_9abc_def1,
            300,
            &[
                "a", "b1", "$x", "_y", "0", "1", "42", "3.5", ".5", "0x1f", "'s'", "\"d\"", "+",
                "-", "*", "%", "<", ">", "<=", ">=", "==", "===", "!=", "!==", "&&", "||", "!",
                "~", "?", ":", ";", ",", "(", ")", "[", "]", "{", "}", "=", "in", "new", "typeof",
                "this", "true", "false", "null", "++", "--", "+=", ">>>=",
            ],
        ),
    ));
    v.push((
        "soup_nl",
        soup(
            0x0fed_cba9_8765_4321,
            120,
            &["\n", "return", "break", "continue", "throw", "a", "1", "+", ";", "(", ")", "{", "}"],
        ),
    ));

    /* throwing inputs (identical throw in both libraries is a pass) */
    v.push(("err_block_comment", "x /* abc".to_string()));
    v.push(("err_block_comment2", "x /*".to_string()));
    v.push(("err_block_comment3", "x /* * ".to_string()));
    v.push(("err_string_eof", "'abc".to_string()));
    v.push(("err_string_nl", "'ab\ncd'".to_string()));
    v.push(("err_string_esc_eof", "'ab\\".to_string()));
    v.push(("err_string_u2028", "'a\u{2028}b'".to_string()));
    v.push(("err_string_u2029", "'a\u{2029}b'".to_string()));
    v.push(("err_ident_cjk", "a \u{5b98} b".to_string()));
    v.push(("err_escape_short", r"'\u00'".to_string()));
    v.push(("err_escape_hex", r"'\xZZ'".to_string()));
    v.push(("err_hex_empty", "0x".to_string()));
    v.push(("err_hex_bad", "0xg".to_string()));
    v.push(("err_leading_zero", "0755".to_string()));
    v.push(("err_leading_zero2", "01.5".to_string()));
    v.push(("err_letter_suffix", "3abc".to_string()));
    v.push(("err_exponent", "1e".to_string()));
    v.push(("err_exponent2", "1e+".to_string()));
    v.push(("err_char_at", "@".to_string()));
    v.push(("err_char_hash", "a # b".to_string()));
    v.push(("err_char_backslash", r"\q".to_string()));
    v.push(("err_char_u_short", r"\u00".to_string()));
    v.push(("err_char_ideographic_space", "a\u{3000}b".to_string()));
    v.push(("err_char_ctrl", "a\u{1}b".to_string()));
    v.push(("err_regexp_eof", "= /abc".to_string()));
    v.push(("err_regexp_nl", "= /ab\nc/".to_string()));
    v.push(("err_regexp_class_eof", "= /[abc/".to_string()));
    v.push(("err_regexp_flag", "= /a/x".to_string()));
    v.push(("err_regexp_dupflag", "= /a/gg".to_string()));

    v
}

#[test]
fn cfg67_lex_tokens() {
    for (i, (name, src)) in lex_corpus().into_iter().enumerate() {
        let src = src.clone();
        diff(&format!("cfg67_{:02}_{}", i, name), move |api| unsafe {
            p_line(name);
            lex_tokens(api, &src, false);
        });
    }
}

/* ================================================================== */
/* row 68: jsY_initlex + jsY_lexjson                                   */
/* ================================================================== */

fn json_corpus() -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();

    v.push(("empty", String::new()));
    v.push(("ws", " \t\n\r \u{a0}\u{feff}".to_string()));
    v.push(("obj_empty", "{}".to_string()));
    v.push(("arr_empty", "[]".to_string()));
    v.push(("obj_flat", r#"{"a":1,"b":true,"c":null}"#.to_string()));
    v.push((
        "nested",
        r#"{ "a" : [ 1 , 2 , { "c" : null , "d" : [ [ ] , { } ] } ] , "b" : "x" }"#.to_string(),
    ));
    v.push((
        "numbers",
        "[0,-0,1,-1,1.5,-1.5,0.0,1e10,1E10,1e+10,1e-10,-1.5e+3,123456789,\
         0.000001,1e309,9007199254740993]"
            .to_string(),
    ));
    v.push(("number_zero_seq", "0 1 -1 0.5".to_string()));
    v.push((
        "strings",
        r#"["","a","\"","\\","\/","\b","\f","\n","\r","\t","\u0041","\u00e9","\uD83D\uDE00","\u0000","tab\there"]"#
            .to_string(),
    ));
    v.push(("string_utf8", "[\"\u{e9}\u{4e2d}\u{1f600}\"]".to_string()));
    v.push(("literals", "[true,false,null]".to_string()));
    v.push(("literal_bare", "true".to_string()));
    v.push(("keys_repeat", r#"{"a":1,"a":2}"#.to_string()));
    v.push((
        "soup",
        soup(
            0x00c0_ffee_0bad_f00d,
            200,
            &[
                "{", "}", "[", "]", ",", ":", r#""k""#, "1", "-2", "3.5", "1e2", "true", "false",
                "null", "0",
            ],
        ),
    ));

    /* malformed */
    v.push(("err_string_eof", r#"{"a":"abc"#.to_string()));
    v.push(("err_string_ctrl", "\"ab\u{1}c\"".to_string()));
    v.push(("err_bad_escape", r#""a\qb""#.to_string()));
    v.push(("err_escape_short", r#""\u00""#.to_string()));
    v.push(("err_escape_eof", r#""\"#.to_string()));
    v.push(("err_trailing_junk", "{} xyzzy".to_string()));
    v.push(("err_single_quote", "'a'".to_string()));
    v.push(("err_minus_only", "-".to_string()));
    v.push(("err_dot_digits", "1.".to_string()));
    v.push(("err_exp_digits", "1e".to_string()));
    v.push(("err_exp_digits2", "1e+".to_string()));
    v.push(("err_true_short", "tru".to_string()));
    v.push(("err_false_short", "fals".to_string()));
    v.push(("err_null_short", "nul".to_string()));
    v.push(("err_plus", "+1".to_string()));
    v.push(("err_semicolon", "[1];".to_string()));
    v.push(("err_leading_dot", ".5".to_string()));

    v
}

#[test]
fn cfg68_lexjson_tokens() {
    for (i, (name, src)) in json_corpus().into_iter().enumerate() {
        let src = src.clone();
        diff(&format!("cfg68_{:02}_{}", i, name), move |api| unsafe {
            p_line(name);
            lex_tokens(api, &src, true);
        });
    }
}

/* ================================================================== */
/* rows 64/65: jsP_parse + jsC_compilescript + jsP_freeparse           */
/* ================================================================== */

/// `print` is provided by mujs' main.c, not by the library, so the corpus
/// gets its own: every argument is echoed, which makes the ordering of the
/// compiled code's side effects observable.
unsafe extern "C" fn js_print(J: JS) {
    let api = &*PANIC_API;
    let top = (api.js_gettop)(J);
    p_int("print_argc", top - 1);
    for i in 1..top {
        p_str("print", (api.js_tostring)(J, i));
    }
    (api.js_pushundefined)(J);
}

unsafe fn install_print(api: &Api, J: JS) {
    (api.js_newcfunction)(J, Some(js_print), cs("print").as_ptr(), 1);
    (api.js_setglobal)(J, cs("print").as_ptr());
}

/// Mirror of `js_loadstringx` (jsstate.c) followed by a protected call.
unsafe fn compile_and_run(api: &Api, src: &str, default_strict: c_int) {
    let J = st(api, 0);
    install_print(api, J);
    let file = cs("script.js");
    let source = cs(src);

    let ast = (api.jsP_parse)(J, file.as_ptr(), source.as_ptr());
    p_ptr_nonnull("ast", ast);
    let fun = (api.jsC_compilescript)(J, ast, default_strict);
    p_ptr_nonnull("fun", fun);
    (api.jsP_freeparse)(J);

    (api.js_newscript)(J, fun, null_mut());
    p_int("top_after_newscript", (api.js_gettop)(J));
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    p_int("pcall_rc", rc);
    p_int("top_after_pcall", (api.js_gettop)(J));
    p_str("restype", (api.js_typeof)(J, -1));
    let dflt = cs("<unprintable>");
    p_str("result", (api.js_trystring)(J, -1, dflt.as_ptr()));
    (api.js_pop)(J, 1);
    p_int("top_end", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

fn script_corpus() -> Vec<(&'static str, &'static str)> {
    vec![
        ("empty", ""),
        ("ws_comment", "  // nothing\n/* nothing */\n"),
        ("expr_value", "1 + 1"),
        ("var_decl", "var a = 1, b = 2, c; print(a + b, c, typeof c);"),
        (
            "if_else",
            "if (1) print(\"then\"); else print(\"else\"); if (0) print(\"no\"); else print(\"yes\");",
        ),
        ("do_while", "var i = 0; do { i++; } while (i < 3); print(i);"),
        ("while_stmt", "var i = 0; while (i < 3) i++; print(i);"),
        ("for_var", "for (var i = 0; i < 3; ++i) print(i);"),
        ("for_expr", "var i; for (i = 0; i < 2; i++) print(\"x\" + i);"),
        ("for_empty", "var i = 0; for (;;) { if (++i > 2) break; } print(i);"),
        ("for_in", "var o = {a:1, b:2}; for (var k in o) print(k, o[k]);"),
        ("for_in_expr", "var k; for (k in [10,20]) print(k);"),
        (
            "switch_default_middle",
            "function f(x) { switch (x) { case 1: return \"one\"; default: return \"other\"; case 2: return \"two\"; } } print(f(1), f(2), f(3));",
        ),
        (
            "switch_fallthrough",
            "var s = \"\"; switch (2) { case 1: s += \"a\"; case 2: s += \"b\"; case 3: s += \"c\"; break; case 4: s += \"d\"; } print(s);",
        ),
        (
            "try_catch_finally",
            "try { throw new Error(\"boom\"); } catch (e) { print(e.name, e.message); } finally { print(\"fin\"); }",
        ),
        ("try_finally", "try { print(\"body\"); } finally { print(\"fin\"); }"),
        (
            "try_catch_nested",
            "try { try { null.x; } catch (e) { print(\"inner\", e.name); throw e; } } catch (e2) { print(\"outer\", e2.name); }",
        ),
        (
            "labels",
            "outer: for (var i = 0; i < 3; i++) { inner: for (var j = 0; j < 3; j++) { if (j == 1) continue outer; if (i == 2) break outer; print(i + \",\" + j); } } print(\"done\");",
        ),
        ("label_block", "a: { print(1); break a; } print(2);"),
        ("with_stmt", "var o = {x:42}; with (o) { print(x); } print(typeof x);"),
        (
            "functions",
            "function f(a, b) { return a + b; } var g = function (a) { return a * 2; }; var h = function named(n) { return n < 1 ? 1 : n * named(n - 1); }; print(f(1,2), g(3), h(4));",
        ),
        (
            "getset",
            "var o = { get x() { return 7; }, set x(v) { print(\"set\", v); } }; print(o.x); o.x = 9;",
        ),
        (
            "closures",
            "function outer() { var n = 0; return function () { return ++n; }; } var c = outer(); print(c(), c(), c());",
        ),
        (
            "arguments",
            "function f() { return arguments.length + \":\" + arguments[0] + \":\" + arguments[1]; } print(f(5,6,7), f());",
        ),
        (
            "nested_fundec",
            "function a() { function b() { return 3; } var c = function () { return b() + 1; }; return c(); } print(a());",
        ),
        (
            "regexp_literal",
            "var re = /a(b+)c/g; print(re.source, re.global, re.ignoreCase); print(re.exec(\"xabbbc\")[1]); print(\"aXbXc\".split(/X/).join(\"-\"));",
        ),
        ("array_holes", "var a = [1,,3,]; print(a.length, a[0], a[1], a[2]);"),
        (
            "object_literal",
            "var o = {a:1, \"b\":2, 3:4, }; print(o.a, o.b, o[3]);",
        ),
        ("comma_ternary", "var x = (1, 2, 3); print(x, x > 2 ? \"big\" : \"small\");"),
        (
            "arith_ops",
            "print(1+2, 5-1, 3*4, 10/4, 10%3, 1<<3, 256>>2, -1>>>28, 5&3, 5|3, 5^3, ~5, !0, -(-3), +\"4\");",
        ),
        (
            "cmp_ops",
            "print(1<2, 2<=2, 3>4, 4>=4, 1==\"1\", 1===1, 1!=2, 1!==\"1\", \"a\"<\"b\", null==undefined, NaN==NaN);",
        ),
        (
            "logic_ops",
            "var o = {a:1}; print(typeof o, void 0, delete o.a, \"a\" in o, o instanceof Object, 0||\"x\", 1&&\"y\", !!\"\");",
        ),
        (
            "assign_ops",
            "var x = 10; x += 1; x -= 2; x *= 3; x /= 2; x %= 7; x <<= 2; x >>= 1; x >>>= 1; x &= 0xF; x |= 0x10; x ^= 0x3; print(x);",
        ),
        ("incdec", "var i = 5; print(i++, i, ++i, i--, --i, i);"),
        (
            "new_member_chain",
            "function P(n) { this.n = n; } P.prototype.get = function () { return this.n; }; var p = new P(5); print(p.get(), (new P(6)).n, new P(7).get());",
        ),
        (
            "member_calls",
            "var o = { a: { b: { c: function (x) { return x * 2; } } } }; print(o.a.b.c(4), o[\"a\"][\"b\"].c(5));",
        ),
        ("this_global", "print(typeof this, this === undefined);"),
        ("empty_stmts", ";;{};if(1);print(\"ok\");"),
        ("string_ops", "print(\"a\\tb\".length, 'q' + \"r\", \"abc\".charAt(1), \"abc\".toUpperCase());"),
        ("uncaught_throw", "print(\"before\"); throw new TypeError(\"boom\");"),
        (
            "deep_expr",
            "print(((((1+2)*3)-4)/5) + (6 % 7) - (8 << 1) + (9 >> 1));",
        ),
        (
            "hoisting",
            "print(typeof f); function f() { return 1; } print(typeof g, g); var g = 2; print(g);",
        ),
        /* syntax errors: identical throws are a pass */
        ("err_unexpected", "var = 1;"),
        ("err_unclosed_brace", "if (1) { print(1);"),
        ("err_bad_lvalue", "1 = 2;"),
        ("err_dup_default", "switch (1) { default: break; default: break; }"),
        ("err_break_outside", "break;"),
        ("err_continue_outside", "continue;"),
        ("err_return_outside", "return 1;"),
        ("err_bad_label", "break nosuchlabel;"),
        ("err_dup_property", "var o = { a: 1, a: 2 };"),
        ("err_delete_expr", "delete 1;"),
    ]
}

#[test]
fn cfg64_parse_compilescript_nonstrict() {
    for (i, (name, src)) in script_corpus().into_iter().enumerate() {
        diff(&format!("cfg64_{:02}_{}", i, name), move |api| unsafe {
            p_line(name);
            compile_and_run(api, src, 0);
        });
    }
}

fn strict_only_corpus() -> Vec<(&'static str, &'static str)> {
    vec![
        ("undeclared_assign", "undeclaredvar = 1; print(undeclaredvar);"),
        ("delete_name", "var x = 1; delete x;"),
        ("dup_params", "function f(a, a) { return a; } print(f(1,2));"),
        ("with_stmt", "var o = {}; with (o) { print(1); }"),
        ("octal_literal", "var x = 0755; print(x);"),
        ("eval_assign", "eval = 1;"),
        ("arguments_assign", "function f() { arguments = 1; } f();"),
        ("var_eval", "var eval = 1;"),
        ("var_arguments", "function f() { var arguments = 1; } f();"),
        ("param_eval", "function f(eval) { return eval; } print(f(1));"),
        ("catch_eval", "try { throw 1; } catch (eval) { print(eval); }"),
        ("this_undefined", "function f() { return typeof this; } print(f());"),
        ("future_word", "var implements = 1;"),
        ("strict_future_word", "var interface = 1;"),
    ]
}

#[test]
fn cfg65_parse_compilescript_strict() {
    /* the whole row 64 corpus, compiled with default_strict = 1 */
    for (i, (name, src)) in script_corpus().into_iter().enumerate() {
        diff(&format!("cfg65_{:02}_{}", i, name), move |api| unsafe {
            p_line(name);
            compile_and_run(api, src, 1);
        });
    }
    /* plus sources that are only invalid in strict mode: run both ways */
    for (i, (name, src)) in strict_only_corpus().into_iter().enumerate() {
        diff(&format!("cfg65_so_{:02}_{}_lax", i, name), move |api| unsafe {
            p_line(name);
            compile_and_run(api, src, 0);
        });
        diff(&format!("cfg65_so_{:02}_{}_strict", i, name), move |api| unsafe {
            p_line(name);
            compile_and_run(api, src, 1);
        });
    }
}

/* ================================================================== */
/* row 66: jsP_parsefunction + jsC_compilefunction                     */
/* ================================================================== */

unsafe fn compile_function(api: &Api, params: Option<&str>, body: &str) {
    let J = st(api, 0);
    install_print(api, J);
    let file = cs("fun.js");
    let cparams = params.map(cs);
    let pptr = match &cparams {
        Some(c) => c.as_ptr(),
        None => null(),
    };
    let cbody = cs(body);

    let ast = (api.jsP_parsefunction)(J, file.as_ptr(), pptr, cbody.as_ptr());
    p_ptr_nonnull("ast", ast);
    let fun = (api.jsC_compilefunction)(J, ast);
    p_ptr_nonnull("fun", fun);
    (api.jsP_freeparse)(J);

    /* instantiate in a global-like environment (mirrors jsB_Function) */
    (api.js_pushglobal)(J);
    let g = (api.js_toobject)(J, -1);
    p_ptr_nonnull("global", g);
    (api.js_pop)(J, 1);
    let env = (api.jsR_newenvironment)(J, g, null_mut());
    p_ptr_nonnull("env", env);

    (api.js_newfunction)(J, fun, env);
    let fnidx = (api.js_gettop)(J) - 1;
    p_int("fnidx", fnidx);
    p_int("iscallable", (api.js_iscallable)(J, -1));
    (api.js_getproperty)(J, -1, cs("length").as_ptr());
    p_num("length", (api.js_tonumber)(J, -1));
    (api.js_pop)(J, 1);

    let dflt = cs("<unprintable>");
    for nargs in [0, 1, 5] {
        (api.js_copy)(J, fnidx);
        (api.js_pushundefined)(J);
        for k in 0..nargs {
            (api.js_pushnumber)(J, (k + 1) as f64 * 10.0);
        }
        let rc = (api.js_pcall)(J, nargs as c_int);
        p_int(&format!("call{}_rc", nargs), rc);
        p_str(&format!("call{}_type", nargs), (api.js_typeof)(J, -1));
        p_str(&format!("call{}_res", nargs), (api.js_trystring)(J, -1, dflt.as_ptr()));
        (api.js_pop)(J, 1);
        p_int(&format!("call{}_top", nargs), (api.js_gettop)(J));
    }
    (api.js_freestate)(J);
}

#[test]
fn cfg66_parsefunction_compilefunction() {
    let cases: Vec<(&str, Option<&str>, &str)> = vec![
        ("no_params", None, "return 1;"),
        ("no_params_empty_body", None, ""),
        ("one_param", Some("a"), "return a * 2;"),
        ("one_param_paren", Some("a)"), "return a + 1;"),
        ("five_params", Some("a,b,c,d,e"), "return a + b + c + d + e;"),
        ("five_params_paren", Some("a,b,c,d,e)"), "return [a,b,c,d,e].join(\"-\");"),
        ("ws_params", Some(" a , b "), "return a - b;"),
        ("ws_params_nl", Some("a,\n b"), "return a * b;"),
        ("dup_params", Some("a,a"), "return a;"),
        ("empty_body", Some("a"), ""),
        ("body_return_undef", Some("a"), "return;"),
        ("body_no_return", Some("a"), "a + 1;"),
        ("body_arguments", Some("a"), "return arguments.length + \":\" + a;"),
        ("body_nested", Some("a"), "function g(x) { return x + 1; } return g(a);"),
        ("body_this", None, "return typeof this;"),
        ("body_var", Some("a"), "var b = a; b += 1; return b;"),
        ("body_print", Some("a,b"), "print(\"in\", a, b); return a;"),
        ("body_closure", Some("a"), "var f = function () { return a * 3; }; return f();"),
        ("body_throw", None, "throw new Error(\"inner\");"),
        /* throwing cases */
        ("err_empty_params", Some(""), "return 1;"),
        ("err_params_number", Some("1,2"), "return 1;"),
        ("err_params_op", Some("a+b"), "return 1;"),
        ("err_params_comma", Some("a,"), "return 1;"),
        ("err_params_keyword", Some("var"), "return 1;"),
        ("err_body_syntax", Some("a"), "a +"),
        ("err_body_brace", Some("a"), "if (a) {"),
        ("err_body_break", Some("a"), "break;"),
    ];
    for (i, (name, params, body)) in cases.into_iter().enumerate() {
        diff(&format!("cfg66_{:02}_{}", i, name), move |api| unsafe {
            p_line(name);
            compile_function(api, params, body);
        });
    }
}

/* ================================================================== */
/* row 76: js_intern / jsS_dumpstrings / jsS_freestrings               */
/* ================================================================== */

#[test]
fn cfg76_intern_dumpstrings() {
    diff("cfg76_intern", |api| unsafe {
        let J = st(api, 0);

        let fixed = [
            "", "a", "b", "c", "aa", "ab", "z", "hello", "Hello", "hello world",
            "0", "1", "42", "$", "_", "__proto__", "length", "prototype",
            "a-very-long-string-used-to-exercise-the-node-allocation-path-0123456789",
            "\u{e9}\u{4e2d}\u{1f600}", "caf\u{e9}", "tab\there", "nl\nhere",
        ];

        /* first interning of each string */
        let mut firsts: Vec<*const c_char> = Vec::new();
        for (i, s) in fixed.iter().enumerate() {
            let cstr = cs(s);
            let p = (api.js_intern)(J, cstr.as_ptr());
            p_ptr_nonnull(&format!("intern{}", i), p as *const c_void);
            p_str(&format!("val{}", i), p);
            firsts.push(p);
        }
        /* interning again must return the very same pointer */
        for (i, s) in fixed.iter().enumerate() {
            let cstr = cs(s);
            let p = (api.js_intern)(J, cstr.as_ptr());
            p_int(&format!("same{}", i), (p == firsts[i]) as c_int);
        }
        /* and a distinct buffer with equal contents must intern to it too */
        for (i, s) in fixed.iter().enumerate() {
            let mut owned = String::from(*s);
            owned.push('x');
            owned.pop();
            let cstr = cs(&owned);
            let p = (api.js_intern)(J, cstr.as_ptr());
            p_int(&format!("copy_same{}", i), (p == firsts[i]) as c_int);
        }

        /* 300 pseudo-random strings (AA-tree skew/split coverage) */
        let mut r = Rng::new(0xdead_beef_1234_5678);
        let alpha = b"abcdefghijklmnopqrstuvwxyzABCDEF0123456789_$";
        let mut prev: *const c_char = null();
        let mut prevs = String::new();
        for i in 0..300 {
            let n = 1 + r.range(12) as usize;
            let mut s = String::new();
            for _ in 0..n {
                s.push(alpha[r.range(alpha.len() as u32) as usize] as char);
            }
            let cstr = cs(&s);
            let p = (api.js_intern)(J, cstr.as_ptr());
            p_str(&format!("rnd{}", i), p);
            if i > 0 && s == prevs {
                p_int(&format!("rnd{}_same_as_prev", i), (p == prev) as c_int);
            }
            prev = p;
            prevs = s;
        }

        p_line("--- dump ---");
        (api.jsS_dumpstrings)(J);
        p_line("--- dump again (must be identical) ---");
        (api.jsS_dumpstrings)(J);

        /* jsS_freestrings does not clear J->strings, so the state is dead
         * afterwards: free the table and leave without touching it again. */
        (api.jsS_freestrings)(J);
        p_line("freed");
    });

    /* empty table: dump / free / dump is well defined */
    diff("cfg76_intern_empty", |api| unsafe {
        let J = st(api, 0);
        (api.jsS_dumpstrings)(J);
        (api.jsS_freestrings)(J);
        (api.jsS_dumpstrings)(J);
        p_line("ok");
    });

    /* strings interned by the compiler itself */
    diff("cfg76_intern_compiler", |api| unsafe {
        let J = st(api, 0);
        let file = cs("interned.js");
        let src = cs("var alpha = 1; function beta(gamma) { return gamma + alpha; } beta(2);");
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        p_ptr_nonnull("ast", ast);
        let fun = (api.jsC_compilescript)(J, ast, 0);
        p_ptr_nonnull("fun", fun);
        (api.jsP_freeparse)(J);
        (api.jsS_dumpstrings)(J);
        let p = (api.js_intern)(J, cs("alpha").as_ptr());
        p_str("alpha", p);
        (api.jsS_dumpstrings)(J);
        (api.js_freestate)(J);
    });
}

/* ================================================================== */
/* row 77: js_malloc / js_realloc / js_free / js_strdup                */
/* ================================================================== */

unsafe fn fill(p: *mut u8, n: usize, seed: u8) {
    for i in 0..n {
        *p.add(i) = ((i as u32 * 7 + seed as u32) & 0x7f) as u8;
    }
}

unsafe fn checksum(p: *const u8, n: usize) -> c_int {
    let mut s: u32 = 0;
    for i in 0..n {
        s = s.wrapping_mul(31).wrapping_add(*p.add(i) as u32);
    }
    (s & 0x7fff_ffff) as c_int
}

#[test]
fn cfg77_malloc_realloc_free_strdup() {
    let sizes: [c_int; 14] = [1, 2, 3, 4, 7, 8, 15, 16, 17, 32, 64, 100, 1000, 4096];

    diff("cfg77_plain", |api| unsafe {
        let J = st(api, 0);
        let mut blocks: Vec<(*mut c_void, c_int)> = Vec::new();
        for (i, &n) in sizes.iter().enumerate() {
            let p = (api.js_malloc)(J, n) as *mut u8;
            p_ptr_nonnull(&format!("m{}", i), p as *const c_void);
            fill(p, n as usize, i as u8);
            p_int(&format!("m{}_size", i), n);
            p_int(&format!("m{}_sum", i), checksum(p, n as usize));
            p_int(&format!("m{}_first", i), *p as c_int);
            p_int(&format!("m{}_last", i), *p.add(n as usize - 1) as c_int);
            blocks.push((p as *mut c_void, n));
        }
        /* realloc up: contents of the old prefix must survive */
        for (i, &(p, n)) in blocks.clone().iter().enumerate() {
            let bigger = n * 3 + 1;
            let q = (api.js_realloc)(J, p, bigger) as *mut u8;
            p_ptr_nonnull(&format!("up{}", i), q as *const c_void);
            p_int(&format!("up{}_size", i), bigger);
            p_int(&format!("up{}_prefix_sum", i), checksum(q, n as usize));
            fill(q, bigger as usize, (i + 100) as u8);
            p_int(&format!("up{}_sum", i), checksum(q, bigger as usize));
            blocks[i] = (q as *mut c_void, bigger);
        }
        /* realloc down (never to 0: that would free and then throw) */
        for (i, &(p, n)) in blocks.clone().iter().enumerate() {
            let smaller = if n / 2 < 1 { 1 } else { n / 2 };
            let q = (api.js_realloc)(J, p, smaller) as *mut u8;
            p_ptr_nonnull(&format!("dn{}", i), q as *const c_void);
            p_int(&format!("dn{}_size", i), smaller);
            p_int(&format!("dn{}_sum", i), checksum(q, smaller as usize));
            blocks[i] = (q as *mut c_void, smaller);
        }
        for &(p, _) in blocks.iter() {
            (api.js_free)(J, p);
        }
        (api.js_free)(J, null_mut());
        p_line("freed all");

        /* js_strdup */
        for (i, s) in ["", "a", "abcdefgh", "abcdefghi", "a longer string \u{e9}\u{4e2d}", "0123456789"]
            .iter()
            .enumerate()
        {
            let cstr = cs(s);
            let d = (api.js_strdup)(J, cstr.as_ptr());
            p_ptr_nonnull(&format!("d{}", i), d as *const c_void);
            p_str(&format!("d{}_val", i), d);
            p_int(&format!("d{}_same_ptr", i), (d as *const c_char == cstr.as_ptr()) as c_int);
            (api.js_free)(J, d as *mut c_void);
        }
        (api.js_freestate)(J);
    });

    /* js_malloc(0): js_defaultalloc frees and returns NULL -> out of memory */
    diff("cfg77_malloc_zero", |api| unsafe {
        let J = st(api, 0);
        p_line("before");
        let p = (api.js_malloc)(J, 0);
        p_ptr_nonnull("p", p);
        p_line("after");
        (api.js_freestate)(J);
    });

    /* js_realloc(p, 0) likewise */
    diff("cfg77_realloc_zero", |api| unsafe {
        let J = st(api, 0);
        let p = (api.js_malloc)(J, 16);
        p_ptr_nonnull("p", p);
        p_line("before");
        let q = (api.js_realloc)(J, p, 0);
        p_ptr_nonnull("q", q);
        p_line("after");
        (api.js_freestate)(J);
    });

    /* memlimit set but never reached */
    diff("cfg77_memlimit_ok", |api| unsafe {
        let J = st(api, 0);
        (api.js_setlimit)(J, 0, 1 << 20);
        let mut ps: Vec<*mut c_void> = Vec::new();
        for i in 0..64 {
            let p = (api.js_malloc)(J, 512) as *mut u8;
            fill(p, 512, i as u8);
            p_int(&format!("sum{}", i), checksum(p, 512));
            ps.push(p as *mut c_void);
        }
        for p in ps {
            (api.js_free)(J, p);
        }
        p_line("memlimit not reached");
        (api.js_freestate)(J);
    });

    /* memlimit exceeded: js_malloc throws "out of memory" */
    diff("cfg77_memlimit_exceeded", |api| unsafe {
        let J = st(api, 0);
        (api.js_setlimit)(J, 0, 4096);
        for i in 0..64 {
            p_int("alloc", i);
            let p = (api.js_malloc)(J, 512) as *mut u8;
            fill(p, 512, i as u8);
            p_int(&format!("sum{}", i), checksum(p, 512));
        }
        p_line("not reached");
        (api.js_freestate)(J);
    });

    /* memlimit exceeded through a single oversized request */
    diff("cfg77_memlimit_single", |api| unsafe {
        let J = st(api, 0);
        (api.js_setlimit)(J, 0, 1024);
        p_line("before");
        let p = (api.js_malloc)(J, 1024);
        p_ptr_nonnull("p", p);
        p_line("after");
        (api.js_freestate)(J);
    });

    /* memlimit exceeded by js_realloc */
    diff("cfg77_memlimit_realloc", |api| unsafe {
        let J = st(api, 0);
        let p = (api.js_malloc)(J, 64);
        p_ptr_nonnull("p", p);
        (api.js_setlimit)(J, 0, 128);
        p_line("before");
        let q = (api.js_realloc)(J, p, 4096);
        p_ptr_nonnull("q", q);
        p_line("after");
        (api.js_freestate)(J);
    });

    /* js_strdup under a memlimit that the copy exceeds */
    diff("cfg77_memlimit_strdup", |api| unsafe {
        let J = st(api, 0);
        (api.js_setlimit)(J, 0, 8);
        p_line("before");
        let d = (api.js_strdup)(J, cs("0123456789abcdef").as_ptr());
        p_str("d", d);
        p_line("after");
        (api.js_freestate)(J);
    });
}

/* ================================================================== */
/* row 78: js_putc / js_puts / js_putm                                 */
/* ================================================================== */

/* struct js_Buffer { int n, m; char s[64]; } -- s is at offset 8 */
const BUF_S_OFF: usize = 8;

unsafe fn buf_n(sb: Buf) -> c_int {
    *(sb as *const c_int)
}
unsafe fn buf_m(sb: Buf) -> c_int {
    *((sb as *const c_int).add(1))
}
unsafe fn buf_s(sb: Buf) -> *const c_char {
    (sb as *const u8).add(BUF_S_OFF) as *const c_char
}
unsafe fn p_buf(label: &str, sb: Buf) {
    if sb.is_null() {
        p_line(&format!("{}=<null>", label));
        return;
    }
    p_int(&format!("{}_n", label), buf_n(sb));
    p_int(&format!("{}_m", label), buf_m(sb));
    p_int(
        &format!("{}_sum", label),
        checksum(buf_s(sb) as *const u8, buf_n(sb) as usize),
    );
}

#[test]
fn cfg78_putc_puts_putm() {
    /* start from NULL exactly like the C callers do */
    diff("cfg78_basic", |api| unsafe {
        let J = st(api, 0);
        let mut sb: Buf = null_mut();
        p_ptr_nonnull("sb_initial", sb);
        (api.js_putc)(J, &mut sb, 'a' as c_int);
        p_ptr_nonnull("sb_after_first_putc", sb);
        p_buf("b1", sb);
        (api.js_puts)(J, &mut sb, cs("bcdef").as_ptr());
        p_buf("b2", sb);
        let s = cs("0123456789");
        (api.js_putm)(J, &mut sb, s.as_ptr(), s.as_ptr().add(4));
        p_buf("b3", sb);
        (api.js_putm)(J, &mut sb, s.as_ptr(), s.as_ptr()); /* empty range */
        p_buf("b4", sb);
        (api.js_putc)(J, &mut sb, 0);
        p_str("contents", buf_s(sb));
        p_buf("b5", sb);
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
    });

    /* grow across 64 -> 128 -> ... -> 4096+ in small mixed pieces */
    diff("cfg78_growth", |api| unsafe {
        let J = st(api, 0);
        let mut sb: Buf = null_mut();
        let piece = cs("0123456789abcdef");
        let mut total = 0usize;
        let mut r = Rng::new(0x5157_beef_0000_0001);
        for i in 0..600 {
            match i % 3 {
                0 => {
                    let c = b'A' + (r.range(26) as u8);
                    (api.js_putc)(J, &mut sb, c as c_int);
                    total += 1;
                }
                1 => {
                    (api.js_puts)(J, &mut sb, piece.as_ptr());
                    total += 16;
                }
                _ => {
                    let k = 1 + r.range(8) as usize;
                    (api.js_putm)(J, &mut sb, piece.as_ptr(), piece.as_ptr().add(k));
                    total += k;
                }
            }
            /* report at every doubling boundary we may have crossed */
            if total == 64
                || total == 65
                || (total >= 128 && total <= 130)
                || (total >= 512 && total <= 514)
                || (total >= 1024 && total <= 1026)
                || (total >= 4096 && total <= 4098)
            {
                p_int("total", total as c_int);
                p_buf("grow", sb);
            }
        }
        p_int("total_end", total as c_int);
        p_buf("end", sb);
        (api.js_putc)(J, &mut sb, 0);
        p_str("contents", buf_s(sb));
        p_buf("terminated", sb);
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
    });

    /* exactly hitting the boundary sizes, one byte at a time */
    diff("cfg78_exact_boundaries", |api| unsafe {
        let J = st(api, 0);
        let mut sb: Buf = null_mut();
        for i in 0..5000 {
            (api.js_putc)(J, &mut sb, (b'0' + (i % 10) as u8) as c_int);
            let n = buf_n(sb);
            let m = buf_m(sb);
            if n == m || n == m / 2 + 1 || n <= 2 {
                p_int("n", n);
                p_int("m", m);
            }
        }
        p_buf("final", sb);
        (api.js_putc)(J, &mut sb, 0);
        p_str("contents", buf_s(sb));
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
    });

    /* embedded NULs and high bytes; js_puts stops at NUL */
    diff("cfg78_bytes", |api| unsafe {
        let J = st(api, 0);
        let mut sb: Buf = null_mut();
        for c in [0x41, 0x00, 0x42, 0xff, 0x80, 0x7f, 0x0a, 0x09] {
            (api.js_putc)(J, &mut sb, c as c_int);
        }
        p_buf("raw", sb);
        p_str("upto_first_nul", buf_s(sb));
        let hi = cs("\u{e9}\u{4e2d}");
        (api.js_puts)(J, &mut sb, hi.as_ptr());
        p_buf("hi", sb);
        (api.js_putc)(J, &mut sb, 0);
        p_buf("term", sb);
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
    });

    /* buffer growth under a memlimit that the growth exceeds */
    diff("cfg78_memlimit", |api| unsafe {
        let J = st(api, 0);
        (api.js_setlimit)(J, 0, 200);
        let mut sb: Buf = null_mut();
        for i in 0..400 {
            (api.js_putc)(J, &mut sb, (b'a' + (i % 26) as u8) as c_int);
        }
        p_buf("nope", sb);
        (api.js_freestate)(J);
    });
}
