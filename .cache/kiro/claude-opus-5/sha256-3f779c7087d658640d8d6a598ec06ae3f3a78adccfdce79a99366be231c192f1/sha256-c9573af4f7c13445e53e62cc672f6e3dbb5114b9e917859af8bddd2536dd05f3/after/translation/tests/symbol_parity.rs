// Phase D — symbol parity enforced as a test, so it cannot silently regress.
//
// Runs `nm -D --defined-only` on both shared objects and requires the set of
// symbols the C `.so` exports to be a subset of what the Rust `.so` exports.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn defined_symbols(so: &PathBuf) -> Vec<String> {
    assert!(so.exists(), "shared object missing: {}", so.display());
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm (binutils required)");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn c_so() -> PathBuf {
    std::env::var("DRIVER_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libdriver.so"))
}

fn rust_so() -> PathBuf {
    // Deterministic: `cargo test` builds the dev-profile cdylib, so that is the
    // default. Set DRIVER_RUST_SO to test the release cdylib (see
    // scripts/verify_all.sh, which runs the suite against BOTH profiles).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/debug/libdriver.so")
}

/// The symbol diff MUST be empty: every symbol exported by the C `.so` must be
/// exported by the Rust `.so` under the exact same name.
#[test]
fn sym_01_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {:?}\n  C   ({}): {:?}\n  Rust ({}): {:?}",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );
}

/// Guard against the artifact drifting from reality: the five documented
/// symbols in SYMBOLS.md are exactly what the C `.so` exports.
#[test]
fn sym_02_c_surface_matches_documented_surface() {
    let c = defined_symbols(&c_so());
    let mut want = vec!["bad", "driver", "good", "printIntLine", "printLine"];
    want.sort();
    assert_eq!(
        c, want,
        "the C export surface changed; SYMBOLS.md / ERRORS.md / CONFIGS.md must \
         be re-derived"
    );
}

/// The Rust `.so` must import nothing outside the platform C runtime, i.e.
/// there must be no unresolved project-level symbol.
#[test]
fn sym_03_rust_so_has_no_non_libc_undefined_symbols() {
    let so = rust_so();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let offenders: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            // Everything legitimately imported is glibc, the libgcc unwinder,
            // or a weak ELF/TM hook.
            !(base.starts_with("_Unwind_")
                || base.starts_with("_ITM_")
                || base.starts_with("__cxa_")
                || base.starts_with("__gmon_")
                || base.starts_with("__tls_get_addr")
                || base.starts_with("__errno_location")
                || base.starts_with("pthread_")
                || s.contains("@GLIBC"))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "Rust .so has non-libc undefined symbols: {offenders:?}"
    );
}
