//! Phase B/C — lowest-level exported entry points (`_pcre2_*`) and exported
//! data tables.
//!
//! CONFIGS.md rows 6, 8-18, 24, 25, 26 · ERRORS.md rows 269-278.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::process::Command;

// ------------------------------------------------------------------ sizes

/// Symbol name -> ELF size, read from `nm -S` on both `.so`s. Mechanically
/// derived so that no table length is guessed.
fn elf_sizes(path: &std::path::Path) -> std::collections::HashMap<String, usize> {
    let out = Command::new("nm")
        .arg("-S")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {:?}", path);
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

/// Every exported read-only data table, compared byte for byte at the exact
/// size recorded in the C `.so`'s ELF symbol table.
#[test]
fn data_tables_byte_identical() {
    let cs = elf_sizes(&c_so_path());
    let rs = elf_sizes(&rust_so_path());

    // Pure data arrays: byte-for-byte comparable.
    let arrays = [
        "_pcre2_OP_lengths_8",
        "_pcre2_callout_end_delims_8",
        "_pcre2_callout_start_delims_8",
        "_pcre2_default_tables_8",
        "_pcre2_hspace_list_8",
        "_pcre2_vspace_list_8",
        "_pcre2_posix_class_maps8",
        "_pcre2_ucd_boolprop_sets_8",
        "_pcre2_ucd_caseless_sets_8",
        "_pcre2_ucd_digit_sets_8",
        "_pcre2_ucd_nocase_ranges_8",
        "_pcre2_ucd_nocase_ranges_size_8",
        "_pcre2_ucd_records_8",
        "_pcre2_ucd_script_sets_8",
        "_pcre2_ucd_stage1_8",
        "_pcre2_ucd_stage2_8",
        "_pcre2_ucd_turkish_dotted_i_caseset_8",
        "_pcre2_ucp_gbtable_8",
        "_pcre2_ucp_gentype_8",
        "_pcre2_utf8_table1",
        "_pcre2_utf8_table1_size",
        "_pcre2_utf8_table2",
        "_pcre2_utf8_table3",
        "_pcre2_utf8_table4",
        "_pcre2_utt_names_8",
        "_pcre2_utt_size_8",
    ];

    for name in arrays {
        let szc = *cs
            .get(name)
            .unwrap_or_else(|| panic!("{name} not in C .so symbol table"));
        let szr = *rs
            .get(name)
            .unwrap_or_else(|| panic!("{name} not in Rust .so symbol table"));
        assert_eq!(szc, szr, "size mismatch for {name}");
        assert!(szc > 0, "{name} has zero size");
        diff_data(name, szc);
    }
}

/// `_pcre2_utt_8` is an array of `{ uint16_t name_offset; uint16_t type;
/// uint16_t value; }` — pure data, so a raw byte compare is valid, but we also
/// walk it and compare the resolved property names against `_pcre2_utt_names_8`.
#[test]
fn utt_table_identical() {
    let cs = elf_sizes(&c_so_path());
    let rs = elf_sizes(&rust_so_path());
    let szc = cs["_pcre2_utt_8"];
    assert_eq!(szc, rs["_pcre2_utt_8"]);
    diff_data("_pcre2_utt_8", szc);

    // Resolve each entry's name through utt_names and compare the strings.
    let (c, r) = apis();
    let n_c = unsafe { *(c.data("_pcre2_utt_size_8") as *const usize) };
    let n_r = unsafe { *(r.data("_pcre2_utt_size_8") as *const usize) };
    assert_eq!(n_c, n_r, "utt_size differs");
    assert!(n_c > 100, "utt_size implausibly small: {n_c}");

    for api in [c, r] {
        let utt = api.data("_pcre2_utt_8");
        let names = api.data("_pcre2_utt_names_8");
        let mut log = Log::new();
        for i in 0..n_c {
            // struct is 3 x uint16_t = 6 bytes, but the compiler may pad; use
            // the ELF size to derive the real stride.
            let stride = szc / n_c;
            let base = unsafe { utt.add(i * stride) as *const u16 };
            let off = unsafe { *base } as usize;
            let ty = unsafe { *base.add(1) };
            let val = unsafe { *base.add(2) };
            let nm = unsafe { cstr(names.add(off)) };
            log.b(&nm).u(ty as u64).u(val as u64);
        }
        // Store per-api and compare afterwards.
        if api.name == "C" {
            unsafe { C_UTT = Some(log) };
        } else {
            unsafe { R_UTT = Some(log) };
        }
    }
    unsafe {
        assert!(C_UTT == R_UTT, "utt resolved names/types diverge");
    }
}

static mut C_UTT: Option<Log> = None;
static mut R_UTT: Option<Log> = None;

/// `_pcre2_unicode_version_8` is a `const char *`, so compare the pointee.
#[test]
fn unicode_version_string() {
    let (c, r) = apis();
    let pc = unsafe { *(c.data("_pcre2_unicode_version_8") as *const *const u8) };
    let pr = unsafe { *(r.data("_pcre2_unicode_version_8") as *const *const u8) };
    let sc = unsafe { cstr(pc) };
    let sr = unsafe { cstr(pr) };
    assert_eq!(sc, sr, "unicode version differs");
    assert!(!sc.is_empty());
}

// ------------------------------------------------------------------ strings

#[test]
fn strlen_strcmp_family() {
    let mut rng = Rng::new(0x5EED_0001);
    for iter in 0..4000 {
        let la = rng.below(64);
        let lb = rng.below(64);
        let mut a: Vec<u8> = (0..la).map(|_| (rng.below(255) + 1) as u8).collect();
        let mut b: Vec<u8> = if iter % 3 == 0 {
            // often make them share a prefix so the comparison is interesting
            let mut v = a.clone();
            v.truncate(rng.below(la + 1));
            while v.len() < lb {
                v.push((rng.below(255) + 1) as u8);
            }
            v
        } else {
            (0..lb).map(|_| (rng.below(255) + 1) as u8).collect()
        };
        a.push(0);
        b.push(0);
        let n = rng.below(70);

        diff(&format!("strfam iter={iter}"), |api| {
            let mut l = Log::new();
            unsafe {
                l.tag("strlen").u((api.p_strlen)(a.as_ptr()) as u64);
                l.tag("strlenb").u((api.p_strlen)(b.as_ptr()) as u64);
                let x = (api.p_strcmp)(a.as_ptr(), b.as_ptr());
                l.tag("strcmp").i(x.signum() as i64);
                let x = (api.p_strncmp)(a.as_ptr(), b.as_ptr(), n);
                l.tag("strncmp").i(x.signum() as i64);
                let x = (api.p_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const i8);
                l.tag("strcmp_c8").i(x.signum() as i64);
                let x = (api.p_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const i8, n);
                l.tag("strncmp_c8").i(x.signum() as i64);
                // strcpy_c8 into a generous buffer
                let mut buf = vec![0xAAu8; 200];
                let got = (api.p_strcpy_c8)(buf.as_mut_ptr(), b.as_ptr() as *const i8);
                l.tag("strcpy_c8").u(got as u64).b(&buf);
            }
            l
        });
    }
}

#[test]
fn strfam_empty_and_equal() {
    let cases: [(&[u8], &[u8]); 6] = [
        (b"\0", b"\0"),
        (b"a\0", b"\0"),
        (b"\0", b"a\0"),
        (b"abc\0", b"abc\0"),
        (b"abc\0", b"abd\0"),
        (b"abc\0", b"ab\0"),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        for n in [0usize, 1, 2, 3, 4, 100] {
            diff(&format!("strfam_fixed {i} n={n}"), |api| {
                let mut l = Log::new();
                unsafe {
                    l.u((api.p_strlen)(a.as_ptr()) as u64);
                    l.i((api.p_strcmp)(a.as_ptr(), b.as_ptr()).signum() as i64);
                    l.i((api.p_strncmp)(a.as_ptr(), b.as_ptr(), n).signum() as i64);
                    l.i((api.p_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const i8).signum() as i64);
                    l.i((api.p_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const i8, n).signum() as i64);
                }
                l
            });
        }
    }
}

// ------------------------------------------------------------------ ord2utf

#[test]
fn ord2utf_all_boundaries_and_sample() {
    let mut points: Vec<u32> = vec![
        0, 1, 0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFFFD, 0xFFFF, 0x1_0000,
        0x10_FFFF, 0x10_0000, 0x1F600,
    ];
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..20000 {
        points.push(rng.next_u32() % 0x11_0000);
    }
    diff("ord2utf", |api| {
        let mut l = Log::new();
        let mut buf = [0u8; 8];
        for &cp in &points {
            buf = [0xAA; 8];
            let n = unsafe { (api.p_ord2utf)(cp, buf.as_mut_ptr()) };
            l.u(cp as u64).u(n as u64).b(&buf);
        }
        l
    });
}

// ------------------------------------------------------------------ valid_utf

/// Builds one representative subject for each of the 21 UTF-8 error classes,
/// plus valid strings and boundary lengths.
fn utf8_error_corpus() -> Vec<Vec<u8>> {
    vec![
        // ERR1..ERR5: missing continuation bytes for 2..6-byte starters
        vec![0xC2],
        vec![0xE0, 0xA0],
        vec![0xF0, 0x90, 0x80],
        vec![0xF8, 0x88, 0x80, 0x80],
        vec![0xFC, 0x84, 0x80, 0x80, 0x80],
        // ERR6..ERR10: bad continuation byte in position 2..6
        vec![0xC2, 0x41],
        vec![0xE0, 0xA0, 0x41],
        vec![0xF0, 0x90, 0x80, 0x41],
        vec![0xF8, 0x88, 0x80, 0x80, 0x41],
        vec![0xFC, 0x84, 0x80, 0x80, 0x80, 0x41],
        // ERR11/ERR12: 5- and 6-byte characters are not allowed
        vec![0xF8, 0x88, 0x80, 0x80, 0x80],
        vec![0xFC, 0x84, 0x80, 0x80, 0x80, 0x80],
        // ERR13: 0xFE / ERR14: 0xFF
        vec![0xFE],
        vec![0xFF],
        // ERR15..ERR18: overlong sequences
        vec![0xC0, 0x80],
        vec![0xE0, 0x80, 0x80],
        vec![0xF0, 0x80, 0x80, 0x80],
        vec![0xF8, 0x80, 0x80, 0x80, 0x80],
        // ERR19: overlong 6-byte
        vec![0xFC, 0x80, 0x80, 0x80, 0x80, 0x80],
        // ERR20: isolated continuation byte
        vec![0x80],
        vec![0xBF],
        // ERR21: > 0x10FFFF
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xF5, 0x80, 0x80, 0x80],
        vec![0xF7, 0xBF, 0xBF, 0xBF],
        // surrogates encoded in UTF-8 (allowed by pcre2's valid_utf? — compare)
        vec![0xED, 0xA0, 0x80],
        vec![0xED, 0xBF, 0xBF],
        // valid
        vec![],
        b"hello".to_vec(),
        vec![0xC2, 0xA9],
        vec![0xE2, 0x82, 0xAC],
        vec![0xF0, 0x9F, 0x98, 0x80],
        vec![0x7F],
        vec![0xC2, 0x80],
        vec![0xDF, 0xBF],
        vec![0xE0, 0xA0, 0x80],
        vec![0xEF, 0xBF, 0xBF],
        vec![0xF0, 0x90, 0x80, 0x80],
        vec![0xF4, 0x8F, 0xBF, 0xBF],
    ]
}

