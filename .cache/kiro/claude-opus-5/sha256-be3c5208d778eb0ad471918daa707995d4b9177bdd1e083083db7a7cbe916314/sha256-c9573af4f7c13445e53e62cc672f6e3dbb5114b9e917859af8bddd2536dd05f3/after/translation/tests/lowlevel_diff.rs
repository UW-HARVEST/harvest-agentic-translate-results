//! Phase B (lowest level first) — differential tests for every exported
//! *private* function and every exported data table.
//!
//! The data-table sizes below are the ELF symbol sizes taken from
//! `nm -D -S c_src/build/libpcre2.so`; the test also verifies that the Rust
//! `.so` exports the same number of bytes (see `tools/gen_symbols.sh`, which prints a
//! size diff).

mod common;
use common::*;
use std::ffi::c_void;

/// (symbol, byte size) for every exported data object in the C `.so`.
static DATA_SYMBOLS: &[(&str, usize)] = &[
    ("_pcre2_OP_lengths_8", 0xad),
    ("_pcre2_callout_end_delims_8", 0x24),
    ("_pcre2_callout_start_delims_8", 0x24),
    ("_pcre2_default_tables_8", 0x440),
    ("_pcre2_hspace_list_8", 0x50),
    ("_pcre2_posix_class_maps8", 0xa8),
    ("_pcre2_ucd_boolprop_sets_8", 0x5f8),
    ("_pcre2_ucd_caseless_sets_8", 0x1d8),
    ("_pcre2_ucd_digit_sets_8", 0x138),
    ("_pcre2_ucd_nocase_ranges_8", 0x150),
    ("_pcre2_ucd_nocase_ranges_size_8", 0x4),
    ("_pcre2_ucd_records_8", 0x4944),
    ("_pcre2_ucd_script_sets_8", 0x770),
    ("_pcre2_ucd_stage1_8", 0x4400),
    ("_pcre2_ucd_stage2_8", 0x13a00),
    ("_pcre2_ucd_turkish_dotted_i_caseset_8", 0x4),
    ("_pcre2_ucp_gbtable_8", 0x3c),
    ("_pcre2_ucp_gentype_8", 0x78),
    ("_pcre2_utf8_table1", 0x18),
    ("_pcre2_utf8_table1_size", 0x4),
    ("_pcre2_utf8_table2", 0x18),
    ("_pcre2_utf8_table3", 0x18),
    ("_pcre2_utf8_table4", 0x40),
    ("_pcre2_utt_8", 0xc24),
    ("_pcre2_utt_names_8", 0xefa),
    ("_pcre2_utt_size_8", 0x8),
    ("_pcre2_vspace_list_8", 0x20),
    // NOTE: `_pcre2_unicode_version_8` is a `const char *` and the three
    // `_pcre2_default_*_context_8` objects embed malloc/free function pointers
    // and a `tables` pointer, so their raw bytes cannot be equal across two
    // different shared objects. They are compared by their observable content /
    // effect instead (see `unicode_version_string_matches` and the compile
    // tests that use a NULL context).
];

#[test]
fn all_exported_data_tables_are_byte_identical() {
    let p = libs();
    // Resolve each symbol in both libraries through the dynamic symbol table.
    for (name, size) in DATA_SYMBOLS {
        let a = resolve(p, name, true);
        let b = resolve(p, name, false);
        let sa = unsafe { std::slice::from_raw_parts(a as *const u8, *size) };
        let sb = unsafe { std::slice::from_raw_parts(b as *const u8, *size) };
        if sa != sb {
            let first = sa.iter().zip(sb).position(|(x, y)| x != y).unwrap();
            panic!(
                "data table {} differs at byte {} of {}\n C: {:02x?}\n R: {:02x?}",
                name,
                first,
                size,
                &sa[first..(first + 32).min(*size)],
                &sb[first..(first + 32).min(*size)]
            );
        }
    }
}

