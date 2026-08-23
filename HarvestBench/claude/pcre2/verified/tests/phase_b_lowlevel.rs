// Phase B — the LOWEST-level exported entry points, called directly through
// both `.so`s.  These are the leaves the whole engine is built on, so they are
// driven with many randomized inputs (fixed seed) rather than one value each.

mod common;
use common::*;
use std::ffi::{c_char, c_int, CStr};
use std::ptr;

const N: usize = 4000;

// ===================================================================== strings

// CONFIGS row: _pcre2_strlen_8 — empty / 1 / many, embedded high bytes.
#[test]
fn strlen() {
    let p = pair();
    let mut rng = Rng::new(1);
    let mut d = Diffs::new();
    for _ in 0..N {
        let mut s = gen_raw(&mut rng, 40);
        s.retain(|&b| b != 0); // NUL terminated: no interior NULs
        s.push(0);
        unsafe {
            d.eq(
                &format!("strlen({})", show(&s)),
                (p.c.p_strlen)(s.as_ptr()),
                (p.r.p_strlen)(s.as_ptr()),
            );
        }
    }
    d.finish("_pcre2_strlen_8: random NUL-terminated strings, len 0..40");
}

// CONFIGS row: _pcre2_strcmp_8 — equal / prefix / differing, all byte values.
#[test]
fn strcmp() {
    let p = pair();
    let mut rng = Rng::new(2);
    let mut d = Diffs::new();
    let mk = |rng: &mut Rng| {
        let mut s = gen_ascii(rng, 12);
        s.retain(|&b| b != 0);
        s.push(0);
        s
    };
    for _ in 0..N {
        let a = mk(&mut rng);
        // half the time compare against a mutated copy so equality/prefix
        // relationships actually occur
        let b = if rng.chance(2) {
            let mut b = a.clone();
            if b.len() > 1 && rng.chance(2) {
                let i = rng.below(b.len() - 1);
                b[i] = rng.byte().max(1);
            }
            if rng.chance(3) && b.len() > 1 {
                b.truncate(b.len() - 1);
                b.push(0);
            }
            b
        } else {
            mk(&mut rng)
        };
        unsafe {
            let (ca, cb) = ((p.c.p_strcmp)(a.as_ptr(), b.as_ptr()), (p.r.p_strcmp)(a.as_ptr(), b.as_ptr()));
            d.eq(&format!("strcmp({},{})", show(&a), show(&b)), ca, cb);
            let n = rng.below(14);
            d.eq(
                &format!("strncmp({},{},{n})", show(&a), show(&b)),
                (p.c.p_strncmp)(a.as_ptr(), b.as_ptr(), n),
                (p.r.p_strncmp)(a.as_ptr(), b.as_ptr(), n),
            );
        }
    }
    d.finish("_pcre2_strcmp_8 / _pcre2_strncmp_8: equal, prefix and differing pairs, n 0..13");
}

// CONFIGS row: the `_c8` variants (PCRE2_SPTR vs C `char *`).
#[test]
fn strcmp_c8_and_strcpy_c8() {
    let p = pair();
    let mut rng = Rng::new(3);
    let mut d = Diffs::new();
    for _ in 0..N {
        let mut a = gen_ascii(&mut rng, 12);
        a.retain(|&b| b != 0);
        a.push(0);
        let mut b = if rng.chance(2) { a.clone() } else { gen_ascii(&mut rng, 12) };
        b.retain(|&x| x != 0);
        b.push(0);
        let bc = b.as_ptr() as *const c_char;
        unsafe {
            d.eq(
                &format!("strcmp_c8({},{})", show(&a), show(&b)),
                (p.c.p_strcmp_c8)(a.as_ptr(), bc),
                (p.r.p_strcmp_c8)(a.as_ptr(), bc),
            );
            let n = rng.below(14);
            d.eq(
                &format!("strncmp_c8({},{},{n})", show(&a), show(&b)),
                (p.c.p_strncmp_c8)(a.as_ptr(), bc, n),
                (p.r.p_strncmp_c8)(a.as_ptr(), bc, n),
            );
            // strcpy_c8 writes; compare returned length AND the written bytes.
            let mut ba = [0xEEu8; 64];
            let mut bb = [0xEEu8; 64];
            let la = (p.c.p_strcpy_c8)(ba.as_mut_ptr(), bc);
            let lb = (p.r.p_strcpy_c8)(bb.as_mut_ptr(), bc);
            d.eq(&format!("strcpy_c8({}) len", show(&b)), la, lb);
            d.eq(&format!("strcpy_c8({}) buf", show(&b)), ba, bb);
        }
    }
    d.finish("_pcre2_strcmp_c8_8 / _pcre2_strncmp_c8_8 / _pcre2_strcpy_c8_8");
}

