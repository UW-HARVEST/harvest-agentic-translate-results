//! Phase C — differential ERROR-PATH tests for rows 1..169 of `ERRORS.md`
//! (the `jslex.c` / `jsparse.c` / `jscompile.c` rejection sites).
//!
//! Every case is driven through the two shared libraries only (libloading);
//! nothing calls a Rust function of the crate directly. Sources are plain
//! strings handed to `js_ploadstring` / `js_dostring`-equivalents so that the
//! *exact* error class and message text of both libraries can be compared.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};

/* ------------------------------------------------------------------ helpers */

/// Compile-only result (no execution). Used for sources that are huge or that
/// would run arbitrary code; also the primitive for the fuzz tests. The report
/// hook is silenced so parser warnings do not spam the test log.
fn load_result(a: &Api, src: &str, flags: c_int) -> String {
    let J = a.newstate(flags);
    unsafe {
        (a.js_setreport)(J, None);
        let name = cs("test.js");
        let source = match std::ffi::CString::new(src) {
            Ok(s) => s,
            Err(_) => {
                (a.js_freestate)(J);
                return "<embedded NUL>".to_string();
            }
        };
        let rc = (a.js_ploadstring)(J, name.as_ptr(), source.as_ptr());
        let e = cs("<tostring failed>");
        let out = if rc != 0 {
            format!(
                "load-error({}) {}",
                rc,
                rs((a.js_trystring)(J, -1, e.as_ptr()))
            )
        } else {
            format!("compiled {}", rs((a.js_typeof)(J, -1)))
        };
        (a.js_pop)(J, 1);
        (a.js_freestate)(J);
        out
    }
}

fn trunc(s: &str) -> String {
    if s.len() <= 120 {
        format!("{:?}", s)
    } else {
        format!("{:?}...[{} bytes]", &s[..120.min(s.len())], s.len())
    }
}

/// Compile `src` in both libraries and compare rc + message byte-for-byte.
#[track_caller]
fn diff_load(label: &str, src: &str, flags: c_int) {
    let p = libs();
    let c = load_result(&p.c, src, flags);
    let r = load_result(&p.r, src, flags);
    same(
        &format!("{} | flags={} | src={}", label, flags, trunc(src)),
        &c,
        &r,
    );
}

/// `diff_eval` under BOTH flag settings (`J->strict` gates many rows).
#[track_caller]
fn both(label: &str, src: &str) {
    diff_eval(label, src, 0);
    diff_eval(label, src, JS_STRICT);
}

/// `diff_load` under BOTH flag settings.
#[track_caller]
fn both_load(label: &str, src: &str) {
    diff_load(label, src, 0);
    diff_load(label, src, JS_STRICT);
}

#[track_caller]
fn all(label: &str, srcs: &[&str]) {
    for s in srcs {
        both(label, s);
    }
}

/// `"use strict";` prefixed variant plus the bare variant, each under both
/// state flags — four combinations per logical source.
#[track_caller]
fn all_strictpair(label: &str, srcs: &[&str]) {
    for s in srcs {
        both(label, s);
        both(label, &format!("\"use strict\"; {}", s));
    }
}

fn rep(unit: &str, n: usize) -> String {
    let mut s = String::with_capacity(unit.len() * n);
    for _ in 0..n {
        s.push_str(unit);
    }
    s
}

/// Quote an arbitrary (NUL-free) string as a JavaScript double-quoted literal.
fn js_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/* ============================================================== rows 1 - 2 */
/* sentinel returns reached through the exported low-level symbols */

/// ERRORS.md row 1 — `jsY_tokenstring` out-of-range / gap returns `"<unknown>"`.
#[test]
fn row001_tokenstring_sentinel() {
    let p = libs();
    let mut probe: Vec<c_int> = Vec::new();
    probe.extend(-1000..=300);
    probe.extend(0x80..=0xFF);
    probe.extend([
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        0x85,
        1000,
        65535,
        65536,
        c_int::MAX - 1,
        c_int::MAX,
    ]);
    let mut c = String::new();
    let mut r = String::new();
    for t in probe {
        unsafe {
            c.push_str(&format!("{}={:?};", t, rs((p.c.jsY_tokenstring)(t))));
            r.push_str(&format!("{}={:?};", t, rs((p.r.jsY_tokenstring)(t))));
        }
    }
    same("row1 jsY_tokenstring", &c, &r);
}

