//! Differential tests for the regexp surface — CONFIGS.md rows 53-56.
//!
//! row 53: `js_newregexp` × all JS_REGEXP flag combos + `js_isregexp`/`js_toregexp`
//! row 54: `js_RegExp_prototype_exec` (lastIndex advance) + the JS-level pipeline
//! row 55: `js_regcomp`/`js_regexec`/`js_regfree` over a large pattern corpus
//! row 56: `js_regcompx`/`js_regfreex` with a counting/failing custom allocator
//!
//! All match positions are printed as *byte offsets* relative to the subject
//! string, never as raw pointers, so the output is address-independent.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering::SeqCst};

/* ------------------------------------------------------------------ helpers */

fn esc_bytes(b: &[u8]) -> String {
    let mut o = String::with_capacity(b.len() + 8);
    for &c in b {
        match c {
            b'\\' => o.push_str("\\\\"),
            b'\n' => o.push_str("\\n"),
            b'\r' => o.push_str("\\r"),
            b'\t' => o.push_str("\\t"),
            0x20..=0x7e => o.push(c as char),
            _ => o.push_str(&format!("\\x{:02x}", c)),
        }
    }
    o
}

fn esc(s: &str) -> String {
    esc_bytes(s.as_bytes())
}

/// Escaped, truncated to `n` bytes (keeps the output bounded for the huge
/// generated patterns).
fn esc_t(s: &str, n: usize) -> String {
    let b = s.as_bytes();
    if b.len() <= n {
        esc_bytes(b)
    } else {
        format!("{}...<+{}>", esc_bytes(&b[..n]), b.len() - n)
    }
}

/// Byte offset of `p` inside the subject that starts at `base` (-1 for NULL).
fn off(p: *const c_char, base: *const c_char) -> i64 {
    if p.is_null() {
        -1
    } else {
        (p as isize - base as isize) as i64
    }
}

unsafe fn flush() {
    libc::fflush(null_mut());
}

/// `js_newstate` does not install `print`, so the scripts append to a global
/// string `OUT` which is read back and printed line by line.
const JS_PRELUDE: &str = "var OUT = ''; function pr(x) { OUT += x + '\\n'; }\n";

unsafe fn run_js(api: &Api, J: JS, script: &str) {
    let full = format!("{}{}", JS_PRELUDE, script);
    let cscript = cs(&full);
    flush();
    let rc = (api.js_dostring)(J, cscript.as_ptr());
    flush();
    p_int("dostring-rc", rc);
    (api.js_getglobal)(J, cs("OUT").as_ptr());
    let s = (api.js_tostring)(J, -1);
    if s.is_null() {
        p_line("OUT=<null>");
    } else {
        for line in CStr::from_ptr(s).to_bytes().split(|&c| c == b'\n') {
            if !line.is_empty() {
                p_line(&format!("| {}", esc_bytes(line)));
            }
        }
    }
    (api.js_pop)(J, 1);
    flush();
}

unsafe fn p_cstr_esc(label: &str, s: *const c_char) {
    if s.is_null() {
        p_line(&format!("{}=<null>", label));
    } else {
        p_line(&format!("{}={}", label, esc_bytes(CStr::from_ptr(s).to_bytes())));
    }
}

/* --------------------------------------------------------------- corpora */

