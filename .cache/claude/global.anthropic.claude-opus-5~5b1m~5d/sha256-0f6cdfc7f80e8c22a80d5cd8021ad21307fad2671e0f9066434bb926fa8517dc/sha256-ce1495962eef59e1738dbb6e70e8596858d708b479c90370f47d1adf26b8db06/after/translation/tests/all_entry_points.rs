//! CONFIGS.md #49 — randomized interleaving of the FULL public API on one shared
//! library instance, comparing every return value AND every output buffer.
//!
//! This is the test that per-function suites structurally cannot replace: the four
//! `operations[]` members mutate statics that `findrep` later branches on, so only a
//! mixed sequence explores the reachable state space. 20 000 steps, fixed seed.

mod common;
use common::*;

use std::ffi::{c_char, c_int};

/// Shadow of `multiplier`, tracked from return values so the driver can avoid the
/// one operand pair that is fatal in both libraries (`INT_MIN / -1`, see crash.rs).
struct Shadow {
    mult: c_int,
}

#[derive(Default)]
struct Hits {
    add: u32,
    mul: u32,
    sub: u32,
    div: u32,
    div_guarded: u32,
    octal: u32,
    replace: u32,
    replace_hit: u32,
    validate: u32,
    findrep: u32,
}

fn interleave(seed: u64, steps: usize) -> Hits {
    let p = fresh_pair();
    let mut sh = Shadow { mult: 1 };
    let mut rng = Rng::new(seed);
    let mut hits = Hits::default();

    // A persistent pair of buffers, so writes from earlier steps stay visible and
    // later steps operate on already-dirty memory (as the C's `strcpy` would).
    let mut cbuf = sentinel_buf();
    let mut rbuf = sentinel_buf();

    for step in 0..steps {
        match rng.below(8) {
            0 => {
                let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
                let cv = unsafe { (p.c.add_to_accumulator)(a, b) };
                let rv = unsafe { (p.r.add_to_accumulator)(a, b) };
                assert_eq!(cv, rv, "step {step}: add_to_accumulator({a}, {b})");
                hits.add += 1;
            }
            1 => {
                let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
                let cv = unsafe { (p.c.multiply_with_multiplier)(a, b) };
                let rv = unsafe { (p.r.multiply_with_multiplier)(a, b) };
                assert_eq!(cv, rv, "step {step}: multiply_with_multiplier({a}, {b})");
                sh.mult = cv;
                hits.mul += 1;
            }
            2 => {
                let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
                let cv = unsafe { (p.c.subtract_from_accumulator)(a, b) };
                let rv = unsafe { (p.r.subtract_from_accumulator)(a, b) };
                assert_eq!(cv, rv, "step {step}: subtract_from_accumulator({a}, {b})");
                hits.sub += 1;
            }
            3 => {
                let a = rng.interesting_i32();
                let mut b = rng.interesting_i32();
                if sh.mult == c_int::MIN && b == -1 {
                    b = 9; // avoid the SIGFPE case, covered in crash.rs
                }
                let cv = unsafe { (p.c.divide_multiplier)(a, b) };
                let rv = unsafe { (p.r.divide_multiplier)(a, b) };
                assert_eq!(cv, rv, "step {step}: divide_multiplier({a}, {b})");
                sh.mult = cv;
                hits.div += 1;
                if b == 0 {
                    hits.div_guarded += 1;
                }
            }
            4 => {
                let v = rng.interesting_i32();
                unsafe {
                    (p.c.process_octal_string)(cbuf.as_mut_ptr() as *mut c_char, v);
                    (p.r.process_octal_string)(rbuf.as_mut_ptr() as *mut c_char, v);
                }
                assert_bytes_eq(&cbuf, &rbuf, &format!("step {step}: process_octal_string({v})"));
                hits.octal += 1;
            }
            5 => {
                // Operate on whatever the buffers currently hold (often the output of
                // a previous process_octal_string call, as inside findrep).
                let ch = match rng.below(3) {
                    0 => b'O' as c_int,
                    1 => rng.range_i32(0, 255),
                    _ => rng.next_i32(),
                };
                let before = cbuf.clone();
                unsafe {
                    (p.c.find_and_replace_char)(cbuf.as_mut_ptr() as *mut c_char, ch);
                    (p.r.find_and_replace_char)(rbuf.as_mut_ptr() as *mut c_char, ch);
                }
                assert_bytes_eq(
                    &cbuf,
                    &rbuf,
                    &format!("step {step}: find_and_replace_char(buf, {ch})"),
                );
                hits.replace += 1;
                if before != cbuf {
                    hits.replace_hit += 1;
                }
            }
            6 => {
                let v = rng.interesting_i32();
                let cv = unsafe { (p.c.validate_and_normalize)(v) };
                let rv = unsafe { (p.r.validate_and_normalize)(v) };
                assert_eq!(cv, rv, "step {step}: validate_and_normalize({v})");
                hits.validate += 1;
            }
            _ => {
                let (a, b, c, d) = (
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                );
                let cv = unsafe { (p.c.findrep)(a, b, c, d) };
                let rv = unsafe { (p.r.findrep)(a, b, c, d) };
                assert_eq!(cv, rv, "step {step}: findrep({a}, {b}, {c}, {d})");
                // findrep internally calls divide_multiplier(multiplier, 2), so the
                // shadow must be refreshed. Read it back with the guarded b == 0 path,
                // which returns `multiplier` without modifying it.
                let cm = unsafe { (p.c.divide_multiplier)(0, 0) };
                let rm = unsafe { (p.r.divide_multiplier)(0, 0) };
                assert_eq!(cm, rm, "step {step}: multiplier readback after findrep");
                sh.mult = cm;
                hits.findrep += 1;
            }
        }
    }
    hits
}

