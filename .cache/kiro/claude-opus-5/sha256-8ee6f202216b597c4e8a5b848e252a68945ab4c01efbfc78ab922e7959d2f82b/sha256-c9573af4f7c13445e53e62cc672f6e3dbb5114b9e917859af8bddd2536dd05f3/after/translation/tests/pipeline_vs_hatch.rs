//! Cross-implementation check of the composed pipeline (`CONFIGS.md` row 50).
//!
//! `hatch` is the only entry point in the public header; the other 11 exports
//! are the low-level API it is built from. This test asserts that **one
//! library's `hatch`** equals **the other library's low-level pipeline**, driven
//! step by step through its individual `.so` exports, in both directions:
//!
//! * C low-level pipeline  ==  Rust `hatch`
//! * Rust low-level pipeline  ==  C `hatch`
//!
//! That is strictly stronger than comparing `hatch` to `hatch`: it would catch a
//! translation where `hatch` and its constituent functions were *consistently*
//! wrong in the same way.
//!
//! Each of the four roles gets its own private copy of the `.so` on disk, so it
//! gets its own `global_counter` / `global_accumulator` (a plain `dlopen` of the
//! same path is refcounted and would share state).

mod harness;

use harness::*;
use std::ffi::c_int;

/// `dlopen`s a private copy of `src` so it has independent global state.
fn load_private_copy(name: &'static str, src: &std::path::Path, tag: &str) -> Lib {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).expect("target tmpdir");
    let dst = dir.join(format!("libprivate_{tag}.so"));
    let _ = std::fs::remove_file(&dst);
    std::fs::copy(src, &dst).expect("copying the .so for a private mapping");
    Lib::load(name, dst)
}

/// Replays exactly what `hatch` does (lib.c:126-176), but through the
/// individual low-level exports of `lib`.
///
/// `counter` / `accumulator` shadow the library's two `static int`s. They are
/// tracked here because the final `result += global_counter + global_accumulator`
/// term reads state that is not exported; the update rules are the only two
/// writers in the whole library (`increment_counter`, `update_accumulator`).
fn manual_hatch(
    lib: &Lib,
    counter: &mut c_int,
    accumulator: &mut c_int,
    p1: c_int,
    p2: c_int,
    p3: c_int,
    p4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    unsafe { (lib.increment_counter)(p1, 999) };
    *counter = counter.wrapping_add(p1);

    unsafe { (lib.update_accumulator)(p2, 888) };
    *accumulator = accumulator.wrapping_mul(2).wrapping_add(p2);

    result =
        result.wrapping_add(unsafe { (lib.apply_operation)(Some(lib.add_three), p1, p2, p3) });
    result =
        result.wrapping_add(unsafe { (lib.apply_operation)(Some(lib.multiply_add), p2, p3, p4) });
    result =
        result.wrapping_add(unsafe { (lib.apply_operation)(Some(lib.complex_calc), p1, p3, p4) });

    let mut dynamic_data: Vec<c_int> = (0..10).map(|i| p1.wrapping_add(i)).collect();
    result = result.wrapping_add(unsafe {
        (lib.process_pointer_data)(dynamic_data.as_mut_ptr().add(5), p2)
    });

    unsafe { (lib.shift_array_data)(dynamic_data.as_mut_ptr(), 10, 3) };
    result = result.wrapping_add(dynamic_data[0]);

    result = result.wrapping_add(unsafe { (lib.get_time_based_value)(p3) });

    let mut records: Vec<DataRecord> = (0..5)
        .map(|i| {
            let mut r = DataRecord::zeroed();
            r.id = i;
            r.value = p4.wrapping_add(i.wrapping_mul(10));
            r
        })
        .collect();
    result =
        result.wrapping_add(unsafe { (lib.manipulate_records)(records.as_mut_ptr(), 5, 2) });

    result = result.wrapping_add(unsafe { (lib.compute_with_dynamic_memory)(p1, 8) });

    result = result.wrapping_add(counter.wrapping_add(*accumulator));
    result
}

#[test]
fn low_level_pipeline_matches_the_other_librarys_hatch() {
    // Four independent instances: two run the manual pipeline, two run `hatch`.
    let c_pipeline = load_private_copy("C(pipeline)", &c_so_path(), "c_pipeline");
    let r_hatch = load_private_copy("Rust(hatch)", &rust_so_path(), "r_hatch");
    let r_pipeline = load_private_copy("Rust(pipeline)", &rust_so_path(), "r_pipeline");
    let c_hatch = load_private_copy("C(hatch)", &c_so_path(), "c_hatch");

    let mut cp = (0i32, 0i32); // shadow globals of c_pipeline
    let mut rp = (0i32, 0i32); // shadow globals of r_pipeline

    let mut rng = rng();
    let mut cases: Vec<(c_int, c_int, c_int, c_int)> = Vec::new();
    for &a in &[0, 1, -1, i32::MIN, i32::MAX] {
        cases.push((a, a, a, a));
        cases.push((a, 1, -1, a));
    }
    for _ in 0..3_000 {
        cases.push((
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        ));
    }
    for _ in 0..2_000 {
        cases.push((
            rng.i32_small(),
            rng.i32_small(),
            rng.i32_small(),
            rng.i32_small(),
        ));
    }

    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        // Direction 1: C low-level pipeline vs Rust `hatch`.
        let c_manual = manual_hatch(&c_pipeline, &mut cp.0, &mut cp.1, p1, p2, p3, p4);
        let rust_one_shot = unsafe { (r_hatch.hatch)(p1, p2, p3, p4) };
        assert_eq!(
            c_manual, rust_one_shot,
            "case {i} hatch({p1}, {p2}, {p3}, {p4}): C low-level pipeline gave {c_manual} \
             but Rust hatch gave {rust_one_shot}"
        );

        // Direction 2: Rust low-level pipeline vs C `hatch`.
        let r_manual = manual_hatch(&r_pipeline, &mut rp.0, &mut rp.1, p1, p2, p3, p4);
        let c_one_shot = unsafe { (c_hatch.hatch)(p1, p2, p3, p4) };
        assert_eq!(
            r_manual, c_one_shot,
            "case {i} hatch({p1}, {p2}, {p3}, {p4}): Rust low-level pipeline gave {r_manual} \
             but C hatch gave {c_one_shot}"
        );

        // And all four agree with each other.
        assert_eq!(c_manual, c_one_shot, "case {i}: C pipeline vs C hatch");
        assert_eq!(r_manual, rust_one_shot, "case {i}: Rust pipeline vs Rust hatch");
    }
}