#[test]
fn valid_utf_error_classes() {
    let corpus = utf8_error_corpus();
    for (i, s) in corpus.iter().enumerate() {
        // explicit length, plus truncations of it, plus zero-terminated form
        let mut lens: Vec<Sz> = (0..=s.len()).collect();
        lens.push(PCRE2_ZERO_TERMINATED);
        for &len in &lens {
            let mut z = s.clone();
            z.push(0);
            diff(&format!("valid_utf corpus={i} len={len:#x}"), |api| {
                let mut l = Log::new();
                let mut off: Sz = 0xDEAD;
                let rc = unsafe { (api.p_valid_utf)(z.as_ptr(), len, &mut off) };
                l.i(rc as i64).u(off as u64);
                l
            });
        }
    }
}

#[test]
fn valid_utf_random() {
    let mut rng = Rng::new(0x5EED_0003);
    for iter in 0..6000 {
        let n = rng.below(40);
        let mut s: Vec<u8> = Vec::with_capacity(n + 1);
        for _ in 0..n {
            // biased so multi-byte starters and continuations occur often
            let b = match rng.below(4) {
                0 => rng.below(128) as u8,
                1 => 0x80 + rng.below(0x40) as u8,
                2 => 0xC0 + rng.below(0x40) as u8,
                _ => rng.next_u32() as u8,
            };
            s.push(b);
        }
        let with_nul = {
            let mut v = s.clone();
            v.push(0);
            v
        };
        for &len in &[s.len(), PCRE2_ZERO_TERMINATED] {
            diff(&format!("valid_utf_rand {iter} len={len:#x}"), |api| {
                let mut l = Log::new();
                let mut off: Sz = 0xDEAD;
                let rc = unsafe { (api.p_valid_utf)(with_nul.as_ptr(), len, &mut off) };
                l.i(rc as i64).u(off as u64);
                l
            });
        }
    }
}

