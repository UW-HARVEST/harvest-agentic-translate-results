//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH `c_src/build/libdriver.so` and this crate's
//! `libdriver.so` via `libloading` and compares the results of
//! `UTIL_createLinePointers` byte-for-byte across randomized inputs
//! (fixed seed, reproducible).

mod common;

use common::*;
use std::os::raw::c_char;

/// CONFIGS row 1: numLines = 0, bufferSize = 0.
#[test]
fn cfg_01_zero_lines_zero_size() {
    let mut buf: Vec<u8> = Vec::new();
    assert_same_raw(buf.as_mut_ptr() as *mut c_char, 0, 0, "cfg01");
    // also with a real (non-dangling) allocation of length 0..
    let mut buf2 = vec![0u8; 8];
    assert_same(&mut buf2, 0, 0, "cfg01b");
}

/// CONFIGS row 2: numLines = 0, non-empty randomized buffer.
#[test]
fn cfg_02_zero_lines_nonempty_buffer() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..ITERS {
        let n = rng.range(1, 64);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.any_byte()).collect();
        let size = rng.range(1, n);
        assert_same(&mut buf, 0, size, &format!("cfg02 iter{i}"));
    }
}

/// CONFIGS row 3: numLines = 1, bufferSize = 1, buffer = single '\0'.
#[test]
fn cfg_03_one_line_single_nul() {
    let mut buf = vec![0u8; 1];
    assert_same(&mut buf, 1, 1, "cfg03");
}

/// CONFIGS row 4: numLines = 1, bufferSize = 1, buffer = one non-NUL byte
/// (the `if (pos < bufferSize) pos++` branch is NOT taken).
#[test]
fn cfg_04_one_line_single_nonnul() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..ITERS {
        let mut buf = vec![rng.nonzero_byte()];
        assert_same(&mut buf, 1, 1, &format!("cfg04 iter{i}"));
    }
}

/// CONFIGS row 5: one NUL-terminated line of randomized length/content.
#[test]
fn cfg_05_one_terminated_line() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..ITERS {
        let len = rng.range(1, 48);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        buf.push(0);
        let size = buf.len();
        assert_same(&mut buf, 1, size, &format!("cfg05 iter{i}"));
    }
}

/// CONFIGS row 6: one line with NO terminator, bufferSize == strlen.
#[test]
fn cfg_06_one_unterminated_line() {
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..ITERS {
        let len = rng.range(1, 48);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let size = buf.len();
        assert_same(&mut buf, 1, size, &format!("cfg06 iter{i}"));
    }
}

/// CONFIGS row 7: numLines == number of NUL-terminated lines present (2..16).
#[test]
fn cfg_07_many_terminated_lines_exact() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..ITERS {
        let count = rng.range(2, 16);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(1, 12);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        assert_same(&mut buf, count, size, &format!("cfg07 iter{i}"));
    }
}

/// CONFIGS row 8: many lines, last one unterminated (buffer ends mid-line).
#[test]
fn cfg_08_many_lines_last_unterminated() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..ITERS {
        let count = rng.range(2, 16);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(1, 12);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        buf.pop(); // drop the final terminator
        let size = buf.len();
        assert_same(&mut buf, count, size, &format!("cfg08 iter{i}"));
    }
}

/// CONFIGS row 9: empty lines mixed with non-empty ones (`len == 0`
/// iterations interleaved with `len > 0` ones).
#[test]
fn cfg_09_mixed_empty_and_nonempty_lines() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..ITERS {
        let count = rng.range(2, 16);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                if rng.below(2) == 0 {
                    Vec::new() // empty line
                } else {
                    let len = rng.range(1, 10);
                    (0..len).map(|_| rng.nonzero_byte()).collect()
                }
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        assert_same(&mut buf, count, size, &format!("cfg09 iter{i}"));
    }
}

/// CONFIGS row 10: all-NUL buffer with numLines == bufferSize.
#[test]
fn cfg_10_all_nul_buffer() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..ITERS {
        let n = rng.range(1, 40);
        let mut buf = vec![0u8; n];
        assert_same(&mut buf, n, n, &format!("cfg10 iter{i}"));
        // and a few sub-counts of the same all-NUL buffer
        let fewer = rng.range(0, n);
        assert_same(&mut buf, fewer, n, &format!("cfg10-sub iter{i}"));
    }
}

