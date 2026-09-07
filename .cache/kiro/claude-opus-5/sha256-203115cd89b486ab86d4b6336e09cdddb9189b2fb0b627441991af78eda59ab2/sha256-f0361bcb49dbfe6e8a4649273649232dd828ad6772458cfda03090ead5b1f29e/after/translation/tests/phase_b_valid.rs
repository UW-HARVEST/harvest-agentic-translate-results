//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through `libloading` with many randomized inputs (fixed seed).

mod common;
use common::*;

const ITERS: usize = 300;

/// Row 1 — numLines = 0, bufferSize = 0.
#[test]
fn cfg_01_zero_lines_zero_size() {
    let mut buf = vec![0u8; 8];
    for _ in 0..ITERS {
        let out = assert_same("cfg01", &mut buf, 0, 0);
        assert_eq!(out, Outcome::Ok(vec![]), "expected non-NULL empty array");
    }
}

/// Row 2 — numLines = 0, random bufferSize > 0, random content.
#[test]
fn cfg_02_zero_lines_nonzero_size() {
    let mut rng = Rng::new(0x0002);
    for _ in 0..ITERS {
        let n = rng.range(1, 512);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let out = assert_same("cfg02", &mut buf, 0, n);
        assert_eq!(out, Outcome::Ok(vec![]));
    }
}

/// Row 3 — numLines = 0 with buffer == NULL (never dereferenced).
#[test]
fn cfg_03_zero_lines_null_buffer() {
    let mut rng = Rng::new(0x0003);
    for _ in 0..ITERS {
        let size = rng.range(1, usize::MAX >> 4);
        let out = assert_same_raw("cfg03", std::ptr::null_mut(), 0, size);
        assert_eq!(out, Outcome::Ok(vec![]));
    }
}

/// Row 4 — numLines = 1, bufferSize = 1, buffer = "\0".
#[test]
fn cfg_04_one_line_size_one_nul() {
    let mut buf = vec![0u8];
    let out = assert_same("cfg04", &mut buf, 1, 1);
    assert_eq!(out, Outcome::Ok(vec![Some(0)]));
}

/// Row 5 — numLines = 1, bufferSize = 1, one random non-NUL byte.
#[test]
fn cfg_05_one_line_size_one_nonnul() {
    let mut rng = Rng::new(0x0005);
    for _ in 0..ITERS {
        let mut buf = vec![rng.nz_byte()];
        let out = assert_same("cfg05", &mut buf, 1, 1);
        assert_eq!(out, Outcome::Ok(vec![Some(0)]));
    }
}

/// Row 6 — numLines = 1, no NUL anywhere: `pos` lands exactly on bufferSize.
#[test]
fn cfg_06_one_line_no_nul() {
    let mut rng = Rng::new(0x0006);
    for _ in 0..ITERS {
        let n = rng.range(1, 1024);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.nz_byte()).collect();
        let out = assert_same("cfg06", &mut buf, 1, n);
        assert_eq!(out, Outcome::Ok(vec![Some(0)]));
    }
}

/// Row 7 — numLines = 1, random text with a trailing NUL at bufferSize-1.
#[test]
fn cfg_07_one_line_trailing_nul() {
    let mut rng = Rng::new(0x0007);
    for _ in 0..ITERS {
        let n = rng.range(1, 1024);
        let mut buf: Vec<u8> = (0..n - 1).map(|_| rng.nz_byte()).collect();
        buf.push(0);
        let out = assert_same("cfg07", &mut buf, 1, n);
        assert_eq!(out, Outcome::Ok(vec![Some(0)]));
    }
}

