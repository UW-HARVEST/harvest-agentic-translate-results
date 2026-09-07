//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input, calls BOTH the C `.so` and the
//! Rust `.so`, and asserts the SAME error code (22 == EINVAL, 34 == ERANGE) AND
//! the same resulting destination buffer -- the side effect `dst[0] = 0` and any
//! partial copy performed before it are part of the observable behaviour.

mod common;

use common::*;

/// E1 — `dst == NULL`, `numElem > 0`, valid `src` => 22.
#[test]
fn e1_null_dst() {
    let mut rng = Rng::new(0xE001);
    for iter in 0..200 {
        let n = rng.range(1, 1 << 20);
        let __len = rng.range(0, 8);
        let src = Domain::AnyNonZero.vec(&mut rng, __len);
        let case = Case::new(vec![], &src).num_elem(n).dst_null();
        assert_same(&format!("E1 iter={iter} n={n}"), &case);
        // Pin the expected C value explicitly, not merely "both agree".
        let l = libs();
        let mut s = src.clone();
        s.push(0);
        let ret = unsafe { (l.c_wcscat)(std::ptr::null_mut(), n, s.as_ptr()) };
        assert_eq!(ret, 22, "E1: C must return 22 (EINVAL)");
        let ret_r = unsafe { (l.rust_wcscat)(std::ptr::null_mut(), n, s.as_ptr()) };
        assert_eq!(ret_r, 22, "E1: Rust must return 22 (EINVAL)");
    }
}

/// E2 — `dst == NULL` AND `numElem == 0`: the `||` short-circuits, `dst` is
/// never dereferenced => 22.
#[test]
fn e2_null_dst_zero_num_elem() {
    let src = vec![1, 2, 3];
    let case = Case::new(vec![], &src).num_elem(0).dst_null();
    assert_same("E2", &case);

    let l = libs();
    let mut s = src.clone();
    s.push(0);
    assert_eq!(
        unsafe { (l.c_wcscat)(std::ptr::null_mut(), 0, s.as_ptr()) },
        22
    );
    assert_eq!(
        unsafe { (l.rust_wcscat)(std::ptr::null_mut(), 0, s.as_ptr()) },
        22
    );
}

/// E3 — `dst == NULL` and `src == NULL`, `numElem > 0`: returns 22 from the
/// `!dst` check, so the `dst[0] = 0` in the `!src` branch is NOT executed
/// (otherwise this would segfault).
#[test]
fn e3_null_dst_and_null_src() {
    for n in [1usize, 2, 1024, usize::MAX] {
        let case = Case::new(vec![], &[]).num_elem(n).dst_null().src_null();
        assert_same(&format!("E3 n={n}"), &case);

        let l = libs();
        assert_eq!(
            unsafe { (l.c_wcscat)(std::ptr::null_mut(), n, std::ptr::null()) },
            22,
            "E3: C must return 22 without dereferencing NULL dst"
        );
        assert_eq!(
            unsafe { (l.rust_wcscat)(std::ptr::null_mut(), n, std::ptr::null()) },
            22,
            "E3: Rust must return 22 without dereferencing NULL dst"
        );
    }
}

/// E4 — valid `dst`, `numElem == 0`, valid `src` => 22 and `dst` left
/// COMPLETELY unmodified (this returns from the line-8 check, so no `dst[0] = 0`).
#[test]
fn e4_zero_num_elem_dst_unmodified() {
    let mut rng = Rng::new(0xE004);
    for iter in 0..300 {
        let capacity = rng.range(1, 32);
        for d in Domain::all() {
            // Every element non-zero, so a stray `dst[0] = 0` would be obvious.
            let dst: Vec<WcharT> = (0..capacity).map(|_| d.gen(&mut rng)).collect();
            let original = dst.clone();
            let __len = rng.range(0, 6);
            let src = d.vec(&mut rng, __len);
            let case = Case::new(dst, &src).num_elem(0);
            assert_same(&format!("E4 iter={iter} cap={capacity} {d:?}"), &case);

            // Explicitly assert the C leaves the buffer untouched and returns 22.
            let l = libs();
            let mut buf = original.clone();
            let mut s = src.clone();
            s.push(0);
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), 0, s.as_ptr()) };
            assert_eq!(ret, 22, "E4: C return");
            assert_eq!(buf, original, "E4: C must not modify dst when numElem == 0");

            let mut buf_r = original.clone();
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), 0, s.as_ptr()) };
            assert_eq!(ret_r, 22, "E4: Rust return");
            assert_eq!(buf_r, original, "E4: Rust must not modify dst when numElem == 0");
        }
    }
}

