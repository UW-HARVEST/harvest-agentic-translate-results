//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `premultiply` symbol and compares the whole byte buffer (plus guard bytes)
//! for byte-for-byte equality.

mod harness;

use harness::*;

const GUARD: usize = 64;
const GUARD_BYTE: u8 = 0xA5;

/// Builds a buffer of `4*px` payload bytes surrounded by `GUARD` guard bytes on
/// both sides, and returns `(bytes, pix_offset)`.
fn guarded(px: usize, rng: &mut Rng) -> (Vec<u8>, usize) {
    let mut v = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
    rng.fill(&mut v[GUARD..GUARD + 4 * px]);
    (v, GUARD)
}

/// Asserts C == Rust and that the guard regions were not touched by either.
fn check_guarded(label: &str, w: i32, h: i32, px: usize, rng: &mut Rng) {
    let (buf, off) = guarded(px, rng);
    let out = assert_same(label, w, h, &buf, off);
    assert_eq!(
        &out[..GUARD],
        &buf[..GUARD],
        "`{label}`: leading guard region was modified (w={w}, h={h})"
    );
    assert_eq!(
        &out[GUARD + 4 * px..],
        &buf[GUARD + 4 * px..],
        "`{label}`: trailing guard region was modified (w={w}, h={h})"
    );
}

// ===========================================================================
// Row 1 — single pixel, exhaustive over all 65536 (colour, alpha) byte pairs
// ===========================================================================

#[test]
fn cfg01_single_pixel_exhaustive_alpha_colour() {
    // For each of the 256*256 (colour, alpha) pairs, put the colour in r, g and
    // b simultaneously and also in three "one channel hot" arrangements so each
    // channel index is exercised independently.
    let mut probes: Vec<[u8; 4]> = Vec::with_capacity(256 * 256 * 4);
    for a in 0u16..256 {
        for c in 0u16..256 {
            let (a, c) = (a as u8, c as u8);
            probes.push([c, c, c, a]);
            probes.push([c, 0, 255, a]);
            probes.push([0, c, 255, a]);
            probes.push([255, 0, c, a]);
        }
    }
    // Feed them one pixel at a time through w=1,h=1 (the row's configuration).
    for p in &probes {
        let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4];
        buf[GUARD..GUARD + 4].copy_from_slice(p);
        let out = assert_same("cfg01 w=1,h=1 exhaustive", 1, 1, &buf, GUARD);
        assert_eq!(
            out[GUARD + 3],
            p[3],
            "cfg01: alpha byte must be preserved, pixel {p:?}"
        );
        assert_eq!(&out[..GUARD], &buf[..GUARD], "cfg01: leading guard modified");
        assert_eq!(
            &out[GUARD + 4..],
            &buf[GUARD + 4..],
            "cfg01: trailing guard modified"
        );
    }
}

// ===========================================================================
// Row 2 — single pixel, randomized
// ===========================================================================

#[test]
fn cfg02_single_pixel_random() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..4096 {
        check_guarded("cfg02 w=1,h=1 random", 1, 1, 1, &mut rng);
    }
}

// ===========================================================================
// Row 3 — 256x256 image holding every (colour, alpha) pair in one single call
// ===========================================================================

#[test]
fn cfg03_full_byte_cross_product_one_call() {
    let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * 256 * 256];
    for a in 0usize..256 {
        for c in 0usize..256 {
            let k = GUARD + 4 * (a * 256 + c);
            buf[k] = c as u8;
            buf[k + 1] = (255 - c) as u8;
            buf[k + 2] = (c ^ 0x55) as u8;
            buf[k + 3] = a as u8;
        }
    }
    let out = assert_same("cfg03 w=256,h=256 exhaustive", 256, 256, &buf, GUARD);
    // alpha preserved everywhere
    for k in 0..256 * 256 {
        assert_eq!(
            out[GUARD + 4 * k + 3],
            buf[GUARD + 4 * k + 3],
            "cfg03: alpha of pixel {k} was modified"
        );
    }
    assert_eq!(&out[..GUARD], &buf[..GUARD], "cfg03: leading guard modified");
    assert_eq!(
        &out[GUARD + 4 * 256 * 256..],
        &buf[GUARD + 4 * 256 * 256..],
        "cfg03: trailing guard modified"
    );
}

