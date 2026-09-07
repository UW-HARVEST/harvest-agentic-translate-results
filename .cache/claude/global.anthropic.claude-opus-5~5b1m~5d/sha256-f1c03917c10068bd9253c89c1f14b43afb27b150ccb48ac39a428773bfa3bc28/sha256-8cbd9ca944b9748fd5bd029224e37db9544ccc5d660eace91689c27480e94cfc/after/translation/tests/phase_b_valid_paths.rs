//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driving BOTH the C `.so` and the Rust
//! `.so` through their exported symbols and comparing return value, stdout and
//! stderr byte-for-byte.  Randomized rows use a fixed seed.

mod common;

use common::*;
use std::path::Path;

const SEED: u64 = 0x0000_C0DE_60_1_0;

// ===========================================================================
// Axis A — forward_goto_example (lowest level, int -> int)
// ===========================================================================

/// A1: x == 0, the boundary of `if (x < 0)`.
#[test]
fn a1_forward_zero() {
    diff_forward("A1", 0);
}

/// A2: small positive values.
#[test]
fn a2_forward_small_positive() {
    for x in 1..=64i32 {
        diff_forward("A2", x);
    }
    let mut rng = Rng::new(SEED ^ 0xA2);
    for _ in 0..200 {
        diff_forward("A2/rand", rng.range_i32(1, 1000));
    }
}

/// A3: large positive values that do not overflow `x * 2`.
#[test]
fn a3_forward_large_no_overflow() {
    let mut rng = Rng::new(SEED ^ 0xA3);
    for _ in 0..500 {
        diff_forward("A3", rng.range_i32(1, i32::MAX / 2));
    }
}

/// A4: x == INT_MAX/2, the largest x whose `x * 2` still fits.
#[test]
fn a4_forward_overflow_boundary() {
    for x in [
        i32::MAX / 2 - 1,
        i32::MAX / 2,     // 1073741823 -> 2147483646
        i32::MAX / 2 + 1, // 1073741824 -> overflow
    ] {
        diff_forward("A4", x);
    }
}

/// A5: x * 2 overflows (UB in C — the Rust must reproduce the codegen).
#[test]
fn a5_forward_overflowing() {
    for x in [
        i32::MAX / 2 + 1,
        i32::MAX / 2 + 2,
        1_500_000_000,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_forward("A5", x);
    }
    let mut rng = Rng::new(SEED ^ 0xA5);
    for _ in 0..500 {
        diff_forward("A5/rand", rng.range_i32(i32::MAX / 2 + 1, i32::MAX));
    }
}

/// A6: full-range randomized sweep (mixes the `x < 0` error branch in).
#[test]
fn a6_forward_full_range_random() {
    let mut rng = Rng::new(SEED ^ 0xA6);
    for _ in 0..3000 {
        diff_forward("A6", rng.i32_any());
    }
    // plus every power-of-two-ish landmark, both signs
    for shift in 0..31 {
        let v = 1i32 << shift;
        diff_forward("A6/pow", v);
        diff_forward("A6/pow", -v);
        diff_forward("A6/pow", v - 1);
        diff_forward("A6/pow", v.wrapping_neg().wrapping_add(1));
    }
    for x in [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX] {
        diff_forward("A6/edge", x);
    }
}

// ===========================================================================
// Axis B — open_with_cleanup (lowest level, const char* -> FILE*)
// ===========================================================================

/// B1: existing but empty file — loop body never executes, success.
#[test]
fn b1_open_empty_file() {
    let f = Fixture::file("b1", b"");
    diff_open("B1", Some(f.path()));
}

/// B2: single short line terminated with '\n'.
#[test]
fn b2_open_single_line_with_newline() {
    let f = Fixture::file("b2", b"hello world\n");
    diff_open("B2", Some(f.path()));
}

