//! CONFIGS.md rows 1–15 — the lowest-level exported helpers, driven directly
//! through both `.so` files with randomized inputs (fixed seed).
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ------------------------------------------------ rows 1-6: string helpers ---

fn rand_cstr(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 255)).collect();
    v.push(0);
    v
}

#[test]
fn row1_strlen() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED);
    for len in (0..=64).chain([100, 255, 256, 1000, 4096]) {
        for _ in 0..8 {
            let s = rand_cstr(&mut rng, len);
            assert_eq!(
                unsafe { (c.priv_strlen)(s.as_ptr()) },
                unsafe { (r.priv_strlen)(s.as_ptr()) },
                "strlen len={len}"
            );
            assert_eq!(unsafe { (c.priv_strlen)(s.as_ptr()) }, len);
        }
    }
}

#[test]
fn row2_strcmp() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..iters(4000) {
        let la = rng.below(20);
        let a = rand_cstr(&mut rng, la);
        let mut b = a.clone();
        match rng.below(4) {
            0 => {} // equal
            1 => {
                b.truncate(rng.below(la + 1));
                b.push(0);
            }
            2 => {
                if la > 0 {
                    let i = rng.below(la);
                    b[i] = b[i].wrapping_add(1).max(1);
                }
            }
            _ => {
                let n = rng.below(20);
                b = rand_cstr(&mut rng, n);
            }
        }
        let cr = unsafe { (c.priv_strcmp)(a.as_ptr(), b.as_ptr()) };
        let rr = unsafe { (r.priv_strcmp)(a.as_ptr(), b.as_ptr()) };
        assert_eq!(cr, rr, "strcmp {a:?} {b:?}");
        // c8 variant (second arg is a plain C string)
        let cr = unsafe { (c.priv_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char) };
        let rr = unsafe { (r.priv_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char) };
        assert_eq!(cr, rr, "strcmp_c8 {a:?} {b:?}");
        // n variants
        for n in [0usize, 1, 2, 5, la, la + 1, 40] {
            assert_eq!(
                unsafe { (c.priv_strncmp)(a.as_ptr(), b.as_ptr(), n) },
                unsafe { (r.priv_strncmp)(a.as_ptr(), b.as_ptr(), n) },
                "strncmp n={n}"
            );
            assert_eq!(
                unsafe { (c.priv_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char, n) },
                unsafe { (r.priv_strncmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char, n) },
                "strncmp_c8 n={n}"
            );
        }
    }
}

#[test]
fn row6_strcpy_c8() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 6);
    for len in 0..=64 {
        for _ in 0..4 {
            let s = rand_cstr(&mut rng, len);
            let mut cb = vec![0xAAu8; len + 8];
            let mut rb = vec![0xAAu8; len + 8];
            let cn = unsafe { (c.priv_strcpy_c8)(cb.as_mut_ptr(), s.as_ptr() as *const c_char) };
            let rn = unsafe { (r.priv_strcpy_c8)(rb.as_mut_ptr(), s.as_ptr() as *const c_char) };
            assert_eq!(cn, rn, "strcpy_c8 return");
            assert_eq!(cb, rb, "strcpy_c8 buffer");
        }
    }
}

// ----------------------------------------------------- row 7: ord2utf ---------

#[test]
fn row7_ord2utf_all_codepoints() {
    let (c, r) = pair();
    for cp in 0u32..=0x10FFFF {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        let cn = unsafe { (c.priv_ord2utf)(cp, cb.as_mut_ptr()) };
        let rn = unsafe { (r.priv_ord2utf)(cp, rb.as_mut_ptr()) };
        assert_eq!(cn, rn, "ord2utf(0x{cp:x}) length");
        assert_eq!(cb, rb, "ord2utf(0x{cp:x}) bytes");
    }
    // beyond the Unicode range too (the C function has no upper check)
    for cp in [0x110000u32, 0x1FFFFF, 0x200000, 0x3FFFFFF, 0x4000000, 0x7FFFFFFF] {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        let cn = unsafe { (c.priv_ord2utf)(cp, cb.as_mut_ptr()) };
        let rn = unsafe { (r.priv_ord2utf)(cp, rb.as_mut_ptr()) };
        assert_eq!((cn, cb), (rn, rb), "ord2utf(0x{cp:x})");
    }
}

