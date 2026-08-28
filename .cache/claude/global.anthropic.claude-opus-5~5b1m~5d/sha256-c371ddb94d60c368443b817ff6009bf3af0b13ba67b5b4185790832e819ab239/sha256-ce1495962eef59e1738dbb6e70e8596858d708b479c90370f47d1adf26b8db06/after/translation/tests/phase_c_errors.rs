//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test constructs the exact invalid
//! input/condition, calls BOTH `.so`s, and asserts they return the SAME error
//! code (22 / 34) *and* leave the destination buffer in the same state.
//!
//! `check()` already asserts C-vs-Rust equality of (ret, dst, src); the extra
//! `assert_eq!`s below pin down the ABSOLUTE expected C result so the test cannot
//! pass by both sides being wrong in the same way.

mod common;

use common::*;

/// Convenience: run the case and additionally assert the shared return code.
fn expect(case: &Case, ret: i32) -> Outcome {
    let out = check(case);
    assert_eq!(
        out.ret, ret,
        "case `{}`: expected both implementations to return {}, both returned {}",
        case.name, ret, out.ret
    );
    out
}

// ===========================================================================
// Row 1 — L7 `!dst`
// ===========================================================================
#[test]
fn err01_dst_null() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..200 {
        let num_elem = rng.range(1, 4096);
        let src = rng.rand_src_below(32);
        expect(
            &Case::null_dst("err01_dst_null", num_elem, Src::Buf(src)),
            22,
        );
    }
    // plus fixed witnesses
    for &n in &[1usize, 2, 3, 8, 1024, usize::MAX / 4, usize::MAX] {
        expect(
            &Case::null_dst("err01_dst_null_fixed", n, Src::Buf(vec![1, 2, 0])),
            22,
        );
    }
}

// ===========================================================================
// Row 2 — L7 `numElem == 0`
// ===========================================================================
#[test]
fn err02_numelem_zero() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..500 {
        let alloc = rng.range(1, 64);
        // Window is empty, so treat the whole allocation as guard.
        let dst = make_dst(&mut rng, alloc, 0, None);
        let src = rng.rand_src_below(32);
        let case = Case::new("err02_numelem_zero", dst.clone(), 0, Src::Buf(src));
        let out = expect(&case, 22);
        assert_eq!(
            out.dst.as_deref(),
            Some(&dst[..]),
            "numElem == 0 must leave dst COMPLETELY untouched (dst[0] is NOT zeroed on this path)"
        );
    }
}

// ===========================================================================
// Row 3 — L7 both disjuncts
// ===========================================================================
#[test]
fn err03_dst_null_and_numelem_zero() {
    expect(
        &Case::null_dst("err03", 0, Src::Buf(vec![1, 2, 3, 0])),
        22,
    );
    expect(&Case::null_dst("err03_src_null", 0, Src::Null), 22);
}

// ===========================================================================
// Row 4 — L7 short-circuit: dst NULL wins over src NULL (no NULL deref at L10)
// ===========================================================================
#[test]
fn err04_dst_null_and_src_null() {
    for &n in &[0usize, 1, 2, 7, 4096, usize::MAX] {
        expect(&Case::null_dst("err04", n, Src::Null), 22);
    }
}

// ===========================================================================
// Row 5 — L7 short-circuit: numElem == 0 wins over src NULL (dst[0] NOT zeroed)
// ===========================================================================
#[test]
fn err05_numelem_zero_and_src_null() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..300 {
        let alloc = rng.range(1, 32);
        let dst = make_dst(&mut rng, alloc, 0, None);
        let case = Case::new("err05", dst.clone(), 0, Src::Null);
        let out = expect(&case, 22);
        assert_eq!(
            out.dst.as_deref(),
            Some(&dst[..]),
            "numElem==0 short-circuits before the `!src` branch, so dst must be untouched"
        );
    }
}

// ===========================================================================
// Row 6 — L9 `!src`: returns 22 AND zeroes dst[0]
// ===========================================================================
#[test]
fn err06_src_null_writes_dst0() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..500 {
        let alloc = rng.range(1, 64);
        let num_elem = rng.range(1, alloc);
        let k = if rng.bool() {
            Some(rng.below(num_elem))
        } else {
            None
        };
        let dst = make_dst(&mut rng, alloc, num_elem, k);
        let case = Case::new("err06", dst.clone(), num_elem, Src::Null);
        let out = expect(&case, 22);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0, "src == NULL must set dst[0] = 0");
        assert_eq!(
            &after[1..],
            &dst[1..],
            "src == NULL must touch ONLY dst[0]"
        );
    }
}