/// Curated pattern corpus: one entry per branch of c_src/src/regexp.c.
fn curated_patterns() -> Vec<String> {
    let fixed: &[&str] = &[
        /* trivial / empty */
        "", "()", "(?:)", "a", "abc", "aBc", "ABC", "a/b",
        /* any */
        ".", "a.c", "..", ".*", ".+",
        /* anchors */
        "^", "$", "^abc", "abc$", "^$", "^abc$", "^a|b$", "$^",
        /* word boundaries */
        r"\b", r"\B", r"\babc", r"abc\b", r"\Babc", r"a\Bb", r"\bA\b",
        /* built-in classes */
        r"\d", r"\D", r"\s", r"\S", r"\w", r"\W", r"\d\d\d", r"\w+\s\w+", r"\D\S\W",
        /* character classes */
        "[abc]", "[a-z]", "[a-zA-Z0-9_]", "[^abc]", "[^a-z]", "[]", "[^]", "[]]", "[]a]",
        "[-a]", "[a-]", "[a-c-e]", "[.*+?]", "[$^]", "[|]", "[{}]",
        r"[a\-z]", r"[\]]", r"[\\]", r"[\d]", r"[\d-x]", r"[\w\s]", r"[^\d]", r"[\b]",
        r"[\n\r\t]", r"[\x41-\x43]", r"[A-Z]", r"[\cA-\cZ]", r"[0\0]",
        /* huge-span classes (addranges_D/S/W) */
        r"[\D]", r"[\S]", r"[\W]", r"[^\S]", r"[\d\D]", r"[\w\W]", r"[\s\S]",
        /* invalid classes */
        "[b-a]", "[abc", "[", r"[a\", r"[z-a]",
        /* quantifiers */
        "a*", "a+", "a?", "a{3}", "a{2,}", "a{2,4}", "a{0}", "a{0,0}", "a{1,1}", "a{0,254}",
        "a{254}", "ab{2}c", "(ab){2}", "[ab]{2,3}", ".{2}",
        /* non-greedy */
        "a*?", "a+?", "a??", "a{2,4}?", "a{3}?", "a{2,}?", "^(a+?)(a*)$", "(.*?)b",
        /* invalid quantifiers */
        "a{4,2}", "a{300}", "a{0,255}", "a{", "a{2", "a{2,", "a{,2}", "a{x}", "*a", "+a", "?a",
        /* alternation */
        "a|b", "a|b|c", "abc|abd", "|a", "a|", "|", "(a|b|c)+", "^(?:ab|cd)$",
        /* groups */
        "(a)", "(a)(b)", "(a(b))", "(a|b)c", "(?:ab)+", "(?:a|b)c", "((a))", "(a)|(b)",
        "(", ")", "(a", "a)", "(?:", "(?", "(?a)",
        /* lookahead */
        "a(?=b)", "a(?=b)c", "a(?!b)", "(?=a)", "(?!a)", "(?=)", "(?!)", "^(?!a)b", "a(?=(b))",
        /* back-references */
        r"(a)\1", r"(a)(b)\2\1", r"\1", r"(\1)", r"(a)\9", r"(a)(b)(c)\3", r"(a+)\1",
        r"(?:(a))\1", r"\0", r"(a)\0",
        /* escapes */
        r"\n", r"\r", r"\t", r"\f", r"\v", r"\x41", r"\x00", r"\xZZ", r"\x4", r"A",
        r"é", r"\uZZZZ", r"\u004", r"\cA", r"\cI", r"\c", r"\q", r"\_", r"\-", r"\\",
        "\\", r"a\", r"\^\$\.\*\+\?\(\)\[\]\{\}\|", r"\/", r"\8", r"\10",
        /* unicode literals */
        "é", "日本", "[à-ÿ]", ".é.", "é+", "\u{2028}",
    ];

    let mut v: Vec<String> = fixed.iter().map(|s| s.to_string()).collect();

    /* capture-group counts (REG_MAXSUB = 16 → at most 15 captures) */
    let groups = |n: usize| -> String {
        (0..n).map(|i| format!("({})", (b'a' + i as u8) as char)).collect()
    };
    v.push(groups(9));
    v.push(groups(14));
    v.push(groups(15));
    v.push(groups(16));
    v.push(groups(20));

    /* nesting (REG_MAXREC) */
    v.push(format!("{}a{}", "(?:".repeat(40), ")".repeat(40)));
    v.push(format!("{}a{}", "(".repeat(10), ")".repeat(10)));

    /* long pattern / REG_MAXPROG via strlen(pattern)*2 */
    v.push("a".repeat(2000));
    v.push("a".repeat(4090));
    v.push("a".repeat(5000));
    v.push("a".repeat(20000));

    /* REG_MAXPROG via count() */
    v.push("(a{100}){100}".to_string());
    v.push("(a{200}){200}".to_string());
    v.push("(?:a{16}){16}".to_string());

    /* REG_MAXCLASS: 129 distinct character classes */
    v.push(r"\d".repeat(129));
    v.push(r"\d".repeat(128));

    /* REG_MAXSPAN: > 31 disjoint ranges in one class */
    let mut span = String::from("[");
    let mut k = 0;
    let mut c = 33u8;
    while k < 40 {
        if c != b'-' && c != b']' && c != b'^' && c != b'\\' {
            span.push(c as char);
            k += 1;
        }
        c += 2;
    }
    span.push(']');
    v.push(span);

    /* "infinite loop matching the empty string" */
    v.push("(a*)*".to_string());
    v.push("(a?)+".to_string());
    v.push("()*".to_string());
    v.push("(|a)*".to_string());
    v.push("(a|)*".to_string());
    v.push(r"(\b)*".to_string());
    v.push("(){2}".to_string());
    v.push("(?:)*".to_string());

    v
}

const RAND_ALPHABET: &[u8] = b"abc()[]{}|*+?.^$\\-,0123456789";

/// ~200 random patterns from a fixed seed; most will not compile (the error
/// strings are part of the compared output).
fn random_patterns(n: usize) -> Vec<String> {
    let mut rng = Rng::new(0x5eed_1234_abcd_0001);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let len = 1 + rng.range(12) as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(RAND_ALPHABET[rng.range(RAND_ALPHABET.len() as u32) as usize] as char);
        }
        v.push(s);
    }
    v
}

