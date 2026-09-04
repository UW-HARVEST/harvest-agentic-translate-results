//! Phase B -- valid-path differential tests for the top-level entry point
//! `driver`, whose only observable effect is the bytes it `printf`s to stdout.
//!
//! Covers rows C28..C43 of CONFIGS.md. stdout is captured at the file-descriptor
//! level so the comparison is byte-for-byte, including buffering behaviour.

mod common;
use common::*;
use std::ffi::c_int;

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Generic row driver: for each len, `DRAWS` seeded draws of `shape`.
fn row(row_id: &str, shape: Shape, lens: &[usize], draws: usize) {
    let p = pair();
    let mut rng = Rng::new(fnv(row_id));
    for &len in lens {
        for draw in 0..draws {
            let data = gen_vals(shape, len, &mut rng);
            let ctx = format!("{row_id} shape={shape:?} len={len} draw={draw}");
            let bytes = diff_driver(p, &data, len as c_int, &ctx);
            // Independent oracle: the exact bytes the C source must produce.
            assert_eq!(
                bytes,
                expected_stdout(&data),
                "{ctx}: stdout does not match the `%d\\n` of x*x+x model"
            );
        }
    }
}

/// Model of `inner`: `fma_array(out,out,out,out,len)` then `printf("%d\n", ...)`.
fn expected_stdout(data: &[c_int]) -> Vec<u8> {
    let mut s = String::new();
    for &x in data {
        let v = x.wrapping_mul(x).wrapping_add(x);
        s.push_str(&v.to_string());
        s.push('\n');
    }
    s.into_bytes()
}

// --- C28: empty ------------------------------------------------------------

#[test]
fn c28_driver_len_zero_no_output() {
    let p = pair();
    for data in [vec![], vec![7i32], vec![1i32, 2, 3, 4]] {
        let bytes = diff_driver(p, &data, 0, "C28 len=0");
        assert!(bytes.is_empty(), "C28: expected no stdout, got {bytes:?}");
    }
}

// --- C29: exactly one element ---------------------------------------------

#[test]
fn c29_driver_len_one() {
    row("C29a", Shape::Extremes, &[1], 40);
    row("C29b", Shape::FullRandom, &[1], 200);
    // And every extreme value explicitly, not just via sampling.
    let p = pair();
    for &x in EXTREMES {
        let data = [x];
        let bytes = diff_driver(p, &data, 1, &format!("C29c x={x}"));
        assert_eq!(bytes, expected_stdout(&data));
    }
}

// --- C30..C32: small "many" with single-digit / signed / mixed formatting ---

#[test]
fn c30_driver_small_positives() {
    row("C30", Shape::SmallPos, &[2, 3, 4, 5, 7, 8], DRAWS);
}

#[test]
fn c31_driver_small_negatives() {
    row("C31", Shape::SmallNeg, &[2, 3, 4, 5, 7, 8], DRAWS);
}

#[test]
fn c32_driver_mixed_small_with_zeros() {
    row("C32", Shape::MixedSmall, &[2, 3, 4, 5, 7, 8], DRAWS);
}

// --- C33: power-of-two boundary lengths -----------------------------------

#[test]
fn c33_driver_pow2_boundary_lens() {
    row("C33", Shape::FullRandom, &[15, 16, 17, 31, 32, 33, 63, 64, 65], DRAWS);
}

// --- C34/C35/C36: larger lengths, stdio buffer boundaries ------------------

#[test]
fn c34_driver_len_100_and_1000() {
    row("C34", Shape::FullRandom, &[100, 1000], DRAWS);
}

#[test]
fn c35_driver_len_4096() {
    row("C35", Shape::FullRandom, &[4096], 3);
}

#[test]
fn c36_driver_len_100k() {
    // Large, but the C VLA (400 KiB) still fits comfortably in an 8 MiB stack.
    row("C36", Shape::FullRandom, &[100_000], 2);
}

// --- C37..C40: value shapes across every length ---------------------------

#[test]
fn c37_driver_safe_magnitudes_all_lens() {
    row("C37", Shape::SafeMag, LENS, DRAWS);
}

#[test]
fn c38_driver_overflow_boundary_all_lens() {
    row("C38", Shape::Boundary, LENS, DRAWS);
}

