// Phase C — the error surface of `pcre2_substitute.c`.
//
// Covers ERRORS.md rows 324..379 (the whole `### pcre2_substitute.c` section).
// Every case drives BOTH shared libraries through `pcre2_substitute_8` and
// compares
//   * the returned code (the same numeric value, not merely "both failed"),
//   * the value written back through `outlengthptr`,
//   * the ENTIRE output buffer, including the bytes past the region the call is
//     allowed to touch, so that a buffer overrun in either library is caught.
//
// In addition every case states the code ERRORS.md documents for its row, and
// asserts that the C really does return it (the C is ground truth: a mismatch
// means the row was mis-derived).
//
// `*outlengthptr` semantics under test (documented in ERRORS.md just above the
// row table, implemented at pcre2_substitute.c:779/1748-1764/1790-1791):
//   * early returns and the `EXIT`-only errors leave it at `PCRE2_UNSET`;
//   * the `PTREXIT` family (-35/-49/-54/-55/-57/-58/-59/-76) set it to the
//     offset within the *replacement* at which the problem was found;
//   * `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` sets it to the required buffer size;
//   * success sets it to the output length excluding the terminating NUL.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

/// `pcre2.h:430` — not re-exported by the harness because nothing else needs it.
const PCRE2_ERROR_INTERNAL_DUPMATCH: c_int = -65;

// ---------------------------------------------------------------- plumbing

/// Expected value of `*outlengthptr`.
#[derive(Copy, Clone, Debug, PartialEq)]
enum L {
    /// `PCRE2_UNSET` — the call failed before it could report an offset.
    Unset,
    /// An exact value.
    At(usize),
}

impl L {
    fn matches(self, v: usize) -> bool {
        match self {
            L::Unset => v == PCRE2_UNSET,
            L::At(n) => v == n,
        }
    }
    fn show(self) -> String {
        match self {
            L::Unset => "UNSET".into(),
            L::At(n) => n.to_string(),
        }
    }
}

fn show_len(v: usize) -> String {
    if v == PCRE2_UNSET {
        "UNSET".into()
    } else {
        v.to_string()
    }
}

/// One `pcre2_substitute_8` error-path case.
struct Case {
    /// ERRORS.md row numbers this case covers.
    rows: &'static [u32],
    pat: &'static str,
    copts: u32,
    /// `pcre2_set_compile_extra_options_8` bits (0 for most rows).
    xopts: u32,
    subj: &'static [u8],
    start: usize,
    rep: &'static [u8],
    sopts: u32,
    /// output buffer capacity handed to the call as `*outlengthptr`
    cap: usize,
    /// the code ERRORS.md documents for this row
    expect: i32,
    /// the `*outlengthptr` the C must write back
    exp_len: L,
    why: &'static str,
}

/// Result of one call, in comparable form.
#[derive(Debug, PartialEq, Eq)]
struct Out {
    rc: c_int,
    len: usize,
    buf: Vec<u8>,
}

/// Runs `pcre2_substitute_8` with a buffer of `cap` usable code units followed
/// by 16 guard bytes; the guard bytes are part of the compared value.
#[allow(clippy::too_many_arguments)]
unsafe fn call(
    api: &Api,
    code: Ptr,
    subj: Sptr,
    slen: Sz,
    start: Sz,
    sopts: u32,
    md: Ptr,
    mc: Ptr,
    rep: Sptr,
    rlen: Sz,
    cap: usize,
    null_buffer: bool,
) -> Out {
    let mut buf = vec![0xEEu8; cap + 16];
    let mut len = cap;
    let bp = if null_buffer {
        ptr::null_mut()
    } else {
        buf.as_mut_ptr()
    };
    let rc = (api.substitute)(code, subj, slen, start, sopts, md, mc, rep, rlen, bp, &mut len);
    Out { rc, len, buf }
}

/// Compiles the same pattern in both libraries, applying `xopts` through a
/// compile context. Panics if the two disagree about compilability.
unsafe fn compile_both(p: &Pair, pat: &str, copts: u32, xopts: u32) -> (Ptr, Ptr, Ptr, Ptr) {
    let pb = pat.as_bytes();
    let cca = (p.c.compile_context_create)(ptr::null_mut());
    let ccb = (p.r.compile_context_create)(ptr::null_mut());
    assert!(!cca.is_null() && !ccb.is_null());
    assert_eq!(
        (p.c.set_compile_extra_options)(cca, xopts),
        (p.r.set_compile_extra_options)(ccb, xopts),
    );
    let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
    let (mut eoa, mut eob) = (0usize, 0usize);
    let a = (p.c.compile)(pb.as_ptr(), pb.len(), copts, &mut eca, &mut eoa, cca);
    let b = (p.r.compile)(pb.as_ptr(), pb.len(), copts, &mut ecb, &mut eob, ccb);
    assert_eq!(
        (a.is_null(), eca, eoa),
        (b.is_null(), ecb, eob),
        "compile of {pat:?} (copts={copts:#x} xopts={xopts:#x}) differs"
    );
    assert!(!a.is_null(), "compile of {pat:?} failed: err {eca} at {eoa}");
    (a, b, cca, ccb)
}

unsafe fn free_both(p: &Pair, a: Ptr, b: Ptr, cca: Ptr, ccb: Ptr) {
    (p.c.code_free)(a);
    (p.r.code_free)(b);
    (p.c.compile_context_free)(cca);
    (p.r.compile_context_free)(ccb);
}

/// Copies `s` into a buffer with 16 trailing guard bytes. Some rows deliberately
/// feed truncated UTF-8 with `PCRE2_NO_UTF_CHECK`, where the C is allowed to
/// look one code unit beyond the nominal end; the guard makes what it finds
/// there defined and, crucially, the *same* for both libraries.
fn padded(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0x41u8; 16]);
    v
}

// =========================================================== coverage index
//
// Rows 324..379 that are exercised by a *dedicated* test rather than by one of
// the data tables above carry their annotation here, in the same literal
// `rows:` slice spelling the coverage script greps for. `coverage_is_exactly_324_to_379`
// below re-derives the union from this very file and checks it.

struct RowNote {
    rows: &'static [u32],
    test: &'static str,
    note: &'static str,
}

const ROW_NOTES: &[RowNote] = &[
    RowNote { rows: &[325], test: "substitute_null_arguments",
        note: "replacement == NULL with rlength != 0 -> PCRE2_ERROR_NULL" },
    RowNote { rows: &[326], test: "substitute_null_arguments",
        note: "subject == NULL with length != 0 -> PCRE2_ERROR_NULL" },
    RowNote { rows: &[329], test: "substitute_matched_consistency",
        note: "SUBSTITUTE_MATCHED with a pcre2_dfa_match_8 match_data -> -41" },
    RowNote { rows: &[334], test: "substitute_allocation_failure",
        note: "match_data == NULL and match_data_create_from_pattern fails -> -48" },
    RowNote { rows: &[335], test: "substitute_allocation_failure",
        note: "SUBSTITUTE_MATCHED and the internal match_data_create fails -> -48" },
    RowNote { rows: &[342], test: "substitute_unreachable_rows",
        note: "PCRE2_ERROR_INTERNAL_DUPMATCH (-65): unreachable internal invariant" },
    RowNote { rows: &[343], test: "substitute_unreachable_rows",
        note: "PCRE2_ERROR_TOOMANYREPLACE (-61): needs INT_MAX substitutions" },
    RowNote { rows: &[351], test: "substitute_dynamic_replacements",
        note: "more than 10 nested ${name:-...} fills ptrstack -> -35" },
    RowNote { rows: &[366], test: "substitute_unavailable_group",
        note: "$+ with match_data->oveccount < top_bracket+1 -> -54" },
    RowNote { rows: &[373], test: "substitute_overflow_length",
        note: "PCRE2_SUBSTITUTE_OVERFLOW_LENGTH reports the required length" },
    RowNote { rows: &[374], test: "substitute_case_callout_errors",
        note: "substitute_case_callout returns PCRE2_SIZE_MAX -> -69" },
    RowNote { rows: &[375], test: "substitute_case_callout_errors",
        note: "PCRE2_SIZE overflow accumulating extra_needed -> -70" },
    RowNote { rows: &[376], test: "substitute_callout_rejects",
        note: "substitute_callout returns non-zero: not an error" },
    RowNote { rows: &[377], test: "substitute_unreachable_rows",
        note: "PCRE2_ERROR_NOUNIQUESUBSTRING (-50) is never returned here" },
];

/// Self-check: the union of every `rows:` annotation in this file must be
/// exactly ERRORS.md rows 324..379 — no gaps, nothing out of scope.
#[test]
fn coverage_is_exactly_324_to_379() {
    let src = include_str!("phase_c_substitute.rs");
    let mut seen = std::collections::BTreeSet::new();
    let mut rest = src;
    // Spelled in two pieces so that this very line is not itself an annotation.
    let needle: &str = concat!("rows", ": &[");
    while let Some(i) = rest.find(needle) {
        rest = &rest[i + needle.len()..];
        let end = rest.find(']').expect("unterminated row annotation");
        for tok in rest[..end].split(',') {
            let t = tok.trim();
            if t.is_empty() {
                continue;
            }
            seen.insert(t.parse::<u32>().unwrap_or_else(|_| {
                panic!("non-numeric row annotation {t:?} — the coverage grep needs plain numbers")
            }));
        }
        rest = &rest[end..];
    }
    let want: std::collections::BTreeSet<u32> = (324u32..=379).collect();
    let missing: Vec<_> = want.difference(&seen).copied().collect();
    let extra: Vec<_> = seen.difference(&want).copied().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "row coverage is wrong: missing {missing:?}, out of scope {extra:?}"
    );
    for n in ROW_NOTES {
        assert!(
            src.contains(&format!("fn {}(", n.test)),
            "ROW_NOTES row {:?} points at a test `{}` that does not exist",
            n.rows, n.test
        );
        assert!(!n.note.is_empty());
    }
    println!("covered ERRORS.md rows: {:?}", seen.iter().copied().collect::<Vec<_>>());
}

// ================================================================ main table

