//! Harness sanity + throughput probe. Confirms both `.so` files load, export
//! `hdr_compare`, and agree on a handful of inputs, and prints the measured
//! call rate used to size the exhaustive sweeps.

mod common;

use common::*;

#[test]
fn harness_loads_both_shared_objects() {
    let l = libs();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.rust_path.display());
    assert!(l.c_path.exists());
    assert!(l.rust_path.exists());

    // A known-valid MPEG-1 Layer III header pair (0xff 0xfb 0x90) must be
    // accepted, and an all-zero pair rejected. Expectations come from the C.
    let ok = [0xffu8, 0xfb, 0x90];
    assert_eq!(assert_same(&ok, &ok), 1, "C itself should accept 0xff 0xfb 0x90");
    let zero = [0u8; 3];
    assert_eq!(assert_same(&zero, &zero), 0, "C itself should reject 00 00 00");
}

#[test]
fn probe_call_throughput() {
    let l = libs();
    let n = 2_000_000usize;
    let mut rng = Rng::new(0xC0FFEE);
    let mut inputs = Vec::with_capacity(n);
    for _ in 0..n {
        inputs.push(([0, rng.next_u8(), rng.next_u8()], [0xff, rng.next_u8(), rng.next_u8()]));
    }
    let t = std::time::Instant::now();
    let mut acc = 0i64;
    for (h1, h2) in &inputs {
        let (c, r) = unsafe {
            (
                (l.c)(h1.as_ptr(), h2.as_ptr()),
                (l.rust)(h1.as_ptr(), h2.as_ptr()),
            )
        };
        assert_eq!(c, r, "divergence h1={h1:02x?} h2={h2:02x?}");
        acc += c as i64;
    }
    let el = t.elapsed();
    eprintln!(
        "throughput: {n} input pairs ({} FFI calls) in {:?} => {:.1} M pairs/s (accepted {acc})",
        2 * n,
        el,
        n as f64 / el.as_secs_f64() / 1e6
    );
}
