//! Differential tests for the JS builtin library layer: RegExp, String, Array,
//! JSON, Date, Math and Number.  Everything is driven through the exported
//! entry points: `js_dostring`, the `js_ploadstring` + `js_pcall` pipeline and
//! -- for the regexp rows -- the exported `js_newregexp` / `js_setglobal`.
//!
//! CONFIGS.md rows covered (the task's stated ranges 88-110 / 207-253 no longer
//! line up with CONFIGS.md, whose rows 88-110 are js_get/set/defproperty and
//! 207-253 are raw `js_regcomp` shapes already covered by `tests/ll_regexp.rs`;
//! the rows that match the *content* of the task are these):
//!
//!   * 90, 98        - JS_CREGEXP synthetic properties on read (source, global,
//!                     ignoreCase, multiline, lastIndex) and on write
//!                     (lastIndex writable through jsV_tointeger, the other
//!                     four read-only)
//!   * 116, 118-121  - array `length` and the flat-vs-unflattened array
//!                     representation, reached through Array.prototype
//!   * 152           - js_tostring on JS_CARRAY (Array.prototype.toString ->
//!                     join, with the Ap_join_cycle guard) and on JS_CDATE
//!   * 285-295       - js_newregexp: all 8 JS_REGEXP_G|I|M combinations, a
//!                     pattern containing `/` (escaperegexp), `new RegExp(re)`
//!                     clones and the `(?:)` empty-pattern substitution
//!   * 296-306       - js_RegExp_prototype_exec: every re->last / REG_NOTBOL
//!                     branch, through RegExp.prototype.exec / .test and
//!                     String.prototype.match
//!   * 404-407       - JSON.parse with and without a reviver, every value shape
//!   * 408-419       - JSON.stringify: replacer absent/function/array, space
//!                     absent/number/string, fmtnum, fmtstr, fmtvalue, fmtindent
//!   * 420-427       - Sp_split / Sp_split_string / Sp_split_regexp
//!   * 428-435       - Sp_replace / Sp_replace_string / Sp_replace_regexp
//!   * 436-437       - Sp_match
//!   * 438-443       - Ap_sort
//!   * 444-450       - jsB_new_Date, D_UTC, D_parse + parseDateTime and
//!                     fmtdate / fmttime / fmtdatetime
//!
//! plus the remaining String.prototype, Array.prototype, Math and
//! Number.prototype entry points named in the task.
//!
//! Undefined-behaviour paths in the C that are deliberately NOT tested (each is
//! also documented at the test that skips it):
//!
//!   * c_src/src/jsstring.c:573 - `Sp_replace_regexp`'s function-replacement arm
//!     walks the captures with `for (x = 0; m.sub[x].sp; ++x)`, which reads
//!     `m.sub[16]` (one past `Resub.sub[REG_MAXSUB]`) when a regexp with the
//!     maximum 15 capture groups has all of them participate.  Both
//!     `libmujs.so` builds segfault on it.  See `t_regexp_many_captures`.
//!   * c_src/src/jsarray.c:279 and :365 - `Ap_sort_cmp` / `Ap_sort_swap` read
//!     `js_tovalue(J,0)->u.object->u.a.*` without checking the class, so
//!     `Array.prototype.sort.call(x, ...)` for a non-array `x` reads a union
//!     member that was never stored (or `u.object` of a primitive).  See
//!     `t_array_sort`.
//!   * c_src/src/jsstring.c:253-258 - `Sp_substring_imp` installs a
//!     `js_try` handler that does `js_free(J, p)` before `p` is assigned, so an
//!     allocation failure frees an indeterminate pointer.  Only reachable by
//!     forcing OOM, which this file never does.
//!   * c_src/src/jsdate.c:319-347 - `fmtdate` / `fmttime` compute
//!     `YearFromTime(t)` etc. *before* the `isfinite(t)` guard, so for a NaN
//!     time they convert NaN to int and overflow `365 * (y - 1970)`.  The
//!     results are discarded, and `src/jsi.rs`'s `d2i` reproduces gcc's
//!     cvttsd2si behaviour, so the invalid-date path IS tested; nothing here
//!     depends on the discarded values.

mod common;
use common::*;
use std::ffi::{c_char, c_int};
use std::sync::OnceLock;

/* ===================================================================== */
/* helpers                                                               */
/* ===================================================================== */

/// Quote a Rust string as a MuJS string literal that round-trips byte-exactly.
///
/// Non-ASCII runes are emitted as raw UTF-8 (chartorune/runetochar preserve
/// them) except U+2028/U+2029, which `jsY_next` normalises to '\n' and which
/// `lexstring` then rejects, so those two have to go through `\u`.
fn jsq(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{b}' => o.push_str("\\v"),
            '\u{c}' => o.push_str("\\f"),
            '\u{2028}' => o.push_str("\\u2028"),
            '\u{2029}' => o.push_str("\\u2029"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\x{:02x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Render an f64 as a JS expression that evaluates to exactly that double.
fn jsnum(x: f64) -> String {
    if x.is_nan() {
        return "(0/0)".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "(-Infinity)".to_string()
        };
    }
    let s = format!("{x:?}");
    if s.starts_with('-') {
        format!("({s})")
    } else {
        s
    }
}

/// Run a batch of complete JS statements as one script, each wrapped in
/// try/catch so a single throw cannot hide the rest of the batch, and assert
/// C == Rust byte for byte through both `js_dostring` and
/// `js_ploadstring`+`js_pcall` (the latter also compares `js_gettop`).
fn diff_lines_n(prelude: &str, lines: &[String], chunk: usize) {
    assert!(!lines.is_empty(), "empty batch");
    let p = libs();
    for part in lines.chunks(chunk) {
        let mut src = String::from(prelude);
        for l in part {
            src.push_str("try{");
            src.push_str(l);
            src.push_str("\n}catch(e){dump('CAUGHT',e)}\n");
        }
        let a = eval(&p.c, 0, &src);
        let b = eval(&p.rs, 0, &src);
        assert_eq!(a, b, "eval divergence\nsrc: {src}");
        // A generated script that failed to *parse* would make both libraries
        // report the same syntax error and the batch would pass without ever
        // exercising anything, so refuse to accept that.
        assert_eq!(a.load_rc, 0, "generated script did not parse: {src}\n{a:?}");
        assert_eq!(a.call_rc, 0, "generated script threw at top level: {src}\n{a:?}");
        assert!(
            a.out.lines().count() * 2 >= part.len(),
            "batch produced only {} output lines for {} statements: {src}",
            a.out.lines().count(),
            part.len()
        );
        diff_dostring(0, &src);
    }
}

fn diff_lines(lines: &[String]) {
    diff_lines_n("", lines, 120);
}

/// Same, but every element is an *expression* whose value is dumped.
fn diff_exprs(exprs: &[String]) {
    let lines: Vec<String> = exprs.iter().map(|e| format!("dump({e});")).collect();
    diff_lines(&lines);
}

/* ------------------------------------------------------- date warm-up ---- */

/// `LocalTZA()` in jsdate.c caches its result in a function-static on first
/// call, and computes it with `gmtime()`/`localtime()`, which share one static
/// `struct tm` inside libc.  cargo runs #[test]s on parallel threads, so if two
/// tests raced into the *first* LocalTZA call of the C library and of the Rust
/// library the two caches could be seeded from interleaved `struct tm` data and
/// end up different.  Force both caches to be filled once, single-threaded.
///
/// TZ is set here exactly once for the *whole process* (never per library, so
/// both libraries always agree), and only before either library has had a
/// chance to cache anything.  A UTC host would make LocalTime() the identity
/// and leave LocalTime()/UTC()/getTimezoneOffset and fmttime's signed-offset
/// arm untested, so a fixed POSIX zone 5h30 west of UTC is pinned instead; that
/// also makes the expected output independent of the host's zone.
/// fmttime's `tza == 0` arm stays covered through Dp_toUTCString /
/// Dp_toISOString, which pass 0 explicitly.  Its `tza > 0` arm is unreachable
/// from a single process, because the only non-zero tza any caller passes is
/// `LocalTZA()`, which is cached once.
fn warm_dates() {
    static W: OnceLock<()> = OnceLock::new();
    W.get_or_init(|| {
        std::env::set_var("TZ", "XYZ5:30");
        let p = libs();
        for l in [&p.c, &p.rs] {
            unsafe {
                out_clear();
                let j = new_state(l, 0);
                let cs = cstr("'' + new Date(0); new Date(0).getTimezoneOffset()");
                l.js_dostring(j, cs.as_ptr());
                l.js_freestate(j);
                let _ = out_take();
            }
        }
        // both libraries must have ended up with the same cached offset
        let mut seen = vec![];
        for l in [&p.c, &p.rs] {
            unsafe {
                out_clear();
                let j = new_state(l, 0);
                let cs = cstr("print(new Date(0).getTimezoneOffset(), new Date(0).toString())");
                l.js_dostring(j, cs.as_ptr());
                l.js_freestate(j);
                seen.push(out_take());
            }
        }
        assert_eq!(seen[0], seen[1], "LocalTZA disagreed between the libraries");
        assert!(
            seen[0].starts_with("330 "),
            "TZ=XYZ5:30 should give a +330 minute offset, got {:?}",
            seen[0]
        );
    });
}

/* --------------------------------------- low level js_newregexp runner --- */

#[derive(Debug, PartialEq, Eq)]
struct Run {
    rc: c_int,
    out: String,
    top: c_int,
}

const RE_NAME: *const c_char = b"re\0".as_ptr() as *const c_char;

/// Build a regexp with the exported `js_newregexp(J, pattern, flags)`, publish
/// it as the global `re`, then run `script`.
///
/// `pattern` must be a pattern that compiles: `js_newregexp` raises a
/// syntaxerror through `js_throw`, and with `trytop == 0` the default panic
/// handler calls `abort()`, so a bad pattern here would kill the test process
/// rather than produce a comparable result.  Compile failures are covered by
/// `tests/ll_regexp.rs` and, at the JS level, by `t_regexp_bad_pattern`.
unsafe fn run_with_regexp(l: &Lib, pattern: &str, flags: c_int, script: &str) -> Run {
    out_clear();
    let j = new_state(l, 0);
    let cp = cstr(pattern);
    l.js_newregexp(j, cp.as_ptr(), flags);
    l.js_setglobal(j, RE_NAME);
    let cs = cstr(script);
    let rc = l.js_dostring(j, cs.as_ptr());
    let top = l.js_gettop(j);
    l.js_freestate(j);
    Run {
        rc,
        out: out_take(),
        top,
    }
}

fn diff_with_regexp(pattern: &str, flags: c_int, script: &str) {
    let p = libs();
    unsafe {
        let a = run_with_regexp(&p.c, pattern, flags, script);
        let b = run_with_regexp(&p.rs, pattern, flags, script);
        assert_eq!(
            a, b,
            "js_newregexp divergence pattern={pattern:?} flags={flags}"
        );
        assert_eq!(a.rc, 0, "script failed: pattern={pattern:?} flags={flags}");
        assert!(
            a.out.lines().count() > 100,
            "script produced only {} lines: pattern={pattern:?} flags={flags}",
            a.out.lines().count()
        );
    }
}

/* ===================================================================== */
/* shared input tables                                                   */
/* ===================================================================== */

/// Patterns that all compile under every cflags combination.
fn re_patterns() -> Vec<&'static str> {
    vec![
        "a",
        "abc",
        "",
        "(?:)",
        "(a)(b)",
        "a+",
        "a*",
        "[a-z]+",
        "\\d+",
        "^a",
        "a$",
        "^",
        "$",
        "a|b",
        ".",
        ".*",
        "\\b\\w+\\b",
        "(a)|(b)",
        "[^a]",
        "A",
        "\u{e9}",
        "\u{4e2d}",
        "(\\w+)@(\\w+)",
        "\\s*",
        "a/b",
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)",
    ]
}

fn subject_strings() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        "a".into(),
        "ab".into(),
        "abc".into(),
        "aXbXXc".into(),
        "a,b,c".into(),
        "a,,b".into(),
        ",a,".into(),
        ",,,".into(),
        "a\nb\nc".into(),
        "  padded  ".into(),
        "\t\u{a0}x\u{feff}".into(),
        "MiXeD CaSe".into(),
        "\u{e9}\u{e8}\u{e0}".into(),
        "\u{4e2d}\u{6587}\u{4e2d}\u{6587}".into(),
        "\u{1f600}\u{1f601}".into(),
        "a\u{1f600}b".into(),
        "\u{df}\u{130}\u{131}\u{17f}".into(),
        "\u{fb03}x".into(),
        "0123456789".into(),
        "aaaaaaaaaaaaaaaaaaaa".into(),
        "the quick brown fox".into(),
    ];
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..30 {
        v.push(rng.ascii_string(18));
    }
    for _ in 0..30 {
        v.push(rng.unicode_string(9));
    }
    v
}

/// Index / count arguments: absent, negative, out of range, fractional,
/// non-numeric.  The empty string means "argument omitted".
const IDX: &[&str] = &[
    "",
    "-100",
    "-5",
    "-1",
    "0",
    "1",
    "2",
    "3",
    "5",
    "100",
    "1e10",
    "-1e10",
    "2147483647",
    "-2147483648",
    "(0/0)",
    "Infinity",
    "(-Infinity)",
    "1.7",
    "(-1.7)",
    "'2'",
    "'abc'",
    "null",
    "true",
    "undefined",
    "{}",
    "[]",
    "[3]",
];