/// Look up a symbol address in one of the two loaded libraries.
fn resolve(p: &'static Pair, name: &str, from_c: bool) -> *const c_void {
    // We already have typed handles for every data symbol in `Api`; map by name.
    let api = if from_c { &p.c } else { &p.r };
    match name {
        "_pcre2_OP_lengths_8" => api.d_OP_lengths as *const c_void,
        "_pcre2_callout_end_delims_8" => api.d_callout_end_delims as *const c_void,
        "_pcre2_callout_start_delims_8" => api.d_callout_start_delims as *const c_void,
        "_pcre2_default_tables_8" => api.d_default_tables as *const c_void,
        "_pcre2_hspace_list_8" => api.d_hspace_list as *const c_void,
        "_pcre2_posix_class_maps8" => api.d_posix_class_maps as *const c_void,
        "_pcre2_ucd_boolprop_sets_8" => api.d_ucd_boolprop_sets as *const c_void,
        "_pcre2_ucd_caseless_sets_8" => api.d_ucd_caseless_sets as *const c_void,
        "_pcre2_ucd_digit_sets_8" => api.d_ucd_digit_sets as *const c_void,
        "_pcre2_ucd_nocase_ranges_8" => api.d_ucd_nocase_ranges as *const c_void,
        "_pcre2_ucd_nocase_ranges_size_8" => api.d_ucd_nocase_ranges_size as *const c_void,
        "_pcre2_ucd_records_8" => api.d_ucd_records as *const c_void,
        "_pcre2_ucd_script_sets_8" => api.d_ucd_script_sets as *const c_void,
        "_pcre2_ucd_stage1_8" => api.d_ucd_stage1 as *const c_void,
        "_pcre2_ucd_stage2_8" => api.d_ucd_stage2 as *const c_void,
        "_pcre2_ucd_turkish_dotted_i_caseset_8" => api.d_ucd_turkish_dotted_i_caseset as *const c_void,
        "_pcre2_ucp_gbtable_8" => api.d_ucp_gbtable as *const c_void,
        "_pcre2_ucp_gentype_8" => api.d_ucp_gentype as *const c_void,
        "_pcre2_utf8_table1" => api.d_utf8_table1 as *const c_void,
        "_pcre2_utf8_table1_size" => api.d_utf8_table1_size as *const c_void,
        "_pcre2_utf8_table2" => api.d_utf8_table2 as *const c_void,
        "_pcre2_utf8_table3" => api.d_utf8_table3 as *const c_void,
        "_pcre2_utf8_table4" => api.d_utf8_table4 as *const c_void,
        "_pcre2_utt_8" => api.d_utt as *const c_void,
        "_pcre2_utt_names_8" => api.d_utt_names as *const c_void,
        "_pcre2_utt_size_8" => api.d_utt_size as *const c_void,
        "_pcre2_vspace_list_8" => api.d_vspace_list as *const c_void,
        other => panic!("unmapped data symbol {}", other),
    }
}

#[test]
fn unicode_version_string_matches() {
    let p = libs();
    unsafe {
        let a = *p.c.d_unicode_version;
        let b = *p.r.d_unicode_version;
        assert!(!a.is_null() && !b.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(a),
            std::ffi::CStr::from_ptr(b),
            "_pcre2_unicode_version_8"
        );
    }
}

#[test]
fn maketables_output_is_byte_identical() {
    let p = libs();
    unsafe {
        let mut tl: u32 = 0;
        (p.c.config)(cfg::TABLES_LENGTH, &mut tl as *mut _ as *mut c_void);
        assert!(tl > 0);
        for _ in 0..3 {
            let tc = (p.c.maketables)(std::ptr::null_mut());
            let tr = (p.r.maketables)(std::ptr::null_mut());
            assert!(!tc.is_null() && !tr.is_null());
            let sc = std::slice::from_raw_parts(tc, tl as usize);
            let sr = std::slice::from_raw_parts(tr, tl as usize);
            assert_eq!(sc, sr, "pcre2_maketables output");
            // The generated tables must also equal the built-in default tables.
            let dc = std::slice::from_raw_parts(p.c.d_default_tables, tl as usize);
            assert_eq!(sc, dc, "maketables vs _pcre2_default_tables_8 (C)");
            (p.c.maketables_free)(std::ptr::null_mut(), tc);
            (p.r.maketables_free)(std::ptr::null_mut(), tr);
        }
        // maketables with a general context that has a failing allocator.
        unsafe extern "C" fn nomalloc(_n: usize, _d: *mut c_void) -> *mut c_void {
            std::ptr::null_mut()
        }
        unsafe extern "C" fn nofree(_p: *mut c_void, _d: *mut c_void) {}
        let gc = (p.c.general_context_create)(Some(nomalloc), Some(nofree), std::ptr::null_mut());
        let gr = (p.r.general_context_create)(Some(nomalloc), Some(nofree), std::ptr::null_mut());
        assert_eq!(gc.is_null(), gr.is_null());
        if !gc.is_null() {
            let tc = (p.c.maketables)(gc);
            let tr = (p.r.maketables)(gr);
            assert_eq!(tc.is_null(), tr.is_null(), "maketables with failing allocator");
        }
    }
}