// ===========================================================================
// Row 7 — L9 `!src` with numElem == 1
// ===========================================================================
#[test]
fn err07_src_null_numelem_one() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let alloc = rng.range(1, 16);
        let dst = make_dst(&mut rng, alloc, 1, None);
        let case = Case::new("err07", dst.clone(), 1, Src::Null);
        let out = expect(&case, 22);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
    // dst[0] already 0
    let out = expect(
        &Case::new("err07_already_zero", vec![0, GUARD, GUARD], 1, Src::Null),
        22,
    );
    assert_eq!(out.dst.unwrap(), vec![0, GUARD, GUARD]);
}

// ===========================================================================
// Row 8 — L9 `!src` on an already-full/unterminated dst (no scan happens)
// ===========================================================================
#[test]
fn err08_src_null_unterminated_dst() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..300 {
        let alloc = rng.range(1, 48);
        let num_elem = rng.range(1, alloc);
        let dst = make_dst(&mut rng, alloc, num_elem, None); // no terminator
        let case = Case::new("err08", dst.clone(), num_elem, Src::Null);
        let out = expect(&case, 22);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 9 — L13/L20 unterminated dst: 34, dst[0]=0, src never read
// ===========================================================================
#[test]
fn err09_unterminated_dst() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..800 {
        let alloc = rng.range(1, 64);
        let num_elem = rng.range(1, alloc);
        let dst = make_dst(&mut rng, alloc, num_elem, None);
        let src = rng.rand_src_below(24);
        let case = Case::new("err09", dst.clone(), num_elem, Src::Buf(src.clone()));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0, "truncation must set dst[0] = 0");
        assert_eq!(
            &after[1..],
            &dst[1..],
            "unterminated dst: the copy loop never runs, so only dst[0] changes"
        );
        assert_eq!(out.src.as_deref(), Some(&src[..]), "src must not be written");
    }
}

// ===========================================================================
// Row 10 — unterminated dst + empty src
// ===========================================================================
#[test]
fn err10_unterminated_dst_empty_src() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..300 {
        let alloc = rng.range(1, 32);
        let num_elem = rng.range(1, alloc);
        let dst = make_dst(&mut rng, alloc, num_elem, None);
        let case = Case::new("err10", dst.clone(), num_elem, Src::Buf(vec![0]));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 11 — L15/L20 truncation with a partial copy, then dst[0]=0 clobber
// ===========================================================================
#[test]
fn err11_truncate_partial_copy() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..1000 {
        let alloc = rng.range(2, 64);
        let num_elem = rng.range(2, alloc);
        let k = rng.below(num_elem); // terminator index inside the window
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let room = num_elem - k; // slots available for payload+terminator
        // Guarantee truncation: strlen(src) >= room
        let l = room + rng.below(8);
        let src = make_src(&mut rng, l);

        let case = Case::new("err11", dst.clone(), num_elem, Src::Buf(src.clone()));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();

        // Reconstruct the exact C effect.
        let mut expected = dst.clone();
        for i in 0..room {
            expected[k + i] = src[i];
        }
        expected[0] = 0;
        assert_eq!(
            after, expected,
            "truncation must fill dst[k..numElem) from src then force dst[0]=0"
        );
        // guard tail beyond the window untouched
        assert_eq!(&after[num_elem..], &dst[num_elem..]);
        assert_eq!(out.src.as_deref(), Some(&src[..]));
    }
}

// ===========================================================================
// Row 12 — exact off-by-one: k + L == numElem (payload fits, terminator doesn't)
// ===========================================================================
#[test]
fn err12_truncate_off_by_one() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..800 {
        let alloc = rng.range(2, 64);
        let num_elem = rng.range(2, alloc);
        let k = rng.below(num_elem);
        let l = num_elem - k; // exactly one too long for the terminator
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = make_src(&mut rng, l);

        let case = Case::new("err12", dst.clone(), num_elem, Src::Buf(src.clone()));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();

        let mut expected = dst.clone();
        for i in 0..l {
            expected[k + i] = src[i];
        }
        expected[0] = 0;
        assert_eq!(after, expected);
    }
}

// ===========================================================================
// Row 13 — numElem == 1, dst empty, src non-empty
// ===========================================================================
#[test]
fn err13_truncate_numelem_one() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..300 {
        let alloc = rng.range(1, 16);
        let mut dst = make_dst(&mut rng, alloc, 1, Some(0));
        dst[0] = 0;
        let src = rng.rand_src_range(1, 12);
        let case = Case::new("err13", dst.clone(), 1, Src::Buf(src));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(
            after[0], 0,
            "src[0] is written into dst[0] and then overwritten by the L19 dst[0]=0"
        );
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 14 — exactly one free slot (k == numElem-1) and src non-empty
// ===========================================================================
#[test]
fn err14_truncate_one_slot_left() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..500 {
        let alloc = rng.range(2, 48);
        let num_elem = rng.range(2, alloc);
        let k = num_elem - 1;
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = rng.rand_src_range(1, 12);

        let case = Case::new("err14", dst.clone(), num_elem, Src::Buf(src.clone()));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();

        let mut expected = dst.clone();
        expected[k] = src[0];
        expected[0] = 0;
        assert_eq!(after, expected);
        assert_eq!(after[num_elem - 1], src[0]);
    }
}