// ===========================================================================
// Row 4 — single row, many widths
// ===========================================================================

#[test]
fn cfg04_single_row_widths() {
    let mut rng = Rng::new(SEED ^ 4);
    for &w in DIM_SMALL {
        for _ in 0..16 {
            check_guarded("cfg04 w=N,h=1", w, 1, w as usize, &mut rng);
        }
    }
}

// ===========================================================================
// Row 5 — single column, many heights
// ===========================================================================

#[test]
fn cfg05_single_column_heights() {
    let mut rng = Rng::new(SEED ^ 5);
    for &h in DIM_SMALL {
        for _ in 0..16 {
            check_guarded("cfg05 w=1,h=N", 1, h, h as usize, &mut rng);
        }
    }
}

// ===========================================================================
// Row 6 — general two-dimensional grid (all MxN combinations)
// ===========================================================================

#[test]
fn cfg06_two_dimensional_grid() {
    let mut rng = Rng::new(SEED ^ 6);
    let dims: Vec<i32> = DIM_SMALL.iter().copied().filter(|&d| d <= 257).collect();
    for &w in &dims {
        for &h in &dims {
            let px = (w as usize) * (h as usize);
            if 4 * px + 2 * GUARD > MAX_BYTES {
                continue;
            }
            check_guarded("cfg06 w=M,h=N", w, h, px, &mut rng);
        }
    }
}

// ===========================================================================
// Row 7 — shape equivalence: the loop is flat over 4*w*h bytes
// ===========================================================================

#[test]
fn cfg07_shape_equivalence() {
    let mut rng = Rng::new(SEED ^ 7);
    // (w, h) sets with identical pixel counts.
    let groups: &[&[(i32, i32)]] = &[
        &[(12, 1), (1, 12), (2, 6), (6, 2), (3, 4), (4, 3)],
        &[(100, 1), (1, 100), (10, 10), (4, 25), (25, 4), (2, 50), (50, 2)],
        &[(64, 9), (9, 64), (3, 192), (192, 3), (576, 1), (1, 576)],
    ];
    for g in groups {
        let px = (g[0].0 as usize) * (g[0].1 as usize);
        for _ in 0..8 {
            let (buf, off) = guarded(px, &mut rng);
            let mut reference: Option<Vec<u8>> = None;
            for &(w, h) in g.iter() {
                assert_eq!(
                    (w as usize) * (h as usize),
                    px,
                    "cfg07: group has inconsistent pixel counts"
                );
                let out = assert_same("cfg07 shape equivalence", w, h, &buf, off);
                match &reference {
                    None => reference = Some(out),
                    Some(r) => assert_eq!(
                        &out, r,
                        "cfg07: shape {w}x{h} produced a different result than the \
                         first shape in its group (the C loop is flat over 4*w*h bytes)"
                    ),
                }
            }
        }
    }
}

// ===========================================================================
// Row 8 — large image
// ===========================================================================

#[test]
fn cfg08_large_image() {
    let mut rng = Rng::new(SEED ^ 8);
    check_guarded("cfg08 w=1024,h=1024", 1024, 1024, 1024 * 1024, &mut rng);
}

// ===========================================================================
// Row 9 — alpha pinned sweep
// ===========================================================================

#[test]
fn cfg09_alpha_pinned_sweep() {
    let mut rng = Rng::new(SEED ^ 9);
    let (w, h) = (64i32, 64i32);
    let px = (w * h) as usize;
    for &a in &[0u8, 1, 2, 63, 64, 127, 128, 129, 192, 253, 254, 255] {
        for _ in 0..4 {
            let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
            rng.fill(&mut buf[GUARD..GUARD + 4 * px]);
            for k in 0..px {
                buf[GUARD + 4 * k + 3] = a;
            }
            let out = assert_same("cfg09 alpha pinned", w, h, &buf, GUARD);
            for k in 0..px {
                assert_eq!(out[GUARD + 4 * k + 3], a, "cfg09: alpha not preserved");
            }
        }
    }
}