// ======================================================================== UTF

// CONFIGS row: _pcre2_ord2utf_8 — EVERY code point 0..=0x10FFFF plus the
// out-of-range values the signed comparison in the C makes reachable.
#[test]
fn ord2utf_exhaustive() {
    let p = pair();
    let mut d = Diffs::new();
    let check = |cp: u32, d: &mut Diffs| unsafe {
        let mut ba = [0xEEu8; 16];
        let mut bb = [0xEEu8; 16];
        let na = (p.c.p_ord2utf)(cp, ba.as_mut_ptr());
        let nb = (p.r.p_ord2utf)(cp, bb.as_mut_ptr());
        d.eq(&format!("ord2utf(U+{cp:X}) len"), na, nb);
        d.eq(&format!("ord2utf(U+{cp:X}) bytes"), ba, bb);
    };
    for cp in 0u32..=0x10_FFFF {
        check(cp, &mut d);
    }
    // beyond Unicode: the C compares `(int)cvalue <= utf8_table1[i]`, so the
    // whole 6-byte encoding range and the negative-when-signed range are live.
    for cp in [
        0x11_0000, 0x1F_FFFF, 0x20_0000, 0x3F_FFFF, 0x40_0000, 0x3FF_FFFF, 0x400_0000,
        0x7FFF_FFFE, 0x7FFF_FFFF, 0x8000_0000, 0x8000_0001, 0xFFFF_FFFE, 0xFFFF_FFFF,
    ] {
        check(cp, &mut d);
    }
    d.finish("_pcre2_ord2utf_8: all code points 0..0x10FFFF + out-of-range/negative-as-signed");
}

// CONFIGS row: _pcre2_valid_utf_8 — valid UTF-8 of every length class, and
// random raw bytes that exercise the 21 distinct UTF-8 error classes.
#[test]
fn valid_utf() {
    let p = pair();
    let mut rng = Rng::new(4);
    let mut d = Diffs::new();
    let mut one = |s: &[u8], d: &mut Diffs| unsafe {
        let (mut oa, mut ob) = (usize::MAX, usize::MAX);
        let ra = (p.c.p_valid_utf)(s.as_ptr(), s.len(), &mut oa);
        let rb = (p.r.p_valid_utf)(s.as_ptr(), s.len(), &mut ob);
        d.eq(&format!("valid_utf({}) rc", show(s)), ra, rb);
        d.eq(&format!("valid_utf({}) erroroffset", show(s)), oa, ob);
    };
    for _ in 0..N {
        one(&gen_utf8(&mut rng, 12), &mut d); // well-formed
        one(&gen_raw(&mut rng, 12), &mut d); // arbitrary bytes
        // targeted malformed shapes: truncated / bad continuation / overlong /
        // surrogate / too-big, embedded at a random offset in valid text
        let mut s = gen_utf8(&mut rng, 6);
        let bad: &[&[u8]] = &[
            &[0x80],
            &[0xBF],
            &[0xC0, 0x80],
            &[0xC1, 0xBF],
            &[0xC2],
            &[0xE0, 0x80, 0x80],
            &[0xE0, 0xA0],
            &[0xE2, 0x28, 0xA1],
            &[0xED, 0xA0, 0x80],
            &[0xED, 0xBF, 0xBF],
            &[0xF0, 0x80, 0x80, 0x80],
            &[0xF0, 0x90, 0x80],
            &[0xF4, 0x90, 0x80, 0x80],
            &[0xF5, 0x80, 0x80, 0x80],
            &[0xF8, 0x88, 0x80, 0x80, 0x80],
            &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x80],
            &[0xFE],
            &[0xFF],
        ];
        s.extend_from_slice(rng.pick_bytes(bad));
        s.extend_from_slice(&gen_utf8(&mut rng, 4));
        one(&s, &mut d);
    }
    d.finish("_pcre2_valid_utf_8: valid 1/2/3/4-byte forms, random bytes, all malformed classes");
}