fn main_subjects() -> Vec<String> {
    [
        "",
        "a",
        "abc",
        "aaa",
        "AAA",
        "aBc",
        "a1b2c3",
        "a\nb\nc",
        "héllo",
        "日本abc",
        "abcabc",
        "0123456789",
        "The quick fox",
        " a-b_c ",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Tiny subject set used for the random patterns (keeps backtracking bounded).
fn small_subjects() -> Vec<String> {
    ["", "a", "abc", "aaa", "a\nb", "0-9"].iter().map(|s| s.to_string()).collect()
}

/// Subjects for patterns whose character classes have huge spans — matching
/// those under REG_ICASE is O(span) per character, so keep them short.
fn heavy_subjects() -> Vec<String> {
    ["", "a", "Z", "9", "a\nb"].iter().map(|s| s.to_string()).collect()
}

fn random_subjects(n: usize) -> Vec<String> {
    const CH: &[char] = &['a', 'b', 'c', 'A', 'B', 'C', '0', '9', ' ', '\n', '\t', '-', '_', 'é', '日'];
    let mut rng = Rng::new(0x5eed_1234_abcd_0002);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let len = rng.range(9) as usize;
        let mut s = String::new();
        for _ in 0..len {
            s.push(CH[rng.range(CH.len() as u32) as usize]);
        }
        v.push(s);
    }
    v
}

fn is_heavy(pat: &str) -> bool {
    pat.contains('[') && (pat.contains("\\D") || pat.contains("\\S") || pat.contains("\\W"))
}

/* ------------------------------------------------------- row 55: regcomp */

unsafe fn exec_one(api: &Api, tag: &str, prog: Reprog, si: usize, subj: &str, ef: c_int, with_sub: bool) {
    let s = cs(subj);
    let base = s.as_ptr();
    if with_sub {
        let mut m = Resub::new();
        let rc = (api.js_regexec)(prog, base, &mut m, ef);
        let mut line = format!("{} s{} ef={} sub=1 rc={} nsub={}", tag, si, ef, rc, m.nsub);
        let n = if m.nsub < 0 || m.nsub > 16 { 16 } else { m.nsub as usize };
        for i in 0..n {
            line += &format!(" [{}:{},{}]", i, off(m.sub[i].sp, base), off(m.sub[i].ep, base));
        }
        p_line(&line);
    } else {
        let rc = (api.js_regexec)(prog, base, null_mut(), ef);
        p_line(&format!("{} s{} ef={} sub=0 rc={}", tag, si, ef, rc));
    }
}

#[test]
fn cfg55_regcomp_regexec_regfree() {
    diff("cfg55_regcomp", |api| unsafe { body_cfg55_regcomp(api) });
}

#[test]
fn cfg55_long_subject() {
    diff("cfg55_long", |api| unsafe { body_cfg55_long(api) });
}

#[test]
fn cfg55_deep_nesting() {
    /* Separate process so that a stack overflow (if any) cannot truncate the
     * output of the main row-55 test. REG_MAXREC = 4096 in count(). */
    diff("cfg55_deep", |api| unsafe { body_cfg55_deep(api) });
}

/* ------------------------------------------------- row 56: custom allocator */

static A_ALLOC: AtomicUsize = AtomicUsize::new(0); /* n > 0, p == NULL  */
static A_REALLOC: AtomicUsize = AtomicUsize::new(0); /* n > 0, p != NULL */
static A_FREE: AtomicUsize = AtomicUsize::new(0); /* n == 0, p != NULL */
static A_FREENULL: AtomicUsize = AtomicUsize::new(0); /* n == 0, p == NULL */
static A_BYTES: AtomicUsize = AtomicUsize::new(0);
static A_FAILAT: AtomicI64 = AtomicI64::new(-1); /* fail the (FAILAT+1)-th alloc */
static A_CTXOK: AtomicBool = AtomicBool::new(true);
static A_LIVE: AtomicI64 = AtomicI64::new(0);

const CTX_MAGIC: usize = 0x5AFE_C0DE;

unsafe extern "C" fn counting_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if ctx as usize != CTX_MAGIC {
        A_CTXOK.store(false, SeqCst);
    }
    if n == 0 {
        if p.is_null() {
            A_FREENULL.fetch_add(1, SeqCst);
        } else {
            A_FREE.fetch_add(1, SeqCst);
            A_LIVE.fetch_sub(1, SeqCst);
        }
        libc::free(p);
        return null_mut();
    }
    let idx = if p.is_null() {
        A_ALLOC.fetch_add(1, SeqCst)
    } else {
        A_REALLOC.fetch_add(1, SeqCst)
    };
    let fail = A_FAILAT.load(SeqCst);
    if fail >= 0 && (idx as i64) >= fail {
        return null_mut();
    }
    A_BYTES.fetch_add(n as usize, SeqCst);
    let q = libc::realloc(p, n as usize);
    if !q.is_null() && p.is_null() {
        A_LIVE.fetch_add(1, SeqCst);
    }
    q
}

fn a_reset(fail_at: i64) {
    A_ALLOC.store(0, SeqCst);
    A_REALLOC.store(0, SeqCst);
    A_FREE.store(0, SeqCst);
    A_FREENULL.store(0, SeqCst);
    A_BYTES.store(0, SeqCst);
    A_LIVE.store(0, SeqCst);
    A_FAILAT.store(fail_at, SeqCst);
}

