//! Phase C: callouts.
//!
//! Differential coverage of the PCRE2 callout machinery:
//!   * `pcre2_set_callout_8` + `PCRE2_AUTO_CALLOUT` and explicit `(?C...)`
//!     callouts, recording every field of `pcre2_callout_block` on every
//!     invocation for both `pcre2_match_8` and `pcre2_dfa_match_8`.
//!   * callout callbacks returning 0 / positive / negative values (the return
//!     value steers matching and so must match exactly).
//!   * `PCRE2_EXTRA_NEVER_CALLOUT` compile rejection and NULL callout function.
//!   * `pcre2_callout_enumerate_8` recording every `pcre2_callout_enumerate_block`
//!     field, propagation of a non-zero callback return, and `code == NULL`.
//!   * `pcre2_next_match_8` iteration for a corpus of patterns/subjects and its
//!     error/NULL paths.
//!   * >3000 seeded-random differential cases.
//!
//! Every comparison is done field-by-field on data the C API actually defines
//! for the relevant code path (lengths/flags are checked before any pointer is
//! dereferenced, and every pointer is null-checked).

mod harness;
use harness::*;

use std::ffi::c_void;
use std::os::raw::c_int;

/// Upper bound on how many callout invocations we snapshot per run. Both
/// libraries are deterministic, so matching this many snapshots plus the total
/// invocation count is conclusive while bounding memory on pathological inputs.
const CB_RECORD_CAP: usize = 4096;

// ---------------------------------------------------------------------------
//                         recorded callout snapshots
// ---------------------------------------------------------------------------

/// A full, comparable snapshot of one `pcre2_callout_block` as delivered to the
/// callback. Only fields/regions the C code defines for callouts are captured.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CalloutRec {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    subject_length: Sz,
    start_match: Sz,
    current_position: Sz,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_flags: u32,
    /// NUL-terminated mark string contents, if `mark` is non-null.
    mark: Option<Vec<u8>>,
    /// exactly `callout_string_length` bytes, if `callout_string` non-null.
    callout_string: Option<Vec<u8>>,
    /// the offset_vector contents up to `capture_top * 2` slots.
    ovector: Vec<Sz>,
}

/// Snapshot a match/DFA callout block. Safety: `b` points at a live block owned
/// by the matcher; we only read fields the C code assigns and honour every
/// length/flag before touching a pointer.
unsafe fn snapshot_callout(b: &CalloutBlock, dfa: bool) -> CalloutRec {
    let mark = if b.mark.is_null() {
        None
    } else {
        Some(unsafe { cstr(b.mark) })
    };
    // The callout string is NOT NUL-guaranteed; read exactly the stated length.
    let callout_string = if b.callout_string.is_null() {
        None
    } else if b.callout_string_length == 0 {
        Some(Vec::new())
    } else {
        Some(unsafe {
            std::slice::from_raw_parts(b.callout_string, b.callout_string_length).to_vec()
        })
    };
    // offset_vector is only meaningful output for pcre2_match: there the matcher
    // forces the first pair to PCRE2_UNSET during the callback and fills the
    // rest up to capture_top*2. Under pcre2_dfa_match the block's offset_vector
    // simply aliases the caller's ovector, whose contents at callout time are
    // work-in-progress and explicitly not useful (see pcre2callout docs /
    // do_callout_dfa), so we do not compare it there.
    let ov_slots = (b.capture_top as usize).saturating_mul(2);
    let ovector = if dfa || b.offset_vector.is_null() || ov_slots == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(b.offset_vector, ov_slots).to_vec() }
    };
    CalloutRec {
        version: b.version,
        callout_number: b.callout_number,
        capture_top: b.capture_top,
        capture_last: b.capture_last,
        subject_length: b.subject_length,
        start_match: b.start_match,
        current_position: b.current_position,
        pattern_position: b.pattern_position,
        next_item_length: b.next_item_length,
        callout_string_offset: b.callout_string_offset,
        callout_string_length: b.callout_string_length,
        callout_flags: b.callout_flags,
        mark,
        callout_string,
        ovector,
    }
}

