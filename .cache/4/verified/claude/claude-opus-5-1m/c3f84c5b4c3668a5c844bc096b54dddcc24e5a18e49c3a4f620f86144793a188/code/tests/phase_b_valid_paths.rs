//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through their exported symbols and compares the `int` return value
//! and the exact stdout bytes. Randomized rows use a fixed seed.

mod common;

use common::*;
use std::ffi::c_int;

/// One representative value per `switch` path: 10 -> case10(+30 via fallthrough),
/// 20 -> case20(+20), 30 -> case30(+70 via fallthrough), 40 -> case40(+40),
/// 7 -> default(+7).
const PATHS: [i32; 5] = [10, 20, 30, 40, 7];

fn call_cleanup(lib: &Lib, a: i32, b: i32, c: i32, d: i32) -> i64 {
    unsafe { (lib.cleanup)(a as c_int, b as c_int, c as c_int, d as c_int) as i64 }
}

// =========================================================== cleanup: A1 x A2

/// B1 — exhaustive 5^4 = 625 per-slot switch-path cross-product.
fn b1_cleanup_exhaustive_switch_path_cross_product() {
    diff_batch("B1", 625, |lib, i| {
        let a = PATHS[i % 5];
        let b = PATHS[(i / 5) % 5];
        let c = PATHS[(i / 25) % 5];
        let d = PATHS[(i / 125) % 5];
        call_cleanup(lib, a, b, c, d)
    });
}

/// B2 — all-default, zero shape.
fn b2_cleanup_all_zero() {
    diff_once("B2", "cleanup(0,0,0,0)", |lib| call_cleanup(lib, 0, 0, 0, 0));
}

/// B3 — all-default, small positive, no overflow.
fn b3_cleanup_small_positive_random() {
    diff_batch("B3", 2000, |lib, i| {
        let i = i as u64;
        call_cleanup(
            lib,
            rnd_range(0xB3, i, 0, 1, 1000),
            rnd_range(0xB3, i, 1, 1, 1000),
            rnd_range(0xB3, i, 2, 1, 1000),
            rnd_range(0xB3, i, 3, 1, 1000),
        )
    });
}

/// B4 — all-default, small negative.
fn b4_cleanup_small_negative_random() {
    diff_batch("B4", 2000, |lib, i| {
        let i = i as u64;
        call_cleanup(
            lib,
            rnd_range(0xB4, i, 0, -1000, -1),
            rnd_range(0xB4, i, 1, -1000, -1),
            rnd_range(0xB4, i, 2, -1000, -1),
            rnd_range(0xB4, i, 3, -1000, -1),
        )
    });
}

/// B5 — off-by-one around every case label: {9,11,19,21,29,31,39,41}^4 = 4096.
fn b5_cleanup_off_by_one_cross_product() {
    const NEAR: [i32; 8] = [9, 11, 19, 21, 29, 31, 39, 41];
    diff_batch("B5", 8 * 8 * 8 * 8, |lib, i| {
        let a = NEAR[i % 8];
        let b = NEAR[(i / 8) % 8];
        let c = NEAR[(i / 64) % 8];
        let d = NEAR[(i / 512) % 8];
        call_cleanup(lib, a, b, c, d)
    });
}

/// B6 — cases and defaults interleaved in every position.
fn b6_cleanup_mixed_cases_and_defaults() {
    diff_batch("B6", 4000, |lib, i| {
        let i = i as u64;
        // Half the slots draw a case label, half draw an arbitrary value.
        let slot = |k: u64| -> i32 {
            if mix(0xB6, i ^ (k << 8)) % 2 == 0 {
                [10, 20, 30, 40][(mix(0xB6A, i ^ (k << 12)) % 4) as usize]
            } else {
                rnd_range(0xB6B, i, k, -5000, 5000)
            }
        };
        call_cleanup(lib, slot(0), slot(1), slot(2), slot(3))
    });
}

/// B7 — signed extremes cross-product, drives accumulator overflow both ways.
fn b7_cleanup_extremes_cross_product() {
    const EXT: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    diff_batch("B7", 7 * 7 * 7 * 7, |lib, i| {
        let a = EXT[i % 7];
        let b = EXT[(i / 7) % 7];
        let c = EXT[(i / 49) % 7];
        let d = EXT[(i / 343) % 7];
        call_cleanup(lib, a, b, c, d)
    });
}

