//! Phase B — every `pcre2_substring_*` entry point plus `pcre2_next_match_8`.
//!
//! CONFIGS.md rows 135-142, 179-180 · ERRORS.md rows 186-203, 257-260.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

// ------------------------------------------------------------------ helpers

const BUF: usize = 192;
/// How many bytes of the copy buffer we log (all writes stay well below this).
const BUFLOG: usize = 80;

/// NUL-terminated copy of a name.
fn zstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

unsafe fn info_u32(api: &Api, code: Code, what: u32) -> u32 {
    let mut v: u32 = 0;
    (api.pattern_info)(code, what, &mut v as *mut u32 as *mut c_void);
    v
}

/// Pre-fills the whole ovector with `PCRE2_UNSET` so that *no* substring
/// function can ever read an uninitialised slot (which would be
/// non-deterministic between the two libraries rather than a real divergence).
unsafe fn clear_ovector(api: &Api, md: MatchData) {
    let n = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    if !ov.is_null() {
        for i in 0..(2 * n) {
            *ov.add(i) = PCRE2_UNSET;
        }
    }
}

/// Logs the *whole* ovector. This is only sound because `clear_ovector` has
/// pre-filled every slot with `PCRE2_UNSET` beforehand, so slots the match
/// engine chose not to write still hold a defined, identical value.
unsafe fn log_full_ovector(api: &Api, md: MatchData, l: &mut Log) {
    let n = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    l.tag("ovall").u(n as u64);
    if !ov.is_null() {
        for i in 0..(2 * n) {
            l.u(*ov.add(i) as u64);
        }
    }
}

/// A test allocator so that `PCRE2_ERROR_NOMEMORY` is reachable on demand.
/// `ALLOC_FAIL` is process-wide, so any test that toggles it must hold
/// `ALLOC_LOCK` for its whole duration (libtest runs tests in parallel).
static mut ALLOC_FAIL: bool = false;
static ALLOC_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

unsafe extern "C" fn tmalloc(size: usize, _d: *mut c_void) -> *mut c_void {
    if ALLOC_FAIL {
        return std::ptr::null_mut();
    }
    let total = size + 16;
    let lay = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(lay);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    *(p as *mut usize) = total;
    p.add(16) as *mut c_void
}

unsafe extern "C" fn tfree(p: *mut c_void, _d: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = *(base as *mut usize);
    std::alloc::dealloc(
        base,
        std::alloc::Layout::from_size_align(total, 16).unwrap(),
    );
}

// ------------------------------------------------------------------ probes

/// rows 135-137 · ERRORS 186-193: `*_bynumber` for every plausible group
/// number, with `sizeptr` NULL/non-NULL and every buffer-size relationship.
unsafe fn probe_bynumber(api: &Api, md: MatchData, top: u32, l: &mut Log) {
    l.tag("byn");

    let mut nums: Vec<u32> = (0..=(top + 2)).collect();
    nums.push(0x1_0000);
    nums.push(u32::MAX - 1);
    nums.push(u32::MAX);
    for n in nums {
        // --- length, with and without sizeptr
        let mut sz: Sz = 0xDEAD_BEEF;
        let rc = (api.substring_length_bynumber)(md, n, &mut sz);
        l.u(n as u64).i(rc as i64).u(sz as u64);
        let rc_nullsz = (api.substring_length_bynumber)(md, n, std::ptr::null_mut());
        l.i(rc_nullsz as i64);

        // --- copy, exact / oversized / undersized
        let base: Sz = if rc == 0 && sz < 64 { sz } else { 3 };
        for cap in [0 as Sz, 1, base, base + 1, base + 2, 64] {
            let mut buf = [0xAAu8; BUF];
            let mut cs: Sz = cap;
            let rcc = (api.substring_copy_bynumber)(md, n, buf.as_mut_ptr(), &mut cs);
            l.u(cap as u64)
                .i(rcc as i64)
                .u(cs as u64)
                .b(&buf[..BUFLOG]);
        }

        // --- get + free
        let mut p: *mut u8 = std::ptr::null_mut();
        let mut gs: Sz = 0xDEAD;
        let rcg = (api.substring_get_bynumber)(md, n, &mut p, &mut gs);
        l.i(rcg as i64).u(gs as u64).i(p.is_null() as i64);
        if rcg == 0 && !p.is_null() {
            l.b(std::slice::from_raw_parts(p, gs + 1));
            (api.substring_free)(p);
        }
    }
    (api.substring_free)(std::ptr::null_mut());
}

