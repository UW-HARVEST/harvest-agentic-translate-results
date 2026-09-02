//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`, each driving MANY randomized inputs
//! from a fixed-seed SplitMix64 generator. Both implementations are invoked
//! through their `.so` exports only.

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 400; // randomized cases per row

fn typical(rng: &mut Rng) -> House {
    House {
        floors: rng.range_i32(0, 20),
        bedrooms: rng.range_i32(0, 20),
        bathrooms: rng.range_i32(0, 40) as f64 / 2.0,
    }
}

// ===========================================================================
// C1 — run, single call, all-typical small values
// ===========================================================================
#[test]
fn c1_run_typical() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC001);
    for i in 0..N {
        let h = typical(&mut rng);
        let e = rng.range_i32(0, 20);
        diff_run(&format!("C1#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C2 — run, extra_bedrooms == 0
// ===========================================================================
#[test]
fn c2_run_zero_extra() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC002);
    for i in 0..N {
        let h = typical(&mut rng);
        diff_run(&format!("C2#{i}"), h, 0);
    }
    });
}

// ===========================================================================
// C3 — run, negative fields
// ===========================================================================
#[test]
fn c3_run_negative() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC003);
    for i in 0..N {
        let h = House {
            floors: rng.range_i32(-1000, -1),
            bedrooms: rng.range_i32(-1000, -1),
            bathrooms: rng.range_i32(-1000, -1) as f64 / 2.0,
        };
        let e = rng.range_i32(-1000, -1);
        diff_run(&format!("C3#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C4 — run, fully random i32 fields (both wrap directions)
// ===========================================================================
#[test]
fn c4_run_full_i32_range() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC004);
    for i in 0..N {
        let h = House {
            floors: rng.next_i32(),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-100, 100) as f64 / 4.0,
        };
        let e = rng.next_i32();
        diff_run(&format!("C4#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C5 — run, floors at the `++` overflow boundary
// ===========================================================================
#[test]
fn c5_run_floors_boundary() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC005);
    let floors = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 0, 1];
    for i in 0..N {
        let h = House {
            floors: *rng.pick(&floors),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-80, 80) as f64 / 8.0,
        };
        let e = rng.next_i32();
        diff_run(&format!("C5#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C6 — run, bedrooms `+=` overflow in both directions
// ===========================================================================
#[test]
fn c6_run_bedrooms_overflow() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC006);
    for i in 0..N {
        let (bedrooms, e) = if i % 2 == 0 {
            (i32::MAX - rng.range_i32(0, 4), rng.range_i32(1, 8))
        } else {
            (i32::MIN + rng.range_i32(0, 4), rng.range_i32(-8, -1))
        };
        let h = House {
            floors: rng.range_i32(-5, 5),
            bedrooms,
            bathrooms: rng.range_i32(-20, 20) as f64 / 2.0,
        };
        diff_run(&format!("C6#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C7 — run, extreme extra_bedrooms
// ===========================================================================
#[test]
fn c7_run_extreme_extra() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC007);
    let extras = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, -1, 1];
    for i in 0..N {
        let h = House {
            floors: rng.next_i32(),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-40, 40) as f64 / 2.0,
        };
        let e = *rng.pick(&extras);
        diff_run(&format!("C7#{i}"), h, e);
    }
    });
}