/// B8 — unrestricted full-width random i32 over the whole 2^32 range.
fn b8_cleanup_full_width_random() {
    diff_batch("B8", 4000, |lib, i| {
        let i = i as u64;
        call_cleanup(
            lib,
            rnd_i32(0xB8, i, 0),
            rnd_i32(0xB8, i, 1),
            rnd_i32(0xB8, i, 2),
            rnd_i32(0xB8, i, 3),
        )
    });
}

/// B9 — the malloc/snprintf/printf tail: `TO_STRING(numbers)` stringizes the
/// *token*, so the line is literally `Processed numbers: numbers`, never the
/// argument values. Verified on both `.so`s across varied inputs.
fn b9_cleanup_tail_line_is_stringized_token() {
    const EXPECT: &[u8] = b"Processed numbers: numbers\n";
    let inputs: [[i32; 4]; 8] = [
        [0, 0, 0, 0],
        [10, 20, 30, 40],
        [7, 7, 7, 7],
        [1234, -5678, 9, 41],
        [i32::MIN, i32::MAX, 0, -1],
        [40, 30, 20, 10],
        [-1, -2, -3, -4],
        [999999, 888888, 777777, 666666],
    ];
    for (n, v) in inputs.iter().enumerate() {
        // Differential first.
        diff_once("B9", &format!("input #{n} {v:?}"), |lib| {
            call_cleanup(lib, v[0], v[1], v[2], v[3])
        });
        // Then pin the absolute expected bytes for both implementations.
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| call_cleanup(lib, v[0], v[1], v[2], v[3]));
            assert_eq!(
                out,
                EXPECT,
                "[B9] {} input {v:?}: expected \"{}\", got \"{}\"",
                lib.name,
                show(EXPECT),
                show(&out)
            );
        }
    }
}

/// B10 — statelessness: repeated identical calls must not drift.
fn b10_cleanup_repeated_calls_are_stateless() {
    let v = [10, 30, 7, 20];
    // 100 identical calls batched: any drift shows up as a stream mismatch, and
    // both libraries must emit exactly 100 identical lines.
    diff_batch("B10", 100, |lib, _| call_cleanup(lib, v[0], v[1], v[2], v[3]));

    for lib in [c_lib(), rust_lib()] {
        let (rets, out) = capture(|| {
            (0..100)
                .map(|_| call_cleanup(lib, v[0], v[1], v[2], v[3]))
                .collect::<Vec<_>>()
        });
        assert!(
            rets.iter().all(|&r| r == rets[0]),
            "[B10] {}: return value drifted across repeats: {:?}",
            lib.name,
            &rets[..10]
        );
        let expected: Vec<u8> = b"Processed numbers: numbers\n".repeat(100);
        assert_eq!(out, expected, "[B10] {}: stdout drifted across repeats", lib.name);
    }
}

// ======================================================== print_result: B1 x B2

fn call_print(lib: &Lib, label: &[u8], result: i32) -> i64 {
    let buf = cstr(label);
    unsafe { (lib.print_result)(buf.as_ptr(), result as c_int) };
    0
}

/// B11 — ordinary shape: short ASCII label, zero/positive/negative results.
fn b11_print_result_ordinary() {
    let labels: [&[u8]; 4] = [b"Result", b"total", b"Sum of numbers", b"x"];
    let results = [0i32, 42, -42, 7];
    diff_batch("B11", labels.len() * results.len(), |lib, i| {
        call_print(lib, labels[i % labels.len()], results[i / labels.len()])
    });
}

/// B12 — empty and 1-char labels x {0, 1, -1, INT_MAX, INT_MIN}.
fn b12_print_result_empty_and_single_char_labels() {
    let labels: [&[u8]; 4] = [b"", b"a", b" ", b":"];
    let results = [0i32, 1, -1, i32::MAX, i32::MIN];
    diff_batch("B12", labels.len() * results.len(), |lib, i| {
        call_print(lib, labels[i % labels.len()], results[i / labels.len()])
    });
}