// ================================================================== chkdint

// CONFIGS row: _pcre2_ckd_smul_8 — positive, zero, negative and huge factors.
#[test]
fn ckd_smul() {
    let p = pair();
    let mut rng = Rng::new(5);
    let mut d = Diffs::new();
    let edges: [c_int; 11] = [
        0, 1, -1, 2, -2, 46341, 65536, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];
    let one = |a: c_int, b: c_int, d: &mut Diffs| unsafe {
        let (mut ra, mut rb) = (0xAAAA_AAAA_AAAA_AAAAusize, 0xAAAA_AAAA_AAAA_AAAAusize);
        let oa = (p.c.p_ckd_smul)(&mut ra, a, b);
        let ob = (p.r.p_ckd_smul)(&mut rb, a, b);
        d.eq(&format!("ckd_smul({a},{b}) overflow"), oa, ob);
        d.eq(&format!("ckd_smul({a},{b}) result"), ra, rb);
    };
    for &a in &edges {
        for &b in &edges {
            one(a, b, &mut d);
        }
    }
    for _ in 0..N {
        let a = rng.next_u32() as c_int;
        let b = rng.next_u32() as c_int;
        one(a, b, &mut d);
        one(rng.below(70000) as c_int, rng.below(70000) as c_int, &mut d);
    }
    d.finish("_pcre2_ckd_smul_8: edge x edge, random full-range and small-positive factors");
}

// ================================================================== newline

// CONFIGS rows: _pcre2_is_newline_8 / _pcre2_was_newline_8 across
// NLTYPE_ANY / NLTYPE_ANYCRLF x utf on/off, over data containing every
// newline character (LF VT FF CR NEL LS PS).
#[test]
fn is_and_was_newline() {
    let p = pair();
    let mut rng = Rng::new(6);
    let mut d = Diffs::new();
    // alphabet loaded with newline characters, incl. multi-byte NEL/LS/PS
    let pieces: &[&[u8]] = &[
        b"a", b"\n", b"\x0b", b"\x0c", b"\r", b"\r\n", b"\xc2\x85", /* NEL */
        b"\xe2\x80\xa8", /* LS */
        b"\xe2\x80\xa9", /* PS */
        b"\x85", b"\xc2", b"\xe2\x80",
    ];
    for _ in 0..N {
        let mut s = Vec::new();
        let n = rng.range(1, 8);
        for _ in 0..n {
            s.extend_from_slice(rng.pick_bytes(pieces));
        }
        let end = unsafe { s.as_ptr().add(s.len()) };
        // is_newline: ptr must be < endptr
        for ty in [1u32, 2] {
            for utf in [0 as Bool, 1] {
                let i = rng.below(s.len());
                // in UTF mode the pointer must be on a character boundary
                let i = if utf != 0 {
                    let mut j = i;
                    while j > 0 && (s[j] & 0xc0) == 0x80 {
                        j -= 1;
                    }
                    j
                } else {
                    i
                };
                unsafe {
                    let ptr = s.as_ptr().add(i);
                    let (mut la, mut lb) = (0xFFFF_FFFFu32, 0xFFFF_FFFFu32);
                    let ra = (p.c.p_is_newline)(ptr, ty, end, &mut la, utf);
                    let rb = (p.r.p_is_newline)(ptr, ty, end, &mut lb, utf);
                    let tag = format!("is_newline({}, @{i}, ty={ty}, utf={utf})", show(&s));
                    d.eq(&format!("{tag} rc"), ra, rb);
                    if ra != 0 && rb != 0 {
                        d.eq(&format!("{tag} len"), la, lb);
                    }
                }
                // was_newline: ptr must be > start
                let j = rng.range(1, s.len());
                let j = if utf != 0 {
                    let mut j = j;
                    while j < s.len() && (s[j] & 0xc0) == 0x80 {
                        j += 1;
                    }
                    j.max(1)
                } else {
                    j
                };
                unsafe {
                    let ptr = s.as_ptr().add(j);
                    let (mut la, mut lb) = (0xFFFF_FFFFu32, 0xFFFF_FFFFu32);
                    let ra = (p.c.p_was_newline)(ptr, ty, s.as_ptr(), &mut la, utf);
                    let rb = (p.r.p_was_newline)(ptr, ty, s.as_ptr(), &mut lb, utf);
                    let tag = format!("was_newline({}, @{j}, ty={ty}, utf={utf})", show(&s));
                    d.eq(&format!("{tag} rc"), ra, rb);
                    if ra != 0 && rb != 0 {
                        d.eq(&format!("{tag} len"), la, lb);
                    }
                }
            }
        }
    }
    d.finish("_pcre2_is_newline_8 / _pcre2_was_newline_8: NLTYPE_ANY|ANYCRLF x utf on/off");
}

