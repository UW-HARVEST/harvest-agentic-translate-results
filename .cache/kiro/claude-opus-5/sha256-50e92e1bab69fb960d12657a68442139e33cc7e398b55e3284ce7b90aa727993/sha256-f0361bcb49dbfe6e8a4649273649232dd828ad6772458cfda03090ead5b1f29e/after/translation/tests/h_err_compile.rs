//! ERRORS.md Part 2 — compile-time error codes.
//!
//! Every pattern in the corpus is a row: the test asserts that the C `.so` and
//! the Rust `.so` return the SAME `(errorcode, erroroffset)` pair, and that the
//! corpus keeps covering at least `MIN_DISTINCT_CODES` distinct compile error
//! codes (so the row set cannot silently shrink).
//!
//! Run with `PRINT_CODES=1` to dump the code -> pattern map that backs the
//! table in `ERRORS.md`.
mod common;
use common::corpus::PATTERNS;
use common::*;
use std::collections::BTreeMap;
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Patterns crafted to reach specific `ERRnn` branches in `pcre2_compile.c`
/// (derived from the message list in `pcre2_error.c` and the `*errorcodeptr =`
/// assignments in the compiler).
const ERR_PATTERNS: &[&str] = &[
    // \ and \c
    "\\", "a\\", "\\c", "a\\c", "\\cA", "\\c\u{00e9}", "\\c\u{0080}", "\\c ", "\\c~",
    "\\q", "\\y", "\\z9", "\\F", "\\L", "\\U", "\\u", "\\N{name}", "\\N{U+0041}",
    "\\N{U+}", "\\N{U+110000}", "\\N{U+d800}",
    // quantifiers
    "a{2,1}", "a{1000000}", "a{2,1000000}", "a{65536}", "a{,65536}", "*", "+", "?",
    "{1}", "a**", "a*+*", "?a", "+a", "{2}a", "a{1,2}{3}",
    // classes
    "[", "[a", "[a-", "[z-a]", "[\\d-z]", "[a-\\d]", "[[:foo:]]", "[[.ch.]]", "[[=e=]]",
    "[[:alpha:]", "[\\", "[\\q]", "[\\N{U+41}]", "[\\N]", "[]", "[^]", "[a-\\w]",
    "[\\x{110000}]", "[\\x{d800}]",
    // groups
    "(", "(a", "((", "(((((", ")", "a)", "(?", "(?z)", "(?#", "(?#comment",
    "(?<", "(?<a", "(?<>a)", "(?<1a>a)", "(?<a)", "(?'a", "(?'a)", "(?P", "(?Pz",
    "(?P<", "(?P<a", "(?P<>a)", "(?P=", "(?P>", "(?<a>x)(?<a>y)", "(?<=a+)", "(?<=a*)",
    "(?<=.{1,300})", "(?<=\\X)", "(?<=\\C)", "(*UTF)(?<=\\C)", "(?=a)(?<!\\K)",
    "(?<=\\Ka)", "(?<a>)(?<b>)\\g{c}",
    // conditions
    "(?(", "(?(1", "(?(1)a", "(?(1)a|b|c", "(?(1)a|b|c)", "(?()a)", "(?(?=a)b|c|d)",
    "(?(R", "(?(R1", "(?(R&", "(?(<a", "(?(<a>", "(?('a", "(?(DEFINE)a|b)",
    "(?(VERSION>=)a)", "(?(VERSION>=1000)a)", "(?(VERSION=abc)a)", "(?(1)|)", "(?(?C)a)",
    "(?(?i)a|b)",
    // references
    "\\1", "\\9", "(a)\\2", "\\g", "\\g{", "\\g{}", "\\g{0}", "\\g{-0}", "\\g{+0}",
    "\\g{a", "\\g<", "\\g<a", "\\g'a", "\\g{999999}", "\\k", "\\k{", "\\k<", "\\k'",
    "\\k{a", "\\k<>", "(?1)", "(?-1)", "(?+1)", "(?+0)", "(?-0)", "(?+", "(?-",
    "(?R", "(?R1", "(?&", "(?&a", "(?&nosuch)",
    // escapes / numeric
    "\\x{", "\\x{}", "\\x{g}", "\\x{110000}", "\\x{d800}", "\\x{ffffffffff}", "\\xg",
    "\\o", "\\o{", "\\o{}", "\\o{9}", "\\o{777777777777}", "\\o{400}", "\\400",
    "\\777", "\\u{", "\\u{}", "\\u{110000}", "\\u0", "\\uzzzz",
    // properties
    "\\p", "\\P", "\\p{", "\\p{}", "\\p{zzz}", "\\pZ", "\\p{^}", "\\p{Wombat}",
    "\\p{Latin", "\\P{Wombat}", "\\p{Is}", "\\p{Sc=}", "\\p{Script=Nosuch}",
    // callouts
    "(?C", "(?C1", "(?C256", "(?C256)", "(?C300)", "(?Cx)", "(?C`", "(?C`x", "(?C'x",
    "(?C\"x", "(?C{x", "(?C^x", "(?C%x", "(?C#x", "(?C$x", "(?C1x)",
    // verbs
    "(*", "(*)", "(*UNKNOWN)", "(*MARK)", "(*MARK:)", "(*:)", "(*ACCEPT:x)",
    "(*COMMIT:x)", "(*FAIL:x)", "(*THEN:", "(*PRUNE:", "(*SKIP:", "(*mark:x)",
    "(*LIMIT_MATCH)", "(*LIMIT_MATCH=)", "(*LIMIT_MATCH=x)", "(*LIMIT_DEPTH=)",
    "(*LIMIT_HEAP=)", "(*BSR)", "(*BSR_X)", "(*NUL", "(*alpha)", "(*pla)", "(*pla:",
    "(*atomic)", "(*script_run)", "(*sr", "(*napla)", "(*positive_lookahead)",
    // option settings
    "(?-", "(?-)", "(?i-)", "(?i-s-m)", "(?-i-s)", "(?^i-s)", "(?J-)", "(?n-)",
    "(?im-sx", "(?xxx)", "(?i", "(?-i", "(?a)", "(?u)", "(?U)",
    // extended classes (PCRE2_ALT_EXTENDED_CLASS / (?[...]) )
    "(?[", "(?[]", "(?[a]", "(?[[a]", "(?[[a]]", "(?[[a]&&]", "(?[[a]||[b]]",
    "(?[[a]--]", "(?[[a]~~]", "(?[[a]&&[b]", "(?[&&[a]]", "(?[[a][b]]", "(?[[a]&&[b]|]",
    "(?[[[[[[[[[[a]]]]]]]]]]", "(?[ ]", "(?[[a]] ", "(?[[a]]x",
    "[[a]&&]", "[&&[a]]", "[[a]--[b]&&[c]]", "[[a]", "[[[[[[[[[[a]]]]]]]]]]",
    // misc
    "a\\Q", "\\E", "\\Qa", "(?|(a)(?<n>b)|(?<n>c))", "(?|(?<a>x)|(?<b>y))",
    "\\K", "(?=\\K)", "(?<!\\K)", "a{1,2}?+", "(?i)(?-i)(?i)",
    // escapes that are invalid inside a character class
    "[\\A]", "[\\B]", "[\\b]", "[\\Z]", "[\\z]", "[\\G]", "[\\K]", "[\\C]", "[\\X]",
    "[\\R]", "[\\1]", "[\\g1]", "[\\g{1}]", "[\\k<n>]", "[\\Q\\E]", "[\\E]",
    // POSIX class / collating element placement
    "[:alpha:]", "x[:alpha:]", "[[:alpha]]", "[[.a.]]", "[[=a=]]", "[a[:alpha]b]",
    "[[:]]", "[[::]]", "[[:^:]]",
    // (?+ / (?- relative references
    "(?+)", "(?+a)", "(?+x)", "(?-)", "(?-a)", "(?+ 1)", "(?-1", "(?+1",
    // \k / \g followed by junk
    "\\kx", "\\k9", "\\gx", "\\g}", "\\g>", "\\k>", "\\k}",
    // verb names with escapes (needs PCRE2_ALT_VERBNAMES for some)
    "(*MARK:\\q)", "(*MARK:\\)", "(*MARK:a\\qb)", "(*THEN:\\x{110000})",
    "(*PRUNE:\\Q)", "(*SKIP:\\E)",
    // conditions / VERSION
    "(?(VERSION>=99999999999)a)", "(?(VERSION>", "(?(VERSION", "(?(VERSION>=1.a)a)",
    "(?(VERSION=)a)", "(?(VERSION<1)a)",
    // lookbehind complexity
    "(?<=(?:a|bc|def|ghij))", "(?<=(?<=a)b)", "(?<=a(?:b|cd)e)", "(?<=(a|bc)+)",
    "(?<=\\b)", "(?<=(?=a))", "(?<=(?<!a))", "(?<=(?(1)a|bc))", "(?<=(?R))",
    "(?<=a{2,4}b{2,4}c{2,4}d{2,4}e{2,4}f{2,4})",
    // extended classes, more shapes
    "(?[[a]&&[b]--[c]]", "(?[[a]&&[b]--[c]])", "(?[[a]||[b]])", "(?[[a]!!]",
    "(?[[a] [b]])", "(?[[a]&&[b]&&]", "(?[[a]-]", "(?[[a]~]", "(?[[a]&]",
    "(?[[]])", "(?[[^]])", "(?[a])", "(?[1])", "(?[(a)])",
    "[[a]&&[b]--[c]]", "[[a]||[b]]", "[[a]&&]", "[[]]", "[[^]]", "[[a] [b]]",
    // \x / \o / \u / \N boundaries
    "\\x{0}", "\\x{-1}", "\\x{ 1}", "\\o{ 1}", "\\o{-1}", "\\N{U+ 1}", "\\N{U+-1}",
    "\\u{ 1}", "\\u{-1}", "\\N{U+d800}", "\\N{U+dfff}", "\\N{U+e000}",
    // (*scs:...) capture lists and recursion arguments (parse_capture_list)
    "(*scs:x)", "(*scs:)", "(*scs:(", "(*scs:(x", "(*scs:(x)", "(*scs:(1)a)",
    "(*scs:(<n>)a)", "(*scs:(999999999999)a)", "(*scs:(0)a)", "(*scs:(-1)a)",
    "(*scs:('n')a)", "(*scs:(<n>,<m>)a)", "(*scs:(1,2)a)", "(*scs:(1,)a)",
    "(?&n(1)x)", "(?1(", "(?1(1)x)", "(?1(x)y)", "(?R(1)x)", "(?+1(1)x)",
    // verb names containing escapes (ERR40 with PCRE2_ALT_VERBNAMES)
    "(*MARK:\\d)", "(*THEN:\\w)", "(*PRUNE:\\s)", "(*SKIP:\\D)", "(*MARK:\\p{L})",
    "(*MARK:\\Q\\E)", "(*MARK:\\x41)",
    // extended-class operator repetition and deep nesting
    "[[a]&&&[b]]", "[[a]---[b]]", "[[a]~~~[b]]", "[[a]&&&&[b]]",
    "[[[[[[[[[[[[[[[[[[[[a]]]]]]]]]]]]]]]]]]]]",
    "(?[[[[[[[[[[[[[[[[[[[[a]]]]]]]]]]]]]]]]]]]])",
    // numbers followed by junk instead of a terminator (ERR119)
    "\\g<12x>", "\\g'12x'", "\\g<12", "\\g'12", "(?12x)", "(?-12x)", "(?+12x)",
    "(*scs:(12x)a)", "(?&n(12x)a)",
    // subpattern numbers
    "(?99999999999)", "(?-99999999999)", "(?+99999999999)", "(?R99999)",
    "\\99999999999", "(?P>99999999999)",
];

