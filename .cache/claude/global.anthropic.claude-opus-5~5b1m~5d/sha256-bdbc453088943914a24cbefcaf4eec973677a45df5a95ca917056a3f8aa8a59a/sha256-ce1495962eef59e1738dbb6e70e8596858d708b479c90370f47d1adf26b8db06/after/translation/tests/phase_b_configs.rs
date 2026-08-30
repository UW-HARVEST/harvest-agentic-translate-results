//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every row runs many randomized inputs from a fixed seed. Both the low-level
//! entry point (`fma_array`) and the convenience wrapper (`driver`) are driven
//! through their `.so` exports.

mod common;

use common::*;
use std::ffi::c_int;

/// Iterations per randomized row.
const ITERS: usize = 200;

// ===========================================================================
// C1..C10 — fma_array, distinct buffers, varying length and value shape
// ===========================================================================

#[test]
fn cfg_c1_fma_len_zero_distinct() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..ITERS {
        let lay = Layout::distinct(8);
        let init = Values::Full.fill(&mut rng, lay.arena);
        // len == 0: nothing must be written, sentinel pattern must survive.
        diff_fma("C1", &init, lay, 0);
    }
}

#[test]
fn cfg_c2_fma_len_one() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..ITERS {
        let lay = Layout::distinct(1);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C2", &init, lay, 1);
    }
}

#[test]
fn cfg_c3_fma_len_two() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS {
        let lay = Layout::distinct(2);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C3", &init, lay, 2);
    }
}

#[test]
fn cfg_c4_fma_len_small_random() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let len = rng.range(3, 64) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C4", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c5_fma_len_medium_random() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..50 {
        let len = rng.range(65, 1024) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C5", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c6_fma_len_large() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..20 {
        let len = 4096usize;
        let lay = Layout::distinct(len);
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C6", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c7_fma_small_values() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Small.fill(&mut rng, lay.arena);
        diff_fma("C7", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c8_fma_boundary_values() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Boundary.fill(&mut rng, lay.arena);
        diff_fma("C8", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c9_fma_all_zeros() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Zeros.fill(&mut rng, lay.arena);
        diff_fma("C9", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c10_fma_all_ones() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let lay = Layout::distinct(len);
        let init = Values::Ones.fill(&mut rng, lay.arena);
        diff_fma("C10", &init, lay, len as c_int);
    }
}

// ===========================================================================
// C11..C18 — fma_array aliasing patterns
// ===========================================================================

fn aliasing_row(label: &str, seed: u64, mk: fn(usize) -> Layout, vals: Values) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let len = rng.range(1, 96) as usize;
        let lay = mk(len);
        let init = vals.fill(&mut rng, lay.arena);
        diff_fma(label, &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c11_fma_out_eq_mul1() {
    aliasing_row("C11", SEED ^ 11, Layout::out_eq_mul1, Values::Full);
}

#[test]
fn cfg_c12_fma_out_eq_mul2() {
    aliasing_row("C12", SEED ^ 12, Layout::out_eq_mul2, Values::Full);
}

#[test]
fn cfg_c13_fma_out_eq_add() {
    aliasing_row("C13", SEED ^ 13, Layout::out_eq_add, Values::Full);
}

#[test]
fn cfg_c14_fma_mul1_eq_mul2_square() {
    aliasing_row("C14", SEED ^ 14, Layout::mul1_eq_mul2, Values::Full);
}

#[test]
fn cfg_c15_fma_all_same_as_inner_does() {
    aliasing_row("C15", SEED ^ 15, Layout::all_same, Values::Full);
}

#[test]
fn cfg_c16_fma_all_same_boundary_values() {
    aliasing_row("C16", SEED ^ 16, Layout::all_same, Values::Boundary);
}

#[test]
fn cfg_c17_fma_mul1_shifted_read_after_write() {
    aliasing_row("C17", SEED ^ 17, Layout::mul1_shifted, Values::Full);
    aliasing_row("C17b", SEED ^ 0x17b, Layout::mul1_shifted, Values::Boundary);
}

#[test]
fn cfg_c18_fma_out_shifted_write_after_read() {
    aliasing_row("C18", SEED ^ 18, Layout::out_shifted, Values::Full);
    aliasing_row("C18b", SEED ^ 0x18b, Layout::out_shifted, Values::Boundary);
}

// ===========================================================================
// C19 — fma_array width sweep across every aliasing pattern
// ===========================================================================

#[test]
fn cfg_c19_fma_width_sweep() {
    let mks: [(&str, fn(usize) -> Layout); 8] = [
        ("distinct", Layout::distinct),
        ("out=mul1", Layout::out_eq_mul1),
        ("out=mul2", Layout::out_eq_mul2),
        ("out=add", Layout::out_eq_add),
        ("mul1=mul2", Layout::mul1_eq_mul2),
        ("all_same", Layout::all_same),
        ("mul1_shift", Layout::mul1_shifted),
        ("out_shift", Layout::out_shifted),
    ];
    let mut rng = Rng::new(SEED ^ 19);
    for (name, mk) in mks {
        for &len in WIDTH_SWEEP.iter() {
            for vals in [Values::Full, Values::Boundary, Values::Small] {
                for _ in 0..12 {
                    // `Layout::*(0)` would give a degenerate arena; keep >= 1.
                    let lay = mk(len.max(1) as usize);
                    let init = vals.fill(&mut rng, lay.arena);
                    diff_fma(&format!("C19/{name}/len={len}"), &init, lay, len);
                }
            }
        }
    }
}

// ===========================================================================
// C20..C29 — driver
// ===========================================================================

#[test]
fn cfg_c20_driver_len_zero() {
    let out = diff_driver("C20", &[7, 8, 9], 0);
    assert!(out.is_empty(), "driver(len=0) must print nothing, got {out:?}");
}

#[test]
fn cfg_c21_driver_len_one() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..ITERS {
        let data = Values::Full.fill(&mut rng, 1);
        let out = diff_driver("C21", &data, 1);
        // Cross-check the formatting against the C semantics x*x + x.
        let expect = format!("{}\n", data[0].wrapping_mul(data[0]).wrapping_add(data[0]));
        assert_eq!(String::from_utf8_lossy(&out), expect, "C21 value {}", data[0]);
    }
}

#[test]
fn cfg_c22_driver_len_two() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..ITERS {
        let data = Values::Full.fill(&mut rng, 2);
        diff_driver("C22", &data, 2);
    }
}

