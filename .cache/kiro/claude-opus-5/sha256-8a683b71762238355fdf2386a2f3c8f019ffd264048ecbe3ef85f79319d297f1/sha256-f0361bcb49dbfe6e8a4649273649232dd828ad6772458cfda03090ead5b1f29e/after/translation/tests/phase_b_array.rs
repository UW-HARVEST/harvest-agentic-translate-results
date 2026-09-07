//! Phase B — CONFIGS.md rows 10-22: `stbds_arrgrowf`, `stbds_arrfreef`,
//! `stbds_hmput_default`, and the table-less `hmget_key*` paths.

mod common;
use common::*;
use std::ffi::c_void;

/// Compare only the fields of a freshly grown array that C actually defines
/// (the payload bytes past `length` are uninitialised in both libraries).
#[track_caller]
unsafe fn eq_grow(ctx: &str, cp: *mut c_void, rp: *mut c_void, elemsize: usize) {
    assert_eq!(cp.is_null(), rp.is_null(), "[{ctx}] nullness");
    if cp.is_null() {
        return;
    }
    let cs = snap_arr(cp, elemsize, ElemFmt::Raw, false);
    let rs = snap_arr(rp, elemsize, ElemFmt::Raw, false);
    eq_snap(ctx, &cs, &rs);
}

