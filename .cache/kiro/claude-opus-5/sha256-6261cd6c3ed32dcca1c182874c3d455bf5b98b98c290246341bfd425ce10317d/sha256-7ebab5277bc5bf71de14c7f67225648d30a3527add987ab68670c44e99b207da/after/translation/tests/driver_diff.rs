//! Differential test: the exported `driver` symbol from the C shared library
//! and from the Rust cdylib must produce byte-identical output for every input.
//!
//! Both libraries are loaded through `libloading` and invoked purely through
//! their FFI exports, so the `#[no_mangle]` wrapper is exercised exactly as an
//! external caller would exercise it.
//!
//! This test uses `harness = false`: the checks temporarily redirect the
//! process's stdout, which is incompatible with libtest's parallel progress
//! reporting on the same descriptor.

mod common;

use common::{DriverFn, c_lib_path, capture_stderr, capture_stdout, rust_lib_path};
use libloading::{Library, Symbol};

struct Libs {
    _c: Library,
    _r: Library,
    c_driver: DriverFn,
    r_driver: DriverFn,
}

fn load() -> Libs {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C .so");
        let r = Library::new(rust_lib_path()).expect("load Rust .so");
        let cs: Symbol<DriverFn> = c.get(b"driver\0").expect("C .so exports `driver`");
        let rs: Symbol<DriverFn> = r.get(b"driver\0").expect("Rust .so exports `driver`");
        let c_driver = *cs;
        let r_driver = *rs;
        Libs {
            _c: c,
            _r: r,
            c_driver,
            r_driver,
        }
    }
}

/// Call both implementations with the same arguments and compare their output.
fn compare(libs: &Libs, x: i32, y: i32, z: i32) {
    let c_out = capture_stdout(|| unsafe { (libs.c_driver)(x, y, z) });
    let r_out = capture_stdout(|| unsafe { (libs.r_driver)(x, y, z) });
    assert_eq!(
        c_out,
        r_out,
        "mismatch for driver({x}, {y}, {z})\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

/// The four distinct branches of `multi_stage`: x-failure, y-failure,
/// z-failure, and the success path.
fn every_branch(libs: &Libs) {
    compare(libs, 0, 2, 3); // x != 1
    compare(libs, 1, 0, 3); // x == 1, y != 2
    compare(libs, 1, 2, 0); // x == 1, y == 2, z != 3
    compare(libs, 1, 2, 3); // success
}

/// Exhaustive sweep of the small neighbourhood containing every branch boundary.
fn exhaustive_small_grid(libs: &Libs) {
    for x in -3..=5 {
        for y in -3..=5 {
            for z in -3..=5 {
                compare(libs, x, y, z);
            }
        }
    }
}

/// Extremes, plus values a naive translation could mishandle (`123` is the
/// initial value of the file-scope `y`; `i32::MIN` has no positive counterpart).
fn edge_values(libs: &Libs) {
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        -123,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        122,
        123,
        124,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &x in &interesting {
        for &y in &interesting {
            for &z in &interesting {
                compare(libs, x, y, z);
            }
        }
    }
}

/// `y` is file-scope state in C (`static int y = 123;`) that `driver` overwrites
/// on entry. Alternating success and failure calls exercises that store and
/// confirms no value leaks between calls.
fn static_state_is_per_call(libs: &Libs) {
    for &(x, y, z) in &[
        (1, 2, 3),
        (1, 999, 3),
        (1, 2, 3),
        (5, 2, 3),
        (1, 2, 3),
        (1, 2, 77),
        (1, 2, 3),
        (1, 123, 3),
        (1, 2, 3),
    ] {
        compare(libs, x, y, z);
    }
}

/// A repeated sequence must be stable: no hidden state makes later calls differ.
fn repeated_calls_are_stable(libs: &Libs) {
    let first = capture_stdout(|| unsafe { (libs.c_driver)(1, 2, 3) });
    for _ in 0..50 {
        let c_out = capture_stdout(|| unsafe { (libs.c_driver)(1, 2, 3) });
        let r_out = capture_stdout(|| unsafe { (libs.r_driver)(1, 2, 3) });
        assert_eq!(c_out, first, "C output drifted across repeated calls");
        assert_eq!(r_out, first, "Rust output drifted across repeated calls");
    }
}

/// The Rust library must write through the *same* libc `stdout` stream as the C
/// library rather than a separate Rust-side buffer. Otherwise output from the
/// two, captured together, would emerge reordered instead of in call order.
fn output_interleaves_in_call_order(libs: &Libs) {
    let mixed = capture_stdout(|| unsafe {
        (libs.c_driver)(1, 2, 3);
        (libs.r_driver)(9, 9, 9);
        (libs.c_driver)(1, 5, 3);
        (libs.r_driver)(1, 2, 3);
    });

    // Expected: each call's own output concatenated in call order.
    let mut expected = Vec::new();
    for &(x, y, z) in &[(1, 2, 3), (9, 9, 9), (1, 5, 3), (1, 2, 3)] {
        expected.extend_from_slice(&capture_stdout(|| unsafe { (libs.c_driver)(x, y, z) }));
    }

    assert_eq!(
        mixed,
        expected,
        "interleaved output diverged\n got: {:?}\nwant: {:?}",
        String::from_utf8_lossy(&mixed),
        String::from_utf8_lossy(&expected),
    );
}

/// Neither library may write to stderr; all output belongs on stdout.
fn nothing_is_written_to_stderr(libs: &Libs) {
    for &(x, y, z) in &[(1, 2, 3), (0, 0, 0), (1, 2, 9), (1, 9, 3)] {
        let c_err = capture_stderr(|| unsafe { (libs.c_driver)(x, y, z) });
        let r_err = capture_stderr(|| unsafe { (libs.r_driver)(x, y, z) });
        assert!(c_err.is_empty(), "C wrote to stderr: {c_err:?}");
        assert_eq!(c_err, r_err, "stderr differs for ({x}, {y}, {z})");
    }
}

/// Every dynamic symbol defined by the C .so must also be defined by the Rust
/// .so under the exact same name.
fn exported_symbols_match() {
    let c = dynamic_defined_symbols(&c_lib_path());
    let r = dynamic_defined_symbols(&rust_lib_path());
    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}"
    );
    assert!(
        c.contains("driver"),
        "sanity check failed: C .so does not export `driver` (nm unavailable?)"
    );
}

fn dynamic_defined_symbols(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn main() {
    let libs = load();

    // Run sequentially: the checks redirect process-wide file descriptors.
    let checks: Vec<(&str, Box<dyn Fn(&Libs)>)> = vec![
        ("every_branch", Box::new(every_branch)),
        ("exhaustive_small_grid", Box::new(exhaustive_small_grid)),
        ("edge_values", Box::new(edge_values)),
        ("static_state_is_per_call", Box::new(static_state_is_per_call)),
        ("repeated_calls_are_stable", Box::new(repeated_calls_are_stable)),
        (
            "output_interleaves_in_call_order",
            Box::new(output_interleaves_in_call_order),
        ),
        (
            "nothing_is_written_to_stderr",
            Box::new(nothing_is_written_to_stderr),
        ),
        ("exported_symbols_match", Box::new(|_| exported_symbols_match())),
    ];

    println!("running {} checks", checks.len());
    for (name, check) in &checks {
        check(&libs);
        println!("check {name} ... ok");
    }
    println!("\nall {} checks passed", checks.len());
}