// ------------------------------------------------------------------ ckd_smul

#[test]
fn ckd_smul_overflow() {
    let mut rng = Rng::new(0x5EED_0004);
    let mut pairs: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 1),
        (-1, -1),
        (i32::MAX, 2),
        (i32::MIN, 2),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, -1),
        (i32::MIN, -1),
        (65535, 65535),
        (0x1_0000, 0x1_0000),
        (-1, i32::MAX),
        (100000, 100000),
        (46341, 46341),
        (46340, 46340),
    ];
    for _ in 0..20000 {
        pairs.push((rng.next_u32() as i32, rng.next_u32() as i32));
    }
    for _ in 0..20000 {
        pairs.push(((rng.next_u32() % 100000) as i32, (rng.next_u32() % 100000) as i32));
    }
    diff("ckd_smul", |api| {
        let mut l = Log::new();
        for &(a, b) in &pairs {
            let mut r: Sz = 0xAAAA_AAAA;
            let rc = unsafe { (api.p_ckd_smul)(&mut r, a, b) };
            l.i(a as i64).i(b as i64).i((rc != 0) as i64).u(r as u64);
        }
        l
    });
}

// ------------------------------------------------------------------ newline

// `_pcre2_is_newline` / `_pcre2_was_newline` are documented (pcre2_newline.c) to
// be called only via IS_NEWLINE/WAS_NEWLINE, i.e. only for NLTYPE_ANY and
// NLTYPE_ANYCRLF, with `ptr < endptr` (is_newline) and `ptr > startptr`
// (was_newline), and in UTF mode only at a character boundary. The tests below
// respect exactly that contract while still covering every switch arm.

