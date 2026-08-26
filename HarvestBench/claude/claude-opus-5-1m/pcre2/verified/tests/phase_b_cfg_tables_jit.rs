// Phase B sign-off for CONFIGS.md rows 445-456:
//   * the exported read-only data tables, cross-checked against the BEHAVIOUR
//     they drive (not just dumped), and
//   * the no-JIT stub arms, including the option-validation side effect that
//     `pcre2_jit_compile_8` leaves behind on `re->overall_options`.

mod common;
use common::*;
use std::ffi::{c_int, CStr};
use std::ptr;

pub const COVERAGE: &[CfgCov] = &[
    CfgCov { cfg_rows: &[445], note: "_pcre2_OP_lengths_8 dump + opcode-walk consistency" },
    CfgCov { cfg_rows: &[446], note: "_pcre2_default_tables_8 dump + per-region behaviour cross-check" },
    CfgCov { cfg_rows: &[447], note: "hspace/vspace lists + \\h/\\v matching, UTF and non-UTF" },
    CfgCov { cfg_rows: &[448], note: "utf8_table1..4 + size, cross-checked against ord2utf/valid_utf" },
    CfgCov { cfg_rows: &[449], note: "utt / utt_names / utt_size + every entry resolvable via \\p{name}" },
    CfgCov { cfg_rows: &[450], note: "ucp_gentype, ucp_gbtable, posix_class_maps, callout delimiters" },
    CfgCov { cfg_rows: &[451], note: "all _pcre2_ucd_* tables, full dumps" },
    CfgCov { cfg_rows: &[452], note: "jit_compile PCRE2_JIT_TEST_ALLOC alone vs combined" },
    CfgCov { cfg_rows: &[453], note: "jit_compile NULL code, out-of-range bits, valid mode bits" },
    CfgCov { cfg_rows: &[454], note: "jit_compile PCRE2_JIT_INVALID_UTF side effect on overall_options" },
    CfgCov { cfg_rows: &[455], note: "jit_stack_create/assign/free, jit_free_unused_memory" },
    CfgCov { cfg_rows: &[456], note: "_pcre2_jit_get_target/get_size/free/free_rodata" },
];

#[test]
fn coverage_declaration_is_sane() {
    check_coverage_decl(COVERAGE);
}

// --------------------------------------------------------------- helpers

unsafe fn compile(api: &Api, pat: &[u8], opts: u32) -> Ptr {
    let (mut ec, mut eo) = (0 as c_int, 0usize);
    (api.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut())
}

/// Does `pat` match `subj` (rc >= 0)?  Compared between the two libraries.
unsafe fn both_match(p: &Pair, pat: &[u8], opts: u32, subj: &[u8], d: &mut Diffs, tag: &str) {
    let a = compile(&p.c, pat, opts);
    let b = compile(&p.r, pat, opts);
    d.eq(&format!("{tag} compile null?"), a.is_null(), b.is_null());
    if a.is_null() || b.is_null() {
        if !a.is_null() {
            (p.c.code_free)(a);
        }
        if !b.is_null() {
            (p.r.code_free)(b);
        }
        return;
    }
    assert_code_eq(a, b, tag);
    let mda = (p.c.match_data_create)(4, ptr::null_mut());
    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
    let ra = (p.c.do_match)(a, subj.as_ptr(), subj.len(), 0, 0, mda, ptr::null_mut());
    let rb = (p.r.do_match)(b, subj.as_ptr(), subj.len(), 0, 0, mdb, ptr::null_mut());
    d.eq(tag, read_match_out(&p.c, mda, ra), read_match_out(&p.r, mdb, rb));
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    (p.c.code_free)(a);
    (p.r.code_free)(b);
}

fn utf8_of(cp: u32) -> Vec<u8> {
    let mut b = [0u8; 4];
    char::from_u32(cp).unwrap().encode_utf8(&mut b).as_bytes().to_vec()
}

// ============================================ row 445: OP_lengths

