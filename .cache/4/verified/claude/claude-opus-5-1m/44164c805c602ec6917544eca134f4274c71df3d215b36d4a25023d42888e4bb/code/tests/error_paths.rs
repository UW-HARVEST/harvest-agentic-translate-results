//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E18) plus the generic FFI boundary
//! rows (G1..G5). Every test drives BOTH shared objects through their exported
//! `wcscat` symbol and asserts the same exact return code (22 = EINVAL,
//! 34 = ERANGE, 0 = success) *and* the same resulting memory image.

mod common;

use common::*;

const EINVAL: i32 = 22;
const ERANGE: i32 = 34;
const POW62: usize = 1usize << 62;

const REPS: usize = 200;

// ===========================================================================
// E1 — dst == NULL, numElem != 0, src valid
// ===========================================================================
#[test]
fn e1_null_dst() {
    let mut rng = Rng::new(0xE001);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 4096);
        let g = rng.range(0, 4);
        let len = rng.range(0, 8);
        let src = make_src(&mut rng, len, g, class);
        let case = Case::new(Dst::Null, n, Src::External(src.clone()));
        assert_same_ret(&case, EINVAL);
        // src must be left completely untouched.
        let o = assert_same(&case);
        assert_eq!(o.src_after.as_deref(), Some(&src[..]));
    }
}

// ===========================================================================
// E2 — dst != NULL, numElem == 0
// ===========================================================================
#[test]
fn e2_zero_numelem() {
    let mut rng = Rng::new(0xE002);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let alloc = rng.range(1, 64);
        let k = rng.range(0, alloc - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        let g = rng.range(0, 4);
        let len = rng.range(0, 8);
        let src = make_src(&mut rng, len, g, class);
        let case = Case::new(Dst::Buf(dst.clone()), 0, Src::External(src));
        let o = assert_same(&case);
        assert_eq!(o.ret, EINVAL);
        // The `numElem == 0` branch returns *before* the `dst[0] = 0` store.
        assert_eq!(
            o.dst_after.as_deref(),
            Some(&dst[..]),
            "numElem == 0 must not modify dst"
        );
    }
}

// ===========================================================================
// E3 — dst == NULL and numElem == 0
// ===========================================================================
#[test]
fn e3_null_dst_and_zero_numelem() {
    let mut rng = Rng::new(0xE003);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let len = rng.range(0, 8);
        let src = make_src(&mut rng, len, 2, class);
        assert_same_ret(&Case::new(Dst::Null, 0, Src::External(src)), EINVAL);
    }
}

// ===========================================================================
// E4 — dst == NULL, numElem == 0, src == NULL (first branch must win)
// ===========================================================================
#[test]
fn e4_all_null_zero() {
    assert_same_ret(&Case::new(Dst::Null, 0, Src::Null), EINVAL);
    // dst NULL + src NULL but a non-zero numElem: still the first branch.
    for n in [1usize, 2, 7, 64, 4096, usize::MAX, POW62, POW62 + 1] {
        assert_same_ret(&Case::new(Dst::Null, n, Src::Null), EINVAL);
    }
}

// ===========================================================================
// E5 — src == NULL with a valid dst: stores dst[0] = 0, returns 22
// ===========================================================================
#[test]
fn e5_null_src_writes_dst0() {
    let mut rng = Rng::new(0xE005);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let alloc = rng.range(1, 64);
        let n = rng.range(1, alloc);
        let k = rng.range(0, alloc - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        let case = Case::new(Dst::Buf(dst.clone()), n, Src::Null);
        let o = assert_same(&case);
        assert_eq!(o.ret, EINVAL);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0, "dst[0] must be cleared");
        assert_eq!(&after[1..], &dst[1..], "only dst[0] may change");
    }
}

// ===========================================================================
// E6 — src == NULL, numElem == 1
// ===========================================================================
#[test]
fn e6_null_src_numelem_1() {
    let mut rng = Rng::new(0xE006);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let dst = vec![nonzero(&mut rng, class), nonzero(&mut rng, class)];
        let case = Case::new(Dst::Buf(dst.clone()), 1, Src::Null);
        let o = assert_same(&case);
        assert_eq!(o.ret, EINVAL);
        let after = o.dst_after.unwrap();
        assert_eq!(after, vec![0, dst[1]]);
    }
}

