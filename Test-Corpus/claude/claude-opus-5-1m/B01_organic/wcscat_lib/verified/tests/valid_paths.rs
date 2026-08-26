//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! (fixed seed). Both implementations are invoked exclusively through the
//! `wcscat` symbol exported by their respective shared objects.

mod common;

use common::*;

const ITERS: usize = 400;

// ---------------------------------------------------------------------------
// C1 — k = 0, empty src
// ---------------------------------------------------------------------------
#[test]
fn c1_empty_dst_empty_src() {
    let mut rng = Rng::new(0xC001);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let dst = make_dst(&mut rng, n, 0, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, 0, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C2 — k = 0, src fits with slack
// ---------------------------------------------------------------------------
#[test]
fn c2_empty_dst_src_fits() {
    let mut rng = Rng::new(0xC002);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 64);
        let len = rng.range(1, n - 2); // len + 1 < n  => slack
        let dst = make_dst(&mut rng, n, 0, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C3 — k = 0, src fits exactly
// ---------------------------------------------------------------------------
#[test]
fn c3_empty_dst_src_exact_fit() {
    let mut rng = Rng::new(0xC003);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let len = n - 1; // len + 1 == n
        let dst = make_dst(&mut rng, n, 0, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C4 — k = 0, src overflows by exactly one
// ---------------------------------------------------------------------------
#[test]
fn c4_empty_dst_src_over_by_one() {
    let mut rng = Rng::new(0xC004);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let len = n; // needs n + 1 slots
        let dst = make_dst(&mut rng, n, 0, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C5 — k = 0, src overflows by many
// ---------------------------------------------------------------------------
#[test]
fn c5_empty_dst_src_over_by_many() {
    let mut rng = Rng::new(0xC005);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let len = n * rng.range(2, 5) + rng.range(1, 9);
        let dst = make_dst(&mut rng, n, 0, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C6 — 0 < k < n-1, empty src
// ---------------------------------------------------------------------------
#[test]
fn c6_midstring_empty_src() {
    let mut rng = Rng::new(0xC006);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 64);
        let k = rng.range(1, n - 2);
        let dst = make_dst(&mut rng, n, k, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, 0, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C7 — 0 < k < n-1, src fits with slack
// ---------------------------------------------------------------------------
#[test]
fn c7_midstring_src_fits() {
    let mut rng = Rng::new(0xC007);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(4, 96);
        let k = rng.range(1, n - 3);
        let len = rng.range(1, n - k - 2); // k + len + 1 < n
        let dst = make_dst(&mut rng, n, k, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C8 — 0 < k < n-1, src fits exactly
// ---------------------------------------------------------------------------
#[test]
fn c8_midstring_src_exact_fit() {
    let mut rng = Rng::new(0xC008);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 96);
        let k = rng.range(1, n - 2);
        let len = n - k - 1; // k + len + 1 == n
        let dst = make_dst(&mut rng, n, k, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C9 — 0 < k < n-1, src overflows by exactly one
// ---------------------------------------------------------------------------
#[test]
fn c9_midstring_over_by_one() {
    let mut rng = Rng::new(0xC009);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 96);
        let k = rng.range(1, n - 2);
        let len = n - k; // needs one more slot than available
        let dst = make_dst(&mut rng, n, k, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C10 — 0 < k < n-1, src overflows by many
// ---------------------------------------------------------------------------
#[test]
fn c10_midstring_over_by_many() {
    let mut rng = Rng::new(0xC00A);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 96);
        let k = rng.range(1, n - 2);
        let len = n * 2 + rng.range(0, 8);
        let dst = make_dst(&mut rng, n, k, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C11 — k == n-1 (only the terminator slot free), empty src
// ---------------------------------------------------------------------------
#[test]
fn c11_k_last_slot_empty_src() {
    let mut rng = Rng::new(0xC00B);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let dst = make_dst(&mut rng, n, n - 1, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, 0, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 0);
    }
}

// ---------------------------------------------------------------------------
// C12 — k == n-1, non-empty src
// ---------------------------------------------------------------------------
#[test]
fn c12_k_last_slot_nonempty_src() {
    let mut rng = Rng::new(0xC00C);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let len = rng.range(1, 8);
        let dst = make_dst(&mut rng, n, n - 1, class);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), n, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C13 — no NUL in dst[0..n]: src must not be read at all
// ---------------------------------------------------------------------------
#[test]
fn c13_unterminated_window() {
    let mut rng = Rng::new(0xC00D);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let dst = make_dst_unterminated(&mut rng, n, class);
        let src_len = rng.range(0, 8);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, src_len, _g, class);
        let o = assert_same(&Case::new(Dst::Buf(dst.clone()), n, Src::External(src)));
        assert_eq!(o.ret, 34);
        // Only dst[0] is cleared; the rest of the window is untouched.
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..]);
    }
}

// ---------------------------------------------------------------------------
// C14 — numElem == 1, all shapes
// ---------------------------------------------------------------------------
#[test]
fn c14_numelem_one_all_shapes() {
    let mut rng = Rng::new(0xC00E);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        // shape 1: dst[0] == 0, empty src -> success
        let dst = vec![0, any(&mut rng, class)];
        let src = make_src(&mut rng, 0, 2, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), 1, Src::External(src)), 0);

        // shape 2: dst[0] == 0, non-empty src -> no room for terminator
        let dst = vec![0, any(&mut rng, class)];
        let _len = rng.range(1, 5);
        let src = make_src(&mut rng, _len, 2, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), 1, Src::External(src)), 34);

        // shape 3: dst[0] != 0 -> window has no terminator
        let dst = vec![nonzero(&mut rng, class), any(&mut rng, class)];
        let _len = rng.range(0, 5);
        let src = make_src(&mut rng, _len, 2, class);
        assert_same_ret(&Case::new(Dst::Buf(dst), 1, Src::External(src)), 34);
    }
}

// ---------------------------------------------------------------------------
// C15 — numElem == 2, full cross of k x src length
// ---------------------------------------------------------------------------
#[test]
fn c15_numelem_two_full_cross() {
    let mut rng = Rng::new(0xC00F);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        for k in [Some(0usize), Some(1usize), None] {
            for len in 0..4usize {
                let dst = match k {
                    Some(k) => make_dst(&mut rng, 2, k, class),
                    None => make_dst_unterminated(&mut rng, 2, class),
                };
                let src = make_src(&mut rng, len, 3, class);
                assert_same(&Case::new(Dst::Buf(dst), 2, Src::External(src)));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — large buffers, random k and fit class
// ---------------------------------------------------------------------------
#[test]
fn c16_large_buffers() {
    let mut rng = Rng::new(0xC010);
    for i in 0..120 {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(256, 4096);
        let k = rng.range(0, n - 1);
        let dst = make_dst(&mut rng, n, k, class);
        // Cover every fit class relative to the free room n-k.
        let room = n - k;
        let len = match rng.range(0, 4) {
            0 => 0,
            1 => rng.range(1, room.max(2) - 1),
            2 => room - 1,           // exact fit
            3 => room,               // over by one
            _ => room + rng.range(1, 512),
        };
        let _g = rng.range(0, 8);
        let src = make_src(&mut rng, len, _g, class);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
    }
}

// ---------------------------------------------------------------------------
// C17 — numElem < allocation, terminator inside the window
// ---------------------------------------------------------------------------
#[test]
fn c17_window_shorter_than_alloc() {
    let mut rng = Rng::new(0xC011);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let extra = rng.range(1, 32);
        let alloc = n + extra;
        let k = rng.range(0, n - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        let room = n - k;
        let len = match rng.range(0, 3) {
            0 => 0,
            1 => room - 1, // exact fit
            2 => room,     // over by one
            _ => room + rng.range(1, 16),
        };
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        let o = assert_same(&Case::new(Dst::Buf(dst.clone()), n, Src::External(src)));
        // Nothing beyond the window may be touched.
        let after = o.dst_after.unwrap();
        assert_eq!(&after[n..], &dst[n..], "wrote past numElem");
    }
}

// ---------------------------------------------------------------------------
// C18 — numElem < allocation, terminator outside the window
// ---------------------------------------------------------------------------
#[test]
fn c18_window_excludes_terminator() {
    let mut rng = Rng::new(0xC012);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let extra = rng.range(1, 32);
        let alloc = n + extra;
        let k = rng.range(n, alloc - 1); // terminator at or beyond the window
        let dst = make_dst(&mut rng, alloc, k, class);
        let _len = rng.range(0, 8);
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, _len, _g, class);
        let o = assert_same(&Case::new(Dst::Buf(dst.clone()), n, Src::External(src)));
        assert_eq!(o.ret, 34);
        let after = o.dst_after.unwrap();
        assert_eq!(after[0], 0);
        assert_eq!(&after[1..], &dst[1..], "only dst[0] should change");
    }
}

// ---------------------------------------------------------------------------
// C19 / C20 / C21 — payload value classes across every fit class
// ---------------------------------------------------------------------------
fn payload_sweep(seed: u64, class: Class) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let n = rng.range(1, 64);
        let k = rng.range(0, n - 1);
        let room = n - k;
        let dst = make_dst(&mut rng, n, k, class);
        let len = match rng.range(0, 4) {
            0 => 0,
            1 if room >= 3 => rng.range(1, room - 2),
            2 => room - 1,
            3 => room,
            _ => room + rng.range(1, 16),
        };
        let _g = rng.range(0, 4);
        let src = make_src(&mut rng, len, _g, class);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
    }
}

#[test]
fn c19_wide_codepoints() {
    payload_sweep(0xC013, Class::Wide);
}

#[test]
fn c20_negative_wchars() {
    payload_sweep(0xC014, Class::Negative);
    // Explicit hand-picked negatives that must not be treated as terminators.
    let mut rng = Rng::new(0xC014_2);
    for v in [-1i32, i32::MIN, -0x10_FFFF, 0x8000_0000u32 as i32] {
        for n in 1..8usize {
            let mut dst = vec![v; n + 2];
            dst[n / 2] = 0;
            let src = vec![v, v, 0, v];
            assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
            let _ = rng.next_u64();
        }
    }
}

#[test]
fn c21_extreme_wchars() {
    payload_sweep(0xC015, Class::Extreme);
}

// ---------------------------------------------------------------------------
// C22 — guaranteed non-zero garbage after both terminators
// ---------------------------------------------------------------------------
#[test]
fn c22_garbage_after_terminators() {
    let mut rng = Rng::new(0xC016);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 64);
        let k = rng.range(0, n - 2);
        // dst: string of length k, NUL, then strictly non-zero garbage.
        let mut dst: Vec<i32> = (0..k).map(|_| nonzero(&mut rng, class)).collect();
        dst.push(0);
        while dst.len() < n {
            dst.push(nonzero(&mut rng, class));
        }
        // src: string, NUL, then strictly non-zero garbage.
        let len = rng.range(0, n);
        let mut src: Vec<i32> = (0..len).map(|_| nonzero(&mut rng, class)).collect();
        src.push(0);
        for _ in 0..(n + 4) {
            src.push(nonzero(&mut rng, class));
        }
        assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
    }
}

// ---------------------------------------------------------------------------
// C23 / C24 / C25 — aliasing between src and dst
// ---------------------------------------------------------------------------
#[test]
fn c23_alias_src_eq_dst() {
    let mut rng = Rng::new(0xC017);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 48);
        let alloc = 2 * n + 1;
        let k = rng.range(0, n - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::AliasDst(0)));
    }
}