/// ERRORS.md row 2 — `jsY_findword` binary-search miss returns `-1`.
#[test]
fn row002_findword_sentinel() {
    /* the three tables jscompile.c / jslex.c search, verbatim and NUL-terminated */
    const KEYWORDS: [&str; 29] = [
        "break\0",
        "case\0",
        "catch\0",
        "continue\0",
        "debugger\0",
        "default\0",
        "delete\0",
        "do\0",
        "else\0",
        "false\0",
        "finally\0",
        "for\0",
        "function\0",
        "if\0",
        "in\0",
        "instanceof\0",
        "new\0",
        "null\0",
        "return\0",
        "switch\0",
        "this\0",
        "throw\0",
        "true\0",
        "try\0",
        "typeof\0",
        "var\0",
        "void\0",
        "while\0",
        "with\0",
    ];
    const FUTURE: [&str; 7] = [
        "class\0",
        "const\0",
        "enum\0",
        "export\0",
        "extends\0",
        "import\0",
        "super\0",
    ];
    const STRICTFUTURE: [&str; 9] = [
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

    fn ptrs(t: &[&'static str]) -> Vec<*const c_char> {
        t.iter().map(|w| w.as_ptr() as *const c_char).collect()
    }
    let kw = ptrs(&KEYWORDS);
    let fw = ptrs(&FUTURE);
    let sw = ptrs(&STRICTFUTURE);

    let needles: Vec<&str> = vec![
        /* misses */
        "foo", "bar", "", "a", "zzz", "brea", "breakk", "Break", "BREAK", "with0", "wit", "clas",
        "supers", "yields", "\u{7f}", "~", "0",
        /* hits, to prove the search itself agrees too */
        "break", "case", "with", "in", "instanceof", "class", "super", "let", "yield", "static",
    ];

    let p = libs();
    let mut c = String::new();
    let mut r = String::new();
    for n in &needles {
        let cn = cs(n);
        for (tag, tab, num) in [
            ("kw", &kw, 29 as c_int),
            ("fw", &fw, 7),
            ("sw", &sw, 9),
            /* degenerate `num` values also exercise the loop bounds */
            ("kw0", &kw, 0),
            ("kw1", &kw, 1),
            ("fwneg", &fw, -1),
        ] {
            unsafe {
                c.push_str(&format!(
                    "{}/{}={};",
                    tag,
                    n,
                    (p.c.jsY_findword)(cn.as_ptr(), tab.as_ptr(), num)
                ));
                r.push_str(&format!(
                    "{}/{}={};",
                    tag,
                    n,
                    (p.r.jsY_findword)(cn.as_ptr(), tab.as_ptr(), num)
                ));
            }
        }
    }
    same("row2 jsY_findword", &c, &r);
}

/* ========================================================= rows 3 - 7 jslex */

/// ERRORS.md rows 3,4,5,6,7 — `jsY_unescape` identifier-position `\u` escapes.
#[test]
fn rows003_007_identifier_escapes() {
    all(
        "row3 \\u bad hex slot 1",
        &["var \\uZ123;", "\\uZ123", "var a\\uZ123;", "function \\uZZZZ(){}"],
    );
    all("row4 \\u bad hex slot 2", &["var \\u0Z12;", "var a\\u0Z12;"]);
    all("row5 \\u bad hex slot 3", &["var \\u00Z1;", "var a\\u00Z1;"]);
    all("row6 \\u bad hex slot 4", &["var \\u000Z;", "var a\\u000Z;"]);
    all(
        "row7 backslash not followed by u",
        &[
            "var \\x41;",
            "var \\q;",
            "\\q",
            "\\",
            "var a\\x41;",
            "var \\;",
            "var \\u;",
            "var \\u0;",
            "var \\u00;",
            "var \\u000;",
        ],
    );
    /* the accepting counterpart: a well formed identifier escape */
    all(
        "row3-7 accepting counterpart",
        &["var \\u0061 = 1; \\u0061", "var a\\u0062c = 2; ab c"],
    );
}

/* ======================================================= rows 8 - 9 comment */

/// ERRORS.md rows 8,9 — `lexcomment` sentinel `-1` and the `jsY_lexx` message.
#[test]
fn rows008_009_unterminated_comment() {
    all(
        "row8/9 unterminated block comment",
        &[
            "/* abc",
            "var a; /* oops",
            "/*",
            "/**",
            "/*/",
            "/* * / ",
            "var a = 1; /* x\ny\nz",
            /* accepting counterparts */
            "/* abc */ 1",
            "/**/1",
            "/* a\nb */ 2",
            "// line comment",
            "1 // trailing",
        ],
    );
}

/* ================================================== rows 10 - 14 numbers */

/// ERRORS.md rows 10,11,12,13,14 — `lexhex` / `lexnumber` rejections + the
/// `'.'`-token sentinel.
#[test]
fn rows010_014_number_lexer() {
    all(
        "row10 malformed hexadecimal number",
        &["0x;", "0xg", "0x", "0X", "0X;", "var a = 0x;", "0x_1"],
    );
    all(
        "row11 number with leading zero",
        &["012", "00", "01", "0123456789", "var a = 08;", "0.0e0 + 012"],
    );
    all(
        "row12 missing exponent",
        &["1e", "1e+", "1E-", "1.5e", "1.5e+", ".5e", "0e", "1e+;", "1e-"],
    );
    all(
        "row13 number with letter suffix",
        &[
            "123abc", "1.5px", "0.1$", "1_", "0x1g", "1e1x", "0abc", "5\\u0061", "1\u{e9}",
        ],
    );
    all(
        "row14 '.' token sentinel (not a number)",
        &["a.b", "a.", ".", "..", "a..b", "1..toString()", "(1).toString"],
    );
    /* accepting numbers, to pin the non-error side of each check */
    all(
        "rows10-14 accepting numbers",
        &[
            "0x1f", "0XFF", "0", "0.5", ".5", "1e5", "1E+5", "1e-5", "1.5e10", "0.0",
        ],
    );
}

/* ================================================== rows 15 - 24 strings */

/// ERRORS.md rows 15,16,17,18,19,20,21,22,23,24 — `lexescape` / `lexstring`.
#[test]
fn rows015_024_string_lexer() {
    all("row15 unterminated escape sequence", &["\"abc\\", "'abc\\", "\"\\"]);
    all("row16 string \\u bad hex 1", &["\"\\uZ123\"", "var a = \"\\uZ123\";"]);
    all("row17 string \\u bad hex 2", &["\"\\u0Z12\""]);
    all("row18 string \\u bad hex 3", &["\"\\u00Z1\""]);
    all("row19 string \\u bad hex 4", &["\"\\u000Z\""]);
    all("row20 string \\x bad hex 1", &["\"\\xZ1\"", "'\\xZ1'"]);
    all("row21 string \\x bad hex 2", &["\"\\x4Z\"", "'\\x4Z'"]);
    all(
        "row22 string not terminated",
        &[
            "\"abc\ndef\"",
            "'abc",
            "\"abc",
            "\"",
            "'",
            "'a\nb'",
            "\"a\r\nb\"",
            "var a = 'x",
        ],
    );
    all(
        "row23 malformed escape sequence",
        &["\"\\xZZ\"", "'\\uZZZZ'", "\"\\x\"", "\"\\u\""],
    );
    /* row 24: `lexstring`'s trailing `jsY_expect(J, q)` is defensive and cannot
     * be reached through any input (the loop only exits on q / newline / EOF,
     * and the latter two throw first). The closest observable behaviour is the
     * accepting path plus row 22, both pinned here. */
    all(
        "row24 defensive expect '<q>' (unreachable) + accepting strings",
        &[
            "\"abc\"",
            "'abc'",
            "\"\\u0041\\x42\\n\\t\\r\\b\\f\\v\\0\\\\\\\"\"",
            "'\\u00e9\\u4e2d'",
            "\"\"",
            "''",
            "\"a\\\nb\"",
        ],
    );
    /* octal / legacy escapes: `\0` is special-cased, other digits are not */
    all_strictpair(
        "rows15-23 octal-ish escapes under both strict settings",
        &["\"\\0\";", "\"\\1\";", "\"\\07\";", "\"\\08\";", "\"\\377\";"],
    );
}

/* ================================================== rows 25 - 29 regexp */

/// ERRORS.md rows 25,26,27,28,29 — `lexregexp`.
#[test]
fn rows025_029_regexp_lexer() {
    all(
        "row25 regexp not terminated",
        &[
            "var r = /abc",
            "var r = /ab\nc/",
            "var r = /",
            "/abc",
            "var r = /[abc",
            "var r = /a[/]",
        ],
    );
    all(
        "row26 regexp not terminated after backslash",
        &["var r = /ab\\", "var r = /ab\\\nc/", "/\\"],
    );
    /* row 27: `lexregexp`'s trailing `jsY_expect(J, '/')` is defensive; the loop
     * can only leave on '/', EOF or newline and the last two throw at row 25.
     * Pinned via the accepting path. */
    all(
        "row27 defensive expect '/' (unreachable) + accepting regexps",
        &["var r = /abc/; r", "/a\\/b/", "/[/]/", "/a/gim", "/(?:a)/"],
    );
    all(
        "row28 illegal flag in regular expression",
        &["/a/x", "/a/y", "/a/1", "/a/gx", "/a/$", "/a/_", "/a/G", "/a/I", "/a/M"],
    );
    all(
        "row29 duplicated flag in regular expression",
        &["/a/gg", "/a/gimg", "/a/ii", "/a/mm", "/a/gig", "/a/mim"],
    );
}

/* ============================================ rows 30 - 31 unexpected char */

/// ERRORS.md rows 30,31 — `jsY_lexx` default case.
#[test]
fn rows030_031_unexpected_character() {
    all(
        "row30 unexpected character (printable ASCII)",
        &["@", "#", "`", "var a = @;", "1 # 2", "a ` b", "?", "a ? b : @"],
    );
    all(
        "row31 unexpected character (non-printable / non-ASCII)",
        &[
            "\u{01}",
            "var a\u{01}",
            "\u{7f}",
            "\u{a1}",
            "1 \u{a1} 2",
            "\u{2028}\u{a1}",
            "\u{ffff}",
            "\u{10000}",
            "\u{fffd}",
        ],
    );
    /* whitespace / line-terminator runes that the lexer DOES accept */
    all(
        "rows30-31 accepted exotic whitespace",
        &[
            "1 ;",
            "\u{a0}1;",
            "\u{2028}1;",
            "\u{2029}1;",
            "\u{feff}1;",
            "\u{0b}1;",
            "\u{0c}1;",
        ],
    );
}

/* ================================================== rows 32 - 42 JSON lexer */

/// ERRORS.md rows 32,33,34,35,36,37,38,39,40,41,42 — the JSON lexer.
#[test]
fn rows032_042_json_lexer() {
    all(
        "row32 JSON unexpected non-digit",
        &[
            "JSON.parse('-x')",
            "JSON.parse('-')",
            "JSON.parse('-.5')",
            "JSON.parse('- 1')",
            "JSON.parse('-e5')",
        ],
    );
    all(
        "row33 JSON missing digits after decimal point",
        &["JSON.parse('1.')", "JSON.parse('1.e5')", "JSON.parse('0.')", "JSON.parse('-1.')"],
    );
    all(
        "row34 JSON missing digits after exponent indicator",
        &[
            "JSON.parse('1e')",
            "JSON.parse('1e+')",
            "JSON.parse('1E-')",
            "JSON.parse('1.5e')",
            "JSON.parse('0e+x')",
        ],
    );
    all(
        "row35 JSON invalid escape sequence",
        &[
            "JSON.parse('\"\\\\q\"')",
            "JSON.parse(\"\\\"\\\\'\\\"\")",
            "JSON.parse('\"\\\\x41\"')",
            "JSON.parse('\"\\\\0\"')",
        ],
    );
    /* rows 36-39: lexjsonescape's return value is DISCARDED by lexjsonstring,
     * so a bad \u silently produces whatever `jsY_tohex` returned. */
    all(
        "row36 JSON \\u bad hex 1 (return discarded)",
        &["JSON.parse('\"\\\\uZ123\"')", "JSON.parse('\"a\\\\uZ123b\"')"],
    );
    all("row37 JSON \\u bad hex 2 (return discarded)", &["JSON.parse('\"\\\\u0Z12\"')"]);
    all("row38 JSON \\u bad hex 3 (return discarded)", &["JSON.parse('\"\\\\u00Z1\"')"]);
    all("row39 JSON \\u bad hex 4 (return discarded)", &["JSON.parse('\"\\\\u000Z\"')"]);
    all(
        "row40 JSON unterminated string",
        &["JSON.parse('\"abc')", "JSON.parse('\"')", "JSON.parse('[\"a')"],
    );
    all(
        "row41 JSON invalid control character in string",
        &[
            "JSON.parse('\"a\tb\"')",
            "JSON.parse('\"a\\u0001b\"')",
            "JSON.parse('\"a\\u001fb\"')",
            "JSON.parse('\"a\nb\"')",
        ],
    );
    /* row 42: lexjsonstring's trailing `jsY_expect(J, '"')` is defensive
     * (the loop leaves only on '"', EOF or a control char). Accepting side: */
    all(
        "row42 defensive JSON expect '\"' (unreachable) + accepting JSON",
        &[
            "JSON.stringify(JSON.parse('\"abc\"'))",
            "JSON.stringify(JSON.parse('\"\\\\u0041\\\\n\\\\t\\\\b\\\\f\\\\r\\\\/\\\\\\\\\\\\\"\"'))",
            "JSON.stringify(JSON.parse('[1,2,3]'))",
            "JSON.stringify(JSON.parse('{\"a\":1}'))",
            "JSON.stringify(JSON.parse('-1.5e+3'))",
            "JSON.stringify(JSON.parse('0'))",
        ],
    );
}

/// ERRORS.md rows 43,44,45,46,47,48,49,50,51,52,53,54 — `jsY_lexjson`
/// keyword spelling and the unexpected-character cases.
#[test]
fn rows043_054_json_keywords() {
    all("row43 JSON expect 'a' in false", &["JSON.parse('fxlse')", "JSON.parse('f')"]);
    all("row44 JSON expect 'l' in false", &["JSON.parse('faxse')", "JSON.parse('fa')"]);
    all("row45 JSON expect 's' in false", &["JSON.parse('falxe')", "JSON.parse('fal')"]);
    all("row46 JSON expect 'e' in false", &["JSON.parse('falsx')", "JSON.parse('fals')"]);
    all("row47 JSON expect 'u' in null", &["JSON.parse('nxll')", "JSON.parse('n')"]);
    all("row48 JSON expect 1st 'l' in null", &["JSON.parse('nuxl')", "JSON.parse('nu')"]);
    all("row49 JSON expect 2nd 'l' in null", &["JSON.parse('nulx')", "JSON.parse('nul')"]);
    all("row50 JSON expect 'r' in true", &["JSON.parse('txue')", "JSON.parse('t')"]);
    all("row51 JSON expect 'u' in true", &["JSON.parse('trxe')", "JSON.parse('tr')"]);
    all("row52 JSON expect 'e' in true", &["JSON.parse('trux')", "JSON.parse('tru')"]);
    all(
        "row53 JSON unexpected character (printable)",
        &[
            "JSON.parse(\"'a'\")",
            "JSON.parse('(')",
            "JSON.parse('+1')",
            "JSON.parse('@')",
            "JSON.parse('*')",
            "JSON.parse('#')",
            "JSON.parse('x')",
            "JSON.parse('')",
            "JSON.parse('undefined')",
            "JSON.parse('NaN')",
        ],
    );
    all(
        "row54 JSON unexpected character (non-printable / non-ASCII)",
        &[
            "JSON.parse('\\u0001')",
            "JSON.parse('\\u00e9')",
            "JSON.parse('\\u4e2d')",
            "JSON.parse('\\u007f')",
            "JSON.parse('\\ufffd')",
        ],
    );
    all(
        "rows43-52 accepting JSON keywords",
        &[
            "JSON.stringify(JSON.parse('true'))",
            "JSON.stringify(JSON.parse('false'))",
            "JSON.stringify(JSON.parse('null'))",
            "JSON.stringify(JSON.parse('[true,false,null]'))",
        ],
    );
}

/* ================================================== row 55 semicolon */

/// ERRORS.md row 55 — `semicolon` (automatic semicolon insertion failure).
#[test]
fn row055_semicolon() {
    all(
        "row55 expected ';'",
        &[
            "var a = 1 var b = 2;",
            "var a = 1 2",
            "a = 1 b = 2",
            "throw 1 2",
            "return 1 2",
            "do ; while(0) 1 2",
            "debugger 1",
            "continue 1",
            "break 1",
            /* accepted by ASI */
            "var a = 1\nvar b = 2;",
            "var a = 1;",
            "{ var a = 1 }",
            "var a = 1",
        ],
    );
}

/* ============================================== rows 56 - 58 identifiers */

/// ERRORS.md rows 56,57,58 — `identifier`, `identifiername`, `identifieropt`.
#[test]
fn rows056_058_identifier_expectations() {
    all(
        "row56 expected identifier",
        &[
            "var 1;",
            "function 2(){}",
            "try{}catch(3){}",
            "({set x(1){}})",
            "for(var ;;) ;",
            "var ;",
            "var \"a\";",
            "function (){}",
            "var if;",
            "({get 1(){}})",
        ],
    );
    all(
        "row57 expected identifier or keyword",
        &[
            "a.1",
            "a.\"x\"",
            "a.;",
            "a.[0]",
            "a..b",
            "({ 1 : 2 }).1",
            "a.+",
            "a.)",
        ],
    );
    /* keywords ARE allowed after '.' and as property names */
    all(
        "row57 accepting keyword member names",
        &[
            "var a = {}; a.if = 1; a.if",
            "var a = {if:1, for:2, function:3}; a.for",
            "({}).while",
            "({}).typeof",
        ],
    );
    all(
        "row58 identifieropt sentinel NULL",
        &[
            "var f = function(){}; typeof f",
            "while(1){break;}",
            "var i=0; while(i<1){++i;continue;}",
            "l: while(1){break l;}",
            "l: for(var j=0;j<1;++j){continue l;}",
            "var f = function g(){}; typeof f",
        ],
    );
}

/* ============================================ rows 59 - 65 empty productions */

/// ERRORS.md rows 59,60,61,62,63,64,65 — the empty-production `NULL` sentinels.
#[test]
fn rows059_065_empty_productions() {
    all("row59 empty array literal", &["[]", "[[]]", "[].length", "var a = []; a.length"]);
    all("row60 empty object literal", &["({})", "({a:{}})", "JSON.stringify({})"]);
    all(
        "row61 empty parameter list",
        &["function f(){} f()", "var f = function(){}; f()", "(function(){})()"],
    );
    all("row62 empty argument list", &["function f(){return 1} f()", "(function(){return 2})()"]);
    all(
        "row63 empty statement list",
        &[
            "{}",
            "switch(1){case 1: case 2: }",
            "{{}}",
            "function f(){{}} f()",
            "switch(1){default:}",
        ],
    );
    all("row64 empty switch body", &["switch(1){}", "switch(1){} 2"]);
    all(
        "row65 empty script / empty function body",
        &["", " ", "\n", "/*x*/", "function f(){} f()", ";"],
    );
}

/* ============================================ rows 66 - 70 object literals */

/// ERRORS.md rows 66,67,68,69,70 — `propassign`.
#[test]
fn rows066_070_propassign() {
    all(
        "row66 getter expected '('",
        &["({ get x 1 })", "({ get x })", "({ get x: 1 })", "({ get x[1](){} })"],
    );
    all(
        "row67 getter expected ')'",
        &["({ get x(a){} })", "({ get x(a,b){} })", "({ get x(1){} })", "({ get x( })"],
    );
    all("row68 setter expected '('", &["({ set x 1 })", "({ set x })", "({ set x: 1 })"]);
    all(
        "row69 setter expected ')'",
        &["({ set x(a,b){} })", "({ set x(a,){} })", "({ set x(){} })", "({ set x(a b){} })"],
    );
    all(
        "row70 property expected ':'",
        &["({ a 1 })", "({ 1 2 })", "({ \"a\" })", "({ a })", "({ a, b })", "({ if })"],
    );
    all_strictpair(
        "rows66-70 accepting accessors",
        &[
            "var o = ({ get x(){return 1}, set x(v){} }); o.x",
            "var o = ({ a:1, \"b\":2, 3:4, if:5 }); o.a + o.b + o[3] + o.if",
            "var o = ({ get:1, set:2 }); o.get + o.set",
        ],
    );
}

/* ============================================ rows 71 - 76 function headers */

/// ERRORS.md rows 71,72,73,74,75,76 — `fundec`, `funstm`, `funexp` headers.
#[test]
fn rows071_076_function_headers() {
    all("row71 fundec expected '('", &["function f {}", "function f;", "function f = 1;"]);
    all(
        "row72 fundec expected ')'",
        &["function f(a {}", "function f(a,{}", "function f(a b){}", "function f("],
    );
    all(
        "row73 funstm expected '('",
        &["if (1) function f {}", "while(0) function f {}", "if (1) function f;"],
    );
    all(
        "row74 funstm expected ')'",
        &["if (1) function f(a {}", "while(0) function f(a b){}", "if (1) function f("],
    );
    all(
        "row75 funexp expected '('",
        &["var f = function {}", "var f = function g {}", "(function {})"],
    );
    all(
        "row76 funexp expected ')'",
        &["var f = function(a {}", "var f = function g(a b){}", "(function(a"],
    );
}

/* ============================================ rows 77 - 81 primary / new */

/// ERRORS.md rows 77,78,79,80,81 — `primary` and `newexp`.
#[test]
fn rows077_081_primary_and_new() {
    all("row77 object literal expected '}'", &["({ a: 1", "({ a: 1,", "({", "({ a: 1 b: 2 })"]);
    all("row78 array literal expected ']'", &["[1, 2", "[", "[1 2]", "[1,"]);
    all("row79 paren expression expected ')'", &["(1 + 2", "(", "(1 2)", "(1,"]);
    all(
        "row80 unexpected token in expression",
        &[
            "var a = ;",
            "+ ;",
            "* 1",
            "a = )",
            "1 + ;",
            "]",
            ")",
            "}",
            ":",
            ",",
            "var a = *;",
            "typeof ;",
            "!;",
        ],
    );
    all("row81 new arguments expected ')'", &["new Foo(1", "new Foo(", "new Foo(1 2)", "new Foo(1,"]);
    all(
        "rows77-81 accepting primaries",
        &[
            "({a:1}).a",
            "[1,2][1]",
            "(1+2)",
            "this === this",
            "null",
            "true",
            "false",
            "new Object()",
            "new Object",
            "typeof new Array(3)",
        ],
    );
}

/* ============================================ rows 82 - 86 member / call */

/// ERRORS.md rows 82,83,84,85,86 — `memberexp` / `callexp` (incl. AST limit).
#[test]
fn rows082_086_member_and_call() {
    /* row 82: >400 member links in a `new` callee */
    let deep_member = format!("new a{}", rep(".b", 500));
    both_load("row82 memberexp too much recursion", &deep_member);
    let ok_member = format!("var a; new a{}", rep(".b", 100));
    both_load("row82 just below the AST limit", &ok_member);

    all("row83 new index expected ']'", &["new a[0", "new a[", "new a[0 1]", "new a[0]["]);

    /* row 84: >400 chained postfix operations */
    let deep_call = format!("a{}", rep("()", 500));
    both_load("row84 callexp too much recursion", &deep_call);
    let deep_dot = format!("a{}", rep(".b", 500));
    both_load("row84 callexp too much recursion (dots)", &deep_dot);
    let deep_idx = format!("a{}", rep("[0]", 500));
    both_load("row84 callexp too much recursion (index)", &deep_idx);
    let ok_call = format!("function a(){{return a}} a{}", rep("()", 50));
    both_load("row84 just below the AST limit", &ok_call);

    all("row85 index expected ']'", &["a[0", "a[1 2]", "a[", "a[0][1"]);
    all("row86 call expected ')'", &["f(1", "f(1 2)", "f(", "f(1,", "f(1)(2"]);
}

/* ==================================== rows 87 - 101 operator AST limits */

/// ERRORS.md rows 87,88,89,90,91,92,93,94,95,96,97,98,99,100,101 — one
/// `INCREC()` site per binary-operator precedence level plus the `?:` colon.
#[test]
fn rows087_101_operator_ast_limits() {
    const N: usize = 500;
    const OK: usize = 40;

    /* (row, label, prefix-unit, operand-chain-unit, tail) */
    let unary_deep = format!("{}x", rep("!", N));
    both_load("row87 unary too much recursion", &unary_deep);
    both_load("row87 just below limit", &format!("{}x", rep("!", OK)));
    both_load(
        "row87 unary mix too much recursion",
        &format!("{}x", rep("typeof ", N)),
    );
    both_load("row87 void chain", &format!("{}x", rep("void ", N)));
    both_load("row87 minus chain", &format!("{}1", rep("-", N)));
    both_load("row87 bitnot chain", &format!("{}1", rep("~", N)));

    let chains: [(&str, &str); 14] = [
        ("row88 multiplicative", "*"),
        ("row88 multiplicative /", "/"),
        ("row88 multiplicative %", "%"),
        ("row89 additive +", "+"),
        ("row89 additive -", "-"),
        ("row90 shift <<", "<<"),
        ("row90 shift >>", ">>"),
        ("row90 shift >>>", ">>>"),
        ("row91 relational <", "<"),
        ("row92 equality ==", "=="),
        ("row93 bitand &", "&"),
        ("row94 bitxor ^", "^"),
        ("row95 bitor |", "|"),
        ("row101 comma ,", ","),
    ];
    for (label, op) in chains {
        let src = format!("1{}", rep(&format!("{}1", op), N));
        both_load(&format!("{} too much recursion", label), &src);
        let ok = format!("1{}", rep(&format!("{}1", op), OK));
        both_load(&format!("{} just below limit", label), &ok);
    }
    /* the remaining relational / equality operators */
    for (label, op) in [
        ("row91 relational >", ">"),
        ("row91 relational <=", "<="),
        ("row91 relational >=", ">="),
        ("row92 equality !=", "!="),
        ("row92 equality ===", "==="),
        ("row92 equality !==", "!=="),
    ] {
        let src = format!("1{}", rep(&format!("{}1", op), N));
        both_load(&format!("{} too much recursion", label), &src);
    }
    both_load(
        "row91 relational instanceof too much recursion",
        &format!("1{}", rep(" instanceof a", N)),
    );
    both_load(
        "row91 relational in too much recursion",
        &format!("1{}", rep(" in a", N)),
    );

    /* rows 96/97: right-recursive && and || */
    both_load("row96 logand too much recursion", &format!("1{}", rep("&&1", N)));
    both_load("row96 logand just below limit", &format!("1{}", rep("&&1", OK)));
    both_load("row97 logor too much recursion", &format!("1{}", rep("||1", N)));
    both_load("row97 logor just below limit", &format!("1{}", rep("||1", OK)));

    /* row 98: nested conditionals */
    both_load(
        "row98 conditional too much recursion",
        &format!("{}1", rep("1?1:", N)),
    );
    both_load(
        "row98 conditional just below limit",
        &format!("{}1", rep("1?1:", 20)),
    );

    /* row 99: conditional missing ':' */
    all(
        "row99 conditional expected ':'",
        &["a ? b c", "a ? b;", "1?2", "1?2 3", "1?"],
    );

    /* row 100: right-nested assignment */
    both_load("row100 assignment too much recursion", &format!("{}1", rep("a=", N)));
    both_load("row100 assignment just below limit", &format!("{}1", rep("a=", OK)));
    both_load(
        "row100 compound assignment too much recursion",
        &format!("a{}1", rep("+=a", N)),
    );

    /* row 101 accepting side already covered; add a comma chain in arguments */
    both_load(
        "row101 comma inside argument list",
        &format!("f({})", rep("1,", N)),
    );
}

/* ============================================ rows 102 - 104 switch bodies */

/// ERRORS.md rows 102,103,104 — `caseclause`.
#[test]
fn rows102_104_switch_clauses() {
    all(
        "row102 case expected ':'",
        &["switch(x){ case 1 break; }", "switch(x){ case 1 }", "switch(x){ case }"],
    );
    all(
        "row103 default expected ':'",
        &["switch(x){ default break; }", "switch(x){ default }", "switch(x){ default 1: }"],
    );
    all(
        "row104 unexpected token in switch",
        &[
            "switch(x){ foo(); }",
            "switch(x){ 1 }",
            "switch(x){ var a; }",
            "switch(x){ ; }",
            "switch(x){ case 1: ; foo(); }",
        ],
    );
    all(
        "rows102-104 accepting switches",
        &[
            "var x=1; switch(x){case 1: 1; break; default: 2}",
            "switch(1){case 1: case 2: default:}",
            "switch(1){}",
        ],
    );
}

/* ============================== rows 105 - 107 block / for-expression */

/// ERRORS.md rows 105,106,107 — `block` and `forexpression`.
#[test]
fn rows105_107_block_and_forexpression() {
    all(
        "row105 block expected '{'",
        &[
            "try 1; catch(e){}",
            "try {} catch(e) 1;",
            "try {} finally 1;",
            "try ;",
        ],
    );
    all(
        "row106 block expected '}'",
        &["{ var a;", "{", "{ var a; { }", "try {", "function f(){ { }"],
    );
    all(
        "row107 forexpression expected ';' / ')'",
        &[
            "for(;1 2;) ;",
            "for(;;1 2) ;",
            "for(;;",
            "for(;",
            "for(;1;",
            "for(var a;1 2;) ;",
            "for(var a;;1 2) ;",
        ],
    );
    all(
        "rows105-107 accepting for/try",
        &[
            "for(;;)break;",
            "for(var i=0;i<1;++i);",
            "for(var i=0;;)break;",
            "try{}catch(e){}",
            "try{}finally{}",
            "try{}catch(e){}finally{}",
        ],
    );
}

/* ============================================ rows 108 - 112 for statement */

/// ERRORS.md rows 108,109,110,111,112 — `forstatement`.
#[test]
fn rows108_112_forstatement() {
    all("row108 for expected '('", &["for x;;) ;", "for ;", "for", "for var a;;) ;"]);
    all(
        "row109 for-var-in expected ')'",
        &["for(var a in b ;", "for(var a in b", "for(var a in b c) ;"],
    );
    all(
        "row110 unexpected token in for-var-statement",
        &["for(var a b) ;", "for(var a)", "for(var a", "for(var a,b c) ;"],
    );
    all(
        "row111 for-in expected ')'",
        &["for(a in b ;", "for(a in b", "for(a in b c) ;"],
    );
    all(
        "row112 unexpected token in for-statement",
        &["for(a b) ;", "for(a)", "for(a", "for(1 2;;) ;"],
    );
    all(
        "rows108-112 accepting for-in",
        &[
            "for(var k in {a:1});",
            "for(var k in {a:1}){}",
            "var k; for(k in {a:1});",
            "var o={},k; for(o[k] in {a:1});",
        ],
    );
}

/* ============================ rows 113 - 120 statement headers + AST limit */

/// ERRORS.md rows 113,114,115,116,117,118,119,120 — `statement` head tokens.
#[test]
fn rows113_120_statement_headers() {
    /* row 113: >400 nested statements */
    let deep_block = format!("{}{}", rep("{", 500), rep("}", 500));
    both_load("row113 nested blocks too much recursion", &deep_block);
    both_load(
        "row113 nested if too much recursion",
        &format!("{};", rep("if(1)", 500)),
    );
    both_load(
        "row113 nested while too much recursion",
        &format!("{};", rep("while(0)", 500)),
    );
    both_load(
        "row113 nested labels too much recursion",
        &format!("{};", rep("l:", 500)),
    );
    both_load(
        "row113 just below limit",
        &format!("{}{}", rep("{", 50), rep("}", 50)),
    );

    all("row114 if expected '('", &["if x ;", "if ;", "if", "if 1) ;"]);
    all("row115 if expected ')'", &["if (x ;", "if (", "if (x", "if (x y) ;"]);
    all(
        "row116 do expected 'while'",
        &["do ; until (0);", "do ;", "do ; whil(0);", "do {} 1"],
    );
    all("row117 do-while expected '('", &["do ; while 0;", "do ; while;", "do ; while"]);
    all("row118 do-while expected ')'", &["do ; while (0 ;", "do ; while (", "do ; while (0"]);
    all("row119 while expected '('", &["while x ;", "while ;", "while"]);
    all("row120 while expected ')'", &["while (x ;", "while (", "while (x"]);
    all(
        "rows113-120 accepting statements",
        &[
            "if(1);",
            "if(1);else;",
            "do ; while(0);",
            "var i=0; while(i){--i}",
            "l: { break l; }",
        ],
    );
}

/* ================== rows 121 - 129 with / switch / catch / try headers */

/// ERRORS.md rows 121,122,123,124,125,126,127,128,129.
#[test]
fn rows121_129_with_switch_try_headers() {
    all("row121 with expected '('", &["with x ;", "with ;", "with"]);
    all("row122 with expected ')'", &["with (x ;", "with (", "with (x"]);
    all("row123 switch expected '('", &["switch x {}", "switch ;", "switch"]);
    all("row124 switch expected ')'", &["switch (x {}", "switch (", "switch (x"]);
    all(
        "row125 switch expected '{'",
        &["switch (x) case 1: ;", "switch (x) ;", "switch (x)"],
    );
    all("row126 switch expected '}'", &["switch (x) {", "switch (x) { case 1: "]);
    all("row127 catch expected '('", &["try{}catch e {}", "try{}catch{}", "try{}catch"]);
    all("row128 catch expected ')'", &["try{}catch(e {}", "try{}catch(e", "try{}catch("]);
    all(
        "row129 unexpected token in try",
        &["try {}", "try {} 1", "try {} else {}", "try {} ;"],
    );
    /* `with` in non-strict code is legal, in strict code it is row 169 */
    all_strictpair("rows121-122 accepting with", &["var o={a:1}; with(o){a}"]);
}

/* ============================================ row 130 parser warning */

/// ERRORS.md row 130 — `jsP_warning` (non-fatal `js_report`, parsing continues).
#[test]
fn row130_function_statement_warning() {
    unsafe extern "C" fn report(_J: JS, msg: *const c_char) {
        emit(&format!("report:{}", unsafe { rs(msg) }));
    }
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_setreport)(J, Some(report));
            let name = cs("test.js");
            let src = ps(0);
            let rc = (a.js_ploadstring)(J, name.as_ptr(), src.as_ptr());
            emit(&format!("rc={}", rc));
            if rc != 0 {
                let e = cs("<tostring failed>");
                emit(&format!("err={}", str_at(a, J, -1)));
                let _ = e;
            } else {
                emit(&format!("typeof={}", rs((a.js_typeof)(J, -1))));
            }
            (a.js_pop)(J, 1);
        }
    }
    for src in [
        "if (1) function f(){}",
        "while(0) function f(){}",
        "if (1) function f(){} else function g(){}",
        "for(;;) function f(){}",
        "l: function f(){}",
        /* no warning: a script-element function declaration */
        "function f(){}",
        /* warning then a hard error */
        "if (1) function f {}",
        /* warning inside a function body */
        "function h(){ if(1) function f(){} }",
    ] {
        set_ps(0, src);
        diff_native(&format!("row130 warning src={:?}", src), act, 0);
        diff_native(&format!("row130 warning strict src={:?}", src), act, JS_STRICT);
    }
}