const NLTYPE_FIXED: u32 = 0;
const NLTYPE_ANY: u32 = 1;
const NLTYPE_ANYCRLF: u32 = 2;

/// Valid-UTF-8 subjects containing every newline-ish code point plus ordinary
/// characters, so that every arm of both switches is reached.
fn newline_subjects_utf() -> Vec<Vec<u8>> {
    let sets: Vec<Vec<char>> = vec![
        vec!['\r'],
        vec!['\n'],
        vec!['\r', '\n'],
        vec!['\n', '\r'],
        vec!['\0'],
        vec!['\u{85}'],
        vec!['\u{2028}'],
        vec!['\u{2029}'],
        vec!['\u{b}'],
        vec!['\u{c}'],
        vec!['a'],
        vec!['\r', '\r', 'x'],
        vec!['\n', '\n', 'x'],
        vec!['\r', '\u{85}', '\n'],
        vec!['\u{2028}', '\r', '\n'],
        vec!['x', '\r', '\n', 'y'],
        vec!['\u{85}', '\u{85}'],
        vec!['\u{10000}', '\n'],
        vec!['\u{7ff}', '\r'],
    ];
    sets.into_iter()
        .map(|cs| cs.into_iter().collect::<String>().into_bytes())
        .collect()
}

#[test]
fn is_newline_was_newline_all_conventions() {
    let subjects = newline_subjects_utf();
    for nltype in [NLTYPE_FIXED, NLTYPE_ANY, NLTYPE_ANYCRLF, 3, 99] {
        for utf in [0i32, 1] {
            for (i, s) in subjects.iter().enumerate() {
                if s.is_empty() {
                    continue;
                }
                // Character-boundary offsets. In non-UTF mode every byte offset
                // is a boundary.
                let bounds: Vec<usize> = if utf == 1 {
                    std::str::from_utf8(s)
                        .unwrap()
                        .char_indices()
                        .map(|(k, _)| k)
                        .collect()
                } else {
                    (0..s.len()).collect()
                };
                diff(
                    &format!("is_newline nl={nltype} utf={utf} subj={i}"),
                    |api| {
                        let mut l = Log::new();
                        let end = unsafe { s.as_ptr().add(s.len()) };
                        for &at in &bounds {
                            // is_newline: contract is ptr < endptr
                            let p = unsafe { s.as_ptr().add(at) };
                            let mut len: u32 = 0xDEAD_BEEF;
                            let rc =
                                unsafe { (api.p_is_newline)(p, nltype, end, &mut len, utf) };
                            l.i((rc != 0) as i64).u(len as u64);
                            // was_newline: contract is ptr > startptr, so use
                            // the offset just past this character.
                            if at > 0 {
                                let mut len2: u32 = 0xDEAD_BEEF;
                                let rc2 = unsafe {
                                    (api.p_was_newline)(p, nltype, s.as_ptr(), &mut len2, utf)
                                };
                                l.i((rc2 != 0) as i64).u(len2 as u64);
                            }
                        }
                        // was_newline at the very end of the subject
                        let mut len3: u32 = 0xDEAD_BEEF;
                        let rc3 = unsafe {
                            (api.p_was_newline)(end, nltype, s.as_ptr(), &mut len3, utf)
                        };
                        l.i((rc3 != 0) as i64).u(len3 as u64);
                        l
                    },
                );
            }
        }
    }
}

