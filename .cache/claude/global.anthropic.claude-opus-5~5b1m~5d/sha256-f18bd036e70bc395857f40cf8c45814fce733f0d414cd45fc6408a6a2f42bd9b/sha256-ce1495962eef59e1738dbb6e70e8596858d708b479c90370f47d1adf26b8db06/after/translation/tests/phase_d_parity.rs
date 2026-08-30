// Phase D -- symbol parity, and the observability limits of this API.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn exported_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Exported code/data only; skip the CRT/compiler bookkeeping that
            // is not part of either library's API.
            if matches!(kind, "T" | "t" | "D" | "B" | "R")
                && !name.starts_with("_init")
                && !name.starts_with("_fini")
                && !name.starts_with("__")
                && !name.starts_with("_ITM_")
                && !name.starts_with("_Unwind")
            {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn c_so() -> PathBuf {
    std::env::var("C_DYLIB").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
    })
}

fn rust_so() -> PathBuf {
    // Force the cdylib to exist / be fresh via the harness, then use its path.
    let _ = rust_api();
    std::env::var("RUST_DYLIB").map(PathBuf::from).unwrap_or_else(|_| {
        let exe = std::env::current_exe().unwrap();
        exe.parent().unwrap().parent().unwrap().join("libdriver.so")
    })
}

/// SYMBOLS.md gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name. The diff must be EMPTY.
#[test]
fn parity_every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so -- is it built?"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C   exports: {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c,
        r
    );

    // The five documented entry points must all be there.
    for want in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert!(c.contains(want), "C .so lost {want}?");
        assert!(r.contains(want), "Rust .so does not export {want}");
    }
}

/// The Rust `.so` must not leave non-libc symbols undefined, or it would fail
/// to load for a real consumer. `dlopen` with RTLD_NOW proves full resolution.
#[test]
fn parity_rust_so_has_no_unresolved_symbols() {
    let out = Command::new("nm").arg("-D").arg("-u").arg(rust_so()).output().expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| {
            // Everything legitimately imported here comes from glibc or libgcc.
            !n.contains("@GLIBC")
                && !n.contains("@GCC")
                && !n.starts_with("_ITM_")
                && !n.starts_with("__gmon_start__")
                && !n.starts_with("_Unwind")
                && !n.starts_with("__cxa")
                && !n.starts_with("statx")
                && !n.starts_with("gettid")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has non-libc undefined symbols: {suspicious:?}"
    );

    // And it really does load and resolve eagerly.
    let _ = rust_api();
}

/// OBSERVABILITY NOTE (justifies the "equivalent mutants" in MUTATION.md):
/// in the C, `good()` and `bad()` emit byte-identical output, so `driver`'s
/// branch cannot be distinguished through the public API. Pin that fact for
/// BOTH libraries so a future change that makes them differ is caught here.
#[test]
fn parity_good_and_bad_are_observationally_identical() {
    for api in [c_api(), rust_api()] {
        let g = capture(|| api.good());
        let b = capture(|| api.bad());
        assert_eq!(
            g, b,
            "[{}] good() and bad() are expected to emit identical bytes",
            api.which
        );
        assert_eq!(g, b"0\n", "[{}] expected \"0\\n\"", api.which);
    }

    // Cross-library too.
    assert_eq!(capture(|| c_api().good()), capture(|| rust_api().good()));
    assert_eq!(capture(|| c_api().bad()), capture(|| rust_api().bad()));
    assert_eq!(capture(|| c_api().good()), capture(|| rust_api().bad()));
}

/// Sanity: the capture harness is not vacuous. If this ever returns empty
/// output, every other comparison in the suite would trivially "pass".
#[test]
fn parity_capture_harness_is_not_vacuous() {
    let c = capture(|| c_api().print_int_line(1234567));
    assert_eq!(c, b"1234567\n", "C capture is broken");
    let r = capture(|| rust_api().print_int_line(1234567));
    assert_eq!(r, b"1234567\n", "Rust capture is broken");
    // Two different inputs must give different captures.
    assert_ne!(
        capture(|| rust_api().print_int_line(1)),
        capture(|| rust_api().print_int_line(2)),
        "capture does not distinguish different inputs"
    );
}
