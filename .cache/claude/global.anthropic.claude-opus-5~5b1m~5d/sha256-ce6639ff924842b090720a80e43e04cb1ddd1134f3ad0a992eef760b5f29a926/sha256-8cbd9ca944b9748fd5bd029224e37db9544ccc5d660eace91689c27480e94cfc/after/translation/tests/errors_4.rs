//! ERRORS.md rows 191-243 — error-path differential tests.
//!
//! Rows 191-222 are the `die()` rejections in `regexp.c` (each makes
//! `js_regcomp`/`js_regcompx` return NULL and sets `*errorp`).  Rows 223-226
//! are the remaining lexer/parser/JSON macro rejections.  Rows 227-243 are the
//! generic surfaces: protected-call failures, allocation failure in
//! `js_newstate`, `js_regexec` return codes, the array/string/env/value-stack
//! limits, the run/memory limits, out-of-range enum values crossing the FFI
//! boundary and NULL/boundary pointer inputs.
//!
//! Every test runs the identical closure against the C `libmujs.so` and the
//! Rust `libmujs.so` in separate forked children and compares captured
//! stdout+stderr bytes and the exit status.

#![allow(non_snake_case, dead_code)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI64, AtomicPtr, Ordering};

/* ------------------------------------------------------------------------- */
/* printing (always flushed so that library writes to stderr interleave
 * deterministically with our own writes to the shared capture fd)           */
/* ------------------------------------------------------------------------- */

unsafe fn fl() {
    libc::fflush(null_mut());
}

unsafe fn pl(s: &str) {
    p_line(s);
    fl();
}

unsafe fn pi(l: &str, v: c_int) {
    p_int(l, v);
    fl();
}

unsafe fn ps(l: &str, s: *const c_char) {
    p_str(l, s);
    fl();
}

unsafe fn pn(l: &str, v: f64) {
    p_num(l, v);
    fl();
}

unsafe fn pp(l: &str, p: *const c_void) {
    p_ptr_nonnull(l, p);
    fl();
}

unsafe fn header(row: u32, what: &str) {
    libc::printf(cs("--- row %d: %s ---\n").as_ptr(), row as c_int, cs(what).as_ptr());
    fl();
}

/// Disable core dumps: several of these tests abort() on purpose and dumping
/// core through systemd-coredump would make the suite take minutes.
unsafe fn nocore() {
    let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    libc::setrlimit(libc::RLIMIT_CORE, &rl);
}

/* ------------------------------------------------------------------------- */
/* "current api" so that C callbacks can call back into the right library     */
/* ------------------------------------------------------------------------- */

static CUR: AtomicPtr<Api> = AtomicPtr::new(null_mut());

fn set_cur(api: &Api) {
    CUR.store(api as *const Api as *mut Api, Ordering::SeqCst);
}

unsafe fn cur() -> &'static Api {
    &*(CUR.load(Ordering::SeqCst) as *const Api)
}

unsafe extern "C" fn cf_print(J: JS) {
    let api = cur();
    let top = (api.js_gettop)(J);
    let mut i = 1;
    while i < top {
        if i > 1 {
            libc::putchar(b' ' as c_int);
        }
        let s = (api.js_tostring)(J, i);
        libc::printf(cs("%s").as_ptr(), s);
        i += 1;
    }
    libc::putchar(b'\n' as c_int);
    fl();
    (api.js_pushundefined)(J);
}

unsafe fn mkstate(api: &Api, flags: c_int) -> JS {
    set_cur(api);
    let J = newstate(api, flags);
    (api.js_newcfunction)(J, Some(cf_print), cs("print").as_ptr(), 1);
    (api.js_setglobal)(J, cs("print").as_ptr());
    J
}

/* ------------------------------------------------------------------------- */
/* countdown allocator: succeeds `ALLOC_LEFT` times, then returns NULL        */
/* ------------------------------------------------------------------------- */

static ALLOC_LEFT: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn count_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        libc::free(p);
        return null_mut();
    }
    let left = ALLOC_LEFT.fetch_sub(1, Ordering::SeqCst);
    if left <= 0 {
        return null_mut();
    }
    libc::realloc(p, n as usize)
}

/* ------------------------------------------------------------------------- */
/* regexp helpers                                                            */
/* ------------------------------------------------------------------------- */

