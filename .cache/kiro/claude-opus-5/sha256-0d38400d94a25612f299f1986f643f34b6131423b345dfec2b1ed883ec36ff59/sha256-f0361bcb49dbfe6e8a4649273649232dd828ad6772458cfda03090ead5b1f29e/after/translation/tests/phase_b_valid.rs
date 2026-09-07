// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row drives BOTH the C `.so` and the
// Rust `.so` through their exported `slice` symbol and compares the return
// value and the raw stdout bytes. Inputs are randomized from a fixed seed.

mod common;

use common::{assert_same, diff_call, libs, capture_stdout, Rng};
use std::ffi::{c_char, c_int};

const ITERS: usize = 400;

// --- C1 -------------------------------------------------------------------
#[test]
fn c1_both_null_ascii() {
    let mut rng = Rng::new(0xC1);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let s = rng.ascii(len);
        assert_same("C1", &s, None, None);
    }
}

// --- C2 -------------------------------------------------------------------
#[test]
fn c2_both_null_empty() {
    // Degenerate shape: len == 0. Deterministic, but assert it repeatedly to
    // be sure there is no hidden state.
    for _ in 0..8 {
        assert_same("C2", b"", None, None);
    }
    let (c, _r) = diff_call(b"", None, None);
    assert_eq!(c.ret, 0);
    assert_eq!(c.stdout, b"\n", "empty string must print a bare newline");
}

// --- C3 -------------------------------------------------------------------
#[test]
fn c3_both_null_arbitrary_bytes() {
    let mut rng = Rng::new(0xC3);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        // 0x01..=0xff: high-bit set, invalid UTF-8, control bytes -- but never
        // 0x00, which would just shorten the C string.
        let s = rng.bytes(len, 0x01, 0xff);
        assert_same("C3", &s, None, None);
    }
}

// --- C4 -------------------------------------------------------------------
#[test]
fn c4_both_null_format_metachars() {
    let fragments: [&[u8]; 8] =
        [b"%s", b"%n", b"%d", b"%%", b"%.*s", b"%1000000d", b"%p", b"abc"];
    let mut rng = Rng::new(0xC4);
    for _ in 0..ITERS {
        let mut s = Vec::new();
        for _ in 0..rng.range(1, 6) {
            s.extend_from_slice(fragments[rng.range(0, fragments.len() - 1)]);
        }
        assert_same("C4", &s, None, None);
        // and with explicit indices over the same payload
        let len = s.len() as c_int;
        assert_same("C4/idx", &s, Some(0), Some(len));
    }
}

// --- C5 -------------------------------------------------------------------
#[test]
fn c5_both_null_long() {
    let mut rng = Rng::new(0xC5);
    for _ in 0..40 {
        let len = rng.range(256, 4096);
        let s = rng.bytes(len, 0x01, 0xff);
        assert_same("C5", &s, None, None);
    }
}

// --- C6 -------------------------------------------------------------------
#[test]
fn c6_start_set_stop_null() {
    let mut rng = Rng::new(0xC6);
    for _ in 0..ITERS {
        let len = rng.range(0, 80);
        let s = rng.bytes(len, 0x01, 0xff);
        // valid start range is 0..=len inclusive
        let start = rng.range(0, len) as c_int;
        assert_same("C6", &s, Some(start), None);
    }
}

// --- C7 -------------------------------------------------------------------
#[test]
fn c7_start_equals_len_stop_null() {
    let mut rng = Rng::new(0xC7);
    for _ in 0..ITERS {
        let len = rng.range(1, 80);
        let s = rng.bytes(len, 0x01, 0xff);
        let (c, r) = diff_call(&s, Some(len as c_int), None);
        assert_eq!(c, r, "C7 divergence at len={len}");
        assert_eq!(c.ret, 0, "start == len must be accepted");
        assert_eq!(c.stdout, b"\n", "zero-width slice prints a bare newline");
    }
}

// --- C8 -------------------------------------------------------------------
#[test]
fn c8_start_zero_empty_stop_null() {
    for _ in 0..8 {
        let (c, r) = diff_call(b"", Some(0), None);
        assert_eq!(c, r);
        assert_eq!(c.ret, 0);
        assert_eq!(c.stdout, b"\n");
    }
}

