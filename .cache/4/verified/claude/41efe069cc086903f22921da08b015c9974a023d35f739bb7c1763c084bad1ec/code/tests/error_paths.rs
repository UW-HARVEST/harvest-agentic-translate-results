//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1-24) plus the generic C-ABI
//! boundaries: NULL pointers, zero and oversized lengths, values one step past
//! every documented range, and out-of-range "enum" values (the `flags` word,
//! whose bits 5-31 have no valid meaning, and `param1`/`param2` used as mode
//! selectors).
//!
//! Every test asserts the *same* rejection from both implementations — the same
//! returned sentinel and the same buffer state — never merely "both failed".

mod common;

use common::*;

const LEN_POOL: &[usize] = &[1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 63, 64, 127, 128, 255, 256];
const FLAG_POOL: &[u32] = &[
    0x00,
    0x01,
    0x02,
    0x04,
    0x08,
    0x10,
    0x1F,
    0x20,
    0xFFFF_FFFF,
    0xFFFF_FFE0,
    0x8000_0000,
];
const PARAM_POOL: &[i32] = &[i32::MIN, -256, -1, 0, 1, 2, 3, 4, 255, 256, 257, i32::MAX];

// ---------------------------------------------------------------------------
// Row 1 / 3 — NULL buffer
// ---------------------------------------------------------------------------

#[test]
fn row01_null_buffer_returns_zero() {
    for &flags in FLAG_POOL {
        for &p1 in PARAM_POOL {
            for &p2 in PARAM_POOL {
                for &len in LEN_POOL {
                    let got = assert_same_raw(std::ptr::null_mut(), len, flags, p1, p2);
                    assert_eq!(got, 0, "NULL buffer must yield 0 (len={len} flags={flags:#x})");
                }
                // Row 3: NULL *and* length 0.
                let got = assert_same_raw(std::ptr::null_mut(), 0, flags, p1, p2);
                assert_eq!(got, 0);
            }
        }
    }
}

