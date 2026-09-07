//! High-volume property-style sweeps.
//!
//! `phase_b_valid.rs` covers each `CONFIGS.md` row with a few hundred inputs
//! under per-call stdout capture. These tests trade per-call granularity for
//! volume: a whole batch of calls runs under a single fd-1 redirection, so
//! hundreds of thousands of inputs per entry point can be compared. Both the
//! accumulated return values and the concatenated stdout must match exactly.
//!
//! Volume is controlled by `CTORUST_BULK` (default 100_000).

mod common;

use common::*;
use std::ffi::CString;

fn bulk() -> usize {
    std::env::var("CTORUST_BULK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000)
}

#[test]
fn b01_check_permissions_bulk() {
    let k = bulk() * 4;
    diff_batch("check_permissions bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB01);
        let f = raw(i).check_permissions;
        (0..k)
            .map(|_| {
                let (p, r) = (rng.any_i32(), rng.any_i32());
                unsafe { f(p, r) }
            })
            .collect::<Vec<i32>>()
    });
}

#[test]
fn b02_safe_add_bulk() {
    let k = bulk() * 2;
    diff_batch("safe_add bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB02);
        let f = raw(i).safe_add;
        (0..k)
            .map(|_| {
                let a = rng.any_i32();
                let b = rng.any_i32();
                // Mix granting and denying permission masks so both branches
                // (including the printed rejection) occur throughout the batch.
                let perms = match rng.next_u64() % 4 {
                    0 => 0o644,
                    1 => 0o600,
                    2 => rng.any_i32(),
                    _ => 0o444,
                };
                unsafe { f(a, b, perms) }
            })
            .collect::<Vec<i32>>()
    });
}

#[test]
fn b03_create_result_string_bulk() {
    let k = bulk();
    diff_batch("create_result_string bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB03);
        let f = raw(i).create_result_string;
        let mut out: Vec<Vec<u8>> = Vec::with_capacity(k);
        for _ in 0..k {
            let len = rng.range(0, 70) as usize;
            let op = rand_cstring(&mut rng, len);
            let val = rng.any_i32();
            let p = unsafe { f(op.as_ptr(), val) };
            if p.is_null() {
                out.push(Vec::new());
            } else {
                out.push(unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes().to_vec());
                unsafe { c_free(p) };
            }
        }
        out
    });
}

#[test]
fn b04_multiply_with_log_bulk() {
    let k = bulk();
    diff_batch("multiply_with_log bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB04);
        let f = raw(i).multiply_with_log;
        let mut out: Vec<(i32, Vec<u8>)> = Vec::with_capacity(k);
        for _ in 0..k {
            let (a, b) = (rng.any_i32(), rng.any_i32());
            let mut log: *mut CChar = std::ptr::null_mut();
            let r = unsafe { f(a, b, &mut log) };
            let bytes = if log.is_null() {
                Vec::new()
            } else {
                let v = unsafe { std::ffi::CStr::from_ptr(log) }.to_bytes().to_vec();
                unsafe { c_free(log) };
                v
            };
            out.push((r, bytes));
        }
        out
    });
}

#[test]
fn b05_copy_and_sum_bulk() {
    let k = bulk() / 4;
    // One shared, immutable buffer so both implementations read identical bytes.
    let mut buf: Vec<i32> = {
        let mut rng = Rng::with_seed(SEED ^ 0x0B0F_u64.wrapping_mul(3));
        (0..4096).map(|_| rng.any_i32()).collect()
    };
    let base = buf.as_mut_ptr();
    diff_batch("copy_and_sum bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB05);
        let f = raw(i).copy_and_sum;
        (0..k)
            .map(|_| {
                // Random sub-slice, random count, occasionally invalid.
                let off = rng.range(0, 4000) as isize;
                let maxc = 4096 - off as i64;
                let count = match rng.next_u64() % 8 {
                    0 => 0,
                    1 => -(rng.range(1, 4096) as i32), // malloc fails
                    _ => rng.range(0, maxc) as i32,
                };
                let ptr = if rng.next_u64() % 32 == 0 {
                    std::ptr::null_mut()
                } else {
                    unsafe { base.offset(off) }
                };
                unsafe { f(ptr, count) }
            })
            .collect::<Vec<i32>>()
    });
}

#[test]
fn b06_compare_operations_bulk() {
    let k = bulk();
    diff_batch("compare_operations bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB06);
        let f = raw(i).compare_operations;
        (0..k)
            .map(|_| {
                let la = rng.range(0, 12) as usize;
                let a = rand_cstring(&mut rng, la);
                // Half the time, derive `b` from `a` so shared prefixes and
                // exact equality both occur often.
                let b = match rng.next_u64() % 4 {
                    0 => a.clone(),
                    1 => {
                        let mut v = a.as_bytes().to_vec();
                        v.push(rng.byte().max(1));
                        CString::new(v).unwrap()
                    }
                    2 => {
                        let mut v = a.as_bytes().to_vec();
                        if !v.is_empty() {
                            let idx = rng.range(0, v.len() as i64 - 1) as usize;
                            v[idx] = rng.byte().max(1);
                        }
                        CString::new(v).unwrap()
                    }
                    _ => {
                        let lb = rng.range(0, 12) as usize;
                        rand_cstring(&mut rng, lb)
                    }
                };
                let (pa, pb) = match rng.next_u64() % 16 {
                    0 => (std::ptr::null(), b.as_ptr()),
                    1 => (a.as_ptr(), std::ptr::null()),
                    _ => (a.as_ptr(), b.as_ptr()),
                };
                unsafe { f(pa, pb) }
            })
            .collect::<Vec<i32>>()
    });
}

#[test]
fn b07_complexmode_bulk() {
    let k = bulk();
    diff_batch("complexmode bulk", |i| {
        let mut rng = Rng::with_seed(SEED ^ 0xB07);
        let f = raw(i).complexmode;
        (0..k)
            .map(|_| {
                // Mostly valid modes, with invalid ones interleaved.
                let mode = match rng.next_u64() % 6 {
                    0 => rng.any_i32(),
                    1 => rng.range(-4, 9) as i32,
                    m => m as i32 - 1, // 1..=4
                };
                let (a, b, c) = (rng.any_i32(), rng.any_i32(), rng.any_i32());
                unsafe { f(mode, a, b, c) }
            })
            .collect::<Vec<i32>>()
    });
}

#[test]
fn b08_complexmode_exhaustive_mode_window() {
    // Every mode in a wide window, each with a fixed set of operand triples.
    diff_batch("complexmode exhaustive modes", |i| {
        let f = raw(i).complexmode;
        let mut out = Vec::new();
        for mode in -600i32..=600 {
            for (a, b, c) in [
                (0, 0, 0),
                (1, 2, 3),
                (-1, -2, -3),
                (i32::MAX, i32::MIN, 1),
                (46341, 46341, i32::MAX),
            ] {
                out.push(unsafe { f(mode, a, b, c) });
            }
        }
        out
    });
}
