//! Harness self-checks. If any of these fail, every other result in this
//! suite is meaningless, so they run first (alphabetically `aa_`).

mod common;
use common::*;

#[test]
fn aa_two_distinct_libraries_are_loaded() {
    let (c, r) = pair();
    // Same linker name, different code: the two dlopen handles must resolve to
    // different addresses, otherwise we would be comparing a library with
    // itself and every test would pass vacuously.
    let ca = c.spec_ray as usize;
    let ra = r.spec_ray as usize;
    assert_ne!(
        ca, ra,
        "C and Rust `spec_ray` resolved to the same address ({ca:#x}) -- \
         only one library is actually loaded"
    );
    for (n, a, b) in [
        ("c2V", c.c2V as usize, r.c2V as usize),
        ("c2Dot", c.c2Dot as usize, r.c2Dot as usize),
        ("c2Norm", c.c2Norm as usize, r.c2Norm as usize),
        ("c2CastRay", c.c2CastRay as usize, r.c2CastRay as usize),
        (
            "c2RaytoCapsule",
            c.c2RaytoCapsule as usize,
            r.c2RaytoCapsule as usize,
        ),
    ] {
        assert_ne!(a, b, "{n} resolved to the same address in both libraries");
    }
}

#[test]
fn ab_diff_actually_detects_divergence() {
    // Feed `Diff` a known mismatch and confirm it panics: guards against a
    // comparison helper that silently accepts everything.
    let res = std::panic::catch_unwind(|| {
        let mut d = Diff::new("self-test");
        d.f32_bits(|| "0.0 vs -0.0".into(), 0.0f32, -0.0f32);
        d.finish();
    });
    assert!(res.is_err(), "Diff::finish accepted +0.0 vs -0.0");

    let res = std::panic::catch_unwind(|| {
        let mut d = Diff::new("self-test nan payload");
        d.f32_bits(
            || "nan payloads".into(),
            f32::from_bits(0x7FC0_0001),
            f32::from_bits(0x7FC0_0002),
        );
        d.finish();
    });
    assert!(res.is_err(), "Diff::finish accepted differing NaN payloads");

    // And that it accepts genuinely identical bits, including NaN == NaN.
    let mut d = Diff::new("self-test ok");
    d.f32_bits(|| "same nan".into(), f32::NAN, f32::NAN);
    d.f32_bits(|| "-0".into(), -0.0f32, -0.0f32);
    d.finish();
}

#[test]
fn ac_out_buffer_is_byte_comparable() {
    let (c, r) = pair();
    // A guaranteed miss must leave the pre-fill pattern untouched in BOTH.
    let ray = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    let far = c2Circle {
        p: c2v { x: 1000.0, y: 1000.0 },
        r: 1.0,
    };
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    let cr = unsafe { (c.c2RaytoCircle)(ray, far, cb.as_ptr()) };
    let rr = unsafe { (r.c2RaytoCircle)(ray, far, rb.as_ptr()) };
    assert_eq!(cr, 0, "expected a miss from C");
    assert_eq!(rr, 0, "expected a miss from Rust");
    assert_eq!(cb, OUT_FILL, "C wrote to *out on a miss");
    assert_eq!(rb, OUT_FILL, "Rust wrote to *out on a miss");

    // A guaranteed hit must overwrite all 12 bytes in both.
    let near = c2Circle {
        p: c2v { x: 5.0, y: 0.0 },
        r: 1.0,
    };
    let ray2 = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    };
    let mut cb = OutBuf::filled();
    let mut rb = OutBuf::filled();
    assert_eq!(unsafe { (c.c2RaytoCircle)(ray2, near, cb.as_ptr()) }, 1);
    assert_eq!(unsafe { (r.c2RaytoCircle)(ray2, near, rb.as_ptr()) }, 1);
    assert_ne!(cb, OUT_FILL, "C did not write *out on a hit");
    assert_eq!(cb, rb, "hit out-parameter bytes differ: {cb:?} vs {rb:?}");
    assert_eq!(cb.words()[0], 4.0f32.to_bits(), "t should be exactly 4.0");
}

#[test]
fn ad_report_loaded_paths() {
    // Not an assertion, just makes the run self-documenting under
    // `cargo test -- --nocapture`.
    let (c, r) = pair();
    println!("C impl   = {} @ {:#x}", c.name, c.spec_ray as usize);
    println!("Rust impl= {} @ {:#x}", r.name, r.spec_ray as usize);
}
