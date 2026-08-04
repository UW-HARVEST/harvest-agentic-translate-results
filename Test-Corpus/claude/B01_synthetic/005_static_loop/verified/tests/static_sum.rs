// Integration test: compares `static_sum` exported by the C and Rust .so files.
//
// Both libraries maintain a running sum via static / thread-local state.
// To keep the comparison deterministic regardless of cargo's test thread
// scheduling, this single test function runs every scenario in order on
// one thread, comparing C and Rust outputs call-for-call.

mod common;

use libloading::{Library, Symbol};

type StaticSum = unsafe extern "C" fn(update: i32) -> i32;

fn run_sequence(c_lib: &Library, r_lib: &Library, updates: &[i32], label: &str) {
    unsafe {
        let c_fn: Symbol<StaticSum> = c_lib
            .get(b"static_sum\0")
            .expect("C .so missing static_sum");
        let r_fn: Symbol<StaticSum> = r_lib
            .get(b"static_sum\0")
            .expect("Rust .so missing static_sum");

        // Snapshot the current accumulated state by adding 0.
        // This isolates this scenario from any prior accumulated state
        // in either library.
        let c_baseline = c_fn(0);
        let r_baseline = r_fn(0);

        for (i, u) in updates.iter().enumerate() {
            let c_out = c_fn(*u);
            let r_out = r_fn(*u);
            let c_delta = c_out.wrapping_sub(c_baseline);
            let r_delta = r_out.wrapping_sub(r_baseline);
            assert_eq!(
                c_delta, r_delta,
                "[{label}] mismatch at step {i} for update={u}: C delta={c_delta}, Rust delta={r_delta}"
            );
        }
    }
}

#[test]
fn static_sum_matches_c() {
    let c_lib = unsafe { Library::new(common::c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(common::rust_so_path()).expect("load Rust .so") };

    // Mirror the C program's loop with stride = 3 (the canonical scenario).
    let stride = 3i32;
    let updates: Vec<i32> = (0..10).map(|i| i * stride).collect();
    run_sequence(&c_lib, &r_lib, &updates, "stride=3");

    // Zero stride.
    let updates: Vec<i32> = (0..10).map(|i| i * 0).collect();
    run_sequence(&c_lib, &r_lib, &updates, "stride=0");

    // Negative stride.
    let stride = -7i32;
    let updates: Vec<i32> = (0..10).map(|i: i32| i.wrapping_mul(stride)).collect();
    run_sequence(&c_lib, &r_lib, &updates, "stride=-7");

    // Two's-complement wraparound exercise.
    let updates: Vec<i32> = vec![
        i32::MAX,
        1,
        i32::MAX,
        i32::MIN,
        -1,
        -1,
        i32::MIN,
        100,
        -100,
        0,
    ];
    run_sequence(&c_lib, &r_lib, &updates, "wrap");

    // Long mixed sequence.
    let updates: Vec<i32> = (-50..50).step_by(3).collect();
    run_sequence(&c_lib, &r_lib, &updates, "mixed");
}