#[test]
fn is_newline_random() {
    let mut rng = Rng::new(0x5EED_0005);
    let alphabet = ['\r', '\n', '\0', 'a', '\u{85}', '\u{b}', '\u{c}', '\u{2028}', '\u{2029}'];
    for iter in 0..4000 {
        let n = 1 + rng.below(12);
        let text: String = (0..n).map(|_| *rng.pick(&alphabet)).collect();
        let s = text.into_bytes();
        let nltype = rng.below(3) as u32;
        let utf = if rng.bool() { 1i32 } else { 0i32 };
        let bounds: Vec<usize> = if utf == 1 {
            std::str::from_utf8(&s)
                .unwrap()
                .char_indices()
                .map(|(k, _)| k)
                .collect()
        } else {
            (0..s.len()).collect()
        };
        diff(&format!("is_newline_rand {iter}"), |api| {
            let mut l = Log::new();
            let end = unsafe { s.as_ptr().add(s.len()) };
            for &at in &bounds {
                let p = unsafe { s.as_ptr().add(at) };
                let mut len: u32 = 0;
                let rc = unsafe { (api.p_is_newline)(p, nltype, end, &mut len, utf) };
                l.i((rc != 0) as i64).u(len as u64);
                if at > 0 {
                    let mut len2: u32 = 0;
                    let rc2 =
                        unsafe { (api.p_was_newline)(p, nltype, s.as_ptr(), &mut len2, utf) };
                    l.i((rc2 != 0) as i64).u(len2 as u64);
                }
            }
            let mut len3: u32 = 0;
            let rc3 = unsafe { (api.p_was_newline)(end, nltype, s.as_ptr(), &mut len3, utf) };
            l.i((rc3 != 0) as i64).u(len3 as u64);
            l
        });
    }
}

// ------------------------------------------------------------------ script_run / extuni

#[test]
fn script_run_random() {
    let mut rng = Rng::new(0x5EED_0006);
    // Build strings out of code points from a variety of scripts.
    let cps: [u32; 24] = [
        0x41, 0x61, 0x30, 0x5F, 0x3A1, 0x430, 0x5D0, 0x627, 0x905, 0x4E00, 0x3042, 0x30A2, 0xAC00,
        0x10A0, 0x1F600, 0x2E80, 0x0300, 0x200D, 0x2019, 0x66C, 0x6F0, 0x1E900, 0x102A0, 0x10480,
    ];
    for iter in 0..3000 {
        let n = rng.below(8);
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..n {
            let cp = *rng.pick(&cps);
            let mut b = [0u8; 4];
            let l = char::from_u32(cp).unwrap().encode_utf8(&mut b).len();
            s.extend_from_slice(&b[..l]);
        }
        if s.is_empty() {
            s.push(b'a');
        }
        diff(&format!("script_run {iter}"), |api| {
            let mut l = Log::new();
            let end = unsafe { s.as_ptr().add(s.len()) };
            for utf in [0i32, 1] {
                let rc = unsafe { (api.p_script_run)(s.as_ptr(), end, utf) };
                l.i((rc != 0) as i64);
            }
            l
        });
    }
}

