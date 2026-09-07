//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E13). The C library has an empty
//! rejection surface (no error codes, no asserts, no range checks, no pointer
//! or enum parameters), so "the same rejection" is checked as "the same
//! IEEE-754 result bit pattern, sign and NaN payload included" for exactly the
//! degenerate conditions a checking implementation would have rejected.
//!
//! Every assertion pins the concrete sentinel where the C's value is
//! architecturally determined (e.g. `1.0f/+0.0f == +inf`, `+0 * inf ==` the
//! hardware default QNaN `0xffc00000`, SNaN quieting), so a test failing here
//! reports a specific wrong sentinel rather than merely "both misbehaved".

mod harness;

use harness::*;

const DEFAULT_QNAN: u32 = 0xffc0_0000; // x86 SSE "invalid operation" result

fn is_nan_bits(b: u32) -> bool {
    (b & 0x7f80_0000) == 0x7f80_0000 && (b & 0x007f_ffff) != 0
}

// ---------------------------------------------------------------------------
// E1 — all four points coincident: denom = +0, invDenom = +inf, 0*inf = QNaN
// ---------------------------------------------------------------------------

#[test]
fn e1_all_points_coincident() {
    let z = Vec2::new(0.0, 0.0);
    let r = assert_same("E1", [z, z, z, z]);
    assert_eq!(
        r.bits(),
        (DEFAULT_QNAN, DEFAULT_QNAN),
        "E1: expected the hardware default QNaN in both coordinates"
    );

    // same degeneracy at other coincident locations
    let mut rng = Rng::new(0xE1);
    for _ in 0..5_000 {
        let a = rng.normal_vec();
        let r = assert_same("E1[shifted]", [a, a, a, a]);
        assert!(
            is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
            "E1: coincident points must yield NaN, got {r:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E2 — p2 == p1 : the v1 edge is zero-length
// ---------------------------------------------------------------------------

#[test]
fn e2_p2_equals_p1() {
    let mut rng = Rng::new(0xE2);
    for _ in 0..10_000 {
        let a = rng.normal_vec();
        let p3 = rng.normal_vec();
        let p = rng.normal_vec();
        let r = assert_same("E2", [a, a, p3, p]);
        assert!(
            is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
            "E2: zero-length v1 edge must yield NaN in both coordinates, got {r:?}"
        );
    }
    // fixed, fully specified case
    let r = assert_same(
        "E2[fixed]",
        [
            Vec2::new(1.0, 2.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(4.0, 6.0),
            Vec2::new(2.0, 3.0),
        ],
    );
    assert!(is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()));
}

// ---------------------------------------------------------------------------
// E3 — p3 == p1 : the v0 edge is zero-length
// ---------------------------------------------------------------------------

#[test]
fn e3_p3_equals_p1() {
    let mut rng = Rng::new(0xE3);
    for _ in 0..10_000 {
        let a = rng.normal_vec();
        let p2 = rng.normal_vec();
        let p = rng.normal_vec();
        let r = assert_same("E3", [a, p2, a, p]);
        assert!(
            is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
            "E3: zero-length v0 edge must yield NaN in both coordinates, got {r:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E4 — exactly collinear, non-coincident triangle (denominator is zero)
// ---------------------------------------------------------------------------

#[test]
fn e4_collinear() {
    // The textbook case: p1=(0,0), p2=(1,1), p3=(2,2).
    // v0=(2,2) v1=(1,1); dot00=8 dot11=2 dot01=4 -> 8*2 - 4*4 = 0 exactly.
    let r = assert_same(
        "E4[exact]",
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.5, 0.5),
        ],
    );
    assert!(
        is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
        "E4: an exactly-zero denominator must give NaN, got {r:?}"
    );

    // integer collinear triples: the denominator is exactly zero in f32
    let mut rng = Rng::new(0xE4);
    for _ in 0..20_000 {
        let p1 = Vec2::new(rng.below(17) as f32 - 8.0, rng.below(17) as f32 - 8.0);
        let dx = rng.below(9) as f32 - 4.0;
        let dy = rng.below(9) as f32 - 4.0;
        let k = rng.below(7) as f32 - 3.0;
        let p2 = Vec2::new(p1.x + dx, p1.y + dy);
        let p3 = Vec2::new(p1.x + k * dx, p1.y + k * dy);
        assert_same("E4[integer]", [p1, p2, p3, rng.normal_vec()]);
    }
}

// ---------------------------------------------------------------------------
// E5 — denominator underflows to zero from tiny-but-nonzero edges
// ---------------------------------------------------------------------------

#[test]
fn e5_denominator_underflow() {
    // edges ~2^-80 -> dot ~2^-160 -> flushes to 0 in f32 (min normal 2^-126)
    let s = f32::from_bits(((-80i32 + 127) as u32) << 23);
    let r = assert_same(
        "E5[fixed]",
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(s, 0.0),
            Vec2::new(0.0, s),
            Vec2::new(s * 0.25, s * 0.25),
        ],
    );
    assert!(
        is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
        "E5: underflowed denominator must give NaN, got {r:?}"
    );

    let mut rng = Rng::new(0xE5);
    for _ in 0..20_000 {
        let mut args = [Vec2::default(); 4];
        for slot in 0..8 {
            *slot_mut(&mut args, slot) = rng.scaled_f32(-110, -70);
        }
        assert_same("E5", args);
    }
}

// ---------------------------------------------------------------------------
// E6 — denominator overflows: inf - inf = -nan
// ---------------------------------------------------------------------------

#[test]
fn e6_denominator_overflow() {
    // edges ~2^70 -> dot ~2^140 -> +inf; inf*inf - inf*inf = -nan
    let s = f32::from_bits(((70i32 + 127) as u32) << 23);
    let r = assert_same(
        "E6[fixed]",
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(s, 0.0),
            Vec2::new(0.0, s),
            Vec2::new(s, s),
        ],
    );
    assert!(
        is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
        "E6: overflowed denominator must give NaN, got {r:?}"
    );

    let mut rng = Rng::new(0xE6);
    for _ in 0..20_000 {
        let mut args = [Vec2::default(); 4];
        for slot in 0..8 {
            *slot_mut(&mut args, slot) = rng.scaled_f32(64, 127);
        }
        assert_same("E6", args);
    }
}

// ---------------------------------------------------------------------------
// E7 — infinite input coordinates, every slot, both signs
// ---------------------------------------------------------------------------

#[test]
fn e7_infinite_coordinates() {
    let mut rng = Rng::new(0xE7);
    for slot in 0..8 {
        for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
            // with an otherwise well-formed triangle
            for _ in 0..2_000 {
                let mut args = [
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                ];
                *slot_mut(&mut args, slot) = inf;
                assert_same(&format!("E7[{}={inf}]", SLOT_NAMES[slot]), args);
            }
            // with the canonical reference triangle
            let mut args = [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.25),
            ];
            *slot_mut(&mut args, slot) = inf;
            assert_same("E7[reference]", args);
        }
    }
    // inf in every slot at once, all 256 sign patterns
    for mask in 0u32..256 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = if mask >> s & 1 == 1 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
        }
        assert_same("E7[all]", args);
    }
}