const CASES: &[Case] = &[
    // ---------------------------------------------------------------- row 324
    Case { rows: &[324], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_SOFT, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PCRE2_PARTIAL_SOFT without PCRE2_SUBSTITUTE_REPLACEMENT_ONLY" },
    Case { rows: &[324], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_HARD, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PCRE2_PARTIAL_HARD without PCRE2_SUBSTITUTE_REPLACEMENT_ONLY" },
    Case { rows: &[324], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_GLOBAL, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PARTIAL_SOFT|GLOBAL still lacks REPLACEMENT_ONLY" },
    Case { rows: &[324], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PARTIAL_HARD|EXTENDED still lacks REPLACEMENT_ONLY" },
    // positive control: with REPLACEMENT_ONLY partial matching is accepted
    Case { rows: &[324], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: 1, exp_len: L::At(1),
        why: "control: PARTIAL_HARD|REPLACEMENT_ONLY is legal" },

    // ---------------------------------------------------------------- row 327
    // (`match_data` is NULL throughout this table.)
    Case { rows: &[327], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"X", sopts: PCRE2_SUBSTITUTE_MATCHED, cap: 64,
        expect: PCRE2_ERROR_NULL, exp_len: L::Unset,
        why: "PCRE2_SUBSTITUTE_MATCHED with match_data == NULL" },
    Case { rows: &[327], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"X", sopts: PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL, cap: 64,
        expect: PCRE2_ERROR_NULL, exp_len: L::Unset,
        why: "PCRE2_SUBSTITUTE_MATCHED|GLOBAL with match_data == NULL" },

    // ---------------------------------------------------------------- row 336
    // invalid UTF in the REPLACEMENT, checked (no PCRE2_NO_UTF_CHECK)
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\x80", sopts: 0, cap: 64,
        expect: -22, exp_len: L::Unset,
        why: "isolated 0x80 in replacement -> PCRE2_ERROR_UTF8_ERR20" },
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\xc3", sopts: 0, cap: 64,
        expect: -3, exp_len: L::Unset,
        why: "truncated 2-byte sequence in replacement -> UTF8_ERR1" },
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\xed\xa0\x80", sopts: 0, cap: 64,
        expect: -16, exp_len: L::Unset,
        why: "surrogate in replacement -> UTF8_ERR14" },
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\xf5\x80\x80\x80", sopts: 0, cap: 64,
        expect: -15, exp_len: L::Unset,
        why: "code point > 0x10ffff in replacement -> UTF8_ERR13" },
    // ... and the same replacements accepted verbatim with PCRE2_NO_UTF_CHECK
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\x80", sopts: PCRE2_NO_UTF_CHECK, cap: 64,
        expect: 1, exp_len: L::At(1),
        why: "NO_UTF_CHECK suppresses the replacement UTF check" },
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\xed\xa0\x80", sopts: PCRE2_NO_UTF_CHECK, cap: 64,
        expect: 1, exp_len: L::At(3),
        why: "NO_UTF_CHECK suppresses the replacement UTF check" },
    Case { rows: &[336], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 0,
        rep: b"\xf5\x80\x80\x80", sopts: PCRE2_NO_UTF_CHECK, cap: 64,
        expect: 1, exp_len: L::At(4),
        why: "NO_UTF_CHECK suppresses the replacement UTF check" },

    // ---------------------------------------------------------------- row 337
    Case { rows: &[337], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 4,
        rep: b"X", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADOFFSET, exp_len: L::Unset,
        why: "start_offset 4 > length 3" },
    Case { rows: &[337], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 100,
        rep: b"X", sopts: PCRE2_SUBSTITUTE_GLOBAL, cap: 64,
        expect: PCRE2_ERROR_BADOFFSET, exp_len: L::Unset,
        why: "start_offset far past the end" },
    Case { rows: &[337], pat: "a", copts: 0, xopts: 0, subj: b"", start: 1,
        rep: b"X", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADOFFSET, exp_len: L::Unset,
        why: "start_offset 1 with an empty subject" },
    Case { rows: &[337], pat: "a", copts: 0, xopts: 0, subj: b"abc", start: 3,
        rep: b"X", sopts: 0, cap: 64,
        expect: 0, exp_len: L::At(3),
        why: "control: start_offset == length is legal; the whole subject is copied" },

    // ---------------------------------------------------------------- row 338
    Case { rows: &[338], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"X", sopts: PCRE2_DFA_SHORTEST, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PCRE2_DFA_SHORTEST is not in PUBLIC_MATCH_OPTIONS" },
    Case { rows: &[338], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"X", sopts: PCRE2_DFA_RESTART, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "PCRE2_DFA_RESTART is not in PUBLIC_MATCH_OPTIONS" },
    Case { rows: &[338], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"X", sopts: 0x0008_0000, cap: 64,
        expect: PCRE2_ERROR_BADOPTION, exp_len: L::Unset,
        why: "a reserved bit that survives the substitute-option stripping" },

    // ---------------------------------------------------------------- row 339
    // (verbatim propagation of pcre2_match_8 errors)
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3\xa9", start: 1,
        rep: b"X", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADUTFOFFSET, exp_len: L::Unset,
        why: "start_offset inside a UTF-8 character -> -36 from pcre2_match" },
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\x80\x80", start: 0,
        rep: b"X", sopts: 0, cap: 64,
        expect: -22, exp_len: L::Unset,
        why: "invalid UTF subject -> UTF8_ERR20 from pcre2_match" },
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\xc3", start: 0,
        rep: b"X", sopts: 0, cap: 64,
        expect: -3, exp_len: L::Unset,
        why: "truncated UTF subject -> UTF8_ERR1 from pcre2_match" },
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"a\xc3", start: 0,
        rep: b"X", sopts: 0, cap: 64,
        expect: -3, exp_len: L::Unset,
        why: "truncated UTF subject (not at offset 0) -> UTF8_ERR1" },
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"\x80\x80", start: 0,
        rep: b"X", sopts: PCRE2_NO_UTF_CHECK, cap: 64,
        expect: 1, exp_len: L::At(1),
        why: "NO_UTF_CHECK: the invalid subject is consumed one code unit at a time" },
    Case { rows: &[339], pat: ".", copts: PCRE2_UTF, xopts: 0, subj: b"a\xc3", start: 0,
        rep: b"X", sopts: PCRE2_NO_UTF_CHECK, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "NO_UTF_CHECK with a truncated tail byte" },
    Case { rows: &[339], pat: "(?(DEFINE)(a\\K))b(?=(?1))", copts: 0, xopts: 0, subj: b"ba",
        start: 0, rep: b"X", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BAD_BACKSLASH_K, exp_len: L::Unset,
        why: "\\K reached through recursion inside a lookahead -> -75 (the \
              compile-time block only catches syntactic nesting)" },
    Case { rows: &[339], pat: "(?(DEFINE)(a\\K))b(?=(?1))", copts: 0, xopts: 0, subj: b"bab",
        start: 0, rep: b"X", sopts: PCRE2_SUBSTITUTE_GLOBAL, cap: 64,
        expect: PCRE2_ERROR_BAD_BACKSLASH_K, exp_len: L::Unset,
        why: "same, under PCRE2_SUBSTITUTE_GLOBAL" },

    // ---------------------------------------------------------------- row 340
    Case { rows: &[340], pat: "abcd", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIAL, exp_len: L::Unset,
        why: "partial match under PARTIAL_HARD -> -2" },
    Case { rows: &[340], pat: "abcd", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"X", sopts: PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIAL, exp_len: L::Unset,
        why: "partial match under PARTIAL_SOFT -> -2" },

    // ---------------------------------------------------------------- row 341
    // NOTE ERRORS.md gives `a\K` + PCRE2_SUBSTITUTE_GLOBAL as the example; that
    // does NOT reach this branch (\K after a one-character match keeps
    // ovector[0] == start_offset, and the substitution succeeds — see the
    // control case below). To make ovector[0] < start_offset the \K must be in
    // a *lookbehind*, which needs PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK (otherwise
    // pcre2_compile rejects it) and a non-zero start_offset.
    Case { rows: &[341], pat: "(?<=\\Ka)b", copts: 0, xopts: PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        subj: b"aab", start: 2, rep: b"X", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADSUBSPATTERN, exp_len: L::Unset,
        why: "\\K in a lookbehind moves ovector[0] before start_offset" },
    Case { rows: &[341], pat: "(?<=\\Ka)b", copts: 0, xopts: PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        subj: b"aab", start: 2, rep: b"X",
        sopts: PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADSUBSPATTERN, exp_len: L::Unset,
        why: "same, with GLOBAL|EXTENDED" },
    Case { rows: &[341], pat: "a\\K", copts: 0, xopts: 0, subj: b"aaa", start: 0,
        rep: b"X", sopts: PCRE2_SUBSTITUTE_GLOBAL, cap: 64,
        expect: 3, exp_len: L::At(6),
        why: "control: the ERRORS.md example `a\\K` + GLOBAL is NOT an error" },

    // ------------------------------------------------- rows 344..351  (-35)
    Case { rows: &[344], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(1),
        why: "replacement ends immediately after $" },
    Case { rows: &[344], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"abc$", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(4),
        why: "trailing $ after literal text" },
    Case { rows: &[345], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "replacement ends after ${" },
    Case { rows: &[345], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"x${", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(3),
        why: "replacement ends after ${ (extended mode too)" },
    Case { rows: &[346], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$<", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "replacement ends after $<" },
    Case { rows: &[347], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${*", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(3),
        why: "replacement ends after ${*" },
    Case { rows: &[347], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$*", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "replacement ends after $* (same branch, no braces)" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$-", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(1),
        why: "read_name_subst: first character is not a word character" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$ ", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(1),
        why: "read_name_subst: space is not a word character" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "read_name_subst: empty name in ${}" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${:", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "read_name_subst: ':' cannot start a name" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$<>", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(2),
        why: "read_name_subst: empty name in $<>" },
    Case { rows: &[348], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${*}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(3),
        why: "read_name_subst: empty name after ${*" },
    Case { rows: &[349], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$<name", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(6),
        why: "$<name with no closing >" },
    Case { rows: &[349], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$<name}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(6),
        why: "$<name} — } is not the > this form needs" },
    Case { rows: &[350], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${*FOO}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(7),
        why: "${*name} where name is not MARK" },
    Case { rows: &[350], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${*mark}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_BADREPLACEMENT, exp_len: L::At(8),
        why: "${*mark} — the MARK comparison is case-sensitive" },
    Case { rows: &[350], pat: "(*MARK:m1)(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${*MARK}", sopts: 0, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "control: ${*MARK} is the one accepted star name" },

    // ------------------------------------------------- rows 352..359  (-57)
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\q", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "check_escape rejects \\q" },
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\y", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "check_escape rejects \\y" },
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\x{110000}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(9),
        why: "check_escape rejects \\x{110000}" },
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\o{}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(3),
        why: "check_escape rejects \\o{}" },
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\c", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "check_escape rejects \\c at the end" },
    Case { rows: &[352], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\N{U+41}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(8),
        why: "check_escape rejects \\N{U+..} without PCRE2_UTF" },
    Case { rows: &[353], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "\\g at end of replacement" },
    Case { rows: &[353], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\gA", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "\\g not followed by <" },
    Case { rows: &[354], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<>", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(3),
        why: "\\g<> — empty name" },
    Case { rows: &[354], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<->", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(3),
        why: "\\g<-> — invalid name" },
    Case { rows: &[355], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<name", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(7),
        why: "\\g<name with no closing >" },
    Case { rows: &[356], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<1a", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(4),
        why: "\\g<1a — number read, terminator missing (ERR119)" },
    Case { rows: &[356], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<1", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(4),
        why: "\\g<1 at end of replacement (ERR119)" },
    Case { rows: &[357], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\g<70000>", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "\\g<70000> — group number above MAX_GROUP_NUMBER (ERR61)" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\d", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2),
        why: "class escape \\d is meaningless in a replacement" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\w", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "class escape \\w" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\s", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "class escape \\s" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\h", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "class escape \\h" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\A", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "assertion escape \\A" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\z", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "assertion escape \\z" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\Z", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "assertion escape \\Z" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\B", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "assertion escape \\B" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\R", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\R" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\X", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\X" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\C", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\C" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\K", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\K" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\N", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\N" },
    Case { rows: &[358], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\p{L}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(2), why: "\\p{L}" },
    Case { rows: &[359], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:-\\q}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(7),
        why: "find_text_end() hits an invalid escape in a ${name:-...} body" },
    Case { rows: &[359], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:+\\d:x}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADREPESCAPE, exp_len: L::At(7),
        why: "find_text_end() hits a class escape in a ${name:+...} body" },

    // ------------------------------------------------- rows 360, 361  (-58)
    Case { rows: &[360], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "${1 with no closing }" },
    Case { rows: &[360], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${name", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(6),
        why: "${name with no closing }" },
    Case { rows: &[360], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1x", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "${1x — junk instead of }" },
    Case { rows: &[360], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${name)", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(6),
        why: "${name) — junk instead of }" },
    Case { rows: &[360, 362], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "too short for the `ptr < repend - 2` extended guard -> -58" },
    Case { rows: &[360, 362], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:x", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "`${1:x` (no closing brace) is the case ERRORS.md meant: -58" },
    Case { rows: &[361], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:-abc", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(8),
        why: "find_text_end() runs off the end of a ${1:-...} body" },
    Case { rows: &[361], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:+abc", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(8),
        why: "find_text_end() runs off the end of a ${1:+...} body" },
    Case { rows: &[361], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:+set:unset", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(14),
        why: "find_text_end() runs off the end of the second body" },

    // ------------------------------------------------------ row 362  (-59)
    Case { rows: &[362], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:xy}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADSUBSTITUTION, exp_len: L::At(4),
        why: "character after : is neither + nor -" },
    Case { rows: &[362], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:=ab}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADSUBSTITUTION, exp_len: L::At(4),
        why: "${1:=ab} — '=' is not + or -" },
    Case { rows: &[362], pat: "(?<name>a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${name:?ab}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADSUBSTITUTION, exp_len: L::At(7),
        why: "${name:?ab} — '?' is not + or -" },
    // ERRORS.md claims `${1:x}` is too short for the `ptr < repend - 2` guard
    // and yields -58. It is NOT: with the closing brace the replacement is 6
    // code units, the guard passes, and the C returns -59.
    Case { rows: &[362], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:x}", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_BADSUBSTITUTION, exp_len: L::At(4),
        why: "${1:x} passes the ptr < repend-2 guard -> -59, not -58" },

    // ------------------------------------------------- rows 363..365  (-49)
    Case { rows: &[363], pat: "abc", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"$+", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(2),
        why: "$+ in a pattern with no capture groups" },
    Case { rows: &[363], pat: "abc", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"[$+]", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(3),
        why: "$+ mid-replacement, no capture groups" },
    Case { rows: &[363], pat: "abc", copts: 0, xopts: 0, subj: b"abc", start: 0,
        rep: b"$+", sopts: PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        cap: 64, expect: 1, exp_len: L::At(0),
        why: "control: UNKNOWN_UNSET|UNSET_EMPTY turns $+ into nothing" },
    Case { rows: &[364], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$2", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(2),
        why: "$2 above top_bracket == 1" },
    Case { rows: &[364], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${99}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(3),
        why: "${99} — detected inside the digit loop" },
    Case { rows: &[364], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$12345", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(2),
        why: "$12345 — detected on the second digit" },
    Case { rows: &[365], pat: "(?<a>x)", copts: 0, xopts: 0, subj: b"x", start: 0,
        rep: b"${b}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(4),
        why: "${b} — name absent from the name table" },
    Case { rows: &[365], pat: "(?<a>x)", copts: 0, xopts: 0, subj: b"x", start: 0,
        rep: b"$<b>", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(4),
        why: "$<b> — name absent from the name table" },
    Case { rows: &[365], pat: "(?<a>x)", copts: 0, xopts: 0, subj: b"x", start: 0,
        rep: b"\\g<b>", sopts: PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(5),
        why: "\\g<b> — name absent from the name table" },
    Case { rows: &[365], pat: "(?<a>x)", copts: 0, xopts: 0, subj: b"x", start: 0,
        rep: b"$b", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_NOSUBSTRING, exp_len: L::At(2),
        why: "$b (bare name) — absent from the name table" },

    // ------------------------------------------------- rows 367..369  (-55)
    Case { rows: &[367], pat: "a(x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$+", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(2),
        why: "$+ but every capture group is unset" },
    Case { rows: &[367], pat: "a(x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$+", sopts: PCRE2_SUBSTITUTE_UNSET_EMPTY, cap: 64,
        expect: 1, exp_len: L::At(0),
        why: "control: UNSET_EMPTY makes the unset $+ expand to nothing" },
    Case { rows: &[368], pat: "a(x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$1", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(2),
        why: "reference to an existing but unset group" },
    Case { rows: &[368], pat: "a(x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(4),
        why: "${1} for an unset group" },
    Case { rows: &[368], pat: "a(?<n>x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${n}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(4),
        why: "${n} for an unset named group" },
    Case { rows: &[368], pat: "a(x)?", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$1", sopts: PCRE2_SUBSTITUTE_UNSET_EMPTY, cap: 64,
        expect: 1, exp_len: L::At(0),
        why: "control: UNSET_EMPTY substitutes nothing for the unset group" },
    Case { rows: &[369], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$2", sopts: PCRE2_SUBSTITUTE_UNKNOWN_UNSET, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(2),
        why: "UNKNOWN_UNSET converts -49 to -55, which still errors without UNSET_EMPTY" },
    Case { rows: &[369], pat: "(?<a>x)", copts: 0, xopts: 0, subj: b"x", start: 0,
        rep: b"${b}", sopts: PCRE2_SUBSTITUTE_UNKNOWN_UNSET, cap: 64,
        expect: PCRE2_ERROR_UNSET, exp_len: L::At(4),
        why: "same for an unknown NAME" },
    Case { rows: &[369], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$2", sopts: PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        cap: 64, expect: 1, exp_len: L::At(0),
        why: "control: adding UNSET_EMPTY makes it succeed" },

    // ------------------------------------------------- rows 370, 371  (-76)
    Case { rows: &[370], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$'", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIALSUBS, exp_len: L::At(2),
        why: "$' (text after the match) is unsupported in partial mode" },
    Case { rows: &[370], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$'", sopts: PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIALSUBS, exp_len: L::At(2),
        why: "same under PARTIAL_SOFT" },
    Case { rows: &[370], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"[$'-$_]", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        cap: 64, expect: PCRE2_ERROR_PARTIALSUBS, exp_len: L::At(3),
        why: "$' mid-replacement" },
    Case { rows: &[370], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$`", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: 1, exp_len: L::At(0),
        why: "control: $` (text BEFORE the match) is allowed in partial mode" },
    Case { rows: &[371], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$_", sopts: PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIALSUBS, exp_len: L::At(2),
        why: "$_ (entire input) is unsupported in partial mode" },
    Case { rows: &[371], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$_", sopts: PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, cap: 64,
        expect: PCRE2_ERROR_PARTIALSUBS, exp_len: L::At(2),
        why: "same under PARTIAL_SOFT" },
    Case { rows: &[371], pat: "a", copts: 0, xopts: 0, subj: b"ab", start: 0,
        rep: b"$_", sopts: 0, cap: 64,
        expect: 1, exp_len: L::At(3),
        why: "control: $_ outside partial mode expands to the whole subject" },

    // ------------------------------------------------------- row 372  (-48)
    Case { rows: &[372], pat: "a", copts: 0, xopts: 0, subj: b"aaa", start: 0,
        rep: b"bbbb", sopts: 0, cap: 4,
        expect: PCRE2_ERROR_NOMEMORY, exp_len: L::Unset,
        why: "output buffer too small, no OVERFLOW_LENGTH" },
    Case { rows: &[372], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"b", sopts: 0, cap: 1,
        expect: PCRE2_ERROR_NOMEMORY, exp_len: L::Unset,
        why: "buffer exactly the result length: no room for the terminating NUL" },
    Case { rows: &[372], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"b", sopts: 0, cap: 0,
        expect: PCRE2_ERROR_NOMEMORY, exp_len: L::Unset,
        why: "*outlengthptr == 0" },
    Case { rows: &[372], pat: "a", copts: 0, xopts: 0, subj: b"aaa", start: 0,
        rep: b"bbbb", sopts: PCRE2_SUBSTITUTE_GLOBAL, cap: 8,
        expect: PCRE2_ERROR_NOMEMORY, exp_len: L::Unset,
        why: "global substitution overruns the buffer half way through" },
    Case { rows: &[372], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"b", sopts: 0, cap: 2,
        expect: 1, exp_len: L::At(1),
        why: "control: length+1 is exactly enough" },

    // ------------------------------------------------------- rows 378, 379
    Case { rows: &[378], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${", sopts: PCRE2_SUBSTITUTE_LITERAL, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "PCRE2_SUBSTITUTE_LITERAL: `${` is inserted verbatim" },
    Case { rows: &[378], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$1", sopts: PCRE2_SUBSTITUTE_LITERAL, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "LITERAL: no group reference is parsed" },
    Case { rows: &[378], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\q", sopts: PCRE2_SUBSTITUTE_LITERAL | PCRE2_SUBSTITUTE_EXTENDED, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "LITERAL beats EXTENDED: `\\q` is two literal code units" },
    Case { rows: &[378], pat: "a", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"$", sopts: PCRE2_SUBSTITUTE_LITERAL, cap: 64,
        expect: 1, exp_len: L::At(1),
        why: "LITERAL: a lone trailing $ is not an error" },
    Case { rows: &[379], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\q", sopts: 0, cap: 64,
        expect: 1, exp_len: L::At(2),
        why: "without EXTENDED, backslash is literal: `\\q` inserts \\q" },
    Case { rows: &[379], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"\\U$1", sopts: 0, cap: 64,
        expect: 1, exp_len: L::At(3),
        why: "without EXTENDED, `\\U` is two literal code units" },
    Case { rows: &[379], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:-x}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "without EXTENDED, ${1:-x} is not an extended construct -> -58" },
    Case { rows: &[379], pat: "(a)", copts: 0, xopts: 0, subj: b"a", start: 0,
        rep: b"${1:+y:n}", sopts: 0, cap: 64,
        expect: PCRE2_ERROR_REPMISSINGBRACE, exp_len: L::At(3),
        why: "without EXTENDED, ${1:+y:n} is not an extended construct -> -58" },
];

#[test]
fn substitute_error_paths() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        for c in CASES {
            let (a, b, cca, ccb) = compile_both(p, c.pat, c.copts, c.xopts);
            // one shared, padded copy of the subject/replacement so that both
            // libraries see the identical pointer AND the identical bytes just
            // past the nominal end.
            let sbuf = padded(c.subj);
            let rbuf = padded(c.rep);
            let (sp, slen) = (sbuf.as_ptr(), c.subj.len());
            let (rp, rlen) = (rbuf.as_ptr(), c.rep.len());
            let oa = call(&p.c, a, sp, slen, c.start, c.sopts, ptr::null_mut(),
                ptr::null_mut(), rp, rlen, c.cap, false);
            let ob = call(&p.r, b, sp, slen, c.start, c.sopts, ptr::null_mut(),
                ptr::null_mut(), rp, rlen, c.cap, false);
            let tag = format!(
                "rows {:?} [{}] pat={:?} copts={:#x} xopts={:#x} subj={} start={} rep={} sopts={:#x} cap={}",
                c.rows, c.why, c.pat, c.copts, c.xopts, show(c.subj), c.start,
                show(c.rep), c.sopts, c.cap
            );
            d.eq(&format!("{tag}\n  RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag}\n  OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag}\n  BUFFER"), show(&oa.buf), show(&ob.buf));
            // the C is ground truth for what ERRORS.md should say
            if oa.rc != c.expect {
                doc.push(format!(
                    "rows {:?}: ERRORS.md documents {} but the C returns {} — {tag}",
                    c.rows, c.expect, oa.rc
                ));
            }
            if !c.exp_len.matches(oa.len) {
                doc.push(format!(
                    "rows {:?}: expected *outlengthptr {} but the C wrote {} — {tag}",
                    c.rows, c.exp_len.show(), show_len(oa.len)
                ));
            }
            free_both(p, a, b, cca, ccb);
        }
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 324/327/336..341/344..365/367..372/378/379");
}

// ======================================================= rows 325, 326 (NULL)

// Rows 325 and 326, plus the generic NULL/zero-length argument boundaries.
//
// NOT TESTED, deliberately: `code == NULL` and `outlengthptr == NULL`.
// `pcre2_substitute` dereferences both before any validation
// (`code->overall_options` at pcre2_substitute.c:758 and `*blength` at :778),
// so those are not comparable observables — they are out-of-bounds in the C
// itself, exactly the situation HARNESS.md says to skip rather than crash the
// harness on. Every other pointer argument IS checked and is exercised below.
#[test]
fn substitute_null_arguments() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        let (a, b, cca, ccb) = compile_both(p, "a", 0, 0);
        let subj = padded(b"aXa");
        let rep = padded(b"Z");

        // rows: [325] — replacement == NULL with rlength != 0
        for (rlen, exp) in [
            (1usize, PCRE2_ERROR_NULL),
            (3, PCRE2_ERROR_NULL),
            (PCRE2_ZERO_TERMINATED, PCRE2_ERROR_NULL),
        ] {
            let oa = call(&p.c, a, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(),
                ptr::null(), rlen, 64, false);
            let ob = call(&p.r, b, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(),
                ptr::null(), rlen, 64, false);
            let tag = format!("rows [325] replacement=NULL rlength={rlen}");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc != exp {
                doc.push(format!("{tag}: documented {exp}, C returned {}", oa.rc));
            }
            if oa.len != PCRE2_UNSET {
                doc.push(format!("{tag}: *outlengthptr should be UNSET, got {}", oa.len));
            }
        }
        // rows: [325] — replacement == NULL with rlength == 0 is LEGAL
        {
            let oa = call(&p.c, a, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(),
                ptr::null(), 0, 64, false);
            let ob = call(&p.r, b, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(),
                ptr::null(), 0, 64, false);
            d.eq("rows [325] control replacement=NULL rlength=0 RC", oa.rc, ob.rc);
            d.eq("rows [325] control OUTLENGTH", oa.len, ob.len);
            d.eq("rows [325] control BUFFER", show(&oa.buf), show(&ob.buf));
            if oa.rc != 1 || oa.len != 2 {
                doc.push(format!(
                    "rows [325] control: NULL/0 replacement should delete the match \
                     (rc 1, len 2), got rc {} len {}",
                    oa.rc, show_len(oa.len)
                ));
            }
        }

        // rows: [326] — subject == NULL with length != 0
        for slen in [1usize, 3, PCRE2_ZERO_TERMINATED] {
            let oa = call(&p.c, a, ptr::null(), slen, 0, 0, ptr::null_mut(), ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let ob = call(&p.r, b, ptr::null(), slen, 0, 0, ptr::null_mut(), ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let tag = format!("rows [326] subject=NULL length={slen}");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc != PCRE2_ERROR_NULL {
                doc.push(format!("{tag}: documented -51, C returned {}", oa.rc));
            }
            if oa.len != PCRE2_UNSET {
                doc.push(format!("{tag}: *outlengthptr should be UNSET, got {}", oa.len));
            }
        }
        // rows: [326] — subject == NULL with length == 0 is LEGAL (empty subject)
        {
            let oa = call(&p.c, a, ptr::null(), 0, 0, 0, ptr::null_mut(), ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let ob = call(&p.r, b, ptr::null(), 0, 0, 0, ptr::null_mut(), ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            d.eq("rows [326] control subject=NULL length=0 RC", oa.rc, ob.rc);
            d.eq("rows [326] control OUTLENGTH", oa.len, ob.len);
            d.eq("rows [326] control BUFFER", show(&oa.buf), show(&ob.buf));
            if oa.rc != 0 {
                doc.push(format!(
                    "rows [326] control: empty subject should not match `a` (rc 0), got {}",
                    oa.rc
                ));
            }
        }

        // NULL output buffer with *outlengthptr == 0. Nothing may be written, so
        // this is well defined: the terminating NUL alone overflows.
        for (sopts, tagname) in [(0u32, "plain"), (PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, "OVERFLOW_LENGTH")] {
            let mut la = 0usize;
            let mut lb = 0usize;
            let ra = (p.c.substitute)(a, subj.as_ptr(), 3, 0, sopts, ptr::null_mut(),
                ptr::null_mut(), rep.as_ptr(), 1, ptr::null_mut(), &mut la);
            let rbv = (p.r.substitute)(b, subj.as_ptr(), 3, 0, sopts, ptr::null_mut(),
                ptr::null_mut(), rep.as_ptr(), 1, ptr::null_mut(), &mut lb);
            d.eq(&format!("rows [325,326] NULL buffer/0 length {tagname} RC"), ra, rbv);
            d.eq(&format!("rows [325,326] NULL buffer/0 length {tagname} OUTLENGTH"),
                show_len(la), show_len(lb));
            if ra != PCRE2_ERROR_NOMEMORY {
                doc.push(format!(
                    "NULL buffer with *outlengthptr 0 ({tagname}) should be -48, got {ra}"
                ));
            }
        }

        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 325, 326 (NULL argument validation)");
}

// ============================================== rows 328..333 SUBSTITUTE_MATCHED

/// One `PCRE2_SUBSTITUTE_MATCHED` consistency case: prime a `match_data` with
/// one (pattern, subject, start_offset, options) and then call
/// `pcre2_substitute` with a DIFFERENT one.
struct MatchedCase {
    rows: &'static [u32],
    /// pattern used for the priming `pcre2_match_8`
    pat_match: &'static str,
    /// pattern passed to `pcre2_substitute_8`
    pat_subst: &'static str,
    /// subject/length/offset/options of the priming match
    m_subj: &'static [u8],
    m_len: usize,
    m_start: usize,
    m_opts: u32,
    /// ... and of the substitution
    s_len: usize,
    s_start: usize,
    s_opts: u32,
    /// use a *different* buffer holding the same bytes for the substitution
    other_buffer: bool,
    expect: i32,
    why: &'static str,
}

const MATCHED: &[MatchedCase] = &[
    // row 328: the stored rc is a real error, returned verbatim before any
    // other check.
    MatchedCase { rows: &[328], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 9, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_BADOFFSET,
        why: "match_data->rc == -33 left by a bad-offset pcre2_match" },
    MatchedCase { rows: &[328], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: PCRE2_DFA_SHORTEST, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_BADOPTION,
        why: "match_data->rc == -34 left by a bad-option pcre2_match" },
    MatchedCase { rows: &[328], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: 1, why: "control: a successful stored match is used as-is" },
    MatchedCase { rows: &[328], pat_match: "(z)", pat_subst: "(z)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: 0, why: "control: a stored PCRE2_ERROR_NOMATCH is not an error here" },
    // row 330
    MatchedCase { rows: &[330], pat_match: "(a)", pat_subst: "(b)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSPATTERN,
        why: "code != match_data->code" },
    // row 331
    MatchedCase { rows: &[331], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 2, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSSUBJECT,
        why: "length differs from match_data->subject_length" },
    MatchedCase { rows: &[331], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: true,
        expect: PCRE2_ERROR_DIFFSUBSSUBJECT,
        why: "same bytes but a different subject pointer (no COPY_MATCHED_SUBJECT)" },
    // row 332
    MatchedCase { rows: &[332], pat_match: "(a)", pat_subst: "(a)", m_subj: b"aba",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 1,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSOFFSET,
        why: "start_offset != match_data->start_offset" },
    MatchedCase { rows: &[332], pat_match: "(a)", pat_subst: "(a)", m_subj: b"aba",
        m_len: 3, m_start: 2, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSOFFSET,
        why: "start_offset differs the other way round" },
    // row 333
    MatchedCase { rows: &[333], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: PCRE2_NOTBOL, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSOPTIONS,
        why: "matched with PCRE2_NOTBOL, substituting without it" },
    MatchedCase { rows: &[333], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED | PCRE2_NOTEOL, other_buffer: false,
        expect: PCRE2_ERROR_DIFFSUBSOPTIONS,
        why: "substituting with an extra non-substitute match option" },
    MatchedCase { rows: &[333], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED | PCRE2_NO_UTF_CHECK, other_buffer: false,
        expect: 1,
        why: "control: PCRE2_NO_UTF_CHECK is explicitly excluded from the comparison" },
    MatchedCase { rows: &[333], pat_match: "(a)", pat_subst: "(a)", m_subj: b"abc",
        m_len: 3, m_start: 0, m_opts: 0, s_len: 3, s_start: 0,
        s_opts: PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        other_buffer: false, expect: 1,
        why: "control: the SUBSTITUTE_* bits are excluded from the comparison" },
];

#[test]
fn substitute_matched_consistency() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        for c in MATCHED {
            let (ma, mb, mcca, mccb) = compile_both(p, c.pat_match, 0, 0);
            // Only compile a SECOND code object when the substitution really is
            // meant to use a different pattern; otherwise `code` must be the very
            // same pointer the match_data recorded, or row 330 fires first.
            let same = c.pat_match == c.pat_subst;
            let (sa, sb, scca, sccb) = if same {
                (ma, mb, ptr::null_mut(), ptr::null_mut())
            } else {
                compile_both(p, c.pat_subst, 0, 0)
            };
            let subj = padded(c.m_subj);
            let other = padded(c.m_subj);
            assert_ne!(subj.as_ptr(), other.as_ptr());
            let rep = padded(b"Z");

            let mda = (p.c.match_data_create_from_pattern)(ma, ptr::null_mut());
            let mdb = (p.r.match_data_create_from_pattern)(mb, ptr::null_mut());
            assert!(!mda.is_null() && !mdb.is_null());
            let pa = (p.c.do_match)(ma, subj.as_ptr(), c.m_len, c.m_start, c.m_opts, mda,
                ptr::null_mut());
            let pb = (p.r.do_match)(mb, subj.as_ptr(), c.m_len, c.m_start, c.m_opts, mdb,
                ptr::null_mut());
            d.eq(
                &format!("rows {:?} priming pcre2_match_8 rc", c.rows),
                read_match_out(&p.c, mda, pa),
                read_match_out(&p.r, mdb, pb),
            );

            let sp = if c.other_buffer { other.as_ptr() } else { subj.as_ptr() };
            let oa = call(&p.c, sa, sp, c.s_len, c.s_start, c.s_opts, mda, ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let ob = call(&p.r, sb, sp, c.s_len, c.s_start, c.s_opts, mdb, ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let tag = format!("rows {:?} [{}]", c.rows, c.why);
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc != c.expect {
                doc.push(format!("{tag}: documented {}, C returned {}", c.expect, oa.rc));
            }
            // Every one of these consistency failures returns before *blength
            // can be given a replacement offset.
            if c.expect < 0 && oa.len != PCRE2_UNSET {
                doc.push(format!("{tag}: *outlengthptr should be UNSET, got {}", oa.len));
            }

            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
            if !same {
                free_both(p, sa, sb, scca, sccb);
            }
            free_both(p, ma, mb, mcca, mccb);
        }

        // row 329: a match_data produced by pcre2_dfa_match_8
        let (a, b, cca, ccb) = compile_both(p, "a", 0, 0);
        let subj = padded(b"a");
        let rep = padded(b"Z");
        let mda = (p.c.match_data_create_from_pattern)(a, ptr::null_mut());
        let mdb = (p.r.match_data_create_from_pattern)(b, ptr::null_mut());
        let mut wsa = [0 as c_int; 64];
        let mut wsb = [0 as c_int; 64];
        let da = (p.c.dfa_match)(a, subj.as_ptr(), 1, 0, 0, mda, ptr::null_mut(),
            wsa.as_mut_ptr(), 64);
        let db = (p.r.dfa_match)(b, subj.as_ptr(), 1, 0, 0, mdb, ptr::null_mut(),
            wsb.as_mut_ptr(), 64);
        d.eq("rows [329] priming pcre2_dfa_match_8 rc", da, db);
        for so in [
            PCRE2_SUBSTITUTE_MATCHED,
            PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL,
        ] {
            let oa = call(&p.c, a, subj.as_ptr(), 1, 0, so, mda, ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            let ob = call(&p.r, b, subj.as_ptr(), 1, 0, so, mdb, ptr::null_mut(),
                rep.as_ptr(), 1, 64, false);
            d.eq(&format!("rows [329] sopts={so:#x} RC"), oa.rc, ob.rc);
            d.eq(&format!("rows [329] sopts={so:#x} OUTLENGTH"), show_len(oa.len),
                show_len(ob.len));
            d.eq(&format!("rows [329] sopts={so:#x} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc != PCRE2_ERROR_DFA_UFUNC {
                doc.push(format!("rows [329]: documented -41, C returned {}", oa.rc));
            }
            if oa.len != PCRE2_UNSET {
                doc.push(format!("rows [329]: *outlengthptr should be UNSET, got {}", oa.len));
            }
        }
        (p.c.match_data_free)(mda);
        (p.r.match_data_free)(mdb);
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 328..333 (PCRE2_SUBSTITUTE_MATCHED consistency) and 329");
}

// ======================================================= rows 334, 335 (NOMEMORY)

// A malloc that fails after N successful calls, with a SEPARATE counter per
// library so the two runs cannot interfere.
static mut BUDGET_C: i64 = -1;
static mut BUDGET_R: i64 = -1;

unsafe fn fallible(budget: &mut i64, n: usize) -> *mut c_void {
    if *budget == 0 {
        return ptr::null_mut();
    }
    if *budget > 0 {
        *budget -= 1;
    }
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let q = std::alloc::alloc(l);
    assert!(!q.is_null());
    *(q as *mut usize) = sz;
    q.add(16) as *mut c_void
}
unsafe extern "C" fn fmalloc_c(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(&mut *ptr::addr_of_mut!(BUDGET_C), n)
}
unsafe extern "C" fn fmalloc_r(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(&mut *ptr::addr_of_mut!(BUDGET_R), n)
}
unsafe extern "C" fn ffree(q: *mut c_void, _d: *mut c_void) {
    if q.is_null() {
        return;
    }
    let base = (q as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

#[test]
fn substitute_allocation_failure() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        let (a, b, cca, ccb) = compile_both(p, "(a)(b)", 0, 0);
        let subj = padded(b"ab");
        let rep = padded(b"[$1$2]");
        let ga = (p.c.general_context_create)(Some(fmalloc_c), Some(ffree), ptr::null_mut());
        let gb = (p.r.general_context_create)(Some(fmalloc_r), Some(ffree), ptr::null_mut());
        assert!(!ga.is_null() && !gb.is_null());
        // The hand-built general context inside pcre2_substitute takes its
        // memctl from the match context when one is supplied
        // (pcre2_substitute.c:890-892 / 902-904), so this is what makes both
        // internal match_data allocations fallible.
        let mca = (p.c.match_context_create)(ga);
        let mcb = (p.r.match_context_create)(gb);
        assert!(!mca.is_null() && !mcb.is_null());

        // rows: [334] — match_data == NULL, pcre2_match_data_create_from_pattern fails
        // rows: [335] — PCRE2_SUBSTITUTE_MATCHED, the internal
        //               pcre2_match_data_create fails
        for use_matched in [false, true] {
            for n in 0..10i64 {
                // For the MATCHED case the external match_data must use the same
                // allocator as the match context: pcre2_substitute memcpy's the
                // external block (memctl included) over the internal one, so the
                // internal block is released through the EXTERNAL block's memctl.
                let (mda, mdb) = if use_matched {
                    let x = (p.c.match_data_create_from_pattern)(a, ga);
                    let y = (p.r.match_data_create_from_pattern)(b, gb);
                    assert!(!x.is_null() && !y.is_null());
                    (p.c.do_match)(a, subj.as_ptr(), 2, 0, 0, x, ptr::null_mut());
                    (p.r.do_match)(b, subj.as_ptr(), 2, 0, 0, y, ptr::null_mut());
                    (x, y)
                } else {
                    (ptr::null_mut(), ptr::null_mut())
                };
                let so = if use_matched { PCRE2_SUBSTITUTE_MATCHED } else { 0 };

                *ptr::addr_of_mut!(BUDGET_C) = n;
                let oa = call(&p.c, a, subj.as_ptr(), 2, 0, so, mda, mca, rep.as_ptr(), 6,
                    64, false);
                *ptr::addr_of_mut!(BUDGET_C) = -1;
                *ptr::addr_of_mut!(BUDGET_R) = n;
                let ob = call(&p.r, b, subj.as_ptr(), 2, 0, so, mdb, mcb, rep.as_ptr(), 6,
                    64, false);
                *ptr::addr_of_mut!(BUDGET_R) = -1;

                let which = if use_matched { "[335]" } else { "[334]" };
                let tag = format!("rows {which} fallible malloc, budget={n}");
                d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                if n == 0 {
                    if oa.rc != PCRE2_ERROR_NOMEMORY {
                        doc.push(format!("{tag}: documented -48, C returned {}", oa.rc));
                    }
                    if oa.len != PCRE2_UNSET {
                        doc.push(format!("{tag}: *outlengthptr should be UNSET, got {}", oa.len));
                    }
                }
                if !mda.is_null() {
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
            }
        }
        (p.c.match_context_free)(mca);
        (p.r.match_context_free)(mcb);
        (p.c.general_context_free)(ga);
        (p.r.general_context_free)(gb);
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 334, 335 (allocation failure, budget swept)");
}

// ============================================ row 339: match errors via limits

#[test]
fn substitute_match_error_propagation() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    // (match_limit, depth_limit, heap_limit, documented code)
    // u32::MAX means "leave the default alone".
    let cases: &[(u32, u32, u32, c_int, &str)] = &[
        (1, u32::MAX, u32::MAX, PCRE2_ERROR_MATCHLIMIT, "match limit 1"),
        (100_000, u32::MAX, u32::MAX, PCRE2_ERROR_MATCHLIMIT, "match limit 100000"),
        (u32::MAX, 1, u32::MAX, PCRE2_ERROR_DEPTHLIMIT, "depth limit 1"),
        (u32::MAX, u32::MAX, 0, PCRE2_ERROR_HEAPLIMIT, "heap limit 0"),
    ];
    unsafe {
        // PCRE2_NO_START_OPTIMIZE stops the start-up optimisation from rejecting
        // the subject outright, so the interpreter really runs and really blows
        // the limit.
        let (a, b, cca, ccb) = compile_both(p, "(a+)+b", PCRE2_NO_START_OPTIMIZE, 0);
        let subj = padded(b"aaaaaaaaaaaaaaaaaaaaaaaaaa");
        let rep = padded(b"Z");
        for &(ml, dl, hl, exp, name) in cases {
            for sopts in [0u32, PCRE2_SUBSTITUTE_GLOBAL, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH] {
                let mca = (p.c.match_context_create)(ptr::null_mut());
                let mcb = (p.r.match_context_create)(ptr::null_mut());
                if ml != u32::MAX {
                    d.eq("set_match_limit", (p.c.set_match_limit)(mca, ml),
                        (p.r.set_match_limit)(mcb, ml));
                }
                if dl != u32::MAX {
                    d.eq("set_depth_limit", (p.c.set_depth_limit)(mca, dl),
                        (p.r.set_depth_limit)(mcb, dl));
                }
                if hl != u32::MAX {
                    d.eq("set_heap_limit", (p.c.set_heap_limit)(mca, hl),
                        (p.r.set_heap_limit)(mcb, hl));
                }
                let oa = call(&p.c, a, subj.as_ptr(), 26, 0, sopts, ptr::null_mut(), mca,
                    rep.as_ptr(), 1, 64, false);
                let ob = call(&p.r, b, subj.as_ptr(), 26, 0, sopts, ptr::null_mut(), mcb,
                    rep.as_ptr(), 1, 64, false);
                let tag = format!("rows [339] {name} sopts={sopts:#x}");
                d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                if oa.rc != exp {
                    doc.push(format!("{tag}: documented {exp}, C returned {}", oa.rc));
                }
                if oa.len != PCRE2_UNSET {
                    doc.push(format!("{tag}: *outlengthptr should be UNSET, got {}", oa.len));
                }
                (p.c.match_context_free)(mca);
                (p.r.match_context_free)(mcb);
            }
        }
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c row 339 (pcre2_match_8 errors propagated verbatim)");
}

// ================================================= row 366: $+ with a small ovector

#[test]
fn substitute_unavailable_group() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        let (a, b, cca, ccb) = compile_both(p, "(a)(b)(c)", 0, 0);
        let subj = padded(b"abc");
        let rep = padded(b"$+");
        // top_bracket == 3, so anything below oveccount 4 is "unavailable".
        for (n, exp) in [(1u32, PCRE2_ERROR_UNAVAILABLE), (2, PCRE2_ERROR_UNAVAILABLE),
                         (3, PCRE2_ERROR_UNAVAILABLE), (4, 1)] {
            let mda = (p.c.match_data_create)(n, ptr::null_mut());
            let mdb = (p.r.match_data_create)(n, ptr::null_mut());
            assert!(!mda.is_null() && !mdb.is_null());
            let oa = call(&p.c, a, subj.as_ptr(), 3, 0, 0, mda, ptr::null_mut(),
                rep.as_ptr(), 2, 64, false);
            let ob = call(&p.r, b, subj.as_ptr(), 3, 0, 0, mdb, ptr::null_mut(),
                rep.as_ptr(), 2, 64, false);
            let tag = format!("rows [366] oveccount={n}");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc != exp {
                doc.push(format!("{tag}: documented {exp}, C returned {}", oa.rc));
            }
            if exp == PCRE2_ERROR_UNAVAILABLE && oa.len != 2 {
                doc.push(format!(
                    "{tag}: PTREXIT should set *outlengthptr to 2, got {}", show_len(oa.len)
                ));
            }
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
        }
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c row 366 ($+ with match_data->oveccount < top_bracket+1)");
}

// ========================================== rows 372, 373: NOMEMORY / OVERFLOW_LENGTH

#[test]
fn substitute_overflow_length() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    let cases: &[(&str, &[u8], &[u8], u32)] = &[
        ("a", b"aaa", b"bbbb", 0),
        ("a", b"aaa", b"bbbb", PCRE2_SUBSTITUTE_GLOBAL),
        ("(a)", b"xaay", b"[$1]", PCRE2_SUBSTITUTE_GLOBAL),
        ("(a)", b"xaay", b"[$1]", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY),
        ("(?<n>a)", b"ba", b"${n}${n}", PCRE2_SUBSTITUTE_EXTENDED),
        ("a", b"aaaa", b"\\U$0", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED),
    ];
    unsafe {
        for &(pat, subj, rep, base) in cases {
            let (a, b, cca, ccb) = compile_both(p, pat, 0, 0);
            let sb = padded(subj);
            let rb = padded(rep);
            for cap in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                // (a) without PCRE2_SUBSTITUTE_OVERFLOW_LENGTH — row 372
                let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, base, ptr::null_mut(),
                    ptr::null_mut(), rb.as_ptr(), rep.len(), cap, false);
                let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, base, ptr::null_mut(),
                    ptr::null_mut(), rb.as_ptr(), rep.len(), cap, false);
                let tag = format!(
                    "rows [372] pat={pat:?} subj={} rep={} sopts={base:#x} cap={cap}",
                    show(subj), show(rep)
                );
                d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                if oa.rc == PCRE2_ERROR_NOMEMORY && oa.len != PCRE2_UNSET {
                    doc.push(format!(
                        "{tag}: -48 without OVERFLOW_LENGTH must leave *outlengthptr UNSET, got {}",
                        oa.len
                    ));
                }

                // (b) with PCRE2_SUBSTITUTE_OVERFLOW_LENGTH — row 373
                let so = base | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
                let oc = call(&p.c, a, sb.as_ptr(), subj.len(), 0, so, ptr::null_mut(),
                    ptr::null_mut(), rb.as_ptr(), rep.len(), cap, false);
                let od = call(&p.r, b, sb.as_ptr(), subj.len(), 0, so, ptr::null_mut(),
                    ptr::null_mut(), rb.as_ptr(), rep.len(), cap, false);
                let tag2 = format!(
                    "rows [373] pat={pat:?} subj={} rep={} sopts={so:#x} cap={cap}",
                    show(subj), show(rep)
                );
                d.eq(&format!("{tag2} RC"), oc.rc, od.rc);
                d.eq(&format!("{tag2} OUTLENGTH"), show_len(oc.len), show_len(od.len));
                d.eq(&format!("{tag2} BUFFER"), show(&oc.buf), show(&od.buf));

                // The two forms must agree on WHETHER it overflowed.
                if (oa.rc == PCRE2_ERROR_NOMEMORY) != (oc.rc == PCRE2_ERROR_NOMEMORY) {
                    doc.push(format!(
                        "{tag2}: OVERFLOW_LENGTH changed whether the call fails ({} vs {})",
                        oa.rc, oc.rc
                    ));
                }
                if oc.rc == PCRE2_ERROR_NOMEMORY {
                    if oc.len == PCRE2_UNSET {
                        doc.push(format!("{tag2}: OVERFLOW_LENGTH must report a length"));
                        continue;
                    }
                    if oc.len <= cap {
                        doc.push(format!(
                            "{tag2}: required length {} is not greater than cap {cap}", oc.len
                        ));
                    }
                    // ... and the reported length must be exactly enough.
                    let need = oc.len;
                    let ra = call(&p.c, a, sb.as_ptr(), subj.len(), 0, base, ptr::null_mut(),
                        ptr::null_mut(), rb.as_ptr(), rep.len(), need, false);
                    let rbv = call(&p.r, b, sb.as_ptr(), subj.len(), 0, base, ptr::null_mut(),
                        ptr::null_mut(), rb.as_ptr(), rep.len(), need, false);
                    d.eq(&format!("{tag2} retry RC"), ra.rc, rbv.rc);
                    d.eq(&format!("{tag2} retry OUTLENGTH"), show_len(ra.len), show_len(rbv.len));
                    d.eq(&format!("{tag2} retry BUFFER"), show(&ra.buf), show(&rbv.buf));
                    if ra.rc < 0 {
                        doc.push(format!(
                            "{tag2}: retry with the reported length {need} still failed: {}",
                            ra.rc
                        ));
                    } else if ra.len + 1 != need {
                        doc.push(format!(
                            "{tag2}: reported length {need} but the result needed only {}",
                            ra.len + 1
                        ));
                    }
                    // one code unit less must NOT be enough
                    let tight = call(&p.c, a, sb.as_ptr(), subj.len(), 0, base,
                        ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), rep.len(), need - 1, false);
                    let tightr = call(&p.r, b, sb.as_ptr(), subj.len(), 0, base,
                        ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), rep.len(), need - 1, false);
                    d.eq(&format!("{tag2} tight RC"), tight.rc, tightr.rc);
                    d.eq(&format!("{tag2} tight OUTLENGTH"), show_len(tight.len),
                        show_len(tightr.len));
                    d.eq(&format!("{tag2} tight BUFFER"), show(&tight.buf), show(&tightr.buf));
                    if tight.rc != PCRE2_ERROR_NOMEMORY {
                        doc.push(format!(
                            "{tag2}: {} code units should NOT have been enough (rc {})",
                            need - 1, tight.rc
                        ));
                    }
                }
            }
            free_both(p, a, b, cca, ccb);
        }
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 372, 373 (buffer overflow with and without OVERFLOW_LENGTH)");
}

// ============================== rows 374, 375: substitute_case_callout failures

static mut CASE_MODE: u32 = 0;
static mut CASE_LOG: Vec<u8> = Vec::new();

/// Case callout whose behaviour is selected by `CASE_MODE`:
///   0 — well behaved: ASCII-case the input, reporting "not enough room" the
///       documented way (a return value larger than `output_cap`);
///   1 — always reports `PCRE2_SIZE_MAX`, i.e. the hard failure (row 374);
///   2 — always claims to need `PCRE2_SIZE_MAX - 1` code units (row 375);
///   3 — always claims to need `PCRE2_SIZE_MAX / 2` code units.
unsafe extern "C" fn case_cb(
    input: Sptr,
    inlen: Sz,
    output: *mut u8,
    outlen: Sz,
    to_case: c_int,
    _d: *mut c_void,
) -> Sz {
    let log = &mut *ptr::addr_of_mut!(CASE_LOG);
    log.extend_from_slice(&(inlen as u64).to_le_bytes());
    log.extend_from_slice(&(outlen as u64).to_le_bytes());
    log.extend_from_slice(&(to_case as i64).to_le_bytes());
    if inlen != 0 {
        log.extend_from_slice(std::slice::from_raw_parts(input, inlen));
    }
    match *ptr::addr_of!(CASE_MODE) {
        1 => usize::MAX,
        2 => usize::MAX - 1,
        3 => usize::MAX / 2,
        _ => {
            if inlen > outlen {
                // The documented "not enough room" report: ask for more space.
                return inlen;
            }
            for i in 0..inlen {
                let ch = *input.add(i);
                *output.add(i) = match to_case {
                    1 => ch.to_ascii_lowercase(),
                    _ => ch.to_ascii_uppercase(),
                };
            }
            inlen
        }
    }
}

#[test]
fn substitute_case_callout_errors() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    let mut seen_69 = 0usize;
    let mut seen_70 = 0usize;
    unsafe {
        for pat in ["(a)", "(a)(b)"] {
            let (a, b, cca, ccb) = compile_both(p, pat, 0, 0);
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            d.eq(
                "set_substitute_case_callout rc",
                (p.c.set_substitute_case_callout)(mca, Some(case_cb), ptr::null_mut()),
                (p.r.set_substitute_case_callout)(mcb, Some(case_cb), ptr::null_mut()),
            );
            for subj in [&b"a"[..], &b"ax"[..], &b"xa"[..], &b"xayz"[..], &b"ab"[..]] {
                let sb = padded(subj);
                for rep in [
                    &b"\\Uabc"[..], &b"\\uabc"[..], &b"\\U$1"[..], &b"\\l\\Uabc"[..],
                    &b"\\LABC"[..], &b"\\u\\Labc"[..], &b"x\\Uy$1z"[..],
                ] {
                    let rb = padded(rep);
                    for mode in [0u32, 1, 2, 3] {
                        for so in [
                            PCRE2_SUBSTITUTE_EXTENDED,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL
                                | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                        ] {
                            for cap in [2usize, 8, 32] {
                                *ptr::addr_of_mut!(CASE_MODE) = mode;
                                (*ptr::addr_of_mut!(CASE_LOG)).clear();
                                let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, so,
                                    ptr::null_mut(), mca, rb.as_ptr(), rep.len(), cap, false);
                                let loga = (*ptr::addr_of!(CASE_LOG)).clone();
                                (*ptr::addr_of_mut!(CASE_LOG)).clear();
                                let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, so,
                                    ptr::null_mut(), mcb, rb.as_ptr(), rep.len(), cap, false);
                                let logb = (*ptr::addr_of!(CASE_LOG)).clone();
                                let which = match mode {
                                    1 => "[374]",
                                    2 | 3 => "[375]",
                                    _ => "[374,375]",
                                };
                                let tag = format!(
                                    "rows {which} case-callout mode={mode} pat={pat:?} \
                                     subj={} rep={} sopts={so:#x} cap={cap}",
                                    show(subj), show(rep)
                                );
                                d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                                d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len),
                                    show_len(ob.len));
                                d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                                d.eq(&format!("{tag} CALLOUT-LOG"), loga, logb);
                                // Documented outcomes. A misbehaving case
                                // callout can only be *observed* when a case
                                // transformation actually runs, so the sweep
                                // asserts the invariants and counts the
                                // witnesses instead of demanding a code per row.
                                if oa.rc == PCRE2_ERROR_REPLACECASE {
                                    // Either the callout reported PCRE2_SIZE_MAX
                                    // outright (mode 1), or two of its huge
                                    // answers overflowed when added together in
                                    // do_case_copy (pcre2_substitute.c:598).
                                    seen_69 += 1;
                                    if mode == 0 {
                                        doc.push(format!(
                                            "{tag}: -69 must not come from a well-behaved callout"
                                        ));
                                    }
                                }
                                if oa.rc == PCRE2_ERROR_TOOLARGEREPLACE {
                                    seen_70 += 1;
                                    if mode == 0 {
                                        doc.push(format!(
                                            "{tag}: -70 must not come from a well-behaved callout"
                                        ));
                                    }
                                }
                                if mode == 0 && oa.rc < 0 && oa.rc != PCRE2_ERROR_NOMEMORY {
                                    doc.push(format!(
                                        "{tag}: a well-behaved case callout must not make the \
                                         call fail, C returned {}", oa.rc
                                    ));
                                }
                                if oa.rc == PCRE2_ERROR_REPLACECASE && oa.len != PCRE2_UNSET {
                                    doc.push(format!(
                                        "{tag}: -69 must leave *outlengthptr UNSET, got {}",
                                        oa.len
                                    ));
                                }
                                if oa.rc == PCRE2_ERROR_TOOLARGEREPLACE && oa.len != PCRE2_UNSET {
                                    doc.push(format!(
                                        "{tag}: -70 must leave *outlengthptr UNSET, got {}",
                                        oa.len
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
            free_both(p, a, b, cca, ccb);
        }

        // Row 375 needs a specific witness: a case callout demanding
        // PCRE2_SIZE_MAX-1 code units makes `extra_needed` exceed
        // `~(PCRE2_SIZE)0 - buff_length` at pcre2_substitute.c:1752.
        // (ERRORS.md's own description — plain GLOBAL|OVERFLOW_LENGTH — cannot
        // be reached without a multi-exabyte subject.)
        let (a, b, cca, ccb) = compile_both(p, "(a)", 0, 0);
        let mca = (p.c.match_context_create)(ptr::null_mut());
        let mcb = (p.r.match_context_create)(ptr::null_mut());
        (p.c.set_substitute_case_callout)(mca, Some(case_cb), ptr::null_mut());
        (p.r.set_substitute_case_callout)(mcb, Some(case_cb), ptr::null_mut());
        let sb = padded(b"ax");
        let rb = padded(b"\\Uabc");
        *ptr::addr_of_mut!(CASE_MODE) = 2;
        (*ptr::addr_of_mut!(CASE_LOG)).clear();
        let so = PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
        let oa = call(&p.c, a, sb.as_ptr(), 2, 0, so, ptr::null_mut(), mca, rb.as_ptr(), 5,
            32, false);
        (*ptr::addr_of_mut!(CASE_LOG)).clear();
        let ob = call(&p.r, b, sb.as_ptr(), 2, 0, so, ptr::null_mut(), mcb, rb.as_ptr(), 5,
            32, false);
        d.eq("rows [375] witness RC", oa.rc, ob.rc);
        d.eq("rows [375] witness OUTLENGTH", show_len(oa.len), show_len(ob.len));
        d.eq("rows [375] witness BUFFER", show(&oa.buf), show(&ob.buf));
        if oa.rc != PCRE2_ERROR_TOOLARGEREPLACE {
            doc.push(format!(
                "rows [375]: documented -70, C returned {} (len {})", oa.rc, show_len(oa.len)
            ));
        }
        // and row 374's witness
        *ptr::addr_of_mut!(CASE_MODE) = 1;
        (*ptr::addr_of_mut!(CASE_LOG)).clear();
        let oa = call(&p.c, a, sb.as_ptr(), 2, 0, PCRE2_SUBSTITUTE_EXTENDED, ptr::null_mut(),
            mca, rb.as_ptr(), 5, 32, false);
        (*ptr::addr_of_mut!(CASE_LOG)).clear();
        let ob = call(&p.r, b, sb.as_ptr(), 2, 0, PCRE2_SUBSTITUTE_EXTENDED, ptr::null_mut(),
            mcb, rb.as_ptr(), 5, 32, false);
        d.eq("rows [374] witness RC", oa.rc, ob.rc);
        d.eq("rows [374] witness OUTLENGTH", show_len(oa.len), show_len(ob.len));
        d.eq("rows [374] witness BUFFER", show(&oa.buf), show(&ob.buf));
        if oa.rc != PCRE2_ERROR_REPLACECASE {
            doc.push(format!("rows [374]: documented -69, C returned {}", oa.rc));
        }
        *ptr::addr_of_mut!(CASE_MODE) = 0;
        (p.c.match_context_free)(mca);
        (p.r.match_context_free)(mcb);
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    assert!(seen_69 > 0, "row 374 (-69) was never actually reached by the sweep");
    assert!(seen_70 > 0, "row 375 (-70) was never actually reached by the sweep");
    d.finish("pcre2_substitute.c rows 374 (PCRE2_ERROR_REPLACECASE) and 375 (TOOLARGEREPLACE)");
}

// ==================================== row 376: substitute_callout rejecting a match

/// Exact layout of `pcre2_substitute_callout_block` (pcre2.h:616-626). `version`
/// is a `uint32_t` followed by padding before the first pointer.
#[repr(C)]
struct SubstituteCalloutBlock {
    version: u32,
    input: Sptr,
    output: *const u8,
    output_offsets: [Sz; 2],
    ovector: *const Sz,
    oveccount: u32,
    subscount: u32,
}

static mut SCB_RET: c_int = 0;
static mut SCB_LOG: Vec<u8> = Vec::new();

unsafe extern "C" fn subs_cb(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let bl = &*(blk as *const SubstituteCalloutBlock);
    let log = &mut *ptr::addr_of_mut!(SCB_LOG);
    for v in [
        bl.version as u64,
        bl.oveccount as u64,
        bl.subscount as u64,
        bl.output_offsets[0] as u64,
        bl.output_offsets[1] as u64,
    ] {
        log.extend_from_slice(&v.to_le_bytes());
    }
    for i in 0..(2 * bl.oveccount as usize) {
        log.extend_from_slice(&(*bl.ovector.add(i) as u64).to_le_bytes());
    }
    let (s, e) = (bl.output_offsets[0], bl.output_offsets[1]);
    if e >= s && e < 1 << 20 {
        log.extend_from_slice(std::slice::from_raw_parts(bl.output.add(s), e - s));
    }
    *ptr::addr_of!(SCB_RET)
}

#[test]
fn substitute_callout_rejects() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        for (pat, subj) in [("a", &b"banana"[..]), ("(a)", &b"banana"[..]),
                            ("[aeiou]", &b"queueing"[..]), ("", &b"xy"[..])] {
            let (a, b, cca, ccb) = compile_both(p, pat, 0, 0);
            let sb = padded(subj);
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            d.eq(
                "set_substitute_callout rc",
                (p.c.set_substitute_callout)(mca, Some(subs_cb), ptr::null_mut()),
                (p.r.set_substitute_callout)(mcb, Some(subs_cb), ptr::null_mut()),
            );
            for rep in [&b"XY"[..], &b"Z"[..], &b""[..], &b"[$0]"[..]] {
                let rb = padded(rep);
                for ret in [0 as c_int, 1, -1, 99, -99, c_int::MAX, c_int::MIN] {
                    for so in [
                        0u32,
                        PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ] {
                        for cap in [1usize, 3, 64] {
                            *ptr::addr_of_mut!(SCB_RET) = ret;
                            (*ptr::addr_of_mut!(SCB_LOG)).clear();
                            let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, so,
                                ptr::null_mut(), mca, rb.as_ptr(), rep.len(), cap, false);
                            let loga = (*ptr::addr_of!(SCB_LOG)).clone();
                            (*ptr::addr_of_mut!(SCB_LOG)).clear();
                            let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, so,
                                ptr::null_mut(), mcb, rb.as_ptr(), rep.len(), cap, false);
                            let logb = (*ptr::addr_of!(SCB_LOG)).clone();
                            let tag = format!(
                                "rows [376] callout->{ret} pat={pat:?} subj={} rep={} \
                                 sopts={so:#x} cap={cap}",
                                show(subj), show(rep)
                            );
                            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                            d.eq(&format!("{tag} CALLOUT-LOG"), loga, logb);
                            // Documented: a non-zero callout return is NOT an
                            // error; only a real buffer problem can make it fail.
                            if oa.rc < 0 && oa.rc != PCRE2_ERROR_NOMEMORY {
                                doc.push(format!(
                                    "{tag}: a substitute callout return must not become an \
                                     error, C returned {}", oa.rc
                                ));
                            }
                        }
                    }
                }
            }
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
            free_both(p, a, b, cca, ccb);
        }
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c row 376 (substitute_callout returning non-zero / negative)");
}

// ========================== rows 324, 338: every option bit, individually

#[test]
fn substitute_option_bit_sweep() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    // Which single bits the C rejects, and with what. Everything else must be
    // accepted (some bits change the result, none may error).
    unsafe {
        let (a, b, cca, ccb) = compile_both(p, "a", 0, 0);
        let sb = padded(b"aXa");
        let rb = padded(b"Z");
        for i in 0..32u32 {
            let so = 1u32 << i;
            let oa = call(&p.c, a, sb.as_ptr(), 3, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), 1, 64, false);
            let ob = call(&p.r, b, sb.as_ptr(), 3, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), 1, 64, false);
            let tag = format!("rows [324,338] single option bit {i} ({so:#x})");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            // Documented classification.
            let expect: Option<c_int> = match so {
                PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD => Some(PCRE2_ERROR_BADOPTION), // row 324
                PCRE2_DFA_RESTART | PCRE2_DFA_SHORTEST => Some(PCRE2_ERROR_BADOPTION), // row 338
                PCRE2_SUBSTITUTE_MATCHED => Some(PCRE2_ERROR_NULL),                    // row 327
                0x0008_0000 | 0x0010_0000 | 0x0020_0000 | 0x0040_0000 | 0x0080_0000
                | 0x0100_0000 | 0x0200_0000 | 0x0400_0000 | 0x0800_0000 | 0x1000_0000 =>
                    Some(PCRE2_ERROR_BADOPTION),                                       // row 338
                _ => None,
            };
            match expect {
                Some(e) if oa.rc != e => doc.push(format!("{tag}: expected {e}, C returned {}", oa.rc)),
                None if oa.rc < 0 => doc.push(format!("{tag}: unexpected error {}", oa.rc)),
                _ => {}
            }
        }
        // ... and everything at once
        for so in [u32::MAX, u32::MAX & !PCRE2_SUBSTITUTE_MATCHED, 0xFFFF_0000, 0x0000_FFFF] {
            let oa = call(&p.c, a, sb.as_ptr(), 3, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), 1, 64, false);
            let ob = call(&p.r, b, sb.as_ptr(), 3, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), 1, 64, false);
            let tag = format!("rows [324,338] options={so:#x}");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            if oa.rc >= 0 {
                doc.push(format!("{tag}: a wholesale option mask should be rejected, got {}", oa.rc));
            }
        }
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 324, 338 (all 32 option bits swept individually)");
}

// ================== rows 348, 351: replacements that have to be built at runtime

#[test]
fn substitute_dynamic_replacements() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Vec::new();
    unsafe {
        // rows: [348] — read_name_subst rejects a name longer than MAX_NAME_SIZE.
        // NOTE ERRORS.md says "longer than 32 (MAX_NAME_SIZE in this file)"; the
        // real limit is MAX_NAME_SIZE == 128 (config.h:223), so its example
        // (35 `a`s) is accepted, and 129 is the first rejected length.
        let (a, b, cca, ccb) = compile_both(p, "(?<a>x)", 0, 0);
        let sb = padded(b"x");
        for n in [1usize, 32, 127, 128, 129, 200] {
            let mut rep = b"${".to_vec();
            rep.extend(std::iter::repeat(b'a').take(n));
            rep.push(b'}');
            let rb = padded(&rep);
            let so = PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY;
            let oa = call(&p.c, a, sb.as_ptr(), 1, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), rep.len(), 512, false);
            let ob = call(&p.r, b, sb.as_ptr(), 1, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), rep.len(), 512, false);
            let tag = format!("rows [348] name of {n} code units");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            let want = if n > 128 { PCRE2_ERROR_BADREPLACEMENT } else { 1 };
            if oa.rc != want {
                doc.push(format!("{tag}: expected {want}, C returned {}", oa.rc));
            }
            if want == PCRE2_ERROR_BADREPLACEMENT && oa.len != rep.len() - 1 {
                doc.push(format!(
                    "{tag}: PTREXIT offset should be {} (just before the closing brace), got {}",
                    rep.len() - 1, show_len(oa.len)
                ));
            }
        }
        free_both(p, a, b, cca, ccb);

        // rows: [351] — more than PTR_STACK_SIZE/2 == 10 nested ${name:-...}.
        let (a, b, cca, ccb) = compile_both(p, "a(x)?", 0, 0);
        let sb = padded(b"a");
        for levels in [1usize, 9, 10, 11, 12, 20] {
            let mut rep = Vec::new();
            for _ in 0..levels {
                rep.extend_from_slice(b"${1:-");
            }
            rep.push(b'x');
            for _ in 0..levels {
                rep.push(b'}');
            }
            let rb = padded(&rep);
            let so = PCRE2_SUBSTITUTE_EXTENDED;
            let oa = call(&p.c, a, sb.as_ptr(), 1, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), rep.len(), 512, false);
            let ob = call(&p.r, b, sb.as_ptr(), 1, 0, so, ptr::null_mut(), ptr::null_mut(),
                rb.as_ptr(), rep.len(), 512, false);
            let tag = format!("rows [351] {levels} nested ${{1:-...}}");
            d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
            d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
            d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
            let want = if levels > 10 { PCRE2_ERROR_BADREPLACEMENT } else { 1 };
            if oa.rc != want {
                doc.push(format!("{tag}: expected {want}, C returned {}", oa.rc));
            }
        }
        free_both(p, a, b, cca, ccb);
    }
    assert!(doc.is_empty(), "ERRORS.md disagrees with the C:\n{}", doc.join("\n"));
    d.finish("pcre2_substitute.c rows 348 (over-long name) and 351 (nesting stack full)");
}