#[test]
fn extuni_random() {
    let mut rng = Rng::new(0x5EED_0007);
    let cps: [u32; 20] = [
        0x41, 0x300, 0x903, 0x94D, 0x1F1E6, 0x1F1E7, 0x1F600, 0x200D, 0x1F9B0, 0x0D4E, 0x1100,
        0x1160, 0x11A8, 0xAC00, 0x261D, 0xFE0F, 0x0A, 0x0D, 0x2028, 0x0910,
    ];
    for iter in 0..3000 {
        let n = 1 + rng.below(6);
        let mut s: Vec<u8> = Vec::new();
        let mut first_cp = 0x41u32;
        for k in 0..n {
            let cp = *rng.pick(&cps);
            if k == 0 {
                first_cp = cp;
            }
            let mut b = [0u8; 4];
            let l = char::from_u32(cp).unwrap().encode_utf8(&mut b).len();
            s.extend_from_slice(&b[..l]);
        }
        let first_len = char::from_u32(first_cp).unwrap().len_utf8();
        diff(&format!("extuni {iter}"), |api| {
            let mut l = Log::new();
            let start = unsafe { s.as_ptr().add(first_len) };
            let end = unsafe { s.as_ptr().add(s.len()) };
            for utf in [0i32, 1] {
                let mut xcount: i32 = 0;
                let p = unsafe {
                    (api.p_extuni)(first_cp, start, s.as_ptr(), end, utf, &mut xcount)
                };
                let off = if p.is_null() {
                    u64::MAX
                } else {
                    (p as usize - s.as_ptr() as usize) as u64
                };
                l.u(off).i(xcount as i64);
                // and with a NULL xcount pointer
                let p2 = unsafe {
                    (api.p_extuni)(
                        first_cp,
                        start,
                        s.as_ptr(),
                        end,
                        utf,
                        std::ptr::null_mut(),
                    )
                };
                let off2 = if p2.is_null() {
                    u64::MAX
                } else {
                    (p2 as usize - s.as_ptr() as usize) as u64
                };
                l.u(off2);
            }
            l
        });
    }
}

// ------------------------------------------------------------------ jit stubs

#[test]
fn jit_get_target_and_size() {
    diff("jit_get_target", |api| {
        let mut l = Log::new();
        let p = unsafe { (api.p_jit_get_target)() };
        l.b(&unsafe { cstr(p as *const u8) });
        l.u(unsafe { (api.p_jit_get_size)(std::ptr::null_mut()) } as u64);
        l
    });
}

// ------------------------------------------------------------------ memctl

#[test]
fn maketables_matches_default_tables() {
    // Row 4/5/6: pcre2_maketables with NULL and with a custom general context;
    // and the exported default tables.
    diff_data("_pcre2_default_tables_8", TABLES_LENGTH);

    diff("maketables", |api| {
        let mut l = Log::new();
        let t = unsafe { (api.maketables)(std::ptr::null_mut()) };
        assert!(!t.is_null());
        l.b(unsafe { std::slice::from_raw_parts(t, TABLES_LENGTH) });
        unsafe { (api.maketables_free)(std::ptr::null_mut(), t) };

        // custom general context
        let g = unsafe { (api.general_context_create)(Some(cb_malloc), Some(cb_free), std::ptr::null_mut()) };
        assert!(!g.is_null());
        let t2 = unsafe { (api.maketables)(g) };
        assert!(!t2.is_null());
        l.b(unsafe { std::slice::from_raw_parts(t2, TABLES_LENGTH) });
        unsafe { (api.maketables_free)(g, t2) };
        unsafe { (api.general_context_free)(g) };

        // free(NULL) must be a no-op
        unsafe { (api.maketables_free)(std::ptr::null_mut(), std::ptr::null()) };
        l.tag("ok");
        l
    });
}

pub unsafe extern "C" fn cb_malloc(n: usize, _d: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    libc_malloc(n)
}
pub unsafe extern "C" fn cb_free(p: *mut std::ffi::c_void, _d: *mut std::ffi::c_void) {
    libc_free(p)
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut std::ffi::c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut std::ffi::c_void);
}