// --------------------------------------------------- row 8: valid_utf ---------

/// Every distinct malformed-UTF-8 shape the C checker distinguishes
/// (`PCRE2_ERROR_UTF8_ERR1`..`ERR21`), plus valid strings and random bytes.
const UTF8_CASES: &[&[u8]] = &[
    b"",
    b"a",
    b"abc",
    b"\x7f",
    &[0xc2, 0xa2],
    &[0xe2, 0x82, 0xac],
    &[0xf0, 0x90, 0x8d, 0x88],
    &[0xc2],                          // ERR1: 1 byte missing at end
    &[0xe2, 0x82],                    // ERR2
    &[0xf0, 0x90, 0x8d],              // ERR3
    &[0xf8, 0x88, 0x80, 0x80],        // ERR4/ERR11
    &[0xfc, 0x84, 0x80, 0x80, 0x80],  // ERR5/ERR12
    &[0xc2, 0x41],                    // ERR6
    &[0xe2, 0x82, 0x41],              // ERR7
    &[0xf0, 0x90, 0x8d, 0x41],        // ERR8
    &[0xf8, 0x88, 0x80, 0x80, 0x41],  // ERR9
    &[0xfc, 0x84, 0x80, 0x80, 0x80, 0x41], // ERR10
    &[0xfe],                          // ERR13
    &[0xff],                          // ERR13
    &[0xed, 0xa0, 0x80],              // ERR14 surrogate
    &[0xed, 0xbf, 0xbf],              // ERR14 surrogate
    &[0xc0, 0x80],                    // ERR15 overlong
    &[0xc1, 0xbf],                    // ERR15
    &[0xe0, 0x80, 0x80],              // ERR16
    &[0xe0, 0x9f, 0xbf],              // ERR16
    &[0xf0, 0x80, 0x80, 0x80],        // ERR17
    &[0xf0, 0x8f, 0xbf, 0xbf],        // ERR17
    &[0xf8, 0x80, 0x80, 0x80, 0x80],  // ERR18
    &[0xfc, 0x80, 0x80, 0x80, 0x80, 0x80], // ERR19
    &[0xf4, 0x90, 0x80, 0x80],        // ERR20 > 0x10ffff
    &[0xf7, 0xbf, 0xbf, 0xbf],        // ERR21
    &[0x80],                          // isolated continuation
    &[0xbf],
    b"abc\xff\xffdef",
    &[0xf4, 0x8f, 0xbf, 0xbf],        // max valid
];

#[test]
fn row8_valid_utf() {
    let (c, r) = pair();
    for s in UTF8_CASES {
        for &len in &[s.len(), PCRE2_ZERO_TERMINATED] {
            if len == PCRE2_ZERO_TERMINATED && s.contains(&0) {
                continue;
            }
            let mut z = s.to_vec();
            z.push(0);
            let mut co: Sz = 0xDEAD;
            let mut ro: Sz = 0xDEAD;
            let crc = unsafe { (c.priv_valid_utf)(z.as_ptr(), len, &mut co) };
            let rrc = unsafe { (r.priv_valid_utf)(z.as_ptr(), len, &mut ro) };
            assert_eq!((crc, co), (rrc, ro), "valid_utf({s:02x?}, len={len})");
        }
    }
    // randomized bytes
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..iters(20000) {
        let n = rng.below(12);
        let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let mut co: Sz = 0xDEAD;
        let mut ro: Sz = 0xDEAD;
        let crc = unsafe { (c.priv_valid_utf)(s.as_ptr(), n, &mut co) };
        let rrc = unsafe { (r.priv_valid_utf)(s.as_ptr(), n, &mut ro) };
        assert_eq!((crc, co), (rrc, ro), "valid_utf random {s:02x?}");
    }
    // Also feed valid multi-byte sequences generated from ord2utf.
    for cp in (0u32..=0x10FFFF).step_by(37) {
        let mut b = [0u8; 8];
        let n = unsafe { (c.priv_ord2utf)(cp, b.as_mut_ptr()) } as usize;
        let mut co: Sz = 0xDEAD;
        let mut ro: Sz = 0xDEAD;
        let crc = unsafe { (c.priv_valid_utf)(b.as_ptr(), n, &mut co) };
        let rrc = unsafe { (r.priv_valid_utf)(b.as_ptr(), n, &mut ro) };
        assert_eq!((crc, co), (rrc, ro), "valid_utf(ord2utf(0x{cp:x}))");
    }
}

