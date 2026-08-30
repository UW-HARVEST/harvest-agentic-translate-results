//! Level 3b: the `cp_dynamic` code-length overrun.
//!
//! `cp_dynamic` re-tests `n < nlit + ndst` only *between* run-length groups, so
//! a symbol 18 can push `n` up to 137 entries past the end of the C code's
//! `uint8_t lens[288 + 32]`. Those writes land on the rest of the stack frame:
//! `lenlens`, `sym`, `nlen`, `ndst`, `nlit`, the three loop counters and `n`
//! itself -- and since the loop condition and the closing `cp_build` calls
//! re-read `nlit`/`ndst` from the frame, the corruption feeds back into the
//! decode.
//!
//! The translation reproduces this by laying its own frame out the way the C
//! compiler does (see `CpDynamicFrame`), so these are the regression tests for
//! that emulation. The sweep walks the overrun depth one entry at a time so
//! every clobbered field is covered.

mod harness;

use harness::deflate::*;
use harness::Differ;

const PAD: usize = 64;

/// Code-length alphabet using symbols 0, 2 and 18.
/// Lengths 2, 2, 1 give a Kraft sum of 1/4 + 1/4 + 1/2 = 1, i.e. a complete
/// code, so `cp_decode` resolves it without tripping its own assertion.
fn cl_lens_0_2_18() -> Vec<u8> {
    let mut v = vec![0u8; 19];
    v[0] = 2;
    v[2] = 2;
    v[18] = 1;
    v
}

/// Code-length alphabet using symbols 0, 2, 16 and 18 (lengths 2, 3, 3, 1).
/// Kraft: 1/4 + 1/8 + 1/8 + 1/2 = 1.
fn cl_lens_with_16() -> Vec<u8> {
    let mut v = vec![0u8; 19];
    v[0] = 2;
    v[2] = 3;
    v[16] = 3;
    v[18] = 1;
    v
}

/// Builds a stream whose code-length sequence puts non-zero lengths at
/// positions 0, 1, 2 and 280, fills up to `stop`, then emits one symbol 18 with
/// a run of `run` entries -- overshooting HLIT + HDIST by `stop + run - (hlit +
/// hdist)`.
///
/// The length at 280 is what makes a corrupted `nlit` observable: if the
/// overrun drops `nlit` to 256, symbol 280 falls out of the literal tree and
/// the block decodes differently.
fn overrun_stream(hlit: usize, hdist: usize, stop: usize, run: usize) -> Vec<u8> {
    assert!(stop >= 282 && stop <= 320);
    assert!((11..=138).contains(&run));
    let mut syms: Vec<(usize, u32, u32)> = Vec::new();
    // lens[0..3] = 2
    for _ in 0..3 {
        syms.push((2, 0, 0));
    }
    // zeros up to 280
    for _ in 3..280 {
        syms.push((0, 0, 0));
    }
    // lens[280] = 2
    syms.push((2, 0, 0));
    // zeros up to `stop`
    for _ in 281..stop {
        syms.push((0, 0, 0));
    }
    // the overshooting run
    syms.push((18, (run - 11) as u32, 7));

    let mut w = BitWriter::new();
    write_dynamic_header_raw(&mut w, true, hlit, hdist, &cl_lens_0_2_18(), &syms);
    with_padding(w.finish(), PAD)
}

#[test]
fn code_length_overrun_depth_sweep() {
    // HLIT + HDIST = 320 is the maximum, so `lens` is exactly full at n == 320
    // and every entry past that is an overrun.
    let mut d = Differ::new();
    for run in 11..=138usize {
        let stream = overrun_stream(288, 32, 319, run);
        let overshoot = 319 + run - 320;
        d.check(
            &format!("overrun by {overshoot} (run={run})"),
            &stream,
            0,
            256,
        );
    }
    d.finish("cp_dynamic overrun depth sweep");
}

#[test]
fn code_length_overrun_start_position_sweep() {
    // Move the point at which the run starts, so the run's own extra bits and
    // the position of the first out-of-bounds write vary independently.
    let mut d = Differ::new();
    for stop in 282..=320usize {
        for run in [11usize, 12, 20, 64, 100, 137, 138] {
            let stream = overrun_stream(288, 32, stop, run);
            d.check(&format!("stop={stop} run={run}"), &stream, 0, 256);
        }
    }
    d.finish("cp_dynamic overrun start sweep");
}

#[test]
fn code_length_overrun_smaller_hlit_hdist() {
    // With a smaller HLIT + HDIST the loop stops earlier, so the overrun starts
    // from a lower `n` and a *shorter* run is enough to reach the frame.
    let mut d = Differ::new();
    for (hlit, hdist) in [
        (257usize, 1usize),
        (257, 32),
        (270, 16),
        (280, 1),
        (288, 1),
        (288, 16),
        (285, 30),
    ] {
        let total = hlit + hdist;
        for run in [11usize, 40, 100, 138] {
            // Start the run just before the declared end.
            let stop = (total - 1).clamp(282, 320);
            let stream = overrun_stream(hlit, hdist, stop, run);
            d.check(
                &format!("hlit={hlit} hdist={hdist} stop={stop} run={run}"),
                &stream,
                0,
                256,
            );
        }
    }
    d.finish("cp_dynamic overrun with smaller HLIT/HDIST");
}

#[test]
fn code_length_overrun_with_repeat_symbol() {
    // Symbol 16 copies the *previous* length, so it writes non-zero bytes. That
    // is what can leave a loop counter non-zero and, in the C original, spin
    // forever. A symbol 18 run first pushes `nlit`/`ndst` upward so the loop
    // keeps going and the 16 runs can reach deep into the frame.
    let mut d = Differ::new();
    for first_run in [11usize, 30, 45, 46, 47, 60, 100, 138] {
        for repeats in 0..8usize {
            let mut syms: Vec<(usize, u32, u32)> = Vec::new();
            for _ in 0..3 {
                syms.push((2, 0, 0));
            }
            for _ in 3..280 {
                syms.push((0, 0, 0));
            }
            syms.push((2, 0, 0));
            for _ in 281..319 {
                syms.push((0, 0, 0));
            }
            syms.push((18, (first_run - 11) as u32, 7));
            for _ in 0..repeats {
                // symbol 16, extra 3 -> repeat the previous length 6 times
                syms.push((16, 3, 2));
            }
            let mut w = BitWriter::new();
            write_dynamic_header_raw(&mut w, true, 288, 32, &cl_lens_with_16(), &syms);
            let stream = with_padding(w.finish(), PAD);
            d.check(
                &format!("first_run={first_run} repeats={repeats}"),
                &stream,
                0,
                256,
            );
        }
    }
    d.finish("cp_dynamic overrun with symbol 16");
}

#[test]
fn code_length_overrun_across_offsets_and_output_sizes() {
    // The same overruns under a shifted input pointer and different output
    // buffer sizes, since the corrupted trees interact with `cp_block`'s guards.
    let mut d = Differ::new();
    for run in [11usize, 37, 44, 45, 50, 57, 138] {
        let stream = overrun_stream(288, 32, 319, run);
        for offset in 0..8usize {
            for out_bytes in [0usize, 1, 7, 64, 1000] {
                d.check(
                    &format!("run={run} off={offset} out={out_bytes}"),
                    &stream,
                    offset,
                    out_bytes,
                );
            }
        }
    }
    d.finish("cp_dynamic overrun offsets/output sizes");
}
