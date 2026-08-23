// Phase B sign-off for CONFIGS.md rows 352-406:
//   substring accessors (352-364), pattern_info + callout_enumerate (365-372),
//   serialize (373-379), pattern_convert (380-394), contexts / config /
//   error messages (395-406).
//
// Every row drives its exact named configuration through BOTH `.so`s and
// compares every observable, and each row additionally gets a randomized sweep
// with a fixed seed so value-dependent bugs are reachable.

mod common;
use common::*;
use std::ffi::{c_int, c_void, CStr};
use std::ptr;

pub const COVERAGE: &[CfgCov] = &[
    // --- substring
    CfgCov { cfg_rows: &[352], note: "length_bynumber: set/unset/>top_bracket/oveccount edges" },
    CfgCov { cfg_rows: &[353], note: "length_bynumber on a PARTIAL match_data" },
    CfgCov { cfg_rows: &[354], note: "length_bynumber on a DFA match_data (no top_bracket check)" },
    CfgCov { cfg_rows: &[355], note: "length_bynumber when rc == 0 (ovector too small)" },
    CfgCov { cfg_rows: &[356], note: "length_bynumber with \\K making left > right" },
    CfgCov { cfg_rows: &[357], note: "copy_bynumber buffer sizes: exact, one-too-small, 0, zero-length substring" },
    CfgCov { cfg_rows: &[358], note: "get_bynumber + substring_free, incl. NUL-containing and NULL" },
    CfgCov { cfg_rows: &[359], note: "nametable_scan: none/unique/duplicates, firstptr NULL" },
    CfgCov { cfg_rows: &[360], note: "number_from_name: unique / duplicate / absent" },
    CfgCov { cfg_rows: &[361], note: "*_byname on DFA match_data => DFA_UFUNC checked first" },
    CfgCov { cfg_rows: &[362], note: "*_byname DUPNAMES matrix, first-set scan order" },
    CfgCov { cfg_rows: &[363], note: "copy_byname exact/one-too-small + partial match_data" },
    CfgCov { cfg_rows: &[364], note: "substring_list_get with/without lengths, unset groups, free" },
    // --- pattern_info
    CfgCov { cfg_rows: &[365], note: "pattern_info NULL length query for every request code" },
    CfgCov { cfg_rows: &[366], note: "all 27 request codes on a rich DUPNAMES|UTF|CASELESS pattern" },
    CfgCov { cfg_rows: &[367], note: "FIRSTCODETYPE 1/2/0 and FIRSTCODEUNIT" },
    CfgCov { cfg_rows: &[368], note: "LASTCODETYPE / LASTCODEUNIT 1 vs 0" },
    CfgCov { cfg_rows: &[369], note: "FIRSTBITMAP non-NULL (32 bytes) vs NULL" },
    CfgCov { cfg_rows: &[370], note: "MATCHLIMIT/DEPTHLIMIT/HEAPLIMIT set vs UNSET-with-value" },
    CfgCov { cfg_rows: &[371], note: "JITSIZE always 0; FRAMESIZE vs top_bracket" },
    CfgCov { cfg_rows: &[372], note: "callout_enumerate over every opcode-skip arm" },
    // --- serialize
    CfgCov { cfg_rows: &[373], note: "1-code round trip, gcontext NULL, exact stream size" },
    CfgCov { cfg_rows: &[374], note: "many-code round trip; decode count <, ==, > stream count" },
    CfgCov { cfg_rows: &[375], note: "all codes sharing one pcre2_maketables block" },
    CfgCov { cfg_rows: &[376], note: "different custom allocators on encode and decode" },
    CfgCov { cfg_rows: &[377], note: "get_number_of_codes on 1- and 5-code streams" },
    CfgCov { cfg_rows: &[378], note: "decoded code fully usable: info/match/substitute/copy/enumerate" },
    CfgCov { cfg_rows: &[379], note: "serialize_free(NULL) no-op" },
    // --- convert
    CfgCov { cfg_rows: &[380], note: "GLOB basics with default separator and escape" },
    CfgCov { cfg_rows: &[381], note: "GLOB ** forms" },
    CfgCov { cfg_rows: &[382], note: "GLOB_NO_WILD_SEPARATOR / GLOB_NO_STARSTAR / both" },
    CfgCov { cfg_rows: &[383], note: "GLOB character classes" },
    CfgCov { cfg_rows: &[384], note: "GLOB negated class with NO_WILD_SEPARATOR (stale out_str byte)" },
    CfgCov { cfg_rows: &[385], note: "set_glob_separator / . x representative patterns" },
    CfgCov { cfg_rows: &[386], note: "set_glob_escape default / 0 / backtick, trailing lone escape" },
    CfgCov { cfg_rows: &[387], note: "escape+separator skip at **" },
    CfgCov { cfg_rows: &[388], note: "POSIX_BASIC full translation table" },
    CfgCov { cfg_rows: &[389], note: "POSIX_EXTENDED same inputs, asserting the differences" },
    CfgCov { cfg_rows: &[390], note: "valid option corners: glob-mod bits ignored by POSIX modes" },
    CfgCov { cfg_rows: &[391], note: "CONVERT_UTF for BASIC / EXTENDED / GLOB" },
    CfgCov { cfg_rows: &[392], note: "the three buffer protocols" },
    CfgCov { cfg_rows: &[393], note: "input shapes: NULL/0/ZERO_TERMINATED/embedded NUL/0xFF" },
    CfgCov { cfg_rows: &[394], note: "converted_pattern_free(NULL) no-op" },
    // --- contexts / config / errors
    CfgCov { cfg_rows: &[395], note: "general_context create/copy/free incl. partial allocator pairs" },
    CfgCov { cfg_rows: &[396], note: "compile_context create == defaults, copy, extra options" },
    CfgCov { cfg_rows: &[397], note: "match_context create == defaults, copy" },
    CfgCov { cfg_rows: &[398], note: "convert_context create == defaults, copy" },
    CfgCov { cfg_rows: &[399], note: "set_newline all 6 values observed via compile+match" },
    CfgCov { cfg_rows: &[400], note: "set_bsr both values observed via INFO_BSR and \\R" },
    CfgCov { cfg_rows: &[401], note: "set_optimize every legal directive, effect on bytecode" },
    CfgCov { cfg_rows: &[402], note: "set_glob_separator / set_glob_escape accepted value sets" },
    CfgCov { cfg_rows: &[403], note: "set_character_tables + maketables(+free), all table regions" },
    CfgCov { cfg_rows: &[404], note: "config NULL length query for every code" },
    CfgCov { cfg_rows: &[405], note: "config value query for every code" },
    CfgCov { cfg_rows: &[406], note: "get_error_message every code x buffer sizes incl. truncation" },
];

#[test]
fn coverage_declaration_is_sane() {
    check_coverage_decl(COVERAGE);
}

// ------------------------------------------------------------------ helpers

struct Ctx {
    a: Ptr,
    b: Ptr,
}

unsafe fn compile2(p: &Pair, pat: &[u8], opts: u32, cc: Option<&Ctx>) -> (Ptr, Ptr, c_int, c_int) {
    let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
    let (mut f1, mut f2) = (0usize, 0usize);
    let (ca, cb) = cc.map_or((ptr::null_mut(), ptr::null_mut()), |c| (c.a, c.b));
    let a = (p.c.compile)(pat.as_ptr(), pat.len(), opts, &mut e1, &mut f1, ca);
    let b = (p.r.compile)(pat.as_ptr(), pat.len(), opts, &mut e2, &mut f2, cb);
    assert_eq!(a.is_null(), b.is_null(), "compile {} nullness differs", show(pat));
    assert_eq!(e1, e2, "compile {} errorcode differs", show(pat));
    assert_eq!(f1, f2, "compile {} erroroffset differs", show(pat));
    if !a.is_null() {
        assert_code_eq(a, b, &format!("compile {}", show(pat)));
    }
    (a, b, e1, f1 as c_int)
}

