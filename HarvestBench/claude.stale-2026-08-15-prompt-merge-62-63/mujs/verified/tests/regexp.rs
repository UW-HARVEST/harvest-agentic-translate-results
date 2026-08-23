//! Phase B rows B1..B18 plus the `regexp.c` ERRORS rows — differential tests
//! for the MuJS regexp engine.
//!
//! Every call crosses the real FFI boundary into BOTH shared libraries
//! (`c_src/build/libmujs.so` and `target/<profile>/libmujs.so`) via the
//! `common::both()` harness.
//!
//! `Reprog *` and the `sp`/`ep` pointers inside `Resub` are raw addresses and
//! can never be equal between the two libraries, so *nothing* compares them
//! directly: compile results are reduced to `(is_null, error string)` and exec
//! results to `(ret, nsub, [Option<(sp-offset, ep-offset, bytes)>; 16])` by
//! `compile_result()` / `exec_result()`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ */
/*  Address-independent result extraction                             */
/* ------------------------------------------------------------------ */

/// Sentinel written into `*errorp` before every `regcomp` call: both
/// libraries must overwrite it (with NULL on success, with the message on
/// failure), so a leftover sentinel is itself a reportable divergence.
const ERRP_POISON: usize = 1;

/// `(compile returned NULL, *errorp as a string)` — no addresses.
type CompileOut = (bool, Option<String>);

/// `(regexec return value, sub.nsub, one entry per REG_MAXSUB slot)`.
/// A slot is `None` when `sp == NULL`, else `Some((sp-base, ep-base, bytes))`.
type ExecOut = (c_int, c_int, Vec<Option<(isize, isize, Vec<u8>)>>);

unsafe fn errp_to_string(errp: *const c_char) -> Option<String> {
    if errp as usize == ERRP_POISON {
        Some("<*errorp WAS NOT WRITTEN>".to_string())
    } else {
        cstr_string(errp)
    }
}

/// `js_regcomp` — returns the raw prog (for freeing) and its derived result.
unsafe fn compile_result(api: &Api, pattern: &CString, cflags: c_int) -> (*mut c_void, CompileOut) {
    let mut errp: *const c_char = ERRP_POISON as *const c_char;
    let prog = (api.js_regcomp)(pattern.as_ptr(), cflags, &mut errp);
    (prog, (prog.is_null(), errp_to_string(errp)))
}

/// `js_regcompx` with a custom allocator — same derived result.
unsafe fn compile_result_x(
    api: &Api,
    alloc: Alloc,
    ctx: *mut c_void,
    pattern: &CString,
    cflags: c_int,
) -> (*mut c_void, CompileOut) {
    let mut errp: *const c_char = ERRP_POISON as *const c_char;
    let prog = (api.js_regcompx)(alloc, ctx, pattern.as_ptr(), cflags, &mut errp);
    (prog, (prog.is_null(), errp_to_string(errp)))
}

