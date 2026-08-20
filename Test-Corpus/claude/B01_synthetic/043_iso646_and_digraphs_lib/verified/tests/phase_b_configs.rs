//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared libraries through their exported `driver`
//! symbol (loaded with `libloading`) and compares the bytes they write to
//! `stdout`, byte for byte.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 / 2 — isolated single calls at the two most special inputs.
// ---------------------------------------------------------------------------

#[test]
fn cfg01_single_call_zero_zero() {
    // x = 0, y = 0  =>  0 | ~0 = -1
    assert_same_each("cfg01", &[(0, 0)]);
    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0, 0) });
    assert_eq!(out, b"-1\n", "C reference output for driver(0,0)");
}

#[test]
fn cfg02_single_call_only_zero_result() {
    // x = 0, y = -1  =>  0 | 0 = 0  (the only input whose result is zero)
    assert_same_each("cfg02", &[(0, -1)]);
    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0, -1) });
    assert_eq!(out, b"0\n", "C reference output for driver(0,-1)");
}

// ---------------------------------------------------------------------------
// Rows 3-6 — the four sign quadrants.
// ---------------------------------------------------------------------------

const N: usize = 5000;

#[test]
fn cfg03_random_pos_pos() {
    let mut rng = Rng::new(SEED ^ 3);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_pos(), rng.next_pos())).collect();
    assert_same_batch("cfg03", &v);
}

#[test]
fn cfg04_random_pos_neg() {
    let mut rng = Rng::new(SEED ^ 4);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_pos(), rng.next_neg())).collect();
    assert_same_batch("cfg04", &v);
}

#[test]
fn cfg05_random_neg_pos() {
    let mut rng = Rng::new(SEED ^ 5);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_neg(), rng.next_pos())).collect();
    assert_same_batch("cfg05", &v);
}

#[test]
fn cfg06_random_neg_neg() {
    let mut rng = Rng::new(SEED ^ 6);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_neg(), rng.next_neg())).collect();
    assert_same_batch("cfg06", &v);
}

// ---------------------------------------------------------------------------
// Row 7 — uniform over the whole 32-bit domain.
// ---------------------------------------------------------------------------

#[test]
fn cfg07_random_full_32bit_domain() {
    let mut rng = Rng::new(SEED ^ 7);
    let v: Vec<(i32, i32)> = (0..20_000)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();
    assert_same_batch("cfg07", &v);
}

// ---------------------------------------------------------------------------
// Rows 8-11 — one operand pinned to a value the expression collapses on.
// ---------------------------------------------------------------------------

#[test]
fn cfg08_x_zero_random_y() {
    let mut rng = Rng::new(SEED ^ 8);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (0, rng.next_i32())).collect();
    assert_same_batch("cfg08", &v);
}

#[test]
fn cfg09_y_zero_random_x() {
    let mut rng = Rng::new(SEED ^ 9);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_i32(), 0)).collect();
    assert_same_batch("cfg09", &v);
}

#[test]
fn cfg10_x_minus_one_random_y() {
    let mut rng = Rng::new(SEED ^ 10);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (-1, rng.next_i32())).collect();
    assert_same_batch("cfg10", &v);
}

#[test]
fn cfg11_y_minus_one_random_x_identity() {
    // y = -1 => ~y = 0 => result == x, so this sweeps every printed width and
    // both signs of the formatted output.
    let mut rng = Rng::new(SEED ^ 11);
    let v: Vec<(i32, i32)> = (0..N).map(|_| (rng.next_i32(), -1)).collect();
    assert_same_batch("cfg11", &v);
}

// ---------------------------------------------------------------------------
// Rows 12-13 — relations between the two operands.
// ---------------------------------------------------------------------------

#[test]
fn cfg12_x_equals_y() {
    let mut rng = Rng::new(SEED ^ 12);
    let v: Vec<(i32, i32)> = (0..N)
        .map(|_| {
            let x = rng.next_i32();
            (x, x)
        })
        .collect();
    assert_same_batch("cfg12", &v);
}

#[test]
fn cfg13_x_equals_complement_y() {
    let mut rng = Rng::new(SEED ^ 13);
    let v: Vec<(i32, i32)> = (0..N)
        .map(|_| {
            let y = rng.next_i32();
            (!y, y)
        })
        .collect();
    assert_same_batch("cfg13", &v);
}

// ---------------------------------------------------------------------------
// Rows 14-16 — exhaustive grids.
// ---------------------------------------------------------------------------

pub const BOUNDARY: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];

#[test]
fn cfg14_exhaustive_boundary_grid() {
    let mut v = Vec::new();
    for &x in &BOUNDARY {
        for &y in &BOUNDARY {
            v.push((x, y));
        }
    }
    assert_eq!(v.len(), 25);
    assert_same_batch("cfg14", &v);
    // also per-call, so a single boundary value cannot hide inside the stream
    assert_same_each("cfg14-each", &v);
}

