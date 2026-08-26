//! Phase B, CONFIGS.md rows 13-19: `stbds_arrgrowf`, `stbds_arrfreef`,
//! `arr_push`, `strkey`.

mod common;
use common::*;
use std::ffi::c_void;

/// Emulate `stbds_arrput(a, v)` on top of the raw grow function, for an
/// `elemsize`-byte element type.
unsafe fn arrput(lib: &Lib, a: *mut c_void, elemsize: usize, val: &[u8]) -> *mut c_void {
    let mut a = a;
    let grow = a.is_null() || {
        let h = (a as *mut u8).sub(HDR_SIZE);
        rd_usize(h, HDR_LENGTH) + 1 > rd_usize(h, HDR_CAPACITY)
    };
    if grow {
        a = (lib.arrgrowf)(a, elemsize, 1, 0);
    }
    let h = (a as *mut u8).sub(HDR_SIZE);
    let len = rd_usize(h, HDR_LENGTH);
    std::ptr::copy_nonoverlapping(
        val.as_ptr(),
        (a as *mut u8).add(len * elemsize),
        val.len().min(elemsize),
    );
    wr_usize(h, HDR_LENGTH, len + 1);
    a
}

// ---------------------------------------------------------------------------
// row 13 — arrgrowf from NULL, full cross product
// ---------------------------------------------------------------------------
#[test]
fn cfg13_arrgrowf_from_null_cross_product() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let elemsizes = [1usize, 2, 4, 8, 16, 24, 32, 100];
    let addlens = [0usize, 1, 2, 3, 4, 5, 7, 8, 100];
    let mincaps = [0usize, 1, 2, 3, 4, 5, 8, 100];
    for &es in &elemsizes {
        for &al in &addlens {
            for &mc in &mincaps {
                unsafe {
                    let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, al, mc);
                    let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, al, mc);
                    let ctx = format!("arrgrowf(NULL, es={es}, addlen={al}, min_cap={mc})");
                    // both must decide identically whether to allocate
                    assert_eq_ctx(a.is_null(), b.is_null(), &ctx);
                    if a.is_null() {
                        continue;
                    }
                    let ha = (a as *mut u8).sub(HDR_SIZE);
                    let hb = (b as *mut u8).sub(HDR_SIZE);
                    assert_eq_ctx(
                        (
                            rd_usize(ha, HDR_LENGTH),
                            rd_usize(ha, HDR_CAPACITY),
                            rd_ptr(ha, HDR_HASH_TABLE).is_null(),
                            rd_isize(ha, HDR_TEMP),
                        ),
                        (
                            rd_usize(hb, HDR_LENGTH),
                            rd_usize(hb, HDR_CAPACITY),
                            rd_ptr(hb, HDR_HASH_TABLE).is_null(),
                            rd_isize(hb, HDR_TEMP),
                        ),
                        &ctx,
                    );
                    // the C growth ladder, recomputed independently
                    let min_len = 0usize + al;
                    let mut want = if min_len > mc { min_len } else { mc };
                    // arrcap(NULL) == 0, so `min_cap <= 0` would have returned
                    // NULL and we would not be here.
                    assert!(want > 0);
                    // `min_cap < 2*0` is impossible, so only the `< 4` rung applies
                    if want < 4 {
                        want = 4;
                    }
                    assert_eq_ctx(rd_usize(ha, HDR_CAPACITY), want, &format!("{ctx}: capacity"));
                    (p.c.arrfreef)(a);
                    (p.rs.arrfreef)(b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 14 — arrgrowf on a non-NULL array
// ---------------------------------------------------------------------------
#[test]
fn cfg14_arrgrowf_from_existing() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let elemsizes = [1usize, 4, 8, 16, 24];
    let starts = [1usize, 4, 5, 8, 17];
    let addlens = [0usize, 1, 2, 5, 9, 64];
    let mincaps = [0usize, 1, 4, 5, 8, 9, 16, 33, 200];
    let mut r = Rng::new(0x140014);
    for &es in &elemsizes {
        for &start in &starts {
            for &al in &addlens {
                for &mc in &mincaps {
                    unsafe {
                        // build two identical arrays with `start` elements
                        let mut a = (p.c.arrgrowf)(std::ptr::null_mut(), es, start, 0);
                        let mut b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, start, 0);
                        let payload = r.bytes(es * start);
                        std::ptr::copy_nonoverlapping(payload.as_ptr(), a as *mut u8, es * start);
                        std::ptr::copy_nonoverlapping(payload.as_ptr(), b as *mut u8, es * start);
                        wr_usize((a as *mut u8).sub(HDR_SIZE), HDR_LENGTH, start);
                        wr_usize((b as *mut u8).sub(HDR_SIZE), HDR_LENGTH, start);
                        // give `temp` and `hash_table` distinctive values so we
                        // can see that growth preserves them
                        (( (a as *mut u8).sub(HDR_SIZE).add(HDR_TEMP)) as *mut isize)
                            .write_unaligned(-7);
                        (( (b as *mut u8).sub(HDR_SIZE).add(HDR_TEMP)) as *mut isize)
                            .write_unaligned(-7);

                        let ctx = format!(
                            "arrgrowf(len={start}, es={es}, addlen={al}, min_cap={mc})"
                        );
                        let cap_before = rd_usize((a as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);

                        let a2 = (p.c.arrgrowf)(a, es, al, mc);
                        let b2 = (p.rs.arrgrowf)(b, es, al, mc);

                        assert_snap_eq(&snap_arr(a2, es), &snap_arr(b2, es), &ctx);

                        // independently recomputed C ladder
                        let min_len = start + al;
                        let mut want = if min_len > mc { min_len } else { mc };
                        // The `min_cap <= arrcap(a)` early-out returns the SAME
                        // pointer without calling realloc, so pointer identity
                        // IS a deterministic observable on that branch.  When a
                        // realloc does happen the address is up to the
                        // allocator, so it must not be compared.
                        if want <= cap_before {
                            assert_eq_ctx(a2 == a, true, &format!("{ctx}: C early-out"));
                            assert_eq_ctx(b2 == b, true, &format!("{ctx}: Rust early-out"));
                        }
                        let expect_cap = if want <= cap_before {
                            cap_before
                        } else {
                            if want < 2 * cap_before {
                                want = 2 * cap_before;
                            } else if want < 4 {
                                want = 4;
                            }
                            want
                        };
                        assert_eq_ctx(
                            rd_usize((a2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY),
                            expect_cap,
                            &format!("{ctx}: capacity ladder"),
                        );
                        a = a2;
                        b = b2;
                        (p.c.arrfreef)(a);
                        (p.rs.arrfreef)(b);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 15 — repeated grow chain: the exact capacity sequence
// ---------------------------------------------------------------------------
#[test]
fn cfg15_arrgrowf_capacity_sequence() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    for es in [1usize, 3, 4, 8, 16, 40] {
        unsafe {
            let mut a: *mut c_void = std::ptr::null_mut();
            let mut b: *mut c_void = std::ptr::null_mut();
            let mut r = Rng::new(0x150015 + es as u64);
            for n in 0..300usize {
                let v = r.bytes(es);
                a = arrput(&p.c, a, es, &v);
                b = arrput(&p.rs, b, es, &v);
                assert_snap_eq(
                    &snap_arr(a, es),
                    &snap_arr(b, es),
                    &format!("push #{n} es={es}"),
                );
            }
            (p.c.arrfreef)(a);
            (p.rs.arrfreef)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// row 16 — the no-op branch returns the SAME pointer, header untouched
// ---------------------------------------------------------------------------
#[test]
fn cfg16_arrgrowf_noop_branch() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // a == NULL, min_cap == 0 and addlen == 0  =>  0 <= 0  =>  return NULL
        for es in [1usize, 4, 8, 32] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert_eq_ctx(a.is_null(), b.is_null(), "arrgrowf(NULL,es,0,0) must be NULL");
            assert!(a.is_null(), "C returned non-NULL for a no-grow request");
        }
        // a != NULL with a request that already fits
        for es in [1usize, 4, 8, 32] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            let before_a = snap_arr(a, es);
            let before_b = snap_arr(b, es);
            for (al, mc) in [(0usize, 0usize), (0, 1), (0, 4), (4, 0), (1, 2)] {
                let a2 = (p.c.arrgrowf)(a, es, al, mc);
                let b2 = (p.rs.arrgrowf)(b, es, al, mc);
                assert_eq_ctx(a2 == a, true, &format!("C noop es={es} al={al} mc={mc}"));
                assert_eq_ctx(b2 == b, true, &format!("Rust noop es={es} al={al} mc={mc}"));
            }
            assert_snap_eq(&before_a, &snap_arr(a, es), "C header untouched");
            assert_snap_eq(&before_b, &snap_arr(b, es), "Rust header untouched");
            (p.c.arrfreef)(a);
            (p.rs.arrfreef)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// row 17 — grow / free interleaved, randomized
// ---------------------------------------------------------------------------
#[test]
fn cfg17_arrgrowf_arrfreef_interleaved() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x170017);
    unsafe {
        for i in 0..200 {
            let es = r.range(1, 64);
            let al = r.below(40);
            let mc = r.below(40);
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, al, mc);
            let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, al, mc);
            let ctx = format!("iter {i} es={es} al={al} mc={mc}");
            assert_eq_ctx(a.is_null(), b.is_null(), &ctx);
            if a.is_null() {
                continue;
            }
            assert_snap_eq(&snap_arr(a, es), &snap_arr(b, es), &ctx);
            // grow again a few times
            let mut a = a;
            let mut b = b;
            for k in 0..r.below(6) {
                let al2 = r.below(20);
                let mc2 = r.below(300);
                a = (p.c.arrgrowf)(a, es, al2, mc2);
                b = (p.rs.arrgrowf)(b, es, al2, mc2);
                assert_snap_eq(
                    &snap_arr(a, es),
                    &snap_arr(b, es),
                    &format!("{ctx} regrow {k} al={al2} mc={mc2}"),
                );
            }
            (p.c.arrfreef)(a);
            (p.rs.arrfreef)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// row 18 — arr_push (the only symbol in the public header)
// ---------------------------------------------------------------------------
#[test]
fn cfg18_arr_push() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for num in [
            0i32, 1, 2, 49, 50, 51, 52, 99, 100, 101, 149, 150, 151, 500, 1000, 5000,
        ] {
            (p.c.arr_push)(num);
            (p.rs.arr_push)(num);
            // arr_push has no observable output; the differential property is
            // that both complete without crashing and leave no global state
            // behind.  Check the global seed is still in lockstep by creating a
            // table on each side.
            let a = (p.c.shmode_func)(8, 0);
            let b = (p.rs.shmode_func)(8, 0);
            let sa = rd_usize(rd_ptr((a as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
            let sb = rd_usize(rd_ptr((b as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
            assert_eq_ctx(sa, sb, &format!("arr_push({num}): global seed unchanged"));
            (p.c.hmfree_func)((a as *mut u8).sub(8) as *mut c_void, 8);
            (p.rs.hmfree_func)((b as *mut u8).sub(8) as *mut c_void, 8);
        }
        let mut r = Rng::new(0x180018);
        for _ in 0..64 {
            let num = r.below(3000) as i32;
            (p.c.arr_push)(num);
            (p.rs.arr_push)(num);
        }
    }
}

// ---------------------------------------------------------------------------
// row 19 — strkey
// ---------------------------------------------------------------------------
#[test]
fn cfg19_strkey() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut ns: Vec<i32> = vec![
            0,
            1,
            -1,
            9,
            10,
            11,
            99,
            100,
            999,
            1000,
            12345,
            -12345,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        let mut r = Rng::new(0x190019);
        for _ in 0..256 {
            ns.push(r.u32() as i32);
        }
        for n in ns {
            let ca = (p.c.strkey)(n);
            let ra = (p.rs.strkey)(n);
            let cs = cstr_bytes(ca as *const u8);
            let rs = cstr_bytes(ra as *const u8);
            assert_eq_ctx(
                String::from_utf8_lossy(&cs).to_string(),
                String::from_utf8_lossy(&rs).to_string(),
                &format!("strkey({n})"),
            );
            assert_eq!(cs, format!("test_{n}").into_bytes(), "C strkey({n})");
            // both must return a stable pointer into their own static buffer
            let ca2 = (p.c.strkey)(n);
            let ra2 = (p.rs.strkey)(n);
            assert_eq!(ca, ca2, "C strkey pointer must be the same static buffer");
            assert_eq!(ra, ra2, "Rust strkey pointer must be the same static buffer");
        }
        // consecutive calls with a shorter result must leave the NUL where the
        // C sprintf puts it (i.e. no stale tail visible through the C string)
        let long = (p.c.strkey)(-2147483648);
        let l1 = cstr_bytes(long as *const u8).len();
        let _ = (p.rs.strkey)(-2147483648);
        let short_c = cstr_bytes((p.c.strkey)(1) as *const u8);
        let short_r = cstr_bytes((p.rs.strkey)(1) as *const u8);
        assert_eq!(l1, 16);
        assert_eq_ctx(short_c, short_r, "strkey long-then-short");
    }
}
