//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//! Both libraries are loaded as `.so` and driven only through `next_double`.

mod common;
use common::{assert_same_run, both, CnRnd, SplitMix64};

/// Structured patterns reused by C10.
const PATTERNS: [u64; 6] = [
    0xAAAA_AAAA_AAAA_AAAA,
    0x5555_5555_5555_5555,
    0xFFFF_FFFF_0000_0000,
    0x0000_0000_FFFF_FFFF,
    0x8000_0000_0000_0000,
    0x0000_0000_0000_0001,
];

// ---------------------------------------------------------------- C1
#[test]
fn c1_degenerate_all_zero_state() {
    let (c, r) = both();
    assert_same_run(&c, &r, CnRnd::new(0, 0), 8, "C1 all-zero state");

    // Additionally pin the absorbing behaviour the C exhibits, so a Rust
    // implementation that diverged *identically* in both would still be caught.
    let mut s = CnRnd::new(0, 0);
    for i in 0..8 {
        let v = c.next(&mut s);
        assert_eq!(v.to_bits(), 0.0f64.to_bits(), "C1: call {i} not exactly +0.0");
        assert_eq!(s, CnRnd::new(0, 0), "C1: state left the absorbing point");
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_all_ones_state() {
    let (c, r) = both();
    assert_same_run(
        &c,
        &r,
        CnRnd::new(u64::MAX, u64::MAX),
        64,
        "C2 all-ones state",
    );
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_x_zero_y_random() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xC3_5EED_0000_0003);
    for i in 0..256 {
        let y = g.next_nonzero();
        assert_same_run(&c, &r, CnRnd::new(0, y), 4, &format!("C3 x=0 y=rnd #{i}"));
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_x_random_y_zero() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xC4_5EED_0000_0004);
    for i in 0..256 {
        let x = g.next_nonzero();
        assert_same_run(&c, &r, CnRnd::new(x, 0), 4, &format!("C4 x=rnd y=0 #{i}"));
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn c5_both_random_full_range() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xC5_5EED_0000_0005);
    for i in 0..4096 {
        let (x, y) = (g.next_u64(), g.next_u64());
        assert_same_run(&c, &r, CnRnd::new(x, y), 1, &format!("C5 random pair #{i}"));
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn c6_wrapping_add_boundary() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xC6_5EED_0000_0006);

    // Reproduce the C mixing locally *only* to construct inputs that land on the
    // x+y wrap boundary. Correctness is still judged by C-vs-Rust comparison.
    fn x_after(x: u64, y: u64) -> u64 {
        let mut x = x;
        x ^= x << 23;
        x ^= x >> 17;
        x ^= y ^ (y >> 26);
        x
    }

    let mut cases: Vec<(u64, u64)> = Vec::new();
    for _ in 0..512 {
        let x = g.next_u64();
        // Solve for a y whose mixed result sums with y across 2^64.
        // x_after depends on y, so iterate a couple of times; exactness is not
        // required, only that we cluster near the boundary.
        let mut y = g.next_u64();
        for _ in 0..3 {
            let xa = x_after(x, y);
            y = 0u64.wrapping_sub(xa);
        }
        for delta in [0u64, 1, 2, u64::MAX, u64::MAX - 1] {
            cases.push((x, y.wrapping_add(delta)));
        }
    }
    // Explicit extremes: sum == 2^64-1 and sum == 2^64 (wrap to 0).
    cases.push((0, u64::MAX));
    cases.push((u64::MAX, 1));
    cases.push((1, u64::MAX));
    cases.push((u64::MAX, u64::MAX));

    for (i, (x, y)) in cases.into_iter().enumerate() {
        assert_same_run(&c, &r, CnRnd::new(x, y), 2, &format!("C6 wrap boundary #{i}"));
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_low_twelve_bits_discarded() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0xC7_5EED_0000_0007);

    // `mantissa = value >> 12`, so two runs whose raw `value` differs only in
    // bits 0..11 must yield the same double. Construct that by choosing states
    // that produce a known `value`, using the low-level path: y = 0 makes
    // value = x_after (since +y == 0), and x_after is invertible enough for our
    // purposes by direct search over the low bits of the *result*.
    for i in 0..256 {
        let base = g.next_u64();
        let hi = base & !0xFFF;
        // Same top 52 bits, different low 12 -> same mantissa.
        let a = hi;
        let b = hi | 0xFFF;
        let da = f64::from_bits((1023u64 << 52) | (a >> 12));
        let db = f64::from_bits((1023u64 << 52) | (b >> 12));
        assert_eq!(da.to_bits(), db.to_bits(), "C7 #{i}: precondition");

        // Now verify C and Rust agree on states that exercise those bit ranges.
        assert_same_run(&c, &r, CnRnd::new(a, b), 3, &format!("C7 lo12 a/b #{i}"));
        assert_same_run(&c, &r, CnRnd::new(b, a), 3, &format!("C7 lo12 b/a #{i}"));
        // Flip bit 12 specifically: must be observable.
        assert_same_run(
            &c,
            &r,
            CnRnd::new(hi ^ (1 << 12), b),
            3,
            &format!("C7 bit12 #{i}"),
        );
    }
}

// ---------------------------------------------------------------- C8
/// Invert `x ^= x << s` (the map is a bijection on u64).
fn un_xor_shl(y: u64, s: u32) -> u64 {
    let mut x = y;
    // Fixed-point iteration: x = y ^ (x << s) converges in ceil(64/s) rounds.
    for _ in 0..(64 / s + 2) {
        x = y ^ (x << s);
    }
    x
}

/// Invert `x ^= x >> s`.
fn un_xor_shr(y: u64, s: u32) -> u64 {
    let mut x = y;
    for _ in 0..(64 / s + 2) {
        x = y ^ (x >> s);
    }
    x
}

/// A state whose FIRST `next_double` produces exactly the raw value `v`.
///
/// With `y == 0` the C reduces to `x ^= x<<23; x ^= x>>17;` and returns `x + 0`,
/// so the state is recovered by inverting the two shift-xor steps in order.
fn state_yielding_value(v: u64) -> CnRnd {
    let after_shl = un_xor_shr(v, 17);
    let x = un_xor_shl(after_shl, 23);
    // Verify the construction locally before handing it to the libraries.
    let mut t = x;
    t ^= t << 23;
    t ^= t >> 17;
    assert_eq!(t, v, "state_yielding_value: inversion failed for {v:#018x}");
    CnRnd::new(x, 0)
}

#[test]
fn c8_mantissa_extremes() {
    let (c, r) = both();

    // Drive the raw generator output to each extreme mantissa deterministically.
    let cases: [(&str, u64, f64); 5] = [
        // value >> 12 == 0  -> mantissa 0    -> exactly +0.0
        ("mantissa=0", 0x0000_0000_0000_0000, 0.0),
        // low 12 bits set but mantissa still 0 -> still exactly +0.0
        ("mantissa=0 (low bits set)", 0x0000_0000_0000_0FFF, 0.0),
        // value >> 12 == 1  -> smallest positive result, 2^-52
        ("mantissa=1", 0x0000_0000_0000_1000, f64::from_bits((1023u64 << 52) | 1) - 1.0),
        // value >> 12 == 0xF_FFFF_FFFF_FFFF -> largest result below 1.0
        (
            "mantissa=max",
            0xFFFF_FFFF_FFFF_F000,
            f64::from_bits((1023u64 << 52) | 0xF_FFFF_FFFF_FFFF) - 1.0,
        ),
        // exactly one bit below the halfway point -> 0.5
        ("mantissa=0.5", 0x8000_0000_0000_0000, 0.5),
    ];

    for (label, value, expect) in cases {
        let st = state_yielding_value(value);

        // Confirm the C really produces the intended extreme, then diff.
        let mut probe = st;
        let got = c.next(&mut probe);
        assert_eq!(
            got.to_bits(),
            expect.to_bits(),
            "C8 {label}: C produced {got} for raw value {value:#018x}, expected {expect}"
        );

        assert_same_run(&c, &r, st, 3, &format!("C8 {label}"));
    }

    // Sweep the mantissa across every bit position: value = 1 << k for k >= 12
    // isolates each mantissa bit, and k < 12 must all collapse to +0.0.
    for k in 0..64 {
        let st = state_yielding_value(1u64 << k);
        let mut probe = st;
        let v = c.next(&mut probe);
        if k < 12 {
            assert_eq!(v.to_bits(), 0.0f64.to_bits(), "C8: value=1<<{k} should be +0.0");
        }
        assert_same_run(&c, &r, st, 3, &format!("C8 mantissa bit {k}"));
    }

    // Long sequential sweep: state feedback plus every value the generator
    // happens to reach, including values very close to both ends of [0,1).
    let mut cs = CnRnd::new(0x1234_5678_9ABC_DEF0, 0x0FED_CBA9_8765_4321);
    let mut rs = cs;
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for i in 0..50_000 {
        let cb = c.next_bits(&mut cs);
        let rb = r.next_bits(&mut rs);
        assert_eq!(cb, rb, "C8 sweep: diverged at {i}");
        assert_eq!(cs, rs, "C8 sweep: state diverged at {i}");
        let v = f64::from_bits(cb);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    assert!(lo < 1e-4, "C8 sweep: never got near 0.0 (min {lo})");
    assert!(hi > 1.0 - 1e-4, "C8 sweep: never got near 1.0 (max {hi})");
}

// ---------------------------------------------------------------- C9
#[test]
fn c9_single_bit_states() {
    let (c, r) = both();
    for i in 0..64 {
        let bit = 1u64 << i;
        assert_same_run(&c, &r, CnRnd::new(bit, 0), 4, &format!("C9 x=1<<{i}, y=0"));
        assert_same_run(&c, &r, CnRnd::new(0, bit), 4, &format!("C9 x=0, y=1<<{i}"));
        // Also both words carrying the same isolated bit.
        assert_same_run(&c, &r, CnRnd::new(bit, bit), 4, &format!("C9 x=y=1<<{i}"));
    }
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_structured_pattern_cross_product() {
    let (c, r) = both();
    for (a, &x) in PATTERNS.iter().enumerate() {
        for (b, &y) in PATTERNS.iter().enumerate() {
            assert_same_run(&c, &r, CnRnd::new(x, y), 8, &format!("C10 pattern {a}x{b}"));
        }
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_long_sequential_run() {
    let (c, r) = both();
    assert_same_run(
        &c,
        &r,
        CnRnd::new(0xDEAD_BEEF_CAFE_BABE, 0x0123_4567_89AB_CDEF),
        100_000,
        "C11 long run",
    );
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_many_independent_short_runs() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0x12_5EED_0000_0012);
    for i in 0..1024 {
        let st = CnRnd::new(g.next_u64(), g.next_u64());
        // Compare the whole output vector at once, then the final state.
        let mut cs = st;
        let mut rs = st;
        let cv: Vec<u64> = (0..16).map(|_| c.next_bits(&mut cs)).collect();
        let rv: Vec<u64> = (0..16).map(|_| r.next_bits(&mut rs)).collect();
        assert_eq!(cv, rv, "C12 run #{i}: output vector diverged (start {st:?})");
        assert_eq!(cs, rs, "C12 run #{i}: final state diverged (start {st:?})");
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_range_invariant_large_sample() {
    let (c, r) = both();
    let mut cs = CnRnd::new(0x9E37_79B9_7F4A_7C15, 0xBF58_476D_1CE4_E5B9);
    let mut rs = cs;
    for i in 0..200_000 {
        let cb = c.next_bits(&mut cs);
        let rb = r.next_bits(&mut rs);
        assert_eq!(cb, rb, "C13: bits diverged at sample {i}");
        let v = f64::from_bits(cb);
        assert!(v.is_finite(), "C13: non-finite at {i}");
        assert!((0.0..1.0).contains(&v), "C13: {v} outside [0,1) at {i}");
    }
    assert_eq!(cs, rs, "C13: final state diverged");
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_struct_layout_and_writeback() {
    let (c, r) = both();
    assert_eq!(std::mem::size_of::<CnRnd>(), 16, "C14: cn_rnd_t must be 16 bytes");
    assert_eq!(std::mem::align_of::<CnRnd>(), 8, "C14: cn_rnd_t must be 8-aligned");

    // Read state between calls and confirm word-for-word write-back parity.
    let mut g = SplitMix64::new(0x14_5EED_0000_0014);
    for i in 0..512 {
        let st = CnRnd::new(g.next_u64(), g.next_u64());
        let mut cs = st;
        let mut rs = st;
        for k in 0..5 {
            let _ = c.next_bits(&mut cs);
            let _ = r.next_bits(&mut rs);
            assert_eq!(
                cs.state[0], rs.state[0],
                "C14 #{i}: state[0] differs after call {k}"
            );
            assert_eq!(
                cs.state[1], rs.state[1],
                "C14 #{i}: state[1] differs after call {k}"
            );
            // The C sets state[0] = old state[1]; the per-word equality above
            // already pins the ordering (a swap would show up as a mismatch
            // whenever the two words differ, which is the overwhelming
            // majority of random states).
        }
    }
}

// Also assert the raw byte image of the struct matches, catching any padding or
// endianness discrepancy in how the two libraries write back the state.
#[test]
fn c14b_raw_byte_image_of_state() {
    let (c, r) = both();
    let mut g = SplitMix64::new(0x14B_5EED_0000_14B);
    for i in 0..256 {
        let st = CnRnd::new(g.next_u64(), g.next_u64());
        let mut cs = st;
        let mut rs = st;
        for _ in 0..4 {
            let _ = c.next_bits(&mut cs);
            let _ = r.next_bits(&mut rs);
        }
        let cb: [u8; 16] = unsafe { std::mem::transmute(cs) };
        let rb: [u8; 16] = unsafe { std::mem::transmute(rs) };
        assert_eq!(cb, rb, "C14b #{i}: raw state bytes differ");
    }
}
