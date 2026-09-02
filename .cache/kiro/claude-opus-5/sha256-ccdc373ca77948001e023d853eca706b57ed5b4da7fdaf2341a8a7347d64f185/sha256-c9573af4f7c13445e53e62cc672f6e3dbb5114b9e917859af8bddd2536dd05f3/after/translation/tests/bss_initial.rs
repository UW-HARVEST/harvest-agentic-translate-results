//! `ERRORS.md` row 5 / `CONFIGS.md` row 1 in their pristine form.
//!
//! This is its own integration-test binary so it owns a fresh process: the
//! exported `array` object is observed **as loaded**, before any other test has
//! written to it.  It must be all-zero `.bss` in both libraries, the objects
//! must be the same size, and one `perform_expensive_operations()` on that
//! untouched state must produce identical images.

mod harness;

use harness::ARRAY_SIZE;

#[test]
fn array_is_zero_bss_at_load_and_pxo_agrees() {
    let _g = harness::lock();
    let cl = harness::c();
    let rl = harness::rust();

    assert!(
        cl.array().iter().all(|&v| v == 0),
        "C `array` was not zero-initialised at load"
    );
    assert!(
        rl.array().iter().all(|&v| v == 0),
        "Rust `array` was not zero-initialised at load"
    );
    assert_eq!(cl.array().len(), ARRAY_SIZE);
    assert_eq!(rl.array().len(), ARRAY_SIZE);

    cl.pxo(1);
    rl.pxo(1);
    harness::assert_arrays_eq(
        "pristine .bss, k=1",
        1,
        &vec![0i32; ARRAY_SIZE],
        cl.array(),
        rl.array(),
    );

    // f^100(0) is a single value, so the whole image must be constant.
    let v = cl.array()[0];
    assert!(
        cl.array().iter().all(|&x| x == v),
        "f^100(0) should be uniform across the array"
    );
    assert!(rl.array().iter().all(|&x| x == v));
}