#[test]
fn c39_driver_extremes_all_lens() {
    row("C39", Shape::Extremes, LENS, DRAWS);
    // Explicitly pin the widest `%d` renderings.
    let p = pair();
    let data = [i32::MIN, i32::MAX, 0, -1, 1, 65536, i32::MIN + 1, i32::MAX - 1];
    let bytes = diff_driver(p, &data, data.len() as c_int, "C39 widest");
    let s = String::from_utf8(bytes).unwrap();
    assert_eq!(s, String::from_utf8(expected_stdout(&data)).unwrap());
    // INT_MIN: (-2147483648)^2 + (-2147483648) wraps to -2147483648.
    assert!(s.starts_with("-2147483648\n"), "unexpected first line: {s:?}");
}

#[test]
fn c40_driver_extreme_pool_all_lens() {
    row("C40", Shape::ExtremePool, LENS, DRAWS);
}

#[test]
fn c30_32_all_shapes_x_all_lens_cross_product() {
    // Full Axis-C x Axis-D cross product for `driver`, so no value shape is
    // left untested at any length.
    for &shape in ALL_SHAPES {
        row("cross-driver", shape, &[0, 1, 2, 3, 4, 7, 8, 16, 17, 33, 64, 65, 100], 3);
    }
}

// --- C41: data buffer larger than len -------------------------------------

#[test]
fn c41_driver_data_larger_than_len() {
    let p = pair();
    let mut rng = Rng::new(fnv("C41"));
    for &cap in &[1usize, 2, 8, 17, 64, 129, 1000] {
        for len in 0..=cap {
            for _ in 0..3 {
                let data = gen_vals(Shape::FullRandom, cap, &mut rng);
                let ctx = format!("C41 cap={cap} len={len}");
                let bytes = diff_driver(p, &data, len as c_int, &ctx);
                // Only the first `len` elements may be read.
                assert_eq!(
                    bytes,
                    expected_stdout(&data[..len]),
                    "{ctx}: driver read beyond `len`"
                );
            }
        }
    }
}

// --- C42: composed pipeline cross-check -----------------------------------

/// `driver`'s stdout must equal: run the low-level `fma_array` in the exact
/// aliasing configuration `inner` uses, then format each element with `%d\n`.
/// Checked in all four cross combinations (C driver vs C fma, C driver vs Rust
/// fma, Rust driver vs C fma, Rust driver vs Rust fma).
#[test]
fn c42_composed_pipeline_matches_low_level() {
    let p = pair();
    let mut rng = Rng::new(fnv("C42"));

    for &len in LENS {
        for draw in 0..DRAWS {
            let data = gen_vals(Shape::FullRandom, len, &mut rng);
            let ctx = format!("C42 len={len} draw={draw}");
            let driver_bytes = diff_driver(p, &data, len as c_int, &ctx);

            for imp in [&p.c, &p.rs] {
                let mut t = data.clone();
                let ptr = t.as_mut_ptr();
                unsafe {
                    (imp.fma_array)(ptr, ptr, ptr, ptr, len as c_int);
                }
                let mut s = Vec::new();
                for &v in &t {
                    s.extend_from_slice(v.to_string().as_bytes());
                    s.push(b'\n');
                }
                assert_eq!(
                    driver_bytes, s,
                    "{ctx}: driver stdout != {} fma_array + %d formatting",
                    imp.name
                );
            }
        }
    }
}

// --- C43: interleaved repeated invocations --------------------------------

/// Alternate C / Rust / C / Rust for 200 rounds with random len and values. Any
/// residual state, or coupling through the shared libc stdio buffer, shows up
/// here as a divergence even though each individual call looks fine.
#[test]
fn c43_interleaved_repeated_invocations() {
    let p = pair();
    let mut rng = Rng::new(fnv("C43"));

    for round in 0..200 {
        let len = (rng.next_u64() % 40) as usize;
        let shape = *ALL_SHAPES
            .get((rng.next_u64() % ALL_SHAPES.len() as u64) as usize)
            .unwrap();
        let data = gen_vals(shape, len, &mut rng);
        let want = expected_stdout(&data);

        let _g = stdout_guard();
        // C, Rust, C, Rust -- four captures back to back in one lock scope.
        for step in 0..4 {
            let imp = if step % 2 == 0 { &p.c } else { &p.rs };
            let buf = data.clone();
            let (_, got) =
                capture_stdout(|| unsafe { (imp.driver)(buf.as_ptr(), len as c_int) });
            assert_eq!(
                got,
                want,
                "DIVERGENCE C43 round={round} step={step} impl={} shape={shape:?} len={len}\n  \
                 data={}\n  got ={:?}\n  want={:?}",
                imp.name,
                trunc(&data),
                String::from_utf8_lossy(&got[..got.len().min(200)]),
                String::from_utf8_lossy(&want[..want.len().min(200)]),
            );
            assert_eq!(buf, data, "round={round}: {} mutated const input", imp.name);
        }
    }
}