/// Patterns × option/extra-option contexts that unlock further error branches.
const ERR_CONTEXTS: &[(&str, u32, u32)] = &[
    ("plain", 0, 0),
    ("UTF", PCRE2_UTF, 0),
    ("UCP", PCRE2_UCP, 0),
    ("UTF|UCP", PCRE2_UTF | PCRE2_UCP, 0),
    ("NEVER_UTF", PCRE2_NEVER_UTF, 0),
    ("NEVER_UCP", PCRE2_NEVER_UCP, 0),
    ("NEVER_BACKSLASH_C", PCRE2_NEVER_BACKSLASH_C, 0),
    ("ALT_BSUX", PCRE2_ALT_BSUX, 0),
    ("ALT_VERBNAMES", PCRE2_ALT_VERBNAMES, 0),
    ("ALT_EXTENDED_CLASS", PCRE2_ALT_EXTENDED_CLASS, 0),
    ("EXTENDED", PCRE2_EXTENDED, 0),
    ("EXTENDED_MORE", PCRE2_EXTENDED_MORE, 0),
    ("DUPNAMES", PCRE2_DUPNAMES, 0),
    ("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT, 0),
    ("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE, 0),
    ("LITERAL", PCRE2_LITERAL, 0),
    ("LITERAL+DOTALL", PCRE2_LITERAL | PCRE2_DOTALL, 0),
    ("x:SURROGATE", 0, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
    ("x:SURROGATE+UTF", PCRE2_UTF, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
    ("x:BAD_ESCAPE_LIT", 0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
    ("x:PYTHON_OCTAL", 0, PCRE2_EXTRA_PYTHON_OCTAL),
    ("x:NO_BS0", 0, PCRE2_EXTRA_NO_BS0),
    ("x:NEVER_CALLOUT", 0, PCRE2_EXTRA_NEVER_CALLOUT),
    ("x:ALLOW_LOOKAROUND_BSK", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
    ("x:TURKISH", 0, PCRE2_EXTRA_TURKISH_CASING),
    ("x:TURKISH+UTF", PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING),
    ("x:TURKISH+UCP", PCRE2_UCP, PCRE2_EXTRA_TURKISH_CASING),
    (
        "x:TURKISH+RESTRICT+UTF",
        PCRE2_UTF,
        PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT,
    ),
    ("x:ALT_BSUX", 0, PCRE2_EXTRA_ALT_BSUX),
    ("x:ESCAPED_CR_IS_LF", 0, PCRE2_EXTRA_ESCAPED_CR_IS_LF),
    ("x:MATCH_WORD", 0, PCRE2_EXTRA_MATCH_WORD),
    ("x:MATCH_LINE", 0, PCRE2_EXTRA_MATCH_LINE),
    ("x:ASCII_ALL", 0, 0x0000_0F80),
    ("ALT_EXT_CLASS+UTF", PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF, 0),
    ("ALT_EXT_CLASS+UCP", PCRE2_ALT_EXTENDED_CLASS | PCRE2_UCP, 0),
    ("ALT_VERBNAMES+UTF", PCRE2_ALT_VERBNAMES | PCRE2_UTF, 0),
];

/// Contexts that additionally clamp a compile-context limit; these unlock the
/// ERR88 / ERR101 branches ("pattern string is longer than the limit set by the
/// application" and "compiled pattern would be longer than the limit").
const NEST_LIMITS: &[u32] = &[0, 1, 5, 250, 5000, 65535];

const LIMIT_CONTEXTS: &[(&str, Sz, Sz)] = &[
    ("maxlen=0", 0, Sz::MAX),
    ("maxlen=1", 1, Sz::MAX),
    ("maxlen=3", 3, Sz::MAX),
    ("maxlen=8", 8, Sz::MAX),
    ("maxcompiled=0", Sz::MAX, 0),
    ("maxcompiled=1", Sz::MAX, 1),
    ("maxcompiled=20", Sz::MAX, 20),
    ("maxcompiled=64", Sz::MAX, 64),
    ("maxcompiled=200", Sz::MAX, 200),
];

/// Guard against the corpus silently losing coverage.
const MIN_DISTINCT_CODES: usize = 100;

static GUARD_DEPTH: AtomicUsize = AtomicUsize::new(0);
extern "C" fn guard_deny(d: u32, _u: *mut c_void) -> c_int {
    if d as usize >= GUARD_DEPTH.load(Ordering::Relaxed) { 1 } else { 0 }
}

/// An allocator that succeeds only for the first `ALLOC_BUDGET` calls, so the
/// `ERR21` ("failed to allocate heap memory") branches become reachable.
static ALLOC_BUDGET: AtomicUsize = AtomicUsize::new(usize::MAX);
extern "C" fn budget_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    loop {
        let cur = ALLOC_BUDGET.load(Ordering::Relaxed);
        if cur == 0 {
            return std::ptr::null_mut();
        }
        let next = if cur == usize::MAX { usize::MAX } else { cur - 1 };
        if ALLOC_BUDGET
            .compare_exchange(cur, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break;
        }
    }
    unsafe { libc_malloc(n) }
}
extern "C" fn budget_free(p: *mut c_void, _d: *mut c_void) {
    unsafe { libc_free(p) }
}
unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

type AllocRow = (&'static str, usize, c_int, Sz);
static C_ALLOC_FAIL: Mutex<Vec<AllocRow>> = Mutex::new(Vec::new());
static R_ALLOC_FAIL: Mutex<Vec<AllocRow>> = Mutex::new(Vec::new());

#[test]
fn compile_error_codes_match() {
    let (c, r) = pair();
    let mut seen: BTreeMap<c_int, (String, String)> = BTreeMap::new();
    let mut checked = 0usize;

    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };

    // A few structural limits also produce distinct codes.
    let deep_parens = "(".repeat(300) + &")".repeat(300);
    let many_groups = "(a)".repeat(70000);
    let many_names: String = (0..10100).map(|i| format!("(?<n{i}>a)")).collect();
    let long_name = format!("(?<{}>a)", "n".repeat(200));
    let long_callout = format!("(?C{{{}}})a", "x".repeat(300));
    let long_mark = format!("(*MARK:{})a", "m".repeat(300));
    let big_class = format!("[{}]", "a-z".repeat(3000));
    let long_lookbehind = format!("(?<={})a", "a".repeat(300));
    let vlb = format!("(?<=a{{1,{}}})b", 300);
    let deep_jx = "(?|".repeat(300) + &")".repeat(300);
    let deep_ext_class = "(?[".to_string() + &"[".repeat(60) + "a" + &"]".repeat(60) + "])";
    let long_verb = format!("(*THEN:{})a", "t".repeat(300));
    // Lookbehind longer than LOOKBEHIND_MAX (65535) -> ERR87.
    let huge_lookbehind = "(?<=a{70000})b".to_string();
    // Deeply nested (?| / (?J: / (?x: groups -> ERR84 (needs a raised
    // parens_nest_limit so that ERR19 does not fire first).
    let deep_bar = "(?|".repeat(1000) + &")".repeat(1000);
    let deep_jcolon = "(?J:".repeat(1000) + &")".repeat(1000);
    let deep_xcolon = "(?x:".repeat(1000) + &")".repeat(1000);
    let deep_asr = "(*asr:".repeat(1000) + &")".repeat(1000);
    // Very long/complex lookbehind -> ERR35 ("lookbehind is too complicated").
    let complex_lb = format!("(?<={})x", "(?|a|bc)".repeat(1200));
    let extra: Vec<&str> = vec![
        &deep_parens,
        &many_groups,
        &many_names,
        &long_name,
        &long_callout,
        &long_mark,
        &big_class,
        &long_lookbehind,
        &vlb,
        &deep_jx,
        &deep_ext_class,
        &long_verb,
        &huge_lookbehind,
        &deep_bar,
        &deep_jcolon,
        &deep_xcolon,
        &deep_asr,
        &complex_lb,
    ];

    for &(cname, opts, xopts) in ERR_CONTEXTS {
        unsafe { (c.set_compile_extra_options)(cc, xopts) };
        unsafe { (r.set_compile_extra_options)(rc, xopts) };
        for pat in ERR_PATTERNS
            .iter()
            .copied()
            .chain(PATTERNS.iter().copied())
            .chain(extra.iter().copied())
        {
            let mut v = pat.as_bytes().to_vec();
            v.push(0);
            let mut ce: c_int = 0x5A5A;
            let mut co: Sz = 0x5A5A;
            let mut re: c_int = 0x5A5A;
            let mut ro: Sz = 0x5A5A;
            let ccode =
                unsafe { (c.compile)(v.as_ptr(), pat.len(), opts, &mut ce, &mut co, cc) };
            let rcode =
                unsafe { (r.compile)(v.as_ptr(), pat.len(), opts, &mut re, &mut ro, rc) };
            checked += 1;
            assert_eq!(
                (ce, co, ccode.is_null()),
                (re, ro, rcode.is_null()),
                "compile error divergence\n  pattern = {:?}\n  context = {cname} (opts=0x{opts:08x} xopts=0x{xopts:08x})\n  C    : err={ce} off={co} null={}\n  RUST : err={re} off={ro} null={}",
                if pat.len() > 120 { format!("<{} bytes>", pat.len()) } else { pat.to_string() },
                ccode.is_null(),
                rcode.is_null()
            );
            if ccode.is_null() {
                seen.entry(ce).or_insert_with(|| {
                    (
                        if pat.len() > 60 {
                            format!("<{} byte pattern>", pat.len())
                        } else {
                            pat.to_string()
                        },
                        cname.to_string(),
                    )
                });
            } else {
                // Successful compiles must also agree byte-for-byte.
                assert_eq!(
                    unsafe { serialize_bytes(c, ccode) },
                    unsafe { serialize_bytes(r, rcode) },
                    "serialized divergence for {pat:?} in {cname}"
                );
                unsafe { (c.code_free)(ccode) };
                unsafe { (r.code_free)(rcode) };
            }
        }
    }
    // ---- Stage 2: a compile-recursion guard that denies past a given depth,
    // which is the only way to reach ERR33 ("parentheses are too deeply nested
    // (stack check)", pcre2_compile.c:8601). ----
    for depth in [0usize, 1, 2, 4, 8] {
        GUARD_DEPTH.store(depth, Ordering::Relaxed);
        unsafe { (c.set_compile_extra_options)(cc, 0) };
        unsafe { (r.set_compile_extra_options)(rc, 0) };
        unsafe {
            (c.set_compile_recursion_guard)(cc, Some(guard_deny), std::ptr::null_mut())
        };
        unsafe {
            (r.set_compile_recursion_guard)(rc, Some(guard_deny), std::ptr::null_mut())
        };
        for pat in ERR_PATTERNS.iter().copied().chain(PATTERNS.iter().copied()) {
            let mut v = pat.as_bytes().to_vec();
            v.push(0);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let ccode = unsafe { (c.compile)(v.as_ptr(), pat.len(), 0, &mut ce, &mut co, cc) };
            let rcode = unsafe { (r.compile)(v.as_ptr(), pat.len(), 0, &mut re, &mut ro, rc) };
            checked += 1;
            assert_eq!(
                (ce, co, ccode.is_null()),
                (re, ro, rcode.is_null()),
                "stack-guard divergence pattern={pat:?} depth={depth}"
            );
            if ccode.is_null() {
                seen.entry(ce)
                    .or_insert_with(|| (pat.to_string(), format!("stack guard depth={depth}")));
            } else {
                unsafe { (c.code_free)(ccode) };
                unsafe { (r.code_free)(rcode) };
            }
        }
    }
    unsafe { (c.set_compile_recursion_guard)(cc, None, std::ptr::null_mut()) };
    unsafe { (r.set_compile_recursion_guard)(rc, None, std::ptr::null_mut()) };
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };

    // ---- Stage 3: a failing allocator, which is the only way to reach ERR21
    // ("failed to allocate heap memory"). ----
    for budget in [0usize, 1, 2, 3, 5] {
        for a in [c, r] {
            ALLOC_BUDGET.store(usize::MAX, Ordering::Relaxed);
            let g = unsafe {
                (a.general_context_create)(
                    Some(budget_malloc),
                    Some(budget_free),
                    std::ptr::null_mut(),
                )
            };
            if g.is_null() {
                continue;
            }
            let ctx = unsafe { (a.compile_context_create)(g) };
            if ctx.is_null() {
                unsafe { (a.general_context_free)(g) };
                continue;
            }
            for pat in ["abc", "(a)(b)(c)", "(?<n>x)+", "[a-z]{2,4}"] {
                let mut v = pat.as_bytes().to_vec();
                v.push(0);
                let mut e: c_int = 0;
                let mut o: Sz = 0;
                ALLOC_BUDGET.store(budget, Ordering::Relaxed);
                let code = unsafe { (a.compile)(v.as_ptr(), pat.len(), 0, &mut e, &mut o, ctx) };
                ALLOC_BUDGET.store(usize::MAX, Ordering::Relaxed);
                if code.is_null() {
                    if std::ptr::eq(a, c) {
                        C_ALLOC_FAIL.lock().unwrap().push((pat, budget, e, o));
                        seen.entry(e)
                            .or_insert_with(|| (pat.to_string(), format!("alloc budget={budget}")));
                    } else {
                        R_ALLOC_FAIL.lock().unwrap().push((pat, budget, e, o));
                    }
                } else {
                    if std::ptr::eq(a, c) {
                        C_ALLOC_FAIL.lock().unwrap().push((pat, budget, 0, 0));
                    } else {
                        R_ALLOC_FAIL.lock().unwrap().push((pat, budget, 0, 0));
                    }
                    unsafe { (a.code_free)(code) };
                }
                checked += 1;
            }
            ALLOC_BUDGET.store(usize::MAX, Ordering::Relaxed);
            unsafe { (a.compile_context_free)(ctx) };
            unsafe { (a.general_context_free)(g) };
        }
    }
    assert_eq!(
        *C_ALLOC_FAIL.lock().unwrap(),
        *R_ALLOC_FAIL.lock().unwrap(),
        "failing-allocator divergence (pattern, budget, errorcode, erroroffset)"
    );

    // ---- Stage 3b: raised parens_nest_limit, needed to reach ERR84 (the
    // "(?| and/or (?J: or (?x: parentheses are too deeply nested" branch) which
    // otherwise hides behind ERR19. ----
    let nl_c = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let nl_r = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for &nlim in NEST_LIMITS {
        unsafe { (c.set_parens_nest_limit)(nl_c, nlim) };
        unsafe { (r.set_parens_nest_limit)(nl_r, nlim) };
        for pat in ERR_PATTERNS.iter().copied().chain(extra.iter().copied()) {
            let mut v = pat.as_bytes().to_vec();
            v.push(0);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let ccode = unsafe { (c.compile)(v.as_ptr(), pat.len(), 0, &mut ce, &mut co, nl_c) };
            let rcode = unsafe { (r.compile)(v.as_ptr(), pat.len(), 0, &mut re, &mut ro, nl_r) };
            checked += 1;
            assert_eq!(
                (ce, co, ccode.is_null()),
                (re, ro, rcode.is_null()),
                "nest-limit divergence pattern={} nlim={nlim}",
                if pat.len() > 80 { format!("<{} bytes>", pat.len()) } else { pat.to_string() }
            );
            if ccode.is_null() {
                seen.entry(ce).or_insert_with(|| {
                    (
                        if pat.len() > 60 { format!("<{} byte pattern>", pat.len()) } else { pat.to_string() },
                        format!("parens_nest_limit={nlim}"),
                    )
                });
            } else {
                unsafe { (c.code_free)(ccode) };
                unsafe { (r.code_free)(rcode) };
            }
        }
    }
    unsafe { (c.compile_context_free)(nl_c) };
    unsafe { (r.compile_context_free)(nl_r) };

    // ---- Stage 4: compile-context length limits (ERR88 / ERR101). ----
    let lc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let lr = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for &(lname, maxlen, maxcomp) in LIMIT_CONTEXTS {
        unsafe {
            (c.set_max_pattern_length)(lc, maxlen);
            (r.set_max_pattern_length)(lr, maxlen);
            (c.set_max_pattern_compiled_length)(lc, maxcomp);
            (r.set_max_pattern_compiled_length)(lr, maxcomp);
        }
        for pat in ERR_PATTERNS.iter().copied().chain(PATTERNS.iter().copied()) {
            let mut v = pat.as_bytes().to_vec();
            v.push(0);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let ccode = unsafe { (c.compile)(v.as_ptr(), pat.len(), 0, &mut ce, &mut co, lc) };
            let rcode = unsafe { (r.compile)(v.as_ptr(), pat.len(), 0, &mut re, &mut ro, lr) };
            checked += 1;
            assert_eq!(
                (ce, co, ccode.is_null()),
                (re, ro, rcode.is_null()),
                "limit-context divergence pattern={pat:?} ctx={lname}"
            );
            if ccode.is_null() {
                seen.entry(ce).or_insert_with(|| (pat.to_string(), lname.to_string()));
            } else {
                unsafe { (c.code_free)(ccode) };
                unsafe { (r.code_free)(rcode) };
            }
        }
    }
    unsafe { (c.compile_context_free)(lc) };
    unsafe { (r.compile_context_free)(lr) };

    // ---- Stage 5: argument-level rejections (ERR16 NULL pattern, ERR17 bad
    // option bits, ERR120 NULL erroroffset). ----
    {
        let pz = b"abc\0";
        // ERR16: NULL pattern with non-zero length
        let mut ce: c_int = 0;
        let mut co: Sz = 0;
        let mut re: c_int = 0;
        let mut ro: Sz = 0;
        let a = unsafe {
            (c.compile)(std::ptr::null(), 3, 0, &mut ce, &mut co, std::ptr::null_mut())
        };
        let b = unsafe {
            (r.compile)(std::ptr::null(), 3, 0, &mut re, &mut ro, std::ptr::null_mut())
        };
        assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()), "NULL pattern len 3");
        seen.entry(ce).or_insert_with(|| ("<NULL, len 3>".into(), "arg check".into()));
        checked += 1;
        // ERR17: undefined option bit
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe {
                (c.compile)(pz.as_ptr(), 3, opt, &mut ce, &mut co, std::ptr::null_mut())
            };
            let b = unsafe {
                (r.compile)(pz.as_ptr(), 3, opt, &mut re, &mut ro, std::ptr::null_mut())
            };
            assert_eq!(
                (ce, co, a.is_null()),
                (re, ro, b.is_null()),
                "single option bit 0x{opt:08x}"
            );
            checked += 1;
            if a.is_null() {
                seen.entry(ce)
                    .or_insert_with(|| (format!("opts=0x{opt:08x}"), "arg check".into()));
            } else {
                unsafe { (c.code_free)(a) };
                unsafe { (r.code_free)(b) };
            }
        }
        // ERR120: erroroffset == NULL
        let mut ce: c_int = 0;
        let mut re: c_int = 0;
        let a = unsafe {
            (c.compile)(pz.as_ptr(), 3, 0, &mut ce, std::ptr::null_mut(), std::ptr::null_mut())
        };
        let b = unsafe {
            (r.compile)(pz.as_ptr(), 3, 0, &mut re, std::ptr::null_mut(), std::ptr::null_mut())
        };
        assert_eq!((ce, a.is_null()), (re, b.is_null()), "NULL erroroffset");
        seen.entry(ce).or_insert_with(|| ("<NULL erroroffset>".into(), "arg check".into()));
        checked += 1;
    }

    if std::env::var_os("PRINT_CODES").is_some() {
        for (code, (pat, ctx)) in &seen {
            let mut buf = [0u8; 256];
            let n = unsafe { (c.get_error_message)(*code, buf.as_mut_ptr(), 256) };
            let msg = if n > 0 {
                String::from_utf8_lossy(&buf[..n as usize]).to_string()
            } else {
                String::new()
            };
            println!("| C{code} | `{pat}` | {code} | {ctx} | {msg} |");
        }
    }
    eprintln!(
        "compile-error corpus: {checked} (pattern, context) pairs, {} distinct error codes",
        seen.len()
    );
    assert!(
        seen.len() >= MIN_DISTINCT_CODES,
        "corpus only reaches {} distinct compile error codes (want >= {MIN_DISTINCT_CODES}): {:?}",
        seen.len(),
        seen.keys().collect::<Vec<_>>()
    );
}
