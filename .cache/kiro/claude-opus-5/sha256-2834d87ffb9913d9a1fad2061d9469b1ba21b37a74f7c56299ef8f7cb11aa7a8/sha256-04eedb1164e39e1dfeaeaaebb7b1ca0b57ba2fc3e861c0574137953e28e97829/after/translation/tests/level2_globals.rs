//! Level 2 — the file-scope globals `G_OP` and `G_OP_NAME`.
//!
//! ```c
//! int (*G_OP)(int,int) = OP_FN(OP);
//! const char *G_OP_NAME = STR(OP);
//! ```
//!
//! Both are *data* symbols, so the test reads the exported object and only then
//! calls through the stored pointer — the same thing `mdmain.c` does.

mod common;

use common::{Impl, OP, operand_pairs};

#[test]
fn g_op_name_matches() {
    let (c, r) = Impl::pair();
    let (cn, rn) = (c.g_op_name(), r.g_op_name());
    assert_eq!(
        cn,
        rn,
        "G_OP_NAME: C={} Rust={}",
        common::show(&cn),
        common::show(&rn)
    );
    // STR(OP) is the literal token CMake passed in.
    assert_eq!(cn, OP.as_bytes(), "G_OP_NAME should be {OP:?}");
}

#[test]
fn g_op_dispatches_identically() {
    let (c, r) = Impl::pair();
    let (cf, rf) = (c.g_op(), r.g_op());
    for (a, b) in operand_pairs() {
        assert_eq!(cf(a, b), rf(a, b), "G_OP({a}, {b})");
    }
}

/// `G_OP` must be the very operation named by `G_OP_NAME`, i.e. `OP_FN(OP)`.
#[test]
fn g_op_agrees_with_the_selected_op_function() {
    let (c, r) = Impl::pair();
    let selected = format!("op_{OP}");
    let (cg, rg) = (c.g_op(), r.g_op());
    let (cd, rd) = (c.fn2(&selected), r.fn2(&selected));
    for (a, b) in operand_pairs() {
        assert_eq!(cg(a, b), cd(a, b), "C G_OP vs {selected}({a}, {b})");
        assert_eq!(rg(a, b), rd(a, b), "Rust G_OP vs {selected}({a}, {b})");
    }
}

/// `G_OP` and `G_OP_NAME` are non-`const` objects in C, so an external consumer
/// may assign to them. Both objects must therefore live in writable storage in
/// the Rust object too; this writes through the exported symbol, observes the
/// effect and restores the original value.
#[test]
fn exported_globals_are_writable_in_both() {
    let (c, r) = Impl::pair();
    for i in [&c, &r] {
        let slot = i.g_op_slot();
        // SAFETY: `slot` is the address of the exported `G_OP` object.
        let original = unsafe { *slot };
        let replacement = i.fn2("op_mul");
        unsafe { *slot = replacement };
        let readback = unsafe { *slot };
        assert_eq!(
            readback(6, 7),
            42,
            "{}: writing G_OP had no effect",
            i.name
        );
        unsafe { *slot = original };
        assert_eq!(unsafe { *slot }(3, 4), original(3, 4), "{}: restore", i.name);

        let nslot = i.g_op_name_slot();
        let orig_name = unsafe { *nslot };
        let probe = c"probe".as_ptr();
        unsafe { *nslot = probe };
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(*nslot) }.to_bytes(),
            b"probe",
            "{}: writing G_OP_NAME had no effect",
            i.name
        );
        unsafe { *nslot = orig_name };
    }
    // After restoring, the two must still agree.
    assert_eq!(c.g_op_name(), r.g_op_name());
    let (cf, rf) = (c.g_op(), r.g_op());
    for (a, b) in operand_pairs() {
        assert_eq!(cf(a, b), rf(a, b), "G_OP({a}, {b}) after restore");
    }
}

/// `G_OP` and `G_OP_NAME` must be the same size and layout as in C: two adjacent
/// pointer-sized objects.
#[test]
fn exported_globals_have_pointer_size_in_both() {
    let (c, r) = Impl::pair();
    for i in [&c, &r] {
        assert_eq!(
            size_of::<extern "C" fn(std::ffi::c_int, std::ffi::c_int) -> std::ffi::c_int>(),
            size_of::<*const ()>(),
            "{}: function pointer size",
            i.name
        );
        assert!(!i.g_op_slot().is_null(), "{}: G_OP address", i.name);
        assert!(!i.g_op_name_slot().is_null(), "{}: G_OP_NAME address", i.name);
    }
}