/// B3: single short line with NO trailing newline.
#[test]
fn b3_open_single_line_no_newline() {
    let f = Fixture::file("b3", b"no trailing newline here");
    diff_open("B3", Some(f.path()));
    // one byte, no newline
    let g = Fixture::file("b3b", b"x");
    diff_open("B3/1byte", Some(g.path()));
}

/// B4: many short lines, randomized counts and lengths.
#[test]
fn b4_open_many_lines() {
    let mut rng = Rng::new(SEED ^ 0xB4);
    for iter in 0..60 {
        let nlines = 2 + rng.below(49) as usize;
        let mut content = Vec::new();
        for _ in 0..nlines {
            let len = rng.below(40) as usize;
            for _ in 0..len {
                // printable, non-newline
                content.push(b'!' + (rng.below(93) as u8));
            }
            content.push(b'\n');
        }
        let f = Fixture::file(&format!("b4_{iter}"), &content);
        diff_open("B4", Some(f.path()));
    }
}

/// B5/B6/B7: the `sizeof(buffer) == 100` chunking boundary.
#[test]
fn b5_b6_b7_open_buffer_boundary_lengths() {
    for len in [
        0usize, 1, 2, 96, 97, 98, // 98 + '\n' == 99 chars, fits one fgets chunk
        99,  // 99 + '\n' == 100 chars, the '\n' spills into the next chunk
        100, 101, 102, 197, 198, 199, 200, 201, 297, 298, 299, 300,
    ] {
        // with trailing newline
        let mut c = vec![b'A'; len];
        c.push(b'\n');
        let f = Fixture::file(&format!("b567_nl_{len}"), &c);
        diff_open(&format!("B5-7/len={len}/nl"), Some(f.path()));

        // without trailing newline
        let c2 = vec![b'B'; len];
        let g = Fixture::file(&format!("b567_nonl_{len}"), &c2);
        diff_open(&format!("B5-7/len={len}/nonl"), Some(g.path()));

        // two such lines, so a boundary-split line is followed by another
        let mut c3 = vec![b'C'; len];
        c3.push(b'\n');
        c3.extend(std::iter::repeat(b'D').take(len));
        c3.push(b'\n');
        let h = Fixture::file(&format!("b567_two_{len}"), &c3);
        diff_open(&format!("B5-7/len={len}/x2"), Some(h.path()));
    }
}

/// B8: embedded NUL bytes — `printf("%s", buffer)` stops at the NUL even though
/// `fgets` consumed the whole line.
#[test]
fn b8_open_embedded_nuls() {
    let cases: &[&[u8]] = &[
        b"\0",
        b"\0\n",
        b"a\0b\n",
        b"a\0b",
        b"\0abc\n\0def\n",
        b"before\0after\nsecond\0line\n",
        b"\0\0\0\0\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        let f = Fixture::file(&format!("b8_{i}"), c);
        diff_open(&format!("B8/{i}"), Some(f.path()));
    }
    // NUL placed exactly at the chunk boundary
    for pos in [0usize, 1, 97, 98, 99, 100, 101] {
        let mut c = vec![b'Z'; 250];
        c[pos] = 0;
        c.push(b'\n');
        let f = Fixture::file(&format!("b8_pos_{pos}"), &c);
        diff_open(&format!("B8/nul@{pos}"), Some(f.path()));
    }
}

/// B9: content made of printf conversion specifiers — must be passed as data.
#[test]
fn b9_open_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s\n",
        b"%d %d %d\n",
        b"%n\n",
        b"%p %x %%\n",
        b"100%\n",
        b"%s%s%s%s%s%s%s%s%s%s\n",
        b"%.999999f\n",
        b"%\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        let f = Fixture::file(&format!("b9_{i}"), c);
        diff_open(&format!("B9/{i}"), Some(f.path()));
    }
}