/// B13 — oversized labels crossing glibc's internal BUFSIZ boundary.
fn b13_print_result_oversized_labels() {
    let sizes = [1024usize, 4096, 8191, 8192, 8193, 65536];
    diff_batch("B13", sizes.len(), |lib, i| {
        let label = vec![b'A'; sizes[i]];
        call_print(lib, &label, rnd_i32(0xB13, i as u64, 0))
    });
}

/// B14 — label containing conversion specifiers: must be printed literally,
/// because it is a `%s` argument and never a format string.
fn b14_print_result_label_with_format_specifiers() {
    let labels: [&[u8]; 8] = [
        b"%d",
        b"%s",
        b"%n",
        b"%%",
        b"%p",
        b"%d %s %n %% %p",
        b"100%",
        b"%1$s%2$n",
    ];
    diff_batch("B14", labels.len(), |lib, i| {
        call_print(lib, labels[i], rnd_i32(0xB14, i as u64, 0))
    });
}

/// B15 — control bytes and all 128 non-UTF-8 high bytes.
fn b15_print_result_control_and_non_utf8_bytes() {
    // Control-byte labels.
    let ctrl: [&[u8]; 5] = [b"a\nb", b"a\tb", b"a\rb", b"a\x0bb", b"a\x0cb"];
    diff_batch("B15-ctrl", ctrl.len(), |lib, i| {
        call_print(lib, ctrl[i], rnd_i32(0xB15, i as u64, 0))
    });

    // Every high byte 0x80..=0xFF on its own.
    diff_batch("B15-high", 128, |lib, i| {
        let byte = 0x80u8 + i as u8;
        let label = [byte, b'-', byte, byte];
        call_print(lib, &label, rnd_i32(0xB15B, i as u64, 0))
    });

    // A long label of mixed invalid UTF-8 sequences.
    diff_batch("B15-mixed", 1, |lib, _| {
        let label: Vec<u8> = (0x80u8..=0xffu8).chain(0x01u8..=0x1fu8).collect();
        call_print(lib, &label, -12345)
    });
}

/// B16 — embedded NUL: `%s` must stop at the NUL and ignore the tail.
fn b16_print_result_embedded_nul() {
    diff_batch("B16", 6, |lib, i| {
        // Build the buffer by hand so the interior NUL survives.
        let mut buf: Vec<std::ffi::c_char> = Vec::new();
        for &b in b"visible" {
            buf.push(b as std::ffi::c_char);
        }
        buf.push(0); // interior NUL
        for &b in b"HIDDEN-TAIL" {
            buf.push(b as std::ffi::c_char);
        }
        buf.push(0); // real terminator
        let result = [0i32, 1, -1, i32::MAX, i32::MIN, 999][i];
        unsafe { (lib.print_result)(buf.as_ptr(), result as c_int) };
        0
    });
}

/// B17 — randomized B1 x B2 cross-product.
fn b17_print_result_randomized_cross_product() {
    diff_batch("B17", 2000, |lib, i| {
        let i = i as u64;
        let label = rnd_label(0xB17, i, 300);
        call_print(lib, &label, rnd_i32(0xB17, i, 77))
    });
}

// ==================================================== cleanup_resources: C1

/// B18 — NULL pointer: the `if (dynamic_str)` guard rejects it, silent no-op.
fn b18_cleanup_resources_null() {
    diff_batch("B18", 100, |lib, _| {
        unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) };
        0
    });
}

/// B19 — genuine libc-malloc'd pointers: guard passes, `free` runs, no output.
/// Each library gets its OWN allocation (freeing the same pointer twice would be
/// UB, not a differential test).
fn b19_cleanup_resources_frees_valid_pointers() {
    for &size in &[1usize, 50, 4096] {
        let (_, c_out) = capture(|| {
            let p = libc_malloc(size);
            unsafe { (c_lib().cleanup_resources)(p) };
        });
        let (_, r_out) = capture(|| {
            let p = libc_malloc(size);
            unsafe { (rust_lib().cleanup_resources)(p) };
        });
        assert!(
            c_out.is_empty() && r_out.is_empty(),
            "[B19] size {size}: expected no output, C=\"{}\" Rust=\"{}\"",
            show(&c_out),
            show(&r_out)
        );
        assert_eq!(c_out, r_out, "[B19] size {size}: stdout differs");
    }
}