unsafe fn p_counts(label: &str) {
    p_line(&format!(
        "{} alloc={} realloc={} free={} freenull={} live={} ctxok={}",
        label,
        A_ALLOC.load(SeqCst),
        A_REALLOC.load(SeqCst),
        A_FREE.load(SeqCst),
        A_FREENULL.load(SeqCst),
        A_LIVE.load(SeqCst),
        A_CTXOK.load(SeqCst) as i32,
    ));
}

#[test]
fn cfg56_regcompx_regfreex_custom_alloc() {
    diff("cfg56_regcompx", |api| unsafe { body_cfg56_regcompx(api) });
}

/* ------------------------------------------------------ row 53: newregexp */

const RE_PATTERNS: &[&str] = &[
    "(?:)",
    "a",
    "abc",
    "a/b",
    "^abc$",
    r"\d+",
    "[a-z]+",
    "(a)(b)(c)",
    "a|b",
    "a*b+c?",
    "(?:ab)+",
    "a(?=b)",
    r"(a)\1",
    "é",
    r"\n\t",
    "a{2,4}",
    ".",
];

const RE_FLAGS: &[c_int] = &[0, 1, 2, 3, 4, 5, 6, 7, 255, -1, 8];

unsafe fn dump_prop(api: &Api, J: JS, idx: c_int, name: &str) {
    let n = cs(name);
    (api.js_getproperty)(J, idx, n.as_ptr());
    let t = (api.js_typeof)(J, -1);
    let b = (api.js_toboolean)(J, -1);
    let num = (api.js_tonumber)(J, -1);
    let s = (api.js_tostring)(J, -1);
    p_line(&format!(
        "  prop {} typeof={} bool={} num={:.17} str={}",
        name,
        if t.is_null() { "<null>".to_string() } else { esc_bytes(CStr::from_ptr(t).to_bytes()) },
        b,
        num,
        if s.is_null() { "<null>".to_string() } else { esc_bytes(CStr::from_ptr(s).to_bytes()) },
    ));
    (api.js_pop)(J, 1);
}

#[test]
fn cfg53_newregexp() {
    diff("cfg53_newregexp", |api| unsafe { body_cfg53_newregexp(api) });
}

static PANIC_HIT: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn on_panic(_J: JS) {
    PANIC_HIT.fetch_add(1, SeqCst);
    p_line("PANIC: uncaught throw from js_newregexp");
    libc::fflush(null_mut());
    libc::_exit(9);
}

#[test]
fn cfg53_newregexp_invalid_throws() {
    /* js_newregexp() with a broken pattern raises a SyntaxError; with no active
     * try buffer that reaches J->panic. Both libraries must get there. */
    diff("cfg53_throw", |api| unsafe { body_cfg53_throw(api) });
}

/* --------------------------------------- row 54: RegExp.prototype.exec (C) */

unsafe fn drive_exec(api: &Api, pat: &str, flags: c_int, text: &str, iters: usize) {
    let J = newstate(api, 0);
    let cp = cs(pat);
    (api.js_newregexp)(J, cp.as_ptr(), flags);
    let ridx = (api.js_gettop)(J) - 1;
    let re = (api.js_toregexp)(J, ridx);
    p_line(&format!("E <{}> flags={} text=<{}>", esc(pat), flags, esc(text)));
    p_ptr_nonnull("  re", re);
    let ct = cs(text);
    for it in 0..iters {
        (api.js_RegExp_prototype_exec)(J, re, ct.as_ptr());
        if (api.js_isnull)(J, -1) != 0 {
            p_line(&format!("  it{} null", it));
            (api.js_pop)(J, 1);
            dump_prop(api, J, ridx, "lastIndex");
            break;
        }
        let aidx = (api.js_gettop)(J) - 1;
        let len = (api.js_getlength)(J, aidx);
        let mut line = format!("  it{} len={}", it, len);
        (api.js_getproperty)(J, aidx, cs("index").as_ptr());
        line += &format!(" index={:.17}", (api.js_tonumber)(J, -1));
        (api.js_pop)(J, 1);
        (api.js_getproperty)(J, aidx, cs("input").as_ptr());
        let inp = (api.js_tostring)(J, -1);
        line += &format!(
            " input=<{}>",
            if inp.is_null() { "<null>".to_string() } else { esc_bytes(CStr::from_ptr(inp).to_bytes()) }
        );
        (api.js_pop)(J, 1);
        for i in 0..len {
            (api.js_getindex)(J, aidx, i);
            let t = (api.js_typeof)(J, -1);
            let s = (api.js_tostring)(J, -1);
            line += &format!(
                " [{}:{}:{}]",
                i,
                if t.is_null() { "?".to_string() } else { esc_bytes(CStr::from_ptr(t).to_bytes()) },
                if s.is_null() { "?".to_string() } else { esc_bytes(CStr::from_ptr(s).to_bytes()) }
            );
            (api.js_pop)(J, 1);
        }
        p_line(&line);
        (api.js_pop)(J, 1);
        dump_prop(api, J, ridx, "lastIndex");
    }
    p_int("  final-top", (api.js_gettop)(J));
    (api.js_freestate)(J);
}