#[test]
fn cfg_445_op_lengths() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let (a, b) = (p.c.data("_pcre2_OP_lengths_8"), p.r.data("_pcre2_OP_lengths_8"));
        let (sa, sb) = (
            std::slice::from_raw_parts(a, 173),
            std::slice::from_raw_parts(b, 173),
        );
        d.eq("_pcre2_OP_lengths_8 full dump", sa.to_vec(), sb.to_vec());
        // Walk the fixed-length opcode prefix of every corpus pattern using each
        // library's own table; the walks must agree step for step.
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            let ka = compile(&p.c, pb, 0);
            let kb = compile(&p.r, pb, 0);
            if ka.is_null() || kb.is_null() {
                if !ka.is_null() {
                    (p.c.code_free)(ka);
                }
                if !kb.is_null() {
                    (p.r.code_free)(kb);
                }
                continue;
            }
            let walk = |code: Ptr, tbl: &[u8]| -> Vec<(usize, u8)> {
                let start = bytecode_ptr(code);
                let n = code_blocksize(code) - (start as usize - code as usize);
                let by = std::slice::from_raw_parts(start, n);
                let mut steps = Vec::new();
                let mut i = 0usize;
                while i < n && steps.len() < 4096 {
                    let op = by[i];
                    steps.push((i, op));
                    let l = tbl[op as usize] as usize;
                    if l == 0 {
                        break; // variable-length opcode: stop, both must stop here
                    }
                    i += l;
                }
                steps
            };
            d.eq(
                &format!("OP_lengths walk of {}", show(pb)),
                walk(ka, sa),
                walk(kb, sb),
            );
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    d.finish("CONFIGS 445: _pcre2_OP_lengths_8 dump + opcode-walk consistency over all corpus patterns");
}

// ============================ row 446: default tables vs behaviour

#[test]
fn cfg_446_default_tables() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mut len = 0u32;
        (p.c.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut len as *mut u32 as Ptr);
        let n = len as usize;
        let (a, b) = (p.c.data("_pcre2_default_tables_8"), p.r.data("_pcre2_default_tables_8"));
        d.eq(
            "_pcre2_default_tables_8 full dump",
            std::slice::from_raw_parts(a, n).to_vec(),
            std::slice::from_raw_parts(b, n).to_vec(),
        );
        // Cross-check every table region through the behaviour it drives, for
        // all 256 single-byte subjects.
        let probes: &[(&str, u32)] = &[
            ("\\d", 0),
            ("\\D", 0),
            ("\\w", 0),
            ("\\W", 0),
            ("\\s", 0),
            ("\\S", 0),
            ("[[:alpha:]]", 0),
            ("[[:alnum:]]", 0),
            ("[[:lower:]]", 0),
            ("[[:upper:]]", 0),
            ("[[:digit:]]", 0),
            ("[[:xdigit:]]", 0),
            ("[[:space:]]", 0),
            ("[[:graph:]]", 0),
            ("[[:print:]]", 0),
            ("[[:punct:]]", 0),
            ("[[:cntrl:]]", 0),
            ("[[:word:]]", 0),
            ("[[:ascii:]]", 0),
            ("[[:blank:]]", 0),
            // the lowercase / case-flip tables
            ("a", PCRE2_CASELESS),
            ("Z", PCRE2_CASELESS),
            ("[a-f]", PCRE2_CASELESS),
            ("(?i)[X-Z]", 0),
        ];
        for &(pat, opts) in probes {
            for byte in 0u8..=255 {
                let subj = [byte];
                both_match(
                    p,
                    pat.as_bytes(),
                    opts,
                    &subj,
                    &mut d,
                    &format!("tables probe {pat} vs {byte:#04x}"),
                );
            }
        }
    }
    d.finish("CONFIGS 446: _pcre2_default_tables_8 dump + all class/case regions over all 256 bytes");
}

// ==================== row 447: hspace / vspace lists vs \h and \v