// ===========================================================================
// Row 10 — colour pinned sweep
// ===========================================================================

#[test]
fn cfg10_colour_pinned_sweep() {
    let mut rng = Rng::new(SEED ^ 10);
    let (w, h) = (64i32, 64i32);
    let px = (w * h) as usize;
    for &c in &[0u8, 1, 127, 128, 254, 255] {
        for _ in 0..4 {
            let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
            rng.fill(&mut buf[GUARD..GUARD + 4 * px]);
            for k in 0..px {
                buf[GUARD + 4 * k] = c;
                buf[GUARD + 4 * k + 1] = c;
                buf[GUARD + 4 * k + 2] = c;
            }
            assert_same("cfg10 colour pinned", w, h, &buf, GUARD);
        }
    }
}

// ===========================================================================
// Row 11 — alpha == 255 round-trip, exhaustive over colours
// ===========================================================================

#[test]
fn cfg11_alpha_255_roundtrip() {
    let px = 256usize;
    let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
    for c in 0..px {
        buf[GUARD + 4 * c] = c as u8;
        buf[GUARD + 4 * c + 1] = c as u8;
        buf[GUARD + 4 * c + 2] = c as u8;
        buf[GUARD + 4 * c + 3] = 255;
    }
    let out = assert_same("cfg11 alpha=255", 256, 1, &buf, GUARD);
    // The C is the oracle; this pins its exact recorded behaviour so any future
    // regression is loud. At alpha=255 the f32 round-trip `(c/255)*1.0*255` is
    // exact for every one of the 256 colour values.
    for c in 0..px {
        assert_eq!(
            out[GUARD + 4 * c],
            c as u8,
            "cfg11: alpha=255 round-trip is exact in the C for every colour"
        );
        assert_eq!(out[GUARD + 4 * c + 1], c as u8, "cfg11: g channel");
        assert_eq!(out[GUARD + 4 * c + 2], c as u8, "cfg11: b channel");
        assert_eq!(out[GUARD + 4 * c + 3], 255, "cfg11: alpha not preserved");
    }
}

// ===========================================================================
// Row 12 — alpha == 0 zeroes all colours, exhaustive over colours
// ===========================================================================

#[test]
fn cfg12_alpha_zero_zeroes_colours() {
    let px = 256usize;
    let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
    for c in 0..px {
        buf[GUARD + 4 * c] = c as u8;
        buf[GUARD + 4 * c + 1] = (255 - c) as u8;
        buf[GUARD + 4 * c + 2] = (c ^ 0x3C) as u8;
        buf[GUARD + 4 * c + 3] = 0;
    }
    let out = assert_same("cfg12 alpha=0", 1, 256, &buf, GUARD);
    for c in 0..px {
        assert_eq!(
            &out[GUARD + 4 * c..GUARD + 4 * c + 4],
            &[0, 0, 0, 0],
            "cfg12: alpha=0 must zero r,g,b (and alpha was already 0)"
        );
    }
}

// ===========================================================================
// Row 13 — degenerate / biased byte distributions
// ===========================================================================

#[test]
fn cfg13_degenerate_distributions() {
    let mut rng = Rng::new(SEED ^ 13);
    let (w, h) = (37i32, 53i32);
    let px = (w * h) as usize;
    let n = 4 * px;

    let mut patterns: Vec<Vec<u8>> = Vec::new();
    patterns.push(vec![0x00; n]);
    patterns.push(vec![0xFF; n]);
    patterns.push((0..n).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect());
    patterns.push((0..n).map(|i| (i % 3) as u8).collect());
    patterns.push((0..n).map(|i| 252u8 + (i % 4) as u8).collect());
    patterns.push((0..n).map(|i| if i % 4 == 3 { 0 } else { 0xFF }).collect());
    patterns.push((0..n).map(|i| if i % 4 == 3 { 0xFF } else { 0 }).collect());
    patterns.push((0..n).map(|i| ((i * 7) % 256) as u8).collect());
    // random but restricted to {0,1,254,255}
    patterns.push(
        (0..n)
            .map(|_| *rng.pick(&[0u8, 1u8, 254u8, 255u8]))
            .collect(),
    );

    for (idx, p) in patterns.iter().enumerate() {
        let mut buf = vec![GUARD_BYTE; GUARD * 2 + n];
        buf[GUARD..GUARD + n].copy_from_slice(p);
        let label = format!("cfg13 degenerate pattern #{idx}");
        let out = assert_same(&label, w, h, &buf, GUARD);
        assert_eq!(&out[..GUARD], &buf[..GUARD], "{label}: leading guard modified");
        assert_eq!(
            &out[GUARD + n..],
            &buf[GUARD + n..],
            "{label}: trailing guard modified"
        );
    }
}