#[test]
fn cfg_c23_driver_len_small_random() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..ITERS {
        let len = rng.range(3, 64) as usize;
        let data = Values::Full.fill(&mut rng, len);
        diff_driver("C23", &data, len as c_int);
    }
}

#[test]
fn cfg_c24_driver_len_medium_random() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..40 {
        let len = rng.range(65, 1024) as usize;
        let data = Values::Full.fill(&mut rng, len);
        diff_driver("C24", &data, len as c_int);
    }
}

#[test]
fn cfg_c25_driver_len_large() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..8 {
        let len = 4096usize;
        let data = Values::Full.fill(&mut rng, len);
        let out = diff_driver("C25", &data, len as c_int);
        assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), len);
    }
}

#[test]
fn cfg_c26_driver_small_values() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let data = Values::Small.fill(&mut rng, len);
        diff_driver("C26", &data, len as c_int);
    }
    // The exact hand-computed vector: x*x + x for {-2,-1,0,1,2}.
    let out = diff_driver("C26/fixed", &[-2, -1, 0, 1, 2], 5);
    assert_eq!(String::from_utf8_lossy(&out), "2\n0\n0\n2\n6\n");
}

#[test]
fn cfg_c27_driver_boundary_values() {
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let data = Values::Boundary.fill(&mut rng, len);
        diff_driver("C27", &data, len as c_int);
    }
    // INT_MIN must print with the full 11-character form.
    let out = diff_driver("C27/fixed", &[i32::MAX, i32::MIN, -1, 2], 4);
    assert_eq!(
        String::from_utf8_lossy(&out),
        "-2147483648\n-2147483648\n0\n6\n"
    );
}