#[test]
fn cfg_447_hspace_vspace() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (sym, len) in [("_pcre2_hspace_list_8", 80usize), ("_pcre2_vspace_list_8", 32)] {
            d.eq(
                &format!("{sym} full dump"),
                std::slice::from_raw_parts(p.c.data(sym), len).to_vec(),
                std::slice::from_raw_parts(p.r.data(sym), len).to_vec(),
            );
        }
        // Read the code points out of the C table and drive \h / \v with each,
        // in UTF and non-UTF mode, plus the ones just outside each list.
        let read_list = |sym: &str, len: usize| -> Vec<u32> {
            let base = p.c.data(sym) as *const u32;
            (0..len / 4).map(|i| *base.add(i)).collect()
        };
        let hs = read_list("_pcre2_hspace_list_8", 80);
        let vs = read_list("_pcre2_vspace_list_8", 32);
        println!("hspace list = {hs:#x?}\nvspace list = {vs:#x?}");
        let mut cps: Vec<u32> = Vec::new();
        for v in hs.iter().chain(vs.iter()) {
            if *v == u32::MAX || *v > 0x10_FFFF {
                continue;
            }
            cps.push(*v);
            if *v > 0 {
                cps.push(*v - 1);
            }
            cps.push(*v + 1);
        }
        cps.extend([0, 0x20, 0x41, 0x7f, 0x80, 0xa0, 0xff, 0x100, 0x2000, 0x3000]);
        cps.sort_unstable();
        cps.dedup();
        for &cp in &cps {
            if cp > 0x10_FFFF || (0xd800..=0xdfff).contains(&cp) {
                continue;
            }
            let utf = utf8_of(cp);
            for pat in ["\\h", "\\H", "\\v", "\\V", "[\\h]", "[\\v]", "[^\\h\\v]", "\\R"] {
                // UTF mode: full code point
                both_match(
                    p,
                    pat.as_bytes(),
                    PCRE2_UTF,
                    &utf,
                    &mut d,
                    &format!("{pat} UTF U+{cp:04X}"),
                );
                // non-UTF: single byte only
                if cp < 0x100 {
                    both_match(
                        p,
                        pat.as_bytes(),
                        0,
                        &[cp as u8],
                        &mut d,
                        &format!("{pat} 8-bit {cp:#04x}"),
                    );
                }
            }
        }
    }
    d.finish("CONFIGS 447: hspace/vspace dumps cross-checked against \\h \\H \\v \\V \\R in UTF and 8-bit");
}

// =============== row 448: utf8_table1..4 vs ord2utf / valid_utf

#[test]
fn cfg_448_utf8_tables() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (sym, len) in [
            ("_pcre2_utf8_table1", 24usize),
            ("_pcre2_utf8_table1_size", 4),
            ("_pcre2_utf8_table2", 24),
            ("_pcre2_utf8_table3", 24),
            ("_pcre2_utf8_table4", 64),
        ] {
            d.eq(
                &format!("{sym} full dump"),
                std::slice::from_raw_parts(p.c.data(sym), len).to_vec(),
                std::slice::from_raw_parts(p.r.data(sym), len).to_vec(),
            );
        }
        let size = *(p.c.data("_pcre2_utf8_table1_size") as *const u32);
        assert_eq!(size, 6, "utf8_table1_size must be 6");
        let t1 = p.c.data("_pcre2_utf8_table1") as *const i32;
        let bands: Vec<i32> = (0..6).map(|i| *t1.add(i)).collect();
        println!("utf8_table1 bands = {bands:#x?}");
        // Cross-check ord2utf band selection at each band edge and just past it.
        let mut probes: Vec<u32> = vec![0];
        for &b in &bands {
            let b = b as u32;
            probes.extend([b.wrapping_sub(1), b, b.wrapping_add(1)]);
        }
        probes.extend([0x10_FFFF, 0x11_0000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX]);
        for &cp in &probes {
            let mut ba = [0xEEu8; 16];
            let mut bb = [0xEEu8; 16];
            let na = (p.c.p_ord2utf)(cp, ba.as_mut_ptr());
            let nb = (p.r.p_ord2utf)(cp, bb.as_mut_ptr());
            d.eq(&format!("ord2utf band U+{cp:X} len"), na, nb);
            d.eq(&format!("ord2utf band U+{cp:X} bytes"), ba, bb);
            // and feed the encoding straight back into the validator
            let enc = ba[..na as usize].to_vec();
            let (mut oa, mut ob) = (usize::MAX, usize::MAX);
            d.eq(
                &format!("valid_utf(ord2utf(U+{cp:X})) rc"),
                (p.c.p_valid_utf)(enc.as_ptr(), enc.len(), &mut oa),
                (p.r.p_valid_utf)(enc.as_ptr(), enc.len(), &mut ob),
            );
            d.eq(&format!("valid_utf(ord2utf(U+{cp:X})) off"), oa, ob);
        }
        // utf8_table4 gives the number of additional bytes per lead byte: check
        // the validator's behaviour for every possible lead byte, truncated.
        for lead in 0xC0u8..=0xFF {
            for extra in 0usize..=5 {
                let mut s = vec![lead];
                s.extend(std::iter::repeat(0x80u8).take(extra));
                let (mut oa, mut ob) = (usize::MAX, usize::MAX);
                d.eq(
                    &format!("valid_utf lead {lead:#04x} +{extra} rc"),
                    (p.c.p_valid_utf)(s.as_ptr(), s.len(), &mut oa),
                    (p.r.p_valid_utf)(s.as_ptr(), s.len(), &mut ob),
                );
                d.eq(&format!("valid_utf lead {lead:#04x} +{extra} off"), oa, ob);
            }
        }
    }
    d.finish("CONFIGS 448: utf8_table1..4 dumps + ord2utf band edges + valid_utf over every lead byte");
}