#[test]
fn cfg49_full_api_interleaving_fuzz() {
    let h = interleave(0x49_0001, 20_000);
    eprintln!(
        "cfg49: add={} mul={} sub={} div={} (guarded={}) octal={} replace={} (hits={}) validate={} findrep={}",
        h.add, h.mul, h.sub, h.div, h.div_guarded, h.octal, h.replace, h.replace_hit,
        h.validate, h.findrep
    );
    // Anti-vacuity: every entry point must actually have been driven, and the
    // interesting sub-paths must have been reached.
    assert!(h.add > 100 && h.mul > 100 && h.sub > 100 && h.div > 100);
    assert!(h.octal > 100 && h.replace > 100 && h.validate > 100 && h.findrep > 100);
    assert!(h.div_guarded > 0, "the b == 0 divide guard was never hit");
    assert!(h.replace_hit > 0, "find_and_replace_char never actually replaced");
}

/// Same driver under several independent seeds, so a single unlucky stream cannot
/// hide a divergence.
#[test]
fn cfg49b_full_api_interleaving_multi_seed() {
    for seed in [1u64, 7, 42, 1337, 0xDEAD, 0xBEEF, 0x1234_5678] {
        let h = interleave(seed, 3000);
        assert!(h.findrep > 0 && h.add > 0 && h.octal > 0, "seed {seed} degenerate");
    }
}

/// The exact call shape `findrep` performs internally, driven manually through the
/// public API: `process_octal_string(msg, 0123)` then `find_and_replace_char(msg,'O')`.
/// This pins the composed buffer pipeline independently of `findrep` itself.
#[test]
fn findrep_internal_buffer_pipeline_reproduced_manually() {
    let p = fresh_pair();
    let mut cbuf = sentinel_buf();
    let mut rbuf = sentinel_buf();
    unsafe {
        (p.c.process_octal_string)(cbuf.as_mut_ptr() as *mut c_char, 0o123);
        (p.r.process_octal_string)(rbuf.as_mut_ptr() as *mut c_char, 0o123);
    }
    assert_bytes_eq(&cbuf, &rbuf, "process_octal_string(msg, 0123)");
    let end = cbuf.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&cbuf[..end], b"Octal: 0123, Decimal: 83");

    unsafe {
        (p.c.find_and_replace_char)(cbuf.as_mut_ptr() as *mut c_char, b'O' as c_int);
        (p.r.find_and_replace_char)(rbuf.as_mut_ptr() as *mut c_char, b'O' as c_int);
    }
    assert_bytes_eq(&cbuf, &rbuf, "find_and_replace_char(msg, 'O')");
    let end = cbuf.iter().position(|&b| b == 0).unwrap();
    assert_eq!(
        &cbuf[..end],
        b"Xctal: 0123, Decimal: 83",
        "the leading 'O' must become 'X'"
    );

    // Applying it a second time must be a no-op (no 'O' left).
    let before = cbuf.clone();
    unsafe {
        (p.c.find_and_replace_char)(cbuf.as_mut_ptr() as *mut c_char, b'O' as c_int);
        (p.r.find_and_replace_char)(rbuf.as_mut_ptr() as *mut c_char, b'O' as c_int);
    }
    assert_bytes_eq(&cbuf, &rbuf, "second find_and_replace_char(msg, 'O')");
    assert_eq!(before, cbuf, "no 'O' remains, so nothing may change");
}
