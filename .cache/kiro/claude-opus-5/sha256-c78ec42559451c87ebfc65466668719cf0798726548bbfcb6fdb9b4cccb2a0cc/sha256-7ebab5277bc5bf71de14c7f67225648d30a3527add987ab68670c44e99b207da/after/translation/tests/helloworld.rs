//! Differential tests for `helloworld`, the only function in the public API
//! (`c_src/include/hello.h`). It is a leaf function — it calls nothing else in
//! the library — so it is simultaneously the lowest- and highest-level entry
//! point in the call hierarchy.
//!
//! Every call goes through `dlopen`/`dlsym` on the two shared objects; the Rust
//! implementation is never invoked directly, so the `#[no_mangle]` export
//! wrapper is covered too.

mod common;

use common::{c_lib_path, outcome, run_helloworld, rust_lib_path};

const EXPECTED: &[u8] = b"Hello World!\n";

#[test]
fn helloworld_return_value_matches() {
    let c = outcome(&c_lib_path());
    let rust = outcome(&rust_lib_path());
    assert_eq!(c.rets, rust.rets, "return value mismatch");
    // The C ground truth returns 0 unconditionally.
    assert_eq!(c.rets, vec![0], "unexpected C return value");
}

#[test]
fn helloworld_stdout_is_byte_identical() {
    let c = outcome(&c_lib_path());
    let rust = outcome(&rust_lib_path());
    assert_eq!(
        c.stdout,
        rust.stdout,
        "stdout mismatch:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stdout,
        EXPECTED,
        "C ground truth emitted unexpected bytes: {:?}",
        String::from_utf8_lossy(&c.stdout)
    );
}

#[test]
fn helloworld_full_outcome_matches() {
    assert_eq!(outcome(&c_lib_path()), outcome(&rust_lib_path()));
}

/// Repeated invocation must stay in lockstep: no per-call state, and no missing
/// or duplicated newline when the stream stays open across calls.
#[test]
fn helloworld_repeated_calls_match() {
    for calls in [0usize, 1, 2, 5, 64, 1000] {
        let c = run_helloworld(&c_lib_path(), calls);
        let rust = run_helloworld(&rust_lib_path(), calls);
        assert_eq!(c.rets, rust.rets, "return values differ for {calls} call(s)");
        assert_eq!(
            c.stdout,
            rust.stdout,
            "stdout differs for {calls} call(s):\n  C    = {:?}\n  Rust = {:?}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&rust.stdout)
        );
        assert_eq!(
            c.stdout,
            EXPECTED.repeat(calls),
            "C output for {calls} call(s) was not {calls} copies of the greeting"
        );
        assert_eq!(c.rets, vec![0; calls]);
    }
}

/// The library must behave identically across independent `dlopen` cycles.
#[test]
fn helloworld_survives_reload_cycles() {
    for _ in 0..3 {
        assert_eq!(outcome(&c_lib_path()), outcome(&rust_lib_path()));
    }
}

/// Both shared objects must export `helloworld` with exactly that name; a
/// mangled or missing symbol makes `dlsym` fail and this test fail with it.
#[test]
fn helloworld_symbol_is_resolvable_in_both_libraries() {
    for path in [c_lib_path(), rust_lib_path()] {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let sym: Result<libloading::Symbol<unsafe extern "C" fn() -> std::ffi::c_int>, _> =
            unsafe { lib.get(b"helloworld\0") };
        assert!(
            sym.is_ok(),
            "`helloworld` not exported by {}",
            path.display()
        );
    }
}
