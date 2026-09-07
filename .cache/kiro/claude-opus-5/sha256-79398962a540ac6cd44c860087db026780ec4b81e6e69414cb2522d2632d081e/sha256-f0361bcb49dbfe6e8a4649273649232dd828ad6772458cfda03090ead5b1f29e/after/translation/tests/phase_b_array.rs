//! Phase B rows 11–15 — the dynamic-array core (`stbds_arrgrowf` / `stbds_arrfreef`).

mod common;
use common::*;
use std::ffi::c_void;

const ELEMSIZES: [usize; 8] = [1, 4, 8, 12, 16, 20, 32, 64];

/// CONFIGS row 11 — bootstrap from NULL over the full (elemsize, addlen, min_cap) grid.
#[test]
fn cfg_11_arrgrowf_from_null_grid() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            for addlen in 0..9usize {
                for min_cap in 0..9usize {
                    let ac = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let ar = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let sc = snap_array(ac, es, 0);
                    let sr = snap_array(ar, es, 0);
                    assert_eq!(
                        sc, sr,
                        "arrgrowf(NULL, es={}, addlen={}, min_cap={})",
                        es, addlen, min_cap
                    );
                    // addlen == 0 && min_cap == 0 hits `min_cap <= arrcap(NULL)`
                    // and returns NULL without allocating.
                    assert_eq!(
                        ac.is_null(),
                        addlen == 0 && min_cap == 0,
                        "unexpected NULL-ness es={} addlen={} min_cap={}",
                        es,
                        addlen,
                        min_cap
                    );
                    if !ac.is_null() {
                        (c.arrfreef)(ac);
                        (r.arrfreef)(ar);
                    }
                }
            }
        }
    });
}

/// CONFIGS row 12 — repeated `addlen=1, min_cap=0` growth (the doubling rule).
#[test]
fn cfg_12_arrgrowf_doubling_chain() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            for step in 0..200usize {
                ac = (c.arrgrowf)(ac, es, 1, 0);
                ar = (r.arrgrowf)(ar, es, 1, 0);
                // simulate arrput: bump length like the stbds_arrput macro does
                let hc = (ac as *mut ArrayHeader).wrapping_sub(1);
                let hr = (ar as *mut ArrayHeader).wrapping_sub(1);
                assert_eq!(
                    snap_array(ac, es, 0),
                    snap_array(ar, es, 0),
                    "arrgrowf chain es={} step={}",
                    es,
                    step
                );
                (*hc).length += 1;
                (*hr).length += 1;
            }
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    });
}

/// CONFIGS row 13 — `min_cap <= arrcap(a)`: the no-op branch returns `a` itself.
#[test]
fn cfg_13_arrgrowf_noop_identity() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            for min_cap in 0..11usize {
                let bc = (c.arrgrowf)(ac, es, 0, min_cap);
                let br = (r.arrgrowf)(ar, es, 0, min_cap);
                assert_eq!(
                    bc == ac,
                    br == ar,
                    "identity mismatch es={} min_cap={} (C same={}, Rust same={})",
                    es,
                    min_cap,
                    bc == ac,
                    br == ar
                );
                assert!(bc == ac, "expected the no-op branch for min_cap={}", min_cap);
                assert_eq!(snap_array(bc, es, 0), snap_array(br, es, 0));
            }
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    });
}

/// CONFIGS row 14 — `min_cap` in `(cap, 2*cap)` (rounds to `2*cap`) and `> 2*cap` (exact).
#[test]
fn cfg_14_arrgrowf_mincap_rules() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_2001);
        for &es in &ELEMSIZES {
            for _ in 0..80 {
                let start = 1 + rng.below(40);
                let mut ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, start);
                let mut ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, start);
                let cap = (*(ac as *mut ArrayHeader).wrapping_sub(1)).capacity;
                for min_cap in [
                    cap,
                    cap + 1,
                    cap + cap / 2,
                    2 * cap - 1,
                    2 * cap,
                    2 * cap + 1,
                    4 * cap,
                ] {
                    for addlen in [0usize, 1, 3, 100] {
                        let bc = (c.arrgrowf)(ac, es, addlen, min_cap);
                        let br = (r.arrgrowf)(ar, es, addlen, min_cap);
                        assert_eq!(
                            snap_array(bc, es, 0),
                            snap_array(br, es, 0),
                            "es={} cap={} min_cap={} addlen={}",
                            es,
                            cap,
                            min_cap,
                            addlen
                        );
                        ac = bc;
                        ar = br;
                    }
                }
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
    });
}

/// CONFIGS row 15 — payload survives growth; header sits at the same offset.
#[test]
fn cfg_15_arrgrowf_payload_preserved() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_2002);
        for &es in &ELEMSIZES {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            let mut n = 0usize;
            for _ in 0..64usize {
                ac = (c.arrgrowf)(ac, es, 1, 0);
                ar = (r.arrgrowf)(ar, es, 1, 0);
                let payload = rng.bytes(es);
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ac as *mut u8).add(es * n),
                    es,
                );
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ar as *mut u8).add(es * n),
                    es,
                );
                n += 1;
                (*(ac as *mut ArrayHeader).wrapping_sub(1)).length = n;
                (*(ar as *mut ArrayHeader).wrapping_sub(1)).length = n;
                assert_eq!(
                    snap_array(ac, es, n),
                    snap_array(ar, es, n),
                    "payload/header divergence es={} n={}",
                    es,
                    n
                );
            }
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    });
}