// ===========================================================================
// Row 14 — w == 0 with every h  (end == 0 -> no-op)
// ===========================================================================

#[test]
fn cfg14_w_zero_all_h() {
    let mut rng = Rng::new(SEED ^ 14);
    for &h in &[0i32, 1, 2, 7, 1000, -1, -1000, i32::MAX, i32::MIN] {
        for _ in 0..8 {
            let (buf, off) = guarded(16, &mut rng);
            assert_eq!(c_end(0, h), 0, "cfg14 precondition: end must be 0");
            assert_noop("cfg14 w=0", 0, h, &buf, off);
        }
    }
}

// ===========================================================================
// Row 15 — h == 0 with every w  (end == 0 -> no-op)
// ===========================================================================

#[test]
fn cfg15_h_zero_all_w() {
    let mut rng = Rng::new(SEED ^ 15);
    for &w in &[0i32, 1, 2, 7, 1000, -1, -1000, i32::MAX, i32::MIN] {
        for _ in 0..8 {
            let (buf, off) = guarded(16, &mut rng);
            assert_eq!(c_end(w, 0), 0, "cfg15 precondition: end must be 0");
            assert_noop("cfg15 h=0", w, 0, &buf, off);
        }
    }
}

// ===========================================================================
// Row 16 — w > 0, h < 0  (end < 0 -> no-op)
// ===========================================================================

#[test]
fn cfg16_pos_w_neg_h() {
    let mut rng = Rng::new(SEED ^ 16);
    for &w in &[1i32, 2, 3, 17, 1000] {
        for &h in &[-1i32, -2, -17, -1000] {
            assert!(c_end(w, h) < 0, "cfg16 precondition: end must be < 0");
            for _ in 0..8 {
                let (buf, off) = guarded(16, &mut rng);
                assert_noop("cfg16 w>0,h<0", w, h, &buf, off);
            }
        }
    }
}

// ===========================================================================
// Row 17 — w < 0, h > 0  (end < 0 -> no-op)
// ===========================================================================

#[test]
fn cfg17_neg_w_pos_h() {
    let mut rng = Rng::new(SEED ^ 17);
    for &w in &[-1i32, -2, -17, -1000] {
        for &h in &[1i32, 2, 3, 17, 1000] {
            assert!(c_end(w, h) < 0, "cfg17 precondition: end must be < 0");
            for _ in 0..8 {
                let (buf, off) = guarded(16, &mut rng);
                assert_noop("cfg17 w<0,h>0", w, h, &buf, off);
            }
        }
    }
}

// ===========================================================================
// Row 18 — w < 0 AND h < 0  ->  end > 0, |w*h| pixels really processed
// ===========================================================================

#[test]
fn cfg18_neg_w_neg_h_processes() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut processed_rows = 0usize;
    for &w in &[-1i32, -2, -3, -17, -64] {
        for &h in &[-1i32, -2, -3, -17, -64] {
            let end = c_end(w, h);
            assert!(end > 0, "cfg18 precondition: end must be > 0 (w={w},h={h})");
            let px = (end / 4) as usize;
            assert_eq!(
                px,
                (w as i64 * h as i64) as usize,
                "cfg18: expected |w*h| pixels"
            );
            for _ in 0..8 {
                let (buf, off) = guarded(px, &mut rng);
                let out = assert_same("cfg18 w<0,h<0", w, h, &buf, off);
                // Confirm work really happened for at least some inputs.
                if out != buf {
                    processed_rows += 1;
                }
            }
        }
    }
    assert!(
        processed_rows > 0,
        "cfg18: both dimensions negative must still process pixels"
    );
}