// ================= row 449: utt / utt_names vs \p{name} compiles

#[test]
fn cfg_449_utt_tables() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (sym, len) in [
            ("_pcre2_utt_8", 3108usize),
            ("_pcre2_utt_names_8", 3834),
            ("_pcre2_utt_size_8", 8),
        ] {
            d.eq(
                &format!("{sym} full dump"),
                std::slice::from_raw_parts(p.c.data(sym), len).to_vec(),
                std::slice::from_raw_parts(p.r.data(sym), len).to_vec(),
            );
        }
        let utt_size = *(p.c.data("_pcre2_utt_size_8") as *const usize);
        println!("utt_size = {utt_size}");
        assert_eq!(utt_size * 6, 3108, "utt entry size should be 6 bytes");
        // Every property name in the table must compile identically via \p{...}
        // and \P{...}, and must then match/not-match identically.
        let utt = p.c.data("_pcre2_utt_8") as *const u16;
        let names = p.c.data("_pcre2_utt_names_8") as *const u8;
        let mut tested = 0;
        for i in 0..utt_size {
            let name_off = *utt.add(i * 3) as usize;
            let nm = CStr::from_ptr(names.add(name_off) as *const i8)
                .to_str()
                .expect("property name is ASCII");
            for form in [
                format!("\\p{{{nm}}}"),
                format!("\\P{{{nm}}}"),
                format!("[\\p{{{nm}}}]"),
            ] {
                let pb = form.as_bytes();
                for opts in [PCRE2_UTF | PCRE2_UCP, PCRE2_UCP] {
                    let a = compile(&p.c, pb, opts);
                    let b = compile(&p.r, pb, opts);
                    d.eq(&format!("compile {form} null?"), a.is_null(), b.is_null());
                    if !a.is_null() && !b.is_null() {
                        assert_code_eq(a, b, &format!("utt {form}"));
                        tested += 1;
                    }
                    if !a.is_null() {
                        (p.c.code_free)(a);
                    }
                    if !b.is_null() {
                        (p.r.code_free)(b);
                    }
                }
            }
        }
        println!("compiled {tested} \\p/\\P forms from the utt table");
        assert!(tested > 1000, "expected to exercise most of the utt table");
        // loose matching and the sc:/scx:/bc: prefixes
        for form in [
            "\\p{Latin}", "\\p{latin}", "\\p{LATIN}", "\\p{ Latin }", "\\p{L_a-t_i n}",
            "\\p{sc:Latin}", "\\p{scx:Latin}", "\\p{bc:L}", "\\p{Script=Latin}",
            "\\p{Script_Extensions=Latin}", "\\p{General_Category=Lu}", "\\p{gc=Lu}",
            "\\p{Bidi_Class=AL}", "\\p{Is_Latin}", "\\p{IsLatin}", "\\p{nonesuch}",
            "\\p{sc:nonesuch}", "\\p{=}", "\\p{:}", "\\p{}",
        ] {
            let pb = form.as_bytes();
            let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
            let (mut f1, mut f2) = (0usize, 0usize);
            let a = (p.c.compile)(pb.as_ptr(), pb.len(), PCRE2_UTF | PCRE2_UCP, &mut e1, &mut f1, ptr::null_mut());
            let b = (p.r.compile)(pb.as_ptr(), pb.len(), PCRE2_UTF | PCRE2_UCP, &mut e2, &mut f2, ptr::null_mut());
            d.eq(&format!("loose-name {form} null?"), a.is_null(), b.is_null());
            d.eq(&format!("loose-name {form} ec"), e1, e2);
            d.eq(&format!("loose-name {form} eo"), f1, f2);
            if !a.is_null() && !b.is_null() {
                assert_code_eq(a, b, &format!("loose {form}"));
            }
            if !a.is_null() {
                (p.c.code_free)(a);
            }
            if !b.is_null() {
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("CONFIGS 449: utt/utt_names/utt_size dumps + every entry compiled via \\p{}/\\P{} + loose matching");
}

// ============ row 450: gentype / gbtable / posix maps / callout delims

#[test]
fn cfg_450_misc_tables() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (sym, len) in [
            ("_pcre2_ucp_gentype_8", 120usize),
            ("_pcre2_ucp_gbtable_8", 60),
            ("_pcre2_posix_class_maps8", 168),
            ("_pcre2_callout_start_delims_8", 36),
            ("_pcre2_callout_end_delims_8", 36),
        ] {
            d.eq(
                &format!("{sym} full dump"),
                std::slice::from_raw_parts(p.c.data(sym), len).to_vec(),
                std::slice::from_raw_parts(p.r.data(sym), len).to_vec(),
            );
        }
        // gbtable drives _pcre2_extuni_8: exercise it over a grapheme corpus.
        let pieces: &[&[u8]] = &[
            b"a", b"\xcc\x81", b"\xe2\x80\x8d", b"\xf0\x9f\x87\xa6", b"\xf0\x9f\x87\xa7",
            b"\xe1\x84\x80", b"\xe1\x85\xa1", b"\xe1\x86\xa8", b"\xf0\x9f\x91\xa8",
            b"\xe2\x9d\xa4", b"\xef\xb8\x8f", b"\xe0\xa4\xa8", b"\xe0\xa4\xbe", b"\r", b"\n",
        ];
        let mut rng = Rng::new(4500);
        for _ in 0..3000 {
            let mut s = Vec::new();
            for _ in 0..rng.range(1, 6) {
                s.extend_from_slice(rng.pick_bytes(pieces));
            }
            let Ok(t) = std::str::from_utf8(&s) else { continue };
            let ch = t.chars().next().unwrap();
            let start = s.as_ptr();
            let end = start.add(s.len());
            let eptr = start.add(ch.len_utf8());
            let (mut xa, mut xb) = (0 as c_int, 0 as c_int);
            let ra = (p.c.p_extuni)(ch as u32, eptr, start, end, 1, &mut xa);
            let rb = (p.r.p_extuni)(ch as u32, eptr, start, end, 1, &mut xb);
            d.eq(
                &format!("gbtable/extuni {}", show(&s)),
                (ra as usize - start as usize, xa),
                (rb as usize - start as usize, xb),
            );
            // and \X over the same data
            both_match(p, b"\\X", PCRE2_UTF, &s, &mut d, &format!("\\X {}", show(&s)));
        }
        // posix_class_maps drives [[:name:]]: covered per-byte in row 446 too,
        // here through every POSIX class name including the negated forms.
        for nm in [
            "alpha", "lower", "upper", "alnum", "ascii", "blank", "cntrl", "digit",
            "graph", "print", "punct", "space", "word", "xdigit",
        ] {
            for form in [format!("[[:{nm}:]]"), format!("[[:^{nm}:]]")] {
                for byte in 0u8..=255 {
                    both_match(
                        p,
                        form.as_bytes(),
                        0,
                        &[byte],
                        &mut d,
                        &format!("posix {form} {byte:#04x}"),
                    );
                }
                // and under UCP, which switches to Unicode properties
                both_match(p, form.as_bytes(), PCRE2_UTF | PCRE2_UCP, "\u{e9}".as_bytes(), &mut d, &format!("posix {form} UCP"));
            }
        }
        // callout delimiters drive (?C{...}) parsing: exercise each delimiter pair
        let delims = std::slice::from_raw_parts(
            p.c.data("_pcre2_callout_start_delims_8") as *const u32,
            9,
        );
        println!("callout start delims = {delims:?}");
        for &sd in delims {
            if sd == 0 || sd > 0x7f {
                continue;
            }
            let c = sd as u8 as char;
            for body in ["", "x", "abc"] {
                let form = format!("a(?C{c}{body}{c})b");
                let pb = form.as_bytes();
                let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
                let (mut f1, mut f2) = (0usize, 0usize);
                let a = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut e1, &mut f1, ptr::null_mut());
                let b = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut e2, &mut f2, ptr::null_mut());
                d.eq(&format!("callout delim {form} null?"), a.is_null(), b.is_null());
                d.eq(&format!("callout delim {form} ec"), e1, e2);
                if !a.is_null() && !b.is_null() {
                    assert_code_eq(a, b, &format!("callout delim {form}"));
                }
                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }
            }
        }
    }
    d.finish("CONFIGS 450: gentype/gbtable/posix_class_maps/callout delimiter dumps + behaviour cross-checks");
}