/// CONFIGS row 11: numLines strictly fewer than the lines present.
#[test]
fn cfg_11_fewer_requested_than_present() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..ITERS {
        let count = rng.range(2, 16);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(0, 10);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        let want = rng.range(0, count - 1);
        assert_same(&mut buf, want, size, &format!("cfg11 iter{i}"));
    }
}

/// CONFIGS row 12: bufferSize larger than the string content — the trailing
/// NUL padding is consumed as additional empty lines.
#[test]
fn cfg_12_trailing_nul_padding() {
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..ITERS {
        let count = rng.range(1, 8);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(1, 10);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let pad = rng.range(1, 12);
        buf.extend(std::iter::repeat(0u8).take(pad));
        let size = buf.len();
        // exactly the number of lines the padded buffer yields
        assert_same(&mut buf, count + pad, size, &format!("cfg12-exact iter{i}"));
        // and every count from 0 up to that, all of which are valid
        for want in 0..=(count + pad) {
            assert_same(&mut buf, want, size, &format!("cfg12-{want} iter{i}"));
        }
    }
}

/// CONFIGS row 13: bufferSize truncates the content mid-line, so the inner
/// scan stops on the size guard rather than on a '\0'.
#[test]
fn cfg_13_truncated_buffersize_midline() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..ITERS {
        let count = rng.range(2, 10);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(2, 10);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let full = buf.len();
        let truncated = rng.range(1, full);
        for want in 0..=count {
            assert_same(&mut buf, want, truncated, &format!("cfg13-{want} iter{i}"));
        }
    }
}

/// CONFIGS row 14: a single long line (up to 4096 bytes).
#[test]
fn cfg_14_single_long_line() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..40 {
        let len = rng.range(1, 4096);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        buf.push(0);
        let size = buf.len();
        assert_same(&mut buf, 1, size, &format!("cfg14 iter{i}"));
        assert_same(&mut buf, 2, size, &format!("cfg14b iter{i}"));
    }
}

/// CONFIGS row 15: buffer containing no NUL at all, numLines == 1.
#[test]
fn cfg_15_no_terminator_single_line() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..ITERS {
        let n = rng.range(1, 64);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        assert_same(&mut buf, 1, n, &format!("cfg15 iter{i}"));
        // partial sizes too
        let s = rng.range(1, n);
        assert_same(&mut buf, 1, s, &format!("cfg15b iter{i}"));
    }
}

/// CONFIGS row 16: fully randomized fuzz across all shapes at once.
#[test]
fn cfg_16_full_random_fuzz() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..5000 {
        let cap = rng.range(0, 64);
        let mut buf: Vec<u8> = (0..cap)
            .map(|_| {
                // bias towards '\0' so line boundaries are frequent
                if rng.below(3) == 0 {
                    0
                } else {
                    rng.nonzero_byte()
                }
            })
            .collect();
        let size = rng.range(0, cap);
        let num_lines = rng.range(0, 20);
        if buf.is_empty() {
            assert_same_raw(buf.as_mut_ptr() as *mut c_char, num_lines, 0, &format!("cfg16 iter{i}"));
        } else {
            assert_same(&mut buf, num_lines, size, &format!("cfg16 iter{i}"));
        }
    }
}

/// CONFIGS row 17: repeated / interleaved calls — no hidden static state.
#[test]
fn cfg_17_repeated_calls_no_state() {
    let mut rng = Rng::new(SEED ^ 17);
    let lines: Vec<Vec<u8>> = (0..8)
        .map(|_| {
            let len = rng.range(1, 10);
            (0..len).map(|_| rng.nonzero_byte()).collect()
        })
        .collect();
    let mut buf = join_terminated(&lines);
    let size = buf.len();
    for round in 0..50 {
        for want in 0..=10 {
            assert_same(&mut buf, want, size, &format!("cfg17 round{round} want{want}"));
        }
        // interleave a couple of failing calls to make sure they leave no residue
        assert_same(&mut buf, 99, size, &format!("cfg17 round{round} fail"));
        assert_same(&mut buf, 3, size, &format!("cfg17 round{round} after-fail"));
    }
}
