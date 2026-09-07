//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both implementations are reached ONLY
//! through `dlopen`/`dlsym` (`libloading`) on their respective `.so` files, so
//! the `#[no_mangle] extern "C"` export wrapper is under test too.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------- row 1
#[test]
fn row01_empty_input_minimum_hex_maxlen() {
    // bin_len = 0, hex_maxlen = 1: only the NUL terminator is stored.
    assert_same("row01", 16, 0, 1, &[], 0);
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_empty_input_randomized_hex_maxlen() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..2000 {
        let hex_maxlen = rng.range(1, 4096);
        assert_same("row02", 32, 0, hex_maxlen, &[], 0);
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_single_byte_all_values_minimum_hex_maxlen() {
    for b in 0u8..=255 {
        assert_same("row03", 8, 0, 3, &[b], 0);
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_single_byte_all_values_x_hex_maxlen() {
    // Exhaustive over byte values => exhaustive over all 4 nibble classes.
    for b in 0u8..=255 {
        for hex_maxlen in [3usize, 4, 64] {
            assert_same("row04", 128, 0, hex_maxlen, &[b], 0);
        }
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_two_bytes_exhaustive() {
    for hi in 0u8..=255 {
        for lo in 0u8..=255 {
            assert_same("row05", 8, 0, 5, &[hi, lo], 0);
        }
    }
}

// ------------------------------------------------- rows 6..9: nibble classes
fn nibble_class_row(label: &str, seed: u64, hi_letter: bool, lo_letter: bool) {
    let mut rng = Rng::new(seed);
    for _ in 0..400 {
        let bin_len = rng.range(1, 256);
        let mut bin = vec![0u8; bin_len];
        for b in bin.iter_mut() {
            let hi = if hi_letter {
                10 + (rng.next_u8() % 6)
            } else {
                rng.next_u8() % 10
            };
            let lo = if lo_letter {
                10 + (rng.next_u8() % 6)
            } else {
                rng.next_u8() % 10
            };
            *b = (hi << 4) | lo;
        }
        let hex_maxlen = bin_len * 2 + 1;
        assert_same_simple(label, &bin, hex_maxlen, 8);
    }
}

#[test]
fn row06_hi_digit_lo_digit() {
    nibble_class_row("row06", SEED ^ 6, false, false);
}

#[test]
fn row07_hi_digit_lo_letter() {
    nibble_class_row("row07", SEED ^ 7, false, true);
}

#[test]
fn row08_hi_letter_lo_digit() {
    nibble_class_row("row08", SEED ^ 8, true, false);
}

#[test]
fn row09_hi_letter_lo_letter() {
    nibble_class_row("row09", SEED ^ 9, true, true);
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_random_minimum_hex_maxlen() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..500 {
        let bin_len = rng.range(1, 512);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        assert_same_simple("row10", &bin, bin_len * 2 + 1, 8);
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_random_one_past_minimum_hex_maxlen() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..500 {
        let bin_len = rng.range(1, 512);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        assert_same_simple("row11", &bin, bin_len * 2 + 2, 8);
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_random_large_hex_maxlen() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..500 {
        let bin_len = rng.range(1, 512);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        // hex_maxlen far larger than needed: the C code never uses it beyond
        // the check, so the extra room must stay untouched (guard bytes).
        let hex_maxlen = bin_len * 2 + 1 + rng.range(0, 4095);
        assert_same_simple("row12", &bin, hex_maxlen, 64);
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_odd_and_even_bin_len() {
    let mut rng = Rng::new(SEED ^ 13);
    for bin_len in 1usize..=200 {
        // covers every parity and every small length
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        assert_same_simple("row13", &bin, bin_len * 2 + 1, 4);
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_unaligned_output_pointer() {
    let mut rng = Rng::new(SEED ^ 14);
    for hex_off in 0usize..=15 {
        for _ in 0..40 {
            let bin_len = rng.range(1, 128);
            let mut bin = vec![0u8; bin_len];
            rng.fill(&mut bin);
            let out_alloc = hex_off + bin_len * 2 + 1 + 16;
            assert_same("row14", out_alloc, hex_off, bin_len * 2 + 1, &bin, 0);
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_guard_bytes_untouched() {
    // `assert_same` already compares the WHOLE allocation (guard = 0xAA), so a
    // divergence in over-writing would fail. Here we additionally assert the
    // absolute invariant against C: nothing past hex[bin_len*2] is written.
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..300 {
        let bin_len = rng.range(0, 256);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        let need = bin_len * 2 + 1;
        let out_alloc = need + 32;
        let hex_maxlen = need + 32; // deliberately generous
        let c = run_one(&cf, out_alloc, 0, hex_maxlen, &bin, 0);
        let r = run_one(&rf, out_alloc, 0, hex_maxlen, &bin, 0);
        assert_eq!(c.buf, r.buf, "row15 buffers differ (bin_len={bin_len})");
        assert_eq!(c.ret_offset, r.ret_offset, "row15 return pointer differs");
        for (impl_name, o) in [("C", &c), ("Rust", &r)] {
            assert!(
                o.buf[need..].iter().all(|&b| b == GUARD),
                "{impl_name} wrote past hex[bin_len*2] (bin_len={bin_len})"
            );
            assert_eq!(o.buf[need - 1], 0, "{impl_name} missing NUL terminator");
        }
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_unaligned_input_pointer() {
    let mut rng = Rng::new(SEED ^ 16);
    for bin_off in 0usize..=15 {
        for _ in 0..40 {
            let bin_len = rng.range(1, 128);
            let mut bin = vec![0u8; bin_len];
            rng.fill(&mut bin);
            assert_same(
                "row16",
                bin_len * 2 + 1 + 16,
                0,
                bin_len * 2 + 1,
                &bin,
                bin_off,
            );
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_returned_pointer_identity() {
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..300 {
        let bin_len = rng.range(0, 64);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        for hex_off in [0usize, 1, 3, 7, 8, 15] {
            let out_alloc = hex_off + bin_len * 2 + 1 + 8;
            let c = run_one(&cf, out_alloc, hex_off, bin_len * 2 + 1, &bin, 0);
            let r = run_one(&rf, out_alloc, hex_off, bin_len * 2 + 1, &bin, 0);
            assert_eq!(c.ret_offset, Some(hex_off as isize), "C returns hex");
            assert_eq!(r.ret_offset, Some(hex_off as isize), "Rust returns hex");
            assert_eq!(c.buf, r.buf, "row17 buffers differ");
        }
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_large_inputs() {
    let mut rng = Rng::new(SEED ^ 18);
    for bin_len in [4096usize, 65536] {
        for _ in 0..8 {
            let mut bin = vec![0u8; bin_len];
            rng.fill(&mut bin);
            assert_same_simple("row18", &bin, bin_len * 2 + 1, 16);
        }
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_huge_but_nonoverflowing_hex_maxlen() {
    // hex_maxlen = SIZE_MAX/2 - 1 passes the check (`hex_maxlen <= bin_len*2`
    // is false) even though the real buffer is small: the C code only ever
    // writes bin_len*2 + 1 bytes, so this is safe and must match.
    let huge = usize::MAX / 2 - 1;
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..200 {
        let bin_len = rng.range(0, 256);
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        assert_same("row19", bin_len * 2 + 1 + 16, 0, huge, &bin, 0);
    }
    // Also usize::MAX itself as hex_maxlen (still a valid, accepted value).
    assert_same("row19-max", 33, 0, usize::MAX, &[1, 2, 3, 4, 5, 6, 7, 8], 0);
}

// ---------------------------------------------------------------- row 20
#[test]
fn row20_repeated_calls_same_buffer_decreasing_len() {
    // No hidden state: reuse one buffer with shrinking bin_len and require the
    // stale trailing bytes to be identical between C and Rust.
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    let mut rng = Rng::new(SEED ^ 20);

    for _ in 0..100 {
        let max_len = 64usize;
        let mut bin = vec![0u8; max_len];
        rng.fill(&mut bin);
        let alloc = max_len * 2 + 1 + 8;

        let mut cbuf = vec![GUARD; alloc];
        let mut rbuf = vec![GUARD; alloc];

        for bin_len in (0..=max_len).rev() {
            unsafe {
                let cret = cf(cbuf.as_mut_ptr() as *mut i8, alloc, bin.as_ptr(), bin_len);
                let rret = rf(rbuf.as_mut_ptr() as *mut i8, alloc, bin.as_ptr(), bin_len);
                assert_eq!(cret as *mut u8, cbuf.as_mut_ptr());
                assert_eq!(rret as *mut u8, rbuf.as_mut_ptr());
            }
            assert_eq!(
                cbuf, rbuf,
                "row20 divergence after call with bin_len={bin_len}"
            );
        }
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn row21_hex_and_bin_same_allocation_nonoverlapping() {
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    let mut rng = Rng::new(SEED ^ 21);

    for _ in 0..300 {
        let bin_len = rng.range(1, 128);
        // Layout: [ hex: bin_len*2+1 ][ pad 8 ][ bin: bin_len ]
        let hex_span = bin_len * 2 + 1;
        let bin_at = hex_span + 8;
        let alloc = bin_at + bin_len;

        let mut src = vec![0u8; bin_len];
        rng.fill(&mut src);

        let mut cbuf = vec![GUARD; alloc];
        let mut rbuf = vec![GUARD; alloc];
        cbuf[bin_at..].copy_from_slice(&src);
        rbuf[bin_at..].copy_from_slice(&src);

        unsafe {
            let p = cbuf.as_mut_ptr();
            cf(p as *mut i8, hex_span, p.add(bin_at), bin_len);
            let p = rbuf.as_mut_ptr();
            rf(p as *mut i8, hex_span, p.add(bin_at), bin_len);
        }
        assert_eq!(cbuf, rbuf, "row21 divergence (bin_len={bin_len})");
    }
}