/// E5 — valid `dst`, `numElem == 0`, `src == NULL` => 22 from line 8; the
/// `!src` branch (and its `dst[0] = 0`) is never reached, so `dst` is unmodified.
#[test]
fn e5_zero_num_elem_null_src() {
    let mut rng = Rng::new(0xE005);
    for iter in 0..200 {
        let capacity = rng.range(1, 32);
        let dst: Vec<WcharT> = (0..capacity).map(|_| rng.nonzero_i32()).collect();
        let original = dst.clone();
        let case = Case::new(dst, &[]).num_elem(0).src_null();
        assert_same(&format!("E5 iter={iter} cap={capacity}"), &case);

        let l = libs();
        let mut buf = original.clone();
        let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), 0, std::ptr::null()) };
        assert_eq!(ret, 22, "E5: C return");
        assert_eq!(buf, original, "E5: C must NOT set dst[0]=0 -- returns before the !src branch");

        let mut buf_r = original.clone();
        let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), 0, std::ptr::null()) };
        assert_eq!(ret_r, 22, "E5: Rust return");
        assert_eq!(buf_r, original, "E5: Rust must NOT set dst[0]=0");
    }
}

/// E6 — valid `dst`, `numElem > 0`, `src == NULL` => sets `dst[0] = 0`,
/// returns 22, leaves `dst[1..]` alone.
#[test]
fn e6_null_src_clears_first_element() {
    let mut rng = Rng::new(0xE006);
    for iter in 0..300 {
        let capacity = rng.range(1, 32);
        let n = rng.range(1, capacity);
        for d in Domain::all() {
            let dst: Vec<WcharT> = (0..capacity).map(|_| d.gen(&mut rng)).collect();
            let original = dst.clone();
            let case = Case::new(dst, &[]).num_elem(n).src_null();
            assert_same(&format!("E6 iter={iter} cap={capacity} n={n} {d:?}"), &case);

            let l = libs();
            let mut buf = original.clone();
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), n, std::ptr::null()) };
            assert_eq!(ret, 22, "E6: C return");
            assert_eq!(buf[0], 0, "E6: C must set dst[0] = 0");
            assert_eq!(
                &buf[1..],
                &original[1..],
                "E6: C must leave dst[1..] untouched"
            );

            let mut buf_r = original.clone();
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), n, std::ptr::null()) };
            assert_eq!(ret_r, 22, "E6: Rust return");
            assert_eq!(buf_r, buf, "E6: Rust buffer must match C byte-for-byte");
        }
    }
}

/// E7 — no NUL anywhere in the `dst` window (unterminated destination): the
/// first loop saturates, the second never runs, `dst[0] = 0`, return 34.
#[test]
fn e7_unterminated_dst() {
    let mut rng = Rng::new(0xE007);
    for iter in 0..400 {
        let n = rng.range(1, 48);
        for d in Domain::all() {
            let dst: Vec<WcharT> = (0..n).map(|_| d.gen(&mut rng)).collect();
            let original = dst.clone();
            let __len = rng.range(0, 8);
            let src = d.vec(&mut rng, __len);
            let case = Case::new(dst, &src);
            assert_same(&format!("E7 iter={iter} n={n} {d:?}"), &case);

            let l = libs();
            let mut buf = original.clone();
            let mut s = src.clone();
            s.push(0);
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), n, s.as_ptr()) };
            assert_eq!(ret, 34, "E7: C must return 34 (ERANGE)");
            assert_eq!(buf[0], 0, "E7: C sets dst[0] = 0");
            assert_eq!(
                &buf[1..],
                &original[1..],
                "E7: C leaves dst[1..] at its original contents (second loop never ran)"
            );

            let mut buf_r = original.clone();
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), n, s.as_ptr()) };
            assert_eq!(ret_r, 34, "E7: Rust must return 34");
            assert_eq!(buf_r, buf, "E7: Rust buffer must match C");
        }
    }
}