// --- C9 -------------------------------------------------------------------
#[test]
fn c9_start_null_stop_set() {
    let mut rng = Rng::new(0xC9);
    for _ in 0..ITERS {
        let len = rng.range(1, 80);
        let s = rng.bytes(len, 0x01, 0xff);
        // start is implicitly 0, so stop must be in 1..=len to be valid
        let stop = rng.range(1, len) as c_int;
        assert_same("C9", &s, None, Some(stop));
    }
}

// --- C10 ------------------------------------------------------------------
#[test]
fn c10_start_null_stop_equals_len() {
    let mut rng = Rng::new(0xCA);
    for _ in 0..ITERS {
        let len = rng.range(1, 120);
        let s = rng.bytes(len, 0x01, 0xff);
        let (c, r) = diff_call(&s, None, Some(len as c_int));
        assert_eq!(c, r, "C10 divergence");
        assert_eq!(c.ret, 0);
        let mut want = s.clone();
        want.push(b'\n');
        assert_eq!(c.stdout, want, "stop == len must print the whole string");
    }
}

// --- C11 ------------------------------------------------------------------
#[test]
fn c11_both_set_interior() {
    let mut rng = Rng::new(0xCB);
    for _ in 0..ITERS {
        let len = rng.range(2, 100);
        let s = rng.bytes(len, 0x01, 0xff);
        // 0 <= start < stop <= len
        let start = rng.range(0, len - 1);
        let stop = rng.range(start + 1, len);
        let (c, r) = diff_call(&s, Some(start as c_int), Some(stop as c_int));
        assert_eq!(c, r, "C11 divergence start={start} stop={stop} len={len}");
        assert_eq!(c.ret, 0);
        let mut want = s[start..stop].to_vec();
        want.push(b'\n');
        assert_eq!(c.stdout, want, "C printed the wrong slice (test bug)");
    }
}

// --- C12 ------------------------------------------------------------------
#[test]
fn c12_both_set_width_boundaries() {
    let mut rng = Rng::new(0xCC);
    for _ in 0..ITERS {
        let len = rng.range(1, 100);
        let s = rng.bytes(len, 0x01, 0xff);
        // narrowest legal slice: stop == start + 1, anywhere in the string
        let start = rng.range(0, len - 1);
        assert_same("C12/narrow", &s, Some(start as c_int), Some(start as c_int + 1));
        // widest: whole string
        assert_same("C12/wide", &s, Some(0), Some(len as c_int));
        // slice ending exactly at len
        assert_same("C12/tail", &s, Some(start as c_int), Some(len as c_int));
        // slice starting exactly at 0
        assert_same("C12/head", &s, Some(0), Some(rng.range(1, len) as c_int));
    }
}

// --- C13 ------------------------------------------------------------------
#[test]
fn c13_full_cross_product_fuzz() {
    let mut rng = Rng::new(0xCD);
    for _ in 0..3000 {
        let len = if rng.range(0, 9) == 0 { 0 } else { rng.range(1, 48) };
        let s = rng.bytes(len, 0x01, 0xff);

        // Mix of near-boundary values and completely unconstrained i32s so the
        // sweep covers both accepted and rejected configurations.
        let pick = |rng: &mut Rng| -> Option<c_int> {
            if rng.range(0, 4) == 0 {
                return None; // NULL pointer
            }
            match rng.range(0, 7) {
                0 => Some(0),
                1 => Some(len as c_int),
                2 => Some(len as c_int + 1),
                3 => Some(len as c_int - 1),
                4 => Some(-(rng.range(1, 4) as c_int)),
                5 => Some(i32::MIN),
                6 => Some(i32::MAX),
                _ => Some(rng.i32()),
            }
        };
        let start = pick(&mut rng);
        let stop = pick(&mut rng);
        assert_same("C13", &s, start, stop);
    }
}

