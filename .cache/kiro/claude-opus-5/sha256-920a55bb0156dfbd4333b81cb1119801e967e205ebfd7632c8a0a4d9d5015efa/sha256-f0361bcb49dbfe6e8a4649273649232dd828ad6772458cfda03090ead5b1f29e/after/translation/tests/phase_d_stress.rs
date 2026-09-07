//! Phase D — heavier randomized sweeps and the exact buffer boundaries, run in
//! addition to the per-row tests of Phases B and C.

mod common;

use common::*;
use std::ffi::c_int;

/// `matrix_to_string` writes each element through `snprintf(buffer, 12, "%d", v)`.
/// `INT_MIN` renders as `-2147483648` = 11 chars + NUL = exactly 12 bytes, so
/// this is the boundary of that buffer.
///
/// A single-column matrix is the widest case that still fits the C's own
/// `11*width` per-row allocation (needed `12*height+1`, allocated `12*height+1`),
/// so it can be exercised without provoking the C's heap overflow.
#[test]
fn stress_to_string_11_char_values_width_1() {
    let _g = lock();
    for h in 1..=6 {
        let rows: Vec<String> = (0..h)
            .map(|i| {
                match i % 4 {
                    0 => "-2147483648".to_string(),
                    1 => "2147483647".to_string(),
                    2 => "-1000000000".to_string(),
                    _ => "1000000000".to_string(),
                }
            })
            .collect();
        let input = format!("{}\n", rows.join("\n"));
        diff_to_string(&format!("stress-w1-h{h}"), &input, 1, h);
    }
}

/// Same boundary reached through `atoi` clamping rather than literal text.
#[test]
fn stress_to_string_atoi_clamped_values_width_1() {
    let _g = lock();
    let inputs = [
        "99999999999999999999",
        "-99999999999999999999",
        "2147483648",
        "-2147483649",
        "4294967296",
        "-0",
    ];
    for (n, v) in inputs.iter().enumerate() {
        diff_to_string(&format!("stress-atoi-{n}"), v, 1, 1);
    }
    let joined = format!("{}\n", inputs.join("\n"));
    diff_to_string("stress-atoi-col", &joined, 1, inputs.len() as c_int);
}