// ================================= row 451: all UCD tables

#[test]
fn cfg_451_ucd_tables() {
    let p = pair();
    let mut d = Diffs::new();
    let tables: &[(&str, usize)] = &[
        ("_pcre2_ucd_records_8", 18756),
        ("_pcre2_ucd_stage1_8", 17408),
        ("_pcre2_ucd_stage2_8", 80384),
        ("_pcre2_ucd_caseless_sets_8", 472),
        ("_pcre2_ucd_boolprop_sets_8", 1528),
        ("_pcre2_ucd_script_sets_8", 1904),
        ("_pcre2_ucd_digit_sets_8", 312),
        ("_pcre2_ucd_nocase_ranges_8", 336),
        ("_pcre2_ucd_nocase_ranges_size_8", 4),
        ("_pcre2_ucd_turkish_dotted_i_caseset_8", 4),
    ];
    unsafe {
        let mut total = 0;
        for &(sym, len) in tables {
            d.eq(
                &format!("{sym} full dump"),
                std::slice::from_raw_parts(p.c.data(sym), len).to_vec(),
                std::slice::from_raw_parts(p.r.data(sym), len).to_vec(),
            );
            total += len;
        }
        println!("compared {total} bytes of UCD tables");
        // _pcre2_unicode_version_8 is a pointer: compare the pointee.
        let va = *(p.c.data("_pcre2_unicode_version_8") as *const *const i8);
        let vb = *(p.r.data("_pcre2_unicode_version_8") as *const *const i8);
        d.eq(
            "unicode version string",
            CStr::from_ptr(va).to_bytes().to_vec(),
            CStr::from_ptr(vb).to_bytes().to_vec(),
        );
    }
    d.finish("CONFIGS 451: all _pcre2_ucd_* tables + unicode version, full byte-for-byte dumps");
}

