//! Phase D -- symbol parity and harness-integrity guards.
//!
//! These tests protect the conclusions of phases B and C:
//!
//!   * `symbol_parity_*` re-derive the `nm -D` comparison from the actual
//!     `.so` files, so SYMBOLS.md can never drift from reality.
//!   * `no_symbol_interposition` proves that loading the C and the Rust library
//!     into one process does not let one library's exported
//!     `spectral_contrast` satisfy the *other* library's internal call to it.
//!     Without this, a wrong Rust `spectral_contrast` could be masked (the C
//!     `match` would silently call the Rust one, or vice versa) and every
//!     `match` test would be vacuous.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .join("libunderhanded_c_nuke_lib.so")
}

/// `nm -D --defined-only <so>` -> the set of exported symbol names with types.
fn exported(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, ty, name) = (it.next()?, it.next()?, it.next()?);
            Some(format!("{ty} {name}"))
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_parity_c_exports_are_all_present_in_rust() {
    // Touch the loader first so the C .so is built if needed.
    let _ = both();
    let c = exported(&c_so());
    let rs = exported(&rust_so());
    assert!(!c.is_empty(), "nm found no exported symbols in the C .so");
    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C  = {c:?}\nRS = {rs:?}"
    );
}

#[test]
fn symbol_parity_is_exact_in_both_directions() {
    let _ = both();
    let c = exported(&c_so());
    let rs = exported(&rust_so());
    assert_eq!(
        c, rs,
        "the two libraries must export exactly the same symbol set\n\
         C  = {c:?}\nRS = {rs:?}"
    );
    // The C statics (`total`, `smoothen`, `differentiate`, `preprocess`,
    // `dot_product`, `normalize`) have internal linkage; neither library may
    // leak them.
    for hidden in
        ["total", "smoothen", "differentiate", "preprocess", "dot_product", "normalize"]
    {
        for (lbl, set) in [("C", &c), ("Rust", &rs)] {
            assert!(
                !set.iter().any(|s| s.ends_with(&format!(" {hidden}"))),
                "{lbl} .so must not export the static function `{hidden}`"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Interposition guard.
// ---------------------------------------------------------------------------

/// Child half: loads exactly ONE library and prints the batch digest.
#[test]
fn isolated_digest_child() {
    let Ok(which) = std::env::var("ISOLATE_IMPL") else { return };
    let imp = load_single(&which);
    println!("ISOLATED_DIGEST={:016X}", batch_digest(&imp));
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

fn isolated_digest(which: &str) -> u64 {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["isolated_digest_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("ISOLATE_IMPL", which)
        .output()
        .expect("spawn isolated digest child");
    let text = String::from_utf8_lossy(&out.stdout);
    let i = text
        .find("ISOLATED_DIGEST=")
        .unwrap_or_else(|| panic!("child produced no digest for {which}:\n{text}"));
    let hex = text[i + "ISOLATED_DIGEST=".len()..]
        .split(|c: char| c.is_whitespace())
        .next()
        .expect("digest token");
    u64::from_str_radix(hex, 16).expect("parse digest")
}

#[test]
fn no_symbol_interposition_between_loaded_libraries() {
    let both = both();

    // Same batch, run with BOTH libraries resident in this process.
    let c_together = batch_digest(&both.c);
    let rs_together = batch_digest(&both.rs);

    // ... and with only one library resident, in a fresh process.
    let c_alone = isolated_digest("c");
    let rs_alone = isolated_digest("rs");

    assert_eq!(
        c_together, c_alone,
        "the C library behaves differently when the Rust library is also loaded \
         ({c_together:016X} vs {c_alone:016X}) -- its internal call to \
         `spectral_contrast` is being interposed, which would make every `match` \
         differential test vacuous"
    );
    assert_eq!(
        rs_together, rs_alone,
        "the Rust library behaves differently when the C library is also loaded \
         ({rs_together:016X} vs {rs_alone:016X}) -- symbol interposition is \
         masking the real behaviour"
    );

    // And, of course, the two must agree with each other.
    assert_eq!(
        c_together, rs_together,
        "C and Rust digests differ: {c_together:016X} vs {rs_together:016X}"
    );
}

/// `match` must be reached through the exported symbol, and `spectral_contrast`
/// must be a *separately* callable public entry point that mutates its inputs.
/// This pins the underhanded `float_t` reinterpretation: a caller that follows
/// `match.h` passes `double*`, and the callee reads `float`s -- so exactly
/// `length * 4` bytes of an 8-byte-per-element buffer are rewritten.
#[test]
fn spectral_contrast_reinterprets_double_buffers_as_float() {
    let both = both();
    for len in [1usize, 2, 3, 4, 8, 17] {
        let src: Vec<f64> = (0..len).map(|i| 1.0 + i as f64).collect();
        let mut outs = Vec::new();
        for imp in [&both.c, &both.rs] {
            let mut a = src.clone();
            let mut b = src.clone();
            // A `match.h`-following caller hands over `double*`.
            let ret = unsafe {
                (imp.spectral_contrast)(
                    a.as_mut_ptr() as *mut f32,
                    b.as_mut_ptr() as *mut f32,
                    len as i32,
                )
            };
            outs.push((
                ret.to_bits(),
                a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                b.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            ));
        }
        assert_eq!(outs[0], outs[1], "double-buffer reinterpretation diverges at len={len}");

        // Bytes at and beyond `len * 4` must be untouched: the callee only ever
        // writes `len` floats even though the buffer holds `len` doubles.
        let after = &outs[0].1;
        let tail_start_byte = len * 4;
        for (i, &bits) in after.iter().enumerate() {
            let elem_start = i * 8;
            if elem_start >= tail_start_byte {
                assert_eq!(
                    bits,
                    src[i].to_bits(),
                    "element {i} (byte {elem_start}) should be untouched at len={len}"
                );
            }
        }
    }
}