// ============================================================ composed pipeline

/// B20 — full end-to-end consumer pipeline: `cleanup` -> `print_result` with the
/// returned value -> `cleanup_resources`. The whole concatenated stdout of the
/// 3-call sequence is compared, so ordering and buffering of the composed
/// pipeline is covered, not just per-wrapper output.
fn b20_composed_pipeline() {
    diff_batch("B20", 1000, |lib, i| {
        let i = i as u64;
        let a = if mix(0x20A, i) % 3 == 0 {
            [10, 20, 30, 40][(mix(0x20B, i) % 4) as usize]
        } else {
            rnd_i32(0x20C, i, 0)
        };
        let b = rnd_range(0x20D, i, 1, -100, 100);
        let c = [10, 20, 30, 40, 0][(mix(0x20E, i) % 5) as usize];
        let d = rnd_i32(0x20F, i, 2);

        let ret = unsafe { (lib.cleanup)(a as c_int, b as c_int, c as c_int, d as c_int) };

        let label = rnd_label(0x2011_u64, i, 40);
        let buf = cstr(&label);
        unsafe { (lib.print_result)(buf.as_ptr(), ret) };

        unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) };

        ret as i64
    });
}

/// B21 — alternating C/Rust call ordering in one process.
fn b21_interleaved_call_ordering() {
    diff_interleaved("B21", 200, |lib, i| {
        let i = i as u64;
        let ret = call_cleanup(
            lib,
            rnd_i32(0xB21, i, 0),
            [10, 20, 30, 40, 5][(mix(0xB21A, i) % 5) as usize],
            rnd_range(0xB21, i, 2, -50, 50),
            rnd_i32(0xB21, i, 3),
        );
        let label = rnd_label(0xB21B, i, 24);
        call_print(lib, &label, ret as i32);
        let p = libc_malloc(50);
        unsafe { (lib.cleanup_resources)(p) };
        ret
    });
}

// ==================================================================== driver

/// Single `#[test]` entry point — see `common::run_rows` for why the rows are
/// not separate `#[test]`s (fd 1 is process-global during captures).
#[test]
fn phase_b_all_config_rows() {
    let rows: &[(&str, fn())] = &[
        ("B1  cleanup exhaustive 5^4 switch paths", b1_cleanup_exhaustive_switch_path_cross_product),
        ("B2  cleanup all-zero", b2_cleanup_all_zero),
        ("B3  cleanup small positive random", b3_cleanup_small_positive_random),
        ("B4  cleanup small negative random", b4_cleanup_small_negative_random),
        ("B5  cleanup off-by-one cross product", b5_cleanup_off_by_one_cross_product),
        ("B6  cleanup mixed cases and defaults", b6_cleanup_mixed_cases_and_defaults),
        ("B7  cleanup extremes cross product", b7_cleanup_extremes_cross_product),
        ("B8  cleanup full-width random", b8_cleanup_full_width_random),
        ("B9  cleanup tail line stringized token", b9_cleanup_tail_line_is_stringized_token),
        ("B10 cleanup repeated calls stateless", b10_cleanup_repeated_calls_are_stateless),
        ("B11 print_result ordinary", b11_print_result_ordinary),
        ("B12 print_result empty/1-char labels", b12_print_result_empty_and_single_char_labels),
        ("B13 print_result oversized labels", b13_print_result_oversized_labels),
        ("B14 print_result format specifiers", b14_print_result_label_with_format_specifiers),
        ("B15 print_result control/non-UTF8 bytes", b15_print_result_control_and_non_utf8_bytes),
        ("B16 print_result embedded NUL", b16_print_result_embedded_nul),
        ("B17 print_result randomized cross product", b17_print_result_randomized_cross_product),
        ("B18 cleanup_resources NULL", b18_cleanup_resources_null),
        ("B19 cleanup_resources valid pointers", b19_cleanup_resources_frees_valid_pointers),
        ("B20 composed pipeline", b20_composed_pipeline),
        ("B21 interleaved call ordering", b21_interleaved_call_ordering),
    ];
    run_rows("Phase B (CONFIGS.md)", rows);
}