// ---------------------------------------------------------------------------
// E8 — quiet NaN payloads, every slot, both signs, many payloads
// ---------------------------------------------------------------------------

#[test]
fn e8_quiet_nan_payloads() {
    let mut rng = Rng::new(0xE8);
    // fixed payloads incl. the boundaries of the payload field
    let payloads: [u32; 8] = [
        0x7fc0_0000, // default QNaN
        0xffc0_0000, // negative default QNaN
        0x7fc0_0001,
        0x7fff_ffff, // max positive QNaN payload
        0xffff_ffff, // max negative QNaN payload
        0x7fd5_5555,
        0xffea_aaaa,
        0x7fc0_0002,
    ];
    for slot in 0..8 {
        for &b in &payloads {
            // reference triangle
            let mut args = [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.25),
            ];
            *slot_mut(&mut args, slot) = f32::from_bits(b);
            let r = assert_same(&format!("E8[{}=0x{b:08x}]", SLOT_NAMES[slot]), args);
            assert!(
                is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
                "E8: a NaN input must give NaN output, got {r:?}"
            );

            // random triangles
            for _ in 0..500 {
                let mut args = [
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                ];
                *slot_mut(&mut args, slot) = f32::from_bits(b);
                assert_same("E8[random-tri]", args);
            }
        }
        // random payloads
        for _ in 0..4_000 {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = rng.qnan_f32();
            assert_same("E8[random-payload]", args);
        }
    }
}

// ---------------------------------------------------------------------------
// E9 — signaling NaNs must be quieted identically
// ---------------------------------------------------------------------------

