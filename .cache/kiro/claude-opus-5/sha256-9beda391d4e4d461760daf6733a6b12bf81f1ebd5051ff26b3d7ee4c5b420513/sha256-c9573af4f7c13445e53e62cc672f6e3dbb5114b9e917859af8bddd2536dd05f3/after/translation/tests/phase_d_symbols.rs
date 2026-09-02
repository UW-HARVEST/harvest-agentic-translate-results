//! Phase D — symbol parity gate, enforced from inside the test suite.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("`nm` must be available");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/// The C `.so`'s exported symbol set must be a subset of the Rust `.so`'s.
#[test]
fn d1_symbol_parity() {
    let c = defined_dynamic_symbols(&common::c_lib_file());
    assert!(
        c.contains("siphash") && c.contains("stbds_hash_bytes"),
        "C .so unexpectedly missing known exports: {c:?}"
    );
    for rp in common::rust_lib_files() {
        let r = defined_dynamic_symbols(&rp);
        let missing: Vec<&String> = c.difference(&r).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {:?} is missing C-exported symbols: {:?}",
            rp.file_name().unwrap(),
            missing
        );
    }
}

/// `stbds_siphash_bytes` is `static` in the C source, so neither library may
/// export it.
#[test]
fn d2_static_helper_not_exported() {
    let c = defined_dynamic_symbols(&common::c_lib_file());
    assert!(
        !c.contains("stbds_siphash_bytes"),
        "C .so unexpectedly exports the static helper"
    );
    for rp in common::rust_lib_files() {
        let r = defined_dynamic_symbols(&rp);
        assert!(
            !r.contains("stbds_siphash_bytes"),
            "Rust .so {:?} must not export the static helper",
            rp.file_name().unwrap()
        );
    }
}

/// The Rust `.so` must not import any non-libc/non-runtime symbol.
#[test]
fn d3_no_unexpected_undefined_symbols() {
    for rp in common::rust_lib_files() {
        let out = Command::new("nm")
            .args(["-D", "-u", rp.to_str().unwrap()])
            .output()
            .expect("nm");
        let text = String::from_utf8_lossy(&out.stdout);
        let bad: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .filter(|s| s.contains("stbds") || s.contains("siphash"))
            .collect();
        assert!(
            bad.is_empty(),
            "Rust .so {:?} has undefined library symbols: {:?}",
            rp.file_name().unwrap(),
            bad
        );
    }
}

/// No stubbed-out implementations in the translation.
#[test]
fn d4_no_stubs_in_source() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("src/lib.rs");
    for needle in ["unimplemented!", "todo!", "unreachable!("] {
        assert!(
            !src.contains(needle),
            "src/lib.rs contains a stub marker: {needle}"
        );
    }
}

/// Both Rust build profiles must be present and under test, since
/// `[profile.release] panic = "abort"` plus debug overflow-checks make them
/// materially different builds of the same source.
#[test]
fn d5_both_profiles_under_test() {
    let files = common::rust_lib_files();
    let names: Vec<String> = files
        .iter()
        .map(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string()
        })
        .collect();
    assert!(
        names.contains(&"debug".to_string()) && names.contains(&"release".to_string()),
        "expected both debug and release cdylibs under test, found {names:?}; \
         run `cargo build && cargo build --release`"
    );
    assert_eq!(common::pairs().len(), files.len());
}

/// Documents the one surviving mutant from the mutation-testing sweep: in
/// `data |= (size_t)(...) << 16 << 16`, the C sign-extends the `int` to `size_t`
/// and *then* shifts left by 32, which discards every bit the sign extension
/// set. So sign-extending or not is bit-identical here, and a translation that
/// omits it is not a divergence. Proven exhaustively over the boundary values
/// and randomly elsewhere.
#[test]
fn d6_high_half_sign_extension_is_a_no_op() {
    fn with_sext(hi: u32) -> u64 {
        ((((hi as i32) as i64) as u64) << 16) << 16
    }
    fn without_sext(hi: u32) -> u64 {
        ((hi as u64) << 16) << 16
    }
    for hi in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0x8000_0001] {
        assert_eq!(with_sext(hi), without_sext(hi), "hi={hi:#010x}");
    }
    let mut x = 0x1234_5678u32;
    for _ in 0..200_000 {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        assert_eq!(with_sext(x), without_sext(x), "hi={x:#010x}");
    }
}
