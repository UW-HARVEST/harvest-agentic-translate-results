//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares
//!   (a) the returned IEEE-754 bit pattern, and
//!   (b) the full 16-byte post-call `cn_rnd_t` state
//! after every single call.

mod common;

use common::{CnRnd, Pair, load_pair, rng};

/// Row 1 — degenerate all-zero state.
///
/// `value == 0` => `mantissa == 0` => `result == 0x3FF0000000000000` => `1.0 - 1.0`
/// which is exactly `+0.0`; the state must stay `{0, 0}` forever.
#[test]
fn row01_zero_state() {
    let p = load_pair();
    p.assert_stream_eq("row01/zero-state/single", CnRnd::new(0, 0), 1);
    p.assert_stream_eq("row01/zero-state/iterated", CnRnd::new(0, 0), 64);

    // Pin down the exact C-observable value as well, so a "both wrong the same
    // way" regression in the harness cannot hide.
    let mut s = CnRnd::new(0, 0);
    let bits = p.c.call(&mut s);
    assert_eq!(bits, 0u64, "C zero-state must return +0.0 bit pattern");
    assert_eq!(s, CnRnd::new(0, 0), "C zero-state must be a fixed point");
}

/// Row 2 — `state[0] == 0`, randomized non-zero `state[1]`.
#[test]
fn row02_zero_lo_random_hi() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..1024 {
        let s = CnRnd::new(0, r.next_nonzero());
        p.assert_stream_eq(&format!("row02/case{i}"), s, 64);
    }
}

/// Row 3 — randomized non-zero `state[0]`, `state[1] == 0`.
#[test]
fn row03_random_lo_zero_hi() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..1024 {
        let s = CnRnd::new(r.next_nonzero(), 0);
        p.assert_stream_eq(&format!("row03/case{i}"), s, 64);
    }
}

/// Row 4 — saturated state: exercises `x << 23` truncation, `y >> 26` and the
/// `x + y` carry all at once.
#[test]
fn row04_saturated_state() {
    let p = load_pair();
    p.assert_stream_eq(
        "row04/saturated",
        CnRnd::new(u64::MAX, u64::MAX),
        64,
    );
}

/// Row 5 — all four corner states, long streams.
#[test]
fn row05_corner_states() {
    let p = load_pair();
    for (i, s) in [
        CnRnd::new(0, 0),
        CnRnd::new(0, u64::MAX),
        CnRnd::new(u64::MAX, 0),
        CnRnd::new(u64::MAX, u64::MAX),
    ]
    .into_iter()
    .enumerate()
    {
        p.assert_stream_eq(&format!("row05/corner{i}"), s, 256);
    }
}

/// Row 6 — single-bit sweep on `state[0]` (every bit's path through
/// `x ^= x << 23` and `x ^= x >> 17`).
#[test]
fn row06_single_bit_sweep_lo() {
    let p = load_pair();
    for i in 0..64 {
        let s = CnRnd::new(1u64 << i, 0);
        p.assert_stream_eq(&format!("row06/bit{i}"), s, 8);
    }
}

/// Row 7 — single-bit sweep on `state[1]` (every bit's path through
/// `x ^= y ^ (y >> 26)`).
#[test]
fn row07_single_bit_sweep_hi() {
    let p = load_pair();
    for j in 0..64 {
        let s = CnRnd::new(0, 1u64 << j);
        p.assert_stream_eq(&format!("row07/bit{j}"), s, 8);
    }
}

/// Row 8 — full 64x64 two-bit cross product.
#[test]
fn row08_two_bit_cross_product() {
    let p = load_pair();
    for i in 0..64 {
        for j in 0..64 {
            let s = CnRnd::new(1u64 << i, 1u64 << j);
            p.assert_stream_eq(&format!("row08/bits({i},{j})"), s, 4);
        }
    }
}

