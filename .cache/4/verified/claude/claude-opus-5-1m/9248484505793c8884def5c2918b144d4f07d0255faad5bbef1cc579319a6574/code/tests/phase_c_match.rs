// Phase C — the error paths of the two matchers and their satellites, as
// tabulated in ERRORS.md rows 222..323:
//
//   * `pcre2_match.c`       rows 222-260
//   * `pcre2_match_next.c`  rows 261-265
//   * `pcre2_study.c`       rows 266-270
//   * `pcre2_jit_compile.c` rows 271-277   (non-JIT stubs)
//   * `pcre2_dfa_match.c`   rows 278-323
//
// Every case below constructs the exact invalid input the row names, calls the
// function in BOTH shared objects, and asserts
//   (a) C and Rust return the same numeric code AND the same observable match
//       state (via `read_match_out` / `read_match_out_of`), and
//   (b) the C's code equals what the row's "expected C result" cell documents.
// A failure of (b) means the row was mis-derived; the C is ground truth.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

// ---------------------------------------------------- codes/constants not in
// ---------------------------------------------------- tests/common/mod.rs
const PCRE2_ERROR_DFA_RECURSE: c_int = -39;
const PCRE2_ERROR_UTF8_ERR1: c_int = -3; // 1 byte missing at end
const PCRE2_ERROR_UTF8_ERR13: c_int = -15; // 4-byte char > U+10FFFF
const PCRE2_ERROR_UTF8_ERR14: c_int = -16; // surrogate
const PCRE2_ERROR_UTF8_ERR15: c_int = -17; // 2-byte overlong
const PCRE2_ERROR_UTF8_ERR20: c_int = -22; // isolated 0x80..0xbf
const PCRE2_ERROR_UTF8_ERR21: c_int = -23; // 0xfe / 0xff
const PCRE2_ERROR_CALLOUT: c_int = -37;

const PCRE2_MODE_MASK: u32 = 0x0000_0007;

/// `PCRE2_SIZE_MAX - 1`, the sentinel row 241 needs in `heapframes_size`.
const HF_SIZE_MAX_M1: usize = usize::MAX - 1;

// =========================================================== tiny plumbing

/// One object per library.
#[derive(Copy, Clone)]
struct Two {
    c: Ptr,
    r: Ptr,
}

const NONE2: Two = Two {
    c: ptr::null_mut(),
    r: ptr::null_mut(),
};

unsafe fn compile2(p: &Pair, pat: &[u8], copts: u32, cc: Two) -> Two {
    let (mut ec, mut er) = (0 as c_int, 0 as c_int);
    let (mut oc, mut or) = (0usize, 0usize);
    let a = (p.c.compile)(pat.as_ptr(), pat.len(), copts, &mut ec, &mut oc, cc.c);
    let b = (p.r.compile)(pat.as_ptr(), pat.len(), copts, &mut er, &mut or, cc.r);
    assert_eq!(
        a.is_null(),
        b.is_null(),
        "compile {} opts={copts:#x}: nullness differs (C ec={ec}, rust ec={er})",
        show(pat)
    );
    assert_eq!(ec, er, "compile {} opts={copts:#x}: errorcode differs", show(pat));
    assert_eq!(oc, or, "compile {} opts={copts:#x}: erroroffset differs", show(pat));
    assert!(
        !a.is_null(),
        "phase-C fixture pattern {} opts={copts:#x} must compile, got error {ec} at {oc}",
        show(pat)
    );
    assert_code_eq(a, b, &format!("phase-C fixture {}", show(pat)));
    Two { c: a, r: b }
}

unsafe fn free_code2(p: &Pair, t: Two) {
    (p.c.code_free)(t.c);
    (p.r.code_free)(t.r);
}

unsafe fn md_from_pattern2(p: &Pair, code: Two) -> Two {
    let a = (p.c.match_data_create_from_pattern)(code.c, ptr::null_mut());
    let b = (p.r.match_data_create_from_pattern)(code.r, ptr::null_mut());
    assert!(!a.is_null() && !b.is_null(), "match_data_create_from_pattern failed");
    Two { c: a, r: b }
}

unsafe fn md_create2(p: &Pair, oveccount: u32, gc: Two) -> Two {
    let a = (p.c.match_data_create)(oveccount, gc.c);
    let b = (p.r.match_data_create)(oveccount, gc.r);
    assert!(!a.is_null() && !b.is_null(), "match_data_create({oveccount}) failed");
    Two { c: a, r: b }
}

unsafe fn free_md2(p: &Pair, t: Two) {
    (p.c.match_data_free)(t.c);
    (p.r.match_data_free)(t.r);
}

unsafe fn mcontext2(p: &Pair) -> Two {
    let a = (p.c.match_context_create)(ptr::null_mut());
    let b = (p.r.match_context_create)(ptr::null_mut());
    assert!(!a.is_null() && !b.is_null(), "match_context_create failed");
    Two { c: a, r: b }
}

unsafe fn free_mcontext2(p: &Pair, t: Two) {
    (p.c.match_context_free)(t.c);
    (p.r.match_context_free)(t.r);
}

/// `pcre2_match_8` in both libraries; returns the two comparable results.
unsafe fn match2(
    p: &Pair,
    code: Two,
    md: Two,
    subj: *const u8,
    len: usize,
    so: usize,
    mopts: u32,
    mc: Two,
) -> (MatchOut, MatchOut) {
    let rc = (p.c.do_match)(code.c, subj, len, so, mopts, md.c, mc.c);
    let rr = (p.r.do_match)(code.r, subj, len, so, mopts, md.r, mc.r);
    (
        read_match_out(&p.c, md.c, rc),
        read_match_out(&p.r, md.r, rr),
    )
}

/// `pcre2_dfa_match_8` in both libraries, each with its own workspace.
#[allow(clippy::too_many_arguments)]
unsafe fn dfa2(
    p: &Pair,
    code: Two,
    md: Two,
    subj: *const u8,
    len: usize,
    so: usize,
    mopts: u32,
    mc: Two,
    wsc: &mut [c_int],
    wsr: &mut [c_int],
) -> (MatchOut, MatchOut) {
    let n = wsc.len();
    assert_eq!(n, wsr.len());
    let rc = (p.c.dfa_match)(code.c, subj, len, so, mopts, md.c, mc.c, wsc.as_mut_ptr(), n);
    let rr = (p.r.dfa_match)(code.r, subj, len, so, mopts, md.r, mc.r, wsr.as_mut_ptr(), n);
    (
        read_match_out_of(&p.c, md.c, rc, Engine::Dfa),
        read_match_out_of(&p.r, md.r, rr, Engine::Dfa),
    )
}

/// As `dfa2`, but with an explicit `wscount` that may differ from the real
/// buffer length (rows 281/283 need a too-small count, never a too-small
/// buffer).
#[allow(clippy::too_many_arguments)]
unsafe fn dfa2_count(
    p: &Pair,
    code: Two,
    md: Two,
    subj: *const u8,
    len: usize,
    so: usize,
    mopts: u32,
    mc: Two,
    wsc: *mut c_int,
    wsr: *mut c_int,
    wscount: usize,
) -> (MatchOut, MatchOut) {
    let rc = (p.c.dfa_match)(code.c, subj, len, so, mopts, md.c, mc.c, wsc, wscount);
    let rr = (p.r.dfa_match)(code.r, subj, len, so, mopts, md.r, mc.r, wsr, wscount);
    (
        read_match_out_of(&p.c, md.c, rc, Engine::Dfa),
        read_match_out_of(&p.r, md.r, rr, Engine::Dfa),
    )
}

fn ws(n: usize) -> Vec<c_int> {
    vec![0; n]
}

/// Records (a) C-vs-Rust and (b) C-vs-ERRORS.md for one case.
fn check(d: &mut Diffs, tag: &str, c: &MatchOut, r: &MatchOut, expect: c_int) {
    d.eq(&format!("{tag}: C vs rust"), c.clone(), r.clone());
    d.eq(&format!("{tag}: C rc vs ERRORS.md"), c.rc, expect);
}

// ----------------------------------------------------------- allocators

unsafe fn raw_alloc(n: usize) -> *mut c_void {
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let q = std::alloc::alloc(l);
    if q.is_null() {
        return ptr::null_mut();
    }
    *(q as *mut usize) = sz;
    q.add(16) as *mut c_void
}

unsafe extern "C" fn raw_free(q: *mut c_void, _d: *mut c_void) {
    if q.is_null() {
        return;
    }
    let base = (q as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

// A SEPARATE budget per library, so the two runs cannot interfere.
static mut BUDGET_C: i64 = -1; // -1 = unlimited
static mut BUDGET_R: i64 = -1;

unsafe extern "C" fn fallible_malloc_c(n: usize, _d: *mut c_void) -> *mut c_void {
    let b = &mut *ptr::addr_of_mut!(BUDGET_C);
    if *b == 0 {
        return ptr::null_mut();
    }
    if *b > 0 {
        *b -= 1;
    }
    raw_alloc(n)
}

unsafe extern "C" fn fallible_malloc_r(n: usize, _d: *mut c_void) -> *mut c_void {
    let b = &mut *ptr::addr_of_mut!(BUDGET_R);
    if *b == 0 {
        return ptr::null_mut();
    }
    if *b > 0 {
        *b -= 1;
    }
    raw_alloc(n)
}

/// Zeroing allocator: makes a freshly created `pcre2_match_data` fully defined,
/// which is what row 264 ("a match_data never filled by a successful match")
/// needs in order to be a comparable observable at all.
unsafe extern "C" fn zeroing_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    let q = raw_alloc(n);
    if !q.is_null() {
        ptr::write_bytes(q as *mut u8, 0, n);
    }
    q
}

unsafe fn fallible_gcontext2(p: &Pair) -> Two {
    let a = (p.c.general_context_create)(Some(fallible_malloc_c), Some(raw_free), ptr::null_mut());
    let b = (p.r.general_context_create)(Some(fallible_malloc_r), Some(raw_free), ptr::null_mut());
    assert!(!a.is_null() && !b.is_null());
    Two { c: a, r: b }
}

unsafe fn set_budgets(n: i64) {
    *ptr::addr_of_mut!(BUDGET_C) = n;
    *ptr::addr_of_mut!(BUDGET_R) = n;
}

// ----------------------------------------------------------- callouts

static mut CALLOUT_RET: c_int = 0;

unsafe extern "C" fn callout(_b: *mut c_void, _d: *mut c_void) -> c_int {
    *ptr::addr_of!(CALLOUT_RET)
}

// ------------------------------------------------- code-block manipulation

unsafe fn head(code: Ptr) -> *mut RealCodeHead {
    check_head(code); // validates the layout before any poke
    code as *mut RealCodeHead
}

// ===================================================================== 222-226

struct Case {
    rows: &'static [u32],
    pat: &'static str,
    copts: u32,
    subj: &'static [u8],
    mopts: u32,
    expect: c_int,
}

/// Rows 222-224 and 226: the plausibility checks at the top of
/// `pcre2_match_8`, in the order the C performs them.
#[test]
fn r222_226_match_argument_validation() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let code = compile2(p, b"abc", 0, NONE2);
        let md = md_from_pattern2(p, code);

        // ---- row 222: match_data == NULL. Checked first, so a NULL code and a
        // bogus start_offset must NOT change the answer.
        for &(cd, so) in &[(code, 0usize), (NONE2, 0), (code, 99)] {
            let rc = (p.c.do_match)(cd.c, b"abc".as_ptr(), 3, so, 0, ptr::null_mut(), ptr::null_mut());
            let rr = (p.r.do_match)(cd.r, b"abc".as_ptr(), 3, so, 0, ptr::null_mut(), ptr::null_mut());
            d.eq(&format!("row222 md=NULL so={so}: C vs rust"), rc, rr);
            d.eq(&format!("row222 md=NULL so={so}: C vs ERRORS.md"), rc, PCRE2_ERROR_NULL);
        }

        // ---- row 223: code == NULL with a valid match_data.
        let (c, r) = match2(p, NONE2, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
        check(&mut d, "row223 code=NULL", &c, &r, PCRE2_ERROR_NULL);

        // ---- row 224: subject == NULL with length != 0 (and the legal
        // NULL/length-0 remapping to an internal empty string).
        for &len in &[1usize, 3, PCRE2_ZERO_TERMINATED] {
            let (c, r) = match2(p, code, md, ptr::null(), len, 0, 0, NONE2);
            check(
                &mut d,
                &format!("row224 subject=NULL len={len:#x}"),
                &c,
                &r,
                PCRE2_ERROR_NULL,
            );
        }
        // legal: NULL + length 0 is an empty string, so this is a plain NOMATCH
        let (c, r) = match2(p, code, md, ptr::null(), 0, 0, 0, NONE2);
        d.eq("row224 subject=NULL len=0 (legal): C vs rust", c.clone(), r);
        d.eq("row224 subject=NULL len=0 (legal) is not NULL error", c.rc, PCRE2_ERROR_NOMATCH);

        // ---- row 226: start_offset > length, explicit and zero-terminated.
        for &(len, so) in &[(3usize, 4usize), (3, 5), (3, usize::MAX / 2), (0, 1)] {
            let (c, r) = match2(p, code, md, b"abc\0".as_ptr(), len, so, 0, NONE2);
            check(
                &mut d,
                &format!("row226 len={len} so={so}"),
                &c,
                &r,
                PCRE2_ERROR_BADOFFSET,
            );
        }
        // PCRE2_ZERO_TERMINATED: the length is strlen(subject) == 3
        for &so in &[4usize, 7] {
            let (c, r) = match2(p, code, md, b"abc\0".as_ptr(), PCRE2_ZERO_TERMINATED, so, 0, NONE2);
            check(
                &mut d,
                &format!("row226 zero-terminated so={so}"),
                &c,
                &r,
                PCRE2_ERROR_BADOFFSET,
            );
        }
        // start_offset == length is legal
        let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 3, 0, NONE2);
        d.eq("row226 so==len (legal): C vs rust", c.clone(), r);
        d.eq("row226 so==len (legal) is not BADOFFSET", c.rc, PCRE2_ERROR_NOMATCH);

        free_md2(p, md);
        free_code2(p, code);
    }
    d.finish("rows 222-224,226: pcre2_match_8 NULL/offset plausibility checks");
}

/// Row 225: every one of the 32 option bits on its own, plus `0xFFFFFFFF`.
#[test]
fn r225_match_unknown_option_bits() {
    const PUBLIC_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
        | PCRE2_ENDANCHORED
        | PCRE2_NOTBOL
        | PCRE2_NOTEOL
        | PCRE2_NOTEMPTY
        | PCRE2_NOTEMPTY_ATSTART
        | PCRE2_NO_UTF_CHECK
        | PCRE2_PARTIAL_HARD
        | PCRE2_PARTIAL_SOFT
        | PCRE2_NO_JIT
        | PCRE2_COPY_MATCHED_SUBJECT
        | PCRE2_DISABLE_RECURSELOOP_CHECK;

    const CASES: &[Case] = &[Case {
        rows: &[225],
        pat: "abc",
        copts: 0,
        subj: b"abc",
        mopts: 0xFFFF_FFFF,
        expect: PCRE2_ERROR_BADOPTION,
    }];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let code = compile2(p, b"abc", 0, NONE2);
        let md = md_from_pattern2(p, code);
        for bit in 0..32u32 {
            let o = 1u32 << bit;
            let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, o, NONE2);
            let want = if o & PUBLIC_MATCH_OPTIONS == 0 {
                PCRE2_ERROR_BADOPTION
            } else {
                // legal bit: whatever it does, both libraries must agree
                c.rc
            };
            check(&mut d, &format!("row225 bit {bit} ({o:#010x})"), &c, &r, want);
        }
        for case in CASES {
            let (c, r) = match2(p, code, md, case.subj.as_ptr(), case.subj.len(), 0, case.mopts, NONE2);
            check(&mut d, &format!("row225 opts={:#x}", case.mopts), &c, &r, case.expect);
        }
        // A legal bit ORed with an illegal one is still rejected.  (Note 0x8 is
        // PCRE2_NOTEMPTY_ATSTART at match time, i.e. legal — the illegal bits
        // used here are DFA_RESTART, SUBSTITUTE_GLOBAL, SUBSTITUTE_UNSET_EMPTY
        // and SUBSTITUTE_LITERAL, none of which pcre2_match_8 accepts.)
        for &illegal in &[PCRE2_DFA_RESTART, 0x0000_0100u32, 0x0000_0400, 0x0000_8000] {
            for &legal in &[PCRE2_ANCHORED, PCRE2_NOTBOL, PCRE2_PARTIAL_SOFT] {
                let o = legal | illegal;
                let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, o, NONE2);
                check(&mut d, &format!("row225 mixed {o:#x}"), &c, &r, PCRE2_ERROR_BADOPTION);
            }
        }
        free_md2(p, md);
        free_code2(p, code);
    }
    d.finish("row 225: pcre2_match_8 options outside PUBLIC_MATCH_OPTIONS");
}