#[test]
fn cfg54_regexp_prototype_exec() {
    diff("cfg54_exec", |api| unsafe { body_cfg54_exec(api) });
}

#[test]
fn cfg54_regexp_from_js() {
    diff("cfg54_js", |api| unsafe { body_cfg54_js(api) });
}


unsafe fn body_cfg55_regcomp(api: &Api) {
        let curated = curated_patterns();
        let nfix = curated.len();
        let mut pats = curated;
        pats.extend(random_patterns(200));

        let subj = main_subjects();
        let rsubj = random_subjects(50);
        let ssubj = small_subjects();
        let hsubj = heavy_subjects();

        let cflag_set: [c_int; 5] = [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE, 255];

        for (pi, pat) in pats.iter().enumerate() {
            let rand_pat = pi >= nfix;
            let heavy = is_heavy(pat);
            p_line(&format!("== p{} len={} pat=<{}>", pi, pat.len(), esc_t(pat, 56)));
            for &cf in cflag_set.iter() {
                let cp = cs(pat);
                let mut errp: *const c_char = null();
                let prog = (api.js_regcomp)(cp.as_ptr(), cf, &mut errp);
                if prog.is_null() {
                    p_line(&format!("p{} cf={} FAIL", pi, cf));
                    p_cstr_esc("  err", errp);
                    continue;
                }
                p_line(&format!("p{} cf={} OK", pi, cf));
                p_cstr_esc("  err", errp);
                let tag = format!("  x{}/{}", pi, cf);
                if rand_pat {
                    for (si, s) in ssubj.iter().enumerate() {
                        for &ef in [0 as c_int, REG_NOTBOL].iter() {
                            exec_one(api, &tag, prog, si, s, ef, true);
                        }
                    }
                } else if heavy {
                    for (si, s) in hsubj.iter().enumerate() {
                        for &ef in [0 as c_int, REG_NOTBOL].iter() {
                            exec_one(api, &tag, prog, si, s, ef, true);
                            exec_one(api, &tag, prog, si, s, ef, false);
                        }
                    }
                } else {
                    for (si, s) in subj.iter().enumerate() {
                        for &ef in [0 as c_int, REG_NOTBOL, 255].iter() {
                            exec_one(api, &tag, prog, si, s, ef, true);
                            exec_one(api, &tag, prog, si, s, ef, false);
                        }
                    }
                    for (si, s) in rsubj.iter().enumerate() {
                        exec_one(api, &tag, prog, 100 + si, s, 0, true);
                    }
                }
                (api.js_regfree)(prog);
            }
        }
        /* js_regfree(NULL) must be a no-op */
        (api.js_regfree)(null_mut());
        p_line("regfree(NULL) survived");
}

unsafe fn body_cfg55_long(api: &Api) {
        /* A 2000 byte subject, only paired with patterns whose matching cost is
         * linear-ish, so the test stays fast. */
        let long1: String = "ab".repeat(1000);
        let long2: String = "a".repeat(2000);
        let long3: String = format!("{}\n{}", "x".repeat(999), "y".repeat(1000));
        let subs = [long1, long2, long3];
        let pats: Vec<String> = vec![
            "".into(),
            "a".into(),
            "abc".into(),
            "^ab".into(),
            "ab$".into(),
            r"\d".into(),
            r"\w+".into(),
            "[ab]+".into(),
            "(a)(b)".into(),
            ".*b".into(),
            "(ab)+".into(),
            "a|b".into(),
            r"\b".into(),
            "(?:ab){10}".into(),
            "a".repeat(5000),
            "y{999}".into(),
            "^y".into(),
            "x*$".into(),
        ];
        for (pi, pat) in pats.iter().enumerate() {
            for &cf in [0 as c_int, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE, 255].iter() {
                let cp = cs(pat);
                let mut errp: *const c_char = null();
                let prog = (api.js_regcomp)(cp.as_ptr(), cf, &mut errp);
                if prog.is_null() {
                    p_line(&format!("L p{} cf={} FAIL", pi, cf));
                    p_cstr_esc("  err", errp);
                    continue;
                }
                p_line(&format!("L p{} cf={} OK len={}", pi, cf, pat.len()));
                let tag = format!("  L{}/{}", pi, cf);
                for (si, s) in subs.iter().enumerate() {
                    for &ef in [0 as c_int, REG_NOTBOL].iter() {
                        exec_one(api, &tag, prog, si, s, ef, true);
                    }
                }
                (api.js_regfree)(prog);
            }
        }
}

