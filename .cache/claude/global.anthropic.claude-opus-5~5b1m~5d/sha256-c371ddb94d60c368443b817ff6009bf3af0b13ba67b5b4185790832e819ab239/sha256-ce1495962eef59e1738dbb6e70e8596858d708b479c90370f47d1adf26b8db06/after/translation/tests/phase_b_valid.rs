//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row is driven with MANY randomized inputs from a fixed-seed PRNG, and
//! every call goes through the exported `wcscat` symbol of both `.so`s. The whole
//! `dst` allocation (window + guard tail) is compared bit-for-bit, plus the `src`
//! allocation, plus the return code.

mod common;

use common::*;

/// The reference model: an independent re-derivation of `c_src/src/lib.c`, used
/// to pin ABSOLUTE expectations so a row cannot pass with both sides wrong.
///
/// Only valid for `num_elem <= dst.len()` and a NUL-terminated `src`.
fn model(dst: &[WcharT], num_elem: usize, src: &[WcharT]) -> (i32, Vec<WcharT>) {
    let mut d = dst.to_vec();
    if num_elem == 0 {
        return (22, d);
    }
    let mut ptr = 0usize;
    while ptr < num_elem && d[ptr] != 0 {
        ptr += 1;
    }
    let mut s = 0usize;
    while ptr < num_elem {
        let c = src[s];
        s += 1;
        d[ptr] = c;
        ptr += 1;
        if c == 0 {
            return (0, d);
        }
    }
    d[0] = 0;
    (34, d)
}

fn check_vs_model(case: &Case) -> Outcome {
    let out = check(case);
    let dst = case.dst.as_ref().expect("model needs a real dst");
    if let Src::Buf(src) = &case.src {
        assert!(case.num_elem <= dst.len(), "model precondition");
        let (want_ret, want_dst) = model(dst, case.num_elem, src);
        assert_eq!(
            out.ret, want_ret,
            "case `{}`: return code {} but the C semantics model says {}",
            case.name, out.ret, want_ret
        );
        assert_eq!(
            out.dst.as_deref(),
            Some(&want_dst[..]),
            "case `{}`: buffer disagrees with the C semantics model",
            case.name
        );
    }
    out
}

// ===========================================================================
// Row 1 — numElem == 1, dst empty, src empty  → exact fit, ret 0
// ===========================================================================
#[test]
fn cfg01_numelem_one_empty_dst_empty_src() {
    let mut rng = Rng::new(SEED ^ 0x01);
    for _ in 0..200 {
        let alloc = rng.range(1, 12);
        let mut dst = make_dst(&mut rng, alloc, alloc, Some(0));
        dst[0] = 0;
        let out = check_vs_model(&Case::new("cfg01", dst.clone(), 1, Src::Buf(vec![0])));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
    // dst[0] non-zero in a 1-element window is the "full" shape -> row 15
    let out = check_vs_model(&Case::new(
        "cfg01_nonzero",
        vec![0x41, GUARD, GUARD],
        1,
        Src::Buf(vec![0]),
    ));
    assert_eq!(out.ret, 34);
}

// ===========================================================================
// Row 2 — numElem == 1, dst empty, src non-empty
// ===========================================================================
#[test]
fn cfg02_numelem_one_empty_dst_nonempty_src() {
    let mut rng = Rng::new(SEED ^ 0x02);
    for _ in 0..300 {
        let alloc = rng.range(1, 12);
        let mut dst = make_dst(&mut rng, alloc, alloc, Some(0));
        dst[0] = 0;
        let src = rng.rand_src_range(1, 10);
        let out = check_vs_model(&Case::new("cfg02", dst.clone(), 1, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        assert_eq!(out.dst.unwrap()[0], 0);
    }
}

// ===========================================================================
// Row 3 — numElem == 2, all dst-empty/full x src-empty/non-empty combinations
// ===========================================================================
#[test]
fn cfg03_numelem_two_matrix() {
    let dsts: [Vec<i32>; 4] = [
        vec![0, 0, GUARD, GUARD],       // empty
        vec![0x41, 0, GUARD, GUARD],    // one char, terminated at 1
        vec![0x41, 0x42, GUARD, GUARD], // full/unterminated
        vec![i32::MIN, -1, GUARD, GUARD],
    ];
    let srcs: [Vec<i32>; 4] = [
        vec![0],
        vec![0x63, 0],
        vec![0x63, 0x64, 0],
        vec![i32::MAX, i32::MIN, -7, 0],
    ];
    let mut want_rets = Vec::new();
    for d in &dsts {
        for s in &srcs {
            let out = check_vs_model(&Case::new("cfg03", d.clone(), 2, Src::Buf(s.clone())));
            let after = out.dst.unwrap();
            assert_eq!(&after[2..], &d[2..], "guard tail intact");
            want_rets.push(out.ret);
        }
    }
    println!("cfg03 return codes: {want_rets:?}");
}

// ===========================================================================
// Row 4 — empty dst, L + 1 < numElem (room to spare)
// ===========================================================================
#[test]
fn cfg04_empty_dst_room_to_spare() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..2000 {
        let num_elem = rng.range(2, 64);
        let alloc = num_elem + rng.below(6);
        let l = rng.below(num_elem - 1); // l + 1 < num_elem
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(0));
        dst[0] = 0;
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg04", dst.clone(), num_elem, Src::Buf(src.clone())));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..l], &src[..l]);
        assert_eq!(after[l], 0);
        assert_eq!(&after[l + 1..], &dst[l + 1..], "tail beyond the NUL preserved");
    }
}

// ===========================================================================
// Row 5 — empty dst, L + 1 == numElem (exact fit)
// ===========================================================================
#[test]
fn cfg05_empty_dst_exact_fit() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..1000 {
        let num_elem = rng.range(1, 64);
        let alloc = num_elem + rng.range(1, 5);
        let l = num_elem - 1;
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(0));
        dst[0] = 0;
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg05", dst.clone(), num_elem, Src::Buf(src.clone())));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..l], &src[..l]);
        assert_eq!(after[num_elem - 1], 0);
        assert_eq!(&after[num_elem..], &dst[num_elem..]);
    }
}

