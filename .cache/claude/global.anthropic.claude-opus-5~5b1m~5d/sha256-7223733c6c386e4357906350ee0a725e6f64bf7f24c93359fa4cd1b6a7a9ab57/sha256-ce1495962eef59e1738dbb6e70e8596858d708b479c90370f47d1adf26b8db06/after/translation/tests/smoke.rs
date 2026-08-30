//! Harness self-check: both `.so`s load, export `slice`, and the fd-level
//! stdout capture really observes what the *loaded library* printed.
//!
//! One `#[test]` only — see the note on `common::Suite` for why.

mod common;

use common::*;

#[test]
fn smoke() {
    silence_panic_hook();
    let mut s = Suite::new("smoke");

    s.row("both_libraries_load_and_export_slice", || {
        let c = c_so_path();
        let r = rust_so_path();
        assert!(c.exists(), "missing {}", c.display());
        assert!(r.exists(), "missing {}", r.display());
        // Resolving the symbols panics inside the harness if either is absent.
        let _ = slice_fn(Impl::C);
        let _ = slice_fn(Impl::Rust);
    });

    s.row("capture_sees_the_printed_slice", || {
        // The capture must contain exactly the slice plus the newline that
        // `printf("%.*s\n", ...)` appends — nothing more.
        for which in [Impl::C, Impl::Rust] {
            let out = call(which, b"hello\0", None, None);
            assert_eq!(out.ret, 0, "{}", which.name());
            assert_eq!(out.stdout, b"hello\n", "{}: {:?}", which.name(), out.stdout);
        }
    });

    s.row("capture_sees_the_error_message", || {
        for which in [Impl::C, Impl::Rust] {
            let out = call(which, b"hello\0", Some(99), None);
            assert_eq!(out.ret, 1, "{}", which.name());
            assert_eq!(
                out.stdout, b"Error: start is off the end of the string!\n",
                "{}",
                which.name()
            );
        }
    });

    s.row("happy_path_matches", || {
        assert_same_str("smoke/whole", b"hello world", None, None);
        assert_same_str("smoke/suffix", b"hello world", Some(6), None);
        assert_same_str("smoke/prefix", b"hello world", None, Some(5));
        assert_same_str("smoke/window", b"hello world", Some(2), Some(7));
    });

    s.finish();
}
