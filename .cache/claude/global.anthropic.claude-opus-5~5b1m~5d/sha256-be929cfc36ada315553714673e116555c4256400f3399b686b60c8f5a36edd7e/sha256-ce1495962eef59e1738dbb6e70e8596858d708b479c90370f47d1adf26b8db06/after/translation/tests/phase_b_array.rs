//! Phase B rows B1-B8: `stbds_arrgrowf` / `stbds_arrfreef`.

mod common;
use common::*;

const ELEMSIZES: &[usize] = &[1, 2, 4, 8, 12, 16, 64];

/// B1 — a=NULL, addlen=0, min_cap=0: `min_len(0) <= min_cap(0) <= arrcap(NULL)(0)`
/// so lib.c:286 returns `a` — i.e. **NULL**, nothing is allocated.
#[test]
fn cfg_b1_arrgrowf_null_zero_zero() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            let mut a = ArrPair::new(c, r, es);
            let (same_c, same_r) = a.grow(0, 0);
            assert!(same_c && same_r, "B1 es={es}: expected early return");
            assert!(
                a.c.is_null() && a.r.is_null(),
                "B1 es={es}: expected NULL, got C={:?} RUST={:?}",
                a.c,
                a.r
            );
            a.assert_same(&format!("B1 elemsize={es}"));
        }
    });
}

/// B2 — a=NULL, addlen=0, varying min_cap
#[test]
fn cfg_b2_arrgrowf_null_min_cap() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            for &mc in &[1usize, 2, 3, 4, 5, 7, 8, 100, 1000] {
                let mut a = ArrPair::new(c, r, es);
                a.grow(0, mc);
                a.assert_same(&format!("B2 elemsize={es} min_cap={mc}"));
                assert_eq!(a.length(), 0, "B2 len es={es} mc={mc}");
                assert_eq!(a.capacity(), mc.max(4), "B2 cap es={es} mc={mc}");
                a.free();
            }
        }
    });
}

/// B3 — a=NULL, addlen>0, min_cap=0 → min_len > min_cap path
#[test]
fn cfg_b3_arrgrowf_null_addlen() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            for &n in &[1usize, 2, 3, 4, 5, 17, 64, 257] {
                let mut a = ArrPair::new(c, r, es);
                a.grow(n, 0);
                a.assert_same(&format!("B3 elemsize={es} addlen={n}"));
                assert_eq!(a.capacity(), n.max(4), "B3 cap es={es} n={n}");
                a.free();
            }
        }
    });
}

/// B4 — non-NULL, min_cap <= arrcap → early `return a`, pointer unchanged
#[test]
fn cfg_b4_arrgrowf_no_grow() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            let mut a = ArrPair::new(c, r, es);
            a.grow(0, 16); // cap 16
            let before = (a.c, a.r);
            for &(addlen, mc) in &[(0usize, 0usize), (0, 1), (0, 16), (5, 0), (5, 16), (16, 0)] {
                let (same_c, same_r) = a.grow(addlen, mc);
                assert!(
                    same_c && same_r,
                    "B4 es={es} addlen={addlen} mc={mc}: expected no realloc (C={same_c} RUST={same_r})"
                );
                a.assert_same(&format!("B4 es={es} addlen={addlen} mc={mc}"));
                assert_eq!(a.capacity(), 16);
            }
            assert_eq!((a.c, a.r), before);
            a.free();
        }
    });
}

/// B5 — doubling path: min_cap < 2*arrcap
#[test]
fn cfg_b5_arrgrowf_doubling() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            let mut a = ArrPair::new(c, r, es);
            a.grow(0, 4); // cap 4
            let mut expect = 4usize;
            for step in 0..12 {
                a.grow(0, expect + 1); // one past capacity → doubles
                expect *= 2;
                a.assert_same(&format!("B5 es={es} step={step}"));
                assert_eq!(a.capacity(), expect, "B5 es={es} step={step}");
            }
            a.free();
        }
    });
}

/// B6 — min_cap >= 2*arrcap (explicit big request)
#[test]
fn cfg_b6_arrgrowf_big_min_cap() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            let mut a = ArrPair::new(c, r, es);
            a.grow(0, 4); // cap 4
            for &mc in &[8usize, 9, 100, 1000, 4096] {
                a.grow(0, mc);
                a.assert_same(&format!("B6 es={es} mc={mc}"));
            }
            a.free();
        }
    });
}

/// B7 — randomized arrput sequences with payload comparison
#[test]
fn cfg_b7_arrput_random_payload() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            for seed in 0u64..3 {
                let mut rng = Rng::new(seed * 7 + es as u64);
                let mut a = ArrPair::new(c, r, es);
                for i in 0..500 {
                    let e = rng.bytes(es);
                    a.put(&e);
                    if i % 37 == 0 || i > 490 {
                        a.assert_same(&format!("B7 es={es} seed={seed} i={i}"));
                    }
                }
                a.assert_same(&format!("B7 final es={es} seed={seed}"));
                a.free();
            }
        }
    });
}

/// B7b — randomized grow(addlen, min_cap) sequences
#[test]
fn cfg_b7b_arrgrowf_random_sequences() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            for seed in 0u64..5 {
                let mut rng = Rng::new(seed * 101 + es as u64);
                let mut a = ArrPair::new(c, r, es);
                for i in 0..60 {
                    let addlen = rng.below(20);
                    let min_cap = rng.below(200);
                    a.grow(addlen, min_cap);
                    if a.c.is_null() {
                        assert!(a.r.is_null(), "B7b RUST allocated where C did not");
                        continue;
                    }
                    assert!(!a.r.is_null(), "B7b RUST returned NULL where C did not");
                    // emulate arraddn: length += addlen when it fits
                    let cap = a.capacity();
                    let len = a.length();
                    if len + addlen <= cap {
                        let payload = rng.bytes(es * addlen);
                        if addlen > 0 {
                            std::ptr::copy_nonoverlapping(
                                payload.as_ptr(),
                                a.c.wrapping_add(es * len),
                                es * addlen,
                            );
                            std::ptr::copy_nonoverlapping(
                                payload.as_ptr(),
                                a.r.wrapping_add(es * len),
                                es * addlen,
                            );
                        }
                        a.set_length(len + addlen);
                    }
                    a.assert_same(&format!("B7b es={es} seed={seed} i={i}"));
                }
                a.free();
            }
        }
    });
}

/// B8 — grow then free
#[test]
fn cfg_b8_arrgrowf_then_arrfreef() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in ELEMSIZES {
            for &n in &[0usize, 1, 4, 100] {
                let mut a = ArrPair::new(c, r, es);
                a.grow(n, 0);
                a.assert_same(&format!("B8 es={es} n={n}"));
                a.free();
                assert!(a.c.is_null() && a.r.is_null());
            }
        }
    });
}