/// `IDX` without the "argument omitted" entry, for positions where a value
/// must actually be written (a trailing `f(1, )` would not parse).
fn idx_nonempty() -> Vec<&'static str> {
    IDX.iter().copied().filter(|s| !s.is_empty()).collect()
}

/* ===================================================================== */
/* RegExp                                                                */
/* ===================================================================== */

/// The full RegExp surface for one compiled program: the synthesised
/// properties, toString, exec/test lastIndex progression, and every
/// String.prototype method that takes a regexp.
const RE_SCRIPT: &str = r#"
var TX = ["", "a", "aa", "abc", "ABC", "abcabc", "xxabcyy", "a\nb", "A\nB",
          "aXbXc", "éé", "中文中", "a b c",
          "foo@bar baz@qux", "a/b/c"];
dump('src', re.source);
dump('flg', re.global, re.ignoreCase, re.multiline, re.lastIndex);
dump('str', re.toString(), '' + re, String(re));
dump('rpr', re, typeof re, re instanceof RegExp);
try { dump('cln', new RegExp(re).toString(), new RegExp(re).source,
                  new RegExp(re).global, new RegExp(re).ignoreCase,
                  new RegExp(re).multiline) }
catch (e) { dump('cln', 'CAUGHT', e) }
try { dump('cl2', new RegExp(re, 'g')) } catch (e) { dump('cl2', 'CAUGHT', e) }
try { dump('cl3', RegExp(re) === re) } catch (e) { dump('cl3', 'CAUGHT', e) }
dump('ro', re.source = 'ZZZ', re.source);
dump('ro', re.global = 1, re.global);
dump('ro', re.ignoreCase = 1, re.ignoreCase);
dump('ro', re.multiline = 1, re.multiline);
dump('del', delete re.source, delete re.lastIndex, re.source, re.lastIndex);
for (var i = 0; i < TX.length; ++i) {
    var t = TX[i];
    re.lastIndex = 0;
    for (var k = 0; k < 5; ++k) {
        try { dump('x', i, k, re.exec(t), re.lastIndex) }
        catch (e) { dump('x', i, k, 'CAUGHT', e) }
    }
    re.lastIndex = 0;
    for (var k = 0; k < 5; ++k) {
        try { dump('T', i, k, re.test(t), re.lastIndex) }
        catch (e) { dump('T', i, k, 'CAUGHT', e) }
    }
    re.lastIndex = 0;
    try { dump('m', i, t.match(re), re.lastIndex) }
    catch (e) { dump('m', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('s', i, t.search(re), re.lastIndex) }
    catch (e) { dump('s', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('p', i, t.split(re), re.lastIndex) }
    catch (e) { dump('p', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('p0', i, t.split(re, 0), t.split(re, 2), t.split(re, -1)) }
    catch (e) { dump('p0', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('r', i, t.replace(re, '<$&>'), re.lastIndex) }
    catch (e) { dump('r', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('r2', i, t.replace(re, '$1|$2|$`|$\x27|$$'), re.lastIndex) }
    catch (e) { dump('r2', i, 'CAUGHT', e) }
    re.lastIndex = 0;
    try { dump('R', i, t.replace(re, function () {
              var s = ''; for (var q = 0; q < arguments.length; ++q)
                  s += '|' + arguments[q]; return s }), re.lastIndex) }
    catch (e) { dump('R', i, 'CAUGHT', e) }
}
"#;

/// CONFIGS 285-306, 90, 98: every js_newregexp flag combination.
#[test]
fn t_regexp_newregexp_flag_matrix() {
    warm_dates();
    for pat in re_patterns() {
        for flags in 0..8 {
            diff_with_regexp(pat, flags, RE_SCRIPT);
        }
    }
}

/// CONFIGS 90/98 + 296-306: lastIndex written by hand to negative, huge,
/// fractional and non-numeric values.  `js_Regexp::last` is an
/// `unsigned short`, so jsV_tointeger's result is taken modulo 65536.
#[test]
fn t_regexp_lastindex_assignment() {
    warm_dates();
    let script = r#"
var V = [-1, -2, -1000, -65535, -65536, -65537, 0, 1, 2, 3, 5, 6, 7, 100,
         65534, 65535, 65536, 65537, 70000, 131072, 1e10, -1e10,
         2147483647, -2147483648, 1.5, 2.7, -0.5, 0.9,
         0/0, Infinity, -Infinity, '0', '3', '', 'abc', '1e2',
         true, false, null, undefined, {}, [], [3], [1,2], new Number(4),
         new String('5')];
var TX = ["", "a", "abc", "aaaa", "abcabc", "a\nb\nc"];
for (var i = 0; i < TX.length; ++i) {
    for (var v = 0; v < V.length; ++v) {
        var t = TX[i];
        try {
            re.lastIndex = V[v];
            dump('w', i, v, re.lastIndex);
            dump('x', re.exec(t), re.lastIndex);
            re.lastIndex = V[v];
            dump('T', re.test(t), re.lastIndex);
            re.lastIndex = V[v];
            dump('m', t.match(re), re.lastIndex);
            re.lastIndex = V[v];
            dump('r', t.replace(re, '#'), re.lastIndex);
            re.lastIndex = V[v];
            dump('p', t.split(re), re.lastIndex);
        } catch (e) { dump('c', i, v, 'CAUGHT', e) }
    }
}
"#;
    for pat in ["a", "a*", "(a)", "^a", "a$", ".", "\\w+", ""] {
        for flags in [0, JS_REGEXP_G, JS_REGEXP_G | JS_REGEXP_M, JS_REGEXP_M] {
            diff_with_regexp(pat, flags, script);
        }
    }
}

/// The same surface built from JS regexp literals and `new RegExp(...)`,
/// including bad patterns and bad flag strings (which must produce identical
/// syntaxerror messages).
#[test]
fn t_regexp_js_level() {
    warm_dates();
    let mut lines = vec![];
    for ctor in [
        "/a/", "/a/g", "/a/i", "/a/m", "/a/gi", "/a/gm", "/a/im", "/a/gim",
        "/a/mig", "new RegExp('a')", "new RegExp('a','g')",
        "new RegExp('a','gim')", "new RegExp()", "new RegExp('')",
        "new RegExp(undefined)", "new RegExp(null)", "new RegExp(5)",
        "new RegExp({})", "new RegExp('a/b')", "/a\\/b/", "/[/]/",
        "new RegExp(/a/g)", "new RegExp(/a/g, undefined)", "RegExp('a')",
        "RegExp(/a/i)", "new RegExp('a','gg')", "new RegExp('a','x')",
        "new RegExp('a','')", "new RegExp(/a/, 'i')", "new RegExp('(')",
        "new RegExp('a{2,1}')", "new RegExp('[z-a]')",
    ] {
        lines.push(format!(
            "var r = {ctor}; dump(r, r.source, r.global, r.ignoreCase, \
             r.multiline, r.lastIndex, r.toString());"
        ));
        lines.push(format!(
            "var r = {ctor}; dump(r.exec('aaa'), r.lastIndex, r.exec('aaa'), \
             r.lastIndex, r.test('aaa'), r.lastIndex);"
        ));
    }
    // exec/test on a non-regexp `this`, and on a regexp with no argument
    for extra in [
        "dump(/a/.exec())",
        "dump(/a/.test())",
        "dump(/undefined/.exec())",
        "dump(/a/.exec('a','b'))",
        "dump(RegExp.prototype.source, RegExp.prototype.global)",
        "dump(RegExp.prototype.toString())",
        "dump(RegExp.prototype.exec.call(/b/, 'abc'))",
        "dump(RegExp.prototype.test.call(/b/, 'abc'))",
        "dump(RegExp.prototype.exec.call({}, 'abc'))",
        "dump(RegExp.prototype.toString.call({}))",
        "dump(/a/ instanceof RegExp, typeof /a/, Object.prototype.toString.call(/a/))",
    ] {
        lines.push(extra.to_string());
    }
    diff_lines(&lines);
}

/// Uncaught syntaxerror text from bad patterns and bad flags.
#[test]
fn t_regexp_bad_pattern() {
    warm_dates();
    for src in [
        "new RegExp('(')",
        "new RegExp('a**')",
        "new RegExp('[z-a]')",
        "new RegExp('a','q')",
        "new RegExp('a','gg')",
        "new RegExp(/a/, 'g')",
        "/a/.exec.call(1,'a')",
        "'x'.split(new RegExp('('))",
    ] {
        diff_dostring(0, src);
        diff_eval(0, src);
        diff_dostring(JS_STRICT, src);
    }
}

/* ===================================================================== */
/* String.prototype.split                                                */
/* ===================================================================== */

/// CONFIGS 420-427.
#[test]
fn t_string_split() {
    warm_dates();
    let seps: Vec<String> = vec![
        "".into(), // argument absent
        "undefined".into(),
        "''".into(),
        "','".into(),
        "'X'".into(),
        "'ab'".into(),
        "'\u{4e2d}'".into(),
        "'zzz'".into(),
        "'a'".into(),
        "'\\n'".into(),
        "5".into(),
        "null".into(),
        "true".into(),
        "{}".into(),
        "/,/".into(),
        "/,/g".into(),
        "/X+/".into(),
        "/(,)/".into(),
        "/()/".into(),
        "/\\s*/".into(),
        "/(a)(b)?/".into(),
        "/$/".into(),
        "/^/m".into(),
        "/\\b/".into(),
        "/(?:)/".into(),
        "/a|b/".into(),
        "/(a)|(b)/".into(),
        "new RegExp('')".into(),
    ];
    let limits: Vec<String> = vec![
        "".into(),
        ", 0".into(),
        ", 1".into(),
        ", 2".into(),
        ", 3".into(),
        ", 1000000000".into(),
        ", -1".into(),
        ", -5".into(),
        ", '2'".into(),
        ", 'abc'".into(),
        ", (0/0)".into(),
        ", 1.9".into(),
        ", null".into(),
        ", undefined".into(),
        ", true".into(),
        ", Infinity".into(),
        ", (-Infinity)".into(),
    ];
    let subjects = subject_strings();
    let mut exprs = vec![];
    for s in &subjects {
        for sep in &seps {
            // limit absent for the whole separator matrix
            exprs.push(format!("{}.split({})", jsq(s), sep));
        }
    }
    // limit matrix over a smaller but representative subject/separator set
    for s in [
        "", "a", "a,b,c", "a,,b", ",a,", "abc", "aXbXXc", "a\nb\nc", "\u{4e2d}\u{6587}\u{4e2d}",
        "\u{1f600}\u{1f601}",
    ] {
        for sep in &seps {
            // an omitted separator cannot be followed by a limit
            let sep = if sep.is_empty() { "undefined" } else { sep };
            for lim in &limits {
                exprs.push(format!("{}.split({}{})", jsq(s), sep, lim));
            }
        }
    }
    // split on non-string `this`
    for extra in [
        "String.prototype.split.call(123, '2')",
        "String.prototype.split.call(true, 'r')",
        "String.prototype.split.call(new String('a,b'), ',')",
        "String.prototype.split.call(null, ',')",
        "String.prototype.split.call(undefined, ',')",
        "String.prototype.split.call({}, 'b')",
        "String.prototype.split.call([1,2], ',')",
    ] {
        exprs.push(extra.to_string());
    }
    diff_exprs(&exprs);
}

/* ===================================================================== */
/* String.prototype.replace                                              */
/* ===================================================================== */

/// CONFIGS 428-435, including the deliberate `if (x > 10)` in jsstring.c:605
/// (the ES spec wants `x >= 10`), which makes an out-of-range `$10` render as
/// `$:` -- reproduced, not fixed.
#[test]
fn t_string_replace() {
    warm_dates();
    let reps: Vec<&str> = vec![
        "''",
        "'-'",
        "'$$'",
        "'$&'",
        "'$`'",
        "'$\\''",
        "'$1'",
        "'$2'",
        "'$3'",
        "'$9'",
        "'$0'",
        "'$00'",
        "'$01'",
        "'$10'",
        "'$11'",
        "'$12'",
        "'$19'",
        "'$20'",
        "'$99'",
        "'$1$2'",
        "'[$&]'",
        "'a$'",
        "'$'",
        "'$$$&'",
        "'$x'",
        "'$-'",
        "'<$`|$&|$\\'>'",
        "'$1$$$2'",
        "'$&$&$&'",
        "'\\u4e2d$&\\u4e2d'",
        "5",
        "null",
        "undefined",
        "true",
        "{}",
        "[1,2]",
        "function(){ return 'F' }",
        "function(m){ return '[' + m + ']' }",
        "function(m,a,b){ return m + '/' + a + '/' + b }",
        "function(){ var s=''; for (var i=0;i<arguments.length;++i) s += '|' + arguments[i]; return s }",
        "function(){ return undefined }",
        "function(){ return 7 }",
        "function(){ throw 'boom' }",
        "function(){ return {} }",
    ];
    let pats: Vec<&str> = vec![
        "''",
        "'a'",
        "'ab'",
        "'X'",
        "'zzz'",
        "','",
        "'\u{4e2d}'",
        "5",
        "null",
        "undefined",
        "/a/",
        "/a/g",
        "/a/i",
        "/a/gi",
        "/(a)(b)/",
        "/(a)(b)/g",
        "/(a)|(b)/g",
        "/a*/",
        "/a*/g",
        "/(?:)/",
        "/(?:)/g",
        "/^/",
        "/^/g",
        "/^/gm",
        "/$/g",
        "/\\b/g",
        "/zzz/",
        "/zzz/g",
        "/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)/",
        "/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)/g",
        "/./g",
        "/\\s+/g",
        "new RegExp('a','g')",
    ];
    let subjects = [
        "",
        "a",
        "ab",
        "abc",
        "abcdefghijkl",
        "aabbcc",
        "XaX",
        "a,b,c",
        "a\nb\nc",
        "\u{4e2d}a\u{6587}",
        "\u{1f600}a",
        "the quick brown fox",
    ];
    let mut exprs = vec![];
    for s in subjects {
        for p in &pats {
            for r in &reps {
                exprs.push(format!("{}.replace({}, {})", jsq(s), p, r));
            }
        }
    }
    // this-coercion and arity
    for extra in [
        "'abc'.replace()",
        "'abc'.replace('b')",
        "String.prototype.replace.call(1231, '2', 'X')",
        "String.prototype.replace.call(null, 'a', 'b')",
        "String.prototype.replace.call(undefined, 'a', 'b')",
        "String.prototype.replace.call(new String('aa'), /a/g, 'b')",
        "String.prototype.replace.call([1,2], ',', ';')",
    ] {
        exprs.push(extra.to_string());
    }
    diff_exprs(&exprs);
}

/* ===================================================================== */
/* String.prototype.match / search                                       */
/* ===================================================================== */

/// CONFIGS 436-437.
#[test]
fn t_string_match_search() {
    warm_dates();
    let args: Vec<&str> = vec![
        "",
        "undefined",
        "''",
        "'a'",
        "'a*'",
        "'('",
        "'\\\\d'",
        "5",
        "null",
        "true",
        "{}",
        "[1,2]",
        "/a/",
        "/a/g",
        "/a/i",
        "/a/gi",
        "/a/m",
        "/a/gm",
        "/(a)(b)/",
        "/(a)(b)/g",
        "/a*/g",
        "/(?:)/g",
        "/^/gm",
        "/$/g",
        "/\\b/g",
        "/./g",
        "new RegExp('a','g')",
    ];
    let subjects = subject_strings();
    let mut exprs = vec![];
    for s in &subjects {
        for a in &args {
            exprs.push(format!("{}.match({})", jsq(s), a));
            exprs.push(format!("{}.search({})", jsq(s), a));
        }
    }
    for extra in [
        "String.prototype.match.call(1231, '2')",
        "String.prototype.search.call(1231, '2')",
        "String.prototype.match.call(null, 'a')",
        "String.prototype.search.call(undefined, 'a')",
        "(function(){var r=/a/g; var m='aaa'.match(r); return [m, r.lastIndex]})()",
        "(function(){var r=/a/; r.lastIndex=2; var m='aaa'.match(r); return [m, r.lastIndex]})()",
    ] {
        exprs.push(extra.to_string());
    }
    diff_exprs(&exprs);
}

/* ===================================================================== */
/* the rest of String.prototype                                          */
/* ===================================================================== */

#[test]
fn t_string_index_methods() {
    warm_dates();
    let subjects = subject_strings();
    let needles = ["''", "'a'", "'ab'", "'\u{4e2d}'", "'\u{1f600}'", "'zzz'", "5", "null", ""];
    let mut exprs = vec![];
    for s in &subjects {
        let q = jsq(s);
        for i in IDX {
            exprs.push(format!("{q}.charAt({i})"));
            exprs.push(format!("{q}.charCodeAt({i})"));
            exprs.push(format!("{q}.slice({i})"));
            exprs.push(format!("{q}.substring({i})"));
            // String.prototype.substr does not exist in this MuJS build, so
            // this exercises the "call a missing method" error path instead.
            exprs.push(format!("{q}.substr({i})"));
        }
        let two = idx_nonempty();
        for a in two.iter().take(14) {
            for b in two.iter().take(14) {
                exprs.push(format!("{q}.slice({a}, {b})"));
                exprs.push(format!("{q}.substring({a}, {b})"));
            }
        }
        for n in needles {
            for i in IDX.iter().take(12) {
                if n.is_empty() {
                    continue;
                }
                if i.is_empty() {
                    exprs.push(format!("{q}.indexOf({n})"));
                    exprs.push(format!("{q}.lastIndexOf({n})"));
                } else {
                    exprs.push(format!("{q}.indexOf({n}, {i})"));
                    exprs.push(format!("{q}.lastIndexOf({n}, {i})"));
                }
            }
        }
        exprs.push(format!("{q}.length"));
        exprs.push(format!("{q}.toUpperCase()"));
        exprs.push(format!("{q}.toLowerCase()"));
        exprs.push(format!("{q}.toLocaleUpperCase()"));
        exprs.push(format!("{q}.toLocaleLowerCase()"));
        exprs.push(format!("{q}.trim()"));
        exprs.push(format!("{q}.concat()"));
        exprs.push(format!("{q}.concat('x')"));
        exprs.push(format!("{q}.concat('x', 5, null, undefined, true, {{}})"));
        exprs.push(format!("{q}.toString()"));
        exprs.push(format!("{q}.valueOf()"));
        exprs.push(format!("{q}[0]"));
        exprs.push(format!("{q}[1]"));
        for t in &subjects[..12] {
            exprs.push(format!("{q}.localeCompare({})", jsq(t)));
        }
    }
    for extra in [
        "String.prototype.charAt.call(1231, 2)",
        "String.prototype.charCodeAt.call(1231, 2)",
        "String.prototype.toUpperCase.call(null)",
        "String.prototype.trim.call(undefined)",
        "String.prototype.concat.call(null, 'x')",
        "String.prototype.toString.call({})",
        "String.prototype.valueOf.call(5)",
        "String.prototype.toString.call(new String('q'))",
        "new String('abc').length",
        "new String('abc')[1]",
        "new String('abc')[9]",
        "String.prototype.length",
        "String.prototype.localeCompare.call(null, 'a')",
    ] {
        exprs.push(extra.to_string());
    }
    diff_exprs(&exprs);
}

#[test]
fn t_string_fromcharcode() {
    warm_dates();
    let cps = [
        "0", "1", "0x41", "0x7f", "0x80", "0x7ff", "0x800", "0xd7ff", "0xd800",
        "0xdbff", "0xdc00", "0xdfff", "0xe000", "0xfffd", "0xffff", "0x10000",
        "0x10ffff", "0x110000", "0x1fffff", "0x200000", "0x7fffffff",
        "0x80000000", "0xffffffff", "0x100000000", "-1", "-0.5", "65.9",
        "(0/0)", "Infinity", "(-Infinity)", "'65'", "'abc'", "null", "true",
        "undefined", "{}", "[66]",
    ];
    let mut exprs = vec!["String.fromCharCode()".to_string()];
    for c in cps {
        exprs.push(format!("String.fromCharCode({c})"));
        exprs.push(format!("String.fromCharCode({c}, 65)"));
        exprs.push(format!("String.fromCharCode(65, {c}, 66)"));
        exprs.push(format!("String.fromCharCode({c}).length"));
        exprs.push(format!("String.fromCharCode({c}).charCodeAt(0)"));
    }
    // many arguments
    exprs.push("(function(){var a=[];for(var i=1;i<=200;++i)a.push(i*7);return String.fromCharCode.apply(null,a).length})()".into());
    exprs.push("(function(){var a=[];for(var i=1;i<=200;++i)a.push(i*7);return String.fromCharCode.apply(null,a)})()".into());
    exprs.push("String.fromCharCode.apply(null, [])".into());
    exprs.push("String.fromCharCode.length".into());
    diff_exprs(&exprs);
}

/* ===================================================================== */
/* Array.prototype                                                       */
/* ===================================================================== */

/// Array shapes: dense, sparse via `delete`, sparse via `length`, arrays with
/// a non-index property, and empty/one/many element arrays.  Those exercise the
/// flat (`u.a.simple`) versus unflattened hashed representation in jsrun.c.
fn array_shapes() -> Vec<&'static str> {
    vec![
        "[]",
        "[1]",
        "[1,2]",
        "[1,2,3]",
        "[3,1,2]",
        "['b','a','c']",
        "[1,'a',true,null,undefined,{},[1,2]]",
        "[undefined,1,undefined,2]",
        "[null,1,null]",
        "[10,9,8,7,6,5,4,3,2,1]",
        // sparse: delete in the middle -> jsR_unflattenarray
        "(function(){var a=[1,2,3,4,5];delete a[2];return a})()",
        // delete the last element of a simple array -> stays flat
        "(function(){var a=[1,2,3];delete a[2];return a})()",
        "(function(){var a=[1,2,3];delete a[0];return a})()",
        // holes created by growing length, array stays simple
        "(function(){var a=[];a.length=5;return a})()",
        "(function(){var a=[1,2];a.length=5;return a})()",
        "(function(){var a=[1,2,3];a.length=2;return a})()",
        "(function(){var a=[1,2,3];a.length=0;return a})()",
        // non-index property on an array
        "(function(){var a=[1,2,3];a.foo='bar';return a})()",
        "(function(){var a=[];a.foo='bar';return a})()",
        // gap write -> unflatten
        "(function(){var a=[1,2,3];a[10]=11;return a})()",
        "(function(){var a=[];a[3]=1;return a})()",
        "(function(){var a=[1,2,3];a[1]=undefined;return a})()",
        // many elements
        "(function(){var a=[];for(var i=0;i<40;++i)a[i]=(i*7)%40;return a})()",
        "(function(){var a=[];for(var i=0;i<20;++i)a[i]=String(i*3%20);return a})()",
        "(function(){var a=[];for(var i=0;i<30;++i)if(i%3)a[i]=i;return a})()",
        // an array whose elements stringify oddly
        "[0,-0,1/0,-1/0,0/0]",
        "[[1,[2,[3]]],{a:1}]",
    ]
}

#[test]
fn t_array_query_methods() {
    warm_dates();
    let ops: Vec<&str> = vec![
        "dump(a, a.length)",
        "dump(a.join())",
        "dump(a.join('-'))",
        "dump(a.join(''))",
        "dump(a.join(undefined))",
        "dump(a.join(null))",
        "dump(a.join(5))",
        "dump(a.join({}))",
        "dump(a.toString())",
        "dump('' + a)",
        "dump(String(a))",
        "dump(a.concat())",
        "dump(a.concat(9))",
        "dump(a.concat([9,8],7))",
        "dump(a.concat([],[[1]]))",
        "dump(a.slice())",
        "dump(a.slice(1))",
        "dump(a.slice(-2))",
        "dump(a.slice(1,3))",
        "dump(a.slice(-3,-1))",
        "dump(a.slice(100,200))",
        "dump(a.slice(0/0, 0/0))",
        "dump(a.slice(-1e10, 1e10))",
        "dump(a.indexOf(1))",
        "dump(a.indexOf(1,1))",
        "dump(a.indexOf(1,-1))",
        "dump(a.indexOf(1,100))",
        "dump(a.indexOf(undefined))",
        "dump(a.indexOf('1'))",
        "dump(a.lastIndexOf(1))",
        "dump(a.lastIndexOf(1,1))",
        "dump(a.lastIndexOf(1,-1))",
        "dump(a.lastIndexOf(1,100))",
        "dump(a.lastIndexOf(undefined))",
        "dump(a.every(function(x){return x !== 2}))",
        "dump(a.every(function(){return true}))",
        "dump(a.every(function(){return false}))",
        "dump(a.some(function(x){return x === 2}))",
        "dump(a.some(function(){return false}))",
        "var o=[]; a.forEach(function(x,i,z){o.push(i+':'+x+':'+(z===a))}); dump(o)",
        "dump(a.map(function(x){return x}))",
        "dump(a.map(function(x,i){return i}))",
        "dump(a.filter(function(){return true}))",
        "dump(a.filter(function(x){return typeof x === 'number'}))",
        "dump(a.reduce(function(p,c){return String(p)+'/'+String(c)}))",
        "dump(a.reduce(function(p,c){return String(p)+'/'+String(c)}, 'I'))",
        "dump(a.reduceRight(function(p,c){return String(p)+'/'+String(c)}))",
        "dump(a.reduceRight(function(p,c){return String(p)+'/'+String(c)}, 'I'))",
        "dump(a.every())",
        "dump(a.some(3))",
        "dump(a.map(null))",
        "dump(a.reduce())",
        "dump(a.forEach(function(x,i,z){}, {t:1}))",
        "dump(a.map(function(){return this === undefined}, undefined))",
        "dump(a.every(function(){return this !== undefined}, {q:1}))",
        "dump(Array.isArray(a), Array.isArray(1), Array.isArray({}))",
        "dump(Object.prototype.toString.call(a))",
        "dump(Object.keys(a))",
        "for (var k in a) dump('in', k)",
    ];
    let mut lines = vec![];
    for shape in array_shapes() {
        for op in &ops {
            lines.push(format!("var a = {shape}; {op};"));
        }
    }
    diff_lines_n("", &lines, 100);
}

#[test]
fn t_array_mutating_methods() {
    warm_dates();
    let ops: Vec<&str> = vec![
        "dump(a.push()); dump(a, a.length)",
        "dump(a.push(9)); dump(a, a.length)",
        "dump(a.push(9,10,11)); dump(a, a.length)",
        "dump(a.push(undefined)); dump(a, a.length)",
        "dump(a.pop()); dump(a, a.length)",
        "dump(a.pop()); dump(a.pop()); dump(a, a.length)",
        "dump(a.shift()); dump(a, a.length)",
        "dump(a.shift()); dump(a.shift()); dump(a, a.length)",
        "dump(a.unshift()); dump(a, a.length)",
        "dump(a.unshift(0)); dump(a, a.length)",
        "dump(a.unshift(0,-1)); dump(a, a.length)",
        "dump(a.reverse()); dump(a, a.length)",
        "dump(a.splice()); dump(a, a.length)",
        "dump(a.splice(0)); dump(a, a.length)",
        "dump(a.splice(1)); dump(a, a.length)",
        "dump(a.splice(1,2)); dump(a, a.length)",
        "dump(a.splice(0,0)); dump(a, a.length)",
        "dump(a.splice(-2,1,'x','y')); dump(a, a.length)",
        "dump(a.splice(1,0,'z')); dump(a, a.length)",
        "dump(a.splice(100,5,'q')); dump(a, a.length)",
        "dump(a.splice(-100,2)); dump(a, a.length)",
        "dump(a.splice(0,1e10)); dump(a, a.length)",
        "dump(a.splice(0,-5,'p')); dump(a, a.length)",
        "dump(a.splice(1,1,'m','n','o')); dump(a, a.length)",
        "a.length = 0; dump(a, a.length)",
        "a.length = 2; dump(a, a.length)",
        "a.length = 7; dump(a, a.length)",
        "a[0] = 'zz'; dump(a, a.length)",
        "a[7] = 'zz'; dump(a, a.length)",
        "delete a[0]; dump(a, a.length)",
        "delete a[1]; dump(a, a.length)",
        "a.foo = 1; dump(a, a.length, a.foo); for (var k in a) dump('in', k)",
        "dump(a.pop(), a.push(1), a.shift(), a.unshift(2), a.reverse(), a)",
    ];
    let mut lines = vec![];
    for shape in array_shapes() {
        for op in &ops {
            lines.push(format!("var a = {shape}; {op};"));
        }
    }
    diff_lines_n("", &lines, 100);
}

/// CONFIGS 438-443.
///
/// `Ap_sort_cmp`/`Ap_sort_swap` read `js_tovalue(J,0)->u.object->u.a.simple`
/// without checking the object's class first (jsarray.c:279, jsarray.c:365), so
/// `Array.prototype.sort.call(nonArray, ...)` reads a union member that was
/// never stored -- indeterminate in C -- and `sort.call(primitive, ...)` reads
/// `u.object` out of a non-object value.  Those two shapes are deliberately not
/// tested.
#[test]
fn t_array_sort() {
    warm_dates();
    let cmps: Vec<&str> = vec![
        "",
        "undefined",
        "function(x,y){return x<y?-1:x>y?1:0}",
        "function(x,y){return y<x?-1:y>x?1:0}",
        "function(x,y){return x-y}",
        "function(x,y){return y-x}",
        "function(){return 0}",
        "function(){return -1}",
        "function(){return 1}",
        "function(){return 0/0}",
        "function(){return 'x'}",
        "function(){return undefined}",
        "function(){return null}",
        "function(){return {}}",
        "function(x,y){return (String(x).length*7)%3-1}",
        "function(){throw 'cmp'}",
        "3",
        "null",
        "'x'",
        "{}",
        "[]",
    ];
    let mut lines = vec![];
    for shape in array_shapes() {
        for c in &cmps {
            lines.push(format!(
                "var a = {shape}; dump(a.sort({c})); dump(a, a.length);"
            ));
        }
    }
    // comparators that mutate the array under the sort
    for extra in [
        "var a=[3,1,2]; dump(a.sort(function(x,y){a.length=1; return x-y})); dump(a)",
        "var a=[3,1,2]; dump(a.sort(function(x,y){a[5]=9; return x-y})); dump(a)",
        "var a=[3,1,2]; dump(a.sort(function(x,y){delete a[0]; return x-y})); dump(a)",
        "var a=[3,1,2,4,5,6]; dump(a.sort(function(x,y){a.push(0); return x-y})); dump(a)",
        "var a=[]; a.length=3; dump(a.sort()); dump(a, a.length)",
        "var a=[undefined,undefined]; dump(a.sort()); dump(a)",
        "var a=[undefined,1]; dump(a.sort()); dump(a)",
        "var a=[1,undefined]; dump(a.sort()); dump(a)",
        "var a=['10','9','1']; dump(a.sort()); dump(a)",
        "var a=[10,9,1]; dump(a.sort()); dump(a)",
        "dump(Array.prototype.sort.call([2,1]))",
    ] {
        lines.push(extra.to_string());
    }
    // randomised arrays with a fixed seed
    let mut rng = Rng::new(0xA5A5_0007);
    for _ in 0..260 {
        let n = rng.below(14) as usize;
        let mut items = vec![];
        for _ in 0..n {
            items.push(match rng.below(6) {
                0 => format!("{}", rng.range(-20, 20)),
                1 => jsq(&rng.ascii_string(3)),
                2 => "undefined".to_string(),
                3 => "null".to_string(),
                4 => format!("{}", rng.range(0, 5)),
                _ => jsq(&rng.unicode_string(2)),
            });
        }
        let lit = format!("[{}]", items.join(","));
        lines.push(format!("var a = {lit}; dump(a.sort()); dump(a);"));
        lines.push(format!(
            "var a = {lit}; dump(a.sort(function(x,y){{return x<y?-1:x>y?1:0}})); dump(a);"
        ));
        lines.push(format!(
            "var a = {lit}; delete a[0]; dump(a.sort()); dump(a);"
        ));
    }
    diff_lines_n("", &lines, 100);
}

/// CONFIGS 152: Array.prototype.join / toString cycle guard.
#[test]
fn t_array_join_cycle() {
    warm_dates();
    for src in [
        "var a=[1]; a[1]=a; print(a.join())",
        "var a=[1]; a[1]=a; print(a.toString())",
        "var a=[1]; a[1]=a; print('' + a)",
        "var a=[]; a[0]=a; print(a.join('-'))",
        "var a=[1,2]; var b=[a]; a[2]=b; print(a.join(), b.join())",
        "var a=[1]; a.join=function(){return 'J'}; print(a.toString())",
        "var a=[1]; a.join=5; print(a.toString())",
        "print(Array.prototype.toString.call({}))",
        "print(Array.prototype.toString.call('ab'))",
        "print(Array.prototype.join.call({length:3, 0:'a', 2:'c'}))",
        "print(Array.prototype.join.call('abc', '-'))",
        "print(Array.prototype.join.call({length:2}, '-'))",
        "print(Array.prototype.join.call({length:-1}))",
        "print([null, undefined, 1].join())",
    ] {
        diff_dostring(0, src);
        diff_eval(0, src);
    }
}

/* ===================================================================== */
/* JSON                                                                  */
/* ===================================================================== */

fn json_values() -> Vec<&'static str> {
    vec![
        "undefined",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1",
        "-1",
        "1.5",
        "1e21",
        "1e-7",
        "1e300",
        "(0/0)",
        "Infinity",
        "(-Infinity)",
        "''",
        "'abc'",
        "'a\"b'",
        "'a\\\\b'",
        "'\\n\\t\\r\\b\\f'",
        "'\\x00\\x01\\x1f'",
        "'\\u00e9\\u4e2d'",
        "'\\ud800'",
        "'\\udfff'",
        "'\\ud83d\\ude00'",
        "'\\u2028\\u2029'",
        "{}",
        "[]",
        "[1,2,3]",
        "{a:1,b:'x'}",
        "{a:{b:{c:[1,{d:2}]}}}",
        "[[[[1]]]]",
        "[1,undefined,function(){},null]",
        "{a:undefined,b:function(){},c:1}",
        "{'':1,' ':2,'a-b':3,'0':4,'1x':5}",
        "new Number(5)",
        "new Number(0/0)",
        "new String('s')",
        "new String('')",
        "new Boolean(true)",
        "new Boolean(false)",
        "new Date(0)",
        "new Date(1e12)",
        "new Date(0/0)",
        "/re/g",
        "function f(){}",
        "Math",
        "JSON",
        "(function(){var o={};o.self=o;return o})()",
        "(function(){var a=[];a[0]=a;return a})()",
        "(function(){var o={a:1};o.b={c:o};return o})()",
        "(function(){var a=[1,2,3];delete a[1];return a})()",
        "(function(){var a=[];a.length=3;return a})()",
        "(function(){var a=[1,2];a.foo='x';return a})()",
        "{toJSON:function(){return 42}}",
        "{toJSON:function(k){return 'k=' + k}}",
        "{toJSON:1}",
        "{toJSON:function(){throw 'tj'}}",
        "[new Date(1e12)]",
        "{a:[1,{b:[2,3]}],c:{}}",
        "Object.create(null)",
        "(function(){var o={}; Object.defineProperty(o,'h',{value:1,enumerable:false}); o.v=2; return o})()",
        "(function(){var o={}; Object.defineProperty(o,'g',{get:function(){return 7}}); return o})()",
        "new Error('boom')",
        "[Math, JSON, /x/]",
    ]
}

/// CONFIGS 408-419.
#[test]
fn t_json_stringify() {
    warm_dates();
    let replacers = [
        "",
        ", null",
        ", undefined",
        ", 5",
        ", 'x'",
        ", function(k,v){return v}",
        ", function(k,v){return typeof v === 'number' ? v*2 : v}",
        ", function(k,v){return k === 'b' ? undefined : v}",
        ", function(k,v){return undefined}",
        ", function(k,v){return k}",
        ", function(k,v){throw 'rep'}",
        ", ['a','c']",
        ", ['a']",
        ", [0,1]",
        ", []",
        ", ['a',1,new String('b'),new Number(2),true,null,{},undefined]",
        ", ['toJSON','self','length']",
    ];
    let spaces = [
        "",
        ", 0",
        ", 1",
        ", 2",
        ", 3",
        ", 9",
        ", 10",
        ", 11",
        ", 12",
        ", -1",
        ", -5",
        ", 1.5",
        ", 10.9",
        ", (0/0)",
        ", Infinity",
        ", ''",
        ", ' '",
        ", '  '",
        ", '\\t'",
        ", 'ab'",
        ", '0123456789'",
        ", '0123456789ABCDEF'",
        ", new Number(4)",
        ", new String('--')",
        ", null",
        ", true",
        ", {}",
        ", [1]",
    ];
    let vals = json_values();
    let mut exprs = vec![];
    for v in &vals {
        for r in replacers {
            exprs.push(format!("JSON.stringify({v}{r})"));
        }
        for s in spaces {
            exprs.push(format!("JSON.stringify({v}, null{s})"));
            exprs.push(format!("JSON.stringify({v}, undefined{s})"));
        }
    }
    // a few full cross combinations on nested data
    for v in [
        "{a:1,b:[1,2,{c:3}],d:{e:{f:[]}}}",
        "[1,[2,[3,[4]]]]",
        "{a:[],b:{}}",
    ] {
        for r in replacers {
            for s in spaces {
                exprs.push(format!("JSON.stringify({v}{r}{s})"));
            }
        }
    }
    exprs.push("JSON.stringify()".into());
    exprs.push("JSON.stringify.length".into());
    exprs.push("JSON.parse.length".into());
    exprs.push("Object.prototype.toString.call(JSON)".into());
    diff_lines_n("", &exprs.iter().map(|e| format!("dump({e});")).collect::<Vec<_>>(), 100);
}

/// CONFIGS 404-407.
#[test]
fn t_json_parse() {
    warm_dates();
    let texts: Vec<&str> = vec![
        "",
        " ",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1",
        "-1",
        "1.5",
        "-1.5e-3",
        "1e5",
        "1E+5",
        "1e-5",
        "0.5",
        r#""""#,
        r#""abc""#,
        r#""a\"b""#,
        r#""\n\t\r\b\f\/\\\"""#,
        r#""\u0041\u00e9\u4e2d""#,
        r#""\ud800""#,
        r#""\ud83d\ude00""#,
        "\"\u{4e2d}\u{6587}\"",
        "{}",
        "[]",
        "[1]",
        "[1,2,3]",
        "[[1],[2,[3]]]",
        "{\"a\":1}",
        "{\"a\":1,\"b\":2}",
        "{\"a\":{\"b\":[1,2,{\"c\":null}]}}",
        "[{\"a\":1},{\"a\":2}]",
        "{\"\":1}",
        "{\"0\":1,\"1\":2}",
        "{\"length\":3}",
        "[null,true,false,\"\",0]",
        " { \"a\" : [ 1 , 2 ] } ",
        "\n\t{\"a\":1}\n",
        // invalid shapes
        "01",
        "+1",
        ".5",
        "1.",
        "0x10",
        "'a'",
        "{a:1}",
        "{\"a\":1,}",
        "[1,]",
        "[,1]",
        "{\"a\"1}",
        "[1 2]",
        "undefined",
        "NaN",
        "Infinity",
        "nul",
        "tru",
        r#""unterminated"#,
        r#""\x41""#,
        r#""\0""#,
        r#""\'""#,
        "// c\n1",
        "/*c*/1",
        "1 2",
        "]",
        "}",
        "[",
        "{",
    ];
    let revivers = [
        "",
        ", null",
        ", undefined",
        ", 5",
        ", function(k,v){return v}",
        ", function(k,v){return typeof v === 'number' ? v+1 : v}",
        ", function(k,v){return k === 'a' ? undefined : v}",
        ", function(k,v){return undefined}",
        ", function(k,v){return k}",
        ", function(k,v){return this}",
        ", function(k,v){throw 'rev'}",
        ", function(k,v){return v instanceof Array ? v.length : v}",
    ];
    let mut exprs = vec![];
    for t in &texts {
        for r in revivers {
            exprs.push(format!("JSON.parse({}{r})", jsq(t)));
        }
    }
    // round-trip: stringify then parse
    for v in json_values() {
        exprs.push(format!(
            "(function(){{ var s = JSON.stringify({v}); return [s, JSON.parse(s)] }})()"
        ));
    }
    diff_lines_n("", &exprs.iter().map(|e| format!("dump({e});")).collect::<Vec<_>>(), 100);
}

/// Deep nesting exercises the recursive jsonvalue/fmtvalue descent.
#[test]
fn t_json_deep() {
    with_big_stack(body_t_json_deep);
}

fn body_t_json_deep() {
    warm_dates();
    let mut lines = vec![];
    for n in [1, 2, 5, 20, 50, 100, 150] {
        lines.push(format!(
            "var s = '{}1{}'; dump({n}, s.length); \
             try{{ var v = JSON.parse(s); dump('ok', JSON.stringify(v)) }} \
             catch(e){{ dump('CAUGHT', e) }}",
            "[".repeat(n),
            "]".repeat(n)
        ));
        lines.push(format!(
            "var s = '{}1{}'; \
             try{{ var v = JSON.parse(s); dump('ok2', JSON.stringify(v, null, 1).length) }} \
             catch(e){{ dump('CAUGHT', e) }}",
            "{\"a\":".repeat(n),
            "}".repeat(n)
        ));
        lines.push(format!(
            "var v = 1; for (var i = 0; i < {n}; ++i) v = [v]; \
             try{{ dump('s', JSON.stringify(v)) }} catch(e){{ dump('CAUGHT', e) }}"
        ));
        lines.push(format!(
            "var v = 1; for (var i = 0; i < {n}; ++i) v = {{a: v}}; \
             try{{ dump('o', JSON.stringify(v, null, 2).length) }} catch(e){{ dump('CAUGHT', e) }}"
        ));
    }
    diff_lines_n("", &lines, 8);
}

/* ===================================================================== */
/* Date                                                                  */
/* ===================================================================== */

const DATE_PRELUDE: &str = r#"
var FMT = ['toString','toDateString','toTimeString','toUTCString','toISOString',
           'toJSON','toLocaleString','toLocaleDateString','toLocaleTimeString',
           'valueOf','getTime'];
function GA(d) {
    dump('t', d.getTime(), d.valueOf());
    dump('L', d.getFullYear(), d.getMonth(), d.getDate(), d.getDay(),
         d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds(),
         d.getTimezoneOffset());
    dump('U', d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(),
         d.getUTCDay(), d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds(),
         d.getUTCMilliseconds());
    for (var i = 0; i < FMT.length; ++i) {
        var m = FMT[i];
        try { dump(m, d[m]()) } catch (e) { dump(m, 'CAUGHT', e) }
    }
    try { dump('json', JSON.stringify(d)) } catch (e) { dump('json', 'CAUGHT', e) }
    try { dump('str', '' + d) } catch (e) { dump('str', 'CAUGHT', e) }
    try { dump('num', +d) } catch (e) { dump('num', 'CAUGHT', e) }
    try { dump('rpr', d) } catch (e) { dump('rpr', 'CAUGHT', e) }
}
function SS(t, meth, args) {
    var d = new Date(t);
    var r;
    try { r = d[meth].apply(d, args) }
    catch (e) { dump(meth, 'CAUGHT', e); return }
    dump(meth, r, d.getTime());
    try { dump(' iso', d.toISOString()) } catch (e) { dump(' iso', 'CAUGHT', e) }
    dump(' s', d.toString(), d.toUTCString(), d.toDateString(), d.toTimeString());
}
"#;

fn date_ms() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "0".into(),
        "(-0)".into(),
        "1".into(),
        "(-1)".into(),
        "1000".into(),
        "86400000".into(),
        "(-86400000)".into(),
        "1234567890123".into(),
        "(-1234567890123)".into(),
        "8.64e15".into(),
        "(-8.64e15)".into(),
        "8640000000000001".into(),
        "(-8640000000000001)".into(),
        "1e16".into(),
        "(0/0)".into(),
        "Infinity".into(),
        "(-Infinity)".into(),
        "1.5".into(),
        "(-1.5)".into(),
        "999.999".into(),
        "(-999.999)".into(),
        "951782400000".into(),
        "2147483648000".into(),
        "(-2208988800000)".into(),
        "(-62135596800000)".into(),
        "253402300799999".into(),
    ];
    let mut rng = Rng::new(0xD47E_0001);
    for _ in 0..60 {
        let ms = rng.range(-8_640_000_000_000_000, 8_640_000_000_000_000) as f64;
        v.push(jsnum(ms));
    }
    v
}

fn date_strings() -> Vec<&'static str> {
    vec![
        "2020",
        "2020-05",
        "2020-05-17",
        "2020-05-17T12:34",
        "2020-05-17T12:34:56",
        "2020-05-17T12:34:56.789",
        "2020-05-17T12:34Z",
        "2020-05-17T12:34:56Z",
        "2020-05-17T12:34:56.789Z",
        "2020-05-17T12:34+05:30",
        "2020-05-17T12:34-08:00",
        "2020-05-17T12:34+05",
        "2020-05-17T12:34-08",
        "2020-05-17T12:34+00:00",
        "2020-05-17T00:00:00.000Z",
        "2020-05-17T24:00",
        "2020-05-17T24:00:00.000",
        "1970-01-01",
        "1970-01-01T00:00:00.000Z",
        "0000-01-01",
        "0001-01-01",
        "9999-12-31T23:59:59.999Z",
        "2020-02-29",
        "2019-02-29",
        "2020-01-31",
        "2020-12-31",
        // rejected shapes
        "",
        " ",
        "abc",
        "202",
        "20201",
        "2020-",
        "2020-1",
        "2020-13",
        "2020-00",
        "2020-05-",
        "2020-05-32",
        "2020-05-00",
        "2020-05-17T",
        "2020-05-17T12",
        "2020-05-17T12:",
        "2020-05-17T1234",
        "2020-05-17T24:00:01",
        "2020-05-17T24:01",
        "2020-05-17T25:00",
        "2020-05-17T12:60",
        "2020-05-17T12:34:60",
        "2020-05-17T12:34:56.1000",
        "2020-05-17T12:34+24:00",
        "2020-05-17T12:34+23:60",
        "2020-05-17T12:34+2",
        "2020-05-17T12:34x",
        "2020-05-17 12:34",
        "2020/05/17",
        "Sun May 17 2020",
        "-2020-05-17",
        "+2020-05-17",
        "2020-05-17Z",
    ]
}

