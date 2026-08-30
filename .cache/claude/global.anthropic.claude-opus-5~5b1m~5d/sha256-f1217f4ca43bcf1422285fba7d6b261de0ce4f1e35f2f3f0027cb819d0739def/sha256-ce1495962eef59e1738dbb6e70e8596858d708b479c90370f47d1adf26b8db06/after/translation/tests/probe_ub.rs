//! Diagnostic probe (`#[ignore]`d — it asserts nothing, the real checks live in
//! `configs.rs` / `errors.rs`). Run with:
//!
//! ```text
//! cargo test --test probe_ub -- --ignored --nocapture --test-threads=1
//! ```
//!
//! It prints, side by side, what each `.so` does on the paths whose behaviour is
//! decided by stale stack contents, so the parity can be inspected by eye and so
//! it is visible that these cases are *not* degenerate (they really do produce
//! stale bytes and real SIGSEGVs, not just an empty line every time).

mod common;

use common::*;
use std::ffi::c_char;

fn row(label: &str, f: impl Fn(&Api)) {
    let [c, r] = isolate_pair(f);
    let flag = if c == r { "MATCH   " } else { "MISMATCH" };
    eprintln!(
        "{flag} {label:<34} C: {:<52} Rust: {}",
        c.describe(),
        r.describe()
    );
}

#[test]
#[ignore]
fn probe_uninitialized_paths() {
    eprintln!();
    row("good()", |a| unsafe { a.good() });
    row("driver(1)", |a| unsafe { a.driver(1) });
    row("driver(0)", |a| unsafe { a.driver(0) });
    row("bad()", |a| unsafe { a.bad() });
    row("good(); bad()", |a| unsafe {
        a.good();
        a.bad()
    });
    row("driver(1); driver(0)", |a| unsafe {
        a.driver(1);
        a.driver(0)
    });
    row("printLine(\"AAAA…\"); bad()", |a| unsafe {
        let s = b"AAAAAAAAAAAAAAAA\0";
        a.print_line(s.as_ptr() as *const c_char);
        a.bad()
    });
    row("printLine(1 as *const)", |a| unsafe {
        a.print_line(1usize as *const c_char)
    });
    row("printLine(0xdeadbeef)", |a| unsafe {
        a.print_line(0xdead_beefusize as *const c_char)
    });
    for fill in [
        0u64,
        1,
        0x4141_4141_4141_4141,
        0xdead_beef_dead_beef,
        u64::MAX,
    ] {
        for depth in 0..3u32 {
            row(&format!("dirty({fill:#018x},{depth}); bad()"), |a| {
                dirty_stack(fill, depth);
                unsafe { a.bad() }
            });
            row(&format!("dirty({fill:#018x},{depth}); drv0"), |a| {
                dirty_stack(fill, depth);
                unsafe { a.driver(0) }
            });
        }
    }
}