fn pat(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn rep(s: &str, n: usize) -> CString {
    CString::new(s.repeat(n)).unwrap()
}

/// `js_newregexp` through a protected call (it throws a SyntaxError).
unsafe extern "C" fn cf_mkre(J: JS) {
    let api = cur();
    let p = (api.js_tostring)(J, 1);
    let f = (api.js_tointeger)(J, 2);
    (api.js_newregexp)(J, p, f);
}

unsafe fn re_newregexp(api: &Api, J: JS, p: &CString, flags: c_int) {
    (api.js_newcfunction)(J, Some(cf_mkre), cs("mkre").as_ptr(), 2);
    (api.js_pushundefined)(J);
    (api.js_pushstring)(J, p.as_ptr());
    (api.js_pushnumber)(J, flags as f64);
    let rc = (api.js_pcall)(J, 2);
    pi("js_newregexp.rc", rc);
    if rc != 0 {
        ps("js_newregexp.err", (api.js_tostring)(J, -1));
    }
    (api.js_pop)(J, 1);
}

/// `new RegExp(P)` from JavaScript; the pattern is handed over in a global so
/// that no JS string escaping is needed.
unsafe fn re_new_regexp_js(api: &Api, J: JS, p: &CString) {
    (api.js_pushstring)(J, p.as_ptr());
    (api.js_setglobal)(J, cs("P").as_ptr());
    let rc = (api.js_dostring)(J, cs("new RegExp(P);").as_ptr());
    pi("new_RegExp.rc", rc);
}

/// One regexp.c `die()` row: direct `js_regcomp`, `js_regcomp` with a NULL
/// errorp, `js_newregexp` and `new RegExp(...)`.
unsafe fn re_row(api: &Api, row: u32, what: &str, p: &CString, cflags: c_int) {
    header(row, what);
    let mut err: *const c_char = null();
    let prog = (api.js_regcomp)(p.as_ptr(), cflags, &mut err);
    pp("regcomp.prog", prog);
    ps("regcomp.err", err);
    if !prog.is_null() {
        (api.js_regfree)(prog);
    }
    /* the same call with errorp == NULL must not crash */
    let prog2 = (api.js_regcomp)(p.as_ptr(), cflags, null_mut());
    pp("regcomp.nullerrp.prog", prog2);
    if !prog2.is_null() {
        (api.js_regfree)(prog2);
    }
    let J = mkstate(api, 0);
    re_newregexp(api, J, p, 0);
    re_new_regexp_js(api, J, p);
    (api.js_freestate)(J);
}

/// A `die()` row that needs a failing allocator: `js_regcompx` with a budget
/// of `budget` successful allocations.
unsafe fn re_alloc_row(api: &Api, row: u32, what: &str, p: &CString, budget: i64) {
    header(row, what);
    ALLOC_LEFT.store(budget, Ordering::SeqCst);
    let mut err: *const c_char = null();
    let prog = (api.js_regcompx)(Some(count_alloc), null_mut(), p.as_ptr(), 0, &mut err);
    pi("alloc_budget", budget as c_int);
    pp("regcompx.prog", prog);
    ps("regcompx.err", err);
    if !prog.is_null() {
        (api.js_regfreex)(Some(count_alloc), null_mut(), prog);
    }
}

/* ------------------------------------------------------------------------- */
/* source helpers                                                            */
/* ------------------------------------------------------------------------- */

/// Run `src` through `js_ploadstring` (prints the error object) and through
/// `js_dostring` (prints the return code; the message goes to the report
/// callback, i.e. stderr).
unsafe fn src_row(api: &Api, J: JS, src: &CString) {
    libc::printf(cs("src=<%s>\n").as_ptr(), src.as_ptr());
    fl();
    let rc = (api.js_ploadstring)(J, cs("[string]").as_ptr(), src.as_ptr());
    pi("ploadstring.rc", rc);
    if rc != 0 {
        ps("ploadstring.err", (api.js_tostring)(J, -1));
    }
    (api.js_pop)(J, 1);
    let rc2 = (api.js_dostring)(J, src.as_ptr());
    pi("dostring.rc", rc2);
}

unsafe fn json_row(api: &Api, J: JS, text: &str) {
    let t = cs(text);
    libc::printf(cs("json=<%s>\n").as_ptr(), t.as_ptr());
    fl();
    (api.js_pushstring)(J, t.as_ptr());
    (api.js_setglobal)(J, cs("S").as_ptr());
    let rc = (api.js_dostring)(J, cs("print(JSON.stringify(JSON.parse(S)));").as_ptr());
    pi("dostring.rc", rc);
}

/* ========================================================================= */
/* rows 191-222: regexp.c die()                                              */
/* ========================================================================= */

unsafe fn body_err191_hex_invalid_escape_sequence(api: &Api) {
    nocore();
    re_row(api, 191, "hex(): invalid escape sequence", &pat(r"a\xzz"), 0);
    re_row(api, 191, "hex(): invalid escape sequence (\\u)", &pat(r"a\uzzzz"), 0);
    re_row(api, 191, "hex(): invalid escape sequence (\\x1g)", &pat(r"\x1g"), 0);
}

#[test]
fn err191_hex_invalid_escape_sequence() {
    diff("err191", |api| unsafe { body_err191_hex_invalid_escape_sequence(api) });
}

unsafe fn body_err192_dec_invalid_quantifier(api: &Api) {
    nocore();
    re_row(api, 192, "dec(): invalid quantifier", &pat("a{z}"), 0);
    re_row(api, 192, "dec(): invalid quantifier (second digit)", &pat("a{1z}"), 0);
    re_row(api, 192, "dec(): invalid quantifier (max)", &pat("a{1,z}"), 0);
}

#[test]
fn err192_dec_invalid_quantifier() {
    diff("err192", |api| unsafe { body_err192_dec_invalid_quantifier(api) });
}

unsafe fn body_err193_unterminated_escape_trailing_backslash(api: &Api) {
    nocore();
    /* regexp.c:128 — '\' as the very last character */
    re_row(api, 193, "nextrune:128 unterminated escape sequence", &pat("a\\"), 0);
}

#[test]
fn err193_unterminated_escape_trailing_backslash() {
    diff("err193", |api| unsafe { body_err193_unterminated_escape_trailing_backslash(api) });
}

unsafe fn body_err194_unterminated_escape_control(api: &Api) {
    nocore();
    /* regexp.c:138 — "\c" with nothing after it */
    re_row(api, 194, "nextrune:138 unterminated escape sequence", &pat(r"a\c"), 0);
}

#[test]
fn err194_unterminated_escape_control() {
    diff("err194", |api| unsafe { body_err194_unterminated_escape_control(api) });
}

unsafe fn body_err195_unterminated_escape_hex(api: &Api) {
    nocore();
    /* regexp.c:143 — "\x" with fewer than two characters left */
    re_row(api, 195, "nextrune:143 unterminated escape sequence", &pat(r"a\x"), 0);
    re_row(api, 195, "nextrune:143 unterminated escape sequence (1 digit)", &pat(r"a\x1"), 0);
}

#[test]
fn err195_unterminated_escape_hex() {
    diff("err195", |api| unsafe { body_err195_unterminated_escape_hex(api) });
}

unsafe fn body_err196_unterminated_escape_unicode(api: &Api) {
    nocore();
    /* regexp.c:153 — "\u" with fewer than four characters left */
    re_row(api, 196, "nextrune:153 unterminated escape sequence", &pat(r"a\u"), 0);
    re_row(api, 196, "nextrune:153 unterminated escape sequence (3 digits)", &pat(r"a\u123"), 0);
}

#[test]
fn err196_unterminated_escape_unicode() {
    diff("err196", |api| unsafe { body_err196_unterminated_escape_unicode(api) });
}

unsafe fn body_err197_invalid_escape_character(api: &Api) {
    nocore();
    /* regexp.c:170 — identity escape of a letter */
    re_row(api, 197, "nextrune:170 invalid escape character", &pat(r"a\q"), 0);
    re_row(api, 197, "nextrune:170 invalid escape character (_)", &pat(r"\_"), 0);
}

#[test]
fn err197_invalid_escape_character() {
    diff("err197", |api| unsafe { body_err197_invalid_escape_character(api) });
}

unsafe fn body_err198_lexcount_numeric_overflow_min(api: &Api) {
    nocore();
    /* regexp.c:186 — yymin >= REPINF (255) */
    re_row(api, 198, "lexcount:186 numeric overflow", &pat("a{1000}"), 0);
}

#[test]
fn err198_lexcount_numeric_overflow_min() {
    diff("err198", |api| unsafe { body_err198_lexcount_numeric_overflow_min(api) });
}

unsafe fn body_err199_lexcount_numeric_overflow_max(api: &Api) {
    nocore();
    /* regexp.c:200 — yymax >= REPINF (255) */
    re_row(api, 199, "lexcount:200 numeric overflow", &pat("a{1,1000}"), 0);
}

#[test]
fn err199_lexcount_numeric_overflow_max() {
    diff("err199", |api| unsafe { body_err199_lexcount_numeric_overflow_max(api) });
}

unsafe fn body_err200_too_many_character_classes(api: &Api) {
    nocore();
    /* regexp.c:213 — more than REG_MAXCLASS (128) classes */
    re_row(api, 200, "newcclass:213 too many character classes", &rep(r"\d", 129), 0);
    /* 128 classes still compiles */
    re_row(api, 200, "newcclass:213 boundary: exactly 128 classes", &rep(r"\d", 128), 0);
}

#[test]
fn err200_too_many_character_classes() {
    diff("err200", |api| unsafe { body_err200_too_many_character_classes(api) });
}

unsafe fn body_err201_invalid_character_class_range(api: &Api) {
    nocore();
    /* regexp.c:224 — a > b */
    re_row(api, 201, "addrange:224 invalid character class range", &pat("[z-a]"), 0);
    re_row(api, 201, "addrange:224 invalid character class range (9-0)", &pat("[9-0]"), 0);
}

#[test]
fn err201_invalid_character_class_range() {
    diff("err201", |api| unsafe { body_err201_invalid_character_class_range(api) });
}

unsafe fn body_err202_too_many_character_class_ranges(api: &Api) {
    nocore();
    /* regexp.c:253 — REG_MAXSPAN is 64 runes == 32 spans; use non-adjacent
     * single characters (stride 2) so that no span merging happens and the
     * 32nd addrange() overflows.  The characters start above 0x7F so that
     * none of them is one of "DSWdsw": `\x53` would be lexed as `\S` and
     * addranges_S() would add ranges that swallow everything else. */
    let mkclass = |k: u32| {
        let mut s = String::from("[");
        for i in 0..k {
            s.push_str(&format!("\\x{:02X}", 0x81 + 2 * i));
        }
        s.push(']');
        pat(&s)
    };
    re_row(api, 202, "addrange:253 too many character class ranges", &mkclass(40), 0);
    re_row(api, 202, "addrange:253 boundary: 31 single characters", &mkclass(31), 0);
    re_row(api, 202, "addrange:253 boundary: 32 single characters", &mkclass(32), 0);
    /* the same limit reached with explicit ranges */
    let mut r = String::from("[");
    for i in 0..40u32 {
        r.push_str(&format!("\\x{:02X}-\\x{:02X}", 0x81 + 3 * i, 0x82 + 3 * i));
    }
    r.push(']');
    re_row(api, 202, "addrange:253 too many explicit ranges", &pat(&r), 0);
}

#[test]
fn err202_too_many_character_class_ranges() {
    diff("err202", |api| unsafe { body_err202_too_many_character_class_ranges(api) });
}

unsafe fn body_err203_unterminated_character_class(api: &Api) {
    nocore();
    /* regexp.c:322 */
    re_row(api, 203, "lexclass:322 unterminated character class", &pat("[a"), 0);
    re_row(api, 203, "lexclass:322 unterminated character class (empty)", &pat("["), 0);
}

#[test]
fn err203_unterminated_character_class() {
    diff("err203", |api| unsafe { body_err203_unterminated_character_class(api) });
}

unsafe fn body_err204_infinite_loop_empty_string(api: &Api) {
    nocore();
    /* regexp.c:493 */
    re_row(api, 204, "newrep:493 infinite loop matching the empty string", &pat("(a*)*"), 0);
    re_row(api, 204, "newrep:493 infinite loop (empty group)", &pat("()*"), 0);
    re_row(api, 204, "newrep:493 infinite loop ((a?)+)", &pat("(a?)+"), 0);
}

#[test]
fn err204_infinite_loop_empty_string() {
    diff("err204", |api| unsafe { body_err204_infinite_loop_empty_string(api) });
}

unsafe fn body_err205_invalid_back_reference(api: &Api) {
    nocore();
    /* regexp.c:541 */
    re_row(api, 205, "parseatom:541 invalid back-reference", &pat(r"\1"), 0);
    re_row(api, 205, "parseatom:541 invalid back-reference (\\9)", &pat(r"(a)\9"), 0);
    re_row(api, 205, "parseatom:541 invalid back-reference (forward)", &pat(r"\1(a)"), 0);
}

#[test]
fn err205_invalid_back_reference() {
    diff("err205", |api| unsafe { body_err205_invalid_back_reference(api) });
}

unsafe fn body_err206_too_many_captures(api: &Api) {
    nocore();
    /* regexp.c:552 — REG_MAXSUB is 16 and nsub starts at 1, so the 16th
     * capturing group is one too many. */
    re_row(api, 206, "parseatom:552 too many captures", &rep("(a)", 16), 0);
    re_row(api, 206, "parseatom:552 boundary: 15 captures", &rep("(a)", 15), 0);
}

#[test]
fn err206_too_many_captures() {
    diff("err206", |api| unsafe { body_err206_too_many_captures(api) });
}

unsafe fn body_err207_unmatched_paren_capture(api: &Api) {
    nocore();
    /* regexp.c:557 */
    re_row(api, 207, "parseatom:557 unmatched '('", &pat("(a"), 0);
}

#[test]
fn err207_unmatched_paren_capture() {
    diff("err207", |api| unsafe { body_err207_unmatched_paren_capture(api) });
}

unsafe fn body_err208_unmatched_paren_noncapture(api: &Api) {
    nocore();
    /* regexp.c:563 */
    re_row(api, 208, "parseatom:563 unmatched '(' (?:)", &pat("(?:a"), 0);
}

#[test]
fn err208_unmatched_paren_noncapture() {
    diff("err208", |api| unsafe { body_err208_unmatched_paren_noncapture(api) });
}

unsafe fn body_err209_unmatched_paren_lookahead(api: &Api) {
    nocore();
    /* regexp.c:570 */
    re_row(api, 209, "parseatom:570 unmatched '(' (?=)", &pat("(?=a"), 0);
}

#[test]
fn err209_unmatched_paren_lookahead() {
    diff("err209", |api| unsafe { body_err209_unmatched_paren_lookahead(api) });
}

unsafe fn body_err210_unmatched_paren_neg_lookahead(api: &Api) {
    nocore();
    /* regexp.c:577 */
    re_row(api, 210, "parseatom:577 unmatched '(' (?!)", &pat("(?!a"), 0);
}

#[test]
fn err210_unmatched_paren_neg_lookahead() {
    diff("err210", |api| unsafe { body_err210_unmatched_paren_neg_lookahead(api) });
}

unsafe fn body_err211_parseatom_syntax_error(api: &Api) {
    nocore();
    /* regexp.c:580 — a quantifier with nothing to repeat */
    re_row(api, 211, "parseatom:580 syntax error", &pat("*"), 0);
    re_row(api, 211, "parseatom:580 syntax error (+)", &pat("+a"), 0);
    re_row(api, 211, "parseatom:580 syntax error ({1})", &pat("{1}"), 0);
}

#[test]
fn err211_parseatom_syntax_error() {
    diff("err211", |api| unsafe { body_err211_parseatom_syntax_error(api) });
}

unsafe fn body_err212_parserep_invalid_quantifier(api: &Api) {
    nocore();
    /* regexp.c:598 — max < min */
    re_row(api, 212, "parserep:598 invalid quantifier", &pat("a{2,1}"), 0);
    re_row(api, 212, "parserep:598 invalid quantifier (5,0)", &pat("a{5,0}"), 0);
}

#[test]
fn err212_parserep_invalid_quantifier() {
    diff("err212", |api| unsafe { body_err212_parserep_invalid_quantifier(api) });
}

unsafe fn body_err213_count_stack_overflow(api: &Api) {
    nocore();
    /* regexp.c:661 — count() recurses once per P_CAT node and the CAT
     * chain built by parsecat() nests in ->y, so 5000 literal characters
     * give a 4999 deep tree (> REG_MAXREC == 4096).  Nested groups cannot
     * be used here: capturing groups hit "too many captures" at 16 and
     * "(?:" groups produce no node at all, while 4097 "(?=" groups would
     * exceed strlen(pattern)*2 > REG_MAXPROG first. */
    re_row(api, 213, "count:661 stack overflow", &rep("a", 5000), 0);
    /* 4000 characters still compiles */
    re_row(api, 213, "count:661 boundary: 4000 characters", &rep("a", 4000), 0);
}

#[test]
fn err213_count_stack_overflow() {
    diff("err213", |api| unsafe { body_err213_count_stack_overflow(api) });
}

unsafe fn body_err214_count_program_too_large(api: &Api) {
    nocore();
    /* regexp.c:672 — 254*254 == 64516 > REG_MAXPROG (32768) */
    re_row(api, 214, "count:672 program too large", &pat("(?:a{254}){254}"), 0);
}

#[test]
fn err214_count_program_too_large() {
    diff("err214", |api| unsafe { body_err214_count_program_too_large(api) });
}

unsafe fn body_err215_cannot_allocate_regular_expression(api: &Api) {
    nocore();
    /* regexp.c:916 — the very first allocation (Reprog) fails */
    re_alloc_row(api, 215, "regcompx:916 cannot allocate regular expression", &pat("abc"), 0);
}

#[test]
fn err215_cannot_allocate_regular_expression() {
    diff("err215", |api| unsafe { body_err215_cannot_allocate_regular_expression(api) });
}

unsafe fn body_err216_regcompx_program_too_large(api: &Api) {
    nocore();
    /* regexp.c:922 — strlen(pattern)*2 > REG_MAXPROG */
    re_row(api, 216, "regcompx:922 program too large", &rep("a", 20000), 0);
    /* 16384 characters is exactly at the limit (2*16384 == 32768) */
    re_row(api, 216, "regcompx:922 boundary: 16385 characters", &rep("a", 16385), 0);
}

#[test]
fn err216_regcompx_program_too_large() {
    diff("err216", |api| unsafe { body_err216_regcompx_program_too_large(api) });
}

unsafe fn body_err217_cannot_allocate_parse_list(api: &Api) {
    nocore();
    /* regexp.c:926 — the second allocation (Renode array) fails */
    re_alloc_row(
        api,
        217,
        "regcompx:926 cannot allocate regular expression parse list",
        &pat("abc"),
        1,
    );
}

#[test]
fn err217_cannot_allocate_parse_list() {
    diff("err217", |api| unsafe { body_err217_cannot_allocate_parse_list(api) });
}

unsafe fn body_err218_unmatched_close_paren(api: &Api) {
    nocore();
    /* regexp.c:940 */
    re_row(api, 218, "regcompx:940 unmatched ')'", &pat("a)"), 0);
    re_row(api, 218, "regcompx:940 unmatched ')' (alone)", &pat(")"), 0);
    re_row(api, 218, "regcompx:940 unmatched ')' (extra)", &pat("(a))"), 0);
}

#[test]
fn err218_unmatched_close_paren() {
    diff("err218", |api| unsafe { body_err218_unmatched_close_paren(api) });
}

unsafe fn body_err219_regcompx_syntax_error_unreachable(api: &Api) {
    nocore();
    /* regexp.c:942 `if (g.lookahead != EOF) die("syntax error")` is
     * UNREACHABLE: parsecat() only stops on EOF, '|' or ')' and parsealt()
     * consumes every '|', so after parsealt() the lookahead is either EOF
     * or ')' — and ')' is taken by the "unmatched ')'" check one line
     * earlier.  The closest reachable behaviour is printed instead. */
    header(219, "regcompx:942 syntax error (UNREACHABLE — proxy below)");
    re_row(api, 219, "proxy: ')' takes regcompx:940 instead", &pat("a|b)"), 0);
    re_row(api, 219, "proxy: parseatom:580 syntax error", &pat("a|*"), 0);
}

#[test]
fn err219_regcompx_syntax_error_unreachable() {
    diff("err219", |api| unsafe { body_err219_regcompx_syntax_error_unreachable(api) });
}

unsafe fn body_err220_regcompx_program_too_large_count(api: &Api) {
    nocore();
    /* regexp.c:951 — count() returns 254*129 == 32766 which passes the
     * check inside count(), but 6 + 32766 == 32772 > REG_MAXPROG. */
    re_row(api, 220, "regcompx:951 program too large", &pat("(?:a{254}){129}"), 0);
    /* one repetition less compiles fine */
    re_row(api, 220, "regcompx:951 boundary: (?:a{254}){128}", &pat("(?:a{254}){128}"), 0);
}

#[test]
fn err220_regcompx_program_too_large_count() {
    diff("err220", |api| unsafe { body_err220_regcompx_program_too_large_count(api) });
}

unsafe fn body_err221_cannot_allocate_instruction_list(api: &Api) {
    nocore();
    /* regexp.c:956 — third allocation (Reinst array) fails */
    re_alloc_row(
        api,
        221,
        "regcompx:956 cannot allocate regular expression instruction list",
        &pat("abc"),
        2,
    );
}

#[test]
fn err221_cannot_allocate_instruction_list() {
    diff("err221", |api| unsafe { body_err221_cannot_allocate_instruction_list(api) });
}

unsafe fn body_err222_cannot_allocate_cclass_list(api: &Api) {
    nocore();
    /* regexp.c:961 — fourth allocation (Reclass array) fails; the pattern
     * must contain a character class for this site to be reached. */
    re_alloc_row(
        api,
        222,
        "regcompx:961 cannot allocate regular expression character class list",
        &pat("[a]b"),
        3,
    );
}

#[test]
fn err222_cannot_allocate_cclass_list() {
    diff("err222", |api| unsafe { body_err222_cannot_allocate_cclass_list(api) });
}

unsafe fn body_err215_222_allocator_sweep(api: &Api) {
    nocore();
    header(215, "allocator sweep 0..12 over js_regcompx (rows 215/217/221/222)");
    let nulltag = cs("<null>");
    let p = pat("[a]b");
    for n in 0..13i64 {
        ALLOC_LEFT.store(n, Ordering::SeqCst);
        let mut err: *const c_char = null();
        let prog =
            (api.js_regcompx)(Some(count_alloc), null_mut(), p.as_ptr(), 0, &mut err);
        libc::printf(
            cs("budget=%d prog=%s err=%s\n").as_ptr(),
            n as c_int,
            cs(if prog.is_null() { "null" } else { "nonnull" }).as_ptr(),
            if err.is_null() { nulltag.as_ptr() } else { err },
        );
        fl();
        if !prog.is_null() {
            (api.js_regfreex)(Some(count_alloc), null_mut(), prog);
        }
    }
    /* the same sweep for a pattern without character classes */
    header(215, "allocator sweep 0..6 over js_regcompx (no character class)");
    let p2 = pat("abc");
    for n in 0..7i64 {
        ALLOC_LEFT.store(n, Ordering::SeqCst);
        let mut err: *const c_char = null();
        let prog =
            (api.js_regcompx)(Some(count_alloc), null_mut(), p2.as_ptr(), 0, &mut err);
        libc::printf(
            cs("budget=%d prog=%s err=%s\n").as_ptr(),
            n as c_int,
            cs(if prog.is_null() { "null" } else { "nonnull" }).as_ptr(),
            if err.is_null() { nulltag.as_ptr() } else { err },
        );
        fl();
        if !prog.is_null() {
            (api.js_regfreex)(Some(count_alloc), null_mut(), prog);
        }
    }
}

#[test]
fn err215_222_allocator_sweep() {
    diff("err215_222_sweep", |api| unsafe { body_err215_222_allocator_sweep(api) });
}

/* ========================================================================= */
/* rows 223-226: lexer / parser / JSON macros                                */
/* ========================================================================= */

unsafe fn body_err223_jsY_expect(api: &Api) {
    nocore();
    header(223, "jsY_expect (jslex.c:177): expected '%c'");
    let J = mkstate(api, 0);
    /* the only reachable jsY_expect sites are in the JSON lexer */
    json_row(api, J, "fals");
    json_row(api, J, "fa");
    json_row(api, J, "nul");
    json_row(api, J, "tru");
    json_row(api, J, "n");
    (api.js_freestate)(J);
}

#[test]
fn err223_jsY_expect() {
    diff("err223", |api| unsafe { body_err223_jsY_expect(api) });
}

unsafe fn body_err224_jsP_expect(api: &Api) {
    nocore();
    header(224, "jsP_expect (jsparse.c:143): unexpected token: %s (expected %s)");
    let J = mkstate(api, 0);
    for s in [
        "if (1",
        "if 1) ;",
        "for (;;",
        "var x = {a:1",
        "function f( {}",
        "while (1 ;",
        "switch (1 {}",
        "a[1",
        "try { } catch e { }",
    ] {
        src_row(api, J, &pat(s));
    }
    (api.js_freestate)(J);
}

#[test]
fn err224_jsP_expect() {
    diff("err224", |api| unsafe { body_err224_jsP_expect(api) });
}

unsafe fn body_err225_increc_too_much_recursion(api: &Api) {
    nocore();
    header(225, "INCREC (jsparse.c:24): too much recursion (JS_ASTLIMIT 400)");
    let J = mkstate(api, 0);
    /* 500 nested parenthesised expressions */
    let mut s = String::new();
    for _ in 0..500 {
        s.push('(');
    }
    s.push('1');
    for _ in 0..500 {
        s.push(')');
    }
    s.push(';');
    let deep = pat(&s);
    let rc = (api.js_ploadstring)(J, cs("[string]").as_ptr(), deep.as_ptr());
    pi("deep500.ploadstring.rc", rc);
    if rc != 0 {
        ps("deep500.err", (api.js_tostring)(J, -1));
    }
    (api.js_pop)(J, 1);
    let rc2 = (api.js_dostring)(J, deep.as_ptr());
    pi("deep500.dostring.rc", rc2);

    /* 399 levels is still accepted */
    let mut t = String::new();
    for _ in 0..300 {
        t.push('(');
    }
    t.push('1');
    for _ in 0..300 {
        t.push(')');
    }
    t.push(';');
    let ok = pat(&t);
    let rc3 = (api.js_ploadstring)(J, cs("[string]").as_ptr(), ok.as_ptr());
    pi("deep300.ploadstring.rc", rc3);
    (api.js_pop)(J, 1);
    (api.js_freestate)(J);
}

#[test]
fn err225_increc_too_much_recursion() {
    diff("err225", |api| unsafe { body_err225_increc_too_much_recursion(api) });
}

unsafe fn body_err226_json_unexpected_token(api: &Api) {
    nocore();
    header(226, "jsonexpect (json.c:41): JSON: unexpected token: %s (expected %s)");
    let J = mkstate(api, 0);
    for s in [
        "[1,]",
        "[1 2]",
        "{\"a\" 1}",
        "{1:2}",
        "{\"a\":1,}",
        "",
        ",",
        "[",
        "{",
        "\"abc",
        "{\"a\":}",
        "01",
    ] {
        json_row(api, J, s);
    }
    (api.js_freestate)(J);
}

#[test]
fn err226_json_unexpected_token() {
    diff("err226", |api| unsafe { body_err226_json_unexpected_token(api) });
}

/* ========================================================================= */
/* rows 227-232: protected calls, try stack, js_newstate                     */
/* ========================================================================= */

/// Calls `js_ploadstring` (which starts with `js_ptry`) from JavaScript so
/// that the try stack can already be full when it runs.
unsafe extern "C" fn cf_pload(J: JS) {
    let api = cur();
    let rc = (api.js_ploadstring)(J, cs("[nested]").as_ptr(), cs("1;").as_ptr());
    pi("  pload.rc", rc);
    if rc != 0 {
        ps("  pload.err", (api.js_tostring)(J, -1));
    }
    (api.js_pop)(J, 1);
    (api.js_pushundefined)(J);
}

unsafe fn body_err227_try_stack_overflow(api: &Api) {
    nocore();
    header(227, "js_ptry / js_savetrypc: exception stack overflow (JS_TRYLIMIT 64)");
    let J = mkstate(api, 0);
    (api.js_newcfunction)(J, Some(cf_pload), cs("pload").as_ptr(), 0);
    (api.js_setglobal)(J, cs("pload").as_ptr());
    /* Each recursion level enters one `try`, so trytop grows by one per
     * level; once JS_TRYLIMIT is reached OP_TRY raises "exception stack
     * overflow" and js_ploadstring's js_ptry starts returning 1.
     * NB: calling js_savetry() 70 times directly is not viable — the
     * longjmp() in js_trystackoverflow() would jump into a jmp_buf that
     * was never initialised by setjmp(). */
    let rc = (api.js_dostring)(
        J,
        cs(concat!(
            "function f(n) {\n",
            "  if (n <= 0) { pload(); return; }\n",
            "  try { f(n-1); } catch (e) { print('caught: ' + e); }\n",
            "}\n",
            "for (var k = 55; k <= 72; ++k) { print('k=' + k); f(k); }\n"
        ))
        .as_ptr(),
    );
    pi("dostring.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err227_try_stack_overflow() {
    diff("err227", |api| unsafe { body_err227_try_stack_overflow(api) });
}

unsafe fn body_err228_ploadstring_syntax_error(api: &Api) {
    nocore();
    header(228, "js_ploadstring (jsstate.c:36): returns 1 with the error object on the stack");
    let J = mkstate(api, 0);
    for s in ["1 +", "function", "var", "}", "/* unterminated", "'unterminated", "1;"] {
        libc::printf(cs("src=<%s>\n").as_ptr(), cs(s).as_ptr());
        fl();
        let before = (api.js_gettop)(J);
        let rc = (api.js_ploadstring)(J, cs("[string]").as_ptr(), cs(s).as_ptr());
        pi("rc", rc);
        pi("pushed", (api.js_gettop)(J) - before);
        if rc != 0 {
            ps("err", (api.js_tostring)(J, -1));
            pi("iserror", (api.js_iserror)(J, -1));
        } else {
            ps("typeof", (api.js_typeof)(J, -1));
        }
        (api.js_pop)(J, 1);
    }
    (api.js_freestate)(J);
}

#[test]
fn err228_ploadstring_syntax_error() {
    diff("err228", |api| unsafe { body_err228_ploadstring_syntax_error(api) });
}

unsafe fn body_err229_pcall_failure(api: &Api) {
    nocore();
    header(229, "js_pcall (jsrun.c:1414): returns 1 with the error object on the stack");
    let J = mkstate(api, 0);

    /* callee is not callable */
    (api.js_pushnumber)(J, 42.0);
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    pi("notcallable.rc", rc);
    ps("notcallable.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    /* callee throws */
    let rc0 = (api.js_dostring)(J, cs("function boom() { throw new Error('boom'); }").as_ptr());
    pi("define.rc", rc0);
    (api.js_getglobal)(J, cs("boom").as_ptr());
    (api.js_pushundefined)(J);
    let rc2 = (api.js_pcall)(J, 0);
    pi("throws.rc", rc2);
    ps("throws.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    /* negative argument count */
    (api.js_getglobal)(J, cs("boom").as_ptr());
    (api.js_pushundefined)(J);
    (api.js_pushnumber)(J, 1.0);
    let rc3 = (api.js_pcall)(J, -1);
    pi("negative_n.rc", rc3);
    ps("negative_n.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    /* success for contrast */
    let rc4 = (api.js_dostring)(J, cs("function ok() { return 7; }").as_ptr());
    pi("define2.rc", rc4);
    (api.js_getglobal)(J, cs("ok").as_ptr());
    (api.js_pushundefined)(J);
    let rc5 = (api.js_pcall)(J, 0);
    pi("ok.rc", rc5);
    ps("ok.result", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    pi("gettop", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

#[test]
fn err229_pcall_failure() {
    diff("err229", |api| unsafe { body_err229_pcall_failure(api) });
}

unsafe fn body_err230_pconstruct_failure(api: &Api) {
    nocore();
    header(230, "js_pconstruct (jsrun.c:1400): returns 1 with the error object on the stack");
    let J = mkstate(api, 0);

    /* js_pconstruct's savetop is TOP-n-2 while js_construct reads the
     * callee at -n-1, so push one spare slot below the callee. */
    (api.js_pushundefined)(J);
    (api.js_pushnumber)(J, 42.0);
    let rc = (api.js_pconstruct)(J, 0);
    pi("notcallable.rc", rc);
    ps("notcallable.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    let rc0 = (api.js_dostring)(J, cs("function Boom() { throw new Error('ctor'); }").as_ptr());
    pi("define.rc", rc0);
    (api.js_pushundefined)(J);
    (api.js_getglobal)(J, cs("Boom").as_ptr());
    let rc2 = (api.js_pconstruct)(J, 0);
    pi("throws.rc", rc2);
    ps("throws.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    (api.js_pushundefined)(J);
    (api.js_getglobal)(J, cs("Object").as_ptr());
    let rc3 = (api.js_pconstruct)(J, 0);
    pi("object.rc", rc3);
    ps("object.result", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    pi("gettop", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

#[test]
fn err230_pconstruct_failure() {
    diff("err230", |api| unsafe { body_err230_pconstruct_failure(api) });
}

unsafe fn body_err231_dostring_failure(api: &Api) {
    nocore();
    header(231, "js_dostring (jsstate.c:139): returns 1, message through the report callback");
    let J = mkstate(api, 0);
    for s in [
        "1 +",
        "throw new Error('thrown');",
        "throw 'a string';",
        "undefinedVariable.x;",
        "null.x;",
        "(void 0)();",
        "1;",
    ] {
        libc::printf(cs("src=<%s>\n").as_ptr(), cs(s).as_ptr());
        fl();
        let rc = (api.js_dostring)(J, cs(s).as_ptr());
        pi("rc", rc);
        pi("gettop", (api.js_gettop)(J));
    }
    (api.js_freestate)(J);
}

#[test]
fn err231_dostring_failure() {
    diff("err231", |api| unsafe { body_err231_dostring_failure(api) });
}

unsafe fn body_err232_newstate_allocation_failure(api: &Api) {
    nocore();
    header(232, "js_newstate (jsstate.c:187): allocator returns NULL");
    /* budget 0: the js_State allocation itself fails */
    ALLOC_LEFT.store(0, Ordering::SeqCst);
    let j0 = (api.js_newstate)(Some(count_alloc), null_mut(), 0);
    pp("budget0.state", j0);
    /* budget 1: the js_State succeeds, the value stack fails */
    ALLOC_LEFT.store(1, Ordering::SeqCst);
    let j1 = (api.js_newstate)(Some(count_alloc), null_mut(), 0);
    pp("budget1.state", j1);
    /* larger budgets fail somewhere inside jsB_init and take the
     * js_try/js_freestate path */
    for n in [2i64, 3, 4, 5, 6, 8, 12, 16, 24, 32] {
        ALLOC_LEFT.store(n, Ordering::SeqCst);
        let j = (api.js_newstate)(Some(count_alloc), null_mut(), 0);
        libc::printf(
            cs("budget=%d state=%s\n").as_ptr(),
            n as c_int,
            cs(if j.is_null() { "null" } else { "nonnull" }).as_ptr(),
        );
        fl();
        if !j.is_null() {
            ALLOC_LEFT.store(i64::MAX, Ordering::SeqCst);
            (api.js_freestate)(j);
        }
    }
    /* unlimited budget must produce a working state */
    ALLOC_LEFT.store(i64::MAX, Ordering::SeqCst);
    let jok = (api.js_newstate)(Some(count_alloc), null_mut(), 0);
    pp("unlimited.state", jok);
    if !jok.is_null() {
        let rc = (api.js_dostring)(jok, cs("1+1;").as_ptr());
        pi("unlimited.dostring.rc", rc);
        (api.js_freestate)(jok);
    }
}

#[test]
fn err232_newstate_allocation_failure() {
    diff("err232", |api| unsafe { body_err232_newstate_allocation_failure(api) });
}

/* ========================================================================= */
/* rows 233-235: js_regcomp / js_regexec return codes                        */
/* ========================================================================= */

unsafe fn body_err233_regcomp_null_with_errorp(api: &Api) {
    nocore();
    header(233, "js_regcomp: returns NULL and sets *errorp for every die()");
    let nulltag = cs("<null>");
    for p in [
        r"a\xzz", "a{z}", "a\\", r"a\c", r"a\x1", r"a\u12", r"a\q", "a{1000}", "a{1,1000}",
        "[z-a]", "[a", "(a*)*", r"\1", "(a", "(?:a", "(?=a", "(?!a", "*", "a{2,1}", "a)",
        "(?:a{254}){254}", "(?:a{254}){129}",
    ] {
        let cp = cs(p);
        let mut err: *const c_char = null();
        let prog = (api.js_regcomp)(cp.as_ptr(), 0, &mut err);
        libc::printf(
            cs("pattern=<%s> prog=%s err=%s\n").as_ptr(),
            cp.as_ptr(),
            cs(if prog.is_null() { "null" } else { "nonnull" }).as_ptr(),
            if err.is_null() { nulltag.as_ptr() } else { err },
        );
        fl();
        if !prog.is_null() {
            (api.js_regfree)(prog);
        }
    }
    /* on success *errorp is cleared */
    let good = cs("a(b)c");
    let stale = cs("stale");
    let mut err: *const c_char = stale.as_ptr();
    let prog = (api.js_regcomp)(good.as_ptr(), 0, &mut err);
    pp("good.prog", prog);
    ps("good.err", err);
    if !prog.is_null() {
        (api.js_regfree)(prog);
    }
}

#[test]
fn err233_regcomp_null_with_errorp() {
    diff("err233", |api| unsafe { body_err233_regcomp_null_with_errorp(api) });
}

unsafe fn body_err234_regexec_nomatch(api: &Api) {
    nocore();
    header(234, "js_regexec: REG_NOMATCH (1) when the subject does not match");
    let p = cs("a(b+)c");
    let mut err: *const c_char = null();
    let prog = (api.js_regcomp)(p.as_ptr(), 0, &mut err);
    pp("prog", prog);
    assert!(!prog.is_null());
    for s in ["xyz", "", "ac", "abbc", "zzabbczz"] {
        let cs_ = cs(s);
        let mut m = Resub::new();
        let r = (api.js_regexec)(prog, cs_.as_ptr(), &mut m, 0);
        libc::printf(cs("subject=<%s> rc=%d nsub=%d\n").as_ptr(), cs_.as_ptr(), r, m.nsub);
        fl();
        if r == 0 {
            let mut i = 0;
            while i < m.nsub {
                let e = m.sub[i as usize];
                let so = if e.sp.is_null() { -1 } else { e.sp.offset_from(cs_.as_ptr()) as c_int };
                let eo = if e.ep.is_null() { -1 } else { e.ep.offset_from(cs_.as_ptr()) as c_int };
                libc::printf(cs("  sub[%d] s=%d e=%d\n").as_ptr(), i, so, eo);
                fl();
                i += 1;
            }
        }
        /* sub == NULL uses the internal scratch Resub */
        let r2 = (api.js_regexec)(prog, cs_.as_ptr(), null_mut(), 0);
        pi("  nullsub.rc", r2);
    }
    (api.js_regfree)(prog);
}

#[test]
fn err234_regexec_nomatch() {
    diff("err234", |api| unsafe { body_err234_regexec_nomatch(api) });
}

unsafe fn body_err235_regexec_recursion_limit(api: &Api) {
    nocore();
    header(235, "js_regexec: -1 when the match recursion limit (REG_MAXREC 4096) is hit");
    /* `a*` recurses once per consumed character, so a 5000 character
     * subject exceeds REG_MAXREC in linear time.  (A catastrophic
     * backtracking pattern like `(a+)+b` would need exponential time to
     * get anywhere near the depth limit.) */
    let p = cs("a*");
    let mut err: *const c_char = null();
    let prog = (api.js_regcomp)(p.as_ptr(), 0, &mut err);
    pp("prog", prog);
    assert!(!prog.is_null());
    for n in [10usize, 100, 4000, 4090, 4095, 4100, 5000] {
        let subj = CString::new("a".repeat(n)).unwrap();
        let mut m = Resub::new();
        let r = (api.js_regexec)(prog, subj.as_ptr(), &mut m, 0);
        libc::printf(cs("len=%d rc=%d\n").as_ptr(), n as c_int, r);
        fl();
    }
    (api.js_regfree)(prog);

    /* the same limit reached through the JS layer -> "regexec failed" */
    let J = mkstate(api, 0);
    let rc = (api.js_dostring)(
        J,
        cs("var s = ''; for (var i = 0; i < 5000; ++i) s += 'a'; print(/a*/.exec(s));").as_ptr(),
    );
    pi("js.dostring.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err235_regexec_recursion_limit() {
    diff("err235", |api| unsafe { body_err235_regexec_recursion_limit(api) });
}

/* ========================================================================= */
/* rows 236-241: array / string / stack / run / memory limits                */
/* ========================================================================= */

unsafe fn body_err236_array_length_limit(api: &Api) {
    nocore();
    header(236, "array length: invalid array length / array too large (JS_ARRAYLIMIT 1<<26)");
    let J = mkstate(api, 0);
    for s in [
        "new Array(-1);",
        "new Array(1.5);",
        "new Array(1e10);",
        "new Array(4294967296);",
        "var a = []; a.length = -1;",
        "var a = []; a.length = 1e10;",
        "var a = []; a.length = 0.5;",
        "var a = []; a.length = 67108865;",
        "var a = []; a.length = 67108864;",
        "var a = []; a[-1] = 1; print(a.length);",
    ] {
        libc::printf(cs("src=<%s>\n").as_ptr(), cs(s).as_ptr());
        fl();
        let rc = (api.js_dostring)(J, cs(s).as_ptr());
        pi("rc", rc);
    }
    (api.js_freestate)(J);
}

#[test]
fn err236_array_length_limit() {
    diff("err236", |api| unsafe { body_err236_array_length_limit(api) });
}

/// `js_pushlstring` rejects a length above JS_STRLIMIT before touching the
/// buffer, which is the cheap way to observe the string length limit.
unsafe extern "C" fn cf_pushbig(J: JS) {
    let api = cur();
    let buf: [c_char; 16] = [0; 16];
    (api.js_pushlstring)(J, buf.as_ptr(), (1 << 28) + 1);
}

unsafe extern "C" fn cf_pushbig2(J: JS) {
    let api = cur();
    let buf: [c_char; 16] = [0; 16];
    (api.js_pushlstring)(J, buf.as_ptr(), c_int::MAX);
}

unsafe fn body_err237_string_length_limit(api: &Api) {
    nocore();
    header(237, "string length limit: invalid string length (JS_STRLIMIT 1<<28)");
    /* Actually building a >256MB string (Sp_concat / Ap_join) needs about
     * a gigabyte of RAM and many seconds, so the limit is observed through
     * js_pushlstring's identical `n > JS_STRLIMIT` check instead. */
    let J = mkstate(api, 0);
    (api.js_newcfunction)(J, Some(cf_pushbig), cs("pushbig").as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    pi("pushlstring_1<<28+1.rc", rc);
    ps("pushlstring_1<<28+1.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    (api.js_newcfunction)(J, Some(cf_pushbig2), cs("pushbig2").as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc2 = (api.js_pcall)(J, 0);
    pi("pushlstring_INT_MAX.rc", rc2);
    ps("pushlstring_INT_MAX.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);

    /* Ap_join / Sp_concat with a moderate size still succeeds */
    let rc3 = (api.js_dostring)(J, cs("print(Array(1000).join('ab').length);").as_ptr());
    pi("join.rc", rc3);
    (api.js_freestate)(J);
}

#[test]
fn err237_string_length_limit() {
    diff("err237", |api| unsafe { body_err237_string_length_limit(api) });
}

unsafe fn body_err238_environment_stack_overflow(api: &Api) {
    nocore();
    header(238, "environment stack (JS_ENVLIMIT 1024) — shadowed by the trace check");
    /* jsR_savescope()'s `envtop + 1 >= JS_ENVLIMIT` check is unreachable
     * through plain recursion: js_call() runs jsR_pushtrace() first and its
     * `tracetop + 1 == JS_ENVLIMIT` check fires one call earlier, so the
     * observable message is "call stack overflow".  OP_WITH / OP_CATCH do
     * not touch envtop at all. */
    let J = mkstate(api, 0);
    let rc = (api.js_dostring)(
        J,
        cs("function f(n) { return n > 0 ? f(n-1) : 0; } print(f(2000));").as_ptr(),
    );
    pi("recursion.rc", rc);
    /* the same with a non-lightweight function (uses `arguments`) */
    let rc2 = (api.js_dostring)(
        J,
        cs("function g(n) { arguments; return n > 0 ? g(n-1) : 0; } print(g(2000));").as_ptr(),
    );
    pi("recursion_arguments.rc", rc2);
    /* deep `with` nesting does not consume the environment stack */
    let rc3 = (api.js_dostring)(
        J,
        cs("var o = {x:1}; var s = ''; for (var i = 0; i < 200; ++i) s += 'with(o){'; s += 'x;'; for (i = 0; i < 200; ++i) s += '}'; print(eval(s));").as_ptr(),
    );
    pi("with_nesting.rc", rc3);
    (api.js_freestate)(J);
}

#[test]
fn err238_environment_stack_overflow() {
    diff("err238", |api| unsafe { body_err238_environment_stack_overflow(api) });
}

unsafe extern "C" fn cf_pushmany(J: JS) {
    let api = cur();
    let mut i = 0;
    while i < 6000 {
        (api.js_pushnumber)(J, i as f64);
        i += 1;
    }
    pi("  unreachable_top", (api.js_gettop)(J));
}

unsafe fn body_err239_value_stack_overflow(api: &Api) {
    nocore();
    header(239, "value stack (JS_STACKSIZE 4096): stack overflow");
    let J = mkstate(api, 0);
    (api.js_newcfunction)(J, Some(cf_pushmany), cs("pushmany").as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    pi("pushmany.rc", rc);
    ps("pushmany.err", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    pi("gettop_after", (api.js_gettop)(J));
    /* and from JavaScript: a very long argument list / deep expression */
    let rc2 = (api.js_dostring)(
        J,
        cs("var s = 'var a = ['; for (var i = 0; i < 5000; ++i) s += '1,'; s += '1];'; eval(s); print(a.length);").as_ptr(),
    );
    pi("js_bigarray.rc", rc2);
    (api.js_freestate)(J);
}

#[test]
fn err239_value_stack_overflow() {
    diff("err239", |api| unsafe { body_err239_value_stack_overflow(api) });
}

unsafe fn body_err240_runlimit(api: &Api) {
    nocore();
    header(240, "js_setlimit runlimit (jsrun.c:1602): script ran too long");
    for lim in [1 as c_int, 2, 10, 1000, 100000] {
        let J = mkstate(api, 0);
        (api.js_setlimit)(J, lim, 0);
        libc::printf(cs("runlimit=%d\n").as_ptr(), lim);
        fl();
        let rc = (api.js_dostring)(J, cs("var i = 0; while (1) ++i;").as_ptr());
        pi("rc", rc);
        (api.js_freestate)(J);
    }
    /* a short script under a generous limit still finishes */
    let J = mkstate(api, 0);
    (api.js_setlimit)(J, 1000000, 0);
    let rc = (api.js_dostring)(J, cs("var s = 0; for (var i = 0; i < 10; ++i) s += i; print(s);").as_ptr());
    pi("ok.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err240_runlimit() {
    diff("err240", |api| unsafe { body_err240_runlimit(api) });
}

unsafe fn body_err241_memlimit(api: &Api) {
    nocore();
    header(241, "js_setlimit memlimit (jsrun.c:55-72): out of memory");
    for lim in [1 as c_int, 16, 1024, 65536, 1000000] {
        let J = mkstate(api, 0);
        (api.js_setlimit)(J, 0, lim);
        libc::printf(cs("memlimit=%d\n").as_ptr(), lim);
        fl();
        let rc = (api.js_dostring)(
            J,
            cs("var a = []; for (var i = 0; i < 100000; ++i) a[i] = 'xxxxxxxxxxxxxxxxxxxxxxxx' + i;").as_ptr(),
        );
        pi("rc", rc);
        (api.js_freestate)(J);
    }
}

#[test]
fn err241_memlimit() {
    diff("err241", |api| unsafe { body_err241_memlimit(api) });
}

/* ========================================================================= */
/* row 242: out-of-range enum values across the FFI boundary                  */
/* ========================================================================= */

unsafe extern "C" fn cf_mkre255(J: JS) {
    let api = cur();
    (api.js_newregexp)(J, cs("a").as_ptr(), 255);
}

unsafe extern "C" fn cf_mkre_neg1(J: JS) {
    let api = cur();
    (api.js_newregexp)(J, cs("a").as_ptr(), -1);
}

unsafe fn body_err242a_newstate_bogus_flags(api: &Api) {
    nocore();
    header(242, "js_newstate with out-of-range flag ints");
    for f in [0 as c_int, 1, 2, 0x7fffffff, -1, i32::MIN, 0x100] {
        let J = (api.js_newstate)(None, null_mut(), f);
        libc::printf(
            cs("flags=%d state=%s\n").as_ptr(),
            f,
            cs(if J.is_null() { "null" } else { "nonnull" }).as_ptr(),
        );
        fl();
        if !J.is_null() {
            set_cur(api);
            (api.js_newcfunction)(J, Some(cf_print), cs("print").as_ptr(), 1);
            (api.js_setglobal)(J, cs("print").as_ptr());
            /* strictness is decided by the JS_STRICT bit only */
            let rc = (api.js_dostring)(J, cs("undeclaredGlobal = 1; print(undeclaredGlobal);").as_ptr());
            pi("  implicit_global.rc", rc);
            (api.js_freestate)(J);
        }
    }
}

#[test]
fn err242a_newstate_bogus_flags() {
    diff("err242a", |api| unsafe { body_err242a_newstate_bogus_flags(api) });
}

unsafe fn body_err242b_newregexp_bogus_flags(api: &Api) {
    nocore();
    header(242, "js_newregexp with flags 255 / -1");
    let J = mkstate(api, 0);
    (api.js_newcfunction)(J, Some(cf_mkre255), cs("re255").as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    pi("flags255.rc", rc);
    pi("flags255.isregexp", (api.js_isregexp)(J, -1));
    ps("flags255.typeof", (api.js_typeof)(J, -1));
    /* keep a copy in a global: js_tostring converts the stack slot in place */
    (api.js_copy)(J, -1);
    (api.js_setglobal)(J, cs("R255").as_ptr());
    ps("flags255.tostring", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    let rc1 = (api.js_dostring)(J, cs("print(R255.source, R255.global, R255.ignoreCase, R255.multiline, R255.lastIndex);").as_ptr());
    pi("flags255.props.rc", rc1);
    let rc1b = (api.js_dostring)(J, cs("print(R255.exec('AAA'));").as_ptr());
    pi("flags255.exec.rc", rc1b);
    let rc1c = (api.js_dostring)(J, cs("print(String(R255), R255.toString());").as_ptr());
    pi("flags255.str.rc", rc1c);

    (api.js_newcfunction)(J, Some(cf_mkre_neg1), cs("reneg").as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc2 = (api.js_pcall)(J, 0);
    pi("flagsneg1.rc", rc2);
    pi("flagsneg1.isregexp", (api.js_isregexp)(J, -1));
    (api.js_copy)(J, -1);
    (api.js_setglobal)(J, cs("RNEG").as_ptr());
    ps("flagsneg1.tostring", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    let rc3 = (api.js_dostring)(J, cs("print(RNEG.source, RNEG.global, RNEG.ignoreCase, RNEG.multiline);").as_ptr());
    pi("flagsneg1.props.rc", rc3);
    (api.js_freestate)(J);
}

#[test]
fn err242b_newregexp_bogus_flags() {
    diff("err242b", |api| unsafe { body_err242b_newregexp_bogus_flags(api) });
}

unsafe fn body_err242c_regexec_bogus_eflags(api: &Api) {
    nocore();
    header(242, "js_regexec with out-of-range eflags");
    let p = cs("^A(b)");
    let mut err: *const c_char = null();
    let prog = (api.js_regcomp)(p.as_ptr(), 0, &mut err);
    pp("prog", prog);
    assert!(!prog.is_null());
    let subj = cs("Abc");
    for ef in [0 as c_int, 1, 2, 4, 255, -1, i32::MIN, 0x7fffffff] {
        let mut m = Resub::new();
        let r = (api.js_regexec)(prog, subj.as_ptr(), &mut m, ef);
        let so = if m.sub[0].sp.is_null() { -1 } else { m.sub[0].sp.offset_from(subj.as_ptr()) as c_int };
        let eo = if m.sub[0].ep.is_null() { -1 } else { m.sub[0].ep.offset_from(subj.as_ptr()) as c_int };
        libc::printf(cs("eflags=%d rc=%d nsub=%d s=%d e=%d\n").as_ptr(), ef, r, m.nsub, so, eo);
        fl();
    }
    (api.js_regfree)(prog);

    /* cflags out of range too */
    for cf in [0 as c_int, 1, 2, 255, -1, i32::MIN] {
        let mut e2: *const c_char = null();
        let pr = (api.js_regcomp)(p.as_ptr(), cf, &mut e2);
        let mut m = Resub::new();
        let r = if pr.is_null() { -99 } else { (api.js_regexec)(pr, subj.as_ptr(), &mut m, 0) };
        libc::printf(
            cs("cflags=%d prog=%s rc=%d\n").as_ptr(),
            cf,
            cs(if pr.is_null() { "null" } else { "nonnull" }).as_ptr(),
            r,
        );
        fl();
        if !pr.is_null() {
            (api.js_regfree)(pr);
        }
    }
}

#[test]
fn err242c_regexec_bogus_eflags() {
    diff("err242c", |api| unsafe { body_err242c_regexec_bogus_eflags(api) });
}

unsafe fn body_err242d_newobject_bogus_class(api: &Api) {
    nocore();
    header(242, "jsV_newobject with an out-of-range js_Class");
    let J = mkstate(api, 0);
    for c in [0 as c_int, 99, -1, 255, i32::MAX] {
        let obj = (api.jsV_newobject)(J, c, null_mut());
        libc::printf(cs("class=%d obj=%s\n").as_ptr(), c, cs(if obj.is_null() { "null" } else { "nonnull" }).as_ptr());
        fl();
        (api.js_pushobject)(J, obj);
        ps("  typeof", (api.js_typeof)(J, -1));
        pi("  type", (api.js_type)(J, -1));
        pi("  isobject", (api.js_isobject)(J, -1));
        pi("  iscallable", (api.js_iscallable)(J, -1));
        pi("  isarray", (api.js_isarray)(J, -1));
        pi("  isregexp", (api.js_isregexp)(J, -1));
        ps("  tostring", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
    }
    (api.js_gc)(J, 0);
    (api.js_freestate)(J);
}

#[test]
fn err242d_newobject_bogus_class() {
    diff("err242d", |api| unsafe { body_err242d_newobject_bogus_class(api) });
}

unsafe fn body_err242e_tokenstring_out_of_range(api: &Api) {
    nocore();
    header(242, "jsY_tokenstring with out-of-range tokens");
    for t in [
        0 as c_int, 1, 32, 127, 128, 200, 255, 256, 257, 300, 9999, -1, -256, i32::MIN,
        i32::MAX,
    ] {
        let s = (api.jsY_tokenstring)(t);
        libc::printf(cs("token=%d -> %s\n").as_ptr(), t, s);
        fl();
    }
}

#[test]
fn err242e_tokenstring_out_of_range() {
    diff("err242e", |api| unsafe { body_err242e_tokenstring_out_of_range(api) });
}

unsafe fn body_err242f_toprimitive_bogus_hint(api: &Api) {
    nocore();
    header(242, "js_toprimitive with out-of-range hints");
    let J = mkstate(api, 0);
    let rc = (api.js_dostring)(
        J,
        cs("var o = { valueOf: function () { return 42; }, toString: function () { return 'str'; } };").as_ptr(),
    );
    pi("define.rc", rc);
    for h in [0 as c_int, 1, 2, 3, 99, -1, i32::MIN, i32::MAX] {
        (api.js_getglobal)(J, cs("o").as_ptr());
        (api.js_toprimitive)(J, -1, h);
        libc::printf(cs("hint=%d -> %s (%s)\n").as_ptr(), h, (api.js_tostring)(J, -1), (api.js_typeof)(J, -1));
        fl();
        (api.js_pop)(J, 1);
    }
    /* plain object: neither valueOf nor toString yields a primitive from
     * valueOf, so both branches end in toString */
    for h in [0 as c_int, 2, 99] {
        (api.js_newobject)(J);
        (api.js_toprimitive)(J, -1, h);
        libc::printf(cs("plain hint=%d -> %s\n").as_ptr(), h, (api.js_tostring)(J, -1));
        fl();
        (api.js_pop)(J, 1);
    }
    /* jsV_toprimitive directly on a non-object value is a no-op */
    (api.js_pushnumber)(J, 7.5);
    let v = (api.js_tovalue)(J, -1);
    (api.jsV_toprimitive)(J, v, 99);
    ps("number hint=99", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    (api.js_freestate)(J);
}

#[test]
fn err242f_toprimitive_bogus_hint() {
    diff("err242f", |api| unsafe { body_err242f_toprimitive_bogus_hint(api) });
}

unsafe fn body_err242g_strtol_bogus_radix(api: &Api) {
    nocore();
    header(242, "js_strtol with out-of-range radix");
    /* NB: any base > 80 makes the `table[c] < base` loop condition true for
     * every byte (including the NUL, whose table entry is 80), so js_strtol
     * runs off the end of the string until it faults.  That case is isolated
     * in err242g2_strtol_runaway_radix. */
    for s in ["123abcZ", "0", "zz", "", "-5", "  7"] {
        let cs_ = cs(s);
        for base in [0 as c_int, 1, 2, 8, 10, 16, 36, 37, 80, -1, i32::MIN] {
            let mut ep: *mut c_char = null_mut();
            let v = (api.js_strtol)(cs_.as_ptr(), &mut ep, base);
            let off = if ep.is_null() { -1 } else { ep.offset_from(cs_.as_ptr()) as c_int };
            libc::printf(
                cs("strtol(<%s>, base=%d) = %.17g end=%d\n").as_ptr(),
                cs_.as_ptr(),
                base,
                v,
                off,
            );
            fl();
        }
    }
    /* the same through parseInt */
    let J = mkstate(api, 0);
    let rc = (api.js_dostring)(
        J,
        cs("print(parseInt('123abcZ', 99), parseInt('123', -1), parseInt('123', 37), parseInt('123', 1));").as_ptr(),
    );
    pi("parseInt.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err242g_strtol_bogus_radix() {
    diff("err242g", |api| unsafe { body_err242g_strtol_bogus_radix(api) });
}

unsafe fn body_err242g2_strtol_runaway_radix(api: &Api) {
    nocore();
    header(242, "js_strtol with radix > 80 walks off the end of the string");
    /* js_strtol's digit table stores 80 for every non-alphanumeric byte,
     * including the terminating NUL, so `table[c] < base` never becomes false
     * for base > 80: the C loop reads past the end of the string until it
     * faults.  An identical SIGSEGV (with identical output before it) is the
     * expected result -- there is no error path here at all. */
    let cs_ = cs("123abcZ");
    pl("calling js_strtol(<123abcZ>, base=99)");
    let mut ep: *mut c_char = null_mut();
    let v = (api.js_strtol)(cs_.as_ptr(), &mut ep, 99);
    /* not reached */
    pn("value", v);
}

#[test]
fn err242g2_strtol_runaway_radix() {
    diff("err242g2", |api| unsafe { body_err242g2_strtol_runaway_radix(api) });
}

unsafe fn body_err242h_stack_index_out_of_range(api: &Api) {
    nocore();
    header(242, "js_type / js_typeof / js_tostring on out-of-range stack indices");
    let J = mkstate(api, 0);
    (api.js_pushnumber)(J, 1.0);
    (api.js_pushstring)(J, cs("two").as_ptr());
    pi("gettop", (api.js_gettop)(J));
    for idx in [0 as c_int, 1, 2, 3, 999, -1, -2, -3, -999, i32::MIN, i32::MAX] {
        libc::printf(
            cs("idx=%d type=%d typeof=%s tostring=%s isdefined=%d\n").as_ptr(),
            idx,
            (api.js_type)(J, idx),
            (api.js_typeof)(J, idx),
            (api.js_tostring)(J, idx),
            (api.js_isdefined)(J, idx),
        );
        fl();
    }
    (api.js_pop)(J, 2);
    (api.js_freestate)(J);
}

#[test]
fn err242h_stack_index_out_of_range() {
    diff("err242h", |api| unsafe { body_err242h_stack_index_out_of_range(api) });
}

unsafe fn body_err242i_defproperty_and_iterator_bogus_ints(api: &Api) {
    nocore();
    header(242, "js_defproperty with atts 255 and js_pushiterator with own 99");
    let J = mkstate(api, 0);
    (api.js_newobject)(J);
    for (name, atts) in [("a", 0 as c_int), ("b", 255), ("c", -1), ("d", i32::MIN), ("e", 8)] {
        (api.js_pushnumber)(J, 1.0);
        (api.js_defproperty)(J, -2, cs(name).as_ptr(), atts);
        libc::printf(cs("defined %s atts=%d\n").as_ptr(), cs(name).as_ptr(), atts);
        fl();
    }
    /* read them back */
    for name in ["a", "b", "c", "d", "e"] {
        (api.js_getproperty)(J, -1, cs(name).as_ptr());
        libc::printf(cs("get %s = %s\n").as_ptr(), cs(name).as_ptr(), (api.js_tostring)(J, -1));
        fl();
        (api.js_pop)(J, 1);
    }
    /* enumerate with own = 0 / 1 / 99 / -1 */
    for own in [0 as c_int, 1, 99, -1, i32::MIN] {
        libc::printf(cs("iterator own=%d:\n").as_ptr(), own);
        fl();
        (api.js_pushiterator)(J, -1, own);
        loop {
            let k = (api.js_nextiterator)(J, -1);
            if k.is_null() {
                break;
            }
            libc::printf(cs("  key=%s\n").as_ptr(), k);
            fl();
        }
        (api.js_pop)(J, 1);
    }
    (api.js_setglobal)(J, cs("O").as_ptr());
    let rc = (api.js_dostring)(
        J,
        cs("var n = 0; for (var k in O) ++n; print('enumerable=' + n); O.b = 99; print('b=' + O.b); print(delete O.b);").as_ptr(),
    );
    pi("js.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err242i_defproperty_and_iterator_bogus_ints() {
    diff("err242i", |api| unsafe { body_err242i_defproperty_and_iterator_bogus_ints(api) });
}

/* ========================================================================= */
/* row 243: NULL / boundary pointers                                         */
/* ========================================================================= */

unsafe fn body_err243a_empty_and_boundary_strings(api: &Api) {
    nocore();
    header(243, "empty / boundary string inputs");
    let J = mkstate(api, 0);

    (api.js_pushstring)(J, cs("").as_ptr());
    ps("pushstring_empty", (api.js_tostring)(J, -1));
    pi("pushstring_empty.type", (api.js_type)(J, -1));
    (api.js_pop)(J, 1);

    /* js_pushlstring with n == 0 and with n larger than the string; the
     * buffer is 16 zero bytes so reading past the NUL is deterministic */
    let mut buf: [c_char; 16] = [0; 16];
    buf[0] = b'a' as c_char;
    buf[1] = b'b' as c_char;
    for n in [0 as c_int, 1, 2, 3, 10, 15] {
        (api.js_pushlstring)(J, buf.as_ptr(), n);
        libc::printf(
            cs("pushlstring n=%d -> <%s> len_type=%d\n").as_ptr(),
            n,
            (api.js_tostring)(J, -1),
            (api.js_type)(J, -1),
        );
        fl();
        (api.js_pop)(J, 1);
    }

    ps("intern_empty", (api.js_intern)(J, cs("").as_ptr()));
    ps("intern_empty2", (api.js_intern)(J, cs("").as_ptr()));

    /* js_pushliteral stores the pointer without copying, so the storage must
     * outlive the stack slot -- use a static, not a temporary CString. */
    static EMPTY: &[u8] = b"\0";
    (api.js_pushliteral)(J, EMPTY.as_ptr() as *const c_char);
    ps("pushliteral_empty", (api.js_tostring)(J, -1));
    pi("pushliteral_empty.type", (api.js_type)(J, -1));
    (api.js_pop)(J, 1);

    /* property name "" */
    (api.js_newobject)(J);
    (api.js_pushnumber)(J, 5.0);
    (api.js_setproperty)(J, -2, cs("").as_ptr());
    (api.js_getproperty)(J, -1, cs("").as_ptr());
    ps("getproperty_empty_name", (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    pi("hasproperty_empty_name", (api.js_hasproperty)(J, -1, cs("").as_ptr()));
    (api.js_pop)(J, 1);

    ps("tostring_999", (api.js_tostring)(J, 999));
    ps("tostring_neg999", (api.js_tostring)(J, -999));
    pi("gettop", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

#[test]
fn err243a_empty_and_boundary_strings() {
    diff("err243a", |api| unsafe { body_err243a_empty_and_boundary_strings(api) });
}

unsafe fn body_err243b_utf_and_number_boundaries(api: &Api) {
    nocore();
    header(243, "js_utflen / jsU_chartorune / js_strtod boundary inputs");
    pi("utflen_empty", (api.js_utflen)(cs("").as_ptr()));
    pi("utflen_ascii", (api.js_utflen)(cs("abc").as_ptr()));

    /* truncated / malformed UTF-8 sequences */
    let cases: [&[u8]; 9] = [
        b"\x00",
        b"\x80\x00",
        b"\xc3\x00",
        b"\xc3\xa9\x00",
        b"\xe2\x00",
        b"\xe2\x82\x00",
        b"\xe2\x82\xac\x00",
        b"\xf0\x9f\x00",
        b"\xff\x00",
    ];
    for (i, c) in cases.iter().enumerate() {
        let mut r: c_int = -12345;
        let n = (api.jsU_chartorune)(&mut r, c.as_ptr() as *const c_char);
        libc::printf(cs("chartorune case=%d n=%d rune=%d\n").as_ptr(), i as c_int, n, r);
        fl();
        let l = (api.js_utflen)(c.as_ptr() as *const c_char);
        libc::printf(cs("  utflen=%d\n").as_ptr(), l);
        fl();
    }

    for s in ["", " ", "abc", "0", "1e", "1e+", ".", "-", "0x", "inf", "nan"] {
        let cs_ = cs(s);
        let mut ep: *mut c_char = null_mut();
        let v = (api.js_strtod)(cs_.as_ptr(), &mut ep);
        let off = if ep.is_null() { -1 } else { ep.offset_from(cs_.as_ptr()) as c_int };
        libc::printf(cs("strtod(<%s>) = %.17g end=%d\n").as_ptr(), cs_.as_ptr(), v, off);
        fl();
        let mut ep2: *mut c_char = null_mut();
        let v2 = (api.js_stringtofloat)(cs_.as_ptr(), &mut ep2);
        let off2 = if ep2.is_null() { -1 } else { ep2.offset_from(cs_.as_ptr()) as c_int };
        libc::printf(cs("  stringtofloat = %.17g end=%d\n").as_ptr(), v2, off2);
        fl();
    }
}

#[test]
fn err243b_utf_and_number_boundaries() {
    diff("err243b", |api| unsafe { body_err243b_utf_and_number_boundaries(api) });
}

unsafe fn body_err243c_empty_regexp(api: &Api) {
    nocore();
    header(243, "js_regcomp(\"\") and js_regexec with empty subject / NULL sub");
    let empty = cs("");
    let mut err: *const c_char = null();
    let prog = (api.js_regcomp)(empty.as_ptr(), 0, &mut err);
    pp("empty.prog", prog);
    ps("empty.err", err);
    if !prog.is_null() {
        let r0 = (api.js_regexec)(prog, empty.as_ptr(), null_mut(), 0);
        pi("exec(<>, NULL sub).rc", r0);
        let mut m = Resub::new();
        let r1 = (api.js_regexec)(prog, empty.as_ptr(), &mut m, 0);
        pi("exec(<>, sub).rc", r1);
        pi("exec(<>, sub).nsub", m.nsub);
        let abc = cs("abc");
        let r2 = (api.js_regexec)(prog, abc.as_ptr(), null_mut(), 0);
        pi("exec(<abc>).rc", r2);
        (api.js_regfree)(prog);
    }
    /* js_regfree(NULL) must be a no-op */
    (api.js_regfree)(null_mut());
    pl("regfree(NULL) ok");
    (api.js_regfreex)(Some(count_alloc), null_mut(), null_mut());
    pl("regfreex(NULL) ok");

    /* empty pattern through the JS layer */
    let J = mkstate(api, 0);
    re_newregexp(api, J, &pat(""), 0);
    re_new_regexp_js(api, J, &pat(""));
    let rc = (api.js_dostring)(J, cs("print(new RegExp('').source, String(new RegExp('')));").as_ptr());
    pi("js.rc", rc);
    (api.js_freestate)(J);
}

#[test]
fn err243c_empty_regexp() {
    diff("err243c", |api| unsafe { body_err243c_empty_regexp(api) });
}

unsafe fn body_err243d_grisu2_zero_aborts(api: &Api) {
    nocore();
    header(243, "js_grisu2(0.0) trips assert(x.f >= y.f) in minus() and aborts");
    pl("about to call js_grisu2(0.0)");
    /* The C library is built with assertions enabled, so it prints an
     * assert message before abort()ing while the Rust translation aborts
     * silently — redirect stderr to /dev/null so that only the identical
     * SIGABRT exit status is compared. */
    let devnull = libc::open(cs("/dev/null").as_ptr(), libc::O_WRONLY);
    if devnull >= 0 {
        libc::dup2(devnull, 2);
        libc::close(devnull);
    }
    let mut buf: [c_char; 64] = [0; 64];
    let mut k: c_int = 0;
    let n = (api.js_grisu2)(0.0, buf.as_mut_ptr(), &mut k);
    /* not reached */
    libc::printf(cs("grisu2 returned n=%d K=%d\n").as_ptr(), n, k);
    fl();
}

#[test]
fn err243d_grisu2_zero_aborts() {
    diff("err243d", |api| unsafe { body_err243d_grisu2_zero_aborts(api) });
}

unsafe fn body_err243e_grisu2_normal_values(api: &Api) {
    nocore();
    header(243, "js_grisu2 on ordinary values (contrast with the 0.0 abort)");
    for v in [1.0f64, 0.5, 1e300, 1e-300, -1.25, 123456789.0] {
        let mut buf: [c_char; 64] = [0; 64];
        let mut k: c_int = 0;
        let n = (api.js_grisu2)(v, buf.as_mut_ptr(), &mut k);
        libc::printf(cs("v=%.17g n=%d K=%d digits=%.*s\n").as_ptr(), v, n, k, n, buf.as_ptr());
        fl();
    }
}

#[test]
fn err243e_grisu2_normal_values() {
    diff("err243e", |api| unsafe { body_err243e_grisu2_normal_values(api) });
}