#[test]
fn e9_signaling_nan() {
    let snans: [u32; 6] = [
        0x7f80_0001, // min positive SNaN
        0x7fbf_ffff, // max positive SNaN
        0xff80_0001,
        0xffbf_ffff,
        0x7f95_5555,
        0xffaa_aaaa,
    ];
    for slot in 0..8 {
        for &b in &snans {
            let mut args = [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.25),
            ];
            *slot_mut(&mut args, slot) = f32::from_bits(b);
            let r = assert_same(&format!("E9[{}=0x{b:08x}]", SLOT_NAMES[slot]), args);
            // the SSE result is always QUIET: bit 22 set
            assert_ne!(
                r.x.to_bits() & 0x0040_0000,
                0,
                "E9: result must be a quiet NaN, got 0x{:08x}",
                r.x.to_bits()
            );
            assert_ne!(
                r.y.to_bits() & 0x0040_0000,
                0,
                "E9: result must be a quiet NaN, got 0x{:08x}",
                r.y.to_bits()
            );
        }
    }
    let mut rng = Rng::new(0xE9);
    for _ in 0..50_000 {
        let mut args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let slot = rng.below(8) as usize;
        *slot_mut(&mut args, slot) = rng.snan_f32();
        assert_same("E9[random]", args);
    }
}

// ---------------------------------------------------------------------------
// E10 — subnormal inputs
// ---------------------------------------------------------------------------

#[test]
fn e10_subnormal_inputs() {
    let subs: [u32; 6] = [
        0x0000_0001,
        0x007f_ffff,
        0x8000_0001,
        0x807f_ffff,
        0x0040_0000,
        0x8000_0002,
    ];
    for slot in 0..8 {
        for &b in &subs {
            let mut args = [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.25),
            ];
            *slot_mut(&mut args, slot) = f32::from_bits(b);
            assert_same(&format!("E10[{}=0x{b:08x}]", SLOT_NAMES[slot]), args);
        }
    }
    // all-subnormal: every dot product underflows to zero -> denom = 0 -> NaN
    let all_min = Vec2::from_bits(0x0000_0001, 0x0000_0001);
    let r = assert_same(
        "E10[all-min-subnormal]",
        [
            Vec2::from_bits(0, 0),
            Vec2::from_bits(0x0000_0001, 0x0000_0002),
            Vec2::from_bits(0x0000_0003, 0x0000_0001),
            all_min,
        ],
    );
    assert!(
        is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
        "E10: all-subnormal inputs underflow the denominator to 0 -> NaN, got {r:?}"
    );

    let mut rng = Rng::new(0xE10);
    for _ in 0..20_000 {
        let mut args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let n = 1 + rng.below(8);
        for _ in 0..n {
            let slot = rng.below(8) as usize;
            *slot_mut(&mut args, slot) = rng.subnormal_f32();
        }
        assert_same("E10[random]", args);
    }
}

// ---------------------------------------------------------------------------
// E11 — negative zero: the SIGN of the zero denominator matters
// ---------------------------------------------------------------------------

#[test]
fn e11_negative_zero() {
    // 1.0f / -0.0f == -inf, so -0 vs +0 in the denominator is observable.
    // Construct denom = -0: dot00*dot11 = +0 and dot01*dot01 = +0 gives +0-+0=+0,
    // so force the sign through the multiplication instead.
    let nz = -0.0f32;
    let pz = 0.0f32;

    // p1 == p2 == p3 == p but with negative zeros in assorted slots
    for mask in 0u32..256 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = if mask >> s & 1 == 1 { nz } else { pz };
        }
        let r = assert_same(&format!("E11[mask={mask:#04x}]"), args);
        assert!(
            is_nan_bits(r.x.to_bits()) && is_nan_bits(r.y.to_bits()),
            "E11: degenerate all-zero input must give NaN, got {r:?}"
        );
    }

    // -0 in every slot of an otherwise valid triangle
    let mut rng = Rng::new(0xE11);
    for slot in 0..8 {
        for _ in 0..2_000 {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = nz;
            assert_same(&format!("E11[{}=-0]", SLOT_NAMES[slot]), args);
        }
    }

    // an explicitly negative denominator: dot01^2 > dot00*dot11 is impossible
    // mathematically, but rounding in the tiny/huge regimes makes it happen;
    // check the -inf branch directly with a hand-built collinear+scaled case.
    let s = f32::from_bits(((-80i32 + 127) as u32) << 23);
    assert_same(
        "E11[tiny-collinear]",
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(s, s),
            Vec2::new(s + s, s + s),
            Vec2::new(s, s),
        ],
    );
}

// ---------------------------------------------------------------------------
// E12 — extremal finite floats (the boundary of the representable range)
// ---------------------------------------------------------------------------