// ===========================================================================
// String helpers
// ===========================================================================

#[test]
fn priv_string_functions() {
    let p = libs();
    let strings: &[&[u8]] = &[
        b"\0",
        b"a\0",
        b"abc\0",
        b"abd\0",
        b"ab\0",
        b"ABC\0",
        b"\xff\xfe\0",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0",
        b"\x01\x02\x03\0",
        b"\x7f\x80\x81\0",
    ];
    unsafe {
        for s in strings {
            assert_eq!(
                (p.c.priv_strlen)(s.as_ptr()),
                (p.r.priv_strlen)(s.as_ptr()),
                "_pcre2_strlen({:?})",
                s
            );
        }
        for a in strings {
            for b in strings {
                assert_eq!(
                    (p.c.priv_strcmp)(a.as_ptr(), b.as_ptr()).signum(),
                    (p.r.priv_strcmp)(a.as_ptr(), b.as_ptr()).signum(),
                    "_pcre2_strcmp({:?},{:?})",
                    a,
                    b
                );
                assert_eq!(
                    (p.c.priv_strcmp)(a.as_ptr(), b.as_ptr()),
                    (p.r.priv_strcmp)(a.as_ptr(), b.as_ptr()),
                    "_pcre2_strcmp exact ({:?},{:?})",
                    a,
                    b
                );
                assert_eq!(
                    (p.c.priv_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const i8),
                    (p.r.priv_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const i8),
                    "_pcre2_strcmp_c8({:?},{:?})",
                    a,
                    b
                );
                for n in [0usize, 1, 2, 3, 4, 8, 64] {
                    assert_eq!(
                        (p.c.priv_strncmp)(a.as_ptr(), b.as_ptr(), n),
                        (p.r.priv_strncmp)(a.as_ptr(), b.as_ptr(), n),
                        "_pcre2_strncmp({:?},{:?},{})",
                        a,
                        b,
                        n
                    );
                    assert_eq!(
                        (p.c.priv_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const i8, n),
                        (p.r.priv_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const i8, n),
                        "_pcre2_strncmp_c8({:?},{:?},{})",
                        a,
                        b,
                        n
                    );
                }
            }
        }
        for s in strings {
            let mut bc = [0xCDu8; 128];
            let mut br = [0xCDu8; 128];
            let a = (p.c.priv_strcpy_c8)(bc.as_mut_ptr(), s.as_ptr() as *const i8);
            let b = (p.r.priv_strcpy_c8)(br.as_mut_ptr(), s.as_ptr() as *const i8);
            assert_eq!(a, b, "_pcre2_strcpy_c8 return for {:?}", s);
            assert_eq!(bc, br, "_pcre2_strcpy_c8 buffer for {:?}", s);
        }
    }
}

// ===========================================================================
// _pcre2_ord2utf, _pcre2_valid_utf
// ===========================================================================

#[test]
fn priv_ord2utf_over_full_range() {
    let p = libs();
    unsafe {
        // Every boundary plus a dense sweep of the low range and a random sample
        // of the whole 32-bit space (the C function accepts any uint32_t).
        let mut points: Vec<u32> = Vec::new();
        points.extend(0u32..=0x2FF);
        for b in [
            0x7Fu32, 0x80, 0x7FF, 0x800, 0xFFF, 0x1000, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFFFD,
            0xFFFE, 0xFFFF, 0x1_0000, 0x1F_FFFF, 0x20_0000, 0x3FF_FFFF, 0x400_0000, 0x7FFF_FFFF,
            0x10_FFFF, 0x11_0000, 0xFFFF_FFFF, 0x8000_0000,
        ] {
            points.push(b);
            points.push(b.wrapping_sub(1));
            points.push(b.wrapping_add(1));
        }
        let mut r = Rng::new(0xC0DE_1234_5678_9ABC);
        for _ in 0..4000 {
            points.push(r.next_u32());
        }
        for cp in points {
            let mut bc = [0xCDu8; 16];
            let mut br = [0xCDu8; 16];
            let a = (p.c.priv_ord2utf)(cp, bc.as_mut_ptr());
            let b = (p.r.priv_ord2utf)(cp, br.as_mut_ptr());
            assert_eq!(a, b, "_pcre2_ord2utf({:#x}) length", cp);
            assert_eq!(bc, br, "_pcre2_ord2utf({:#x}) bytes", cp);
        }
    }
}