/// B10: large file — many buffer refills.
#[test]
fn b10_open_large_file() {
    let mut rng = Rng::new(SEED ^ 0xB10);
    for (iter, size) in [4096usize, 65_536, 200_000].into_iter().enumerate() {
        let mut c = Vec::with_capacity(size + 8);
        while c.len() < size {
            if rng.below(20) == 0 {
                c.push(b'\n');
            } else {
                c.push(b' ' + (rng.below(94) as u8));
            }
        }
        let f = Fixture::file(&format!("b10_{iter}"), &c);
        diff_open(&format!("B10/size={size}"), Some(f.path()));
    }
}

/// B11: only newlines — many 1-byte lines.
#[test]
fn b11_open_only_newlines() {
    for n in [1usize, 2, 99, 100, 101, 1000] {
        let c = vec![b'\n'; n];
        let f = Fixture::file(&format!("b11_{n}"), &c);
        diff_open(&format!("B11/n={n}"), Some(f.path()));
    }
}

/// B12: no newline anywhere, longer than the buffer — pure chunking.
#[test]
fn b12_open_no_newline_long() {
    for n in [99usize, 100, 101, 999, 1000, 4097] {
        let c = vec![b'q'; n];
        let f = Fixture::file(&format!("b12_{n}"), &c);
        diff_open(&format!("B12/n={n}"), Some(f.path()));
    }
}

/// B13: randomized arbitrary binary content, randomized sizes.
#[test]
fn b13_open_random_binary() {
    let mut rng = Rng::new(SEED ^ 0xB13);
    for iter in 0..250 {
        let n = rng.below(4097) as usize;
        let mut c = Vec::with_capacity(n);
        for _ in 0..n {
            c.push(rng.byte());
        }
        let f = Fixture::file(&format!("b13_{iter}"), &c);
        diff_open(&format!("B13/iter={iter}/n={n}"), Some(f.path()));
    }
}

/// B14: the returned `FILE*` is a real, caller-`fclose`-able handle in both
/// implementations (the harness closes it and asserts success).
#[test]
fn b14_open_returns_usable_handle() {
    let f = Fixture::file("b14", b"one\ntwo\n");
    let (c_null, _) = call_open(Impl::C, Some(f.path()));
    let (r_null, _) = call_open(Impl::Rust, Some(f.path()));
    assert!(!c_null, "C returned NULL for a readable file");
    assert!(!r_null, "Rust returned NULL for a readable file");
    assert_eq!(c_null, r_null);
}

// ===========================================================================
// Axis C — driver (the composed pipeline)
// ===========================================================================

/// C1: num == 0 with an empty file.
#[test]
fn c1_driver_zero_empty_file() {
    let f = Fixture::file("c1", b"");
    diff_driver("C1", 0, Some(f.path()));
}

/// C2: positive num with a multi-line file.
#[test]
fn c2_driver_positive_multiline() {
    let f = Fixture::file("c2", b"alpha\nbeta\ngamma\n");
    for num in [1i32, 2, 3, 17, 1000, 123_456] {
        diff_driver("C2", num, Some(f.path()));
    }
}

/// C3: overflowing `num * 2` — the printed `Goto output:` is the wrapped value.
#[test]
fn c3_driver_overflow() {
    let f = Fixture::file("c3", b"data\n");
    for num in [i32::MAX / 2, i32::MAX / 2 + 1, 1_500_000_000, i32::MAX - 1] {
        diff_driver("C3", num, Some(f.path()));
    }
}

/// C4: num == INT_MAX.
#[test]
fn c4_driver_int_max() {
    let f = Fixture::file("c4", b"x\n");
    diff_driver("C4", i32::MAX, Some(f.path()));
}

