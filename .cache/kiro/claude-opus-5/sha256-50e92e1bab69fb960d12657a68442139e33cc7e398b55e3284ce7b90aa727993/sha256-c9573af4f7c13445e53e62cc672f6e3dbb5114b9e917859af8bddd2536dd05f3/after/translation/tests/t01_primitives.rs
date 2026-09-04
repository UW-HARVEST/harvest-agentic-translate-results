//! Phase B/C: lowest-level exported primitives and exported data tables.
//! CONFIGS.md rows 91-95, 107; ERRORS.md rows 163-168, 176.
mod harness;
use harness::*;
use std::os::raw::c_int;

// ------------------------------------------------------------------ row 107
/// Symbol sizes straight out of the ELF symbol tables, so the comparison covers
/// exactly the bytes the object actually defines (no guessing).
fn sym_sizes(so: &str) -> std::collections::HashMap<String, usize> {
    let out = std::process::Command::new("nm")
        .args(["-S", "--defined-only", so])
        .output()
        .expect("nm must be available");
    let mut m = std::collections::HashMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 4 {
            if let Ok(sz) = usize::from_str_radix(f[1], 16) {
                m.insert(f[3].to_string(), sz);
            }
        }
    }
    m
}

/// Compare an exported byte array between the two .so files.
unsafe fn cmp_bytes(name: &str, cp: *const u8, rp: *const u8, len: usize) {
    assert!(!cp.is_null(), "{name} missing from C .so");
    assert!(!rp.is_null(), "{name} missing from Rust .so");
    let cs = unsafe { std::slice::from_raw_parts(cp, len) };
    let rs = unsafe { std::slice::from_raw_parts(rp, len) };
    if cs != rs {
        let first = cs.iter().zip(rs).position(|(a, b)| a != b).unwrap();
        panic!("{name} differs at byte {first}: C={:?} Rust={:?}", cs[first], rs[first]);
    }
}

