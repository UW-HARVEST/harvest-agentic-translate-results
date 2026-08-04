use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(non_snake_case)]
struct CbRgb255 {
    R: u8,
    G: u8,
    B: u8,
}

type ContrastRatioFn = unsafe extern "C" fn(CbRgb255, CbRgb255) -> f32;

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Built via cargo as cdylib; cargo emits target/<profile>/libcontrast_ratio_lib.so
    // Determine the profile from env (tests run debug by default).
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Cargo provides PROFILE only at build time, not test runtime.
    // Try debug first, fall back to release.
    let candidates = ["debug", "release"];
    for prof in &candidates {
        let mut q = p.clone();
        q.push(prof);
        q.push("libcontrast_ratio_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!("Could not find Rust .so under target/{{debug,release}}");
}

unsafe fn load_contrast_ratio(lib: &Library) -> Symbol<ContrastRatioFn> {
    unsafe { lib.get(b"contrast_ratio\0").expect("symbol contrast_ratio") }
}

fn build_rust_so() {
    // Ensure the cdylib is built so the test can load it.
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to invoke cargo build");
    assert!(status.success(), "cargo build --lib failed");
}

fn run_compare(cases: &[(CbRgb255, CbRgb255)]) {
    build_rust_so();
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { load_contrast_ratio(&c_lib) };
    let r_fn = unsafe { load_contrast_ratio(&r_lib) };

    for (i, (a, b)) in cases.iter().enumerate() {
        let cv = unsafe { c_fn(*a, *b) };
        let rv = unsafe { r_fn(*a, *b) };
        let cb = cv.to_bits();
        let rb = rv.to_bits();
        assert_eq!(
            cb, rb,
            "Mismatch at case #{i}: A={a:?} B={b:?}: C={cv} ({cb:#x}) R={rv} ({rb:#x})"
        );
    }
}

#[test]
fn contrast_ratio_basic_pairs() {
    let cases = vec![
        (CbRgb255 { R: 0, G: 0, B: 0 }, CbRgb255 { R: 255, G: 255, B: 255 }),
        (CbRgb255 { R: 255, G: 255, B: 255 }, CbRgb255 { R: 0, G: 0, B: 0 }),
        (CbRgb255 { R: 128, G: 128, B: 128 }, CbRgb255 { R: 128, G: 128, B: 128 }),
        (CbRgb255 { R: 255, G: 0, B: 0 }, CbRgb255 { R: 0, G: 0, B: 255 }),
        (CbRgb255 { R: 0, G: 255, B: 0 }, CbRgb255 { R: 0, G: 0, B: 255 }),
        (CbRgb255 { R: 1, G: 1, B: 1 }, CbRgb255 { R: 254, G: 254, B: 254 }),
        (CbRgb255 { R: 10, G: 10, B: 10 }, CbRgb255 { R: 10, G: 10, B: 10 }),
        (CbRgb255 { R: 11, G: 11, B: 11 }, CbRgb255 { R: 12, G: 12, B: 12 }),
    ];
    run_compare(&cases);
}

#[test]
fn contrast_ratio_threshold_around_0_04045() {
    // 0.04045 * 255 ~= 10.31 -> R==10 (below) vs R==11 (above)
    let mut cases = Vec::new();
    for v in 0u8..=20 {
        cases.push((
            CbRgb255 { R: v, G: v, B: v },
            CbRgb255 { R: 255, G: 255, B: 255 },
        ));
    }
    run_compare(&cases);
}

#[test]
fn contrast_ratio_pseudo_random_sweep() {
    // Deterministic LCG sweep
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    let mut cases = Vec::new();
    for _ in 0..2000 {
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state >> 32) as u32
        };
        let r1 = next() as u8;
        let g1 = next() as u8;
        let b1 = next() as u8;
        let r2 = next() as u8;
        let g2 = next() as u8;
        let b2 = next() as u8;
        cases.push((
            CbRgb255 { R: r1, G: g1, B: b1 },
            CbRgb255 { R: r2, G: g2, B: b2 },
        ));
    }
    run_compare(&cases);
}

#[test]
fn contrast_ratio_all_grays() {
    // All-gray pairs: catch division-by-near-zero/zero-equality identical outputs
    let mut cases = Vec::new();
    for v in 0u8..=255 {
        cases.push((
            CbRgb255 { R: v, G: v, B: v },
            CbRgb255 { R: 255 - v, G: 255 - v, B: 255 - v },
        ));
    }
    run_compare(&cases);
}

#[test]
fn exported_symbol_present() {
    // Verify the rust .so exports contrast_ratio (already covered by load above,
    // but also assert the C .so exports it).
    let _c = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let _r = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
}