// ===========================================================================
// Row 19 — 32-bit `stride` wrap matrix
// ===========================================================================

#[test]
fn cfg19_stride_wrap_matrix() {
    let mut rng = Rng::new(SEED ^ 19);
    let ws: &[i32] = &[
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        0x4000_0002,
        0x4000_0401,
        0x7FFF_FFFF,
        -0x4000_0000,
        i32::MIN,
    ];
    let hs: &[i32] = &[-2i32, -1, 0, 1, 2, 3, 1000];
    let mut skipped = Vec::new();
    let mut covered_pos = 0usize;
    for &w in ws {
        for &h in hs {
            let end = c_end(w, h);
            let px = if end > 0 { (end / 4) as usize } else { 0 };
            if 4 * px + 2 * GUARD > MAX_BYTES {
                skipped.push((w, h, end));
                continue;
            }
            if end > 0 {
                covered_pos += 1;
            }
            for _ in 0..4 {
                let (buf, off) = guarded(px.max(1), &mut rng);
                assert_same("cfg19 stride wrap", w, h, &buf, off);
            }
        }
    }
    assert!(
        skipped.is_empty(),
        "cfg19: unexpectedly skipped combinations {skipped:?}"
    );
    assert!(
        covered_pos >= 4,
        "cfg19: expected several stride-wrap combinations to yield end>0, got {covered_pos}"
    );
}

// ===========================================================================
// Row 20 — 32-bit `end` wrap matrix
// ===========================================================================

#[test]
fn cfg20_end_wrap_matrix() {
    let mut rng = Rng::new(SEED ^ 20);
    let ws: &[i32] = &[0x2000_0000, 0x1000_0000, 0x0800_0000, 3, 5, 0x4000_0001];
    let hs: &[i32] = &[2i32, 3, 4, 8, 16, 0x2000_0000, i32::MAX, i32::MIN, 1000];
    // Only these need a buffer bigger than MAX_BYTES (documented in CONFIGS.md).
    let expect_skipped: &[(i32, i32)] = &[(0x0800_0000, 2), (0x0800_0000, 3)];
    let mut skipped = Vec::new();
    for &w in ws {
        for &h in hs {
            let end = c_end(w, h);
            let px = if end > 0 { (end / 4) as usize } else { 0 };
            if 4 * px + 2 * GUARD > MAX_BYTES {
                skipped.push((w, h));
                continue;
            }
            for _ in 0..4 {
                let (buf, off) = guarded(px.max(1), &mut rng);
                assert_same("cfg20 end wrap", w, h, &buf, off);
            }
        }
    }
    assert_eq!(
        skipped, expect_skipped,
        "cfg20: skip set changed (only over-8MiB positive-end combinations may be skipped)"
    );
}

// ===========================================================================
// Row 21 — unaligned `pix`
// ===========================================================================

#[test]
fn cfg21_unaligned_pix() {
    let mut rng = Rng::new(SEED ^ 21);
    let (w, h) = (29i32, 7i32);
    let px = (w * h) as usize;
    for extra in 0usize..4 {
        for _ in 0..16 {
            let n = 4 * px;
            let mut buf = vec![GUARD_BYTE; GUARD * 2 + n + 4];
            let off = GUARD + extra;
            rng.fill(&mut buf[off..off + n]);
            let label = format!("cfg21 pix misaligned by +{extra}");
            let out = assert_same(&label, w, h, &buf, off);
            assert_eq!(&out[..off], &buf[..off], "{label}: leading guard modified");
            assert_eq!(
                &out[off + n..],
                &buf[off + n..],
                "{label}: trailing guard modified"
            );
        }
    }
}

// ===========================================================================
// Row 22 — pix == NULL for every end<=0 configuration
// ===========================================================================