// ===========================================================================
// C8 — run, `%.1f` rounding-tie bathrooms
// ===========================================================================
#[test]
fn c8_run_rounding_ties() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC008);
    // n/20 gives x.x5 exactly-representable-ish ties; n/16 and n/32 are exact
    // dyadics whose decimal expansion ends in 5 -> printf must break the tie.
    for i in 0..(N * 3) {
        let n = rng.range_i64(-400, 400) as f64;
        let bathrooms = match i % 3 {
            0 => n / 20.0,
            1 => n / 16.0,
            _ => n / 32.0,
        };
        let h = House {
            floors: rng.range_i32(-9, 9),
            bedrooms: rng.range_i32(-9, 9),
            bathrooms,
        };
        let e = rng.range_i32(-9, 9);
        diff_run(&format!("C8#{i}"), h, e);
    }
    // Exhaustive sweep of the classic tie values.
    for n in -400i32..=400 {
        for d in [10.0f64, 20.0, 4.0, 8.0, 16.0, 32.0, 64.0] {
            diff_run(
                &format!("C8-sweep {n}/{d}"),
                House {
                    floors: 2,
                    bedrooms: 5,
                    bathrooms: n as f64 / d,
                },
                1,
            );
        }
    }
    });
}

// ===========================================================================
// C9 — run, random finite f64 bit patterns
// ===========================================================================
#[test]
fn c9_run_random_finite_f64() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC009);
    let mut done = 0usize;
    while done < N {
        let b = rng.f64_bits();
        if !b.is_finite() {
            continue;
        }
        let h = House {
            floors: rng.next_i32(),
            bedrooms: rng.next_i32(),
            bathrooms: b,
        };
        let e = rng.next_i32();
        diff_run(&format!("C9#{done} bits={:#018x}", b.to_bits()), h, e);
        done += 1;
    }
    });
}

// ===========================================================================
// C10 — run, special finite magnitudes / precision-loss shapes
// ===========================================================================
#[test]
fn c10_run_special_finite() {
    common::isolated(|| {
    let specials: [f64; 20] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f64::MIN_POSITIVE,
        5e-324,             // smallest subnormal
        -5e-324,
        1e-10,
        1e15,
        1e16,
        1e17,               // += 1.0 is a no-op from here up
        1e18,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        2.220446049250313e-16,
    ];
    let mut rng = Rng::new(0xC010);
    for (i, &b) in specials.iter().enumerate() {
        for k in 0..24 {
            let h = House {
                floors: if k == 0 { 2 } else { rng.next_i32() },
                bedrooms: if k == 0 { 5 } else { rng.next_i32() },
                bathrooms: b,
            };
            let e = if k == 0 { 1 } else { rng.next_i32() };
            diff_run(&format!("C10#{i}.{k} b={b:e}"), h, e);
        }
    }
    });
}

// ===========================================================================
// C11 — run, non-finite bathrooms
// ===========================================================================
#[test]
fn c11_run_non_finite() {
    common::isolated(|| {
    let nonfinite: [f64; 8] = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xfff8_0000_dead_beef), // negative quiet NaN, payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut rng = Rng::new(0xC011);
    for (i, &b) in nonfinite.iter().enumerate() {
        for k in 0..24 {
            let h = House {
                floors: if k == 0 { 2 } else { rng.next_i32() },
                bedrooms: if k == 0 { 5 } else { rng.next_i32() },
                bathrooms: b,
            };
            let e = if k == 0 { 1 } else { rng.next_i32() };
            diff_run(&format!("C11#{i}.{k} bits={:#018x}", b.to_bits()), h, e);
        }
    }
    });
}

// ===========================================================================
// C12 — run twice on the same struct (the `driver` composition)
// ===========================================================================
#[test]
fn c12_run_twice_same_struct() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC012);
    for i in 0..N {
        let h = typical(&mut rng);
        let e = rng.range_i32(-20, 20);
        diff_run_seq(&format!("C12#{i}"), h, &[e, e]);
    }
    // The exact composition `driver` performs, over the whole extra range.
    for i in 0..N {
        let e = rng.next_i32();
        diff_run_seq(
            &format!("C12-driverlike#{i}"),
            House {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            },
            &[e, e],
        );
    }
    });
}