/// C5: randomized num over the full i32 range crossed with randomized file
/// shapes — the composed pipeline including stdout write ordering.
#[test]
fn c5_driver_random_cross_product() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for iter in 0..250 {
        let n = rng.below(600) as usize;
        let mut c = Vec::with_capacity(n);
        for _ in 0..n {
            match rng.below(10) {
                0 => c.push(b'\n'),
                1 => c.push(0),
                _ => c.push(rng.byte()),
            }
        }
        let f = Fixture::file(&format!("c5_{iter}"), &c);
        // a mix of negative, zero, small, huge
        let num = match iter % 5 {
            0 => rng.i32_any(),
            1 => rng.range_i32(i32::MIN, -1),
            2 => 0,
            3 => rng.range_i32(1, 1_000_000),
            _ => rng.range_i32(i32::MAX / 2, i32::MAX),
        };
        diff_driver(&format!("C5/iter={iter}"), num, Some(f.path()));
    }
}

/// C6: num >= 0 crossed with buffer-boundary file shapes.
#[test]
fn c6_driver_buffer_boundary_files() {
    for len in [98usize, 99, 100, 101, 199, 200] {
        let mut c = vec![b'M'; len];
        c.push(b'\n');
        c.extend(std::iter::repeat(b'N').take(len));
        let f = Fixture::file(&format!("c6_{len}"), &c);
        for num in [0i32, 1, 7, i32::MAX] {
            diff_driver(&format!("C6/len={len}"), num, Some(f.path()));
        }
    }
}

/// C7: num >= 0 crossed with NUL / format-specifier file content.
#[test]
fn c7_driver_nuls_and_formats() {
    let cases: &[&[u8]] = &[b"a\0b\n", b"%s %d\n", b"%n\0%s\n", b"\0", b"pre\0post"];
    for (i, c) in cases.iter().enumerate() {
        let f = Fixture::file(&format!("c7_{i}"), c);
        for num in [0i32, 5, 1_000_000_000] {
            diff_driver(&format!("C7/{i}"), num, Some(f.path()));
        }
    }
}

/// C8: num >= 0 with a directory as filename (fopen ok, fgets sets ferror).
#[test]
fn c8_driver_directory() {
    let d = Fixture::dir("c8");
    for num in [0i32, 1, 42, i32::MAX] {
        diff_driver("C8", num, Some(d.path()));
    }
}

/// C9: stdout and stderr are both written in one call; the merged-stream
/// ordering is compared (diff_driver already does the combined capture).
#[test]
fn c9_driver_stream_interleaving() {
    let good = Fixture::file("c9_good", b"line1\nline2\n");
    let missing = Fixture::missing("c9_missing");

    // stderr only (num < 0)
    diff_driver("C9/stderr-only", -1, Some(good.path()));
    // stdout only (success)
    diff_driver("C9/stdout-only", 3, Some(good.path()));
    // both, stdout first then stderr
    diff_driver("C9/both", 3, Some(&missing));
    // both, error before any stdout
    diff_driver("C9/neg+missing", -9, Some(&missing));
}

/// C10: repeated calls — no hidden per-call state; output concatenates.
#[test]
fn c10_driver_repeated_calls() {
    let f = Fixture::file("c10", b"repeat me\nagain\n");
    let p: &Path = f.path();

    let mut c_all = Vec::new();
    let mut r_all = Vec::new();
    let mut c_rets = Vec::new();
    let mut r_rets = Vec::new();
    for num in [0i32, 1, -1, 2, i32::MAX, -i32::MAX, 3] {
        let (cr, co) = call_driver(Impl::C, num, Some(p));
        let (rr, ro) = call_driver(Impl::Rust, num, Some(p));
        c_rets.push(cr);
        r_rets.push(rr);
        c_all.extend_from_slice(&co.stdout);
        c_all.extend_from_slice(&co.stderr);
        r_all.extend_from_slice(&ro.stdout);
        r_all.extend_from_slice(&ro.stderr);
    }
    assert_eq!(c_rets, r_rets, "C10: return sequence differs");
    assert_eq!(
        c_all,
        r_all,
        "C10: concatenated output differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_all),
        String::from_utf8_lossy(&r_all)
    );
}