#[test]
fn priv_valid_utf_randomized() {
    let p = libs();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    unsafe {
        for _ in 0..40000 {
            let n = rng.below(9);
            let buf: Vec<u8> = (0..n).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
            let mut oc: Sz = 0xAAAA;
            let mut or: Sz = 0x5555;
            let a = (p.c.priv_valid_utf)(buf.as_ptr(), buf.len(), &mut oc);
            let b = (p.r.priv_valid_utf)(buf.as_ptr(), buf.len(), &mut or);
            assert_eq!(a, b, "_pcre2_valid_utf({:02x?}) rc", buf);
            if a != 0 {
                assert_eq!(oc, or, "_pcre2_valid_utf({:02x?}) offset", buf);
            }
        }
        // Also biased toward plausible lead bytes.
        for _ in 0..40000 {
            let n = 1 + rng.below(6);
            let buf: Vec<u8> = (0..n)
                .map(|i| {
                    if i == 0 {
                        *rng.pick(&[
                            0x41u8, 0x7F, 0x80, 0xBF, 0xC0, 0xC2, 0xDF, 0xE0, 0xED, 0xEF, 0xF0,
                            0xF4, 0xF5, 0xF8, 0xFC, 0xFE, 0xFF,
                        ])
                    } else {
                        *rng.pick(&[0x00u8, 0x41, 0x7F, 0x80, 0x9F, 0xA0, 0xBF, 0xC0, 0xFF])
                    }
                })
                .collect();
            let mut oc: Sz = 0;
            let mut or: Sz = 0;
            let a = (p.c.priv_valid_utf)(buf.as_ptr(), buf.len(), &mut oc);
            let b = (p.r.priv_valid_utf)(buf.as_ptr(), buf.len(), &mut or);
            assert_eq!((a, oc), (b, or), "_pcre2_valid_utf({:02x?})", buf);
        }
    }
}

// ===========================================================================
// _pcre2_is_newline / _pcre2_was_newline
// ===========================================================================

#[test]
fn priv_newline_functions() {
    let p = libs();
    // Every newline convention, every byte value, both UTF and non-UTF.
    let nltypes: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 100, u32::MAX, // includes out-of-range enum values
    ];
    unsafe {
        for &nltype in nltypes {
            for utf in [0i32, 1] {
                for lead in 0u32..=0xFF {
                    // A 4-byte window so multi-unit newlines have room.
                    let buf = [lead as u8, 0x0A, 0x0D, 0x85];
                    for at in 0..4usize {
                        let mut lc: u32 = 0xAAAA;
                        let mut lr: u32 = 0xAAAA;
                        let a = (p.c.priv_is_newline)(
                            buf.as_ptr().add(at),
                            nltype,
                            buf.as_ptr().add(4),
                            &mut lc,
                            utf,
                        );
                        let b = (p.r.priv_is_newline)(
                            buf.as_ptr().add(at),
                            nltype,
                            buf.as_ptr().add(4),
                            &mut lr,
                            utf,
                        );
                        assert_eq!(
                            (a, lc),
                            (b, lr),
                            "_pcre2_is_newline(nltype={} utf={} buf={:02x?} at={})",
                            nltype, utf, buf, at
                        );
                        let mut lc: u32 = 0xAAAA;
                        let mut lr: u32 = 0xAAAA;
                        let a = (p.c.priv_was_newline)(
                            buf.as_ptr().add(at),
                            nltype,
                            buf.as_ptr(),
                            &mut lc,
                            utf,
                        );
                        let b = (p.r.priv_was_newline)(
                            buf.as_ptr().add(at),
                            nltype,
                            buf.as_ptr(),
                            &mut lr,
                            utf,
                        );
                        assert_eq!(
                            (a, lc),
                            (b, lr),
                            "_pcre2_was_newline(nltype={} utf={} buf={:02x?} at={})",
                            nltype, utf, buf, at
                        );
                    }
                }
            }
        }
        // Multi-byte UTF sequences around U+0085 / U+2028 / U+2029.
        let seqs: &[&[u8]] = &[
            &[0xC2, 0x85],
            &[0xE2, 0x80, 0xA8],
            &[0xE2, 0x80, 0xA9],
            &[0x0D, 0x0A],
            &[0x0A, 0x0D],
            &[0x00],
            &[0x0B],
            &[0x0C],
        ];
        for s in seqs {
            for &nltype in &[1u32, 2, 3, 4, 5, 6] {
                for utf in [0i32, 1] {
                    let mut lc: u32 = 0xAAAA;
                    let mut lr: u32 = 0xAAAA;
                    let a = (p.c.priv_is_newline)(
                        s.as_ptr(),
                        nltype,
                        s.as_ptr().add(s.len()),
                        &mut lc,
                        utf,
                    );
                    let b = (p.r.priv_is_newline)(
                        s.as_ptr(),
                        nltype,
                        s.as_ptr().add(s.len()),
                        &mut lr,
                        utf,
                    );
                    assert_eq!((a, lc), (b, lr), "is_newline {:02x?} nl={} utf={}", s, nltype, utf);
                    let mut lc: u32 = 0xAAAA;
                    let mut lr: u32 = 0xAAAA;
                    let a = (p.c.priv_was_newline)(
                        s.as_ptr().add(s.len()),
                        nltype,
                        s.as_ptr(),
                        &mut lc,
                        utf,
                    );
                    let b = (p.r.priv_was_newline)(
                        s.as_ptr().add(s.len()),
                        nltype,
                        s.as_ptr(),
                        &mut lr,
                        utf,
                    );
                    assert_eq!((a, lc), (b, lr), "was_newline {:02x?} nl={} utf={}", s, nltype, utf);
                }
            }
        }
    }
}