// --------------------------------------------------- row 9: ckd_smul ---------

#[test]
fn row9_ckd_smul() {
    let (c, r) = pair();
    let vals: Vec<c_int> = vec![
        0, 1, -1, 2, -2, 3, 255, 256, 65535, 65536, 0x7FFFFFFF, -0x7FFFFFFF, i32::MIN,
        46341, 46340, -46341, 1 << 16, 1 << 20,
    ];
    for &a in &vals {
        for &b in &vals {
            let mut co: Sz = 0xDEAD;
            let mut ro: Sz = 0xDEAD;
            let crc = unsafe { (c.priv_ckd_smul)(&mut co, a, b) };
            let rrc = unsafe { (r.priv_ckd_smul)(&mut ro, a, b) };
            assert_eq!((crc, co), (rrc, ro), "ckd_smul({a},{b})");
        }
    }
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..iters(20000) {
        let a = rng.next_u32() as c_int;
        let b = rng.next_u32() as c_int;
        let mut co: Sz = 0xDEAD;
        let mut ro: Sz = 0xDEAD;
        let crc = unsafe { (c.priv_ckd_smul)(&mut co, a, b) };
        let rrc = unsafe { (r.priv_ckd_smul)(&mut ro, a, b) };
        assert_eq!((crc, co), (rrc, ro), "ckd_smul({a},{b})");
    }
}

// ------------------------------------------- rows 10-11: newline helpers -----

#[test]
fn row10_11_is_was_newline() {
    let (c, r) = pair();
    // Each interesting code unit sequence, in a padded buffer so that
    // ptr[-1] / ptr[1] are always readable.
    let seqs: &[&[u8]] = &[
        b"\n", b"\r", b"\r\n", b"\n\r", b"\x0b", b"\x0c", b"\x85", b"A", b"\x00",
        &[0xc2, 0x85],             // NEL in UTF-8
        &[0xe2, 0x80, 0xa8],       // LS
        &[0xe2, 0x80, 0xa9],       // PS
        &[0xff],
        &[0x80],
    ];
    for seq in seqs {
        for nltype in 0u32..=3 {
            for utf in [0 as c_int, 1] {
                // buffer: 4 bytes of pad, seq, 4 bytes of pad
                let mut buf = vec![b'X'; 4];
                buf.extend_from_slice(seq);
                buf.extend_from_slice(b"XXXX");
                let start = buf.as_ptr();
                let ptr = unsafe { start.add(4) };
                let end = unsafe { start.add(buf.len()) };
                let mut cl: u32 = 0xDEAD;
                let mut rl: u32 = 0xDEAD;
                let crc = unsafe { (c.priv_is_newline)(ptr, nltype, end, &mut cl, utf) };
                let rrc = unsafe { (r.priv_is_newline)(ptr, nltype, end, &mut rl, utf) };
                assert_eq!(
                    (crc != 0, cl),
                    (rrc != 0, rl),
                    "is_newline({seq:02x?}, type={nltype}, utf={utf})"
                );
                // was_newline: ptr just past the sequence
                let pptr = unsafe { start.add(4 + seq.len()) };
                let mut cl: u32 = 0xDEAD;
                let mut rl: u32 = 0xDEAD;
                let crc = unsafe { (c.priv_was_newline)(pptr, nltype, start, &mut cl, utf) };
                let rrc = unsafe { (r.priv_was_newline)(pptr, nltype, start, &mut rl, utf) };
                assert_eq!(
                    (crc != 0, cl),
                    (rrc != 0, rl),
                    "was_newline({seq:02x?}, type={nltype}, utf={utf})"
                );
            }
        }
    }
    // also with ptr == endptr-1 (CR at very end) and ptr == startptr (was_newline)
    for nltype in 1u32..=2 {
        let buf = b"\r".to_vec();
        let start = buf.as_ptr();
        let end = unsafe { start.add(1) };
        let mut cl: u32 = 0;
        let mut rl: u32 = 0;
        let crc = unsafe { (c.priv_is_newline)(start, nltype, end, &mut cl, 0) };
        let rrc = unsafe { (r.priv_is_newline)(start, nltype, end, &mut rl, 0) };
        assert_eq!((crc != 0, cl), (rrc != 0, rl), "is_newline CR at end");
    }
}

