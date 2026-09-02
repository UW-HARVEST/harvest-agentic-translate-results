//! Quick broad differential smoke test: catches divergences early before the
//! per-row Phase B / Phase C suites narrow them down.

mod common;

use common::*;

#[test]
fn smoke_broad_random() {
    let p = Pair::load();
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let mut failures: Vec<String> = Vec::new();
    for _ in 0..20_000 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let gc = p.c(a, b, c, d);
        let gr = p.rust(a, b, c, d);
        if gc != gr && failures.len() < 20 {
            failures.push(format!(
                "a=0x{a:08x} b=0x{b:08x} c=0x{c:08x} d=0x{d:08x} -> C={gc} Rust={gr}"
            ));
        }
    }
    assert!(failures.is_empty(), "divergences:\n{}", failures.join("\n"));
}

#[test]
fn smoke_interesting_cross_product() {
    let p = Pair::load();
    // Full 4-way cross product over the interesting-value table.
    for &a in INTERESTING.iter() {
        for &b in INTERESTING.iter() {
            for &c in INTERESTING.iter() {
                for &d in INTERESTING.iter() {
                    p.assert_same("interesting", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn smoke_float_branch_sweep() {
    let p = Pair::load();
    // Sweep the exact `f > 0.0f && f < 1000.0f` decision boundary in `a`.
    let mut probes: Vec<i32> = Vec::new();
    for base in [
        0u32,
        1,
        0x007F_FFFF,
        0x0080_0000,
        0x3F7F_FFFF,
        0x3F80_0000,
        0x4479_FFFF,
        0x447A_0000,
        0x7F7F_FFFF,
        0x7F80_0000,
        0x7FC0_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
    ] {
        for delta in -3i64..=3 {
            let v = (base as i64 + delta) as u32;
            probes.push(v as i32);
        }
    }
    // Every representable integer value of f in [1, 1000) matters for (int)f.
    for k in 1..1000u32 {
        probes.push((k as f32).to_bits() as i32);
        probes.push(((k as f32) + 0.5).to_bits() as i32);
        probes.push(f32::from_bits((k as f32).to_bits() - 1).to_bits() as i32);
    }
    for &a in &probes {
        p.assert_same("float-branch", a, 0, 0, 0);
        p.assert_same("float-branch", a, -1, 1, -1);
        p.assert_same("float-branch", a, i32::MIN, i32::MAX, -1);
    }
}