// ==================================== rows 452-454: jit_compile

#[test]
fn cfg_452_454_jit_compile() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // row 453: NULL code
        d.eq(
            "jit_compile(NULL, 0)",
            (p.c.jit_compile)(ptr::null_mut(), 0),
            (p.r.jit_compile)(ptr::null_mut(), 0),
        );
        d.eq(
            "jit_compile(NULL, PCRE2_JIT_COMPLETE)",
            (p.c.jit_compile)(ptr::null_mut(), PCRE2_JIT_COMPLETE),
            (p.r.jit_compile)(ptr::null_mut(), PCRE2_JIT_COMPLETE),
        );

        // rows 452-454 on real codes, over every single option bit plus the
        // documented interesting combinations.
        let mut opts: Vec<u32> = vec![
            0,
            PCRE2_JIT_COMPLETE,
            PCRE2_JIT_PARTIAL_SOFT,
            PCRE2_JIT_PARTIAL_HARD,
            PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT,
            PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
            PCRE2_JIT_INVALID_UTF,
            PCRE2_JIT_TEST_ALLOC,
            PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE,
            PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_INVALID_UTF,
            0xFFFF_FFFF,
        ];
        for bit in 0..32 {
            opts.push(1u32 << bit);
        }
        for &copts in &[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_MATCH_INVALID_UTF] {
            for &o in &opts {
                let ka = compile(&p.c, b"a(b)c", copts);
                let kb = compile(&p.r, b"a(b)c", copts);
                assert!(!ka.is_null() && !kb.is_null());
                let before_a = (*(ka as *const RealCodeHead)).overall_options;
                let before_b = (*(kb as *const RealCodeHead)).overall_options;
                d.eq("jit_compile pre-state overall_options", before_a, before_b);
                let ra = (p.c.jit_compile)(ka, o);
                let rb = (p.r.jit_compile)(kb, o);
                d.eq(&format!("jit_compile(copts={copts:#x}, o={o:#x}) rc"), ra, rb);
                // row 454: the option-validation path may already have OR-ed
                // PCRE2_MATCH_INVALID_UTF into overall_options before failing.
                let after_a = (*(ka as *const RealCodeHead)).overall_options;
                let after_b = (*(kb as *const RealCodeHead)).overall_options;
                d.eq(
                    &format!("jit_compile(copts={copts:#x}, o={o:#x}) overall_options side effect"),
                    (after_a, after_a ^ before_a),
                    (after_b, after_b ^ before_b),
                );
                // and pattern_info must reflect it identically
                let (mut ia, mut ib) = (0u32, 0u32);
                d.eq(
                    "info[ALLOPTIONS] rc after jit_compile",
                    (p.c.pattern_info)(ka, PCRE2_INFO_ALLOPTIONS, &mut ia as *mut u32 as Ptr),
                    (p.r.pattern_info)(kb, PCRE2_INFO_ALLOPTIONS, &mut ib as *mut u32 as Ptr),
                );
                d.eq(
                    &format!("info[ALLOPTIONS] after jit_compile(o={o:#x})"),
                    ia,
                    ib,
                );
                let (mut ja, mut jb) = (usize::MAX, usize::MAX);
                (p.c.pattern_info)(ka, PCRE2_INFO_JITSIZE, &mut ja as *mut usize as Ptr);
                (p.r.pattern_info)(kb, PCRE2_INFO_JITSIZE, &mut jb as *mut usize as Ptr);
                d.eq("info[JITSIZE] after jit_compile", ja, jb);
                (p.c.code_free)(ka);
                (p.r.code_free)(kb);
            }
        }
    }
    d.finish("CONFIGS 452-454: pcre2_jit_compile_8 over all option bits incl. TEST_ALLOC and the INVALID_UTF side effect");
}