// ===========================================================================
// E7 — truncation with a partial copy
// ===========================================================================
#[test]
fn e7_truncation_partial_copy() {
    let mut rng = Rng::new(0xE007);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 96);
        let k = rng.range(0, n - 1);
        let room = n - k;
        let len = room + rng.range(1, 32); // strictly too long
        let dst = make_dst(&mut rng, n, k, class);
        let g = rng.range(0, 4);
        let src = make_src(&mut rng, len, g, class);
        let case = Case::new(Dst::Buf(dst), n, Src::External(src.clone()));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        // The partial copy really happened: dst[k..n] == src[0..room], except
        // dst[0] which was clobbered by the trailing `dst[0] = 0` store.
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        for j in k.max(1)..n {
            assert_eq!(after[j], src[j - k], "partial copy mismatch at {j}");
        }
        assert_eq!(o.src_after.as_deref(), Some(&src[..]));
    }
}

// ===========================================================================
// E8 — truncation by exactly one element
// ===========================================================================
#[test]
fn e8_truncation_off_by_one() {
    let mut rng = Rng::new(0xE008);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 96);
        let k = rng.range(0, n - 1);
        let len = n - k; // needs exactly one slot more than available
        let dst = make_dst(&mut rng, n, k, class);
        let g = rng.range(0, 4);
        let src = make_src(&mut rng, len, g, class);
        assert_same_ret(
            &Case::new(Dst::Buf(dst), n, Src::External(src)),
            ERANGE,
        );
    }
}

// ===========================================================================
// E9 — no NUL in dst[0..numElem]: src never read
// ===========================================================================
#[test]
fn e9_unterminated_dst() {
    let mut rng = Rng::new(0xE009);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 96);
        let dst = make_dst_unterminated(&mut rng, n, class);
        // A source with no terminator at all — proves it is never read.
        let sl = rng.range(1, 8);
        let src = make_src_unterminated(&mut rng, sl, class);
        let case = Case::new(Dst::Buf(dst.clone()), n, Src::External(src.clone()));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
        assert_eq!(o.src_after.as_deref(), Some(&src[..]));
    }
}

// ===========================================================================
// E10 — numElem == 1 with dst[0] != 0
// ===========================================================================
#[test]
fn e10_numelem_1_unterminated() {
    let mut rng = Rng::new(0xE00A);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let dst = vec![nonzero(&mut rng, class), nonzero(&mut rng, class)];
        let src = make_src_unterminated(&mut rng, 4, class);
        let case = Case::new(Dst::Buf(dst.clone()), 1, Src::External(src));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        assert_eq!(o.dst_after.unwrap(), vec![0, dst[1]]);
    }
}

// ===========================================================================
// E11 — numElem == 1, dst[0] == 0, src non-empty
// ===========================================================================
#[test]
fn e11_numelem_1_no_room() {
    let mut rng = Rng::new(0xE00B);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let tail = nonzero(&mut rng, class);
        let dst = vec![0, tail];
        let len = rng.range(1, 6);
        let src = make_src(&mut rng, len, 2, class);
        let case = Case::new(Dst::Buf(dst), 1, Src::External(src));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        // src[0] is written into dst[0] and then overwritten by the final
        // `dst[0] = 0` store, so the observable result is [0, tail].
        assert_eq!(o.dst_after.unwrap(), vec![0, tail]);
    }
}

