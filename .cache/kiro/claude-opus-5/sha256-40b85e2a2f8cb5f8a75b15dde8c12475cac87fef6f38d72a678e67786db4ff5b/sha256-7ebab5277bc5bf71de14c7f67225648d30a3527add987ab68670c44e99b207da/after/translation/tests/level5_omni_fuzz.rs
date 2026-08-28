//! Level 5: heavy fuzzing of `omni_manifold`, the only function declared in
//! `c_src/include/lib.h` and therefore the library's actual public entry point.
//!
//! Inputs are raw 32-bit patterns as well as physically plausible values, so
//! infinities, subnormals and NaNs (with assorted payloads) all reach the
//! arithmetic. The whole `c2Manifold` is compared byte for byte, including the
//! fields the C leaves untouched.
#![allow(non_snake_case)]

mod common;
use common::*;

type OmniFn = unsafe extern "C" fn(
    *mut c2Manifold,
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
    C2_TYPE,
    f32,
    f32,
    f32,
    f32,
    f32,
);

fn seed_manifold(rng: &mut Rng) -> c2Manifold {
    // Random seed bytes: any field the C does not write must survive identically.
    let mut m = c2Manifold::default();
    let p = &mut m as *mut c2Manifold as *mut u8;
    for i in 0..std::mem::size_of::<c2Manifold>() {
        unsafe { *p.add(i) = rng.next_u32() as u8 };
    }
    m
}

fn run(seed: u64, iters: usize, mode: u32) {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<OmniFn>("omni_manifold");
    let types = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY];
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let ta = types[rng.below(4) as usize];
        let tb = types[rng.below(4) as usize];
        let mut v = [0f32; 10];
        for x in v.iter_mut() {
            *x = match mode {
                0 => rng.tame(),
                1 => rng.wild(),
                _ => f32::from_bits(rng.next_u32()),
            };
        }
        let m0 = seed_manifold(&mut rng);
        let mut mc = m0;
        let mut mr = m0;
        scrub_stack();
        unsafe { cf(&mut mc, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9]) };
        unsafe { rf(&mut mr, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9]) };
        assert_same_lazy(&mc, &mr, || {
            let bits: Vec<String> = v.iter().map(|x| format!("{:08x}", x.to_bits())).collect();
            format!(
                "omni_manifold #{it} mode={mode} seed={seed} ta={ta} tb={tb} args=[{}] seedm={}",
                bits.join(","),
                hex(bytes_of(&m0))
            )
        });
    }
}

#[test]
fn omni_fuzz_plausible() {
    run(9001, 300_000, 0);
}

#[test]
fn omni_fuzz_extremes() {
    run(9002, 300_000, 1);
}

#[test]
fn omni_fuzz_raw_bits() {
    run(9003, 300_000, 2);
}

/// Every type pair against a dense set of notable float values in every argument
/// slot, one slot varied at a time over an otherwise plausible configuration.
#[test]
fn omni_notable_per_slot() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<OmniFn>("omni_manifold");
    let types = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY];
    let base: [f32; 10] = [-1.0, -0.5, 1.0, 0.5, 0.75, -0.25, 0.25, 1.25, 1.5, 0.5];
    for &ta in &types {
        for &tb in &types {
            for slot in 0..10 {
                for &val in NOTABLE {
                    let mut v = base;
                    v[slot] = val;
                    let m0 = c2Manifold {
                        count: 0x5555_5555,
                        depths: [-1.5, 2.5],
                        contact_points: [c2v { x: 3.0, y: 4.0 }, c2v { x: 5.0, y: 6.0 }],
                        n: c2v { x: 7.0, y: 8.0 },
                    };
                    let mut mc = m0;
                    let mut mr = m0;
                    scrub_stack();
                    unsafe {
                        cf(&mut mc, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
                    };
                    unsafe {
                        rf(&mut mr, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
                    };
                    assert_same_lazy(&mc, &mr, || {
                        format!(
                            "omni notable ta={ta} tb={tb} slot={slot} val={val:e} args={:?}",
                            v.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                        )
                    });
                }
            }
        }
    }
}

/// A fine translation sweep of two overlapping shapes for every type pair — the
/// configurations a real collision query spends its time in.
#[test]
fn omni_translation_sweep() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<OmniFn>("omni_manifold");
    let types = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];
    let mut n = 0usize;
    for &ta in &types {
        for &tb in &types {
            let mut step = -3.0f32;
            while step <= 3.0 {
                let mut step2 = -3.0f32;
                while step2 <= 3.0 {
                    for &r in &[0.0f32, 0.125, 0.5, 1.0] {
                        let a = [-0.5f32, -0.5, 0.5, 0.5, r];
                        let b = [step - 0.5, step2 - 0.5, step + 0.5, step2 + 0.5, r];
                        let m0 = c2Manifold {
                            count: -1,
                            depths: [f32::NAN, 0.0],
                            contact_points: [c2v { x: 1.0, y: 1.0 }; 2],
                            n: c2v { x: 2.0, y: 2.0 },
                        };
                        let mut mc = m0;
                        let mut mr = m0;
                        scrub_stack();
                        unsafe {
                            cf(&mut mc, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
                        };
                        unsafe {
                            rf(&mut mr, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
                        };
                        assert_same_lazy(&mc, &mr, || format!("omni sweep ta={ta} tb={tb} dx={step} dy={step2} r={r}"));
                        n += 1;
                    }
                    step2 += 0.125;
                }
                step += 0.125;
            }
        }
    }
    assert!(n > 10_000, "sweep too small: {n}");
}