// ===========================================================================
// Row 6 — empty dst, L == numElem (off by one) -> 34
// ===========================================================================
#[test]
fn cfg06_empty_dst_off_by_one() {
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..1000 {
        let num_elem = rng.range(1, 64);
        let alloc = num_elem + rng.range(1, 5);
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(0));
        dst[0] = 0;
        let src = make_src(&mut rng, num_elem);
        let out = check_vs_model(&Case::new("cfg06", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
    }
}

// ===========================================================================
// Row 7 — empty dst, L >> numElem
// ===========================================================================
#[test]
fn cfg07_empty_dst_src_far_longer() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..800 {
        let num_elem = rng.range(1, 32);
        let alloc = num_elem + rng.range(1, 5);
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(0));
        dst[0] = 0;
        let l = num_elem * rng.range(2, 6) + rng.below(9);
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg07", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        assert_eq!(&out.dst.unwrap()[num_elem..], &dst[num_elem..]);
    }
}

// ===========================================================================
// Row 8 — non-empty prefix, src empty -> single NUL written at dst[k]
// ===========================================================================
#[test]
fn cfg08_prefix_empty_src() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..1000 {
        let num_elem = rng.range(2, 64);
        let alloc = num_elem + rng.below(5);
        let k = rng.range(1, num_elem - 1);
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let out = check_vs_model(&Case::new("cfg08", dst.clone(), num_elem, Src::Buf(vec![0])));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(after, dst, "writing a NUL over the existing NUL is a no-op");
    }
}

// ===========================================================================
// Row 9 — prefix k, k + L + 1 < numElem
// ===========================================================================
#[test]
fn cfg09_prefix_room_to_spare() {
    let mut rng = Rng::new(SEED ^ 0x09);
    for _ in 0..3000 {
        let num_elem = rng.range(3, 96);
        let alloc = num_elem + rng.below(6);
        let k = rng.range(1, num_elem - 2);
        let room = num_elem - k;
        let l = rng.below(room - 1); // k + l + 1 < num_elem
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg09", dst.clone(), num_elem, Src::Buf(src.clone())));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..k], &dst[..k], "existing prefix preserved");
        assert_eq!(&after[k..k + l], &src[..l]);
        assert_eq!(after[k + l], 0);
        assert_eq!(&after[k + l + 1..], &dst[k + l + 1..]);
    }
}

