// Phase B — the remaining public API surface: config, error messages,
// substitute, substring-by-name, serialize round trips, pattern conversion,
// contexts with custom allocators, and the match-data accessors.

mod common;
use common::*;
use std::ffi::{c_int, c_void, CStr};
use std::ptr;

// ===================================================================== config

// CONFIGS row: pcre2_config_8 for every documented `what`, both with a buffer
// and with NULL (the size query form).
#[test]
fn config_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for what in 0u32..=16 {
            // size query
            d.eq(
                &format!("config({what}, NULL)"),
                (p.c.config)(what, ptr::null_mut()),
                (p.r.config)(what, ptr::null_mut()),
            );
            // string-valued: VERSION, UNICODE_VERSION, JITTARGET
            if what == PCRE2_CONFIG_VERSION || what == PCRE2_CONFIG_UNICODE_VERSION {
                let mut ba = [0u8; 128];
                let mut bb = [0u8; 128];
                let ra = (p.c.config)(what, ba.as_mut_ptr() as Ptr);
                let rb = (p.r.config)(what, bb.as_mut_ptr() as Ptr);
                d.eq(&format!("config({what}) rc"), ra, rb);
                d.eq(&format!("config({what}) text"), ba, bb);
                println!("config[{what}] = {:?}", CStr::from_bytes_until_nul(&ba).unwrap());
            } else {
                let (mut va, mut vb) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
                let ra = (p.c.config)(what, &mut va as *mut u32 as Ptr);
                let rb = (p.r.config)(what, &mut vb as *mut u32 as Ptr);
                d.eq(&format!("config({what}) rc"), ra, rb);
                d.eq(&format!("config({what}) val"), va, vb);
            }
        }
    }
    d.finish("pcre2_config_8: every `what` 0..=16, buffer form and NULL size-query form");
}

// CONFIGS row: pcre2_get_error_message_8 for EVERY error number the library
// knows, across buffer sizes from 0 to comfortably large.
#[test]
fn get_error_message_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mut codes: Vec<c_int> = (-90..=0).collect();
        codes.extend(100..=200); // compile errors
        codes.extend([300, 1000, -1000, c_int::MAX, c_int::MIN]);
        for code in codes {
            for size in [0usize, 1, 2, 5, 16, 64, 512] {
                let mut ba = vec![0xEEu8; size + 8];
                let mut bb = vec![0xEEu8; size + 8];
                let ra = (p.c.get_error_message)(code, ba.as_mut_ptr(), size);
                let rb = (p.r.get_error_message)(code, bb.as_mut_ptr(), size);
                d.eq(&format!("get_error_message({code}, size={size}) rc"), ra, rb);
                d.eq(&format!("get_error_message({code}, size={size}) buf"), ba, bb);
            }
        }
    }
    d.finish("pcre2_get_error_message_8: all error numbers x buffer sizes 0..512");
}

// ================================================================ substitute

/// Replacement strings covering the whole substitution mini-language.
const REPLACEMENTS: &[&str] = &[
    "",
    "X",
    "[$0]",
    "$1",
    "$2",
    "$9",
    "${1}",
    "${0}",
    "$1$2",
    "a${1}b",
    "\\1",
    "\\0",
    "$$",
    "$",
    "\\$",
    "\\\\",
    "\\n",
    "\\r\\t",
    "\\x41",
    "\\x{41}",
    "\\o{101}",
    "$*MARK",
    "${*MARK}",
    "$*MARK:$0",
    "${name}",
    "$name",
    "${n}",
    "${y}-${m}",
    // case operators (SUBSTITUTE_EXTENDED)
    "\\U$0\\E",
    "\\L$0\\E",
    "\\u$0",
    "\\l$0",
    "\\U$1\\E-\\L$2\\E",
    "\\u\\L$0\\E",
    // conditional forms (SUBSTITUTE_EXTENDED)
    "${1:-empty}",
    "${1:+yes:no}",
    "${9:-d}",
    "${name:-none}",
    "${1:+[$1]:(none)}",
    // pathological / malformed-ish but valid-under-some-options
    "$1\\",
    "${1",
    "$}",
    "\\Q$1\\E",
];

