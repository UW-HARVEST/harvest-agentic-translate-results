//! Meta-test: proves the harness really captures library output and really
//! would fail on a mismatch, so a green comparison run means something.

mod common;

use common::{call_driver, call_run, house_t, show, Impl};

#[test]
fn harness_is_not_vacuous() {
    // 1. Both libraries actually produce output through the capture.
    let c_out = call_driver(Impl::C, b"3");
    let rust_out = call_driver(Impl::Rust, b"3");
    assert!(!c_out.is_empty(), "C driver produced no captured output");
    assert!(!rust_out.is_empty(), "Rust driver produced no captured output");
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        8,
        "expected 8 lines from driver(\"3\"), got:\n{}",
        show(&c_out)
    );
    assert!(show(&c_out).starts_with("The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n"));

    // 2. The error path is reached and captured.
    let err_c = call_driver(Impl::C, b"zzz");
    let err_rust = call_driver(Impl::Rust, b"zzz");
    assert_eq!(show(&err_c), "An error occurred\n");
    assert_eq!(show(&err_rust), "An error occurred\n");

    // 3. Distinct inputs really do produce distinct captures, i.e. the capture
    //    is not returning stale or empty data that would mask a mismatch.
    let other = call_driver(Impl::C, b"4");
    assert_ne!(c_out, other, "capture is insensitive to the input value");
    assert_ne!(c_out, err_c, "capture is insensitive to the code path taken");

    // 4. `run` mutates the caller's struct in both implementations, and the
    //    struct comparison is therefore meaningful.
    let start = house_t {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    };
    let (out_c, after_c) = call_run(Impl::C, start, 10);
    let (out_rust, after_rust) = call_run(Impl::Rust, start, 10);
    assert!(!out_c.is_empty() && !out_rust.is_empty());
    assert_eq!(after_c.floors, 3);
    assert_eq!(after_c.bedrooms, 15);
    assert_eq!(after_c.bathrooms, 3.5);
    assert_eq!(after_c.raw(), after_rust.raw());
    assert_ne!(
        after_c.raw(),
        start.raw(),
        "run() did not mutate the struct, so struct comparison proves nothing"
    );

    // 5. The two libraries are genuinely different objects: their `driver`
    //    symbol addresses must differ.
    let libs = common::libs();
    unsafe {
        let c_sym: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
            libs.c.get(b"driver\0").unwrap();
        let r_sym: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
            libs.rust.get(b"driver\0").unwrap();
        let c_addr = *c_sym as usize;
        let r_addr = *r_sym as usize;
        assert_ne!(
            c_addr, r_addr,
            "C and Rust `driver` resolve to the same address; the same library was loaded twice"
        );
    }
}
