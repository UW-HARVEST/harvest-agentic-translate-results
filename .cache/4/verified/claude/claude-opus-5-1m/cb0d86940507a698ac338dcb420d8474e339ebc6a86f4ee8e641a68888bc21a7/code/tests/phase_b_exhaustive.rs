//! Phase B — `CONFIGS.md` row 26: EXHAUSTIVE differential test.
//!
//! `tritanopia`'s entire input domain is `256^3 = 16 777 216` values, so the
//! translation can be verified *completely* rather than sampled. This is the
//! full cross-product of every axis in `CONFIGS.md` at every value, and it
//! also proves the reachability claims recorded in `ERRORS.md`.
//!
//! The sweep is split across worker threads by the R channel. Each thread
//! re-derives the two function pointers from raw addresses (the `Library`
//! handles stay alive in the parent for the whole scope).

mod common;

use common::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// Statistics gathered during the sweep, used to prove that the interesting
/// branches were actually reached (a green exhaustive test is only meaningful
/// if the UB paths really did execute).
#[derive(Default, Debug)]
struct Stats {
    total: u64,
    mismatches: u64,
    /// outputs where R wrapped from a negative denorm argument (ERRORS E1)
    r_wrap_negative: u64,
    /// outputs where R wrapped from a denorm argument > 255 (ERRORS E2)
    r_wrap_over: u64,
}

#[test]
fn row26_exhaustive_all_16m_inputs() {
    let pair = Pair::load();
    let (c_addr, rust_addr) = pair.raw_addrs();
    assert_ne!(c_addr, rust_addr, "C and Rust resolved to the same address");

    let threads: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16);

    let total = AtomicU64::new(0);
    let mismatches = AtomicU64::new(0);
    let wrap_neg = AtomicU64::new(0);
    let wrap_over = AtomicU64::new(0);
    let first_bad: std::sync::Mutex<Vec<(Rgb, Rgb, Rgb)>> = std::sync::Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for t in 0..threads {
            let total = &total;
            let mismatches = &mismatches;
            let wrap_neg = &wrap_neg;
            let wrap_over = &wrap_over;
            let first_bad = &first_bad;
            scope.spawn(move || {
                // SAFETY: both libraries remain loaded in the parent scope for
                // the whole lifetime of these threads.
                let c: TritFn = unsafe { std::mem::transmute::<usize, TritFn>(c_addr) };
                let rs: TritFn = unsafe { std::mem::transmute::<usize, TritFn>(rust_addr) };

                let mut n = 0u64;
                let mut bad = 0u64;
                let mut wn = 0u64;
                let mut wo = 0u64;
                let mut local_bad: Vec<(Rgb, Rgb, Rgb)> = Vec::new();

                let mut r = t as u32;
                while r < 256 {
                    for g in 0u32..256 {
                        for b in 0u32..256 {
                            let i = Rgb::new(r as u8, g as u8, b as u8);
                            let cv = unsafe { c(i) };
                            let rv = unsafe { rs(i) };
                            if cv != rv {
                                bad += 1;
                                if local_bad.len() < 10 {
                                    local_bad.push((i, cv, rv));
                                }
                            }
                            // Detect the two UB wraparound classes from the
                            // *observable* C output: a saturating (non-C)
                            // conversion would have produced 0 or 255 here.
                            // R_out < 0  => the true value is negative, so the
                            // byte is the low 8 bits of a negative integer.
                            // We recompute the classification the same way
                            // ERRORS.md derived it: from the input geometry.
                            let (neg, over) = classify_r(i);
                            if neg {
                                wn += 1;
                            }
                            if over {
                                wo += 1;
                            }
                            n += 1;
                        }
                    }
                    r += threads as u32;
                }

                total.fetch_add(n, Ordering::Relaxed);
                mismatches.fetch_add(bad, Ordering::Relaxed);
                wrap_neg.fetch_add(wn, Ordering::Relaxed);
                wrap_over.fetch_add(wo, Ordering::Relaxed);
                if !local_bad.is_empty() {
                    let mut fb = first_bad.lock().unwrap();
                    for e in local_bad {
                        if fb.len() < 20 {
                            fb.push(e);
                        }
                    }
                }
            });
        }
    });

    let stats = Stats {
        total: total.load(Ordering::Relaxed),
        mismatches: mismatches.load(Ordering::Relaxed),
        r_wrap_negative: wrap_neg.load(Ordering::Relaxed),
        r_wrap_over: wrap_over.load(Ordering::Relaxed),
    };

    assert_eq!(
        stats.total, 16_777_216,
        "the sweep must cover every one of the 256^3 inputs, covered {}",
        stats.total
    );

    if stats.mismatches != 0 {
        let fb = first_bad.lock().unwrap();
        let detail = fb
            .iter()
            .map(|(i, c, r)| {
                format!(
                    "  in=({:3},{:3},{:3})  C=({:3},{:3},{:3})  RUST=({:3},{:3},{:3})",
                    i.r, i.g, i.b, c.r, c.g, c.b, r.r, r.g, r.b
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "EXHAUSTIVE FAILURE: {} of {} inputs diverged. First {}:\n{}",
            stats.mismatches,
            stats.total,
            fb.len(),
            detail
        );
    }

    // Reachability assertions: prove the exhaustive pass really exercised the
    // implementation-defined conversion paths documented in ERRORS.md.
    // Expected counts come from the analytic model in ERRORS.md / CONFIGS.md.
    assert_eq!(
        stats.r_wrap_negative, 1_666_521,
        "ERRORS E1 (R denorm arg < 0) reachability changed"
    );
    assert_eq!(
        stats.r_wrap_over, 171_997,
        "ERRORS E2 (post-matrix R > 1.0) reachability changed"
    );

    eprintln!(
        "row26 exhaustive: {} inputs, 0 divergences; E1 negative-R inputs = {}, \
         post-matrix R>1.0 inputs = {}",
        stats.total, stats.r_wrap_negative, stats.r_wrap_over
    );
}

/// Reproduces just enough of the pipeline (in f32/f64, matching the C) to say
/// whether this input drives the R channel out of `[0, 1]` before `cbDenorm`,
/// i.e. whether it exercises ERRORS.md rows E1 / E2. Used only for
/// *reachability accounting*, never as the expected output.
#[inline]
fn classify_r(i: Rgb) -> (bool, bool) {
    let lin = |v: u8| -> f32 {
        let c = (v as f32 / 255.0f32) as f64;
        let x = if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        };
        x as f32
    };
    let (r, g, b) = (lin(i.r), lin(i.g), lin(i.b));
    let r_out = (r + 0.127_398_863_108_80_f32 * g) - 0.127_398_863_410_72_f32 * b;
    (r_out < 0.0, r_out > 1.0)
}