/// Row 9 — high-bit-heavy shapes that make `x << 23` truncate bits away.
#[test]
fn row09_shift_truncation_masks() {
    let p = load_pair();
    let mut r = rng();
    for k in 0..64u32 {
        let masks = [
            u64::MAX << k,
            u64::MAX >> k,
            (u64::MAX << k) ^ (u64::MAX >> k),
            !(u64::MAX >> k),
        ];
        for (m, &mask) in masks.iter().enumerate() {
            // deterministic randomized partner state
            let s = CnRnd::new(mask, r.next_u64());
            p.assert_stream_eq(&format!("row09/mask{k}.{m}/lo"), s, 16);
            let s = CnRnd::new(r.next_u64(), mask);
            p.assert_stream_eq(&format!("row09/mask{k}.{m}/hi"), s, 16);
        }
    }
}

/// Row 10 — shapes that force the final `return x + y` to wrap modulo 2^64.
#[test]
fn row10_addition_wraparound() {
    let p = load_pair();
    let mut r = rng();

    // Hand-constructed guaranteed-carry shapes.
    let mut cases = vec![
        CnRnd::new(u64::MAX, 1),
        CnRnd::new(1, u64::MAX),
        CnRnd::new(1u64 << 63, 1u64 << 63),
        CnRnd::new(u64::MAX, u64::MAX),
        CnRnd::new(u64::MAX - 1, 2),
    ];
    for _ in 0..256 {
        let x = r.next_u64();
        // y == !x  => x + y == u64::MAX (no carry, boundary)
        cases.push(CnRnd::new(x, !x));
        // y == !x + 1 => x + y == 0 (carry, wraps to zero)
        cases.push(CnRnd::new(x, (!x).wrapping_add(1)));
        // y == x => doubling, carries iff the top bit is set
        cases.push(CnRnd::new(x, x));
        // near-boundary partners
        cases.push(CnRnd::new(x, u64::MAX.wrapping_sub(x).wrapping_add(1)));
    }
    for (i, s) in cases.into_iter().enumerate() {
        p.assert_stream_eq(&format!("row10/case{i}"), s, 8);
    }
}

/// Row 11 — mantissa boundaries and a broad sweep over the reachable `[0, 1)`
/// range. Also asserts the C-observable invariants the bit assembly implies.
#[test]
fn row11_mantissa_boundaries() {
    let p = load_pair();
    let mut r = rng();

    // mantissa == 0 boundary (reachable only from the zero state).
    p.assert_call_eq("row11/mantissa-zero", CnRnd::new(0, 0));

    // Broad sweep; check the shared value additionally lies in [0, 1) and that
    // the exponent/mantissa assembly is what the C computes.
    for i in 0..1024 {
        let s = r.next_state();
        p.assert_stream_eq(&format!("row11/sweep{i}"), s, 8);

        let mut cs = s;
        let bits = p.c.call(&mut cs);
        let v = f64::from_bits(bits);
        assert!(
            (0.0..1.0).contains(&v),
            "row11: result out of [0,1): {v} (bits {bits:#018x}) from {:#018x?}",
            s.state
        );
        // No negative zero can be produced by `[1,2) - 1.0`.
        assert_eq!(bits >> 63, 0, "row11: sign bit set for {v}");
    }
}

/// Row 12 — sensitivity to the discarded low 12 bits of `value`.
///
/// `mantissa = value >> 12` throws away 12 bits. Perturbing a state and
/// comparing C against Rust for both the original and the perturbed state
/// proves both implementations discard *the same* bits.
#[test]
fn row12_low_bit_discard() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..512 {
        let base = r.next_state();
        p.assert_call_eq(&format!("row12/base{i}"), base);
        for bit in 0..12 {
            // Perturb low bits of each word; whatever the effect on `value`,
            // C and Rust must agree.
            let a = CnRnd::new(base.state[0] ^ (1u64 << bit), base.state[1]);
            let b = CnRnd::new(base.state[0], base.state[1] ^ (1u64 << bit));
            p.assert_call_eq(&format!("row12/case{i}/lo{bit}"), a);
            p.assert_call_eq(&format!("row12/case{i}/hi{bit}"), b);
        }
    }
}

