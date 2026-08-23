//! Differential tests for ERRORS.md rows 731-933:
//!   regexp.c 731-793, jsregexp.c 794-808, json.c 809-824, jsdate.c 825-879,
//!   jsfunction.c 880-889, jsmath.c 890-902, jsrepr.c 903-910, utf.c 911-933.
//!
//! Every call goes through the two `.so` exports via `tests/common/mod.rs`.
//! Anything that can throw is driven from inside a cfunction invoked through
//! `js_pcall`, or through `js_dostring` / `js_ploadstring` + `js_pcall`, so a
//! `js_throw` always finds a handler instead of reaching `abort()` with
//! `trytop == 0`.
//!
//! ==========================================================================
//! ROWS DELIBERATELY NOT DRIVEN, and why (each verified against the C source)
//! ==========================================================================
//!
//! * row 761 -- regexp.c:942 `die(&g, "syntax error")` when the lookahead after
//!   `parsealt` is neither EOF nor `)`.  UNREACHABLE.  `parsecat`
//!   (regexp.c:610/614) only loops while `lookahead != EOF && lookahead != '|'
//!   && lookahead != ')'`, so it always returns with the lookahead in
//!   `{EOF, '|', ')'}`; `parsealt` (regexp.c:630) then consumes every `'|'` in
//!   its `while (accept(g, '|'))` loop, so it can only return with the lookahead
//!   in `{EOF, ')'}`.  regexp.c:939 catches `)` (row 760) and EOF is the
//!   success case, so line 942 cannot be reached by any pattern.  The *other*
//!   `"syntax error"` site (regexp.c:580, row 753) IS reachable and is driven
//!   exhaustively by `t_regexp_die_sites`.
//!
//! * row 788 -- regexp.c:1221 `default:` in `match()`, an unknown opcode.
//!   UNREACHABLE without memory corruption.  Every instruction reaching
//!   `match()` is produced by `emit()` (regexp.c:680), which only ever stores
//!   one of the 17 `I_*` enumerators, and every `pc` transition is either
//!   `pc + 1`, `pc->x` or `pc->y`, all of which are back-patched to slots that
//!   are filled in by a later `emit()` before `regcompx` returns (the last two
//!   emits at regexp.c:975-976 are `I_RPAR` + `I_END`).  The instruction array
//!   is also never overrun: `count()` (regexp.c:657) returns a value that is
//!   >= the number of instructions `compile()` emits for every node type
//!   (equal for P_CAT/P_ALT/P_PAR/P_PLA/P_NLA and for P_REP with `min == max`
//!   or `max < REPINF` or `min == 0`, and strictly greater for P_REP with
//!   `min > 0 && max == REPINF`).  So no uninitialised `Reinst` can ever be
//!   executed.  `t_regexp_match_nomatch_sites` drives the 21 *reachable*
//!   `return 1` sites instead.
//!
//! * regexp.c:672 `n < 0` (the overflow half of row 756).  UNREACHABLE:
//!   `count(node->x)` has already been checked `<= REG_MAXPROG` (32768) on
//!   every recursive return, and `max <= REPINF` (255), so the products
//!   `count*min`, `count*max` and `count*(min+1)` are at most 32768*256 =
//!   8388608, far below `INT_MAX`.  The `n > REG_MAXPROG` half is driven by
//!   `t_regexp_rep_program_too_large`.
//!
//! * regexp.c:951 `n < 0` (the overflow half of row 762).  UNREACHABLE for the
//!   same reason: `count()` returns at most REG_MAXPROG.
//!
//! * `js_regcomp(pattern = NULL)`.  UNDEFINED BEHAVIOUR: regexp.c:920 does
//!   `strlen(pattern)` with no NULL check, so a NULL pattern is an immediate
//!   NULL dereference.  Not testable in-process.
//!
//! * `js_regexec(prog = NULL, ...)`.  UNDEFINED BEHAVIOUR: regexp.c:1235 does
//!   `sub->nsub = prog->nsub` with no NULL check.
//!
//! * `js_regexec(prog, sp = NULL, ...)`.  UNDEFINED BEHAVIOUR: `match()`
//!   dereferences `sp` unconditionally (regexp.c:1116 `if (!*sp)` is the first
//!   opcode reached from the standard `I_SPLIT` / `I_ANYNL` prologue).
//!   `sub = NULL` IS well defined (regexp.c:1232 substitutes a local
//!   `scratch`) and is exercised by `t_regexp_ffi_boundary_abuse`.
//!
//! * rows 888, 889 -- jsfunction.c:145 / jsfunction.c:169, `n < 0` after
//!   `js_getlength(J, args)` in `callbound` / `constructbound`.  UNREACHABLE:
//!   `args` is always the `__BoundArguments__` property, which `Fp_bind`
//!   (jsfunction.c:207-212) creates with `js_newarray` and fills with
//!   `js_setindex(J, -2, i - 2)` for `2 <= i < top`, so its `length` is exactly
//!   `top - 2`, a small non-negative int.  The property is defined
//!   `JS_READONLY | JS_DONTENUM | JS_DONTCONF`, so no script can replace it
//!   with an object whose `length` coerces negative, nor delete it.
//!   `t_function_bound_calls` drives `callbound` / `constructbound` over many
//!   bound-argument counts and asserts the `length` seen there is never
//!   negative, and `t_function_apply_negative_length` drives the *reachable*
//!   sibling clamp (jsfunction.c:110, row 884), where the "array" is an
//!   arbitrary user object.
//!
//! * row 909 -- jsrepr.c:262 `js_pushstring(J, sb ? sb->s : "undefined")`.
//!   DEAD CODE: jsrepr.c:261 unconditionally runs `js_putc(J, &sb, 0)` first,
//!   and `js_putc` (jsintern.c:5-11) allocates the buffer when `*sbp` is NULL,
//!   so `sb` is never NULL at line 262.  `t_repr_empty_buffer_is_dead` shows
//!   that the one value class whose `reprvalue` writes nothing before that
//!   point does not exist: `reprvalue` (jsrepr.c:151) covers undefined / null /
//!   boolean / number / string / object, which is every `js_Value` type, and
//!   every object `case` writes at least one byte.
//!
//! Set `MUJS_DUMP=1` to print every regexp transcript.

#![allow(unused_unsafe, clippy::too_many_arguments)]

mod common;
use common::*;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;

/* ===================================================================== *
 *  process-wide setup
 * ===================================================================== */

/// Anything that writes to stdout (`js_gc(J,1)`, `jsS_dumpstrings`, ...) must
/// hold this for its whole duration or the suite is flaky.  Nothing in this
/// file currently writes to stdout except the optional `MUJS_DUMP` tracing,
/// which goes through it as well.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn stdout_guard() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn trace(tag: &str, s: &str) {
    if std::env::var_os("MUJS_DUMP").is_some() {
        let _g = stdout_guard();
        println!("=== [{tag}] ===\n{s}");
    }
}