#[test]
fn cfg22_null_pix_when_no_work() {
    let combos: &[(i32, i32)] = &[
        (0, 0),
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0),
        (1, -1),
        (-1, 1),
        (1000, -1000),
        (-1000, 1000),
        (i32::MAX, 1),
        (i32::MIN, 1),
        (0x4000_0000, 1000),
        (0x2000_0000, 2),
        (0x2000_0000, 3),
        (1, i32::MAX),
        (2, i32::MIN),
    ];
    for &(w, h) in combos {
        assert!(
            c_end(w, h) <= 0,
            "cfg22 precondition: end must be <= 0 for w={w},h={h}"
        );
        let mut ci = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        let mut ri = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        unsafe {
            (c_fn())(&mut ci);
            (rust_fn())(&mut ri);
        }
        assert_eq!(
            (ci.w, ci.h, ci.pix.is_null()),
            (ri.w, ri.h, ri.pix.is_null()),
            "cfg22: struct diverged for w={w},h={h} with pix=NULL"
        );
        assert_eq!(
            (ci.w, ci.h),
            (w, h),
            "cfg22: C mutated the struct for w={w},h={h}"
        );
    }
}

// ===========================================================================
// Row 23 — repeated application (composed pipeline)
// ===========================================================================

#[test]
fn cfg23_repeated_application() {
    let mut rng = Rng::new(SEED ^ 23);
    let (w, h) = (48i32, 48i32);
    let px = (w * h) as usize;
    let n = 4 * px;
    for _ in 0..16 {
        let mut buf = vec![GUARD_BYTE; GUARD * 2 + n];
        rng.fill(&mut buf[GUARD..GUARD + n]);

        let mut cb = AlignedBuf::from_bytes(&buf);
        let mut rb = AlignedBuf::from_bytes(&buf);
        for pass in 1..=3 {
            unsafe {
                let mut ci = CpImage {
                    w,
                    h,
                    pix: cb.as_mut_ptr().add(GUARD) as *mut CpPixel,
                };
                (c_fn())(&mut ci);
                let mut ri = CpImage {
                    w,
                    h,
                    pix: rb.as_mut_ptr().add(GUARD) as *mut CpPixel,
                };
                (rust_fn())(&mut ri);
            }
            assert_eq!(
                cb.as_slice(),
                rb.as_slice(),
                "cfg23: diverged after {pass} application(s)"
            );
        }
    }
}

// ===========================================================================
// Row 24 — exact touched extent
// ===========================================================================

#[test]
fn cfg24_guarded_extent() {
    let mut rng = Rng::new(SEED ^ 24);
    for &(w, h) in &[(1i32, 1i32), (7, 3), (16, 16), (33, 5), (1, 100), (100, 1)] {
        let px = (w * h) as usize;
        let n = 4 * px;
        for _ in 0..16 {
            let mut buf = vec![GUARD_BYTE; GUARD * 2 + n + GUARD];
            rng.fill(&mut buf[GUARD..GUARD + n]);
            let label = format!("cfg24 extent {w}x{h}");
            let out = assert_same(&label, w, h, &buf, GUARD);
            // Bytes before and after the 4*w*h payload must be untouched.
            assert_eq!(&out[..GUARD], &buf[..GUARD], "{label}: byte before extent changed");
            assert_eq!(
                &out[GUARD + n..],
                &buf[GUARD + n..],
                "{label}: byte at/after 4*w*h changed"
            );
            // Only channels 0,1,2 of each in-extent pixel may change.
            for k in 0..px {
                assert_eq!(
                    out[GUARD + 4 * k + 3],
                    buf[GUARD + 4 * k + 3],
                    "{label}: alpha of pixel {k} changed"
                );
            }
        }
    }
}

// ===========================================================================
// Row 25 — struct ABI: field offsets 0/4/8, sizeof == 16, struct immutable
// ===========================================================================