// ===========================================================================
// _pcre2_script_run, _pcre2_extuni
// ===========================================================================

#[test]
fn priv_script_run() {
    let p = libs();
    let mut rng = Rng::new(0x5C21_9700_0000_0001);
    let samples: &[&[u8]] = &[
        b"",
        b"a",
        b"abc",
        b"abc123",
        "\u{0041}\u{0391}".as_bytes(), // Latin + Greek
        "\u{0391}\u{0392}".as_bytes(),
        "\u{05D0}\u{05D1}".as_bytes(),
        "\u{3042}\u{3043}".as_bytes(),
        "\u{30A2}\u{3042}".as_bytes(),
        "\u{0301}a".as_bytes(),
        "0\u{0660}".as_bytes(),
        "\u{0669}\u{0660}".as_bytes(),
    ];
    unsafe {
        for s in samples {
            for utf in [0i32, 1] {
                let a = (p.c.priv_script_run)(s.as_ptr(), s.as_ptr().add(s.len()), utf);
                let b = (p.r.priv_script_run)(s.as_ptr(), s.as_ptr().add(s.len()), utf);
                assert_eq!(a, b, "_pcre2_script_run({:02x?}, utf={})", s, utf);
            }
        }
        // Randomized valid-UTF strings.
        for _ in 0..4000 {
            let n = rng.below(6);
            let mut buf = Vec::new();
            for _ in 0..n {
                let cp = match rng.below(6) {
                    0 => 0x41 + rng.below(26) as u32,
                    1 => 0x391 + rng.below(24) as u32,
                    2 => 0x5D0 + rng.below(20) as u32,
                    3 => 0x660 + rng.below(10) as u32,
                    4 => 0x3042 + rng.below(80) as u32,
                    _ => 0x30 + rng.below(10) as u32,
                };
                let mut tmp = [0u8; 8];
                let l = (p.c.priv_ord2utf)(cp, tmp.as_mut_ptr());
                buf.extend_from_slice(&tmp[..l as usize]);
            }
            let a = (p.c.priv_script_run)(buf.as_ptr(), buf.as_ptr().add(buf.len()), 1);
            let b = (p.r.priv_script_run)(buf.as_ptr(), buf.as_ptr().add(buf.len()), 1);
            assert_eq!(a, b, "_pcre2_script_run random {:02x?}", buf);
        }
    }
}