unsafe fn body_cfg55_deep(api: &Api) {
        for depth in [40usize, 1000, 4095, 4200] {
            let pat = format!("{}a{}", "(?:".repeat(depth), ")".repeat(depth));
            let cp = cs(&pat);
            let mut errp: *const c_char = null();
            let prog = (api.js_regcomp)(cp.as_ptr(), 0, &mut errp);
            p_line(&format!("deep {} nullprog={}", depth, prog.is_null() as i32));
            p_cstr_esc("  err", errp);
            if !prog.is_null() {
                let s = cs("aaa");
                let mut m = Resub::new();
                let rc = (api.js_regexec)(prog, s.as_ptr(), &mut m, 0);
                p_line(&format!(
                    "  rc={} nsub={} [0:{},{}]",
                    rc,
                    m.nsub,
                    off(m.sub[0].sp, s.as_ptr()),
                    off(m.sub[0].ep, s.as_ptr())
                ));
                (api.js_regfree)(prog);
            }
        }
}

unsafe fn body_cfg56_regcompx(api: &Api) {
        let ctx = CTX_MAGIC as *mut c_void;
        let pats: Vec<String> = vec![
            "".into(),
            "a".into(),
            "abc".into(),
            "(a)(b)".into(),
            "[abc]x".into(),
            r"[\d]+[a-z]*".into(),
            r"^(\w+)\s(\w+)$".into(),
            "a{2,4}?".into(),
            "(?:ab|cd)+".into(),
            "a(?=b)(?!c)".into(),
            r"(a)\1".into(),
            "é+".into(),
            "(a*)*".into(),  /* compile error path */
            "[b-a]".into(),  /* compile error path */
            "(".into(),      /* compile error path */
            "a{4,2}".into(), /* compile error path */
            r"\q".into(),    /* compile error path */
            "a".repeat(20000),
            "(a{200}){200}".into(),
            r"\d".repeat(129),
            format!("{}a{}", "(?:".repeat(40), ")".repeat(40)),
            "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)".into(),
        ];
        let subj = ["", "a", "ab", "abc", "a b", "AB", "aa\nbb", "héllo", "abcd"];

        for (pi, pat) in pats.iter().enumerate() {
            for &cf in [0 as c_int, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE, 255].iter() {
                a_reset(-1);
                let cp = cs(pat);
                let mut errp: *const c_char = null();
                let prog = (api.js_regcompx)(Some(counting_alloc), ctx, cp.as_ptr(), cf, &mut errp);
                p_line(&format!(
                    "X p{} cf={} len={} nullprog={}",
                    pi,
                    cf,
                    pat.len(),
                    prog.is_null() as i32
                ));
                p_cstr_esc("  err", errp);
                p_counts("  after-compile");
                if !prog.is_null() {
                    let tag = format!("  X{}/{}", pi, cf);
                    for (si, s) in subj.iter().enumerate() {
                        for &ef in [0 as c_int, REG_NOTBOL, 255].iter() {
                            exec_one(api, &tag, prog, si, s, ef, true);
                        }
                        exec_one(api, &tag, prog, si, s, 0, false);
                    }
                    p_counts("  after-exec");
                    (api.js_regfreex)(Some(counting_alloc), ctx, prog);
                }
                p_counts("  after-free");
            }
        }

        /* regfreex(NULL) is a no-op */
        a_reset(-1);
        (api.js_regfreex)(Some(counting_alloc), ctx, null_mut());
        p_counts("freex-null");

        /* Allocator returning NULL after N successful allocations: exercises
         * every "cannot allocate ..." path in regcompx. */
        let fail_pats = ["[abc]x(y)", "", "abc", "(a)(b)[0-9]"];
        for (pi, pat) in fail_pats.iter().enumerate() {
            for fail_at in 0i64..6 {
                a_reset(fail_at);
                let cp = cs(pat);
                let mut errp: *const c_char = null();
                let prog = (api.js_regcompx)(Some(counting_alloc), ctx, cp.as_ptr(), 0, &mut errp);
                p_line(&format!(
                    "F p{} failat={} nullprog={}",
                    pi,
                    fail_at,
                    prog.is_null() as i32
                ));
                p_cstr_esc("  err", errp);
                p_counts("  counts");
                if !prog.is_null() {
                    let s = cs("abcxy0");
                    let mut m = Resub::new();
                    let rc = (api.js_regexec)(prog, s.as_ptr(), &mut m, 0);
                    let mut line = format!("  rc={} nsub={}", rc, m.nsub);
                    let n = if m.nsub < 0 || m.nsub > 16 { 16 } else { m.nsub as usize };
                    for i in 0..n {
                        line += &format!(
                            " [{}:{},{}]",
                            i,
                            off(m.sub[i].sp, s.as_ptr()),
                            off(m.sub[i].ep, s.as_ptr())
                        );
                    }
                    p_line(&line);
                    a_reset(-1);
                    (api.js_regfreex)(Some(counting_alloc), ctx, prog);
                    p_counts("  freed");
                }
            }
        }

        /* errorp == NULL on the failure path must not crash */
        a_reset(-1);
        let cp = cs("(a*)*");
        let prog = (api.js_regcompx)(Some(counting_alloc), ctx, cp.as_ptr(), 0, null_mut());
        p_line(&format!("noerrp nullprog={}", prog.is_null() as i32));
        p_counts("  counts");
        let cp2 = cs("ab+");
        let prog2 = (api.js_regcompx)(Some(counting_alloc), ctx, cp2.as_ptr(), 0, null_mut());
        p_line(&format!("noerrp2 nullprog={}", prog2.is_null() as i32));
        if !prog2.is_null() {
            (api.js_regfreex)(Some(counting_alloc), ctx, prog2);
        }
        p_counts("  counts2");
}

