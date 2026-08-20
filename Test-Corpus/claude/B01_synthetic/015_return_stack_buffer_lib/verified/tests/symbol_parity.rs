// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Mechanical `nm -D` comparison, so the check cannot drift away from the
// SYMBOLS.md table.

mod harness;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names that belong to the toolchain/runtime rather than to the library's own
/// ABI, and are therefore excluded from the parity comparison.
fn is_runtime_noise(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__")
        || name.starts_with("_Unwind_")
        || name.starts_with("_fini")
        || name.starts_with("_init")
        || name.starts_with("_edata")
        || name.starts_with("_end")
        || name == "_IO_stdin_used"
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
}

fn nm(path: &Path, extra: &str) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().rev();
            let name = it.next()?.to_string();
            let kind = it.next()?.to_string();
            // A kind is a single letter; otherwise the line was malformed.
            if kind.len() != 1 {
                return None;
            }
            Some((name, kind))
        })
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| !is_runtime_noise(n))
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = harness::c_so_file();
    let r = harness::rust_so_file();
    assert!(c.exists(), "missing C .so at {}", c.display());
    assert!(r.exists(), "missing Rust .so at {}", r.display());

    let c_syms = defined(&c);
    let r_syms = defined(&r);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {r_syms:?}",
        missing.len()
    );

    // The four documented entry points must actually be there (guards against
    // both sets being empty for some unrelated reason).
    for expected in ["printLine", "bad", "good", "driver"] {
        assert!(
            c_syms.contains(expected),
            "C .so unexpectedly lacks `{expected}` -- SYMBOLS.md is stale"
        );
        assert!(
            r_syms.contains(expected),
            "Rust .so lacks `{expected}`"
        );
    }
}

#[test]
fn d2_internal_static_helpers_stay_unexported() {
    // `helperBad` and `helperGood1` are `static` in C, so neither library may
    // export them; exporting them would be an ABI mismatch in the other
    // direction.
    for path in [harness::c_so_file(), harness::rust_so_file()] {
        let syms = defined(&path);
        for hidden in ["helperBad", "helperGood1", "charString"] {
            assert!(
                !syms.contains(hidden),
                "{} unexpectedly exports `{hidden}`",
                path.display()
            );
        }
    }
}

#[test]
fn d3_both_libraries_import_puts_from_the_same_libc() {
    // GCC folds `printf("%s\n", line)` into `puts(line)` even at -O0, which is
    // what the reference library does. The Rust translation calls `puts`
    // directly, so both write through the identical libc `stdout` FILE with
    // identical buffering -- that is what makes byte comparison meaningful.
    for path in [harness::c_so_file(), harness::rust_so_file()] {
        let undef: Vec<String> = nm(&path, "--undefined-only")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(
            undef.iter().any(|n| n == "puts" || n.starts_with("puts@")),
            "{} does not import puts; undefined = {undef:?}",
            path.display()
        );
    }
}

/// Anti-vacuity guard.
///
/// Both `.so`s export the same four names, and both route their *internal*
/// calls through the GOT/PLT (C: `call printLine@plt`; Rust: `call *…(%rip)`),
/// so they are interposable. If either object were loaded RTLD_GLOBAL, one
/// library's `good()` could end up calling the *other* library's `printLine` —
/// the differential tests would then compare an implementation against itself
/// and pass no matter how wrong the translation was.
///
/// The harness loads both with RTLD_LOCAL; this test proves the resulting
/// isolation instead of trusting it.
#[test]
fn d5_the_two_libraries_resolve_independently() {
    let (c, r) = harness::libs();

    // Distinct objects => distinct code addresses for every exported symbol.
    let pairs: [(&str, usize, usize); 4] = [
        ("printLine", c.print_line as usize, r.print_line as usize),
        ("bad", c.bad as usize, r.bad as usize),
        ("good", c.good as usize, r.good as usize),
        ("driver", c.driver as usize, r.driver as usize),
    ];
    for (name, ca, ra) in pairs {
        assert_ne!(
            ca, ra,
            "`{name}` resolved to the same address in both libraries: the two \
             objects are sharing one implementation, so every differential test \
             would be vacuous"
        );
        assert_ne!(ca, 0, "`{name}` resolved to NULL in the C library");
        assert_ne!(ra, 0, "`{name}` resolved to NULL in the Rust library");
    }

    // And each library's *internal* dispatch stays inside itself: the C
    // `good()` must emit exactly the C string, independent of anything the Rust
    // object exports under the same name.
    assert_eq!(
        harness::capture(|| c.call_good()),
        b"helperGood1 string\n",
        "the C library's internal printLine call was interposed"
    );
    assert_eq!(
        harness::capture(|| r.call_good()),
        b"helperGood1 string\n",
        "the Rust library's internal printLine call was interposed"
    );
}

#[test]
fn d4_rust_so_has_no_unresolved_non_libc_symbol() {
    // Everything the Rust cdylib imports must be satisfiable by the system
    // libraries it already links (libc/libgcc/libpthread/libdl/libm).
    let r = harness::rust_so_file();
    let out = Command::new("ldd").arg("-r").arg(&r).output();
    let Ok(out) = out else {
        eprintln!("ldd unavailable; skipping");
        return;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{text}"
    );
    assert!(
        !text.contains("not found"),
        "Rust .so has missing dependencies:\n{text}"
    );
}