#[test]
fn data_tables_identical() {
    let (c, r) = (c(), r());
    let csz = sym_sizes(&c_so_path());
    let rsz = sym_sizes(&rust_so_path());
    // Every exported data object: compare the full C-declared extent.
    let mut check = |sym: &str, cp: *const u8, rp: *const u8| {
        let n = *csz.get(sym).unwrap_or_else(|| panic!("no size for {sym} in C .so"));
        let m = *rsz.get(sym).unwrap_or_else(|| panic!("no size for {sym} in Rust .so"));
        assert!(
            m >= n,
            "{sym}: Rust object is smaller ({m} bytes) than C ({n} bytes)"
        );
        unsafe { cmp_bytes(sym, cp, rp, n) };
        n
    };
    check("_pcre2_OP_lengths_8", c.d_OP_lengths, r.d_OP_lengths);
    check("_pcre2_default_tables_8", c.d_default_tables, r.d_default_tables);
    check("_pcre2_utf8_table2", c.d_utf8_table2, r.d_utf8_table2);
    check("_pcre2_utf8_table3", c.d_utf8_table3, r.d_utf8_table3);
    check("_pcre2_utf8_table4", c.d_utf8_table4, r.d_utf8_table4);
    check("_pcre2_utf8_table1", c.d_utf8_table1 as *const u8, r.d_utf8_table1 as *const u8);
    check(
        "_pcre2_utf8_table1_size",
        c.d_utf8_table1_size as *const u8,
        r.d_utf8_table1_size as *const u8,
    );
    check(
        "_pcre2_callout_start_delims_8",
        c.d_callout_start_delims,
        r.d_callout_start_delims,
    );
    check("_pcre2_callout_end_delims_8", c.d_callout_end_delims, r.d_callout_end_delims);
    check("_pcre2_hspace_list_8", c.d_hspace_list as *const u8, r.d_hspace_list as *const u8);
    check("_pcre2_vspace_list_8", c.d_vspace_list as *const u8, r.d_vspace_list as *const u8);
    check(
        "_pcre2_posix_class_maps8",
        c.d_posix_class_maps as *const u8,
        r.d_posix_class_maps as *const u8,
    );
    check("_pcre2_ucp_gbtable_8", c.d_ucp_gbtable as *const u8, r.d_ucp_gbtable as *const u8);
    check("_pcre2_ucp_gentype_8", c.d_ucp_gentype as *const u8, r.d_ucp_gentype as *const u8);
    check("_pcre2_ucd_records_8", c.d_ucd_records, r.d_ucd_records);
    check("_pcre2_ucd_stage1_8", c.d_ucd_stage1 as *const u8, r.d_ucd_stage1 as *const u8);
    check("_pcre2_ucd_stage2_8", c.d_ucd_stage2 as *const u8, r.d_ucd_stage2 as *const u8);
    check(
        "_pcre2_ucd_caseless_sets_8",
        c.d_ucd_caseless_sets as *const u8,
        r.d_ucd_caseless_sets as *const u8,
    );
    check(
        "_pcre2_ucd_digit_sets_8",
        c.d_ucd_digit_sets as *const u8,
        r.d_ucd_digit_sets as *const u8,
    );
    check(
        "_pcre2_ucd_script_sets_8",
        c.d_ucd_script_sets as *const u8,
        r.d_ucd_script_sets as *const u8,
    );
    check(
        "_pcre2_ucd_boolprop_sets_8",
        c.d_ucd_boolprop_sets as *const u8,
        r.d_ucd_boolprop_sets as *const u8,
    );
    check(
        "_pcre2_ucd_nocase_ranges_8",
        c.d_ucd_nocase_ranges as *const u8,
        r.d_ucd_nocase_ranges as *const u8,
    );
    check(
        "_pcre2_ucd_nocase_ranges_size_8",
        c.d_ucd_nocase_ranges_size as *const u8,
        r.d_ucd_nocase_ranges_size as *const u8,
    );
    check(
        "_pcre2_ucd_turkish_dotted_i_caseset_8",
        c.d_ucd_turkish_dotted_i_caseset as *const u8,
        r.d_ucd_turkish_dotted_i_caseset as *const u8,
    );
    check("_pcre2_utt_8", c.d_utt, r.d_utt);
    check("_pcre2_utt_names_8", c.d_utt_names as *const u8, r.d_utt_names as *const u8);
    check("_pcre2_utt_size_8", c.d_utt_size as *const u8, r.d_utt_size as *const u8);

    unsafe {
        // unicode_version is a `const char *`, so compare the pointed-to string.
        assert_eq!(
            cstr(*(c.d_unicode_version as *const *const u8)),
            cstr(*(r.d_unicode_version as *const *const u8)),
            "unicode_version differs"
        );
        // utt name offsets must resolve to identical names.
        // ucp_type_table = { uint16 name_offset; uint16 type; uint16 value; }
        let utt_size = *c.d_utt_size;
        assert_eq!(utt_size, *r.d_utt_size, "utt_size differs");
        assert_eq!(utt_size * 6, csz["_pcre2_utt_8"], "unexpected utt stride");
        for i in 0..utt_size {
            let off = *(c.d_utt.add(6 * i) as *const u16) as usize;
            assert_eq!(off, *(r.d_utt.add(6 * i) as *const u16) as usize);
            assert_eq!(
                cstr(c.d_utt_names.add(off) as *const u8),
                cstr(r.d_utt_names.add(off) as *const u8),
                "utt_names[{i}] differs"
            );
        }
    }
}

// ------------------------------------------------------------------- row 92
#[test]
fn ord2utf() {
    let (c, r) = (c(), r());
    let mut points: Vec<u32> = vec![
        0, 1, 0x7f, 0x80, 0x7ff, 0x800, 0xfff, 0x1000, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xffff,
        0x10000, 0x3ffff, 0x40000, 0x10ffff,
    ];
    let mut rng = Rng::new(0x0d_d2);
    for _ in 0..4000 {
        points.push((rng.next_u64() % 0x110000) as u32);
    }
    for cp in points {
        let mut cbuf = [0u8; 8];
        let mut rbuf = [0u8; 8];
        let cn = unsafe { (c.priv_ord2utf)(cp, cbuf.as_mut_ptr()) };
        let rn = unsafe { (r.priv_ord2utf)(cp, rbuf.as_mut_ptr()) };
        diff_eq!(cn, rn, "ord2utf({cp:#x}) length");
        diff_eq!(cbuf, rbuf, "ord2utf({cp:#x}) bytes");
    }
}