// =============================================================== extuni / scripts

// CONFIGS row: _pcre2_extuni_8 — grapheme cluster stepping, utf on/off,
// with and without an xcount accumulator.
#[test]
fn extuni() {
    let p = pair();
    let mut rng = Rng::new(7);
    let mut d = Diffs::new();
    // include combining marks, ZWJ, regional indicators, Hangul jamo, emoji
    let pieces: &[&[u8]] = &[
        b"a",
        b"\xcc\x81",         // U+0301 combining acute
        b"\xe2\x80\x8d",     // U+200D ZWJ
        b"\xf0\x9f\x87\xa6", // U+1F1E6 regional indicator A
        b"\xf0\x9f\x87\xa7", // U+1F1E7 regional indicator B
        b"\xe1\x84\x80",     // U+1100 Hangul choseong
        b"\xe1\x85\xa1",     // U+1161 Hangul jungseong
        b"\xe1\x86\xa8",     // U+11A8 Hangul jongseong
        b"\xf0\x9f\x91\xa8", // U+1F468 man
        b"\xe2\x9d\xa4",     // U+2764 heart
        b"\xef\xb8\x8f",     // U+FE0F variation selector
        b"\xe0\xa4\xa8",     // U+0928 devanagari na
        b"\xe0\xa4\xbe",     // U+093E devanagari sign aa
        b"\n",
        b"\r",
    ];
    for _ in 0..N {
        let mut s = Vec::new();
        for _ in 0..rng.range(1, 8) {
            s.extend_from_slice(rng.pick_bytes(pieces));
        }
        for utf in [0 as Bool, 1] {
            unsafe {
                let start = s.as_ptr();
                let end = start.add(s.len());
                // decode the first character the way the engine would
                let (c, adv) = if utf != 0 {
                    match std::str::from_utf8(&s) {
                        Ok(t) => {
                            let ch = t.chars().next().unwrap();
                            (ch as u32, ch.len_utf8())
                        }
                        Err(_) => continue,
                    }
                } else {
                    (s[0] as u32, 1)
                };
                let eptr = start.add(adv);
                for use_count in [false, true] {
                    let (mut xa, mut xb) = (0 as c_int, 0 as c_int);
                    let (pa, pb) = if use_count {
                        (&mut xa as *mut c_int, &mut xb as *mut c_int)
                    } else {
                        (ptr::null_mut(), ptr::null_mut())
                    };
                    let ra = (p.c.p_extuni)(c, eptr, start, end, utf, pa);
                    let rb = (p.r.p_extuni)(c, eptr, start, end, utf, pb);
                    let tag = format!("extuni({}, utf={utf}, xcount={use_count})", show(&s));
                    d.eq(
                        &format!("{tag} advance"),
                        ra as usize - start as usize,
                        rb as usize - start as usize,
                    );
                    if use_count {
                        d.eq(&format!("{tag} xcount"), xa, xb);
                    }
                }
            }
        }
    }
    d.finish("_pcre2_extuni_8: grapheme clusters (marks/ZWJ/RI/Hangul/emoji), utf on/off, xcount null+set");
}