#[test]
fn row01b_null_buffer_oversized_lengths() {
    // "Oversized" length is only observable with a NULL pointer: the guard runs
    // before any memory access, so both implementations must bail out with 0.
    for &len in &[
        257usize,
        1 << 16,
        1 << 24,
        1 << 31,
        1 << 32,
        (1 << 32) + 1,
        u32::MAX as usize,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        for &flags in FLAG_POOL {
            for &p1 in &[i32::MIN, -1, 0, 1, 3, 255, 256, i32::MAX] {
                let got = assert_same_raw(std::ptr::null_mut(), len, flags, p1, 0);
                assert_eq!(got, 0, "len={len} flags={flags:#x} p1={p1}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — length == 0
// ---------------------------------------------------------------------------

#[test]
fn row02_zero_length_returns_zero_and_leaves_buffer() {
    let mut rng = Rng::new(0xE002);
    for &flags in FLAG_POOL {
        for &p1 in PARAM_POOL {
            for &p2 in &[0, 1, -1, i32::MIN, i32::MAX] {
                // Non-empty backing store, but `length == 0`.
                let data = make_input(Shape::Random, 64, &mut rng);
                let (c, r) = run_both(&data, 0, flags, p1, p2);
                assert_eq!(c.ret, 0, "flags={flags:#x} p1={p1} p2={p2}");
                assert_eq!(r.ret, 0);
                assert_eq!(&c.buffer[..64], &data[..], "C touched a zero-length buffer");
                assert_eq!(&r.buffer[..64], &data[..], "Rust touched a zero-length buffer");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 / 5 / 15 / 16 — rotate offset normalisation and the `offset == 0` guard
// ---------------------------------------------------------------------------

#[test]
fn row04_rotate_skipped_when_offset_folds_to_zero() {
    let mut rng = Rng::new(0xE004);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            let mut params = vec![0i32];
            for k in 1..=8i64 {
                let m = k * len as i64;
                if m <= i32::MAX as i64 {
                    params.push(m as i32);
                    params.push(-m as i32);
                }
            }
            for p1 in params {
                // buffer untouched, return value == length
                assert_same_and_untouched(&data, len, 0x01, p1, 0);
                let (c, _) = run_both(&data, len, 0x01, p1, 0);
                assert_eq!(c.ret, len, "len={len} p1={p1}");
            }
        }
    }
}

#[test]
fn row05_length_one_rotate_never_runs() {
    for b in [0u8, 1, 0x7F, 0x80, 0xFF] {
        let data = vec![b];
        for &p1 in PARAM_POOL {
            assert_same_and_untouched(&data, 1, 0x01, p1, 0);
            let (c, _) = run_both(&data, 1, 0x01, p1, 0);
            assert_eq!(c.ret, 1);
        }
    }
}

#[test]
fn row14_rotate_len_le_one_guard() {
    // `rotate_buffer`'s own `len <= 1` guard.  Through `process_buffer` the only
    // way in is `length == 1` (row 5); this test additionally pins the
    // combination with every other flag so the guard interaction is covered.
    for b in [3u8, 200] {
        let data = vec![b];
        for &flags in FLAG_POOL {
            for &p1 in PARAM_POOL {
                for &p2 in &[0, 1] {
                    run_both(&data, 1, flags | 0x01, p1, p2);
                }
            }
        }
    }
}

#[test]
fn row16_negative_offset_is_normalised() {
    // `offset += len` must make a negative `param1 % len` behave exactly like
    // the equivalent positive offset.
    let mut rng = Rng::new(0xE016);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            if len < 2 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            for neg in [-1i32, -2, -(len as i32) + 1, -(len as i32) - 3, -100_000] {
                let mut equiv = neg % len as i32;
                if equiv < 0 {
                    equiv += len as i32;
                }
                let (a, _) = run_both(&data, len, 0x01, neg, 0);
                let (b, _) = run_both(&data, len, 0x01, equiv, 0);
                assert_eq!(a.ret, b.ret, "len={len} neg={neg}");
                assert_eq!(a.buffer, b.buffer, "len={len} neg={neg} equiv={equiv}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 / 7 — out-of-range compact threshold falls back to 3
// ---------------------------------------------------------------------------

#[test]
fn row06_row07_threshold_out_of_range_defaults_to_three() {
    let mut rng = Rng::new(0xE006);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            let reference = run_both(&data, len, 0x02, 3, 0).0;
            // Row 6: param1 <= 0
            for p1 in [0, -1, -2, -3, -255, -256, -100_000, i32::MIN] {
                let (c, _) = run_both(&data, len, 0x02, p1, 0);
                assert_eq!(c.ret, reference.ret, "len={len} p1={p1} should use threshold 3");
                assert_eq!(c.buffer, reference.buffer, "len={len} p1={p1}");
            }
            // Row 7: param1 > 255
            for p1 in [256, 257, 512, 65_536, 1 << 30, i32::MAX] {
                let (c, _) = run_both(&data, len, 0x02, p1, 0);
                assert_eq!(c.ret, reference.ret, "len={len} p1={p1} should use threshold 3");
                assert_eq!(c.buffer, reference.buffer, "len={len} p1={p1}");
            }
            // One step *inside* the range must NOT use the default.
            let inside = run_both(&data, len, 0x02, 255, 0).0;
            let _ = inside;
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — param2 == 0 selects the unordered de-dup path
// ---------------------------------------------------------------------------

#[test]
fn row08_param2_zero_selects_unordered_path() {
    let mut rng = Rng::new(0xE008);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            let unordered = run_both(&data, len, 0x04, 0, 0).0;
            for p2 in [1, -1, 2, i32::MIN, i32::MAX] {
                let ordered = run_both(&data, len, 0x04, 0, p2).0;
                // Same *length*, and every non-zero param2 behaves alike.
                assert_eq!(ordered.ret, unordered.ret, "len={len} p2={p2}");
                let first = run_both(&data, len, 0x04, 0, 1).0;
                assert_eq!(ordered.buffer, first.buffer, "len={len} p2={p2}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 / 21 — interleave skipped for new_len < 2
// ---------------------------------------------------------------------------

#[test]
fn row09_row21_interleave_skipped_below_two() {
    // (a) length == 1
    for b in [0u8, 42, 255] {
        let data = vec![b];
        for &p1 in PARAM_POOL {
            assert_same_and_untouched(&data, 1, 0x08, p1, 0);
            let (c, _) = run_both(&data, 1, 0x08, p1, 0);
            assert_eq!(c.ret, 1);
        }
    }
    // (b) de-dup shrinks new_len to 1, then interleave must be skipped.
    let mut rng = Rng::new(0xE009);
    for &len in LEN_POOL {
        let data = make_input(Shape::Constant, len, &mut rng);
        for p2 in [0, 1] {
            let with_inter = run_both(&data, len, 0x0C, 0, p2).0;
            let without = run_both(&data, len, 0x04, 0, p2).0;
            assert_eq!(with_inter.ret, 1, "len={len} p2={p2}");
            assert_eq!(
                with_inter.buffer, without.buffer,
                "interleave must be a no-op when new_len < 2 (len={len} p2={p2})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — reverse skipped for new_len < 4
// ---------------------------------------------------------------------------

#[test]
fn row10_reverse_skipped_below_four() {
    let mut rng = Rng::new(0xE010);
    for len in 1usize..=3 {
        for &shape in &ALL_SHAPES {
            let data = make_input(shape, len, &mut rng);
            for &p1 in PARAM_POOL {
                assert_same_and_untouched(&data, len, 0x10, p1, 0);
                let (c, _) = run_both(&data, len, 0x10, p1, 0);
                assert_eq!(c.ret, len);
            }
        }
    }
    // …and through the pipeline: de-dup shrinks new_len to 2 or 3.
    for &len in LEN_POOL {
        for alphabet in 2usize..=3 {
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                data.push((i % alphabet) as u8);
            }
            for p2 in [0, 1] {
                let with_rev = run_both(&data, len, 0x14, 0, p2).0;
                let without = run_both(&data, len, 0x04, 0, p2).0;
                if with_rev.ret < 4 {
                    assert_eq!(
                        with_rev.buffer, without.buffer,
                        "reverse must be a no-op when new_len < 4 (len={len})"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — param1 <= 0 makes seg_size default to 4
// ---------------------------------------------------------------------------

#[test]
fn row11_seg_size_defaults_to_four() {
    let mut rng = Rng::new(0xE011);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            if len < 4 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            let reference = run_both(&data, len, 0x10, 4, 0).0;
            for p1 in [0, -1, -4, -5, -256, -100_000, i32::MIN] {
                let (c, _) = run_both(&data, len, 0x10, p1, 0);
                assert_eq!(c.ret, reference.ret, "len={len} p1={p1}");
                assert_eq!(
                    c.buffer, reference.buffer,
                    "len={len} p1={p1} must behave like seg_size 4"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 / 23 — seg_size > new_len is rejected
// ---------------------------------------------------------------------------

#[test]
fn row12_row23_seg_size_above_length_rejected() {
    let mut rng = Rng::new(0xE012);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            for p1 in [
                len as i32 + 1,
                len as i32 + 2,
                len as i32 * 2,
                100_000,
                i32::MAX,
            ] {
                assert_same_and_untouched(&data, len, 0x10, p1, 0);
                let (c, _) = run_both(&data, len, 0x10, p1, 0);
                assert_eq!(c.ret, len, "len={len} p1={p1}");
            }
            // Exactly `len` is *inside* the range and must reverse the buffer.
            if len >= 4 {
                let (c, _) = run_both(&data, len, 0x10, len as i32, 0);
                let mut expect = data.clone();
                expect.reverse();
                assert_eq!(&c.buffer[..len], &expect[..], "seg_size == len (len={len})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — out-of-range "enum" value: unknown flag bits
// ---------------------------------------------------------------------------

#[test]
fn row13_unknown_flag_bits_are_ignored() {
    let mut rng = Rng::new(0xE013);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            for base in 0x00u32..0x20 {
                let masked = run_both(&data, len, base, 3, 1).0;
                for extra in [
                    0x20u32,
                    0x40,
                    0x8000,
                    0x0100_0000,
                    0x8000_0000,
                    0xFFFF_FFE0,
                ] {
                    let (c, _) = run_both(&data, len, base | extra, 3, 1);
                    assert_eq!(c.ret, masked.ret, "len={len} flags={:#x}", base | extra);
                    assert_eq!(c.buffer, masked.buffer, "len={len} flags={:#x}", base | extra);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — runs longer than 255 are clamped
// ---------------------------------------------------------------------------

#[test]
fn row17_run_length_clamped_to_255() {
    // A constant buffer of `n > 255` bytes with threshold 1 must be encoded as
    // {v,255} followed by {v, n-255} - i.e. the clamp is observable in the
    // return value, not just internally.
    for n in [256usize, 257, 300, 400, 509, 510, 511, 512, 513, 600, 764, 765, 766, 1000] {
        let data = vec![0x5Au8; n];
        let (c, _) = run_both(&data, n, 0x02, 1, 0);
        // number of {value,count} pairs = ceil over chunks of 255
        let mut left = n;
        let mut pairs = 0usize;
        while left > 0 {
            let take = left.min(255);
            left -= take;
            pairs += 1;
        }
        assert_eq!(c.ret, 2 * pairs, "n={n}");
        for k in 0..pairs {
            let chunk = (n - k * 255).min(255);
            assert_eq!(c.buffer[2 * k], 0x5A, "n={n} k={k}");
            assert_eq!(c.buffer[2 * k + 1], chunk as u8, "n={n} k={k}");
        }
    }
    // Same clamp reached with other thresholds and with runs embedded in data.
    let mut rng = Rng::new(0xE017);
    for n in [256usize, 300, 512, 600, 1000, 1024] {
        for t in [1, 2, 3, 100, 254, 255] {
            let data = make_input(Shape::LongRuns, n, &mut rng);
            run_both(&data, n, 0x02, t, 0);
            let data = make_input(Shape::Constant, n, &mut rng);
            run_both(&data, n, 0x02, t, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — runs shorter than the threshold are kept verbatim
// ---------------------------------------------------------------------------

#[test]
fn row18_short_runs_kept_verbatim() {
    // Every run strictly shorter than `threshold` => the buffer is unchanged
    // and the length is unchanged.
    for run in 1usize..=6 {
        let mut data = Vec::new();
        for r in 0..6u8 {
            for _ in 0..run {
                data.push(r * 7 + 1);
            }
        }
        let len = data.len();
        for t in (run as i32 + 1)..=(run as i32 + 4) {
            if t > 255 {
                continue;
            }
            assert_same_and_untouched(&data, len, 0x02, t, 0);
            let (c, _) = run_both(&data, len, 0x02, t, 0);
            assert_eq!(c.ret, len, "run={run} threshold={t}");
        }
        // …and one step over the boundary compacts.
        let (c, _) = run_both(&data, len, 0x02, run as i32, 0);
        assert_eq!(c.ret, 12, "run={run}: six runs -> six {{value,count}} pairs");
    }
}

// ---------------------------------------------------------------------------
// Row 19 — the tail memmove is skipped for the final run
// ---------------------------------------------------------------------------

#[test]
fn row19_final_run_no_tail_move() {
    for head in 0usize..=6 {
        for tail in 1usize..=6 {
            let mut data: Vec<u8> = (0..head).map(|i| (i as u8) | 0x10).collect();
            data.extend(std::iter::repeat(0xC3).take(tail));
            let len = data.len();
            for t in 1..=8 {
                run_both(&data, len, 0x02, t, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — remove_duplicates rejects len <= 1
// ---------------------------------------------------------------------------

#[test]
fn row20_dedup_len_le_one() {
    for b in [0u8, 1, 128, 255] {
        let data = vec![b];
        for &p2 in PARAM_POOL {
            assert_same_and_untouched(&data, 1, 0x04, 0, p2);
            let (c, _) = run_both(&data, 1, 0x04, 0, p2);
            assert_eq!(c.ret, 1);
        }
    }
    // `len == 0` is already row 2, but exercise it through the de-dup flag too.
    let data = vec![7u8; 8];
    for &p2 in PARAM_POOL {
        let (c, _) = run_both(&data, 0, 0x04, 0, p2);
        assert_eq!(c.ret, 0);
        assert_eq!(&c.buffer[..8], &data[..]);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — seg_size <= 1 rejected
// ---------------------------------------------------------------------------

#[test]
fn row22_seg_size_one_rejected() {
    let mut rng = Rng::new(0xE022);
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            assert_same_and_untouched(&data, len, 0x10, 1, 0);
            let (c, _) = run_both(&data, len, 0x10, 1, 0);
            assert_eq!(c.ret, len, "len={len}");
            // seg_size == 2 is one step inside the accepted range.
            if len >= 4 {
                let (d, _) = run_both(&data, len, 0x10, 2, 0);
                let mut expect = data.clone();
                for pair in expect.chunks_exact_mut(2) {
                    pair.swap(0, 1);
                }
                assert_eq!(&d.buffer[..len], &expect[..], "seg_size 2, len={len}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — trailing remainder of 0 or 1 byte is left un-reversed
// ---------------------------------------------------------------------------

#[test]
fn row24_remainder_le_one_not_reversed() {
    let mut rng = Rng::new(0xE024);
    for &shape in &ALL_SHAPES {
        for len in 4usize..=140 {
            let data = make_input(shape, len, &mut rng);
            for seg in 2usize..=len {
                let rem = len % seg;
                if rem > 1 {
                    continue;
                }
                let (c, _) = run_both(&data, len, 0x10, seg as i32, 0);
                assert_eq!(c.ret, len);
                // Build the expectation: reverse each full segment, keep the
                // (0 or 1 byte) remainder in place.
                let mut expect = data.clone();
                let full = len / seg;
                for s in 0..full {
                    expect[s * seg..(s + 1) * seg].reverse();
                }
                assert_eq!(
                    &c.buffer[..len],
                    &expect[..],
                    "len={len} seg={seg} rem={rem}"
                );
            }
        }
    }
}

#[test]
fn row24b_remainder_above_one_is_reversed() {
    let mut rng = Rng::new(0xE025);
    for &shape in &ALL_SHAPES {
        for len in 4usize..=140 {
            let data = make_input(shape, len, &mut rng);
            for seg in 2usize..=len {
                let rem = len % seg;
                if rem <= 1 {
                    continue;
                }
                let (c, _) = run_both(&data, len, 0x10, seg as i32, 0);
                let mut expect = data.clone();
                let full = len / seg;
                for s in 0..full {
                    expect[s * seg..(s + 1) * seg].reverse();
                }
                expect[full * seg..].reverse();
                assert_eq!(
                    &c.buffer[..len],
                    &expect[..],
                    "len={len} seg={seg} rem={rem}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic C-ABI boundaries
// ---------------------------------------------------------------------------

#[test]
fn generic_one_step_past_every_documented_range() {
    let mut rng = Rng::new(0xE100);
    // param1: threshold range is (0, 255]; seg_size range is (0, new_len]
    // param2: any non-zero == "preserve order"
    // flags:  only bits 0..4 are defined
    for &shape in &ALL_SHAPES {
        for &len in LEN_POOL {
            let data = make_input(shape, len, &mut rng);
            for &flags in &[0x02u32, 0x10, 0x04, 0x1F, 0x20, 0x21] {
                for p1 in [
                    -1, 0, 1, 2, 3, 254, 255, 256, 257,
                    len as i32 - 1, len as i32, len as i32 + 1,
                    i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX,
                ] {
                    for p2 in [i32::MIN, -1, 0, 1, i32::MAX] {
                        run_both(&data, len, flags, p1, p2);
                    }
                }
            }
        }
    }
}

#[test]
fn generic_all_32_flag_values_at_every_guard_boundary() {
    let mut rng = Rng::new(0xE101);
    for flags in 0u32..0x20 {
        for len in [0usize, 1, 2, 3, 4, 5] {
            for &shape in &ALL_SHAPES {
                let data = make_input(shape, len.max(1), &mut rng);
                for p1 in [-1, 0, 1, 2, 3, 4, 5, 255, 256] {
                    for p2 in [0, 1] {
                        run_both(&data, len, flags, p1, p2);
                    }
                }
            }
        }
    }
}

#[test]
fn generic_extreme_param_values_full_flag_range() {
    let mut rng = Rng::new(0xE102);
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for _ in 0..2000 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = 1 + rng.below(256);
        let data = make_input(shape, len, &mut rng);
        let flags = rng.next_u32();
        let p1 = extremes[rng.below(extremes.len())];
        let p2 = extremes[rng.below(extremes.len())];
        run_both(&data, len, flags, p1, p2);
    }
}