/// row 140 · ERRORS 203: nametable_scan, with and without the out params.
/// Only *relative* entry indexes and the entry bytes are logged.
unsafe fn probe_nametable(api: &Api, code: Code, z: &[u8], l: &mut Log) {
    let nc = info_u32(api, code, INFO_NAMECOUNT) as usize;
    let nes = info_u32(api, code, INFO_NAMEENTRYSIZE) as usize;
    let mut nt: *const u8 = std::ptr::null();
    (api.pattern_info)(code, INFO_NAMETABLE, &mut nt as *mut _ as *mut c_void);

    let mut first: *const u8 = std::ptr::null();
    let mut last: *const u8 = std::ptr::null();
    let rc = (api.substring_nametable_scan)(code, z.as_ptr(), &mut first, &mut last);
    l.tag("nts").i(rc as i64).u(nc as u64).u(nes as u64);
    if rc > 0 && !nt.is_null() && nes > 0 && !first.is_null() && !last.is_null() {
        let fi = (first as usize - nt as usize) / nes;
        let li = (last as usize - nt as usize) / nes;
        l.u(fi as u64).u(li as u64);
        let mut i = fi;
        while i <= li && i < nc {
            l.b(std::slice::from_raw_parts(nt.add(i * nes), nes));
            i += 1;
        }
    }
    // firstptr == NULL => group-number query form (lastptr is then ignored).
    let mut dummy: *const u8 = std::ptr::null();
    let rcq = (api.substring_nametable_scan)(
        code,
        z.as_ptr(),
        std::ptr::null_mut(),
        &mut dummy,
    );
    let rcq2 = (api.substring_nametable_scan)(
        code,
        z.as_ptr(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    l.i(rcq as i64).i(rcq2 as i64).i(dummy.is_null() as i64);
}

/// `match_data->code` / `->matchedby` are only assigned once the match engine
/// reaches its "fill in fields that are always returned" point
/// (pcre2_match.c:8168, pcre2_dfa_match.c:3688). The early argument-validation
/// returns bail out before that, leaving those fields uninitialised — and
/// `pcre2_substring_*_byname` reads them *before* looking at `rc`, so calling
/// it after such an error is not a legal use of the API in either library.
fn md_code_is_defined(rc: c_int) -> bool {
    rc >= 0 || rc == ERR_NOMATCH || rc == ERR_PARTIAL
}

/// row 138 · ERRORS 194/195/203: the two `code`-only name lookups. These are
/// always safe because they take the compiled code directly.
unsafe fn probe_names_code(api: &Api, code: Code, names: &[&str], l: &mut Log) {
    l.tag("nmcode");
    for nm in names {
        let z = zstr(nm);
        l.b(nm.as_bytes());
        l.i((api.substring_number_from_name)(code, z.as_ptr()) as i64);
        probe_nametable(api, code, &z, l);
    }
}

/// row 139 · ERRORS 196-201: `*_byname` against the match data.
unsafe fn probe_byname(api: &Api, md: MatchData, names: &[&str], l: &mut Log) {
    l.tag("bynm");
    for nm in names {
        let z = zstr(nm);
        l.b(nm.as_bytes());

        let mut sz: Sz = 0xDEAD;
        let rc = (api.substring_length_byname)(md, z.as_ptr(), &mut sz);
        l.i(rc as i64).u(sz as u64);
        l.i((api.substring_length_byname)(md, z.as_ptr(), std::ptr::null_mut()) as i64);

        let base: Sz = if rc == 0 && sz < 64 { sz } else { 3 };
        for cap in [0 as Sz, 1, base, base + 1, 64] {
            let mut buf = [0xAAu8; BUF];
            let mut cs: Sz = cap;
            let rcc = (api.substring_copy_byname)(md, z.as_ptr(), buf.as_mut_ptr(), &mut cs);
            l.u(cap as u64)
                .i(rcc as i64)
                .u(cs as u64)
                .b(&buf[..BUFLOG]);
        }

        let mut p: *mut u8 = std::ptr::null_mut();
        let mut gs: Sz = 0xDEAD;
        let rcg = (api.substring_get_byname)(md, z.as_ptr(), &mut p, &mut gs);
        l.i(rcg as i64).u(gs as u64).i(p.is_null() as i64);
        if rcg == 0 && !p.is_null() {
            l.b(std::slice::from_raw_parts(p, gs + 1));
            (api.substring_free)(p);
        }
    }
}

/// row 141 · ERRORS 202: the complete substring list, with and without lengths.
unsafe fn probe_list(api: &Api, md: MatchData, l: &mut Log) {
    l.tag("list");
    let mut list: *mut *mut u8 = std::ptr::null_mut();
    let mut lens: *mut Sz = std::ptr::null_mut();
    let rc = (api.substring_list_get)(md, &mut list, &mut lens);
    l.i(rc as i64);
    if rc == 0 {
        l.i(list.is_null() as i64).i(lens.is_null() as i64);
        if !list.is_null() && !lens.is_null() {
            // Relative offset of the lengths vector inside the single block.
            l.u((lens as usize - list as usize) as u64);
            let mut i = 0usize;
            while i < 400 && !(*list.add(i)).is_null() {
                let p = *list.add(i);
                let ln = *lens.add(i);
                l.u(i as u64)
                    .u(ln as u64)
                    .b(std::slice::from_raw_parts(p, ln))
                    .u((p as usize - list as usize) as u64)
                    .b(&cstr(p));
                i += 1;
            }
            l.u(i as u64);
            (api.substring_list_free)(list);
        }
    }

    let mut list2: *mut *mut u8 = std::ptr::null_mut();
    let rc2 = (api.substring_list_get)(md, &mut list2, std::ptr::null_mut());
    l.i(rc2 as i64);
    if rc2 == 0 && !list2.is_null() {
        let mut i = 0usize;
        while i < 400 && !(*list2.add(i)).is_null() {
            let p = *list2.add(i);
            l.u(i as u64)
                .b(&cstr(p))
                .u((p as usize - list2 as usize) as u64);
            i += 1;
        }
        l.u(i as u64);
        (api.substring_list_free)(list2);
    }
    (api.substring_list_free)(std::ptr::null_mut());
}

/// rows 179 · ERRORS 257-260.
unsafe fn probe_next_match(api: &Api, md: MatchData, l: &mut Log) {
    let mut so: Sz = 0xDEAD_BEEF;
    let mut op: u32 = 0xDEAD_BEEF;
    let rc = (api.next_match)(md, &mut so, &mut op);
    l.tag("nm").i(rc as i64).u(so as u64).u(op as u64);
}

// ------------------------------------------------------------------ driver

#[derive(Clone, Copy)]
enum How {
    Interp,
    Dfa,
}

#[allow(clippy::too_many_arguments)]
unsafe fn one_case(
    api: &Api,
    pat: &[u8],
    copts: u32,
    subj: &[u8],
    oveccount: u32,
    mopts: u32,
    start: Sz,
    names: &[&str],
    how: How,
    l: &mut Log,
) {
    let code = compile_logged(api, pat, pat.len(), copts, std::ptr::null_mut(), l);
    if code.is_null() {
        return;
    }
    let top = info_u32(api, code, INFO_CAPTURECOUNT);
    let md = (api.match_data_create)(oveccount, std::ptr::null_mut());
    if md.is_null() {
        (api.code_free)(code);
        return;
    }
    clear_ovector(api, md);
    let rc = match how {
        How::Interp => (api.do_match)(
            code,
            subj.as_ptr(),
            subj.len(),
            start,
            mopts,
            md,
            std::ptr::null_mut(),
        ),
        How::Dfa => {
            let mut ws = [0i32; 512];
            (api.dfa_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                start,
                mopts,
                md,
                std::ptr::null_mut(),
                ws.as_mut_ptr(),
                512,
            )
        }
    };
    log_match_result_full(api, code, md, rc, l);
    log_full_ovector(api, md, l);
    probe_bynumber(api, md, top, l);
    probe_names_code(api, code, names, l);
    l.tag("mdok").i(md_code_is_defined(rc) as i64);
    if md_code_is_defined(rc) {
        probe_byname(api, md, names, l);
    }
    probe_list(api, md, l);
    probe_next_match(api, md, l);
    (api.match_data_free)(md);
    (api.code_free)(code);
}

#[allow(clippy::too_many_arguments)]
fn run_case(
    label: &str,
    pat: &[u8],
    copts: u32,
    subj: &[u8],
    oveccount: u32,
    mopts: u32,
    start: Sz,
    names: &[&str],
    how: How,
) {
    if std::env::var_os("PCRE2_TRACE").is_some() {
        eprintln!("CASE {label}");
    }
    diff(label, |api| {
        let mut l = Log::new();
        unsafe {
            one_case(
                api, pat, copts, subj, oveccount, mopts, start, names, how, &mut l,
            )
        };
        l
    });
}

// ------------------------------------------------------------------ corpora

/// Patterns with capturing groups, covering set/unset/empty/nested groups.
const GRP_PATTERNS: &[&str] = &[
    "a",
    "(a)",
    "(a)(b)",
    "(a)(b)(c)",
    "((a)(b))",
    "(a)?(b)",
    "(a)|(b)",
    "(a)(?:b)(c)",
    r"(\w+)\s+(\w+)",
    r"(\d{2})-(\d{2})",
    "(a+)(b*)(c?)",
    "(?:(x)|(y))z",
    "(.)(.)(.)",
    "^(.*)$",
    r"(a)\1",
    "(a(b(c(d))))",
    "()",
    "()()()",
    "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)",
    r"\K(a)",
    "(?=(a))b",
    "(?<=(a))b",
    "(abcd)",
    "(a)(b)?(c)",
];

/// Named-group patterns; those with duplicates need `PCRE2_DUPNAMES`.
const NAMED_PATTERNS: &[&str] = &[
    "(?<a>x)",
    "(?<a>x)(?<b>y)",
    "(?<a>x)?(?<b>y)?",
    "(?<name1>a)(?<name2>b)(?<name3>c)",
    "(?<b>x)(?<a>y)",
    "(?<z>1)(?<y>2)(?<x>3)",
    "(?<a>a)(?<aa>b)(?<aaa>c)",
    "(?<year>[0-9]{4})-(?<mon>[0-9]{2})",
    "(?<e>)",
    "x(?<a>y)?",
];

/// Duplicate-name patterns (compiled with `PCRE2_DUPNAMES`).
const DUP_PATTERNS: &[&str] = &[
    "(?<dup>a)|(?<dup>b)",
    "(?<dup>a)|(?<dup>b)|(?<dup>c)",
    "(?<x>a)(?<y>b)|(?<x>c)(?<y>d)",
    "(?J)(?<n>a)|(?<n>b)",
    "z(?<dup>a)?(?<dup>b)?",
    "(?<dup>a)?(?<dup>b)?(?<dup>c)?",
    "(?<one>1)(?<dup>a)|(?<one>2)(?<dup>b)",
];

const NAMES_PLAIN: &[&str] = &["a", "n", "nope", ""];
const NAMES_FULL: &[&str] = &[
    "a", "b", "aa", "aaa", "x", "y", "z", "n", "e", "dup", "one", "name1", "name3", "year",
    "mon", "nope", "", "A",
];

/// Short subjects (< 64 bytes) so every copy buffer is provably large enough.
const SUBJ: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "abc",
    "abcd",
    "xyz",
    "xy",
    "1234-56",
    "hello world",
    "aabbcc",
    "a\0b",
    "\r\n",
    "a\nb",
    "z",
    "ac",
];

// ------------------------------------------------------------- rows 135-137

#[test]
fn substring_bynumber_matrix() {
    for (pi, p) in GRP_PATTERNS.iter().enumerate() {
        for (si, s) in SUBJ.iter().enumerate() {
            for ov in [0u32, 1, 2, 3, 8] {
                run_case(
                    &format!("byn pat[{pi}]={p:?} subj[{si}]={s:?} ov={ov}"),
                    p.as_bytes(),
                    0,
                    s.as_bytes(),
                    ov,
                    0,
                    0,
                    NAMES_PLAIN,
                    How::Interp,
                );
            }
        }
    }
}

// ------------------------------------------------------------- rows 138-140

#[test]
fn substring_byname_unique() {
    for (pi, p) in NAMED_PATTERNS.iter().enumerate() {
        for (si, s) in SUBJ.iter().enumerate() {
            for ov in [1u32, 2, 4] {
                run_case(
                    &format!("bynm pat[{pi}]={p:?} subj[{si}]={s:?} ov={ov}"),
                    p.as_bytes(),
                    0,
                    s.as_bytes(),
                    ov,
                    0,
                    0,
                    NAMES_FULL,
                    How::Interp,
                );
            }
        }
    }
}

#[test]
fn substring_byname_duplicates() {
    for (pi, p) in DUP_PATTERNS.iter().enumerate() {
        for (si, s) in SUBJ.iter().enumerate() {
            // ov == 1 keeps every named group *outside* the ovector, which is
            // the only way to see PCRE2_ERROR_UNAVAILABLE from *_byname.
            for ov in [1u32, 2, 3, 8] {
                run_case(
                    &format!("dup pat[{pi}]={p:?} subj[{si}]={s:?} ov={ov}"),
                    p.as_bytes(),
                    PCRE2_DUPNAMES,
                    s.as_bytes(),
                    ov,
                    0,
                    0,
                    NAMES_FULL,
                    How::Interp,
                );
            }
        }
    }
    // Without DUPNAMES the same patterns must fail to compile identically.
    for (pi, p) in DUP_PATTERNS.iter().enumerate() {
        run_case(
            &format!("dup-nodup pat[{pi}]={p:?}"),
            p.as_bytes(),
            0,
            b"ab",
            4,
            0,
            0,
            NAMES_FULL,
            How::Interp,
        );
    }
}

// -------------------------------------------------------------- row 142 (a)

#[test]
fn substring_after_partial() {
    let pats: &[&str] = &[
        "abcd",
        "(a)(b)(c)(d)",
        "(?<a>ab)(?<b>cd)",
        r"\d{4}",
        "(a)(b)?(c)(d)",
        "xyz(a)",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for (si, s) in ["", "a", "ab", "abc", "12", "123", "xy"].iter().enumerate() {
            for mopts in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                for ov in [1u32, 2, 8] {
                    run_case(
                        &format!("partial pat[{pi}]={p:?} subj[{si}]={s:?} m={mopts:#x} ov={ov}"),
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        ov,
                        mopts,
                        0,
                        NAMES_FULL,
                        How::Interp,
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------- row 142 (b)

#[test]
fn substring_after_dfa_match() {
    for (pi, p) in GRP_PATTERNS.iter().enumerate() {
        for (si, s) in SUBJ.iter().enumerate() {
            for ov in [1u32, 2, 8] {
                for mopts in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_DFA_SHORTEST] {
                    run_case(
                        &format!("dfa pat[{pi}]={p:?} subj[{si}]={s:?} ov={ov} m={mopts:#x}"),
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        ov,
                        mopts,
                        0,
                        NAMES_PLAIN,
                        How::Dfa,
                    );
                }
            }
        }
    }
    // ERRORS 196: *_byname on DFA match data => PCRE2_ERROR_DFA_UFUNC (-41).
    for (pi, p) in NAMED_PATTERNS.iter().enumerate() {
        for (si, s) in ["", "x", "xy", "abc"].iter().enumerate() {
            run_case(
                &format!("dfa-named pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                0,
                s.as_bytes(),
                4,
                0,
                0,
                NAMES_FULL,
                How::Dfa,
            );
        }
    }
    for (pi, p) in DUP_PATTERNS.iter().enumerate() {
        run_case(
            &format!("dfa-dup pat[{pi}]={p:?}"),
            p.as_bytes(),
            PCRE2_DUPNAMES,
            b"ab",
            4,
            0,
            0,
            NAMES_FULL,
            How::Dfa,
        );
    }
}

// ---------------------------------------------------- row 142 (c): NOMATCH

#[test]
fn substring_after_nomatch() {
    let pats: &[&str] = &[
        "zzz",
        "(zzz)",
        "(?<a>zzz)",
        "(?<dup>q)|(?<dup>w)",
        r"^\d+$",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for (si, s) in ["", "a", "abc", "hello"].iter().enumerate() {
            for how in [How::Interp, How::Dfa] {
                for ov in [1u32, 4] {
                    run_case(
                        &format!(
                            "nomatch pat[{pi}]={p:?} subj[{si}]={s:?} ov={ov} dfa={}",
                            matches!(how, How::Dfa)
                        ),
                        p.as_bytes(),
                        PCRE2_DUPNAMES,
                        s.as_bytes(),
                        ov,
                        0,
                        0,
                        NAMES_FULL,
                        how,
                    );
                }
            }
        }
    }
}

// -------------------------------------------------- ERRORS rows 186-203

#[test]
fn substring_error_paths() {
    let _guard = ALLOC_LOCK.lock().unwrap();
    diff("substring_errors", |api| {
        let mut l = Log::new();
        unsafe {
            // ---- NOSUBSTRING / UNAVAILABLE / UNSET across ovector sizes.
            let pat = b"(a)(b)?(c)";
            let code = compile_logged(api, pat, pat.len(), 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let subj = b"ac";
            for ov in [1u32, 2, 3, 4, 8] {
                let md = (api.match_data_create)(ov, std::ptr::null_mut());
                clear_ovector(api, md);
                let rc = (api.do_match)(
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                );
                log_match_result_full(api, code, md, rc, &mut l);
                log_full_ovector(api, md, &mut l);
                for n in [0u32, 1, 2, 3, 4, 5, 100, u32::MAX] {
                    let mut sz: Sz = 0xDEAD;
                    l.tag("len")
                        .i((api.substring_length_bynumber)(md, n, &mut sz) as i64)
                        .u(sz as u64);
                    let mut buf = [0xAAu8; BUF];
                    let mut cs: Sz = 64;
                    l.tag("cp")
                        .i((api.substring_copy_bynumber)(md, n, buf.as_mut_ptr(), &mut cs) as i64)
                        .u(cs as u64)
                        .b(&buf[..BUFLOG]);
                    let mut p: *mut u8 = std::ptr::null_mut();
                    let mut gs: Sz = 0;
                    let rcg = (api.substring_get_bynumber)(md, n, &mut p, &mut gs);
                    l.tag("gt").i(rcg as i64).u(gs as u64);
                    if rcg == 0 && !p.is_null() {
                        (api.substring_free)(p);
                    }
                }
                (api.match_data_free)(md);
            }

            // ---- ERRORS 190: PCRE2_ERROR_INVALIDOFFSET (-67).
            // The ovector is public memory, so we can drive the "start/end
            // beyond the subject" branch that is otherwise unreachable.
            let md = (api.match_data_create)(4, std::ptr::null_mut());
            clear_ovector(api, md);
            let subj2 = b"abc";
            let rc = (api.do_match)(
                code,
                subj2.as_ptr(),
                subj2.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            log_match_result_full(api, code, md, rc, &mut l);
            log_full_ovector(api, md, &mut l);
            let ov = (api.get_ovector_pointer)(md);
            for (a, b) in [
                (4usize, 4usize),
                (0, 99),
                (99, 0),
                (3, 1),
                (2, 2),
                (1, 0),
                (usize::MAX - 1, 1),
            ] {
                *ov.add(0) = a;
                *ov.add(1) = b;
                let mut sz: Sz = 0xDEAD;
                l.tag("invoff")
                    .u(a as u64)
                    .u(b as u64)
                    .i((api.substring_length_bynumber)(md, 0, &mut sz) as i64)
                    .u(sz as u64);
                let mut buf = [0xAAu8; BUF];
                let mut cs: Sz = 64;
                l.i((api.substring_copy_bynumber)(md, 0, buf.as_mut_ptr(), &mut cs) as i64)
                    .u(cs as u64)
                    .b(&buf[..BUFLOG]);
            }
            (api.match_data_free)(md);
            (api.code_free)(code);

            // ---- ERRORS 194/195/203: name lookups.
            for (pat, opts) in [
                (&b"(?<a>x)(?<b>y)"[..], 0u32),
                (&b"(?<dup>a)|(?<dup>b)"[..], PCRE2_DUPNAMES),
                (&b"abc"[..], 0u32),
            ] {
                let code = compile_logged(api, pat, pat.len(), opts, std::ptr::null_mut(), &mut l);
                if code.is_null() {
                    continue;
                }
                for nm in ["a", "b", "dup", "nope", "", "A", "aa", "\u{7f}"] {
                    let z = zstr(nm);
                    l.tag("nfn")
                        .b(nm.as_bytes())
                        .i((api.substring_number_from_name)(code, z.as_ptr()) as i64);
                    probe_nametable(api, code, &z, &mut l);
                }
                (api.code_free)(code);
            }

            // ---- ERRORS 191/200: NOMEMORY from a too-small copy buffer, and
            // ERRORS 193/201/202: NOMEMORY from a failing allocator.
            let gc = (api.general_context_create)(Some(tmalloc), Some(tfree), std::ptr::null_mut());
            assert!(!gc.is_null());
            let pat = b"(?<a>ab)(?<b>cd)";
            let code = compile_logged(api, pat, pat.len(), 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let md = (api.match_data_create)(8, gc);
            assert!(!md.is_null());
            clear_ovector(api, md);
            let subj = b"abcd";
            let rc = (api.do_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            log_match_result_full(api, code, md, rc, &mut l);
            log_full_ovector(api, md, &mut l);

            ALLOC_FAIL = true;
            for n in [0u32, 1, 2] {
                let mut p: *mut u8 = std::ptr::null_mut();
                let mut gs: Sz = 0xDEAD;
                let rcg = (api.substring_get_bynumber)(md, n, &mut p, &mut gs);
                l.tag("nomem_get").i(rcg as i64).i(p.is_null() as i64);
                if rcg == 0 && !p.is_null() {
                    (api.substring_free)(p);
                }
            }
            for nm in ["a", "b"] {
                let z = zstr(nm);
                let mut p: *mut u8 = std::ptr::null_mut();
                let mut gs: Sz = 0xDEAD;
                let rcg = (api.substring_get_byname)(md, z.as_ptr(), &mut p, &mut gs);
                l.tag("nomem_getname").i(rcg as i64).i(p.is_null() as i64);
                if rcg == 0 && !p.is_null() {
                    (api.substring_free)(p);
                }
            }
            {
                let mut list: *mut *mut u8 = std::ptr::null_mut();
                let mut lens: *mut Sz = std::ptr::null_mut();
                let rcl = (api.substring_list_get)(md, &mut list, &mut lens);
                l.tag("nomem_list").i(rcl as i64);
                if rcl == 0 && !list.is_null() {
                    (api.substring_list_free)(list);
                }
                let mut list2: *mut *mut u8 = std::ptr::null_mut();
                let rcl2 = (api.substring_list_get)(md, &mut list2, std::ptr::null_mut());
                l.tag("nomem_list2").i(rcl2 as i64);
                if rcl2 == 0 && !list2.is_null() {
                    (api.substring_list_free)(list2);
                }
            }
            ALLOC_FAIL = false;

            // With the allocator working again everything must succeed.
            probe_bynumber(api, md, info_u32(api, code, INFO_CAPTURECOUNT), &mut l);
            probe_list(api, md, &mut l);
            (api.match_data_free)(md);
            (api.general_context_free)(gc);
            (api.code_free)(code);

            // ERRORS 250: NULL frees are no-ops.
            (api.substring_free)(std::ptr::null_mut());
            (api.substring_list_free)(std::ptr::null_mut());
            l.tag("nullfree").i(0);
        }
        l
    });
}

// ------------------------------------------------------- randomized coverage

#[test]
fn substring_random_patterns() {
    let mut rng = Rng::new(0x5B57_0001u64);
    for iter in 0..1500 {
        let pat = PatternGen::gen(&mut rng);
        let subj = gen_subject(&mut rng, false);
        if subj.len() > 40 {
            continue;
        }
        let ov = *rng.pick(&[0u32, 1, 2, 3, 8, 16]);
        let (copts, how) = match rng.below(5) {
            0 => (PCRE2_DUPNAMES, How::Interp),
            1 => (0, How::Dfa),
            2 => (PCRE2_CASELESS, How::Interp),
            3 => (PCRE2_NO_AUTO_CAPTURE, How::Interp),
            _ => (0, How::Interp),
        };
        let mopts = *rng.pick(&[0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_NOTEMPTY]);
        run_case(
            &format!("rand iter={iter} pat={pat:?} subj={subj:?} ov={ov} c={copts:#x} m={mopts:#x}"),
            pat.as_bytes(),
            copts,
            &subj,
            ov,
            mopts,
            0,
            NAMES_FULL,
            how,
        );
    }
}

#[test]
fn substring_random_corpus_patterns() {
    let mut rng = Rng::new(0x5B57_0002u64);
    let bsubs = byte_subjects();
    for iter in 0..2000 {
        let pat = *rng.pick(PATTERNS);
        let subj = rng.pick(&bsubs).clone();
        if subj.len() > 40 {
            continue;
        }
        let ov = *rng.pick(&[0u32, 1, 2, 4, 16]);
        let copts = *rng.pick(&[
            0u32,
            PCRE2_DUPNAMES,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_NO_UTF_CHECK,
            PCRE2_CASELESS,
            PCRE2_MULTILINE,
        ]);
        let mopts = *rng.pick(&[0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD]);
        let how = if rng.bool() { How::Interp } else { How::Dfa };
        let start = if rng.bool() { 0 } else { rng.below(subj.len() + 1) };
        run_case(
            &format!(
                "randc iter={iter} pat={pat:?} subj={subj:?} ov={ov} c={copts:#x} m={mopts:#x} s={start}"
            ),
            pat.as_bytes(),
            copts,
            &subj,
            ov,
            mopts,
            start,
            NAMES_PLAIN,
            how,
        );
    }
}

/// rows 135-142 across a compile-option × match-option cross-product, for both
/// the interpreter and the DFA engine. `PCRE2_COPY_MATCHED_SUBJECT` is included
/// because `substring_copy/get` read `match_data->subject`, which then points at
/// a private copy rather than at the caller's buffer.
#[test]
fn substring_option_crossproduct() {
    let pats: &[&str] = &[
        "(a)(b)?(c)",
        "(?<a>a)(?<b>b)?",
        "(?<dup>a)|(?<dup>B)",
        "(A)(b)",
        r"(\w)(\W)?",
        "(.)(.)?(.)?",
    ];
    let subs: &[&str] = &["", "abc", "AbC", "ab", "a", "aéb", "a\r\nb"];
    let copts_list = [
        0u32,
        PCRE2_CASELESS,
        PCRE2_DUPNAMES,
        PCRE2_DUPNAMES | PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_DUPNAMES,
        PCRE2_UTF | PCRE2_UCP | PCRE2_DUPNAMES,
        PCRE2_NO_AUTO_CAPTURE | PCRE2_DUPNAMES,
        PCRE2_MATCH_UNSET_BACKREF | PCRE2_DUPNAMES,
        PCRE2_ANCHORED | PCRE2_DUPNAMES,
        PCRE2_MULTILINE | PCRE2_DUPNAMES,
    ];
    let mopts_list = [
        0u32,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_NO_UTF_CHECK,
        PCRE2_COPY_MATCHED_SUBJECT,
        PCRE2_COPY_MATCHED_SUBJECT | PCRE2_PARTIAL_SOFT,
    ];
    for (pi, p) in pats.iter().enumerate() {
        for (si, s) in subs.iter().enumerate() {
            for copts in copts_list {
                for mopts in mopts_list {
                    for ov in [1u32, 3] {
                        for how in [How::Interp, How::Dfa] {
                            run_case(
                                &format!(
                                    "xprod p[{pi}] s[{si}] c={copts:#x} m={mopts:#x} ov={ov} dfa={}",
                                    matches!(how, How::Dfa)
                                ),
                                p.as_bytes(),
                                copts,
                                s.as_bytes(),
                                ov,
                                mopts,
                                0,
                                NAMES_FULL,
                                how,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 179-180

/// Single-step `pcre2_next_match` after every kind of previous outcome.
#[test]
fn next_match_single_step() {
    let cases: &[(&str, u32)] = &[
        ("a", 0),
        ("a*", 0),
        ("", 0),
        ("b*", 0),
        (".", 0),
        ("^", PCRE2_MULTILINE),
        ("$", PCRE2_MULTILINE),
        (r"\b", 0),
        ("abcd", 0),
        ("(?=a)", 0),
        (r"a\K", 0),
        (r"\Ka", 0),
        (r"(?=a\K)", 0),
        (r"(?:a\K)*", 0),
        ("x", 0),
        (r"\R", 0),
        (r".*", 0),
    ];
    let subs: &[&str] = &[
        "", "a", "aa", "ab", "b", "abc", "a\r\nb", "\r\n", "aéb", "é", "a\nb", "aaa",
    ];
    for (ci, (pat, copts)) in cases.iter().enumerate() {
        for (si, s) in subs.iter().enumerate() {
            for start in [0usize, 1] {
                if start > s.len() {
                    continue;
                }
                for mopts in [0u32, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART, PCRE2_PARTIAL_SOFT] {
                    diff(
                        &format!("nm1 c[{ci}]={pat:?} s[{si}]={s:?} st={start} m={mopts:#x}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let code = compile_logged(
                                    api,
                                    pat.as_bytes(),
                                    pat.len(),
                                    *copts,
                                    std::ptr::null_mut(),
                                    &mut l,
                                );
                                if code.is_null() {
                                    return l;
                                }
                                let md = (api.match_data_create_from_pattern)(
                                    code,
                                    std::ptr::null_mut(),
                                );
                                clear_ovector(api, md);
                                let rc = (api.do_match)(
                                    code,
                                    s.as_bytes().as_ptr(),
                                    s.len(),
                                    start,
                                    mopts,
                                    md,
                                    std::ptr::null_mut(),
                                );
                                log_match_result_full(api, code, md, rc, &mut l);
                log_full_ovector(api, md, &mut l);
                                // Repeated calls must be idempotent.
                                probe_next_match(api, md, &mut l);
                                probe_next_match(api, md, &mut l);
                                (api.match_data_free)(md);
                                (api.code_free)(code);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

/// A complete `pcre2_next_match`-driven global match loop.
fn global_loop(label: &str, pat: &[u8], copts: u32, newline: u32, subj: &[u8]) {
    diff(label, |api| {
        let mut l = Log::new();
        unsafe {
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            if newline != 0 {
                l.i((api.set_newline)(cc, newline) as i64);
            }
            let code = compile_logged(api, pat, pat.len(), copts, cc, &mut l);
            (api.compile_context_free)(cc);
            if code.is_null() {
                return l;
            }
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let mut off: Sz = 0;
            let mut opts: u32 = 0;
            let mut iter = 0u32;
            loop {
                clear_ovector(api, md);
                let rc = (api.do_match)(
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    off,
                    opts,
                    md,
                    std::ptr::null_mut(),
                );
                l.tag("it").u(iter as u64).u(off as u64).u(opts as u64);
                log_match_result_full(api, code, md, rc, &mut l);
                log_full_ovector(api, md, &mut l);
                if rc >= 0 {
                    // Row 141 interaction: the list must be consistent at
                    // every step of the loop.
                    probe_list(api, md, &mut l);
                }
                let mut nso: Sz = 0xDEAD_BEEF;
                let mut nopt: u32 = 0xDEAD_BEEF;
                let t = (api.next_match)(md, &mut nso, &mut nopt);
                l.tag("nm").i(t as i64).u(nso as u64).u(nopt as u64);
                if t == 0 {
                    break;
                }
                off = nso;
                opts = nopt;
                iter += 1;
                if iter > 120 {
                    l.tag("cap");
                    break;
                }
            }
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
        l
    });
}

#[test]
fn next_match_global_loops() {
    let pats: &[&str] = &[
        "a",
        "a*",
        "a*?",
        "",
        ".",
        r"\b",
        r"\R",
        "(a)|(b)",
        r"\w+",
        r"\s*",
        "^",
        "$",
        r"a\K",
        r"\Ka",
        "(?=a)",
        "x?",
        r"[\r\n]",
        r"\X",
        r"(?<n>a)|(?<n>b)",
    ];
    let ascii: &[&str] = &[
        "",
        "a",
        "aaa",
        "ab",
        "abcabc",
        "a\r\nb",
        "\r\n\r\n",
        "a\nb\nc",
        "  a  b  ",
        "aaaaaaaaaa",
    ];
    let utf: &[&str] = &["", "é", "ééé", "aéb", "日本語", "😀😀", "a\r\né", "é\nü"];

    for (pi, p) in pats.iter().enumerate() {
        // non-UTF, default newline
        for (si, s) in ascii.iter().enumerate() {
            global_loop(
                &format!("gl pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                0,
                0,
                s.as_bytes(),
            );
        }
        // CRLF newline convention
        for (si, s) in ascii.iter().enumerate() {
            global_loop(
                &format!("gl-crlf pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                PCRE2_MULTILINE,
                PCRE2_NEWLINE_CRLF,
                s.as_bytes(),
            );
            global_loop(
                &format!("gl-anycrlf pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                0,
                PCRE2_NEWLINE_ANYCRLF,
                s.as_bytes(),
            );
        }
        // UTF mode
        for (si, s) in utf.iter().enumerate() {
            global_loop(
                &format!("gl-utf pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                PCRE2_UTF,
                0,
                s.as_bytes(),
            );
            global_loop(
                &format!("gl-utf-crlf pat[{pi}]={p:?} subj[{si}]={s:?}"),
                p.as_bytes(),
                PCRE2_UTF,
                PCRE2_NEWLINE_CRLF,
                s.as_bytes(),
            );
        }
    }
    // Bytes that are not valid UTF-8, in UTF mode with NO_UTF_CHECK off/on.
    for (si, s) in byte_subjects().iter().enumerate() {
        if s.len() > 24 {
            continue;
        }
        for copts in [PCRE2_UTF, PCRE2_UTF | PCRE2_NO_UTF_CHECK, 0] {
            global_loop(
                &format!("gl-bytes subj[{si}]={s:?} c={copts:#x}"),
                b".",
                copts,
                0,
                s,
            );
        }
    }
}

/// ERRORS 257: `next_match` after an *error* return (not just NOMATCH).
#[test]
fn next_match_after_errors() {
    diff("nm_after_errors", |api| {
        let mut l = Log::new();
        unsafe {
            let pat = b"(a)(b)";
            let code = compile_logged(api, pat, pat.len(), 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let subj = b"ab";
            let md = (api.match_data_create)(4, std::ptr::null_mut());
            // Bad option bit, bad offset, bad option combination …
            for (off, mopts) in [
                (0usize, 0xFFFF_FFFFu32),
                (99usize, 0u32),
                (3usize, 0u32),
                (0usize, PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD),
                (0usize, PCRE2_DFA_RESTART),
            ] {
                clear_ovector(api, md);
                let rc = (api.do_match)(
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    off,
                    mopts,
                    md,
                    std::ptr::null_mut(),
                );
                l.tag("err").u(off as u64).u(mopts as u64).i(rc as i64);
                probe_next_match(api, md, &mut l);
            }
            // A real match first, then the DFA on the same block, then
            // next_match: the DFA result must drive it just the same.
            let mut ws = [0i32; 256];
            clear_ovector(api, md);
            let rc = (api.dfa_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
                ws.as_mut_ptr(),
                256,
            );
            log_match_result_full(api, code, md, rc, &mut l);
            log_full_ovector(api, md, &mut l);
            probe_next_match(api, md, &mut l);
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
        l
    });
}