// ===========================================================================
// Row 15 — numElem == SIZE_MAX: `dst + numElem` wraps, both loops skipped
// ===========================================================================
#[test]
fn err15_numelem_size_max() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..200 {
        let alloc = rng.range(1, 32);
        // Terminate early so the C stays in bounds regardless of which branch it takes.
        let mut dst = make_dst(&mut rng, alloc, alloc, Some(0));
        dst[0] = 0;
        let src = rng.rand_src_below(8);
        let case = Case::new("err15", dst.clone(), usize::MAX, Src::Buf(src));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..], "no loop runs, so only dst[0] is set");
    }
}

// ===========================================================================
// Row 16 — other pointer-wrap witnesses
// ===========================================================================
#[test]
fn err16_numelem_wrap_witnesses() {
    let mut rng = Rng::new(SEED ^ 16);
    // Any numElem where numElem * 4 (mod 2^64) lands at or before dst.
    let witnesses: [usize; 8] = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 4,          // *4 == -4
        usize::MAX / 2,          // *4 == -4 (odd/2 -> ...)
        1usize << 62,            // *4 == 0
        (1usize << 62) + 1,      // *4 == 4  -> NOT a wrap-to-before; see note
        1usize << 63,            // *4 == 0
        (usize::MAX >> 2) + 1,   // *4 == 0
    ];
    for &n in &witnesses {
        let mut dst = make_dst(&mut rng, 8, 8, Some(0));
        dst[0] = 0;
        let src = vec![0x41i32, 0x42, 0];
        // Whatever the C does for this wrap, the Rust must do the same; the
        // shared return code is asserted by `check`, and only 0/22/34 are legal.
        let case = Case::new(format!("err16_n_{n:#x}"), dst, n, Src::Buf(src));
        let out = check(&case);
        assert!(
            out.ret == 0 || out.ret == 34,
            "wrap witness {n:#x}: unexpected ret {}",
            out.ret
        );
    }
    // The four witnesses whose scaled offset is <= 0 must all report truncation.
    for &n in &[usize::MAX, usize::MAX / 4, 1usize << 62, 1usize << 63] {
        let mut dst = make_dst(&mut rng, 8, 8, Some(0));
        dst[0] = 0;
        let case = Case::new(
            format!("err16_trunc_{n:#x}"),
            dst.clone(),
            n,
            Src::Buf(vec![0x41, 0]),
        );
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 17 — the terminator exists but lies OUTSIDE the window
// ===========================================================================
#[test]
fn err17_nul_outside_window() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..500 {
        let num_elem = rng.range(1, 32);
        let alloc = num_elem + rng.range(1, 16);
        let mut dst = make_dst(&mut rng, alloc, num_elem, None);
        let nul_at = rng.range(num_elem, alloc - 1);
        dst[nul_at] = 0; // a perfectly good C string, just not inside the window
        let src = rng.rand_src_below(16);

        let case = Case::new("err17", dst.clone(), num_elem, Src::Buf(src));
        let out = expect(&case, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
        assert_eq!(after[nul_at], 0);
    }
}

// ===========================================================================
// Generic boundaries G3 / G4 / G6
// ===========================================================================

#[test]
fn boundary_numelem_one_matrix() {
    // numElem == 1 is the smallest non-rejected window; enumerate every dst/src
    // shape at that boundary.
    let dst_shapes: [Vec<i32>; 3] = [
        vec![0, GUARD, GUARD],           // empty
        vec![0x41, GUARD, GUARD],        // unterminated window
        vec![i32::MIN, GUARD, GUARD],    // unterminated with an extreme value
    ];
    let src_shapes: [Src; 5] = [
        Src::Null,
        Src::Buf(vec![0]),
        Src::Buf(vec![0x41, 0]),
        Src::Buf(vec![i32::MIN, 0]),
        Src::Buf(vec![-1, -2, -3, 0]),
    ];
    for (di, d) in dst_shapes.iter().enumerate() {
        for (si, s) in src_shapes.iter().enumerate() {
            let case = Case::new(format!("numelem_one_{di}_{si}"), d.clone(), 1, s.clone());
            let out = check(&case);
            let after = out.dst.unwrap();
            // Whatever the code, dst[0] ends up 0 in every one of these shapes.
            assert_eq!(after[0], 0, "case d{di} s{si}");
            assert_eq!(&after[1..], &d[1..], "guard tail must be intact");
        }
    }
}

#[test]
fn boundary_fit_off_by_one_sweep() {
    // Walk the fit boundary one step at a time: k + L + 1 < n (ok),
    // k + L + 1 == n (exact fit, ok), k + L == n (34), k + L == n + 1 (34).
    let mut rng = Rng::new(SEED ^ 0x0FF);
    for n in 1usize..=24 {
        for k in 0..n {
            let room = n - k;
            for delta in 0..=2usize {
                // l = room - 1 - delta would go negative; clamp
                let l_fit = room - 1; // exact fit
                let candidates: Vec<(usize, i32)> = vec![
                    (l_fit.saturating_sub(delta), 0),
                    (l_fit + 1, 34),
                    (l_fit + 2, 34),
                ];
                for (l, want) in candidates {
                    if l_fit == 0 && want == 0 && l != 0 {
                        continue;
                    }
                    let alloc = n + 3;
                    let dst = make_dst(&mut rng, alloc, n, Some(k));
                    let src = make_src(&mut rng, l);
                    let case = Case::new(
                        format!("fit_sweep_n{n}_k{k}_l{l}"),
                        dst.clone(),
                        n,
                        Src::Buf(src.clone()),
                    );
                    let out = expect(&case, want);
                    let after = out.dst.unwrap();
                    if want == 0 {
                        // prefix preserved, payload copied, NUL written
                        assert_eq!(&after[..k], &dst[..k]);
                        for i in 0..l {
                            assert_eq!(after[k + i], src[i]);
                        }
                        assert_eq!(after[k + l], 0);
                        assert_eq!(&after[k + l + 1..], &dst[k + l + 1..]);
                    } else {
                        assert_eq!(after[0], 0);
                        assert_eq!(&after[n..], &dst[n..], "guard tail intact");
                    }
                }
            }
        }
    }
}

#[test]
fn extreme_wchar_values() {
    const EXTREMES: [i32; 10] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        -2,
        i32::MAX,
        i32::MAX - 1,
        0x41424344,
        0xD800u32 as i32,
        0x0011_0000,
        0x7FFF_FFFE,
    ];
    for &a in &EXTREMES {
        for &b in &EXTREMES {
            // src carries extremes; dst prefix carries extremes.
            let dst = vec![a, b, 0, GUARD, GUARD, GUARD, GUARD, GUARD];
            let src = vec![b, a, a, 0, b];
            let out = check(&Case::new(
                format!("extreme_{a:#x}_{b:#x}"),
                dst,
                8,
                Src::Buf(src),
            ));
            assert_eq!(out.ret, 0, "no extreme value may act as a terminator");
            assert_eq!(
                &out.dst.unwrap()[..6],
                &[a, b, b, a, a, 0],
                "extreme values must be copied verbatim"
            );
        }
    }
}