/// CONFIGS 444-446: jsB_new_Date argument shapes.
#[test]
fn t_date_construct() {
    warm_dates();
    let mut lines = vec![];
    // 1 numeric argument
    for ms in date_ms() {
        lines.push(format!("GA(new Date({ms}));"));
        lines.push(format!("dump('num', new Date({ms}).getTime());"));
    }
    // 1 string argument (and Date.parse of the same text)
    for s in date_strings() {
        let q = jsq(s);
        lines.push(format!("GA(new Date({q}));"));
        lines.push(format!("dump('parse', Date.parse({q}));"));
    }
    // 1 argument of every other type -> ToPrimitive(HNONE)
    for a in [
        "true",
        "false",
        "null",
        "undefined",
        "{}",
        "[]",
        "[0]",
        "['2020-05-17']",
        "new Number(1e12)",
        "new String('2020-05-17')",
        "new Date(1e12)",
        "{valueOf:function(){return 1e12}}",
        "{toString:function(){return '2020-05-17'}}",
        "function(){}",
        "/x/",
    ] {
        lines.push(format!("GA(new Date({a}));"));
    }
    // `new Date()` is not deterministic: only its structure is compared.
    lines.push(
        "var d = new Date(); dump('now', typeof d, d instanceof Date, \
         d.getTime() > 1e12, d.getTime() < 1e14, typeof d.toString());"
            .into(),
    );
    diff_lines_n(DATE_PRELUDE, &lines, 20);
}