#[test]
fn cfg_c28_driver_all_zeros() {
    for len in [1usize, 2, 5, 33, 100] {
        let data = vec![0i32; len];
        let out = diff_driver("C28", &data, len as c_int);
        assert_eq!(String::from_utf8_lossy(&out), "0\n".repeat(len));
    }
}

#[test]
fn cfg_c29_driver_width_sweep() {
    let mut rng = Rng::new(SEED ^ 29);
    for &len in WIDTH_SWEEP.iter() {
        for vals in [Values::Full, Values::Boundary, Values::Small, Values::Zeros] {
            for _ in 0..8 {
                let data = vals.fill(&mut rng, len.max(1) as usize);
                diff_driver(&format!("C29/len={len}"), &data, len);
            }
        }
    }
}

// ===========================================================================
// C30..C34 — pointer offset, composed pipeline, statelessness
// ===========================================================================

#[test]
fn cfg_c30_driver_data_at_offset() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..ITERS {
        let len = rng.range(1, 64) as usize;
        let pad_before = rng.range(1, 16) as usize;
        let pad_after = rng.range(1, 16) as usize;
        let data = Values::Full.fill(&mut rng, pad_before + len + pad_after);
        diff_driver_offset("C30", &data, pad_before, len as c_int);
    }
}

#[test]
fn cfg_c31_composed_pipeline_matches_low_level() {
    // `driver` is `memcpy` + `fma_array(out,out,out,out,len)` + printf loop.
    // Verify the composition by reproducing it from the low-level entry point:
    // whatever `fma_array` writes for the all-aliased layout must be exactly
    // what `driver` prints.
    let p = common::pair();
    let mut rng = Rng::new(SEED ^ 31);

    for _ in 0..120 {
        let len = rng.range(1, 200) as usize;
        let data = Values::Full.fill(&mut rng, len);

        // Low level, on each .so independently.
        for imp in [&p.c, &p.rust] {
            let mut buf = data.clone();
            let base = buf.as_mut_ptr();
            let f = imp.fma_sym();
            unsafe {
                f(
                    base,
                    base as *const c_int,
                    base as *const c_int,
                    base as *const c_int,
                    len as c_int,
                );
            }
            let expected: String = buf.iter().map(|v| format!("{v}\n")).collect();
            let got = diff_driver("C31", &data, len as c_int);
            assert_eq!(
                String::from_utf8_lossy(&got),
                expected,
                "C31: {} driver stdout disagrees with its own fma_array over len={len}",
                imp.name
            );
        }
    }
}

#[test]
fn cfg_c32_driver_stateless_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 32);
    // Interleave many different lengths/data on the same dlopen handles, then
    // replay the first case to prove no residual state accumulated.
    let first = Values::Full.fill(&mut rng, 7);
    let out_first = diff_driver("C32/first", &first, 7);

    for _ in 0..150 {
        let len = rng.range(0, 80) as usize;
        let data = Values::Full.fill(&mut rng, len.max(1));
        diff_driver("C32", &data, len as c_int);
    }

    let out_again = diff_driver("C32/replay", &first, 7);
    assert_eq!(out_first, out_again, "C32: driver is not stateless");
}

#[test]
fn cfg_c33_fma_stateless_repeated_calls() {
    let mks: [fn(usize) -> Layout; 6] = [
        Layout::distinct,
        Layout::out_eq_mul1,
        Layout::out_eq_mul2,
        Layout::out_eq_add,
        Layout::mul1_eq_mul2,
        Layout::all_same,
    ];
    let mut rng = Rng::new(SEED ^ 33);
    for i in 0..400 {
        let mk = mks[i % mks.len()];
        let len = rng.range(0, 70) as usize;
        let lay = mk(len.max(1));
        let init = Values::Full.fill(&mut rng, lay.arena);
        diff_fma("C33", &init, lay, len as c_int);
    }
}

#[test]
fn cfg_c34_driver_len_zero_nonempty_buffer() {
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..ITERS {
        let n = rng.range(1, 32) as usize;
        let data = Values::Full.fill(&mut rng, n);
        let out = diff_driver("C34", &data, 0);
        assert!(out.is_empty(), "C34: len=0 must print nothing");
    }
}