/// Row 13 — state mutation (word swap + write-back) must be identical.
#[test]
fn row13_state_mutation() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..1024 {
        let start = r.next_state();
        let mut cs = start;
        let mut rs = start;
        let cb = p.c.call(&mut cs);
        let rb = p.rust.call(&mut rs);
        assert_eq!(cb, rb, "row13/case{i}: return bits differ");

        // The C code sets state[0] = old state[1] before overwriting state[1].
        assert_eq!(
            cs.state[0], start.state[1],
            "row13/case{i}: C must move old state[1] into state[0]"
        );
        // memcmp-equality of the raw 16 bytes.
        let cbytes: [u8; 16] = unsafe { std::mem::transmute(cs) };
        let rbytes: [u8; 16] = unsafe { std::mem::transmute(rs) };
        assert_eq!(
            cbytes, rbytes,
            "row13/case{i}: raw struct bytes differ for start {:#018x?}",
            start.state
        );
    }
}

/// Row 14 — property-style bulk run: many random states, many sequential calls.
#[test]
fn row14_bulk_random_streams() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..4096 {
        let s = r.next_state();
        p.assert_stream_eq(&format!("row14/case{i}"), s, 32);
    }
}

/// Row 15 — one very long stream (1,000,000 calls) to catch slow drift.
#[test]
fn row15_long_stream() {
    let p = load_pair();
    let start = CnRnd::new(0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210);
    p.assert_stream_eq("row15/long", start, 1_000_000);
}

/// Row 16 — several independent instances driven interleaved: proves there is
/// no hidden global/`static` state in either implementation.
#[test]
fn row16_interleaved_instances() {
    let p = load_pair();
    let mut r = rng();
    const N: usize = 16;
    let starts: Vec<CnRnd> = (0..N).map(|_| r.next_state()).collect();
    let mut cs = starts.clone();
    let mut rs = starts.clone();

    for round in 0..512 {
        for k in 0..N {
            let cb = p.c.call(&mut cs[k]);
            let rb = p.rust.call(&mut rs[k]);
            assert_eq!(
                cb, rb,
                "row16: bits differ (round {round}, instance {k}, start {:#018x?})",
                starts[k].state
            );
            assert_eq!(
                cs[k], rs[k],
                "row16: state differs (round {round}, instance {k})"
            );
        }
    }

    // Cross-check: the interleaved streams must equal the standalone streams,
    // i.e. instances really are independent.
    for (k, start) in starts.iter().enumerate() {
        let mut solo = *start;
        let mut last = 0u64;
        for _ in 0..512 {
            last = p.rust.call(&mut solo);
        }
        let _ = last;
        assert_eq!(
            solo, rs[k],
            "row16: instance {k} diverged between interleaved and standalone runs"
        );
    }
}