/* ============================================ rows 131 - 132 function body */

/// ERRORS.md rows 131,132 — `funbody`.
#[test]
fn rows131_132_funbody() {
    all(
        "row131 funbody expected '{'",
        &[
            "function f() return 1;",
            "function f() 1;",
            "function f()",
            "var f = function() 1;",
            "({get x() 1})",
        ],
    );
    all(
        "row132 funbody expected '}'",
        &[
            "function f(){",
            "function f(){ var a;",
            "var f = function(){",
            "({get x(){ })",
            "function f(){ function g(){",
        ],
    );
}

/* ============================================ row 133 instruction coding */

/// ERRORS.md row 133 — `emitraw` line number does not round-trip through
/// `js_Instruction` (`unsigned short`).
#[test]
fn row133_instruction_coding_overflow() {
    /* a statement on line 70001 => emit() writes lastline == 70001 > 65535 */
    let mut src = String::with_capacity(70_100);
    for _ in 0..70_000 {
        src.push('\n');
    }
    src.push_str("a;");
    both_load("row133 line number > 65535", &src);

    /* just below the limit: line 65000 still fits */
    let mut ok = String::with_capacity(65_100);
    for _ in 0..64_000 {
        ok.push('\n');
    }
    ok.push_str("a;");
    both_load("row133 line number below 65536", &ok);
}

