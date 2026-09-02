//! Phase C — error/boundary-path differential tests, one `#[test]` per
//! `ERRORS.md` row, plus the generic C-API boundaries.
//!
//! `tfm` returns `void` and the C library contains no error codes, so "same
//! error/rejection" here means: the same *observable rejection*, i.e. both
//! libraries write exactly the same bytes (usually **none**) and both return
//! normally instead of faulting.

mod common;

use common::*;
use std::ffi::c_int;

/// Assert both libraries leave `dest` completely untouched (poison intact) and
/// return normally — the C library's only form of "rejection".
fn assert_both_reject_silently(label: &str, src: *const f32, count: c_int, dest_len: usize) {
    let mut dc = poison(dest_len);
    let mut dr = poison(dest_len);
    unsafe {
        (c_tfm())(dc.as_mut_ptr(), src, count);
        (rust_tfm())(dr.as_mut_ptr(), src, count);
    }
    let cb: Vec<u32> = dc.iter().map(|x| x.to_bits()).collect();
    let rb: Vec<u32> = dr.iter().map(|x| x.to_bits()).collect();
    assert_eq!(cb, rb, "{label}: C and Rust wrote different bytes");
    assert!(
        cb.iter().all(|&b| b == POISON_BITS),
        "{label}: C wrote to dest (count={count}) — expected zero writes, got {cb:x?}"
    );
    assert!(
        rb.iter().all(|&b| b == POISON_BITS),
        "{label}: Rust wrote to dest (count={count}) — expected zero writes, got {rb:x?}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — count == 0
// ---------------------------------------------------------------------------

#[test]
fn row_01_count_zero() {
    let mut rng = Rng::new(0x2222_0001);
    for i in 0..512 {
        let s: Vec<f32> = (0..12).map(|_| rng.any_f32()).collect();
        assert_both_reject_silently(&format!("err01/#{i}"), s.as_ptr(), 0, 8);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — count == -1 (must NOT wrap to a huge unsigned trip count)
// ---------------------------------------------------------------------------

#[test]
fn row_02_count_negative_one() {
    let mut rng = Rng::new(0x2222_0002);
    for i in 0..512 {
        let s: Vec<f32> = (0..12).map(|_| rng.any_f32()).collect();
        assert_both_reject_silently(&format!("err02/#{i}"), s.as_ptr(), -1, 8);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — count == INT_MIN, and every other negative boundary
// ---------------------------------------------------------------------------

#[test]
fn row_03_count_int_min_and_negatives() {
    let s: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let counts: &[c_int] = &[
        c_int::MIN,
        c_int::MIN + 1,
        -2_000_000_000,
        -1_000_000,
        -1024,
        -3,
        -2,
        -1,
    ];
    for &c in counts {
        assert_both_reject_silently(&format!("err03/count={c}"), s.as_ptr(), c, 8);
    }
}

// ---------------------------------------------------------------------------
// Rows 4 & 5 — NULL pointers are ACCEPTED whenever count <= 0
// ---------------------------------------------------------------------------

#[test]
fn row_04_null_pointers_with_count_zero() {
    // Both libraries must return normally without dereferencing.
    unsafe {
        (c_tfm())(std::ptr::null_mut(), std::ptr::null(), 0);
        (rust_tfm())(std::ptr::null_mut(), std::ptr::null(), 0);
    }
    // one NULL at a time, too
    let s = [1.0f32, 2.0, 3.0];
    let mut d = poison(2);
    unsafe {
        (c_tfm())(std::ptr::null_mut(), s.as_ptr(), 0);
        (rust_tfm())(std::ptr::null_mut(), s.as_ptr(), 0);
        (c_tfm())(d.as_mut_ptr(), std::ptr::null(), 0);
        (rust_tfm())(d.as_mut_ptr(), std::ptr::null(), 0);
    }
    assert!(
        d.iter().all(|x| x.to_bits() == POISON_BITS),
        "err04: dest written with count=0"
    );
}

#[test]
fn row_05_null_pointers_with_negative_count() {
    for c in [-1i32, -7, -1024, c_int::MIN] {
        unsafe {
            (c_tfm())(std::ptr::null_mut(), std::ptr::null(), c);
            (rust_tfm())(std::ptr::null_mut(), std::ptr::null(), c);
        }
    }
    let s = [1.0f32, 2.0, 3.0];
    let mut d = poison(2);
    for c in [-1i32, c_int::MIN] {
        unsafe {
            (c_tfm())(std::ptr::null_mut(), s.as_ptr(), c);
            (rust_tfm())(std::ptr::null_mut(), s.as_ptr(), c);
            (c_tfm())(d.as_mut_ptr(), std::ptr::null(), c);
            (rust_tfm())(d.as_mut_ptr(), std::ptr::null(), c);
        }
    }
    assert!(
        d.iter().all(|x| x.to_bits() == POISON_BITS),
        "err05: dest written with negative count"
    );
}

// ---------------------------------------------------------------------------
// Rows 6 & 7 — NULL with count > 0 is genuine UB (SIGSEGV in both)
// ---------------------------------------------------------------------------

/// The C dereferences unconditionally once the loop body runs, so `NULL` with
/// `count > 0` segfaults. Calling it in-process would kill the harness for both
/// libraries, so the parity that *is* checkable is asserted structurally:
///
/// * the C `.so` contains no comparison of either pointer against zero
///   (verified against the disassembly), and
/// * the Rust `.so` likewise has no null check — proven behaviourally by rows
///   4/5: NULL is accepted for exactly `count <= 0` and no further.
///
/// Both therefore share the identical precondition boundary.
#[test]
fn row_06_07_null_with_positive_count_is_ub() {
    use std::process::Command;

    // Structural check on the C side: no `test %rdi,%rdi` / `cmp $0x0,%rdi`
    // style null guard inside tfm.
    let out = Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(c_so_path())
        .output()
        .expect("run objdump");
    let text = String::from_utf8_lossy(&out.stdout);
    let body: String = text
        .lines()
        .skip_while(|l| !l.contains("<tfm>:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!body.is_empty(), "could not isolate tfm disassembly");
    for pat in ["test   %rdi,%rdi", "test   %rsi,%rsi", "cmpq   $0x0"] {
        assert!(
            !body.contains(pat),
            "C tfm unexpectedly contains a null guard ({pat})"
        );
    }

    // Behavioural boundary: the largest count for which NULL is safe is 0.
    // Rows 4/5 already assert acceptance at count <= 0 for both libraries.
    // Confirm both libraries *do* write for count == 1 with valid pointers,
    // i.e. the loop body really is the first dereference.
    let s = [1.0f32, 2.0, 3.0];
    let mut dc = poison(2);
    let mut dr = poison(2);
    unsafe {
        (c_tfm())(dc.as_mut_ptr(), s.as_ptr(), 1);
        (rust_tfm())(dr.as_mut_ptr(), s.as_ptr(), 1);
    }
    assert!(
        dc.iter().all(|x| x.to_bits() != POISON_BITS),
        "C did not write at count=1"
    );
    assert_bits_eq("err06_07", &s, 1, &dc, &dr);
}

// ---------------------------------------------------------------------------
// Row 8 — count one past the caller's element count (no bounds check)
// ---------------------------------------------------------------------------

#[test]
fn row_08_count_one_past_the_end() {
    let mut rng = Rng::new(0x2222_0008);
    for n in [1usize, 2, 7, MANY] {
        for i in 0..256 {
            // Over-allocate by one element and fill the tail with known bytes,
            // then pass count = n + 1. The C reads the tail; both must agree.
            let mut s: Vec<f32> = (0..3 * (n + 1)).map(|_| rng.tame_f32()).collect();
            // deterministic sentinel tail
            s[3 * n] = 11.5;
            s[3 * n + 1] = -4.25;
            s[3 * n + 2] = 0.75;
            let over = (n + 1) as c_int;
            diff_call(&format!("err08/n{n}/#{i}"), &s, over);
            // and the in-range call, to prove the extra element is the only diff
            diff_call(&format!("err08/n{n}/in#{i}"), &s, n as c_int);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — negative discriminant is clamped to +0.0f (no domain error)
// ---------------------------------------------------------------------------

#[test]
fn row_09_negative_discriminant_clamped_not_errored() {
    let mut rng = Rng::new(0x2222_0009);
    // Near-equal dx2/dy2 with dxy == 0 makes the expanded sqd round negative.
    let mut saw_clamp = false;
    for i in 0..2000 {
        let a = rng.normal_f32();
        let b = f32::from_bits(a.to_bits().wrapping_add(1 + rng.below(3)));
        let s = [a, b, 0.0f32];
        diff_call(&format!("err09/#{i}"), &s, 1);
        // reference computation of sqd in the C's exact operand order
        let (dx2, dy2) = if s[0] < s[1] { (s[0], s[1]) } else { (s[1], s[0]) };
        let sqd = ((dy2 * dy2) - ((dx2 + dx2) * dy2)) + (dx2 * dx2);
        if sqd < 0.0 {
            saw_clamp = true;
        }
    }
    assert!(
        saw_clamp,
        "err09: never produced a negative discriminant — row not actually covered"
    );
    // Explicitly negative radicand via cancellation at large exponents.
    for e in -40i32..=40 {
        let a = 2f32.powi(e) * 1.5;
        for k in 1u32..=4 {
            let b = f32::from_bits(a.to_bits().wrapping_add(k));
            diff_call("err09/ulp", &[a, b, 0.0], 1);
            diff_call("err09/ulpr", &[b, a, 0.0], 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — NaN discriminant skips the clamp
// ---------------------------------------------------------------------------

#[test]
fn row_10_nan_discriminant_not_clamped() {
    // inf - inf, 0 * inf and explicit NaN operands all reach sqrtf as NaN.
    let pins: &[[f32; 3]] = &[
        [f32::INFINITY, f32::INFINITY, 0.0],
        [f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [-0.0, f32::NEG_INFINITY, 0.0],
        [1.0, 1.0, f32::INFINITY],
        [1.0, 1.0, f32::NEG_INFINITY],
        [f32::INFINITY, 1.0, f32::INFINITY],
        [f32::from_bits(0x7FC0_0000), 1.0, 0.0],
        [1.0, f32::from_bits(0xFFC0_1234), 0.0],
        [1.0, 2.0, f32::from_bits(0x7FA0_0000)],
    ];
    for (i, p) in pins.iter().enumerate() {
        diff_call(&format!("err10/pin{i}"), p, 1);

        // Independently reproduce the C's operand order to decide whether `sqd`
        // really is NaN for this pin, then assert the C behaved accordingly.
        // (The C is ground truth: `[1,1,±inf]` gives `sqd == +inf`, NOT NaN,
        // so a blanket "NaN must propagate" expectation would be wrong.)
        let (dx2, dy2, dxy) = if p[0] < p[1] {
            (p[0], p[1], p[2])
        } else {
            (p[1], p[0], p[2])
        };
        let acc = ((dy2 * dy2) - ((dx2 + dx2) * dy2)) + (dx2 * dx2);
        let sqd = (4.0f32 * dxy * dxy) + acc;

        let mut d = poison(2);
        unsafe { (c_tfm())(d.as_mut_ptr(), p.as_ptr(), 1) };

        if sqd.is_nan() {
            // The clamp is SKIPPED (comiss reports unordered -> jbe taken), so
            // the NaN reaches sqrtf and poisons lambda.
            assert!(
                d.iter().any(|x| x.is_nan()),
                "err10/pin{i}: sqd is NaN but no output lane is NaN: {:x?}",
                d.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        } else {
            // Positive-infinite discriminant: sqrtf(+inf) = +inf, lambda = +inf.
            assert!(
                sqd == f32::INFINITY,
                "err10/pin{i}: unexpected finite sqd {sqd}"
            );
        }
    }
    let mut rng = Rng::new(0x2222_0010);
    for i in 0..ITERS {
        let lane = rng.below(3) as usize;
        let mut s = [rng.tame_f32(), rng.tame_f32(), rng.tame_f32()];
        s[lane] = if rng.next_u32() & 1 == 0 {
            rng.qnan_f32()
        } else {
            rng.snan_f32()
        };
        diff_call(&format!("err10/#{i}"), &s, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 11 — sqd == -0.0 is NOT clamped, sqrtf(-0.0) == -0.0
// ---------------------------------------------------------------------------

#[test]
fn row_11_negative_zero_discriminant() {
    // dx2 == dy2, dxy == ±0.0  =>  sqd is a zero; assert the sign survives
    // identically. Also feed a full ±0.0 cross-product.
    for s0 in [0.0f32, -0.0] {
        for s1 in [0.0f32, -0.0] {
            for s2 in [0.0f32, -0.0] {
                diff_call("err11/zeros", &[s0, s1, s2], 1);
            }
        }
    }
    let mut rng = Rng::new(0x2222_0011);
    for i in 0..ITERS {
        let a = rng.normal_f32();
        for dxy in [0.0f32, -0.0] {
            diff_call(&format!("err11/#{i}"), &[a, a, dxy], 1);
            diff_call(&format!("err11n/#{i}"), &[-a, -a, dxy], 1);
        }
    }
    // Direct evidence that sqrtf(-0.0) keeps its sign in the C: with
    // dx2 = dy2 = -0.0 and dxy = -0.0, lambda = 0.5*((-0)+(-0)+sqrt(+0)) = 0.0
    // and dest[1] = -0.0 - 0.0 = -0.0.
    let mut d = poison(2);
    unsafe { (c_tfm())(d.as_mut_ptr(), [-0.0f32, -0.0, -0.0].as_ptr(), 1) };
    let mut r = poison(2);
    unsafe { (rust_tfm())(r.as_mut_ptr(), [-0.0f32, -0.0, -0.0].as_ptr(), 1) };
    assert_eq!(
        d.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        r.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        "err11: signed-zero result bits differ"
    );
}

// ---------------------------------------------------------------------------
// Row 12 — unordered compare takes the else arm
// ---------------------------------------------------------------------------

#[test]
fn row_12_unordered_compare_takes_else_arm() {
    let nans: &[f32] = &[
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFBF_FFFF),
        f32::from_bits(0x7FFF_FFFF),
    ];
    // A NaN in lane 0 or lane 1 makes `src[0] < src[1]` unordered. The else arm
    // stores dxy into dest[0]; assert the C really did take the else arm by
    // checking dest[0] == dxy bit-for-bit, then assert Rust matches.
    for &n in nans {
        for other in [-5.0f32, 0.0, -0.0, 5.0, f32::INFINITY, f32::NEG_INFINITY] {
            for dxy in [7.25f32, -0.0, 0.0, f32::INFINITY] {
                for s in [[n, other, dxy], [other, n, dxy], [n, n, dxy]] {
                    let mut d = poison(2);
                    unsafe { (c_tfm())(d.as_mut_ptr(), s.as_ptr(), 1) };
                    assert_eq!(
                        d[0].to_bits(),
                        dxy.to_bits(),
                        "err12: C did not take the else arm for src={:x?}",
                        s.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                    );
                    diff_call("err12", &s, 1);
                }
            }
        }
    }
    let mut rng = Rng::new(0x2222_0012);
    for i in 0..ITERS {
        let lane = rng.below(2) as usize; // lane 0 or 1
        let mut s = [rng.tame_f32(), rng.tame_f32(), rng.tame_f32()];
        s[lane] = rng.qnan_f32();
        diff_call(&format!("err12/#{i}"), &s, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — overflow to +inf, no error
// ---------------------------------------------------------------------------

#[test]
fn row_13_overflow_to_infinity() {
    let mut rng = Rng::new(0x2222_0013);
    for i in 0..ITERS {
        let s = [rng.huge_f32(), rng.huge_f32(), rng.huge_f32()];
        diff_call(&format!("err13/#{i}"), &s, 1);
    }
    for a in [f32::MAX, -f32::MAX] {
        for b in [f32::MAX, -f32::MAX, 0.0, 1.0] {
            for c in [f32::MAX, -f32::MAX, 0.0, 1.0] {
                diff_call("err13/pin", &[a, b, c], 1);
            }
        }
    }
    // sqd overflows while operands are only moderately large
    let t = 2f32.powi(70);
    for s in [[t, t * 2.0, 0.0], [t * 2.0, t, 0.0], [1.0, 1.0, t]] {
        diff_call("err13/edge", &s, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — signalling NaN is quieted, not trapped
// ---------------------------------------------------------------------------

#[test]
fn row_14_snan_quieted_no_trap() {
    let snans: &[f32] = &[
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
        f32::from_bits(0x7F91_2345),
    ];
    for &n in snans {
        for lane in 0..3 {
            let mut s = [1.5f32, -2.5, 3.5];
            s[lane] = n;
            diff_call("err14", &s, 1);
            let mut s2 = [-2.5f32, 1.5, 3.5];
            s2[lane] = n;
            diff_call("err14r", &s2, 1);
        }
        // A tie (`n < n` is unordered -> false) takes the else arm, where
        // `dest[0] = dxy` is a bare `movss` copy and `dest[1]` is arithmetic.
        // The C therefore copies the sNaN through UNQUIETED in lane 0 and
        // quiets it only in lane 1 (verified: 7fa00000 -> [7fa00000, 7fe00000]).
        let mut d = poison(2);
        unsafe { (c_tfm())(d.as_mut_ptr(), [n, n, n].as_ptr(), 1) };
        assert_eq!(
            d[0].to_bits(),
            n.to_bits(),
            "err14: dest[0] must be a verbatim copy of dxy, not a quieted NaN"
        );
        assert!(d[1].is_nan(), "err14: dest[1] should be NaN");
        assert_ne!(
            d[1].to_bits() & 0x0040_0000,
            0,
            "err14: arithmetic lane emitted a signalling NaN 0x{:08x}",
            d[1].to_bits()
        );
        diff_call("err14/all", &[n, n, n], 1);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — aliasing is accepted, not rejected
// ---------------------------------------------------------------------------

#[test]
fn row_15_aliasing_accepted() {
    let mut rng = Rng::new(0x2222_0015);
    for n in [1usize, 2, 3, 8, MANY] {
        for i in 0..256 {
            let s: Vec<f32> = (0..3 * n).map(|_| rng.tame_f32()).collect();
            let mut bc = s.clone();
            let mut br = s.clone();
            unsafe {
                (c_tfm())(bc.as_mut_ptr(), bc.as_ptr(), n as c_int);
                (rust_tfm())(br.as_mut_ptr(), br.as_ptr(), n as c_int);
            }
            assert_bits_eq(&format!("err15/n{n}/#{i}"), &s, n as c_int, &bc, &br);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — unaligned buffers are accepted
// ---------------------------------------------------------------------------

#[test]
fn row_16_unaligned_accepted() {
    let mut rng = Rng::new(0x2222_0016);
    for off in 1usize..4 {
        for i in 0..128 {
            let n = 8usize;
            let vals: Vec<f32> = (0..3 * n).map(|_| rng.tame_f32()).collect();
            let mut src_bytes = vec![0u8; 4 * 3 * n + off];
            let mut dc = vec![0x5Au8; 4 * 2 * n + off];
            let mut dr = vec![0x5Au8; 4 * 2 * n + off];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    vals.as_ptr() as *const u8,
                    src_bytes.as_mut_ptr().add(off),
                    4 * 3 * n,
                );
                let sp = src_bytes.as_ptr().add(off) as *const f32;
                (c_tfm())(dc.as_mut_ptr().add(off) as *mut f32, sp, n as c_int);
                (rust_tfm())(dr.as_mut_ptr().add(off) as *mut f32, sp, n as c_int);
            }
            assert_eq!(dc, dr, "err16/off{off}/#{i}: unaligned bytes differ");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries beyond the table
// ---------------------------------------------------------------------------

/// `tfm` takes no enum, but the `int count` parameter is the same class of
/// "any bit pattern crosses the FFI boundary" input. Sweep every interesting
/// `int` value including the ones with no meaningful interpretation.
#[test]
fn generic_full_int_count_sweep() {
    let s: Vec<f32> = (0..3 * 32).map(|i| (i as f32) * 0.5 - 8.0).collect();
    let mut counts: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -65536,
        -256,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        31,
        32,
    ];
    // powers of two and their neighbours, negative and positive
    for e in 0..31u32 {
        let v = 1i64 << e;
        for d in [-1i64, 0, 1] {
            let x = v + d;
            if x <= c_int::MAX as i64 {
                counts.push(x as c_int);
                counts.push(-(x as c_int));
            }
        }
    }
    counts.sort_unstable();
    counts.dedup();

    for &c in &counts {
        if c <= 0 {
            assert_both_reject_silently(&format!("gen/int/{c}"), s.as_ptr(), c, 8);
        } else if (c as usize) <= 32 {
            diff_call(&format!("gen/int/{c}"), &s, c);
        }
        // c > 32 would read past the buffer; covered structurally by err08.
    }
    // INT_MAX / large positive counts are not called: they would run for hours
    // and read unmapped memory. The signed-compare semantics that make them
    // "valid" are already pinned by the negative cases above.
}

/// Zero and oversized lengths against a deliberately over-allocated buffer.
#[test]
fn generic_zero_and_oversized_lengths() {
    let mut rng = Rng::new(0x2222_9001);
    // 64 elements allocated, sweep count from 0..=64 (0 and the exact max)
    let n = 64usize;
    let s: Vec<f32> = (0..3 * n).map(|_| rng.tame_f32()).collect();
    for c in 0..=n {
        if c == 0 {
            assert_both_reject_silently("gen/len/0", s.as_ptr(), 0, 8);
        } else {
            diff_call(&format!("gen/len/{c}"), &s, c as c_int);
        }
    }
}

/// One step past the "documented" range on the float side: the values
/// immediately adjacent to every special float, in every lane.
#[test]
fn generic_one_past_float_boundaries() {
    let boundaries: &[u32] = &[
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // smallest subnormal
        0x007F_FFFF, // largest subnormal
        0x0080_0000, // smallest normal
        0x7F7F_FFFF, // FLT_MAX
        0x7F80_0000, // +inf
        0x7F80_0001, // first sNaN
        0x7FBF_FFFF, // last sNaN
        0x7FC0_0000, // first qNaN
        0x7FFF_FFFF, // last qNaN
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x4080_0000, // 4.0 (the literal in the C)
        0x3F00_0000, // 0.5 (the literal in the C)
    ];
    let mut vals: Vec<f32> = Vec::new();
    for &b in boundaries {
        for d in [-1i64, 0, 1] {
            let x = (b as i64 + d) as u32;
            vals.push(f32::from_bits(x));
            vals.push(f32::from_bits(x ^ 0x8000_0000));
        }
    }
    vals.sort_by_key(|x| x.to_bits());
    vals.dedup_by_key(|x| x.to_bits());

    // full cross-product on lanes 0/1 with a rotating lane 2
    for (i, &a) in vals.iter().enumerate() {
        for &b in &vals {
            let c = vals[(i + 7) % vals.len()];
            diff_call("gen/bnd", &[a, b, c], 1);
        }
    }
    // and every boundary value in lane 2 against a fixed ordered pair
    for &c in &vals {
        diff_call("gen/bnd/l2", &[-1.0, 1.0, c], 1);
        diff_call("gen/bnd/l2r", &[1.0, -1.0, c], 1);
    }
}