// ==================== rows 342, 343, 377: not reachable in this build

// Row 342 — PCRE2_ERROR_INTERNAL_DUPMATCH (-65), pcre2_substitute.c:1014-1022.
//   Unreachable: the branch is an internal invariant guarded by
//   PCRE2_DEBUG_UNREACHABLE() and it fires only if the global loop fails to
//   advance. pcre2_next_match() always advances ovector[1] or produces the
//   single permitted "empty match after a non-empty one at the same position",
//   which is exactly what the condition whitelists. There is no public input
//   that reaches it, so the nearest reachable behaviour — every shape of
//   non-advancing / empty-match global substitution — is compared instead.
// Row 343 — PCRE2_ERROR_TOOMANYREPLACE (-61), pcre2_substitute.c:1030-1035.
//   Unreachable here: it needs subs == INT_MAX, i.e. 2147483647 completed
//   substitutions, which requires a subject of more than 2 GiB. The nearest
//   reachable behaviour (long global substitutions with a correct subs count) is
//   compared instead.
// Row 377 — PCRE2_ERROR_NOUNIQUESUBSTRING (-50).
//   Never returned: pcre2_substitute calls pcre2_substring_nametable_scan with
//   non-NULL firstptr/lastptr and that function only returns -50 when
//   firstptr == NULL (pcre2_substring.c:517). Asserted below over duplicate-name
//   patterns, which is the only way -50 could ever arise.
#[test]
fn substitute_unreachable_rows() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // rows: [342] — global loops that repeatedly match empty strings.
        for (pat, copts, subj) in [
            ("x*", 0u32, &b"axbxc"[..]),
            ("", 0, &b"abc"[..]),
            ("\\b", 0, &b"ab cd"[..]),
            ("a*", 0, &b"aaa"[..]),
            ("(?=a)", 0, &b"aaa"[..]),
            ("$", PCRE2_MULTILINE, &b"a\nb\n"[..]),
            ("^", PCRE2_MULTILINE, &b"a\nb\n"[..]),
            ("\\K", 0, &b"abc"[..]),
            ("(?:)", 0, &b""[..]),
            (".*", 0, &b"abc"[..]),
        ] {
            let (a, b, cca, ccb) = compile_both(p, pat, copts, 0);
            let sb = padded(subj);
            for rep in [&b"-"[..], &b""[..], &b"[$0]"[..]] {
                let rb = padded(rep);
                for so in [
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_NOTEMPTY,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_NOTEMPTY_ATSTART,
                ] {
                    let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, so, ptr::null_mut(),
                        ptr::null_mut(), rb.as_ptr(), rep.len(), 128, false);
                    let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, so, ptr::null_mut(),
                        ptr::null_mut(), rb.as_ptr(), rep.len(), 128, false);
                    let tag = format!(
                        "rows [342,343] nearest reachable: global pat={pat:?} subj={} rep={} \
                         sopts={so:#x}", show(subj), show(rep)
                    );
                    d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                    d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                    d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                    assert_ne!(
                        oa.rc, PCRE2_ERROR_INTERNAL_DUPMATCH,
                        "{tag}: row 342 is supposed to be unreachable but the C hit it"
                    );
                    assert_ne!(
                        oa.rc, PCRE2_ERROR_TOOMANYREPLACE,
                        "{tag}: row 343 is supposed to be unreachable but the C hit it"
                    );
                }
            }
            free_both(p, a, b, cca, ccb);
        }

        // rows: [343] — a long global substitution: subs must be counted
        // identically (the counter that row 343 overflows).
        {
            let (a, b, cca, ccb) = compile_both(p, "a", 0, 0);
            let subj: Vec<u8> = std::iter::repeat(b'a').take(5000).collect();
            let sb = padded(&subj);
            let rb = padded(b"bb");
            let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, PCRE2_SUBSTITUTE_GLOBAL,
                ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), 2, 20_000, false);
            let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, PCRE2_SUBSTITUTE_GLOBAL,
                ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), 2, 20_000, false);
            d.eq("rows [343] 5000 substitutions RC", oa.rc, ob.rc);
            d.eq("rows [343] 5000 substitutions OUTLENGTH", oa.len, ob.len);
            d.eq("rows [343] 5000 substitutions BUFFER", show(&oa.buf), show(&ob.buf));
            assert_eq!(oa.rc, 5000, "the substitution counter itself must be right");
            free_both(p, a, b, cca, ccb);
        }

        // rows: [377] — duplicate group names can never produce -50.
        for (pat, copts) in [
            ("(?<n>a)|(?<n>b)", PCRE2_DUPNAMES),
            ("(?<n>a)(?<n>b)?", PCRE2_DUPNAMES),
            ("(?|(?<n>a)|(?<n>b))", PCRE2_DUPNAMES),
            ("(?<n>a)(x)(?<n>b)?", PCRE2_DUPNAMES),
        ] {
            let (a, b, cca, ccb) = compile_both(p, pat, copts, 0);
            for subj in [&b"a"[..], &b"b"[..], &b"ab"[..], &b"axb"[..], &b"zz"[..]] {
                let sb = padded(subj);
                for rep in [&b"${n}"[..], &b"$<n>"[..], &b"$n"[..], &b"\\g<n>"[..]] {
                    let rb = padded(rep);
                    for so in [
                        0u32,
                        PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
                            | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    ] {
                        let oa = call(&p.c, a, sb.as_ptr(), subj.len(), 0, so,
                            ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), rep.len(), 128, false);
                        let ob = call(&p.r, b, sb.as_ptr(), subj.len(), 0, so,
                            ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), rep.len(), 128, false);
                        let tag = format!(
                            "rows [377] dupnames pat={pat:?} subj={} rep={} sopts={so:#x}",
                            show(subj), show(rep)
                        );
                        d.eq(&format!("{tag} RC"), oa.rc, ob.rc);
                        d.eq(&format!("{tag} OUTLENGTH"), show_len(oa.len), show_len(ob.len));
                        d.eq(&format!("{tag} BUFFER"), show(&oa.buf), show(&ob.buf));
                        assert_ne!(
                            oa.rc, PCRE2_ERROR_NOUNIQUESUBSTRING,
                            "{tag}: row 377 says -50 is never returned by pcre2_substitute_8"
                        );
                    }
                }
            }
            free_both(p, a, b, cca, ccb);
        }
    }
    d.finish("pcre2_substitute.c rows 342, 343 (unreachable) and 377 (-50 never returned)");
}