// CONFIGS row: _pcre2_script_run_8 — single-script and mixed-script spans.
#[test]
fn script_run() {
    let p = pair();
    let mut rng = Rng::new(8);
    let mut d = Diffs::new();
    let pieces: &[&[u8]] = &[
        b"a", b"Z", b"0", b"9", b"_",
        b"\xd0\xb0",         // Cyrillic a
        b"\xce\xb1",         // Greek alpha
        b"\xd7\x90",         // Hebrew alef
        b"\xd8\xa7",         // Arabic alef
        b"\xe3\x81\x82",     // Hiragana a
        b"\xe4\xb8\x80",     // Han one
        b"\xe0\xa4\xa8",     // Devanagari na
        b"\xcc\x81",         // combining acute (inherited)
        b"\xef\xbc\x91",     // fullwidth digit one
        b"\xd9\xa1",         // Arabic-Indic digit one
        b"\xe0\xa5\xa7",     // Devanagari digit one
    ];
    for _ in 0..N {
        let mut s = Vec::new();
        for _ in 0..rng.range(1, 7) {
            s.extend_from_slice(rng.pick_bytes(pieces));
        }
        for utf in [0 as Bool, 1] {
            if utf != 0 && std::str::from_utf8(&s).is_err() {
                continue;
            }
            unsafe {
                let start = s.as_ptr();
                let end = start.add(s.len());
                d.eq(
                    &format!("script_run({}, utf={utf})", show(&s)),
                    (p.c.p_script_run)(start, end, utf),
                    (p.r.p_script_run)(start, end, utf),
                );
            }
        }
    }
    d.finish("_pcre2_script_run_8: single/mixed script spans incl. digits and inherited marks, utf on/off");
}

// ============================================================ class bitmaps

// CONFIGS row: _pcre2_update_classbits_8 — every property type, many pdata
// values, negated on/off, over pre-seeded bitmaps.
#[test]
fn update_classbits() {
    let p = pair();
    let mut rng = Rng::new(9);
    let mut d = Diffs::new();
    // ptype range: PT_ANY .. PT_TABLE_LENGTH; go one past to cover the default
    for ptype in 0u32..24 {
        for pdata in 0u32..40 {
            for negated in [0 as Bool, 1] {
                unsafe {
                    let seed: [u8; 32] = std::array::from_fn(|_| rng.byte());
                    let mut ba = seed;
                    let mut bb = seed;
                    (p.c.p_update_classbits)(ptype, pdata, negated, ba.as_mut_ptr());
                    (p.r.p_update_classbits)(ptype, pdata, negated, bb.as_mut_ptr());
                    d.eq(
                        &format!("update_classbits(ptype={ptype}, pdata={pdata}, neg={negated})"),
                        ba,
                        bb,
                    );
                }
            }
        }
    }
    d.finish("_pcre2_update_classbits_8: ptype 0..23 x pdata 0..39 x negated, random seed bitmaps");
}

// ============================================================== name hashing

// CONFIGS row: _pcre2_compile_get_hash_from_name8 — all lengths incl. 0.
#[test]
fn get_hash_from_name() {
    let p = pair();
    let mut rng = Rng::new(10);
    let mut d = Diffs::new();
    for _ in 0..N {
        let mut s = gen_ascii(&mut rng, 40);
        if s.is_empty() {
            s.push(b'x');
        }
        // The C reads name[0] and name[length-1] unconditionally
        // (`PCRE2_ASSERT(length > 0)` compiles away here), so length 0 is
        // out-of-bounds in the C itself and not a meaningful differential.
        let n = rng.range(1, s.len()) as u32;
        unsafe {
            d.eq(
                &format!("get_hash_from_name({}, {n})", show(&s)),
                (p.c.p_get_hash_from_name)(s.as_ptr(), n),
                (p.r.p_get_hash_from_name)(s.as_ptr(), n),
            );
        }
    }
    // also every single byte value, length 1
    for b in 0u8..=255 {
        let s = [b];
        unsafe {
            d.eq(
                &format!("get_hash_from_name([{b:#04x}], 1)"),
                (p.c.p_get_hash_from_name)(s.as_ptr(), 1),
                (p.r.p_get_hash_from_name)(s.as_ptr(), 1),
            );
        }
    }
    d.finish("_pcre2_compile_get_hash_from_name8: random names, lengths 0..len, all single bytes");
}

// ============================================================== JIT stubs