#[test]
fn oversized_numelem_no_overflow() {
    // G5: numElem far larger than the real allocation but not overflowing.
    let mut rng = Rng::new(SEED ^ 0xA5A5);
    for &n in &[
        1usize << 20,
        1usize << 32,
        1usize << 40,
        1usize << 48,
        1usize << 56,
    ] {
        let alloc = 16;
        let mut dst = make_dst(&mut rng, alloc, alloc, Some(2));
        dst[2] = 0;
        let src = vec![0x41i32, 0x42, 0x43, 0];
        let case = Case::new(format!("oversized_{n:#x}"), dst.clone(), n, Src::Buf(src));
        let out = expect(&case, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..6], &[dst[0], dst[1], 0x41, 0x42, 0x43, 0]);
        assert_eq!(&after[6..], &dst[6..]);
    }
}

#[test]
fn return_code_domain_is_closed() {
    // Sweep a broad grid and prove the shared return-code domain is exactly {0,22,34}.
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..5000 {
        let alloc = rng.range(1, 40);
        let num_elem = rng.range(0, alloc);
        let k = if num_elem > 0 && rng.bool() {
            Some(rng.below(num_elem))
        } else {
            None
        };
        let dst = make_dst(&mut rng, alloc, num_elem.max(1).min(alloc), k);
        let src = if rng.below(8) == 0 {
            Src::Null
        } else {
            Src::Buf(rng.rand_src_below(48))
        };
        let out = check(&Case::new("domain", dst, num_elem, src));
        seen.insert(out.ret);
        // NULL dst variant
        let out2 = check(&Case::null_dst("domain_null_dst", num_elem, Src::Buf(vec![1, 0])));
        seen.insert(out2.ret);
    }
    println!("observed return codes: {seen:?}");
    assert_eq!(
        seen,
        [0, 22, 34].into_iter().collect::<std::collections::BTreeSet<i32>>(),
        "the sweep must observe every legal return code and nothing else"
    );
}