#[test]
fn t_date_construct_components() {
    warm_dates();
    let ys = [
        "0", "1", "50", "99", "99.9", "100", "1899", "1900", "1969", "1970",
        "2020", "-1", "-100", "275760", "-271821", "1e6", "(0/0)", "Infinity",
        "1.9", "'2020'", "null", "undefined", "true", "{}",
    ];
    let ms = [
        "0", "1", "11", "12", "13", "-1", "-13", "1.5", "(0/0)", "Infinity",
        "'3'", "null", "undefined", "true", "24", "1e6",
    ];
    let ds = ["0", "1", "15", "31", "32", "100", "-5", "1.5", "(0/0)", "undefined", "'7'"];
    let hs = ["0", "12", "23", "24", "100", "-1", "1.5", "(0/0)", "undefined"];
    let mins = ["0", "30", "59", "60", "-1", "1.5", "(0/0)", "undefined"];
    let secs = ["0", "30", "59", "60", "-1", "1.5", "(0/0)", "undefined"];
    let mss = ["0", "1", "500", "999", "1000", "-1", "1.5", "(0/0)", "undefined"];

    let mut lines = vec![];
    // 2 args (the minimum for the component path), varying both
    for y in ys {
        for m in ms {
            lines.push(format!("GA(new Date({y}, {m}));"));
            lines.push(format!("dump('utc2', Date.UTC({y}, {m}));"));
        }
    }
    // 3..7 args
    for d in ds {
        lines.push(format!("GA(new Date(2020, 5, {d}));"));
        lines.push(format!("dump('utc3', Date.UTC(2020, 5, {d}));"));
    }
    for h in hs {
        lines.push(format!("GA(new Date(2020, 5, 17, {h}));"));
        lines.push(format!("dump('utc4', Date.UTC(2020, 5, 17, {h}));"));
    }
    for mi in mins {
        lines.push(format!("GA(new Date(2020, 5, 17, 12, {mi}));"));
        lines.push(format!("dump('utc5', Date.UTC(2020, 5, 17, 12, {mi}));"));
    }
    for s in secs {
        lines.push(format!("GA(new Date(2020, 5, 17, 12, 34, {s}));"));
        lines.push(format!("dump('utc6', Date.UTC(2020, 5, 17, 12, 34, {s}));"));
    }
    for m in mss {
        lines.push(format!("GA(new Date(2020, 5, 17, 12, 34, 56, {m}));"));
        lines.push(format!(
            "dump('utc7', Date.UTC(2020, 5, 17, 12, 34, 56, {m}));"
        ));
    }
    // Date.UTC with 0 and 1 arguments (js_tonumber of a missing slot)
    for extra in [
        "dump('u0', Date.UTC())",
        "dump('u1', Date.UTC(2020))",
        "dump('u1', Date.UTC(70))",
        "dump('u8', Date.UTC(2020,5,17,12,34,56,789,999))",
        "dump('d8', new Date(2020,5,17,12,34,56,789,999).getTime())",
        "dump('now', typeof Date.now(), Date.now() > 1e12)",
        "dump('df', typeof Date(), Date().length > 0)",
        "dump('len', Date.length, Date.UTC.length, Date.parse.length)",
    ] {
        lines.push(extra.to_string());
    }
    // randomised component tuples
    let mut rng = Rng::new(0xDA7E_C0DE);
    for _ in 0..600 {
        let y = rng.range(-3000, 4000);
        let m = rng.range(-20, 30);
        let d = rng.range(-40, 60);
        let h = rng.range(-5, 30);
        let mi = rng.range(-5, 70);
        let s = rng.range(-5, 70);
        let msv = rng.range(-50, 1500);
        lines.push(format!("GA(new Date({y}, {m}, {d}, {h}, {mi}, {s}, {msv}));"));
        lines.push(format!(
            "dump('U', Date.UTC({y}, {m}, {d}, {h}, {mi}, {s}, {msv}));"
        ));
    }
    diff_lines_n(DATE_PRELUDE, &lines, 20);
}