// ===========================================================================
// C13 — long stateful pipeline: 8 sequential run() calls
// ===========================================================================
#[test]
fn c13_run_eight_calls() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC013);
    for i in 0..N {
        let h = House {
            floors: if i % 3 == 0 { i32::MAX - 3 } else { rng.next_i32() },
            bedrooms: if i % 5 == 0 { i32::MIN + 2 } else { rng.next_i32() },
            bathrooms: if i % 7 == 0 {
                *rng.pick(&[f64::NAN, f64::INFINITY, 1e17, -0.0, f64::MAX])
            } else {
                rng.range_i32(-200, 200) as f64 / 16.0
            },
        };
        let extras: Vec<c_int> = (0..8).map(|_| rng.next_i32()).collect();
        diff_run_seq(&format!("C13#{i}"), h, &extras);
    }
    });
}

// ===========================================================================
// C14 — struct ABI readback (covered inside diff_run_seq, asserted explicitly)
// ===========================================================================
#[test]
fn c14_struct_abi_readback() {
    common::isolated(|| {
    assert_eq!(std::mem::size_of::<House>(), 16, "sizeof(house_t)");
    assert_eq!(std::mem::align_of::<House>(), 8, "alignof(house_t)");

    let p = pair();
    let mut rng = Rng::new(0xC014);
    for i in 0..N {
        let start = House {
            floors: rng.next_i32(),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-1000, 1000) as f64 / 32.0,
        };
        let e = rng.next_i32();

        let mut hc = start;
        let _ = capture(|| unsafe { (p.c.run)(&mut hc, e) });
        let mut hr = start;
        let _ = capture(|| unsafe { (p.rs.run)(&mut hr, e) });

        assert_eq!(hc.raw(), hr.raw(), "C14#{i}: struct bytes differ");
        // And the C's own semantics, as documented, hold for both.
        assert_eq!(hc.floors, start.floors.wrapping_add(1), "C14#{i} floors");
        assert_eq!(hc.bedrooms, start.bedrooms.wrapping_add(e), "C14#{i} bedrooms");
        assert_eq!(hc.bathrooms.to_bits(), (start.bathrooms + 1.0).to_bits(), "C14#{i} baths");
    }
    });
}

// ===========================================================================
// driver-side rows
// ===========================================================================

// C15 — plain non-negative decimal
#[test]
fn c15_driver_plain_nonneg() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC015);
    for i in 0..N {
        let v = rng.range_i64(0, i32::MAX as i64);
        diff_driver(&format!("C15#{i}"), v.to_string().as_bytes());
    }
    });
}

// C16 — plain negative decimal
#[test]
fn c16_driver_plain_neg() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC016);
    for i in 0..N {
        let v = rng.range_i64(i32::MIN as i64, -1);
        diff_driver(&format!("C16#{i}"), v.to_string().as_bytes());
    }
    });
}

// C17 — explicit '+' sign
#[test]
fn c17_driver_plus_sign() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC017);
    for i in 0..N {
        let v = rng.range_i64(0, i32::MAX as i64);
        diff_driver(&format!("C17#{i}"), format!("+{v}").as_bytes());
    }
    });
}

// C18 — leading whitespace runs
#[test]
fn c18_driver_leading_whitespace() {
    common::isolated(|| {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    let mut rng = Rng::new(0xC018);
    for i in 0..N {
        let n = rng.below(8) + 1;
        let mut s: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        match rng.below(3) {
            0 => s.push(b'+'),
            1 => s.push(b'-'),
            _ => {}
        }
        let v = rng.range_i64(0, i32::MAX as i64);
        s.extend_from_slice(v.to_string().as_bytes());
        diff_driver(&format!("C18#{i}"), &s);
    }
    // Whitespace followed by nothing / by junk (rejection side of the axis).
    for i in 0..64 {
        let n = rng.below(8) + 1;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        diff_driver(&format!("C18-bare#{i}"), &s);
    }
    });
}