/// `LocalTZA()` (jsdate.c:28) caches its result in a function-static on the
/// first call and derives it from `gmtime()` / `localtime()`, which share one
/// static `struct tm` inside libc.  TZ is therefore pinned ONCE for the whole
/// process -- never per library -- and both caches are primed single-threaded
/// before any test can race into them.  A UTC host would make `LocalTime()` the
/// identity and leave the signed-offset arm of `fmttime` and
/// `getTimezoneOffset` untested, so a fixed POSIX zone 5h30 west of UTC is
/// pinned instead; that also makes every expectation independent of the host.
fn warm_dates() {
    static W: OnceLock<()> = OnceLock::new();
    W.get_or_init(|| {
        std::env::set_var("TZ", "XYZ5:30");
        let p = libs();
        let mut seen = vec![];
        for l in [&p.c, &p.rs] {
            unsafe {
                out_clear();
                let j = new_state(l, 0);
                let cs = cstr(
                    "print(new Date(0).getTimezoneOffset(), new Date(0).toString(), \
                     Date.parse('1970-01-01T00:00'))",
                );
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

/* ===================================================================== *
 *  regexp.c: compile-time rejections (rows 731-764)
 * ===================================================================== */

const CFLAG_SET: [c_int; 4] = [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE];

/// A poison value for `*errorp` so we can prove the callee actually wrote it.
const ERRP_POISON: *const c_char = 0x5AFE_0000_1234_usize as *const c_char;

#[derive(Debug, PartialEq, Eq, Clone)]
enum Comp {
    /// compiled; the value is what `*errorp` was left as (C sets it to NULL)
    Ok(bool),
    /// rejected; the exact `*errorp` string
    Err(String),
}

unsafe fn comp(l: &Lib, pat: &str, cflags: c_int) -> Comp {
    let cp = cstr(pat);
    let mut e: *const c_char = ERRP_POISON;
    let prog = l.js_regcomp(cp.as_ptr(), cflags, &mut e);
    if prog.is_null() {
        assert_ne!(
            e, ERRP_POISON,
            "{}: js_regcomp({pat:?}, {cflags}) returned NULL without writing *errorp",
            l.name
        );
        Comp::Err(from_c(e))
    } else {
        l.js_regfree(prog);
        // regexp.c:984 `if (errorp) *errorp = NULL;` on success
        Comp::Ok(e.is_null())
    }
}

/// `js_regcomp` with `errorp == NULL`: must not crash and must agree on
/// success/failure (regexp.c:904 and regexp.c:984 both guard on `errorp`).
unsafe fn comp_noerrp(l: &Lib, pat: &str, cflags: c_int) -> bool {
    let cp = cstr(pat);
    let prog = l.js_regcomp(cp.as_ptr(), cflags, std::ptr::null_mut());
    let ok = !prog.is_null();
    if ok {
        l.js_regfree(prog);
    }
    ok
}

/// Both libraries must reject `pat` with EXACTLY `msg`, under every cflags
/// combination, with `errorp` both non-NULL and NULL.
fn expect_err(pat: &str, msg: &str) {
    let p = libs();
    unsafe {
        for cflags in CFLAG_SET {
            let a = comp(&p.c, pat, cflags);
            let b = comp(&p.rs, pat, cflags);
            assert_eq!(a, b, "js_regcomp({pat:?}, {cflags}) divergence");
            assert_eq!(
                a,
                Comp::Err(msg.to_string()),
                "js_regcomp({pat:?}, {cflags}) wrong rejection"
            );
            let na = comp_noerrp(&p.c, pat, cflags);
            let nb = comp_noerrp(&p.rs, pat, cflags);
            assert_eq!(na, nb, "js_regcomp({pat:?}, {cflags}) NULL errorp divergence");
            assert!(!na, "js_regcomp({pat:?}, {cflags}) NULL errorp should fail");
        }
    }
}

/// Both libraries must ACCEPT `pat` (used to bracket every limit from below).
fn expect_ok(pat: &str) {
    let p = libs();
    unsafe {
        for cflags in CFLAG_SET {
            let a = comp(&p.c, pat, cflags);
            let b = comp(&p.rs, pat, cflags);
            assert_eq!(a, b, "js_regcomp({pat:?}, {cflags}) divergence");
            assert_eq!(
                a,
                Comp::Ok(true),
                "js_regcomp({pat:?}, {cflags}) should compile and NULL *errorp"
            );
            assert!(comp_noerrp(&p.c, pat, cflags));
            assert!(comp_noerrp(&p.rs, pat, cflags));
        }
    }
}

/// Compare only (no expectation about the outcome) -- for boundary sweeps.
fn diff_comp(pat: &str) -> Comp {
    let p = libs();
    unsafe {
        let mut out = None;
        for cflags in CFLAG_SET {
            let a = comp(&p.c, pat, cflags);
            let b = comp(&p.rs, pat, cflags);
            assert_eq!(
                a,
                b,
                "js_regcomp(len={}, {cflags}) divergence",
                pat.len()
            );
            if cflags == 0 {
                out = Some(a);
            }
        }
        out.unwrap()
    }
}

/// ERRORS rows 731, 732 and every `die()` site whose trigger is a small fixed
/// pattern: 733-739, 743, 745-747, 749-754, 760.
///
/// Every one of the FOUR `"unmatched '('"` sites (regexp.c:557 capturing,
/// :563 `(?:`, :570 `(?=`, :577 `(?!`) and all FOUR `"unterminated escape
/// sequence"` sites (:128 bare `\` at EOF, :138 `\c` at EOF, :143 `\x` short,
/// :153 `\u` short) get their own pattern, as do both `"invalid quantifier"`
/// sites (:108 `dec()`, :598 `max < min`).
#[test]
fn t_regexp_die_sites() {
    body_t_regexp_die_sites();
}

fn body_t_regexp_die_sites() {
    /* --- row 733: regexp.c:101 hex() -- reached from BOTH \x and \u --- */
    for pat in [
        r"\xZZ", r"\xg0", r"\x0g", r"\x/0", r"\x:0", r"\x@0", r"\x`0", r"\xG0",
        r"a\xzz", r"[\xzz]",
        r"\uZZZZ", r"\u000Z", r"\u00Z0", r"\u0Z00", r"\uZ000", r"[\uZZZZ]",
    ] {
        expect_err(pat, "invalid escape sequence");
    }

    /* --- row 734: regexp.c:108 dec() inside {M,N} --- */
    for pat in [
        "a{x}", "a{,}", "a{}", "a{1,x}", "a{ }", "a{1 }", "a{-1}", "a{+1}",
        "a{1,-1}", "a{1.5}", "a{1,2,3}", "a{", "a{1", "a{1,", "a{1,2",
    ] {
        expect_err(pat, "invalid quantifier");
    }

    /* --- row 735: regexp.c:128 -- pattern ends immediately after `\` --- */
    for pat in ["\\", "a\\", "abc\\", "[a]\\", "(a)\\"] {
        expect_err(pat, "unterminated escape sequence");
    }
    /* --- row 736: regexp.c:138 -- `\c` with no control letter --- */
    for pat in ["\\c", "a\\c", "[\\c", "(\\c"] {
        expect_err(pat, "unterminated escape sequence");
    }
    /* --- row 737: regexp.c:143 -- `\x` with fewer than 2 bytes left --- */
    for pat in ["\\x", "\\x4", "a\\x", "a\\x4", "[\\x4"] {
        expect_err(pat, "unterminated escape sequence");
    }
    /* --- row 738: regexp.c:153 -- `\u` with fewer than 4 bytes left --- */
    for pat in ["\\u", "\\u1", "\\u12", "\\u123", "a\\u12", "[\\u123"] {
        expect_err(pat, "unterminated escape sequence");
    }

    /* --- row 739: regexp.c:170 identity escape of a letter or `_` --- */
    for pat in [
        r"\y", r"\_", r"\A", r"\C", r"\E", r"\F", r"\G", r"\H", r"\I", r"\J",
        r"\K", r"\L", r"\M", r"\N", r"\O", r"\P", r"\Q", r"\R", r"\T", r"\U",
        r"\V", r"\X", r"\Y", r"\Z", r"\a", r"\e", r"\g", r"\h", r"\i", r"\j",
        r"\k", r"\l", r"\m", r"\o", r"\p", r"\q", r"\y", r"\z",
        // a non-ASCII unicode letter goes through isalpharune()
        "\\\u{e9}", "\\\u{391}", "\\\u{3b1}", "\\\u{5d0}", "\\\u{104}", "\\\u{1e9e}",
    ] {
        expect_err(pat, "invalid escape character");
    }
    // ...while a non-ASCII NON-letter falls off regexp.c:171 and is accepted.
    // U+4E2D is absent from `ucd_alpha2` / `ucd_alpha1`, so `isalpharune`
    // returns 0 (row 931) and `isunicodeletter` (regexp.c:114) is false.
    for pat in ["\\\u{4e2d}", "\\\u{a0}", "\\\u{2028}", "\\\u{1f600}"] {
        expect_ok(pat);
    }
    // A unicode LETTER whose low byte happens to be an ESCAPES character is
    // accepted before the isunicodeletter() test ever runs: regexp.c:167 calls
    // `strchr(ESCAPES, g->yychar)` and strchr converts its `int` argument to
    // `char`, so U+0131 (305) is matched as '1' (0x31) and U+0130 (304) as '0'.
    // Both are in ucd_alpha2's 0xF8..0x2C1 range, i.e. real letters.
    for pat in ["\\\u{131}", "\\\u{130}", "\\\u{142}", "\\\u{129}"] {
        expect_ok(pat);
    }
    // ...and the ESCAPES set is accepted (brackets the same site from below)
    for pat in [
        r"\B", r"\b", r"\D", r"\d", r"\S", r"\s", r"\W", r"\w", r"\^", r"\$",
        r"\\", r"\.", r"\*", r"\+", r"\?", r"\(", r"\)", r"\[", r"\]", r"\{",
        r"\}", r"\|", r"\-", r"\0", r"\f", r"\n", r"\r", r"\t", r"\v",
        // a non-letter identity escape is silently allowed (falls off :171)
        r"\/", r"\ ", r"\~", r"\#", r"\@",
    ] {
        expect_ok(pat);
    }

    /* --- row 743: regexp.c:224 addrange a > b --- */
    for pat in [
        "[z-a]", "[b-a]", "[9-0]", "[\\u0041-\\u0040]", "[a-\\x01]",
        "[\u{4e2d}-a]", "[\\u1234-\\u1233]", "[b-a][c-d]",
    ] {
        expect_err(pat, "invalid character class range");
    }
    for pat in ["[a-a]", "[a-b]", "[0-9]", "[\\u0040-\\u0041]"] {
        expect_ok(pat);
    }
    // `\d` / `\s` / `\w` / `\D` / `\S` / `\W` inside a class are handled by the
    // regexp.c:338 arm, which never forms a range with the neighbouring `-`:
    // both `[\d-x]` and `[x-\d]` add `-` as a literal instead of dying.
    for pat in ["[\\d-\\x01]", "[\\d-x]", "[x-\\d]", "[a-\\w]", "[\\s-\\S]"] {
        expect_ok(pat);
    }
    // The same `strchr` truncation quirk applies inside a class: regexp.c:338
    // tests `strchr("DSWdsw", g->yychar)`, so an escaped rune whose low byte is
    // 0x00 matches strchr's terminating NUL and one whose low byte is 'd'/'s'/
    // 'w'/'D'/'S'/'W' matches that letter.  Both take the class-shorthand arm
    // and never form a range, so these compile instead of dying.
    for pat in [
        "[\\u1000-\\u0999]", "[\\u0100-\\u00ff]", "[\\u1064-\\u1063]",
        "[\\u1073-\\u1072]", "[\\u1044-\\u1043]",
    ] {
        expect_ok(pat);
    }

    /* --- row 745: regexp.c:322 EOF inside [...] --- */
    for pat in [
        "[", "[a", "[abc", "[^", "[^a", "[a-", "[\\]", "[\\d", "[a-b", "a[b",
    ] {
        expect_err(pat, "unterminated character class");
    }
    // `[]` is NOT unterminated: regexp.c:323 breaks on the very first `]`, so
    // the class is simply EMPTY and everything after it is ordinary pattern.
    for pat in ["[]", "[]a", "[]a]", "[]]", "[^]", "[^]]", "[^]a"] {
        expect_ok(pat);
    }

    /* --- row 746: regexp.c:493 unbounded repeat of an empty atom --- */
    for pat in [
        "(?:)*", "(?:)+", "(?:){0,}", "(?:){1,}", "()*", "()+", "(a*)*",
        "(a?)+", "(|)*", "(|a)*", "(a|)*", "(?:a*)*", "(?:|)+", "((a)*)*",
        "(?:a?)*", "(a{0})*", "(?=a)*", "(?!a)*", "(a{0,3})*", "(\\b)*",
        "(^)*", "($)*", "(\\B)+", "(?:(?:))*",
    ] {
        expect_err(pat, "infinite loop matching the empty string");
    }
    // the bounded forms of the same atoms are fine
    for pat in ["(?:){0,3}", "(?:)?", "(a*)?", "(a*){2}", "(|a){0,2}"] {
        expect_ok(pat);
    }

    /* --- row 747: regexp.c:541 invalid back-reference (all 3 conjuncts) --- */
    for pat in [
        // yychar >= nsub
        r"\1", r"\2", r"\9", r"\15", r"(a)\2", r"(a)(b)\3", r"(a)\1\2",
        // !g->sub[yychar]: a reference inside the group it names
        r"(\1)", r"((\2))", r"(a(\2)b)", r"(?:(\1))",
    ] {
        expect_err(pat, "invalid back-reference");
    }
    for pat in [r"(a)\1", r"(a)(b)\2\1", r"(a(b))\2", r"(?:(a))\1"] {
        expect_ok(pat);
    }

    /* --- row 749/750/751/752: the four `unmatched '('` sites --- */
    expect_err("(a", "unmatched '('"); // :557 capturing
    expect_err("(", "unmatched '('");
    expect_err("((a)", "unmatched '('");
    expect_err("(a|b", "unmatched '('");
    expect_err("(?:a", "unmatched '('"); // :563 (?:
    expect_err("(?:", "unmatched '('");
    expect_err("(?:a|b", "unmatched '('");
    expect_err("(?=a", "unmatched '('"); // :570 (?=
    expect_err("(?=", "unmatched '('");
    expect_err("(?=a|b", "unmatched '('");
    expect_err("(?!a", "unmatched '('"); // :577 (?!
    expect_err("(?!", "unmatched '('");
    expect_err("(?!a|b", "unmatched '('");

    /* --- row 753: regexp.c:580 `die(g, "syntax error")` --- */
    // the only tokens reaching parseatom's fallthrough are '*', '+', '?' and
    // L_COUNT; EOF / '|' / ')' are filtered out by parsecat (regexp.c:610).
    for pat in [
        "*", "+", "?", "{2}", "{2,3}", "{0}", "*a", "+a", "?a", "{1}a",
        "a|*", "a|+", "a|?", "a|{2}", "|*", "(*)", "(?:*)", "(?=*)", "(?!*)",
        "(|*)", "a**", "a*+", "a*?*", "a{2}{3}", "^*", "$*", "\\b*", "\\B*",
        // a quantifier immediately after another quantifier is always the
        // regexp.c:580 site, never the regexp.c:493 empty-loop site: parserep
        // consumes exactly one postfix and the second one reaches parseatom.
        "[a]{0}*", "(?:a){0,}*", "a?*", "a+*", "a{0}+",
    ] {
        expect_err(pat, "syntax error");
    }

    /* --- row 754: regexp.c:598 `{M,N}` with max < min --- */
    for pat in ["a{3,1}", "a{2,0}", "a{10,9}", "a{254,1}", "a{1,0}"] {
        expect_err(pat, "invalid quantifier");
    }
    for pat in ["a{1,1}", "a{0,0}", "a{1,2}", "a{254,254}"] {
        expect_ok(pat);
    }

    /* --- row 760: regexp.c:940 unbalanced `)` --- */
    for pat in [")", "a)", "(a))", ")a", "a|)", "(?:a))", "[a])x)"] {
        expect_err(pat, "unmatched ')'");
    }
}

/// ERRORS rows 740 / 741: REPINF (255) in `lexcount`, from below and above.
/// regexp.c:185 tests `yymin >= REPINF` AFTER folding a digit, so `a{255}` is
/// the first rejected minimum and `a{254}` the last accepted one; likewise
/// regexp.c:199 for the maximum.
#[test]
fn t_regexp_numeric_overflow_boundary() {
    body_t_regexp_numeric_overflow_boundary();
}

fn body_t_regexp_numeric_overflow_boundary() {
    // exact boundary for the minimum (row 740, regexp.c:186)
    for n in 0..=300u32 {
        let pat = format!("a{{{n}}}");
        let got = diff_comp(&pat);
        let want = if n >= 255 {
            Comp::Err("numeric overflow".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "a{{{n}}} REPINF minimum boundary");
    }
    // exact boundary for the maximum (row 741, regexp.c:200)
    for n in 0..=300u32 {
        let pat = format!("a{{1,{n}}}");
        let got = diff_comp(&pat);
        let want = if n >= 255 {
            Comp::Err("numeric overflow".into())
        } else if n < 1 {
            Comp::Err("invalid quantifier".into()) // max < min, row 754
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "a{{1,{n}}} REPINF maximum boundary");
    }
    // and much larger / randomised counts, both positions
    let mut rng = Rng::new(0x5EED_0740);
    for _ in 0..600 {
        let n = rng.range(255, 1 << 40) as u64;
        expect_err(&format!("a{{{n}}}"), "numeric overflow");
        expect_err(&format!("a{{1,{n}}}"), "numeric overflow");
    }
    for pat in [
        "a{1000}", "a{99999}", "a{4294967296}", "a{2147483648}",
        "a{18446744073709551616}", "a{0,99999999999}", "a{1,256}", "a{255,}",
        "a{0000255}", "a{00254}",
    ] {
        let got = diff_comp(pat);
        // `a{0000255}` folds to 255 -> overflow; `a{00254}` folds to 254 -> ok
        let want = if pat == "a{00254}" {
            Comp::Ok(true)
        } else {
            Comp::Err("numeric overflow".into())
        };
        assert_eq!(got, want, "{pat}");
    }
}

/// ERRORS row 742: REG_MAXCLASS (128) in `newcclass`, driven from below to
/// above one class at a time.  Every `[...]`, `\d`, `\s`, `\w`, `\D`, `\S`,
/// `\W` allocates one.
#[test]
fn t_regexp_maxclass_boundary() {
    body_t_regexp_maxclass_boundary();
}

fn body_t_regexp_maxclass_boundary() {
    for unit in ["\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "[a]", "[^a]", "[a-b]"] {
        let mut first_fail = None;
        for n in 120..=134usize {
            let pat = unit.repeat(n);
            let got = diff_comp(&pat);
            match got {
                Comp::Err(ref m) => {
                    assert_eq!(m, "too many character classes", "unit={unit:?} n={n}");
                    if first_fail.is_none() {
                        first_fail = Some(n);
                    }
                }
                Comp::Ok(_) => assert!(
                    first_fail.is_none(),
                    "unit={unit:?} n={n} compiled after n={:?} failed",
                    first_fail
                ),
            }
        }
        assert_eq!(
            first_fail,
            Some(129),
            "REG_MAXCLASS boundary for unit={unit:?} (128 classes must be the last accepted)"
        );
    }
    // mixed units reach exactly the same boundary
    for n in 126..=131usize {
        let mut pat = String::new();
        for i in 0..n {
            pat.push_str(["\\d", "[a]", "\\W", "[^b-c]"][i % 4]);
        }
        let got = diff_comp(&pat);
        let want = if n > 128 {
            Comp::Err("too many character classes".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "mixed classes n={n}");
    }
}

/// ERRORS row 744: REG_MAXSPAN (64) in `addrange`.  regexp.c:252 rejects when
/// `cc->end + 2 >= cc->spans + 64`, i.e. the 32nd non-overlapping span, so 31
/// spans is the last accepted count.  The characters are spaced two apart so
/// the four "overlap / extend" fast paths at regexp.c:229-249 never merge them.
#[test]
fn t_regexp_maxspan_boundary() {
    body_t_regexp_maxspan_boundary();
}

fn body_t_regexp_maxspan_boundary() {
    // Every code point used here has a low byte outside {0x00, 'D', 'S', 'W',
    // 'd', 's', 'w'}: regexp.c:338 tests `strchr("DSWdsw", g->yychar)`, and
    // strchr truncates its `int` argument to `char`, so an escaped rune whose
    // low byte collides takes the class-shorthand arm and adds NO span at all.
    // n single characters spaced two apart -> n spans
    for n in 25..=40usize {
        let mut pat = String::from("[");
        for i in 0..n {
            pat.push_str(&format!("\\u{:04X}", 0x1001 + 2 * i));
        }
        pat.push(']');
        let got = diff_comp(&pat);
        let want = if n >= 32 {
            Comp::Err("too many character class ranges".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "{n} single-char spans");
    }
    // n explicit a-b ranges, spaced so they cannot merge
    for n in 25..=40usize {
        let mut pat = String::from("[");
        for i in 0..n {
            let lo = 0x2001 + 4 * i;
            pat.push_str(&format!("\\u{:04X}-\\u{:04X}", lo, lo + 1));
        }
        pat.push(']');
        let got = diff_comp(&pat);
        let want = if n >= 32 {
            Comp::Err("too many character class ranges".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "{n} explicit ranges");
    }
    // negated classes use the same Reclass storage
    for n in [31usize, 32] {
        let mut pat = String::from("[^");
        for i in 0..n {
            pat.push_str(&format!("\\u{:04X}", 0x3001 + 2 * i));
        }
        pat.push(']');
        let got = diff_comp(&pat);
        let want = if n >= 32 {
            Comp::Err("too many character class ranges".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "negated {n} spans");
    }
    // `\S` alone already installs 6 spans, `\W` 5: combining them with singles
    // must trip at the same total
    for extra in 24..=30usize {
        let mut pat = String::from("[\\S");
        for i in 0..extra {
            pat.push_str(&format!("\\u{:04X}", 0x4001 + 2 * i));
        }
        pat.push(']');
        diff_comp(&pat);
    }
    // a long a-z run overlaps into ONE span, so it must be accepted
    expect_ok(&format!("[{}]", "a-z".repeat(100)));
}

/// ERRORS row 748: REG_MAXSUB (16) in `parseatom`.  `g->nsub` starts at 1, so
/// 15 capturing groups is the maximum and the 16th `(` dies.
#[test]
fn t_regexp_maxsub_boundary() {
    body_t_regexp_maxsub_boundary();
}

fn body_t_regexp_maxsub_boundary() {
    for n in 0..=20usize {
        // flat groups
        let pat = "(a)".repeat(n);
        let got = diff_comp(&pat);
        let want = if n > 15 {
            Comp::Err("too many captures".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "{n} flat capturing groups");

        // nested groups reach the same limit
        let pat = format!("{}a{}", "(".repeat(n), ")".repeat(n));
        let got = diff_comp(&pat);
        assert_eq!(got, want, "{n} nested capturing groups");

        // `(?:` groups are NOT captures and must never trip it
        let pat = format!("{}a{}", "(?:".repeat(n), ")".repeat(n));
        assert_eq!(diff_comp(&pat), Comp::Ok(true), "{n} non-capturing groups");
    }
    assert_eq!(
        diff_comp("(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)"),
        Comp::Ok(true)
    );
    expect_err(
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)",
        "too many captures",
    );
}

/// ERRORS row 755: REG_MAXREC (4096) in `count()`.  `parsecat`
/// (regexp.c:611-620) builds a RIGHT-leaning `P_CAT` chain, so a run of N
/// literal characters produces a tree of depth N-1 and `count()` recurses
/// exactly that deep; `++depth > REG_MAXREC` therefore trips at 4097
/// characters (a chain of N-1 `P_CAT` nodes bottoms out in a leaf at depth N,
/// and the guard fires when the incremented depth exceeds 4096).  Note this is
/// the first limit a long literal run hits: the
/// regexp.c:922 `strlen * 2 > REG_MAXPROG` test needs 16385 characters
/// (row 758) and the regexp.c:951 total-size test needs even more.
#[test]
fn t_regexp_count_recursion_limit() {
    with_big_stack(body_t_regexp_count_recursion_limit);
}

fn body_t_regexp_count_recursion_limit() {
    for n in 4090..=4106usize {
        let pat = "a".repeat(n);
        let got = diff_comp(&pat);
        let want = if n >= 4097 {
            Comp::Err("stack overflow".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "literal run of {n} -> count() depth {}", n - 1);
    }
    // other atoms that produce one P_CAT link each
    // 4-byte units would trip the regexp.c:922 length test first (4097 * 4 >
    // 16384), so only 1- and 2-byte atoms are used here.
    for unit in ["a", ".", "\\n", "\\t"] {
        for n in [4094usize, 4095, 4096, 4097, 4098, 4099] {
            let pat = unit.repeat(n);
            let got = diff_comp(&pat);
            let want = if n >= 4097 {
                Comp::Err("stack overflow".into())
            } else {
                Comp::Ok(true)
            };
            assert_eq!(got, want, "unit={unit:?} n={n}");
        }
    }
    // P_ALT chains recurse through count(node->x) as well
    for n in [2040usize, 2048, 2049, 4097] {
        let pat = "a|".repeat(n);
        diff_comp(&pat);
    }
}

/// ERRORS row 756: regexp.c:672 `n > REG_MAXPROG` for a single `P_REP` node.
/// `(?:a{M}){N}` has `count = M * N`, so the per-node site fires at
/// `M * N > 32768` with both factors under REPINF.
///
/// Both `"program too large"` sites reachable here emit the identical string,
/// and the regexp.c:950 whole-program test (`6 + count > 32768`, row 762) is
/// only 6 instructions away from the per-node one, so the predicate below is
/// the union `6 + count > REG_MAXPROG`; the `deep` cases at the end have
/// `M * N` far above 32768 and can therefore only be the regexp.c:672 site.
#[test]
fn t_regexp_rep_program_too_large() {
    body_t_regexp_rep_program_too_large();
}

fn body_t_regexp_rep_program_too_large() {
    let want = |count: u64| -> Comp {
        if count > 32768 || count + 6 > 32768 {
            Comp::Err("program too large".into())
        } else {
            Comp::Ok(true)
        }
    };
    // min == max arm: count = count(x) * min -- these are the row-756 site
    // proper, since M*N is far past REG_MAXPROG.
    for (m, n) in [
        (254u32, 254u32), (200, 200), (181, 181), (181, 182), (182, 181),
        (128, 254), (254, 128), (2, 254), (100, 100), (254, 130), (16, 254),
    ] {
        let pat = format!("(?:a{{{m}}}){{{n}}}");
        assert_eq!(
            diff_comp(&pat),
            want(m as u64 * n as u64),
            "(?:a{{{m}}}){{{n}}} count={}",
            m * n
        );
    }
    // walk the exact boundary of the min == max arm
    for n in 125..=135u32 {
        let pat = format!("(?:a{{254}}){{{n}}}");
        assert_eq!(diff_comp(&pat), want(254 * n as u64), "(?:a{{254}}){{{n}}}");
    }
    // max < REPINF arm: count = count(x) * max + (max - min)
    for n in 125..=135u32 {
        let pat = format!("(?:a{{254}}){{1,{n}}}");
        assert_eq!(
            diff_comp(&pat),
            want(254 * n as u64 + (n as u64 - 1)),
            "(?:a{{254}}){{1,{n}}}"
        );
    }
    // max == REPINF arm: count = count(x) * (min + 1) + 2
    for n in 125..=135u32 {
        let pat = format!("(?:a{{254}}){{{n},}}");
        assert_eq!(
            diff_comp(&pat),
            want(254 * (n as u64 + 1) + 2),
            "(?:a{{254}}){{{n},}}"
        );
    }
    // three levels of nesting: the innermost overflowing node trips first
    expect_err("(?:(?:a{100}){100}){100}", "program too large");
    expect_err("(?:(?:(?:a{20}){20}){20}){20}", "program too large");
}

/// ERRORS row 758: regexp.c:921 `strlen(pattern) * 2 > REG_MAXPROG`, checked
/// BEFORE anything is parsed, so 16384 bytes is the last accepted length.  A
/// 16384-byte literal run then dies in `count()` instead (row 755), which is
/// exactly what the boundary sweep asserts.
#[test]
fn t_regexp_pattern_length_limit() {
    with_big_stack(body_t_regexp_pattern_length_limit);
}

fn body_t_regexp_pattern_length_limit() {
    for n in 16380..=16390usize {
        let pat = "a".repeat(n);
        let got = diff_comp(&pat);
        let want = if n > 16384 {
            // regexp.c:922 -- rejected on length alone, before parsing
            Comp::Err("program too large".into())
        } else {
            // parsed, then rejected by count() at depth n > 4096
            Comp::Err("stack overflow".into())
        };
        assert_eq!(got, want, "pattern length {n}");
    }
    // a pattern that is long but parses shallowly still trips the length test
    for n in [8191usize, 8192, 8193] {
        // "(?:)" is 4 bytes and adds no parse node
        let pat = "(?:)".repeat(n);
        let got = diff_comp(&pat);
        let want = if 4 * n > 16384 {
            Comp::Err("program too large".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "(?:) x {n} = {} bytes", 4 * n);
    }
    // and much longer
    for n in [20000usize, 40000, 100_000] {
        expect_err(&"a".repeat(n), "program too large");
    }
    // the empty pattern is the other end of the same computation (n == 0 skips
    // the parse-list allocation entirely, regexp.c:923)
    expect_ok("");
}

/// ERRORS row 762: regexp.c:950 `6 + count(...) > REG_MAXPROG`.  Reached with a
/// pattern that is SHORT (so row 758 does not fire), SHALLOW (so row 755 does
/// not fire) and whose per-node counts are all small (so row 756 does not
/// fire), but whose total is over 32768: a flat concatenation of `a{254}`.
#[test]
fn t_regexp_total_program_size_limit() {
    body_t_regexp_total_program_size_limit();
}

fn body_t_regexp_total_program_size_limit() {
    for k in 120..=140usize {
        let pat = "a{254}".repeat(k);
        let got = diff_comp(&pat);
        let want = if 6 + 254 * k > 32768 {
            Comp::Err("program too large".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(got, want, "a{{254}} x {k}, total = {}", 6 + 254 * k);
    }
    // the same boundary through a different shape: `[a]{254}` (one cclass)
    for k in 126..=131usize {
        let pat = format!("[a]{{254}}{}", "a{254}".repeat(k));
        diff_comp(&pat);
    }
    // Captures cost 2 extra instructions each (regexp.c:674) but REG_MAXSUB
    // caps them at 15, so pad 15 capturing groups with plain repeats and walk
    // the boundary again: total = 6 + 15*(254+2) + 254*k.
    for k in 110..=132usize {
        let pat = format!("{}{}", "(a{254})".repeat(15), "a{254}".repeat(k));
        let got = diff_comp(&pat);
        let want = if 6 + 15 * 256 + 254 * k > 32768 {
            Comp::Err("program too large".into())
        } else {
            Comp::Ok(true)
        };
        assert_eq!(
            got,
            want,
            "15 captures + a{{254}} x {k}, total = {}",
            6 + 15 * 256 + 254 * k
        );
    }
    // and through P_ALT nodes, which cost 2 extra each (regexp.c:665)
    for k in 126..=132usize {
        let pat = "a{254}|".repeat(k);
        diff_comp(&pat);
    }
}

/* --------------------------------------------------------------------- *
 *  regexp.c rows 757 / 759 / 763 / 764: the four allocation failures
 * -------------------------------------------------------------------- */

extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

thread_local! {
    /// 1-based index of the non-zero-size allocation to fail; 0 = never fail.
    static AFAIL_AT: Cell<i32> = const { Cell::new(0) };
    static AFAIL_N: Cell<i32> = const { Cell::new(0) };
    /// (ptr-was-NULL, requested size) for every callback, in order.
    static ALOG: RefCell<Vec<(bool, c_int)>> = const { RefCell::new(Vec::new()) };
}

const ACTX: usize = 0xBADA_55;

unsafe extern "C" fn fail_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    assert_eq!(ctx as usize, ACTX, "regcompx passed the wrong ctx through");
    ALOG.with(|v| v.borrow_mut().push((p.is_null(), n)));
    if n == 0 {
        libc_free(p);
        return std::ptr::null_mut();
    }
    let k = AFAIL_N.with(|c| {
        let v = c.get() + 1;
        c.set(v);
        v
    });
    if AFAIL_AT.with(|c| c.get()) == k {
        return std::ptr::null_mut();
    }
    libc_realloc(p, n as usize)
}

/// ERRORS rows 757 (regexp.c:916 Reprog), 759 (regexp.c:926 parse list),
/// 763 (regexp.c:956 instruction list) and 764 (regexp.c:961 character class
/// list): `js_regcompx` with an allocator that returns NULL for the k-th
/// request.  The FULL callback sequence -- including every `alloc(ctx, p, 0)`
/// free performed by the `setjmp` cleanup at regexp.c:903-911 -- is recorded
/// and compared, so the two libraries must agree on the number, the order and
/// the requested SIZE of every allocation as well as on the error string.
#[test]
fn t_regexp_alloc_failures() {
    body_t_regexp_alloc_failures();
}

fn body_t_regexp_alloc_failures() {
    let p = libs();
    // "" allocates Reprog + instruction list only (n == 0 skips the parse
    // list); "a" adds the parse list; "[a]" adds the character class list.
    let cases: &[(&str, &[&str])] = &[
        ("", &["cannot allocate regular expression", "cannot allocate regular expression instruction list"]),
        ("a", &["cannot allocate regular expression", "cannot allocate regular expression parse list", "cannot allocate regular expression instruction list"]),
        ("[a]", &["cannot allocate regular expression", "cannot allocate regular expression parse list", "cannot allocate regular expression instruction list", "cannot allocate regular expression character class list"]),
        ("(a)\\1", &["cannot allocate regular expression", "cannot allocate regular expression parse list", "cannot allocate regular expression instruction list"]),
        ("\\d[b-c]x", &["cannot allocate regular expression", "cannot allocate regular expression parse list", "cannot allocate regular expression instruction list", "cannot allocate regular expression character class list"]),
        ("(?:a|b)+[^x]", &["cannot allocate regular expression", "cannot allocate regular expression parse list", "cannot allocate regular expression instruction list", "cannot allocate regular expression character class list"]),
    ];
    unsafe {
        for (pat, msgs) in cases {
            for fail_at in 0..=(msgs.len() as i32 + 1) {
                for cflags in [0, REG_ICASE] {
                    let mut got = vec![];
                    for l in [&p.c, &p.rs] {
                        AFAIL_AT.with(|c| c.set(fail_at));
                        AFAIL_N.with(|c| c.set(0));
                        ALOG.with(|v| v.borrow_mut().clear());
                        let cp = cstr(pat);
                        let mut e: *const c_char = ERRP_POISON;
                        let prog = l.js_regcompx(
                            Some(fail_alloc),
                            ACTX as *mut c_void,
                            cp.as_ptr(),
                            cflags,
                            &mut e,
                        );
                        let res = if prog.is_null() {
                            assert_ne!(
                                e, ERRP_POISON,
                                "{}: regcompx({pat:?}) NULL without writing *errorp",
                                l.name
                            );
                            from_c(e)
                        } else {
                            assert!(
                                e.is_null(),
                                "{}: regcompx({pat:?}) success must NULL *errorp",
                                l.name
                            );
                            l.js_regfreex(Some(fail_alloc), ACTX as *mut c_void, prog);
                            "<ok>".to_string()
                        };
                        let log = ALOG.with(|v| v.borrow().clone());
                        got.push((res, log));
                    }
                    assert_eq!(
                        got[0], got[1],
                        "regcompx alloc-failure divergence pat={pat:?} \
                         fail_at={fail_at} cflags={cflags}"
                    );
                    let want = if fail_at >= 1 && fail_at <= msgs.len() as i32 {
                        msgs[(fail_at - 1) as usize].to_string()
                    } else {
                        "<ok>".to_string()
                    };
                    assert_eq!(
                        got[0].0, want,
                        "regcompx pat={pat:?} fail_at={fail_at} wrong error"
                    );
                    trace(
                        &format!("alloc pat={pat:?} fail_at={fail_at}"),
                        &format!("{:?}", got[0]),
                    );
                }
            }
        }
        // an allocator that fails EVERY request, over a wide pattern set
        for pat in [
            "", "a", "abc", "[a-z]+", "(a)(b)(c)", "(?=x)y", "\\d\\s\\w",
            "^a$", "a{2,4}", ".*", "(a)\\1",
        ] {
            for l in [&p.c, &p.rs] {
                AFAIL_AT.with(|c| c.set(1));
                AFAIL_N.with(|c| c.set(0));
                ALOG.with(|v| v.borrow_mut().clear());
                let cp = cstr(pat);
                let mut e: *const c_char = ERRP_POISON;
                let prog =
                    l.js_regcompx(Some(fail_alloc), ACTX as *mut c_void, cp.as_ptr(), 0, &mut e);
                assert!(prog.is_null(), "{}: {pat:?} should fail", l.name);
                assert_eq!(from_c(e), "cannot allocate regular expression");
            }
        }
        AFAIL_AT.with(|c| c.set(0));
    }
}

/* ===================================================================== *
 *  regexp.c: match-time rejections (rows 765-793)
 * ===================================================================== */

/// Everything `js_regexec` writes, for one (subject, eflags) pair.  ALL 16
/// `sub` slots are recorded whatever the return value, because ERRORS row 793
/// says regexp.c:1236-1237 pre-clears every one of them; the slots are poisoned
/// with the non-pointer values 1 and 2 first so an untouched slot is visible.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Exec {
    rc: c_int,
    nsub: c_int,
    sub: Vec<(i64, i64)>,
}

fn pv(p: *const c_char, base: *const c_char) -> i64 {
    if p.is_null() {
        i64::MIN
    } else if (p as usize) < 4096 {
        // still a poison value: report it verbatim
        -(p as i64) - 1_000_000
    } else {
        (p as isize - base as isize) as i64
    }
}

unsafe fn exec1(l: &Lib, prog: *mut c_void, text: &str, eflags: c_int) -> Exec {
    let ct = cstr(text);
    let base = ct.as_ptr();
    let mut sub = Resub {
        nsub: 12345,
        ..Default::default()
    };
    for i in 0..REG_MAXSUB {
        sub.sub[i].sp = 1 as *const c_char;
        sub.sub[i].ep = 2 as *const c_char;
    }
    let rc = l.js_regexec(prog, base, &mut sub, eflags);
    Exec {
        rc,
        nsub: sub.nsub,
        sub: (0..REG_MAXSUB)
            .map(|i| (pv(sub.sub[i].sp, base), pv(sub.sub[i].ep, base)))
            .collect(),
    }
}

/// Compile `pat` on both libraries, run every `(text, eflags)` pair, and
/// assert the results are identical.  Returns the C library's results so a
/// caller can additionally assert the exact return code.
fn diff_exec(pat: &str, cflags: c_int, runs: &[(&str, c_int)]) -> Vec<Exec> {
    let p = libs();
    unsafe {
        let cp = cstr(pat);
        let mut ea: *const c_char = std::ptr::null();
        let mut eb: *const c_char = std::ptr::null();
        let pa = p.c.js_regcomp(cp.as_ptr(), cflags, &mut ea);
        let pb = p.rs.js_regcomp(cp.as_ptr(), cflags, &mut eb);
        assert_eq!(pa.is_null(), pb.is_null(), "compile mismatch for {pat:?}");
        assert!(
            !pa.is_null(),
            "{pat:?} must compile for a match-time test (C said {:?})",
            from_c(ea)
        );
        let mut out = vec![];
        for (text, eflags) in runs {
            let a = exec1(&p.c, pa, text, *eflags);
            let b = exec1(&p.rs, pb, text, *eflags);
            assert_eq!(
                a, b,
                "js_regexec divergence pat={pat:?} cflags={cflags} \
                 text={text:?} eflags={eflags}"
            );
            out.push(a);
        }
        p.c.js_regfree(pa);
        p.rs.js_regfree(pb);
        out
    }
}

/// Assert both libraries return exactly `want` for every run.
fn expect_exec(pat: &str, cflags: c_int, runs: &[(&str, c_int)], want: c_int) {
    for (i, e) in diff_exec(pat, cflags, runs).iter().enumerate() {
        assert_eq!(
            e.rc, want,
            "js_regexec({pat:?}, {cflags}, {:?}) should return {want}",
            runs[i]
        );
        // row 793: every sub slot is cleared before matching, so an untouched
        // slot reads back as NULL (i64::MIN), never as the poison 1 / 2.
        for (k, (sp, ep)) in e.sub.iter().enumerate() {
            assert!(
                *sp > -1_000_000 || *sp == i64::MIN,
                "sub[{k}].sp left poisoned for {pat:?} {:?}",
                runs[i]
            );
            assert!(
                *ep > -1_000_000 || *ep == i64::MIN,
                "sub[{k}].ep left poisoned for {pat:?} {:?}",
                runs[i]
            );
        }
    }
}

/// ERRORS rows 768, 770-787, 789-793: every REACHABLE `return 1` (no match)
/// site in `match()`, each driven by an input that can only get there.
#[test]
fn t_regexp_match_nomatch_sites() {
    body_t_regexp_match_nomatch_sites();
}

fn body_t_regexp_match_nomatch_sites() {
    /* row 768 -- regexp.c:1102 I_PLA body failed */
    expect_exec("a(?=b)", 0, &[("ac", 0), ("a", 0), ("axb", 0)], 1);
    expect_exec("(?=b)", 0, &[("aaa", 0)], 1);
    /* row 770 -- regexp.c:1111 I_NLA body DID match */
    expect_exec("a(?!b)", 0, &[("ab", 0)], 1);
    expect_exec("^(?!a)", 0, &[("abc", 0)], 1);

    /* row 771 -- regexp.c:1116 I_ANYNL at end of subject.  I_ANYNL only
     * appears in the standard prologue (regexp.c:967-972), which advances the
     * start position one rune at a time; it is what finally reports "no
     * match" once the subject is exhausted. */
    expect_exec("zzz", 0, &[("", 0), ("a", 0), ("ab", 0), ("\n", 0)], 1);
    expect_exec("$x", 0, &[("", 0), ("abc", 0)], 1);

    /* row 772 -- regexp.c:1121 I_ANY at end of subject */
    expect_exec(".", 0, &[("", 0)], 1);
    expect_exec("a.", 0, &[("a", 0)], 1);
    expect_exec("..", 0, &[("a", 0)], 1);
    /* row 773 -- regexp.c:1124 I_ANY on a line terminator */
    expect_exec("a.b", 0, &[("a\nb", 0), ("a\rb", 0), ("a\u{2028}b", 0), ("a\u{2029}b", 0)], 1);
    expect_exec("^.$", REG_NEWLINE, &[("\n", 0)], 1);
    // and the same positions DO match with I_ANYNL semantics via the prologue
    expect_exec("a.b", 0, &[("a\tb", 0), ("axb", 0)], 0);

    /* row 774 -- regexp.c:1128 I_CHAR at end of subject */
    expect_exec("a", 0, &[("", 0)], 1);
    expect_exec("ab", 0, &[("a", 0), ("xa", 0)], 1);
    /* row 775 -- regexp.c:1133 I_CHAR mismatch (with and without ICASE) */
    expect_exec("a", 0, &[("b", 0), ("B", 0), ("A", 0)], 1);
    expect_exec("a", REG_ICASE, &[("b", 0), ("B", 0)], 1);
    expect_exec("a", REG_ICASE, &[("A", 0)], 0);
    expect_exec("\u{131}", REG_ICASE, &[("i", 0), ("I", 0)], 1);

    /* row 776 -- regexp.c:1137 I_CCLASS at end of subject */
    expect_exec("a[b]", 0, &[("a", 0)], 1);
    expect_exec("[a]", 0, &[("", 0)], 1);
    /* row 778 / 789 -- regexp.c:1144 + incclass() returning 0 */
    expect_exec("[a]", 0, &[("b", 0), ("A", 0), ("\u{e9}", 0)], 1);
    expect_exec("[a-c]", 0, &[("d", 0), ("`", 0)], 1);
    expect_exec("[\\d]", 0, &[("x", 0)], 1);
    /* row 777 / 790 -- regexp.c:1141 + incclasscanon() returning 0 */
    expect_exec("[a]", REG_ICASE, &[("b", 0), ("B", 0)], 1);
    expect_exec("[a]", REG_ICASE, &[("A", 0)], 0);
    expect_exec("[a-c]", REG_ICASE, &[("d", 0), ("D", 0)], 1);

    /* row 779 -- regexp.c:1149 I_NCCLASS at end of subject */
    expect_exec("a[^b]", 0, &[("a", 0)], 1);
    expect_exec("[^a]", 0, &[("", 0)], 1);
    /* row 781 -- regexp.c:1156 I_NCCLASS hit */
    expect_exec("^[^a]$", 0, &[("a", 0)], 1);
    expect_exec("^[^a-c]$", 0, &[("a", 0), ("b", 0), ("c", 0)], 1);
    /* row 780 -- regexp.c:1153 I_NCCLASS ICASE hit */
    expect_exec("^[^a]$", REG_ICASE, &[("a", 0), ("A", 0)], 1);
    expect_exec("^[^a-c]$", REG_ICASE, &[("B", 0), ("C", 0)], 1);

    /* row 783 -- regexp.c:1167 I_REF byte mismatch */
    expect_exec("^(a)\\1$", 0, &[("ax", 0), ("ab", 0), ("aA", 0)], 1);
    expect_exec("^(a)\\1$", 0, &[("aa", 0)], 0);
    /* row 782 -- regexp.c:1164 I_REF canonical mismatch */
    expect_exec("^(a)\\1$", REG_ICASE, &[("ax", 0), ("ab", 0)], 1);
    expect_exec("^(a)\\1$", REG_ICASE, &[("aA", 0), ("aa", 0)], 0);
    /* row 791 -- regexp.c:1056 strncmpcanon: the SUBJECT runs out first
     * (returns -1).  `(abc)\1` against "abca" compares 3 runes but only 1 is
     * left after the group. */
    expect_exec("(abc)\\1", REG_ICASE, &[("abca", 0), ("abcab", 0), ("abc", 0)], 1);
    /* row 792 -- regexp.c:1057 strncmpcanon: the REFERENCE text runs out
     * first (returns 1).  `i` counts BYTES, not runes, so a 2-byte rune group
     * makes strncmpcanon walk two runes from a position where only one is
     * left.  The lookahead captures a group AFTER the position `\1` runs at,
     * so `b` is closer to the NUL than `a` is. */
    expect_exec("(?=.(\u{e9}))\\1", REG_ICASE, &[("\u{e9}\u{e9}", 0)], 1);
    // ...and without ICASE the same input takes the plain strncmp path, which
    // compares BYTES and therefore succeeds -- a divergence trap.
    expect_exec("(?=.(\u{e9}))\\1", 0, &[("\u{e9}\u{e9}", 0)], 0);

    /* row 784 -- regexp.c:1185 I_BOL */
    expect_exec("^b", 0, &[("ab", 0)], 1);
    expect_exec("^a", 0, &[("a", REG_NOTBOL)], 1);
    expect_exec("^a", REG_NEWLINE, &[("ba", 0)], 1);
    expect_exec("^a", REG_NEWLINE, &[("b\na", 0)], 0);
    expect_exec("^a", REG_NEWLINE, &[("a", REG_NOTBOL)], 1);
    expect_exec("^a", REG_NEWLINE, &[("\na", REG_NOTBOL)], 0);
    /* row 785 -- regexp.c:1197 I_EOL */
    expect_exec("a$", 0, &[("ab", 0)], 1);
    expect_exec("a$", REG_NEWLINE, &[("ab", 0)], 1);
    expect_exec("a$", REG_NEWLINE, &[("a\nb", 0)], 0);
    expect_exec("^$", 0, &[("a", 0)], 1);
    /* row 786 -- regexp.c:1202 I_WORD (\b) */
    expect_exec("\\ba", 0, &[("ba", 0), ("1a", 0), ("_a", 0)], 1);
    expect_exec("\\b", 0, &[("", 0)], 1);
    expect_exec("a\\b", 0, &[("ab", 0), ("a1", 0), ("a_", 0)], 1);
    /* row 787 -- regexp.c:1209 I_NWORD (\B) */
    expect_exec("^\\Ba", 0, &[("a", 0)], 1);
    // `\B` never matches a single word character: position 0 has
    // iswordchar(sp[-1]) forced to 0 (regexp.c:1206 guards on `sp > bol`) and
    // position 1 has iswordchar(sp[0]) == 0, so the xor is 1 at both.
    expect_exec("\\B", 0, &[("a", 0), ("1", 0), ("_", 0)], 1);
    expect_exec("\\B", 0, &[("ab", 0), (" ", 0), ("", 0)], 0);
    expect_exec("a\\Bb", 0, &[("a b", 0)], 1);

    /* row 793 -- regexp.c:1239 the return-value contract itself */
    assert_eq!(diff_exec("a", 0, &[("a", 0)])[0].rc, 0);
    assert_eq!(diff_exec("a", 0, &[("b", 0)])[0].rc, 1);
    // nsub is always overwritten with prog->nsub, even on no-match
    for (pat, want_nsub) in [("a", 1), ("(a)", 2), ("(a)(b)", 3), ("(?:a)", 1)] {
        for text in ["", "zzz", "ab"] {
            let e = &diff_exec(pat, 0, &[(text, 0)])[0];
            assert_eq!(e.nsub, want_nsub, "nsub for {pat:?} on {text:?}");
        }
    }
}

/// ERRORS rows 765-767, 769: `match()` hitting REG_MAXREC (4096) returns -1,
/// and the I_SPLIT / I_PLA / I_NLA arms propagate it.
///
/// LINEAR patterns over a long subject are used so the depth counter is walked
/// straight to the limit; a catastrophic-backtracking pattern like `(a*)*b`
/// also gets there but explores exponentially many paths first.  Every pattern
/// is additionally ANCHORED with `^`: without REG_NEWLINE the I_BOL at
/// regexp.c:1174 fails instantly at every start position after 0, which keeps
/// the whole scan O(n) instead of O(n^2).  An unanchored `a*b` copies a
/// 260-byte `Resub` twice per I_SPLIT frame (regexp.c:1086/1091), so the
/// quadratic form costs gigabytes of memcpy for n near 4096.
#[test]
fn t_regexp_match_recursion_limit() {
    with_big_stack(body_t_regexp_match_recursion_limit);
}

fn body_t_regexp_match_recursion_limit() {
    // row 765/766: the recursion happens inside I_SPLIT, so `^a*b` over N a's
    // recurses N deep and the -1 is propagated back up through every frame.
    for n in [4095usize, 4096, 4097, 4098, 4200, 9000] {
        let text = "a".repeat(n);
        for pat in [
            "^a*b", "^a+b", "^a*?b", "^[a]*b", "^(?:a)*b", "^.*x", "^(a)*b",
        ] {
            diff_exec(pat, 0, &[(text.as_str(), 0)]);
        }
    }
    /// Walk `n` one character at a time and return the first subject length at
    /// which `regexec` flips from `below` to -1.  `diff_exec` compares the two
    /// libraries at every single `n`, so the returned flip point is necessarily
    /// identical for C and Rust -- a divergence of even one character in the
    /// depth accounting would fail inside `diff_exec` first.
    fn flip_point(pat: &str, below: c_int) -> usize {
        let mut flip = None;
        for n in 4080..=4110usize {
            let text = "a".repeat(n);
            let rc = diff_exec(pat, 0, &[(text.as_str(), 0)])[0].rc;
            if rc == -1 && flip.is_none() {
                flip = Some(n);
            }
            if flip.is_some() {
                assert_eq!(rc, -1, "{pat:?} n={n} should stay at -1 once it flips");
            } else {
                assert_eq!(rc, below, "{pat:?} n={n} should still be {below}");
            }
        }
        flip.unwrap_or_else(|| panic!("{pat:?} never reached REG_MAXREC in 4080..4110"))
    }

    // row 765/766: I_SPLIT.  The exact flip depends on how many frames the
    // standard prologue and the compiled shape add on top of one frame per
    // repetition, so it is only required to be near REG_MAXREC -- what matters
    // is that C and Rust agree on it to the character (enforced inside
    // `flip_point` through `diff_exec`).
    for pat in ["^a*b", "^a+b", "^(a)*b", "^(?:a)*b", "^[a]*b", "^.*x"] {
        let f = flip_point(pat, 1);
        assert!(
            (4090..=4100).contains(&f),
            "{pat:?} flipped to -1 at n={f}, expected within 10 of REG_MAXREC"
        );
        trace("regexec -1 flip", &format!("{pat} -> {f}"));
    }
    // A NON-GREEDY unbounded repeat never reaches REG_MAXREC at all: for
    // `a*?` the I_SPLIT's `x` is the LOOP EXIT (regexp.c:740-742), so the
    // recursive call at regexp.c:1087 always returns immediately and the
    // iteration itself proceeds through `pc = pc->y` without adding a frame.
    for pat in ["^a*?b", "^a+?b", "^(?:a)*?b", "^[a]*?b"] {
        for n in [4096usize, 4200, 20000] {
            let text = "a".repeat(n);
            let rc = diff_exec(pat, 0, &[(text.as_str(), 0)])[0].rc;
            assert_eq!(rc, 1, "{pat:?} n={n} must be a plain no-match, not -1");
        }
    }
    // row 767 (I_PLA) and row 769 (I_NLA) propagating -1: the deep repetition
    // sits INSIDE a lookahead, so the -1 must cross that frame.  `(?!...)`
    // turns the inner NO-MATCH into a match (below == 0) but must propagate a
    // -1 verbatim (regexp.c:1108-1109), so the two shapes have different
    // "below" values and identical flip behaviour.
    for (pat, below) in [
        ("^(?=a*b)", 1),
        ("^(?!a*b)", 0),
        ("^(?=a*b)a", 1),
        ("^(?:(?=a*b))", 1),
        ("^(?:x|(?=a*b))", 1),
        ("^(?=(?=a*b))", 1),
        ("^(?!(?!a*b))", 1),
    ] {
        let f = flip_point(pat, below);
        assert!(
            (4085..=4100).contains(&f),
            "{pat:?} flipped to -1 at n={f}"
        );
        trace("regexec -1 flip (lookahead)", &format!("{pat} -> {f}"));
    }
    // a small catastrophic case, cheap enough to finish quickly
    // (`(a*)*b` itself is rejected at compile time by regexp.c:493.)
    for pat in ["(a|a)*b", "(a+)+b", "(?:a+)+b", "(a?a)*b"] {
        for n in [12usize, 16, 18] {
            let text = "a".repeat(n);
            diff_exec(pat, 0, &[(text.as_str(), 0)]);
        }
    }
}

/// Generic FFI boundary abuse for the regexp entry points: `sub == NULL`, zero
/// length subjects, out-of-range `cflags` / `eflags` (negative, 8, 255,
/// INT_MAX, INT_MIN -- none of which name a valid REG_* variant).
#[test]
fn t_regexp_ffi_boundary_abuse() {
    body_t_regexp_ffi_boundary_abuse();
}

fn body_t_regexp_ffi_boundary_abuse() {
    let p = libs();
    let pats = [
        "a", "^a", "a$", "A", "(a)(b)", ".", "[a-z]+", "\\b", "(a)\\1", "(?=a)",
        "(?!a)", "a*b", "",
    ];
    let texts = ["", "a", "A", "abc", "ABC", "a\nb", "\n", "\u{e9}", "aaab"];
    let oor: [c_int; 14] = [
        -1, -2, 0, 3, 4, 5, 7, 8, 16, 64, 255, 1024, c_int::MAX, c_int::MIN,
    ];
    unsafe {
        for pat in pats {
            for cflags in oor {
                let cp = cstr(pat);
                let mut ea: *const c_char = ERRP_POISON;
                let mut eb: *const c_char = ERRP_POISON;
                let pa = p.c.js_regcomp(cp.as_ptr(), cflags, &mut ea);
                let pb = p.rs.js_regcomp(cp.as_ptr(), cflags, &mut eb);
                assert_eq!(
                    pa.is_null(),
                    pb.is_null(),
                    "oor cflags={cflags} pat={pat:?}"
                );
                if pa.is_null() {
                    assert_eq!(from_c(ea), from_c(eb));
                    continue;
                }
                for text in texts {
                    for eflags in oor {
                        let a = exec1(&p.c, pa, text, eflags);
                        let b = exec1(&p.rs, pb, text, eflags);
                        assert_eq!(
                            a, b,
                            "oor pat={pat:?} cflags={cflags} eflags={eflags} text={text:?}"
                        );
                        // sub == NULL is well defined (regexp.c:1232)
                        let ct = cstr(text);
                        let ra =
                            p.c.js_regexec(pa, ct.as_ptr(), std::ptr::null_mut(), eflags);
                        let rb =
                            p.rs.js_regexec(pb, ct.as_ptr(), std::ptr::null_mut(), eflags);
                        assert_eq!(
                            (ra, rb),
                            (a.rc, a.rc),
                            "sub=NULL pat={pat:?} eflags={eflags} text={text:?}"
                        );
                    }
                }
                p.c.js_regfree(pa);
                p.rs.js_regfree(pb);
            }
        }
        // js_regfree(NULL) / js_regfreex(.., NULL) stay no-ops
        p.c.js_regfree(std::ptr::null_mut());
        p.rs.js_regfree(std::ptr::null_mut());
        AFAIL_AT.with(|c| c.set(0));
        p.c.js_regfreex(Some(fail_alloc), ACTX as *mut c_void, std::ptr::null_mut());
        p.rs.js_regfreex(Some(fail_alloc), ACTX as *mut c_void, std::ptr::null_mut());
    }
}

/// Randomised compile fuzzing focused on REJECTIONS: fragments are drawn from a
/// grammar that includes every malformed shape, and the exact `*errorp` string
/// is compared.  Fixed seed.
#[test]
fn t_regexp_error_fuzz() {
    body_t_regexp_error_fuzz();
}

fn body_t_regexp_error_fuzz() {
    let frags = [
        "a", "b", ".", "\\d", "\\W", "[abc]", "[^a-c]", "(a)", "(?:b)", "(?=c)",
        "(?!d)", "\\1", "\\2", "(", ")", "[", "]", "{", "}", "*", "+", "?",
        "|", "^", "$", "\\", "\\x", "\\x4", "\\u", "\\u12", "\\c", "\\y", "\\_",
        "{2}", "{2,1}", "{255}", "{1,255}", "{x}", "[z-a]", "(?:)*", "(a*)*",
        "\\b", "\\B", "\\0", "\\xZZ", "\\uZZZZ", "(?", "(?a", "[\\d-x]",
    ];
    let mut rng = Rng::new(0x0BAD_C0DE_0000_0731);
    let mut seen_msgs = std::collections::BTreeSet::new();
    for _ in 0..40000 {
        let n = 1 + rng.below(6) as usize;
        let mut pat = String::new();
        for _ in 0..n {
            pat.push_str(frags[rng.below(frags.len() as u32) as usize]);
        }
        let got = diff_comp(&pat);
        if let Comp::Err(m) = got {
            seen_msgs.insert(m);
        }
    }
    // the fuzzer must have exercised a broad slice of the message set
    assert!(
        seen_msgs.len() >= 12,
        "regexp error fuzz only produced {} distinct messages: {seen_msgs:?}",
        seen_msgs.len()
    );
    trace("regexp error fuzz messages", &format!("{seen_msgs:?}"));
}

/* ===================================================================== *
 *  jsregexp.c (rows 794-808)
 * ===================================================================== */

thread_local! {
    static NEWRE_PAT: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
    static NEWRE_FLAGS: Cell<c_int> = const { Cell::new(0) };
}

/// Calls `js_newregexp` inside a protected call so the `SyntaxError` raised by
/// jsregexp.c:38 is caught by `js_pcall` instead of reaching `abort()` with
/// `trytop == 0`.  On success it leaves the regexp object on the stack and
/// returns its `js_torepr`, so a successful compile is observable too.
unsafe extern "C" fn newregexp_probe(j: JS) {
    let l = cur();
    l.js_newregexp(j, NEWRE_PAT.with(|c| c.get()), NEWRE_FLAGS.with(|c| c.get()));
    let s = l.js_torepr(j, -1);
    l.js_pushstring(j, s);
}

/// `js_newregexp(pattern, flags)` under `js_pcall`: returns
/// `(rc, thrown-or-repr, final top)`.
fn newregexp_diff(pattern: &str, flags: c_int) -> String {
    let p = libs();
    let mut got = vec![];
    for l in [&p.c, &p.rs] {
        unsafe {
            out_clear();
            let j = new_state(l, 0);
            let cp = cstr(pattern);
            NEWRE_PAT.with(|c| c.set(cp.as_ptr()));
            NEWRE_FLAGS.with(|c| c.set(flags));
            let base = l.js_gettop(j);
            l.js_newcfunction(j, Some(newregexp_probe), b"mk\0".as_ptr() as *const c_char, 0);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            let v = from_c(l.js_trystring(j, -1, b"<nostring>\0".as_ptr() as *const c_char));
            l.js_pop(j, 1);
            let top = l.js_gettop(j);
            let out = out_take();
            l.js_freestate(j);
            got.push(format!("rc={rc} v={v:?} top={base}->{top} out={out}"));
        }
    }
    assert_eq!(
        got[0], got[1],
        "js_newregexp({pattern:?}, {flags}) divergence"
    );
    trace(&format!("js_newregexp({pattern:?},{flags})"), &got[0]);
    got.pop().unwrap()
}

/// ERRORS row 794: jsregexp.c:38 turns EVERY `js_regcompx` failure into
/// `js_syntaxerror(J, "regular expression: %s", error)`, so the exact regexp.c
/// `die()` message must be embedded verbatim.
#[test]
fn t_newregexp_syntaxerror() {
    // one representative pattern per distinct regexp.c message
    let cases: &[(&str, &str)] = &[
        (r"\xZZ", "invalid escape sequence"),
        ("a{x}", "invalid quantifier"),
        ("a\\", "unterminated escape sequence"),
        (r"\y", "invalid escape character"),
        ("a{255}", "numeric overflow"),
        ("[z-a]", "invalid character class range"),
        ("[abc", "unterminated character class"),
        ("(?:)*", "infinite loop matching the empty string"),
        (r"\1", "invalid back-reference"),
        ("(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)", "too many captures"),
        ("(a", "unmatched '('"),
        ("*", "syntax error"),
        ("a)", "unmatched ')'"),
    ];
    for (pat, msg) in cases {
        let got = newregexp_diff(pat, 0);
        assert!(
            got.contains(&format!("SyntaxError: regular expression: {msg}")),
            "js_newregexp({pat:?}) should raise \
             `SyntaxError: regular expression: {msg}`, got {got}"
        );
    }
    // too many character classes / ranges and the program-size limits too
    for (pat, msg) in [
        ("\\d".repeat(129), "too many character classes"),
        ("a".repeat(20000), "program too large"),
        ("a".repeat(5000), "stack overflow"),
        ("a{254}".repeat(140), "program too large"),
    ] {
        let got = newregexp_diff(&pat, 0);
        assert!(
            got.contains(&format!("SyntaxError: regular expression: {msg}")),
            "js_newregexp(len={}) should say {msg}, got {got}",
            pat.len()
        );
    }
    // patterns that DO compile, including the `/` escaping of `escaperegexp`
    for pat in ["a", "", "a/b", "//", "\\/", "[/]", "(?:)", "\u{e9}", "a*b"] {
        newregexp_diff(pat, 0);
    }
    // flags is an `int` parameter stored into an `unsigned short` field
    // (jsi.h:365), while REG_ICASE / REG_NEWLINE are derived from the FULL int
    // first (jsregexp.c:33-34), so out-of-range values are observable twice.
    for pat in ["a", "A", "a\nb", "^a$"] {
        for flags in [
            -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 255, 65535, 65536, 65537,
            0x1_0002, c_int::MAX, c_int::MIN,
        ] {
            newregexp_diff(pat, flags);
        }
    }
}

/// ERRORS rows 795 / 799: a `/g` regexp whose `lastIndex` is past the end of
/// the subject resets it to 0 and yields `null` (`exec`) / `false` (`test`),
/// with no error.  `js_Regexp::last` is an `unsigned short` (jsi.h:366) and
/// jsrun.c:749 assigns `jsV_tointeger(J, value)` straight into it, so the
/// value read back is the low 16 bits: 70000 -> 4464 and -1 -> 65535.
#[test]
fn t_regexp_lastindex_past_end() {
    let mut src = String::from(
        "function show(re, s, li) { re.lastIndex = li; \
         var a = re.exec(s); \
         print('exec', li, re.lastIndex, a === null ? 'null' : a[0] + '@' + a.index, \
               re.lastIndex); \
         re.lastIndex = li; \
         print('test', li, re.lastIndex, re.test(s), re.lastIndex); }\n",
    );
    for li in [
        "0", "1", "2", "3", "4", "5", "100", "65535", "65536", "65537", "70000",
        "-1", "-2", "-65536", "1.9", "-1.9", "NaN", "Infinity", "-Infinity",
        "'3'", "'abc'", "null", "undefined", "true", "false", "1e21", "-1e21",
    ] {
        for (re, subj) in [
            ("/a/g", "'abc'"),
            ("/a/", "'abc'"),
            ("/a/gi", "'ABC'"),
            ("/(b)(c)/g", "'abc'"),
            ("/x/g", "'abc'"),
            ("/a*/g", "'aaa'"),
            ("/^a/gm", "'a\\na'"),
            ("/a/g", "''"),
        ] {
            src.push_str(&format!("show({re}, {subj}, {li});\n"));
        }
    }
    // the exact truncation the row calls out
    src.push_str(
        "var r = /a/g; r.lastIndex = 70000; print('trunc', r.lastIndex); \
         r.lastIndex = -1; print('trunc', r.lastIndex); \
         r.lastIndex = 65536; print('trunc', r.lastIndex); \
         r.lastIndex = 4464; print('trunc', r.lastIndex);\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS rows 796 / 800: `js_regexec` returning < 0 becomes
/// `js_error(J, "regexec failed")` -- a plain `Error`, not a SyntaxError -- in
/// both `js_RegExp_prototype_exec` (jsregexp.c:77) and `Rp_test`
/// (jsregexp.c:126).  Driven from JS through a deep `match()` recursion.
#[test]
fn t_regexp_regexec_failed() {
    with_big_stack(body_t_regexp_regexec_failed);
}

fn body_t_regexp_regexec_failed() {
    // Anchored so the scan stays O(n); see t_regexp_match_recursion_limit.
    let mut src = String::from(
        "function big(n) { var s = ''; while (s.length < n) s += 'aaaaaaaaaa'; \
         return s.substring(0, n); }\n\
         function probe(re, n) { var s = big(n); \
           try { var a = re.exec(s); print('exec', n, a === null ? 'null' : a[0].length) } \
           catch (e) { print('exec', n, e.name + ': ' + e.message) } \
           try { print('test', n, re.test(s)) } \
           catch (e) { print('test', n, e.name + ': ' + e.message) } }\n",
    );
    for n in [10, 100, 4000, 4090, 4094, 4095, 4096, 4097, 4098, 5000, 9000] {
        for re in ["/^a*b/", "/^a+b/", "/^(a)*b/", "/^a*b/g", "/^(?=a*b)/"] {
            src.push_str(&format!("probe({re}, {n});\n"));
        }
    }
    // String.prototype.replace / match / search reach the same regexec
    src.push_str(
        "var s = big(9000); \
         try { print('replace', s.replace(/^a*b/, 'X').length) } \
         catch (e) { print('replace', e.name + ': ' + e.message) } \
         try { print('match', s.match(/^a*b/)) } \
         catch (e) { print('match', e.name + ': ' + e.message) } \
         try { print('search', s.search(/^a*b/)) } \
         catch (e) { print('search', e.name + ': ' + e.message) } \
         try { print('split', s.split(/^a*b/).length) } \
         catch (e) { print('split', e.name + ': ' + e.message) }\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS rows 797 / 801: `js_regexec` returning 1 pushes `null` / `false` and,
/// for a `/g` regexp, resets `re->last` to 0 (jsregexp.c:93-96 and 134-137).
/// Also covers the `/g` advance on success and the REG_NOTBOL / `\n` handling
/// at jsregexp.c:70 and 119.
#[test]
fn t_regexp_nomatch_resets_lastindex() {
    let mut src = String::from(
        "function walk(re, s, k) { var out = []; \
         for (var i = 0; i < k; ++i) { \
           var a = re.exec(s); \
           out.push('[' + re.lastIndex + ' ' + (a === null ? 'null' : a[0] + '@' + a.index) + ']'); } \
         print(out.join(' ')); }\n\
         function walkt(re, s, k) { var out = []; \
         for (var i = 0; i < k; ++i) out.push('[' + re.lastIndex + ' ' + re.test(s) + ']'); \
         print(out.join(' ')); }\n",
    );
    for (re, subj) in [
        ("/a/g", "'abcabc'"),
        ("/a/", "'abcabc'"),
        ("/x/g", "'abcabc'"),
        ("/a*/g", "'aab'"),
        ("/^a/g", "'aa'"),
        ("/^a/gm", "'a\\na'"),
        ("/^a/m", "'a\\na'"),
        ("/$/g", "'ab'"),
        ("/(a)(b)?/g", "'ab a'"),
        ("/\\b/g", "'ab cd'"),
        ("/(?:)/g", "'abc'"),
        ("/a/g", "''"),
        ("/\\B/g", "'abc'"),
    ] {
        src.push_str(&format!("walk({re}, {subj}, 6);\nwalkt({re}, {subj}, 6);\n"));
    }
    diff_script(0, 5, &src);
}

/// ERRORS rows 798 / 807 / 808: `js_toregexp` (jsrun.c:368) raising
/// `TypeError "not a regexp"` from `Rp_test`, `Rp_toString` and `Rp_exec`.
#[test]
fn t_regexp_not_a_regexp() {
    let mut src = String::from(
        "function t(m, r) { try { print(m, RegExp.prototype[m].call(r, 'x')) } \
         catch (e) { print(m, e.name + ': ' + e.message) } }\n",
    );
    for m in ["test", "toString", "exec"] {
        for recv in [
            "{}", "[]", "1", "'s'", "true", "null", "undefined", "new Date(0)",
            "new String('x')", "new Number(1)", "Math", "JSON", "(function(){})",
            "new Error('e')", "RegExp.prototype", "/a/",
        ] {
            src.push_str(&format!("t('{m}', {recv});\n"));
        }
    }
    // and the same three through the ordinary property path
    src.push_str(
        "var o = {}; o.test = RegExp.prototype.test; \
         try { print(o.test('x')) } catch (e) { print(e.name + ': ' + e.message) }\n\
         try { print(RegExp.prototype.toString()) } \
         catch (e) { print(e.name + ': ' + e.message) }\n\
         try { print(RegExp.prototype.exec('x')) } \
         catch (e) { print(e.name + ': ' + e.message) }\n\
         try { print(RegExp.prototype.test('x')) } \
         catch (e) { print(e.name + ': ' + e.message) }\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS rows 802-806: `jsB_new_RegExp` argument / flag validation.
///   802 jsregexp.c:149 flags supplied when cloning a RegExp -> TypeError
///   803 jsregexp.c:172 a flag character other than g / i / m -> SyntaxError
///   804 / 805 / 806 jsregexp.c:175-177 a repeated g / i / m -> SyntaxError
#[test]
fn t_regexp_ctor_flags() {
    let mut src = String::from(
        "function mk(a, b) { try { var r = eval('new RegExp(' + a + ',' + b + ')'); \
           print(a, b, r.source, r.global, r.ignoreCase, r.multiline, String(r)) } \
         catch (e) { print(a, b, e.name + ': ' + e.message) } }\n",
    );
    // row 802: arg 1 is a RegExp and arg 2 is DEFINED (even `null` counts)
    for a in ["/a/", "/a/gim", "new RegExp('a')"] {
        for b in [
            "undefined", "null", "''", "'g'", "'i'", "'m'", "'gim'", "'x'", "0",
            "false",
        ] {
            src.push_str(&format!("mk(\"{a}\", \"{b}\");\n"));
        }
    }
    // rows 803-806: every flag string shape
    for b in [
        "''", "'g'", "'i'", "'m'", "'gi'", "'gm'", "'im'", "'gim'", "'mig'",
        "'gg'", "'ii'", "'mm'", "'ggg'", "'gig'", "'gimg'", "'gimi'", "'gimm'",
        "'x'", "'G'", "'I'", "'M'", "'a'", "'gx'", "'xg'", "'g i'", "'g,i'",
        "' g'", "'g '", "'0'", "'\\u00e9'", "'\\u0000g'", "'y'", "'s'", "'u'",
        "0", "1", "null", "true", "false", "undefined", "{}", "[]",
        "['g']", "['g','i']", "new String('gi')", "new String('gg')",
    ] {
        for a in ["'a'", "''", "undefined", "'(?:)'", "0", "null", "{}", "[]"] {
            src.push_str(&format!("mk(\"{a}\", \"{b}\");\n"));
        }
    }
    // the non-constructor call form dispatches to the same code (jsregexp.c:186)
    src.push_str(
        "function call(a, b) { try { var r = eval('RegExp(' + a + ',' + b + ')'); \
           print('call', a, b, r.source, String(r)) } \
         catch (e) { print('call', a, b, e.name + ': ' + e.message) } }\n",
    );
    for a in ["'a'", "/a/", "undefined", "''"] {
        for b in ["undefined", "'g'", "'x'", "'gg'", "null"] {
            src.push_str(&format!("call(\"{a}\", \"{b}\");\n"));
        }
    }
    diff_script(0, 5, &src);
}

/// Randomised flag-string fuzzing for rows 803-806 (fixed seed): the message
/// embeds `*s` with `%c`, so every rejected byte must be reproduced exactly.
#[test]
fn t_regexp_ctor_flag_fuzz() {
    let mut rng = Rng::new(0x5EED_0803);
    let alphabet: Vec<char> = "gim GIMxysu0123,.-_".chars().collect();
    let mut src = String::from(
        "function mk(b) { try { var r = new RegExp('a', b); \
           print(b, r.global, r.ignoreCase, r.multiline, String(r)) } \
         catch (e) { print(b, e.name + ': ' + e.message) } }\n",
    );
    for _ in 0..1500 {
        let n = rng.below(6) as usize;
        let s: String = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        // keep the embedded literal safe: no quotes or backslashes in alphabet
        src.push_str(&format!("mk('{s}');\n"));
    }
    diff_script(0, 5, &src);
}

/* ===================================================================== *
 *  json.c (rows 809-824)
 * ===================================================================== */

/// Escape `s` into a single-quoted JS literal, byte by byte, so an arbitrary
/// (possibly non-UTF8) fuzz input can be embedded in a script verbatim.
fn jq(s: &[u8]) -> String {
    let mut out = String::from("'");
    for &b in s {
        match b {
            b'\'' => out.push_str("\\'"),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02X}")),
        }
    }
    out.push('\'');
    out
}


/// `diff_dostring` plus a NON-VACUITY guard.
///
/// Comparing two identical failures is not a test: a typo in a generated script
/// produces the same `SyntaxError` in both libraries and the diff passes while
/// nothing was actually exercised.  So every script driven from this file must
/// (a) run to completion, (b) leave nothing on the report hook and (c) emit at
/// least `min_lines` lines of `print` / `dump` output.
fn diff_script(flags: c_int, min_lines: usize, src: &str) {
    let p = libs();
    let a = dostring(&p.c, flags, src);
    let b = dostring(&p.rs, flags, src);
    assert_eq!(a, b, "dostring divergence (flags={flags})\nsrc: {src}");
    assert_eq!(
        a.0, 0,
        "script did not run to completion (flags={flags}); output was:\n{}",
        a.1
    );
    assert!(
        !a.1.contains("[report]"),
        "script raised an uncaught error (flags={flags}):\n{}",
        a.1
    );
    // Every fragment this file feeds to `eval()` is hand written and must
    // parse; an `(eval):` prefixed SyntaxError or any ReferenceError therefore
    // means the TEST is broken, not the library, and would otherwise "pass"
    // because both libraries fail identically.
    assert!(
        !a.1.contains("(eval):"),
        "a generated eval() fragment failed to parse (flags={flags}):\n{}",
        a.1
    );
    assert!(
        !a.1.contains("ReferenceError"),
        "a generated fragment referenced an undefined name (flags={flags}):\n{}",
        a.1
    );
    let lines = a.1.lines().count();
    assert!(
        lines >= min_lines,
        "only {lines} lines of output, wanted >= {min_lines} (flags={flags}); \
         the script is probably not exercising anything:\n{}",
        a.1
    );
}

/// ERRORS rows 809-814: every `JSON.parse` rejection.
///   809 json.c:41  jsonexpect        -> "JSON: unexpected token: X (expected Y)"
///   810 json.c:70  missing `:`
///   811 json.c:75  missing `}`
///   812 json.c:88  missing `]`
///   813 json.c:67  object key is not a string token
///   814 json.c:107 token cannot start a value
#[test]
fn t_json_parse_errors() {
    let mut src = String::from(
        "function p(s) { try { print(s, '->', JSON.stringify(JSON.parse(s))) } \
         catch (e) { print(s, '->', e.name + ': ' + e.message) } }\n",
    );
    let cases: &[&str] = &[
        // row 810 -- expected ':'
        r#"{"a" 1}"#, r#"{"a"}"#, r#"{"a","b":1}"#, r#"{"a" "b"}"#, r#"{"a"]}"#,
        r#"{"a""#, r#"{"a":1,"b" 2}"#,
        // row 811 -- expected '}'
        r#"{"a":1"#, r#"{"a":1,"#, r#"{"a":1]"#, r#"{"a":1 "b":2}"#,
        r#"{"a":1,"b":2"#, r#"{"a":{"b":1}"#,
        // row 812 -- expected ']'
        "[1,2", "[1,2,", "[1,2}", "[1 2]", "[[1]", "[1,",
        // row 813 -- key is not a string token
        r#"{a:1}"#, "{1:2}", "{true:1}", "{null:1}", "{[]:1}", "{{}:1}",
        r#"{"a":1,b:2}"#, "{,}", r#"{"a":1,,}"#, "{:1}", r#"{'a':1}"#,
        // row 814 -- token cannot start a value
        "", " ", "\n", "undefined", ",", "]", "}", ":", "+1", "'x'", "a",
        "True", "NULL", "NaN", "Infinity", "-", ".", "/x/", "\\", "*",
        r#"["#, r#"{"#, r#"[,]"#, r#"[1,]"#, r#"[]]"#, r#"{}}"#, "()", "()=>1",
        // and the accepted forms, so the boundary is real
        "1", "-1", "0", "1.5", "1e3", "-0", r#""s""#, "true", "false", "null",
        "[]", "{}", "[1]", r#"{"a":1}"#, "[[[]]]", r#"{"a":{"b":[1,2]}}"#,
        " 1 ", "\t[\n1\n]\t", r#"["é"]"#, r#"["\n\t\\\"\/\b\f\r"]"#,
        // trailing junk after a complete value
        "1 2", "[] []", "{} 1", "null null", "1x", "truex",
        // strings with problems
        r#"""#, r#""abc"#, r#""\u00"#, r#""\q""#, r#""\x41""#, "\"a\nb\"",
        // numbers with problems
        "01", "1.", ".5", "1e", "1e+", "0x10", "+0", "- 1", "1..2", "--1",
    ];
    for c in cases {
        src.push_str(&format!("p({});\n", jq(c.as_bytes())));
    }
    diff_script(0, 5, &src);
}

/// Randomised `JSON.parse` source fuzzing (fixed seed): the error messages
/// embed `jsY_tokenstring(J->lookahead)`, so every rejected token name must be
/// reproduced exactly.
#[test]
fn t_json_parse_fuzz() {
    let mut rng = Rng::new(0x5EED_0809);
    let toks = [
        "{", "}", "[", "]", ":", ",", "1", "-2", "1.5", "1e3", "\"a\"", "\"\"",
        "true", "false", "null", " ", "\n", "\t", "undefined", "x", "+", "-",
        ".", "'", "\\", "\"", "0x1", "01", "NaN", "Infinity", "/*c*/", "//c\n",
    ];
    for chunk in 0..8 {
        let mut src = String::from(
            "function p(s) { try { print(JSON.stringify(JSON.parse(s))) } \
             catch (e) { print(e.name + ': ' + e.message) } }\n",
        );
        for _ in 0..600 {
            let n = 1 + rng.below(7) as usize;
            let mut s = String::new();
            for _ in 0..n {
                s.push_str(toks[rng.below(toks.len() as u32) as usize]);
            }
            src.push_str(&format!("p({});\n", jq(s.as_bytes())));
        }
        // also raw random bytes
        for _ in 0..200 {
            let b = rng.raw_bytes(10);
            src.push_str(&format!("p({});\n", jq(&b)));
        }
        trace(&format!("json fuzz chunk {chunk}"), "");
        diff_script(0, 5, &src);
    }
}

/// A `JSON.parse` reviver that is missing, not callable, or throws.
/// json.c:165 gates the whole `jsonrevive` walk on `js_iscallable(J, 2)`.
#[test]
fn t_json_parse_reviver() {
    let mut src = String::from(
        "function p(s, r) { try { print(s, r, '->', \
           JSON.stringify(JSON.parse(s, eval(r)))) } \
         catch (e) { print(s, r, '->', e.name + ': ' + e.message) } }\n",
    );
    for s in [
        r#"{"a":1,"b":2}"#, "[1,2,3]", "1", r#""s""#, "null", "[]", "{}",
        r#"{"a":{"b":[1,{"c":2}]}}"#, r#"[[1],[2]]"#, r#"{"":1}"#,
    ] {
        for r in [
            "undefined", "null", "0", "1", "'x'", "{}", "[]", "true",
            "(function(k,v){return v})",
            "(function(k,v){return undefined})",
            "(function(k,v){return k === 'a' ? undefined : v})",
            "(function(k,v){throw new Error('boom')})",
            "(function(k,v){throw 'plain'})",
            "(function(k,v){return typeof v === 'number' ? v * 2 : v})",
            "(function(k,v){delete this[k]; return v})",
            "(function(k,v){return this})",
            "(function(){return arguments.length})",
            "Math.max",
            "new Number(1)",
        ] {
            src.push_str(&format!("p({}, {});\n", jq(s.as_bytes()), jq(r.as_bytes())));
        }
    }
    diff_script(0, 5, &src);
}

/// ERRORS row 815: json.c:246 `filterprop` returns 0 for a key that is not in
/// the replacer array, and only string / number / String-object / Number-object
/// entries are even considered (json.c:241-242).
#[test]
fn t_json_replacer_array() {
    let mut src = String::from(
        "function s(v, r) { try { print(r, '->', JSON.stringify(eval(v), eval(r))) } \
         catch (e) { print(r, '->', e.name + ': ' + e.message) } }\n",
    );
    let values = [
        r#"({a:1,b:2,c:3})"#,
        r#"({a:{b:1,c:2},d:3})"#,
        r#"([{a:1,b:2}])"#,
        r#"({'1':1,'2':2})"#,
        r#"({'':0,a:1})"#,
        r#"({a:undefined,b:1})"#,
    ];
    let replacers = [
        "undefined", "null", "[]", "['a']", "['b']", "['a','b']", "['z']",
        "[1]", "['1']", "[1,2]", "[new String('a')]", "[new Number(1)]",
        "[true]", "[null]", "[undefined]", "[{}]", "[[]]", "[function(){}]",
        "['a',null,'b']", "[{toString:function(){return 'a'}}]",
        "['a','a','a']", "({length:1,0:'a'})", "['']",
        // a replacer array whose length is a lie
        "(function(){var a=['a','b']; a.length=1; return a})()",
        "(function(){var a=['a']; a.length=5; return a})()",
        "(function(){var a=[]; a.length=-1; return a})()",
        // a callable replacer takes the OTHER json.c:331 path entirely
        "(function(k,v){return v})",
        "(function(k,v){return k === 'b' ? undefined : v})",
        "(function(k,v){throw new Error('rep')})",
        // a non-callable, non-array replacer is ignored
        "0", "'x'", "true", "({})", "Math",
    ];
    for v in values {
        for r in replacers {
            src.push_str(&format!("s({}, {});\n", jq(v.as_bytes()), jq(r.as_bytes())));
        }
    }
    diff_script(0, 5, &src);
}

/// ERRORS rows 816 / 817: json.c:261 and json.c:297
/// `js_typeerror(J, "cyclic object value")`.  The scan starts at stack slot 4
/// and stops at `top - 1`, so only the ancestors currently being formatted
/// count -- a repeated but non-ancestral object is fine.
#[test]
fn t_json_cyclic() {
    let mut src = String::from(
        "function s(v) { try { print(v, '->', JSON.stringify(eval(v))) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    let cases = [
        // row 816
        "(function(){var a={}; a.a=a; return a})()",
        "(function(){var a={}; a.x={y:a}; return a})()",
        "(function(){var a={},b={}; a.b=b; b.a=a; return a})()",
        "(function(){var a={}; a.p=a; return {q:a}})()",
        // row 817
        "(function(){var a=[]; a[0]=a; return a})()",
        "(function(){var a=[]; a.push([a]); return a})()",
        "(function(){var a=[],b=[]; a[0]=b; b[0]=a; return a})()",
        "(function(){var a=[]; a[0]=a; return {k:a}})()",
        // mixed
        "(function(){var a={},b=[]; a.b=b; b[0]=a; return a})()",
        "(function(){var a=[]; a[0]={x:a}; return a})()",
        // NOT cyclic: the same object twice, side by side
        "(function(){var a={x:1}; return {p:a,q:a}})()",
        "(function(){var a=[1]; return [a,a]})()",
        "(function(){var a={x:1}; return [a,{y:a}]})()",
        // deep but finite
        "(function(){var a={}; var t=a; for (var i=0;i<20;++i){t.n={}; t=t.n} return a})()",
        // cyclic through a toJSON that returns the holder
        "(function(){var a={}; a.toJSON=function(){return {z:a}}; return a})()",
        // cyclic behind a replacer function
        "(function(){var a={}; a.a=a; return a})()",
    ];
    for c in cases {
        src.push_str(&format!("s({});\n", jq(c.as_bytes())));
    }
    // with an indent gap, and with a replacer, the same sites must fire
    src.push_str(
        "function s2(v, extra) { try { print(extra, '->', \
           eval('JSON.stringify(' + v + ',' + extra + ')')) } \
         catch (e) { print(extra, '->', e.name + ': ' + e.message) } }\n",
    );
    for extra in ["null,2", "null,'  '", "function(k,x){return x},2", "['a'],4"] {
        src.push_str(&format!(
            "s2({}, {});\n",
            jq(b"(function(){var a={}; a.a=a; return a})()"),
            jq(extra.as_bytes())
        ));
        src.push_str(&format!(
            "s2({}, {});\n",
            jq(b"(function(){var a=[]; a[0]=a; return a})()"),
            jq(extra.as_bytes())
        ));
    }
    diff_script(0, 5, &src);
}

/// ERRORS rows 818-821: `fmtvalue` returning 0.
///   818 json.c:359 the value is undefined or callable
///   819 json.c:277 `fmtobject` rewinds the buffer to `save`, dropping the
///       property AND the comma / indent it had already written
///   820 json.c:305 `fmtarray` writes `"null"` in that slot instead
///   821 json.c:403 a top-level undefined / function pushes `undefined`
#[test]
fn t_json_skipped_values() {
    let mut src = String::from(
        "function s(v) { try { var r = JSON.stringify(eval(v)); \
           print(v, '->', typeof r, r) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    let cases = [
        // row 821: nothing at all comes out
        "undefined", "(function(){})", "Math.max", "(void 0)",
        // row 818/819: object properties are dropped, commas included
        "({a:undefined})",
        "({a:undefined,b:1})",
        "({a:1,b:undefined})",
        "({a:1,b:undefined,c:2})",
        "({a:undefined,b:undefined})",
        "({a:function(){},b:1})",
        "({a:1,b:function(){},c:3})",
        "({a:Math.max})",
        "({a:1,b:undefined,c:function(){},d:2})",
        // row 820: array holes become "null"
        "[undefined]",
        "[1,undefined,3]",
        "[function(){},1]",
        "[undefined,undefined]",
        "(function(){var a=[1]; a[2]=3; return a})()",
        "(function(){var a=[]; a.length=3; return a})()",
        "(function(){var a=[1,2,3]; delete a[1]; return a})()",
        "[,,]",
        "[1,,3]",
        // values that DO print
        "null", "0", "-0", "1", "1.5", "NaN", "Infinity", "-Infinity",
        "''", "'a'", "true", "false", "[]", "({})", "[[]]", "({a:{}})",
        "new Number(3)", "new String('s')", "new Boolean(true)",
        "new Date(0)", "/re/g", "new Error('e')",
        // a toJSON that returns undefined / a function
        "({toJSON:function(){return undefined}})",
        "({toJSON:function(){return function(){}}})",
        "({a:{toJSON:function(){return undefined}},b:1})",
        "[{toJSON:function(){return undefined}}]",
        "({toJSON:1})",
        "({toJSON:function(){throw new Error('tj')}})",
    ];
    for c in cases {
        src.push_str(&format!("s({});\n", jq(c.as_bytes())));
    }
    // the same, with an indent, so the dropped fmtindent is visible too
    src.push_str(
        "function si(v) { try { print(v, '->', JSON.stringify(eval(v), null, 2)) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    for c in cases {
        src.push_str(&format!("si({});\n", jq(c.as_bytes())));
    }
    diff_script(0, 5, &src);
}

/// ERRORS rows 822-824: the `space` argument.
///   822 json.c:380 numeric `space` < 0 clamps to 0 (no indent at all)
///   823 json.c:381 numeric `space` > 10 clamps to 10
///   824 json.c:388 a string `space` longer than 10 is truncated to 10 bytes
/// Note json.c:384 / 391 only install a gap when `n > 0`, so `space == 0`,
/// `space == ""` and a negative space are all indistinguishable from `null`.
#[test]
fn t_json_space() {
    let mut src = String::from(
        "function s(sp) { try { print(sp, '->', \
           JSON.stringify({a:[1,{b:2}],c:'x'}, null, eval(sp))) } \
         catch (e) { print(sp, '->', e.name + ': ' + e.message) } }\n",
    );
    for sp in [
        "undefined", "null", "0", "-0", "1", "2", "3", "9", "10", "11", "12",
        "100", "-1", "-2", "-10", "-11", "-100", "1.9", "-1.9", "10.9", "NaN",
        "Infinity", "-Infinity", "1e21", "-1e21", "2147483647", "-2147483648",
        "4294967296", "new Number(4)", "new Number(-1)", "new Number(99)",
        "''", "' '", "'  '", "'\\t'", "'ab'", "'0123456789'", "'0123456789a'",
        "'0123456789abcdef'", "'\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9\\u00e9'",
        "new String('----------XXXX')", "new String('')",
        "true", "false", "[]", "({})", "[1]", "'x'", "(function(){})",
    ] {
        src.push_str(&format!("s({});\n", jq(sp.as_bytes())));
    }
    // nested / empty containers with a gap: json.c:284 and 307 only emit the
    // closing fmtindent when at least one member was written
    src.push_str(
        "function s2(v) { print(v, '->', JSON.stringify(eval(v), null, 3)) }\n",
    );
    for v in [
        "({})", "[]", "({a:{}})", "([[]])", "({a:undefined})", "[undefined]",
        "({a:1})", "[1]", "({a:[]})", "([{}])", "({a:{b:{c:{}}}})",
    ] {
        src.push_str(&format!("s2({});\n", jq(v.as_bytes())));
    }
    diff_script(0, 5, &src);
}

/// Randomised `space` fuzzing (fixed seed) over the whole clamp range plus
/// arbitrary doubles, since json.c:379 goes through `js_tointeger`.
#[test]
fn t_json_space_fuzz() {
    let mut rng = Rng::new(0x5EED_0822);
    let mut src = String::from(
        "function s(sp) { try { print(sp, '->', \
           JSON.stringify({a:[1,2],b:{c:3}}, null, sp)) } \
         catch (e) { print(sp, '->', e.name + ': ' + e.message) } }\n",
    );
    for _ in 0..800 {
        let v = match rng.below(4) {
            0 => rng.range(-20, 20) as f64,
            1 => rng.range(-1_000_000, 1_000_000) as f64,
            2 => rng.f64_sane(),
            _ => rng.range(-12, 13) as f64 + 0.5,
        };
        if v.is_nan() {
            src.push_str("s(NaN);\n");
        } else if v.is_infinite() {
            src.push_str(if v > 0.0 { "s(Infinity);\n" } else { "s(-Infinity);\n" });
        } else {
            src.push_str(&format!("s({v:e});\n"));
        }
    }
    for _ in 0..400 {
        let n = rng.below(16) as usize;
        let s: Vec<u8> = (0..n).map(|_| b'a' + rng.below(26) as u8).collect();
        src.push_str(&format!("s({});\n", jq(&s)));
    }
    diff_script(0, 5, &src);
}

/* ===================================================================== *
 *  jsdate.c (rows 825-879)
 * ===================================================================== */

/// Run one script against both libraries with TZ already pinned.
fn diff_date(src: &str) {
    warm_dates();
    diff_script(0, 5, src);
}

/// ERRORS row 825: jsdate.c:214 `im < 0 || im >= 12` in `MakeDay`.
///
/// For every FINITE `m`, `pmod(m, 12)` (jsdate.c:61) lands in `[0, 12)`, so the
/// guard can only fire when `m` is NaN or +/-Inf: `fmod` then returns NaN and
/// `(int)NAN` is taken.  That cast is not defined by ISO C, but it is fully
/// determined by the target: gcc on x86-64 emits `cvttsd2si`, whose "integer
/// indefinite" result is `INT_MIN`, which is `< 0` and returns NAN.  The Rust
/// translation reproduces exactly that through `jsi.rs::d2i`, and both
/// libraries run in this same process on this same target, so the comparison is
/// meaningful rather than accidental.
#[test]
fn t_date_makeday_nan() {
    let mut src = String::from(
        "function u(a) { try { print(a, '->', eval('Date.UTC(' + a + ')')) } \
         catch (e) { print(a, '->', e.name + ': ' + e.message) } }\n\
         function d(a) { try { print(a, '->', \
           eval('new Date(' + a + ').getTime()')) } \
         catch (e) { print(a, '->', e.name + ': ' + e.message) } }\n",
    );
    for a in [
        // month NaN / Inf -> pmod -> NaN -> (int)NaN
        "1970,NaN", "1970,Infinity", "1970,-Infinity", "1970,'x'",
        "1970,undefined", "1970,{}", "1970,[]",
        "1970,NaN,1", "1970,Infinity,1,0,0,0,0",
        // year NaN as well
        "NaN,0", "Infinity,0", "-Infinity,0", "NaN,NaN",
        // finite but extreme months: pmod keeps them in range, so MakeDay
        // returns a number and TimeClip does the rejecting
        "1970,-1", "1970,12", "1970,13", "1970,-13", "1970,1e9", "1970,-1e9",
        "1970,11.9", "1970,-0.5", "1970,2147483648", "1970,-2147483649",
        "1970,1e300", "1970,-1e300", "1970,0.5", "1970,-0",
        // date / hour / min / sec / ms NaN
        "1970,0,NaN", "1970,0,1,NaN", "1970,0,1,0,NaN", "1970,0,1,0,0,NaN",
        "1970,0,1,0,0,0,NaN", "1970,0,Infinity", "1970,0,1,Infinity",
    ] {
        src.push_str(&format!("u({});\n", jq(a.as_bytes())));
        src.push_str(&format!("d({});\n", jq(a.as_bytes())));
    }
    diff_date(&src);
}

/// ERRORS rows 826 / 827 / 853 / 856: `TimeClip` (jsdate.c:228).
///   826 `!isfinite(t)` -> NAN
///   827 `fabs(t) > 8.64e15` -> NAN
///   853 every `Dp_set*` stores `TimeClip(t)` and pushes it
///   856 `new Date(n)` with a non-finite or out-of-range n
#[test]
fn t_date_timeclip() {
    let mut src = String::from(
        "function n(v) { try { print(v, '->', new Date(eval(v)).getTime()) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n\
         function st(v) { try { var d = new Date(0); \
           print(v, '->', d.setTime(eval(v)), d.getTime()) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    for v in [
        "0", "-0", "1", "-1", "8639999999999999", "8640000000000000",
        "8640000000000001", "-8639999999999999", "-8640000000000000",
        "-8640000000000001", "8.64e15", "-8.64e15", "8.64e15+1", "-8.64e15-1",
        "8.6400000000001e15", "1e16", "-1e16", "NaN", "Infinity", "-Infinity",
        "1e300", "-1e300", "1.5", "-1.5", "0.4", "-0.4", "0.5", "-0.5",
        "'0'", "'abc'", "''", "null", "undefined", "true", "false", "[]",
        "({})", "new Number(5)", "new String('7')", "new Date(0)",
        "1/0", "-1/0", "0/0", "Number.MAX_VALUE", "Number.MIN_VALUE",
    ] {
        src.push_str(&format!("n({});\n", jq(v.as_bytes())));
        src.push_str(&format!("st({});\n", jq(v.as_bytes())));
    }
    // row 853: every setter, on a valid date and on an already-NaN date
    src.push_str(
        "var setters = ['setTime','setMilliseconds','setUTCMilliseconds',\
         'setSeconds','setUTCSeconds','setMinutes','setUTCMinutes','setHours',\
         'setUTCHours','setDate','setUTCDate','setMonth','setUTCMonth',\
         'setFullYear','setUTCFullYear'];\n\
         function sweep(base, arg) { for (var i = 0; i < setters.length; ++i) { \
           var d = new Date(eval(base)); \
           try { var r = d[setters[i]](eval(arg)); \
             print(base, setters[i], arg, r, d.getTime(), String(d)) } \
           catch (e) { print(base, setters[i], arg, e.name + ': ' + e.message) } } }\n",
    );
    for base in ["0", "NaN", "8.64e15", "-8.64e15", "1e12", "'x'"] {
        for arg in [
            "0", "1", "-1", "NaN", "Infinity", "-Infinity", "1e300", "1e20",
            "-1e20", "'x'", "undefined", "null", "1.5", "-1.5", "999999",
            "8.64e15", "2147483648",
        ] {
            src.push_str(&format!(
                "sweep({}, {});\n",
                jq(base.as_bytes()),
                jq(arg.as_bytes())
            ));
        }
    }
    diff_date(&src);
}

/// ERRORS rows 828-847 and 854 / 855: every `parseDateTime` rejection, reached
/// through both `Date.parse` (jsdate.c:381, row 854) and `new Date(str)`
/// (jsdate.c:423, row 855).  Row 828 (`toint` seeing a non-digit, including the
/// terminating NUL) is the mechanism behind all the "not N digits" rows.
#[test]
fn t_date_parse_failures() {
    let mut src = String::from(
        "function p(s) { print(s, '|', Date.parse(s), '|', \
           new Date(s).getTime(), '|', String(new Date(s))) }\n",
    );
    let cases: &[&str] = &[
        // row 829: the first 4 characters are not digits
        "", "a", "ab", "abc", "abcd", "19a0", "197", "-1970", "+1970", " 1970",
        "197 ", "1970", "0000", "9999", "1e70",
        // row 830: `-` then MM is not 2 digits
        "1970-", "1970-1", "1970-1-01", "1970-a1", "1970-1a", "1970- 1",
        // row 831: second `-` then DD is not 2 digits
        "1970-01-", "1970-01-1", "1970-01-a1", "1970-01-1a", "1970-01-011",
        // row 832: after `T`, HH is not 2 digits
        "1970-01-01T", "1970-01-01Tx", "1970-01-01T1", "1970-01-01Ta1",
        // row 833: after THH the next char is not `:`
        "1970-01-01T12", "1970-01-01T12x", "1970-01-01T12.", "1970-01-01T12-",
        // row 834: after `T HH :`, mm is not 2 digits
        "1970-01-01T12:", "1970-01-01T12:0", "1970-01-01T12:a0",
        // row 835: after seconds `:`, ss is not 2 digits
        "1970-01-01T12:00:", "1970-01-01T12:00:0", "1970-01-01T12:00:a0",
        // row 836: after `.`, sss is not exactly 3 digits
        "1970-01-01T12:00:00.", "1970-01-01T12:00:00.5",
        "1970-01-01T12:00:00.50", "1970-01-01T12:00:00.a00",
        // row 837: timezone sign then HH is not 2 digits
        "1970-01-01T00:00+", "1970-01-01T00:00+1", "1970-01-01T00:00-1",
        "1970-01-01T00:00+a1", "1970-01-01T00:00-",
        // row 838: timezone `:` then mm is not 2 digits
        "1970-01-01T00:00+01:", "1970-01-01T00:00+01:0",
        "1970-01-01T00:00-01:0", "1970-01-01T00:00+01:a0",
        // row 839: tzh > 23 || tzm > 59
        "1970-01-01T00:00+24:00", "1970-01-01T00:00+23:59",
        "1970-01-01T00:00+23:60", "1970-01-01T00:00+99:00",
        "1970-01-01T00:00-24:00", "1970-01-01T00:00+00:60",
        "1970-01-01T00:00+24", "1970-01-01T00:00+23",
        // row 840: trailing unconsumed characters
        "1970-01-01 junk", "1970-01-01x", "1970-01-01T00:00:00.000Zx",
        "1970-01-01T00:00Z ", "1970 ", "1970-01-01T00:00+01:00x", "1970x",
        "1970-01-01T00:00ZZ", "1970-01-01Z",
        // row 841: month out of range
        "1970-13-01", "1970-00-01", "1970-99-01", "1970-12-01", "1970-01-01",
        // row 842: day out of range
        "1970-01-32", "1970-01-00", "1970-01-99", "1970-01-31",
        // row 843: hour out of range
        "1970-01-01T25:00", "1970-01-01T99:00", "1970-01-01T24:00",
        "1970-01-01T23:00",
        // row 844: minute out of range
        "1970-01-01T00:60", "1970-01-01T00:99", "1970-01-01T00:59",
        // row 845: second out of range
        "1970-01-01T00:00:60", "1970-01-01T00:00:99", "1970-01-01T00:00:59",
        // row 846: millisecond out of range (only 3 digits fit, so <= 999)
        "1970-01-01T00:00:00.999", "1970-01-01T00:00:00.000",
        // row 847: H == 24 with non-zero M / S / ms
        "1970-01-01T24:01", "1970-01-01T24:00:01", "1970-01-01T24:00:00.001",
        "1970-01-01T24:00:00.000", "1970-01-01T24:00:00",
        // valid forms, so the boundary is real
        "1970-01-01T00:00:00.000Z", "1970-01-01T00:00:00Z",
        "1970-01-01T00:00Z", "1970-01-01T00:00", "1970-01-01", "1970-01",
        "2000-02-29", "1900-02-28", "2000-12-31T23:59:59.999+23:59",
        "2000-12-31T23:59:59.999-23:59", "0000-01-01", "9999-12-31",
        // non-string arguments coerce first
        "0", "1", "true", "null",
    ];
    for c in cases {
        src.push_str(&format!("p({});\n", jq(c.as_bytes())));
    }
    // row 854 / 855 with non-string arguments
    src.push_str(
        "function q(v) { try { print(v, '|', Date.parse(eval(v)), '|', \
           new Date(eval(v)).getTime()) } \
         catch (e) { print(v, '|', e.name + ': ' + e.message) } }\n",
    );
    for v in [
        "undefined", "null", "0", "1", "true", "false", "[]", "({})", "'x'",
        "new String('1970-01-01')", "new Date(0)", "new Number(5)",
        "({toString:function(){return '1970-01-01'}})",
        "({valueOf:function(){return 5}})",
        "({toString:function(){throw new Error('ts')}})",
        "[1970]", "[1970,1]",
    ] {
        src.push_str(&format!("q({});\n", jq(v.as_bytes())));
    }
    diff_date(&src);
}

/// Randomised ISO-8601-ish `Date.parse` fuzzing (fixed seed).
#[test]
fn t_date_parse_fuzz() {
    warm_dates();
    let mut rng = Rng::new(0x5EED_0829);
    let alphabet: Vec<u8> = b"0123456789-T:.Z+ xa".to_vec();
    for chunk in 0..6 {
        let mut src = String::from("function p(s) { print(s, Date.parse(s)) }\n");
        for _ in 0..700 {
            let n = 1 + rng.below(26) as usize;
            let s: Vec<u8> = (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
                .collect();
            src.push_str(&format!("p({});\n", jq(&s)));
        }
        // structured near-valid stamps
        for _ in 0..400 {
            let s = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}{}",
                rng.range(0, 10000),
                rng.range(0, 20),
                rng.range(0, 40),
                rng.range(0, 30),
                rng.range(0, 70),
                rng.range(0, 70),
                rng.range(0, 1000),
                ["Z", "", "+01:00", "-05:30", "+24:00", "+00:60", "x"]
                    [rng.below(7) as usize]
            );
            src.push_str(&format!("p({});\n", jq(s.as_bytes())));
        }
        trace(&format!("date parse fuzz chunk {chunk}"), "");
        diff_script(0, 5, &src);
    }
}

/// ERRORS rows 848 / 849 / 850: `fmtdate`, `fmttime` and `fmtdatetime` return
/// the literal string `"Invalid Date"` when `!isfinite(t)`.  Note all three
/// compute their y/m/d/H/M/S fields BEFORE the check (jsdate.c:321-323,
/// 332-337), so the NaN path also exercises `YearFromTime(NaN)` etc.
#[test]
fn t_date_invalid_format() {
    let mut src = String::from(
        "var fmts = ['toString','toDateString','toTimeString','toLocaleString',\
         'toLocaleDateString','toLocaleTimeString','toUTCString','toISOString',\
         'toJSON','valueOf','getTime'];\n\
         function f(v) { var d = new Date(eval(v)); \
           for (var i = 0; i < fmts.length; ++i) { \
             try { print(v, fmts[i], '->', d[fmts[i]]()) } \
             catch (e) { print(v, fmts[i], '->', e.name + ': ' + e.message) } } \
           try { print(v, 'String', '->', String(d)) } \
           catch (e) { print(v, 'String', '->', e.name + ': ' + e.message) } \
           try { print(v, 'concat', '->', '' + d) } \
           catch (e) { print(v, 'concat', '->', e.name + ': ' + e.message) } }\n",
    );
    for v in [
        "NaN", "Infinity", "-Infinity", "'x'", "1e300", "8.64e15+1",
        "-8.64e15-1", "0", "-0", "1", "-1", "8.64e15", "-8.64e15",
        "1000000000000", "-1000000000000", "86399999", "-1",
    ] {
        src.push_str(&format!("f({});\n", jq(v.as_bytes())));
    }
    diff_date(&src);
}

/// ERRORS rows 851 / 852: `js_todate` (jsdate.c:366) and `js_setdate`
/// (jsdate.c:374) raising `TypeError "not a date"` for a `this` whose
/// `type != JS_CDATE`.
#[test]
fn t_date_not_a_date() {
    let mut src = String::from(
        "var getters = ['valueOf','getTime','toString','toDateString',\
         'toTimeString','toLocaleString','toUTCString','toISOString','toJSON',\
         'getFullYear','getMonth','getDate','getDay','getHours','getMinutes',\
         'getSeconds','getMilliseconds','getUTCFullYear','getUTCMonth',\
         'getUTCDate','getUTCDay','getUTCHours','getUTCMinutes','getUTCSeconds',\
         'getUTCMilliseconds','getTimezoneOffset'];\n\
         var setters = ['setTime','setMilliseconds','setUTCMilliseconds',\
         'setSeconds','setUTCSeconds','setMinutes','setUTCMinutes','setHours',\
         'setUTCHours','setDate','setUTCDate','setMonth','setUTCMonth',\
         'setFullYear','setUTCFullYear'];\n\
         function g(r) { var recv = eval(r); \
           for (var i = 0; i < getters.length; ++i) { \
             try { print(r, getters[i], '->', \
               Date.prototype[getters[i]].call(recv)) } \
             catch (e) { print(r, getters[i], '->', e.name + ': ' + e.message) } } \
           for (i = 0; i < setters.length; ++i) { \
             try { print(r, setters[i], '->', \
               Date.prototype[setters[i]].call(recv, 0)) } \
             catch (e) { print(r, setters[i], '->', e.name + ': ' + e.message) } } }\n",
    );
    for r in [
        "({})", "[]", "1", "'s'", "true", "false", "new Number(0)",
        "new String('x')", "new Boolean(true)", "/re/", "Math", "JSON",
        "(function(){})", "new Error('e')", "Date.prototype", "Date",
        "new Date(0)", "new Date(NaN)",
    ] {
        src.push_str(&format!("g({});\n", jq(r.as_bytes())));
    }
    // `null` / `undefined` as `this` go through js_toobject first
    src.push_str(
        "try { print(Date.prototype.getTime.call(null)) } \
         catch (e) { print('null', e.name + ': ' + e.message) }\n\
         try { print(Date.prototype.getTime.call(undefined)) } \
         catch (e) { print('undefined', e.name + ': ' + e.message) }\n\
         try { print(Date.prototype.setTime.call(null, 0)) } \
         catch (e) { print('null-set', e.name + ': ' + e.message) }\n",
    );
    diff_date(&src);
}

/// ERRORS rows 857 / 858: `jsB_new_Date` with component arguments
/// (jsdate.c:437 `TimeClip(UTC(t))`) and `D_UTC` (jsdate.c:397 `TimeClip(t)`).
#[test]
fn t_date_utc_and_components() {
    let mut src = String::from(
        "function u(a) { try { print('UTC', a, '->', eval('Date.UTC(' + a + ')')) } \
         catch (e) { print('UTC', a, '->', e.name + ': ' + e.message) } }\n\
         function c(a) { try { var d = eval('new Date(' + a + ')'); \
           print('new', a, '->', d.getTime(), d.toISOString ? '' : '', \
                 String(d)) } \
         catch (e) { print('new', a, '->', e.name + ': ' + e.message) } }\n",
    );
    for a in [
        // the y < 100 -> y += 1900 fixup (jsdate.c:389 / 429)
        "0,0", "99,0", "100,0", "-1,0", "1899,0", "1900,0", "70,0", "69,0",
        "99.9,0", "-0,0",
        // out-of-range components
        "1970,0,0", "1970,0,-1", "1970,0,32", "1970,0,1,24", "1970,0,1,25",
        "1970,0,1,0,60", "1970,0,1,0,0,60", "1970,0,1,0,0,0,1000",
        "1970,-1,1", "1970,12,1", "1970,24,1", "275760,8,13", "275760,8,14",
        "-271821,3,20", "-271821,3,19", "1e9,0", "-1e9,0",
        // the arity switch: 1 / 2 / 3+ arguments.  `new Date()` with NO
        // arguments takes the jsdate.c:419 `Now()` branch, whose value is the
        // wall clock and therefore differs between the two runs; it is checked
        // separately below without comparing the timestamp itself.
        "0", "1970,0", "1970,0,1", "1970,0,1,0", "1970,0,1,0,0",
        "1970,0,1,0,0,0", "1970,0,1,0,0,0,0", "1970,0,1,0,0,0,0,0",
        "1970,0,1,0,0,0,0,0,0",
        // non-numeric components
        "'a','b'", "1970,'x'", "1970,0,'x'", "null,null", "undefined,undefined",
        "{},{}", "[],[]", "true,false", "1970,0,1,'x','y','z','w'",
    ] {
        src.push_str(&format!("u({});\n", jq(a.as_bytes())));
        src.push_str(&format!("c({});\n", jq(a.as_bytes())));
    }
    // `Date.UTC()` with no arguments at all: y and m are both js_tonumber of
    // undefined = NaN.  (Date.UTC never reads the clock, so this IS comparable.)
    src.push_str("u('');\nprint(Date.UTC());\nprint(Date.UTC(1970));\n");
    // `new Date()` / `Date()` / `Date.now()` read the wall clock, so only their
    // SHAPE can be compared, not their value.
    src.push_str(
        "var z = new Date(); \
         print('now', typeof z.getTime(), isFinite(z.getTime()), \
               z.getTime() === Math.floor(z.getTime()), \
               String(z).length === String(new Date(0)).length, \
               typeof Date(), typeof Date.now(), \
               Date.now() === Math.floor(Date.now()));\n",
    );
    diff_date(&src);
}

/// ERRORS row 859: jsdate.c:485 `js_rangeerror(J, "invalid date")` from
/// `Dp_toISOString` when `!isfinite(t)`.  Note it is a RangeError, not a
/// TypeError, and it is raised for +/-Infinity as well as NaN even though
/// `TimeClip` means a Date can only ever hold NaN or a finite value.
#[test]
fn t_date_toisostring_invalid() {
    let mut src = String::from(
        "function iso(v) { var d = new Date(eval(v)); \
           try { print(v, '->', d.toISOString()) } \
           catch (e) { print(v, '->', e.name + ': ' + e.message) } \
           try { print(v, 'json ->', JSON.stringify(d)) } \
           catch (e) { print(v, 'json ->', e.name + ': ' + e.message) } \
           try { print(v, 'toJSON ->', d.toJSON()) } \
           catch (e) { print(v, 'toJSON ->', e.name + ': ' + e.message) } }\n",
    );
    for v in [
        "NaN", "Infinity", "-Infinity", "'x'", "'1970-13-01'", "8.64e15+1",
        "0", "-0", "1", "-1", "8.64e15", "-8.64e15", "86400000",
    ] {
        src.push_str(&format!("iso({});\n", jq(v.as_bytes())));
    }
    // toISOString via call / apply on a non-date and on a NaN date
    src.push_str(
        "try { print(Date.prototype.toISOString.call({})) } \
         catch (e) { print('obj', e.name + ': ' + e.message) }\n\
         try { print(Date.prototype.toISOString.apply(new Date(NaN))) } \
         catch (e) { print('nan', e.name + ': ' + e.message) }\n",
    );
    diff_date(&src);
}

/// ERRORS rows 860-876: every getter pushes `NAN` when `isnan(t)`.  The
/// `LocalTime()`-based getters (rows 860-867, 876) and the UTC ones (868-875)
/// are listed separately so a divergence names the exact row.
#[test]
fn t_date_getters_nan() {
    let mut src = String::from(
        "var local = ['getFullYear','getMonth','getDate','getDay','getHours',\
         'getMinutes','getSeconds','getMilliseconds','getTimezoneOffset'];\n\
         var utc = ['getUTCFullYear','getUTCMonth','getUTCDate','getUTCDay',\
         'getUTCHours','getUTCMinutes','getUTCSeconds','getUTCMilliseconds'];\n\
         function sweep(v) { var d = new Date(eval(v)); \
           for (var i = 0; i < local.length; ++i) \
             dump(v, local[i], d[local[i]]()); \
           for (i = 0; i < utc.length; ++i) \
             dump(v, utc[i], d[utc[i]]()); \
           dump(v, 'valueOf', d.valueOf()); \
           dump(v, 'getTime', d.getTime()); }\n",
    );
    for v in [
        "NaN", "'x'", "Infinity", "-Infinity", "8.64e15+1", "-8.64e15-1",
        "0", "-0", "1", "-1", "86399999", "86400000", "-86400000",
        "8.64e15", "-8.64e15", "1000000000000", "-1000000000000",
        "978307200000", "951782400000",
    ] {
        src.push_str(&format!("sweep({});\n", jq(v.as_bytes())));
    }
    diff_date(&src);
}

/// ERRORS row 877: jsdate.c:748 `Dp_setUTCHours` defaults its MINUTES argument
/// from `HourFromTime(t)` instead of `MinFromTime(t)` -- an upstream C bug that
/// must be replicated verbatim.  `Dp_setHours` (jsdate.c:681) uses the correct
/// `MinFromTime`, so the two differ whenever hour != minute.
#[test]
fn t_date_setutchours_bug() {
    let mut src = String::from(
        "function cmp(t, h) { \
           var a = new Date(t); var ra = a.setUTCHours(h); \
           var b = new Date(t); var rb = b.setHours(h); \
           print(t, h, '| utc', ra, a.toISOString ? '' : '', \
                 a.getUTCHours(), a.getUTCMinutes(), a.getUTCSeconds(), \
                 a.getUTCMilliseconds(), \
                 '| loc', rb, b.getUTCHours(), b.getUTCMinutes(), \
                 b.getUTCSeconds(), b.getUTCMilliseconds()); }\n",
    );
    // pick base times whose UTC hour differs from its UTC minute
    for t in [
        "0", "Date.UTC(2000,0,1,5,30,7,8)", "Date.UTC(2000,0,1,0,0,0,0)",
        "Date.UTC(2000,0,1,23,59,59,999)", "Date.UTC(2000,0,1,1,2,3,4)",
        "Date.UTC(1970,0,1,12,34,56,789)", "NaN", "-1", "8.64e15",
    ] {
        for h in ["0", "1", "5", "12", "23", "24", "-1", "NaN", "Infinity", "99"] {
            src.push_str(&format!("cmp({t}, {h});\n"));
        }
    }
    // and with the minutes argument EXPLICITLY supplied, the bug is invisible
    src.push_str(
        "function exp(t, h, m) { var d = new Date(t); d.setUTCHours(h, m); \
           print('exp', t, h, m, d.getUTCHours(), d.getUTCMinutes()); }\n\
         exp(Date.UTC(2000,0,1,5,30), 9, 0);\n\
         exp(Date.UTC(2000,0,1,5,30), 9, 30);\n\
         exp(Date.UTC(2000,0,1,5,30), 9, 59);\n\
         exp(Date.UTC(2000,0,1,5,30), 9, undefined);\n",
    );
    // the same default-argument shape in every other setter, for contrast
    src.push_str(
        "var partial = [['setSeconds',1],['setUTCSeconds',1],['setMinutes',1],\
         ['setUTCMinutes',1],['setHours',1],['setUTCHours',1],['setMonth',1],\
         ['setFullYear',1],['setUTCFullYear',1],['setUTCMonth',1]];\n\
         for (var i = 0; i < partial.length; ++i) { \
           var d = new Date(Date.UTC(2000,0,1,5,30,7,8)); \
           d[partial[i][0]](partial[i][1]); \
           print('partial', partial[i][0], d.getTime(), \
                 d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds(), \
                 d.getUTCMilliseconds()); }\n",
    );
    diff_date(&src);
}

/// ERRORS rows 878 / 879: `Dp_toJSON`.
///   878 jsdate.c:786 a non-finite JS_HNUMBER coercion pushes `null`
///   879 jsdate.c:793 `this.toISOString` missing or not callable -> TypeError
///       "this.toISOString is not a function"
#[test]
fn t_date_tojson() {
    let mut src = String::from(
        "function tj(r) { var recv = eval(r); \
           try { print(r, '->', Date.prototype.toJSON.call(recv)) } \
           catch (e) { print(r, '->', e.name + ': ' + e.message) } \
           try { print(r, 'json ->', JSON.stringify(recv)) } \
           catch (e) { print(r, 'json ->', e.name + ': ' + e.message) } }\n",
    );
    for r in [
        // row 878: the primitive coercion is non-finite
        "new Date(NaN)", "new Date('x')", "({valueOf:function(){return NaN}})",
        "({valueOf:function(){return Infinity}})",
        "({valueOf:function(){return -Infinity}})",
        "NaN", "Infinity", "-Infinity",
        // row 879: no callable toISOString
        "({})", "[]", "({toISOString:1})", "({toISOString:'x'})",
        "({toISOString:null})", "({toISOString:undefined})",
        "({toISOString:{}})", "({valueOf:function(){return 0}})",
        "({valueOf:function(){return 'x'}})",
        "0", "1", "'s'", "true", "Math", "JSON", "/re/",
        // and the working paths
        "new Date(0)", "new Date(86400000)",
        "({toISOString:function(){return 'CUSTOM'}})",
        "({valueOf:function(){return 0},toISOString:function(){return 'C2'}})",
        "({toISOString:function(){throw new Error('iso')}})",
        "new Number(0)", "new String('')",
    ] {
        src.push_str(&format!("tj({});\n", jq(r.as_bytes())));
    }
    // toJSON's argument is ignored, and it works through JSON.stringify too
    src.push_str(
        "print(new Date(0).toJSON('key'));\n\
         print(JSON.stringify({d:new Date(0), n:new Date(NaN)}));\n\
         print(JSON.stringify([new Date(NaN)]));\n",
    );
    diff_date(&src);
}

/* ===================================================================== *
 *  jsfunction.c (rows 880-889)
 * ===================================================================== */

/// ERRORS rows 880 / 881: `jsB_Function`.
///   880 jsfunction.c:11 the `js_try` handler frees `sb`, runs
///       `jsP_freeparse(J)` and RETHROWS the original error unchanged
///   881 jsfunction.c:31 `jsP_parsefunction` reports the file as `[string]`,
///       the parameters are joined with `,` and terminated by a synthetic `)`
#[test]
fn t_function_ctor_errors() {
    let mut src = String::from(
        "function f(a) { try { var g = eval('new Function(' + a + ')'); \
           print(a, '->', typeof g, g.length, String(g), g()) } \
         catch (e) { print(a, '->', e.name + ': ' + e.message) } }\n",
    );
    let cases = [
        // no arguments at all: body is `js_isdefined(J, top-1)` on the callee
        "", "''", "'return 1'", "'return'", "';'", "'//c'", "'/*c*/'",
        // bad bodies -> the parser's SyntaxError, file `[string]`
        "'return &'", "'{'", "'}'", "'('", "')'", "'var'", "'1 2'", "'a b'",
        "'return 1;;;'", "'if'", "'function'", "'throw'", "'\\\\'",
        "'return ]'", "'*'", "'x ='",
        // parameter lists
        "'a', 'return a'", "'a,b', 'return a+b'", "'a b', ''", "'1', ''",
        "'a,', ''", "',a', ''", "'a)', ''", "'a(', ''", "')', ''", "'(', ''",
        "'a', 'b', 'return a+b'", "'a', 'a', 'return a'", "'', ''",
        "'a=1', ''", "'a...', ''", "'this', ''", "'return', ''",
        "'/*x*/a', 'return a'", "'a/*', ''", "'a//', ''", "'a\\n', ''",
        // the synthetic `)` means a parameter list may not close itself
        "'a)b', ''", "'a),(b', ''",
        // non-string arguments coerce through js_tostring FIRST (row 880)
        "1, 2", "null, null", "undefined, undefined", "{}, {}", "[], []",
        "({toString:function(){return 'a'}}), 'return a'",
        "({toString:function(){throw new Error('boom')}}), ''",
        "'a', ({toString:function(){throw new Error('body')}})",
        "({toString:function(){throw 'plain'}}), ''",
        "'a', ({toString:function(){return 'return a'}})",
        // a huge parameter list, to exercise the js_Buffer growth
        "'a0,a1,a2,a3,a4,a5,a6,a7,a8,a9', 'return a9'",
    ];
    for c in cases {
        src.push_str(&format!("f({});\n", jq(c.as_bytes())));
    }
    // the non-constructor call form goes through the SAME cfunction
    src.push_str(
        "function fc(a) { try { var g = eval('Function(' + a + ')'); \
           print('call', a, '->', typeof g, String(g)) } \
         catch (e) { print('call', a, '->', e.name + ': ' + e.message) } }\n",
    );
    for c in ["''", "'return 1'", "'a b', ''", "'return &'", "1"] {
        src.push_str(&format!("fc({});\n", jq(c.as_bytes())));
    }
    // Function.prototype itself is a cfunction returning undefined
    src.push_str(
        "print(typeof Function.prototype, Function.prototype(), \
               Function.prototype(1,2,3), Function.prototype.length);\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS rows 882 / 883 / 885 / 886: `js_typeerror(J, "not a function")` from
/// `Fp_toString` (jsfunction.c:53), `Fp_apply` (:100), `Fp_call` (:123) and
/// `Fp_bind` (:186), all gated on `js_iscallable(J, 0)`.
#[test]
fn t_function_not_callable() {
    let mut src = String::from(
        "var ms = ['toString','apply','call','bind'];\n\
         function t(r) { var recv = eval(r); \
           for (var i = 0; i < ms.length; ++i) { \
             try { print(r, ms[i], '->', \
               Function.prototype[ms[i]].call(recv, null, [])) } \
             catch (e) { print(r, ms[i], '->', e.name + ': ' + e.message) } } }\n",
    );
    for r in [
        "({})", "[]", "1", "'s'", "true", "false", "null", "undefined",
        "new Number(1)", "new String('x')", "new Boolean(true)", "new Date(0)",
        "/re/", "Math", "JSON", "new Error('e')", "Function.prototype",
        "(function(){})", "Function", "Math.max", "(function(){}).bind(null)",
        "({toString:function(){return 'x'}})",
    ] {
        src.push_str(&format!("t({});\n", jq(r.as_bytes())));
    }
    // Fp_toString on each callable CLASS (jsfunction.c:55 / :76 / :90)
    src.push_str(
        "print(String(function foo(a,b){return a}));\n\
         print(String(Math.max));\n\
         print(String(function(){}.bind(null)));\n\
         print(String(Function.prototype));\n\
         print(String(new Function('a','b','return a')));\n\
         print((function(){}).toString.call(Function));\n\
         print(String(RegExp));\nprint(String(Date));\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS row 884: jsfunction.c:109 clamps a negative `argArray.length` to 0
/// with no TypeError, even though arg 2 is not a real array.  `js_getlength`
/// (jsarray.c:7) is `js_tointeger` of the `length` property, so a length above
/// `INT_MAX` also lands here through the C `(int)` conversion.
#[test]
fn t_function_apply_negative_length() {
    let mut src = String::from(
        "function f() { var a = []; for (var i = 0; i < arguments.length; ++i) \
           a.push(String(arguments[i])); \
         return arguments.length + ':' + a.join(','); }\n\
         function ap(v) { try { print(v, '->', f.apply(null, eval(v))) } \
         catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    for v in [
        "null", "undefined", "[]", "[1]", "[1,2,3]",
        "({length:-1})", "({length:-5})", "({length:-1e9})",
        "({length:'-3'})", "({length:NaN})", "({length:Infinity})",
        "({length:-Infinity})", "({length:0})", "({length:1})",
        "({length:1,0:'a'})", "({length:3,0:'a',2:'c'})",
        "({length:-0})", "({length:1.9})", "({length:-1.9})",
        "({length:null})", "({length:undefined})", "({length:{}})",
        "({length:true})", "({length:'x'})",
        "({length:2147483647})", "({length:2147483648})",
        "({length:4294967296})", "({length:1e21})", "({length:-1e21})",
        "'abc'", "1", "true", "({})", "(function(){})",
        "new String('ab')", "({length:new Number(-2)})",
        "({get length(){return -4}})",
        "({get length(){throw new Error('len')}})",
        "(function(){var a=[1,2,3]; a.length=-1; return a})()",
        "(function(){var a=[1,2,3]; a.length=1; return a})()",
    ] {
        src.push_str(&format!("ap({});\n", jq(v.as_bytes())));
    }
    // Fp_call's own arity arithmetic (jsfunction.c:128 `top - 2`)
    src.push_str(
        "print(f.call());\nprint(f.call(null));\nprint(f.call(null,1));\n\
         print(f.call(null,1,2));\nprint(Function.prototype.call.call(f));\n\
         print(f.apply());\nprint(f.apply(null));\n",
    );
    diff_script(0, 5, &src);
}

/// ERRORS row 887: jsfunction.c:189 clamps the bound function's `length` to 0
/// instead of letting `js_getlength(J,0) - (top - 2)` go negative.
#[test]
fn t_function_bind_length_clamp() {
    let mut src = String::from(
        "function mk(n) { var s = []; for (var i = 0; i < n; ++i) s.push('a' + i); \
           return new Function(s.join(','), 'return arguments.length'); }\n\
         function b(n, k) { var f = mk(n); var args = [null]; \
           for (var i = 0; i < k; ++i) args.push(i); \
           var g = f.bind.apply(f, args); \
           print(n, k, '->', g.length, g(), g(1,2), String(g)); }\n",
    );
    for n in 0..=5 {
        for k in 0..=6 {
            src.push_str(&format!("b({n}, {k});\n"));
        }
    }
    // a target whose `length` is a lie, including negative
    src.push_str(
        "function lie(v, k) { var f = function(a,b,c){}; \
           try { f.length = v } catch (e) {} \
           var args = [null]; for (var i = 0; i < k; ++i) args.push(i); \
           var g = f.bind.apply(f, args); \
           print('lie', v, k, g.length, String(g)); }\n",
    );
    for v in ["-1", "0", "1", "100", "NaN", "Infinity", "'x'"] {
        for k in [0, 1, 3, 5] {
            src.push_str(&format!("lie({v}, {k});\n"));
        }
    }
    // bind on a cfunction and on an already-bound function
    src.push_str(
        "var m = Math.max.bind(null, 1, 2); print(m.length, m(), m(9), String(m));\n\
         var mm = m.bind(null, 5); print(mm.length, mm(), mm(0));\n\
         var mmm = mm.bind(null); print(mmm.length, mmm());\n",
    );
    diff_script(0, 5, &src);
}

/// `callbound` (jsfunction.c:131) and `constructbound` (jsfunction.c:156).
/// Rows 888 / 889 are the `n < 0` clamps in those two functions, which are
/// UNREACHABLE (see the file header): `__BoundArguments__` is always the array
/// `Fp_bind` built, and it is `JS_READONLY | JS_DONTENUM | JS_DONTCONF`.  This
/// test drives both functions over many bound-argument counts and PROVES the
/// property cannot be subverted.
#[test]
fn t_function_bound_calls() {
    let mut src = String::from(
        "function T() { this.args = Array.prototype.slice ? 0 : 0; \
           this.n = arguments.length; this.a = []; \
           for (var i = 0; i < arguments.length; ++i) this.a.push(arguments[i]); }\n\
         function F() { var a = []; \
           for (var i = 0; i < arguments.length; ++i) a.push(String(arguments[i])); \
           return arguments.length + '/' + this + ':' + a.join(','); }\n\
         function drive(nb, nc) { var args = ['THIS']; \
           for (var i = 0; i < nb; ++i) args.push('b' + i); \
           var g = F.bind.apply(F, args); \
           var cargs = []; for (i = 0; i < nc; ++i) cargs.push('c' + i); \
           print('call', nb, nc, g.apply(null, cargs)); \
           var h = T.bind.apply(T, args); \
           var o = nc === 0 ? new h() : new h('c0'); \
           print('ctor', nb, nc, o.n, o.a.join(',')); }\n",
    );
    for nb in 0..=4 {
        for nc in 0..=3 {
            src.push_str(&format!("drive({nb}, {nc});\n"));
        }
    }
    // the bound properties are not enumerable, not writable, not configurable
    src.push_str(
        "var g = F.bind('T', 1, 2);\n\
         var ks = []; for (var k in g) ks.push(k); print('keys', ks.join(','));\n\
         print('has', '__TargetFunction__' in g, '__BoundThis__' in g, \
               '__BoundArguments__' in g);\n\
         print('read', typeof g.__TargetFunction__, g.__BoundThis__, \
               g.__BoundArguments__.length, g.__BoundArguments__.join(','));\n\
         try { g.__BoundArguments__ = {length:-1}; \
               print('after-write', g.__BoundArguments__.length, g()) } \
         catch (e) { print('after-write', e.name + ': ' + e.message, \
                           g.__BoundArguments__.length, g()) }\n\
         try { print('delete', delete g.__BoundArguments__, \
               g.__BoundArguments__ ? g.__BoundArguments__.length : 'gone', g()) } \
         catch (e) { print('delete', e.name + ': ' + e.message, \
                           g.__BoundArguments__.length, g()) }\n\
         try { g.__BoundArguments__.length = -3; \
               print('after-len', g.__BoundArguments__.length, g()) } \
         catch (e) { print('after-len', e.name + ': ' + e.message, \
                           g.__BoundArguments__.length, g()) }\n\
         try { g.__BoundArguments__.length = 4294967296; \
               print('after-len2', g.__BoundArguments__.length, g()) } \
         catch (e) { print('after-len2', e.name + ': ' + e.message, \
                           g.__BoundArguments__.length, g()) }\n\
         g.__BoundArguments__[5] = 'x';\n\
         print('after-idx', g.__BoundArguments__.length, g());\n\
         print('instanceof', (function(){}).bind(null) instanceof Function);\n",
    );
    // and in strict mode, where the readonly write throws instead
    diff_script(0, 5, &src);
    diff_script(JS_STRICT, 5, &src);
}

/* ===================================================================== *
 *  jsmath.c (rows 890-902)
 * ===================================================================== */

/// ERRORS rows 890-893: `jsM_round` (jsmath.c:12).
///   890 `isnan(x)`      -> x
///   891 `isinf(x)`      -> x
///   892 `0 < x < 0.5`   -> +0   (NOT floor(x + 0.5))
///   893 `-0.5 <= x < 0` -> -0   (negative zero; `1/x` distinguishes it)
/// `dump()` uses `js_torepr`, whose `reprnum` (jsrepr.c:9) prints `-0` for
/// negative zero, so the sign of the zero is compared directly.
#[test]
fn t_math_round() {
    let mut src = String::from(
        "function r(v) { var x = eval(v); var y = Math.round(x); \
           dump(v, y, 1 / y, y === 0 ? (1 / y < 0 ? '-0' : '+0') : ''); }\n",
    );
    for v in [
        "NaN", "0/0", "Infinity", "-Infinity", "0", "-0", "0.1", "0.4",
        "0.49999999999999994", "0.5", "0.5000000000000001", "0.6", "1",
        "-0.1", "-0.4", "-0.49999999999999994", "-0.5",
        "-0.5000000000000001", "-0.6", "-1", "1.5", "-1.5", "2.5", "-2.5",
        "1.4999999999999998", "-1.4999999999999998",
        "4503599627370495.5", "-4503599627370495.5",
        "9007199254740991", "-9007199254740991", "1e300", "-1e300",
        "Number.MIN_VALUE", "-Number.MIN_VALUE", "Number.MAX_VALUE",
        "'x'", "''", "null", "undefined", "true", "false", "[]", "({})",
        "'0.5'", "'-0.5'", "new Number(0.5)",
    ] {
        src.push_str(&format!("r({});\n", jq(v.as_bytes())));
    }
    src.push_str("print(Math.round(), Math.round.length);\n");
    diff_script(0, 5, &src);
}

/// ERRORS row 894: jsmath.c:78 `!isfinite(y) && fabs(x) == 1` pushes NaN,
/// OVERRIDING the C library's `pow`, which returns 1.0 for those inputs.
#[test]
fn t_math_pow_edge() {
    let mut src = String::from("function p(a, b) { dump(a, b, Math.pow(eval(a), eval(b))) }\n");
    let xs = [
        "1", "-1", "1.0", "-1.0", "0.9999999999999999", "1.0000000000000002",
        "0", "-0", "2", "-2", "0.5", "NaN", "Infinity", "-Infinity", "1e300",
        "'1'", "'-1'", "new Number(1)",
    ];
    let ys = [
        "Infinity", "-Infinity", "NaN", "0", "-0", "1", "-1", "2", "0.5",
        "1e300", "'Infinity'", "3", "-3",
    ];
    for a in xs {
        for b in ys {
            src.push_str(&format!("p({}, {});\n", jq(a.as_bytes()), jq(b.as_bytes())));
        }
    }
    src.push_str("dump(Math.pow(), Math.pow(1), Math.pow.length);\n");
    diff_script(0, 5, &src);
}

/// ERRORS rows 895-898: `Math_max` / `Math_min`.
///   895 jsmath.c:127 `Math.max()` with no arguments -> -Infinity
///   896 jsmath.c:130 any NaN argument breaks the loop with x = NaN
///   897 jsmath.c:145 `Math.min()` with no arguments -> +Infinity
///   898 jsmath.c:148 any NaN argument -> NaN
/// The signbit-based comparisons (jsmath.c:134-137 / 152-155) make -0 vs +0
/// observable, so `dump` is used throughout.
#[test]
fn t_math_minmax() {
    let mut src = String::from(
        "function mm(a) { try { dump('max', a, eval('Math.max(' + a + ')')) } \
         catch (e) { print('max', a, e.name + ': ' + e.message) } \
         try { dump('min', a, eval('Math.min(' + a + ')')) } \
         catch (e) { print('min', a, e.name + ': ' + e.message) } }\n",
    );
    for a in [
        "", "1", "-1", "0", "-0", "0,-0", "-0,0", "-0,-0", "0,0",
        "1,2", "2,1", "1,'x'", "'x',1", "NaN", "NaN,1", "1,NaN", "1,NaN,2",
        "NaN,NaN", "Infinity", "-Infinity", "Infinity,-Infinity",
        "-Infinity,Infinity", "-0,1", "1,-0", "-0,-1", "-1,-0",
        "0,Infinity", "-0,-Infinity", "'1','2'", "null", "undefined",
        "null,1", "undefined,1", "true,false", "[],1", "[1],[2]", "({}),1",
        "1,2,3,4,5", "5,4,3,2,1", "1,undefined,3", "-1,-2,-3",
        "new Number(3),2", "'',1", "' ',1", "'0x10',1",
    ] {
        src.push_str(&format!("mm({});\n", jq(a.as_bytes())));
    }
    src.push_str("print(Math.max.length, Math.min.length);\n");
    diff_script(0, 5, &src);
}

/// ERRORS rows 899-902: the four unguarded libm domains.
///   899 jsmath.c:29 `acos(|x| > 1)`
///   900 jsmath.c:34 `asin(|x| > 1)`
///   901 jsmath.c:71 `log(x < 0)` and `log(+/-0)` -> -Infinity
///   902 jsmath.c:116 `sqrt(x < 0)`
/// mujs performs no domain check at all, so whatever libm returns is pushed;
/// both libraries call the SAME libm in this process.
#[test]
fn t_math_domain() {
    let mut src = String::from(
        "var fns = ['abs','acos','asin','atan','ceil','cos','exp','floor','log',\
         'sin','sqrt','tan'];\n\
         function d(v) { var x = eval(v); for (var i = 0; i < fns.length; ++i) \
           dump(fns[i], v, Math[fns[i]](x)); \
           dump('atan2', v, Math.atan2(x, 1), Math.atan2(1, x), Math.atan2(x, x)); }\n",
    );
    for v in [
        "0", "-0", "1", "-1", "1.0000000000000002", "-1.0000000000000002",
        "2", "-2", "0.5", "-0.5", "1e300", "-1e300", "NaN", "Infinity",
        "-Infinity", "Number.MIN_VALUE", "-Number.MIN_VALUE",
        "Number.MAX_VALUE", "-Number.MAX_VALUE", "'x'", "''", "null",
        "undefined", "true", "false", "[]", "[2]", "({})", "'2'", "'-2'",
    ] {
        src.push_str(&format!("d({});\n", jq(v.as_bytes())));
    }
    src.push_str(
        "for (var i = 0; i < fns.length; ++i) \
           dump('noarg', fns[i], Math[fns[i]](), Math[fns[i]].length);\n\
         dump('atan2 noarg', Math.atan2(), Math.atan2(1), Math.atan2.length);\n",
    );
    diff_script(0, 5, &src);
}

/// Randomised sweep of every `Math` function (fixed seed), including
/// subnormals, infinities and NaN payloads, so the domain rows are exercised
/// over the whole double range and not just at the boundaries.
#[test]
fn t_math_fuzz() {
    let mut rng = Rng::new(0x5EED_0890);
    for chunk in 0..4 {
        let mut src = String::from(
            "var fns = ['abs','acos','asin','atan','ceil','cos','exp','floor',\
             'log','round','sin','sqrt','tan'];\n\
             function d(x) { for (var i = 0; i < fns.length; ++i) \
               dump(fns[i], x, Math[fns[i]](x)); \
               dump('pow', x, Math.pow(x, 2), Math.pow(2, x), Math.pow(1, x), \
                    Math.pow(-1, x), Math.pow(x, x)); \
               dump('mm', x, Math.max(x, 0), Math.min(x, 0), Math.max(x, -0), \
                    Math.min(x, -0), Math.max(x), Math.min(x)); }\n",
        );
        for _ in 0..350 {
            let v = rng.f64_sane();
            if v.is_nan() {
                src.push_str("d(NaN);\n");
            } else if v.is_infinite() {
                src.push_str(if v > 0.0 { "d(Infinity);\n" } else { "d(-Infinity);\n" });
            } else {
                src.push_str(&format!("d({v:e});\n"));
            }
        }
        trace(&format!("math fuzz chunk {chunk}"), "");
        diff_script(0, 5, &src);
    }
}

/* ===================================================================== *
 *  jsrepr.c (rows 903-910)
 * ===================================================================== */

/// ERRORS rows 903 / 904: `reprobject` (jsrepr.c:88) writes `"{}"` and
/// `reprarray` (jsrepr.c:118) writes `"[]"` for an object/array already on the
/// repr stack, with no exception.  The scan starts at slot 0 relative to the
/// `J->bot` that `js_repr` installed (jsrepr.c:255), so only true ancestors
/// count and a repeated sibling is printed in full.
#[test]
fn t_repr_cyclic() {
    // `dump` is the harness cfunction that writes `js_torepr` of each argument
    // to the comparison buffer, so the repr text itself is what gets diffed.
    let mut src = String::from(
        "function r(v) { var x; \
           try { x = eval(v) } \
           catch (e) { print(v, '-> eval', e.name + ': ' + e.message); return } \
           try { dump(v, '->', x) } \
           catch (e) { print(v, '->', e.name + ': ' + e.message) } }\n",
    );
    let cases = [
        // row 903
        "(function(){var a={}; a.a=a; return a})()",
        "(function(){var a={}; a.x={y:a}; return a})()",
        "(function(){var a={},b={}; a.b=b; b.a=a; return a})()",
        "(function(){var a={}; a.p=a; return {q:a}})()",
        "(function(){var a={}; a.p=a; a.q=a; return a})()",
        // row 904
        "(function(){var a=[]; a[0]=a; return a})()",
        "(function(){var a=[]; a.push([a]); return a})()",
        "(function(){var a=[],b=[]; a[0]=b; b[0]=a; return a})()",
        "(function(){var a=[]; a[0]=a; return {k:a}})()",
        "(function(){var a=[]; a[0]=a; a[1]=a; return a})()",
        // mixed
        "(function(){var a={},b=[]; a.b=b; b[0]=a; return a})()",
        "(function(){var a=[]; a[0]={x:a}; return a})()",
        // NOT cyclic: the same object appearing twice side by side
        "(function(){var a={x:1}; return {p:a,q:a}})()",
        "(function(){var a=[1]; return [a,a]})()",
        "(function(){var a={x:1}; return [a,{y:a}]})()",
        // deep but finite
        "(function(){var a={}; var t=a; for (var i=0;i<30;++i){t.n={}; t=t.n} return a})()",
        "(function(){var a=[]; var t=a; for (var i=0;i<30;++i){t[0]=[]; t=t[0]} return a})()",
        // wrappers whose repr recurses through reprvalue
        "(function(){var e=new Error('x'); e.message=e; return e})()",
        "(function(){var a={}; a.e=new Error('m'); return a})()",
    ];
    for c in cases {
        src.push_str(&format!("r({});\n", jq(c.as_bytes())));
    }
    diff_script(0, 5, &src);
}

/// ERRORS row 905: jsrepr.c:127 emits the `", "` separator BEFORE the
/// `js_hasindex` test at jsrepr.c:129, so a sparse array reprs as `[1, , 3]`
/// with an EMPTY slot.  This is an upstream C quirk and is reproduced verbatim.
#[test]
fn t_repr_sparse_array() {
    // The exact strings the row calls out, asserted against a literal.
    let p = libs();
    let cases: &[(&str, &str)] = &[
        ("var a=[1]; a[2]=3; return a", "[1, , 3]"),
        ("var a=[]; a[0]=1; a[2]=3; return a", "[1, , 3]"),
        ("var a=[1,2,3]; delete a[1]; return a", "[1, , 3]"),
        ("var a=[]; a.length=3; return a", "[, , ]"),
        ("var a=[]; a.length=1; return a", "[]"),
        ("var a=[]; a.length=0; return a", "[]"),
        ("var a=[1]; a[3]=4; return a", "[1, , , 4]"),
        ("var a=[1,2,3]; return a", "[1, 2, 3]"),
        ("var a=[1,,3]; return a", "[1, , 3]"),
        // `[,,]` has length 2, not 3 (the last elision is the trailing comma)
        ("var a=[,,]; return a", "[, ]"),
        ("var a=[,]; return a", "[]"),
        ("var a=[,,,]; return a", "[, , ]"),
        ("var a=[]; a[5]=6; return a", "[, , , , , 6]"),
        ("var a=[1,2,3]; delete a[0]; return a", "[, 2, 3]"),
        ("var a=[1,2,3]; delete a[2]; return a", "[1, 2, ]"),
        ("var a=[1,2,3]; a.length=10; return a", "[1, 2, 3, , , , , , , ]"),
        ("var a=[[1],[,2]]; return a", "[[1], [, 2]]"),
        ("var a=[{x:1},,{y:2}]; return a", "[{x: 1}, , {y: 2}]"),
    ];
    unsafe {
        for (setup, want) in cases {
            let mut got = vec![];
            for l in [&p.c, &p.rs] {
                out_clear();
                let j = new_state(l, 0);
                let cs = cstr(&format!("dump({{v: (function(){{ {setup} }})()}})"));
                // wrap so the array is reached through reprobject as well
                let _ = l.js_dostring(j, cs.as_ptr());
                let wrapped = out_take();
                out_clear();
                let cs = cstr(&format!("dump((function(){{ {setup} }})())"));
                let rc = l.js_dostring(j, cs.as_ptr());
                let bare = out_take();
                l.js_freestate(j);
                got.push((rc, bare, wrapped));
            }
            assert_eq!(got[0], got[1], "sparse repr divergence for {setup:?}");
            assert_eq!(
                got[0].1.trim_end(),
                *want,
                "sparse repr of {setup:?} must reproduce the C exactly"
            );
        }
    }
    // and a randomised sweep of hole patterns
    let mut rng = Rng::new(0x5EED_0905);
    let mut src = String::from(
        "function s(spec) { var a = []; a.length = spec.length; \
           for (var i = 0; i < spec.length; ++i) if (spec[i]) a[i] = i; \
           dump(spec.join(''), a, a.length); }\n",
    );
    for _ in 0..400 {
        let n = rng.below(9) as usize;
        let spec: Vec<String> = (0..n)
            .map(|_| if rng.below(2) == 0 { "0" } else { "1" }.to_string())
            .collect();
        src.push_str(&format!("s([{}]);\n", spec.join(",")));
    }
    diff_script(0, 5, &src);
}

/// ERRORS row 906: jsrepr.c:73 `p > name && *p == 0` -- a key that is not a
/// bare identifier or an all-digits run terminated by NUL falls back to
/// `reprstr`, which quotes and escapes it.
#[test]
fn t_repr_ident_fallback() {
    let mut src = String::from("function k(key) { var o = {}; o[key] = 1; dump(o); }\n");
    for key in [
        "a", "ab", "A", "_", "_a", "a_", "a1", "_1", "z9", "abc123",
        "0", "1", "9", "10", "007", "1234567890",
        "", "a b", " a", "a ", "1a", "1_", "0x", "a-b", "a.b", "a:b", "a,b",
        "a(b", "a)b", "a[b", "a]b", "a{b", "a}b", "a\"b", "a'b", "a\\b",
        "a\nb", "a\tb", "a\rb", "\u{0}", "\u{1}", "\u{7f}", "\u{e9}",
        "\u{4e2d}\u{6587}", "\u{1f600}", "-1", "+1", "1.5", "1e3",
        "$", "$a", "a$", "@", "#", "%", "^", "&", "*", "!", "?", "~", "`",
        "true", "null", "undefined", "function", "var", "if", "for",
        "constructor", "prototype", "__proto__", "toString",
        "\u{feff}", "  ", "\u{a0}",
    ] {
        src.push_str(&format!("k({});\n", jq(key.as_bytes())));
    }
    // the same keys through an array-ish object and through nesting
    src.push_str(
        "var o = {}; o['a'] = 1; o['1'] = 2; o['a b'] = 3; o[''] = 4; \
         o['1a'] = 5; o['_'] = 6; dump(o);\n\
         dump({a: {'b c': {'1': [1, 2]}}});\n",
    );
    // randomised keys, fixed seed
    let mut rng = Rng::new(0x5EED_0906);
    for _ in 0..600 {
        let n = rng.below(6) as usize;
        let bytes: Vec<u8> = (0..n)
            .map(|_| {
                let t = rng.below(5);
                match t {
                    0 => b'a' + rng.below(26) as u8,
                    1 => b'0' + rng.below(10) as u8,
                    2 => b'A' + rng.below(26) as u8,
                    3 => b'_',
                    _ => 0x21 + rng.below(0x5e) as u8,
                }
            })
            .filter(|b| *b != 0)
            .collect();
        src.push_str(&format!("k({});\n", jq(&bytes)));
    }
    diff_script(0, 5, &src);
}

/* ------------------- jsrepr.c probes through the C API ------------------ */

/// Pushes a `JS_CITERATOR` and reprs it -- row 907.
unsafe extern "C" fn cf_iter_repr(j: JS) {
    let l = cur();
    l.js_newobject(j);
    l.js_pushnumber(j, 1.0);
    l.js_setproperty(j, -2, b"a\0".as_ptr() as *const c_char);
    l.js_pushnumber(j, 2.0);
    l.js_setproperty(j, -2, b"b\0".as_ptr() as *const c_char);
    l.js_pushiterator(j, -1, 1);
    let s = l.js_torepr(j, -1);
    l.js_pushstring(j, s);
}

/// Pushes an EMPTY-object iterator and reprs it -- row 907 again.
unsafe extern "C" fn cf_iter_repr_empty(j: JS) {
    let l = cur();
    l.js_newobject(j);
    l.js_pushiterator(j, -1, 0);
    let s = l.js_torepr(j, -1);
    l.js_pushstring(j, s);
}

/// Pushes a `JS_CUSERDATA` and reprs it -- the sibling of row 907 that IS
/// balanced (jsrepr.c:233-237 closes its bracket), so the two together show the
/// missing `]` is specific to the iterator case.
unsafe extern "C" fn cf_userdata_repr(j: JS) {
    let l = cur();
    l.js_newuserdata(
        j,
        b"mytag\0".as_ptr() as *const c_char,
        0xDEAD_BEEF_usize as *mut c_void,
        None,
    );
    let s = l.js_torepr(j, -1);
    l.js_pushstring(j, s);
}

fn probe_repr(tag: &str, f: unsafe extern "C" fn(JS)) -> String {
    let p = libs();
    let mut got = vec![];
    for l in [&p.c, &p.rs] {
        unsafe {
            out_clear();
            let j = new_state(l, 0);
            l.js_newcfunction(j, Some(f), b"probe\0".as_ptr() as *const c_char, 0);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            let v = from_c(l.js_trystring(j, -1, b"<nostring>\0".as_ptr() as *const c_char));
            l.js_pop(j, 1);
            let top = l.js_gettop(j);
            let out = out_take();
            l.js_freestate(j);
            got.push(format!("rc={rc} v={v:?} top={top} out={out}"));
        }
    }
    assert_eq!(got[0], got[1], "{tag} divergence");
    trace(tag, &got[0]);
    got.pop().unwrap()
}

/// ERRORS row 907: jsrepr.c:231 writes `"[iterator "` with NO closing `]`.
/// An iterator is not reachable from script, so it is pushed with
/// `js_pushiterator` from a protected cfunction.
#[test]
fn t_repr_iterator_unbalanced() {
    let a = probe_repr("iterator repr", cf_iter_repr);
    assert!(
        a.contains("v=\"[iterator \""),
        "the repr of a JS_CITERATOR must be exactly \"[iterator \" \
         (no closing bracket); got {a}"
    );
    let b = probe_repr("iterator repr empty", cf_iter_repr_empty);
    assert!(b.contains("v=\"[iterator \""), "got {b}");
    // the userdata sibling IS balanced
    let c = probe_repr("userdata repr", cf_userdata_repr);
    assert!(
        c.contains("v=\"[userdata mytag]\""),
        "the repr of a JS_CUSERDATA must close its bracket; got {c}"
    );
}

/// ERRORS row 908: jsrepr.c:247 -- the `js_try` handler inside `js_repr` frees
/// `sb` and RETHROWS.  Driven with a throwing getter, reached through
/// `js_getproperty` at jsrepr.c:102 (objects) and `js_hasindex` /
/// `js_getproperty` for arrays.
///
/// ERRORS row 910: jsrepr.c:279 -- `js_tryrepr` catches exactly that rethrow,
/// pops the exception and returns the caller-supplied fallback string.
#[test]
fn t_repr_throwing_getter() {
    let p = libs();
    let setups: &[&str] = &[
        "var v = { get x() { throw new Error('boom') } }",
        "var v = { a: 1, get x() { throw new Error('boom') }, b: 2 }",
        "var v = { get x() { throw 'plain' } }",
        "var v = { get x() { throw { toString: function(){ throw new Error('nested') } } } }",
        "var v = [1, 2]; v.__proto__ = { get 0() { throw new Error('idx') } }",
        "var v = { x: { get y() { throw new Error('deep') } } }",
        "var v = [ { get y() { throw new Error('inarr') } } ]",
        "var v = new Error('e'); v.__defineGetter__ ? 0 : 0; v",
        "var v = { get name() { throw new Error('nm') } }",
        "var v = 1",
        "var v = 'plain'",
        "var v = undefined",
        "var v = null",
        "var v = {}",
    ];
    unsafe {
        for setup in setups {
            let mut got = vec![];
            for l in [&p.c, &p.rs] {
                out_clear();
                let j = new_state(l, 0);
                // row 908 from JS: `dump` calls js_torepr unprotected, so the
                // rethrow is caught by the surrounding JS try/catch.
                let cs = cstr(&format!(
                    "{setup}; try {{ dump(v) }} catch (e) {{ print('caught', \
                     typeof e === 'object' && e !== null ? e.name + ': ' + e.message \
                     : String(e)) }}"
                ));
                let rc1 = l.js_dostring(j, cs.as_ptr());
                let out1 = out_take();

                // row 910 from the C API: js_tryrepr must return the fallback.
                out_clear();
                let cs = cstr(&format!("{setup}; v"));
                let mut r2 = String::new();
                let load = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                if load == 0 {
                    l.js_pushundefined(j);
                    let call = l.js_pcall(j, 0);
                    let before = l.js_gettop(j);
                    let s = from_c(l.js_tryrepr(
                        j,
                        -1,
                        b"<FALLBACK>\0".as_ptr() as *const c_char,
                    ));
                    let after = l.js_gettop(j);
                    r2 = format!("call={call} tryrepr={s:?} top {before}->{after}");
                    l.js_pop(j, 1);
                } else {
                    l.js_pop(j, 1);
                }
                let out2 = out_take();
                let top = l.js_gettop(j);
                l.js_freestate(j);
                got.push(format!("rc1={rc1} out1={out1} {r2} out2={out2} top={top}"));
            }
            assert_eq!(got[0], got[1], "throwing-repr divergence for {setup:?}");
            trace(&format!("repr throw {setup:?}"), &got[0]);
            // Only the setups that actually reached a throw can assert the
            // fallback: an accessor installed on `__proto__` is never consulted
            // for an index of a SIMPLE array (jsrun.c:596 answers from the flat
            // storage), so that case reprs normally.
            if got[0].contains("caught") {
                assert!(
                    got[0].contains("tryrepr=\"<FALLBACK>\""),
                    "js_tryrepr must return the caller's fallback for {setup:?}; \
                     got {}",
                    got[0]
                );
            }
        }
    }
}

/// ERRORS row 909: jsrepr.c:262's `sb ? sb->s : "undefined"` fallback is DEAD
/// CODE, because jsrepr.c:261 runs `js_putc(J, &sb, 0)` first and `js_putc`
/// allocates the buffer when `*sbp` is NULL.  This test shows the premise the
/// row depends on cannot happen: `reprvalue` emits at least one byte for EVERY
/// value class, so the repr of any value is a non-empty string.
#[test]
fn t_repr_empty_buffer_is_dead() {
    // `dump` prints `js_torepr` of every argument, so an EMPTY repr would show
    // up as a missing field; the length is also asserted explicitly below via
    // the C API, where `js_torepr` is called directly.
    let mut src = String::from("function n(v) { dump(v, eval(v)) }\n");
    for v in [
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN",
        "Infinity", "-Infinity", "''", "'a'", "({})", "[]", "(function(){})",
        "Math.max", "new Number(0)", "new String('')", "new Boolean(false)",
        "/re/", "new Date(0)", "new Date(NaN)", "new Error('')", "Math",
        "JSON", "(function(){}).bind(null)", "[[]]", "({a:{}})",
        "new Error()", "Object", "Array", "String.prototype",
    ] {
        src.push_str(&format!("n({});\n", jq(v.as_bytes())));
    }
    diff_script(0, 5, &src);

    // And directly through `js_tryrepr`, so the returned STRING LENGTH of every
    // value class is asserted rather than merely compared.
    let p = libs();
    let exprs = [
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN",
        "Infinity", "-Infinity", "''", "'a'", "({})", "[]", "(function(){})",
        "Math.max", "new Number(0)", "new String('')", "new Boolean(false)",
        "/re/", "new Date(0)", "new Date(NaN)", "new Error('')", "Math",
        "JSON", "(function(){}).bind(null)", "[[]]", "({a:{}})",
        "(function(){var a={}; a.a=a; return a})()",
        "(function(){var a=[]; a[0]=a; return a})()",
    ];
    unsafe {
        for e in exprs {
            let mut got = vec![];
            for l in [&p.c, &p.rs] {
                out_clear();
                let j = new_state(l, 0);
                let cs = cstr(&format!("({e})"));
                let load = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                assert_eq!(load, 0, "{e} must parse");
                l.js_pushundefined(j);
                let call = l.js_pcall(j, 0);
                let s = from_c(l.js_tryrepr(j, -1, b"<FB>\0".as_ptr() as *const c_char));
                assert_ne!(s, "", "row 909: js_tryrepr({e}) must never be empty");
                assert_ne!(s, "<FB>", "js_tryrepr({e}) must not have thrown");
                got.push(format!("call={call} s={s:?} len={}", s.len()));
                l.js_pop(j, 1);
                l.js_freestate(j);
                let _ = out_take();
            }
            assert_eq!(got[0], got[1], "js_tryrepr({e}) divergence");
        }
    }

    // the two classes not reachable from script are covered by the probes in
    // `t_repr_iterator_unbalanced`, both of which produce non-empty output.
    let a = probe_repr("iterator non-empty", cf_iter_repr);
    assert!(!a.contains("v=\"\""), "iterator repr must not be empty: {a}");
}

/* ===================================================================== *
 *  utf.c (rows 911-933)
 * ===================================================================== */

/// `Runeerror` from utf.h -- the value `Bad` is defined to (utf.c:48).
const RUNEERROR: Rune = 0xFFFD;
/// `Runemax` from utf.h.
const RUNEMAX: Rune = 0x10FFFF;

/// Decode `bytes` (NUL padded) with both libraries, assert they agree, and
/// return `(returned length, decoded rune)`.
///
/// The padding is safe: `chartorune` never reads past the first NUL, because a
/// NUL continuation byte always fails the `c & Testx` test and jumps to `bad`.
fn chartorune_both(bytes: &[u8]) -> (c_int, Rune) {
    let p = libs();
    let mut buf: Vec<u8> = bytes.to_vec();
    buf.extend_from_slice(&[0, 0, 0, 0]);
    unsafe {
        let mut ra: Rune = -12345;
        let mut rb: Rune = -12345;
        let na = p.c.jsU_chartorune(&mut ra, buf.as_ptr() as *const c_char);
        let nb = p.rs.jsU_chartorune(&mut rb, buf.as_ptr() as *const c_char);
        assert_eq!((na, ra), (nb, rb), "jsU_chartorune({bytes:02x?})");
        (na, ra)
    }
}

/// Assert `bytes` decodes to exactly `(n, rune)` in BOTH libraries.
fn expect_rune(bytes: &[u8], n: c_int, rune: Rune) {
    let got = chartorune_both(bytes);
    assert_eq!(
        got,
        (n, rune),
        "jsU_chartorune({bytes:02x?}) must return {n} and decode {rune:#x}"
    );
}

/// Assert `bytes` takes a `goto bad` path: `*rune = Runeerror` and return 1
/// (exactly one byte consumed) -- ERRORS row 921, the shared `bad:` label.
fn expect_bad(bytes: &[u8], row: u32) {
    let got = chartorune_both(bytes);
    assert_eq!(
        got,
        (1, RUNEERROR),
        "row {row}: jsU_chartorune({bytes:02x?}) must set *rune = Runeerror \
         ({RUNEERROR:#x}) and return 1"
    );
}

/// ERRORS rows 911-921: every `chartorune` outcome, asserted against the exact
/// sentinel the row names.
#[test]
fn t_utf_chartorune_sentinels() {
    /* row 911 -- utf.c:58 the overlong/modified-UTF-8 NUL is ACCEPTED */
    expect_rune(&[0xC0, 0x80], 2, 0);
    expect_rune(&[0xC0, 0x80, b'a'], 2, 0);
    expect_rune(&[0xC0, 0x80, 0xC0, 0x80], 2, 0);
    // ...but only that exact pair: C0 followed by anything else is row 914
    expect_bad(&[0xC0, 0x81], 914);
    expect_bad(&[0xC0, 0xBF], 914);
    expect_bad(&[0xC0, 0x00], 912);
    expect_bad(&[0xC0], 912);

    /* row 912 -- utf.c:78 second byte is not a continuation byte */
    for bytes in [
        vec![0xC2, 0x41],
        vec![0xC2, 0x00],
        vec![0xC2],
        vec![0xC2, 0x7F],
        vec![0xC2, 0xC0],
        vec![0xC2, 0xFF],
        vec![0xDF],
        vec![0xE0, 0x41],
        vec![0xE0],
        vec![0xF0, 0x41],
        vec![0xF0],
        vec![0x80, 0x41],
        vec![0xF8, 0x41],
        vec![0xFF, 0x41],
    ] {
        expect_bad(&bytes, 912);
    }

    /* row 913 -- utf.c:81 a stray continuation byte as the LEAD byte */
    for lead in 0x80u8..=0xBF {
        expect_bad(&[lead, 0x80], 913);
        expect_bad(&[lead, 0xBF], 913);
        expect_bad(&[lead, 0xA5], 913);
    }

    /* row 914 -- utf.c:84 a 2-byte sequence decoding to <= Rune1 (0x7F) */
    for second in 0x80u8..=0xBF {
        // C0 80 is intercepted by row 911, so skip exactly that pair
        if second != 0x80 {
            expect_bad(&[0xC0, second], 914);
        }
        expect_bad(&[0xC1, second], 914);
    }
    // the first ACCEPTED 2-byte sequence is C2 80 -> U+0080 == Rune1 + 1
    expect_rune(&[0xC2, 0x80], 2, 0x80);
    expect_rune(&[0xDF, 0xBF], 2, 0x7FF);

    /* row 915 -- utf.c:95 third byte is not a continuation byte */
    for bytes in [
        vec![0xE0, 0x80, 0x41],
        vec![0xE0, 0x80],
        vec![0xE1, 0xBF, 0x00],
        vec![0xE1, 0xBF],
        vec![0xEF, 0xBF, 0x7F],
        vec![0xEF, 0xBF, 0xC0],
        vec![0xF0, 0x80, 0x41],
        vec![0xF0, 0x80],
        vec![0xF8, 0x80, 0x41],
    ] {
        expect_bad(&bytes, 915);
    }

    /* row 916 -- utf.c:99 a 3-byte sequence decoding to <= Rune2 (0x7FF) */
    for bytes in [
        vec![0xE0, 0x80, 0x80],
        vec![0xE0, 0x80, 0xAF],
        vec![0xE0, 0x9F, 0xBF],
        vec![0xE0, 0x81, 0x81],
        vec![0xE0, 0x9F, 0x80],
    ] {
        expect_bad(&bytes, 916);
    }
    // the first ACCEPTED 3-byte sequence is E0 A0 80 -> U+0800 == Rune2 + 1
    expect_rune(&[0xE0, 0xA0, 0x80], 3, 0x800);
    expect_rune(&[0xEF, 0xBF, 0xBF], 3, 0xFFFF);
    // surrogates and U+FFFD itself are NOT rejected
    expect_rune(&[0xED, 0xA0, 0x80], 3, 0xD800);
    expect_rune(&[0xED, 0xBF, 0xBF], 3, 0xDFFF);
    expect_rune(&[0xEF, 0xBF, 0xBD], 3, RUNEERROR);

    /* row 917 -- utf.c:111 fourth byte is not a continuation byte */
    for bytes in [
        vec![0xF0, 0x80, 0x80, 0x41],
        vec![0xF0, 0x90, 0x80, 0x41],
        vec![0xF0, 0x90, 0x80],
        vec![0xF4, 0x8F, 0xBF, 0x00],
        vec![0xF7, 0xBF, 0xBF, 0x7F],
        vec![0xF8, 0x80, 0x80, 0x41],
        vec![0xFF, 0xBF, 0xBF, 0x41],
    ] {
        expect_bad(&bytes, 917);
    }

    /* row 918 -- utf.c:113 lead byte >= T5 (0xF8): no 5-byte forms */
    for lead in 0xF8u8..=0xFF {
        expect_bad(&[lead, 0x88, 0x80, 0x80], 918);
        expect_bad(&[lead, 0xBF, 0xBF, 0xBF], 918);
        expect_bad(&[lead, 0x80, 0x80, 0x80], 918);
    }

    /* row 919 -- utf.c:116 a 4-byte sequence decoding to <= Rune3 (0xFFFF) */
    for bytes in [
        vec![0xF0, 0x80, 0x80, 0x80],
        vec![0xF0, 0x8F, 0xBF, 0xBF],
        vec![0xF0, 0x80, 0xA0, 0x80],
        vec![0xF0, 0x8F, 0x80, 0x80],
    ] {
        expect_bad(&bytes, 919);
    }
    // the first ACCEPTED 4-byte sequence is F0 90 80 80 -> U+10000 == Rune3 + 1
    expect_rune(&[0xF0, 0x90, 0x80, 0x80], 4, 0x10000);
    expect_rune(&[0xF4, 0x8F, 0xBF, 0xBF], 4, RUNEMAX);

    /* row 920 -- utf.c:117 a 4-byte sequence decoding to > Runemax */
    for bytes in [
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xF5, 0x80, 0x80, 0x80],
        vec![0xF6, 0x80, 0x80, 0x80],
        vec![0xF7, 0xBF, 0xBF, 0xBF],
        vec![0xF4, 0xBF, 0xBF, 0xBF],
    ] {
        expect_bad(&bytes, 920);
    }

    /* the 1-byte forms, for completeness */
    for c in 0x00u8..=0x7F {
        expect_rune(&[c], 1, c as Rune);
    }
    expect_rune(&[0], 1, 0);

    /* row 921 -- exhaustive: EVERY 2-byte prefix must land on one of the
     * above outcomes, and every `bad` outcome must be exactly (1, 0xFFFD). */
    for lead in 0x00u16..=0xFF {
        for second in 0x00u16..=0xFF {
            let (n, r) = chartorune_both(&[lead as u8, second as u8]);
            assert!(
                (1..=4).contains(&n),
                "chartorune([{lead:02x},{second:02x}]) returned {n}"
            );
            if n == 1 && lead >= 0x80 {
                assert_eq!(
                    r, RUNEERROR,
                    "chartorune([{lead:02x},{second:02x}]) consumed 1 byte but \
                     did not set Runeerror"
                );
            }
        }
    }
    /* and every 4-byte sequence built from structurally plausible bytes */
    let mut rng = Rng::new(0x5EED_0911);
    for _ in 0..40000 {
        let bytes: Vec<u8> = (0..4).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
        let (n, r) = chartorune_both(&bytes);
        assert!((1..=4).contains(&n));
        if n == 1 && bytes[0] >= 0x80 {
            assert_eq!(r, RUNEERROR, "random {bytes:02x?}");
        }
    }
}

/// ERRORS rows 922-924: `runetochar`.
///   922 utf.c:138 `*rune == 0` encodes the 2-byte overlong NUL C0 80 and
///       returns 2 -- NEVER a bare `\0`
///   923 utf.c:148 a NEGATIVE rune satisfies `c <= Rune1`, so the truncated low
///       byte is written and 1 is returned; there is no range rejection
///   924 utf.c:167 `c > Runemax` is silently replaced by Runeerror and emitted
///       as its 3-byte form, returning 3
#[test]
fn t_utf_runetochar_sentinels() {
    let p = libs();
    /// Encode `c` with both libraries and return `(len, bytes)`.
    fn enc(c: Rune) -> (c_int, Vec<u8>) {
        let p = libs();
        unsafe {
            let mut ba = [0i8; 32];
            let mut bb = [0i8; 32];
            let r = c;
            let na = p.c.jsU_runetochar(ba.as_mut_ptr(), &r);
            let nb = p.rs.jsU_runetochar(bb.as_mut_ptr(), &r);
            assert_eq!(na, nb, "jsU_runetochar({c:#x}) return");
            assert_eq!(ba, bb, "jsU_runetochar({c:#x}) bytes");
            (na, (0..na.max(0) as usize).map(|i| ba[i] as u8).collect())
        }
    }

    /* row 922 */
    assert_eq!(enc(0), (2, vec![0xC0, 0x80]), "row 922: runetochar(0)");

    /* row 923 -- negative runes write the truncated low byte, length 1 */
    for c in [-1, -2, -0x7F, -0x80, -0x81, -0xFF, -0x100, -0x1234, i32::MIN, -1000000] {
        let (n, b) = enc(c);
        assert_eq!(n, 1, "row 923: runetochar({c:#x}) must return 1");
        assert_eq!(
            b,
            vec![(c as u32 & 0xFF) as u8],
            "row 923: runetochar({c:#x}) must write the truncated low byte"
        );
    }

    /* row 924 -- c > Runemax becomes Runeerror's 3-byte encoding */
    for c in [
        RUNEMAX + 1, 0x110001, 0x1FFFFF, 0x200000, 0x7FFFFFFF, 0x1000000,
        0x123456, RUNEMAX + 2,
    ] {
        let (n, b) = enc(c);
        assert_eq!(n, 3, "row 924: runetochar({c:#x}) must return 3");
        assert_eq!(
            b,
            vec![0xEF, 0xBF, 0xBD],
            "row 924: runetochar({c:#x}) must emit Runeerror's encoding"
        );
    }

    /* the well-defined lengths, from below and above each boundary */
    for (c, n) in [
        (1, 1), (0x7F, 1), (0x80, 2), (0x7FF, 2), (0x800, 3), (0xFFFF, 3),
        (0x10000, 4), (RUNEMAX, 4),
    ] {
        assert_eq!(enc(c).0, n, "runetochar({c:#x}) length");
    }
    // round trip: every rune that encodes to 1..4 bytes decodes back, except
    // 0 (which decodes back through the row-911 special case) and > Runemax
    let mut rng = Rng::new(0x5EED_0922);
    for _ in 0..30000 {
        let c = (rng.next_u32() & 0x1F_FFFF) as Rune;
        let (n, b) = enc(c);
        if c > 0 && c <= RUNEMAX {
            assert_eq!(chartorune_both(&b), (n, c), "round trip {c:#x}");
        }
    }
    for _ in 0..5000 {
        let c = rng.next_u32() as Rune;
        enc(c);
    }
    // sub == NULL is not a thing here, but the RUNE POINTER is read once
    unsafe {
        let mut ba = [0i8; 32];
        let r: Rune = 0x41;
        assert_eq!(p.c.jsU_runetochar(ba.as_mut_ptr(), &r), 1);
        assert_eq!(p.rs.jsU_runetochar(ba.as_mut_ptr(), &r), 1);
    }
}

/// ERRORS row 925: `runelen` (utf.c:187) delegates to `runetochar` with a local
/// 10-byte buffer, so it returns 1, 2, 3 or 4 and NEVER an error code: 2 for
/// `c == 0`, 3 for `c > Runemax`, 1 for a negative `c`.
#[test]
fn t_utf_runelen_sentinels() {
    let p = libs();
    unsafe {
        let both = |c: c_int| -> c_int {
            let a = p.c.jsU_runelen(c);
            let b = p.rs.jsU_runelen(c);
            assert_eq!(a, b, "jsU_runelen({c:#x})");
            a
        };
        assert_eq!(both(0), 2, "row 925: runelen(0) must be 2");
        for c in [-1, -2, -0xFF, -0x10000, i32::MIN] {
            assert_eq!(both(c), 1, "row 925: runelen({c:#x}) must be 1");
        }
        for c in [RUNEMAX + 1, 0x1FFFFF, 0x7FFFFFFF, 0x110000] {
            assert_eq!(both(c), 3, "row 925: runelen({c:#x}) must be 3");
        }
        for (c, n) in [
            (1, 1), (0x7F, 1), (0x80, 2), (0x7FF, 2), (0x800, 3), (0xFFFF, 3),
            (0x10000, 4), (RUNEMAX, 4),
        ] {
            assert_eq!(both(c), n, "runelen({c:#x})");
        }
        // never anything else, over the whole plausible range and at random
        for c in 0..0x11_0002 {
            let n = both(c);
            assert!((1..=4).contains(&n), "runelen({c:#x}) returned {n}");
        }
        let mut rng = Rng::new(0x5EED_0925);
        for _ in 0..40000 {
            let c = rng.next_u32() as c_int;
            let n = both(c);
            assert!((1..=4).contains(&n), "runelen({c:#x}) returned {n}");
        }
    }
}

/// ERRORS rows 926-931: the `ucd_bsearch` miss (utf.c:212 returns 0) and the
/// six predicates / case mappers built on it.
///   926 ucd_bsearch          -> 0 (NULL)
///   927 tolowerrune          -> c unchanged
///   928 toupperrune          -> c unchanged
///   929 islowerrune          -> 0
///   930 isupperrune          -> 0
///   931 isalpharune          -> 0
#[test]
fn t_utf_table_misses() {
    let p = libs();
    unsafe {
        let pred = |name: &'static str, c: Rune| -> c_int {
            let a = p.c.rune_pred(name, c);
            let b = p.rs.rune_pred(name, c);
            assert_eq!(a, b, "{name}({c:#x})");
            a
        };
        // Runes BELOW the first table entry can only take the utf.c:212 miss:
        // every one of ucd_alpha2 / ucd_tolower1/2 / ucd_toupper1/2 starts at
        // 0x41 or later, so `n && c >= t[0]` is false and bsearch returns NULL.
        for c in [
            0, 1, 0x20, 0x2F, 0x30, 0x39, 0x40, -1, -2, -0x41, i32::MIN,
        ] {
            assert_eq!(pred("jsU_tolowerrune", c), c, "row 927: tolowerrune({c:#x})");
            assert_eq!(pred("jsU_toupperrune", c), c, "row 928: toupperrune({c:#x})");
            assert_eq!(pred("jsU_islowerrune", c), 0, "row 929: islowerrune({c:#x})");
            assert_eq!(pred("jsU_isupperrune", c), 0, "row 930: isupperrune({c:#x})");
            assert_eq!(pred("jsU_isalpharune", c), 0, "row 931: isalpharune({c:#x})");
        }
        // Runes ABOVE every table entry take the `c > p[1]` / `c != p[0]` miss
        // instead, with the same sentinels.
        for c in [
            0x10FFFF, 0x110000, 0x1FFFFF, 0x7FFFFFFF, 0xE01F0, 0xF0000,
            0x10FFFE, 0x300000,
        ] {
            assert_eq!(pred("jsU_tolowerrune", c), c, "row 927: tolowerrune({c:#x})");
            assert_eq!(pred("jsU_toupperrune", c), c, "row 928: toupperrune({c:#x})");
            assert_eq!(pred("jsU_islowerrune", c), 0, "row 929: islowerrune({c:#x})");
            assert_eq!(pred("jsU_isupperrune", c), 0, "row 930: isupperrune({c:#x})");
            assert_eq!(pred("jsU_isalpharune", c), 0, "row 931: isalpharune({c:#x})");
        }
        // Interior gaps: digits, punctuation and unassigned blocks are in
        // range but between entries.
        for c in [
            0x5B, 0x5C, 0x5D, 0x5E, 0x60, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0xA0,
            0xD7, 0xF7, 0x378, 0x380, 0x1680, 0x2000, 0x3000, 0x4E2D, 0xFFFE,
            0xFFFF, 0x1FFFE, 0x2FFFE,
        ] {
            let lo = pred("jsU_tolowerrune", c);
            let up = pred("jsU_toupperrune", c);
            let il = pred("jsU_islowerrune", c);
            let iu = pred("jsU_isupperrune", c);
            let ia = pred("jsU_isalpharune", c);
            // whatever the tables say, C and Rust must agree AND a miss must
            // mean the identity / 0 sentinel
            if il == 0 && iu == 0 {
                assert_eq!(lo, c, "tolowerrune({c:#x}) with no case mapping");
                assert_eq!(up, c, "toupperrune({c:#x}) with no case mapping");
            }
            assert!((0..=1).contains(&ia), "isalpharune({c:#x}) = {ia}");
        }
        // exhaustive sweep of the whole BMP + SMP tail, asserting only that the
        // two libraries agree and that the predicates are 0/1
        for c in 0..0x11_0000 {
            for name in ["jsU_islowerrune", "jsU_isupperrune", "jsU_isalpharune"] {
                let v = pred(name, c);
                assert!((0..=1).contains(&v), "{name}({c:#x}) = {v}");
            }
            pred("jsU_tolowerrune", c);
            pred("jsU_toupperrune", c);
        }
        let mut rng = Rng::new(0x5EED_0926);
        for _ in 0..20000 {
            let c = rng.next_u32() as Rune;
            for name in [
                "jsU_islowerrune", "jsU_isupperrune", "jsU_isalpharune",
                "jsU_tolowerrune", "jsU_toupperrune",
            ] {
                pred(name, c);
            }
        }
    }
}

/// ERRORS rows 932 / 933: `tolowerrune_full` (utf.c:294) and
/// `toupperrune_full` (utf.c:305) return NULL when `!p || c != p[0]`.
#[test]
fn t_utf_full_case_null() {
    let p = libs();
    unsafe {
        let full = |name: &'static str, c: Rune| -> (bool, Vec<Rune>) {
            let pa = p.c.rune_full(name, c);
            let pb = p.rs.rune_full(name, c);
            assert_eq!(
                pa.is_null(),
                pb.is_null(),
                "{name}({c:#x}) NULL-ness divergence"
            );
            if pa.is_null() {
                return (true, vec![]);
            }
            let mut va = vec![];
            let mut vb = vec![];
            for i in 0..8isize {
                let a = *pa.offset(i);
                let b = *pb.offset(i);
                if a == 0 && b == 0 {
                    break;
                }
                va.push(a);
                vb.push(b);
            }
            assert_eq!(va, vb, "{name}({c:#x}) payload");
            (false, va)
        };
        // Below / above every table entry, and in obvious gaps: must be NULL.
        for c in [
            0, 1, 0x20, 0x30, 0x39, 0x40, 0x41, 0x5A, 0x61, 0x7A, -1, -2,
            i32::MIN, 0x10FFFF, 0x110000, 0x1FFFFF, 0x7FFFFFFF, 0x4E2D,
            0xFFFE, 0xFFFF,
        ] {
            for name in ["jsU_tolowerrune_full", "jsU_toupperrune_full"] {
                let (is_null, _) = full(name, c);
                assert!(
                    is_null,
                    "rows 932/933: {name}({c:#x}) must return NULL"
                );
            }
        }
        // U+00DF LATIN SMALL LETTER SHARP S has a full UPPERCASE mapping but no
        // full lowercase one -- the two sentinels are independent.
        let (lo_null, _) = full("jsU_tolowerrune_full", 0xDF);
        let (up_null, up) = full("jsU_toupperrune_full", 0xDF);
        assert!(lo_null, "row 932: tolowerrune_full(U+00DF) must be NULL");
        assert!(
            !up_null && up.len() > 1,
            "toupperrune_full(U+00DF) must return a multi-rune mapping, got {up:?}"
        );
        // exhaustive sweep: every rune in the BMP + SMP, plus randoms
        let mut null_lo = 0usize;
        let mut null_up = 0usize;
        for c in 0..0x11_0000 {
            if full("jsU_tolowerrune_full", c).0 {
                null_lo += 1;
            }
            if full("jsU_toupperrune_full", c).0 {
                null_up += 1;
            }
        }
        assert!(
            null_lo > 0x10_0000 && null_up > 0x10_0000,
            "the NULL path must dominate: {null_lo} / {null_up}"
        );
        let mut rng = Rng::new(0x5EED_0932);
        for _ in 0..20000 {
            let c = rng.next_u32() as Rune;
            full("jsU_tolowerrune_full", c);
            full("jsU_toupperrune_full", c);
        }
    }
}

/// The utf.c sentinels as observed through the higher level string entry
/// points, so the byte counts the rows name are also exercised in context:
/// `js_utflen` (jsstring.c) counts a `bad` byte as ONE position, and
/// `js_runeat` returns the same Runeerror.
#[test]
fn t_utf_sentinels_in_context() {
    let p = libs();
    unsafe {
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        let cases: Vec<Vec<u8>> = vec![
            vec![0xC0, 0x80],                   // row 911: one position
            vec![0x80],                         // row 913
            vec![0xC2],                         // row 912
            vec![0xC1, 0xBF],                   // row 914: TWO bad bytes
            vec![0xE0, 0x80, 0xAF],             // row 916: THREE bad bytes
            vec![0xF0, 0x80, 0x80, 0x80],       // row 919
            vec![0xF7, 0xBF, 0xBF, 0xBF],       // row 920
            vec![0xF8, 0x88, 0x80, 0x80],       // row 918
            vec![0xFF],                         // row 912
            vec![b'a', 0x80, b'b'],
            vec![0xC0, 0x80, b'x', 0xC0, 0x80],
            vec![0xED, 0xA0, 0x80],             // a surrogate: accepted
            vec![0xF4, 0x8F, 0xBF, 0xBF],       // Runemax: TWO positions
            vec![0xEF, 0xBF, 0xBD],             // a literal U+FFFD
        ];
        for bytes in &cases {
            let mut buf = bytes.clone();
            buf.push(0);
            let sp = buf.as_ptr() as *const c_char;
            let la = p.c.js_utflen(sp);
            let lb = p.rs.js_utflen(sp);
            assert_eq!(la, lb, "js_utflen({bytes:02x?})");
            for i in -1..(la + 2) {
                set_cur(&p.c);
                let a = p.c.js_runeat(jc, sp, i);
                set_cur(&p.rs);
                let b = p.rs.js_runeat(jr, sp, i);
                assert_eq!(a, b, "js_runeat({bytes:02x?}, {i})");
            }
            for off in 0..=bytes.len() {
                let a = p.c.js_utfptrtoidx(sp, sp.add(off));
                let b = p.rs.js_utfptrtoidx(sp, sp.add(off));
                assert_eq!(a, b, "js_utfptrtoidx({bytes:02x?}, +{off})");
            }
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
    // and through JS: charCodeAt / length / fromCharCode on the same bytes
    let mut src = String::from(
        "function s(x) { var out = [x.length]; \
           for (var i = 0; i < x.length; ++i) out.push(x.charCodeAt(i)); \
           print(out.join(',')); }\n",
    );
    for bytes in [
        vec![0xC0u8, 0x80],
        vec![0x80],
        vec![0xC2],
        vec![0xC1, 0xBF],
        vec![0xE0, 0x80, 0xAF],
        vec![0xF0, 0x80, 0x80, 0x80],
        vec![0xF7, 0xBF, 0xBF, 0xBF],
        vec![0xF8, 0x88, 0x80, 0x80],
        vec![0xFF],
        vec![b'a', 0x80, b'b'],
    ] {
        src.push_str(&format!("s({});\n", jq(&bytes)));
    }
    src.push_str(
        "for (var c = -3; c < 5; ++c) print('fcc', c, \
           String.fromCharCode(c).length);\n\
         print('fcc big', String.fromCharCode(0x110000).length, \
               String.fromCharCode(0x10FFFF).length, \
               String.fromCharCode(0).length, \
               String.fromCharCode(-1).length);\n",
    );
    diff_script(0, 5, &src);
}