/// Every setter, including the jsdate.c:748 quirk where Dp_setUTCHours
/// defaults its minutes argument from HourFromTime(t) instead of MinFromTime(t).
#[test]
fn t_date_setters() {
    warm_dates();
    let setters: [(&str, usize); 15] = [
        ("setTime", 1),
        ("setMilliseconds", 1),
        ("setUTCMilliseconds", 1),
        ("setSeconds", 2),
        ("setUTCSeconds", 2),
        ("setMinutes", 3),
        ("setUTCMinutes", 3),
        ("setHours", 4),
        ("setUTCHours", 4),
        ("setDate", 1),
        ("setUTCDate", 1),
        ("setMonth", 2),
        ("setUTCMonth", 2),
        ("setFullYear", 3),
        ("setUTCFullYear", 3),
    ];
    let vals = [
        "0", "1", "5", "12", "31", "59", "60", "99", "1970", "2020", "-1",
        "-100", "1.5", "(0/0)", "Infinity", "(-Infinity)", "1e10", "8.64e15",
        "'7'", "null", "undefined", "true", "{}", "[2]",
    ];
    let bases = [
        "0",
        "1234567890123",
        "(-1234567890123)",
        "(0/0)",
        "8.64e15",
        "86399999",
        "(-1)",
    ];
    let mut lines = vec![];
    for (m, arity) in setters {
        for base in bases {
            // no arguments at all
            lines.push(format!("SS({base}, '{m}', []);"));
            for v in vals {
                lines.push(format!("SS({base}, '{m}', [{v}]);"));
            }
            if arity >= 2 {
                for v in ["0", "5", "59", "-1", "(0/0)", "undefined", "1.5"] {
                    lines.push(format!("SS({base}, '{m}', [3, {v}]);"));
                }
            }
            if arity >= 3 {
                for v in ["0", "5", "59", "-1", "(0/0)", "undefined"] {
                    lines.push(format!("SS({base}, '{m}', [3, 4, {v}]);"));
                }
            }
            if arity >= 4 {
                for v in ["0", "5", "999", "-1", "(0/0)", "undefined"] {
                    lines.push(format!("SS({base}, '{m}', [3, 4, 5, {v}]);"));
                }
            }
            // more arguments than the declared arity
            lines.push(format!("SS({base}, '{m}', [1,2,3,4,5,6]);"));
        }
    }
    // setters on a non-date `this`
    for extra in [
        "try{ dump(Date.prototype.setTime.call({}, 0)) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.getTime.call({})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.getTime.call(5)) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.toISOString.call({})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.getTime.call(Date.prototype)) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.toString.call(Date.prototype)) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.toJSON.call({toISOString:function(){return 'X'}})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Date.prototype.toJSON.call({})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(new Date(0/0).toJSON()) }catch(e){ dump('CAUGHT', e) }",
    ] {
        lines.push(extra.to_string());
    }
    diff_lines_n(DATE_PRELUDE, &lines, 40);
}