unsafe fn body_cfg53_newregexp(api: &Api) {
        for (pi, pat) in RE_PATTERNS.iter().enumerate() {
            let J = newstate(api, 0);
            for &fl in RE_FLAGS.iter() {
                let cp = cs(pat);
                (api.js_newregexp)(J, cp.as_ptr(), fl);
                let idx = (api.js_gettop)(J) - 1;
                p_line(&format!("R p{} <{}> flags={} top={}", pi, esc(pat), fl, idx + 1));
                p_int("  isregexp", (api.js_isregexp)(J, idx));
                p_int("  isobject", (api.js_isobject)(J, idx));
                p_int("  iscallable", (api.js_iscallable)(J, idx));
                let re = (api.js_toregexp)(J, idx);
                p_ptr_nonnull("  toregexp", re);
                p_cstr_esc("  typeof", (api.js_typeof)(J, idx));
                for name in ["source", "global", "ignoreCase", "multiline", "lastIndex", "nope"] {
                    dump_prop(api, J, idx, name);
                }
                /* js_tostring() on an object replaces the stack slot with the
                 * primitive, so always work on a copy. */
                (api.js_repr)(J, idx);
                p_cstr_esc("  repr", (api.js_tostring)(J, -1));
                (api.js_pop)(J, 1);
                (api.js_copy)(J, idx);
                p_cstr_esc("  tostring", (api.js_tostring)(J, -1));
                (api.js_pop)(J, 1);
                p_int("  isregexp-after", (api.js_isregexp)(J, idx));
                (api.js_pop)(J, 1);
                p_int("  top-after-pop", (api.js_gettop)(J));
            }
            (api.js_freestate)(J);
        }

        /* js_isregexp on non-regexps */
        let J = newstate(api, 0);
        (api.js_pushnumber)(J, 1.0);
        (api.js_pushstring)(J, cs("abc").as_ptr());
        (api.js_newobject)(J);
        (api.js_newarray)(J);
        (api.js_pushnull)(J);
        (api.js_pushundefined)(J);
        let top = (api.js_gettop)(J);
        for i in 0..top {
            p_line(&format!("  nonre {} isregexp={}", i, (api.js_isregexp)(J, i)));
        }
        (api.js_freestate)(J);

        /* invalid patterns via the JS constructor: must throw identically */
        let J = newstate(api, 0);
        run_js(api, J, concat!(
            "var bad = ['(', ')', '[', 'a{4,2}', '(a*)*', '\\\\q', '[b-a]', 'a{300}', 'a{300', ''];\n",
            "for (var i = 0; i < bad.length; ++i) {\n",
            "  try { var r = new RegExp(bad[i]); pr('ok ' + i + ' ' + r); }\n",
            "  catch (e) { pr('threw ' + i + ' ' + e.name + ': ' + e.message); }\n",
            "}\n",
            "pr(String(new RegExp()));\n",
            "pr(String(new RegExp('')));\n",
            "pr(String(new RegExp('a/b', 'gim')));\n",
            "pr(String(new RegExp('a', 'g')));\n",
            "pr(String(new RegExp('a', 'mi')));\n",
            "try { new RegExp('a', 'gg'); } catch (e) { pr('flag ' + e.message); }\n",
            "try { new RegExp('a', 'x'); } catch (e) { pr('flag ' + e.message); }\n",
            "var q = /a/gim;\n",
            "pr(q.source + ' ' + q.global + ' ' + q.ignoreCase + ' ' + q.multiline + ' ' + q.lastIndex);\n",
            "pr(String(new RegExp(q)));\n",
            "pr(typeof q + ' ' + (q instanceof RegExp) + ' ' + Object.prototype.toString.call(q));\n",
            "try { q.source = 'z'; pr('set source ' + q.source); } catch (e) { pr('set ' + e.name); }\n",
            "q.lastIndex = 3; pr('lastIndex ' + q.lastIndex);\n",
            "try { new RegExp(/a/, 'g'); } catch (e) { pr('clone-flags ' + e.name + ': ' + e.message); }\n",
            "pr(String(RegExp(/a\\/b/g)));\n",
        ));
        (api.js_freestate)(J);
}

unsafe fn body_cfg53_throw(api: &Api) {
        let J = newstate(api, 0);
        (api.js_atpanic)(J, Some(on_panic));
        p_line("before");
        let cp = cs("(a*)*");
        (api.js_newregexp)(J, cp.as_ptr(), 0);
        p_line("after (not reached)");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
}