// ============================ row 455: jit stack API (no-JIT)

#[test]
fn cfg_455_jit_stack() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for (a, b) in [(1usize, 1024usize), (0, 0), (32 * 1024, 512 * 1024), (usize::MAX, usize::MAX)] {
            let sa = (p.c.jit_stack_create)(a, b, ptr::null_mut());
            let sb = (p.r.jit_stack_create)(a, b, ptr::null_mut());
            d.eq(&format!("jit_stack_create({a},{b}) null?"), sa.is_null(), sb.is_null());
            if !sa.is_null() {
                (p.c.jit_stack_free)(sa);
            }
            if !sb.is_null() {
                (p.r.jit_stack_free)(sb);
            }
        }
        // with a general context too
        let ga = (p.c.general_context_create)(None, None, ptr::null_mut());
        let gb = (p.r.general_context_create)(None, None, ptr::null_mut());
        d.eq("general_context_create(NULL,NULL) null?", ga.is_null(), gb.is_null());
        let sa = (p.c.jit_stack_create)(0, 0, ga);
        let sb = (p.r.jit_stack_create)(0, 0, gb);
        d.eq("jit_stack_create(0,0,gcontext) null?", sa.is_null(), sb.is_null());
        if !sa.is_null() {
            (p.c.jit_stack_free)(sa);
        }
        if !sb.is_null() {
            (p.r.jit_stack_free)(sb);
        }
        if !ga.is_null() {
            (p.c.general_context_free)(ga);
        }
        if !gb.is_null() {
            (p.r.general_context_free)(gb);
        }

        // jit_stack_assign must be a no-op that leaves the match context intact
        let ma = (p.c.match_context_create)(ptr::null_mut());
        let mb = (p.r.match_context_create)(ptr::null_mut());
        let snap = |m: Ptr| std::slice::from_raw_parts(m as *const u8, 96).to_vec();
        let (ba, bb) = (snap(ma), snap(mb));
        (p.c.jit_stack_assign)(ma, ptr::null_mut(), ptr::null_mut());
        (p.r.jit_stack_assign)(mb, ptr::null_mut(), ptr::null_mut());
        d.eq("jit_stack_assign leaves match context unchanged (C)", ba.clone(), snap(ma));
        d.eq("jit_stack_assign leaves match context unchanged (rust)", bb.clone(), snap(mb));
        // NULL match context must be tolerated
        (p.c.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        (p.r.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        (p.c.match_context_free)(ma);
        (p.r.match_context_free)(mb);

        // jit_free_unused_memory: NULL and a real general context
        (p.c.jit_free_unused_memory)(ptr::null_mut());
        (p.r.jit_free_unused_memory)(ptr::null_mut());
        let ga = (p.c.general_context_create)(None, None, ptr::null_mut());
        let gb = (p.r.general_context_create)(None, None, ptr::null_mut());
        if !ga.is_null() && !gb.is_null() {
            (p.c.jit_free_unused_memory)(ga);
            (p.r.jit_free_unused_memory)(gb);
            (p.c.general_context_free)(ga);
            (p.r.general_context_free)(gb);
        }
        // jit_stack_free(NULL) must be safe
        (p.c.jit_stack_free)(ptr::null_mut());
        (p.r.jit_stack_free)(ptr::null_mut());
        d.checked += 1;
    }
    d.finish("CONFIGS 455: jit_stack_create/assign/free and jit_free_unused_memory in a no-JIT build");
}