// --- C14 ------------------------------------------------------------------
#[test]
fn c14_sequential_calls_stream_order() {
    // Many calls inside a SINGLE capture window: verifies the byte stream
    // produced by a run of successive calls (mixing successes and errors) is
    // identical, i.e. no buffering/ordering/statefulness difference.
    let mut rng = Rng::new(0xCE);
    let f = libs();

    for _ in 0..40 {
        let n = rng.range(3, 25);
        let mut cases: Vec<(Vec<u8>, Option<c_int>, Option<c_int>)> = Vec::new();
        for _ in 0..n {
            let len = rng.range(0, 24);
            let s = rng.bytes(len, 0x01, 0xff);
            let a = if rng.bool() { Some(rng.range(0, len + 2) as c_int) } else { None };
            let b = if rng.bool() { Some(rng.range(0, len + 2) as c_int) } else { None };
            cases.push((s, a, b));
        }

        let run = |fun: common::SliceFn, cases: &[(Vec<u8>, Option<c_int>, Option<c_int>)]| {
            capture_stdout(|| {
                let mut rets = Vec::new();
                for (s, a, b) in cases {
                    let mut buf = s.clone();
                    buf.push(0);
                    let mut sa = a.unwrap_or(0);
                    let mut sb = b.unwrap_or(0);
                    let pa: *mut c_int = if a.is_some() { &mut sa } else { core::ptr::null_mut() };
                    let pb: *mut c_int = if b.is_some() { &mut sb } else { core::ptr::null_mut() };
                    rets.push(unsafe { fun(buf.as_mut_ptr() as *mut c_char, pa, pb) });
                }
                rets
            })
        };

        let (c_rets, c_out) = run(f.c_slice, &cases);
        let (r_rets, r_out) = run(f.rust_slice, &cases);
        assert_eq!(c_rets, r_rets, "C14: return sequence differs");
        assert_eq!(
            c_out,
            r_out,
            "C14: stdout stream differs\n C: {:?}\n R: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

// --- C15 ------------------------------------------------------------------
#[test]
fn c15_aliased_index_pointers() {
    // start_ptr and stop_ptr aliasing one object: legal C, forces stop == start.
    let f = libs();
    let mut rng = Rng::new(0xCF);
    for _ in 0..200 {
        let len = rng.range(0, 40);
        let s = rng.bytes(len, 0x01, 0xff);
        let v = rng.range(0, len) as c_int;

        let call = |fun: common::SliceFn| {
            let mut buf = s.clone();
            buf.push(0);
            let mut idx: c_int = v;
            let p: *mut c_int = &mut idx;
            let (ret, out) =
                capture_stdout(|| unsafe { fun(buf.as_mut_ptr() as *mut c_char, p, p) });
            // also confirm neither implementation writes back through the pointers
            (ret, out, idx, buf)
        };
        let (cr, co, ci, cb) = call(f.c_slice);
        let (rr, ro, ri, rb) = call(f.rust_slice);
        assert_eq!(cr, rr, "C15 return differs (v={v}, len={len})");
        assert_eq!(co, ro, "C15 stdout differs (v={v}, len={len})");
        assert_eq!(ci, ri, "C15: index cell mutated differently");
        assert_eq!(ci, v, "C15: index cell must not be written back");
        assert_eq!(cb, rb, "C15: input buffer mutated differently");
    }

    // Adjacent cells in a shared array (distinct pointers, one allocation).
    for _ in 0..200 {
        let len = rng.range(2, 40);
        let s = rng.bytes(len, 0x01, 0xff);
        let a = rng.range(0, len - 1) as c_int;
        let b = rng.range(a as usize + 1, len) as c_int;

        let call = |fun: common::SliceFn| {
            let mut buf = s.clone();
            buf.push(0);
            let mut arr: [c_int; 2] = [a, b];
            let pa = arr.as_mut_ptr();
            let pb = unsafe { arr.as_mut_ptr().add(1) };
            let (ret, out) =
                capture_stdout(|| unsafe { fun(buf.as_mut_ptr() as *mut c_char, pa, pb) });
            (ret, out, arr, buf)
        };
        let (cr, co, ca, cb) = call(f.c_slice);
        let (rr, ro, ra, rb) = call(f.rust_slice);
        assert_eq!(cr, rr, "C15/adj return differs");
        assert_eq!(co, ro, "C15/adj stdout differs");
        assert_eq!(ca, ra, "C15/adj index cells mutated differently");
        assert_eq!(ca, [a, b], "C15/adj: index cells must not be written back");
        assert_eq!(cb, rb, "C15/adj input buffer mutated differently");
    }
}

// --- exhaustive small-domain sweep (backs up every row above) -------------
#[test]
fn exhaustive_small_domain() {
    // For every string length 0..=8 and every start/stop in {NULL} U -3..=len+3,
    // compare exhaustively. This closes the gap between the randomized rows.
    let mut rng = Rng::new(0xE0);
    for len in 0usize..=8 {
        let s = rng.bytes(len, 0x41, 0x5a);
        let mut vals: Vec<Option<c_int>> = vec![None];
        for v in -3i32..=(len as i32 + 3) {
            vals.push(Some(v));
        }
        for a in &vals {
            for b in &vals {
                assert_same("exhaustive", &s, *a, *b);
            }
        }
    }
}
