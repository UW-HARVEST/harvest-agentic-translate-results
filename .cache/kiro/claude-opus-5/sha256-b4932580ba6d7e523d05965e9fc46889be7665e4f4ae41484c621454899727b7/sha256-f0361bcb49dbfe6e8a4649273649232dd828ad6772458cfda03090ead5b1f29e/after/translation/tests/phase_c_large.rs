//! Large-trip-count differential tests.
//!
//! `phase_c_errors::generic_every_dimension_boundary_and_one_past` defers the
//! 172 boundary combinations whose wrapped loop bound exceeds `INPROC_MAX_PX`
//! (1 Mi pixels). Those combinations collapse to 16 DISTINCT trip counts, and
//! this file covers every one of them exactly — no combination is skipped, only
//! deduplicated by the observable (the number of pixels processed).
//!
//! The biggest case walks 536,870,911 pixels (~2.1 GiB per arena, ~4.3 GiB for
//! the pair), so these tests are deliberately separated from the fast suite.
//! Run with `--test-threads=1` to bound peak memory.

mod common;

use common::*;

/// Deterministic byte pattern, recomputable from the index so a second copy of a
/// multi-gigabyte buffer never has to be kept around.
fn pattern(i: usize) -> u8 {
    ((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as u8
}

const PAD: usize = 64;

/// Differential run tuned for very large spans: two arenas, pattern-filled,
/// compared byte-for-byte, with poison guards on both sides.
fn diff_large(pair: &Pair, w: i32, h: i32) {
    let iters = expected_iterations(w, h) as usize;
    assert!(iters > 0, "diff_large expects a running loop for {w}x{h}");
    // Four extra pixels of live data so an overrun past the wrapped end is
    // visible rather than immediately faulting.
    let live = (iters + 4) * PIXEL_SIZE;
    let total = PAD + live + PAD;

    let mut c_buf: Vec<u8> = vec![POISON; total];
    for i in 0..live {
        c_buf[PAD + i] = pattern(i);
    }
    let mut r_buf = c_buf.clone();

    let mut c_img = CpImage {
        w,
        h,
        pix: unsafe { c_buf.as_mut_ptr().add(PAD) } as *mut CpPixel,
    };
    let mut r_img = CpImage {
        w,
        h,
        pix: unsafe { r_buf.as_mut_ptr().add(PAD) } as *mut CpPixel,
    };

    unsafe {
        pair.c.premultiply(&mut c_img);
        pair.rust.premultiply(&mut r_img);
    }

    if c_buf != r_buf {
        let first = c_buf
            .iter()
            .zip(r_buf.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "DIVERGENCE w={w} h={h} iters={iters}: first differing byte at arena \
             index {first} (pix at {PAD}); C={:#04x} Rust={:#04x}",
            c_buf[first], r_buf[first]
        );
    }

    // Guards intact on both sides.
    assert!(
        c_buf[..PAD].iter().all(|&b| b == POISON) && c_buf[PAD + live..].iter().all(|&b| b == POISON),
        "guard clobbered for {w}x{h}"
    );

    // The processed span is exactly `iters` pixels; the 4 trailing pixels keep
    // their original pattern bytes.
    let span = iters * PIXEL_SIZE;
    for i in span..live {
        assert_eq!(
            c_buf[PAD + i],
            pattern(i),
            "wrote past the wrapped end for {w}x{h} at live byte {i}"
        );
    }

    // Spot-check the transform itself at the two ends of the span against the
    // reference model (a full per-pixel check over 2 GiB would dominate runtime).
    for &i in &[0usize, 1, 2, iters / 2, iters - 2, iters - 1] {
        if i >= iters {
            continue;
        }
        let b = i * PIXEL_SIZE;
        let input = [pattern(b), pattern(b + 1), pattern(b + 2), pattern(b + 3)];
        let got = [
            c_buf[PAD + b],
            c_buf[PAD + b + 1],
            c_buf[PAD + b + 2],
            c_buf[PAD + b + 3],
        ];
        assert_eq!(got, model_pixel(input), "pixel {i} of {w}x{h}");
    }
}

/// The distinct trip counts > `INPROC_MAX_PX` produced by the boundary sweep,
/// recomputed here from the same value set so the two tests cannot drift apart.
fn deferred_cases() -> Vec<(u32, i32, i32)> {
    let values = boundary_values();
    let mut seen: std::collections::BTreeMap<u32, (i32, i32)> = std::collections::BTreeMap::new();
    let mut deferred = 0usize;
    for &w in &values {
        for &h in &values {
            let iters = expected_iterations(w, h);
            if iters as usize > INPROC_MAX_PX {
                deferred += 1;
                seen.entry(iters).or_insert((w, h));
            }
        }
    }
    assert_eq!(
        deferred, 172,
        "deferred combination count must match the sweep's accounting"
    );
    seen.into_iter().map(|(i, (w, h))| (i, w, h)).collect()
}

/// Every deferred combination whose span is around 1 Mi pixels (cheap).
#[test]
fn deferred_small_end() {
    let pair = load_pair();
    let cases = deferred_cases();
    assert_eq!(cases.len(), 16, "distinct deferred trip counts");
    let mut ran = 0;
    for (iters, w, h) in cases {
        if iters as usize > 4 << 20 {
            continue;
        }
        diff_large(&pair, w, h);
        ran += 1;
    }
    assert_eq!(ran, 6, "expected 6 cases near the 1 Mi boundary");
}

/// Every deferred combination with a multi-gigabyte span. Ten cases, ~2.1 GiB
/// of live span each; the whole test walks ~21 GiB through both libraries.
#[test]
fn deferred_gigabyte_spans() {
    let pair = load_pair();
    let cases = deferred_cases();
    let mut ran = 0;
    for (iters, w, h) in cases {
        if (iters as usize) <= 4 << 20 {
            continue;
        }
        diff_large(&pair, w, h);
        ran += 1;
    }
    assert_eq!(ran, 10, "expected 10 gigabyte-span cases");
}

/// `stride * h` overflowing to a LARGE positive bound (the case that made the
/// original `ERRORS.md` row-21 example wrong): 100000 x 100000 does not wrap
/// negative, it wraps to 1345294336 => 336323584 pixels processed.
#[test]
fn overflow_to_large_positive_bound() {
    let pair = load_pair();
    assert_eq!(expected_iterations(100_000, 100_000), 336_323_584);
    diff_large(&pair, 100_000, 100_000);
    // A second, unrelated large-positive wrap.
    assert_eq!(expected_iterations(20_000, 60_000), 126_258_176);
    diff_large(&pair, 20_000, 60_000);
}