#[test]
fn c24_alias_src_before_k() {
    let mut rng = Rng::new(0xC018);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(2, 48);
        let alloc = 2 * n + 1;
        let k = rng.range(1, n - 1);
        let j = rng.range(0, k - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::AliasDst(j)));
    }
}

#[test]
fn c25_alias_src_after_k() {
    let mut rng = Rng::new(0xC019);
    for i in 0..ITERS {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(3, 48);
        let alloc = 2 * n + 1;
        let k = rng.range(0, n - 2);
        let j = rng.range(k + 1, n - 1);
        let dst = make_dst(&mut rng, alloc, k, class);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::AliasDst(j)));
    }
}

// ---------------------------------------------------------------------------
// C26 — repeated appends on the same buffer (stateful pipeline)
// ---------------------------------------------------------------------------
#[test]
fn c26_repeated_appends() {
    let l = libs();
    let mut rng = Rng::new(0xC01A);
    for i in 0..120 {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 40);
        let mut c_buf = make_dst(&mut rng, n, 0, class);
        let mut r_buf = c_buf.clone();
        for _ in 0..rng.range(2, 10) {
            let _len = rng.range(0, 6);
            let _g = rng.range(0, 4);
            let src = make_src(&mut rng, _len, _g, class);
            let mut c_src = src.clone();
            let mut r_src = src.clone();
            let c_ret = unsafe {
                (l.c.wcscat)(c_buf.as_mut_ptr(), n, c_src.as_ptr())
            };
            let r_ret = unsafe {
                (l.rust.wcscat)(r_buf.as_mut_ptr(), n, r_src.as_ptr())
            };
            let _ = (&mut c_src, &mut r_src);
            assert_eq!(c_ret, r_ret, "return code diverged (n={n})");
            assert_eq!(c_buf, r_buf, "buffer diverged (n={n}, ret={c_ret})");
            assert_eq!(c_src, r_src, "src was modified differently");
        }
    }
}