// ------------------------------------------------- row 12: extuni -------------

#[test]
fn row12_extuni() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..iters(8000) {
        // build a random UTF-8 (or Latin-1) subject
        let n = 1 + rng.below(10);
        let mut subj: Vec<u8> = Vec::new();
        let utf = rng.bool();
        for _ in 0..n {
            if utf {
                let cp = match rng.below(4) {
                    0 => rng.below(0x80) as u32,
                    1 => 0x80 + rng.below(0x780) as u32,
                    2 => 0x800 + rng.below(0xF800) as u32,
                    _ => 0x10000 + rng.below(0x100000) as u32,
                };
                let cp = if (0xD800..0xE000).contains(&cp) { 0x41 } else { cp };
                let mut b = [0u8; 8];
                let l = unsafe { (c.priv_ord2utf)(cp, b.as_mut_ptr()) } as usize;
                subj.extend_from_slice(&b[..l]);
            } else {
                subj.push(rng.byte());
            }
        }
        // first char
        let start = subj.as_ptr();
        let end = unsafe { start.add(subj.len()) };
        let (first, adv) = if utf {
            let mut cl: Sz = 0;
            let _ = unsafe { (c.priv_valid_utf)(start, subj.len(), &mut cl) };
            // decode first char with ord2utf-inverse: use the length table
            let b0 = subj[0];
            let l = if b0 < 0x80 {
                1
            } else if b0 < 0xE0 {
                2
            } else if b0 < 0xF0 {
                3
            } else {
                4
            };
            let l = l.min(subj.len());
            let cp = match l {
                1 => b0 as u32,
                2 => ((b0 as u32 & 0x1f) << 6) | (subj[1] as u32 & 0x3f),
                3 => {
                    ((b0 as u32 & 0x0f) << 12)
                        | ((subj[1] as u32 & 0x3f) << 6)
                        | (subj[2] as u32 & 0x3f)
                }
                _ => {
                    ((b0 as u32 & 0x07) << 18)
                        | ((subj[1] as u32 & 0x3f) << 12)
                        | ((subj[2] as u32 & 0x3f) << 6)
                        | (subj[3] as u32 & 0x3f)
                }
            };
            (cp, l)
        } else {
            (subj[0] as u32, 1)
        };
        let eptr = unsafe { start.add(adv) };
        let utf_i = utf as c_int;
        let mut cx: c_int = 0;
        let mut rx: c_int = 0;
        let cp = unsafe { (c.priv_extuni)(first, eptr, start, end, utf_i, &mut cx) };
        let rp = unsafe { (r.priv_extuni)(first, eptr, start, end, utf_i, &mut rx) };
        let coff = cp as usize - start as usize;
        let roff = rp as usize - start as usize;
        assert_eq!(
            (coff, cx),
            (roff, rx),
            "extuni(c=0x{first:x}, subj={subj:02x?}, utf={utf})"
        );
        // NULL xcount pointer is also a real call shape in the C code
        let cp = unsafe { (c.priv_extuni)(first, eptr, start, end, utf_i, std::ptr::null_mut()) };
        let rp = unsafe { (r.priv_extuni)(first, eptr, start, end, utf_i, std::ptr::null_mut()) };
        assert_eq!(
            cp as usize - start as usize,
            rp as usize - start as usize,
            "extuni NULL xcount"
        );
    }
}

// ---------------------------------------------- row 13: script_run -----------