// ===========================================================================
// Row 10 — prefix k, k + L + 1 == numElem (exact fit)
// ===========================================================================
#[test]
fn cfg10_prefix_exact_fit() {
    let mut rng = Rng::new(SEED ^ 0x0A);
    for _ in 0..2000 {
        let num_elem = rng.range(2, 96);
        let alloc = num_elem + rng.range(1, 5);
        let k = rng.range(1, num_elem - 1);
        let l = num_elem - k - 1;
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg10", dst.clone(), num_elem, Src::Buf(src.clone())));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..k], &dst[..k]);
        assert_eq!(&after[k..k + l], &src[..l]);
        assert_eq!(after[num_elem - 1], 0);
        assert_eq!(&after[num_elem..], &dst[num_elem..]);
    }
}

// ===========================================================================
// Row 11 — prefix k, k + L == numElem (off by one) -> 34
// ===========================================================================
#[test]
fn cfg11_prefix_off_by_one() {
    let mut rng = Rng::new(SEED ^ 0x0B);
    for _ in 0..2000 {
        let num_elem = rng.range(2, 96);
        let alloc = num_elem + rng.range(1, 5);
        let k = rng.range(1, num_elem - 1);
        let l = num_elem - k;
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg11", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        assert_eq!(&out.dst.unwrap()[num_elem..], &dst[num_elem..]);
    }
}

