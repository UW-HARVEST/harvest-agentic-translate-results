//! Harness self-check plus in-process differential test.
//!
//! This binary contains **exactly one** `#[test]` on purpose: it redirects file
//! descriptor 1 to capture what each library prints, and a second concurrently
//! running test would interleave its own progress output into that capture.
//! With a single test there is no other thread writing to stdout, so the
//! capture is exact no matter how `cargo test` is invoked.
//!
//! It loads both shared libraries into the *same* process at the same time,
//! which additionally proves that the two implementations export the same
//! symbols without confusing the dynamic loader, and it verifies that the
//! harness really observes the libraries' output (rather than comparing two
//! empty buffers).

mod common;

use common::*;

#[test]
fn both_libraries_loaded_in_one_process_agree() {
    // 1. The harness observes the C library's real output.
    let c_one = c_impl().driver(1);
    assert_eq!(
        String::from_utf8_lossy(&c_one),
        "01000000030000000000000000000040\n",
        "the C library's output was not captured as expected"
    );

    // 2. The Rust library, loaded in the same process, produces the same bytes.
    let r_one = rust_impl().driver(1);
    assert_eq!(c_one, r_one, "driver(1) diverged in-process");

    // 3. A spread of arguments, alternating between the two libraries so that
    //    neither can be affected by the other's state.
    let mut rng = Rng::new(0xC0E1);
    let mut values = vec![0, 1, -1, i32::MIN, i32::MAX, 0x0102_0304, -0x0102_0304];
    for _ in 0..64 {
        values.push(rng.next_i32());
    }
    for v in values {
        let c = c_impl().driver(v);
        let r = rust_impl().driver(v);
        assert_eq!(
            c,
            r,
            "driver({v}) diverged in-process\n  C   : {}\n  Rust: {}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert_eq!(c.len(), 33, "unexpected C output length for driver({v})");
    }

    // 4. The same `driver` symbol seen as `fn(i64)`: the unused upper half of
    //    the argument register must be ignored by both.
    for w in [
        0x0000_0001_0000_0000u64 as i64,
        0xffff_ffff_0000_002au64 as i64,
        i64::MIN,
        -1,
    ] {
        let c = c_impl().driver_wide(w);
        let r = rust_impl().driver_wide(w);
        assert_eq!(c, r, "driver({w:#018x}) as fn(i64) diverged in-process");
        assert_eq!(
            c,
            c_impl().driver(w as i32),
            "the C library did not ignore the upper half of {w:#018x}"
        );
    }

    // 5. The exported `main` of both libraries, and both linked executables,
    //    agree on a representative input (proves all four call paths work).
    let cm = run_main_via_so(c_lib(), b"42");
    let rm = run_main_via_so(rust_lib(), b"42");
    assert_eq!(
        String::from_utf8_lossy(&cm.stdout),
        "2a000000030000000000000000000040\n",
        "the C library's main() output was not captured as expected"
    );
    assert_eq!(cm.status, 0, "C main() must return 0");
    assert_eq!(cm, rm, "main() diverged for \"42\"");

    let ce = run_exe(c_exe(), b"7\n");
    let re = run_exe(rust_exe(), b"7\n");
    assert_eq!(
        String::from_utf8_lossy(&ce.stdout),
        "07000000030000000000000000000040\n",
        "the C executable's output was not captured as expected"
    );
    assert_eq!(ce, re, "executables diverged for \"7\\n\"");
}