unsafe fn body_cfg54_exec(api: &Api) {
        let cases: &[(&str, &str)] = &[
            ("a", "banana"),
            ("a(b*)", "ab abb xx abbb"),
            ("(a)(b)?", "ab a b"),
            (r"\d+", "a1bb22ccc333"),
            ("^a", "aaa"),
            ("^a", "b\na\nc"),
            ("x", "aaa"),
            ("a*", "bbb"),
            ("", "abc"),
            ("(?:)", "ab"),
            (r"\bfox\b", "the fox and the fox"),
            ("é", "aéaéa"),
            ("(a)|(b)", "ba"),
            ("A", "aAa"),
        ];
        for &(pat, text) in cases {
            for &fl in [0, JS_REGEXP_G, JS_REGEXP_I, JS_REGEXP_M, JS_REGEXP_G | JS_REGEXP_I,
                        JS_REGEXP_G | JS_REGEXP_M, JS_REGEXP_G | JS_REGEXP_I | JS_REGEXP_M]
                .iter()
            {
                let iters = if fl & JS_REGEXP_G != 0 { 6 } else { 2 };
                drive_exec(api, pat, fl, text, iters);
            }
        }
        /* exec after lastIndex has been pushed past the end of the subject */
        let J = newstate(api, 0);
        (api.js_newregexp)(J, cs("a").as_ptr(), JS_REGEXP_G);
        let ridx = (api.js_gettop)(J) - 1;
        let re = (api.js_toregexp)(J, ridx);
        (api.js_pushnumber)(J, 99.0);
        (api.js_setproperty)(J, ridx, cs("lastIndex").as_ptr());
        dump_prop(api, J, ridx, "lastIndex");
        (api.js_RegExp_prototype_exec)(J, re, cs("aaa").as_ptr());
        p_int("past-end isnull", (api.js_isnull)(J, -1));
        (api.js_pop)(J, 1);
        dump_prop(api, J, ridx, "lastIndex");
        (api.js_freestate)(J);
}

unsafe fn body_cfg54_js(api: &Api) {
        let J = newstate(api, 0);
        run_js(api, J, concat!(
            "function show(x) { pr(JSON.stringify(x)); }\n",
            "var s = 'ab abb xx abbb';\n",
            "var re = /a(b*)/g;\n",
            "var m;\n",
            "while ((m = re.exec(s)) !== null) {\n",
            "  pr('exec ' + m.index + ' [' + m[0] + '] [' + m[1] + '] last=' + re.lastIndex + ' input=' + m.input);\n",
            "}\n",
            "pr('final last=' + re.lastIndex);\n",
            "pr('again ' + re.exec(s) + ' last=' + re.lastIndex);\n",
            "var re2 = /a(b*)/;\n",
            "for (var i = 0; i < 3; ++i) {\n",
            "  var r = re2.exec(s);\n",
            "  pr('nog ' + i + ' ' + r.index + ' [' + r[0] + '] last=' + re2.lastIndex);\n",
            "}\n",
            "var eg = /x*/g;\n",
            "for (var i = 0; i < 5; ++i) { var r = eg.exec('abc'); pr('empty ' + i + ' ' + (r ? r.index + ' [' + r[0] + ']' : 'null') + ' last=' + eg.lastIndex); }\n",
            "pr('test1 ' + /^ab/.test('abc'));\n",
            "var tg = /b/g;\n",
            "pr('testg ' + tg.test('abb') + ' ' + tg.lastIndex + ' ' + tg.test('abb') + ' ' + tg.lastIndex + ' ' + tg.test('abb') + ' ' + tg.lastIndex);\n",
            "pr('replace ' + s.replace(/b+/g, 'B'));\n",
            "pr('replace1 ' + s.replace(/b+/, 'B'));\n",
            "pr('replace$ ' + s.replace(/a(b*)/g, '<$1|$&|$`|$\\'|$$>'));\n",
            "pr('replacefn ' + s.replace(/a(b*)/g, function (a, b, off, str) { return '[' + a + '/' + b + '/' + off + '/' + str.length + ']'; }));\n",
            "pr('replace-empty ' + 'abc'.replace(/x*/g, '-'));\n",
            "show(s.match(/a(b*)/g));\n",
            "show(s.match(/a(b*)/));\n",
            "show('aaa'.match(/b*/g));\n",
            "show('aaa'.match(/x/g));\n",
            "show('a1b22c333d'.split(/[0-9]+/));\n",
            "show('abc'.split(/(b)/));\n",
            "show('abc'.split(/x/));\n",
            "show('abc'.split(/(?:)/));\n",
            "show('a,b,,c'.split(/,/));\n",
            "pr('search ' + s.search(/b+/) + ' ' + s.search(/zz/));\n",
            "pr('multiline ' + 'a\\nb'.match(/^b$/m));\n",
            "pr('icase ' + 'ABC'.match(/abc/i));\n",
            "pr('src ' + /a\\/b/.source + ' ' + String(/a\\/b/));\n",
            "show(/./g.exec('\u{e9}x'));\n",
            "pr('utf-index ' + /x/.exec('\u{e9}x').index);\n",
            "try { 'abc'.match(/(/); } catch (e) { pr('bad-literal ' + e.name); }\n",
        ));
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
}


