//! Differential tests under a **non-default floating-point rounding mode**.
//!
//! Neither `c_src/src/lib.c` nor the Rust translation touches the FP
//! environment, so both inherit whatever MXCSR rounding mode the caller has
//! installed. All of `mulss` / `addss` / `subss` honour it, so a caller that
//! ran `fesetround(FE_UPWARD)` gets different — but *identical* — results from
//! the two libraries. A translation that constant-folded or reassociated the
//! arithmetic, or that computed the guards in a different precision, could
//! diverge only under such a mode.
//!
//! This lives in its own test binary because changing the rounding mode is
//! process-global and would perturb tests running concurrently.

mod common;

use common::*;

// x86-64 `fenv.h` values.
const FE_TONEAREST: i32 = 0x000;
const FE_DOWNWARD: i32 = 0x400;
const FE_UPWARD: i32 = 0x800;
const FE_TOWARDZERO: i32 = 0xC00;

type FeSetRound = unsafe extern "C" fn(i32) -> i32;
type FeGetRound = unsafe extern "C" fn() -> i32;

struct Fenv {
    set: FeSetRound,
    get: FeGetRound,
    _lib: libloading::Library,
}

fn load_fenv() -> Option<Fenv> {
    for name in ["libm.so.6", "libc.so.6", "libm.so"] {
        let lib = match unsafe { libloading::Library::new(name) } {
            Ok(l) => l,
            Err(_) => continue,
        };
        let set = unsafe { lib.get::<FeSetRound>(b"fesetround\0") };
        let get = unsafe { lib.get::<FeGetRound>(b"fegetround\0") };
        if let (Ok(s), Ok(g)) = (set, get) {
            let (s, g) = (*s, *g);
            return Some(Fenv { set: s, get: g, _lib: lib });
        }
    }
    None
}

#[test]
fn differential_under_every_rounding_mode() {
    let Some(fenv) = load_fenv() else {
        eprintln!("SKIP: fesetround/fegetround not found in libm/libc");
        return;
    };

    let p = pair();
    let original = unsafe { (fenv.get)() };
    let modes = [
        ("FE_TONEAREST", FE_TONEAREST),
        ("FE_DOWNWARD", FE_DOWNWARD),
        ("FE_UPWARD", FE_UPWARD),
        ("FE_TOWARDZERO", FE_TOWARDZERO),
    ];

    // Pre-generate all inputs with the DEFAULT rounding mode so the test data
    // itself is not mode-dependent.
    let mut rng = Rng::new(0xF0_0D);
    let mut inputs: Vec<(i32, Vec<f32>)> = Vec::new();
    for i in 0..2_000 {
        let nch = 1 + (i % 2) as i32;
        let z: Vec<f32> = (0..Z_MIN_LEN)
            .map(|_| match rng.below(4) {
                0 => rng.signed_unit(),
                1 => rng.signed_unit() * 0.45, // straddles the clamp thresholds
                2 => rng.wide_exponent_f32(-25, 25),
                _ => BOUNDARY_POOL[rng.below(BOUNDARY_POOL.len())],
            })
            .collect();
        inputs.push((nch, z));
    }

    let mut differing_modes = 0usize;
    let mut baseline: Vec<Vec<i16>> = Vec::new();

    for (mode_name, mode) in modes {
        let rc = unsafe { (fenv.set)(mode) };
        assert_eq!(rc, 0, "fesetround({mode_name}) failed");
        assert_eq!(
            unsafe { (fenv.get)() },
            mode,
            "rounding mode did not stick for {mode_name}"
        );

        let mut results = Vec::with_capacity(inputs.len());
        for (idx, (nch, z)) in inputs.iter().enumerate() {
            let mut out_c = vec![0x5A5A_u16 as i16; 16 * 8 + 16];
            let mut out_r = vec![0x5A5A_u16 as i16; 16 * 8 + 16];
            unsafe {
                (p.c.synth_pair)(out_c.as_mut_ptr(), *nch, z.as_ptr());
                (p.rust.synth_pair)(out_r.as_mut_ptr(), *nch, z.as_ptr());
            }
            if out_c != out_r {
                // Restore before panicking so the harness prints normally.
                unsafe { (fenv.set)(original) };
                let bad = out_c
                    .iter()
                    .zip(out_r.iter())
                    .position(|(a, b)| a != b)
                    .unwrap();
                panic!(
                    "rounding mode {mode_name}: DIVERGENCE at input #{idx}, \
                     pcm[{bad}]: C={} Rust={}",
                    out_c[bad], out_r[bad]
                );
            }
            results.push(out_c);
        }

        if baseline.is_empty() {
            baseline = results;
        } else if results != baseline {
            differing_modes += 1;
        }
    }

    unsafe { (fenv.set)(original) };
    assert_eq!(
        unsafe { (fenv.get)() },
        original,
        "failed to restore the rounding mode"
    );

    // Sanity: the non-default modes must actually have changed some output,
    // otherwise this test would be vacuous.
    assert!(
        differing_modes >= 2,
        "changing the rounding mode never altered any result \
         ({differing_modes} of 3 non-default modes differed) — the test is vacuous"
    );
    eprintln!(
        "all 4 rounding modes agree between C and Rust \
         ({differing_modes}/3 non-default modes produced different values, as expected)"
    );
}
