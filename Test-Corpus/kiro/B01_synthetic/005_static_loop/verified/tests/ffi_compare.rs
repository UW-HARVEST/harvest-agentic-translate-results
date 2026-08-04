use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libstatic_sum_c.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libstatic_sum.so")
}

/// Call static_sum with a sequence of updates and collect results.
unsafe fn run_sequence(
    lib: &Library,
    updates: &[i32],
) -> Vec<i32> {
    let func: Symbol<unsafe extern "C" fn(i32) -> i32> =
        lib.get(b"static_sum").expect("symbol static_sum not found");
    updates.iter().map(|&u| func(u)).collect()
}

/// The C library uses a process-global static, so we must load it fresh
/// each time (dlopen with unique handle). For the Rust lib we have a reset fn.
unsafe fn reset_rust(lib: &Library) {
    let reset: Symbol<unsafe extern "C" fn()> =
        lib.get(b"static_sum_reset").expect("static_sum_reset not found");
    reset();
}

#[test]
fn test_static_sum_loop_stride() {
    // Replicate what main() does: static_sum(i * stride) for i in 0..10
    for stride in [1, 2, 3, -1, 0, 100, -50] {
        let updates: Vec<i32> = (0..10).map(|i| i * stride).collect();

        // Load C lib fresh each time (new dlopen = fresh static)
        let c_results = unsafe {
            let c_lib = Library::new(c_lib_path()).expect("load C .so");
            run_sequence(&c_lib, &updates)
        };

        let rust_results = unsafe {
            let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
            reset_rust(&r_lib);
            run_sequence(&r_lib, &updates)
        };

        assert_eq!(
            c_results, rust_results,
            "Mismatch for stride={stride}: C={c_results:?} Rust={rust_results:?}"
        );
    }
}

#[test]
fn test_static_sum_accumulation() {
    // Test that the static variable accumulates across calls
    let updates = vec![1, 2, 3, 4, 5];

    let c_results = unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        run_sequence(&c_lib, &updates)
    };

    let rust_results = unsafe {
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        reset_rust(&r_lib);
        run_sequence(&r_lib, &updates)
    };

    assert_eq!(c_results, rust_results);
    // Expected: [1, 3, 6, 10, 15]
    assert_eq!(c_results, vec![1, 3, 6, 10, 15]);
}

#[test]
fn test_static_sum_negative() {
    let updates = vec![10, -3, -7, 5, -5];

    let c_results = unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        run_sequence(&c_lib, &updates)
    };

    let rust_results = unsafe {
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        reset_rust(&r_lib);
        run_sequence(&r_lib, &updates)
    };

    assert_eq!(c_results, rust_results);
}

#[test]
fn test_static_sum_zero() {
    let updates = vec![0, 0, 0];

    let c_results = unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        run_sequence(&c_lib, &updates)
    };

    let rust_results = unsafe {
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");
        reset_rust(&r_lib);
        run_sequence(&r_lib, &updates)
    };

    assert_eq!(c_results, rust_results);
    assert_eq!(c_results, vec![0, 0, 0]);
}