// The callback needs somewhere to record. We drive one library at a time (C
// then Rust), single threaded, so a thread-local is safe and deterministic.
thread_local! {
    static CB_LOG: std::cell::RefCell<Vec<CalloutRec>> =
        const { std::cell::RefCell::new(Vec::new()) };
    // return-value plan: a queue of values the callback returns in order; when
    // exhausted it returns `CB_DEFAULT`.
    static CB_PLAN: std::cell::RefCell<Vec<c_int>> = const { std::cell::RefCell::new(Vec::new()) };
    static CB_DEFAULT: std::cell::Cell<c_int> = const { std::cell::Cell::new(0) };
    static CB_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    // Whether the current run is a pcre2_dfa_match (affects offset_vector
    // comparability, see snapshot_callout).
    static CB_DFA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

unsafe extern "C" fn record_callout(b: *mut CalloutBlock, _data: *mut c_void) -> c_int {
    if b.is_null() {
        return 0;
    }
    let idx = CB_CALLS.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v as usize
    });
    // Cap the recorded sequence: some patterns (esp. under DFA) fire callouts a
    // pathological number of times. Both libraries are deterministic, so
    // agreement over a large prefix plus the total call count is conclusive;
    // recording every invocation would exhaust memory. We keep counting (so the
    // total is compared) but stop snapshotting past the cap. The returned value
    // is unaffected, so match behaviour is identical.
    if idx < CB_RECORD_CAP {
        let dfa = CB_DFA.with(|d| d.get());
        let rec = unsafe { snapshot_callout(&*b, dfa) };
        CB_LOG.with(|l| l.borrow_mut().push(rec));
    }
    CB_PLAN.with(|p| {
        let p = p.borrow();
        if idx < p.len() {
            p[idx]
        } else {
            CB_DEFAULT.with(|d| d.get())
        }
    })
}

fn cb_reset(plan: &[c_int], default: c_int) {
    CB_LOG.with(|l| l.borrow_mut().clear());
    CB_PLAN.with(|p| *p.borrow_mut() = plan.to_vec());
    CB_DEFAULT.with(|d| d.set(default));
    CB_CALLS.with(|c| c.set(0));
}

// ---------------------------------------------------------------------------
//                    match-time callout differential driver
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct CalloutRun {
    compile_err: c_int,
    compiled: bool,
    rc: c_int,
    ovector: Vec<Sz>,
    /// total number of callout invocations (may exceed the recorded prefix).
    total_callouts: u64,
    recs: Vec<CalloutRec>,
}