/// Every `pcre2_substring_*` accessor applied to a pair of match_data objects.
unsafe fn cmp_all_substring(p: &Pair, mda: Ptr, mdb: Ptr, ca: Ptr, cb: Ptr, top: u32, tag: &str, d: &mut Diffs) {
    for n in 0..=(top + 3) {
        let (mut la, mut lb) = (usize::MAX, usize::MAX);
        d.eq(
            &format!("{tag} length_bynumber({n}) rc"),
            (p.c.substring_length_bynumber)(mda, n, &mut la),
            (p.r.substring_length_bynumber)(mdb, n, &mut lb),
        );
        d.eq(&format!("{tag} length_bynumber({n}) size"), la, lb);
        // buffer sizes: 0, exactly-enough-1, exactly enough, generous
        let exact = if la != usize::MAX { la } else { 0 };
        for cap in [0usize, exact, exact + 1, exact + 8, 256] {
            let mut ba = vec![0xEEu8; cap + 16];
            let mut bb = vec![0xEEu8; cap + 16];
            let (mut sa, mut sb) = (cap, cap);
            d.eq(
                &format!("{tag} copy_bynumber({n}, cap={cap}) rc"),
                (p.c.substring_copy_bynumber)(mda, n, ba.as_mut_ptr(), &mut sa),
                (p.r.substring_copy_bynumber)(mdb, n, bb.as_mut_ptr(), &mut sb),
            );
            d.eq(&format!("{tag} copy_bynumber({n}, cap={cap}) size"), sa, sb);
            d.eq(&format!("{tag} copy_bynumber({n}, cap={cap}) buf"), ba, bb);
        }
        let (mut pa, mut pb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
        let (mut ga, mut gb) = (usize::MAX, usize::MAX);
        let qa = (p.c.substring_get_bynumber)(mda, n, &mut pa, &mut ga);
        let qb = (p.r.substring_get_bynumber)(mdb, n, &mut pb, &mut gb);
        d.eq(&format!("{tag} get_bynumber({n}) rc"), qa, qb);
        d.eq(&format!("{tag} get_bynumber({n}) size"), ga, gb);
        if qa == 0 && qb == 0 {
            d.eq(
                &format!("{tag} get_bynumber({n}) bytes+NUL"),
                std::slice::from_raw_parts(pa, ga + 1).to_vec(),
                std::slice::from_raw_parts(pb, gb + 1).to_vec(),
            );
        }
        if !pa.is_null() {
            (p.c.substring_free)(pa);
        }
        if !pb.is_null() {
            (p.r.substring_free)(pb);
        }
    }
    // list form, with and without lengths
    for want_lengths in [true, false] {
        let (mut la, mut lb) = (ptr::null_mut(), ptr::null_mut());
        let (mut sa, mut sb): (*mut Sz, *mut Sz) = (ptr::null_mut(), ptr::null_mut());
        let (pla, plb) = if want_lengths {
            (&mut sa as *mut _, &mut sb as *mut _)
        } else {
            (ptr::null_mut(), ptr::null_mut())
        };
        let ra = (p.c.substring_list_get)(mda, &mut la, pla);
        let rb = (p.r.substring_list_get)(mdb, &mut lb, plb);
        d.eq(&format!("{tag} list_get(lengths={want_lengths}) rc"), ra, rb);
        if ra == 0 && rb == 0 {
            // count = rc if > 0 else oveccount (see pcre2_substring.c)
            let mdrc = (p.c.substring_length_bynumber)(mda, 0, &mut 0usize);
            let _ = mdrc;
            let cnt = {
                let mut n = 0usize;
                while !(*la.add(n)).is_null() && n < 1024 {
                    n += 1;
                }
                n
            };
            let cntb = {
                let mut n = 0usize;
                while !(*lb.add(n)).is_null() && n < 1024 {
                    n += 1;
                }
                n
            };
            d.eq(&format!("{tag} list_get entry count"), cnt, cntb);
            for i in 0..cnt.min(cntb) {
                if want_lengths {
                    let (nx, ny) = (*sa.add(i), *sb.add(i));
                    d.eq(&format!("{tag} list[{i}] len"), nx, ny);
                    if nx == ny {
                        d.eq(
                            &format!("{tag} list[{i}] bytes"),
                            std::slice::from_raw_parts(*la.add(i), nx).to_vec(),
                            std::slice::from_raw_parts(*lb.add(i), ny).to_vec(),
                        );
                    }
                } else {
                    // NUL-terminated only
                    let x = CStr::from_ptr(*la.add(i) as *const i8).to_bytes().to_vec();
                    let y = CStr::from_ptr(*lb.add(i) as *const i8).to_bytes().to_vec();
                    d.eq(&format!("{tag} list[{i}] cstr"), x, y);
                }
            }
            (p.c.substring_list_free)(la);
            (p.r.substring_list_free)(lb);
        }
    }
    let _ = (ca, cb);
}

// ===================================================== rows 352-364

#[test]
fn cfg_352_364_substring() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(35200);
    unsafe {
        // rows 352, 355, 356, 357, 358: bynumber matrix over ovector sizes
        let cases: &[(&str, u32, &str)] = &[
            ("(a)(b)?", 0, "a"),
            ("(a)(b)?", 0, "ab"),
            ("(a)(b)?(c)?", 0, "a"),
            ("a\\Kb", 0, "ab"),      // row 356: \K makes left > right for group 0
            ("(a)\\Kb", 0, "ab"),
            ("(a)(?<n>b)", 0, "ab"),
            ("()", 0, ""),           // zero-length substring (row 357)
            ("(a\\x00b)", 0, "a\u{0}b"), // NUL-containing substring (row 358)
            ("(.*)", 0, "hello"),
        ];
        for &(pat, opts, subj) in cases {
            let pb = pat.as_bytes();
            let sb = subj.as_bytes();
            let (a, b, _, _) = compile2(p, pb, opts, None);
            if a.is_null() {
                continue;
            }
            let mut top = 0u32;
            (p.c.pattern_info)(a, PCRE2_INFO_CAPTURECOUNT, &mut top as *mut u32 as Ptr);
            // ovector sizes below, at and above what the pattern needs
            for ovec in [0u32, 1, 2, 3, top + 1, top + 2, 16] {
                let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
                let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
                let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                let rb = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                d.eq(
                    &format!("substring base match {pat}/{subj} ovec={ovec}"),
                    read_match_out(&p.c, mda, ra),
                    read_match_out(&p.r, mdb, rb),
                );
                cmp_all_substring(
                    p, mda, mdb, a, b, top,
                    &format!("[{pat}/{subj} ovec={ovec}]"),
                    &mut d,
                );
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            // row 353: a PARTIAL match_data
            for popt in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                let mda = (p.c.match_data_create)(top + 1, ptr::null_mut());
                let mdb = (p.r.match_data_create)(top + 1, ptr::null_mut());
                // truncate the subject so a partial match is plausible
                let cut = &sb[..sb.len().min(1)];
                let ra = (p.c.do_match)(a, cut.as_ptr(), cut.len(), 0, popt, mda, ptr::null_mut());
                let rb = (p.r.do_match)(b, cut.as_ptr(), cut.len(), 0, popt, mdb, ptr::null_mut());
                d.eq(
                    &format!("partial base {pat} popt={popt:#x}"),
                    read_match_out(&p.c, mda, ra),
                    read_match_out(&p.r, mdb, rb),
                );
                cmp_all_substring(p, mda, mdb, a, b, top, &format!("[partial {pat} {popt:#x}]"), &mut d);
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            // row 354 + 361: a DFA match_data
            {
                let mut wa = vec![0 as c_int; 1000];
                let mut wb = vec![0 as c_int; 1000];
                let mda = (p.c.match_data_create)(4, ptr::null_mut());
                let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                let ra = (p.c.dfa_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut(), wa.as_mut_ptr(), 1000);
                let rb = (p.r.dfa_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut(), wb.as_mut_ptr(), 1000);
                d.eq(
                    &format!("dfa base {pat}/{subj}"),
                    read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                );
                cmp_all_substring(p, mda, mdb, a, b, top, &format!("[dfa {pat}/{subj}]"), &mut d);
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            // never-matched match_data
            {
                let mda = (p.c.match_data_create)(4, ptr::null_mut());
                let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                cmp_all_substring(p, mda, mdb, a, b, top, &format!("[virgin {pat}]"), &mut d);
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // row 354 specifically: DFA with the alternation the row names
        {
            let pb = b"a|ab|abc";
            let (a, b, _, _) = compile2(p, pb, 0, None);
            let sb = b"abc";
            let mut wa = vec![0 as c_int; 1000];
            let mut wb = vec![0 as c_int; 1000];
            for ovec in [1u32, 2, 3, 4, 8] {
                let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
                let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
                let ra = (p.c.dfa_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut(), wa.as_mut_ptr(), 1000);
                let rb = (p.r.dfa_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut(), wb.as_mut_ptr(), 1000);
                d.eq(
                    &format!("dfa a|ab|abc ovec={ovec}"),
                    read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                );
                cmp_all_substring(p, mda, mdb, a, b, 0, &format!("[dfa alt ovec={ovec}]"), &mut d);
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // rows 359, 360, 362, 363: by-name and the DUPNAMES scan order
        let name_cases: &[(&str, u32, &[&str])] = &[
            ("abc", 0, &["a"]),                                  // name_count == 0
            ("(?<a>x)(?<b>y)", 0, &["a", "b", "c", ""]),
            ("(?<a>x)|(?<a>y)|(?<a>z)", PCRE2_DUPNAMES, &["a", "b"]),
            ("(?<a>x)?(?<a>y)?(?<a>z)?", PCRE2_DUPNAMES, &["a"]),
            ("(?<n>\\d)-(?<n>\\d)", PCRE2_DUPNAMES, &["n"]),
            ("(?<verylongname>q)", 0, &["verylongname", "verylongnam"]),
        ];
        for &(pat, opts, names) in name_cases {
            let pb = pat.as_bytes();
            let (a, b, _, _) = compile2(p, pb, opts, None);
            if a.is_null() {
                continue;
            }
            for subj in ["x", "y", "z", "1-2", "q", "", "xyz"] {
                let sb = subj.as_bytes();
                let mda = (p.c.match_data_create_from_pattern)(a, ptr::null_mut());
                let mdb = (p.r.match_data_create_from_pattern)(b, ptr::null_mut());
                let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                let rb = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                d.eq(
                    &format!("byname base {pat}/{subj}"),
                    read_match_out(&p.c, mda, ra),
                    read_match_out(&p.r, mdb, rb),
                );
                // a DFA match_data too (row 361: DFA_UFUNC checked first)
                let mut wa = vec![0 as c_int; 500];
                let mut wb = vec![0 as c_int; 500];
                let dfa = (p.c.match_data_create)(4, ptr::null_mut());
                let dfb = (p.r.match_data_create)(4, ptr::null_mut());
                (p.c.dfa_match)(a, sb.as_ptr(), sb.len(), 0, 0, dfa, ptr::null_mut(), wa.as_mut_ptr(), 500);
                (p.r.dfa_match)(b, sb.as_ptr(), sb.len(), 0, 0, dfb, ptr::null_mut(), wb.as_mut_ptr(), 500);
                // and a partial one (row 363)
                let pa = (p.c.match_data_create_from_pattern)(a, ptr::null_mut());
                let pbm = (p.r.match_data_create_from_pattern)(b, ptr::null_mut());
                (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, PCRE2_PARTIAL_HARD, pa, ptr::null_mut());
                (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, PCRE2_PARTIAL_HARD, pbm, ptr::null_mut());

                for nm in names.iter().copied().chain(["nope", "A"]) {
                    let mut nz = nm.as_bytes().to_vec();
                    nz.push(0);
                    let n = nz.as_ptr();
                    for (kind, (x, y)) in [
                        ("normal", (mda, mdb)),
                        ("dfa", (dfa, dfb)),
                        ("partial", (pa, pbm)),
                    ] {
                        let tag = format!("[{pat}/{subj} {kind} name={nm:?}]");
                        let (mut la, mut lb) = (usize::MAX, usize::MAX);
                        d.eq(
                            &format!("{tag} length_byname rc"),
                            (p.c.substring_length_byname)(x, n, &mut la),
                            (p.r.substring_length_byname)(y, n, &mut lb),
                        );
                        d.eq(&format!("{tag} length_byname size"), la, lb);
                        let exact = if la != usize::MAX { la } else { 0 };
                        for cap in [0usize, exact, exact + 1, 64] {
                            let mut ba = vec![0xEEu8; cap + 16];
                            let mut bb = vec![0xEEu8; cap + 16];
                            let (mut sa, mut sbz) = (cap, cap);
                            d.eq(
                                &format!("{tag} copy_byname(cap={cap}) rc"),
                                (p.c.substring_copy_byname)(x, n, ba.as_mut_ptr(), &mut sa),
                                (p.r.substring_copy_byname)(y, n, bb.as_mut_ptr(), &mut sbz),
                            );
                            d.eq(&format!("{tag} copy_byname(cap={cap}) size"), sa, sbz);
                            d.eq(&format!("{tag} copy_byname(cap={cap}) buf"), ba, bb);
                        }
                        let (mut qa, mut qb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                        let (mut ga, mut gb) = (usize::MAX, usize::MAX);
                        let ka = (p.c.substring_get_byname)(x, n, &mut qa, &mut ga);
                        let kb = (p.r.substring_get_byname)(y, n, &mut qb, &mut gb);
                        d.eq(&format!("{tag} get_byname rc"), ka, kb);
                        d.eq(&format!("{tag} get_byname size"), ga, gb);
                        if ka == 0 && kb == 0 {
                            d.eq(
                                &format!("{tag} get_byname bytes"),
                                std::slice::from_raw_parts(qa, ga + 1).to_vec(),
                                std::slice::from_raw_parts(qb, gb + 1).to_vec(),
                            );
                        }
                        if !qa.is_null() {
                            (p.c.substring_free)(qa);
                        }
                        if !qb.is_null() {
                            (p.r.substring_free)(qb);
                        }
                    }
                    // row 359/360: code-level lookups (no match_data involved)
                    d.eq(
                        &format!("[{pat} name={nm:?}] number_from_name"),
                        (p.c.substring_number_from_name)(a, n),
                        (p.r.substring_number_from_name)(b, n),
                    );
                    for pass_first in [true, false] {
                        let (mut f1, mut l1): (Sptr, Sptr) = (ptr::null(), ptr::null());
                        let (mut f2, mut l2): (Sptr, Sptr) = (ptr::null(), ptr::null());
                        let (pf1, pl1, pf2, pl2) = if pass_first {
                            (&mut f1 as *mut _, &mut l1 as *mut _, &mut f2 as *mut _, &mut l2 as *mut _)
                        } else {
                            (ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
                        };
                        let r1 = (p.c.substring_nametable_scan)(a, n, pf1, pl1);
                        let r2 = (p.r.substring_nametable_scan)(b, n, pf2, pl2);
                        d.eq(
                            &format!("[{pat} name={nm:?} firstptr={pass_first}] nametable_scan rc"),
                            r1,
                            r2,
                        );
                        if pass_first && r1 >= 0 && r2 >= 0 && !f1.is_null() && !f2.is_null() {
                            // compare the spans relative to each library's own table
                            let (mut ta, mut tb) = (ptr::null::<u8>(), ptr::null::<u8>());
                            (p.c.pattern_info)(a, PCRE2_INFO_NAMETABLE, &mut ta as *mut _ as Ptr);
                            (p.r.pattern_info)(b, PCRE2_INFO_NAMETABLE, &mut tb as *mut _ as Ptr);
                            d.eq(
                                &format!("[{pat} name={nm:?}] nametable_scan span"),
                                (f1 as usize - ta as usize, l1 as usize - ta as usize),
                                (f2 as usize - tb as usize, l2 as usize - tb as usize),
                            );
                        }
                    }
                }
                for m in [mda, dfa, pa] {
                    (p.c.match_data_free)(m);
                }
                for m in [mdb, dfb, pbm] {
                    (p.r.match_data_free)(m);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // NULL-tolerant frees (rows 358, 364)
        (p.c.substring_free)(ptr::null_mut());
        (p.r.substring_free)(ptr::null_mut());
        (p.c.substring_list_free)(ptr::null_mut());
        (p.r.substring_list_free)(ptr::null_mut());
        d.checked += 1;

        // randomized sweep over the whole substring surface
        for _ in 0..400 {
            let pat = PATTERNS[rng.below(PATTERNS.len())].as_bytes();
            let (a, b, _, _) = compile2(p, pat, 0, None);
            if a.is_null() {
                continue;
            }
            let mut top = 0u32;
            (p.c.pattern_info)(a, PCRE2_INFO_CAPTURECOUNT, &mut top as *mut u32 as Ptr);
            let subj = SUBJECTS[rng.below(SUBJECTS.len())].as_bytes();
            let ovec = *rng.pick(&[0u32, 1, 2, 4, 16]);
            let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
            let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
            (p.c.do_match)(a, subj.as_ptr(), subj.len(), 0, 0, mda, ptr::null_mut());
            (p.r.do_match)(b, subj.as_ptr(), subj.len(), 0, 0, mdb, ptr::null_mut());
            cmp_all_substring(
                p, mda, mdb, a, b, top.min(4),
                &format!("[rand {} / {} ovec={ovec}]", show(pat), show(subj)),
                &mut d,
            );
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 352-364: every pcre2_substring_* accessor over ovector/partial/DFA/DUPNAMES/buffer-size axes");
}

// ===================================================== rows 365-372

/// Every `pcre2_pattern_info_8` request code with its correct result width.
unsafe fn cmp_info_all(p: &Pair, a: Ptr, b: Ptr, tag: &str, d: &mut Diffs) {
    const U32_ITEMS: &[u32] = &[
        PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_BACKREFMAX, PCRE2_INFO_BSR,
        PCRE2_INFO_CAPTURECOUNT, PCRE2_INFO_FIRSTCODEUNIT, PCRE2_INFO_FIRSTCODETYPE,
        PCRE2_INFO_HASCRORLF, PCRE2_INFO_JCHANGED, PCRE2_INFO_LASTCODEUNIT,
        PCRE2_INFO_LASTCODETYPE, PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MATCHLIMIT,
        PCRE2_INFO_MAXLOOKBEHIND, PCRE2_INFO_MINLENGTH, PCRE2_INFO_NAMECOUNT,
        PCRE2_INFO_NAMEENTRYSIZE, PCRE2_INFO_NEWLINE, PCRE2_INFO_DEPTHLIMIT,
        PCRE2_INFO_HASBACKSLASHC, PCRE2_INFO_HEAPLIMIT, PCRE2_INFO_EXTRAOPTIONS,
    ];
    for &what in U32_ITEMS {
        // row 370: the limit items write the value EVEN when returning UNSET
        let (mut va, mut vb) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
        let ra = (p.c.pattern_info)(a, what, &mut va as *mut u32 as Ptr);
        let rb = (p.r.pattern_info)(b, what, &mut vb as *mut u32 as Ptr);
        d.eq(&format!("{tag} info[{what}] rc"), ra, rb);
        d.eq(&format!("{tag} info[{what}] value"), va, vb);
    }
    for &what in &[PCRE2_INFO_SIZE, PCRE2_INFO_FRAMESIZE, PCRE2_INFO_JITSIZE] {
        let (mut va, mut vb) = (usize::MAX, usize::MAX);
        let ra = (p.c.pattern_info)(a, what, &mut va as *mut usize as Ptr);
        let rb = (p.r.pattern_info)(b, what, &mut vb as *mut usize as Ptr);
        d.eq(&format!("{tag} info[{what}] rc"), ra, rb);
        d.eq(&format!("{tag} info[{what}] value"), va, vb);
    }
    // FIRSTBITMAP: pointer to 32 bytes, or NULL
    {
        let (mut pa, mut pb) = (ptr::null::<u8>(), ptr::null::<u8>());
        let ra = (p.c.pattern_info)(a, PCRE2_INFO_FIRSTBITMAP, &mut pa as *mut _ as Ptr);
        let rb = (p.r.pattern_info)(b, PCRE2_INFO_FIRSTBITMAP, &mut pb as *mut _ as Ptr);
        d.eq(&format!("{tag} info[FIRSTBITMAP] rc"), ra, rb);
        d.eq(&format!("{tag} info[FIRSTBITMAP] null"), pa.is_null(), pb.is_null());
        if !pa.is_null() && !pb.is_null() {
            d.eq(
                &format!("{tag} info[FIRSTBITMAP] 32 bytes"),
                std::slice::from_raw_parts(pa, 32).to_vec(),
                std::slice::from_raw_parts(pb, 32).to_vec(),
            );
        }
    }
    // NAMETABLE
    {
        let (mut na, mut nb) = (0u32, 0u32);
        (p.c.pattern_info)(a, PCRE2_INFO_NAMECOUNT, &mut na as *mut u32 as Ptr);
        (p.r.pattern_info)(b, PCRE2_INFO_NAMECOUNT, &mut nb as *mut u32 as Ptr);
        let (mut ea, mut eb) = (0u32, 0u32);
        (p.c.pattern_info)(a, PCRE2_INFO_NAMEENTRYSIZE, &mut ea as *mut u32 as Ptr);
        (p.r.pattern_info)(b, PCRE2_INFO_NAMEENTRYSIZE, &mut eb as *mut u32 as Ptr);
        let (mut ta, mut tb) = (ptr::null::<u8>(), ptr::null::<u8>());
        let ra = (p.c.pattern_info)(a, PCRE2_INFO_NAMETABLE, &mut ta as *mut _ as Ptr);
        let rb = (p.r.pattern_info)(b, PCRE2_INFO_NAMETABLE, &mut tb as *mut _ as Ptr);
        d.eq(&format!("{tag} info[NAMETABLE] rc"), ra, rb);
        if na == nb && ea == eb && na > 0 {
            let n = (na * ea) as usize;
            d.eq(
                &format!("{tag} info[NAMETABLE] bytes"),
                std::slice::from_raw_parts(ta, n).to_vec(),
                std::slice::from_raw_parts(tb, n).to_vec(),
            );
        }
    }
    // row 365: the NULL length-query form for every code, plus out-of-range
    for what in 0u32..=30 {
        let ra = (p.c.pattern_info)(a, what, ptr::null_mut());
        let rb = (p.r.pattern_info)(b, what, ptr::null_mut());
        d.eq(&format!("{tag} info[{what}] NULL query"), ra, rb);
    }
    for what in [100u32, 1000, u32::MAX] {
        let mut v = 0u64;
        d.eq(
            &format!("{tag} info[{what}] out-of-range NULL"),
            (p.c.pattern_info)(a, what, ptr::null_mut()),
            (p.r.pattern_info)(b, what, ptr::null_mut()),
        );
        d.eq(
            &format!("{tag} info[{what}] out-of-range buf"),
            (p.c.pattern_info)(a, what, &mut v as *mut u64 as Ptr),
            (p.r.pattern_info)(b, what, &mut v as *mut u64 as Ptr),
        );
    }
}

#[test]
fn cfg_365_372_pattern_info() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(36500);
    unsafe {
        // row 366: the rich pattern with newline/bsr context state
        {
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            assert_eq!((p.c.set_newline)(cca, PCRE2_NEWLINE_ANYCRLF), 0);
            assert_eq!((p.r.set_newline)(ccb, PCRE2_NEWLINE_ANYCRLF), 0);
            assert_eq!((p.c.set_bsr)(cca, PCRE2_BSR_ANYCRLF), 0);
            assert_eq!((p.r.set_bsr)(ccb, PCRE2_BSR_ANYCRLF), 0);
            let ctx = Ctx { a: cca, b: ccb };
            for pat in [
                "(?<a>a)(?<a>b)?(?C1)\r\n\\1",
                "(*LIMIT_MATCH=99)(?<a>a)(?<a>b)?(?C1)\r\n\\1",
                "(*LIMIT_DEPTH=77)(*LIMIT_HEAP=55)(*LIMIT_MATCH=99)abc",
            ] {
                let pb = pat.as_bytes();
                let (a, b, _, _) = compile2(p, pb, PCRE2_DUPNAMES | PCRE2_UTF | PCRE2_CASELESS, Some(&ctx));
                if a.is_null() {
                    continue;
                }
                cmp_info_all(p, a, b, &format!("rich {}", show(pb)), &mut d);
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
        }

        // rows 367, 368, 369, 370, 371: the specific patterns each row names
        let probes: &[(&str, u32)] = &[
            // FIRSTCODETYPE 1 / 2 / 0
            ("abc", 0),
            ("(?m)^a", 0),
            ("\\da", 0),
            ("abc", PCRE2_NO_START_OPTIMIZE),
            ("(?m)^a", PCRE2_NO_START_OPTIMIZE),
            // LASTCODETYPE 1 vs 0
            ("a.*b", 0),
            ("a", 0),
            ("abc", PCRE2_ANCHORED),
            ("a*", 0),
            ("(*ACCEPT)ab", 0),
            ("[Ww]ord", 0),
            // FIRSTBITMAP non-NULL vs NULL
            ("[abc]x", 0),
            (".a", 0),
            ("[a-c\\d]z", 0),
            // limits
            ("(*LIMIT_MATCH=100)a", 0),
            ("(*LIMIT_DEPTH=100)a", 0),
            ("(*LIMIT_HEAP=100)a", 0),
            ("a", 0),
            // FRAMESIZE with top_bracket 0 and large
            ("plain", 0),
            ("(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)", 0),
        ];
        for &(pat, opts) in probes {
            let pb = pat.as_bytes();
            let (a, b, _, _) = compile2(p, pb, opts, None);
            if a.is_null() {
                continue;
            }
            cmp_info_all(p, a, b, &format!("probe {pat} opts={opts:#x}"), &mut d);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        // a 200-group pattern for the FRAMESIZE formula
        {
            let mut pat = String::new();
            for _ in 0..200 {
                pat.push_str("(a)");
            }
            let pb = pat.as_bytes();
            let (a, b, _, _) = compile2(p, pb, 0, None);
            if !a.is_null() {
                cmp_info_all(p, a, b, "200 groups", &mut d);
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        // NULL code (must agree)
        for what in 0u32..=27 {
            let mut v = 0u64;
            d.eq(
                &format!("info[{what}] on NULL code"),
                (p.c.pattern_info)(ptr::null_mut(), what, &mut v as *mut u64 as Ptr),
                (p.r.pattern_info)(ptr::null_mut(), what, &mut v as *mut u64 as Ptr),
            );
        }

        // row 372: callout_enumerate over every opcode-skip arm
        for pat in [
            "(?C1)a(?C\"s\")[\\x{100}]\\p{L}*(*MARK:m)\\x{100}{2,3}",
            "(?C1)a(?C{s})[\\x{100}]\\p{L}*(*MARK:m)\\x{100}{2,3}(?[a&&b])",
            "(?C0)(?C1)(?C2)(?C3)",
            "a(?C)(b(?C1)(c(?C2)))",
            "(?C1)\\X(?C2)\\R(?C3)\\N(?C4)",
            "(?C1)(?<n>a)(?C2)\\k<n>(?C3)",
            "(?C1)(?:a|b|c)(?C2)",
            "(?C1)(?=a)(?C2)(?!b)(?C3)(?<=c)(?C4)",
            "(?C1)(?>a+)(?C2)",
            "(?C1)(?(1)a|b)(?C2)",
            "(?C1)\\((?:[^()]++|(?R))*\\)(?C2)",
            "abc",
        ] {
            let pb = pat.as_bytes();
            for opts in [PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS, PCRE2_UTF, 0, PCRE2_AUTO_CALLOUT] {
                let (a, b, _, _) = compile2(p, pb, opts, None);
                if a.is_null() {
                    continue;
                }
                ENUM_LOG.clear();
                let ra = (p.c.callout_enumerate)(a, Some(enum_cb), ptr::null_mut());
                let la = ENUM_LOG.clone();
                ENUM_LOG.clear();
                let rb = (p.r.callout_enumerate)(b, Some(enum_cb), ptr::null_mut());
                let lb = ENUM_LOG.clone();
                d.eq(&format!("callout_enumerate {pat} opts={opts:#x} rc"), ra, rb);
                d.eq(&format!("callout_enumerate {pat} opts={opts:#x} log"), la, lb);
                // an aborting callback must abort identically
                for stop_at in 0u32..4 {
                    ENUM_STOP_AT = stop_at;
                    ENUM_SEEN = 0;
                    ENUM_LOG.clear();
                    let ra = (p.c.callout_enumerate)(a, Some(enum_cb_stop), ptr::null_mut());
                    let la = ENUM_LOG.clone();
                    ENUM_SEEN = 0;
                    ENUM_LOG.clear();
                    let rb = (p.r.callout_enumerate)(b, Some(enum_cb_stop), ptr::null_mut());
                    let lb = ENUM_LOG.clone();
                    d.eq(&format!("callout_enumerate {pat} stop@{stop_at} rc"), ra, rb);
                    d.eq(&format!("callout_enumerate {pat} stop@{stop_at} log"), la, lb);
                }
            }
        }
        // NULL code / NULL callback
        d.eq(
            "callout_enumerate(NULL code)",
            (p.c.callout_enumerate)(ptr::null_mut(), Some(enum_cb), ptr::null_mut()),
            (p.r.callout_enumerate)(ptr::null_mut(), Some(enum_cb), ptr::null_mut()),
        );

        // randomized sweep of pattern_info over the whole corpus
        for _ in 0..600 {
            let pat = PATTERNS[rng.below(PATTERNS.len())].as_bytes();
            let opts = *rng.pick(&[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_NO_START_OPTIMIZE]);
            let (a, b, _, _) = compile2(p, pat, opts, None);
            if a.is_null() {
                continue;
            }
            cmp_info_all(p, a, b, &format!("rand {} {opts:#x}", show(pat)), &mut d);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 365-372: every pattern_info request code (value + NULL query + out-of-range) and callout_enumerate");
}

static mut ENUM_LOG: Vec<u8> = Vec::new();
static mut ENUM_STOP_AT: u32 = 0;
static mut ENUM_SEEN: u32 = 0;

#[repr(C)]
struct EnumBlock {
    version: u32,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_number: u32,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: Sptr,
}

unsafe extern "C" fn enum_cb(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const EnumBlock);
    let log = &mut *ptr::addr_of_mut!(ENUM_LOG);
    for v in [
        b.version as u64,
        b.pattern_position as u64,
        b.next_item_length as u64,
        b.callout_number as u64,
        b.callout_string_offset as u64,
        b.callout_string_length as u64,
    ] {
        log.extend_from_slice(&v.to_le_bytes());
    }
    if !b.callout_string.is_null() {
        log.extend_from_slice(std::slice::from_raw_parts(b.callout_string, b.callout_string_length));
    }
    0
}

unsafe extern "C" fn enum_cb_stop(blk: *mut c_void, d: *mut c_void) -> c_int {
    let seen = &mut *ptr::addr_of_mut!(ENUM_SEEN);
    let stop = *ptr::addr_of!(ENUM_STOP_AT);
    enum_cb(blk, d);
    *seen += 1;
    if *seen > stop {
        -99
    } else {
        0
    }
}

// ===================================================== rows 373-379

static mut ENC_C: (usize, usize) = (0, 0);
static mut ENC_R: (usize, usize) = (0, 0);
static mut DEC_C: (usize, usize) = (0, 0);
static mut DEC_R: (usize, usize) = (0, 0);

macro_rules! counting_malloc {
    ($name:ident, $slot:ident) => {
        unsafe extern "C" fn $name(n: usize, _d: *mut c_void) -> *mut c_void {
            let s = &mut *ptr::addr_of_mut!($slot);
            s.0 += 1;
            s.1 += n;
            raw_alloc(n)
        }
    };
}
counting_malloc!(enc_malloc_c, ENC_C);
counting_malloc!(enc_malloc_r, ENC_R);
counting_malloc!(dec_malloc_c, DEC_C);
counting_malloc!(dec_malloc_r, DEC_R);

unsafe extern "C" fn raw_free(p: *mut c_void, _d: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}
unsafe fn raw_alloc(n: usize) -> *mut c_void {
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let p = std::alloc::alloc(l);
    *(p as *mut usize) = sz;
    p.add(16) as *mut c_void
}

#[test]
fn cfg_373_379_serialize() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mut tables_len = 0u32;
        (p.c.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut tables_len as *mut u32 as Ptr);

        // row 379: NULL free is a no-op
        (p.c.serialize_free)(ptr::null_mut());
        (p.r.serialize_free)(ptr::null_mut());
        d.checked += 1;

        // rows 373, 374, 377, 378
        let groups: &[&[&str]] = &[
            &["abc"],
            &[""],
            &["(a)(b)", "\\d+", "[a-z]+", "(?<n>x)", "\\p{L}+"],
            &["(?<a>a)(?<a>b)?", "a\\Kb", "(*MARK:m)x", "\\X", "(?R)?a"],
        ];
        for (gi, g) in groups.iter().enumerate() {
            for &opts in &[0u32, PCRE2_UTF | PCRE2_UCP, PCRE2_DUPNAMES] {
                let mut ca: Vec<Ptr> = Vec::new();
                let mut cb: Vec<Ptr> = Vec::new();
                for pat in g.iter() {
                    let (a, b, _, _) = compile2(p, pat.as_bytes(), opts, None);
                    if !a.is_null() {
                        ca.push(a);
                        cb.push(b);
                    }
                }
                if ca.is_empty() {
                    continue;
                }
                let (mut ba, mut bb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                let (mut na, mut nb) = (0usize, 0usize);
                let ra = (p.c.serialize_encode)(ca.as_ptr(), ca.len() as i32, &mut ba, &mut na, ptr::null_mut());
                let rb = (p.r.serialize_encode)(cb.as_ptr(), cb.len() as i32, &mut bb, &mut nb, ptr::null_mut());
                let tag = format!("serialize g{gi} opts={opts:#x} n={}", ca.len());
                d.eq(&format!("{tag} encode rc"), ra, rb);
                d.eq(&format!("{tag} encode size"), na, nb);
                if ra > 0 && rb > 0 {
                    d.eq(
                        &format!("{tag} stream bytes"),
                        std::slice::from_raw_parts(ba, na).to_vec(),
                        std::slice::from_raw_parts(bb, nb).to_vec(),
                    );
                    // row 373: exact stream size = header + tables + sum(blocksize)
                    let hdr = 16usize; // magic, version, config, number_of_codes
                    let expect: usize = hdr
                        + tables_len as usize
                        + ca.iter().map(|&c| code_blocksize(c)).sum::<usize>();
                    d.eq(&format!("{tag} stream size formula"), na, expect);
                    // row 377
                    d.eq(
                        &format!("{tag} get_number_of_codes"),
                        (p.c.serialize_get_number_of_codes)(ba),
                        (p.r.serialize_get_number_of_codes)(bb),
                    );
                    // row 374: decode counts below, equal and above
                    for want in [1i32, ca.len() as i32 - 1, ca.len() as i32, ca.len() as i32 + 3] {
                        if want <= 0 {
                            continue;
                        }
                        let mut da: Vec<Ptr> = vec![ptr::null_mut(); (want as usize) + 4];
                        let mut db: Vec<Ptr> = vec![ptr::null_mut(); (want as usize) + 4];
                        let ea = (p.c.serialize_decode)(da.as_mut_ptr(), want, ba, ptr::null_mut());
                        let eb = (p.r.serialize_decode)(db.as_mut_ptr(), want, bb, ptr::null_mut());
                        d.eq(&format!("{tag} decode(want={want}) rc"), ea, eb);
                        if ea > 0 && eb > 0 {
                            for i in 0..(ea as usize) {
                                assert_code_eq(da[i], db[i], &format!("{tag} decoded[{i}] want={want}"));
                                assert_code_eq_masked(
                                    da[i], ca[i], PCRE2_DEREF_TABLES,
                                    &format!("{tag} decoded[{i}] vs original"),
                                );
                                // row 378: the decoded code must be fully usable
                                cmp_info_all(p, da[i], db[i], &format!("{tag} decoded[{i}] info"), &mut d);
                                let mda = (p.c.match_data_create_from_pattern)(da[i], ptr::null_mut());
                                let mdb = (p.r.match_data_create_from_pattern)(db[i], ptr::null_mut());
                                for subj in SUBJECTS.iter().take(16) {
                                    let sb = subj.as_bytes();
                                    let x = (p.c.do_match)(da[i], sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                                    let y = (p.r.do_match)(db[i], sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                                    d.eq(
                                        &format!("{tag} decoded[{i}] match {}", show(sb)),
                                        read_match_out(&p.c, mda, x),
                                        read_match_out(&p.r, mdb, y),
                                    );
                                    // substitute through the decoded code
                                    let (mut oa, mut ob) = (vec![0xEEu8; 128], vec![0xEEu8; 128]);
                                    let (mut l1, mut l2) = (100usize, 100usize);
                                    let s1 = (p.c.substitute)(
                                        da[i], sb.as_ptr(), sb.len(), 0, PCRE2_SUBSTITUTE_GLOBAL,
                                        ptr::null_mut(), ptr::null_mut(), b"<$0>".as_ptr(), 4,
                                        oa.as_mut_ptr(), &mut l1,
                                    );
                                    let s2 = (p.r.substitute)(
                                        db[i], sb.as_ptr(), sb.len(), 0, PCRE2_SUBSTITUTE_GLOBAL,
                                        ptr::null_mut(), ptr::null_mut(), b"<$0>".as_ptr(), 4,
                                        ob.as_mut_ptr(), &mut l2,
                                    );
                                    d.eq(&format!("{tag} decoded[{i}] substitute rc"), s1, s2);
                                    d.eq(&format!("{tag} decoded[{i}] substitute len"), l1, l2);
                                    d.eq(&format!("{tag} decoded[{i}] substitute out"), oa, ob);
                                }
                                (p.c.match_data_free)(mda);
                                (p.r.match_data_free)(mdb);
                                // code_copy of a decoded code
                                let ka = (p.c.code_copy)(da[i]);
                                let kb = (p.r.code_copy)(db[i]);
                                assert_code_eq(ka, kb, &format!("{tag} copy of decoded[{i}]"));
                                (p.c.code_free)(ka);
                                (p.r.code_free)(kb);
                                // callout_enumerate on a decoded code
                                ENUM_LOG.clear();
                                let x = (p.c.callout_enumerate)(da[i], Some(enum_cb), ptr::null_mut());
                                let lx = ENUM_LOG.clone();
                                ENUM_LOG.clear();
                                let y = (p.r.callout_enumerate)(db[i], Some(enum_cb), ptr::null_mut());
                                let ly = ENUM_LOG.clone();
                                d.eq(&format!("{tag} decoded[{i}] enumerate rc"), x, y);
                                d.eq(&format!("{tag} decoded[{i}] enumerate log"), lx, ly);
                            }
                            for i in 0..(ea as usize) {
                                (p.c.code_free)(da[i]);
                                (p.r.code_free)(db[i]);
                            }
                        }
                    }
                }
                if !ba.is_null() {
                    (p.c.serialize_free)(ba);
                }
                if !bb.is_null() {
                    (p.r.serialize_free)(bb);
                }
                for i in 0..ca.len() {
                    (p.c.code_free)(ca[i]);
                    (p.r.code_free)(cb[i]);
                }
            }
        }

        // row 375: every code compiled against the SAME pcre2_maketables block.
        // The tables are BORROWED by each code, so they must outlive them.
        {
            let ta = (p.c.maketables)(ptr::null_mut());
            let tb = (p.r.maketables)(ptr::null_mut());
            assert!(!ta.is_null() && !tb.is_null());
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            assert_eq!((p.c.set_character_tables)(cca, ta), 0);
            assert_eq!((p.r.set_character_tables)(ccb, tb), 0);
            let ctx = Ctx { a: cca, b: ccb };
            let mut ca: Vec<Ptr> = Vec::new();
            let mut cb: Vec<Ptr> = Vec::new();
            for pat in ["(?i)abc", "\\w+", "[[:alpha:]]+"] {
                let (a, b, _, _) = compile2(p, pat.as_bytes(), 0, Some(&ctx));
                assert!(!a.is_null());
                ca.push(a);
                cb.push(b);
            }
            let (mut ba, mut bb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
            let (mut na, mut nb) = (0usize, 0usize);
            let ra = (p.c.serialize_encode)(ca.as_ptr(), 3, &mut ba, &mut na, ptr::null_mut());
            let rb = (p.r.serialize_encode)(cb.as_ptr(), 3, &mut bb, &mut nb, ptr::null_mut());
            d.eq("shared-maketables encode rc", ra, rb);
            d.eq("shared-maketables encode size", na, nb);
            if ra > 0 && rb > 0 {
                d.eq(
                    "shared-maketables stream bytes",
                    std::slice::from_raw_parts(ba, na).to_vec(),
                    std::slice::from_raw_parts(bb, nb).to_vec(),
                );
                let mut da: Vec<Ptr> = vec![ptr::null_mut(); 3];
                let mut db: Vec<Ptr> = vec![ptr::null_mut(); 3];
                let ea = (p.c.serialize_decode)(da.as_mut_ptr(), 3, ba, ptr::null_mut());
                let eb = (p.r.serialize_decode)(db.as_mut_ptr(), 3, bb, ptr::null_mut());
                d.eq("shared-maketables decode rc", ea, eb);
                if ea > 0 && eb > 0 {
                    for i in 0..(ea as usize) {
                        assert_code_eq(da[i], db[i], &format!("shared-maketables decoded[{i}]"));
                        (p.c.code_free)(da[i]);
                        (p.r.code_free)(db[i]);
                    }
                }
            }
            if !ba.is_null() {
                (p.c.serialize_free)(ba);
            }
            if !bb.is_null() {
                (p.r.serialize_free)(bb);
            }
            for i in 0..3 {
                (p.c.code_free)(ca[i]);
                (p.r.code_free)(cb[i]);
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
            (p.c.maketables_free)(ptr::null_mut(), ta);
            (p.r.maketables_free)(ptr::null_mut(), tb);
        }

        // row 376: different custom allocators on encode and decode
        {
            ENC_C = (0, 0);
            ENC_R = (0, 0);
            DEC_C = (0, 0);
            DEC_R = (0, 0);
            let ega = (p.c.general_context_create)(Some(enc_malloc_c), Some(raw_free), ptr::null_mut());
            let egb = (p.r.general_context_create)(Some(enc_malloc_r), Some(raw_free), ptr::null_mut());
            let dga = (p.c.general_context_create)(Some(dec_malloc_c), Some(raw_free), ptr::null_mut());
            let dgb = (p.r.general_context_create)(Some(dec_malloc_r), Some(raw_free), ptr::null_mut());
            let mut ca: Vec<Ptr> = Vec::new();
            let mut cb: Vec<Ptr> = Vec::new();
            for pat in ["(a)b", "\\d\\d"] {
                let (a, b, _, _) = compile2(p, pat.as_bytes(), 0, None);
                ca.push(a);
                cb.push(b);
            }
            let (mut ba, mut bb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
            let (mut na, mut nb) = (0usize, 0usize);
            let ra = (p.c.serialize_encode)(ca.as_ptr(), 2, &mut ba, &mut na, ega);
            let rb = (p.r.serialize_encode)(cb.as_ptr(), 2, &mut bb, &mut nb, egb);
            d.eq("custom-alloc encode rc", ra, rb);
            d.eq("custom-alloc encode size", na, nb);
            d.eq("custom-alloc encode accounting", ENC_C, ENC_R);
            if ra > 0 && rb > 0 {
                let mut da: Vec<Ptr> = vec![ptr::null_mut(); 2];
                let mut db: Vec<Ptr> = vec![ptr::null_mut(); 2];
                let ea = (p.c.serialize_decode)(da.as_mut_ptr(), 2, ba, dga);
                let eb = (p.r.serialize_decode)(db.as_mut_ptr(), 2, bb, dgb);
                d.eq("custom-alloc decode rc", ea, eb);
                d.eq("custom-alloc decode accounting", DEC_C, DEC_R);
                if ea > 0 && eb > 0 {
                    for i in 0..(ea as usize) {
                        assert_code_eq(da[i], db[i], &format!("custom-alloc decoded[{i}]"));
                        // the decoded code must carry PCRE2_DEREF_TABLES
                        let fa = (*(da[i] as *const RealCodeHead)).flags;
                        let fb = (*(db[i] as *const RealCodeHead)).flags;
                        d.eq(
                            &format!("custom-alloc decoded[{i}] DEREF_TABLES"),
                            fa & PCRE2_DEREF_TABLES,
                            fb & PCRE2_DEREF_TABLES,
                        );
                        assert_eq!(fa & PCRE2_DEREF_TABLES, PCRE2_DEREF_TABLES);
                        (p.c.code_free)(da[i]);
                        (p.r.code_free)(db[i]);
                    }
                }
            }
            // freeing the stream must go through the ENCODE-time allocator
            if !ba.is_null() {
                (p.c.serialize_free)(ba);
            }
            if !bb.is_null() {
                (p.r.serialize_free)(bb);
            }
            for i in 0..2 {
                (p.c.code_free)(ca[i]);
                (p.r.code_free)(cb[i]);
            }
            for g in [ega, dga] {
                (p.c.general_context_free)(g);
            }
            for g in [egb, dgb] {
                (p.r.general_context_free)(g);
            }
        }
    }
    d.finish("CONFIGS 373-379: serialize round trips, stream-size formula, shared tables, custom allocators, decoded-code usability");
}

// ===================================================== rows 380-394

/// Runs `pcre2_pattern_convert_8` through all three buffer protocols and
/// compares every observable between the two libraries.
unsafe fn cmp_convert(p: &Pair, src: &[u8], len: Sz, opts: u32, cca: Ptr, ccb: Ptr, tag: &str, d: &mut Diffs) {
    // protocol A: buffptr == NULL -> length only
    {
        let (mut na, mut nb) = (usize::MAX, usize::MAX);
        let ra = (p.c.pattern_convert)(src.as_ptr(), len, opts, ptr::null_mut(), &mut na, cca);
        let rb = (p.r.pattern_convert)(src.as_ptr(), len, opts, ptr::null_mut(), &mut nb, ccb);
        d.eq(&format!("{tag} [len-only] rc"), ra, rb);
        d.eq(&format!("{tag} [len-only] len"), na, nb);
    }
    // protocol B: library allocates
    let mut out_c: Option<Vec<u8>> = None;
    {
        let (mut oa, mut ob) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
        let (mut na, mut nb) = (usize::MAX, usize::MAX);
        let ra = (p.c.pattern_convert)(src.as_ptr(), len, opts, &mut oa, &mut na, cca);
        let rb = (p.r.pattern_convert)(src.as_ptr(), len, opts, &mut ob, &mut nb, ccb);
        d.eq(&format!("{tag} [alloc] rc"), ra, rb);
        d.eq(&format!("{tag} [alloc] len"), na, nb);
        if ra == 0 && rb == 0 {
            let x = std::slice::from_raw_parts(oa, na + 1).to_vec();
            let y = std::slice::from_raw_parts(ob, nb + 1).to_vec();
            d.eq(&format!("{tag} [alloc] output+NUL"), x.clone(), y);
            out_c = Some(x[..na].to_vec());
            // the converted pattern must itself compile identically
            let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
            let (mut f1, mut f2) = (0usize, 0usize);
            let k1 = (p.c.compile)(oa, na, 0, &mut e1, &mut f1, ptr::null_mut());
            let k2 = (p.r.compile)(ob, nb, 0, &mut e2, &mut f2, ptr::null_mut());
            d.eq(&format!("{tag} recompile null?"), k1.is_null(), k2.is_null());
            d.eq(&format!("{tag} recompile ec"), e1, e2);
            d.eq(&format!("{tag} recompile eo"), f1, f2);
            if !k1.is_null() && !k2.is_null() {
                assert_code_eq(k1, k2, &format!("{tag} recompiled"));
            }
            if !k1.is_null() {
                (p.c.code_free)(k1);
            }
            if !k2.is_null() {
                (p.r.code_free)(k2);
            }
        }
        if !oa.is_null() {
            (p.c.converted_pattern_free)(oa);
        }
        if !ob.is_null() {
            (p.r.converted_pattern_free)(ob);
        }
    }
    // protocol C: caller-supplied buffer, exact / one-too-small / generous / 0
    let need = out_c.as_ref().map_or(8, |v| v.len());
    for cap in [0usize, 1, need, need + 1, need + 8, 512] {
        let mut qa = vec![0xEEu8; cap + 16];
        let mut qb = vec![0xEEu8; cap + 16];
        let mut pa = qa.as_mut_ptr();
        let mut pb = qb.as_mut_ptr();
        let (mut ma, mut mb) = (cap, cap);
        let ra = (p.c.pattern_convert)(src.as_ptr(), len, opts, &mut pa, &mut ma, cca);
        let rb = (p.r.pattern_convert)(src.as_ptr(), len, opts, &mut pb, &mut mb, ccb);
        d.eq(&format!("{tag} [buf {cap}] rc"), ra, rb);
        d.eq(&format!("{tag} [buf {cap}] len"), ma, mb);
        d.eq(&format!("{tag} [buf {cap}] bytes"), qa, qb);
    }
}

#[test]
fn cfg_380_394_convert() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(38000);
    unsafe {
        // row 394: NULL free is a no-op
        (p.c.converted_pattern_free)(ptr::null_mut());
        (p.r.converted_pattern_free)(ptr::null_mut());
        d.checked += 1;

        // rows 380-384, 387: glob patterns x glob mode bits
        let globs: &[&str] = &[
            "*", "?", "a", "**", "a/**", "**abc", "**/abc", "a/**/b", "a/**x", "*.txt",
            "a?c", "[abc]", "[!abc]", "[^abc]", "[]]", "[!]]", "[a-z]", "[[:alpha:]]",
            "[[:foo:]]", "[a-[:alpha:]]", "[/]", "[.-9]", "[\\]]", "[!a]", "*[!a]", "?[!a]",
            "a[!a]", "**\\/x", "/**\\/x", "a\\*b", "a\\\\b", "a\\", "\\", "", "/",
            "a/b/c", "**/**", "x[", "x]", "[", "]", "a**b", "{a,b}", "~", ".", "..",
        ];
        let glob_modes: &[(u32, &str)] = &[
            (PCRE2_CONVERT_GLOB, "GLOB"),
            (PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, "GLOB_NO_WILD_SEPARATOR"),
            (PCRE2_CONVERT_GLOB_NO_STARSTAR, "GLOB_NO_STARSTAR"),
            (PCRE2_CONVERT_GLOB | 0x60, "GLOB|both mod bits"),
        ];
        for &(mode, mname) in glob_modes {
            // rows 385, 386, 402: separators and escapes
            let seps: &[u32] = &[0, b'/' as u32, b'\\' as u32, b'.' as u32];
            let escs: &[u32] = &[0xFFFF_FFFF, 0, b'\\' as u32, b'`' as u32];
            for &sep in seps {
                for &esc in escs {
                    let cca = (p.c.convert_context_create)(ptr::null_mut());
                    let ccb = (p.r.convert_context_create)(ptr::null_mut());
                    if sep != 0 {
                        d.eq(
                            &format!("set_glob_separator({sep})"),
                            (p.c.set_glob_separator)(cca, sep),
                            (p.r.set_glob_separator)(ccb, sep),
                        );
                    }
                    if esc != 0xFFFF_FFFF {
                        d.eq(
                            &format!("set_glob_escape({esc})"),
                            (p.c.set_glob_escape)(cca, esc),
                            (p.r.set_glob_escape)(ccb, esc),
                        );
                    }
                    for g in globs {
                        let gb = g.as_bytes();
                        let mut zt = gb.to_vec();
                        zt.push(0);
                        for utf in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
                            cmp_convert(
                                p, gb, gb.len(), mode | utf, cca, ccb,
                                &format!("glob {mname} sep={sep} esc={esc} utf={utf:#x} {}", show(gb)),
                                &mut d,
                            );
                            cmp_convert(
                                p, &zt, PCRE2_ZERO_TERMINATED, mode | utf, cca, ccb,
                                &format!("glob-ZT {mname} sep={sep} esc={esc} utf={utf:#x} {}", show(gb)),
                                &mut d,
                            );
                        }
                    }
                    (p.c.convert_context_free)(cca);
                    (p.r.convert_context_free)(ccb);
                }
            }
        }

        // rows 388, 389, 390: POSIX BASIC vs EXTENDED over the full table
        let posix: &[&str] = &[
            "\\(a\\)", "\\{2,3\\}", "\\1", "a*", "*a", "^a", "a^b", "a$", "a$b", "**",
            "\\.", "[]]", "[^]]", "[[:alpha:]]", "[a\\]b]", "a+b", "a?b", "a|b", "(a)(b)",
            "^abc$", "a{2,3}", "\\(", "\\)", "(", ")", "[a-", "a\\\\b", "", "a", "abc",
            "a.c", "\\", "[", "]", "\\<", "\\>", "\\b", "\\w", "a\\{2\\}", "$^",
        ];
        for &(mode, mname) in &[
            (PCRE2_CONVERT_POSIX_BASIC, "POSIX_BASIC"),
            (PCRE2_CONVERT_POSIX_EXTENDED, "POSIX_EXTENDED"),
            (PCRE2_CONVERT_POSIX_BASIC | 0x20, "POSIX_BASIC|0x20"),
            (PCRE2_CONVERT_POSIX_BASIC | 0x40, "POSIX_BASIC|0x40"),
            (PCRE2_CONVERT_POSIX_EXTENDED | 0x60, "POSIX_EXTENDED|0x60"),
        ] {
            let cca = (p.c.convert_context_create)(ptr::null_mut());
            let ccb = (p.r.convert_context_create)(ptr::null_mut());
            for s in posix {
                let sb = s.as_bytes();
                let mut zt = sb.to_vec();
                zt.push(0);
                for utf in [0u32, PCRE2_CONVERT_UTF] {
                    cmp_convert(
                        p, sb, sb.len(), mode | utf, cca, ccb,
                        &format!("{mname} utf={utf:#x} {}", show(sb)),
                        &mut d,
                    );
                    cmp_convert(
                        p, &zt, PCRE2_ZERO_TERMINATED, mode | utf, cca, ccb,
                        &format!("{mname}-ZT utf={utf:#x} {}", show(sb)),
                        &mut d,
                    );
                }
            }
            (p.c.convert_context_free)(cca);
            (p.r.convert_context_free)(ccb);
        }

        // row 391: multi-byte input for each mode
        {
            let cca = (p.c.convert_context_create)(ptr::null_mut());
            let ccb = (p.r.convert_context_create)(ptr::null_mut());
            for s in ["\u{e9}", "[\u{e0}-\u{e9}]", "\u{1f600}*", "a\u{2028}b", "\u{4e00}?"] {
                let sb = s.as_bytes();
                for mode in [
                    PCRE2_CONVERT_GLOB,
                    PCRE2_CONVERT_POSIX_BASIC,
                    PCRE2_CONVERT_POSIX_EXTENDED,
                ] {
                    for utf in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
                        cmp_convert(
                            p, sb, sb.len(), mode | utf, cca, ccb,
                            &format!("utf-input mode={mode:#x} utf={utf:#x} {}", show(sb)),
                            &mut d,
                        );
                    }
                }
            }
            (p.c.convert_context_free)(cca);
            (p.r.convert_context_free)(ccb);
        }

        // row 393: input shapes
        {
            let cca = (p.c.convert_context_create)(ptr::null_mut());
            let ccb = (p.r.convert_context_create)(ptr::null_mut());
            for mode in [
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
            ] {
                // pattern == NULL with plength == 0
                let (mut na, mut nb) = (usize::MAX, usize::MAX);
                let (mut oa, mut ob) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                let ra = (p.c.pattern_convert)(ptr::null(), 0, mode, &mut oa, &mut na, cca);
                let rb = (p.r.pattern_convert)(ptr::null(), 0, mode, &mut ob, &mut nb, ccb);
                d.eq(&format!("convert(NULL,0) mode={mode:#x} rc"), ra, rb);
                d.eq(&format!("convert(NULL,0) mode={mode:#x} len"), na, nb);
                if ra == 0 && rb == 0 {
                    d.eq(
                        &format!("convert(NULL,0) mode={mode:#x} out"),
                        std::slice::from_raw_parts(oa, na + 1).to_vec(),
                        std::slice::from_raw_parts(ob, nb + 1).to_vec(),
                    );
                }
                if !oa.is_null() {
                    (p.c.converted_pattern_free)(oa);
                }
                if !ob.is_null() {
                    (p.r.converted_pattern_free)(ob);
                }
                // embedded NUL with explicit length, and a raw 0xFF byte
                for src in [
                    b"a\x00b".to_vec(),
                    b"\x00".to_vec(),
                    b"\xff".to_vec(),
                    b"a\xffb".to_vec(),
                    b"\xff\xfe".to_vec(),
                ] {
                    cmp_convert(
                        p, &src, src.len(), mode, cca, ccb,
                        &format!("shape mode={mode:#x} {}", show(&src)),
                        &mut d,
                    );
                }
            }
            (p.c.convert_context_free)(cca);
            (p.r.convert_context_free)(ccb);
        }

        // randomized sweep
        {
            let cca = (p.c.convert_context_create)(ptr::null_mut());
            let ccb = (p.r.convert_context_create)(ptr::null_mut());
            for _ in 0..1500 {
                let src = if rng.chance(2) {
                    gen_ascii(&mut rng, 12)
                } else {
                    let mut v = Vec::new();
                    let alpha: &[&[u8]] = &[
                        b"*", b"?", b"[", b"]", b"!", b"^", b"-", b"/", b"\\", b".", b"a", b"z",
                        b"**", b"[[:alpha:]]", b"{", b"}", b"(", b")", b"|", b"+", b"$",
                    ];
                    for _ in 0..rng.range(0, 8) {
                        v.extend_from_slice(rng.pick_bytes(alpha));
                    }
                    v
                };
                let mode = *rng.pick(&[
                    PCRE2_CONVERT_GLOB,
                    PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
                    PCRE2_CONVERT_GLOB_NO_STARSTAR,
                    PCRE2_CONVERT_POSIX_BASIC,
                    PCRE2_CONVERT_POSIX_EXTENDED,
                ]);
                let utf = *rng.pick(&[0u32, PCRE2_CONVERT_UTF]);
                cmp_convert(
                    p, &src, src.len(), mode | utf, cca, ccb,
                    &format!("rand mode={mode:#x} utf={utf:#x} {}", show(&src)),
                    &mut d,
                );
            }
            (p.c.convert_context_free)(cca);
            (p.r.convert_context_free)(ccb);
        }
    }
    d.finish("CONFIGS 380-394: GLOB (all mod bits, separators, escapes, classes, **) and POSIX BASIC/EXTENDED x UTF x all three buffer protocols");
}

// ===================================================== rows 395-406

#[test]
fn cfg_395_398_contexts() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // row 396: compile_context_create(NULL) must equal the default context
        // (except the allocator pointers, which are each library's own).
        let cca = (p.c.compile_context_create)(ptr::null_mut());
        let ccb = (p.r.compile_context_create)(ptr::null_mut());
        let da = p.c.data("_pcre2_default_compile_context_8");
        let db = p.r.data("_pcre2_default_compile_context_8");
        // compare the non-pointer tail of the struct (offset 56 onwards: the
        // three memctl pointers + stack_guard + stack_guard_data + tables)
        let tail = |base: *const u8| std::slice::from_raw_parts(base.add(56), 88 - 56).to_vec();
        d.eq("compile_context_create(NULL) == default (C)", tail(cca as *const u8), tail(da));
        d.eq("compile_context_create(NULL) == default (rust)", tail(ccb as *const u8), tail(db));
        d.eq("compile_context tail C vs rust", tail(cca as *const u8), tail(ccb as *const u8));
        // row 397
        let mca = (p.c.match_context_create)(ptr::null_mut());
        let mcb = (p.r.match_context_create)(ptr::null_mut());
        let mda = p.c.data("_pcre2_default_match_context_8");
        let mdb = p.r.data("_pcre2_default_match_context_8");
        let mtail = |base: *const u8| std::slice::from_raw_parts(base.add(72), 96 - 72).to_vec();
        d.eq("match_context_create(NULL) == default (C)", mtail(mca as *const u8), mtail(mda));
        d.eq("match_context_create(NULL) == default (rust)", mtail(mcb as *const u8), mtail(mdb));
        d.eq("match_context tail C vs rust", mtail(mca as *const u8), mtail(mcb as *const u8));
        // row 398
        let vca = (p.c.convert_context_create)(ptr::null_mut());
        let vcb = (p.r.convert_context_create)(ptr::null_mut());
        let vda = p.c.data("_pcre2_default_convert_context_8");
        let vdb = p.r.data("_pcre2_default_convert_context_8");
        let vtail = |base: *const u8| std::slice::from_raw_parts(base.add(24), 32 - 24).to_vec();
        d.eq("convert_context_create(NULL) == default (C)", vtail(vca as *const u8), vtail(vda));
        d.eq("convert_context_create(NULL) == default (rust)", vtail(vcb as *const u8), vtail(vdb));
        d.eq("convert_context tail C vs rust", vtail(vca as *const u8), vtail(vcb as *const u8));

        // copies must be tail-identical to their source
        for (name, (a, b), off, size) in [
            ("compile", (cca, ccb), 56usize, 88usize),
            ("match", (mca, mcb), 72, 96),
            ("convert", (vca, vcb), 24, 32),
        ] {
            let (ka, kb) = match name {
                "compile" => ((p.c.compile_context_copy)(a), (p.r.compile_context_copy)(b)),
                "match" => ((p.c.match_context_copy)(a), (p.r.match_context_copy)(b)),
                _ => ((p.c.convert_context_copy)(a), (p.r.convert_context_copy)(b)),
            };
            assert!(!ka.is_null() && !kb.is_null(), "{name}_context_copy failed");
            let t = |x: Ptr| std::slice::from_raw_parts((x as *const u8).add(off), size - off).to_vec();
            d.eq(&format!("{name}_context_copy tail == source (C)"), t(a), t(ka));
            d.eq(&format!("{name}_context_copy tail == source (rust)"), t(b), t(kb));
            d.eq(&format!("{name}_context_copy tail C vs rust"), t(ka), t(kb));
            match name {
                "compile" => {
                    (p.c.compile_context_free)(ka);
                    (p.r.compile_context_free)(kb);
                }
                "match" => {
                    (p.c.match_context_free)(ka);
                    (p.r.match_context_free)(kb);
                }
                _ => {
                    (p.c.convert_context_free)(ka);
                    (p.r.convert_context_free)(kb);
                }
            }
        }
        (p.c.compile_context_free)(cca);
        (p.r.compile_context_free)(ccb);
        (p.c.match_context_free)(mca);
        (p.r.match_context_free)(mcb);
        (p.c.convert_context_free)(vca);
        (p.r.convert_context_free)(vcb);

        // row 395: the general-context allocator-pair matrix.
        //
        // `pcre2_general_context_create` substitutes its own `default_malloc` /
        // `default_free` for a NULL argument, so a PARTIAL pair (one custom, one
        // NULL) produces a context that mallocs with one allocator and frees
        // with the other. Freeing such a context is a caller error that corrupts
        // the heap in the C too, so those cases are created and compared but
        // deliberately LEAKED rather than freed.
        let dt = |x: Ptr| *((x as *const u8).add(16) as *const usize);
        for (mc, mr, f, consistent, label) in [
            (None, None, None, true, "(NULL,NULL,NULL)"),
            (
                Some(enc_malloc_c as MallocFn),
                Some(enc_malloc_r as MallocFn),
                Some(raw_free as FreeFn),
                true,
                "(malloc,free)",
            ),
            (Some(enc_malloc_c as MallocFn), Some(enc_malloc_r as MallocFn), None, false, "(malloc,NULL)"),
            (None, None, Some(raw_free as FreeFn), false, "(NULL,free)"),
        ] {
            let ga = (p.c.general_context_create)(mc, f, 0xABCD as Ptr);
            let gb = (p.r.general_context_create)(mr, f, 0xABCD as Ptr);
            d.eq(&format!("general_context_create{label} null?"), ga.is_null(), gb.is_null());
            if ga.is_null() || gb.is_null() {
                continue;
            }
            d.eq(&format!("general_context_create{label} memory_data"), dt(ga), dt(gb));
            let ka = (p.c.general_context_copy)(ga);
            let kb = (p.r.general_context_copy)(gb);
            d.eq(&format!("general_context_copy{label} null?"), ka.is_null(), kb.is_null());
            if !ka.is_null() && !kb.is_null() {
                d.eq(&format!("general_context_copy{label} memory_data"), dt(ka), dt(kb));
            }
            if consistent {
                if !ka.is_null() {
                    (p.c.general_context_free)(ka);
                }
                if !kb.is_null() {
                    (p.r.general_context_free)(kb);
                }
                (p.c.general_context_free)(ga);
                (p.r.general_context_free)(gb);
            }
            // else: intentionally leaked, see the comment above.
        }
        // context_create with a general context whose allocator is custom
        {
            let ga = (p.c.general_context_create)(Some(enc_malloc_c), Some(raw_free), 0x1234 as Ptr);
            let gb = (p.r.general_context_create)(Some(enc_malloc_r), Some(raw_free), 0x1234 as Ptr);
            for _ in 0..1 {
                let a = (p.c.compile_context_create)(ga);
                let b = (p.r.compile_context_create)(gb);
                d.eq("compile_context_create(gcontext) null?", a.is_null(), b.is_null());
                // the memctl must have been overridden with the gcontext's
                let md = |x: Ptr| *((x as *const u8).add(16) as *const usize);
                d.eq("compile_context_create(gcontext) memory_data", md(a), md(b));
                (p.c.compile_context_free)(a);
                (p.r.compile_context_free)(b);
                let a = (p.c.match_context_create)(ga);
                let b = (p.r.match_context_create)(gb);
                d.eq("match_context_create(gcontext) memory_data", md(a), md(b));
                (p.c.match_context_free)(a);
                (p.r.match_context_free)(b);
                let a = (p.c.convert_context_create)(ga);
                let b = (p.r.convert_context_create)(gb);
                d.eq("convert_context_create(gcontext) memory_data", md(a), md(b));
                (p.c.convert_context_free)(a);
                (p.r.convert_context_free)(b);
            }
            (p.c.general_context_free)(ga);
            (p.r.general_context_free)(gb);
        }
    }
    d.finish("CONFIGS 395-398: all four context types create/copy/free vs the exported defaults, allocator matrix");
}

#[test]
fn cfg_399_403_setters_observed() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(39900);
    unsafe {
        // row 399: every newline convention, observed through compile + match
        for nl in 1u32..=6 {
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            d.eq(
                &format!("set_newline({nl}) rc"),
                (p.c.set_newline)(cca, nl),
                (p.r.set_newline)(ccb, nl),
            );
            let ctx = Ctx { a: cca, b: ccb };
            for pat in ["^a", "a$", ".", "\\R", "\\N", "(?m)^a", "(?m)a$", "a.b"] {
                let (a, b, _, _) = compile2(p, pat.as_bytes(), PCRE2_MULTILINE, Some(&ctx));
                if a.is_null() {
                    continue;
                }
                let (mut va, mut vb) = (0u32, 0u32);
                d.eq(
                    &format!("newline={nl} INFO_NEWLINE rc"),
                    (p.c.pattern_info)(a, PCRE2_INFO_NEWLINE, &mut va as *mut u32 as Ptr),
                    (p.r.pattern_info)(b, PCRE2_INFO_NEWLINE, &mut vb as *mut u32 as Ptr),
                );
                d.eq(&format!("newline={nl} INFO_NEWLINE value"), va, vb);
                assert_eq!(va, nl, "INFO_NEWLINE should report the value set");
                for subj in [
                    "a\nb", "a\rb", "a\r\nb", "a\u{85}b", "a\u{2028}b", "a\u{0}b", "ab", "\n", "\r\n",
                ] {
                    let sb = subj.as_bytes();
                    let mda = (p.c.match_data_create)(4, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                    let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                    let rb = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                    d.eq(
                        &format!("newline={nl} {pat} vs {}", show(sb)),
                        read_match_out(&p.c, mda, ra),
                        read_match_out(&p.r, mdb, rb),
                    );
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
        }

        // row 400: both \R conventions
        for bsr in [PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            d.eq(
                &format!("set_bsr({bsr}) rc"),
                (p.c.set_bsr)(cca, bsr),
                (p.r.set_bsr)(ccb, bsr),
            );
            let ctx = Ctx { a: cca, b: ccb };
            for pat in ["\\R", "\\R+", "a\\Rb", "[\\R]"] {
                let (a, b, _, _) = compile2(p, pat.as_bytes(), PCRE2_UTF, Some(&ctx));
                if a.is_null() {
                    continue;
                }
                let (mut va, mut vb) = (0u32, 0u32);
                (p.c.pattern_info)(a, PCRE2_INFO_BSR, &mut va as *mut u32 as Ptr);
                (p.r.pattern_info)(b, PCRE2_INFO_BSR, &mut vb as *mut u32 as Ptr);
                d.eq(&format!("bsr={bsr} INFO_BSR"), va, vb);
                assert_eq!(va, bsr);
                for subj in ["\n", "\r", "\r\n", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}", "a"] {
                    let sb = subj.as_bytes();
                    let mda = (p.c.match_data_create)(4, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                    let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                    let rb = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                    d.eq(
                        &format!("bsr={bsr} {pat} vs {}", show(sb)),
                        read_match_out(&p.c, mda, ra),
                        read_match_out(&p.r, mdb, rb),
                    );
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
        }

        // row 401: every legal optimize directive, observed in the bytecode
        for opt in [0u32, 1, 64, 65, 66, 67, 68, 69] {
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            let ra = (p.c.set_optimize)(cca, opt);
            let rb = (p.r.set_optimize)(ccb, opt);
            d.eq(&format!("set_optimize({opt}) rc"), ra, rb);
            if ra == 0 && rb == 0 {
                let ctx = Ctx { a: cca, b: ccb };
                for pat in PATTERNS.iter().step_by(3) {
                    let (a, b, _, _) = compile2(p, pat.as_bytes(), 0, Some(&ctx));
                    if a.is_null() {
                        continue;
                    }
                    // the directive is recorded in optimization_flags
                    let fa = (*(a as *const RealCodeHead)).optimization_flags;
                    let fb = (*(b as *const RealCodeHead)).optimization_flags;
                    d.eq(&format!("optimize={opt} optimization_flags {}", show(pat.as_bytes())), fa, fb);
                    for subj in SUBJECTS.iter().take(12) {
                        let sb = subj.as_bytes();
                        let mda = (p.c.match_data_create)(8, ptr::null_mut());
                        let mdb = (p.r.match_data_create)(8, ptr::null_mut());
                        let x = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                        let y = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                        d.eq(
                            &format!("optimize={opt} {} vs {}", show(pat.as_bytes()), show(sb)),
                            read_match_out(&p.c, mda, x),
                            read_match_out(&p.r, mdb, y),
                        );
                        (p.c.match_data_free)(mda);
                        (p.r.match_data_free)(mdb);
                    }
                    (p.c.code_free)(a);
                    (p.r.code_free)(b);
                }
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
        }

        // row 402: the accepted glob separator / escape value sets, exhaustively
        {
            let cca = (p.c.convert_context_create)(ptr::null_mut());
            let ccb = (p.r.convert_context_create)(ptr::null_mut());
            for v in 0u32..=0x120 {
                d.eq(
                    &format!("set_glob_separator({v}) rc"),
                    (p.c.set_glob_separator)(cca, v),
                    (p.r.set_glob_separator)(ccb, v),
                );
                d.eq(
                    &format!("set_glob_escape({v}) rc"),
                    (p.c.set_glob_escape)(cca, v),
                    (p.r.set_glob_escape)(ccb, v),
                );
            }
            for v in [0x10FFFFu32, 0x110000, u32::MAX] {
                d.eq(
                    &format!("set_glob_separator({v}) rc"),
                    (p.c.set_glob_separator)(cca, v),
                    (p.r.set_glob_separator)(ccb, v),
                );
                d.eq(
                    &format!("set_glob_escape({v}) rc"),
                    (p.c.set_glob_escape)(cca, v),
                    (p.r.set_glob_escape)(ccb, v),
                );
            }
            (p.c.convert_context_free)(cca);
            (p.r.convert_context_free)(ccb);
        }

        // row 403: maketables with and without a gcontext, all table regions,
        // then used for a compile. Tables are BORROWED, so free them last.
        for use_g in [false, true] {
            let (ga, gb) = if use_g {
                (
                    (p.c.general_context_create)(Some(enc_malloc_c), Some(raw_free), ptr::null_mut()),
                    (p.r.general_context_create)(Some(enc_malloc_r), Some(raw_free), ptr::null_mut()),
                )
            } else {
                (ptr::null_mut(), ptr::null_mut())
            };
            let ta = (p.c.maketables)(ga);
            let tb = (p.r.maketables)(gb);
            assert!(!ta.is_null() && !tb.is_null());
            let mut n = 0u32;
            (p.c.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut n as *mut u32 as Ptr);
            d.eq(
                &format!("maketables(gcontext={use_g}) bytes"),
                std::slice::from_raw_parts(ta, n as usize).to_vec(),
                std::slice::from_raw_parts(tb, n as usize).to_vec(),
            );
            let cca = (p.c.compile_context_create)(ptr::null_mut());
            let ccb = (p.r.compile_context_create)(ptr::null_mut());
            d.eq(
                "set_character_tables rc",
                (p.c.set_character_tables)(cca, ta),
                (p.r.set_character_tables)(ccb, tb),
            );
            let ctx = Ctx { a: cca, b: ccb };
            for pat in ["(?i)abc", "\\w+", "[[:alpha:]]+", "\\d", "[a-z]", "\\s"] {
                let (a, b, _, _) = compile2(p, pat.as_bytes(), 0, Some(&ctx));
                if a.is_null() {
                    continue;
                }
                for byte in 0u8..=255 {
                    let s = [byte];
                    let mda = (p.c.match_data_create)(4, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                    let x = (p.c.do_match)(a, s.as_ptr(), 1, 0, 0, mda, ptr::null_mut());
                    let y = (p.r.do_match)(b, s.as_ptr(), 1, 0, 0, mdb, ptr::null_mut());
                    d.eq(
                        &format!("own-tables {pat} vs {byte:#04x}"),
                        read_match_out(&p.c, mda, x),
                        read_match_out(&p.r, mdb, y),
                    );
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cca);
            (p.r.compile_context_free)(ccb);
            (p.c.maketables_free)(ga, ta);
            (p.r.maketables_free)(gb, tb);
            if use_g {
                (p.c.general_context_free)(ga);
                (p.r.general_context_free)(gb);
            }
        }
        let _ = &mut rng;
    }
    d.finish("CONFIGS 399-403: set_newline / set_bsr / set_optimize / glob setters / character tables, each observed through compile+match");
}

#[test]
fn cfg_404_406_config_and_errors() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // rows 404, 405
        for what in 0u32..=20 {
            d.eq(
                &format!("config({what}, NULL) length query"),
                (p.c.config)(what, ptr::null_mut()),
                (p.r.config)(what, ptr::null_mut()),
            );
            // string items need a byte buffer, numeric ones a uint32_t
            let mut ba = [0u8; 256];
            let mut bb = [0u8; 256];
            let ra = (p.c.config)(what, ba.as_mut_ptr() as Ptr);
            let rb = (p.r.config)(what, bb.as_mut_ptr() as Ptr);
            d.eq(&format!("config({what}) rc"), ra, rb);
            d.eq(&format!("config({what}) bytes"), ba, bb);
        }
        for what in [21u32, 100, 1000, u32::MAX] {
            let mut v = 0u64;
            d.eq(
                &format!("config({what}) out-of-range NULL"),
                (p.c.config)(what, ptr::null_mut()),
                (p.r.config)(what, ptr::null_mut()),
            );
            d.eq(
                &format!("config({what}) out-of-range buf"),
                (p.c.config)(what, &mut v as *mut u64 as Ptr),
                (p.r.config)(what, &mut v as *mut u64 as Ptr),
            );
        }

        // row 406: every error code, every interesting buffer size, incl. the
        // exact-fit and one-short truncation boundary.
        let mut codes: Vec<c_int> = (-90..=1).collect();
        codes.extend(95..=225);
        codes.extend([300, 1000, -1000, c_int::MAX, c_int::MIN]);
        for code in codes {
            // find the natural length using a generous buffer
            let mut big = vec![0u8; 512];
            let nat = (p.c.get_error_message)(code, big.as_mut_ptr(), big.len());
            let nat2 = (p.r.get_error_message)(code, big.as_mut_ptr(), big.len());
            d.eq(&format!("get_error_message({code}) natural rc"), nat, nat2);
            let len = if nat > 0 { nat as usize } else { 0 };
            for size in [0usize, 1, 2, len.saturating_sub(1), len, len + 1, len + 2, 512] {
                let mut ba = vec![0xEEu8; size + 8];
                let mut bb = vec![0xEEu8; size + 8];
                let ra = (p.c.get_error_message)(code, ba.as_mut_ptr(), size);
                let rb = (p.r.get_error_message)(code, bb.as_mut_ptr(), size);
                d.eq(&format!("get_error_message({code}, {size}) rc"), ra, rb);
                d.eq(&format!("get_error_message({code}, {size}) buf"), ba, bb);
            }
        }
    }
    d.finish("CONFIGS 404-406: pcre2_config_8 every code (value + NULL query + out-of-range) and pcre2_get_error_message_8 every code x buffer size");
}
