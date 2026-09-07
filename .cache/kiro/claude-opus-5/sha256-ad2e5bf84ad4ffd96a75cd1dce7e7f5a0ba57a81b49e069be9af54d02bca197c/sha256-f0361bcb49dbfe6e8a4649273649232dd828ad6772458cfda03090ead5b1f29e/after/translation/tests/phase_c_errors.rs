//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! `tfm` returns `void` and has no error codes, so "same rejection" is asserted
//! on the only observable channels: how many `dest` slots are written (byte-wise
//! including untouched canary/garbage slots) and the exact sentinel bit patterns
//! produced. Both `.so`s are driven through `dlsym`.

mod common;

use common::*;

/// Call both `.so`s with raw pointers the caller controls, and compare the full
/// destination buffer bit-for-bit. Used for the NULL-pointer rows where we must
/// not materialise a slice.
unsafe fn diff_raw(
    c_dest: *mut f32,
    r_dest: *mut f32,
    c_src: *const f32,
    r_src: *const f32,
    count: i32,
) {
    (c_tfm())(c_dest, c_src, count);
    (rust_tfm())(r_dest, r_src, count);
}

// ---------------------------------------------------------------------------
// Row 1 — count == 0: no-op, zero bytes written
// ---------------------------------------------------------------------------
#[test]
fn err_row1_count_zero() {
    let mut rng = Rng::new(SEED ^ 101);
    for it in 0..2000 {
        let src: Vec<f32> = (0..12).map(|_| rng.any_bits_f32()).collect();
        let sentinel: Vec<f32> = (0..12).map(|_| rng.any_bits_f32()).collect();

        let mut c_dest = sentinel.clone();
        let mut r_dest = sentinel.clone();
        unsafe {
            diff_raw(c_dest.as_mut_ptr(), r_dest.as_mut_ptr(), src.as_ptr(), src.as_ptr(), 0);
        }
        let cb: Vec<u32> = c_dest.iter().map(|x| x.to_bits()).collect();
        let rb: Vec<u32> = r_dest.iter().map(|x| x.to_bits()).collect();
        let sb: Vec<u32> = sentinel.iter().map(|x| x.to_bits()).collect();
        assert_eq!(cb, rb, "row1 [iter {it}]: C and Rust differ for count=0");
        assert_eq!(cb, sb, "row1 [iter {it}]: C wrote to dest despite count=0");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — count < 0: no-op
// ---------------------------------------------------------------------------
#[test]
fn err_row2_count_negative() {
    let mut rng = Rng::new(SEED ^ 102);
    let mut counts: Vec<i32> = vec![-1, -2, -3, -7, -100, -1024, -65536, i32::MIN + 1];
    for _ in 0..64 {
        counts.push(-(rng.below(1 << 20) as i32) - 1);
    }
    for &count in &counts {
        let src: Vec<f32> = (0..12).map(|_| rng.any_bits_f32()).collect();
        let sentinel: Vec<f32> = (0..12).map(|_| rng.any_bits_f32()).collect();
        let mut c_dest = sentinel.clone();
        let mut r_dest = sentinel.clone();
        unsafe {
            diff_raw(c_dest.as_mut_ptr(), r_dest.as_mut_ptr(), src.as_ptr(), src.as_ptr(), count);
        }
        let cb: Vec<u32> = c_dest.iter().map(|x| x.to_bits()).collect();
        let rb: Vec<u32> = r_dest.iter().map(|x| x.to_bits()).collect();
        let sb: Vec<u32> = sentinel.iter().map(|x| x.to_bits()).collect();
        assert_eq!(cb, rb, "row2: C and Rust differ for count={count}");
        assert_eq!(cb, sb, "row2: C wrote to dest despite count={count}");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — count == INT_MIN (one step past the negative range)
// ---------------------------------------------------------------------------
#[test]
fn err_row3_count_int_min() {
    let src: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let sentinel: Vec<f32> = vec![f32::from_bits(0xA5A5_A5A5); 8];
    for &count in &[i32::MIN, i32::MIN + 1] {
        let mut c_dest = sentinel.clone();
        let mut r_dest = sentinel.clone();
        unsafe {
            diff_raw(c_dest.as_mut_ptr(), r_dest.as_mut_ptr(), src.as_ptr(), src.as_ptr(), count);
        }
        let cb: Vec<u32> = c_dest.iter().map(|x| x.to_bits()).collect();
        let rb: Vec<u32> = r_dest.iter().map(|x| x.to_bits()).collect();
        let sb: Vec<u32> = sentinel.iter().map(|x| x.to_bits()).collect();
        assert_eq!(cb, rb, "row3: mismatch for count={count}");
        assert_eq!(cb, sb, "row3: C wrote to dest for count={count}");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — exact minimum extent: reads exactly 3, writes exactly 2
// ---------------------------------------------------------------------------
#[test]
fn err_row4_exact_minimum_extent() {
    let mut rng = Rng::new(SEED ^ 104);
    for it in 0..5000 {
        // src buffer is exactly 3 floats followed by a poison page-adjacent
        // guard we assert is untouched; dest is exactly 2 followed by canaries
        // (diff_disjoint already appends and checks 8 canary slots).
        let src = [rng.finite_f32(), rng.finite_f32(), rng.finite_f32()];
        diff_disjoint(&src, 1, 2, &format!("row4 exact extent [iter {it}]"));
    }
    // Explicitly verify the read extent: place a trap value at src[3] and check
    // that the outputs are unaffected by changing it.
    for it in 0..2000 {
        let core = [rng.finite_f32(), rng.finite_f32(), rng.finite_f32()];
        let mut out = [Vec::new(), Vec::new()];
        for (k, trap) in [0.0f32, f32::NAN].iter().enumerate() {
            let src = [core[0], core[1], core[2], *trap, *trap, *trap];
            let mut c_dest = [f32::from_bits(0xA5A5_A5A5); 2];
            let mut r_dest = c_dest;
            unsafe {
                diff_raw(c_dest.as_mut_ptr(), r_dest.as_mut_ptr(), src.as_ptr(), src.as_ptr(), 1);
            }
            assert_eq!(
                c_dest.map(f32::to_bits),
                r_dest.map(f32::to_bits),
                "row4 [iter {it}]: C/Rust differ"
            );
            out[k] = c_dest.map(f32::to_bits).to_vec();
        }
        assert_eq!(out[0], out[1], "row4 [iter {it}]: output depends on src[3..], read extent wrong");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — NULL pointers with count <= 0 (never dereferenced)
// ---------------------------------------------------------------------------
#[test]
fn err_row5_null_pointers_count_zero() {
    for &count in &[0i32, -1, -12345, i32::MIN] {
        unsafe {
            // Both NULL.
            (c_tfm())(std::ptr::null_mut(), std::ptr::null(), count);
            (rust_tfm())(std::ptr::null_mut(), std::ptr::null(), count);
            // Only src NULL.
            let mut d = [0.0f32; 4];
            (c_tfm())(d.as_mut_ptr(), std::ptr::null(), count);
            (rust_tfm())(d.as_mut_ptr(), std::ptr::null(), count);
            // Only dest NULL.
            let s = [1.0f32, 2.0, 3.0];
            (c_tfm())(std::ptr::null_mut(), s.as_ptr(), count);
            (rust_tfm())(std::ptr::null_mut(), s.as_ptr(), count);
            assert_eq!(d.map(f32::to_bits), [0u32; 4], "row5: dest written for count={count}");
        }
    }
    // Also: misaligned-but-nonnull is not exercised, since the C dereferences
    // `float*` and unaligned float access is UB in both languages.
}

// ---------------------------------------------------------------------------
// Row 6 — NaN in the arm predicate: unordered => else arm
// ---------------------------------------------------------------------------
#[test]
fn err_row6_nan_compare_takes_else_arm() {
    let mut rng = Rng::new(SEED ^ 106);
    for it in 0..30_000 {
        let nan = rng.nan_f32();
        let other = if it % 5 == 0 { rng.special_f32() } else { rng.finite_f32() };
        let dxy = if it % 3 == 0 { rng.special_f32() } else { rng.finite_f32() };

        // NaN in lane 0, lane 1, and both.
        for t in [[nan, other, dxy], [other, nan, dxy], [nan, rng.nan_f32(), dxy]] {
            assert!(!(t[0] < t[1]), "row6: predicate should be false (unordered)");
            diff_disjoint(&t, 1, 2, &format!("row6 unordered predicate [iter {it}]"));
        }
    }
    // Sanity: the else arm stores dxy into dest[0]. When dxy is not NaN the C
    // must therefore emit dxy verbatim in slot 0 — proves the else arm ran.
    let t = [f32::NAN, 1.0f32, 7.5f32];
    let mut c_dest = [0.0f32; 2];
    unsafe { (c_tfm())(c_dest.as_mut_ptr(), t.as_ptr(), 1) };
    assert_eq!(c_dest[0].to_bits(), 7.5f32.to_bits(), "row6: else arm was not taken");
}

// ---------------------------------------------------------------------------
// Row 7 — src[0] == src[1]: `<` false => else arm
// ---------------------------------------------------------------------------
#[test]
fn err_row7_equal_operands_take_else_arm() {
    let mut rng = Rng::new(SEED ^ 107);
    for it in 0..20_000 {
        let v = match it % 4 {
            0 => rng.finite_f32(),
            1 => rng.special_f32(),
            2 => f32::from_bits(rng.below(0x0080_0000)),
            _ => rng.small_f32(),
        };
        let dxy = if it % 2 == 0 { rng.finite_f32() } else { rng.special_f32() };
        diff_disjoint(&[v, v, dxy], 1, 2, &format!("row7 equal [iter {it}]"));
    }
    // Signed zeros compare equal -> else arm for all four sign combinations.
    for (a, b) in [(0.0f32, 0.0f32), (0.0, -0.0), (-0.0, 0.0), (-0.0, -0.0)] {
        let t = [a, b, 9.25f32];
        let mut c_dest = [0.0f32; 2];
        unsafe { (c_tfm())(c_dest.as_mut_ptr(), t.as_ptr(), 1) };
        assert_eq!(
            c_dest[0].to_bits(),
            9.25f32.to_bits(),
            "row7: else arm not taken for ({a}, {b})"
        );
        diff_disjoint(&t, 1, 2, "row7 signed zeros");
    }
}

// ---------------------------------------------------------------------------
// Row 8 — sqd < 0 is clamped to 0.0f before sqrtf (no EDOM)
// ---------------------------------------------------------------------------
#[test]
fn err_row8_negative_sqd_clamped_to_zero() {
    let mut rng = Rng::new(SEED ^ 108);
    let mut hits = 0usize;
    for it in 0..40_000 {
        let base = match it % 3 {
            0 => rng.finite_f32(),
            1 => f32::from_bits(0x7F00_0000 | (rng.next_u32() & 0x007F_FFFF)),
            _ => rng.small_f32(),
        };
        let ulps = rng.below(9) as i32 - 4;
        let other = f32::from_bits((base.to_bits() as i32).wrapping_add(ulps) as u32);
        // Tiny dxy so 4*dxy^2 cannot rescue the cancelled square.
        let dxy = f32::from_bits(rng.below(0x0040_0000));
        let (a, b) = if it % 2 == 0 { (base, other) } else { (other, base) };
        let (dx2, dy2) = if a < b { (a, b) } else { (b, a) };
        let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * dxy * dxy);
        if sqd < 0.0 {
            hits += 1;
            // When the clamp fires, sqrtf sees +0.0, so lambda == 0.5*(dy2+dx2)
            // exactly. Verify the C really does that (proves the clamp, not just
            // that both agree).
            let lambda = 0.5f32 * (dy2 + dx2 + 0.0f32);
            let expect = if a < b { [dx2 - lambda, dxy] } else { [dxy, dx2 - lambda] };
            let mut c_dest = [0.0f32; 2];
            unsafe { (c_tfm())(c_dest.as_mut_ptr(), [a, b, dxy].as_ptr(), 1) };
            assert_eq!(
                c_dest.map(f32::to_bits),
                expect.map(f32::to_bits),
                "row8: C did not clamp negative sqd to 0 (sqd={sqd:e}, a={a:e}, b={b:e}, dxy={dxy:e})"
            );
        }
        diff_disjoint(&[a, b, dxy], 1, 2, &format!("row8 clamp [iter {it}]"));
    }
    assert!(hits > 0, "row8: never produced sqd < 0, the clamp branch was not reached");
    eprintln!("row8: negative-sqd clamp reached {hits} times");
}

// ---------------------------------------------------------------------------
// Row 9 — sqd is NaN: unordered compare, NOT clamped, reaches sqrtf
// ---------------------------------------------------------------------------
#[test]
fn err_row9_nan_sqd_not_clamped() {
    let inf = f32::INFINITY;
    // dx2 == dy2 == inf -> inf*inf - inf + inf = NaN.
    let cases: [[f32; 3]; 8] = [
        [inf, inf, 0.0],
        [inf, inf, 1.0],
        [-inf, -inf, 0.0],
        [inf, -inf, 0.0],
        [-inf, inf, 0.0],
        [f32::MAX, f32::MAX, f32::MAX],
        [inf, 1.0, inf],
        [0.0, inf, inf],
    ];
    for t in cases {
        let mut c_dest = [0.0f32; 2];
        unsafe { (c_tfm())(c_dest.as_mut_ptr(), t.as_ptr(), 1) };
        // At least one output slot must be NaN: proves the NaN survived the
        // clamp instead of being replaced by 0.0.
        assert!(
            c_dest[0].is_nan() || c_dest[1].is_nan(),
            "row9: expected a NaN to propagate for {t:?}, got {c_dest:?}"
        );
        diff_disjoint(&t, 1, 2, "row9 NaN sqd");
    }
    let mut rng = Rng::new(SEED ^ 109);
    for it in 0..20_000 {
        let pick = |rng: &mut Rng| match rng.below(6) {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            2 => f32::MAX,
            3 => f32::MIN,
            4 => rng.nan_f32(),
            _ => rng.finite_f32(),
        };
        let t = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        diff_disjoint(&t, 1, 2, &format!("row9 NaN sqd random [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 10 — sqd == +inf
// ---------------------------------------------------------------------------
#[test]
fn err_row10_inf_sqd() {
    let mut rng = Rng::new(SEED ^ 110);
    // Huge finite dxy so 4*dxy*dxy overflows to +inf while the arm operands
    // stay ordered and finite.
    for it in 0..20_000 {
        let a = rng.small_f32();
        let b = a + 1.0;
        let dxy = f32::from_bits(0x7F00_0000 | (rng.next_u32() & 0x007F_FFFF));
        let t = [a, b, dxy];
        let mut c_dest = [0.0f32; 2];
        unsafe { (c_tfm())(c_dest.as_mut_ptr(), t.as_ptr(), 1) };
        assert_eq!(
            c_dest[0],
            f32::NEG_INFINITY,
            "row10 [iter {it}]: expected dx2-inf == -inf for {t:?}"
        );
        diff_disjoint(&t, 1, 2, &format!("row10 inf sqd [iter {it}]"));
    }
    // Explicit +inf dxy in both arms.
    for t in [
        [1.0f32, 2.0, f32::INFINITY],
        [2.0f32, 1.0, f32::INFINITY],
        [1.0f32, 2.0, f32::NEG_INFINITY],
        [2.0f32, 1.0, f32::NEG_INFINITY],
    ] {
        diff_disjoint(&t, 1, 2, "row10 explicit inf dxy");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — oversized count with matching buffers: no early exit / off-by-one
// ---------------------------------------------------------------------------
#[test]
fn err_row11_large_count() {
    let mut rng = Rng::new(SEED ^ 111);
    for &count in &[4096i32, 4097, 8191, 8192, 10_000] {
        let src: Vec<f32> = (0..count as usize * 3).map(|_| rng.any_bits_f32()).collect();
        diff_disjoint(&src, count, count as usize * 2, &format!("row11 count={count}"));
        // Assert every output slot really was written (no early exit): the
        // canary check in diff_disjoint covers overrun; here check under-run by
        // confirming no slot retains the 0xA5A5A5A5 fill unless the computed
        // value genuinely equals it (impossible for 2*count slots at once).
        let mut c_dest = vec![f32::from_bits(0xA5A5_A5A5); count as usize * 2];
        unsafe { (c_tfm())(c_dest.as_mut_ptr(), src.as_ptr(), count) };
        let untouched = c_dest.iter().filter(|x| x.to_bits() == 0xA5A5_A5A5).count();
        assert!(
            untouched * 100 < c_dest.len(),
            "row11 count={count}: {untouched} of {} dest slots look unwritten",
            c_dest.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — signalling NaN across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_row12_signalling_nan() {
    let snans: [f32; 6] = [
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
    ];
    let others: [f32; 7] = [0.0, -0.0, 1.0, -1.0, f32::INFINITY, f32::MAX, f32::MIN_POSITIVE];
    for &s in &snans {
        for &o in &others {
            for t in [[s, o, o], [o, s, o], [o, o, s], [s, s, s]] {
                diff_disjoint(&t, 1, 2, "row12 sNaN grid");
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 112);
    for it in 0..20_000 {
        let mut t = [rng.finite_f32(), rng.finite_f32(), rng.finite_f32()];
        // Random signalling NaN: exponent all ones, quiet bit CLEAR, payload != 0.
        let mk_snan = |rng: &mut Rng| {
            let sign = (rng.next_u32() & 1) << 31;
            let payload = rng.below(0x0040_0000).max(1);
            f32::from_bits(sign | 0x7F80_0000 | payload)
        };
        t[(it % 3) as usize] = mk_snan(&mut rng);
        assert!(t[(it % 3) as usize].is_nan());
        diff_disjoint(&t, 1, 2, &format!("row12 random sNaN [iter {it}]"));
    }
}

// ---------------------------------------------------------------------------
// Row 13 — subnormals, signed zeros, FLT_MIN / FLT_MAX extremes
// ---------------------------------------------------------------------------
#[test]
fn err_row13_subnormal_and_extremes() {
    let extremes: [f32; 14] = [
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::from_bits(0x7F7F_FFFE),
        f32::from_bits(0xFF7F_FFFE),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    // Exhaustive 14^3 = 2744 grid.
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                diff_disjoint(&[a, b, c], 1, 2, "row13 extremes grid");
            }
        }
    }
    // Verify sign-of-zero really is observable and matched: dest[1] = dxy in the
    // then arm, so -0.0 must come out as -0.0 (bits 0x80000000).
    let t = [1.0f32, 2.0f32, -0.0f32];
    let mut c_dest = [0.0f32; 2];
    unsafe { (c_tfm())(c_dest.as_mut_ptr(), t.as_ptr(), 1) };
    assert_eq!(c_dest[1].to_bits(), 0x8000_0000, "row13: -0.0 sign lost by C?");
    diff_disjoint(&t, 1, 2, "row13 negative zero passthrough");
}

// ---------------------------------------------------------------------------
// Row 14 — dest == src, no aliasing guard in C
// ---------------------------------------------------------------------------
#[test]
fn err_row14_aliased_dest_src() {
    let mut rng = Rng::new(SEED ^ 114);
    for it in 0..20_000 {
        let count = (rng.below(24) + 1) as i32;
        let n = count as usize * 3 + 4;
        let buf: Vec<f32> = (0..n)
            .map(|_| match it % 3 {
                0 => rng.finite_f32(),
                1 => rng.special_f32(),
                _ => rng.any_bits_f32(),
            })
            .collect();
        diff_aliased(&buf, 0, 0, count, &format!("row14 dest==src [iter {it}]"));
    }
    // A hand-checked case: with count=2 and dest==src, element 1 reads src[3..6]
    // where src[3] and src[4] were already overwritten by element 0's output.
    let buf = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    diff_aliased(&buf, 0, 0, 2, "row14 hand-checked clobber");
}