#[test]
fn row10_arrgrowf_null_min_cap_below_4() {
    let (p, _g) = libs();
    for elemsize in [1usize, 4, 8, 16, 20, 24] {
        for min_cap in 0usize..=3 {
            unsafe {
                let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let ctx = format!("arrgrowf(NULL,{elemsize},0,{min_cap})");
                // min_cap == 0 hits the `min_cap <= arrcap(NULL)` early return
                // and yields NULL; 1..3 are bumped to the floor of 4.
                if min_cap == 0 {
                    assert!(cp.is_null(), "[{ctx}] C should return NULL");
                    assert!(rp.is_null(), "[{ctx}] Rust should return NULL");
                    continue;
                }
                eq_grow(&ctx, cp, rp, elemsize);
                assert_eq!(snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity, 4);
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

#[test]
fn row11_arrgrowf_null_various_min_cap() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 11);
    for elemsize in [1usize, 4, 8, 16, 20, 24] {
        let mut caps: Vec<usize> = (4..40).collect();
        caps.extend([64, 100, 255, 256, 999, 1000]);
        for _ in 0..200 {
            caps.push(1 + rng.below(100_000));
        }
        for &min_cap in &caps {
            unsafe {
                let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                eq_grow(
                    &format!("arrgrowf(NULL,{elemsize},0,{min_cap})"),
                    cp,
                    rp,
                    elemsize,
                );
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

#[test]
fn row12_arrgrowf_null_addlen() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 12);
    for elemsize in [1usize, 4, 8, 16, 20, 24] {
        let mut lens: Vec<usize> = (0..40).collect();
        for _ in 0..200 {
            lens.push(rng.below(100_000));
        }
        for &addlen in &lens {
            unsafe {
                let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                let ctx = format!("arrgrowf(NULL,{elemsize},{addlen},0)");
                assert_eq!(cp.is_null(), rp.is_null(), "[{ctx}]");
                if cp.is_null() {
                    continue;
                }
                eq_grow(&ctx, cp, rp, elemsize);
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

#[test]
fn row13_arrgrowf_early_return_nop() {
    let (p, _g) = libs();
    for elemsize in [4usize, 8, 16, 24] {
        unsafe {
            let mut cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let mut rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let cap = snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity;
            assert_eq!(cap, 10);
            // set a length so `min_len = length + addlen` is exercised too,
            // and initialise those elements in both copies so the payload is
            // defined and comparable
            ((cp as *mut ArrHeader).sub(1)).as_mut().unwrap().length = 3;
            ((rp as *mut ArrHeader).sub(1)).as_mut().unwrap().length = 3;
            for i in 0..3usize {
                for b in 0..elemsize {
                    let v = ((i * 29 + b * 3) & 0xff) as u8;
                    *(cp as *mut u8).add(i * elemsize + b) = v;
                    *(rp as *mut u8).add(i * elemsize + b) = v;
                }
            }
            for (addlen, min_cap) in [(0usize, 0usize), (0, 5), (0, 10), (7, 0), (7, 10), (1, 1)] {
                let c2 = (p.c.arrgrowf)(cp, elemsize, addlen, min_cap);
                let r2 = (p.rs.arrgrowf)(rp, elemsize, addlen, min_cap);
                let ctx = format!("nop arrgrowf(es={elemsize},addlen={addlen},min_cap={min_cap})");
                assert_eq!(c2, cp, "[{ctx}] C must return the same pointer");
                assert_eq!(r2, rp, "[{ctx}] Rust must return the same pointer");
                eq_grow(&ctx, c2, r2, elemsize);
            }
            // one step past: addlen=8 with length=3 => min_len=11 > cap 10
            let c2 = (p.c.arrgrowf)(cp, elemsize, 8, 0);
            let r2 = (p.rs.arrgrowf)(rp, elemsize, 8, 0);
            eq_grow("grow past cap", c2, r2, elemsize);
            assert_eq!(snap_arr(c2, elemsize, ElemFmt::Raw, false).capacity, 20);
            cp = c2;
            rp = r2;
            (p.c.arrfreef)(cp);
            (p.rs.arrfreef)(rp);
        }
    }
}

#[test]
fn row14_arrgrowf_doubling_straddle() {
    let (p, _g) = libs();
    for elemsize in [4usize, 16] {
        for start_cap in [4usize, 5, 8, 10, 16, 100] {
            for delta in 0..6usize {
                unsafe {
                    let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start_cap);
                    let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start_cap);
                    let cap = snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity;
                    // straddle 2*cap: cap+1 .. 2*cap+2
                    let min_cap = cap + 1 + delta * (cap / 3).max(1);
                    let c2 = (p.c.arrgrowf)(cp, elemsize, 0, min_cap);
                    let r2 = (p.rs.arrgrowf)(rp, elemsize, 0, min_cap);
                    eq_grow(
                        &format!("straddle es={elemsize} cap={cap} min_cap={min_cap}"),
                        c2,
                        r2,
                        elemsize,
                    );
                    let got = snap_arr(c2, elemsize, ElemFmt::Raw, false).capacity;
                    let want = if min_cap < 2 * cap { 2 * cap } else { min_cap };
                    assert_eq!(got, want, "cap={cap} min_cap={min_cap}");
                    (p.c.arrfreef)(c2);
                    (p.rs.arrfreef)(r2);
                }
            }
        }
    }
}

#[test]
fn row15_arrgrowf_random_chains() {
    let (p, _g) = libs();
    for elemsize in [1usize, 4, 8, 16, 20, 24] {
        for trial in 0..60u64 {
            let mut rng = Rng::new(0xC0FFEE ^ 15 ^ (elemsize as u64) << 8 ^ trial);
            unsafe {
                let mut cp: *mut c_void = std::ptr::null_mut();
                let mut rp: *mut c_void = std::ptr::null_mut();
                for step in 0..20 {
                    let addlen = rng.below(40);
                    let min_cap = rng.below(80);
                    cp = (p.c.arrgrowf)(cp, elemsize, addlen, min_cap);
                    rp = (p.rs.arrgrowf)(rp, elemsize, addlen, min_cap);
                    let ctx =
                        format!("chain es={elemsize} trial={trial} step={step} +{addlen}/{min_cap}");
                    assert_eq!(cp.is_null(), rp.is_null(), "[{ctx}] nullness");
                    if cp.is_null() {
                        continue;
                    }
                    // grow the logical length like the arr* macros do and write
                    // a deterministic payload, so that the next realloc's
                    // content preservation is checked too.
                    let cap = snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity;
                    let newlen = rng.below(cap + 1);
                    let ch = (cp as *mut ArrHeader).sub(1);
                    let rh = (rp as *mut ArrHeader).sub(1);
                    let oldlen = (*ch).length;
                    (*ch).length = newlen;
                    (*rh).length = newlen;
                    if newlen > oldlen {
                        for i in oldlen..newlen {
                            for b in 0..elemsize {
                                let v = ((i * 31 + b * 7 + step) & 0xff) as u8;
                                *(cp as *mut u8).add(i * elemsize + b) = v;
                                *(rp as *mut u8).add(i * elemsize + b) = v;
                            }
                        }
                    }
                    eq_grow(&ctx, cp, rp, elemsize);
                }
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

#[test]
fn row16_arrgrowf_then_free() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 16);
    for elemsize in [4usize, 16] {
        for _ in 0..200 {
            unsafe {
                let mut cp: *mut c_void = std::ptr::null_mut();
                let mut rp: *mut c_void = std::ptr::null_mut();
                for _ in 0..8 {
                    let addlen = 1 + rng.below(10);
                    cp = (p.c.arrgrowf)(cp, elemsize, addlen, 0);
                    rp = (p.rs.arrgrowf)(rp, elemsize, addlen, 0);
                    let ch = (cp as *mut ArrHeader).sub(1);
                    let rh = (rp as *mut ArrHeader).sub(1);
                    let old = (*ch).length;
                    (*ch).length = old + addlen;
                    (*rh).length = old + addlen;
                    // initialise the newly exposed elements in both copies
                    for i in old..old + addlen {
                        for b in 0..elemsize {
                            let v = ((i * 13 + b * 5) & 0xff) as u8;
                            *(cp as *mut u8).add(i * elemsize + b) = v;
                            *(rp as *mut u8).add(i * elemsize + b) = v;
                        }
                    }
                    eq_grow("free-chain", cp, rp, elemsize);
                }
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

#[test]
fn row17_hmput_default_null() {
    let (p, _g) = libs();
    for elemsize in [8usize, 16, 20, 24] {
        unsafe {
            let cp = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rp = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
            let cs = snap_hash(cp, elemsize, ElemFmt::Raw);
            let rs = snap_hash(rp, elemsize, ElemFmt::Raw);
            eq_snap(&format!("hmput_default(NULL,{elemsize})"), &cs, &rs);
            assert_eq!(cs.length, 1);
            assert_eq!(cs.capacity, 4);
            assert!(!cs.has_table);
            assert!(cs.elem0.iter().all(|&b| b == 0));
            (p.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.rs.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn row18_hmput_default_nop_on_nonempty() {
    let (p, _g) = libs();
    let elemsize = 16usize;
    unsafe {
        let mut cp = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let mut rp = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
        // write a distinguishable default value
        for b in 0..elemsize {
            *(cp as *mut u8).sub(elemsize).add(b) = (0xA0 + b) as u8;
            *(rp as *mut u8).sub(elemsize).add(b) = (0xA0 + b) as u8;
        }
        for _ in 0..5 {
            let c2 = (p.c.hmput_default)(cp, elemsize);
            let r2 = (p.rs.hmput_default)(rp, elemsize);
            assert_eq!(c2, cp, "C hmput_default must be a NOP here");
            assert_eq!(r2, rp, "Rust hmput_default must be a NOP here");
            eq_snap(
                "hmput_default nop",
                &snap_hash(c2, elemsize, ElemFmt::Raw),
                &snap_hash(r2, elemsize, ElemFmt::Raw),
            );
            cp = c2;
            rp = r2;
        }
        (p.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.rs.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn row19_hmput_default_on_zero_length_array() {
    let (p, _g) = libs();
    for elemsize in [8usize, 16, 24] {
        unsafe {
            // an array built by arrgrowf has length == 0
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7);
            let ra = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7);
            let ch = (ca as *mut u8).add(elemsize) as *mut c_void;
            let rh = (ra as *mut u8).add(elemsize) as *mut c_void;
            let cp = (p.c.hmput_default)(ch, elemsize);
            let rp = (p.rs.hmput_default)(rh, elemsize);
            let cs = snap_hash(cp, elemsize, ElemFmt::Raw);
            let rs = snap_hash(rp, elemsize, ElemFmt::Raw);
            eq_snap(&format!("hmput_default(len0,{elemsize})"), &cs, &rs);
            assert_eq!(cs.length, 1);
            assert_eq!(cs.capacity, 7);
            assert!(cs.elem0.iter().all(|&b| b == 0));
            (p.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.rs.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn row20_hmget_key_ts_null_map() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 20);
    for elemsize in [8usize, 16, 20, 24] {
        for mode in [HM_BINARY, HM_STRING, 2, -1] {
            for keysize in [0usize, 1, 4, 8, 16] {
                let key = CKey::new(&rng.nonnul(keysize.max(1)));
                unsafe {
                    let mut ct: isize = 0x5555;
                    let mut rt: isize = 0x5555;
                    let cp = (p.c.hmget_key_ts)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.ptr() as *mut c_void,
                        keysize,
                        &mut ct,
                        mode,
                    );
                    let rp = (p.rs.hmget_key_ts)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.ptr() as *mut c_void,
                        keysize,
                        &mut rt,
                        mode,
                    );
                    let ctx = format!("hmget_key_ts(NULL,{elemsize},{keysize},mode={mode})");
                    assert_eq!(ct, rt, "[{ctx}] *temp");
                    assert_eq!(ct, -1, "[{ctx}] *temp must be STBDS_INDEX_EMPTY");
                    eq_snap(
                        &ctx,
                        &snap_hash(cp, elemsize, ElemFmt::Raw),
                        &snap_hash(rp, elemsize, ElemFmt::Raw),
                    );
                    (p.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                    (p.rs.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                }
            }
        }
    }
}

#[test]
fn row21_hmget_key_ts_tableless_map() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 21);
    for elemsize in [8usize, 16, 24] {
        for mode in [HM_BINARY, HM_STRING, 7, -3] {
            unsafe {
                let cp = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
                let rp = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
                let key = CKey::new(&rng.nonnul(4));
                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                let c2 = (p.c.hmget_key_ts)(
                    cp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    4,
                    &mut ct,
                    mode,
                );
                let r2 = (p.rs.hmget_key_ts)(
                    rp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    4,
                    &mut rt,
                    mode,
                );
                let ctx = format!("hmget_key_ts(tableless,{elemsize},mode={mode})");
                assert_eq!(ct, rt, "[{ctx}] *temp");
                assert_eq!(ct, -1, "[{ctx}]");
                assert_eq!(c2, cp, "[{ctx}] C must return the input pointer");
                assert_eq!(r2, rp, "[{ctx}] Rust must return the input pointer");
                eq_snap(
                    &ctx,
                    &snap_hash(c2, elemsize, ElemFmt::Raw),
                    &snap_hash(r2, elemsize, ElemFmt::Raw),
                );
                (p.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.rs.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

#[test]
fn row22_hmget_key_sets_header_temp() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 22);
    for elemsize in [8usize, 16, 24] {
        for mode in [HM_BINARY, HM_STRING] {
            let key = CKey::new(&rng.nonnul(4));
            unsafe {
                // a == NULL
                let cp = (p.c.hmget_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.ptr() as *mut c_void,
                    4,
                    mode,
                );
                let rp = (p.rs.hmget_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.ptr() as *mut c_void,
                    4,
                    mode,
                );
                let cs = snap_hash(cp, elemsize, ElemFmt::Raw);
                let rs = snap_hash(rp, elemsize, ElemFmt::Raw);
                eq_snap(&format!("hmget_key(NULL,{elemsize},{mode})"), &cs, &rs);
                assert_eq!(cs.temp, -1, "header->temp must be -1");
                // table-less follow-up on the returned map
                let c2 = (p.c.hmget_key)(cp, elemsize, key.ptr() as *mut c_void, 4, mode);
                let r2 = (p.rs.hmget_key)(rp, elemsize, key.ptr() as *mut c_void, 4, mode);
                eq_snap(
                    "hmget_key(tableless)",
                    &snap_hash(c2, elemsize, ElemFmt::Raw),
                    &snap_hash(r2, elemsize, ElemFmt::Raw),
                );
                assert_eq!(snap_hash(c2, elemsize, ElemFmt::Raw).temp, -1);
                (p.c.hmfree_func)((c2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.rs.hmfree_func)((r2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}
