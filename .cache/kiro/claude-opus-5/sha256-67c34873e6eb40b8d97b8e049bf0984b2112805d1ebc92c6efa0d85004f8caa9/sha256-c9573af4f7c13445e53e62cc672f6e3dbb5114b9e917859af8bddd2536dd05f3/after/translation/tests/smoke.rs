//! Harness smoke test: proves both `.so`s load and every one of the 20 C
//! symbols is resolvable from the Rust `.so` too (Phase D symbol parity,
//! checked at runtime through `dlsym` rather than only via `nm`).

mod common;

use common::*;

/// Every exported symbol of the C library, per SYMBOLS.md.
const ALL_SYMBOLS: &[&str] = &[
    "c2V",
    "c2Maxv",
    "c2Minv",
    "c2Clampv",
    "c2Sub",
    "c2Dot",
    "c2CircletoCircle",
    "c2CircletoAABB",
    "c2AABBtoAABB",
    "f2",
    "f3",
    "f4",
    "f5",
    "f7",
    "f9",
    "f10",
    "f11",
    "f12",
    "f13",
    "agglom",
];

#[test]
fn both_libraries_export_every_symbol() {
    let l = libs();
    for s in ALL_SYMBOLS {
        let _c: libloading::Symbol<*const ()> = l.c.get(s);
        let _r: libloading::Symbol<*const ()> = l.r.get(s);
    }
}

#[test]
fn statics_are_not_exported() {
    // These are `static` in the C source; the Rust build must not leak them
    // either (they would be a spurious extra symbol, not a parity failure,
    // but leaking `m__mantissa` etc. would indicate a fidelity slip).
    let l = libs();
    for s in [
        "cn_rnd_next",
        "lm_v2",
        "lm_sub2",
        "lm_dot2",
        "tflac_crc16_tables",
        "m__mantissa",
        "m__offset",
        "m__exponent",
    ] {
        assert!(!l.c.has(s), "C unexpectedly exports static `{s}`");
        assert!(!l.r.has(s), "Rust unexpectedly exports static `{s}`");
    }
}

#[test]
fn trivial_call_through_both() {
    let l = libs();
    let c: libloading::Symbol<FnF5> = l.c.get("f5");
    let r: libloading::Symbol<FnF5> = l.r.get("f5");
    unsafe {
        eq_u32("f5(1)", c(1), r(1));
    }
}
