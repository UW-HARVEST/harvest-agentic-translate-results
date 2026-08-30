// Harness self-check: proves the capture mechanism and both `.so` loads work
// before any of the real differential suites are trusted.
//
// `harness = false` — see `tests/common/mod.rs::Runner`.

mod common;

use common::*;

fn main() {
    let mut r = Runner::new("smoke");

    r.case("both_libraries_load_and_expose_all_four_symbols", || {
        let c = c_lib();
        let rl = rust_lib();
        println!("\n    C   .so: {:?}", c.path);
        println!("    Rust.so: {:?}", rl.path);
        // Resolution happened in `Lib::load`; reaching here means all four
        // `dlsym` lookups succeeded in both shared objects.
        assert_eq!(c.name, "C");
        assert_eq!(rl.name, "Rust");
    });

    r.case("capture_actually_captures_library_output", || {
        // If the capture were broken (e.g. it always returned an empty Vec)
        // every differential test would pass vacuously. Pin a known-non-empty
        // output from each library.
        let out = capture(|| unsafe { c_lib().good_raw() });
        assert_eq!(out, GOOD_OUTPUT, "capture of the C good() misbehaved");
        let out = capture(|| unsafe { rust_lib().good_raw() });
        assert_eq!(out, GOOD_OUTPUT, "capture of the Rust good() misbehaved");
    });

    r.case("capture_is_isolated_between_calls", || {
        let a = capture(|| unsafe { c_lib().good_raw() });
        let b = capture(|| {});
        let c = capture(|| unsafe { c_lib().good_raw() });
        assert_eq!(a, GOOD_OUTPUT);
        assert!(b.is_empty(), "leaked bytes into a no-op capture: {b:?}");
        assert_eq!(c, GOOD_OUTPUT);
    });

    r.case("capture_survives_large_output", || {
        // 1 MiB in one go: makes sure the scratch-file capture does not deadlock
        // or truncate the way a fixed-size pipe would.
        let payload = vec![b'Z'; 1 << 20];
        let out = with_cstr(&payload, |p| capture(|| unsafe { c_lib().print_line_raw(p) }));
        assert_eq!(out.len(), (1 << 20) + 1);
    });

    r.case("capture_detects_a_real_difference", || {
        // Negative control: if `assert_same` could never fail, the whole suite
        // would be meaningless. Feed the two libraries deliberately different
        // work and require the comparison to report a mismatch.
        let c_out = capture(|| unsafe { c_lib().good_raw() });
        let r_out = capture(|| unsafe { rust_lib().bad_raw() });
        assert_ne!(
            c_out, r_out,
            "the comparison machinery cannot distinguish good() from bad()"
        );
    });

    r.case("rng_is_deterministic", || {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    });

    r.finish();
}