/// E8 — degenerate E7: `numElem == 1`, `dst[0] != 0` => 34, `dst[0]` becomes 0.
#[test]
fn e8_num_elem_one_unterminated() {
    let mut rng = Rng::new(0xE008);
    for iter in 0..200 {
        for d in Domain::all() {
            let v = d.gen(&mut rng);
            let __len = rng.range(0, 4);
            let src = d.vec(&mut rng, __len);
            let case = Case::new(vec![v], &src);
            assert_same(&format!("E8 iter={iter} v={v} {d:?}"), &case);

            let l = libs();
            let mut buf = vec![v];
            let mut s = src.clone();
            s.push(0);
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), 1, s.as_ptr()) };
            assert_eq!(ret, 34, "E8: C return");
            assert_eq!(buf[0], 0, "E8: C clears dst[0]");
            let mut buf_r = vec![v];
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), 1, s.as_ptr()) };
            assert_eq!(ret_r, 34);
            assert_eq!(buf_r, buf);
        }
    }
}

/// E9 — NUL at index `k` but `strlen(src) > numElem - k`: partial copy IS
/// performed, then `dst[0] = 0`, return 34. The partial copy is observable.
#[test]
fn e9_src_overflows_partial_copy_observable() {
    let mut rng = Rng::new(0xE009);
    for iter in 0..600 {
        let n = rng.range(2, 48);
        let k = rng.range(0, n - 1);
        let room = n - k;
        // Strictly more than `room` elements => cannot fit even the terminator.
        let len = room + rng.range(1, 8);
        for d in Domain::all() {
            let dst = make_dst(&mut rng, n, k, d, true);
            let original = dst.clone();
            let src = d.vec(&mut rng, len);
            let case = Case::new(dst, &src);
            assert_same(&format!("E9 iter={iter} n={n} k={k} len={len} {d:?}"), &case);

            let l = libs();
            let mut buf = original.clone();
            let mut s = src.clone();
            s.push(0);
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), n, s.as_ptr()) };
            assert_eq!(ret, 34, "E9: C must return 34 (ERANGE)");
            assert_eq!(buf[0], 0, "E9: C sets dst[0] = 0 after the partial copy");
            // The partial copy filled dst[k..n] with src[0..room] BEFORE dst[0]
            // was clobbered. Verify that is what actually happened in C.
            let mut expected = original.clone();
            for i in 0..room {
                expected[k + i] = src[i];
            }
            expected[0] = 0;
            assert_eq!(
                buf, expected,
                "E9: C partial-copy image mismatch (this is the exact byte pattern \
                 the Rust must reproduce)"
            );

            let mut buf_r = original.clone();
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), n, s.as_ptr()) };
            assert_eq!(ret_r, 34, "E9: Rust return");
            assert_eq!(buf_r, buf, "E9: Rust must reproduce the partial copy exactly");
        }
    }
}

/// E10 — degenerate E9: `numElem == 1`, `dst[0] == 0`, non-empty `src`.
/// `src[0]` is written to `dst[0]`, the window exhausts, then `dst[0] = 0` => 34.
#[test]
fn e10_num_elem_one_nonempty_src() {
    let mut rng = Rng::new(0xE010);
    for iter in 0..200 {
        for d in Domain::all() {
            let __len = rng.range(1, 5);
            let src = d.vec(&mut rng, __len);
            let case = Case::new(vec![0], &src);
            assert_same(&format!("E10 iter={iter} {d:?}"), &case);

            let l = libs();
            let mut buf = vec![0];
            let mut s = src.clone();
            s.push(0);
            let ret = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), 1, s.as_ptr()) };
            assert_eq!(ret, 34, "E10: C return");
            assert_eq!(buf[0], 0, "E10: dst[0] ends at 0");
            let mut buf_r = vec![0];
            let ret_r = unsafe { (l.rust_wcscat)(buf_r.as_mut_ptr(), 1, s.as_ptr()) };
            assert_eq!(ret_r, 34);
            assert_eq!(buf_r, buf);
        }
    }
}