// CONFIGS row: the no-JIT stubs must agree exactly (this build has no JIT).
#[test]
fn jit_stubs() {
    let p = pair();
    unsafe {
        let (ta, tb) = ((p.c.p_jit_get_target)(), (p.r.p_jit_get_target)());
        match (ta.is_null(), tb.is_null()) {
            (true, true) => {}
            (false, false) => assert_eq!(CStr::from_ptr(ta), CStr::from_ptr(tb)),
            _ => panic!("_pcre2_jit_get_target_8 nullness differs"),
        }
        assert_eq!(
            (p.c.p_jit_get_size)(ptr::null_mut()),
            (p.r.p_jit_get_size)(ptr::null_mut()),
            "_pcre2_jit_get_size_8(NULL) differs"
        );
        // pcre2_jit_stack_create must fail identically without JIT
        for (a, b) in [(1usize, 1usize), (32 * 1024, 512 * 1024), (0, 0)] {
            let sa = (p.c.jit_stack_create)(a, b, ptr::null_mut());
            let sb = (p.r.jit_stack_create)(a, b, ptr::null_mut());
            assert_eq!(
                sa.is_null(),
                sb.is_null(),
                "jit_stack_create({a},{b}) nullness differs"
            );
            if !sa.is_null() {
                (p.c.jit_stack_free)(sa);
            }
            if !sb.is_null() {
                (p.r.jit_stack_free)(sb);
            }
        }
        // free_unused_memory / stack_assign are no-ops; just prove they don't trap
        (p.c.jit_free_unused_memory)(ptr::null_mut());
        (p.r.jit_free_unused_memory)(ptr::null_mut());
        (p.c.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        (p.r.jit_stack_assign)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
    }
}

// ============================================================ memctl_malloc

static mut REQ_C: usize = 0;
static mut REQ_R: usize = 0;

unsafe extern "C" fn rec_malloc_c(n: usize, _d: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    REQ_C = n;
    libc_malloc(n)
}
unsafe extern "C" fn rec_malloc_r(n: usize, _d: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    REQ_R = n;
    libc_malloc(n)
}
unsafe extern "C" fn rec_free(p: *mut std::ffi::c_void, _d: *mut std::ffi::c_void) {
    libc_free(p)
}

// Minimal allocator so the test does not depend on the `libc` crate.
unsafe fn libc_malloc(n: usize) -> *mut std::ffi::c_void {
    let layout = std::alloc::Layout::from_size_align(n.max(1) + 16, 16).unwrap();
    let p = std::alloc::alloc(layout);
    *(p as *mut usize) = n.max(1) + 16;
    p.add(16) as *mut std::ffi::c_void
}
unsafe fn libc_free(p: *mut std::ffi::c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

#[repr(C)]
struct MemCtl {
    malloc: Option<MallocFn>,
    free: Option<FreeFn>,
    memory_data: *mut std::ffi::c_void,
}

// CONFIGS row: _pcre2_memctl_malloc_8 — the requested size must match exactly
// (it embeds a pcre2_memctl header ahead of the payload).
#[test]
fn memctl_malloc() {
    let p = pair();
    unsafe {
        // The C writes a whole `pcre2_memctl` (3 pointers = 24 bytes) at the
        // front of the block it obtains, so every real caller passes
        // `size >= sizeof(pcre2_memctl)`; smaller sizes corrupt the heap in the
        // C too and are not a meaningful differential.
        const MEMCTL: usize = std::mem::size_of::<MemCtl>();
        for size in [MEMCTL, MEMCTL + 1, 32, 88, 96, 100, 4096, 65536] {
            let mut ma = MemCtl {
                malloc: Some(rec_malloc_c),
                free: Some(rec_free),
                memory_data: ptr::null_mut(),
            };
            let mut mb = MemCtl {
                malloc: Some(rec_malloc_r),
                free: Some(rec_free),
                memory_data: ptr::null_mut(),
            };
            REQ_C = 0;
            REQ_R = 0;
            let a = (p.c.p_memctl_malloc)(size, &mut ma as *mut _ as Ptr);
            let b = (p.r.p_memctl_malloc)(size, &mut mb as *mut _ as Ptr);
            assert_eq!(a.is_null(), b.is_null(), "memctl_malloc({size}) nullness differs");
            assert_eq!(REQ_C, REQ_R, "memctl_malloc({size}) requested size differs");
            assert!(REQ_C >= size, "requested less than asked");
            // the header written into the block must carry the memctl through
            if !a.is_null() {
                let ha = &*(a as *const MemCtl);
                let hb = &*(b as *const MemCtl);
                assert_eq!(ha.memory_data, hb.memory_data);
                assert!(ha.malloc.is_some() && hb.malloc.is_some());
                rec_free(a, ptr::null_mut());
                rec_free(b, ptr::null_mut());
            }
        }
    }
}
