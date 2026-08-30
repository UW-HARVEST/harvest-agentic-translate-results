//! `perform_expensive_operations` relies on signed integer overflow, which is
//! undefined behaviour in C, so the C compiler is in principle free to produce
//! different results at different optimisation levels. `c_src/CMakeLists.txt`
//! sets no `CMAKE_BUILD_TYPE`, so the ground-truth library is built with no
//! `-O` flag at all.
//!
//! This test rebuilds the C library out-of-tree (nothing under `c_src/` is
//! touched) at several optimisation levels and checks that they all agree with
//! each other *and* with the Rust translation. If a level ever disagrees, the
//! failure message names it, so the divergence is attributed to the C build
//! rather than to the translation.

mod common;

use common::{assert_arrays_equal, load_both, SplitMix64, ARRAY_SIZE};
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

const LEVELS: [&str; 6] = ["-O0", "-O1", "-O2", "-O3", "-Os", "-Ofast"];

fn build_variant(level: &str) -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let out = common::target_dir()
        .join("c-opt-variants")
        .join(level.trim_start_matches('-'));

    let configure = Command::new("cmake")
        .arg("-S")
        .arg(root.join("c_src"))
        .arg("-B")
        .arg(&out)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg(format!("-DCMAKE_C_FLAGS={level}"))
        .output();
    match configure {
        Ok(o) if o.status.success() => {}
        Ok(_) | Err(_) => return None,
    }
    let build = Command::new("cmake").arg("--build").arg(&out).output();
    match build {
        Ok(o) if o.status.success() => {}
        Ok(_) | Err(_) => return None,
    }

    let so = out.join("liblong.so");
    so.exists().then_some(so)
}

#[test]
fn c_results_are_stable_across_optimisation_levels() {
    // Reference result: the checked-in C build, driven through the shared
    // harness so the comparison path is identical to the other suites.
    let mut rng = SplitMix64(0xC0FF_EE00_1234);
    let payload: Vec<c_int> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();

    let guard = load_both();
    let (c, rust) = &*guard;
    c.write_array(&payload);
    c.perform_expensive_operations();
    let reference = c.read_array();

    rust.write_array(&payload);
    rust.perform_expensive_operations();
    assert_arrays_equal(
        "default C build vs Rust",
        &reference,
        &rust.read_array(),
    );

    let mut checked = Vec::new();
    for level in LEVELS {
        let Some(so) = build_variant(level) else {
            eprintln!("skipping {level}: could not build variant");
            continue;
        };
        let variant = common::load_extra(level, &so);
        variant.write_array(&payload);
        variant.perform_expensive_operations();
        assert_arrays_equal(
            &format!("C built with {level} vs default C build"),
            &reference,
            &variant.read_array(),
        );
        checked.push(level);
    }

    assert!(
        !checked.is_empty(),
        "no optimisation variants could be built, so nothing was verified"
    );
    eprintln!("C output identical across: default (no -O) and {checked:?}");
}