/* ===================================================================== */
/* Math                                                                  */
/* ===================================================================== */

const MATH_PRELUDE: &str = r#"
function MA(x) {
    dump(x, Math.abs(x), Math.acos(x), Math.asin(x), Math.atan(x),
         Math.ceil(x), Math.cos(x), Math.exp(x), Math.floor(x), Math.log(x),
         Math.round(x), Math.sin(x), Math.sqrt(x), Math.tan(x));
    dump('mm', Math.max(x), Math.min(x), Math.max(x, x), Math.min(x, x));
}
function M2(x, y) {
    dump(x, y, Math.atan2(x, y), Math.pow(x, y), Math.max(x, y), Math.min(x, y),
         Math.max(x, y, 0), Math.min(x, y, 0), Math.max(0, x, y),
         Math.min(0, x, y));
}
"#;

fn math_values() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "0", "(-0)", "1", "(-1)", "0.5", "(-0.5)", "2", "(-2)", "0.1", "(0/0)",
        "Infinity", "(-Infinity)", "Math.PI", "(-Math.PI)", "Math.PI/2",
        "Math.E", "1e-300", "1e300", "9007199254740992", "(-9007199254740992)",
        "0.49999999999999994", "4.5", "(-4.5)", "2.5", "(-2.5)", "1e21",
        "(-1e21)", "5e-324", "1.7976931348623157e308", "2147483647",
        "(-2147483648)", "4294967295", "2147483648", "4294967296",
        "1023.9999999999999", "(-0.9999999999999999)", "1", "0.9999999999999999",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mut rng = Rng::new(0x3141_5926);
    for _ in 0..420 {
        v.push(jsnum(rng.f64_sane()));
    }
    v
}

#[test]
fn t_math_unary() {
    warm_dates();
    let vals = math_values();
    let mut lines: Vec<String> = vals.iter().map(|v| format!("MA({v});")).collect();
    for extra in [
        "dump('c', Math.E, Math.LN10, Math.LN2, Math.LOG2E, Math.LOG10E, Math.PI, Math.SQRT1_2, Math.SQRT2)",
        "dump('n', Math.abs(), Math.acos(), Math.ceil(), Math.floor(), Math.round(), Math.sqrt())",
        "dump('s', Math.abs('-3'), Math.floor('2.5'), Math.round(null), Math.ceil([]), Math.sqrt([4]))",
        "dump('o', Math.abs({}), Math.floor({}), Math.round(undefined))",
        "dump('x', Math.max(), Math.min())",
        "dump('r', Object.prototype.toString.call(Math), typeof Math.random)",
        "dump('l', Math.abs.length, Math.max.length, Math.pow.length, Math.atan2.length)",
    ] {
        lines.push(extra.to_string());
    }
    diff_lines_n(MATH_PRELUDE, &lines, 60);
}

#[test]
fn t_math_binary() {
    warm_dates();
    let base = [
        "0", "(-0)", "1", "(-1)", "0.5", "(-0.5)", "2", "(-2)", "(0/0)",
        "Infinity", "(-Infinity)", "1e300", "1e-300", "3",
    ];
    let mut lines = vec![];
    for a in base {
        for b in base {
            lines.push(format!("M2({a}, {b});"));
        }
    }
    let mut rng = Rng::new(0x2718_2818);
    for _ in 0..700 {
        let a = jsnum(rng.f64_sane());
        let b = jsnum(rng.f64_sane());
        lines.push(format!("M2({a}, {b});"));
    }
    for extra in [
        "dump('mx', Math.max(1,2,3), Math.min(1,2,3), Math.max(-0,0), Math.min(0,-0), Math.max(0,-0), Math.min(-0,0))",
        "dump('nan', Math.max(1,0/0,2), Math.min(1,0/0,2), Math.max(0/0,1), Math.min(0/0,1))",
        "dump('p1', Math.pow(1, Infinity), Math.pow(-1, Infinity), Math.pow(1, -Infinity), Math.pow(-1, 0/0))",
        "dump('p2', Math.pow(0, 0), Math.pow(-0, -1), Math.pow(0, -1), Math.pow(-0, 3), Math.pow(-2, 0.5))",
        "dump('p3', Math.pow(), Math.atan2())",
        "dump('p4', Math.pow('2','3'), Math.atan2('1','1'))",
    ] {
        lines.push(extra.to_string());
    }
    diff_lines_n(MATH_PRELUDE, &lines, 60);
}

/// Math.random() must never be compared by value, only for range and shape.
#[test]
fn t_math_random() {
    warm_dates();
    let src = r#"
var ok = true, lo = 1, hi = 0, n = 0;
for (var i = 0; i < 5000; ++i) {
    var r = Math.random();
    if (typeof r !== 'number' || !(r >= 0) || !(r < 1)) ok = false;
    if (r < lo) lo = r;
    if (r > hi) hi = r;
    if (r !== r) ok = false;
    ++n;
}
print('range-ok', ok, 'n', n, 'lo>=0', lo >= 0, 'hi<1', hi < 1);
print('typeof', typeof Math.random(), 'args-ignored',
      typeof Math.random(1, 2, 3));
"#;
    diff_dostring(0, src);
}

/* ===================================================================== */
/* Number.prototype                                                      */
/* ===================================================================== */

const NUM_PRELUDE: &str = r#"
function NF(x) {
    for (var w = -2; w <= 22; ++w) {
        try { dump('F', w, x.toFixed(w)) } catch (e) { dump('F', w, 'CAUGHT', e) }
        try { dump('E', w, x.toExponential(w)) } catch (e) { dump('E', w, 'CAUGHT', e) }
        try { dump('P', w, x.toPrecision(w)) } catch (e) { dump('P', w, 'CAUGHT', e) }
    }
    for (var r = 0; r <= 38; ++r) {
        try { dump('R', r, x.toString(r)) } catch (e) { dump('R', r, 'CAUGHT', e) }
    }
    try { dump('X', x.toString(), x.toLocaleString(), x.valueOf(), '' + x, x + 0) }
    catch (e) { dump('X', 'CAUGHT', e) }
    try { dump('A', x.toFixed(), x.toExponential()) } catch (e) { dump('A', 'CAUGHT', e) }
    try { dump('B', x.toPrecision()) } catch (e) { dump('B', 'CAUGHT', e) }
    try { dump('U', x.toString(undefined), x.toString(null), x.toString(0/0),
                    x.toString('16'), x.toString(16.9), x.toString(true)) }
    catch (e) { dump('U', 'CAUGHT', e) }
    try { dump('W', x.toFixed('3'), x.toFixed(3.9), x.toFixed(null),
                    x.toFixed(0/0), x.toFixed(true), x.toFixed([2])) }
    catch (e) { dump('W', 'CAUGHT', e) }
    try { dump('N', new Number(x).toString(), new Number(x).toFixed(4),
                    new Number(x).toString(16)) }
    catch (e) { dump('N', 'CAUGHT', e) }
}
"#;

fn number_values() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "0", "(-0)", "1", "(-1)", "0.5", "1/3", "1e-7", "1e21", "1e-21",
        "123456789", "1e20", "9.99e20", "999999999999999999999", "1e21-1",
        "9007199254740992", "(0/0)", "Infinity", "(-Infinity)", "5e-324",
        "1.7976931348623157e308", "255", "4095", "0.1", "(-0.1)", "100",
        "1000000", "0.000001", "0.0000001", "12345.6789", "(-12345.6789)",
        "2", "36", "35", "1023", "1024", "(-1e-300)", "1e-300",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let mut rng = Rng::new(0x1618_0339);
    for _ in 0..80 {
        v.push(jsnum(rng.f64_sane()));
    }
    v
}

#[test]
fn t_number_formatting() {
    warm_dates();
    let vals = number_values();
    let lines: Vec<String> = vals.iter().map(|v| format!("NF({v});")).collect();
    diff_lines_n(NUM_PRELUDE, &lines, 6);
}

#[test]
fn t_number_misc() {
    warm_dates();
    let mut lines: Vec<String> = vec![];
    for extra in [
        "dump(Number.MAX_VALUE, Number.MIN_VALUE, Number.NaN, Number.NEGATIVE_INFINITY, Number.POSITIVE_INFINITY)",
        "dump(Number(), Number(''), Number('  12  '), Number('0x10'), Number('abc'), Number(null), Number(undefined), Number(true), Number([]), Number([5]), Number({}))",
        "dump(new Number().valueOf(), new Number('5').valueOf(), typeof new Number(1))",
        "dump(Number.prototype.valueOf.call(new Number(3)))",
        "try{ dump(Number.prototype.valueOf.call({})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Number.prototype.toFixed.call('3', 2)) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump(Number.prototype.toString.call({}, 2)) }catch(e){ dump('CAUGHT', e) }",
        "dump(Number.prototype.toString.call(new Number(255), 16))",
        "dump((255).toString(16), (255).toString(2), (255).toString(8), (255).toString(36))",
        "dump((0.5).toString(2), (0.1).toString(2), (1/3).toString(3))",
        "dump((-255).toString(16), (-0.5).toString(2))",
        "dump((1e21).toString(2).length, (1e-21).toString(2).length)",
        "dump((5e-324).toString(2).length, (5e-324).toString(36).length)",
        "dump(Number.prototype.toString.length, Number.prototype.toFixed.length)",
        "dump((0).toFixed(0), (0).toFixed(20), (-0).toFixed(2))",
        "dump((1e20).toFixed(20), (1e21).toFixed(2), (-1e21).toFixed(2))",
        "dump((1.005).toFixed(2), (1.45).toFixed(1), (0.5).toFixed(0), (1.5).toFixed(0), (2.5).toFixed(0))",
        "dump((123.456).toExponential(0), (123.456).toExponential(2), (0).toExponential(2))",
        "dump((0.000001).toExponential(3), (1e-100).toExponential(3), (1e100).toExponential(3))",
        "dump((123.456).toPrecision(1), (123.456).toPrecision(5), (0).toPrecision(3))",
        "dump((1e-7).toPrecision(3), (1e21).toPrecision(3), (1e21).toPrecision(21))",
        "dump((0/0).toFixed(2), Infinity.toFixed(2), (-Infinity).toExponential(2), (0/0).toPrecision(3))",
    ] {
        lines.push(extra.to_string());
    }
    // radix over the full accepted range on a fixed set of values
    for v in [
        "0", "1", "(-1)", "255", "4095", "0.5", "(0.1)", "123456789",
        "1e21", "5e-324", "(0/0)", "Infinity", "(-Infinity)", "(-0)",
        "9007199254740992", "1e-300", "1e300",
    ] {
        for r in 2..=36 {
            lines.push(format!("dump('r{r}', ({v}).toString({r}));"));
        }
        for r in ["0", "1", "37", "38", "-1", "1e10", "(0/0)", "2.9", "36.9"] {
            lines.push(format!("dump('b', ({v}).toString({r}));"));
        }
    }
    diff_lines_n("", &lines, 80);
}

/* ===================================================================== */
/* cross-cutting: strict mode                                            */
/* ===================================================================== */

/// A representative slice of the whole surface re-run with JS_STRICT, where
/// the read-only regexp properties and the array `length` throw instead of
/// silently dropping the write.
#[test]
fn t_strict_mode_slice() {
    warm_dates();
    let srcs = [
        "var r=/a/g; r.source='x'; print(r.source)",
        "var r=/a/g; r.global=0; print(r.global)",
        "var r=/a/g; r.lastIndex=3; print(r.lastIndex)",
        "var r=/a/g; print(delete r.lastIndex)",
        "var a=[1,2,3]; a.length=1; print(a.length)",
        "var a=[1,2,3]; print(delete a.length)",
        "print('abc'.split('b'), 'abc'.replace('b','X'))",
        "print(JSON.stringify({a:1},null,2))",
        "print(JSON.parse('[1,2]'))",
        "print(new Date(0).toISOString())",
        "print((255).toString(16), (1.5).toFixed(3))",
        "print(Math.round(-0.3), 1/Math.round(-0.3))",
        "print([3,1,2].sort())",
        "print('abc'.length, 'abc'[1])",
        "var s='abc'; s.length=9; print(s.length)",
        "var s=new String('abc'); s.length=9; print(s.length)",
        "var s=new String('abc'); s[0]='z'; print(s[0])",
        "print('x'.substr)",
        "print([].reduce)",
    ];
    for src in srcs {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
        diff_eval(0, src);
        diff_eval(JS_STRICT, src);
    }
}

