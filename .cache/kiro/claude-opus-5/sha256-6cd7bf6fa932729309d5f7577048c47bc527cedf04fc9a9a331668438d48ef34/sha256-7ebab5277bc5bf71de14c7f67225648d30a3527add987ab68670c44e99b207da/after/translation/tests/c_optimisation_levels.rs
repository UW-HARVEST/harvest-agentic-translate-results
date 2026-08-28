//! Optional cross-check: if `LDEXP_Q2_EXTRA_C_LIBS` is set (colon-separated
//! paths to additional C builds, e.g. the same source at -O0/-O1/-O2/-O3),
//! every one of them is compared against the Rust cdylib as well.
//!
//! This guards against the Rust translation only matching one particular
//! compilation of the C code — the `e >> 2` / negative shift-count path in the
//! original is the kind of thing an optimiser could in principle change.

use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

type LdexpQ2 = unsafe extern "C" fn(f32, c_int) -> f32;

fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();
    profile_dir.join("libldexp_q2_lib.so")
}

#[test]
fn matches_every_c_optimisation_level() {
    let Ok(extra) = std::env::var("LDEXP_Q2_EXTRA_C_LIBS") else {
        eprintln!("LDEXP_Q2_EXTRA_C_LIBS not set; skipping");
        return;
    };

    let rust_path = rust_library_path();
    let rust_lib = unsafe { Library::new(&rust_path) }
        .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));
    let rust: LdexpQ2 = unsafe {
        let s: Symbol<LdexpQ2> = rust_lib.get(b"ldexp_q2\0").expect("rust ldexp_q2");
        *s
    };

    let mut ys: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        3.0,
        -1.5,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x007f_ffff),
        f32::from_bits(0x7f80_0001),
    ];
    let mut state: u32 = 0x9E37_79B9;
    for _ in 0..48 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        ys.push(f32::from_bits(state));
    }

    let mut exps: Vec<c_int> = (-600..=600).collect();
    exps.extend([
        c_int::MIN,
        c_int::MIN + 1,
        -2_000_000_000,
        -1_073_741_824,
        -536_870_912,
        -100_000,
        -1_024,
        1_201,
        12_001,
        100_000,
    ]);

    for path in extra.split(':').filter(|p| !p.is_empty()) {
        let c_lib =
            unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("failed to load {path}: {e}"));
        let c: LdexpQ2 = unsafe {
            let s: Symbol<LdexpQ2> = c_lib.get(b"ldexp_q2\0").expect("c ldexp_q2");
            *s
        };

        for &exp_q2 in &exps {
            for &y in &ys {
                let cv = unsafe { c(y, exp_q2) }.to_bits();
                let rv = unsafe { rust(y, exp_q2) }.to_bits();
                assert_eq!(
                    cv,
                    rv,
                    "mismatch vs {path}: ldexp_q2(bits 0x{yb:08x}, {exp_q2}) \
                     C = 0x{cv:08x}, Rust = 0x{rv:08x}",
                    yb = y.to_bits(),
                );
            }
        }
        eprintln!("matched {path}");
    }
}