// C19 — leading zeros
#[test]
fn c19_driver_leading_zeros() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC019);
    for i in 0..N {
        let zeros = rng.below(30) + 1;
        let v = rng.range_i64(0, i32::MAX as i64);
        let sign = *rng.pick(&["", "+", "-"]);
        let s = format!("{sign}{}{v}", "0".repeat(zeros));
        diff_driver(&format!("C19#{i}"), s.as_bytes());
    }
    // All-zero strings of varying length.
    for z in 1..40 {
        diff_driver(&format!("C19-zeros{z}"), "0".repeat(z).as_bytes());
        diff_driver(&format!("C19-negzeros{z}"), format!("-{}", "0".repeat(z)).as_bytes());
    }
    });
}

// C20 — trailing garbage after a valid prefix (accepted by this C)
#[test]
fn c20_driver_trailing_garbage() {
    common::isolated(|| {
    const JUNK: [u8; 18] = [
        b'a', b'z', b'A', b'Z', b'.', b' ', b'-', b'+', b'x', b'e', b'E', b'_', b'%', b'/', b':',
        0xff, 0x80, b'\t',
    ];
    let mut rng = Rng::new(0xC020);
    for i in 0..N {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let mut s = v.to_string().into_bytes();
        let n = rng.below(6) + 1;
        for _ in 0..n {
            s.push(*rng.pick(&JUNK));
        }
        diff_driver(&format!("C20#{i}"), &s);
    }
    for lit in [
        &b"12abc"[..], b"7 8", b"5-", b"1.9", b"0x10", b"0X1F", b"007e9", b"42\n", b"42\t7",
        b"-13xyz", b"+13 ", b"2147483647x", b"-2147483648x", b"1,000", b"1e5", b"3.14159",
    ] {
        diff_driver("C20-lit", lit);
    }
    });
}

// C21 — in-range boundary values
#[test]
fn c21_driver_boundaries() {
    common::isolated(|| {
    let mut vals: Vec<i64> = vec![
        0,
        1,
        -1,
        i32::MIN as i64,
        (i32::MIN as i64) + 1,
        i32::MAX as i64,
        (i32::MAX as i64) - 1,
    ];
    for k in 0..32u32 {
        let p = 1i64 << k;
        vals.push(p);
        vals.push(p - 1);
        vals.push(-p);
        vals.push(-(p - 1));
    }
    vals.retain(|v| *v >= i32::MIN as i64 && *v <= i32::MAX as i64);
    for v in vals {
        diff_driver(&format!("C21 {v}"), v.to_string().as_bytes());
        diff_driver(&format!("C21+ {v}"), format!("{}{}", if v >= 0 { "+" } else { "" }, v).as_bytes());
    }
    diff_driver("C21 -0", b"-0");
    diff_driver("C21 +0", b"+0");
    });
}

// C22 — combined shape cross-product, randomized
#[test]
fn c22_driver_combined_shapes() {
    common::isolated(|| {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    const JUNK: [u8; 10] = [b'a', b'.', b' ', b'-', b'+', b'x', b'e', b'%', 0xff, b'\t'];
    let mut rng = Rng::new(0xC022);
    for i in 0..(N * 2) {
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..rng.below(4) {
            s.push(*rng.pick(&WS));
        }
        match rng.below(3) {
            0 => s.push(b'+'),
            1 => s.push(b'-'),
            _ => {}
        }
        for _ in 0..rng.below(5) {
            s.push(b'0');
        }
        // magnitude band chosen to straddle the accept/reject boundary
        let mag: i64 = match rng.below(5) {
            0 => rng.range_i64(0, 9),
            1 => rng.range_i64(0, i32::MAX as i64),
            2 => rng.range_i64(i32::MAX as i64 - 4, i32::MAX as i64 + 4),
            3 => rng.range_i64(0, i64::MAX / 2),
            _ => i64::MAX - rng.range_i64(0, 4),
        };
        s.extend_from_slice(mag.to_string().as_bytes());
        for _ in 0..rng.below(4) {
            s.push(*rng.pick(&JUNK));
        }
        diff_driver(&format!("C22#{i}"), &s);
    }
    });
}