/* ===================================================================== */
/* randomised fuzzing                                                    */
/* ===================================================================== */

/// Regexp fragments that always compile, so a fuzzed pattern is guaranteed to
/// reach the match engine rather than the syntaxerror path.
const RE_FRAGS: &[&str] = &[
    "a", "b", "c", "x", ".", "\\\\d", "\\\\w", "\\\\s", "\\\\D", "\\\\W", "\\\\S",
    "[abc]", "[^abc]", "[a-z]", "[0-9]", "^", "$", "\\\\b", "\\\\B", "(a)",
    "(?:b)", "(a|b)", "(?=a)", "(?!a)", "\\\\.", "\\\\n", "\\\\x41", "\\\\u0062",
    "[\\\\w-]", "[^\\\\s]", ",", "-", "\\u00e9", "\\u4e2d", "(a)(b)", "([a-c])",
];
const RE_QUANTS: &[&str] = &["", "*", "+", "?", "{2}", "{1,3}", "{0,2}", "*?", "+?", "??", "{2,}"];

fn rand_pattern(rng: &mut Rng) -> String {
    let n = 1 + rng.below(4) as usize;
    let mut pat = String::new();
    for i in 0..n {
        if i > 0 && rng.below(5) == 0 {
            pat.push('|');
        }
        pat.push_str(RE_FRAGS[rng.below(RE_FRAGS.len() as u32) as usize]);
        pat.push_str(RE_QUANTS[rng.below(RE_QUANTS.len() as u32) as usize]);
    }
    pat
}

fn rand_flags(rng: &mut Rng) -> &'static str {
    match rng.below(8) {
        0 => "",
        1 => "g",
        2 => "i",
        3 => "m",
        4 => "gi",
        5 => "gm",
        6 => "im",
        _ => "gim",
    }
}

/// Randomised String.prototype.match / search / split / replace over fuzzed
/// regexps (CONFIGS 420-437).
#[test]
fn t_regexp_string_fuzz() {
    warm_dates();
    let reps = [
        "'<$&>'", "'$1'", "'$2|$10|$99'", "'$`/$\\''", "'$$'", "'$'", "'$x'", "''",
        "function(){var s='';for(var i=0;i<arguments.length;++i)s+='|'+arguments[i];return s}",
        "function(m){return m.length}",
    ];
    let mut rng = Rng::new(0xF0F0_1234);
    let mut lines = vec![];
    for _ in 0..1400 {
        let pat = rand_pattern(&mut rng);
        let fl = rand_flags(&mut rng);
        let subj = match rng.below(4) {
            0 => rng.ascii_string(16),
            1 => rng.unicode_string(8),
            2 => {
                let mut s = String::new();
                for _ in 0..rng.below(6) {
                    s.push_str(["a", "b", "c", ",", "\n", "ab", "x", "\u{4e2d}"][rng.below(8) as usize]);
                }
                s
            }
            _ => "aXbXc,d\ne".to_string(),
        };
        let rep = reps[rng.below(reps.len() as u32) as usize];
        let lim = ["", ", 0", ", 1", ", 3", ", -1", ", 1e9"][rng.below(6) as usize];
        lines.push(format!(
            "var r = new RegExp('{}', '{}'); var s = {};\n\
             dump('re', r, r.source, r.global, r.ignoreCase, r.multiline);\n\
             for (var k = 0; k < 4; ++k) dump('x', k, r.exec(s), r.lastIndex);\n\
             r.lastIndex = 0;\n\
             for (var k = 0; k < 4; ++k) dump('T', k, r.test(s), r.lastIndex);\n\
             r.lastIndex = 0; dump('m', s.match(r), r.lastIndex);\n\
             r.lastIndex = 0; dump('h', s.search(r));\n\
             r.lastIndex = 0; dump('p', s.split(r{}), r.lastIndex);\n\
             r.lastIndex = 0; dump('c', s.replace(r, {}), r.lastIndex);",
            pat, fl, jsq(&subj), lim, rep
        ));
    }
    diff_lines_n("", &lines, 40);
}

/// Randomised String.prototype method sweep over ASCII and multi-byte inputs.
#[test]
fn t_string_fuzz() {
    warm_dates();
    let mut rng = Rng::new(0x1357_9BDF);
    let mut lines = vec![];
    for _ in 0..2500 {
        let s = match rng.below(3) {
            0 => rng.ascii_string(20),
            1 => rng.unicode_string(10),
            _ => {
                // strings built from boundary runes
                let cps = [
                    0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0x41, 0xd7ff,
                    0xe000, 0xdf, 0x130, 0x131, 0x17f, 0xfb03,
                ];
                let mut t = String::new();
                for _ in 0..rng.below(6) {
                    let c = cps[rng.below(cps.len() as u32) as usize];
                    if let Some(ch) = char::from_u32(c) {
                        t.push(ch);
                    }
                }
                t
            }
        };
        let n = match rng.below(4) {
            0 => rng.ascii_string(3),
            1 => rng.unicode_string(2),
            2 => String::new(),
            _ => s.chars().take(1 + rng.below(3) as usize).collect(),
        };
        let two = idx_nonempty();
        let a = two[rng.below(two.len() as u32) as usize];
        let b = two[rng.below(two.len() as u32) as usize];
        let q = jsq(&s);
        let nq = jsq(&n);
        lines.push(format!(
            "var s = {q}, n = {nq};\n\
             dump('L', s.length, s.charAt({a}), s.charCodeAt({a}));\n\
             dump('S', s.slice({a}), s.slice({a},{b}), s.substring({a}), s.substring({a},{b}));\n\
             dump('I', s.indexOf(n), s.indexOf(n,{a}), s.lastIndexOf(n), s.lastIndexOf(n,{a}));\n\
             dump('C', s.toUpperCase(), s.toLowerCase(), s.trim(), s.concat(n, 5, null));\n\
             dump('K', s.localeCompare(n), n.localeCompare(s), s.localeCompare(s));\n\
             dump('P', s.split(n), s.split(n, 3), s.split(''), s.split(undefined));\n\
             dump('R', s.replace(n, '[$&]'), s.replace(n, function(m,o,w){{return o + '/' + w.length}}));"
        ));
    }
    diff_lines_n("", &lines, 40);
}

/// Randomised Array.prototype op sequences over randomised (and sparse)
/// arrays.  Exercises the flat / unflattened transitions in jsrun.c.
#[test]
fn t_array_fuzz() {
    warm_dates();
    let elems = [
        "0", "1", "2", "-1", "'a'", "'b'", "'10'", "true", "false", "null",
        "undefined", "{}", "[]", "[1]", "0/0", "1/0", "-0", "'\\u4e2d'",
        "1e21", "function(){}",
    ];
    let ops = [
        "a.push(7)", "a.pop()", "a.shift()", "a.unshift(8)", "a.reverse()",
        "a.sort()", "a.sort(function(x,y){return x<y?-1:x>y?1:0})",
        "a.splice(1,1)", "a.splice(0,0,'q')", "a.slice(1,-1)", "a.concat([9])",
        "a.join('-')", "a.toString()", "a.indexOf(1)", "a.lastIndexOf(1)",
        "a.map(function(x){return x})", "a.filter(function(x){return !!x})",
        "a.every(function(x){return x !== 0})", "a.some(function(x){return !x})",
        "a.reduce(function(p,c){return String(p)+String(c)}, '')",
        "a.reduceRight(function(p,c){return String(p)+String(c)}, '')",
        "(a.length = 3)", "(a.length = 0)", "(a.length = 9)", "(delete a[0])",
        "(delete a[1])", "(a[9] = 'g')", "(a.zz = 'p')", "a.length",
        "JSON.stringify(a)", "a.forEach(function(){})",
    ];
    let mut rng = Rng::new(0xBEEF_2468);
    let mut lines = vec![];
    for _ in 0..1600 {
        let n = rng.below(9) as usize;
        let mut items = vec![];
        for _ in 0..n {
            items.push(elems[rng.below(elems.len() as u32) as usize]);
        }
        let lit = format!("[{}]", items.join(","));
        let mut body = format!("var a = {lit};");
        // an optional shape mutation before the ops
        match rng.below(6) {
            0 => body.push_str(" delete a[0];"),
            1 => body.push_str(" a.length = 6;"),
            2 => body.push_str(" a[6] = 'z';"),
            3 => body.push_str(" a.tag = 't';"),
            4 => body.push_str(" a.length = 1;"),
            _ => {}
        }
        for _ in 0..3 {
            let op = ops[rng.below(ops.len() as u32) as usize];
            body.push_str(&format!(" dump('o', {op}); dump('a', a, a.length);"));
        }
        lines.push(body);
    }
    diff_lines_n("", &lines, 60);
}

/// Generate a random JS value expression (nested objects/arrays/scalars).
fn rand_json_value(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 || rng.below(3) == 0 {
        return match rng.below(18) {
            0 => "null".into(),
            1 => "true".into(),
            2 => "false".into(),
            3 => "undefined".into(),
            4 => "0".into(),
            5 => "-0".into(),
            6 => "(0/0)".into(),
            7 => "Infinity".into(),
            8 => jsnum(rng.f64_sane()),
            9 => format!("{}", rng.range(-1000, 1000)),
            10 => jsq(&rng.ascii_string(8)),
            11 => jsq(&rng.unicode_string(5)),
            12 => "function(){}".into(),
            13 => "new Date(1e12)".into(),
            14 => "new Number(3)".into(),
            15 => "new String('w')".into(),
            16 => "new Boolean(false)".into(),
            _ => "/rx/g".into(),
        };
    }
    if rng.below(2) == 0 {
        let n = rng.below(4) as usize;
        let items: Vec<String> = (0..n).map(|_| rand_json_value(rng, depth - 1)).collect();
        format!("[{}]", items.join(","))
    } else {
        let n = rng.below(4) as usize;
        let items: Vec<String> = (0..n)
            .map(|i| {
                let k = match rng.below(4) {
                    0 => format!("k{i}"),
                    1 => "a".to_string(),
                    2 => format!("{i}"),
                    _ => rng.ascii_string(4).replace(['\\', '"', '\''], "_"),
                };
                format!("{}: {}", jsq(&k), rand_json_value(rng, depth - 1))
            })
            .collect();
        format!("{{{}}}", items.join(","))
    }
}

/// Randomised JSON.stringify / JSON.parse round-trips (CONFIGS 404-419).
#[test]
fn t_json_fuzz() {
    warm_dates();
    let replacers = [
        "null",
        "undefined",
        "function(k,v){return v}",
        "function(k,v){return typeof v === 'number' ? v + 1 : v}",
        "function(k,v){return k === 'a' ? undefined : v}",
        "['a','k0','k1','0','1']",
        "[]",
    ];
    let spaces = ["", ", 0", ", 1", ", 4", ", 10", ", 12", ", '\\t'", ", 'xy'", ", '0123456789ABC'"];
    let revivers = [
        "",
        ", function(k,v){return v}",
        ", function(k,v){return k === 'a' ? undefined : v}",
        ", function(k,v){return typeof v === 'number' ? -v : v}",
    ];
    let mut rng = Rng::new(0x50DA_5011);
    let mut lines = vec![];
    for _ in 0..1200 {
        let v = rand_json_value(&mut rng, 3);
        let r = replacers[rng.below(replacers.len() as u32) as usize];
        let sp = spaces[rng.below(spaces.len() as u32) as usize];
        let rv = revivers[rng.below(revivers.len() as u32) as usize];
        lines.push(format!(
            "var v = {v};\n\
             var s = JSON.stringify(v, {r}{sp});\n\
             dump('s', s);\n\
             if (s !== undefined) {{ dump('p', JSON.parse(s{rv})); \
             dump('rt', JSON.stringify(JSON.parse(s))); }}"
        ));
    }
    diff_lines_n("", &lines, 40);
}