#[test]
fn priv_extuni() {
    let p = libs();
    let mut rng = Rng::new(0xEE11_2233_4455_6677);
    unsafe {
        for _ in 0..6000 {
            // Build a random grapheme-ish sequence out of interesting code points.
            let pool: &[u32] = &[
                0x41, 0x61, 0x0300, 0x0301, 0x094D, 0x0915, 0x1100, 0x1161, 0x11A8, 0xAC00,
                0x1F600, 0x200D, 0x1F1E6, 0x1F1E7, 0x0600, 0x0A03, 0x0903, 0x0D4E, 0x0E33,
            ];
            let n = 1 + rng.below(5);
            let mut buf = Vec::new();
            let mut firsts = Vec::new();
            for _ in 0..n {
                let cp = *rng.pick(pool);
                let mut tmp = [0u8; 8];
                let l = (p.c.priv_ord2utf)(cp, tmp.as_mut_ptr());
                firsts.push((cp, buf.len()));
                buf.extend_from_slice(&tmp[..l as usize]);
            }
            let (c0, off0) = firsts[0];
            let start = buf.as_ptr().add(off0);
            // `ptr` must point just past the first character.
            let mut tmp = [0u8; 8];
            let l0 = (p.c.priv_ord2utf)(c0, tmp.as_mut_ptr()) as usize;
            let after = buf.as_ptr().add(off0 + l0);
            let end = buf.as_ptr().add(buf.len());
            for utf in [1i32, 0] {
                let mut xc: i32 = 0;
                let mut xr: i32 = 0;
                let a = (p.c.priv_extuni)(c0, after, start, end, utf, &mut xc);
                let b = (p.r.priv_extuni)(c0, after, start, end, utf, &mut xr);
                let oa = if a.is_null() { -1i64 } else { a.offset_from(buf.as_ptr()) as i64 };
                let ob = if b.is_null() { -1i64 } else { b.offset_from(buf.as_ptr()) as i64 };
                assert_eq!(oa, ob, "_pcre2_extuni ptr for {:02x?} utf={}", buf, utf);
                assert_eq!(xc, xr, "_pcre2_extuni xcount for {:02x?} utf={}", buf, utf);
            }
        }
    }
}

// ===========================================================================
// _pcre2_ckd_smul
// ===========================================================================

#[test]
fn priv_ckd_smul() {
    let p = libs();
    let mut rng = Rng::new(0xC7D5_ADD1_0000_0000);
    let interesting: &[i32] = &[
        0, 1, -1, 2, -2, 3, 100, -100, 32767, 32768, 65535, 65536, i32::MAX, i32::MIN,
        i32::MAX - 1, i32::MIN + 1, 46340, 46341, -46340, -46341,
    ];
    unsafe {
        for &a in interesting {
            for &b in interesting {
                let mut rc_: Sz = 0xAAAA;
                let mut rr: Sz = 0x5555;
                let x = (p.c.priv_ckd_smul)(&mut rc_, a, b);
                let y = (p.r.priv_ckd_smul)(&mut rr, a, b);
                assert_eq!(x != 0, y != 0, "_pcre2_ckd_smul({},{}) overflow flag", a, b);
                if x == 0 {
                    assert_eq!(rc_, rr, "_pcre2_ckd_smul({},{}) result", a, b);
                }
            }
        }
        for _ in 0..20000 {
            let a = rng.next_u32() as i32;
            let b = rng.next_u32() as i32;
            let mut rc_: Sz = 0;
            let mut rr: Sz = 0;
            let x = (p.c.priv_ckd_smul)(&mut rc_, a, b);
            let y = (p.r.priv_ckd_smul)(&mut rr, a, b);
            assert_eq!(x != 0, y != 0, "_pcre2_ckd_smul({},{}) overflow flag", a, b);
            if x == 0 {
                assert_eq!(rc_, rr, "_pcre2_ckd_smul({},{}) result", a, b);
            }
        }
    }
}

// ===========================================================================
// _pcre2_compile_get_hash_from_name, _pcre2_update_classbits
// ===========================================================================

#[test]
fn priv_get_hash_from_name() {
    let p = libs();
    let mut r = Rng::new(0x1A2B_3C4D_5E6F_7081);
    unsafe {
        // NOTE: the C function asserts `length > 0` and reads `name[length - 1]`,
        // so length 0 is undefined behaviour; the domain starts at 1.
        for _ in 0..20000 {
            let n = 1 + r.below(24);
            let name: Vec<u8> = (0..n).map(|_| (0x20 + (r.next_u32() % 0x60)) as u8).collect();
            let a = (p.c.priv_get_hash_from_name)(name.as_ptr(), n as u32);
            let b = (p.r.priv_get_hash_from_name)(name.as_ptr(), n as u32);
            assert_eq!(a, b, "_pcre2_compile_get_hash_from_name({:?})", name);
        }
        // All-0xff names and single-byte extremes.
        for n in [1u32, 2, 3, 128] {
            for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
                let name = vec![fill; n as usize + 1];
                assert_eq!(
                    (p.c.priv_get_hash_from_name)(name.as_ptr(), n),
                    (p.r.priv_get_hash_from_name)(name.as_ptr(), n),
                    "hash of {} x {:#02x}",
                    n,
                    fill
                );
            }
        }
    }
}