/// Row 8 — two fully terminated segments.
#[test]
fn cfg_08_two_terminated_segments() {
    let mut rng = Rng::new(0x0008);
    for _ in 0..ITERS {
        let (mut buf, offs) = build_segments(&mut rng, 2, 32, true);
        let n = buf.len();
        let out = assert_same("cfg08", &mut buf, 2, n);
        assert_eq!(
            out,
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 9 — two segments, final one unterminated.
#[test]
fn cfg_09_two_segments_unterminated_tail() {
    let mut rng = Rng::new(0x0009);
    for _ in 0..ITERS {
        let (mut buf, offs) = build_segments(&mut rng, 2, 32, false);
        let n = buf.len();
        let out = assert_same("cfg09", &mut buf, 2, n);
        assert_eq!(
            out,
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 10 — "\0\0": two consecutive zero-length lines.
#[test]
fn cfg_10_two_empty_lines() {
    let mut buf = vec![0u8, 0u8];
    let out = assert_same("cfg10", &mut buf, 2, 2);
    assert_eq!(out, Outcome::Ok(vec![Some(0), Some(1)]));
}

/// Row 11 — all-NUL buffer of size k with numLines = k.
#[test]
fn cfg_11_all_nul() {
    let mut rng = Rng::new(0x0011);
    for _ in 0..ITERS {
        let k = rng.range(1, 256);
        let mut buf = vec![0u8; k];
        let out = assert_same("cfg11", &mut buf, k, k);
        let expect: Vec<Option<isize>> = (0..k).map(|i| Some(i as isize)).collect();
        assert_eq!(out, Outcome::Ok(expect));
    }
}

/// Row 12 — many random-length terminated segments, numLines = exact count.
#[test]
fn cfg_12_many_terminated_segments() {
    let mut rng = Rng::new(0x0012);
    for _ in 0..ITERS {
        let k = rng.range(1, 512);
        let (mut buf, offs) = build_segments(&mut rng, k, 16, true);
        let n = buf.len();
        let out = assert_same("cfg12", &mut buf, k, n);
        assert_eq!(
            out,
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 13 — many segments with the last NUL omitted.
#[test]
fn cfg_13_many_segments_unterminated_tail() {
    let mut rng = Rng::new(0x0013);
    for _ in 0..ITERS {
        let k = rng.range(1, 512);
        let (mut buf, offs) = build_segments(&mut rng, k, 16, false);
        let n = buf.len();
        let out = assert_same("cfg13", &mut buf, k, n);
        assert_eq!(
            out,
            Outcome::Ok(offs.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 14 — numLines FEWER than the segments present (early loop exit).
#[test]
fn cfg_14_fewer_lines_requested() {
    let mut rng = Rng::new(0x0014);
    for _ in 0..ITERS {
        let k = rng.range(2, 128);
        let (mut buf, offs) = build_segments(&mut rng, k, 16, true);
        let want = rng.range(1, k - 1);
        let n = buf.len();
        let out = assert_same("cfg14", &mut buf, want, n);
        assert_eq!(
            out,
            Outcome::Ok(offs[..want].iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 15 — bufferSize smaller than real storage; NULs only past the window.
#[test]
fn cfg_15_truncated_window_no_visible_nul() {
    let mut rng = Rng::new(0x0015);
    for _ in 0..ITERS {
        let real = rng.range(2, 512);
        let window = rng.range(1, real - 1);
        let mut buf: Vec<u8> = (0..real).map(|_| rng.nz_byte()).collect();
        // Put NULs only beyond the window so the scan cannot see them.
        for i in window..real {
            if rng.bool() {
                buf[i] = 0;
            }
        }
        buf[real - 1] = 0;
        let out = assert_same("cfg15", &mut buf, 1, window);
        assert_eq!(out, Outcome::Ok(vec![Some(0)]));
    }
}

/// Row 16 — truncated window cutting mid-segment; numLines = segments visible.
#[test]
fn cfg_16_truncated_window_mid_segment() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..ITERS {
        let k = rng.range(2, 64);
        let (mut buf, _) = build_segments(&mut rng, k, 16, true);
        if buf.len() < 3 {
            continue;
        }
        let window = rng.range(1, buf.len() - 1);
        let expect = scan_offsets(&buf, window);
        let out = assert_same("cfg16", &mut buf, expect.len(), window);
        assert_eq!(
            out,
            Outcome::Ok(expect.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 17 — fully randomized fuzz over all three arguments and NUL densities.
/// Success and failure outcomes are both acceptable; they must AGREE.
#[test]
fn cfg_17_full_fuzz() {
    let mut rng = Rng::new(0x0017);
    let mut n_ok = 0usize;
    let mut n_null = 0usize;
    for _ in 0..4000 {
        let size = rng.below(4097);
        // NUL density class: 0 = none, 1 = sparse, 2 = medium, 3 = dense, 4 = all
        let density = rng.below(5);
        let mut buf: Vec<u8> = Vec::with_capacity(size.max(1));
        for _ in 0..size {
            let b = match density {
                0 => rng.nz_byte(),
                1 => {
                    if rng.below(16) == 0 {
                        0
                    } else {
                        rng.nz_byte()
                    }
                }
                2 => {
                    if rng.bool() {
                        0
                    } else {
                        rng.nz_byte()
                    }
                }
                3 => {
                    if rng.below(4) == 0 {
                        rng.nz_byte()
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            buf.push(b);
        }
        if buf.is_empty() {
            buf.push(0); // keep a valid allocation to hand out
        }
        let real_lines = scan_offsets(&buf, size).len();
        // Pick numLines around the true count so both branches are hit.
        let num_lines = match rng.below(4) {
            0 => real_lines,
            1 => real_lines.saturating_sub(rng.range(0, 3)),
            2 => real_lines + rng.range(1, 4),
            _ => rng.below(size + 5),
        };
        let out = assert_same("cfg17", &mut buf, num_lines, size);
        // Cross-check against the reference model of the C loop.
        let expect_offs = scan_offsets(&buf, size);
        if num_lines <= expect_offs.len() {
            assert_eq!(
                out,
                Outcome::Ok(
                    expect_offs[..num_lines]
                        .iter()
                        .map(|&o| Some(o as isize))
                        .collect()
                ),
                "model mismatch: numLines={num_lines} bufferSize={size}"
            );
            n_ok += 1;
        } else {
            assert_eq!(out, Outcome::Null, "expected NULL: numLines={num_lines} bufferSize={size}");
            n_null += 1;
        }
    }
    assert!(n_ok > 100 && n_null > 100, "fuzz did not hit both branches: ok={n_ok} null={n_null}");
}

/// Row 18 — 64 KiB buffer, exact segment count.
#[test]
fn cfg_18_large_scale() {
    let mut rng = Rng::new(0x0018);
    for _ in 0..20 {
        let mut buf: Vec<u8> = Vec::with_capacity(1 << 16);
        while buf.len() < (1 << 16) {
            let seg = rng.below(40);
            for _ in 0..seg {
                buf.push(rng.nz_byte());
            }
            buf.push(0);
        }
        let n = buf.len();
        let expect = scan_offsets(&buf, n);
        let out = assert_same("cfg18", &mut buf, expect.len(), n);
        assert_eq!(
            out,
            Outcome::Ok(expect.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 19 — interleaved empty and non-empty lines.
#[test]
fn cfg_19_interleaved_empty_lines() {
    let mut rng = Rng::new(0x0019);
    for _ in 0..ITERS {
        let k = rng.range(1, 200);
        let mut buf = Vec::new();
        for _ in 0..k {
            if rng.bool() {
                // empty line
            } else {
                for _ in 0..rng.range(1, 8) {
                    buf.push(rng.nz_byte());
                }
            }
            buf.push(0);
        }
        let n = buf.len();
        let expect = scan_offsets(&buf, n);
        assert_eq!(expect.len(), k);
        let out = assert_same("cfg19", &mut buf, k, n);
        assert_eq!(
            out,
            Outcome::Ok(expect.iter().map(|&o| Some(o as isize)).collect())
        );
    }
}

/// Row 20 — bufferSize = 1, numLines = 1, exhaustively over all 256 bytes.
#[test]
fn cfg_20_all_byte_values_size_one() {
    for b in 0u8..=255 {
        let mut buf = vec![b];
        let out = assert_same("cfg20", &mut buf, 1, 1);
        assert_eq!(out, Outcome::Ok(vec![Some(0)]), "byte value {b}");
    }
}
