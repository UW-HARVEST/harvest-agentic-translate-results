//! Phase B — CONFIGS.md rows 1-13: the lowest-level entry points
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`,
//! `stbds_arrgrowf`, `stbds_arrfreef`).

mod common;
use common::*;
use std::ffi::c_void;

const SEEDS: [usize; 6] = [
    0,
    1,
    0x3141_5926,
    usize::MAX,
    0x8000_0000_0000_0000,
    0xdead_beef_cafe_f00d,
];

/// rows 1-6
#[test]
fn row_1_to_6_hash_bytes() {
    let (_g, b) = both();
    let mut rng = Rng::new(0xB0);

    // row 1: len == 0 (and a NULL pointer, ERRORS.md #44)
    for &s in &SEEDS {
        unsafe {
            let cv = (b.c.hash_bytes)(std::ptr::null_mut(), 0, s);
            let rv = (b.r.hash_bytes)(std::ptr::null_mut(), 0, s);
            assert_eq!(cv, rv, "row1 len=0 seed={s:#x}");
        }
    }

    // rows 2-5: every length 0..=256, many random buffers each
    for len in 0usize..=256 {
        let reps = if len <= 16 { 40 } else { 6 };
        for _ in 0..reps {
            let data = rng.bytes(len);
            let cb = CBuf::new(&data);
            let rb = CBuf::new(&data);
            for &s in &SEEDS {
                unsafe {
                    let cv = (b.c.hash_bytes)(cb.ptr(), len, s);
                    let rv = (b.r.hash_bytes)(rb.ptr(), len, s);
                    assert_eq!(cv, rv, "hash_bytes len={len} seed={s:#x} data={data:02x?}");
                }
            }
        }
    }

    // row 6: force the high bit at every offset (int-overflow sign-extension)
    for len in 1usize..=40 {
        for pattern in [0x80u8, 0xFF, 0x7F, 0x00] {
            let mut data = rng.bytes(len);
            for (i, x) in data.iter_mut().enumerate() {
                if i % 4 == 3 {
                    *x = pattern;
                }
            }
            let cb = CBuf::new(&data);
            let rb = CBuf::new(&data);
            for &s in &SEEDS {
                unsafe {
                    assert_eq!(
                        (b.c.hash_bytes)(cb.ptr(), len, s),
                        (b.r.hash_bytes)(rb.ptr(), len, s),
                        "row6 len={len} pat={pattern:#x} seed={s:#x}"
                    );
                }
            }
        }
    }

    // all-0xFF and all-0x00 buffers of every length
    for len in 0usize..=72 {
        for fill in [0x00u8, 0xFF, 0x80] {
            let data = vec![fill; len];
            let cb = CBuf::new(&data);
            let rb = CBuf::new(&data);
            for &s in &SEEDS {
                unsafe {
                    assert_eq!(
                        (b.c.hash_bytes)(cb.ptr(), len, s),
                        (b.r.hash_bytes)(rb.ptr(), len, s),
                        "fill={fill:#x} len={len} seed={s:#x}"
                    );
                }
            }
        }
    }
}

/// row 7
#[test]
fn row_7_hash_string() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x57);

    // empty string (ERRORS.md #41)
    let e = CBuf::new(b"\0");
    for &s in &SEEDS {
        unsafe {
            assert_eq!(
                (b.c.hash_string)(e.cptr(), s),
                (b.r.hash_string)(e.cptr(), s),
                "empty string seed={s:#x}"
            );
        }
    }

    for len in 1usize..=200 {
        let reps = if len <= 8 { 20 } else { 4 };
        for _ in 0..reps {
            let sbuf = rng.cstring(len);
            let cb = CBuf::new(&sbuf);
            for &s in &SEEDS {
                unsafe {
                    assert_eq!(
                        (b.c.hash_string)(cb.cptr(), s),
                        (b.r.hash_string)(cb.cptr(), s),
                        "hash_string len={len} seed={s:#x}"
                    );
                }
            }
        }
    }

    // pure-high-byte strings (`(unsigned char) *str` promotion)
    for len in 1usize..=32 {
        let sbuf: Vec<u8> = (0..len).map(|i| 0x80u8 + (i as u8 & 0x7f)).chain([0]).collect();
        let cb = CBuf::new(&sbuf);
        for &s in &SEEDS {
            unsafe {
                assert_eq!(
                    (b.c.hash_string)(cb.cptr(), s),
                    (b.r.hash_string)(cb.cptr(), s),
                    "high-byte string len={len} seed={s:#x}"
                );
            }
        }
    }
}

/// row 8 — `stbds_rand_seed` and the seed advance inside `make_hash_index`
#[test]
fn row_8_rand_seed_advance() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x8E);

    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, usize::MAX - 1, 0x3141_5926];
    for _ in 0..16 {
        seeds.push(rng.next_u64() as usize);
    }

    for s in seeds {
        seed_both(b, s);
        // Each shmode_func creates a table with ot == NULL, so it consumes and
        // advances the global seed.  Compare the whole chain.
        let mut ct = Vec::new();
        let mut rt = Vec::new();
        for _ in 0..8 {
            unsafe {
                let c = (b.c.shmode_func)(16, 2);
                let r = (b.r.shmode_func)(16, 2);
                ct.push(c);
                rt.push(r);
                let cs = snap(c, 16, 8, KeyKind::Str, 8, 8);
                let rs = snap(r, 16, 8, KeyKind::Str, 8, 8);
                assert_eq!(cs.seed, rs.seed, "seed chain start={s:#x}");
                assert_eq!(cs, rs, "shmode_func snapshot start={s:#x}");
            }
        }
        for (c, r) in ct.into_iter().zip(rt) {
            unsafe {
                (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
                (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
            }
        }
    }
}

fn growf_snapshot(p: *mut c_void) -> (usize, usize, bool, isize) {
    unsafe {
        let h = &*hdr(p);
        (h.length, h.capacity, h.hash_table.is_null(), h.temp)
    }
}

/// rows 9-13 — `stbds_arrgrowf` / `stbds_arrfreef`
#[test]
fn row_9_to_13_arrgrowf() {
    let (_g, b) = both();

    let elemsizes = [1usize, 2, 4, 8, 16, 24, 32, 7];
    let args = [0usize, 1, 2, 3, 4, 5, 8, 17, 63, 64, 1000];

    // row 9: a == NULL
    for &es in &elemsizes {
        for &addlen in &args {
            for &min_cap in &args {
                unsafe {
                    let c = (b.c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let r = (b.r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    // ERRORS.md #1: with addlen == 0 and min_cap == 0 the
                    // `min_cap <= arrcap(a)` early-return hands back NULL.
                    assert_eq!(
                        c.is_null(),
                        r.is_null(),
                        "row9 nullness es={es} addlen={addlen} min_cap={min_cap}"
                    );
                    if c.is_null() {
                        assert!(addlen == 0 && min_cap == 0);
                        continue;
                    }
                    assert_eq!(
                        growf_snapshot(c),
                        growf_snapshot(r),
                        "row9 es={es} addlen={addlen} min_cap={min_cap}"
                    );
                    // row 13: free the freshly grown array
                    (b.c.arrfreef)(c);
                    (b.r.arrfreef)(r);
                }
            }
        }
    }

    // rows 10-12: non-NULL `a`, sweeping `min_cap` right across all three
    // capacity branches.  Each case starts from a FRESH array of a known
    // capacity so the sweep cannot grow without bound.
    for &es in &elemsizes {
        for &cap in &[4usize, 5, 8, 12, 16, 33] {
            for len in [0usize, 1, cap / 2, cap] {
                for addlen in [0usize, 1, 2, cap] {
                    for min_cap in 0..=(3 * cap + 3) {
                        unsafe {
                            let mut c = (b.c.arrgrowf)(std::ptr::null_mut(), es, 0, cap);
                            let mut r = (b.r.arrgrowf)(std::ptr::null_mut(), es, 0, cap);
                            let real_cap = (*hdr(c)).capacity;
                            assert_eq!(real_cap, (*hdr(r)).capacity);
                            (*(hdr(c) as *mut ArrHeader)).length = len;
                            (*(hdr(r) as *mut ArrHeader)).length = len;

                            // `stbds_arrgrowf` returns its input untouched iff
                            // max(min_cap, len+addlen) <= capacity.  That
                            // decision is semantic; whether `realloc` returns
                            // the same address when it *is* called is an
                            // allocator artifact and is not compared.
                            let early = min_cap.max(len + addlen) <= real_cap;
                            let c2 = (b.c.arrgrowf)(c, es, addlen, min_cap);
                            let r2 = (b.r.arrgrowf)(r, es, addlen, min_cap);
                            let ctx = format!(
                                "es={es} cap={real_cap} len={len} addlen={addlen} min_cap={min_cap}"
                            );
                            assert_eq!(
                                growf_snapshot(c2),
                                growf_snapshot(r2),
                                "{ctx}"
                            );
                            if early {
                                assert_eq!(c2, c, "C must return its input unchanged ({ctx})");
                                assert_eq!(r2, r, "Rust must return its input unchanged ({ctx})");
                            }
                            let newcap = (*hdr(c2)).capacity;
                            assert!(
                                newcap >= min_cap.max(len + addlen),
                                "capacity {newcap} does not satisfy the request ({ctx})"
                            );
                            c = c2;
                            r = r2;
                            (b.c.arrfreef)(c);
                            (b.r.arrfreef)(r);
                        }
                    }
                }
            }
        }
    }
}

/// ERRORS.md #4 — `addlen` so large that `min_len` wraps.  Only the *decision*
/// is compared; no allocation of that size is attempted because
/// `elemsize * min_cap` also wraps to something small.
#[test]
fn err_4_arrgrowf_overflow_decision() {
    let (_g, b) = both();
    unsafe {
        // arrlen(NULL)=0, addlen=SIZE_MAX -> min_len = SIZE_MAX > min_cap(0)
        // -> min_cap = SIZE_MAX; elemsize(1) * SIZE_MAX + 32 wraps to 31.
        // realloc(NULL, 31) succeeds in both, capacity = SIZE_MAX.
        let c = (b.c.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0);
        let r = (b.r.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0);
        assert!(!c.is_null() && !r.is_null());
        assert_eq!(growf_snapshot(c), growf_snapshot(r), "SIZE_MAX addlen");
        (b.c.arrfreef)(c);
        (b.r.arrfreef)(r);

        // addlen = SIZE_MAX/2 with elemsize 4 -> also wraps
        let c = (b.c.arrgrowf)(std::ptr::null_mut(), 4, usize::MAX / 2 + 9, 0);
        let r = (b.r.arrgrowf)(std::ptr::null_mut(), 4, usize::MAX / 2 + 9, 0);
        assert_eq!(c.is_null(), r.is_null());
        if !c.is_null() {
            assert_eq!(growf_snapshot(c), growf_snapshot(r));
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}