/// Run `js_regexec` and reduce everything observable to address-independent
/// values. `sub == None` passes a NULL `Resub *` (row B7); `nsub` is then
/// reported as `-1` and the slot vector is empty.
fn exec_result(
    api: &Api,
    prog: *mut c_void,
    subject: &CString,
    sub: Option<&mut Resub>,
    eflags: c_int,
) -> ExecOut {
    let base = subject.as_ptr();
    let len = subject.as_bytes().len() as isize;
    unsafe {
        match sub {
            None => {
                let ret = (api.js_regexec)(prog, base, std::ptr::null_mut(), eflags);
                (ret, -1, Vec::new())
            }
            Some(s) => {
                let p: *mut Resub = s;
                let ret = (api.js_regexec)(prog, base, p, eflags);
                let mut slots: Vec<Option<(isize, isize, Vec<u8>)>> = Vec::with_capacity(REG_MAXSUB);
                for i in 0..REG_MAXSUB {
                    let sp = (*p).sub[i].sp;
                    let ep = (*p).sub[i].ep;
                    if sp.is_null() {
                        slots.push(None);
                        continue;
                    }
                    let so = sp as isize - base as isize;
                    // A NULL `ep` has no meaningful offset: use a sentinel so
                    // the comparison stays address independent.
                    let eo = if ep.is_null() {
                        isize::MIN
                    } else {
                        ep as isize - base as isize
                    };
                    let bytes = if eo != isize::MIN && so >= 0 && eo >= so && eo <= len {
                        std::slice::from_raw_parts(
                            base.add(so as usize) as *const u8,
                            (eo - so) as usize,
                        )
                        .to_vec()
                    } else {
                        b"<span outside subject>".to_vec()
                    };
                    slots.push(Some((so, eo, bytes)));
                }
                (ret, (*p).nsub, slots)
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/*  Small utilities                                                    */
/* ------------------------------------------------------------------ */

/// Shorten a (possibly huge) pattern for assertion messages.
fn brief(p: &str) -> String {
    if p.len() <= 64 {
        format!("{:?}", p)
    } else {
        format!(
            "{:?}...<{} bytes total>",
            p.chars().take(32).collect::<String>(),
            p.len()
        )
    }
}

/// Several tests deliberately drive the 4096-deep `count()` / `match()`
/// recursion limits inside the libraries; the default 2 MiB test-thread stack
/// is not enough head-room for that, so run those bodies on a big stack.
fn with_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let h = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .expect("spawn big-stack thread");
    if let Err(e) = h.join() {
        std::panic::resume_unwind(e);
    }
}

/* ------------------------------------------------------------------ */
/*  Corpora                                                            */
/* ------------------------------------------------------------------ */

/// ~60 hand written patterns: literals, `.`, `^`, `$`, `\b`, `\B`, character
/// classes, every quantifier, alternation, groups, lookahead, back-references
/// and escapes.  (CONFIGS rows B9-B13.)
const PATTERNS: &[&str] = &[
    /* literals */
    "a",
    "abc",
    "aBc",
    "abcabc",
    /* any */
    ".",
    "..",
    "a.c",
    /* anchors */
    "^a",
    "a$",
    "^abc$",
    "^$",
    "^.$",
    /* word boundaries */
    r"\ba",
    r"a\b",
    r"\Ba",
    r"a\Bb",
    r"\bworld\b",
    /* character classes */
    "[abc]",
    "[^abc]",
    "[a-c]",
    "[a-cA-C]",
    "[]",
    "[^]",
    "[-a]",
    "[a-]",
    r"[\d]",
    r"[\w-]",
    r"[\s]",
    r"[^\d]",
    /* class escapes */
    r"\d",
    r"\d+",
    r"\D",
    r"\s",
    r"\S+",
    r"\w+",
    r"\W",
    /* quantifiers */
    "a*",
    "a+",
    "a?",
    "a{2}",
    "a{2,}",
    "a{1,3}",
    "a*?",
    "a+?",
    "a??",
    "a{2,4}?",
    "[a-c]{2,3}",
    "(?:abc)?",
    /* alternation & groups */
    "ab|cd",
    "^a|b$",
    "a|b|c|",
    "(a)",
    "(ab)+",
    "(a)(b)(c)",
    "((a)(b))",
    "(?:ab)+",
    "(a|b)c",
    /* lookahead */
    "(?=a)",
    "a(?=b)",
    "a(?!b)",
    "(?!x)a",
    "(?=.*b)a",
    /* back-references */
    r"(a)\1",
    r"(a)(b)\2\1",
    r"(ab)\1",
    r"(a)|\1",
    /* escapes */
    r"\x41\x42",
    r"[A-Z]",
    r"\cA",
    r"\0",
    r"\n",
    "a\\nb",
    r"\f\r\t\v",
    r"\$\^\.\*\+\?\(\)\[\]\{\}\|\-",
    r"\/",
];

/// Subject shapes: empty, ASCII, mixed case, embedded newlines, UTF-8,
/// 200 characters.  (CONFIGS row B17.)
const SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "abc",
    "ABC",
    "aBc",
    "abcabc",
    "aaa",
    "aaaa",
    "ab\ncd",
    "\n",
    "\na",
    "a\n",
    "line1\nline2\nline3",
    "hello world",
    "The quick brown fox",
    "0123456789",
    "a-b_c d",
    "_",
    " ",
    "\u{e9}",
    "h\u{e9}llo w\u{f6}rld",
    "\u{65e5}\u{672c}\u{8a9e}abc",
    "\u{10ffff}x",
    "aAbBcC",
    "xyz",
];

/// A 200-character subject (kept ASCII so byte length == char length).
fn long_subject() -> String {
    let unit = "abcABC012 xyz\n";
    let mut s = String::new();
    while s.len() < 200 {
        s.push_str(unit);
    }
    s.truncate(200);
    s
}

fn all_subjects() -> Vec<String> {
    let mut v: Vec<String> = SUBJECTS.iter().map(|s| s.to_string()).collect();
    v.push(long_subject());
    v
}

const CFLAGS_ALL: [c_int; 8] = [
    0,
    REG_ICASE,
    REG_NEWLINE,
    REG_ICASE | REG_NEWLINE,
    8,
    16,
    -1,
    0x7fffffff,
];
const CFLAGS_VALID: [c_int; 4] = [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE];
const EFLAGS_ALL: [c_int; 4] = [0, REG_NOTBOL, 8, -1];

/* ------------------------------------------------------------------ */
/*  B1-B6, B9-B13, B17                                                 */
/* ------------------------------------------------------------------ */

#[test]
fn b1_b6_cflags_eflags_cross_product() {
    let subjects = all_subjects();
    let csubjects: Vec<CString> = subjects.iter().map(|s| cs(s)).collect();
    // (subject index, eflags) in a fixed order so both sides line up.
    let mut cases: Vec<(usize, c_int)> = Vec::new();
    for si in 0..csubjects.len() {
        for &ef in EFLAGS_ALL.iter() {
            cases.push((si, ef));
        }
    }

    for pat in PATTERNS {
        let cpat = cs(pat);
        for &cflags in CFLAGS_ALL.iter() {
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, cflags);
                let mut execs: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    for &(si, ef) in &cases {
                        let mut sub = Resub::default();
                        execs.push(exec_result(api, prog, &csubjects[si], Some(&mut sub), ef));
                    }
                    (api.js_regfree)(prog);
                }
                (comp, execs)
            });

            assert_eq!(
                c.0, r.0,
                "js_regcomp DIVERGENCE: pattern={} cflags={:#x}: C=(null={},err={:?}) Rust=(null={},err={:?})",
                brief(pat), cflags, c.0.0, c.0.1, r.0.0, r.0.1
            );
            assert_eq!(
                c.1.len(),
                r.1.len(),
                "exec-result count DIVERGENCE: pattern={} cflags={:#x}: C={} Rust={}",
                brief(pat),
                cflags,
                c.1.len(),
                r.1.len()
            );
            for (i, &(si, ef)) in cases.iter().enumerate() {
                if i >= c.1.len() {
                    break;
                }
                assert_eq!(
                    c.1[i], r.1[i],
                    "js_regexec DIVERGENCE: pattern={} cflags={:#x} eflags={:#x} subject={}: C={:?} Rust={:?}",
                    brief(pat), cflags, ef, brief(&subjects[si]), c.1[i], r.1[i]
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B7 — sub == NULL                                                   */
/* ------------------------------------------------------------------ */

#[test]
fn b7_regexec_null_sub() {
    let subjects = all_subjects();
    let csubjects: Vec<CString> = subjects.iter().map(|s| cs(s)).collect();
    let eflags_list = [0, REG_NOTBOL];
    let mut cases: Vec<(usize, c_int)> = Vec::new();
    for si in 0..csubjects.len() {
        for &ef in eflags_list.iter() {
            cases.push((si, ef));
        }
    }

    for pat in PATTERNS {
        let cpat = cs(pat);
        for &cflags in CFLAGS_ALL.iter() {
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, cflags);
                let mut execs: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    for &(si, ef) in &cases {
                        execs.push(exec_result(api, prog, &csubjects[si], None, ef));
                    }
                    (api.js_regfree)(prog);
                }
                (comp, execs)
            });

            assert_eq!(
                c.0, r.0,
                "js_regcomp DIVERGENCE (null-sub test): pattern={} cflags={:#x}: C={:?} Rust={:?}",
                brief(pat), cflags, c.0, r.0
            );
            for (i, &(si, ef)) in cases.iter().enumerate() {
                if i >= c.1.len() || i >= r.1.len() {
                    break;
                }
                assert_eq!(
                    c.1[i], r.1[i],
                    "js_regexec(sub=NULL) DIVERGENCE: pattern={} cflags={:#x} eflags={:#x} subject={}: C={:?} Rust={:?}",
                    brief(pat), cflags, ef, brief(&subjects[si]), c.1[i], r.1[i]
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B8 — nsub handling                                                 */
/* ------------------------------------------------------------------ */

#[test]
fn b8_nsub_handling() {
    // 0..17 capture groups (16 and 17 must fail with "too many captures").
    let patterns: Vec<String> = (0..=17).map(|n| "(a)".repeat(n)).collect();
    let subjects: Vec<String> = vec![
        String::new(),
        "abc".to_string(),
        "a".repeat(20),
        "aaaaaaaaaaaaaaaaaaaaXaaa".to_string(),
    ];
    let csubjects: Vec<CString> = subjects.iter().map(|s| cs(s)).collect();
    let preset_nsub = [0, 1, 99];

    let mut cases: Vec<(usize, c_int, c_int)> = Vec::new(); // subject, preset nsub, eflags
    for si in 0..csubjects.len() {
        for &pn in preset_nsub.iter() {
            for &ef in [0, REG_NOTBOL].iter() {
                cases.push((si, pn, ef));
            }
        }
    }

    for pat in &patterns {
        let cpat = cs(pat);
        for &cflags in CFLAGS_VALID.iter() {
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, cflags);
                let mut execs: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    for &(si, pn, ef) in &cases {
                        let mut sub = Resub::default();
                        sub.nsub = pn; // deliberately wrong on input
                        execs.push(exec_result(api, prog, &csubjects[si], Some(&mut sub), ef));
                    }
                    (api.js_regfree)(prog);
                }
                (comp, execs)
            });

            assert_eq!(
                c.0, r.0,
                "js_regcomp DIVERGENCE (nsub test): pattern={} cflags={:#x}: C={:?} Rust={:?}",
                brief(pat), cflags, c.0, r.0
            );
            for (i, &(si, pn, ef)) in cases.iter().enumerate() {
                if i >= c.1.len() || i >= r.1.len() {
                    break;
                }
                assert_eq!(
                    c.1[i], r.1[i],
                    "js_regexec nsub DIVERGENCE: pattern={} cflags={:#x} eflags={:#x} preset nsub={} subject={}: C={:?} Rust={:?}",
                    brief(pat), cflags, ef, pn, brief(&subjects[si]), c.1[i], r.1[i]
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B14 — empty pattern                                                */
/* ------------------------------------------------------------------ */

#[test]
fn b14_empty_pattern() {
    let subjects = ["", "a", "abc", "\n", "\na", "hello", "\u{e9}x"];
    let csubjects: Vec<CString> = subjects.iter().map(|s| cs(s)).collect();
    let cpat = cs("");
    for &cflags in CFLAGS_VALID.iter() {
        for &ef in EFLAGS_ALL.iter() {
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, cflags);
                let mut execs: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    for s in &csubjects {
                        let mut sub = Resub::default();
                        execs.push(exec_result(api, prog, s, Some(&mut sub), ef));
                    }
                    (api.js_regfree)(prog);
                }
                (comp, execs)
            });
            assert_eq!(
                c.0, r.0,
                "empty-pattern js_regcomp DIVERGENCE: pattern=\"\" cflags={:#x} eflags={:#x}: C={:?} Rust={:?}",
                cflags, ef, c.0, r.0
            );
            for (i, s) in subjects.iter().enumerate() {
                if i >= c.1.len() || i >= r.1.len() {
                    break;
                }
                assert_eq!(
                    c.1[i], r.1[i],
                    "empty-pattern js_regexec DIVERGENCE: pattern=\"\" cflags={:#x} eflags={:#x} subject={:?}: C={:?} Rust={:?}",
                    cflags, ef, s, c.1[i], r.1[i]
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B15 — errorp == NULL                                               */
/* ------------------------------------------------------------------ */

#[test]
fn b15_null_errorp() {
    for pat in ["a+b", "(a)(b)|c", "", "(", "a\\", "[z-a]"] {
        let cpat = cs(pat);
        for &cflags in CFLAGS_VALID.iter() {
            let (c, r) = both(|api, _| unsafe {
                let prog =
                    (api.js_regcomp)(cpat.as_ptr(), cflags, std::ptr::null_mut::<*const c_char>());
                let isnull = prog.is_null();
                let mut execs: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    let subj = cs("aab");
                    let mut sub = Resub::default();
                    execs.push(exec_result(api, prog, &subj, Some(&mut sub), 0));
                    (api.js_regfree)(prog);
                }
                (isnull, execs)
            });
            assert_eq!(
                c.0, r.0,
                "js_regcomp(errorp=NULL) NULL-ness DIVERGENCE: pattern={} cflags={:#x}: C null={} Rust null={}",
                brief(pat), cflags, c.0, r.0
            );
            assert_eq!(
                c.1, r.1,
                "js_regcomp(errorp=NULL) exec DIVERGENCE: pattern={} cflags={:#x} eflags=0 subject=\"aab\": C={:?} Rust={:?}",
                brief(pat), cflags, c.1, r.1
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B16 — js_regcompx / js_regfreex with a counting allocator           */
/* ------------------------------------------------------------------ */

static mut ALLOC_CALLS: c_int = 0;

unsafe extern "C" fn counting_alloc(_ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    ALLOC_CALLS += 1;
    if n == 0 {
        libc::free(ptr);
        std::ptr::null_mut()
    } else {
        libc::realloc(ptr, n as usize)
    }
}

#[test]
fn b16_regcompx_custom_allocator() {
    let patterns = [
        "a",
        "abc",
        "",
        "[a-c]+",
        r"(a)(b)\2\1",
        "(?:ab|cd)*",
        r"^\d{2,4}$",
        r"[\w]|[^\d]",
        "(?=a)(?!b)c",
        "(",      // fails after 2 allocations
        "[z-a]",  // fails inside a character class
        "a{2,1}", // fails in the parser
    ];
    let subjects = ["", "a", "abc", "ab\ncd", "aabbcc", "12345", "\u{e9}"];
    let csubjects: Vec<CString> = subjects.iter().map(|s| cs(s)).collect();
    let ctx = 0x1234usize as *mut c_void;

    for pat in patterns {
        let cpat = cs(pat);
        for &cflags in CFLAGS_VALID.iter() {
            let (c, r) = both(|api, _| unsafe {
                ALLOC_CALLS = 0;
                /* --- custom allocator path --- */
                let (progx, compx) =
                    compile_result_x(api, Some(counting_alloc), ctx, &cpat, cflags);
                let mut execx: Vec<ExecOut> = Vec::new();
                if !progx.is_null() {
                    for s in &csubjects {
                        let mut sub = Resub::default();
                        execx.push(exec_result(api, progx, s, Some(&mut sub), 0));
                    }
                }
                (api.js_regfreex)(Some(counting_alloc), ctx, progx);
                let calls = ALLOC_CALLS;

                /* --- default allocator path --- */
                let (prog, comp) = compile_result(api, &cpat, cflags);
                let mut exec: Vec<ExecOut> = Vec::new();
                if !prog.is_null() {
                    for s in &csubjects {
                        let mut sub = Resub::default();
                        exec.push(exec_result(api, prog, s, Some(&mut sub), 0));
                    }
                    (api.js_regfree)(prog);
                }
                (compx, execx, comp, exec, calls)
            });

            // regcompx must behave exactly like regcomp inside each library.
            for (side, res) in [("C", &c), ("Rust", &r)] {
                assert_eq!(
                    res.0, res.2,
                    "{}: js_regcompx result differs from js_regcomp: pattern={} cflags={:#x}: x={:?} plain={:?}",
                    side, brief(pat), cflags, res.0, res.2
                );
                assert_eq!(
                    res.1, res.3,
                    "{}: js_regcompx exec results differ from js_regcomp: pattern={} cflags={:#x} eflags=0: x={:?} plain={:?}",
                    side, brief(pat), cflags, res.1, res.3
                );
            }
            // ... and identically in both libraries.
            assert_eq!(
                c.0, r.0,
                "js_regcompx DIVERGENCE: pattern={} cflags={:#x}: C={:?} Rust={:?}",
                brief(pat), cflags, c.0, r.0
            );
            for (i, s) in subjects.iter().enumerate() {
                if i >= c.1.len() || i >= r.1.len() {
                    break;
                }
                assert_eq!(
                    c.1[i], r.1[i],
                    "js_regexec (regcompx prog) DIVERGENCE: pattern={} cflags={:#x} eflags=0 subject={:?}: C={:?} Rust={:?}",
                    brief(pat), cflags, s, c.1[i], r.1[i]
                );
            }
            assert_eq!(
                c.4, r.4,
                "allocator call-count DIVERGENCE: pattern={} cflags={:#x} eflags=0: C={} calls, Rust={} calls",
                brief(pat), cflags, c.4, r.4
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/*  B18 — randomized pattern x subject fuzz                            */
/* ------------------------------------------------------------------ */

#[test]
fn b18_randomized_pattern_subject_fuzz() {
    const PAT_ALPHABET: &[&str] = &[
        "a", "b", "c", "A", "B", ".", "^", "$", "*", "+", "?", "|", "(", ")", "[", "]", "{", "}",
        ",", "-", "\\", "1", "2", ":", "=", "!", "d", "s", "w", "D", "S", "W", "n", "x", "u", "0",
        "9", " ",
    ];
    const SUBJ_ALPHABET: &[&str] = &["a", "b", "c", "A", "B", "\n", " ", "1", "2", "."];

    let mut rng = Rng::new(0x5EED_C0FFEE_9E37);
    for iter in 0..20000u32 {
        let plen = rng.range(1, 13) as usize;
        let mut pat = String::new();
        for _ in 0..plen {
            pat.push_str(*rng.pick(PAT_ALPHABET));
        }
        let slen = rng.range(0, 21) as usize;
        let mut subj = String::new();
        for _ in 0..slen {
            subj.push_str(*rng.pick(SUBJ_ALPHABET));
        }
        let cflags = *rng.pick(&CFLAGS_VALID);
        let eflags = *rng.pick(&EFLAGS_ALL);

        let cpat = cs(&pat);
        let csubj = cs(&subj);
        let (c, r) = both(|api, _| unsafe {
            let (prog, comp) = compile_result(api, &cpat, cflags);
            let mut exec: Option<ExecOut> = None;
            if !prog.is_null() {
                let mut sub = Resub::default();
                exec = Some(exec_result(api, prog, &csubj, Some(&mut sub), eflags));
                (api.js_regfree)(prog);
            }
            (comp, exec)
        });

        assert_eq!(
            c.0, r.0,
            "fuzz[{}] js_regcomp DIVERGENCE: pattern={} cflags={:#x} eflags={:#x} subject={}: C=(null={},err={:?}) Rust=(null={},err={:?})",
            iter, brief(&pat), cflags, eflags, brief(&subj), c.0.0, c.0.1, r.0.0, r.0.1
        );
        assert_eq!(
            c.1, r.1,
            "fuzz[{}] js_regexec DIVERGENCE: pattern={} cflags={:#x} eflags={:#x} subject={}: C={:?} Rust={:?}",
            iter, brief(&pat), cflags, eflags, brief(&subj), c.1, r.1
        );
    }
}

/* ------------------------------------------------------------------ */
/*  ERRORS — every die() message in regexp.c                           */
/* ------------------------------------------------------------------ */

/// `[` + `n` runes spaced two apart (so no two ranges can be merged by
/// `addrange`) + `]`.  Non-ASCII runes are used so that no character is a
/// character-class metacharacter (`-`, `]`, `\`, `^`).
fn spaced_class(n: u32) -> String {
    let mut s = String::from("[");
    for i in 0..n {
        s.push(char::from_u32(0x100 + 2 * i).unwrap());
    }
    s.push(']');
    s
}

#[test]
fn err_regexp_die_messages() {
    with_big_stack(|| {
        let mut table: Vec<(String, c_int, &'static str)> = vec![
            ("a\\".to_string(), 0, "unterminated escape sequence"),   // regexp.c:128
            ("a\\c".to_string(), 0, "unterminated escape sequence"),  // :138
            ("a\\x".to_string(), 0, "unterminated escape sequence"),  // :143
            ("a\\u12".to_string(), 0, "unterminated escape sequence"), // :153
            ("\\xZZ".to_string(), 0, "invalid escape sequence"),      // :101
            ("\\uZZZZ".to_string(), 0, "invalid escape sequence"),    // :101
            ("a{x}".to_string(), 0, "invalid quantifier"),            // :108
            ("a{2,1}".to_string(), 0, "invalid quantifier"),          // :598
            ("a\\q".to_string(), 0, "invalid escape character"),      // :170
            ("a{255}".to_string(), 0, "numeric overflow"),            // :186
            ("a{1,255}".to_string(), 0, "numeric overflow"),          // :200
            ("[z-a]".to_string(), 0, "invalid character class range"), // :224
            ("[a".to_string(), 0, "unterminated character class"),    // :322
            ("()*".to_string(), 0, "infinite loop matching the empty string"), // :493
            ("(a*)*".to_string(), 0, "infinite loop matching the empty string"),
            ("\\1".to_string(), 0, "invalid back-reference"), // :541
            ("(".to_string(), 0, "unmatched '('"),            // :557
            ("(?:".to_string(), 0, "unmatched '('"),          // :563
            ("(?=".to_string(), 0, "unmatched '('"),          // :570
            ("(?!".to_string(), 0, "unmatched '('"),          // :577
            ("*".to_string(), 0, "syntax error"),             // :580
            ("+".to_string(), 0, "syntax error"),
            ("(?".to_string(), 0, "syntax error"),
            (")".to_string(), 0, "unmatched ')'"), // :940
        ];
        // programmatically built limit violations
        table.push(("[a]".repeat(129), 0, "too many character classes")); // :213
        table.push((spaced_class(32), 0, "too many character class ranges")); // :253
        table.push(("()".repeat(16), 0, "too many captures")); // :552
        table.push(("a".repeat(4097), 0, "stack overflow")); // :661
        table.push(("(?:a{254}){254}".to_string(), 0, "program too large")); // :672
        table.push(("a".repeat(16385), 0, "program too large")); // :922
        table.push(("(?:a{254}){129}".to_string(), 0, "program too large")); // :951

        for (pat, cflags, expect) in &table {
            let cpat = cs(pat);
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, *cflags);
                if !prog.is_null() {
                    (api.js_regfree)(prog);
                }
                comp
            });
            assert!(
                c.0,
                "C js_regcomp unexpectedly SUCCEEDED: pattern={} cflags={:#x} (expected error {:?})",
                brief(pat),
                cflags,
                expect
            );
            assert!(
                r.0,
                "Rust js_regcomp unexpectedly SUCCEEDED: pattern={} cflags={:#x} (expected error {:?})",
                brief(pat),
                cflags,
                expect
            );
            assert_eq!(
                c.1, r.1,
                "error-message DIVERGENCE: pattern={} cflags={:#x}: C={:?} Rust={:?}",
                brief(pat), cflags, c.1, r.1
            );
            let msg = c.1.clone().unwrap_or_default();
            assert!(
                msg.contains(expect),
                "unexpected error message: pattern={} cflags={:#x}: got {:?}, expected it to contain {:?}",
                brief(pat),
                cflags,
                msg,
                expect
            );
        }

        /* "just inside the limit" near misses must compile in BOTH */
        let near_miss: Vec<String> = vec![
            "[a]".repeat(128),
            "()".repeat(15),
            "a".repeat(4096),
            "a{254}".to_string(),
            "a{1,254}".to_string(),
            "(?:a{254}){128}".to_string(),
            spaced_class(31),
        ];
        for pat in &near_miss {
            let cpat = cs(pat);
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, 0);
                if !prog.is_null() {
                    (api.js_regfree)(prog);
                }
                comp
            });
            assert_eq!(
                c, r,
                "near-miss compile DIVERGENCE: pattern={} cflags=0x0: C={:?} Rust={:?}",
                brief(pat), c, r
            );
            assert!(
                !c.0 && c.1.is_none(),
                "near-miss pattern should compile in C: pattern={} cflags=0x0: null={} err={:?}",
                brief(pat),
                c.0,
                c.1
            );
            assert!(
                !r.0 && r.1.is_none(),
                "near-miss pattern should compile in Rust: pattern={} cflags=0x0: null={} err={:?}",
                brief(pat),
                r.0,
                r.1
            );
        }
    });
}

/* ------------------------------------------------------------------ */
/*  ERRORS — regexec REG_MAXREC execution-depth limit                  */
/* ------------------------------------------------------------------ */

#[test]
fn err_regexec_exec_recursion_limit() {
    with_big_stack(|| {
        let cpat = cs("a*");
        for n in [4000usize, 4094, 4095, 4096, 5000] {
            let subj = cs(&"a".repeat(n));
            let (c, r) = both(|api, _| unsafe {
                let (prog, comp) = compile_result(api, &cpat, 0);
                assert!(!prog.is_null(), "compiling \"a*\" failed in {}: {:?}", api.path, comp.1);
                let mut sub = Resub::default();
                let with_sub = exec_result(api, prog, &subj, Some(&mut sub), 0);
                let without_sub = exec_result(api, prog, &subj, None, 0);
                (api.js_regfree)(prog);
                (comp, with_sub, without_sub)
            });
            assert_eq!(
                c.0, r.0,
                "compile DIVERGENCE: pattern=\"a*\" cflags=0x0: C={:?} Rust={:?}",
                c.0, r.0
            );
            assert_eq!(
                c.1, r.1,
                "js_regexec DIVERGENCE: pattern=\"a*\" cflags=0x0 eflags=0x0 subject={} 'a's: C={:?} Rust={:?}",
                n, c.1, r.1
            );
            assert_eq!(
                c.2, r.2,
                "js_regexec(sub=NULL) DIVERGENCE: pattern=\"a*\" cflags=0x0 eflags=0x0 subject={} 'a's: C={:?} Rust={:?}",
                n, c.2, r.2
            );
        }
    });
}

/* ------------------------------------------------------------------ */
/*  ERRORS — allocation-failure die() paths                            */
/* ------------------------------------------------------------------ */

static mut FAIL_AT: c_int = 0;
static mut NZ_CALLS: c_int = 0;

/// Fails (returns NULL) on the `FAIL_AT`-th non-zero-size allocation.
unsafe extern "C" fn failing_alloc(_ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        libc::free(ptr);
        return std::ptr::null_mut();
    }
    NZ_CALLS += 1;
    if NZ_CALLS == FAIL_AT {
        return std::ptr::null_mut();
    }
    libc::realloc(ptr, n as usize)
}

#[test]
fn err_allocation_failure_paths() {
    let cases: [(c_int, &str, &str); 4] = [
        (1, "a", "cannot allocate regular expression"),
        (2, "a", "cannot allocate regular expression parse list"),
        (3, "a", "cannot allocate regular expression instruction list"),
        (4, "[a]", "cannot allocate regular expression character class list"),
    ];
    let ctx = 0xABCDusize as *mut c_void;
    for (n, pat, expect) in cases {
        let cpat = cs(pat);
        let (c, r) = both(|api, _| unsafe {
            FAIL_AT = n;
            NZ_CALLS = 0;
            let (prog, comp) = compile_result_x(api, Some(failing_alloc), ctx, &cpat, 0);
            let calls = NZ_CALLS;
            FAIL_AT = 0;
            if !prog.is_null() {
                (api.js_regfreex)(Some(failing_alloc), ctx, prog);
            }
            (comp, calls)
        });
        assert!(
            c.0.0,
            "C js_regcompx unexpectedly SUCCEEDED with alloc failing at call {}: pattern={} cflags=0x0",
            n, brief(pat)
        );
        assert!(
            r.0.0,
            "Rust js_regcompx unexpectedly SUCCEEDED with alloc failing at call {}: pattern={} cflags=0x0",
            n, brief(pat)
        );
        assert_eq!(
            c.0.1, r.0.1,
            "allocation-failure error DIVERGENCE (fail at non-zero alloc #{}): pattern={} cflags=0x0: C={:?} Rust={:?}",
            n, brief(pat), c.0.1, r.0.1
        );
        let msg = c.0.1.clone().unwrap_or_default();
        assert!(
            msg.contains(expect),
            "unexpected allocation-failure message (fail at non-zero alloc #{}): pattern={} cflags=0x0: got {:?}, expected it to contain {:?}",
            n, brief(pat), msg, expect
        );
        assert_eq!(
            c.1, r.1,
            "allocation-attempt count DIVERGENCE (fail at non-zero alloc #{}): pattern={} cflags=0x0: C={} Rust={}",
            n, brief(pat), c.1, r.1
        );
    }
}