#[test]
fn cfg25_struct_abi_and_immutability() {
    assert_eq!(std::mem::size_of::<CpPixel>(), 4, "sizeof(cp_pixel_t)");
    assert_eq!(std::mem::align_of::<CpPixel>(), 1, "alignof(cp_pixel_t)");
    assert_eq!(std::mem::size_of::<CpImage>(), 16, "sizeof(cp_image_t)");
    assert_eq!(std::mem::align_of::<CpImage>(), 8, "alignof(cp_image_t)");

    let mut rng = Rng::new(SEED ^ 25);
    let (w, h) = (13i32, 11i32);
    let px = (w * h) as usize;

    for _ in 0..64 {
        let (buf, off) = guarded(px, &mut rng);
        let mut cb = AlignedBuf::from_bytes(&buf);
        let mut rb = AlignedBuf::from_bytes(&buf);

        // Hand each library a raw 16-byte struct built by hand at offsets 0/4/8.
        let mut c_raw = [0u8; 16];
        let mut r_raw = [0u8; 16];
        unsafe {
            let cp = cb.as_mut_ptr().add(off);
            let rp = rb.as_mut_ptr().add(off);
            c_raw[0..4].copy_from_slice(&w.to_ne_bytes());
            c_raw[4..8].copy_from_slice(&h.to_ne_bytes());
            c_raw[8..16].copy_from_slice(&(cp as usize).to_ne_bytes());
            r_raw[0..4].copy_from_slice(&w.to_ne_bytes());
            r_raw[4..8].copy_from_slice(&h.to_ne_bytes());
            r_raw[8..16].copy_from_slice(&(rp as usize).to_ne_bytes());

            let c_before = c_raw;
            let r_before = r_raw;
            (c_fn())(c_raw.as_mut_ptr() as *mut CpImage);
            (rust_fn())(r_raw.as_mut_ptr() as *mut CpImage);
            assert_eq!(
                c_raw, c_before,
                "cfg25: C mutated the cp_image_t struct bytes"
            );
            assert_eq!(
                r_raw, r_before,
                "cfg25: Rust mutated the cp_image_t struct bytes (C does not)"
            );
        }
        assert_eq!(
            cb.as_slice(),
            rb.as_slice(),
            "cfg25: pixel data diverged when driven through a hand-built struct"
        );
    }
}

// ===========================================================================
// Row 26 — broad randomized fuzz over dimensions and pixel data
// ===========================================================================

#[test]
fn cfg26_fuzz_dimensions_and_pixels() {
    let mut rng = Rng::new(SEED ^ 26);
    const BUF_PX: usize = 4096; // 16 KiB payload
    let mut ran = 0usize;
    let mut ran_with_work = 0usize;

    for _ in 0..20_000 {
        let (w, h) = if rng.below(2) == 0 {
            // "safe small" region: |w*h| <= 1600 pixels
            (
                rng.below(81) as i32 - 40,
                rng.below(81) as i32 - 40,
            )
        } else {
            (*rng.pick(DIM_SPECIAL), *rng.pick(DIM_SPECIAL))
        };

        let end = c_end(w, h);
        let px = if end > 0 { (end / 4) as usize } else { 0 };
        if px > BUF_PX {
            continue;
        }
        ran += 1;
        if px > 0 {
            ran_with_work += 1;
        }

        let n = 4 * px.max(1);
        let mut buf = vec![GUARD_BYTE; GUARD * 2 + n];
        rng.fill(&mut buf[GUARD..GUARD + n]);
        let out = assert_same("cfg26 fuzz", w, h, &buf, GUARD);
        assert_eq!(&out[..GUARD], &buf[..GUARD], "cfg26: leading guard modified");
        assert_eq!(
            &out[GUARD + n..],
            &buf[GUARD + n..],
            "cfg26: trailing guard modified"
        );
    }

    assert!(ran > 15_000, "cfg26: too many iterations skipped ({ran} ran)");
    assert!(
        ran_with_work > 1_000,
        "cfg26: too few iterations actually processed pixels ({ran_with_work})"
    );
}

// ===========================================================================
// Row 27 — mixed-sign dimensions whose 32-bit wrap makes `end` POSITIVE
//
// Discovered by the differential harness: a negative dimension is NOT always a
// no-op. When `4*w*h` wraps back into the positive `int` range the C library
// happily processes `end/4` pixels, so this is a *valid-path* configuration
// too, not merely an error row.
// ===========================================================================