/* ============================================ rows 134 - 135 future words */

/// ERRORS.md rows 134,135 — `checkfutureword`.
#[test]
fn rows134_135_future_reserved_words() {
    for w in [
        "class", "const", "enum", "export", "extends", "import", "super",
    ] {
        all(
            "row134 future reserved word",
            &[
                &format!("var {};", w),
                &format!("function {}(){{}}", w),
                &format!("break {};", w),
                &format!("{} = 1;", w),
                &format!("function f({}){{}}", w),
                &format!("try{{}}catch({}){{}}", w),
                &format!("{}: while(1) break {};", w, w),
                &format!("var a = {};", w),
                &format!("typeof {};", w),
                &format!("delete {};", w),
            ],
        );
    }
    for w in [
        "implements",
        "interface",
        "let",
        "package",
        "private",
        "protected",
        "public",
        "static",
        "yield",
    ] {
        /* only rejected when the *function* is strict */
        all(
            "row135 strict future reserved word",
            &[
                &format!("var {};", w),
                &format!("\"use strict\"; var {};", w),
                &format!("\"use strict\"; function f({}){{}}", w),
                &format!("\"use strict\"; {} = 1;", w),
                &format!("function f(){{\"use strict\"; var {};}}", w),
                &format!("\"use strict\"; try{{}}catch({}){{}}", w),
                &format!("\"use strict\"; {}: while(1) break {};", w, w),
            ],
        );
    }
    /* these names are only special as identifiers, not as property names */
    all(
        "rows134-135 accepting reserved words as property names",
        &[
            "var o = {class:1, const:2, let:3}; o.class + o.const + o.let",
            "({}).super",
        ],
    );
}