#[test]
fn cfg15_exhaustive_small_magnitude_grid() {
    let mut v = Vec::new();
    for x in -4..=4 {
        for y in -4..=4 {
            v.push((x, y));
        }
    }
    assert_eq!(v.len(), 81);
    assert_same_batch("cfg15", &v);
    assert_same_each("cfg15-each", &v);
}

#[test]
fn cfg16_exhaustive_single_bit_sweep() {
    let mut v = Vec::new();
    for i in 0..32u32 {
        for j in 0..32u32 {
            v.push(((1u32 << i) as i32, (1u32 << j) as i32));
        }
    }
    assert_eq!(v.len(), 1024);
    assert_same_batch("cfg16", &v);
}

// ---------------------------------------------------------------------------
// Row 17 — printed field-width sweep (1..10 digits, both signs, INT_MIN).
// ---------------------------------------------------------------------------

#[test]
fn cfg17_printed_width_sweep() {
    // With y = -1 the result equals x exactly, so pick x to hit every width.
    let widths: [i32; 25] = [
        0,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        9_999,
        -9_999,
        10_000,
        -10_000,
        999_999_999,
        -999_999_999,
        1_000_000_000,
        -1_000_000_000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        123_456_789,
        -123_456_789,
    ];
    let v: Vec<(i32, i32)> = widths.iter().map(|&x| (x, -1)).collect();
    assert_same_batch("cfg17", &v);
    assert_same_each("cfg17-each", &v);

    // Sanity-check a couple of widths against the C reference directly.
    let c = c_lib();
    assert_eq!(
        capture_stdout("c", || unsafe { (c.driver)(i32::MIN, -1) }),
        b"-2147483648\n"
    );
    assert_eq!(
        capture_stdout("c", || unsafe { (c.driver)(i32::MAX, -1) }),
        b"2147483647\n"
    );
}

// ---------------------------------------------------------------------------
// Rows 18-19 — call sequencing / capture granularity.
// ---------------------------------------------------------------------------

#[test]
fn cfg18_batched_sequential_output_accumulation() {
    let mut rng = Rng::new(SEED ^ 18);
    let v: Vec<(i32, i32)> = (0..20_000)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();
    assert_same_batch("cfg18", &v);

    // The concatenated stream must be exactly one line per call.
    let c_out = capture_stdout("c", || {
        for &(x, y) in &v {
            unsafe { (c_lib().driver)(x, y) };
        }
    });
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        v.len(),
        "one newline per call"
    );
    assert!(c_out.ends_with(b"\n"));
}

#[test]
fn cfg19_isolated_capture_per_call() {
    let mut rng = Rng::new(SEED ^ 19);
    let v: Vec<(i32, i32)> = (0..300).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    assert_same_each("cfg19", &v);
}

// ---------------------------------------------------------------------------
// Row 20 — interleaving with the caller's own stdio writes.
// ---------------------------------------------------------------------------

#[test]
fn cfg20_interleaved_with_caller_stdio() {
    let mut rng = Rng::new(SEED ^ 20);
    let v: Vec<(i32, i32)> = (0..200).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    let run = |f: DriverFn| {
        for (i, &(x, y)) in v.iter().enumerate() {
            caller_printf(&format!("<{i}"));
            unsafe { f(x, y) };
            caller_printf(&format!("{i}>"));
        }
    };

    let c_out = capture_stdout("c", || run(c_lib().driver));
    let r_out = capture_stdout("rust", || run(rust_lib().driver));
    assert_eq!(
        c_out,
        r_out,
        "[cfg20] interleaved stream mismatch\n C={:?}\n R={:?}",
        String::from_utf8_lossy(&c_out[..c_out.len().min(200)]),
        String::from_utf8_lossy(&r_out[..r_out.len().min(200)])
    );
    // The library must write through the *same* FILE, so the caller's marker
    // always precedes the library's digits.
    assert!(c_out.starts_with(b"<0"), "ordering: {:?}", &c_out[..8]);
}

// ---------------------------------------------------------------------------
// Rows 21-23 — stdout buffering modes.
// ---------------------------------------------------------------------------

fn buffering_row(row: &str, mode: std::ffi::c_int, size: usize, count: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    let v: Vec<(i32, i32)> = (0..count)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();

    let c = c_lib();
    let r = rust_lib();

    let c_out = capture_stdout("c", || {
        set_stdout_buffering(mode, size);
        for &(x, y) in &v {
            unsafe { (c.driver)(x, y) };
        }
    });
    let r_out = capture_stdout("rust", || {
        set_stdout_buffering(mode, size);
        for &(x, y) in &v {
            unsafe { (r.driver)(x, y) };
        }
    });
    // Restore a sane default for the remaining tests.
    set_stdout_buffering(IOFBF, 4096);

    assert_eq!(c_out.len(), r_out.len(), "[{row}] length differs");
    assert!(c_out == r_out, "[{row}] byte streams differ");
}