#[test]
fn row13_script_run() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 13);
    let fixed: &[&[u8]] = &[
        b"",
        b"a",
        b"abc",
        b"abc123",
        "αβγ".as_bytes(),
        "abcαβγ".as_bytes(),
        "日本語".as_bytes(),
        "עברית".as_bytes(),
        "ᬅᬆ".as_bytes(),
        "a\u{0301}".as_bytes(),
        "١٢٣".as_bytes(),
        "1٢".as_bytes(),
    ];
    for s in fixed {
        for utf in [0 as c_int, 1] {
            let start = s.as_ptr();
            let end = unsafe { start.add(s.len()) };
            let crc = unsafe { (c.priv_script_run)(start, end, utf) };
            let rrc = unsafe { (r.priv_script_run)(start, end, utf) };
            assert_eq!(crc != 0, rrc != 0, "script_run({s:02x?}, utf={utf})");
        }
    }
    for _ in 0..iters(8000) {
        let n = rng.below(8);
        let mut subj = Vec::new();
        for _ in 0..n {
            let cp = match rng.below(5) {
                0 => rng.below(0x80) as u32,
                1 => 0x80 + rng.below(0x300) as u32,
                2 => 0x370 + rng.below(0x200) as u32,
                3 => 0x900 + rng.below(0x800) as u32,
                _ => 0x3000 + rng.below(0x6000) as u32,
            };
            let cp = if (0xD800..0xE000).contains(&cp) { 0x41 } else { cp };
            let mut b = [0u8; 8];
            let l = unsafe { (c.priv_ord2utf)(cp, b.as_mut_ptr()) } as usize;
            subj.extend_from_slice(&b[..l]);
        }
        let start = subj.as_ptr();
        let end = unsafe { start.add(subj.len()) };
        for utf in [0 as c_int, 1] {
            let crc = unsafe { (c.priv_script_run)(start, end, utf) };
            let rrc = unsafe { (r.priv_script_run)(start, end, utf) };
            assert_eq!(crc != 0, rrc != 0, "script_run({subj:02x?}, utf={utf})");
        }
    }
}

// ---------------------------------------------- row 15: memctl_malloc --------

extern "C" fn my_malloc(n: usize, data: *mut c_void) -> *mut c_void {
    unsafe {
        let cnt = data as *mut usize;
        if !cnt.is_null() {
            *cnt += 1;
        }
        libc_malloc(n)
    }
}
extern "C" fn my_free(p: *mut c_void, _data: *mut c_void) {
    unsafe { libc_free(p) }
}
unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn row15_memctl_malloc() {
    let (c, r) = pair();
    let mut ccount: usize = 0;
    let mut rcount: usize = 0;
    for size in [0usize, 1, 8, 16, 24, 100, 4096, 1 << 20] {
        let mut cm = MemCtl {
            malloc: Some(my_malloc),
            free: Some(my_free),
            memory_data: &mut ccount as *mut usize as *mut c_void,
        };
        let mut rm = MemCtl {
            malloc: Some(my_malloc),
            free: Some(my_free),
            memory_data: &mut rcount as *mut usize as *mut c_void,
        };
        let cp = unsafe { (c.priv_memctl_malloc)(size, &mut cm) };
        let rp = unsafe { (r.priv_memctl_malloc)(size, &mut rm) };
        assert_eq!(cp.is_null(), rp.is_null(), "memctl_malloc({size}) nullness");
        if !cp.is_null() {
            // The returned block starts with a copy of the memctl.
            let ch = unsafe { *(cp as *const MemCtl) };
            let rh = unsafe { *(rp as *const MemCtl) };
            assert_eq!(
                ch.malloc.map(|f| f as usize),
                rh.malloc.map(|f| f as usize),
                "memctl copy malloc"
            );
            assert_eq!(
                ch.free.map(|f| f as usize),
                rh.free.map(|f| f as usize),
                "memctl copy free"
            );
            my_free(cp, std::ptr::null_mut());
            my_free(rp, std::ptr::null_mut());
        }
    }
    assert_eq!(ccount, rcount, "allocation count differs");
    assert!(ccount > 0);
}