// ------------------------------------------------------------- rows 91 / 163
fn valid_utf_cases() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        b"hello".to_vec(),
        vec![0x80],             // ERR20 isolated continuation byte
        vec![0xfe],             // ERR21
        vec![0xff],             // ERR21
        vec![0xc2],             // ERR1 missing 1 byte at end
        vec![0xe1, 0x80],       // ERR2
        vec![0xf0, 0x90, 0x80], // ERR3
        vec![0xc2, 0x41],       // ERR6 bad 2nd byte
        vec![0xe1, 0x41, 0x80], // ERR7
        vec![0xe1, 0x80, 0x41],
        vec![0xf0, 0x41, 0x80, 0x80],
        vec![0xf0, 0x90, 0x41, 0x80],
        vec![0xf0, 0x90, 0x80, 0x41],
        vec![0xc0, 0x80],                         // ERR15 overlong
        vec![0xc1, 0xbf],                         // overlong
        vec![0xe0, 0x80, 0x80],                   // ERR16 overlong
        vec![0xf0, 0x80, 0x80, 0x80],             // ERR17 overlong
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],       // 5-byte -> ERR4/ERR18
        vec![0xfc, 0x84, 0x80, 0x80, 0x80, 0x80], // 6-byte
        vec![0xf4, 0x90, 0x80, 0x80],             // > 0x10ffff -> ERR13
        vec![0xed, 0xa0, 0x80],                   // surrogate -> ERR14
        vec![0xed, 0xbf, 0xbf],
        vec![0xef, 0xbf, 0xbe], // non-character
        vec![0xef, 0xbf, 0xbf],
        "héllo wörld ∀x∃y".as_bytes().to_vec(),
        "𝄞𝄢".as_bytes().to_vec(),
    ];
    // Every single byte value on its own, and every byte value appended to a
    // valid prefix - exercises all the "bad first/continuation byte" paths.
    for b in 0u16..256 {
        v.push(vec![b as u8]);
        v.push(vec![b'a', b as u8, b'z']);
        v.push(vec![0xc2, b as u8]);
        v.push(vec![0xe1, 0x80, b as u8]);
        v.push(vec![0xf0, 0x90, 0x80, b as u8]);
    }
    // Random byte soup - reproducible.
    let mut rng = Rng::new(0x0a11d);
    for _ in 0..3000 {
        let n = rng.below(12);
        v.push((0..n).map(|_| rng.next_u64() as u8).collect());
    }
    // Random valid UTF-8 strings.
    for _ in 0..2000 {
        let n = rng.below(8);
        let mut s = Vec::new();
        for _ in 0..n {
            let cp = loop {
                let x = (rng.next_u64() % 0x110000) as u32;
                if !(0xd800..=0xdfff).contains(&x) {
                    break x;
                }
            };
            let mut buf = [0u8; 4];
            s.extend_from_slice(char::from_u32(cp).unwrap().encode_utf8(&mut buf).as_bytes());
        }
        v.push(s);
    }
    v
}

#[test]
fn valid_utf() {
    let (c, r) = (c(), r());
    for case in valid_utf_cases() {
        let mut co: Sz = 0xdead;
        let mut ro: Sz = 0xdead;
        let cp = if case.is_empty() { std::ptr::NonNull::dangling().as_ptr() } else { case.as_ptr() };
        let crc = unsafe { (c.priv_valid_utf)(cp, case.len(), &mut co) };
        let rrc = unsafe { (r.priv_valid_utf)(cp, case.len(), &mut ro) };
        diff_eq!(crc, rrc, "valid_utf({case:02x?}) rc");
        if crc != 0 {
            diff_eq!(co, ro, "valid_utf({case:02x?}) erroroffset");
        }
    }
}