// ===========================================================================
// E12 — dst's terminator lies outside the numElem window
// ===========================================================================
#[test]
fn e12_terminator_outside_window() {
    let mut rng = Rng::new(0xE00C);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let alloc = n + rng.range(1, 32);
        let k = rng.range(n, alloc - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        let g = rng.range(0, 4);
        let len = rng.range(0, 8);
        let src = make_src(&mut rng, len, g, class);
        let case = Case::new(Dst::Buf(dst.clone()), n, Src::External(src));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// E13 — k == numElem - 1: room for the terminator only, src non-empty
// ===========================================================================
#[test]
fn e13_room_for_terminator_only() {
    let mut rng = Rng::new(0xE00D);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let dst = make_dst(&mut rng, n, n - 1, class);
        let len = rng.range(1, 8);
        let src = make_src(&mut rng, len, 2, class);
        let case = Case::new(Dst::Buf(dst.clone()), n, Src::External(src.clone()));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(after[n - 1], src[0], "src[0] must land in dst[numElem-1]");
        if n > 2 {
            assert_eq!(&after[1..n - 1], &dst[1..n - 1]);
        }
    }
}

// ===========================================================================
// E14 — numElem == SIZE_MAX (dst + numElem wraps below dst)
// ===========================================================================
#[test]
fn e14_numelem_size_max() {
    let mut rng = Rng::new(0xE00E);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let alloc = rng.range(4, 32);
        let k = rng.range(0, alloc - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        let sl = rng.range(0, 4);
        let src = make_src(&mut rng, sl, 2, class);
        let case = Case::new(Dst::Buf(dst.clone()), usize::MAX, Src::External(src));
        let o = assert_same(&case);
        assert_eq!(o.ret, ERANGE);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// E15 — numElem * sizeof(wchar_t) wraps to exactly 0 => end == dst
// ===========================================================================
#[test]
fn e15_numelem_wraps_to_zero() {
    let mut rng = Rng::new(0xE00F);
    for n in [POW62, 1usize << 63, POW62 * 3] {
        for i in 0..64 {
            let class = ALL_CLASSES[i % ALL_CLASSES.len()];
            let alloc = rng.range(4, 32);
            let k = rng.range(0, alloc - 1);
            let dst = make_dst(&mut rng, alloc, k, class);
            let sl = rng.range(0, 4);
            let src = make_src(&mut rng, sl, 2, class);
            let case = Case::new(Dst::Buf(dst.clone()), n, Src::External(src));
            let o = assert_same(&case);
            assert_eq!(o.ret, ERANGE, "n = {n:#x}");
            let after = o.dst_after.unwrap();
            assert_eq!(after[0], 0);
            assert_eq!(&after[1..], &dst[1..]);
        }
    }
}

// ===========================================================================
// E16 — numElem wraps to end == dst + 1 (behaves like numElem == 1)
// ===========================================================================
#[test]
fn e16_numelem_wraps_to_one() {
    let mut rng = Rng::new(0xE010);
    let n = POW62 + 1;
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        // (a) dst[0] != 0 -> ERANGE
        let dst = vec![nonzero(&mut rng, class), nonzero(&mut rng, class)];
        let o = assert_same(&Case::new(
            Dst::Buf(dst.clone()),
            n,
            Src::External(make_src_unterminated(&mut rng, 4, class)),
        ));
        assert_eq!(o.ret, ERANGE);
        assert_eq!(o.dst_after.unwrap(), vec![0, dst[1]]);

        // (b) dst[0] == 0, src non-empty -> ERANGE
        let tail = nonzero(&mut rng, class);
        let sl = rng.range(1, 4);
        let src = make_src(&mut rng, sl, 2, class);
        let o = assert_same(&Case::new(Dst::Buf(vec![0, tail]), n, Src::External(src)));
        assert_eq!(o.ret, ERANGE);
        assert_eq!(o.dst_after.unwrap(), vec![0, tail]);

        // (c) dst[0] == 0, src empty -> success
        let src = make_src(&mut rng, 0, 2, class);
        let o = assert_same(&Case::new(Dst::Buf(vec![0, tail]), n, Src::External(src)));
        assert_eq!(o.ret, 0);
        assert_eq!(o.dst_after.unwrap(), vec![0, tail]);
    }
}

// ===========================================================================
// E17 — numElem == SIZE_MAX combined with src == NULL: EINVAL, not ERANGE
// ===========================================================================
#[test]
fn e17_size_max_null_src() {
    let mut rng = Rng::new(0xE011);
    for n in [usize::MAX, usize::MAX - 1, POW62, POW62 + 1, 1usize << 63] {
        for i in 0..32 {
            let class = ALL_CLASSES[i % ALL_CLASSES.len()];
            let alloc = rng.range(2, 16);
            let k = rng.range(0, alloc - 1);
            let dst = make_dst(&mut rng, alloc, k, class);
            let case = Case::new(Dst::Buf(dst.clone()), n, Src::Null);
            let o = assert_same(&case);
            assert_eq!(o.ret, EINVAL, "n = {n:#x} must take the !src branch");
            let after = o.dst_after.unwrap();
            assert_eq!(after[0], 0);
            assert_eq!(&after[1..], &dst[1..]);
        }
    }
}

// ===========================================================================
// E18 — numElem == 0 with dst[0] != 0: buffer untouched
// ===========================================================================
#[test]
fn e18_zero_numelem_no_write() {
    let mut rng = Rng::new(0xE012);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let alloc = rng.range(1, 32);
        let dst = make_dst_unterminated(&mut rng, alloc, class);
        for src in [
            Src::Null,
            Src::External(make_src(&mut rng, 3, 2, class)),
            Src::AliasDst(0),
        ] {
            let case = Case::new(Dst::Buf(dst.clone()), 0, src);
            let o = assert_same(&case);
            assert_eq!(o.ret, EINVAL);
            assert_eq!(o.dst_after.as_deref(), Some(&dst[..]));
        }
    }
}

// ===========================================================================
// G1 — full null-pointer matrix
// ===========================================================================
#[test]
fn g1_null_pointer_matrix() {
    let mut rng = Rng::new(0x6001);
    for &n in &[0usize, 1, 2, 8, 4096] {
        // (NULL, NULL)
        assert_same_ret(&Case::new(Dst::Null, n, Src::Null), EINVAL);
        // (NULL, valid)
        let src = make_src(&mut rng, 3, 2, Class::Ascii);
        assert_same_ret(&Case::new(Dst::Null, n, Src::External(src)), EINVAL);
        // (valid, NULL)
        let dst = make_dst(&mut rng, 8, 3, Class::Ascii);
        let o = assert_same(&Case::new(Dst::Buf(dst.clone()), n, Src::Null));
        assert_eq!(o.ret, EINVAL);
        let after = o.dst_after.unwrap();
        if n == 0 {
            assert_eq!(after, dst, "numElem == 0 short-circuits before the store");
        } else {
            assert_eq!(after[0], 0);
            assert_eq!(&after[1..], &dst[1..]);
        }
        // (valid, valid) as the control
        let src = make_src(&mut rng, 2, 2, Class::Ascii);
        let dst = make_dst(&mut rng, 8, 3, Class::Ascii);
        assert_same(&Case::new(Dst::Buf(dst), n.min(8), Src::External(src)));
    }
}

// ===========================================================================
// G2 — zero and oversized lengths
// ===========================================================================
#[test]
fn g2_length_boundaries() {
    let lengths: &[usize] = &[
        0,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - 2,
        usize::MAX - 3,
        POW62,
        POW62 + 1,
        POW62 + 2,
        POW62 - 1,
        1usize << 63,
        (1usize << 63) + 1,
        POW62 * 3,
    ];
    let mut rng = Rng::new(0x6002);
    for (i, &n) in lengths.iter().enumerate() {
        for _ in 0..32 {
            let class = ALL_CLASSES[i % ALL_CLASSES.len()];
            let alloc = rng.range(4, 24);
            // Terminated buffer.
            let k = rng.range(0, alloc - 1);
            let dst = make_dst(&mut rng, alloc, k, class);
            let sl = rng.range(0, 2);
            let src = make_src(&mut rng, sl, 2, class);
            // Every length in the list multiplies by sizeof(wchar_t) == 4 to
            // an offset that wraps into [-4, +2] elements of `dst`, so the
            // window is always inside the (>= 4 element) allocation.
            assert_same(&Case::new(Dst::Buf(dst.clone()), n, Src::External(src)));
            let g = rng.range(0, 2);
            let dstu = make_dst_unterminated(&mut rng, alloc, class);
            let srcu = make_src(&mut rng, g, 2, class);
            assert_same(&Case::new(Dst::Buf(dstu), n, Src::External(srcu)));
            // NULL dst with the same length.
            assert_same_ret(&Case::new(Dst::Null, n, Src::Null), EINVAL);
        }
    }
}

// ===========================================================================
// G3 — one step past the valid element range
// ===========================================================================
#[test]
fn g3_one_past_range() {
    let mut rng = Rng::new(0x6003);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 48);
        let alloc = n + 4;
        // terminator exactly at numElem-1, at numElem, and at numElem+1
        for k in [n - 1, n, n + 1] {
            let dst = make_dst(&mut rng, alloc, k, class);
            for len in [0usize, 1, 2] {
                let src = make_src(&mut rng, len, 2, class);
                let o = assert_same(&Case::new(
                    Dst::Buf(dst.clone()),
                    n,
                    Src::External(src),
                ));
                let expect = if k == n - 1 && len == 0 { 0 } else { ERANGE };
                assert_eq!(o.ret, expect, "k={k} n={n} len={len}");
            }
        }
    }
}