/// Row 17 — struct embedded in a larger buffer with guard canaries: neither
/// implementation may write outside the 16 bytes of `cn_rnd_t`.
#[test]
fn row17_guard_bytes() {
    const GUARD: usize = 32;
    // 8-aligned backing storage; the struct lives at word offset 4.
    #[repr(C, align(8))]
    struct Buf {
        words: [u64; 4 + 2 + 4],
    }

    let p = load_pair();
    let mut r = rng();

    for i in 0..512 {
        let start = r.next_state();
        let canary_lo = [r.next_u64(), r.next_u64(), r.next_u64(), r.next_u64()];
        let canary_hi = [r.next_u64(), r.next_u64(), r.next_u64(), r.next_u64()];

        let build = || {
            let mut b = Buf { words: [0; 10] };
            b.words[0..4].copy_from_slice(&canary_lo);
            b.words[4] = start.state[0];
            b.words[5] = start.state[1];
            b.words[6..10].copy_from_slice(&canary_hi);
            b
        };

        let mut cb_buf = build();
        let mut rb_buf = build();

        let cbits = unsafe {
            let ptr = (&raw mut cb_buf.words[4]).cast::<CnRnd>();
            p.c.call_raw(ptr)
        };
        let rbits = unsafe {
            let ptr = (&raw mut rb_buf.words[4]).cast::<CnRnd>();
            p.rust.call_raw(ptr)
        };

        assert_eq!(cbits, rbits, "row17/case{i}: return bits differ");
        assert_eq!(
            cb_buf.words, rb_buf.words,
            "row17/case{i}: full backing buffer differs (start {:#018x?})",
            start.state
        );
        assert_eq!(
            &cb_buf.words[0..4],
            &canary_lo,
            "row17/case{i}: C clobbered the leading {GUARD}-byte canary"
        );
        assert_eq!(
            &rb_buf.words[0..4],
            &canary_lo,
            "row17/case{i}: Rust clobbered the leading {GUARD}-byte canary"
        );
        assert_eq!(
            &cb_buf.words[6..10],
            &canary_hi,
            "row17/case{i}: C clobbered the trailing canary"
        );
        assert_eq!(
            &rb_buf.words[6..10],
            &canary_hi,
            "row17/case{i}: Rust clobbered the trailing canary"
        );
    }
}

/// Row 18 — every comparison above is on `f64::to_bits`. This test makes the
/// requirement explicit: results are compared as raw bit patterns, so a
/// `+0.0` vs `-0.0` (or any alternative encoding) difference cannot pass.
#[test]
fn row18_bit_exact_comparison() {
    let p = load_pair();
    let mut r = rng();

    // The zero state is the one input that yields a zero result; assert it is
    // *positive* zero in both, which `==` alone would not distinguish.
    let mut cz = CnRnd::new(0, 0);
    let mut rz = CnRnd::new(0, 0);
    let cbits = p.c.call(&mut cz);
    let rbits = p.rust.call(&mut rz);
    assert_eq!(cbits, 0x0000_0000_0000_0000, "C must return +0.0, not -0.0");
    assert_eq!(rbits, cbits, "Rust must return the identical zero encoding");
    assert_ne!(cbits, 0x8000_0000_0000_0000, "must not be -0.0");

    // And a bulk bit-pattern comparison over random inputs.
    let mut differing_patterns = std::collections::HashSet::new();
    for i in 0..4096 {
        let s = r.next_state();
        let mut cs = s;
        let mut rs = s;
        let cb = p.c.call(&mut cs);
        let rb = p.rust.call(&mut rs);
        assert_eq!(cb, rb, "row18/case{i}: bit patterns differ");
        differing_patterns.insert(cb);
    }
    // Sanity: the sweep really did explore many distinct results, so the
    // equality assertions above are meaningful rather than vacuous.
    assert!(
        differing_patterns.len() > 4000,
        "row18: expected a wide spread of results, got {}",
        differing_patterns.len()
    );
}

/// Heavy soak run, kept out of the default suite for runtime reasons.
///
/// Run with:
///   cargo test --test phase_b_valid_paths -- --ignored --nocapture
#[test]
#[ignore = "soak test: ~10M calls, run explicitly"]
fn soak_ten_million_calls() {
    let p = load_pair();
    let mut r = rng();
    for i in 0..320_000 {
        let s = r.next_state();
        p.assert_stream_eq(&format!("soak/case{i}"), s, 32);
    }
}

/// Extra assurance that the two `.so`s really are two different files and both
/// were loaded (guards against the harness accidentally testing one library
/// against itself, which would make every assertion vacuous).
#[test]
fn harness_loads_two_distinct_shared_objects() {
    let c = common::c_so_path();
    let rust = common::rust_so_path();
    assert_ne!(c, rust, "C and Rust .so paths must differ");
    let cm = std::fs::metadata(&c).unwrap().len();
    let rm = std::fs::metadata(&rust).unwrap().len();
    assert!(cm > 0 && rm > 0);
    let _p: Pair = load_pair();
}