// ------------------------------------------------------------------- row 93
#[test]
fn string_utils() {
    let (c, r) = (c(), r());
    let strs: Vec<Vec<u8>> = vec![
        cs(""),
        cs("a"),
        cs("ab"),
        cs("abc"),
        cs("abd"),
        cs("ABC"),
        cs("abcd"),
        cb(&[0xff, 0x80, 0x01]),
        cb(&[0x80]),
        cs("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"),
    ];
    for a in &strs {
        diff_eq!(
            unsafe { (c.priv_strlen)(a.as_ptr()) },
            unsafe { (r.priv_strlen)(a.as_ptr()) },
            "strlen({a:02x?})"
        );
        for b in &strs {
            let cc = unsafe { (c.priv_strcmp)(a.as_ptr(), b.as_ptr()) };
            let rc = unsafe { (r.priv_strcmp)(a.as_ptr(), b.as_ptr()) };
            diff_eq!(cc.signum(), rc.signum(), "strcmp({a:02x?},{b:02x?})");
            diff_eq!(cc, rc, "strcmp exact({a:02x?},{b:02x?})");
            for n in [0usize, 1, 2, 3, 4, 32] {
                let cc = unsafe { (c.priv_strncmp)(a.as_ptr(), b.as_ptr(), n) };
                let rc = unsafe { (r.priv_strncmp)(a.as_ptr(), b.as_ptr(), n) };
                diff_eq!(cc, rc, "strncmp({a:02x?},{b:02x?},{n})");
            }
            let bc = b.as_ptr() as *const std::os::raw::c_char;
            let cc = unsafe { (c.priv_strcmp_c8)(a.as_ptr(), bc) };
            let rc = unsafe { (r.priv_strcmp_c8)(a.as_ptr(), bc) };
            diff_eq!(cc, rc, "strcmp_c8({a:02x?},{b:02x?})");
            for n in [0usize, 1, 2, 3, 4, 32] {
                let cc = unsafe { (c.priv_strncmp_c8)(a.as_ptr(), bc, n) };
                let rc = unsafe { (r.priv_strncmp_c8)(a.as_ptr(), bc, n) };
                diff_eq!(cc, rc, "strncmp_c8({a:02x?},{b:02x?},{n})");
            }
        }
    }
    for s in &strs {
        let mut cbuf = [0xaau8; 64];
        let mut rbuf = [0xaau8; 64];
        let sc = s.as_ptr() as *const std::os::raw::c_char;
        let cn = unsafe { (c.priv_strcpy_c8)(cbuf.as_mut_ptr(), sc) };
        let rn = unsafe { (r.priv_strcpy_c8)(rbuf.as_mut_ptr(), sc) };
        diff_eq!(cn, rn, "strcpy_c8({s:02x?}) len");
        diff_eq!(cbuf, rbuf, "strcpy_c8({s:02x?}) buf");
    }
}

// ------------------------------------------------------------- rows 95 / 166
#[test]
fn ckd_smul() {
    let (c, r) = (c(), r());
    let mut vals: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x10000,
        0x7fff,
        -0x8000,
        65535,
    ];
    let mut rng = Rng::new(0xc4d5);
    for _ in 0..3000 {
        vals.push(rng.next_u64() as c_int);
    }
    for a in &vals {
        for b in [0, 1, -1, 2, 3, i32::MAX, i32::MIN, 0x10000] {
            let mut co: Sz = 0xdeadbeef;
            let mut ro: Sz = 0xdeadbeef;
            let crc = unsafe { (c.priv_ckd_smul)(&mut co, *a, b) };
            let rrc = unsafe { (r.priv_ckd_smul)(&mut ro, *a, b) };
            diff_eq!(crc != 0, rrc != 0, "ckd_smul({a},{b}) overflow flag");
            diff_eq!(co, ro, "ckd_smul({a},{b}) result");
        }
    }
}