#[test]
fn priv_update_classbits() {
    let p = libs();
    // ptype/pdata pairs cover every Unicode property type the compiler emits,
    // plus out-of-range values (the C switch has a default branch).
    unsafe {
        for ptype in 0u32..=20 {
            for pdata in 0u32..=40 {
                for negated in [0i32, 1] {
                    let mut bc = [0u8; 32];
                    let mut br = [0u8; 32];
                    (p.c.priv_update_classbits)(ptype, pdata, negated, bc.as_mut_ptr());
                    (p.r.priv_update_classbits)(ptype, pdata, negated, br.as_mut_ptr());
                    assert_eq!(
                        bc, br,
                        "_pcre2_update_classbits(ptype={}, pdata={}, neg={})",
                        ptype, pdata, negated
                    );
                    // Also with a pre-seeded bitmap, to check OR-ing semantics.
                    let mut bc = [0xA5u8; 32];
                    let mut br = [0xA5u8; 32];
                    (p.c.priv_update_classbits)(ptype, pdata, negated, bc.as_mut_ptr());
                    (p.r.priv_update_classbits)(ptype, pdata, negated, br.as_mut_ptr());
                    assert_eq!(
                        bc, br,
                        "_pcre2_update_classbits(seeded, ptype={}, pdata={}, neg={})",
                        ptype, pdata, negated
                    );
                }
            }
        }
        for ptype in [100u32, 255, 1000, u32::MAX] {
            let mut bc = [0u8; 32];
            let mut br = [0u8; 32];
            (p.c.priv_update_classbits)(ptype, 0, 0, bc.as_mut_ptr());
            (p.r.priv_update_classbits)(ptype, 0, 0, br.as_mut_ptr());
            assert_eq!(bc, br, "_pcre2_update_classbits(ptype={})", ptype);
        }
    }
}

// ===========================================================================
// _pcre2_memctl_malloc
// ===========================================================================

#[test]
fn priv_memctl_malloc() {
    let p = libs();
    unsafe {
        // With a NULL memctl the default malloc is used.
        for n in [1usize, 8, 100, 4096] {
            let a = (p.c.priv_memctl_malloc)(n, std::ptr::null_mut());
            let b = (p.r.priv_memctl_malloc)(n, std::ptr::null_mut());
            assert_eq!(a.is_null(), b.is_null(), "_pcre2_memctl_malloc({})", n);
            // The block starts with a copy of the memctl struct; we cannot compare
            // its function pointers across libraries, so just free it again.
            if !a.is_null() {
                libc_free(a);
                libc_free(b);
            }
        }
    }
}

unsafe fn libc_free(p: *mut c_void) {
    unsafe extern "C" {
        fn free(p: *mut c_void);
    }
    unsafe { free(p) }
}

// ===========================================================================
// _pcre2_study / _pcre2_auto_possessify / _pcre2_find_bracket /
// _pcre2_xclass / _pcre2_eclass
//
// These take internal pointers (compile_block *, compiled code) that cannot be
// synthesised safely from outside the library. They are exercised through the
// public API instead:
//   * _pcre2_study            -> every pcre2_compile() call (start-bitmap,
//                                minlength, first/last code unit are compared by
//                                cmp_all_pattern_info + cmp_compiled_bytes)
//   * _pcre2_auto_possessify  -> pcre2_compile() unless PCRE2_NO_AUTO_POSSESS
//   * _pcre2_find_bracket     -> back/forward references and recursion
//   * _pcre2_xclass/_eclass   -> matching against [\x{...}] and (?[...]) classes
// The tests below drive those paths directly and additionally call
// _pcre2_find_bracket on a real compiled code block.
// ===========================================================================