#[test]
fn cfg21_stdout_line_buffered() {
    buffering_row("cfg21", IOLBF, 4096, 2000, SEED ^ 21);
}

#[test]
fn cfg22_stdout_unbuffered() {
    buffering_row("cfg22", IONBF, 0, 2000, SEED ^ 22);
}

#[test]
fn cfg23_stdout_fully_buffered_buffer_wrap() {
    buffering_row("cfg23", IOFBF, 4096, 50_000, SEED ^ 23);
}

// ---------------------------------------------------------------------------
// Row 24 — stdout is a pipe.
// ---------------------------------------------------------------------------

#[test]
fn cfg24_stdout_is_a_pipe() {
    let mut rng = Rng::new(SEED ^ 24);
    // Keep well under the 64 KiB pipe capacity: no reader runs concurrently.
    let v: Vec<(i32, i32)> = (0..1000).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    let c = c_lib();
    let r = rust_lib();
    let c_out = capture_stdout_pipe(|| {
        for &(x, y) in &v {
            unsafe { (c.driver)(x, y) };
        }
    });
    let r_out = capture_stdout_pipe(|| {
        for &(x, y) in &v {
            unsafe { (r.driver)(x, y) };
        }
    });
    assert_eq!(c_out, r_out, "[cfg24] pipe-destination stream mismatch");
    assert!(!c_out.is_empty());
}

// ---------------------------------------------------------------------------
// Row 25 — both libraries resident at once; no symbol interposition.
// ---------------------------------------------------------------------------

#[test]
fn cfg25_no_interposition_alternating_calls() {
    let c = c_lib();
    let r = rust_lib();
    assert_ne!(
        c.driver as usize, r.driver as usize,
        "the two .so files must resolve to distinct `driver` implementations"
    );
    assert_ne!(c.path, r.path);

    let mut rng = Rng::new(SEED ^ 25);
    let v: Vec<(i32, i32)> = (0..500).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    // Alternate C, Rust, C, Rust ... within a single capture: every value must
    // appear twice in a row.
    let out = capture_stdout("alt", || {
        for &(x, y) in &v {
            unsafe {
                (c.driver)(x, y);
                (r.driver)(x, y);
            }
        }
    });
    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
    // trailing empty element after the final '\n'
    assert_eq!(lines.len(), 2 * v.len() + 1);
    assert!(lines.last().unwrap().is_empty());
    for i in 0..v.len() {
        assert_eq!(
            lines[2 * i],
            lines[2 * i + 1],
            "[cfg25] call {i} driver({}, {}): C={:?} Rust={:?}",
            v[i].0,
            v[i].1,
            String::from_utf8_lossy(lines[2 * i]),
            String::from_utf8_lossy(lines[2 * i + 1])
        );
    }
}

// ---------------------------------------------------------------------------
// Row 26 — deep formatting sweep. Random sampling of a 2^32 domain leaves the
// narrow decimal-carry windows (999->1000, 2^k boundaries, INT_MIN/MAX edges)
// mostly untouched, so sweep them contiguously and exhaustively.
//
// With y = -1 the expression collapses to `result == x`, which turns this into
// an exhaustive check of the `printf("%d")` path over the swept values.
// ---------------------------------------------------------------------------

#[test]
fn cfg26_deep_contiguous_and_boundary_window_sweep() {
    let mut v: Vec<(i32, i32)> = Vec::new();

    // (a) one contiguous block straddling zero: 2 * 2^19 + 1 consecutive values
    let span: i32 = 1 << 19;
    for x in -span..=span {
        v.push((x, -1));
    }

    // (b) +/-64 windows around every power of ten and power of two, and around
    //     the two extremes of the domain.
    let mut centres: Vec<i64> = vec![i32::MIN as i64, i32::MAX as i64, 0];
    let mut p: i64 = 1;
    while p <= i32::MAX as i64 {
        centres.push(p);
        centres.push(-p);
        p *= 10;
    }
    for k in 0..32u32 {
        let b = 1i64 << k;
        centres.push(b);
        centres.push(-b);
    }
    for c in centres {
        for d in -64i64..=64 {
            let t = c + d;
            if t >= i32::MIN as i64 && t <= i32::MAX as i64 {
                v.push((t as i32, -1));
            }
        }
    }

    assert!(v.len() > 1_000_000, "sweep should be large, got {}", v.len());
    assert_same_batch("cfg26", &v);
}