#[test]
fn cfg27_mixed_sign_wrapped_positive() {
    let mut rng = Rng::new(SEED ^ 27);
    let combos: &[(i32, i32)] = &[
        (-0x3FFF_FFFF, 1),
        (-0x3FFF_FFFF, 2),
        (-0x3FFF_FFFF, 3),
        (-0x3FFF_FFFF, 7),
        (-0x3FFF_FF00, 1),
        (-0x3FFF_FF00, 2),
        (-0x3FFF_F000, 1),
        (3, -357_913_941),
        (1, -0x3FFF_FFFF),
        (2, -0x3FFF_FFFF),
        (4, -0x1FFF_FFFF),
        (16, -0x07FF_FFFF),
        (-0x3FFF_0000, 1),
        (-0x3FFF_0000, 3),
    ];
    let mut worked = 0usize;
    for &(w, h) in combos {
        let end = c_end(w, h);
        assert!(
            end > 0,
            "cfg27 precondition: end must wrap positive (w={w},h={h},end={end})"
        );
        for _ in 0..8 {
            match run_combo("cfg27 mixed sign, wrapped positive", w, h, GUARD, &mut rng) {
                None => panic!("cfg27: combination ({w},{h}) needs a buffer > MAX_BYTES"),
                Some(did) => {
                    if did {
                        worked += 1;
                    }
                }
            }
        }
    }
    assert!(
        worked > 0,
        "cfg27: expected real pixel work in the wrapped-positive combinations"
    );
}

// ===========================================================================
// Row 28 — proof by exhaustion that the float -> uint8 conversion can never
// diverge.
//
// The C stores `(uint8_t)(x * 255.0f)`, which GCC compiles to
// `cvttss2si %xmm0,%edx` + `mov %dl,(%rax)`. Rust's `as` casts saturate,
// whereas `cvttss2si` yields `INT_MIN` on overflow — the two only differ if a
// value outside `[-2^31, 2^31)` (or NaN) ever reaches the conversion.
//
// This test walks all 65 536 `(colour, alpha)` byte pairs, reproduces the exact
// IEEE-754 f32 chain, and asserts (a) the intermediate is always inside
// `[0.0, 255.0]` so no saturation path is reachable, and (b) the C `.so` agrees
// with that model on every pair. Together with row 1 (which compares C against
// Rust on the same 65 536 pairs) the arithmetic core is verified exhaustively.
// ===========================================================================

#[test]
fn cfg28_conversion_is_never_out_of_range() {
    // 65536 pixels: pixel (a*256 + c) has r=g=b=c and alpha=a.
    let px = 256 * 256usize;
    let mut buf = vec![GUARD_BYTE; GUARD * 2 + 4 * px];
    for a in 0usize..256 {
        for c in 0usize..256 {
            let k = GUARD + 4 * (a * 256 + c);
            buf[k] = c as u8;
            buf[k + 1] = c as u8;
            buf[k + 2] = c as u8;
            buf[k + 3] = a as u8;
        }
    }
    let out = assert_same("cfg28 conversion range", 256, 256, &buf, GUARD);

    let mut max_seen = f32::NEG_INFINITY;
    let mut min_seen = f32::INFINITY;
    for a in 0usize..256 {
        for c in 0usize..256 {
            let av = a as f32 / 255.0f32;
            let rv = c as f32 / 255.0f32;
            let scaled = (rv * av) * 255.0f32;
            assert!(
                scaled.is_finite() && (0.0f32..=255.0f32).contains(&scaled),
                "cfg28: intermediate {scaled} out of [0,255] for colour={c} alpha={a}; \
                 the saturating-vs-cvttss2si difference would become reachable"
            );
            max_seen = max_seen.max(scaled);
            min_seen = min_seen.min(scaled);

            let expect = scaled as u8;
            let k = GUARD + 4 * (a * 256 + c);
            for (ch, name) in [(0usize, "r"), (1, "g"), (2, "b")] {
                assert_eq!(
                    out[k + ch], expect,
                    "cfg28: channel {name} of (colour={c}, alpha={a}): the C .so \
                     produced {} but the exact IEEE f32 chain gives {expect}",
                    out[k + ch]
                );
            }
            assert_eq!(out[k + 3], a as u8, "cfg28: alpha must be preserved");
        }
    }
    assert_eq!(min_seen, 0.0f32, "cfg28: expected 0.0 to be reachable");
    assert_eq!(max_seen, 255.0f32, "cfg28: expected 255.0 to be reachable");
}