/* ============================================ rows 136 - 139 addlocal */

/// ERRORS.md rows 136,137,138,139 — `addlocal`.
#[test]
fn rows136_139_addlocal() {
    all(
        "row136 redefining 'arguments' in strict mode",
        &[
            "\"use strict\"; var arguments;",
            "\"use strict\"; function f(arguments){}",
            "\"use strict\"; function arguments(){}",
            "function f(){\"use strict\"; var arguments;}",
            "var arguments;",
            "function f(arguments){}",
        ],
    );
    all(
        "row137 redefining 'eval' in strict mode",
        &[
            "\"use strict\"; var eval;",
            "\"use strict\"; function f(eval){}",
            "\"use strict\"; function eval(){}",
            "function f(){\"use strict\"; var eval;}",
        ],
    );
    all(
        "row138 invalid use of 'eval' (non-strict declaration)",
        &[
            "var eval;",
            "function f(eval){}",
            "function eval(){}",
            "var eval = 1;",
            "function f(a, eval){}",
        ],
    );
    all(
        "row139 duplicate formal parameter",
        &[
            "\"use strict\"; function f(a,a){}",
            "\"use strict\"; function f(a,b,a){}",
            "function f(a,a){}",
            "function f(a,b,a){ return a }",
            "\"use strict\"; var f = function(a,a){};",
            "\"use strict\"; function f(a,a,a){}",
        ],
    );
    /* `reuse` path: duplicate *var* declarations are legal even in strict mode */
    all(
        "rows136-139 accepting duplicate var declarations",
        &[
            "var a; var a; a",
            "\"use strict\"; var a; var a; a",
            "function f(){var a; var a; return a} f()",
            "\"use strict\"; function f(){var a; var a; return a} f()",
        ],
    );
}

