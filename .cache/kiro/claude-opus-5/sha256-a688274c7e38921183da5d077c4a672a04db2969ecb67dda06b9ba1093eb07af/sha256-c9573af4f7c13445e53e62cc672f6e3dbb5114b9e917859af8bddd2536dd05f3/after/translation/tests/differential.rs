//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through their exported symbols with many randomized inputs
//! (fixed seed) and asserts byte-identical results: the full contents of every
//! buffer the call may have written for `fma_array`, and the exact stdout bytes
//! for `driver`.

mod common;

use common::*;
use std::os::raw::c_int;

/// The pointer arrangement handed to `fma_array`. `mul1`/`mul2`/`add` are
/// `const int *` in C but `inner` aliases all four onto one buffer, so
/// aliasing is a real input shape that has to be diffed.
#[derive(Copy, Clone, Debug)]
enum Alias {
    Distinct,
    OutIsMul1,
    OutIsMul2,
    OutIsAdd,
    Mul1IsMul2,
    Full,
}

/// Sentinel prefilled into the `out` buffer so "the callee wrote nothing" is
/// observable rather than indistinguishable from "wrote zeros".
const SENTINEL: c_int = -0x5EED_BEEF;

/// Run one `fma_array` call on `which` implementation and return every buffer
/// that was visible to it, so unexpected writes are caught too.
fn run_fma(
    which: Impl,
    alias: Alias,
    len: c_int,
    v1: &[c_int],
    v2: &[c_int],
    va: &[c_int],
    out_len: usize,
) -> Vec<Vec<c_int>> {
    let f = fma_array_of(which);

    match alias {
        Alias::Distinct => {
            let mut out = vec![SENTINEL; out_len];
            let m1 = v1.to_vec();
            let m2 = v2.to_vec();
            let a = va.to_vec();
            unsafe { f(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len) };
            vec![out, m1.clone(), m2.clone(), a.clone()]
        }
        Alias::OutIsMul1 => {
            let mut buf = v1.to_vec();
            let m2 = v2.to_vec();
            let a = va.to_vec();
            let p = buf.as_mut_ptr();
            unsafe { f(p, p, m2.as_ptr(), a.as_ptr(), len) };
            vec![buf, m2.clone(), a.clone()]
        }
        Alias::OutIsMul2 => {
            let mut buf = v2.to_vec();
            let m1 = v1.to_vec();
            let a = va.to_vec();
            let p = buf.as_mut_ptr();
            unsafe { f(p, m1.as_ptr(), p, a.as_ptr(), len) };
            vec![buf, m1.clone(), a.clone()]
        }
        Alias::OutIsAdd => {
            let mut buf = va.to_vec();
            let m1 = v1.to_vec();
            let m2 = v2.to_vec();
            let p = buf.as_mut_ptr();
            unsafe { f(p, m1.as_ptr(), m2.as_ptr(), p, len) };
            vec![buf, m1.clone(), m2.clone()]
        }
        Alias::Mul1IsMul2 => {
            let mut out = vec![SENTINEL; out_len];
            let mut m = v1.to_vec();
            let a = va.to_vec();
            let pm = m.as_mut_ptr();
            unsafe { f(out.as_mut_ptr(), pm, pm, a.as_ptr(), len) };
            vec![out, m.clone(), a.clone()]
        }
        Alias::Full => {
            let mut buf = v1.to_vec();
            let p = buf.as_mut_ptr();
            unsafe { f(p, p, p, p, len) };
            vec![buf]
        }
    }
}

/// `CONFIGS.md` row driver for `fma_array`: `iters` randomized inputs, C vs
/// Rust, byte-for-byte.
fn row_fma(row: &str, seed: u64, alias: Alias, len: c_int, dist: Dist, iters: usize) {
    preload_both();
    let n = if len > 0 { len as usize } else { 0 };
    // For len <= 0 still hand the callee a real buffer, so an erroneous write
    // would be caught instead of faulting.
    let out_len = n.max(8);
    let mut rng = Rng::new(seed);

    for it in 0..iters {
        let v1 = dist.vec(&mut rng, out_len);
        let v2 = dist.vec(&mut rng, out_len);
        let va = dist.vec(&mut rng, out_len);

        let c = run_fma(Impl::C, alias, len, &v1, &v2, &va, out_len);
        let r = run_fma(Impl::Rust, alias, len, &v1, &v2, &va, out_len);

        assert_eq!(
            c.len(),
            r.len(),
            "[{row}] internal harness mismatch on buffer count"
        );
        for (bi, (cb, rb)) in c.iter().zip(r.iter()).enumerate() {
            if cb != rb {
                let first = cb
                    .iter()
                    .zip(rb.iter())
                    .position(|(x, y)| x != y)
                    .unwrap_or(0);
                panic!(
                    "[{row}] divergence: alias={alias:?} len={len} dist={dist:?} \
                     iter={it} buffer#{bi} index={first}\n  \
                     mul1[{first}]={:?} mul2[{first}]={:?} add[{first}]={:?}\n  \
                     C   = {:?}\n  Rust= {:?}",
                    v1.get(first),
                    v2.get(first),
                    va.get(first),
                    cb.get(first),
                    rb.get(first),
                );
            }
        }

        // len <= 0 must leave the destination untouched in both.
        if len <= 0 {
            if let Alias::Distinct = alias {
                assert!(
                    c[0].iter().all(|&x| x == SENTINEL),
                    "[{row}] C wrote to `out` despite len={len}"
                );
                assert!(
                    r[0].iter().all(|&x| x == SENTINEL),
                    "[{row}] Rust wrote to `out` despite len={len}"
                );
            }
        }
    }
}