// ===========================================================================
// Row 12 — prefix k, k + L > numElem -> 34 with partial copy
// ===========================================================================
#[test]
fn cfg12_prefix_truncate() {
    let mut rng = Rng::new(SEED ^ 0x0C);
    for _ in 0..2000 {
        let num_elem = rng.range(2, 96);
        let alloc = num_elem + rng.range(1, 5);
        let k = rng.range(1, num_elem - 1);
        let l = num_elem - k + rng.range(1, 12);
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = make_src(&mut rng, l);
        let out = check_vs_model(&Case::new("cfg12", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        assert_eq!(out.dst.unwrap()[0], 0);
    }
}

// ===========================================================================
// Row 13 — terminator exactly at numElem-1, src empty -> success
// ===========================================================================
#[test]
fn cfg13_last_slot_empty_src() {
    let mut rng = Rng::new(SEED ^ 0x0D);
    for _ in 0..800 {
        let num_elem = rng.range(1, 64);
        let alloc = num_elem + rng.below(5);
        let k = num_elem - 1;
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let out = check_vs_model(&Case::new("cfg13", dst.clone(), num_elem, Src::Buf(vec![0])));
        assert_eq!(out.ret, 0);
        assert_eq!(out.dst.unwrap(), dst);
    }
}

// ===========================================================================
// Row 14 — terminator exactly at numElem-1, src non-empty -> 34
// ===========================================================================
#[test]
fn cfg14_last_slot_nonempty_src() {
    let mut rng = Rng::new(SEED ^ 0x0E);
    for _ in 0..800 {
        let num_elem = rng.range(2, 64);
        let alloc = num_elem + rng.below(5);
        let k = num_elem - 1;
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let src = rng.rand_src_range(1, 10);
        let out = check_vs_model(&Case::new("cfg14", dst.clone(), num_elem, Src::Buf(src.clone())));
        assert_eq!(out.ret, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[num_elem - 1], src[0]);
        assert_eq!(after[0], 0);
    }
}

// ===========================================================================
// Row 15 — dst full / unterminated in the window
// ===========================================================================
#[test]
fn cfg15_full_dst() {
    let mut rng = Rng::new(SEED ^ 0x0F);
    for _ in 0..1500 {
        let num_elem = rng.range(1, 64);
        let alloc = num_elem + rng.below(5);
        let dst = make_dst(&mut rng, alloc, num_elem, None);
        let src = rng.rand_src_below(20);
        let out = check_vs_model(&Case::new("cfg15", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 16 — terminator outside the window
// ===========================================================================
#[test]
fn cfg16_nul_outside_window() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for _ in 0..800 {
        let num_elem = rng.range(1, 48);
        let alloc = num_elem + rng.range(2, 12);
        let mut dst = make_dst(&mut rng, alloc, num_elem, None);
        let nul_at = rng.range(num_elem, alloc - 1);
        dst[nul_at] = 0;
        let src = rng.rand_src_below(16);
        let out = check_vs_model(&Case::new("cfg16", dst.clone(), num_elem, Src::Buf(src)));
        assert_eq!(out.ret, 34);
        let after = out.dst.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ===========================================================================
// Row 17 — guard region after the window must be bit-identical
// ===========================================================================
#[test]
fn cfg17_no_write_past_window() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..3000 {
        let num_elem = rng.range(1, 40);
        let guard = rng.range(1, 16);
        let alloc = num_elem + guard;
        let k = if rng.bool() {
            Some(rng.below(num_elem))
        } else {
            None
        };
        let dst = make_dst(&mut rng, alloc, num_elem, k);
        let src = rng.rand_src_below(50);
        let out = check_vs_model(&Case::new("cfg17", dst.clone(), num_elem, Src::Buf(src)));
        let after = out.dst.unwrap();
        assert_eq!(
            &after[num_elem..],
            &dst[num_elem..],
            "the library must never write past dst + numElem"
        );
    }
}

// ===========================================================================
// Row 18 — huge but non-overflowing numElem
// ===========================================================================
#[test]
fn cfg18_huge_numelem_success() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for &n in &[
        1usize << 16,
        1usize << 20,
        1usize << 24,
        1usize << 32,
        1usize << 40,
        1usize << 48,
        1usize << 56,
        (1usize << 40) + 12345,
    ] {
        for _ in 0..25 {
            let alloc = rng.range(4, 40);
            let k = rng.below(alloc / 2);
            let mut dst = make_dst(&mut rng, alloc, alloc, Some(k));
            dst[k] = 0;
            // src must fit inside the real allocation so the C stays in bounds.
            let room = alloc - k - 1;
            let l = rng.below(room.max(1));
            let src = make_src(&mut rng, l);
            let out = check(&Case::new(
                format!("cfg18_{n:#x}"),
                dst.clone(),
                n,
                Src::Buf(src.clone()),
            ));
            assert_eq!(out.ret, 0, "n={n:#x} alloc={alloc} k={k} l={l}");
            let after = out.dst.unwrap();
            assert_eq!(&after[..k], &dst[..k]);
            assert_eq!(&after[k..k + l], &src[..l]);
            assert_eq!(after[k + l], 0);
            assert_eq!(&after[k + l + 1..], &dst[k + l + 1..]);
        }
    }
}

// ===========================================================================
// Row 19 — numElem where dst + numElem overflows
// ===========================================================================
#[test]
fn cfg19_overflowing_numelem() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for &n in &[usize::MAX, usize::MAX / 4, usize::MAX / 2, 1usize << 62, 1usize << 63] {
        for _ in 0..30 {
            let alloc = rng.range(2, 24);
            let mut dst = make_dst(&mut rng, alloc, alloc, Some(0));
            dst[0] = 0;
            let src = rng.rand_src_below(6);
            let out = check(&Case::new(
                format!("cfg19_{n:#x}"),
                dst.clone(),
                n,
                Src::Buf(src),
            ));
            assert_eq!(out.ret, 34, "n={n:#x}");
            let after = out.dst.unwrap();
            assert_eq!(after[0], 0);
            assert_eq!(&after[1..], &dst[1..]);
        }
    }
}

// ===========================================================================
// Row 20 — extreme / negative wchar_t payloads
// ===========================================================================
#[test]
fn cfg20_extreme_payloads() {
    const EX: [i32; 8] = [
        i32::MIN,
        -1,
        i32::MAX,
        0x41424344,
        0xD800u32 as i32,
        0x0011_0000,
        1,
        0x7FFF_FFFF,
    ];
    let mut rng = Rng::new(SEED ^ 0x14);
    for _ in 0..2000 {
        let num_elem = rng.range(2, 32);
        let alloc = num_elem + rng.range(1, 4);
        let k = rng.below(num_elem);
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        for i in 0..k {
            dst[i] = EX[rng.below(EX.len())];
        }
        let l = rng.below(num_elem + 4);
        let mut src: Vec<i32> = (0..l).map(|_| EX[rng.below(EX.len())]).collect();
        src.push(0);
        src.push(EX[rng.below(EX.len())]);
        check_vs_model(&Case::new("cfg20", dst, num_elem, Src::Buf(src)));
    }
}

// ===========================================================================
// Row 21 — src aliases dst at an offset inside the prefix
// ===========================================================================
#[test]
fn cfg21_src_aliases_dst_prefix() {
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..1500 {
        let num_elem = rng.range(2, 24);
        // Reads can reach dst + off + (num_elem - k); size the allocation so any
        // such read stays inside it.
        let alloc = 2 * num_elem + 4;
        let k = rng.range(1, num_elem - 1);
        let dst = make_dst(&mut rng, alloc, num_elem, Some(k));
        let off = rng.below(k.max(1));
        check(&Case {
            name: format!("cfg21_off{off}_k{k}_n{num_elem}"),
            dst: Some(dst),
            num_elem,
            src: Src::AliasDst(off),
        });
    }
}

// ===========================================================================
// Row 22 — src == dst exactly
// ===========================================================================
#[test]
fn cfg22_src_equals_dst() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for _ in 0..800 {
        let num_elem = rng.range(1, 24);
        let alloc = 2 * num_elem + 4;
        let k = if rng.bool() {
            Some(rng.below(num_elem))
        } else {
            None
        };
        let dst = make_dst(&mut rng, alloc, num_elem, k);
        check(&Case {
            name: format!("cfg22_n{num_elem}_k{k:?}"),
            dst: Some(dst),
            num_elem,
            src: Src::AliasDst(0),
        });
    }
}

// ===========================================================================
// Row 23 — src aliases the guard tail beyond the window
// ===========================================================================
#[test]
fn cfg23_src_aliases_tail() {
    let mut rng = Rng::new(SEED ^ 0x17);
    for _ in 0..1000 {
        let num_elem = rng.range(1, 24);
        let alloc = 2 * num_elem + 6;
        let k = if rng.bool() {
            Some(rng.below(num_elem))
        } else {
            None
        };
        let mut dst = make_dst(&mut rng, alloc, num_elem, k);
        // Terminate the aliased source region so reads stay bounded.
        let term = rng.range(num_elem, alloc - 1);
        dst[term] = 0;
        check(&Case {
            name: format!("cfg23_n{num_elem}_k{k:?}_term{term}"),
            dst: Some(dst),
            num_elem,
            src: Src::AliasDst(num_elem),
        });
    }
}

// ===========================================================================
// Row 24 — two calls in a row (the real consumer pattern)
// ===========================================================================
#[test]
fn cfg24_two_appends() {
    let mut rng = Rng::new(SEED ^ 0x18);
    for _ in 0..2000 {
        let num_elem = rng.range(1, 48);
        let alloc = num_elem + rng.range(1, 6);
        let k = if rng.below(4) == 0 {
            None
        } else {
            Some(rng.below(num_elem))
        };
        let dst = make_dst(&mut rng, alloc, num_elem, k);
        let s1 = rng.rand_src_below(20);
        let s2 = rng.rand_src_below(20);
        check_sequence(
            "cfg24",
            &dst,
            &[
                Step::new(num_elem, Src::Buf(s1)),
                Step::new(num_elem, Src::Buf(s2)),
            ],
        );
    }
}

// ===========================================================================
// Row 25 — append until saturation, checking the whole trajectory
// ===========================================================================
#[test]
fn cfg25_append_until_saturated() {
    let mut rng = Rng::new(SEED ^ 0x19);
    for _ in 0..600 {
        let num_elem = rng.range(1, 32);
        let alloc = num_elem + rng.range(1, 5);
        let mut dst = make_dst(&mut rng, alloc, num_elem, Some(0));
        dst[0] = 0;
        let steps: Vec<Step> = (0..12)
            .map(|i| {
                // Mix in NULL src and numElem == 0 steps to exercise the error
                // paths inside a live sequence.
                let src = if i == 5 {
                    Src::Null
                } else {
                    Src::Buf(rng.rand_src_range(1, 5))
                };
                let n = if i == 8 { 0 } else { num_elem };
                Step::new(n, src)
            })
            .collect();
        let rets = check_sequence("cfg25", &dst, &steps);
        assert_eq!(rets[8], 22, "the numElem==0 step must report 22");
        assert_eq!(rets[5], 22, "the NULL-src step must report 22");
    }
}

// ===========================================================================
// Row 26 — full randomized fuzz over the whole axis cross-product
// ===========================================================================
#[test]
fn cfg26_full_fuzz() {
    let mut rng = Rng::new(SEED);
    let n_cases = if std::env::var("WCSCAT_FUZZ_FAST").is_ok() {
        20_000
    } else {
        200_000
    };
    let mut ret_hist = [0usize; 3];
    for i in 0..n_cases {
        let alloc = rng.range(1, 72);
        let num_elem = rng.range(0, alloc);
        let dst = if num_elem == 0 {
            make_dst(&mut rng, alloc, 0, None)
        } else {
            let k = if rng.below(5) == 0 {
                None
            } else {
                Some(rng.below(num_elem))
            };
            make_dst(&mut rng, alloc, num_elem, k)
        };
        let src = if rng.below(16) == 0 {
            Src::Null
        } else {
            Src::Buf(rng.rand_src_below(alloc + 8))
        };
        let dst_opt = if rng.below(32) == 0 { None } else { Some(dst) };
        let case = Case {
            name: format!("cfg26_#{i}"),
            dst: dst_opt,
            num_elem,
            src,
        };
        let out = if case.dst.is_some() {
            if matches!(case.src, Src::Buf(_)) {
                check_vs_model(&case)
            } else {
                check(&case)
            }
        } else {
            check(&case)
        };
        match out.ret {
            0 => ret_hist[0] += 1,
            22 => ret_hist[1] += 1,
            34 => ret_hist[2] += 1,
            r => panic!("illegal return code {r}"),
        }
    }
    println!("cfg26: {n_cases} cases; ret 0/22/34 = {ret_hist:?}");
    assert!(ret_hist.iter().all(|&c| c > 0), "every path must be exercised");
}

// ===========================================================================
// Row 27 — fuzz with numElem > alloc, dst terminated inside the allocation
// ===========================================================================
#[test]
fn cfg27_fuzz_numelem_beyond_alloc() {
    let mut rng = Rng::new(SEED ^ 0x1B);
    for i in 0..20_000 {
        let alloc = rng.range(4, 48);
        let k = rng.below(alloc / 2);
        let mut dst = make_dst(&mut rng, alloc, alloc, Some(k));
        dst[k] = 0;
        // src must fit in [k, alloc) so the C never leaves the allocation.
        let room = alloc - k - 1;
        let l = rng.below(room.max(1));
        let src = make_src(&mut rng, l);
        let num_elem = alloc + rng.range(1, 1 << 20);
        let out = check(&Case::new(
            format!("cfg27_#{i}"),
            dst.clone(),
            num_elem,
            Src::Buf(src.clone()),
        ));
        assert_eq!(out.ret, 0);
        let after = out.dst.unwrap();
        assert_eq!(&after[..k], &dst[..k]);
        assert_eq!(&after[k..k + l], &src[..l]);
        assert_eq!(after[k + l], 0);
        assert_eq!(&after[k + l + 1..], &dst[k + l + 1..]);
    }
}

// ===========================================================================
// Row 28 — return-code domain closure over the fuzz corpus
// ===========================================================================
#[test]
fn cfg28_return_code_domain() {
    let mut rng = Rng::new(SEED ^ 0x1C);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..20_000 {
        let alloc = rng.range(1, 32);
        let num_elem = rng.range(0, alloc + 4);
        let dst = if num_elem == 0 || num_elem > alloc {
            make_dst(&mut rng, alloc, 0, None)
        } else {
            let k = if rng.below(4) == 0 { None } else { Some(rng.below(num_elem)) };
            make_dst(&mut rng, alloc, num_elem, k)
        };
        if num_elem > alloc {
            continue; // out-of-bounds for the C; covered by cfg27 shape instead
        }
        let src = if rng.below(10) == 0 {
            Src::Null
        } else {
            Src::Buf(rng.rand_src_below(alloc + 4))
        };
        seen.insert(check(&Case::new("cfg28", dst, num_elem, src)).ret);
    }
    assert_eq!(
        seen,
        [0, 22, 34].into_iter().collect::<std::collections::BTreeSet<i32>>()
    );
}