// C23 — digit-count / length axis
#[test]
fn c23_driver_digit_counts() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC023);
    for digits in 1..=10u32 {
        let lo = if digits == 1 { 0 } else { 10i64.pow(digits - 1) };
        let hi = 10i64.pow(digits) - 1;
        for k in 0..40 {
            let v = rng.range_i64(lo, hi);
            diff_driver(&format!("C23 d={digits} #{k}"), v.to_string().as_bytes());
            diff_driver(&format!("C23- d={digits} #{k}"), format!("-{v}").as_bytes());
        }
    }
    // Total-length axis via zero padding, 1..=64 bytes.
    for len in 1..=64usize {
        let v = rng.range_i64(0, 999);
        let ds = v.to_string();
        if ds.len() > len {
            continue;
        }
        let s = format!("{}{}", "0".repeat(len - ds.len()), ds);
        diff_driver(&format!("C23 len={len}"), s.as_bytes());
    }
    });
}

// C24 — repeated driver calls, alternating accept/reject (no state leakage)
#[test]
fn c24_driver_repeated_alternating() {
    common::isolated(|| {
    let mut rng = Rng::new(0xC024);
    let bad: [&[u8]; 6] = [b"", b"abc", b" ", b"+", b"9999999999999999999999", b"3000000000"];
    for i in 0..N {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        // Interleave in one capture so any cross-call state shows up.
        let good = v.to_string().into_bytes();
        let b = rng.pick(&bad).to_vec();

        let mut g = good.clone();
        g.push(0);
        let mut bb = b.clone();
        bb.push(0);

        let p = pair();
        let c_out = capture(|| unsafe {
            (p.c.driver)(bb.as_ptr() as *const _);
            (p.c.driver)(g.as_ptr() as *const _);
            (p.c.driver)(bb.as_ptr() as *const _);
            (p.c.driver)(g.as_ptr() as *const _);
        });
        let r_out = capture(|| unsafe {
            (p.rs.driver)(bb.as_ptr() as *const _);
            (p.rs.driver)(g.as_ptr() as *const _);
            (p.rs.driver)(bb.as_ptr() as *const _);
            (p.rs.driver)(g.as_ptr() as *const _);
        });
        assert_eq!(
            c_out,
            r_out,
            "C24#{i} alternating good={:?} bad={:?}\n C   : {}\n Rust: {}",
            show(&good),
            show(&b),
            show(&c_out),
            show(&r_out)
        );
    }
    });
}

// C25 — interleaved driver + run in one process
#[test]
fn c25_interleaved_entry_points() {
    common::isolated(|| {
    let p = pair();
    let mut rng = Rng::new(0xC025);
    for i in 0..N {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let mut good = v.to_string().into_bytes();
        good.push(0);
        let bad = b"nope\0";
        let start = House {
            floors: rng.next_i32(),
            bedrooms: rng.next_i32(),
            bathrooms: rng.range_i32(-500, 500) as f64 / 64.0,
        };
        let e = rng.next_i32();

        let mut hc = start;
        let c_out = capture(|| unsafe {
            (p.c.driver)(good.as_ptr() as *const _);
            (p.c.run)(&mut hc, e);
            (p.c.driver)(bad.as_ptr() as *const _);
            (p.c.run)(&mut hc, e);
        });
        let mut hr = start;
        let r_out = capture(|| unsafe {
            (p.rs.driver)(good.as_ptr() as *const _);
            (p.rs.run)(&mut hr, e);
            (p.rs.driver)(bad.as_ptr() as *const _);
            (p.rs.run)(&mut hr, e);
        });

        assert_eq!(
            c_out, r_out,
            "C25#{i} interleaved stdout differs\n C   : {}\n Rust: {}",
            show(&c_out),
            show(&r_out)
        );
        assert_eq!(hc.raw(), hr.raw(), "C25#{i} struct differs");
    }
    });
}