// ------------------------------------------------------------- rows 94 / 168
#[test]
fn newline_detection() {
    let (c, r) = (c(), r());
    let types = [
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ];
    let bufs: Vec<Vec<u8>> = vec![
        b"\r".to_vec(),
        b"\n".to_vec(),
        b"\r\n".to_vec(),
        b"\n\r".to_vec(),
        b"a\r\nb".to_vec(),
        b"a\n\rb".to_vec(),
        b"\0".to_vec(),
        b"a\0b".to_vec(),
        vec![0x0b],
        vec![0x0c],
        vec![0x85],
        vec![0xc2, 0x85],             // NEL in UTF
        vec![0xe2, 0x80, 0xa8],       // LS
        vec![0xe2, 0x80, 0xa9],       // PS
        b"abc".to_vec(),
        b"".to_vec(),
        b"a\rb\nc\0d".to_vec(),
        vec![0x0d, 0x0d],
        vec![0xc2, 0x85, 0x0a],
    ];
    for &nltype in &types {
        for buf in &bufs {
            for utf in [0, 1] {
                for pos in 0..=buf.len() {
                    unsafe {
                        // is_newline(ptr, nltype, endptr, &lenptr, utf)
                        let mut cl: u32 = 0xffff;
                        let mut rl: u32 = 0xffff;
                        let p = buf.as_ptr().add(pos);
                        let e = buf.as_ptr().add(buf.len());
                        if pos < buf.len() {
                            let cv = (c.priv_is_newline)(p, nltype, e, &mut cl, utf);
                            let rv = (r.priv_is_newline)(p, nltype, e, &mut rl, utf);
                            diff_eq!(cv != 0, rv != 0, "is_newline({buf:02x?},{nltype},{pos},{utf})");
                            if cv != 0 {
                                diff_eq!(cl, rl, "is_newline len({buf:02x?},{nltype},{pos},{utf})");
                            }
                        }
                        // was_newline(ptr, nltype, startptr, &lenptr, utf)
                        if pos > 0 {
                            let mut cl: u32 = 0xffff;
                            let mut rl: u32 = 0xffff;
                            let s = buf.as_ptr();
                            let cv = (c.priv_was_newline)(p, nltype, s, &mut cl, utf);
                            let rv = (r.priv_was_newline)(p, nltype, s, &mut rl, utf);
                            diff_eq!(
                                cv != 0,
                                rv != 0,
                                "was_newline({buf:02x?},{nltype},{pos},{utf})"
                            );
                            if cv != 0 {
                                diff_eq!(cl, rl, "was_newline len({buf:02x?},{nltype},{pos},{utf})");
                            }
                        }
                    }
                }
            }
        }
    }
    // Randomized fuzz over the same axes.
    let mut rng = Rng::new(0x9e00_11ee);
    for _ in 0..4000 {
        let n = rng.range(1, 8);
        let buf: Vec<u8> = (0..n)
            .map(|_| *rng.pick(&[b'\r', b'\n', 0u8, b'a', 0x85, 0xc2, 0xe2, 0x80, 0xa8, 0x0b]))
            .collect();
        let nltype = *rng.pick(&types);
        let utf = if rng.bool() { 1 } else { 0 };
        let pos = rng.below(buf.len());
        unsafe {
            let mut cl: u32 = 0;
            let mut rl: u32 = 0;
            let p = buf.as_ptr().add(pos);
            let e = buf.as_ptr().add(buf.len());
            let cv = (c.priv_is_newline)(p, nltype, e, &mut cl, utf);
            let rv = (r.priv_is_newline)(p, nltype, e, &mut rl, utf);
            diff_eq!((cv != 0, cl), (rv != 0, rl), "fuzz is_newline({buf:02x?},{nltype},{pos},{utf})");
            if pos > 0 {
                let mut cl: u32 = 0;
                let mut rl: u32 = 0;
                let cv = (c.priv_was_newline)(p, nltype, buf.as_ptr(), &mut cl, utf);
                let rv = (r.priv_was_newline)(p, nltype, buf.as_ptr(), &mut rl, utf);
                diff_eq!(
                    (cv != 0, cl),
                    (rv != 0, rl),
                    "fuzz was_newline({buf:02x?},{nltype},{pos},{utf})"
                );
            }
        }
    }
}