/// Randomised Date construction / getter / setter sweeps (CONFIGS 444-450).
#[test]
fn t_date_fuzz() {
    warm_dates();
    let setters = [
        "setTime", "setMilliseconds", "setUTCMilliseconds", "setSeconds",
        "setUTCSeconds", "setMinutes", "setUTCMinutes", "setHours",
        "setUTCHours", "setDate", "setUTCDate", "setMonth", "setUTCMonth",
        "setFullYear", "setUTCFullYear",
    ];
    let mut rng = Rng::new(0x0DA7_E001);
    let mut lines = vec![];
    for _ in 0..900 {
        let base = match rng.below(6) {
            0 => "(0/0)".to_string(),
            1 => "0".to_string(),
            2 => jsnum(rng.range(-8_640_000_000_000_000, 8_640_000_000_000_000) as f64),
            3 => jsnum(rng.range(-100_000_000_000, 100_000_000_000) as f64),
            4 => jsnum(rng.range(-1000, 1000) as f64),
            _ => jsnum(rng.range(-8_700_000_000_000_000, 8_700_000_000_000_000) as f64),
        };
        let m = setters[rng.below(setters.len() as u32) as usize];
        let nargs = rng.below(5) as usize;
        let args: Vec<String> = (0..nargs)
            .map(|_| match rng.below(8) {
                0 => "undefined".to_string(),
                1 => "(0/0)".to_string(),
                2 => "Infinity".to_string(),
                3 => format!("{}", rng.range(-100, 100)),
                4 => format!("{}", rng.range(0, 60)),
                5 => jsnum(rng.range(-1000, 1000) as f64 + 0.5),
                6 => format!("'{}'", rng.range(0, 60)),
                _ => jsnum(rng.range(-10_000_000_000, 10_000_000_000) as f64),
            })
            .collect();
        lines.push(format!("GA(new Date({base}));"));
        lines.push(format!("SS({base}, '{m}', [{}]);", args.join(",")));
    }
    // randomised ISO-8601-ish strings, valid and invalid
    for _ in 0..600 {
        let y = rng.range(0, 10000);
        let mo = rng.range(0, 15);
        let d = rng.range(0, 35);
        let h = rng.range(0, 27);
        let mi = rng.range(0, 63);
        let s = rng.range(0, 63);
        let ms = rng.range(0, 1100);
        let tz = match rng.below(6) {
            0 => "Z".to_string(),
            1 => format!("+{:02}:{:02}", rng.range(0, 26), rng.range(0, 62)),
            2 => format!("-{:02}:{:02}", rng.range(0, 26), rng.range(0, 62)),
            3 => format!("+{:02}", rng.range(0, 26)),
            4 => String::new(),
            _ => "x".to_string(),
        };
        let text = match rng.below(6) {
            0 => format!("{y:04}"),
            1 => format!("{y:04}-{mo:02}"),
            2 => format!("{y:04}-{mo:02}-{d:02}"),
            3 => format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}{tz}"),
            4 => format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}{tz}"),
            _ => format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}{tz}"),
        };
        let q = jsq(&text);
        lines.push(format!("dump('P', {q}, Date.parse({q}));"));
        lines.push(format!("GA(new Date({q}));"));
    }
    diff_lines_n(DATE_PRELUDE, &lines, 24);
}

/// Randomised Number.prototype.toFixed / toExponential / toPrecision /
/// toString(radix) over arbitrary doubles.
#[test]
fn t_number_fuzz() {
    warm_dates();
    let mut rng = Rng::new(0x2A2A_7777);
    let mut lines = vec![];
    for _ in 0..3000 {
        let x = jsnum(rng.f64_sane());
        let w = rng.range(-3, 24);
        let r = rng.range(-2, 40);
        lines.push(format!(
            "var x = {x};\n\
             try {{ dump('F', x.toFixed({w})) }} catch (e) {{ dump('F', 'CAUGHT', e) }}\n\
             try {{ dump('E', x.toExponential({w})) }} catch (e) {{ dump('E', 'CAUGHT', e) }}\n\
             try {{ dump('P', x.toPrecision({w})) }} catch (e) {{ dump('P', 'CAUGHT', e) }}\n\
             try {{ dump('R', x.toString({r})) }} catch (e) {{ dump('R', 'CAUGHT', e) }}\n\
             dump('X', x, x.toString(), String(x));"
        ));
    }
    // every digit count 0..21 and every radix 2..36 on random doubles
    for _ in 0..120 {
        let x = jsnum(rng.f64_sane());
        lines.push(format!(
            "var x = {x};\n\
             for (var w = 0; w <= 21; ++w) {{\n\
             try {{ dump('f', w, x.toFixed(w)) }} catch (e) {{ dump('f', w, 'CAUGHT', e) }}\n\
             try {{ dump('e', w, x.toExponential(w)) }} catch (e) {{ dump('e', w, 'CAUGHT', e) }}\n\
             try {{ dump('p', w, x.toPrecision(w)) }} catch (e) {{ dump('p', w, 'CAUGHT', e) }}\n\
             }}\n\
             for (var b = 2; b <= 36; ++b) {{\n\
             try {{ dump('r', b, x.toString(b)) }} catch (e) {{ dump('r', b, 'CAUGHT', e) }}\n\
             }}"
        ));
    }
    diff_lines_n("", &lines, 60);
}

/// Objects with many randomly-ordered keys: JSON.stringify's fmtobject walks
/// the property AA-tree through js_pushiterator, so the emitted order (and
/// `Object.keys`, and js_repr) is only equal if the tree is built identically
/// (CONFIGS 419).
#[test]
fn t_json_object_key_order() {
    warm_dates();
    let mut rng = Rng::new(0x0B0E_1234);
    let mut lines = vec![];
    for _ in 0..500 {
        let n = 1 + rng.below(28) as usize;
        let mut sets = String::new();
        for i in 0..n {
            let k = match rng.below(6) {
                0 => format!("{}", rng.range(0, 40)),
                1 => format!("k{}", rng.range(0, 40)),
                2 => rng.ascii_string(3).replace(['\\', '"', '\''], "_"),
                3 => rng.ascii_string(9).replace(['\\', '"', '\''], "_"),
                4 => rng.unicode_string(3),
                _ => ["length", "toString", "a", "", " ", "-1", "0.5", "1e3"]
                    [rng.below(8) as usize]
                    .to_string(),
            };
            sets.push_str(&format!("o[{}] = {};", jsq(&k), i));
        }
        lines.push(format!(
            "var o = {{}}; {sets}\n\
             dump('o', o); dump('k', Object.keys(o)); \
             dump('n', Object.getOwnPropertyNames(o));\n\
             dump('s', JSON.stringify(o)); dump('i', JSON.stringify(o, null, 2));\n\
             dump('f', JSON.stringify(o, function(k,v){{return v}}));\n\
             var q = []; for (var kk in o) q.push(kk); dump('e', q);\n\
             dump('r', JSON.stringify(JSON.parse(JSON.stringify(o))));"
        ));
    }
    diff_lines_n("", &lines, 30);
}

/// Bigger arrays: Ap_sort_heapsort/leaf/sift over 2..200 elements on both the
/// flat (`u.a.simple`) fast path and the generic has/set/del path
/// (CONFIGS 440-443).
#[test]
fn t_array_sort_large() {
    warm_dates();
    let mut rng = Rng::new(0x504F_1111);
    let mut lines = vec![];
    for n in [2usize, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 64, 100, 199] {
        for variant in 0..6 {
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(match rng.below(7) {
                    0 => format!("{}", rng.range(-1000, 1000)),
                    1 => jsq(&rng.ascii_string(4)),
                    2 => "undefined".to_string(),
                    3 => "null".to_string(),
                    4 => jsnum(rng.f64_sane()),
                    5 => format!("{}", rng.range(0, 10)),
                    _ => jsq(&rng.unicode_string(2)),
                });
            }
            let lit = format!("[{}]", items.join(","));
            let shape = match variant {
                0 => "".to_string(),
                1 => format!(" delete a[{}];", n / 2),
                2 => format!(" a.length = {};", n + 5),
                3 => format!(" a[{}] = 'x';", n + 3),
                4 => " a.tag = 't';".to_string(),
                _ => format!(" delete a[0]; delete a[{}];", n - 1),
            };
            lines.push(format!(
                "var a = {lit};{shape} dump(a.sort()); dump(a, a.length);"
            ));
            lines.push(format!(
                "var a = {lit};{shape} \
                 dump(a.sort(function(x,y){{return x<y?-1:x>y?1:0}})); dump(a, a.length);"
            ));
            lines.push(format!(
                "var a = {lit};{shape} dump(a.sort(function(x,y){{return y-x}})); dump(a, a.length);"
            ));
            lines.push(format!(
                "var a = {lit};{shape} dump(a.reverse()); dump(a.join('|')); dump(a.length);"
            ));
        }
    }
    diff_lines_n("", &lines, 30);
}

/// Regexps with many capture groups feeding exec/split/match/replace
/// (CONFIGS 425, 431, 432, 306).
///
/// UNDEFINED BEHAVIOUR, deliberately not tested: `Sp_replace_regexp`'s
/// *function* replacement walks the capture array with
/// `for (x = 0; m.sub[x].sp; ++x)` (c_src/src/jsstring.c:573), relying on a
/// NULL terminator that only exists because `regexec` clears all
/// `REG_MAXSUB` == 16 slots.  A regexp with the maximum 15 capture groups whose
/// groups *all* participate fills every one of `Resub.sub[0..15]`, so the loop
/// reads `m.sub[16]`, one element past the end of the stack-local `Resub m`,
/// pushes whatever garbage pointer it finds and segfaults.  Both libmujs.so
/// builds crash on it, so the function-replacement variant is capped at 13
/// groups here; 14 groups would also be safe (`sub[14]` is still in bounds)
/// but leaves no margin.
#[test]
fn t_regexp_many_captures() {
    warm_dates();
    let mut lines = vec![];
    for ngroups in 1..=15 {
        let pat: String = (0..ngroups)
            .map(|i| format!("({})", (b'a' + i as u8) as char))
            .collect();
        let subj: String = (0..ngroups).map(|i| (b'a' + i as u8) as char).collect();
        let long = format!("x{subj}y{subj}z");
        for fl in ["", "g", "i", "gi", "m", "gm"] {
            for s in [subj.as_str(), long.as_str(), "", "abc"] {
                lines.push(format!(
                    "var r = new RegExp({}, '{}'); var s = {};\n\
                     dump('x', r.exec(s), r.lastIndex);\n\
                     r.lastIndex = 0; dump('p', s.split(r), s.split(r, 4));\n\
                     r.lastIndex = 0; dump('m', s.match(r));\n\
                     r.lastIndex = 0; dump('r1', s.replace(r, '$1|$2|$9|$10|$11|$15|$16|$99|$&|$$'));",
                    jsq(&pat),
                    fl,
                    jsq(s)
                ));
                if ngroups <= 13 {
                    lines.push(format!(
                        "var r = new RegExp({}, '{}'); var s = {};\n\
                         dump('r2', s.replace(r, function(){{\
                            var q = []; for (var i = 0; i < arguments.length; ++i) \
                            q.push(String(arguments[i])); return q.join('~') }}));",
                        jsq(&pat),
                        fl,
                        jsq(s)
                    ));
                }
            }
        }
        // optional groups that do not participate (CONFIGS 306)
        let optpat: String = (0..ngroups)
            .map(|i| format!("({})?", (b'a' + i as u8) as char))
            .collect();
        for fl in ["", "g"] {
            lines.push(format!(
                "var r = new RegExp({}, '{}'); var s = 'a';\n\
                 dump('o', r.exec(s), r.lastIndex);\n\
                 r.lastIndex = 0; dump('or', s.replace(r, '<$1|$2|$3>'));\n\
                 r.lastIndex = 0; dump('os', s.split(r));",
                jsq(&optpat),
                fl
            ));
        }
    }
    diff_lines_n("", &lines, 20);
}

/// Callbacks that mutate the array while forEach/map/filter/every/some/reduce
/// are walking it.
#[test]
fn t_array_reentrant_callbacks() {
    warm_dates();
    let muts = [
        "a.push(99)",
        "a.pop()",
        "a.shift()",
        "a.unshift(0)",
        "a.length = 1",
        "a.length = 8",
        "delete a[0]",
        "delete a[2]",
        "a[5] = 'q'",
        "a.reverse()",
        "a.sort()",
        "a.foo = 1",
        "0",
    ];
    let walkers = [
        "a.forEach(function(x,i){ M(); })",
        "a.map(function(x,i){ M(); return x })",
        "a.filter(function(x,i){ M(); return true })",
        "a.every(function(x,i){ M(); return true })",
        "a.some(function(x,i){ M(); return false })",
        "a.reduce(function(p,c){ M(); return String(p)+String(c) }, '')",
        "a.reduceRight(function(p,c){ M(); return String(p)+String(c) }, '')",
    ];
    let mut lines = vec![];
    for shape in ["[1,2,3,4]", "[1,2,3]", "(function(){var a=[1,2,3,4];delete a[1];return a})()", "(function(){var a=[1];a.length=4;return a})()"] {
        for m in muts {
            for w in walkers {
                lines.push(format!(
                    "var a = {shape}; var n = 0; \
                     function M(){{ if (n++ < 2) {{ {m}; }} }} \
                     dump('w', {w}); dump('a', a, a.length, n);"
                ));
            }
        }
    }
    // reduce / reduceRight with no initial value over holes
    for extra in [
        "dump([].reduce(function(){}, 'i'))",
        "try{ dump([].reduce(function(){})) }catch(e){ dump('CAUGHT', e) }",
        "try{ dump([].reduceRight(function(){})) }catch(e){ dump('CAUGHT', e) }",
        "var a=[]; a.length=3; try{ dump(a.reduce(function(){})) }catch(e){ dump('CAUGHT', e) }",
        "var a=[]; a.length=3; try{ dump(a.reduceRight(function(){})) }catch(e){ dump('CAUGHT', e) }",
        "var a=[1]; a.length=3; dump(a.reduce(function(p,c){return String(p)+'/'+String(c)}))",
        "var a=[]; a[2]=1; dump(a.reduce(function(p,c){return String(p)+'/'+String(c)}))",
        "var a=[]; a[2]=1; dump(a.reduceRight(function(p,c){return String(p)+'/'+String(c)}))",
        "dump([1,2,3].reduce(function(p,c,i,z){return p+'|'+c+':'+i+':'+(z.length)}, 'S'))",
        "dump([1,2,3].reduceRight(function(p,c,i,z){return p+'|'+c+':'+i+':'+(z.length)}, 'S'))",
    ] {
        lines.push(extra.to_string());
    }
    diff_lines_n("", &lines, 40);
}