// ===========================================================================
// G4 — out-of-domain wchar_t values crossing the FFI boundary
//
// `wcscat` takes no enum parameter, so the analogous "value with no valid
// variant" class is a `wchar_t` that is not a legal Unicode scalar: negative
// values, lone surrogates, values above 0x10FFFF, and INT_MIN/INT_MAX. The C
// only ever compares against 0, so all of them must round-trip unchanged.
// ===========================================================================
#[test]
fn g4_out_of_domain_wchar_values() {
    let weird: &[i32] = &[
        -1,
        i32::MIN,
        i32::MAX,
        0x11_0000,
        0x7FFF_FFFF,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0xD800,
        0xDFFF,
        0xFFFE,
        -0x10_FFFF,
        0x110000 * -1,
    ];
    for &v in weird {
        for n in 1..10usize {
            for k in 0..n {
                // dst filled with the weird value, terminator at k
                let mut dst = vec![v; n + 3];
                dst[k] = 0;
                for srclen in 0..4usize {
                    let mut src = vec![v; srclen];
                    src.push(0);
                    src.extend_from_slice(&[v, v, v, v, v, v, v, v, v, v, v, v]);
                    let o = assert_same(&Case::new(
                        Dst::Buf(dst.clone()),
                        n,
                        Src::External(src),
                    ));
                    let expect = if k + srclen + 1 <= n { 0 } else { ERANGE };
                    assert_eq!(o.ret, expect, "v={v:#x} n={n} k={k} srclen={srclen}");
                }
            }
            // fully unterminated buffer of the weird value
            let dst = vec![v; n + 3];
            let mut src = vec![v; 4];
            src.push(0);
            let o = assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
            assert_eq!(o.ret, ERANGE);
        }
    }
    // Mixed weird values, randomized.
    let mut rng = Rng::new(0x6004);
    for _ in 0..REPS {
        let n = rng.range(1, 32);
        let alloc = n + rng.range(0, 4);
        let dst: Vec<i32> = (0..alloc)
            .map(|_| if rng.range(0, 9) == 0 { 0 } else { rng.pick(weird) })
            .collect();
        let mut src: Vec<i32> = (0..n).map(|_| rng.pick(weird)).collect();
        src.push(0);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
    }
}

// ===========================================================================
// G5 — numElem larger than the real allocation, but a NUL inside the real part
// ===========================================================================
#[test]
fn g5_numelem_beyond_allocation_but_terminated() {
    let mut rng = Rng::new(0x6005);
    for i in 0..REPS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let alloc = rng.range(8, 64);
        // numElem claims more room than exists ...
        let n = alloc + rng.range(1, 64);
        // ... but dst is terminated early and src is short enough that the
        // copy stops well inside the real allocation.
        let k = rng.range(0, alloc / 2);
        let dst = make_dst(&mut rng, alloc, k, class);
        let maxlen = alloc - k - 2;
        let len = rng.range(0, maxlen);
        let src = make_src(&mut rng, len, 2, class);
        let o = assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
        assert_eq!(o.ret, 0, "should succeed: k={k} len={len} alloc={alloc}");
    }
}