/// E11 — the exact boundary: `strlen(src) == numElem - k`, i.e. ONE element too
/// long (the largest accepted length is `numElem - k - 1`). Must return 34,
/// while length-1 must return 0.
#[test]
fn e11_one_past_maximum_length() {
    let mut rng = Rng::new(0xE011);
    for iter in 0..500 {
        let n = rng.range(2, 48);
        let k = rng.range(0, n - 1);
        let room = n - k;
        for d in Domain::all() {
            // Exactly `room` elements: terminator has no room => 34.
            let too_long = d.vec(&mut rng, room);
            let dst = make_dst(&mut rng, n, k, d, true);
            let case = Case::new(dst.clone(), &too_long);
            assert_same(&format!("E11 iter={iter} n={n} k={k} len={room} (too long)"), &case);

            let l = invoke_both(&case);
            assert_eq!(l.0, 34, "E11: len == room must give 34, got {}", l.0);

            // One shorter: exactly fits => 0. Confirms the boundary is where the
            // C source says it is.
            let just_fits: Vec<WcharT> = too_long[..room - 1].to_vec();
            let case_ok = Case::new(dst, &just_fits);
            assert_same(
                &format!("E11 iter={iter} n={n} k={k} len={} (fits)", room - 1),
                &case_ok,
            );
            let l_ok = invoke_both(&case_ok);
            assert_eq!(l_ok.0, 0, "E11: len == room-1 must give 0, got {}", l_ok.0);
        }
    }
}