/* ============================================ row 140 findlocal sentinel */

/// ERRORS.md row 140 — `findlocal` returns `-1` for free/global variables so
/// `emitlocal` emits the `OP_*VAR` string form.
#[test]
fn row140_findlocal_global_sentinel() {
    all(
        "row140 free variable emits OP_*VAR",
        &[
            "undeclaredGlobal;",
            "typeof undeclaredGlobal",
            "function f(){ return undeclaredGlobal } typeof f",
            "function f(){ return undeclaredGlobal } f()",
            "function f(){ undeclaredGlobal = 1 } f(); undeclaredGlobal",
            "function f(){ var a = 1; return a } f()",
            "function f(a){ return a } f(7)",
            "var g = 1; function f(){ return g } f()",
            "undeclaredGlobal",
        ],
    );
}

/* ============================================ rows 141 - 143 emitlocal */

/// ERRORS.md rows 141,142,143 — `emitlocal` strict read-only + `eval` misuse.
#[test]
fn rows141_143_emitlocal() {
    all(
        "row141 'arguments' is read-only in strict mode",
        &[
            "\"use strict\"; function f(){ arguments = 1; }",
            "\"use strict\"; function f(){ arguments += 1; }",
            "\"use strict\"; function f(){ ++arguments; }",
            "\"use strict\"; function f(){ arguments++; }",
            "function f(){ arguments = 1; } f()",
            "arguments = 1;",
        ],
    );
    all(
        "row142 'eval' is read-only in strict mode",
        &["\"use strict\"; eval = 1;", "\"use strict\"; function f(){ eval = 1; }"],
    );
    all(
        "row143 invalid use of 'eval'",
        &[
            "eval = 1;",
            "var x = eval;",
            "delete eval;",
            "typeof eval;",
            "eval;",
            "eval.call;",
            "function f(){ return eval }",
            "eval += 1;",
            "++eval;",
        ],
    );
    /* the CALL form of eval is the one shape that is allowed */
    all(
        "rows141-143 accepting eval call",
        &["eval('1+1')", "eval('var a=1; a')", "\"use strict\"; eval('1')"],
    );
}

/* ============================================ rows 144 - 145 jump overflow */

/// ERRORS.md rows 144,145 — `emitjumpto` / `labelto` jump address overflow.
#[test]
fn rows144_145_jump_address_overflow() {
    /* Each `a;` statement emits ~5 instructions, so 20000 of them push the
     * loop's backward jump target well past 65535. */
    let body = rep("a;", 20_000);

    /* row 144: the backward `OP_JUMP` of a while loop */
    both_load(
        "row144 backward jump address overflow",
        &format!("while(1){{{}}}", body),
    );
    both_load(
        "row144 backward jump address overflow (do-while)",
        &format!("do{{{}}}while(1);", body),
    );

    /* row 145: a forward patch (`labelto`) with no preceding backward jump */
    both_load(
        "row145 forward jump address overflow (if)",
        &format!("if(x){{{}}}", body),
    );
    both_load(
        "row145 forward jump address overflow (&&)",
        &format!("x && (function(){{{}}})();", body),
    );

    /* below the limit both compile */
    let small = rep("a;", 1_000);
    both_load(
        "rows144-145 below the limit",
        &format!("while(1){{{}}} if(x){{{}}}", small, small),
    );
}

/* ============================================ rows 146 - 147 object keys */

/// ERRORS.md rows 146,147 — `checkdup` and `cobject`'s defensive default.
#[test]
fn rows146_147_object_literal_keys() {
    all(
        "row146 duplicate property in object literal",
        &[
            "\"use strict\"; ({a:1, a:2})",
            "\"use strict\"; ({1:1, 1.0:2})",
            "\"use strict\"; ({\"a\":1, a:2})",
            "\"use strict\"; ({a:1, get a(){}})",
            "\"use strict\"; ({get a(){}, get a(){}})",
            "\"use strict\"; ({get a(){}, set a(v){}})",
            "\"use strict\"; ({1:1, \"1\":2})",
            "\"use strict\"; ({0.5:1, \".5\":2})",
            /* non-strict: duplicates are allowed */
            "({a:1, a:2}).a",
            "({1:1, 1.0:2})[1]",
            "\"use strict\"; ({a:1, b:2}).a",
        ],
    );
    /* row 147: `cobject`'s `invalid property name in object initializer` is
     * defensive — `propname` only ever yields AST_IDENTIFIER / EXP_STRING /
     * EXP_NUMBER. Every propname shape is pinned here instead. */
    all_strictpair(
        "row147 defensive invalid property name (unreachable)",
        &[
            "({a:1}).a",
            "({\"a\":1}).a",
            "({1:1})[1]",
            "({0:1})[0]",
            "({1e21:1})[1e21]",
            "({if:1}).if",
            "({null:1}).null",
            "({true:1}).true",
        ],
    );
}

/* ============================================ rows 148 - 152 l-values */

/// ERRORS.md rows 148,149,150,151,152 — assignment / for-in / compound-assign
/// l-value checks.
#[test]
fn rows148_152_invalid_lvalues() {
    all(
        "row148 invalid l-value in assignment",
        &[
            "1 = 2;",
            "f() = 1;",
            "(a,b) = 1;",
            "this = 1;",
            "\"s\" = 1;",
            "null = 1;",
            "true = 1;",
            "[1] = 2;",
            "({}) = 1;",
            "-a = 1;",
            "(a+b) = 1;",
            "/re/ = 1;",
        ],
    );
    all(
        "row149 more than one loop variable in for-in",
        &[
            "for (var a, b in c) ;",
            "for (var a, b, c in d) ;",
            "for (var a=1, b in c) ;",
        ],
    );
    all(
        "row150 invalid l-value in for-in loop assignment",
        &[
            "for (1 in x) ;",
            "for (f() in x) ;",
            "for (this in x) ;",
            "for (\"a\" in x) ;",
            "for ((a,b) in x) ;",
            "for (-a in x) ;",
        ],
    );
    all(
        "row151 invalid l-value in compound assignment",
        &[
            "1 += 2;",
            "1++;",
            "--f();",
            "this += 1;",
            "f() -= 1;",
            "++1;",
            "1--;",
            "(a+b) *= 2;",
            "null ^= 1;",
            "\"s\" >>>= 1;",
            "[1]++;",
            "({})--;",
        ],
    );
    /* row 152: `cassignop2` fires only if `cassignop1` did not — unreachable
     * for every input, since both switch on the same node type. The accepting
     * l-value shapes are pinned instead. */
    all(
        "row152 defensive store-phase l-value (unreachable) + accepting l-values",
        &[
            "var a=1; a=2; a",
            "var o={x:1}; o.x=2; o.x",
            "var o={}; o['y']=3; o.y",
            "var a=1; a+=1; a",
            "var o={x:1}; o.x+=1; o.x",
            "var o={x:1}; ++o.x; o.x",
            "var o={x:1}; o.x++; o.x",
            "var a=[1]; a[0]++; a[0]",
        ],
    );
}

/* ============================================ rows 153 - 154 delete */

/// ERRORS.md rows 153,154 — `cdelete`.
#[test]
fn rows153_154_delete() {
    all(
        "row153 delete on an unqualified name in strict mode",
        &[
            "\"use strict\"; delete x;",
            "\"use strict\"; function f(){ var a; delete a; }",
            "\"use strict\"; delete undefinedName;",
            /* non-strict: allowed */
            "delete x;",
            "var a; delete a;",
        ],
    );
    all(
        "row154 invalid l-value in delete expression",
        &[
            "delete 1;",
            "delete f();",
            "delete this;",
            "delete \"s\";",
            "delete (a,b);",
            "delete -a;",
            "delete [1];",
            "delete null;",
            "delete delete a;",
        ],
    );
    all(
        "rows153-154 accepting deletes",
        &[
            "var o={x:1}; delete o.x; o.x",
            "var o={x:1}; delete o['x']; typeof o.x",
            "\"use strict\"; var o={x:1}; delete o.x; typeof o.x",
        ],
    );
}

/* ============================================ row 155 unknown exp type */

