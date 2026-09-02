//! Phase D — symbol parity plus cross-library ABI interoperability.
//!
//! The symbol test shells out to `nm -D` on both `.so`s and requires the
//! defined-symbol diff to be EMPTY. The interop tests hand objects produced by
//! one implementation to the other, which is only sound if `matrix_t`'s layout,
//! calling convention and allocator are byte-compatible.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::process::Command;

fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

fn c_so() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent();
    while let Some(d) = dir {
        let c = d.join("libdriver.so");
        if c.exists() {
            return c;
        }
        dir = d.parent();
    }
    panic!("Rust libdriver.so not found");
}

/// Phase D gate: every symbol the C `.so` exports must be exported by the Rust
/// `.so` under the exact same name.
#[test]
fn symbol_parity_diff_is_empty() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but missing from Rust: {missing:?}"
    );
    // Sanity: the C library really does export the seven documented entries, so
    // an empty diff cannot be the result of an empty C symbol set.
    for want in [
        "allocate_matrix",
        "free_matrix",
        "initialize_matrix_from_string",
        "multiply_matrices",
        "matrix_to_string",
        "write_to_file",
        "driver",
    ] {
        assert!(c.contains(want), "C .so unexpectedly lacks {want}");
        assert!(r.contains(want), "Rust .so lacks {want}");
    }
    assert_eq!(c.len(), 7, "C symbol set changed: {c:?}");
}

/// The Rust `.so` must not leave any non-libc symbol undefined.
#[test]
fn no_unresolved_nonlibc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so())
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // Everything legitimately unresolved comes from glibc, libgcc's
            // unwinder, or the weak ITM/gmon hooks.
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("__gmon_start__")
                || s.starts_with("_Unwind_")
                || s.starts_with("__cxa_")
                || *s == "gettid"
                || *s == "statx")
        })
        .collect();
    assert!(bad.is_empty(), "unresolved non-libc symbols: {bad:?}");
}

/// A `matrix_t` allocated by one `.so` must be fully usable by the other:
/// identical struct layout, identical `malloc` arena, identical semantics.
#[test]
fn cross_library_matrix_interop() {
    let b = load_both();
    let mut rng = Rng::new(0xD00D_0001);
    for _ in 0..100 {
        let w = rng.range(1, 6) as c_int;
        let h = rng.range(1, 6) as c_int;
        let vals: Vec<c_int> = (0..(w * h)).map(|_| rng.safe_value()).collect();

        // Build with C, stringify with Rust; build with Rust, stringify with C.
        let via_c_then_rs = unsafe {
            let m = make_matrix(&b.c, w, h, &vals);
            let p = (b.rs.matrix_to_string)(m);
            let bytes = cstr_bytes(p);
            libc_free(p as *mut c_void);
            // Freed by the *other* implementation on purpose.
            (b.rs.free_matrix)(m);
            bytes
        };
        let via_rs_then_c = unsafe {
            let m = make_matrix(&b.rs, w, h, &vals);
            let p = (b.c.matrix_to_string)(m);
            let bytes = cstr_bytes(p);
            libc_free(p as *mut c_void);
            (b.c.free_matrix)(m);
            bytes
        };
        assert_eq!(via_c_then_rs, via_rs_then_c, "cross-library stringify mismatch");
    }
}

/// A mixed pipeline: parse with one implementation, multiply with the other.
#[test]
fn cross_library_mixed_pipeline() {
    let b = load_both();
    let mut rng = Rng::new(0xD00D_0002);
    for _ in 0..100 {
        let ha = rng.range(1, 5) as usize;
        let wa = rng.range(1, 5) as usize;
        let wb = rng.range(1, 5) as usize;
        let va: Vec<c_int> = (0..ha * wa).map(|_| rng.range(-300, 300) as c_int).collect();
        let vb: Vec<c_int> = (0..wa * wb).map(|_| rng.range(-300, 300) as c_int).collect();
        let ta = cs(&render_matrix_text(wa, ha, &va));
        let tb = cs(&render_matrix_text(wb, wa, &vb));

        let mix = |parse: &Api, mul: &Api, stringify: &Api| unsafe {
            let ma = (parse.initialize_matrix_from_string)(ta.as_ptr(), wa as c_int, ha as c_int);
            let mb = (parse.initialize_matrix_from_string)(tb.as_ptr(), wb as c_int, wa as c_int);
            assert!(!ma.is_null() && !mb.is_null());
            let res = (mul.multiply_matrices)(ma, mb);
            assert!(!res.is_null());
            let p = (stringify.matrix_to_string)(res);
            let bytes = cstr_bytes(p);
            libc_free(p as *mut c_void);
            (mul.free_matrix)(res);
            (parse.free_matrix)(ma);
            (parse.free_matrix)(mb);
            bytes
        };
        let all_c = mix(&b.c, &b.c, &b.c);
        let all_rs = mix(&b.rs, &b.rs, &b.rs);
        let mixed1 = mix(&b.c, &b.rs, &b.c);
        let mixed2 = mix(&b.rs, &b.c, &b.rs);
        assert_eq!(all_c, all_rs, "pure pipelines diverged");
        assert_eq!(all_c, mixed1, "C-parse / Rust-multiply diverged");
        assert_eq!(all_c, mixed2, "Rust-parse / C-multiply diverged");
    }
}

/// `matrix_t` must have the same size and field offsets in both worlds; if it
/// did not, the interop tests above would already be reading garbage. This
/// pins the expected layout explicitly.
#[test]
fn matrix_t_layout() {
    assert_eq!(std::mem::size_of::<MatrixT>(), 16);
    assert_eq!(std::mem::align_of::<MatrixT>(), 8);
    let b = load_both();
    unsafe {
        // Write width/height through one library's allocation and read them
        // back through the other's accessor path (free_matrix walks `height`).
        let m = (b.c.allocate_matrix)(3, 4);
        assert_eq!((*m).width, 3);
        assert_eq!((*m).height, 4);
        (b.rs.free_matrix)(m);
        let m = (b.rs.allocate_matrix)(5, 6);
        assert_eq!((*m).width, 5);
        assert_eq!((*m).height, 6);
        (b.c.free_matrix)(m);
    }
}
