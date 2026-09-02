//! Targeted high-volume differential sweep of the two libm-dependent paths.
//!
//! The C `.so` calls glibc's `fmodf` / `floorf` (CMake links `m`), while the
//! Rust `.so` statically links `compiler_builtins`' own implementations (see
//! SYMBOLS.md). Those are independent code bases, so this file sweeps the
//! `h` argument across EVERY f32 exponent (all 256 values of the exponent
//! byte, both signs) with many mantissas each — roughly 2 million calls per
//! function — rather than relying on uniform random bit patterns, which would
//! almost never sample the small/large exponents where the two
//! implementations are most likely to differ.

mod common;

use common::*;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

/// Mantissas per exponent. 256 exponents x 2 signs x this = calls per fn.
const MANTISSAS: u32 = 4096;

fn sweep_exponents(name: &str) {
    let l = libs();
    let (c, r) = bind!(l, name, FnTriple);
    let mut g = Rng::seeded();
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    for exp in 0u32..256 {
        for sign in 0u32..2 {
            for k in 0..MANTISSAS {
                // Sweep the low mantissa bits deterministically and the rest
                // randomly, so both the dense boundary region and the whole
                // mantissa space are covered.
                let mant = if k < 64 {
                    k
                } else if k < 128 {
                    0x7F_FFFF - (k - 64)
                } else {
                    g.next_u32() & 0x7F_FFFF
                };
                let h = f32::from_bits((sign << 31) | (exp << 23) | mant);
                let s = match k % 5 {
                    0 => 0.5,
                    1 => 1.0,
                    2 => -1.0,
                    3 => 1e-30,
                    _ => g.range_f32(0.0001, 1.0),
                };
                let v = match k % 4 {
                    0 => 0.25,
                    1 => 1.0,
                    2 => -0.5,
                    _ => g.range_f32(0.0, 1.0),
                };
                let src = [h, s, v];
                unsafe {
                    c(dc.as_mut_ptr(), src.as_ptr());
                    r(dr.as_mut_ptr(), src.as_ptr());
                }
                if dc.map(f32::to_bits) != dr.map(f32::to_bits) {
                    panic!(
                        "{name} diverged at h=0x{:08x} ({h:e}) s={s:e} v={v:e}\n  \
                         C    = {:08x?}\n  Rust = {:08x?}",
                        h.to_bits(),
                        dc.map(f32::to_bits),
                        dr.map(f32::to_bits)
                    );
                }
            }
        }
    }
}

/// `f11` routes `h` through `fmodf(h / 60.0f, 2.0f)`.
#[test]
fn libm_f11_fmodf_all_exponents() {
    sweep_exponents("f11");
}

/// `f12` routes `h` through `floorf(h / 60.0f)` and then an `(int)` cast.
#[test]
fn libm_f12_floorf_all_exponents() {
    sweep_exponents("f12");
}

/// `f13` uses no libm call, but it is the third writer with the same shape;
/// sweeping it too costs little and covers the `min`/`max`/`delta` compare
/// chain across every exponent.
#[test]
fn f13_all_exponents() {
    let l = libs();
    let (c, r) = bind!(l, "f13", FnTriple);
    let mut g = Rng::seeded();
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    for exp in 0u32..256 {
        for sign in 0u32..2 {
            for k in 0..MANTISSAS {
                let mant = if k < 64 { k } else { g.next_u32() & 0x7F_FFFF };
                let a = f32::from_bits((sign << 31) | (exp << 23) | mant);
                let src = match k % 4 {
                    0 => [a, g.mixed_f32(), g.mixed_f32()],
                    1 => [g.mixed_f32(), a, g.mixed_f32()],
                    2 => [g.mixed_f32(), g.mixed_f32(), a],
                    _ => [a, a, g.mixed_f32()],
                };
                unsafe {
                    c(dc.as_mut_ptr(), src.as_ptr());
                    r(dr.as_mut_ptr(), src.as_ptr());
                }
                if dc.map(f32::to_bits) != dr.map(f32::to_bits) {
                    panic!(
                        "f13 diverged at src={:08x?}\n  C    = {:08x?}\n  Rust = {:08x?}",
                        src.map(f32::to_bits),
                        dc.map(f32::to_bits),
                        dr.map(f32::to_bits)
                    );
                }
            }
        }
    }
}

/// `f9` and `c2Dot` across every exponent, since they are the other places
/// where operand order and rounding interact.
#[test]
fn f9_and_dot_all_exponents() {
    let l = libs();
    let (f9c, f9r) = bind!(l, "f9", FnF9);
    let (dc, dr) = bind!(l, "c2Dot", FnC2Dot);
    let mut g = Rng::seeded();
    for exp in 0u32..256 {
        for sign in 0u32..2 {
            for k in 0..1024u32 {
                let mant = if k < 64 { k } else { g.next_u32() & 0x7F_FFFF };
                let a = f32::from_bits((sign << 31) | (exp << 23) | mant);
                let pts = [
                    LmVec2 { x: a, y: g.mixed_f32() },
                    LmVec2 { x: g.mixed_f32(), y: a },
                    LmVec2 { x: a, y: a },
                    LmVec2 { x: g.mixed_f32(), y: g.mixed_f32() },
                ];
                unsafe {
                    eq_lmvec2(
                        &format!("f9 exp={exp} sign={sign} k={k} a=0x{:08x}", a.to_bits()),
                        f9c(pts[0], pts[1], pts[2], pts[3]),
                        f9r(pts[0], pts[1], pts[2], pts[3]),
                    );
                }
                let u = C2v { x: a, y: g.mixed_f32() };
                let v = C2v { x: g.mixed_f32(), y: a };
                unsafe {
                    eq_f32(
                        &format!("c2Dot exp={exp} sign={sign} k={k}"),
                        dc(u, v),
                        dr(u, v),
                    );
                }
            }
        }
    }
}