/// ERRORS.md row 155 — `cexp`'s `unknown expression type` default branch
/// (defensive: no `propname` / `statement` shape reaches it).
#[test]
fn row155_unknown_expression_type() {
    /* every statement shape that funnels through `cstm`'s `default: cexp(...)`
     * branch, plus the AST node types the row mentions in the positions where
     * they could conceivably leak. All must behave identically. */
    all_strictpair(
        "row155 defensive unknown expression type (unreachable)",
        &[
            "1;",
            "a;",
            "a.b;",
            "a[0];",
            "f();",
            "new f();",
            "(1,2);",
            "a?b:c;",
            "typeof a;",
            "void 0;",
            "-1;",
            "!0;",
            "1+1;",
            "a=1;",
            "a+=1;",
            "++a;",
            "a++;",
            "/re/;",
            "({a:1});",
            "[1,2];",
            "(function(){});",
            "this;",
            "null;",
            "true;",
            "false;",
            "\"s\";",
            "l: 1;",
            "l: ;",
        ],
    );
}

/* ==================== rows 156 - 158 break/continue/return target sentinels */

/// ERRORS.md rows 156,157,158 — `breaktarget` / `continuetarget` /
/// `returntarget` returning `NULL`.
#[test]
fn rows156_158_jump_target_sentinels() {
    all(
        "row156 breaktarget NULL",
        &[
            "break;",
            "function f(){ break; }",
            "x: { function g(){ break x; } }",
            "x: while(1){ function g(){ break x; } }",
            "break x;",
            "if(1) break;",
        ],
    );
    all(
        "row157 continuetarget NULL",
        &[
            "continue;",
            "switch(x){case 1: continue;}",
            "function f(){ continue; }",
            "x: { continue x; }",
            "continue x;",
            "x: while(1){ function g(){ continue x; } }",
        ],
    );
    all(
        "row158 returntarget NULL",
        &["return 1;", "return;", "if(1) return;", "{ return; }", "while(0) return;"],
    );
    all(
        "rows156-158 accepting jumps",
        &[
            "while(1) break;",
            "var i=0; while(i<1){++i; continue;} i",
            "switch(1){case 1: break;}",
            "x: while(1){ break x; }",
            "x: for(var i=0;i<1;++i){ continue x; }",
            "function f(){ return 1 } f()",
            "function f(){ return } typeof f()",
            "(function(){ while(1) break; return 2 })()",
            "({get x(){ return 3 }}).x",
        ],
    );
}

/* ============================ rows 159 - 162 strict catch parameter */

/// ERRORS.md rows 159,160,161,162 — `ctrycatch` / `ctrycatchfinally`.
#[test]
fn rows159_162_strict_catch_parameter() {
    all(
        "row159 catch(arguments) strict, no finally",
        &[
            "\"use strict\"; try{}catch(arguments){}",
            "function f(){\"use strict\"; try{}catch(arguments){}}",
            "try{}catch(arguments){}",
        ],
    );
    all(
        "row160 catch(eval) strict, no finally",
        &[
            "\"use strict\"; try{}catch(eval){}",
            "function f(){\"use strict\"; try{}catch(eval){}}",
            "try{}catch(eval){}",
        ],
    );
    all(
        "row161 catch(arguments) strict, with finally",
        &[
            "\"use strict\"; try{}catch(arguments){}finally{}",
            "function f(){\"use strict\"; try{}catch(arguments){}finally{}}",
            "try{}catch(arguments){}finally{}",
        ],
    );
    all(
        "row162 catch(eval) strict, with finally",
        &[
            "\"use strict\"; try{}catch(eval){}finally{}",
            "function f(){\"use strict\"; try{}catch(eval){}finally{}}",
            "try{}catch(eval){}finally{}",
        ],
    );
    all(
        "rows159-162 accepting catch parameters",
        &[
            "try{throw 1}catch(e){e}",
            "\"use strict\"; try{throw 1}catch(e){e}",
            "try{throw 1}catch(e){e}finally{}",
            "\"use strict\"; try{throw 1}catch(e){e}finally{}",
        ],
    );
}

/* ============================================ row 163 default labels */

/// ERRORS.md row 163 — `cswitch` more than one `default`.
#[test]
fn row163_more_than_one_default() {
    all(
        "row163 more than one default label in switch",
        &[
            "switch(x){default: ; default: ;}",
            "switch(x){default:; case 1:; default:;}",
            "switch(x){default:; default:; default:;}",
            /* accepting */
            "switch(1){default:;}",
            "switch(1){case 1:; default:;}",
        ],
    );
}

/* ============ rows 164 - 169 break/continue label, return, strict with */

/// ERRORS.md rows 164,165,166,167,168,169 — `cstm`'s label / return / with
/// checks.
#[test]
fn rows164_169_labels_return_with() {
    all(
        "row164 break label not found",
        &[
            "foo: while(1){} break bar;",
            "function f(){ break foo; }",
            "foo: while(1){ break bar; }",
            "break bar;",
            "foo: { bar: { break baz; } }",
        ],
    );
    all(
        "row165 unlabelled break must be inside loop or switch",
        &["break;", "if(1) break;", "function f(){ break; }", "{ break; }", "foo: { break; }"],
    );
    all(
        "row166 continue label not found",
        &[
            "foo: { continue foo; }",
            "continue bar;",
            "foo: while(1){ continue bar; }",
            "function f(){ continue foo; }",
            "foo: switch(1){case 1: continue foo;}",
        ],
    );
    all(
        "row167 continue must be inside loop",
        &[
            "continue;",
            "switch(x){case 1: continue;}",
            "function f(){ continue; }",
            "{ continue; }",
            "foo: { continue; }",
        ],
    );
    all(
        "row168 return not in function",
        &[
            "return;",
            "return 1;",
            "{ return; }",
            "if(1) return 1;",
            "try{ return }catch(e){}",
            "while(0){ return }",
        ],
    );
    all(
        "row169 'with' in strict mode",
        &[
            "\"use strict\"; with(x){}",
            "\"use strict\"; function f(){ with(x){} }",
            "with(x){}",
            "\"use strict\"; with({a:1}){a}",
        ],
    );
    /* `J->default_strict` (the JS_STRICT state flag) must gate row 169 too —
     * `all()` already runs every source with flags=0 and flags=JS_STRICT. */
    all(
        "rows164-169 accepting labels/return/with",
        &[
            "foo: while(1){ break foo; }",
            "foo: for(var i=0;i<1;++i){ continue foo; }",
            "foo: switch(1){case 1: break foo;}",
            "function f(){ return 1 } f()",
            "var o={a:5}; with(o){ a }",
        ],
    );
}

/* ================================================================== fuzzing */

const SEED: u64 = 0x5C_A11_ED_7E57;

/// Seed corpus for the mutation fuzzer: one snippet per interesting rejection
/// shape from the tables above, so a single random edit lands near a real
/// error site far more often than a uniformly random string would.
const CORPUS: [&str; 98] = [
    "var \\u0061;",
    "var \\uZ123;",
    "var \\x41;",
    "/* abc */ 1",
    "0x1f",
    "012",
    "1e5",
    "123abc",
    "a.b",
    "\"abc\\\\\"",
    "\"\\u0041\"",
    "\"\\x41\"",
    "'abc'",
    "\"a\nb\"",
    "var r = /abc/gim;",
    "/a/gg",
    "/a/x",
    "@",
    "\u{a1}",
    "JSON.parse('-x')",
    "JSON.parse('1.')",
    "JSON.parse('1e')",
    "JSON.parse('\"\\\\q\"')",
    "JSON.parse('\"abc')",
    "JSON.parse('falsx')",
    "JSON.parse('nulx')",
    "JSON.parse('trux')",
    "var a = 1 var b = 2;",
    "var 1;",
    "a.1",
    "[]",
    "({})",
    "function f(){}",
    "f()",
    "{}",
    "switch(x){}",
    "({ get x(){return 1} })",
    "({ set x(v){} })",
    "({ a: 1 })",
    "function f {}",
    "function f(a {}",
    "if (1) function f {}",
    "var f = function {}",
    "({ a: 1",
    "[1, 2",
    "(1 + 2",
    "var a = ;",
    "new Foo(1",
    "new a[0",
    "a[0",
    "f(1",
    "!!x",
    "1*1*1",
    "1+1+1",
    "1<<1",
    "1<1",
    "1==1",
    "1&1",
    "1^1",
    "1|1",
    "1&&1",
    "1||1",
    "1?1:1",
    "a ? b c",
    "a=a=1",
    "1,1,1",
    "switch(x){ case 1 break; }",
    "switch(x){ default break; }",
    "switch(x){ foo(); }",
    "try 1; catch(e){}",
    "{ var a;",
    "for(;1 2;) ;",
    "for x;;) ;",
    "for(var a in b ;",
    "for(var a b) ;",
    "for(a in b ;",
    "for(a b) ;",
    "if x ;",
    "if (x ;",
    "do ; until (0);",
    "do ; while 0;",
    "while x ;",
    "with x ;",
    "switch x {}",
    "switch (x) case 1: ;",
    "try{}catch e {}",
    "try {}",
    "function f() return 1;",
    "function f(){",
    "var class;",
    "var let;",
    "var arguments;",
    "var eval;",
    "function f(a,a){}",
    "({a:1, a:2})",
    "1 = 2;",
    "for (var a, b in c) ;",
    "delete 1;",
];