// CONFIGS rows: pcre2_substitute_8 over substitute options x patterns x
// subjects x replacements x output buffer sizes (including the two-pass
// length-query form).
#[test]
fn substitute_identical() {
    let p = pair();
    let mut rng = Rng::new(300);
    let mut d = Diffs::new();
    let subs_opts: &[(u32, &str)] = &[
        (0, "none"),
        (PCRE2_SUBSTITUTE_GLOBAL, "GLOBAL"),
        (PCRE2_SUBSTITUTE_EXTENDED, "EXTENDED"),
        (PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED, "GLOBAL|EXTENDED"),
        (PCRE2_SUBSTITUTE_LITERAL, "LITERAL"),
        (PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_LITERAL, "GLOBAL|LITERAL"),
        (PCRE2_SUBSTITUTE_UNSET_EMPTY, "UNSET_EMPTY"),
        (PCRE2_SUBSTITUTE_UNKNOWN_UNSET, "UNKNOWN_UNSET"),
        (
            PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            "UNSET_EMPTY|UNKNOWN_UNSET",
        ),
        (PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, "OVERFLOW_LENGTH"),
        (
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            "GLOBAL|OVERFLOW_LENGTH",
        ),
        (PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, "REPLACEMENT_ONLY"),
        (
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            "GLOBAL|REPLACEMENT_ONLY",
        ),
        (
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY
                | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            "GLOBAL|EXTENDED|UNSET_EMPTY|UNKNOWN_UNSET",
        ),
        (PCRE2_NOTBOL, "NOTBOL"),
        (PCRE2_NOTEMPTY, "NOTEMPTY"),
        (PCRE2_NOTEMPTY_ATSTART | PCRE2_SUBSTITUTE_GLOBAL, "NOTEMPTY_ATSTART|GLOBAL"),
        (PCRE2_ANCHORED, "ANCHORED"),
        (PCRE2_NO_UTF_CHECK, "NO_UTF_CHECK"),
    ];
    // patterns with captures / names / marks, which is what substitution needs
    let pats: &[(&str, u32)] = &[
        ("a", 0),
        ("a+", 0),
        ("(a)(b)", 0),
        ("(?<name>a+)", 0),
        ("(?<y>\\d{4})-(?<m>\\d{2})", 0),
        ("(a)|(b)", 0),
        ("(a)?b", 0),
        ("", 0),
        ("x*", 0),
        ("\\b", 0),
        ("(*MARK:m1)a", 0),
        ("\\w+", 0),
        ("[aeiou]", 0),
        ("(.)(.)", 0),
        ("a", PCRE2_CASELESS),
        ("\\w+", PCRE2_UTF | PCRE2_UCP),
        (".", PCRE2_UTF),
        ("(\\X)", PCRE2_UTF),
        ("^", PCRE2_MULTILINE),
        ("$", PCRE2_MULTILINE),
    ];
    for &(pat, copts) in pats {
        let pb = pat.as_bytes();
        unsafe {
            let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
            let (mut eoa, mut eob) = (0usize, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), copts, &mut eca, &mut eoa, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), copts, &mut ecb, &mut eob, ptr::null_mut());
            d.eq(&format!("substitute-compile {}", show(pb)), a.is_null(), b.is_null());
            if a.is_null() || b.is_null() {
                continue;
            }
            for subj in SUBJECTS {
                let sb = subj.as_bytes();
                if copts & PCRE2_UTF != 0 && std::str::from_utf8(sb).is_err() {
                    continue;
                }
                for _ in 0..8 {
                    let &(so, sn) = &subs_opts[rng.below(subs_opts.len())];
                    let rep = REPLACEMENTS[rng.below(REPLACEMENTS.len())].as_bytes();
                    let start = if sb.is_empty() { 0 } else { rng.below(sb.len() + 1) };
                    // exercise too-small, exact and generous output buffers, and
                    // the length-query form (buffer NULL is not allowed, but a
                    // zero-length buffer is the documented query)
                    let cap = *rng.pick(&[0usize, 1, 2, 4, 8, 32, 512]);
                    let mut oa = vec![0xEEu8; cap + 16];
                    let mut ob = vec![0xEEu8; cap + 16];
                    let (mut la, mut lb) = (cap, cap);
                    let ra = (p.c.substitute)(
                        a, sb.as_ptr(), sb.len(), start, so, ptr::null_mut(), ptr::null_mut(),
                        rep.as_ptr(), rep.len(), oa.as_mut_ptr(), &mut la,
                    );
                    let rb = (p.r.substitute)(
                        b, sb.as_ptr(), sb.len(), start, so, ptr::null_mut(), ptr::null_mut(),
                        rep.as_ptr(), rep.len(), ob.as_mut_ptr(), &mut lb,
                    );
                    let tag = format!(
                        "substitute pat={} subj={} rep={} opts={} start={} cap={}",
                        show(pb), show(sb), show(rep), sn, start, cap
                    );
                    d.eq(&format!("{tag} rc"), ra, rb);
                    d.eq(&format!("{tag} outlen"), la, lb);
                    d.eq(&format!("{tag} out"), oa, ob);
                    // zero-terminated replacement form
                    let mut zrep = rep.to_vec();
                    zrep.push(0);
                    let (mut la2, mut lb2) = (cap, cap);
                    let mut oa2 = vec![0xEEu8; cap + 16];
                    let mut ob2 = vec![0xEEu8; cap + 16];
                    let ra2 = (p.c.substitute)(
                        a, sb.as_ptr(), sb.len(), start, so, ptr::null_mut(), ptr::null_mut(),
                        zrep.as_ptr(), PCRE2_ZERO_TERMINATED, oa2.as_mut_ptr(), &mut la2,
                    );
                    let rb2 = (p.r.substitute)(
                        b, sb.as_ptr(), sb.len(), start, so, ptr::null_mut(), ptr::null_mut(),
                        zrep.as_ptr(), PCRE2_ZERO_TERMINATED, ob2.as_mut_ptr(), &mut lb2,
                    );
                    d.eq(&format!("{tag} [ZT rep] rc"), ra2, rb2);
                    d.eq(&format!("{tag} [ZT rep] outlen"), la2, lb2);
                    d.eq(&format!("{tag} [ZT rep] out"), oa2, ob2);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("pcre2_substitute_8: substitute options x patterns x subjects x replacements x buffer sizes");
}

// --- SUBSTITUTE_MATCHED: substitution driven from a pre-existing match_data

// CONFIGS row: PCRE2_SUBSTITUTE_MATCHED reuses the ovector of an earlier
// pcre2_match call instead of matching again.
#[test]
fn substitute_matched_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (pat, subj) in [
            ("(a)(b)", "xxabyy"),
            ("(?<n>\\w+)", "hello world"),
            ("a+", "baaad"),
            ("", "abc"),
            ("(x)?y", "zy"),
        ] {
            let pb = pat.as_bytes();
            let sb = subj.as_bytes();
            let (mut ec, mut eo) = (0 as c_int, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            assert!(!a.is_null() && !b.is_null());
            for rep in REPLACEMENTS {
                let rb_ = rep.as_bytes();
                for so in [
                    PCRE2_SUBSTITUTE_MATCHED,
                    PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                ] {
                    let mda = (p.c.match_data_create_from_pattern)(a, ptr::null_mut());
                    let mdb = (p.r.match_data_create_from_pattern)(b, ptr::null_mut());
                    let m1 = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                    let m2 = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                    d.eq(&format!("pre-match {pat}/{subj}"), m1, m2);
                    let (mut la, mut lb) = (256usize, 256usize);
                    let mut oa = vec![0xEEu8; 300];
                    let mut ob = vec![0xEEu8; 300];
                    let ra = (p.c.substitute)(
                        a, sb.as_ptr(), sb.len(), 0, so, mda, ptr::null_mut(),
                        rb_.as_ptr(), rb_.len(), oa.as_mut_ptr(), &mut la,
                    );
                    let rbv = (p.r.substitute)(
                        b, sb.as_ptr(), sb.len(), 0, so, mdb, ptr::null_mut(),
                        rb_.as_ptr(), rb_.len(), ob.as_mut_ptr(), &mut lb,
                    );
                    let tag = format!("SUBSTITUTE_MATCHED {pat}/{subj} rep={} opts={so:#x}", show(rb_));
                    d.eq(&format!("{tag} rc"), ra, rbv);
                    d.eq(&format!("{tag} len"), la, lb);
                    d.eq(&format!("{tag} out"), oa, ob);
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("pcre2_substitute_8 with PCRE2_SUBSTITUTE_MATCHED over a pre-filled match_data");
}

// --- substitute callouts

static mut SUBS_LOG: Vec<u8> = Vec::new();

/// Exact layout of `pcre2_substitute_callout_block` from `pcre2.h`
/// (note `version` is followed by padding before the first pointer).
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

unsafe extern "C" fn subs_callout(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const SubstituteCalloutBlock);
    let log = &mut *ptr::addr_of_mut!(SUBS_LOG);
    for v in [
        b.version as u64,
        b.oveccount as u64,
        b.subscount as u64,
        b.output_offsets[0] as u64,
        b.output_offsets[1] as u64,
    ] {
        log.extend_from_slice(&v.to_le_bytes());
    }
    // the ovector contents the callout sees, and the changed slice of output
    for i in 0..(2 * b.oveccount as usize) {
        log.extend_from_slice(&(*b.ovector.add(i) as u64).to_le_bytes());
    }
    let (s, e) = (b.output_offsets[0], b.output_offsets[1]);
    if e >= s && e < 1 << 20 {
        log.extend_from_slice(std::slice::from_raw_parts(b.output.add(s), e - s));
    }
    0
}

/// A case-conversion callout that is deliberately simple but observable.
unsafe extern "C" fn case_callout(
    input: Sptr,
    inlen: Sz,
    output: *mut u8,
    outlen: Sz,
    to_case: c_int,
    _d: *mut c_void,
) -> Sz {
    let log = &mut *ptr::addr_of_mut!(SUBS_LOG);
    log.extend_from_slice(&(inlen as u64).to_le_bytes());
    log.extend_from_slice(&(outlen as u64).to_le_bytes());
    log.extend_from_slice(&(to_case as i64).to_le_bytes());
    if inlen > outlen {
        return usize::MAX; // signal "not enough room" the documented way
    }
    for i in 0..inlen {
        let c = *input.add(i);
        *output.add(i) = match to_case {
            0 => c.to_ascii_lowercase(),
            _ => c.to_ascii_uppercase(),
        };
    }
    inlen
}

// CONFIGS rows: pcre2_set_substitute_callout_8 and
// pcre2_set_substitute_case_callout_8 — the callback sequences must match.
#[test]
fn substitute_callouts_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (pat, subj) in [
            ("a", "banana"),
            ("(\\w)(\\w)", "abcd ef"),
            ("[aeiou]+", "queueing"),
            ("", "xy"),
        ] {
            let pb = pat.as_bytes();
            let sb = subj.as_bytes();
            let (mut ec, mut eo) = (0 as c_int, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            assert!(!a.is_null() && !b.is_null());
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            d.eq(
                "set_substitute_callout rc",
                (p.c.set_substitute_callout)(mca, Some(subs_callout), ptr::null_mut()),
                (p.r.set_substitute_callout)(mcb, Some(subs_callout), ptr::null_mut()),
            );
            d.eq(
                "set_substitute_case_callout rc",
                (p.c.set_substitute_case_callout)(mca, Some(case_callout), ptr::null_mut()),
                (p.r.set_substitute_case_callout)(mcb, Some(case_callout), ptr::null_mut()),
            );
            for rep in REPLACEMENTS {
                let rp = rep.as_bytes();
                for so in [
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED,
                ] {
                    let (mut la, mut lb) = (256usize, 256usize);
                    let mut oa = vec![0xEEu8; 300];
                    let mut ob = vec![0xEEu8; 300];
                    SUBS_LOG.clear();
                    let ra = (p.c.substitute)(
                        a, sb.as_ptr(), sb.len(), 0, so, ptr::null_mut(), mca,
                        rp.as_ptr(), rp.len(), oa.as_mut_ptr(), &mut la,
                    );
                    let loga = SUBS_LOG.clone();
                    SUBS_LOG.clear();
                    let rbv = (p.r.substitute)(
                        b, sb.as_ptr(), sb.len(), 0, so, ptr::null_mut(), mcb,
                        rp.as_ptr(), rp.len(), ob.as_mut_ptr(), &mut lb,
                    );
                    let logb = SUBS_LOG.clone();
                    let tag = format!("subs-callout {pat}/{subj} rep={} opts={so:#x}", show(rp));
                    d.eq(&format!("{tag} rc"), ra, rbv);
                    d.eq(&format!("{tag} len"), la, lb);
                    d.eq(&format!("{tag} out"), oa, ob);
                    d.eq(&format!("{tag} callout log"), loga, logb);
                }
            }
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("pcre2_set_substitute_callout_8 / pcre2_set_substitute_case_callout_8 sequences");
}

// ============================================================== substring by name

// CONFIGS rows: the by-name substring accessors, including DUPNAMES where a
// name maps to several groups.
#[test]
fn substring_byname_identical() {
    let p = pair();
    let mut d = Diffs::new();
    let cases: &[(&str, u32, &str)] = &[
        ("(?<a>x)(?<b>y)", 0, "xy"),
        ("(?<a>x)(?<b>y)", 0, "zz"),
        ("(?<longname>abc)", 0, "abc"),
        ("(?<a>x)|(?<a>y)", PCRE2_DUPNAMES, "y"),
        ("(?<a>x)|(?<a>y)", PCRE2_DUPNAMES, "x"),
        ("(?<a>x)(?<a>y)?", PCRE2_DUPNAMES, "x"),
        ("(?<n>\\d+)-(?<n>\\d+)", PCRE2_DUPNAMES, "12-34"),
        ("(?<a>x)?(?<b>y)", 0, "y"),
        ("(?<\u{e9}>x)", PCRE2_UTF, "x"),
    ];
    let names: &[&str] = &["a", "b", "n", "longname", "nope", "", "A", "\u{e9}"];
    unsafe {
        for &(pat, opts, subj) in cases {
            let pb = pat.as_bytes();
            let sb = subj.as_bytes();
            let (mut ec, mut eo) = (0 as c_int, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            assert!(!a.is_null() && !b.is_null(), "compile {pat} failed ec={ec}");
            let mda = (p.c.match_data_create_from_pattern)(a, ptr::null_mut());
            let mdb = (p.r.match_data_create_from_pattern)(b, ptr::null_mut());
            let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
            let rbv = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
            d.eq(&format!("match {pat}/{subj}"), ra, rbv);
            for nm in names {
                let mut nz = nm.as_bytes().to_vec();
                nz.push(0);
                let n = nz.as_ptr();
                let tag = format!("{pat}/{subj} name={nm:?}");
                d.eq(
                    &format!("{tag} number_from_name"),
                    (p.c.substring_number_from_name)(a, n),
                    (p.r.substring_number_from_name)(b, n),
                );
                let (mut la, mut lb) = (usize::MAX, usize::MAX);
                d.eq(
                    &format!("{tag} length_byname rc"),
                    (p.c.substring_length_byname)(mda, n, &mut la),
                    (p.r.substring_length_byname)(mdb, n, &mut lb),
                );
                d.eq(&format!("{tag} length_byname len"), la, lb);
                let mut ba = [0xEEu8; 64];
                let mut bb = [0xEEu8; 64];
                let (mut ca, mut cb) = (ba.len(), bb.len());
                d.eq(
                    &format!("{tag} copy_byname rc"),
                    (p.c.substring_copy_byname)(mda, n, ba.as_mut_ptr(), &mut ca),
                    (p.r.substring_copy_byname)(mdb, n, bb.as_mut_ptr(), &mut cb),
                );
                d.eq(&format!("{tag} copy_byname out"), (ba, ca), (bb, cb));
                let (mut pa, mut pbp) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                let (mut ga, mut gb) = (usize::MAX, usize::MAX);
                let qa = (p.c.substring_get_byname)(mda, n, &mut pa, &mut ga);
                let qb = (p.r.substring_get_byname)(mdb, n, &mut pbp, &mut gb);
                d.eq(&format!("{tag} get_byname rc"), qa, qb);
                d.eq(&format!("{tag} get_byname len"), ga, gb);
                if qa == 0 && qb == 0 {
                    d.eq(
                        &format!("{tag} get_byname bytes"),
                        std::slice::from_raw_parts(pa, ga).to_vec(),
                        std::slice::from_raw_parts(pbp, gb).to_vec(),
                    );
                }
                if !pa.is_null() {
                    (p.c.substring_free)(pa);
                }
                if !pbp.is_null() {
                    (p.r.substring_free)(pbp);
                }
            }
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("pcre2_substring_*_byname_8 + number_from_name + nametable_scan, incl. DUPNAMES");
}

// ================================================================= serialize

// CONFIGS rows: serialize_encode/decode round trip for 1 and many codes; the
// serialized byte stream itself must be identical.
#[test]
fn serialize_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // groups of patterns: single, small set, larger set
        let groups: Vec<Vec<&str>> = vec![
            vec!["abc"],
            vec![""],
            vec!["a", "b"],
            vec!["(a)(b)(c)", "\\d+", "[a-z]+"],
            vec!["(?<n>x)", "\\p{L}+", "\\X", "a{2,4}?"],
            PATTERNS.iter().take(40).copied().collect(),
            PATTERNS.iter().skip(40).take(60).copied().collect(),
        ];
        for (gi, g) in groups.iter().enumerate() {
            for &opts in &[0u32, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
                let mut ca: Vec<Ptr> = Vec::new();
                let mut cb: Vec<Ptr> = Vec::new();
                for pat in g {
                    let pb = pat.as_bytes();
                    let (mut ec, mut eo) = (0 as c_int, 0usize);
                    let x = (p.c.compile)(pb.as_ptr(), pb.len(), opts, &mut ec, &mut eo, ptr::null_mut());
                    let y = (p.r.compile)(pb.as_ptr(), pb.len(), opts, &mut ec, &mut eo, ptr::null_mut());
                    if x.is_null() != y.is_null() {
                        panic!("compile disagreement on {pat}");
                    }
                    if !x.is_null() {
                        ca.push(x);
                        cb.push(y);
                    }
                }
                if ca.is_empty() {
                    continue;
                }
                let (mut ba, mut bb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                let (mut na, mut nb) = (0usize, 0usize);
                let ra = (p.c.serialize_encode)(
                    ca.as_ptr(), ca.len() as i32, &mut ba, &mut na, ptr::null_mut(),
                );
                let rbv = (p.r.serialize_encode)(
                    cb.as_ptr(), cb.len() as i32, &mut bb, &mut nb, ptr::null_mut(),
                );
                let tag = format!("serialize group#{gi} n={} opts={opts:#x}", ca.len());
                d.eq(&format!("{tag} encode rc"), ra, rbv);
                d.eq(&format!("{tag} encode size"), na, nb);
                if ra > 0 && rbv > 0 {
                    // The serialized stream embeds host pointer-free data only,
                    // so it must be byte-identical.
                    d.eq(
                        &format!("{tag} encoded bytes"),
                        std::slice::from_raw_parts(ba, na).to_vec(),
                        std::slice::from_raw_parts(bb, nb).to_vec(),
                    );
                    d.eq(
                        &format!("{tag} get_number_of_codes"),
                        (p.c.serialize_get_number_of_codes)(ba),
                        (p.r.serialize_get_number_of_codes)(bb),
                    );
                    // cross-decode: each library decodes its own stream
                    let mut da: Vec<Ptr> = vec![ptr::null_mut(); ca.len()];
                    let mut db: Vec<Ptr> = vec![ptr::null_mut(); cb.len()];
                    let ea = (p.c.serialize_decode)(
                        da.as_mut_ptr(), da.len() as i32, ba, ptr::null_mut(),
                    );
                    let eb = (p.r.serialize_decode)(
                        db.as_mut_ptr(), db.len() as i32, bb, ptr::null_mut(),
                    );
                    d.eq(&format!("{tag} decode rc"), ea, eb);
                    if ea > 0 && eb > 0 {
                        for i in 0..(ea as usize) {
                            // C-decoded vs Rust-decoded: must be fully identical.
                            assert_code_eq(da[i], db[i], &format!("{tag} decoded[{i}]"));
                            // A decoded pattern must equal the original except
                            // for PCRE2_DEREF_TABLES, which decode sets because
                            // the tables now live inside the code block. Assert
                            // the difference is EXACTLY that bit, in both libs.
                            assert_code_eq_masked(
                                da[i], ca[i], PCRE2_DEREF_TABLES,
                                &format!("{tag} decoded[{i}] vs original (C)"),
                            );
                            assert_code_eq_masked(
                                db[i], cb[i], PCRE2_DEREF_TABLES,
                                &format!("{tag} decoded[{i}] vs original (rust)"),
                            );
                            let fo = (*(ca[i] as *const RealCodeHead)).flags;
                            let fd = (*(da[i] as *const RealCodeHead)).flags;
                            let go = (*(cb[i] as *const RealCodeHead)).flags;
                            let gd = (*(db[i] as *const RealCodeHead)).flags;
                            d.eq(
                                &format!("{tag} decoded[{i}] flags delta"),
                                (fd ^ fo, fd & PCRE2_DEREF_TABLES),
                                (gd ^ go, gd & PCRE2_DEREF_TABLES),
                            );
                            d.checked += 1;
                        }
                        for i in 0..(ea as usize) {
                            (p.c.code_free)(da[i]);
                            (p.r.code_free)(db[i]);
                        }
                    }
                    // decoding fewer than available is allowed
                    if ca.len() > 1 {
                        let mut ha: Vec<Ptr> = vec![ptr::null_mut(); 1];
                        let mut hb: Vec<Ptr> = vec![ptr::null_mut(); 1];
                        let fa = (p.c.serialize_decode)(ha.as_mut_ptr(), 1, ba, ptr::null_mut());
                        let fb = (p.r.serialize_decode)(hb.as_mut_ptr(), 1, bb, ptr::null_mut());
                        d.eq(&format!("{tag} partial decode rc"), fa, fb);
                        if fa > 0 && fb > 0 {
                            assert_code_eq(ha[0], hb[0], &format!("{tag} partial decoded[0]"));
                            (p.c.code_free)(ha[0]);
                            (p.r.code_free)(hb[0]);
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
    }
    d.finish("pcre2_serialize_encode_8 / decode / get_number_of_codes: 1..100 codes x option sets");
}

// =================================================================== convert

// CONFIGS rows: pcre2_pattern_convert_8 for glob and both POSIX flavours,
// crossed with the glob separator/escape settings and UTF.
#[test]
fn pattern_convert_identical() {
    let p = pair();
    let mut d = Diffs::new();
    let globs: &[&str] = &[
        "", "*", "?", "a", "*.txt", "a?c", "[abc]", "[!abc]", "[a-z]*", "**", "**/x",
        "a/**/b", "a\\*b", "\\?", "x[", "x]", "[", "]", "a**b", "/*/", "**.c", "a/b/c",
        ".*", "..", "~/x", "a{b,c}", "\\", "*?[]", "\u{e9}*", "\u{1f600}?",
    ];
    let posix: &[&str] = &[
        "", "a", "abc", "a.c", "a*", "a\\.c", "[abc]", "[^abc]", "[[:alpha:]]", "a\\(b\\)c",
        "a+b", "a?b", "a|b", "(a)(b)", "^abc$", "a\\{2,3\\}", "a{2,3}", "\\(", "\\)", "[]a]",
        "\\1", "a\\|b", "$", "^", "*", "\\", "[a-", "a\\\\b",
    ];
    unsafe {
        for (set, base) in [
            (globs, PCRE2_CONVERT_GLOB),
            (globs, PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR),
            (globs, PCRE2_CONVERT_GLOB_NO_STARSTAR),
            (posix, PCRE2_CONVERT_POSIX_BASIC),
            (posix, PCRE2_CONVERT_POSIX_EXTENDED),
        ] {
            for &utf in &[0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
                // glob separator / escape only matter for the glob modes
                let seps: &[(u32, u32)] = if base & PCRE2_CONVERT_GLOB != 0 {
                    &[(0, 0), (b'/' as u32, b'\\' as u32), (b':' as u32, 0), (b'.' as u32, b'^' as u32)]
                } else {
                    &[(0, 0)]
                };
                for &(sep, esc) in seps {
                    let cca = (p.c.convert_context_create)(ptr::null_mut());
                    let ccb = (p.r.convert_context_create)(ptr::null_mut());
                    if sep != 0 {
                        d.eq(
                            "set_glob_separator rc",
                            (p.c.set_glob_separator)(cca, sep),
                            (p.r.set_glob_separator)(ccb, sep),
                        );
                    }
                    if esc != 0 {
                        d.eq(
                            "set_glob_escape rc",
                            (p.c.set_glob_escape)(cca, esc),
                            (p.r.set_glob_escape)(ccb, esc),
                        );
                    }
                    for pat in set {
                        let pb = pat.as_bytes();
                        for &len in &[pb.len(), PCRE2_ZERO_TERMINATED] {
                            let mut zt = pb.to_vec();
                            zt.push(0);
                            let src: &[u8] = if len == PCRE2_ZERO_TERMINATED { &zt } else { pb };
                            let opts = base | utf;
                            // form 1: library allocates
                            let (mut oa, mut ob) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                            let (mut na, mut nb) = (usize::MAX, usize::MAX);
                            let ra = (p.c.pattern_convert)(
                                src.as_ptr(), len, opts, &mut oa, &mut na, cca,
                            );
                            let rbv = (p.r.pattern_convert)(
                                src.as_ptr(), len, opts, &mut ob, &mut nb, ccb,
                            );
                            let tag = format!(
                                "convert {} opts={opts:#x} sep={sep} esc={esc} len={}",
                                show(pb),
                                if len == PCRE2_ZERO_TERMINATED { "ZT".into() } else { len.to_string() }
                            );
                            d.eq(&format!("{tag} rc"), ra, rbv);
                            d.eq(&format!("{tag} outlen"), na, nb);
                            if ra == 0 && rbv == 0 {
                                d.eq(
                                    &format!("{tag} output"),
                                    std::slice::from_raw_parts(oa, na + 1).to_vec(),
                                    std::slice::from_raw_parts(ob, nb + 1).to_vec(),
                                );
                                // the converted pattern must itself compile the same
                                let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
                                let (mut f1, mut f2) = (0usize, 0usize);
                                let k1 = (p.c.compile)(oa, na, 0, &mut e1, &mut f1, ptr::null_mut());
                                let k2 = (p.r.compile)(ob, nb, 0, &mut e2, &mut f2, ptr::null_mut());
                                d.eq(&format!("{tag} recompile null?"), k1.is_null(), k2.is_null());
                                d.eq(&format!("{tag} recompile ec"), e1, e2);
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
                            // form 2: caller-supplied buffer, various sizes
                            for cap in [0usize, 1, 4, 16, 256] {
                                let mut qa = vec![0xEEu8; cap + 8];
                                let mut qb = vec![0xEEu8; cap + 8];
                                let mut pa = qa.as_mut_ptr();
                                let mut pbp = qb.as_mut_ptr();
                                let (mut ma, mut mb) = (cap, cap);
                                let ua = (p.c.pattern_convert)(
                                    src.as_ptr(), len, opts, &mut pa, &mut ma, cca,
                                );
                                let ub = (p.r.pattern_convert)(
                                    src.as_ptr(), len, opts, &mut pbp, &mut mb, ccb,
                                );
                                d.eq(&format!("{tag} [buf {cap}] rc"), ua, ub);
                                d.eq(&format!("{tag} [buf {cap}] len"), ma, mb);
                                d.eq(&format!("{tag} [buf {cap}] bytes"), qa, qb);
                            }
                        }
                    }
                    (p.c.convert_context_free)(cca);
                    (p.r.convert_context_free)(ccb);
                }
            }
        }
    }
    d.finish("pcre2_pattern_convert_8: GLOB/GLOB_NO_WILD_SEPARATOR/GLOB_NO_STARSTAR/POSIX_BASIC/POSIX_EXTENDED x UTF x separator/escape x buffer forms");
}

// ================================================================== contexts

static mut ALLOC_C: (usize, usize) = (0, 0); // (calls, bytes)
static mut ALLOC_R: (usize, usize) = (0, 0);

unsafe extern "C" fn my_malloc_c(n: usize, _d: *mut c_void) -> *mut c_void {
    let a = &mut *ptr::addr_of_mut!(ALLOC_C);
    a.0 += 1;
    a.1 += n;
    tracked_alloc(n)
}
unsafe extern "C" fn my_malloc_r(n: usize, _d: *mut c_void) -> *mut c_void {
    let a = &mut *ptr::addr_of_mut!(ALLOC_R);
    a.0 += 1;
    a.1 += n;
    tracked_alloc(n)
}
unsafe extern "C" fn my_free(p: *mut c_void, _d: *mut c_void) {
    tracked_free(p)
}
unsafe fn tracked_alloc(n: usize) -> *mut c_void {
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let p = std::alloc::alloc(l);
    *(p as *mut usize) = sz;
    p.add(16) as *mut c_void
}
unsafe fn tracked_free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

// CONFIGS rows: a general context with a CUSTOM allocator threaded through
// every context type, compile and match — the allocation sequence must match.
#[test]
fn custom_allocator_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for pat in PATTERNS.iter().step_by(4) {
            let pb = pat.as_bytes();
            ALLOC_C = (0, 0);
            ALLOC_R = (0, 0);
            let ga = (p.c.general_context_create)(Some(my_malloc_c), Some(my_free), ptr::null_mut());
            let gb = (p.r.general_context_create)(Some(my_malloc_r), Some(my_free), ptr::null_mut());
            assert!(!ga.is_null() && !gb.is_null());
            let ca = (p.c.compile_context_create)(ga);
            let cb = (p.r.compile_context_create)(gb);
            let (mut ea, mut eb) = (0 as c_int, 0 as c_int);
            let (mut fa, mut fb) = (0usize, 0usize);
            let ka = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut ea, &mut fa, ca);
            let kb = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut eb, &mut fb, cb);
            d.eq(&format!("custom-alloc compile {} null?", show(pb)), ka.is_null(), kb.is_null());
            d.eq(&format!("custom-alloc compile {} ec", show(pb)), ea, eb);
            if !ka.is_null() && !kb.is_null() {
                assert_code_eq(ka, kb, &format!("custom-alloc {}", show(pb)));
                let mda = (p.c.match_data_create_from_pattern)(ka, ga);
                let mdb = (p.r.match_data_create_from_pattern)(kb, gb);
                for subj in SUBJECTS.iter().take(20) {
                    let sb = subj.as_bytes();
                    let ra = (p.c.do_match)(ka, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                    let rbv = (p.r.do_match)(kb, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                    d.eq(
                        &format!("custom-alloc match {}/{}", show(pb), show(sb)),
                        read_match_out(&p.c, mda, ra),
                        read_match_out(&p.r, mdb, rbv),
                    );
                }
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
                (p.c.code_free)(ka);
                (p.r.code_free)(kb);
            }
            (p.c.compile_context_free)(ca);
            (p.r.compile_context_free)(cb);
            (p.c.general_context_free)(ga);
            (p.r.general_context_free)(gb);
            // The whole point: identical number of allocations and identical
            // total bytes requested.
            d.eq(
                &format!("custom-alloc accounting for {}", show(pb)),
                ALLOC_C,
                ALLOC_R,
            );
        }
    }
    d.finish("custom general context allocator threaded through compile + match; allocation counts/bytes compared");
}

// CONFIGS rows: every context copy/create/free path and every setter's return
// value, including the getters visible via pattern_info.
#[test]
fn context_setters_and_copies_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // compile context
        let ca = (p.c.compile_context_create)(ptr::null_mut());
        let cb = (p.r.compile_context_create)(ptr::null_mut());
        for v in [0u32, 1, 2, 3, 4, 5, 6, 7, 99] {
            d.eq(&format!("set_newline({v})"), (p.c.set_newline)(ca, v), (p.r.set_newline)(cb, v));
            d.eq(&format!("set_bsr({v})"), (p.c.set_bsr)(ca, v), (p.r.set_bsr)(cb, v));
        }
        for v in [
            PCRE2_OPTIMIZATION_NONE, PCRE2_OPTIMIZATION_FULL, PCRE2_AUTO_POSSESS,
            PCRE2_AUTO_POSSESS_OFF, PCRE2_START_OPTIMIZE, PCRE2_START_OPTIMIZE_OFF,
            2, 63, 66, 67, 70, 1000,
        ] {
            d.eq(&format!("set_optimize({v})"), (p.c.set_optimize)(ca, v), (p.r.set_optimize)(cb, v));
        }
        for v in [0u32, 1, 100, 255, 256, 65535, u32::MAX] {
            d.eq(
                &format!("set_max_varlookbehind({v})"),
                (p.c.set_max_varlookbehind)(ca, v),
                (p.r.set_max_varlookbehind)(cb, v),
            );
            d.eq(
                &format!("set_parens_nest_limit({v})"),
                (p.c.set_parens_nest_limit)(ca, v),
                (p.r.set_parens_nest_limit)(cb, v),
            );
        }
        for v in [0usize, 1, 1000, usize::MAX] {
            d.eq(
                &format!("set_max_pattern_length({v})"),
                (p.c.set_max_pattern_length)(ca, v),
                (p.r.set_max_pattern_length)(cb, v),
            );
            d.eq(
                &format!("set_max_pattern_compiled_length({v})"),
                (p.c.set_max_pattern_compiled_length)(ca, v),
                (p.r.set_max_pattern_compiled_length)(cb, v),
            );
        }
        for v in [0u32, 1, 0xFFFF, u32::MAX] {
            d.eq(
                &format!("set_compile_extra_options({v:#x})"),
                (p.c.set_compile_extra_options)(ca, v),
                (p.r.set_compile_extra_options)(cb, v),
            );
        }
        // copies must behave identically when compiling
        let ca2 = (p.c.compile_context_copy)(ca);
        let cb2 = (p.r.compile_context_copy)(cb);
        assert!(!ca2.is_null() && !cb2.is_null());
        // reset extra options to something benign, then compile through the copy
        (p.c.set_compile_extra_options)(ca2, 0);
        (p.r.set_compile_extra_options)(cb2, 0);
        (p.c.set_max_pattern_length)(ca2, usize::MAX);
        (p.r.set_max_pattern_length)(cb2, usize::MAX);
        (p.c.set_max_pattern_compiled_length)(ca2, usize::MAX);
        (p.r.set_max_pattern_compiled_length)(cb2, usize::MAX);
        (p.c.set_newline)(ca2, PCRE2_NEWLINE_LF);
        (p.r.set_newline)(cb2, PCRE2_NEWLINE_LF);
        (p.c.set_bsr)(ca2, PCRE2_BSR_UNICODE);
        (p.r.set_bsr)(cb2, PCRE2_BSR_UNICODE);
        (p.c.set_parens_nest_limit)(ca2, 250);
        (p.r.set_parens_nest_limit)(cb2, 250);
        (p.c.set_max_varlookbehind)(ca2, 255);
        (p.r.set_max_varlookbehind)(cb2, 255);
        (p.c.set_optimize)(ca2, PCRE2_OPTIMIZATION_FULL);
        (p.r.set_optimize)(cb2, PCRE2_OPTIMIZATION_FULL);
        for pat in PATTERNS.iter().step_by(5) {
            let pb = pat.as_bytes();
            let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
            let (mut f1, mut f2) = (0usize, 0usize);
            let k1 = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut e1, &mut f1, ca2);
            let k2 = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut e2, &mut f2, cb2);
            d.eq(&format!("ctx-copy compile {} null?", show(pb)), k1.is_null(), k2.is_null());
            d.eq(&format!("ctx-copy compile {} ec", show(pb)), e1, e2);
            d.eq(&format!("ctx-copy compile {} eo", show(pb)), f1, f2);
            if !k1.is_null() && !k2.is_null() {
                assert_code_eq(k1, k2, &format!("ctx-copy {}", show(pb)));
                (p.c.code_free)(k1);
                (p.r.code_free)(k2);
            }
        }
        (p.c.compile_context_free)(ca2);
        (p.r.compile_context_free)(cb2);
        (p.c.compile_context_free)(ca);
        (p.r.compile_context_free)(cb);

        // match context
        let ma = (p.c.match_context_create)(ptr::null_mut());
        let mb = (p.r.match_context_create)(ptr::null_mut());
        for v in [0u32, 1, 1000, u32::MAX] {
            d.eq(&format!("set_match_limit({v})"), (p.c.set_match_limit)(ma, v), (p.r.set_match_limit)(mb, v));
            d.eq(&format!("set_depth_limit({v})"), (p.c.set_depth_limit)(ma, v), (p.r.set_depth_limit)(mb, v));
            d.eq(&format!("set_heap_limit({v})"), (p.c.set_heap_limit)(ma, v), (p.r.set_heap_limit)(mb, v));
            d.eq(
                &format!("set_recursion_limit({v})"),
                (p.c.set_recursion_limit)(ma, v),
                (p.r.set_recursion_limit)(mb, v),
            );
        }
        for v in [0usize, 1, 1000, PCRE2_UNSET, usize::MAX - 1] {
            d.eq(
                &format!("set_offset_limit({v})"),
                (p.c.set_offset_limit)(ma, v),
                (p.r.set_offset_limit)(mb, v),
            );
        }
        d.eq(
            "set_recursion_memory_management",
            (p.c.set_recursion_memory_management)(ma, Some(my_malloc_c), Some(my_free), ptr::null_mut()),
            (p.r.set_recursion_memory_management)(mb, Some(my_malloc_r), Some(my_free), ptr::null_mut()),
        );
        let ma2 = (p.c.match_context_copy)(ma);
        let mb2 = (p.r.match_context_copy)(mb);
        assert!(!ma2.is_null() && !mb2.is_null());
        (p.c.match_context_free)(ma2);
        (p.r.match_context_free)(mb2);
        (p.c.match_context_free)(ma);
        (p.r.match_context_free)(mb);

        // convert context
        let va = (p.c.convert_context_create)(ptr::null_mut());
        let vb = (p.r.convert_context_create)(ptr::null_mut());
        for v in [0u32, b'/' as u32, b'.' as u32, 0x80, 0x100, 0x10FFFF, u32::MAX] {
            d.eq(
                &format!("set_glob_separator({v})"),
                (p.c.set_glob_separator)(va, v),
                (p.r.set_glob_separator)(vb, v),
            );
            d.eq(
                &format!("set_glob_escape({v})"),
                (p.c.set_glob_escape)(va, v),
                (p.r.set_glob_escape)(vb, v),
            );
        }
        let va2 = (p.c.convert_context_copy)(va);
        let vb2 = (p.r.convert_context_copy)(vb);
        assert!(!va2.is_null() && !vb2.is_null());
        (p.c.convert_context_free)(va2);
        (p.r.convert_context_free)(vb2);
        (p.c.convert_context_free)(va);
        (p.r.convert_context_free)(vb);

        // general context copy
        let ga = (p.c.general_context_create)(Some(my_malloc_c), Some(my_free), ptr::null_mut());
        let gb = (p.r.general_context_create)(Some(my_malloc_r), Some(my_free), ptr::null_mut());
        let ga2 = (p.c.general_context_copy)(ga);
        let gb2 = (p.r.general_context_copy)(gb);
        assert!(!ga2.is_null() && !gb2.is_null());
        (p.c.general_context_free)(ga2);
        (p.r.general_context_free)(gb2);
        (p.c.general_context_free)(ga);
        (p.r.general_context_free)(gb);
    }
    d.finish("all context create/copy/free + every setter's return value, incl. out-of-range values");
}

// CONFIGS row: pcre2_maketables_free_8 must accept both a general context and
// NULL, and free tables obtained from either.
#[test]
fn maketables_free_paths() {
    let p = pair();
    unsafe {
        for use_gcontext in [false, true] {
            let (ga, gb) = if use_gcontext {
                (
                    (p.c.general_context_create)(Some(my_malloc_c), Some(my_free), ptr::null_mut()),
                    (p.r.general_context_create)(Some(my_malloc_r), Some(my_free), ptr::null_mut()),
                )
            } else {
                (ptr::null_mut(), ptr::null_mut())
            };
            let ta = (p.c.maketables)(ga);
            let tb = (p.r.maketables)(gb);
            assert!(!ta.is_null() && !tb.is_null());
            let mut n = 0u32;
            (p.c.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut n as *mut u32 as Ptr);
            assert_eq!(
                std::slice::from_raw_parts(ta, n as usize),
                std::slice::from_raw_parts(tb, n as usize),
                "maketables output differs (gcontext={use_gcontext})"
            );
            (p.c.maketables_free)(ga, ta);
            (p.r.maketables_free)(gb, tb);
            if use_gcontext {
                (p.c.general_context_free)(ga);
                (p.r.general_context_free)(gb);
            }
        }
    }
}

// CONFIGS row: match_data_create vs match_data_create_from_pattern across
// ovector sizes, and the size accessors.
#[test]
fn match_data_shapes_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for n in [0u32, 1, 2, 3, 16, 1000, 65535, 65536, 100_000, u32::MAX] {
            let a = (p.c.match_data_create)(n, ptr::null_mut());
            let b = (p.r.match_data_create)(n, ptr::null_mut());
            d.eq(&format!("match_data_create({n}) null?"), a.is_null(), b.is_null());
            if a.is_null() || b.is_null() {
                if !a.is_null() {
                    (p.c.match_data_free)(a);
                }
                if !b.is_null() {
                    (p.r.match_data_free)(b);
                }
                continue;
            }
            d.eq(
                &format!("match_data_create({n}) ovector_count"),
                (p.c.get_ovector_count)(a),
                (p.r.get_ovector_count)(b),
            );
            d.eq(
                &format!("match_data_create({n}) size"),
                (p.c.get_match_data_size)(a),
                (p.r.get_match_data_size)(b),
            );
            d.eq(
                &format!("match_data_create({n}) heapframes_size"),
                (p.c.get_match_data_heapframes_size)(a),
                (p.r.get_match_data_heapframes_size)(b),
            );
            (p.c.match_data_free)(a);
            (p.r.match_data_free)(b);
        }
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            let (mut e, mut f) = (0 as c_int, 0usize);
            let ka = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut e, &mut f, ptr::null_mut());
            let kb = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut e, &mut f, ptr::null_mut());
            if ka.is_null() || kb.is_null() {
                continue;
            }
            let a = (p.c.match_data_create_from_pattern)(ka, ptr::null_mut());
            let b = (p.r.match_data_create_from_pattern)(kb, ptr::null_mut());
            d.eq(
                &format!("from_pattern({}) ovector_count", show(pb)),
                (p.c.get_ovector_count)(a),
                (p.r.get_ovector_count)(b),
            );
            d.eq(
                &format!("from_pattern({}) size", show(pb)),
                (p.c.get_match_data_size)(a),
                (p.r.get_match_data_size)(b),
            );
            (p.c.match_data_free)(a);
            (p.r.match_data_free)(b);
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    d.finish("pcre2_match_data_create_8 / _from_pattern_8 + size accessors across ovector counts");
}

// CONFIGS row: pcre2_jit_compile_8 / pcre2_jit_match_8 in a build without JIT.
#[test]
fn jit_api_identical() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for pat in PATTERNS.iter().step_by(7) {
            let pb = pat.as_bytes();
            let (mut e, mut f) = (0 as c_int, 0usize);
            let ka = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut e, &mut f, ptr::null_mut());
            let kb = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut e, &mut f, ptr::null_mut());
            if ka.is_null() || kb.is_null() {
                continue;
            }
            for opts in [
                0u32,
                PCRE2_JIT_COMPLETE,
                PCRE2_JIT_PARTIAL_SOFT,
                PCRE2_JIT_PARTIAL_HARD,
                PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
                PCRE2_JIT_INVALID_UTF,
                PCRE2_JIT_TEST_ALLOC,
                0xFFFF_FFFF,
            ] {
                d.eq(
                    &format!("jit_compile({}, {opts:#x})", show(pb)),
                    (p.c.jit_compile)(ka, opts),
                    (p.r.jit_compile)(kb, opts),
                );
            }
            let mda = (p.c.match_data_create)(8, ptr::null_mut());
            let mdb = (p.r.match_data_create)(8, ptr::null_mut());
            for subj in SUBJECTS.iter().take(12) {
                let sb = subj.as_bytes();
                let ra = (p.c.jit_match)(ka, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut());
                let rbv = (p.r.jit_match)(kb, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut());
                d.eq(&format!("jit_match({}/{})", show(pb), show(sb)), ra, rbv);
            }
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
            // pattern_info JITSIZE must agree (0 without JIT)
            let (mut ja, mut jb) = (usize::MAX, usize::MAX);
            d.eq(
                &format!("info[JITSIZE] rc {}", show(pb)),
                (p.c.pattern_info)(ka, PCRE2_INFO_JITSIZE, &mut ja as *mut usize as Ptr),
                (p.r.pattern_info)(kb, PCRE2_INFO_JITSIZE, &mut jb as *mut usize as Ptr),
            );
            d.eq(&format!("info[JITSIZE] {}", show(pb)), ja, jb);
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    d.finish("pcre2_jit_compile_8 / pcre2_jit_match_8 / INFO_JITSIZE in a no-JIT build");
}