/// `CONFIGS.md` row driver for `driver`: compares the captured stdout bytes.
fn row_driver(row: &str, seed: u64, len: c_int, dist: Dist, iters: usize) {
    preload_both();
    let dc = driver_of(Impl::C);
    let dr = driver_of(Impl::Rust);
    let n = if len > 0 { len as usize } else { 0 };
    let mut rng = Rng::new(seed);

    for it in 0..iters {
        let data = dist.vec(&mut rng, n.max(1));
        let ptr = data.as_ptr();

        let c_out = capture_stdout(|| unsafe { dc(ptr, len) });
        let r_out = capture_stdout(|| unsafe { dr(ptr, len) });

        if c_out != r_out {
            let first = c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(c_out.len().min(r_out.len()));
            panic!(
                "[{row}] stdout divergence: len={len} dist={dist:?} iter={it} \
                 first differing byte at {first}\n  data={:?}\n  \
                 C   ({} bytes) = {:?}\n  Rust({} bytes) = {:?}",
                &data[..n.min(16)],
                c_out.len(),
                String::from_utf8_lossy(&c_out[..c_out.len().min(400)]),
                r_out.len(),
                String::from_utf8_lossy(&r_out[..r_out.len().min(400)]),
            );
        }

        if len <= 0 {
            assert!(
                c_out.is_empty(),
                "[{row}] C printed {:?} for len={len}",
                String::from_utf8_lossy(&c_out)
            );
            assert!(
                r_out.is_empty(),
                "[{row}] Rust printed {:?} for len={len}",
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 1-20 — `fma_array`, the low-level entry point.
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_fma_len0_distinct_small() {
    row_fma("cfg_01", 0x0101, Alias::Distinct, 0, Dist::Small, 200);
}

#[test]
fn cfg_02_fma_len1_distinct_small() {
    row_fma("cfg_02", 0x0202, Alias::Distinct, 1, Dist::Small, 500);
}

#[test]
fn cfg_03_fma_len2_distinct_small() {
    row_fma("cfg_03", 0x0303, Alias::Distinct, 2, Dist::Small, 500);
}

#[test]
fn cfg_04_fma_len3_distinct_small() {
    row_fma("cfg_04", 0x0404, Alias::Distinct, 3, Dist::Small, 500);
}

#[test]
fn cfg_05_fma_len8_distinct_small() {
    row_fma("cfg_05", 0x0505, Alias::Distinct, 8, Dist::Small, 400);
}

#[test]
fn cfg_06_fma_len17_distinct_small() {
    row_fma("cfg_06", 0x0606, Alias::Distinct, 17, Dist::Small, 400);
}

#[test]
fn cfg_07_fma_len64_distinct_small() {
    row_fma("cfg_07", 0x0707, Alias::Distinct, 64, Dist::Small, 300);
}

#[test]
fn cfg_08_fma_len1000_distinct_small() {
    row_fma("cfg_08", 0x0808, Alias::Distinct, 1000, Dist::Small, 100);
}

#[test]
fn cfg_09_fma_len64_distinct_full() {
    row_fma("cfg_09", 0x0909, Alias::Distinct, 64, Dist::Full, 300);
}

#[test]
fn cfg_10_fma_len1000_distinct_full() {
    row_fma("cfg_10", 0x0A0A, Alias::Distinct, 1000, Dist::Full, 100);
}

#[test]
fn cfg_11_fma_len64_distinct_boundary() {
    row_fma("cfg_11", 0x0B0B, Alias::Distinct, 64, Dist::Boundary, 300);
}

#[test]
fn cfg_12_fma_len1_distinct_boundary() {
    row_fma("cfg_12", 0x0C0C, Alias::Distinct, 1, Dist::Boundary, 1000);
}

#[test]
fn cfg_13_fma_len64_out_is_mul1_full() {
    row_fma("cfg_13", 0x0D0D, Alias::OutIsMul1, 64, Dist::Full, 300);
}

#[test]
fn cfg_14_fma_len64_out_is_mul2_full() {
    row_fma("cfg_14", 0x0E0E, Alias::OutIsMul2, 64, Dist::Full, 300);
}

#[test]
fn cfg_15_fma_len64_out_is_add_full() {
    row_fma("cfg_15", 0x0F0F, Alias::OutIsAdd, 64, Dist::Full, 300);
}

#[test]
fn cfg_16_fma_len64_mul1_is_mul2_full() {
    row_fma("cfg_16", 0x1010, Alias::Mul1IsMul2, 64, Dist::Full, 300);
}

#[test]
fn cfg_17_fma_len64_full_alias_small() {
    row_fma("cfg_17", 0x1111, Alias::Full, 64, Dist::Small, 300);
}

#[test]
fn cfg_18_fma_len1000_full_alias_full() {
    row_fma("cfg_18", 0x1212, Alias::Full, 1000, Dist::Full, 100);
}

#[test]
fn cfg_19_fma_len17_full_alias_boundary() {
    row_fma("cfg_19", 0x1313, Alias::Full, 17, Dist::Boundary, 400);
}

#[test]
fn cfg_20_fma_negative_len_distinct() {
    row_fma("cfg_20/-1", 0x1414, Alias::Distinct, -1, Dist::Full, 100);
    row_fma("cfg_20/-7", 0x1415, Alias::Distinct, -7, Dist::Full, 100);
    row_fma(
        "cfg_20/INT_MIN",
        0x1416,
        Alias::Distinct,
        c_int::MIN,
        Dist::Full,
        100,
    );
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 21-32 — `driver`, the wrapper (stdout side effect).
// ---------------------------------------------------------------------------

#[test]
fn cfg_21_driver_len0() {
    row_driver("cfg_21", 0x2101, 0, Dist::Small, 50);
}

#[test]
fn cfg_22_driver_len1_small() {
    row_driver("cfg_22", 0x2202, 1, Dist::Small, 200);
}

#[test]
fn cfg_23_driver_len2_small() {
    row_driver("cfg_23", 0x2303, 2, Dist::Small, 200);
}

#[test]
fn cfg_24_driver_len3_small() {
    row_driver("cfg_24", 0x2404, 3, Dist::Small, 200);
}

#[test]
fn cfg_25_driver_len8_small() {
    row_driver("cfg_25", 0x2505, 8, Dist::Small, 150);
}

#[test]
fn cfg_26_driver_len17_small() {
    row_driver("cfg_26", 0x2606, 17, Dist::Small, 150);
}

#[test]
fn cfg_27_driver_len64_small() {
    row_driver("cfg_27", 0x2707, 64, Dist::Small, 100);
}

#[test]
fn cfg_28_driver_len1000_small() {
    row_driver("cfg_28", 0x2808, 1000, Dist::Small, 40);
}

#[test]
fn cfg_29_driver_len64_full() {
    row_driver("cfg_29", 0x2909, 64, Dist::Full, 100);
}

#[test]
fn cfg_30_driver_len1000_full() {
    row_driver("cfg_30", 0x2A0A, 1000, Dist::Full, 40);
}

#[test]
fn cfg_31_driver_len64_boundary() {
    row_driver("cfg_31", 0x2B0B, 64, Dist::Boundary, 100);
}

#[test]
fn cfg_32_driver_len1_boundary() {
    row_driver("cfg_32", 0x2C0C, 1, Dist::Boundary, 300);
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 33 — composed pipeline: fma_array output feeds driver.
// ---------------------------------------------------------------------------

#[test]
fn cfg_33_composed_fma_then_driver() {
    preload_both();
    const LEN: c_int = 64;
    let n = LEN as usize;
    let mut rng = Rng::new(0x3333);

    let fc = fma_array_of(Impl::C);
    let fr = fma_array_of(Impl::Rust);
    let dc = driver_of(Impl::C);
    let dr = driver_of(Impl::Rust);

    for it in 0..100 {
        let v1 = Dist::Full.vec(&mut rng, n);
        let v2 = Dist::Full.vec(&mut rng, n);
        let va = Dist::Full.vec(&mut rng, n);

        let mut c_mid = vec![SENTINEL; n];
        let mut r_mid = vec![SENTINEL; n];
        unsafe {
            fc(
                c_mid.as_mut_ptr(),
                v1.as_ptr(),
                v2.as_ptr(),
                va.as_ptr(),
                LEN,
            );
            fr(
                r_mid.as_mut_ptr(),
                v1.as_ptr(),
                v2.as_ptr(),
                va.as_ptr(),
                LEN,
            );
        }
        assert_eq!(c_mid, r_mid, "[cfg_33] fma_array stage diverged, iter={it}");

        // Feed each implementation's own intermediate into its own `driver`,
        // so a divergence anywhere in the chain shows up.
        let cp = c_mid.as_ptr();
        let rp = r_mid.as_ptr();
        let c_out = capture_stdout(|| unsafe { dc(cp, LEN) });
        let r_out = capture_stdout(|| unsafe { dr(rp, LEN) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
            "[cfg_33] driver stage diverged, iter={it}"
        );
    }
}
