//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that the Rust cdylib exports every symbol
//! the C shared library exports, under the exact same name, and that it pulls in
//! no non-libc undefined symbol the C library does not.

mod harness;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &[&str]) -> BTreeSet<String> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    cmd.args(extra);
    cmd.arg(path);
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        // Drop the `@GLIBC_2.2.5` / `@@GLIBC_2.2.5` version suffix so names
        // compare as plain symbol names.
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Symbols that are toolchain/runtime glue rather than library API.
fn is_glue(s: &str) -> bool {
    const PREFIXES: [&str; 8] = [
        "_ITM_",
        "__gmon_start__",
        "__cxa_",
        "_Unwind_",
        "__tls_get_addr",
        "__stack_chk_",
        "_init",
        "_fini",
    ];
    PREFIXES.iter().any(|p| s.starts_with(p))
}

/// The set of libraries a shared object declares as `NEEDED`, plus the
/// always-present `ld-linux`, is what its undefined symbols may resolve against.
/// Rather than hard-coding a list of libc symbol names (which are versioned and
/// vary between glibc releases), ask the dynamic loader itself.
fn unresolved_symbols(path: &Path) -> Vec<String> {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running `ldd -r` on {}: {e}", path.display()));
    // `ldd -r` prints "undefined symbol: NAME  (path)" on stdout/stderr for each
    // symbol that the loader cannot bind.
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text.lines()
        .filter_map(|l| l.split("undefined symbol:").nth(1))
        .map(|rest| {
            rest.trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn exported_symbols_match_exactly() {
    let libs = harness::libs();
    let c = nm(&libs.c_path, &["--defined-only"]);
    let rust = nm(&libs.rust_path, &["--defined-only"]);

    let c_api: BTreeSet<_> = c.iter().filter(|s| !is_glue(s)).cloned().collect();
    let rust_api: BTreeSet<_> = rust.iter().filter(|s| !is_glue(s)).cloned().collect();

    let missing: Vec<_> = c_api.difference(&rust_api).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_api:?}\n\
         Rust exports: {rust_api:?}",
        missing.len()
    );

    // Sanity: the one documented API symbol really is there in both.
    assert!(c_api.contains("slice"), "C .so must export `slice`");
    assert!(rust_api.contains("slice"), "Rust .so must export `slice`");
}

fn no_unresolved_non_libc_symbols_in_rust_so() {
    let libs = harness::libs();

    // Baseline: the C library itself must be fully resolvable.
    let c_bad = unresolved_symbols(&libs.c_path);
    assert!(
        c_bad.is_empty(),
        "C .so has unresolved symbol(s) (unexpected baseline): {c_bad:?}"
    );

    // The Rust cdylib must be equally self-sufficient: every symbol it imports
    // has to bind against its NEEDED libraries (libc, ld-linux, ...).
    let rust_bad = unresolved_symbols(&libs.rust_path);
    assert!(
        rust_bad.is_empty(),
        "Rust .so has unresolved non-libc symbol(s): {rust_bad:?}"
    );

    // Cross-check with nm: the C library's own imports (`printf`, `strlen`) must
    // be present in the Rust library's import list too, proving the translation
    // routes output through the platform C library rather than reimplementing it.
    let rust_undef = nm(&libs.rust_path, &["--undefined-only"]);
    for required in ["printf", "strlen"] {
        assert!(
            rust_undef.contains(required),
            "Rust .so must import `{required}` from libc (imports: {rust_undef:?})"
        );
    }
}

fn no_stub_or_panic_markers_in_rust_so() {
    // A stub that lies about behaviour is worse than a missing symbol: make sure
    // the cdylib contains no `unimplemented!` / `todo!` marker strings.
    let libs = harness::libs();
    let bytes = std::fs::read(&libs.rust_path).expect("read Rust .so");
    for marker in [
        &b"not implemented"[..],
        b"not yet implemented",
        b"unimplemented",
    ] {
        assert!(
            bytes
                .windows(marker.len())
                .all(|w| w != marker),
            "Rust .so contains the stub marker {:?}",
            String::from_utf8_lossy(marker)
        );
    }
}

fn both_libraries_are_loadable_and_callable() {
    // Smoke test that the dlopen + dlsym path used by every other test works.
    let _c = harness::c_slice();
    let _r = harness::rust_slice();
}

// ---------------------------------------------------------------------------
// Sequential runner (`harness = false`): fd 1 is redirected around every
// library call, so the cases must not run concurrently.
// ---------------------------------------------------------------------------

fn main() {
    harness::run_all(
        "phase_d_symbols",
        &[
            ("exported_symbols_match_exactly", exported_symbols_match_exactly as fn()),
            ("no_unresolved_non_libc_symbols_in_rust_so", no_unresolved_non_libc_symbols_in_rust_so as fn()),
            ("no_stub_or_panic_markers_in_rust_so", no_stub_or_panic_markers_in_rust_so as fn()),
            ("both_libraries_are_loadable_and_callable", both_libraries_are_loadable_and_callable as fn()),
        ],
    );
}
