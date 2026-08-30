//! Phase C, `ERRORS.md` rows 15–17 — the two exported **data** symbols.
//!
//! These live in their own integration-test binary on purpose. `G_OP` and
//! `G_OP_NAME` are process-global mutable objects inside the two `dlopen`ed
//! libraries, and the tests here deliberately *overwrite* them. `dlopen` of an
//! already-loaded path is reference-counted and returns the same mapping, so a
//! clobbering test sharing a process with a test that reads `G_OP` would race.
//! Cargo runs each integration-test file as a separate process, which isolates
//! the clobbering from `valid_paths.rs` / `errors.rs` / `symbols.rs`; within
//! this file a mutex serialises the two tests as well.

mod common;

use std::ffi::c_char;
use std::sync::Mutex;

use common::{load_pair, same, OP_NAME};

/// Serialises the tests in this binary, which mutate process-global state.
static GLOBALS: Mutex<()> = Mutex::new(());

/// `ERRORS.md` rows 15–16: `G_OP` lives in writable `.data` (`nm` type `D`), so
/// an external consumer may legally store through the `dlsym` address. The store
/// must **succeed** in both libraries (an immutable Rust `static` would have
/// been emitted into `.data.rel.ro` and faulted here), and — because the library
/// itself uses `OP_FN(OP)` rather than reading `G_OP` — overwriting it must not
/// change what `helper_call` / `helper_ptr` / `use_generated` compute.
#[test]
fn rows_15_16_g_op_is_writable_and_not_used_internally() {
    let _guard = GLOBALS.lock().unwrap();
    let (c, r) = load_pair();

    let baseline_a = 123456;
    let baseline_b = -654321;
    // SAFETY: all three are exported with the signatures declared in `Api`.
    let before = unsafe {
        (
            (
                (c.helper_call)(baseline_a, baseline_b),
                (r.helper_call)(baseline_a, baseline_b),
            ),
            (
                (c.helper_ptr)(baseline_a, baseline_b),
                (r.helper_ptr)(baseline_a, baseline_b),
            ),
            ((c.use_generated)(4), (r.use_generated)(4)),
        )
    };
    same("helper_call", "baseline", before.0 .0, before.0 .1);
    same("helper_ptr", "baseline", before.1 .0, before.1 .1);
    same("use_generated", "4 baseline", before.2 .0, before.2 .1);

    let c_orig = c.g_op_value();
    let r_orig = r.g_op_value();
    assert!(c_orig.is_some(), "C: G_OP must start non-null");
    assert!(r_orig.is_some(), "Rust: G_OP must start non-null");

    // Row 15: store a *different, valid* op into G_OP through the dlsym address.
    c.set_g_op(Some(c.op_mul));
    r.set_g_op(Some(r.op_mul));
    assert_eq!(
        c.g_op_value().unwrap() as usize,
        c.op_mul as usize,
        "ERRORS.md row 15: the store into C's G_OP did not take effect"
    );
    assert_eq!(
        r.g_op_value().unwrap() as usize,
        r.op_mul as usize,
        "ERRORS.md row 15: the store into Rust's G_OP did not take effect \
         (is G_OP a `static mut` in .data, not an immutable static in .data.rel.ro?)"
    );
    // Calling through the clobbered global must reach the newly-stored function
    // in both libraries — i.e. the global really is the indirection point.
    for (a, b) in [(3, 4), (-7, 9), (i32::MAX, 2)] {
        // SAFETY: `op_mul` has the C signature `int(int,int)`.
        let expect = unsafe { (c.op_mul)(a, b) };
        assert_eq!(c.call_g_op(a, b), expect);
        assert_eq!(r.call_g_op(a, b), expect);
    }

    // Row 16: store a null function pointer. The library never dereferences
    // G_OP, so nothing must break.
    c.set_g_op(None);
    r.set_g_op(None);
    assert!(c.g_op_value().is_none());
    assert!(
        r.g_op_value().is_none(),
        "ERRORS.md row 16: Rust G_OP not writable"
    );

    // With G_OP clobbered (and null), the library's own behaviour is unchanged.
    // SAFETY: as above.
    let after = unsafe {
        (
            (
                (c.helper_call)(baseline_a, baseline_b),
                (r.helper_call)(baseline_a, baseline_b),
            ),
            (
                (c.helper_ptr)(baseline_a, baseline_b),
                (r.helper_ptr)(baseline_a, baseline_b),
            ),
            ((c.use_generated)(4), (r.use_generated)(4)),
        )
    };
    assert_eq!(
        before, after,
        "ERRORS.md row 16: clobbering G_OP changed library behaviour \
         (the C library uses OP_FN(OP), not G_OP, internally)"
    );

    // Restore so the process is left in a pristine state.
    c.set_g_op(c_orig);
    r.set_g_op(r_orig);
    assert_eq!(c.g_op_value().unwrap() as usize, c.selected_op() as usize);
    assert_eq!(r.g_op_value().unwrap() as usize, r.selected_op() as usize);
}

/// `ERRORS.md` row 17: `G_OP_NAME` is likewise a writable `.data` object (only
/// the *pointee* is `const`), so storing a new pointer must succeed in both.
#[test]
fn row_17_g_op_name_pointer_is_writable() {
    let _guard = GLOBALS.lock().unwrap();
    let (c, r) = load_pair();
    let c_orig = c.g_op_name_ptr();
    let r_orig = r.g_op_name_ptr();
    assert_eq!(c.g_op_name_bytes(), OP_NAME.as_bytes());
    assert_eq!(r.g_op_name_bytes(), OP_NAME.as_bytes());

    let replacement: *const c_char = c"clobbered".as_ptr();
    c.set_g_op_name(replacement);
    r.set_g_op_name(replacement);

    assert_eq!(
        c.g_op_name_bytes(),
        b"clobbered",
        "ERRORS.md row 17: the store into C's G_OP_NAME did not take effect"
    );
    assert_eq!(
        r.g_op_name_bytes(),
        b"clobbered",
        "ERRORS.md row 17: the store into Rust's G_OP_NAME did not take effect \
         (is G_OP_NAME a `static mut` in .data?)"
    );

    c.set_g_op_name(c_orig);
    r.set_g_op_name(r_orig);
    assert_eq!(c.g_op_name_bytes(), OP_NAME.as_bytes());
    assert_eq!(r.g_op_name_bytes(), OP_NAME.as_bytes());
}