// ---------------------------------------------------------------------------
// C27 — unconstrained fuzz
// ---------------------------------------------------------------------------
#[test]
fn c27_unconstrained_fuzz() {
    let mut rng = Rng::new(0xC01B);
    for i in 0..(ITERS * 6) {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        let n = rng.range(1, 64);
        let alloc = n + rng.range(0, 8);
        // Completely random buffer: k and fit class fall out of the data.
        let dst: Vec<i32> = (0..alloc).map(|_| any(&mut rng, class)).collect();
        // src must always be long enough that the C never reads past it:
        // at most `n` elements can be consumed.
        let mut src: Vec<i32> = (0..n).map(|_| any(&mut rng, class)).collect();
        src.push(0);
        assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
    }
}

// ---------------------------------------------------------------------------
// C28 — numElem values whose byte size wraps 2^64
// ---------------------------------------------------------------------------
#[test]
fn c28_wrapping_numelem() {
    const POW62: usize = 1usize << 62;
    let wrapping: &[usize] = &[
        POW62,           // *4 == 0 (mod 2^64)  -> end == dst
        POW62 + 1,       // -> end == dst + 1   (behaves like numElem == 1)
        POW62 + 2,       // -> end == dst + 2
        1usize << 63,    // *4 == 0 (mod 2^64)  -> end == dst
        usize::MAX,      // -> end == dst - 1
        usize::MAX - 1,  // -> end == dst - 2
        usize::MAX - 3,
        (1usize << 62) * 3, // *4 == 0 (mod 2^64)
    ];
    let mut rng = Rng::new(0xC01C);
    for (i, &n) in wrapping.iter().enumerate() {
        for _ in 0..32 {
            let class = ALL_CLASSES[i % ALL_CLASSES.len()];
            // >= 4 elements so the `end == dst + 2` variants stay in bounds.
            let alloc = rng.range(4, 16);
            // terminated buffer
            let k = rng.range(0, alloc - 1);
            let dst = make_dst(&mut rng, alloc, k, class);
            let _len = rng.range(0, 3);
            let src = make_src(&mut rng, _len, 2, class);
            assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
            // unterminated buffer
            let dst = make_dst_unterminated(&mut rng, alloc, class);
            let _len = rng.range(0, 3);
            let src = make_src(&mut rng, _len, 2, class);
            assert_same(&Case::new(Dst::Buf(dst), n, Src::External(src)));
        }
    }
}