/// Helper: run a case through both libraries and return `(c_ret, rust_ret)`
/// after asserting they agree.
fn invoke_both(case: &Case) -> (i32, i32) {
    let l = libs();
    let mut dc = case.dst.clone();
    let mut dr = case.dst.clone();
    let src = case.src.clone();
    let sp: *const WcharT = if case.src_null {
        std::ptr::null()
    } else {
        src.as_ptr()
    };
    let pc: *mut WcharT = if case.dst_null {
        std::ptr::null_mut()
    } else {
        dc.as_mut_ptr()
    };
    let pr: *mut WcharT = if case.dst_null {
        std::ptr::null_mut()
    } else {
        dr.as_mut_ptr()
    };
    let rc = unsafe { (l.c_wcscat)(pc, case.num_elem, sp) };
    let rr = unsafe { (l.rust_wcscat)(pr, case.num_elem, sp) };
    assert_eq!(rc, rr, "return codes diverge");
    assert_eq!(dc, dr, "buffers diverge");
    (rc, rr)
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases (G1..G6)
// ---------------------------------------------------------------------------

/// G1 — both pointers NULL and `numElem == 0`.
#[test]
fn g1_all_null_zero_len() {
    let l = libs();
    let rc = unsafe { (l.c_wcscat)(std::ptr::null_mut(), 0, std::ptr::null()) };
    let rr = unsafe { (l.rust_wcscat)(std::ptr::null_mut(), 0, std::ptr::null()) };
    assert_eq!(rc, 22, "G1: C");
    assert_eq!(rr, 22, "G1: Rust");
}

/// G2 — `numElem == 1` crossed with `dst[0] in {0, nonzero}` and empty /
/// non-empty `src`. Only `dst[0]==0 && src` empty yields 0.
#[test]
fn g2_num_elem_one_exhaustive() {
    let l = libs();
    for dst0 in [0i32, 1, -1, i32::MIN, i32::MAX, 0x10_FFFF] {
        for src_body in [vec![], vec![7], vec![7, 8], vec![-1], vec![i32::MIN, 3]] {
            let case = Case::new(vec![dst0], &src_body);
            assert_same(&format!("G2 dst0={dst0} src={src_body:?}"), &case);

            let mut buf = vec![dst0];
            let mut s = src_body.clone();
            s.push(0);
            let rc = unsafe { (l.c_wcscat)(buf.as_mut_ptr(), 1, s.as_ptr()) };
            let expect = if dst0 == 0 && src_body.is_empty() { 0 } else { 34 };
            assert_eq!(
                rc, expect,
                "G2: C for dst0={dst0} src={src_body:?} expected {expect}"
            );
        }
    }
}

/// G3 — oversized `numElem` with an early NUL in a small real buffer; the copied
/// terminator stops the loop well inside the allocation.
#[test]
fn g3_oversized_num_elem() {
    let mut rng = Rng::new(0xE0A3);
    for n in [1usize << 20, 1usize << 40, 1usize << 48] {
        for iter in 0..50 {
            let capacity = 64usize;
            let dst = make_dst(&mut rng, capacity, 0, Domain::AnyNonZero, true);
            let __len = rng.range(0, 8);
            let src = Domain::AnyNonZero.vec(&mut rng, __len);
            let case = Case::new(dst, &src).num_elem(n);
            assert_same(&format!("G3 n={n} iter={iter}"), &case);
            let (rc, _) = invoke_both(&case);
            assert_eq!(rc, 0, "G3: an early NUL means the copy succeeds");
        }
    }
}

/// G4 — `numElem == usize::MAX`, where `dst + numElem` wraps the pointer. The
/// compiled C's observed behaviour is the ground truth; the Rust must agree.
#[test]
fn g4_num_elem_usize_max_pointer_wrap() {
    let mut rng = Rng::new(0xE0A4);
    for n in [usize::MAX, usize::MAX - 1, usize::MAX / 4, usize::MAX / 2] {
        for iter in 0..20 {
            let capacity = 32usize;
            let dst = make_dst(&mut rng, capacity, 0, Domain::AnyNonZero, true);
            let __len = rng.range(0, 4);
            let src = Domain::AnyNonZero.vec(&mut rng, __len);
            let case = Case::new(dst, &src).num_elem(n);
            assert_same(&format!("G4 n={n} iter={iter}"), &case);
        }
    }
}

/// G5 — the API has no enum parameter, so "out-of-range enum value" degenerates
/// to out-of-range values of the only scalar parameter, `size_t numElem`. Sweep
/// the extremes of its domain plus every power of two.
#[test]
fn g5_num_elem_full_scalar_domain() {
    let mut rng = Rng::new(0xE0A5);
    let mut ns: Vec<usize> = vec![0, 1, 2, 3, 31, 32, 33, 63, 64, 65];
    for shift in 0..64 {
        ns.push(1usize << shift);
    }
    ns.push(usize::MAX);
    ns.push(usize::MAX - 1);

    for &n in &ns {
        let capacity = 32usize;
        // NUL at index 0 keeps every oversized `n` memory-safe: the copy always
        // terminates within the real allocation.
        let dst = make_dst(&mut rng, capacity, 0, Domain::AnyNonZero, true);
        let __len = rng.range(0, 4);
        let src = Domain::AnyNonZero.vec(&mut rng, __len);
        let case = Case::new(dst, &src).num_elem(n);
        assert_same(&format!("G5 n={n}"), &case);
    }
}

/// G6 — signedness: `wchar_t` is signed here, so only exact 0 terminates and
/// negative values must be treated as ordinary non-NUL elements.
#[test]
fn g6_signedness_only_zero_terminates() {
    let mut rng = Rng::new(0xE0A6);
    let sentinels = [-1i32, i32::MIN, i32::MAX, 0x8000_0000u32 as i32, -0x8000, 0x10_FFFF];
    for iter in 0..400 {
        let n = rng.range(2, 40);
        let k = rng.range(1, n - 1);
        // dst prefix made entirely of high-bit-set values.
        let mut dst: Vec<WcharT> = (0..n)
            .map(|i| {
                if i == k {
                    0
                } else {
                    sentinels[rng.range(0, sentinels.len() - 1)]
                }
            })
            .collect();
        dst[k] = 0;
        let len = rng.range(0, n + 2);
        let src: Vec<WcharT> = (0..len)
            .map(|_| sentinels[rng.range(0, sentinels.len() - 1)])
            .collect();
        assert_same(&format!("G6 iter={iter} n={n} k={k} len={len}"), &Case::new(dst, &src));
    }
}