#[test]
fn priv_find_bracket_on_real_code() {
    let p = libs();
    for pat in [
        &b"(a)(b)(c)"[..],
        &b"(?<x>a)(?:b)(c)"[..],
        &b"(a(b(c)))"[..],
        &b"a|b|c"[..],
        &b"(?|(a)|(b))"[..],
        &b"(a)(?1)"[..],
    ] {
        let cp = compile_both(p, pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), "fb")
            .unwrap();
        unsafe {
            // The byte code starts at `code_start` bytes into the block; rather
            // than parse the header we search from the serialized form offsets.
            // `pcre2_pattern_info(PCRE2_INFO_SIZE)` plus the known header layout is
            // avoided here: instead we walk from the code start reported by
            // PCRE2_INFO_FRAMESIZE-independent means — the first OP_BRA/OP_CBRA.
            let mut sz: Sz = 0;
            (p.c.pattern_info)(cp.c, info::SIZE, &mut sz as *mut _ as *mut c_void);
            // Locate the byte code by scanning for the magic then reading code_start.
            let base_c = cp.c as *const u8;
            let base_r = cp.r as *const u8;
            let mut code_start = usize::MAX;
            for off in (0..256usize).step_by(4) {
                if *(base_c.add(off) as *const u32) == 0x5043_5245 {
                    // code_start is the field *before* magic_number
                    code_start = *(base_c.add(off - std::mem::size_of::<usize>()) as *const usize);
                    break;
                }
            }
            assert_ne!(code_start, usize::MAX);
            for n in [0i32, 1, 2, 3, 4, -1, 100] {
                for capturing in [0i32, 1] {
                    let a = (p.c.priv_find_bracket)(base_c.add(code_start), capturing, n);
                    let b = (p.r.priv_find_bracket)(base_r.add(code_start), capturing, n);
                    let oa = if a.is_null() { -1i64 } else { a.offset_from(base_c) as i64 };
                    let ob = if b.is_null() { -1i64 } else { b.offset_from(base_r) as i64 };
                    assert_eq!(
                        oa, ob,
                        "_pcre2_find_bracket({:?}, utf={}, n={})",
                        String::from_utf8_lossy(pat), capturing, n
                    );
                }
            }
        }
        free_code_pair(p, cp);
    }
}

#[test]
fn xclass_and_eclass_via_matching() {
    let p = libs();
    // Patterns whose matching goes through _pcre2_xclass / _pcre2_eclass.
    let pats: &[(&[u8], u32)] = &[
        (b"[\\x{100}-\\x{200}]", o::UTF),
        (b"[^\\x{100}-\\x{200}]", o::UTF),
        (b"[\\p{L}\\x{2000}]", o::UTF),
        (b"[[:alpha:]\\x{500}]", o::UTF),
        (b"(?[[\\x{100}-\\x{200}]&&[\\x{150}-\\x{300}]])", o::UTF),
        (b"(?[[\\p{L}]--[a-z]])", o::UTF),
        (b"(?[[\\p{L}]||[0-9]])", o::UTF),
        (b"(?[![a-z]])", o::UTF),
        (b"[\\x{100}-\\x{200}]", 0),
        (b"[\\p{Han}]", o::UTF),
        (b"[\\p{Greek}\\p{Latin}]", o::UTF),
    ];
    let mut rng = Rng::new(0xC1A5_5000_0000_0001);
    for (pat, opts) in pats {
        let cp = match compile_both(p, pat, pat.len(), *opts, std::ptr::null_mut(), std::ptr::null_mut(), "xc")
        {
            Ok(cp) => cp,
            Err(_) => continue,
        };
        cmp_compiled_bytes(p, &cp, "xclass");
        unsafe {
            let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
            let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
            for _ in 0..3000 {
                let cpnt = match rng.below(4) {
                    0 => rng.below(0x80) as u32,
                    1 => 0x80 + rng.below(0x400) as u32,
                    2 => 0x100 + rng.below(0x2400) as u32,
                    _ => rng.next_u32() % 0x11_0000,
                };
                if (0xD800..=0xDFFF).contains(&cpnt) {
                    continue;
                }
                let mut tmp = [0u8; 8];
                let l = (p.c.priv_ord2utf)(cpnt, tmp.as_mut_ptr()) as usize;
                let subj = &tmp[..l];
                let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), l, 0, 0, mdc, std::ptr::null_mut());
                let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), l, 0, 0, mdr, std::ptr::null_mut());
                assert_eq!(
                    a, b,
                    "match {:?} against U+{:04X}",
                    String::from_utf8_lossy(pat), cpnt
                );
                if a > 0 {
                    let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), 2);
                    let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), 2);
                    assert_eq!(oc, or, "ovector for U+{:04X}", cpnt);
                }
            }
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
        free_code_pair(p, cp);
    }
}