// ===================================================================== 227-252

/// Rows 227, 228, 235, 246, 250 and 251 need a deliberately damaged
/// `pcre2_code`; rows 247-249 and 252 additionally need a damaged *heapframe*
/// vector, which no public entry point can produce (see the notes below).
#[test]
fn r227_252_corrupted_code_block() {
    /// A row whose only trigger is corrupted state.
    struct Corrupt {
        rows: &'static [u32],
        what: &'static str,
        expect: c_int,
    }
    const REACHABLE: &[Corrupt] = &[
        Corrupt { rows: &[227], what: "magic_number", expect: PCRE2_ERROR_BADMAGIC },
        Corrupt { rows: &[228], what: "flags mode bits", expect: PCRE2_ERROR_BADMODE },
        Corrupt { rows: &[235], what: "newline_convention", expect: PCRE2_ERROR_INTERNAL },
        Corrupt { rows: &[250], what: "OP_PROP proptype byte", expect: PCRE2_ERROR_INTERNAL },
        Corrupt { rows: &[251], what: "opcode byte", expect: PCRE2_ERROR_INTERNAL },
        Corrupt { rows: &[246], what: "extra_options ALLOW_LOOKAROUND_BSK cleared", expect: PCRE2_ERROR_BAD_BACKSLASH_K },
    ];
    // Rows that this build cannot reach at all:
    //  * 247 `OP_CLOSE` walking `last_group_offset` to PCRE2_UNSET,
    //  * 248 `OP_ACCEPT` inside a recursion finding no GF_RECURSE frame,
    //  * 249 end of whole-pattern recursion with Flast_group_offset UNSET,
    //  * 252 `RETURN_SWITCH` default (corrupted `Freturn_id`).
    // All four live in the *heapframe* vector, which `pcre2_match_8` allocates
    // and initialises itself; there is no public argument that reaches it, and
    // `match_data->heapframes` is only ever produced by a previous well-formed
    // run.  The assertions below therefore exercise the nearest reachable
    // inputs — the very constructs whose bookkeeping those branches guard —
    // and require C and Rust to agree.
    struct Nearest {
        rows: &'static [u32],
        pat: &'static str,
        subj: &'static [u8],
        why: &'static str,
    }
    const NEAREST: &[Nearest] = &[
        Nearest { rows: &[247], pat: "(a)(?:(b)|c)\\2?", subj: b"ac", why: "OP_CLOSE group chain, intact" },
        Nearest { rows: &[248], pat: "(a(*ACCEPT))(?1)", subj: b"aa", why: "OP_ACCEPT inside a recursion, intact chain" },
        Nearest { rows: &[249], pat: "a(?R)?z", subj: b"aazz", why: "end of whole-pattern recursion, intact chain" },
        Nearest { rows: &[252], pat: "(a+)(b|c)+d", subj: b"aabcd", why: "many RETURN_SWITCH ids, all valid" },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // ---------------- row 227: bad magic, both via a zero-filled block and
        // by corrupting a copy of a real code block.
        {
            let zeros_c = vec![0u8; 512];
            let zeros_r = vec![0u8; 512];
            let fake = Two {
                c: zeros_c.as_ptr() as Ptr,
                r: zeros_r.as_ptr() as Ptr,
            };
            let md = md_create2(p, 4, NONE2);
            let (c, r) = match2(p, fake, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            check(&mut d, "row227 zero-filled block (match)", &c, &r, PCRE2_ERROR_BADMAGIC);
            let (mut wc, mut wr) = (ws(64), ws(64));
            let (c, r) = dfa2(p, fake, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, "row287 zero-filled block (dfa)", &c, &r, PCRE2_ERROR_BADMAGIC);
            free_md2(p, md);
        }

        let code = compile2(p, b"abc", 0, NONE2);
        let copy = Two {
            c: (p.c.code_copy)(code.c),
            r: (p.r.code_copy)(code.r),
        };
        assert!(!copy.c.is_null() && !copy.r.is_null());
        let md = md_from_pattern2(p, copy);
        let (hc, hr) = (head(copy.c), head(copy.r));

        // magic
        for bad in [0u32, 1, 0x5043_5244, u32::MAX] {
            let (oc, or) = ((*hc).magic_number, (*hr).magic_number);
            (*hc).magic_number = bad;
            (*hr).magic_number = bad;
            let (c, r) = match2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            check(&mut d, &format!("row227 magic={bad:#x} (match)"), &c, &r, PCRE2_ERROR_BADMAGIC);
            let (mut wc, mut wr) = (ws(64), ws(64));
            let (c, r) = dfa2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row287 magic={bad:#x} (dfa)"), &c, &r, PCRE2_ERROR_BADMAGIC);
            (*hc).magic_number = oc;
            (*hr).magic_number = or;
        }

        // ---------------- row 228 / 288: wrong code-unit width
        for bad_mode in [0u32, 2, 4, 6, 7] {
            let (oc, or) = ((*hc).flags, (*hr).flags);
            (*hc).flags = (oc & !PCRE2_MODE_MASK) | bad_mode;
            (*hr).flags = (or & !PCRE2_MODE_MASK) | bad_mode;
            let (c, r) = match2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            check(&mut d, &format!("row228 mode={bad_mode} (match)"), &c, &r, PCRE2_ERROR_BADMODE);
            let (mut wc, mut wr) = (ws(64), ws(64));
            let (c, r) = dfa2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row288 mode={bad_mode} (dfa)"), &c, &r, PCRE2_ERROR_BADMODE);
            (*hc).flags = oc;
            (*hr).flags = or;
        }

        // ---------------- row 235 / 291: newline_convention out of range
        for nl in [0u16, 7, 8, 9, 0xFFFF] {
            let (oc, or) = ((*hc).newline_convention, (*hr).newline_convention);
            (*hc).newline_convention = nl;
            (*hr).newline_convention = nl;
            let (c, r) = match2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            check(&mut d, &format!("row235 nl={nl} (match)"), &c, &r, PCRE2_ERROR_INTERNAL);
            let (mut wc, mut wr) = (ws(64), ws(64));
            let (c, r) = dfa2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row291 nl={nl} (dfa)"), &c, &r, PCRE2_ERROR_INTERNAL);
            (*hc).newline_convention = oc;
            (*hr).newline_convention = or;
        }
        // the six legal conventions must NOT produce -44
        for nl in 1u16..=6 {
            let (oc, or) = ((*hc).newline_convention, (*hr).newline_convention);
            (*hc).newline_convention = nl;
            (*hr).newline_convention = nl;
            let (c, r) = match2(p, copy, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            d.eq(&format!("row235 legal nl={nl}: C vs rust"), c.clone(), r);
            d.eq(&format!("row235 legal nl={nl} is not INTERNAL"), c.rc != PCRE2_ERROR_INTERNAL, true);
            (*hc).newline_convention = oc;
            (*hr).newline_convention = or;
        }

        free_md2(p, md);
        free_code2(p, copy);
        free_code2(p, code);

        // ---------------- row 251: an opcode the main dispatch does not know.
        // `/abc/` compiles to OP_BRA LINK OP_CHAR 'a' OP_CHAR 'b' OP_CHAR 'c'
        // OP_KET LINK OP_END, so byte 3 is the first OP_CHAR.
        {
            let code = compile2(p, b"abc", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let bc_c = bytecode_ptr(code.c) as *mut u8;
            let bc_r = bytecode_ptr(code.r) as *mut u8;
            assert_eq!(*bc_c.add(3), *bc_r.add(3), "row251 fixture: bytecode differs");
            for bad in [250u8, 253, 255] {
                let orig = *bc_c.add(3);
                *bc_c.add(3) = bad;
                *bc_r.add(3) = bad;
                let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
                check(&mut d, &format!("row251 opcode={bad} (match)"), &c, &r, PCRE2_ERROR_INTERNAL);
                let (mut wc, mut wr) = (ws(256), ws(256));
                let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
                // the DFA's own `default:` for an unknown opcode is DFA_UITEM
                check(&mut d, &format!("row296 dfa opcode={bad}"), &c, &r, PCRE2_ERROR_DFA_UITEM);
                *bc_c.add(3) = orig;
                *bc_r.add(3) = orig;
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---------------- row 250: unrecognised `proptype` in an OP_PROP /
        // char-type-repeat switch. `/\p{L}x/` is OP_BRA LINK OP_PROP proptype
        // propvalue ...; `/\p{L}+x/` puts OP_TYPEPLUS in front.
        for (pat, off) in [(&b"\\p{L}x"[..], 4usize), (&b"\\p{L}+x"[..], 5)] {
            let code = compile2(p, pat, PCRE2_UTF, NONE2);
            let md = md_from_pattern2(p, code);
            let bc_c = bytecode_ptr(code.c) as *mut u8;
            let bc_r = bytecode_ptr(code.r) as *mut u8;
            let orig = *bc_c.add(off);
            for bad in [99u8, 200, 255] {
                *bc_c.add(off) = bad;
                *bc_r.add(off) = bad;
                let (c, r) = match2(p, code, md, b"ax".as_ptr(), 2, 0, 0, NONE2);
                check(
                    &mut d,
                    &format!("row250 {} proptype={bad}", show(pat)),
                    &c,
                    &r,
                    PCRE2_ERROR_INTERNAL,
                );
            }
            *bc_c.add(off) = orig;
            *bc_r.add(off) = orig;
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---------------- row 246: PCRE2_ERROR_BAD_BACKSLASH_K.
        // `\K` inside a lookaround is rejected at compile time (row 85) unless
        // PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK is set; but the *match*-time check
        // at pcre2_match.c:1030 only fires when that same option is NOT set
        // (mb->allowlookaroundbsk == FALSE).  The only way to reach it is
        // therefore to compile with the option and clear
        // `re->extra_options` afterwards, which is what this does.
        {
            let cc = Two {
                c: (p.c.compile_context_create)(ptr::null_mut()),
                r: (p.r.compile_context_create)(ptr::null_mut()),
            };
            assert_eq!(
                (p.c.set_compile_extra_options)(cc.c, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
                (p.r.set_compile_extra_options)(cc.r, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK)
            );
            // (pattern, subject, start_offset) triples whose match starts before
            // start_offset because of the \K in the lookbehind.
            let cases: &[(&[u8], &[u8], usize)] = &[
                (b"(?<=\\Ka)b", b"ab", 1),
                (b"(?<=\\Ka)b", b"abc", 1),
                (b"(?<=\\Ka)b", b"aab", 2),
                (b"(?<=\\Kab)c", b"abc", 1),
                (b"(?<=\\Kab)c", b"abc", 2),
                (b"(?<=a\\Kb)c", b"abc", 2),
            ];
            for &(pat, subj, so) in cases {
                // the compile-time rejection without the extra option (row 85)
                let (mut ec, mut er) = (0 as c_int, 0 as c_int);
                let (mut oc, mut or) = (0usize, 0usize);
                let a = (p.c.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut oc, ptr::null_mut());
                let b = (p.r.compile)(pat.as_ptr(), pat.len(), 0, &mut er, &mut or, ptr::null_mut());
                d.eq(&format!("row246 {} no-option compile", show(pat)), (a.is_null(), ec), (b.is_null(), er));
                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }

                let code = compile2(p, pat, 0, cc);
                let md = md_from_pattern2(p, code);
                let (hc, hr) = (head(code.c), head(code.r));
                // with the option still set the match succeeds ...
                let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), so, 0, NONE2);
                d.eq(&format!("row246 {} so={so} allowed: C vs rust", show(pat)), c.clone(), r);
                d.eq(&format!("row246 {} so={so} allowed is not -75", show(pat)), c.rc, 1);
                // ... and with it cleared, pcre2_match_8 reports -75
                (*hc).extra_options = 0;
                (*hr).extra_options = 0;
                let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), so, 0, NONE2);
                check(
                    &mut d,
                    &format!("row246 {} so={so} cleared", show(pat)),
                    &c,
                    &r,
                    PCRE2_ERROR_BAD_BACKSLASH_K,
                );
                (*hc).extra_options = PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK;
                (*hr).extra_options = PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK;
                free_md2(p, md);
                free_code2(p, code);
            }
            (p.c.compile_context_free)(cc.c);
            (p.r.compile_context_free)(cc.r);
        }

        // ---------------- rows 247-249, 252: nearest reachable inputs
        for n in NEAREST {
            let code = compile2(p, n.pat.as_bytes(), 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, n.subj.as_ptr(), n.subj.len(), 0, 0, NONE2);
            d.eq(
                &format!("rows {:?} nearest reachable ({}): C vs rust", n.rows, n.why),
                c.clone(),
                r,
            );
            d.eq(
                &format!("rows {:?} nearest reachable ({}) is not INTERNAL", n.rows, n.why),
                c.rc != PCRE2_ERROR_INTERNAL,
                true,
            );
            free_md2(p, md);
            free_code2(p, code);
        }
        for c in REACHABLE {
            assert!(!c.rows.is_empty() && c.expect < 0, "{}", c.what);
        }
    }
    d.finish("rows 227,228,235,246,250,251 (+247-249,252 unreachable): corrupted pcre2_code");
}

// ===================================================================== 229-230

#[test]
fn r229_230_partial_endanchored_and_offset_limit() {
    struct PCase {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        mopts: u32,
        expect: c_int,
    }
    // row 229 (match) / row 285 (dfa): PARTIAL_* together with ENDANCHORED,
    // whether the ENDANCHORED comes from the options or from the pattern.
    const P: &[PCase] = &[
        PCase { rows: &[229, 285], pat: "abc", copts: 0, mopts: PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED, expect: PCRE2_ERROR_BADOPTION },
        PCase { rows: &[229, 285], pat: "abc", copts: 0, mopts: PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED, expect: PCRE2_ERROR_BADOPTION },
        PCase { rows: &[229, 285], pat: "abc", copts: PCRE2_ENDANCHORED, mopts: PCRE2_PARTIAL_SOFT, expect: PCRE2_ERROR_BADOPTION },
        PCase { rows: &[229, 285], pat: "abc", copts: PCRE2_ENDANCHORED, mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_BADOPTION },
        PCase { rows: &[229, 285], pat: "abc", copts: PCRE2_ENDANCHORED, mopts: PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT, expect: PCRE2_ERROR_BADOPTION },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for case in P {
            let code = compile2(p, case.pat.as_bytes(), case.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, case.mopts, NONE2);
            check(
                &mut d,
                &format!("row229 {} copts={:#x} mopts={:#x}", case.pat, case.copts, case.mopts),
                &c,
                &r,
                case.expect,
            );
            let (mut wc, mut wr) = (ws(256), ws(256));
            let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, case.mopts, NONE2, &mut wc, &mut wr);
            check(
                &mut d,
                &format!("row285 dfa {} copts={:#x} mopts={:#x}", case.pat, case.copts, case.mopts),
                &c,
                &r,
                case.expect,
            );
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- rows 230 / 290: an offset limit without PCRE2_USE_OFFSET_LIMIT.
        let mc = mcontext2(p);
        for &lim in &[0usize, 1, 2, 3, 100] {
            d.eq(
                &format!("set_offset_limit({lim})"),
                (p.c.set_offset_limit)(mc.c, lim),
                (p.r.set_offset_limit)(mc.r, lim),
            );
            for &copts in &[0u32, PCRE2_USE_OFFSET_LIMIT] {
                let code = compile2(p, b"b", copts, NONE2);
                let md = md_from_pattern2(p, code);
                let want_match = if copts == 0 { PCRE2_ERROR_BADOFFSETLIMIT } else { -999 };
                let (c, r) = match2(p, code, md, b"aab".as_ptr(), 3, 0, 0, mc);
                if want_match == -999 {
                    d.eq(&format!("row230 lim={lim} with USE_OFFSET_LIMIT: C vs rust"), c.clone(), r);
                    d.eq(
                        &format!("row230 lim={lim} with USE_OFFSET_LIMIT is not -56"),
                        c.rc != PCRE2_ERROR_BADOFFSETLIMIT,
                        true,
                    );
                } else {
                    check(&mut d, &format!("row230 lim={lim} (match)"), &c, &r, want_match);
                }
                let (mut wc, mut wr) = (ws(256), ws(256));
                let (c, r) = dfa2(p, code, md, b"aab".as_ptr(), 3, 0, 0, mc, &mut wc, &mut wr);
                if want_match == -999 {
                    d.eq(&format!("row290 lim={lim} with USE_OFFSET_LIMIT: C vs rust"), c.clone(), r);
                } else {
                    check(&mut d, &format!("row290 lim={lim} (dfa)"), &c, &r, want_match);
                }
                free_md2(p, md);
                free_code2(p, code);
            }
        }
        // PCRE2_UNSET restores the "no limit" state
        (p.c.set_offset_limit)(mc.c, PCRE2_UNSET);
        (p.r.set_offset_limit)(mc.r, PCRE2_UNSET);
        let code = compile2(p, b"b", 0, NONE2);
        let md = md_from_pattern2(p, code);
        let (c, r) = match2(p, code, md, b"aab".as_ptr(), 3, 0, 0, mc);
        d.eq("row230 limit=UNSET: C vs rust", c.clone(), r);
        d.eq("row230 limit=UNSET is not -56", c.rc, 1);
        free_md2(p, md);
        free_code2(p, code);
        free_mcontext2(p, mc);
    }
    d.finish("rows 229,230 (+285,290): PARTIAL|ENDANCHORED and offset limit without USE_OFFSET_LIMIT");
}

// ===================================================================== 231-234

#[test]
fn r231_234_utf_subject_and_offsets() {
    struct UCase {
        rows: &'static [u32],
        subj: &'static [u8],
        so: usize,
        expect: c_int,
    }
    const U: &[UCase] = &[
        // row 231 / 292: start_offset inside a UTF-8 character
        UCase { rows: &[231, 292], subj: b"\xc3\xa9", so: 1, expect: PCRE2_ERROR_BADUTFOFFSET },
        UCase { rows: &[231, 292], subj: b"a\xc3\xa9", so: 2, expect: PCRE2_ERROR_BADUTFOFFSET },
        UCase { rows: &[231, 292], subj: b"\xe2\x82\xac", so: 1, expect: PCRE2_ERROR_BADUTFOFFSET },
        UCase { rows: &[231, 292], subj: b"\xe2\x82\xac", so: 2, expect: PCRE2_ERROR_BADUTFOFFSET },
        // row 232: isolated continuation byte at start_offset == 0
        UCase { rows: &[232], subj: b"\x80abc", so: 0, expect: PCRE2_ERROR_UTF8_ERR20 },
        UCase { rows: &[232], subj: b"\xbf", so: 0, expect: PCRE2_ERROR_UTF8_ERR20 },
        // row 233 / 293: malformed UTF-8 in the scanned subject
        UCase { rows: &[233, 293], subj: b"\xc3", so: 0, expect: PCRE2_ERROR_UTF8_ERR1 },
        UCase { rows: &[233, 293], subj: b"ab\xc3", so: 0, expect: PCRE2_ERROR_UTF8_ERR1 },
        UCase { rows: &[233, 293], subj: b"\xed\xa0\x80", so: 0, expect: PCRE2_ERROR_UTF8_ERR14 },
        UCase { rows: &[233, 293], subj: b"\xfe", so: 0, expect: PCRE2_ERROR_UTF8_ERR21 },
        UCase { rows: &[233, 293], subj: b"ab\xfex", so: 0, expect: PCRE2_ERROR_UTF8_ERR21 },
        UCase { rows: &[233, 293], subj: b"\xf5\x80\x80\x80", so: 0, expect: PCRE2_ERROR_UTF8_ERR13 },
        UCase { rows: &[233, 293], subj: b"\xc0\x80", so: 0, expect: PCRE2_ERROR_UTF8_ERR15 },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let code = compile2(p, b"a", PCRE2_UTF, NONE2);
        let md = md_from_pattern2(p, code);
        for u in U {
            let (c, r) = match2(p, code, md, u.subj.as_ptr(), u.subj.len(), u.so, 0, NONE2);
            check(
                &mut d,
                &format!("rows {:?} {} so={} (match)", u.rows, show(u.subj), u.so),
                &c,
                &r,
                u.expect,
            );
            let (mut wc, mut wr) = (ws(256), ws(256));
            let (c, r) = dfa2(p, code, md, u.subj.as_ptr(), u.subj.len(), u.so, 0, NONE2, &mut wc, &mut wr);
            check(
                &mut d,
                &format!("rows {:?} {} so={} (dfa)", u.rows, show(u.subj), u.so),
                &c,
                &r,
                u.expect,
            );
            // With PCRE2_NO_UTF_CHECK the validity scan is skipped entirely, so
            // whatever happens must at least be identical in both libraries.
            // (A mid-character start_offset is then simply not diagnosed.)
            if u.so == 0 {
                let (c, r) = match2(p, code, md, u.subj.as_ptr(), u.subj.len(), u.so, PCRE2_NO_UTF_CHECK, NONE2);
                d.eq(
                    &format!("rows {:?} {} NO_UTF_CHECK (match)", u.rows, show(u.subj)),
                    c.clone(),
                    r,
                );
                d.eq(
                    &format!("rows {:?} {} NO_UTF_CHECK gives no UTF error", u.rows, show(u.subj)),
                    !(-28..=-3).contains(&c.rc),
                    true,
                );
                let (mut wc, mut wr) = (ws(256), ws(256));
                let (c, r) = dfa2(
                    p, code, md, u.subj.as_ptr(), u.subj.len(), u.so, PCRE2_NO_UTF_CHECK, NONE2, &mut wc, &mut wr,
                );
                d.eq(
                    &format!("rows {:?} {} NO_UTF_CHECK (dfa)", u.rows, show(u.subj)),
                    c.clone(),
                    r,
                );
            }
        }
        free_md2(p, md);
        free_code2(p, code);

        // ---- row 234: PCRE2_MATCH_INVALID_UTF turns the same subjects into
        // fragment-by-fragment matching with no UTF error at all.
        let code = compile2(p, b"a", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF, NONE2);
        let md = md_from_pattern2(p, code);
        for u in U {
            let (c, r) = match2(p, code, md, u.subj.as_ptr(), u.subj.len(), 0, 0, NONE2);
            d.eq(
                &format!("row234 {} MATCH_INVALID_UTF: C vs rust", show(u.subj)),
                c.clone(),
                r,
            );
            d.eq(
                &format!("row234 {} MATCH_INVALID_UTF gives no UTF error", show(u.subj)),
                !(-28..=-3).contains(&c.rc) && c.rc != PCRE2_ERROR_BADUTFOFFSET,
                true,
            );
        }
        // subjects where a match does exist before/after the bad code unit
        for subj in [&b"a\xc3"[..], &b"\xc3a"[..], &b"\x80a\x80"[..], &b"a\xfea"[..]] {
            let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, NONE2);
            d.eq(&format!("row234 fragment {}: C vs rust", show(subj)), c.clone(), r);
            d.eq(&format!("row234 fragment {} matches", show(subj)), c.rc, 1);
        }
        // row 286: the DFA refuses PCRE2_MATCH_INVALID_UTF outright
        let (mut wc, mut wr) = (ws(256), ws(256));
        let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
        check(&mut d, "row286 dfa MATCH_INVALID_UTF", &c, &r, PCRE2_ERROR_DFA_UINVALID_UTF);
        free_md2(p, md);
        free_code2(p, code);
    }
    d.finish("rows 231-234 (+286,292,293): UTF start offsets, invalid UTF, MATCH_INVALID_UTF");
}

// ===================================================================== 236-243

#[test]
fn r236_243_match_limits() {
    struct Sweep {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        subj_a: usize, // subject is this many 'a's
        expect: c_int,
    }
    const MATCH_LIMIT_SWEEPS: &[Sweep] = &[
        Sweep { rows: &[239], pat: "abc", copts: 0, subj_a: 0, expect: PCRE2_ERROR_MATCHLIMIT },
        Sweep { rows: &[239], pat: "(a|b)*c", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 6, expect: PCRE2_ERROR_MATCHLIMIT },
        Sweep { rows: &[239], pat: "(a+)+b", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 6, expect: PCRE2_ERROR_MATCHLIMIT },
    ];
    const DEPTH_LIMIT_SWEEPS: &[Sweep] = &[
        Sweep { rows: &[240], pat: "abc", copts: 0, subj_a: 0, expect: PCRE2_ERROR_DEPTHLIMIT },
        Sweep { rows: &[240], pat: "(a|b)*c", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 6, expect: PCRE2_ERROR_DEPTHLIMIT },
        Sweep { rows: &[240], pat: "(a+)+b", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 6, expect: PCRE2_ERROR_DEPTHLIMIT },
    ];
    // rows 242 and 243 are the two arms of the frame-vector growth clamp; both
    // return PCRE2_ERROR_HEAPLIMIT and are not distinguishable from outside, so
    // the whole heap-limit range is swept and every value compared.
    const HEAP_LIMIT_SWEEPS: &[Sweep] = &[
        Sweep { rows: &[236, 242, 243], pat: "(a)+b", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 1500, expect: PCRE2_ERROR_HEAPLIMIT },
        Sweep { rows: &[236, 242, 243], pat: "(a(?1)?)b", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 600, expect: PCRE2_ERROR_HEAPLIMIT },
        Sweep { rows: &[236, 242, 243], pat: "((a)|(b)|(c)|(d)|(e)|(f)|(g)|(h))+z", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 400, expect: PCRE2_ERROR_HEAPLIMIT },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mc = mcontext2(p);
        let big = vec![b'a'; 2000];

        // ---- row 239: match limit, swept so the crossover point is compared
        for s in MATCH_LIMIT_SWEEPS {
            let code = compile2(p, s.pat.as_bytes(), s.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let subj = &big[..s.subj_a];
            let mut saw = false;
            for lim in 0u32..=30 {
                (p.c.set_match_limit)(mc.c, lim);
                (p.r.set_match_limit)(mc.r, lim);
                (p.c.set_depth_limit)(mc.c, 10_000_000);
                (p.r.set_depth_limit)(mc.r, 10_000_000);
                let (sp, sl) = if s.subj_a == 0 {
                    (b"abc".as_ptr(), 3)
                } else {
                    (subj.as_ptr(), subj.len())
                };
                let (c, r) = match2(p, code, md, sp, sl, 0, 0, mc);
                d.eq(&format!("row239 {} match_limit={lim}: C vs rust", s.pat), c.clone(), r);
                if c.rc == s.expect {
                    saw = true;
                }
            }
            d.eq(&format!("row239 {} reaches MATCHLIMIT", s.pat), saw, true);
            free_md2(p, md);
            free_code2(p, code);
        }
        (p.c.set_match_limit)(mc.c, 10_000_000);
        (p.r.set_match_limit)(mc.r, 10_000_000);

        // ---- row 240: depth limit, same treatment
        for s in DEPTH_LIMIT_SWEEPS {
            let code = compile2(p, s.pat.as_bytes(), s.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let subj = &big[..s.subj_a];
            let mut saw = false;
            for lim in 0u32..=30 {
                (p.c.set_depth_limit)(mc.c, lim);
                (p.r.set_depth_limit)(mc.r, lim);
                let (sp, sl) = if s.subj_a == 0 {
                    (b"abc".as_ptr(), 3)
                } else {
                    (subj.as_ptr(), subj.len())
                };
                let (c, r) = match2(p, code, md, sp, sl, 0, 0, mc);
                d.eq(&format!("row240 {} depth_limit={lim}: C vs rust", s.pat), c.clone(), r);
                if c.rc == s.expect {
                    saw = true;
                }
            }
            d.eq(&format!("row240 {} reaches DEPTHLIMIT", s.pat), saw, true);
            free_md2(p, md);
            free_code2(p, code);
        }
        (p.c.set_depth_limit)(mc.c, 10_000_000);
        (p.r.set_depth_limit)(mc.r, 10_000_000);

        // ---- row 236: heap_limit 0 fires before any matching at all, for
        // every pattern including a trivial one.
        (p.c.set_heap_limit)(mc.c, 0);
        (p.r.set_heap_limit)(mc.r, 0);
        for pat in [&b"abc"[..], &b"a"[..], &b"(a)(b)(c)"[..], &b"(a+)*b"[..]] {
            let code = compile2(p, pat, 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, 0, mc);
            check(&mut d, &format!("row236 heap_limit=0 {}", show(pat)), &c, &r, PCRE2_ERROR_HEAPLIMIT);
            free_md2(p, md);
            free_code2(p, code);
        }
        // ... and `(*LIMIT_HEAP=0)` in the pattern does the same
        {
            let code = compile2(p, b"(*LIMIT_HEAP=0)abc", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            check(&mut d, "row236 (*LIMIT_HEAP=0)", &c, &r, PCRE2_ERROR_HEAPLIMIT);
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- rows 242/243: growth denied by the heap limit
        for s in HEAP_LIMIT_SWEEPS {
            let code = compile2(p, s.pat.as_bytes(), s.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let subj = &big[..s.subj_a];
            let mut saw = false;
            for hl in [0u32, 1, 2, 3, 5, 8, 13, 19, 20, 21, 22, 29, 30, 31, 40, 48, 49, 50, 64] {
                (p.c.set_heap_limit)(mc.c, hl);
                (p.r.set_heap_limit)(mc.r, hl);
                let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, mc);
                d.eq(&format!("rows242/243 {} heap_limit={hl}: C vs rust", s.pat), c.clone(), r);
                d.eq(
                    &format!("rows242/243 {} heap_limit={hl}: heapframes_size", s.pat),
                    (p.c.get_match_data_heapframes_size)(md.c),
                    (p.r.get_match_data_heapframes_size)(md.r),
                );
                if c.rc == s.expect {
                    saw = true;
                }
            }
            d.eq(&format!("rows242/243 {} reaches HEAPLIMIT", s.pat), saw, true);
            free_md2(p, md);
            free_code2(p, code);
        }
        free_mcontext2(p, mc);
    }
    d.finish("rows 236,239,240,242,243: match/depth/heap limit exhaustion, limits swept");
}

// ===================================================================== 237-244

#[test]
fn r237_244_match_allocation_failures() {
    struct ACase {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        subj_a: usize,
        mopts: u32,
        budgets: usize,
        expect: c_int,
    }
    const A: &[ACase] = &[
        // row 237: the very first malloc is the heapframes vector
        ACase { rows: &[237], pat: "abc", copts: 0, subj_a: 0, mopts: 0, budgets: 3, expect: PCRE2_ERROR_NOMEMORY },
        // row 244: the doubled frame vector
        ACase { rows: &[244], pat: "(a)+b", copts: PCRE2_NO_START_OPTIMIZE, subj_a: 1500, mopts: 0, budgets: 8, expect: PCRE2_ERROR_NOMEMORY },
        // row 238: the PCRE2_COPY_MATCHED_SUBJECT copy
        ACase { rows: &[238], pat: "abc", copts: 0, subj_a: 0, mopts: PCRE2_COPY_MATCHED_SUBJECT, budgets: 4, expect: PCRE2_ERROR_NOMEMORY },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let gc = fallible_gcontext2(p);
        let big = vec![b'a'; 2000];
        for a in A {
            let code = compile2(p, a.pat.as_bytes(), a.copts, NONE2);
            let (sp, sl) = if a.subj_a == 0 {
                (b"abc".as_ptr(), 3usize)
            } else {
                (big.as_ptr(), a.subj_a)
            };
            let mut saw = false;
            for n in 0..=a.budgets as i64 {
                set_budgets(-1);
                let md = md_create2(p, 8, gc);
                set_budgets(n);
                let (c, r) = match2(p, code, md, sp, sl, 0, a.mopts, NONE2);
                set_budgets(-1);
                d.eq(&format!("rows {:?} {} budget={n}: C vs rust", a.rows, a.pat), c.clone(), r);
                d.eq(
                    &format!("rows {:?} {} budget={n}: heapframes_size", a.rows, a.pat),
                    (p.c.get_match_data_heapframes_size)(md.c),
                    (p.r.get_match_data_heapframes_size)(md.r),
                );
                if c.rc == a.expect {
                    saw = true;
                }
                free_md2(p, md);
            }
            d.eq(&format!("rows {:?} {} reaches NOMEMORY", a.rows, a.pat), saw, true);
            free_code2(p, code);
        }

        // ---- row 241: the frame vector must grow but
        // `match_data->heapframes_size == PCRE2_SIZE_MAX - 1`.
        // The field is private, so it is located by value: run one match to
        // give it a known non-zero size, then find that size inside the block.
        {
            let code = compile2(p, b"(a)+b", PCRE2_NO_START_OPTIMIZE, NONE2);
            let md = md_create2(p, 8, NONE2);
            let subj = vec![b'a'; 200];
            let _ = match2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, NONE2);
            let hfc = (p.c.get_match_data_heapframes_size)(md.c);
            let hfr = (p.r.get_match_data_heapframes_size)(md.r);
            d.eq("row241 fixture heapframes_size", hfc, hfr);
            assert!(hfc > 0, "row241 fixture: no frame vector was allocated");
            let slot_c = find_usize_slot(md.c, (p.c.get_match_data_size)(md.c), hfc);
            let slot_r = find_usize_slot(md.r, (p.r.get_match_data_size)(md.r), hfr);
            match (slot_c, slot_r) {
                (Some(sc), Some(sr)) => {
                    *sc = HF_SIZE_MAX_M1;
                    *sr = HF_SIZE_MAX_M1;
                    let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, NONE2);
                    check(&mut d, "row241 heapframes_size=SIZE_MAX-1", &c, &r, PCRE2_ERROR_NOMEMORY);
                    *sc = hfc;
                    *sr = hfr;
                }
                _ => {
                    // Could not locate the field unambiguously: fall back to the
                    // nearest reachable input (a growth that succeeds) and still
                    // require the two libraries to agree.
                    let (c, r) = match2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, NONE2);
                    d.eq("row241 (field not locatable) nearest reachable: C vs rust", c.clone(), r);
                }
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        (p.c.general_context_free)(gc.c);
        (p.r.general_context_free)(gc.r);
    }
    d.finish("rows 237,238,241,244: allocation failures inside pcre2_match_8");
}

/// Finds the unique `usize`-aligned slot in `[block, block+len)` holding
/// `value`; returns `None` if there is not exactly one.
unsafe fn find_usize_slot(block: Ptr, len: usize, value: usize) -> Option<*mut usize> {
    let mut found: Option<*mut usize> = None;
    let mut off = 0usize;
    while off + std::mem::size_of::<usize>() <= len {
        let q = (block as *mut u8).add(off) as *mut usize;
        if *q == value {
            if found.is_some() {
                return None;
            }
            found = Some(q);
        }
        off += std::mem::size_of::<usize>();
    }
    found
}

// ===================================================================== 245

#[test]
fn r245_recurse_loop() {
    struct RCase {
        rows: &'static [u32],
        pat: &'static str,
        subj: &'static [u8],
        expect: c_int,
    }
    // ERRORS.md row 245 names `(?1)()` and `(a(?2))((?1))` on "a"; neither
    // actually loops (the C returns 2 and -1 respectively — see the report).
    // These mutual recursions do.
    const R: &[RCase] = &[
        RCase { rows: &[245], pat: "((?2))((?1))", subj: b"a", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "((?2))((?1))", subj: b"", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "(a|(?2))((?1))", subj: b"xy", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "((?2)|a)((?1)|b)", subj: b"ab", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "((?2))((?3))((?1))", subj: b"a", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "(?1)(?2)((?2))((?1))", subj: b"ab", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "((?2)a)((?1))", subj: b"aaa", expect: PCRE2_ERROR_RECURSELOOP },
        RCase { rows: &[245], pat: "((?2))((?1)a)", subj: b"aaa", expect: PCRE2_ERROR_RECURSELOOP },
        // patterns from ERRORS.md's own trigger cell: they do NOT loop
        RCase { rows: &[245], pat: "(?1)()", subj: b"a", expect: 2 },
        RCase { rows: &[245], pat: "(a(?2))((?1))", subj: b"a", expect: PCRE2_ERROR_NOMATCH },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mc = mcontext2(p);
        // A small match limit keeps the DISABLE_RECURSELOOP_CHECK runs quick;
        // the C documents that real loops are then caught by the match limit.
        (p.c.set_match_limit)(mc.c, 2000);
        (p.r.set_match_limit)(mc.r, 2000);
        for case in R {
            let code = compile2(p, case.pat.as_bytes(), 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, case.subj.as_ptr(), case.subj.len(), 0, 0, NONE2);
            check(
                &mut d,
                &format!("row245 {} on {}", case.pat, show(case.subj)),
                &c,
                &r,
                case.expect,
            );
            // With the check disabled, the loop must instead be caught by the
            // match limit (or, for the non-looping patterns, be unaffected).
            let (c2, r2) = match2(
                p,
                code,
                md,
                case.subj.as_ptr(),
                case.subj.len(),
                0,
                PCRE2_DISABLE_RECURSELOOP_CHECK,
                mc,
            );
            d.eq(
                &format!("row245 {} DISABLE_RECURSELOOP_CHECK: C vs rust", case.pat),
                c2.clone(),
                r2,
            );
            d.eq(
                &format!("row245 {} DISABLE_RECURSELOOP_CHECK is never -52", case.pat),
                c2.rc != PCRE2_ERROR_RECURSELOOP,
                true,
            );
            if case.expect == PCRE2_ERROR_RECURSELOOP {
                d.eq(
                    &format!("row245 {} DISABLE_RECURSELOOP_CHECK hits the match limit", case.pat),
                    c2.rc,
                    PCRE2_ERROR_MATCHLIMIT,
                );
            }
            free_md2(p, md);
            free_code2(p, code);
        }
        free_mcontext2(p, mc);
    }
    d.finish("row 245: PCRE2_ERROR_RECURSELOOP and PCRE2_DISABLE_RECURSELOOP_CHECK");
}

// ===================================================================== 253-260

#[test]
fn r253_260_match_outcomes() {
    const CASES: &[Case] = &[
        // row 253: no starting position yields a match
        Case { rows: &[253], pat: "xyz", copts: 0, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        Case { rows: &[253], pat: "^xyz", copts: 0, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        Case { rows: &[253], pat: "abc", copts: 0, subj: b"abd", mopts: PCRE2_ANCHORED, expect: PCRE2_ERROR_NOMATCH },
        Case { rows: &[253], pat: "a(*COMMIT)b", copts: 0, subj: b"ac", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        Case { rows: &[253], pat: "b", copts: PCRE2_FIRSTLINE, subj: b"a\nb", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        // row 254: soft partial match
        Case { rows: &[254], pat: "abcd", copts: 0, subj: b"ab", mopts: PCRE2_PARTIAL_SOFT, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[254], pat: "abcd", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_SOFT, expect: PCRE2_ERROR_PARTIAL },
        // row 255: hard partial match via SCHECK_PARTIAL
        Case { rows: &[255], pat: "abcd", copts: 0, subj: b"ab", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[255], pat: "abc", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        // row 256: PARTIAL_HARD + CRLF + subject ending in a lone CR
        Case { rows: &[256], pat: "(*CRLF)a.", copts: 0, subj: b"a\r", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[256], pat: "(*CRLF)a$", copts: 0, subj: b"a\r", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[256], pat: "(*CRLF)a\\Z", copts: 0, subj: b"a\r", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[256], pat: "(*CRLF)(?m)a$", copts: 0, subj: b"a\r", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        Case { rows: &[256], pat: "(*CRLF)a.+", copts: 0, subj: b"ax\r", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        // row 258: a callout returning > 0 becomes a local NOMATCH
        Case { rows: &[258], pat: "a(?C1)b", copts: 0, subj: b"ab", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mc = mcontext2(p);
        d.eq(
            "set_callout",
            (p.c.set_callout)(mc.c, Some(callout), ptr::null_mut()),
            (p.r.set_callout)(mc.r, Some(callout), ptr::null_mut()),
        );
        for case in CASES {
            let code = compile2(p, case.pat.as_bytes(), case.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let use_mc = case.rows.contains(&258);
            if use_mc {
                *ptr::addr_of_mut!(CALLOUT_RET) = 1;
            }
            let (c, r) = match2(
                p,
                code,
                md,
                case.subj.as_ptr(),
                case.subj.len(),
                0,
                case.mopts,
                if use_mc { mc } else { NONE2 },
            );
            check(
                &mut d,
                &format!("rows {:?} /{}/ on {}", case.rows, case.pat, show(case.subj)),
                &c,
                &r,
                case.expect,
            );
            free_md2(p, md);
            free_code2(p, code);
        }

        // row 258 with several positive callout values
        {
            let code = compile2(p, b"a(?C1)b", 0, NONE2);
            let md = md_from_pattern2(p, code);
            for v in [1, 2, 99, c_int::MAX] {
                *ptr::addr_of_mut!(CALLOUT_RET) = v;
                let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, mc);
                check(&mut d, &format!("row258 callout={v}"), &c, &r, PCRE2_ERROR_NOMATCH);
            }
            // row 259: a negative value other than -1 propagates unchanged
            for v in [PCRE2_ERROR_CALLOUT, -99, -1000, -2] {
                *ptr::addr_of_mut!(CALLOUT_RET) = v;
                let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, mc);
                check(&mut d, &format!("row259 callout={v}"), &c, &r, v);
            }
            // and -1 (PCRE2_ERROR_NOMATCH) is treated as a local no-match
            *ptr::addr_of_mut!(CALLOUT_RET) = PCRE2_ERROR_NOMATCH;
            let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, mc);
            check(&mut d, "row258 callout=-1", &c, &r, PCRE2_ERROR_NOMATCH);
            // 0 lets the match through
            *ptr::addr_of_mut!(CALLOUT_RET) = 0;
            let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, mc);
            d.eq("row258 callout=0: C vs rust", c.clone(), r);
            d.eq("row258 callout=0 matches", c.rc, 1);
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- row 257: successful match whose ovector is too small -> rc 0
        {
            let code = compile2(p, b"(a)(b)", 0, NONE2);
            for oveccount in [1u32, 2, 3] {
                let md = md_create2(p, oveccount, NONE2);
                let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, NONE2);
                let want = if oveccount < 3 { 0 } else { 3 };
                check(&mut d, &format!("row257 oveccount={oveccount}"), &c, &r, want);
                if oveccount < 3 {
                    // documented: ovector[0..1] still hold the whole match
                    d.eq("row257 ovector[0..1]", (c.ovector[0], c.ovector[1]), (0usize, 2usize));
                }
                free_md2(p, md);
            }
            // pcre2_match_data_create clamps 0 up to 1
            let md = md_create2(p, 0, NONE2);
            d.eq(
                "row257 oveccount=0 clamped",
                (p.c.get_ovector_count)(md.c),
                (p.r.get_ovector_count)(md.r),
            );
            let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, NONE2);
            check(&mut d, "row257 oveccount=0", &c, &r, 0);
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- row 260: the JIT-only branches of pcre2_match_8 (mid-character
        // start_offset, isolated 0x80, valid_utf failure and the
        // COPY_MATCHED_SUBJECT malloc failure around a JIT run at
        // pcre2_match.c:7161/7163/7200-7205/7225) are inside `#ifdef
        // SUPPORT_JIT`, which is undefined here, so they are not compiled at
        // all.  The nearest reachable inputs are the interpreter's own copies
        // of those checks (rows 231/232/233/238); assert that C and Rust agree
        // on them with PCRE2_NO_JIT set, i.e. on exactly the inputs a JIT build
        // would route through the excluded code.
        {
            struct J {
                rows: &'static [u32],
                subj: &'static [u8],
                so: usize,
            }
            const JJ: &[J] = &[
                J { rows: &[260], subj: b"\xc3\xa9", so: 1 },
                J { rows: &[260], subj: b"\x80abc", so: 0 },
                J { rows: &[260], subj: b"\xc3", so: 0 },
            ];
            let code = compile2(p, b"a", PCRE2_UTF, NONE2);
            let md = md_from_pattern2(p, code);
            for j in JJ {
                for mopts in [PCRE2_NO_JIT, PCRE2_NO_JIT | PCRE2_COPY_MATCHED_SUBJECT] {
                    let (c, r) = match2(p, code, md, j.subj.as_ptr(), j.subj.len(), j.so, mopts, NONE2);
                    d.eq(
                        &format!("row260 (JIT path not compiled) {} mopts={mopts:#x}", show(j.subj)),
                        c.clone(),
                        r,
                    );
                }
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        free_mcontext2(p, mc);
    }
    d.finish("rows 253-259 outcomes (+260 unreachable: SUPPORT_JIT undefined)");
}

// ===================================================================== 261-265

#[test]
fn r261_265_next_match() {
    struct NCase {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        subj: &'static [u8],
        so: usize,
        expect_more: c_int,
    }
    const N: &[NCase] = &[
        // row 261: the preceding match stored a negative rc
        NCase { rows: &[261], pat: "z", copts: 0, subj: b"abc", so: 0, expect_more: 0 },
        NCase { rows: &[261], pat: "abcd", copts: 0, subj: b"ab", so: 0, expect_more: 0 },
        // row 263: previous match was empty and at the end of the subject
        NCase { rows: &[263], pat: "a*", copts: 0, subj: b"", so: 0, expect_more: 0 },
        NCase { rows: &[263], pat: "a*", copts: 0, subj: b"b", so: 1, expect_more: 0 },
        // ... and not at the end: another attempt with NOTEMPTY_ATSTART
        NCase { rows: &[263], pat: "a*", copts: 0, subj: b"bb", so: 0, expect_more: 1 },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for case in N {
            let code = compile2(p, case.pat.as_bytes(), case.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, case.subj.as_ptr(), case.subj.len(), case.so, 0, NONE2);
            d.eq(&format!("rows {:?} /{}/ prior match", case.rows, case.pat), c.clone(), r);
            let (mut soc, mut sor) = (0xAAAA_AAAAusize, 0xAAAA_AAAAusize);
            let (mut opc, mut opr) = (0xBBBB_BBBBu32, 0xBBBB_BBBBu32);
            let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
            let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
            d.eq(&format!("rows {:?} /{}/ next_match rc", case.rows, case.pat), mc, mr);
            d.eq(&format!("rows {:?} /{}/ next_match out", case.rows, case.pat), (soc, opc), (sor, opr));
            d.eq(
                &format!("rows {:?} /{}/ next_match vs ERRORS.md", case.rows, case.pat),
                mc,
                case.expect_more,
            );
            if mc == 0 {
                // documented: *pstart_offset / *poptions are untouched
                d.eq(
                    &format!("rows {:?} /{}/ out params untouched", case.rows, case.pat),
                    (soc, opc),
                    (0xAAAA_AAAAusize, 0xBBBB_BBBBu32),
                );
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- a full iteration sequence, including the step past the end
        // (rows 261 and 263 in their normal roles).
        {
            let code = compile2(p, b"a*", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let subj = b"baab";
            let (mut soc, mut sor) = (0usize, 0usize);
            let (mut opc, mut opr) = (0u32, 0u32);
            for step in 0..10 {
                let rc = (p.c.do_match)(code.c, subj.as_ptr(), 4, soc, opc, md.c, ptr::null_mut());
                let rr = (p.r.do_match)(code.r, subj.as_ptr(), 4, sor, opr, md.r, ptr::null_mut());
                d.eq(
                    &format!("row261/263 iteration step {step} match"),
                    read_match_out(&p.c, md.c, rc),
                    read_match_out(&p.r, md.r, rr),
                );
                let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
                let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
                d.eq(&format!("row261/263 iteration step {step} next"), (mc, soc, opc), (mr, sor, opr));
                if mc == 0 {
                    break;
                }
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- row 262: a non-empty match that makes no progress, i.e.
        // ovector[0] != start_offset && ovector[1] == start_offset.  Only \K
        // inside a lookbehind can do that, so PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK
        // is required at compile time.
        {
            let cc = Two {
                c: (p.c.compile_context_create)(ptr::null_mut()),
                r: (p.r.compile_context_create)(ptr::null_mut()),
            };
            (p.c.set_compile_extra_options)(cc.c, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK);
            (p.r.set_compile_extra_options)(cc.r, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK);
            struct K {
                rows: &'static [u32],
                pat: &'static str,
                subj: &'static [u8],
                so: usize,
                expect_more: c_int,
            }
            const KK: &[K] = &[
                // at the end of the subject -> FALSE (row 262 first branch)
                K { rows: &[262], pat: "(?<=a\\Kb)", subj: b"ab", so: 2, expect_more: 0 },
                // not at the end -> bump along by one code unit (row 265's
                // do_bumpalong, on a well-formed match_data)
                K { rows: &[262, 265], pat: "(?<=a\\Kb)", subj: b"abc", so: 2, expect_more: 1 },
                // do_bumpalong over a CRLF pair
                K { rows: &[265], pat: "(*CRLF)(?<=a\\Kb)", subj: b"ab\r\ncd", so: 2, expect_more: 1 },
                // do_bumpalong over a multi-byte UTF-8 character
                K { rows: &[265], pat: "(?<=a\\Kb)", subj: b"ab\xc3\xa9z", so: 2, expect_more: 1 },
            ];
            for k in KK {
                let copts = if k.subj.contains(&0xc3) { PCRE2_UTF } else { 0 };
                let code = compile2(p, k.pat.as_bytes(), copts, cc);
                let md = md_from_pattern2(p, code);
                let (c, r) = match2(p, code, md, k.subj.as_ptr(), k.subj.len(), k.so, 0, NONE2);
                d.eq(&format!("rows {:?} /{}/ prior match", k.rows, k.pat), c.clone(), r);
                d.eq(&format!("rows {:?} /{}/ prior match succeeded", k.rows, k.pat), c.rc, 1);
                let (mut soc, mut sor) = (0xAAAA_AAAAusize, 0xAAAA_AAAAusize);
                let (mut opc, mut opr) = (0xBBBB_BBBBu32, 0xBBBB_BBBBu32);
                let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
                let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
                d.eq(&format!("rows {:?} /{}/ next rc", k.rows, k.pat), mc, mr);
                d.eq(&format!("rows {:?} /{}/ next out", k.rows, k.pat), (soc, opc), (sor, opr));
                d.eq(&format!("rows {:?} /{}/ next vs ERRORS.md", k.rows, k.pat), mc, k.expect_more);
                free_md2(p, md);
                free_code2(p, code);
            }
            (p.c.compile_context_free)(cc.c);
            (p.r.compile_context_free)(cc.r);
        }

        // ---- row 264: `PCRE2_ASSERT(ovector[1] >= start_offset)` is a no-op in
        // this build.  Two ways to observe that:
        //  (a) a match_data that was never filled by a match at all.  Its `rc`
        //      is uninitialised heap, so a zeroing allocator is used to make
        //      the observation well-defined and identical for both libraries.
        {
            let gz = Two {
                c: (p.c.general_context_create)(Some(zeroing_malloc), Some(raw_free), ptr::null_mut()),
                r: (p.r.general_context_create)(Some(zeroing_malloc), Some(raw_free), ptr::null_mut()),
            };
            let md = md_create2(p, 8, gz);
            let (mut soc, mut sor) = (0xAAAA_AAAAusize, 0xAAAA_AAAAusize);
            let (mut opc, mut opr) = (0xBBBB_BBBBu32, 0xBBBB_BBBBu32);
            let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
            let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
            d.eq("row264 unused match_data next rc", mc, mr);
            d.eq("row264 unused match_data next out", (soc, opc), (sor, opr));
            d.eq("row264 unused match_data: no diagnostic, returns FALSE", mc, 0);
            free_md2(p, md);
            (p.c.general_context_free)(gz.c);
            (p.r.general_context_free)(gz.r);
        }
        //  (b) ovector[1] hand-set to PCRE2_UNSET after a successful match: the
        //      assertion would fire in a PCRE2_DEBUG build; here the function
        //      simply hands back the nonsense offset.
        {
            let code = compile2(p, b"a", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, b"a".as_ptr(), 1, 0, 0, NONE2);
            d.eq("row264 poke fixture", c.clone(), r);
            let ovc = (p.c.get_ovector_pointer)(md.c);
            let ovr = (p.r.get_ovector_pointer)(md.r);
            *ovc.add(0) = 0;
            *ovc.add(1) = PCRE2_UNSET;
            *ovr.add(0) = 0;
            *ovr.add(1) = PCRE2_UNSET;
            let (mut soc, mut sor) = (0usize, 0usize);
            let (mut opc, mut opr) = (0u32, 0u32);
            let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
            let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
            d.eq("row264 ovector[1]=UNSET next rc", mc, mr);
            d.eq("row264 ovector[1]=UNSET next out", (soc, opc), (sor, opr));
            d.eq("row264 ovector[1]=UNSET returns TRUE (assert is a no-op)", mc, 1);
            d.eq("row264 ovector[1]=UNSET hands back UNSET", soc, PCRE2_UNSET);
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- NULL out-pointers.  Safe only on the FALSE paths, which return
        // before writing anything; on a TRUE path the C dereferences them.
        {
            let code = compile2(p, b"z", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (c, r) = match2(p, code, md, b"a".as_ptr(), 1, 0, 0, NONE2);
            d.eq("row261 NULL-out fixture", c.clone(), r);
            let mc = (p.c.next_match)(md.c, ptr::null_mut(), ptr::null_mut());
            let mr = (p.r.next_match)(md.r, ptr::null_mut(), ptr::null_mut());
            d.eq("row261 next_match(md, NULL, NULL) after failure", mc, mr);
            d.eq("row261 next_match(md, NULL, NULL) returns FALSE", mc, 0);
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- row 265: `do_bumpalong` indexes `subject[offset]` with no bounds
        // check when `match_data->subject` and `subject_length` are mutually
        // inconsistent.  Producing that state means writing private fields with
        // a deliberately wrong length, after which the read is out of bounds in
        // the C itself — not a comparable observable, so it is not attempted.
        // The `KK` table above exercises every in-bounds branch of
        // do_bumpalong instead (plain, CRLF and UTF advance).
    }
    d.finish("rows 261-265: pcre2_next_match_8 (before/after failure, past the end, NULL outputs)");
}

// ===================================================================== 266-270

#[test]
fn r266_270_study() {
    struct SCase {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        /// documented `_pcre2_study_8` return value
        expect: c_int,
    }
    // Rows 266-268 are the three non-zero returns; all three are guarded by
    // PCRE2_DEBUG_UNREACHABLE and need an opcode the scanners do not know, i.e.
    // corrupted bytecode reached from `pcre2_compile_8` — impossible for a
    // pattern that compiled successfully.  They are listed here so the sweep
    // records them, and the assertion for them is that `_pcre2_study_8` agrees
    // between C and Rust (return value AND the resulting code block) for every
    // reachable construct, which is the nearest reachable input.
    const S: &[SCase] = &[
        SCase { rows: &[266], pat: "[a-z]+", copts: 0, expect: 0 },
        SCase { rows: &[267], pat: "(?<n>a)\\k<n>", copts: 0, expect: 0 },
        SCase { rows: &[268], pat: "(a)\\1(b)\\2", copts: 0, expect: 0 },
        // row 269: find_minlength returns -1 -> study returns 0, minlength 0
        SCase { rows: &[269], pat: "(*UTF)\\C", copts: 0, expect: 0 },
        SCase { rows: &[269], pat: "\\Cabc", copts: PCRE2_UTF, expect: 0 },
        SCase { rows: &[269], pat: "a(*ACCEPT)bcd", copts: 0, expect: 0 },
        // row 270: set_start_bits returns SSB_FAIL / SSB_TOODEEP -> no bitmap
        SCase { rows: &[270], pat: "\\X+", copts: PCRE2_UTF, expect: 0 },
        SCase { rows: &[270], pat: "\\p{L}", copts: PCRE2_UTF, expect: 0 },
        SCase { rows: &[270], pat: "\\C", copts: PCRE2_UTF, expect: 0 },
        SCase { rows: &[270], pat: "(*UTF)\\p{Greek}+", copts: 0, expect: 0 },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // the documented triggers, plus a broad sweep over the shared corpus
        let mut pats: Vec<(&[u8], u32)> = S.iter().map(|s| (s.pat.as_bytes(), s.copts)).collect();
        for pat in PATTERNS.iter() {
            pats.push((pat.as_bytes(), 0));
        }
        for (pat, copts) in pats {
            let (mut ec, mut er) = (0 as c_int, 0 as c_int);
            let (mut oc, mut or) = (0usize, 0usize);
            let a = (p.c.compile)(pat.as_ptr(), pat.len(), copts, &mut ec, &mut oc, ptr::null_mut());
            let b = (p.r.compile)(pat.as_ptr(), pat.len(), copts, &mut er, &mut or, ptr::null_mut());
            d.eq(&format!("study fixture {} compiles", show(pat)), (a.is_null(), ec), (b.is_null(), er));
            if a.is_null() || b.is_null() {
                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }
                continue;
            }
            // MINLENGTH / FIRSTBITMAP as produced by the compile-time study
            let (mut mlc, mut mlr) = (0u32, 0u32);
            d.eq(
                &format!("study {} MINLENGTH rc", show(pat)),
                (p.c.pattern_info)(a, PCRE2_INFO_MINLENGTH, &mut mlc as *mut u32 as Ptr),
                (p.r.pattern_info)(b, PCRE2_INFO_MINLENGTH, &mut mlr as *mut u32 as Ptr),
            );
            d.eq(&format!("study {} MINLENGTH", show(pat)), mlc, mlr);
            let (mut bmc, mut bmr) = (ptr::null::<u8>(), ptr::null::<u8>());
            d.eq(
                &format!("study {} FIRSTBITMAP rc", show(pat)),
                (p.c.pattern_info)(a, PCRE2_INFO_FIRSTBITMAP, &mut bmc as *mut *const u8 as Ptr),
                (p.r.pattern_info)(b, PCRE2_INFO_FIRSTBITMAP, &mut bmr as *mut *const u8 as Ptr),
            );
            d.eq(&format!("study {} FIRSTBITMAP set", show(pat)), bmc.is_null(), bmr.is_null());
            if !bmc.is_null() && !bmr.is_null() {
                d.eq(
                    &format!("study {} bitmap bytes", show(pat)),
                    std::slice::from_raw_parts(bmc, 32).to_vec(),
                    std::slice::from_raw_parts(bmr, 32).to_vec(),
                );
            }
            // and the exported entry point itself
            let rc = (p.c.p_study)(a);
            let rr = (p.r.p_study)(b);
            d.eq(&format!("_pcre2_study_8({}) rc", show(pat)), rc, rr);
            assert_code_eq(a, b, &format!("_pcre2_study_8({}) code block", show(pat)));
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        // the documented return values
        for s in S {
            let code = compile2(p, s.pat.as_bytes(), s.copts, NONE2);
            let rc = (p.c.p_study)(code.c);
            let rr = (p.r.p_study)(code.r);
            d.eq(&format!("rows {:?} /{}/ study rc", s.rows, s.pat), rc, rr);
            d.eq(&format!("rows {:?} /{}/ study rc vs ERRORS.md", s.rows, s.pat), rc, s.expect);
            if s.rows.contains(&269) {
                let (mut mlc, mut mlr) = (0u32, 0u32);
                (p.c.pattern_info)(code.c, PCRE2_INFO_MINLENGTH, &mut mlc as *mut u32 as Ptr);
                (p.r.pattern_info)(code.r, PCRE2_INFO_MINLENGTH, &mut mlr as *mut u32 as Ptr);
                d.eq(&format!("row269 /{}/ minlength", s.pat), mlc, mlr);
            }
            if s.rows.contains(&270) {
                let (mut bmc, mut bmr) = (ptr::null::<u8>(), ptr::null::<u8>());
                (p.c.pattern_info)(code.c, PCRE2_INFO_FIRSTBITMAP, &mut bmc as *mut *const u8 as Ptr);
                (p.r.pattern_info)(code.r, PCRE2_INFO_FIRSTBITMAP, &mut bmr as *mut *const u8 as Ptr);
                d.eq(&format!("row270 /{}/ no bitmap", s.pat), bmc.is_null(), bmr.is_null());
                d.eq(&format!("row270 /{}/ bitmap is absent", s.pat), bmc.is_null(), true);
            }
            free_code2(p, code);
        }
        // rows 218/266-268 also reach pcre2_compile_8: a successful compile
        // must never report PCRE2_ERROR_INTERNAL_STUDY_ERROR (131).
        for pat in PATTERNS.iter().take(60) {
            let pb = pat.as_bytes();
            let (mut ec, mut er) = (0 as c_int, 0 as c_int);
            let (mut oc, mut or) = (0usize, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut oc, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut er, &mut or, ptr::null_mut());
            d.eq(&format!("rows266-268 compile {} ec", show(pb)), ec, er);
            d.eq(&format!("rows266-268 compile {} is not 131", show(pb)), ec != 131, true);
            if !a.is_null() {
                (p.c.code_free)(a);
            }
            if !b.is_null() {
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("rows 266-270: _pcre2_study_8 return values, minlength and start bitmap");
}

// ===================================================================== 271-277

#[test]
fn r271_277_jit_stubs() {
    struct JCase {
        rows: &'static [u32],
        options: u32,
        code_null: bool,
        expect: c_int,
    }
    const J: &[JCase] = &[
        // row 271: TEST_ALLOC combined with anything else
        JCase { rows: &[271], options: PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[271], options: PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_PARTIAL_SOFT, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[271], options: PCRE2_JIT_TEST_ALLOC | 0x10, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[271], options: PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE, code_null: true, expect: PCRE2_ERROR_JIT_BADOPTION },
        // row 272: exactly TEST_ALLOC, checked before the code==NULL test
        JCase { rows: &[272], options: PCRE2_JIT_TEST_ALLOC, code_null: false, expect: PCRE2_ERROR_JIT_UNSUPPORTED },
        JCase { rows: &[272], options: PCRE2_JIT_TEST_ALLOC, code_null: true, expect: PCRE2_ERROR_JIT_UNSUPPORTED },
        // row 273: code == NULL with any other options
        JCase { rows: &[273], options: 0, code_null: true, expect: PCRE2_ERROR_NULL },
        JCase { rows: &[273], options: PCRE2_JIT_COMPLETE, code_null: true, expect: PCRE2_ERROR_NULL },
        JCase { rows: &[273], options: 0x10, code_null: true, expect: PCRE2_ERROR_NULL },
        JCase { rows: &[273], options: 0xFFFF_FFFF & !PCRE2_JIT_TEST_ALLOC, code_null: true, expect: PCRE2_ERROR_NULL },
        // row 274: a bit outside PUBLIC_JIT_COMPILE_OPTIONS
        JCase { rows: &[274], options: 0x10, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[274], options: 0x8000_0000, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[274], options: 0xFFFF_FFFF & !PCRE2_JIT_TEST_ALLOC, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        // row 275: every otherwise-valid call still fails, no JIT support
        JCase { rows: &[275], options: 0, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[275], options: PCRE2_JIT_COMPLETE, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[275], options: PCRE2_JIT_PARTIAL_SOFT, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[275], options: PCRE2_JIT_PARTIAL_HARD, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
        JCase { rows: &[275], options: PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD, code_null: false, expect: PCRE2_ERROR_JIT_BADOPTION },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // ---- rows 271-275: every option combination, plus all 32 single bits
        for j in J {
            let code = compile2(p, b"abc", 0, NONE2);
            let (cc, cr) = if j.code_null {
                (ptr::null_mut(), ptr::null_mut())
            } else {
                (code.c, code.r)
            };
            let rc = (p.c.jit_compile)(cc, j.options);
            let rr = (p.r.jit_compile)(cr, j.options);
            d.eq(
                &format!("rows {:?} jit_compile(code={}, {:#x})", j.rows, !j.code_null, j.options),
                rc,
                rr,
            );
            d.eq(
                &format!("rows {:?} jit_compile({:#x}) vs ERRORS.md", j.rows, j.options),
                rc,
                j.expect,
            );
            free_code2(p, code);
        }
        for bit in 0..32u32 {
            let o = 1u32 << bit;
            let code = compile2(p, b"abc", 0, NONE2);
            let rc = (p.c.jit_compile)(code.c, o);
            let rr = (p.r.jit_compile)(code.r, o);
            d.eq(&format!("rows271-275 jit_compile bit {bit} ({o:#x})"), rc, rr);
            let rc2 = (p.c.jit_compile)(ptr::null_mut(), o);
            let rr2 = (p.r.jit_compile)(ptr::null_mut(), o);
            d.eq(&format!("row273 jit_compile(NULL) bit {bit} ({o:#x})"), rc2, rr2);
            // and the ALLOPTIONS side effect of PCRE2_JIT_INVALID_UTF (row 275)
            let (mut aoc, mut aor) = (0u32, 0u32);
            (p.c.pattern_info)(code.c, PCRE2_INFO_ALLOPTIONS, &mut aoc as *mut u32 as Ptr);
            (p.r.pattern_info)(code.r, PCRE2_INFO_ALLOPTIONS, &mut aor as *mut u32 as Ptr);
            d.eq(&format!("row275 ALLOPTIONS after bit {bit}"), aoc, aor);
            if o == PCRE2_JIT_INVALID_UTF {
                d.eq(
                    "row275 PCRE2_JIT_INVALID_UTF ORs in PCRE2_MATCH_INVALID_UTF",
                    aoc & PCRE2_MATCH_INVALID_UTF,
                    PCRE2_MATCH_INVALID_UTF,
                );
            }
            free_code2(p, code);
        }

        // ---- row 276: pcre2_jit_match_8 fails unconditionally and stores the
        // code in match_data->rc (observable through pcre2_next_match_8, which
        // returns FALSE for a negative stored rc).
        {
            let code = compile2(p, b"abc", 0, NONE2);
            let md = md_from_pattern2(p, code);
            // first make the stored rc positive with a real match
            let (c, r) = match2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2);
            d.eq("row276 fixture match", c.clone(), r);
            d.eq("row276 fixture matched", c.rc, 1);
            for (subj, len, so, opts) in [
                (&b"abc"[..], 3usize, 0usize, 0u32),
                (&b"abc"[..], 3, 1, PCRE2_ANCHORED),
                (&b""[..], 0, 0, PCRE2_PARTIAL_HARD),
                (&b"abc"[..], 3, 0, 0xFFFF_FFFF),
            ] {
                let rc = (p.c.jit_match)(code.c, subj.as_ptr(), len, so, opts, md.c, ptr::null_mut());
                let rr = (p.r.jit_match)(code.r, subj.as_ptr(), len, so, opts, md.r, ptr::null_mut());
                d.eq(&format!("row276 jit_match opts={opts:#x}"), rc, rr);
                d.eq(&format!("row276 jit_match opts={opts:#x} vs ERRORS.md"), rc, PCRE2_ERROR_JIT_BADOPTION);
                // the stored rc is now negative
                let (mut soc, mut sor) = (7usize, 7usize);
                let (mut opc, mut opr) = (7u32, 7u32);
                let mc = (p.c.next_match)(md.c, &mut soc, &mut opc);
                let mr = (p.r.next_match)(md.r, &mut sor, &mut opr);
                d.eq(&format!("row276 stored rc via next_match opts={opts:#x}"), (mc, soc, opc), (mr, sor, opr));
                d.eq(&format!("row276 stored rc is negative opts={opts:#x}"), mc, 0);
            }
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- row 277: pcre2_jit_stack_create_8 always returns NULL
        {
            let gc = fallible_gcontext2(p);
            set_budgets(-1);
            for &(s, m) in &[
                (0usize, 0usize),
                (1, 1),
                (1, 0),
                (0, 1),
                (32 * 1024, 1024 * 1024),
                (1024 * 1024, 32 * 1024),
                (usize::MAX, usize::MAX),
            ] {
                for gcx in [NONE2, gc] {
                    let a = (p.c.jit_stack_create)(s, m, gcx.c);
                    let b = (p.r.jit_stack_create)(s, m, gcx.r);
                    d.eq(&format!("row277 jit_stack_create({s},{m})"), a.is_null(), b.is_null());
                    d.eq(&format!("row277 jit_stack_create({s},{m}) is NULL"), a.is_null(), true);
                    (p.c.jit_stack_free)(a);
                    (p.r.jit_stack_free)(b);
                }
            }
            // the no-rejection stubs accept everything, including NULL
            let mc = mcontext2(p);
            (p.c.jit_stack_assign)(mc.c, ptr::null_mut(), ptr::null_mut());
            (p.r.jit_stack_assign)(mc.r, ptr::null_mut(), ptr::null_mut());
            (p.c.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            (p.r.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            (p.c.jit_free_unused_memory)(ptr::null_mut());
            (p.r.jit_free_unused_memory)(ptr::null_mut());
            (p.c.jit_free_unused_memory)(gc.c);
            (p.r.jit_free_unused_memory)(gc.r);
            (p.c.jit_stack_free)(ptr::null_mut());
            (p.r.jit_stack_free)(ptr::null_mut());
            (p.c.p_jit_free)(ptr::null_mut(), ptr::null_mut());
            (p.r.p_jit_free)(ptr::null_mut(), ptr::null_mut());
            (p.c.p_jit_free_rodata)(ptr::null_mut(), ptr::null_mut());
            (p.r.p_jit_free_rodata)(ptr::null_mut(), ptr::null_mut());
            d.eq(
                "row277 _pcre2_jit_get_size_8(NULL)",
                (p.c.p_jit_get_size)(ptr::null_mut()),
                (p.r.p_jit_get_size)(ptr::null_mut()),
            );
            d.eq("row277 _pcre2_jit_get_size_8 is 0", (p.c.p_jit_get_size)(ptr::null_mut()), 0);
            let tc = std::ffi::CStr::from_ptr((p.c.p_jit_get_target)());
            let tr = std::ffi::CStr::from_ptr((p.r.p_jit_get_target)());
            d.eq("row277 _pcre2_jit_get_target_8", tc, tr);
            free_mcontext2(p, mc);
            (p.c.general_context_free)(gc.c);
            (p.r.general_context_free)(gc.r);
        }

        // ---- PCRE2_INFO_JITSIZE always reports 0
        {
            for pat in [&b"abc"[..], &b"(a)(b)"[..], &b"a+"[..]] {
                let code = compile2(p, pat, 0, NONE2);
                for _ in 0..2 {
                    let (mut jc, mut jr) = (0xDEADusize, 0xDEADusize);
                    let rc = (p.c.pattern_info)(code.c, PCRE2_INFO_JITSIZE, &mut jc as *mut usize as Ptr);
                    let rr = (p.r.pattern_info)(code.r, PCRE2_INFO_JITSIZE, &mut jr as *mut usize as Ptr);
                    d.eq(&format!("row277 JITSIZE rc {}", show(pat)), rc, rr);
                    d.eq(&format!("row277 JITSIZE {}", show(pat)), jc, jr);
                    d.eq(&format!("row277 JITSIZE {} is 0", show(pat)), jc, 0);
                    // after a jit_compile attempt it is still 0
                    (p.c.jit_compile)(code.c, PCRE2_JIT_COMPLETE);
                    (p.r.jit_compile)(code.r, PCRE2_JIT_COMPLETE);
                }
                free_code2(p, code);
            }
        }
    }
    d.finish("rows 271-277: the non-JIT stubs of pcre2_jit_compile.c");
}

// ===================================================================== 278-293

#[test]
fn r278_293_dfa_argument_validation() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let code = compile2(p, b"abc", 0, NONE2);
        let md = md_from_pattern2(p, code);
        let (mut wc, mut wr) = (ws(1000), ws(1000));

        // ---- row 278: match_data == NULL, checked before everything else
        for &(cd, so, wsn) in &[(code, 0usize, 1000usize), (NONE2, 0, 1000), (code, 99, 3)] {
            let (mut a, mut b) = (ws(wsn.max(1)), ws(wsn.max(1)));
            let rc = (p.c.dfa_match)(cd.c, b"abc".as_ptr(), 3, so, 0, ptr::null_mut(), ptr::null_mut(), a.as_mut_ptr(), wsn);
            let rr = (p.r.dfa_match)(cd.r, b"abc".as_ptr(), 3, so, 0, ptr::null_mut(), ptr::null_mut(), b.as_mut_ptr(), wsn);
            d.eq(&format!("row278 md=NULL so={so} ws={wsn}: C vs rust"), rc, rr);
            d.eq(&format!("row278 md=NULL so={so} ws={wsn}: vs ERRORS.md"), rc, PCRE2_ERROR_NULL);
        }

        // ---- row 279: code == NULL
        let (c, r) = dfa2(p, NONE2, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
        check(&mut d, "row279 code=NULL", &c, &r, PCRE2_ERROR_NULL);

        // ---- row 280: subject == NULL with length != 0
        for &len in &[1usize, 3, PCRE2_ZERO_TERMINATED] {
            let (c, r) = dfa2(p, code, md, ptr::null(), len, 0, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row280 subject=NULL len={len:#x}"), &c, &r, PCRE2_ERROR_NULL);
        }
        let (c, r) = dfa2(p, code, md, ptr::null(), 0, 0, 0, NONE2, &mut wc, &mut wr);
        d.eq("row280 subject=NULL len=0 (legal): C vs rust", c.clone(), r);
        d.eq("row280 subject=NULL len=0 is not NULL error", c.rc, PCRE2_ERROR_NOMATCH);

        // ---- row 281: workspace == NULL
        for &wsn in &[0usize, 19, 20, 1000] {
            let (c, r) = dfa2_count(
                p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2, ptr::null_mut(), ptr::null_mut(), wsn,
            );
            check(&mut d, &format!("row281 workspace=NULL wscount={wsn}"), &c, &r, PCRE2_ERROR_NULL);
        }

        // ---- row 282: options outside PUBLIC_DFA_MATCH_OPTIONS, every bit
        const PUBLIC_DFA_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
            | PCRE2_ENDANCHORED
            | PCRE2_NOTBOL
            | PCRE2_NOTEOL
            | PCRE2_NOTEMPTY
            | PCRE2_NOTEMPTY_ATSTART
            | PCRE2_NO_UTF_CHECK
            | PCRE2_PARTIAL_HARD
            | PCRE2_PARTIAL_SOFT
            | PCRE2_DFA_SHORTEST
            | PCRE2_DFA_RESTART
            | PCRE2_COPY_MATCHED_SUBJECT;
        for bit in 0..32u32 {
            let o = 1u32 << bit;
            // the workspace is re-zeroed so PCRE2_DFA_RESTART is deterministic
            for v in wc.iter_mut() {
                *v = 0;
            }
            for v in wr.iter_mut() {
                *v = 0;
            }
            let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, o, NONE2, &mut wc, &mut wr);
            let want = if o & PUBLIC_DFA_MATCH_OPTIONS == 0 {
                PCRE2_ERROR_BADOPTION
            } else {
                c.rc
            };
            check(&mut d, &format!("row282 bit {bit} ({o:#010x})"), &c, &r, want);
        }
        for o in [
            0xFFFF_FFFFu32,
            PCRE2_NO_JIT,
            PCRE2_DISABLE_RECURSELOOP_CHECK,
            0x0000_0200,
            PCRE2_ANCHORED | PCRE2_NO_JIT,
        ] {
            for v in wc.iter_mut() {
                *v = 0;
            }
            for v in wr.iter_mut() {
                *v = 0;
            }
            let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, o, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row282 opts={o:#x}"), &c, &r, PCRE2_ERROR_BADOPTION);
        }

        // ---- row 283: wscount < 20 (checked BEFORE start_offset)
        for wsn in [0usize, 1, 2, 10, 19] {
            let (c, r) = dfa2_count(
                p, code, md, b"abc".as_ptr(), 3, 99, 0, NONE2, wc.as_mut_ptr(), wr.as_mut_ptr(), wsn,
            );
            check(&mut d, &format!("row283 wscount={wsn}"), &c, &r, PCRE2_ERROR_DFA_WSSIZE);
        }
        // 20 is the smallest legal value
        let (c, r) = dfa2_count(
            p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2, wc.as_mut_ptr(), wr.as_mut_ptr(), 20,
        );
        d.eq("row283 wscount=20 (legal): C vs rust", c.clone(), r);
        d.eq("row283 wscount=20 is not DFA_WSSIZE", c.rc, 1);

        // ---- row 284: start_offset > length
        for &(len, so) in &[(3usize, 4usize), (3, 100), (0, 1)] {
            let (c, r) = dfa2(p, code, md, b"abc\0".as_ptr(), len, so, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row284 len={len} so={so}"), &c, &r, PCRE2_ERROR_BADOFFSET);
        }
        for &so in &[4usize, 9] {
            let (c, r) = dfa2(p, code, md, b"abc\0".as_ptr(), PCRE2_ZERO_TERMINATED, so, 0, NONE2, &mut wc, &mut wr);
            check(&mut d, &format!("row284 zero-terminated so={so}"), &c, &r, PCRE2_ERROR_BADOFFSET);
        }

        free_md2(p, md);
        free_code2(p, code);

        // ---- row 289: PCRE2_DFA_RESTART with invalid workspace contents
        {
            let code = compile2(p, b"abcd", 0, NONE2);
            let md = md_from_pattern2(p, code);
            // an all-zero workspace is invalid (workspace[1] < 1)
            let (mut a, mut b) = (ws(1000), ws(1000));
            let (c, r) = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, PCRE2_DFA_RESTART, NONE2, &mut a, &mut b);
            check(&mut d, "row289 all-zero workspace", &c, &r, PCRE2_ERROR_DFA_BADRESTART);

            // establish a genuine partial match, then corrupt the workspace
            let fresh = |a: &mut Vec<c_int>, b: &mut Vec<c_int>| {
                for v in a.iter_mut() {
                    *v = 0;
                }
                for v in b.iter_mut() {
                    *v = 0;
                }
            };
            fresh(&mut a, &mut b);
            let (c, r) = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, NONE2, &mut a, &mut b);
            d.eq("row289 partial fixture: C vs rust", c.clone(), r);
            d.eq("row289 partial fixture is PARTIAL", c.rc, PCRE2_ERROR_PARTIAL);
            d.eq("row289 workspace after partial", (a[0], a[1]), (b[0], b[1]));
            let (good0, good1) = (a[0], a[1]);
            // a valid restart works
            let (c, r) = dfa2(p, code, md, b"abcd".as_ptr(), 4, 2, PCRE2_DFA_RESTART, NONE2, &mut a, &mut b);
            d.eq("row289 valid restart: C vs rust", c.clone(), r);
            d.eq("row289 valid restart succeeds", c.rc, 1);
            // (workspace[0] & ~1) != 0
            for bad0 in [2, 3, 4, -1, i32::MIN] {
                fresh(&mut a, &mut b);
                let _ = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, NONE2, &mut a, &mut b);
                a[0] = bad0;
                b[0] = bad0;
                let (c, r) = dfa2(p, code, md, b"abcd".as_ptr(), 4, 2, PCRE2_DFA_RESTART, NONE2, &mut a, &mut b);
                check(&mut d, &format!("row289 workspace[0]={bad0}"), &c, &r, PCRE2_ERROR_DFA_BADRESTART);
            }
            // workspace[1] < 1
            for bad1 in [0, -1, i32::MIN] {
                fresh(&mut a, &mut b);
                let _ = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, NONE2, &mut a, &mut b);
                a[1] = bad1;
                b[1] = bad1;
                let (c, r) = dfa2(p, code, md, b"abcd".as_ptr(), 4, 2, PCRE2_DFA_RESTART, NONE2, &mut a, &mut b);
                check(&mut d, &format!("row289 workspace[1]={bad1}"), &c, &r, PCRE2_ERROR_DFA_BADRESTART);
            }
            // workspace[1] > (wscount-2)/INTS_PER_STATEBLOCK
            for (bad1, wsn) in [(100, 20usize), (1000, 20), (i32::MAX, 64)] {
                fresh(&mut a, &mut b);
                let _ = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, NONE2, &mut a, &mut b);
                a[0] = good0;
                b[0] = good0;
                a[1] = bad1;
                b[1] = bad1;
                let (c, r) = dfa2_count(
                    p, code, md, b"abcd".as_ptr(), 4, 2, PCRE2_DFA_RESTART, NONE2, a.as_mut_ptr(), b.as_mut_ptr(), wsn,
                );
                check(
                    &mut d,
                    &format!("row289 workspace[1]={bad1} wscount={wsn}"),
                    &c,
                    &r,
                    PCRE2_ERROR_DFA_BADRESTART,
                );
            }
            let _ = good1;
            free_md2(p, md);
            free_code2(p, code);
        }
    }
    d.finish("rows 278-284,289 (+287,288,290-293 covered elsewhere): pcre2_dfa_match_8 argument validation");
}

// ===================================================================== 295-305, 322

#[test]
fn r295_322_dfa_unsupported_items() {
    struct DCase {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        subj: &'static [u8],
        expect: c_int,
    }
    const D: &[DCase] = &[
        // row 295: quantified \C (OP_ANYBYTE only exists in UTF mode in the
        // 8-bit library — see the report; without PCRE2_UTF the compiler emits
        // OP_ALLANY and the DFA handles it).
        DCase { rows: &[295], pat: "\\C*", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[295], pat: "\\C+", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[295], pat: "\\C?", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[295], pat: "\\C{3}", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[295], pat: "\\C{2,3}", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        // row 296: unquantified \C
        DCase { rows: &[296], pat: "a\\Cb", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[296], pat: "\\C", copts: PCRE2_UTF, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        // row 297: back references
        DCase { rows: &[297], pat: "(a)\\1", copts: 0, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[297], pat: "(?i)(a)\\1", copts: 0, subj: b"aA", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[297], pat: "(a)\\g{1}", copts: 0, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[297], pat: "(?J)(?<n>a)(?<n>b)\\k<n>", copts: 0, subj: b"aba", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[297], pat: "(?Ji)(?<n>a)(?<n>b)\\k<n>", copts: 0, subj: b"abA", expect: PCRE2_ERROR_DFA_UITEM },
        // row 298: \K
        DCase { rows: &[298], pat: "ab\\Kcd", copts: 0, subj: b"abcd", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[298], pat: "\\Kabc", copts: 0, subj: b"abc", expect: PCRE2_ERROR_DFA_UITEM },
        // row 299: backtracking-control verbs
        DCase { rows: &[299], pat: "a(*MARK:X)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[299], pat: "a(*PRUNE)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[299], pat: "a(*SKIP)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[299], pat: "a(*THEN)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[299], pat: "a(*COMMIT)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[299], pat: "a(*ACCEPT)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        // row 300: script runs
        DCase { rows: &[300], pat: "(*script_run:\\w+)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[300], pat: "(*sr:ab)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[300], pat: "(*atomic_script_run:\\w+)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        // row 301: non-atomic lookarounds
        DCase { rows: &[301], pat: "(*napla:a)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[301], pat: "(*naplb:a)b", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        // row 302: scan-substring assertion
        DCase { rows: &[302], pat: "(a)(*scs:(1)b)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UITEM },
        // row 303: OP_RECURSE followed by OP_CREF
        DCase { rows: &[303], pat: "(a)(?1(1))", copts: 0, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        // row 304: conditions on whether a group is set
        DCase { rows: &[304], pat: "(a)?(?(1)b|c)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UCOND },
        DCase { rows: &[304], pat: "(?<n>a)?(?(n)b|c)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UCOND },
        DCase { rows: &[304], pat: "(?<n>a)?(?(<n>)b|c)", copts: 0, subj: b"ab", expect: PCRE2_ERROR_DFA_UCOND },
        DCase { rows: &[304], pat: "(?J)(?<n>a)(?<n>b)(?(R&n)x|y)", copts: 0, subj: b"aby", expect: PCRE2_ERROR_DFA_UCOND },
        // row 305: OP_RREF for a specific group
        DCase { rows: &[305], pat: "(a)(?(R1)b|c)", copts: 0, subj: b"ac", expect: PCRE2_ERROR_DFA_UCOND },
        DCase { rows: &[305], pat: "(?<n>a)(?(R&n)b|c)", copts: 0, subj: b"ac", expect: PCRE2_ERROR_DFA_UCOND },
        // row 322: a nested internal_dfa_match error propagates verbatim
        DCase { rows: &[322], pat: "(?=\\C)a", copts: PCRE2_UTF, subj: b"a", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(?:(?=\\C))a", copts: PCRE2_UTF, subj: b"a", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(?<=\\C)a", copts: PCRE2_UTF, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(?>\\C)a", copts: PCRE2_UTF, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(\\C)++a", copts: PCRE2_UTF, subj: b"aa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(\\Ca)(?1)", copts: PCRE2_UTF, subj: b"aaaa", expect: PCRE2_ERROR_DFA_UITEM },
        DCase { rows: &[322], pat: "(?(?=\\C)a|b)", copts: PCRE2_UTF, subj: b"a", expect: PCRE2_ERROR_DFA_UITEM },
    ];
    // constructs the DFA *does* support, so the rows above are not vacuous
    const SUPPORTED: &[DCase] = &[
        DCase { rows: &[299], pat: "a(*FAIL)|b", copts: 0, subj: b"ab", expect: 1 },
        DCase { rows: &[299], pat: "a(*F)|b", copts: 0, subj: b"ab", expect: 1 },
        DCase { rows: &[305], pat: "(?(R)b|c)", copts: 0, subj: b"c", expect: 1 },
        DCase { rows: &[295, 296], pat: "\\C*", copts: 0, subj: b"abc", expect: 1 },
        DCase { rows: &[295, 296], pat: "a\\Cb", copts: 0, subj: b"abc", expect: PCRE2_ERROR_NOMATCH },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for case in D.iter().chain(SUPPORTED.iter()) {
            let code = compile2(p, case.pat.as_bytes(), case.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let (mut wc, mut wr) = (ws(1000), ws(1000));
            let (c, r) = dfa2(
                p, code, md, case.subj.as_ptr(), case.subj.len(), 0, 0, NONE2, &mut wc, &mut wr,
            );
            check(
                &mut d,
                &format!("rows {:?} dfa /{}/ on {}", case.rows, case.pat, show(case.subj)),
                &c,
                &r,
                case.expect,
            );
            // the same pattern under pcre2_match_8 must behave identically in
            // both libraries too (it supports all of these constructs)
            let (c, r) = match2(p, code, md, case.subj.as_ptr(), case.subj.len(), 0, 0, NONE2);
            d.eq(
                &format!("rows {:?} match /{}/ on {}", case.rows, case.pat, show(case.subj)),
                c.clone(),
                r,
            );
            free_md2(p, md);
            free_code2(p, code);
        }
    }
    d.finish("rows 295-305,322: constructs pcre2_dfa_match_8 refuses");
}

// ===================================================================== 294, 306-312

#[test]
fn r294_312_dfa_recursion_workspace_limits() {
    struct WCase {
        rows: &'static [u32],
        pat: &'static str,
        subj_a: usize,
        wscount: usize,
        expect: c_int,
    }
    const W: &[WCase] = &[
        // row 306: the nested call's fixed 1000-slot local ovector overflows
        WCase { rows: &[306], pat: "(a+)(?1)", subj_a: 700, wscount: 1000, expect: PCRE2_ERROR_DFA_RECURSE },
        WCase { rows: &[306], pat: "(a+)(?1)", subj_a: 900, wscount: 1000, expect: PCRE2_ERROR_DFA_RECURSE },
        // row 307: OP_RECURSE repeats the same group at the same position
        WCase { rows: &[307], pat: "((?2))((?1))", subj_a: 1, wscount: 1000, expect: PCRE2_ERROR_RECURSELOOP },
        WCase { rows: &[307], pat: "(a|(?2))((?1))", subj_a: 1, wscount: 1000, expect: PCRE2_ERROR_RECURSELOOP },
        WCase { rows: &[307], pat: "((?2)|a)((?1)|b)", subj_a: 1, wscount: 1000, expect: PCRE2_ERROR_RECURSELOOP },
        WCase { rows: &[307], pat: "((?2))((?3))((?1))", subj_a: 1, wscount: 1000, expect: PCRE2_ERROR_RECURSELOOP },
        // row 308: state-list overflow with the minimum legal wscount
        WCase { rows: &[308], pat: "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p)+", subj_a: 0, wscount: 20, expect: PCRE2_ERROR_DFA_WSSIZE },
        WCase { rows: &[308], pat: "(a+)(?1)", subj_a: 400, wscount: 1000, expect: PCRE2_ERROR_DFA_WSSIZE },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let big = vec![b'a'; 1000];
        for w in W {
            let code = compile2(p, w.pat.as_bytes(), 0, NONE2);
            let md = md_from_pattern2(p, code);
            let (mut wc, mut wr) = (ws(w.wscount), ws(w.wscount));
            let (sp, sl) = if w.pat.contains('|') && w.subj_a == 0 {
                (b"abcdefghijklmnop".as_ptr(), 16usize)
            } else {
                (big.as_ptr(), w.subj_a)
            };
            let (c, r) = dfa2(p, code, md, sp, sl, 0, 0, NONE2, &mut wc, &mut wr);
            check(
                &mut d,
                &format!("rows {:?} dfa /{}/ n={} ws={}", w.rows, w.pat, w.subj_a, w.wscount),
                &c,
                &r,
                w.expect,
            );
            free_md2(p, md);
            free_code2(p, code);
        }

        let mc = mcontext2(p);

        // ---- row 309: match limit (counts internal_dfa_match calls), swept
        for (pat, subj) in [(&b"(?:(?=a)a)+"[..], &b"aaa"[..]), (&b"(a)(?1)"[..], &b"aa"[..])] {
            let code = compile2(p, pat, 0, NONE2);
            let md = md_from_pattern2(p, code);
            let mut saw = false;
            for lim in 0u32..=12 {
                (p.c.set_match_limit)(mc.c, lim);
                (p.r.set_match_limit)(mc.r, lim);
                (p.c.set_depth_limit)(mc.c, 10_000_000);
                (p.r.set_depth_limit)(mc.r, 10_000_000);
                let (mut wc, mut wr) = (ws(1000), ws(1000));
                let (c, r) = dfa2(p, code, md, subj.as_ptr(), subj.len(), 0, 0, mc, &mut wc, &mut wr);
                d.eq(&format!("row309 dfa {} match_limit={lim}: C vs rust", show(pat)), c.clone(), r);
                if c.rc == PCRE2_ERROR_MATCHLIMIT {
                    saw = true;
                }
            }
            d.eq(&format!("row309 dfa {} reaches MATCHLIMIT", show(pat)), saw, true);
            free_md2(p, md);
            free_code2(p, code);
        }
        (p.c.set_match_limit)(mc.c, 10_000_000);
        (p.r.set_match_limit)(mc.r, 10_000_000);

        // ---- row 310: recursion depth limit, swept
        for pat in [&b"(?=(?=(?=a)))a"[..], &b"(?=(?=(?=(?=a))))a"[..]] {
            let code = compile2(p, pat, 0, NONE2);
            let md = md_from_pattern2(p, code);
            let mut saw = false;
            for lim in 0u32..=8 {
                (p.c.set_depth_limit)(mc.c, lim);
                (p.r.set_depth_limit)(mc.r, lim);
                let (mut wc, mut wr) = (ws(1000), ws(1000));
                let (c, r) = dfa2(p, code, md, b"a".as_ptr(), 1, 0, 0, mc, &mut wc, &mut wr);
                d.eq(&format!("row310 dfa {} depth_limit={lim}: C vs rust", show(pat)), c.clone(), r);
                if c.rc == PCRE2_ERROR_DEPTHLIMIT {
                    saw = true;
                }
            }
            d.eq(&format!("row310 dfa {} reaches DEPTHLIMIT", show(pat)), saw, true);
            free_md2(p, md);
            free_code2(p, code);
        }
        (p.c.set_depth_limit)(mc.c, 10_000_000);
        (p.r.set_depth_limit)(mc.r, 10_000_000);

        // ---- row 311: more_workspace() cannot grow within the heap limit.
        // The base recursion workspace is DFA_START_RWS_SIZE (30720 bytes =
        // 7680 ints) and each nested call takes RWS_RSIZE + ovec, so eight
        // nested lookaheads exhaust it.
        {
            let pats: &[&[u8]] = &[
                b"(?=(?=(?=(?=(?=(?=(?=(?=a))))))))a",
                b"(?=(?=(?=(?=(?=(?=(?=(?=(?=(?=a))))))))))a",
            ];
            for pat in pats {
                let code = compile2(p, pat, 0, NONE2);
                let md = md_from_pattern2(p, code);
                let mut saw = false;
                for hl in [0u32, 1, 2, 3, 4, 8, 16, 29, 30, 31, 32, 59, 60, 61, 120] {
                    (p.c.set_heap_limit)(mc.c, hl);
                    (p.r.set_heap_limit)(mc.r, hl);
                    let (mut wc, mut wr) = (ws(1000), ws(1000));
                    let (c, r) = dfa2(p, code, md, b"a".as_ptr(), 1, 0, 0, mc, &mut wc, &mut wr);
                    d.eq(&format!("row311 dfa {} heap_limit={hl}: C vs rust", show(pat)), c.clone(), r);
                    if c.rc == PCRE2_ERROR_HEAPLIMIT {
                        saw = true;
                    }
                }
                d.eq(&format!("row311 dfa {} reaches HEAPLIMIT", show(pat)), saw, true);
                free_md2(p, md);
                free_code2(p, code);
            }
            (p.c.set_heap_limit)(mc.c, 20_000_000);
            (p.r.set_heap_limit)(mc.r, 20_000_000);
        }
        free_mcontext2(p, mc);

        // ---- row 312: more_workspace()'s malloc fails.  mb->memctl comes from
        // the match context (or the code), not the match data.
        {
            let gc = fallible_gcontext2(p);
            set_budgets(-1);
            let fmc = Two {
                c: (p.c.match_context_create)(gc.c),
                r: (p.r.match_context_create)(gc.r),
            };
            assert!(!fmc.c.is_null() && !fmc.r.is_null());
            let code = compile2(p, b"(?=(?=(?=(?=(?=(?=(?=(?=a))))))))a", 0, NONE2);
            let md = md_from_pattern2(p, code);
            let mut saw = false;
            for n in 0..=4i64 {
                let (mut wc, mut wr) = (ws(1000), ws(1000));
                set_budgets(n);
                let (c, r) = dfa2(p, code, md, b"a".as_ptr(), 1, 0, 0, fmc, &mut wc, &mut wr);
                set_budgets(-1);
                d.eq(&format!("row312 dfa budget={n}: C vs rust"), c.clone(), r);
                if c.rc == PCRE2_ERROR_NOMEMORY {
                    saw = true;
                }
            }
            d.eq("row312 reaches NOMEMORY", saw, true);
            free_md2(p, md);
            free_code2(p, code);

            // ---- row 294: PCRE2_COPY_MATCHED_SUBJECT and the subject copy's
            // malloc fails.  This one uses match_data->memctl.
            let code = compile2(p, b"abc", 0, NONE2);
            let mut saw = false;
            for n in 0..=3i64 {
                set_budgets(-1);
                let md = md_create2(p, 4, gc);
                let (mut wc, mut wr) = (ws(1000), ws(1000));
                set_budgets(n);
                let (c, r) = dfa2(
                    p, code, md, b"abc".as_ptr(), 3, 0, PCRE2_COPY_MATCHED_SUBJECT, NONE2, &mut wc, &mut wr,
                );
                set_budgets(-1);
                d.eq(&format!("row294 dfa COPY_MATCHED_SUBJECT budget={n}: C vs rust"), c.clone(), r);
                if c.rc == PCRE2_ERROR_NOMEMORY {
                    saw = true;
                }
                free_md2(p, md);
            }
            d.eq("row294 reaches NOMEMORY", saw, true);
            free_code2(p, code);

            (p.c.match_context_free)(fmc.c);
            (p.r.match_context_free)(fmc.r);
            (p.c.general_context_free)(gc.c);
            (p.r.general_context_free)(gc.r);
        }
    }
    d.finish("rows 294,306-312: DFA recursion, workspace and resource limits");
}

// ===================================================================== 313-321

#[test]
fn r313_321_dfa_outcomes() {
    struct DOut {
        rows: &'static [u32],
        pat: &'static str,
        copts: u32,
        subj: &'static [u8],
        mopts: u32,
        expect: c_int,
    }
    const D: &[DOut] = &[
        // row 313: PARTIAL_HARD with \z at/after the end
        DOut { rows: &[313], pat: "abc\\z", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        DOut { rows: &[313], pat: "a\\z", copts: 0, subj: b"a", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        // row 314: PARTIAL_HARD with \Z at the end
        DOut { rows: &[314], pat: "abc\\Z", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        DOut { rows: &[314], pat: "a\\Z", copts: 0, subj: b"a", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        // row 315: no new states but could_continue / soft partial
        DOut { rows: &[315], pat: "abcd", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_SOFT, expect: PCRE2_ERROR_PARTIAL },
        DOut { rows: &[315], pat: "abcd", copts: 0, subj: b"abc", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        DOut { rows: &[315], pat: "ab+", copts: 0, subj: b"ab", mopts: PCRE2_PARTIAL_HARD, expect: PCRE2_ERROR_PARTIAL },
        // row 316: match found but ENDANCHORED and ptr < end_subject
        DOut { rows: &[316], pat: "ab", copts: PCRE2_ENDANCHORED, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        DOut { rows: &[316], pat: "ab", copts: 0, subj: b"abc", mopts: PCRE2_ENDANCHORED, expect: PCRE2_ERROR_NOMATCH },
        DOut { rows: &[316], pat: "a", copts: PCRE2_ENDANCHORED, subj: b"aa", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        // row 317: bumpalong exhausted / start optimizations prove failure
        DOut { rows: &[317], pat: "xyz", copts: 0, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        DOut { rows: &[317], pat: "abcd", copts: 0, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        DOut { rows: &[317], pat: "^xyz", copts: 0, subj: b"abc", mopts: 0, expect: PCRE2_ERROR_NOMATCH },
        DOut { rows: &[317], pat: "abc", copts: 0, subj: b"abd", mopts: PCRE2_ANCHORED, expect: PCRE2_ERROR_NOMATCH },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for case in D {
            let code = compile2(p, case.pat.as_bytes(), case.copts, NONE2);
            let md = md_from_pattern2(p, code);
            let (mut wc, mut wr) = (ws(1000), ws(1000));
            let (c, r) = dfa2(
                p, code, md, case.subj.as_ptr(), case.subj.len(), 0, case.mopts, NONE2, &mut wc, &mut wr,
            );
            check(
                &mut d,
                &format!("rows {:?} dfa /{}/ on {} mopts={:#x}", case.rows, case.pat, show(case.subj), case.mopts),
                &c,
                &r,
                case.expect,
            );
            free_md2(p, md);
            free_code2(p, code);
        }

        // ---- rows 318/319: match_count * 2 > offsetcount is NOT an error; the
        // longest match is still in ovector[0..1] and the rc is 0.
        // PCRE2_ERROR_UNSET (-55) is never produced by pcre2_dfa_match_8.
        {
            let code = compile2(p, b"a|ab|abc", 0, NONE2);
            for (rows, oveccount) in [(&[318u32][..], 1u32), (&[319][..], 0), (&[318][..], 2), (&[318][..], 3)] {
                let md = md_create2(p, oveccount, NONE2);
                let (mut wc, mut wr) = (ws(1000), ws(1000));
                let (c, r) = dfa2(p, code, md, b"abc".as_ptr(), 3, 0, 0, NONE2, &mut wc, &mut wr);
                let want = if oveccount <= 1 { 0 } else { 3.min(oveccount as c_int) };
                check(&mut d, &format!("rows {rows:?} dfa oveccount={oveccount}"), &c, &r, want);
                d.eq(
                    &format!("rows {rows:?} dfa oveccount={oveccount} is never -55"),
                    c.rc != PCRE2_ERROR_UNSET,
                    true,
                );
                if oveccount <= 1 {
                    d.eq(
                        &format!("rows {rows:?} longest match in ovector[0..1]"),
                        (c.ovector[0], c.ovector[1]),
                        (0usize, 3usize),
                    );
                }
                free_md2(p, md);
            }
            free_code2(p, code);
        }

        // ---- rows 320/321: negative callout returns propagate verbatim
        {
            let mc = mcontext2(p);
            (p.c.set_callout)(mc.c, Some(callout), ptr::null_mut());
            (p.r.set_callout)(mc.r, Some(callout), ptr::null_mut());
            struct CB {
                rows: &'static [u32],
                pat: &'static str,
                copts: u32,
                subj: &'static [u8],
            }
            const CBS: &[CB] = &[
                // row 320: the callout auto-inserted between OP_COND and an
                // assertion condition
                CB { rows: &[320], pat: "(?(?=a)b|a)", copts: PCRE2_AUTO_CALLOUT, subj: b"a" },
                CB { rows: &[320], pat: "(?(?=a)a|b)", copts: PCRE2_AUTO_CALLOUT, subj: b"a" },
                // row 321: OP_CALLOUT / OP_CALLOUT_STR
                CB { rows: &[321], pat: "a(?C1)b", copts: 0, subj: b"ab" },
                CB { rows: &[321], pat: "a(?C{txt})b", copts: 0, subj: b"ab" },
                CB { rows: &[321], pat: "a(?C)b", copts: 0, subj: b"ab" },
            ];
            for cb in CBS {
                let code = compile2(p, cb.pat.as_bytes(), cb.copts, NONE2);
                let md = md_from_pattern2(p, code);
                for v in [PCRE2_ERROR_CALLOUT, -99, -1000, i32::MIN + 1] {
                    *ptr::addr_of_mut!(CALLOUT_RET) = v;
                    let (mut wc, mut wr) = (ws(1000), ws(1000));
                    let (c, r) = dfa2(
                        p, code, md, cb.subj.as_ptr(), cb.subj.len(), 0, 0, mc, &mut wc, &mut wr,
                    );
                    check(
                        &mut d,
                        &format!("rows {:?} dfa /{}/ callout={v}", cb.rows, cb.pat),
                        &c,
                        &r,
                        v,
                    );
                }
                // non-negative values do not abandon the match
                for v in [0, 1] {
                    *ptr::addr_of_mut!(CALLOUT_RET) = v;
                    let (mut wc, mut wr) = (ws(1000), ws(1000));
                    let (c, r) = dfa2(
                        p, code, md, cb.subj.as_ptr(), cb.subj.len(), 0, 0, mc, &mut wc, &mut wr,
                    );
                    d.eq(
                        &format!("rows {:?} dfa /{}/ callout={v}: C vs rust", cb.rows, cb.pat),
                        c.clone(),
                        r,
                    );
                    d.eq(
                        &format!("rows {:?} dfa /{}/ callout={v} is not an error", cb.rows, cb.pat),
                        c.rc >= PCRE2_ERROR_NOMATCH,
                        true,
                    );
                }
                free_md2(p, md);
                free_code2(p, code);
            }
            *ptr::addr_of_mut!(CALLOUT_RET) = 0;
            free_mcontext2(p, mc);
        }
    }
    d.finish("rows 313-321: pcre2_dfa_match_8 partial/nomatch/overflow/callout outcomes");
}

// ===================================================================== 323

#[test]
fn r323_dfa_ufunc() {
    struct FCase {
        rows: &'static [u32],
        which: &'static str,
        expect: c_int,
    }
    // ERRORS.md names the *_bynumber_8 functions, but pcre2_substring.c:75,
    // 163 and 270 are the *_byname_8 ones (see the report); the bynumber
    // functions have no matchedby check and report PCRE2_ERROR_UNSET instead.
    const F: &[FCase] = &[
        FCase { rows: &[323], which: "substring_length_byname", expect: PCRE2_ERROR_DFA_UFUNC },
        FCase { rows: &[323], which: "substring_copy_byname", expect: PCRE2_ERROR_DFA_UFUNC },
        FCase { rows: &[323], which: "substring_get_byname", expect: PCRE2_ERROR_DFA_UFUNC },
        FCase { rows: &[323], which: "substitute (SUBSTITUTE_MATCHED)", expect: PCRE2_ERROR_DFA_UFUNC },
    ];

    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let code = compile2(p, b"(?<n>a)(b)", 0, NONE2);
        let md = md_from_pattern2(p, code);
        let (mut wc, mut wr) = (ws(1000), ws(1000));
        let (c, r) = dfa2(p, code, md, b"ab".as_ptr(), 2, 0, 0, NONE2, &mut wc, &mut wr);
        d.eq("row323 dfa fixture", c.clone(), r);
        d.eq("row323 dfa fixture matched", c.rc, 1);

        let name = b"n\0";
        // length_byname
        let (mut lc, mut lr) = (0usize, 0usize);
        let a = (p.c.substring_length_byname)(md.c, name.as_ptr(), &mut lc);
        let b = (p.r.substring_length_byname)(md.r, name.as_ptr(), &mut lr);
        d.eq("row323 substring_length_byname", a, b);
        d.eq("row323 substring_length_byname vs ERRORS.md", a, PCRE2_ERROR_DFA_UFUNC);
        // copy_byname
        let (mut bufc, mut bufr) = ([0u8; 32], [0u8; 32]);
        let (mut sc, mut sr) = (32usize, 32usize);
        let a = (p.c.substring_copy_byname)(md.c, name.as_ptr(), bufc.as_mut_ptr(), &mut sc);
        let b = (p.r.substring_copy_byname)(md.r, name.as_ptr(), bufr.as_mut_ptr(), &mut sr);
        d.eq("row323 substring_copy_byname", a, b);
        d.eq("row323 substring_copy_byname vs ERRORS.md", a, PCRE2_ERROR_DFA_UFUNC);
        // get_byname
        let (mut pc, mut pr) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
        let a = (p.c.substring_get_byname)(md.c, name.as_ptr(), &mut pc, &mut lc);
        let b = (p.r.substring_get_byname)(md.r, name.as_ptr(), &mut pr, &mut lr);
        d.eq("row323 substring_get_byname", a, b);
        d.eq("row323 substring_get_byname vs ERRORS.md", a, PCRE2_ERROR_DFA_UFUNC);
        if !pc.is_null() {
            (p.c.substring_free)(pc);
        }
        if !pr.is_null() {
            (p.r.substring_free)(pr);
        }
        // substitute with a DFA-produced match data
        let (mut oc, mut or) = ([0u8; 64], [0u8; 64]);
        let (mut olc, mut olr) = (64usize, 64usize);
        let a = (p.c.substitute)(
            code.c, b"ab".as_ptr(), 2, 0, PCRE2_SUBSTITUTE_MATCHED, md.c, ptr::null_mut(),
            b"X".as_ptr(), 1, oc.as_mut_ptr(), &mut olc,
        );
        let b = (p.r.substitute)(
            code.r, b"ab".as_ptr(), 2, 0, PCRE2_SUBSTITUTE_MATCHED, md.r, ptr::null_mut(),
            b"X".as_ptr(), 1, or.as_mut_ptr(), &mut olr,
        );
        d.eq("row323 substitute(SUBSTITUTE_MATCHED)", a, b);
        d.eq("row323 substitute vs ERRORS.md", a, PCRE2_ERROR_DFA_UFUNC);

        // the *_bynumber_8 functions have no matchedby check: they report
        // PCRE2_ERROR_UNSET for a DFA match data (still identical in both).
        let a = (p.c.substring_length_bynumber)(md.c, 1, &mut lc);
        let b = (p.r.substring_length_bynumber)(md.r, 1, &mut lr);
        d.eq("row323 substring_length_bynumber", a, b);
        d.eq("row323 substring_length_bynumber is UNSET not UFUNC", a, PCRE2_ERROR_UNSET);
        let (mut sc, mut sr) = (32usize, 32usize);
        let a = (p.c.substring_copy_bynumber)(md.c, 1, bufc.as_mut_ptr(), &mut sc);
        let b = (p.r.substring_copy_bynumber)(md.r, 1, bufr.as_mut_ptr(), &mut sr);
        d.eq("row323 substring_copy_bynumber", a, b);
        let a = (p.c.substring_get_bynumber)(md.c, 1, &mut pc, &mut lc);
        let b = (p.r.substring_get_bynumber)(md.r, 1, &mut pr, &mut lr);
        d.eq("row323 substring_get_bynumber", a, b);
        if !pc.is_null() {
            (p.c.substring_free)(pc);
        }
        if !pr.is_null() {
            (p.r.substring_free)(pr);
        }

        // an interpreter-produced match data must NOT get -41
        let (c, r) = match2(p, code, md, b"ab".as_ptr(), 2, 0, 0, NONE2);
        d.eq("row323 interpreter fixture", c.clone(), r);
        let a = (p.c.substring_length_byname)(md.c, name.as_ptr(), &mut lc);
        let b = (p.r.substring_length_byname)(md.r, name.as_ptr(), &mut lr);
        d.eq("row323 interpreter substring_length_byname", (a, lc), (b, lr));
        d.eq("row323 interpreter substring_length_byname is not -41", a != PCRE2_ERROR_DFA_UFUNC, true);

        for f in F {
            assert!(!f.rows.is_empty() && f.expect == PCRE2_ERROR_DFA_UFUNC, "{}", f.which);
        }
        free_md2(p, md);
        free_code2(p, code);
    }
    d.finish("row 323: PCRE2_ERROR_DFA_UFUNC from the by-name substring functions and pcre2_substitute_8");
}