/// Larger shapes than the per-row tests use, still inside the safe value bound.
#[test]
fn stress_large_shapes_pipeline() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x1234);
    let mut skipped = 0usize;
    for i in 0..60 {
        let ha = rng.range(1, 24) as i32;
        let inner = rng.range(1, 24) as i32;
        let wb = rng.range(1, 24) as i32;
        let bound: i32 = *rng.pick(&[9, 999, 999_999]);
        let a: Vec<Vec<i32>> = (0..ha)
            .map(|_| (0..inner).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        let b: Vec<Vec<i32>> = (0..inner)
            .map(|_| (0..wb).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        if !diff_pipeline_if_safe(
            &format!("stress-big#{i}"),
            &render(&a),
            inner,
            ha,
            &render(&b),
            wb,
            inner,
        ) {
            skipped += 1;
        }
    }
    // The guard must not swallow the whole sweep.
    assert!(
        skipped < 60,
        "stress_large_shapes_pipeline: all {skipped} cases skipped by the C-UB guard"
    );
    eprintln!("stress_large_shapes_pipeline: 60 cases, {skipped} skipped (C heap-overflow UB)");
}

/// The composed pipeline (init x2 -> multiply -> to_string) over the same axes
/// Phase B fuzzes individually — bugs in the composition are invisible to the
/// per-function tests.
#[test]
fn stress_pipeline_fuzz_all_shapes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x9999);
    let mut skipped = 0usize;
    for i in 0..400 {
        let ha = rng.range(0, 7) as i32;
        let inner = rng.range(0, 7) as i32;
        let wb = rng.range(0, 7) as i32;
        // Sometimes deliberately break conformability so the NULL path composes.
        let inner_b = if rng.range(0, 5) == 0 {
            rng.range(0, 7) as i32
        } else {
            inner
        };
        let bound: i32 = *rng.pick(&[9, 99, 999_999_999]);
        let a: Vec<Vec<i32>> = (0..ha)
            .map(|_| (0..inner).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        let b: Vec<Vec<i32>> = (0..inner_b)
            .map(|_| (0..wb).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        if !diff_pipeline_if_safe(
            &format!("stress-pipe#{i}"),
            &render(&a),
            inner,
            ha,
            &render(&b),
            wb,
            inner_b,
        ) {
            skipped += 1;
        }
    }
    assert!(
        skipped < 400,
        "stress_pipeline_fuzz_all_shapes: all {skipped} cases skipped by the C-UB guard"
    );
    eprintln!("stress_pipeline_fuzz_all_shapes: 400 cases, {skipped} skipped (C heap-overflow UB)");
}

/// End-to-end `driver` sweep, including non-conformable and degenerate shapes,
/// comparing the produced `matrix.txt` byte-for-byte.
#[test]
fn stress_driver_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x4242);
    let mut skipped = 0usize;
    for i in 0..150 {
        let ha = rng.range(0, 5) as i32;
        let inner = rng.range(0, 5) as i32;
        let wb = rng.range(0, 5) as i32;
        let inner_b = if rng.range(0, 4) == 0 {
            rng.range(0, 5) as i32
        } else {
            inner
        };
        let bound: i32 = *rng.pick(&[9, 9999, 999_999]);
        let a: Vec<Vec<i32>> = (0..ha)
            .map(|_| (0..inner).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        let b: Vec<Vec<i32>> = (0..inner_b)
            .map(|_| (0..wb).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        if !diff_driver_if_safe(
            &format!("stress-drv#{i}"),
            inner,
            ha,
            &render(&a),
            wb,
            inner_b,
            &render(&b),
        ) {
            skipped += 1;
        }
    }
    assert!(
        skipped < 150,
        "stress_driver_fuzz: all {skipped} cases skipped by the C-UB guard"
    );
    eprintln!("stress_driver_fuzz: 150 cases, {skipped} skipped (C heap-overflow UB)");
}

/// Randomised token soup: strings that are not well-formed matrices at all.
#[test]
fn stress_init_token_soup() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x7777);
    let mut skipped = 0usize;
    let alphabet = [
        " ", "\n", "\t", "0", "1", "9", "-", "+", "a", "z", ".", ",", "2147483647",
        "-2147483648", "99999999999", "  ", "\n\n",
    ];
    for i in 0..500 {
        let n = rng.range(0, 40) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(rng.pick(&alphabet));
        }
        let w = rng.range(0, 5) as i32;
        let h = rng.range(0, 5) as i32;
        diff_init(&format!("soup#{i}"), &s, w, h);
        if !diff_to_string_if_safe(&format!("soup-ts#{i}"), &s, w, h) {
            skipped += 1;
        }
    }
    assert!(
        skipped < 500,
        "stress_init_token_soup: all {skipped} cases skipped by the C-UB guard"
    );
    eprintln!("stress_init_token_soup: 500 cases, {skipped} skipped (C heap-overflow UB)");
}

/// Randomised `write_to_file` contents including bytes that look like format
/// specifiers, mixed with newlines and high bytes.
#[test]
fn stress_write_fuzz_bytes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x5151);
    let chunks: [&[u8]; 8] = [
        b"%s", b"%n", b"%%", b"\n", b"\r\n", b"\xff\xfe", b"abc", b"\t",
    ];
    for i in 0..150 {
        let n = rng.range(0, 200) as usize;
        let mut content = Vec::new();
        for _ in 0..n {
            content.extend_from_slice(rng.pick(&chunks));
        }
        diff_write(&format!("wfuzz#{i}"), &format!("s{i}"), &content);
    }
}

/// Repeated allocate/free churn at many shapes, to catch allocator-protocol
/// mismatches (wrong allocator, double free, mismatched row count).
#[test]
fn stress_allocator_churn() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x3131);
    for _ in 0..500 {
        let w = rng.range(0, 12) as c_int;
        let h = rng.range(0, 12) as c_int;
        for imp in [&p.c, &p.rs] {
            unsafe {
                let m = imp.allocate_matrix(w, h);
                assert!(!m.is_null(), "allocate_matrix({w},{h}) unexpectedly NULL");
                let _ = probe_writable(m);
                imp.free_matrix(m);
            }
        }
    }
}