/// Mutation fuzzer over the seed corpus: splice / delete / duplicate / insert
/// random bytes and tokens into known-interesting sources and require the two
/// libraries to agree on every result.
#[test]
fn fuzz_corpus_mutation() {
    const INSERTS: [&str; 40] = [
        "\\", "\\u", "\\u00", "\\x", "\"", "'", "/", "/*", "*/", "//", "(", ")", "{", "}", "[",
        "]", ";", ",", ":", "?", ".", "=", "0x", "0", "1e", ".", "e+", "use strict", "\n", "\t",
        " ", "@", "#", "`", "\u{a1}", "\u{4e2d}", "\u{01}", "get", "set", "function",
    ];
    let mut rng = Rng::new(SEED ^ 0xDEAD_BEEF);
    for i in 0..8000u32 {
        /* splice one or two corpus entries */
        let mut src = String::from(CORPUS[rng.below(CORPUS.len() as u64) as usize]);
        if rng.below(3) == 0 {
            src.push_str(if rng.below(2) == 0 { " " } else { "\n" });
            src.push_str(CORPUS[rng.below(CORPUS.len() as u64) as usize]);
        }
        if rng.below(4) == 0 {
            src = format!("\"use strict\"; {}", src);
        }
        /* 1..4 random single edits, always on a char boundary */
        let edits = 1 + rng.below(4) as usize;
        for _ in 0..edits {
            let bounds: Vec<usize> = src
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(src.len()))
                .collect();
            let at = bounds[rng.below(bounds.len() as u64) as usize];
            match rng.below(4) {
                /* delete one char */
                0 => {
                    if at < src.len() {
                        let end = src[at..].chars().next().map(|c| at + c.len_utf8()).unwrap();
                        src.replace_range(at..end, "");
                    }
                }
                /* duplicate one char */
                1 => {
                    if at < src.len() {
                        let c = src[at..].chars().next().unwrap();
                        src.insert(at, c);
                    }
                }
                /* insert a token fragment */
                2 => {
                    let ins = INSERTS[rng.below(INSERTS.len() as u64) as usize];
                    src.insert_str(at, ins);
                }
                /* truncate here */
                _ => {
                    src.truncate(at);
                }
            }
            if src.len() > 4096 {
                src.truncate(4096);
                /* keep the truncation on a char boundary */
                while !src.is_empty() && !src.is_char_boundary(src.len()) {
                    src.pop();
                }
            }
        }
        diff_load(&format!("fuzz mutation #{}", i), &src, 0);
        diff_load(&format!("fuzz mutation #{}", i), &src, JS_STRICT);
    }
}

/// Random token soups: every source is compiled by both libraries and the
/// accept/reject decision plus the full message text must match.
#[test]
fn fuzz_random_token_soup() {
    const TOKENS: [&str; 96] = [
        "var", "function", "if", "else", "do", "while", "for", "in", "instanceof", "new", "delete",
        "typeof", "void", "return", "break", "continue", "switch", "case", "default", "try",
        "catch", "finally", "throw", "with", "this", "null", "true", "false", "debugger", "get",
        "set", "class", "const", "let", "static", "yield", "eval", "arguments", "a", "b", "f",
        "x_1", "\\u0061", "0", "1", "012", "0x1f", "1e5", "1e", ".5", "1.5", "\"s\"", "'t'",
        "\"\\x41\"", "/re/", "/re/gi", "/re/gg", "(", ")", "{", "}", "[", "]", ";", ",", ":", "?",
        ".", "=", "==", "===", "!=", "!", "+", "-", "*", "/", "%", "&", "|", "^", "~", "<", ">",
        "<=", ">=", "<<", ">>", ">>>", "&&", "||", "++", "--", "+=", "@", "#",
    ];
    let mut rng = Rng::new(SEED);
    for i in 0..20000u32 {
        let n = 1 + rng.below(14) as usize;
        let mut src = String::new();
        for k in 0..n {
            if k > 0 {
                /* sometimes glue tokens together, sometimes separate them */
                match rng.below(4) {
                    0 => {}
                    1 => src.push(' '),
                    2 => src.push('\n'),
                    _ => src.push(' '),
                }
            }
            src.push_str(TOKENS[rng.below(TOKENS.len() as u64) as usize]);
        }
        diff_load(&format!("fuzz token soup #{}", i), &src, 0);
        diff_load(&format!("fuzz token soup #{}", i), &src, JS_STRICT);
    }
}

/// Random (mostly invalid) unicode source text: exercises `jsY_lexx`'s
/// unexpected-character paths, the identifier-escape paths and the string /
/// regexp lexers with arbitrary bytes.
#[test]
fn fuzz_random_unicode_sources() {
    let mut rng = Rng::new(SEED ^ 0xABCDEF);
    for i in 0..5000u32 {
        let s = rng.string(40);
        diff_load(&format!("fuzz unicode #{}", i), &s, 0);
        diff_load(&format!("fuzz unicode #{}", i), &s, JS_STRICT);

        /* the same payload in the three lexer sub-modes */
        let inside_string = format!("var a = \"{}\";", s);
        diff_load(&format!("fuzz unicode in string #{}", i), &inside_string, 0);
        let inside_regexp = format!("var a = /{}/;", s);
        diff_load(&format!("fuzz unicode in regexp #{}", i), &inside_regexp, 0);
        let after_backslash = format!("var \\{};", s);
        diff_load(&format!("fuzz unicode after backslash #{}", i), &after_backslash, 0);
    }
}

/// Random JSON payloads through `JSON.parse` — the second lexer in jslex.c.
#[test]
fn fuzz_random_json() {
    const PIECES: [&str; 34] = [
        "{", "}", "[", "]", ":", ",", "\"a\"", "\"\"", "\"\\u0041\"", "\"\\uZ123\"", "\"\\q\"",
        "true", "false", "null", "tru", "fals", "nul", "trux", "falsx", "nulx", "0", "-1", "1.5",
        "1.", "1e", "1e+5", "-", "-x", "012", "'a'", "+1", "@", "\u{e9}", "\u{01}",
    ];
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    for i in 0..4000u32 {
        let n = 1 + rng.below(8) as usize;
        let mut json = String::new();
        for k in 0..n {
            if k > 0 && rng.below(3) == 0 {
                json.push(' ');
            }
            json.push_str(PIECES[rng.below(PIECES.len() as u64) as usize]);
        }
        let src = format!("JSON.stringify(JSON.parse({}))", js_quote(&json));
        let flags = if i % 2 == 0 { 0 } else { JS_STRICT };
        diff_eval(&format!("fuzz json #{}", i), &src, flags);
    }
}

/// Random deep-nesting shapes around the `JS_ASTLIMIT` boundary (400).
#[test]
fn fuzz_ast_limit_boundary() {
    const SHAPES: [(&str, &str); 12] = [
        ("!", "x"),
        ("-", "1"),
        ("typeof ", "x"),
        ("(", ""),
        ("{", ""),
        ("if(1)", ";"),
        ("while(0)", ";"),
        ("l:", ";"),
        ("1?1:", "1"),
        ("a=", "1"),
        ("new ", "a"),
        ("[", ""),
    ];
    let mut rng = Rng::new(SEED ^ 0xFEED);
    for i in 0..400u32 {
        let (pre, tail) = SHAPES[rng.below(SHAPES.len() as u64) as usize];
        let n = rng.range_i64(395, 410) as usize;
        let closer = match pre {
            "(" => ")",
            "{" => "}",
            "[" => "]",
            _ => "",
        };
        let src = format!("{}{}{}", rep(pre, n), tail, rep(closer, n));
        diff_load(&format!("fuzz astlimit #{} pre={:?} n={}", i, pre, n), &src, 0);
        diff_load(
            &format!("fuzz astlimit #{} pre={:?} n={}", i, pre, n),
            &src,
            JS_STRICT,
        );
    }
    /* infix chains straddling the boundary */
    let mut rng2 = Rng::new(SEED ^ 0xBEEF);
    for i in 0..400u32 {
        let ops = ["*", "+", "<<", "<", "==", "&", "^", "|", "&&", "||", ","];
        let op = ops[rng2.below(ops.len() as u64) as usize];
        let n = rng2.range_i64(395, 410) as usize;
        let src = format!("1{}", rep(&format!("{}1", op), n));
        diff_load(
            &format!("fuzz astlimit infix #{} op={:?} n={}", i, op, n),
            &src,
            0,
        );
    }
}