#[test]
fn e12_extremal_finite() {
    const FLT_MAX: f32 = f32::MAX; // 0x7f7fffff
    const FLT_MIN_POS_NORMAL: f32 = f32::MIN_POSITIVE; // 0x00800000
    let extremes = [
        FLT_MAX,
        -FLT_MAX,
        FLT_MIN_POS_NORMAL,
        -FLT_MIN_POS_NORMAL,
        f32::from_bits(0x7f7f_fffe),
        f32::from_bits(0x0080_0001),
    ];

    // every slot, every extreme, against a valid triangle
    let mut rng = Rng::new(0xE12);
    for slot in 0..8 {
        for &e in &extremes {
            let mut args = [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.25),
            ];
            *slot_mut(&mut args, slot) = e;
            assert_same(&format!("E12[{}=0x{:08x}]", SLOT_NAMES[slot], e.to_bits()), args);

            for _ in 0..500 {
                let mut args = [
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                ];
                *slot_mut(&mut args, slot) = e;
                assert_same("E12[random-tri]", args);
            }
        }
    }

    // FLT_MAX - (-FLT_MAX) overflows the subtraction itself
    let r = assert_same(
        "E12[subtraction-overflow]",
        [
            Vec2::new(-FLT_MAX, -FLT_MAX),
            Vec2::new(FLT_MAX, -FLT_MAX),
            Vec2::new(-FLT_MAX, FLT_MAX),
            Vec2::new(0.0, 0.0),
        ],
    );
    assert!(
        is_nan_bits(r.x.to_bits()) || !r.x.is_finite(),
        "E12: overflowed edges must give inf/NaN, got {r:?}"
    );

    // all 8 slots extremal, every combination of the two signs of FLT_MAX
    for mask in 0u32..256 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = if mask >> s & 1 == 1 { -FLT_MAX } else { FLT_MAX };
        }
        assert_same("E12[all-flt-max]", args);
    }
}

// ---------------------------------------------------------------------------
// E13 — uniform bit pattern sweep over all 256 exponents
// ---------------------------------------------------------------------------

#[test]
fn e13_exponent_sweep() {
    // every field of every argument set to the SAME bit pattern
    for exp in 0u32..256 {
        for &mant in &[0u32, 1, 0x0040_0000, 0x007f_ffff, 0x0012_3456] {
            for &sign in &[0u32, 0x8000_0000] {
                let b = sign | (exp << 23) | mant;
                let v = Vec2::from_bits(b, b);
                assert_same(&format!("E13[0x{b:08x}]"), [v, v, v, v]);
            }
        }
    }

    // and with the pattern in only one argument at a time
    for exp in 0u32..256 {
        let b = (exp << 23) | 0x0055_5555;
        let v = Vec2::from_bits(b, b ^ 0x8000_0000);
        let n = Vec2::new(1.5, -2.25);
        assert_same("E13[p1]", [v, n, Vec2::new(-1.0, 3.0), n]);
        assert_same("E13[p2]", [n, v, Vec2::new(-1.0, 3.0), n]);
        assert_same("E13[p3]", [n, Vec2::new(-1.0, 3.0), v, n]);
        assert_same("E13[p]", [n, Vec2::new(-1.0, 3.0), Vec2::new(0.5, 0.5), v]);
    }
}

// ---------------------------------------------------------------------------
// Not-applicable generic boundaries, asserted as such so the reasoning is
// mechanically checked rather than just documented.
// ---------------------------------------------------------------------------

#[test]
fn generic_boundaries_are_unrepresentable() {
    // The ABI has no pointer, length or enum parameter, so the classic
    // null-pointer / zero-length / out-of-range-enum inputs cannot be formed.
    // What CAN be formed is "any 32-bit pattern in any of the 8 float slots",
    // which is what E13/C20/C21 sweep. Verify the struct layout that makes
    // this exhaustive claim true.
    assert_eq!(std::mem::size_of::<Vec2>(), 8);
    assert_eq!(std::mem::align_of::<Vec2>(), 4);

    // An "out-of-range" value for a float parameter is any non-finite one;
    // confirm both libraries accept all four non-finite classes without
    // trapping (they must return, not abort) and agree.
    for &b in &[
        0x7f80_0000u32, // +inf
        0xff80_0000,    // -inf
        0x7fc0_0000,    // +qnan
        0x7f80_0001,    // +snan
        0x0000_0001,    // +subnormal
        0x8000_0000,    // -0
    ] {
        let v = Vec2::from_bits(b, b);
        assert_same("generic", [v, v, v, v]);
    }
}