// ======================= row 456: exported JIT internals

#[test]
fn cfg_456_jit_internals() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let (ta, tb) = ((p.c.p_jit_get_target)(), (p.r.p_jit_get_target)());
        d.eq("jit_get_target nullness", ta.is_null(), tb.is_null());
        if !ta.is_null() && !tb.is_null() {
            let (sa, sb) = (CStr::from_ptr(ta).to_bytes().to_vec(), CStr::from_ptr(tb).to_bytes().to_vec());
            println!("jit_get_target = {:?}", String::from_utf8_lossy(&sa));
            d.eq("jit_get_target string", sa, sb);
        }
        d.eq(
            "jit_get_size(NULL)",
            (p.c.p_jit_get_size)(ptr::null_mut()),
            (p.r.p_jit_get_size)(ptr::null_mut()),
        );
        // a non-NULL (but JIT-less) pointer: use a compiled code's executable_jit
        let ka = compile(&p.c, b"abc", 0);
        let kb = compile(&p.r, b"abc", 0);
        assert!(!ka.is_null() && !kb.is_null());
        let ja = (*(ka as *const RealCodeHead)).executable_jit;
        let jb = (*(kb as *const RealCodeHead)).executable_jit;
        d.eq("executable_jit is NULL without JIT", ja.is_null(), jb.is_null());
        d.eq(
            "jit_get_size(code->executable_jit)",
            (p.c.p_jit_get_size)(ja),
            (p.r.p_jit_get_size)(jb),
        );
        // jit_free / jit_free_rodata must tolerate NULL
        (p.c.p_jit_free)(ptr::null_mut(), ptr::null_mut());
        (p.r.p_jit_free)(ptr::null_mut(), ptr::null_mut());
        (p.c.p_jit_free_rodata)(ptr::null_mut(), ptr::null_mut());
        (p.r.p_jit_free_rodata)(ptr::null_mut(), ptr::null_mut());
        (p.c.code_free)(ka);
        (p.r.code_free)(kb);
        d.checked += 1;
    }
    d.finish("CONFIGS 456: _pcre2_jit_get_target_8 / _pcre2_jit_get_size_8 / _pcre2_jit_free_8 / _pcre2_jit_free_rodata_8");
}
