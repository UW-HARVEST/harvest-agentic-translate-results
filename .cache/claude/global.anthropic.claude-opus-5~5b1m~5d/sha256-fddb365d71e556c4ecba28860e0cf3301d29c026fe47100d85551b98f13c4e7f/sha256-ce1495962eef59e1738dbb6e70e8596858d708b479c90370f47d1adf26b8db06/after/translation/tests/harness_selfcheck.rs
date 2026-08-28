// Harness self-checks.
//
// A differential suite that silently captures nothing, or that loads the same
// object twice, would "pass" everything. These tests make such a failure loud.

mod common;

use common::*;

#[test]
fn loads_two_distinct_objects_one_c_one_rust() {
    let c = c_so_path().canonicalize().unwrap();
    let r = rust_so_path().canonicalize().unwrap();
    assert_ne!(c, r, "the two .so paths must differ");
    assert!(c.to_string_lossy().contains("c_src"), "C .so is {c:?}");
    assert!(
        r.file_name().unwrap() == "libconfusion_lib.so",
        "Rust .so is {r:?}"
    );
}

/// `cargo test` does not build a cdylib, so the harness must never fall back to
/// the other profile's object — that would invalidate the whole release run.
#[test]
fn rust_so_matches_the_profile_under_test() {
    let r = rust_so_path();
    let parent = r.parent().unwrap().file_name().unwrap().to_string_lossy().to_string();
    if std::env::var("RUST_LIB_PATH").is_ok() {
        return; // explicitly overridden (mutation check)
    }
    let expected = if cfg!(debug_assertions) { "debug" } else { "release" };
    assert_eq!(
        parent, expected,
        "test binary is {expected} but it loaded {r:?}"
    );
}

#[test]
fn capture_actually_captures_library_stdout() {
    let (c, r) = both();

    let (rc, out_c) = capture(|| unsafe { (c.confusion)(1234, 42, 7, 1) });
    let (rr, out_r) = capture(|| unsafe { (r.confusion)(1234, 42, 7, 1) });

    assert!(!out_c.is_empty(), "captured nothing from the C .so");
    assert!(!out_r.is_empty(), "captured nothing from the Rust .so");
    // The exact text the C source prints, so a mis-wired capture cannot pass.
    let text = String::from_utf8(out_c.clone()).unwrap();
    for expected in [
        "Debug: param1 = 1234\n",
        "Debug: param2 = 42\n",
        "Debug: param3 = 7\n",
        "Debug: param4 = 1\n",
        "Debug: state->flags.counter = 1\n",
        "Bit fields - flag1:0 flag2:1 flag3:0 mode:5\n",
        "Read as float: ",
        "Final result: ",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text:?}");
    }
    assert_eq!(out_c, out_r);
    assert_eq!(rc, rr);
}

#[test]
fn capture_is_isolated_between_calls() {
    let c = c_lib();
    let (_, a) = capture(|| unsafe { (c.confusion)(1, 0, 0, 0) });
    let (_, b) = capture(|| {});
    assert!(!a.is_empty());
    assert!(
        b.is_empty(),
        "a capture leaked into the next one: {:?}",
        show(&b)
    );
}

/// The struct layout the snapshot helper assumes must match what the C actually
/// writes; otherwise every state comparison would be reading noise.
#[test]
fn process_state_layout_matches_the_c_library() {
    let c = c_lib();
    let (_, _) = capture(|| unsafe {
        let s = (c.create_state)(-987654321, 77);
        assert!(!s.is_null());
        let snap = snapshot(s);
        // create_state assigns every bit of the 32-bit bit-field unit:
        // flag1=1, flag2=0, flag3=1, counter=0, mode=3, status=15, reserved=0
        assert_eq!(snap.flags, 0x0000_7B05, "flags word");
        assert_eq!((snap.flag1(), snap.flag2(), snap.flag3()), (1, 0, 1));
        assert_eq!(snap.counter(), 0);
        assert_eq!(snap.mode(), 3);
        assert_eq!(snap.status(), 15);
        assert_eq!(snap.reserved(), 0);
        assert_eq!(snap.data, (-987654321i32) as u32, "union payload");
        assert_eq!(snap.capacity, 77);
        assert_eq!(
            snap.buffer.as_deref(),
            Some(&b"State:-987654321:Mode:3"[..]),
            "snprintf output"
        );
        (c.destroy_state)(s);
    });
    assert_eq!(STATE_SIZE, 24);
    assert_eq!((OFF_FLAGS, OFF_DATA, OFF_BUFFER, OFF_CAPACITY), (0, 4, 8, 16));
}

/// The differential driver must fail when the two sides disagree.
#[test]
fn diff_driver_detects_a_divergence() {
    // Feed the driver a scenario whose recorded log depends on which library it
    // is (the library *name*), i.e. an artificial divergence.
    let outcome = std::panic::catch_unwind(|| {
        diff("selfcheck-divergence", &|lib, log| {
            log.push(format!("name={}", lib.name));
        })
    });
    assert!(
        outcome.is_err(),
        "the diff driver failed to notice a divergent log"
    );

    // Same for stdout: only one of the two sides prints.
    let outcome = std::panic::catch_unwind(|| {
        diff("selfcheck-stdout-divergence", &|lib, _log| unsafe {
            if lib.name == "C" {
                let s = (lib.create_state)(5, 128);
                (lib.update_flags)(s, 1); // prints two lines
                (lib.destroy_state)(s);
            }
        })
    });
    assert!(
        outcome.is_err(),
        "the diff driver failed to notice divergent stdout"
    );
}