// ------------------------------------------------------------- rows 96 / 97
#[test]
fn extuni_and_script_run() {
    let (c, r) = (c(), r());
    let samples: Vec<Vec<u8>> = vec![
        "a".into(),
        "e\u{301}".into(),          // base + combining
        "e\u{301}\u{302}".into(),
        "\u{1F1FA}\u{1F1F8}".into(), // regional indicators
        "\u{1100}\u{1161}\u{11A8}".into(), // Hangul jamo L V T
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".into(), // emoji ZWJ
        "\u{0D4E}\u{0D15}".into(),  // prepend
        "abc".into(),
        "日本語".into(),
        "ひらがなカタカナ漢字".into(),
        "Ελληνικά".into(),
        "Русский".into(),
        "abcАбc".into(),          // mixed Latin/Cyrillic
        "a\u{0660}".into(),        // Latin + Arabic-Indic digit
        "١٢٣".into(),
        "1٢3".into(),
        "".into(),
    ];
    let mut all = samples.clone();
    let mut rng = Rng::new(0xe47001);
    for _ in 0..1500 {
        let n = rng.range(1, 5);
        let mut s = String::new();
        for _ in 0..n {
            let cp = loop {
                let x = (rng.next_u64() % 0x2ffff) as u32;
                if let Some(ch) = char::from_u32(x) {
                    break ch;
                }
            };
            s.push(cp);
        }
        all.push(s.into_bytes());
    }
    for s in &all {
        if s.is_empty() {
            continue;
        }
        unsafe {
            let start = s.as_ptr();
            let end = start.add(s.len());
            // extuni(c, eptr, start_subject, end_subject, utf, xcount)
            // The first code point is consumed by the caller in the C code, so
            // decode it here the same way the interpreter does.
            let first = std::str::from_utf8(s).ok().and_then(|t| t.chars().next());
            if let Some(fc) = first {
                let flen = fc.len_utf8();
                let mut cx: c_int = 0;
                let mut rx: c_int = 0;
                let cp1 = (c.priv_extuni)(fc as u32, start.add(flen), start, end, 1, &mut cx);
                let rp1 = (r.priv_extuni)(fc as u32, start.add(flen), start, end, 1, &mut rx);
                diff_eq!(
                    cp1 as usize - start as usize,
                    rp1 as usize - start as usize,
                    "extuni({s:02x?}) end"
                );
                diff_eq!(cx, rx, "extuni({s:02x?}) xcount");
                // and without a count pointer
                let cp2 = (c.priv_extuni)(fc as u32, start.add(flen), start, end, 1,
                                          std::ptr::null_mut());
                let rp2 = (r.priv_extuni)(fc as u32, start.add(flen), start, end, 1,
                                          std::ptr::null_mut());
                diff_eq!(
                    cp2 as usize - start as usize,
                    rp2 as usize - start as usize,
                    "extuni-nocount({s:02x?})"
                );
            }
            for utf in [0, 1] {
                let cv = (c.priv_script_run)(start, end, utf);
                let rv = (r.priv_script_run)(start, end, utf);
                diff_eq!(cv != 0, rv != 0, "script_run({s:02x?},utf={utf})");
            }
        }
    }
}

// -------------------------------------------------------------- row 174 (err)
#[test]
fn memctl_malloc_null_on_failure() {
    let (c, r) = (c(), r());
    // A huge request must fail identically (NULL) in both.
    unsafe {
        let gc_c = (c.general_context_create)(None, None, std::ptr::null_mut());
        let gc_r = (r.general_context_create)(None, None, std::ptr::null_mut());
        assert!(!gc_c.is_null() && !gc_r.is_null());
        let huge = usize::MAX - 4096;
        let pc = (c.priv_memctl_malloc)(huge, gc_c);
        let pr = (r.priv_memctl_malloc)(huge, gc_r);
        diff_eq!(pc.is_null(), pr.is_null(), "memctl_malloc(huge)");
        // A small request must succeed in both.
        let pc = (c.priv_memctl_malloc)(64, gc_c);
        let pr = (r.priv_memctl_malloc)(64, gc_r);
        diff_eq!(pc.is_null(), pr.is_null(), "memctl_malloc(64)");
        (c.general_context_free)(gc_c);
        (r.general_context_free)(gc_r);
    }
}