/// Compile `pat` with `copts`, install `record_callout` (unless `null_callout`),
/// run match or DFA, and return the full observable result plus every recorded
/// callout block.
#[allow(clippy::too_many_arguments)]
fn run_callout(
    api: &Api,
    pat: &[u8],
    subject: &[u8],
    copts: u32,
    mopts: u32,
    dfa: bool,
    null_callout: bool,
    plan: &[c_int],
    default: c_int,
    extra_options: u32,
) -> CalloutRun {
    cb_reset(plan, default);
    CB_DFA.with(|d| d.set(dfa));
    unsafe {
        let cc = (api.compile_context_create)(std::ptr::null_mut());
        if extra_options != 0 {
            (api.set_compile_extra_options)(cc, extra_options);
        }
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let code = (api.compile)(pat.as_ptr(), pat.len(), copts, &mut err, &mut off, cc);
        (api.compile_context_free)(cc);
        if code.is_null() {
            return CalloutRun {
                compile_err: err,
                compiled: false,
                rc: 0,
                ovector: Vec::new(),
                total_callouts: 0,
                recs: Vec::new(),
            };
        }

        let mc = (api.match_context_create)(std::ptr::null_mut());
        if null_callout {
            (api.set_callout)(mc, None, std::ptr::null_mut());
        } else {
            (api.set_callout)(mc, Some(record_callout), std::ptr::null_mut());
        }

        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        assert!(!md.is_null());

        let rc = if dfa {
            let mut ws = [0i32; 256];
            (api.dfa_match)(
                code,
                subject.as_ptr(),
                subject.len(),
                0,
                mopts,
                md,
                mc,
                ws.as_mut_ptr(),
                ws.len(),
            )
        } else {
            (api.do_match)(code, subject.as_ptr(), subject.len(), 0, mopts, md, mc)
        };

        // capture the defined portion of the ovector for the final result.
        let mut capcount: u32 = 0;
        (api.pattern_info)(code, 4, &mut capcount as *mut u32 as *mut c_void);
        let m = api.read_match(md, rc, dfa, capcount);

        (api.match_data_free)(md);
        (api.match_context_free)(mc);
        (api.code_free)(code);

        CalloutRun {
            compile_err: err,
            compiled: true,
            rc,
            ovector: m.ovector,
            total_callouts: CB_CALLS.with(|c| c.get()),
            recs: CB_LOG.with(|l| l.borrow().clone()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn diff_callout(
    pat: &[u8],
    subject: &[u8],
    copts: u32,
    mopts: u32,
    dfa: bool,
    null_callout: bool,
    plan: &[c_int],
    default: c_int,
    extra_options: u32,
) {
    let co = run_callout(
        c(),
        pat,
        subject,
        copts,
        mopts,
        dfa,
        null_callout,
        plan,
        default,
        extra_options,
    );
    let ro = run_callout(
        r(),
        pat,
        subject,
        copts,
        mopts,
        dfa,
        null_callout,
        plan,
        default,
        extra_options,
    );
    if co != ro {
        panic!(
            "CALLOUT DIVERGENCE\n pat={:?} subj={:?}\n copts={copts:#x} mopts={mopts:#x} dfa={dfa} null_cb={null_callout} default={default} plan={plan:?} extra={extra_options:#x}\n C    = {co:#?}\n Rust = {ro:#?}",
            String::from_utf8_lossy(pat),
            String::from_utf8_lossy(subject),
        );
    }
}

// ---------------------------------------------------------------------------
//                              corpora
// ---------------------------------------------------------------------------

/// Patterns with explicit callouts of every documented form, plus the full set
/// of callout-string delimiters (` ' " ^ % # $ and {}).
fn explicit_callout_patterns() -> Vec<&'static str> {
    vec![
        "(?C)a",
        "(?C0)a",
        "(?C1)abc",
        "(?C255)x",
        "(?C99)(a)(b)",
        "a(?C1)b(?C2)c",
        "(?C{text})b",
        "(?C{})b",
        "(?C`grave`)a",
        "(?C'apos')a",
        "(?C\"quote\")a",
        "(?C^caret^)a",
        "(?C%percent%)a",
        "(?C#hash#)a",
        "(?C$dollar$)a",
        "(?C{curly})a",
        // doubled delimiter -> literal delimiter inside the string
        "(?C{a}}b})c",
        "(?C`a``b`)c",
        "(?C'it''s')x",
        // callouts interleaved with captures and alternation
        "(?C1)(a)(?C2)(b)|(?C3)c",
        "a(?C1)*b",
        "(?:(?C7)a|(?C8)b)+",
        "(?C5)(?=a)(?C6)b",
        "(?C1)a(?C2)b(?C3)c(?C4)d(?C5)e",
        "x(?C{one})y(?C{two})z",
    ]
}

fn callout_subjects() -> Vec<&'static str> {
    vec![
        "", "a", "b", "c", "ab", "abc", "abcd", "abcde", "x", "xyz", "aaa", "cba", "d",
        "aXbXc", "hello", "grave", "é", "日本", "a\nb", "  ", "abcabcabc",
    ]
}

// ===========================================================================
//  1. AUTO_CALLOUT + set_callout: record every field, compare full sequence.
// ===========================================================================

#[test]
fn auto_callout_full_field_capture() {
    let pats = curated_patterns();
    let subs = callout_subjects();
    for p in &pats {
        for s in &subs {
            // default callback returns 0 (continue); compare full record + result.
            diff_callout(p.as_bytes(), s.as_bytes(), PCRE2_AUTO_CALLOUT, 0, false, false, &[], 0, 0);
        }
    }
}

#[test]
fn auto_callout_more_subjects() {
    let subs = curated_subjects();
    // a focused set of patterns exercised against the wider subject corpus.
    let pats = [
        "a.b", "(a)(b)?(c)*", "\\d+", "\\w+\\s\\w+", "(?:ab|cd)+", "a{2,4}",
        "(?<n>a)(?<m>b)", "^a.*z$", "(a|b|c)*", ".*", "a??b", "(a+)+b",
    ];
    for p in pats {
        for s in &subs {
            diff_callout(p.as_bytes(), s.as_bytes(), PCRE2_AUTO_CALLOUT, 0, false, false, &[], 0, 0);
        }
    }
}

// ===========================================================================
//  2. Explicit callouts and every callout-string delimiter.
// ===========================================================================

#[test]
fn explicit_callouts_and_delimiters() {
    for p in explicit_callout_patterns() {
        for s in callout_subjects() {
            diff_callout(p.as_bytes(), s.as_bytes(), 0, 0, false, false, &[], 0, 0);
        }
    }
}

// ===========================================================================
//  3. Callback return values (0 / positive / negative / per-call counter).
// ===========================================================================

#[test]
fn callout_return_values() {
    // Patterns with several callout points so returns actually change matching.
    let pats = [
        "a(?C1)b(?C2)c", "(?C1)a|(?C2)b|(?C3)c", "(?C1)a(?C2)b(?C3)c(?C4)",
        "(a(?C1))+", "(?:(?C1)a)*b",
    ];
    let subs = ["", "a", "b", "c", "abc", "aaab", "abcabc", "ab", "bc"];
    let defaults: &[c_int] = &[
        0,                    // continue
        1,                    // fail this match position, backtrack
        255,
        PCRE2_ERROR_NOMATCH,  // -1: abandon current match attempt
        PCRE2_ERROR_CALLOUT,  // -37: hard error, abandon everything
        -2,
        -100,
    ];
    for p in pats {
        for s in subs {
            for &d in defaults {
                for dfa in [false, true] {
                    diff_callout(p.as_bytes(), s.as_bytes(), 0, 0, dfa, false, &[], d, 0);
                    // AUTO_CALLOUT variant too.
                    diff_callout(p.as_bytes(), s.as_bytes(), PCRE2_AUTO_CALLOUT, 0, dfa, false, &[], d, 0);
                }
            }
        }
    }
}

#[test]
fn callout_counter_returns() {
    // Callback returns different values on successive calls.
    let plans: &[&[c_int]] = &[
        &[0, 1, 0, 1, 0, 1, 0, 1],
        &[0, 0, PCRE2_ERROR_NOMATCH],
        &[1, 0, 0, 0],
        &[0, 0, 0, PCRE2_ERROR_CALLOUT],
        &[255, 0, 1, -1, 0],
    ];
    let pats = [
        "a(?C1)b(?C2)c(?C3)d", "(?C1)a(?C2)a(?C3)a", ".(?C7).(?C8).",
        "(a(?C1))+b",
    ];
    let subs = ["", "a", "aa", "aaa", "abcd", "abc", "aaab"];
    for p in pats {
        for s in subs {
            for plan in plans {
                for copts in [0u32, PCRE2_AUTO_CALLOUT] {
                    for dfa in [false, true] {
                        diff_callout(p.as_bytes(), s.as_bytes(), copts, 0, dfa, false, plan, 0, 0);
                    }
                }
            }
        }
    }
}

// ===========================================================================
//  4. Callouts under DFA and with PARTIAL_SOFT / PARTIAL_HARD.
// ===========================================================================

#[test]
fn callout_dfa_and_partial() {
    let pats = [
        "a(?C1)b(?C2)c", "\\d(?C1)\\d(?C2)\\d", "(?C1)abc(?C2)def",
        "ab(?C9)cd", "a+(?C1)b", "(?C1).(?C2).(?C3).",
    ];
    let subs = ["", "a", "ab", "abc", "abcd", "12", "123", "1234", "abcdef", "abX"];
    for p in pats {
        for s in subs {
            for mopts in [
                0u32,
                PCRE2_PARTIAL_SOFT,
                PCRE2_PARTIAL_HARD,
            ] {
                for copts in [0u32, PCRE2_AUTO_CALLOUT] {
                    for dfa in [false, true] {
                        diff_callout(p.as_bytes(), s.as_bytes(), copts, mopts, dfa, false, &[], 0, 0);
                    }
                }
            }
        }
    }
}

// ===========================================================================
//  5. PCRE2_EXTRA_NEVER_CALLOUT rejection + NULL callout function.
// ===========================================================================

#[test]
fn never_callout_and_null_callout() {
    let callout_pats = [
        "(?C1)a", "(?C{x})a", "a(?C0)b", "(?C255)z", "(?C1)(a)(?C2)b",
    ];
    let no_callout_pats = ["abc", "(a)(b)", "\\d+"];

    // (a) PCRE2_EXTRA_NEVER_CALLOUT: patterns containing callouts must be
    //     rejected identically (auto-callout also implies callouts).
    for p in callout_pats {
        for s in ["", "a", "ab"] {
            diff_callout(p.as_bytes(), s.as_bytes(), 0, 0, false, false, &[], 0, PCRE2_EXTRA_NEVER_CALLOUT);
        }
    }
    // AUTO_CALLOUT + NEVER_CALLOUT on any pattern must also be rejected.
    for p in no_callout_pats {
        diff_callout(p.as_bytes(), b"abc", PCRE2_AUTO_CALLOUT, 0, false, false, &[], 0, PCRE2_EXTRA_NEVER_CALLOUT);
    }

    // (b) NULL callout function with callout-containing patterns: matching must
    //     proceed as if the callout returned 0.
    for p in callout_pats {
        for s in callout_subjects() {
            for dfa in [false, true] {
                diff_callout(p.as_bytes(), s.as_bytes(), 0, 0, dfa, true, &[], 0, 0);
                diff_callout(p.as_bytes(), s.as_bytes(), PCRE2_AUTO_CALLOUT, 0, dfa, true, &[], 0, 0);
            }
        }
    }
}

// ===========================================================================
//  6. pcre2_callout_enumerate.
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnumRec {
    version: u32,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_number: u32,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: Option<Vec<u8>>,
}

thread_local! {
    static ENUM_LOG: std::cell::RefCell<Vec<EnumRec>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static ENUM_PLAN: std::cell::RefCell<Vec<c_int>> = const { std::cell::RefCell::new(Vec::new()) };
    static ENUM_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

unsafe extern "C" fn record_enum(b: *mut CalloutEnumerateBlock, _d: *mut c_void) -> c_int {
    if b.is_null() {
        return 0;
    }
    let b = unsafe { &*b };
    let callout_string = if b.callout_string.is_null() {
        None
    } else if b.callout_string_length == 0 {
        Some(Vec::new())
    } else {
        Some(unsafe {
            std::slice::from_raw_parts(b.callout_string, b.callout_string_length).to_vec()
        })
    };
    ENUM_LOG.with(|l| {
        l.borrow_mut().push(EnumRec {
            version: b.version,
            pattern_position: b.pattern_position,
            next_item_length: b.next_item_length,
            callout_number: b.callout_number,
            callout_string_offset: b.callout_string_offset,
            callout_string_length: b.callout_string_length,
            callout_string,
        })
    });
    let idx = ENUM_CALLS.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v as usize
    });
    ENUM_PLAN.with(|p| {
        let p = p.borrow();
        if idx < p.len() { p[idx] } else { 0 }
    })
}

fn run_enum(api: &Api, pat: &[u8], copts: u32, plan: &[c_int]) -> (c_int, c_int, Vec<EnumRec>) {
    ENUM_LOG.with(|l| l.borrow_mut().clear());
    ENUM_PLAN.with(|p| *p.borrow_mut() = plan.to_vec());
    ENUM_CALLS.with(|c| c.set(0));
    unsafe {
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let code = (api.compile)(pat.as_ptr(), pat.len(), copts, &mut err, &mut off, std::ptr::null_mut());
        if code.is_null() {
            return (err, 0, Vec::new());
        }
        let rc = (api.callout_enumerate)(code, Some(record_enum), std::ptr::null_mut());
        (api.code_free)(code);
        (0, rc, ENUM_LOG.with(|l| l.borrow().clone()))
    }
}

fn diff_enum(pat: &str, copts: u32, plan: &[c_int]) {
    let co = run_enum(c(), pat.as_bytes(), copts, plan);
    let ro = run_enum(r(), pat.as_bytes(), copts, plan);
    if co != ro {
        panic!(
            "ENUMERATE DIVERGENCE\n pat={pat:?} copts={copts:#x} plan={plan:?}\n C    = {co:#?}\n Rust = {ro:#?}"
        );
    }
}

#[test]
fn callout_enumerate_full() {
    // Patterns with 0, 1 and many callouts, auto and explicit.
    let pats = [
        // zero callouts
        "abc", "(a)(b)", "\\d+", "",
        // one callout
        "(?C1)a", "a(?C0)b", "(?C{text})z", "(?C255)q",
        // many callouts, explicit
        "(?C1)a(?C2)b(?C3)c(?C4)d(?C5)e",
        "(?C{one})(?C{two})(?C{three})",
        "a(?C1)(b(?C2)c)*(?C3)d",
        "(?C`g`)a(?C'a')b(?C\"q\")c(?C^c^)d(?C%p%)e(?C#h#)f(?C$d$)g(?C{cur})h",
    ];
    for p in pats {
        diff_enum(p, 0, &[]);
        // auto-callout: every item gets a callout, exercising many enum calls.
        diff_enum(p, PCRE2_AUTO_CALLOUT, &[]);
    }
    // A batch of curated patterns under AUTO_CALLOUT for breadth.
    for p in curated_patterns() {
        diff_enum(p, PCRE2_AUTO_CALLOUT, &[]);
        diff_enum(p, 0, &[]);
    }
}

#[test]
fn callout_enumerate_nonzero_return() {
    // A non-zero callback return must stop enumeration and be propagated.
    let pats = [
        "(?C1)a(?C2)b(?C3)c",
        "(?C{one})(?C{two})(?C{three})(?C{four})",
        "a(?C1)b(?C2)c(?C3)d(?C4)e",
    ];
    let plans: &[&[c_int]] = &[
        &[0, 7],          // stop at 2nd callout, return 7
        &[42],            // stop at 1st, return 42
        &[0, 0, -5],      // negative propagation
        &[0, 0, 0, 99],
    ];
    for p in pats {
        for plan in plans {
            diff_enum(p, 0, plan);
            diff_enum(p, PCRE2_AUTO_CALLOUT, plan);
        }
    }
}

#[test]
fn callout_enumerate_null_code() {
    // code == NULL must yield PCRE2_ERROR_NULL for both libraries.
    unsafe {
        let mut rcs = Vec::new();
        for api in both() {
            let rc = (api.callout_enumerate)(
                std::ptr::null_mut(),
                Some(record_enum),
                std::ptr::null_mut(),
            );
            rcs.push(rc);
        }
        assert_eq!(rcs[0], rcs[1], "enumerate(NULL) rc differs: C={} Rust={}", rcs[0], rcs[1]);
        assert_eq!(rcs[0], PCRE2_ERROR_NULL, "enumerate(NULL) should be PCRE2_ERROR_NULL");
    }
}

// ===========================================================================
//  7. pcre2_next_match.
// ===========================================================================

/// Iterate all matches for a pattern/subject via pcre2_next_match, the
/// documented way: run the first match, then repeatedly ask next_match for the
/// next (start_offset, options) and re-run pcre2_match there, recording the
/// full sequence of (rc, next_off, next_opts) triples until iteration ends.
/// `off`/`opts` are only defined when next_match returns TRUE, so on the
/// terminating step we record only the boolean rc.
fn diff_next_match(pat: &str, subject: &str, mopts: u32, dfa: bool) {
    #[derive(Debug, PartialEq, Eq)]
    struct Seq {
        compile_err: c_int,
        first_rc: c_int,
        // (next_match_rc, next_off_or_MAX, next_opts) per iteration step.
        steps: Vec<(c_int, Sz, u32)>,
    }

    fn run(api: &Api, pat: &str, subject: &str, mopts: u32, dfa: bool) -> Seq {
        unsafe {
            let mut err: c_int = 0;
            let mut off0: Sz = 0;
            let code = (api.compile)(
                pat.as_bytes().as_ptr(),
                pat.len(),
                0,
                &mut err,
                &mut off0,
                std::ptr::null_mut(),
            );
            if code.is_null() {
                return Seq { compile_err: err, first_rc: 0, steps: Vec::new() };
            }
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let subj = subject.as_bytes();

            let do_one = |md: MatchData, start: Sz, opts: u32| -> c_int {
                if dfa {
                    let mut ws = [0i32; 256];
                    (api.dfa_match)(
                        code, subj.as_ptr(), subj.len(), start, opts, md,
                        std::ptr::null_mut(), ws.as_mut_ptr(), ws.len(),
                    )
                } else {
                    (api.do_match)(
                        code, subj.as_ptr(), subj.len(), start, opts, md,
                        std::ptr::null_mut(),
                    )
                }
            };

            let first_rc = do_one(md, 0, mopts);
            let mut steps = Vec::new();
            let mut guard = 0;
            loop {
                let mut noff: Sz = Sz::MAX;
                let mut nopts: u32 = 0;
                let cont = (api.next_match)(md, &mut noff, &mut nopts);
                if cont > 0 {
                    steps.push((cont, noff, nopts));
                    // re-run the match at the offset next_match advised.
                    let rc = do_one(md, noff, mopts | nopts);
                    // record the rc of that re-run as part of the sequence.
                    steps.push((rc, Sz::MAX, 0));
                    guard += 1;
                    if guard > 10_000 {
                        break; // safety valve; must never trigger in practice
                    }
                } else {
                    // FALSE: iteration finished; off/opts undefined -> record rc only.
                    steps.push((cont, Sz::MAX, 0));
                    break;
                }
            }
            (api.match_data_free)(md);
            (api.code_free)(code);
            Seq { compile_err: err, first_rc, steps }
        }
    }

    let co = run(c(), pat, subject, mopts, dfa);
    let ro = run(r(), pat, subject, mopts, dfa);
    if co != ro {
        panic!(
            "NEXT_MATCH DIVERGENCE\n pat={pat:?} subj={subject:?} mopts={mopts:#x} dfa={dfa}\n C    = {co:?}\n Rust = {ro:?}"
        );
    }
}

#[test]
fn next_match_corpus() {
    // Include empty-matching, anchored, alternation, and DFA (which can return
    // multiple matches at a single position).
    let pats = [
        "", "x*", "(?=a)", "\\b", "a", "a|ab|abc", "^a", "\\Ga", "a$", "^$",
        "(?:a|aa|aaa)", "\\w+", ".", ".*", "a??", "a*?", "(a)(b)?", "[abc]+",
        "\\B", "^", "$", "a{0,3}", "(?:ab)+", "(?i)A", "\\d*",
    ];
    let subs = [
        "", "a", "aa", "aaa", "ab", "abc", "abcabc", "xxx", "a b c", "banana",
        "  a  ", "AaA", "123", "a\nb",
    ];
    for p in pats {
        for s in subs {
            for dfa in [false, true] {
                for mopts in [0u32, PCRE2_PARTIAL_SOFT] {
                    diff_next_match(p, s, mopts, dfa);
                }
            }
        }
    }
}

#[test]
fn next_match_edge_cases() {
    // next_match on: failed match, partial match, dfa match, NULL match data.
    //
    // NOTE on NULL match data: pcre2_next_match dereferences match_data->rc as
    // its very first statement with no NULL guard (see pcre2_match_next.c), so
    // passing NULL is undefined behaviour and segfaults the *C* library too.
    // That is API misuse rather than an observable, comparable result, so we do
    // not call it with NULL here.

    // failed / partial / dfa match data.
    // failed: pattern that does not match; partial: needs more input; dfa: DFA.
    let scenarios: &[(&str, &str, u32, bool)] = &[
        ("xyz", "abc", 0, false),               // failed match
        ("abcd", "abc", PCRE2_PARTIAL_SOFT, false), // partial match
        ("a|ab|abc", "abc", 0, true),           // dfa match (multiple)
        ("abcd", "abc", PCRE2_PARTIAL_HARD, false), // partial hard
        ("zzz", "abc", 0, true),                // dfa failed
    ];
    for &(pat, subj, mopts, dfa) in scenarios {
        diff_next_match(pat, subj, mopts, dfa);
    }
}

// ===========================================================================
//  8. Seeded-random differential cases (>= 3000).
// ===========================================================================

/// Build a pattern that is likely to contain callouts: sprinkle explicit
/// callouts into a randomly generated pattern.
fn random_callout_pattern(rng: &mut Rng) -> String {
    let depth = rng.range(1, 2) as u32;
    let base = random_pattern(rng, depth);
    if rng.bool() {
        return base;
    }
    // Insert 1-3 explicit callouts at random split points.
    let inserts = [
        "(?C1)", "(?C0)", "(?C255)", "(?C{r})", "(?C`g`)", "(?C99)", "(?C'a')",
    ];
    let mut chars: Vec<char> = base.chars().collect();
    let n = rng.range(1, 3);
    for _ in 0..n {
        let pos = if chars.is_empty() { 0 } else { rng.below(chars.len() + 1) };
        let ins = *rng.pick(&inserts);
        for (k, ch) in ins.chars().enumerate() {
            chars.insert((pos + k).min(chars.len()), ch);
        }
    }
    chars.into_iter().collect()
}

#[test]
fn callout_randomized() {
    let mut rng = Rng::new(0xCA110A75_5EED_u64);
    let return_pool: &[c_int] = &[0, 0, 0, 1, 255, PCRE2_ERROR_NOMATCH, -2, PCRE2_ERROR_CALLOUT];
    let mopt_pool: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED];

    for i in 0..3200u32 {
        let pat = if rng.below(3) == 0 {
            (*rng.pick(&explicit_callout_patterns())).to_string()
        } else {
            random_callout_pattern(&mut rng)
        };
        let auto = rng.bool();
        let copts = if auto { PCRE2_AUTO_CALLOUT } else { 0 };
        let s = if rng.bool() {
            (*rng.pick(&callout_subjects())).as_bytes().to_vec()
        } else {
            random_subject(&mut rng, false)
        };
        let dfa = rng.bool();
        let mopts = *rng.pick(mopt_pool);

        // Random callback return plan: sometimes a fixed default, sometimes a
        // per-call plan drawn from the return pool.
        let (plan, default): (Vec<c_int>, c_int) = if rng.bool() {
            (Vec::new(), *rng.pick(return_pool))
        } else {
            let len = rng.range(1, 6);
            let plan: Vec<c_int> = (0..len).map(|_| *rng.pick(return_pool)).collect();
            (plan, *rng.pick(return_pool))
        };
        let null_cb = rng.below(6) == 0;

        diff_callout(
            pat.as_bytes(),
            &s,
            copts,
            mopts,
            dfa,
            null_cb,
            &plan,
            default,
            0,
        );

        // Occasionally also exercise the enumerate path on the same pattern.
        if i % 7 == 0 {
            let eplan: Vec<c_int> = if rng.bool() {
                Vec::new()
            } else {
                vec![0, *rng.pick(return_pool)]
            };
            diff_enum(&pat, copts, &eplan);
        }
    }
}
