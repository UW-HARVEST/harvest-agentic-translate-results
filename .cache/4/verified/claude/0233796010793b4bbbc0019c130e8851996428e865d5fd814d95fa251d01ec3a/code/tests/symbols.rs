//! Phase D — symbol-parity gate, executed as a test so it cannot rot.
//!
//! `nm -D` on both artifacts.  Every symbol the C artifact exports (i.e. defines
//! in `.dynsym`) must also be exported by the Rust artifact, and every
//! behaviour-defining import must be shared.  Documented exceptions are listed
//! explicitly below and justified in SYMBOLS.md.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
struct Syms {
    /// symbols DEFINED in .dynsym (exported)
    defined: BTreeSet<String>,
    /// strong undefined symbols (imports)
    imported: BTreeSet<String>,
    /// weak undefined symbols (optional CRT hooks)
    weak: BTreeSet<String>,
}

fn nm_dynamic(p: &Path) -> Syms {
    let out = Command::new("nm").arg("-D").arg(p).output().expect("nm -D failed to run");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        p.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = Syms::default();
    for line in text.lines() {
        // "                 U name@VER"  |  "0000000000404040 B name@VER"
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
            (Some(_addr), Some(k), Some(n)) => (k.to_string(), n.to_string()),
            _ => continue,
        };
        match kind.as_str() {
            "U" => {
                s.imported.insert(name);
            }
            "w" | "v" => {
                s.weak.insert(name);
            }
            _ => {
                s.defined.insert(name);
            }
        }
    }
    s
}

/// Symbols the C artifact has in `.dynsym` that the Rust artifact legitimately
/// does not need.  Each entry is justified in SYMBOLS.md.
const DOCUMENTED_EXCEPTIONS: &[(&str, &str)] = &[
    (
        "stderr@GLIBC_2.2.5",
        "glibc's own FILE* data object, pulled in as a COPY RELOCATION because the C \
         binary is linked non-PIE; it is not an API the program provides",
    ),
    (
        "printf@GLIBC_2.2.5",
        "output formatting only; the Rust binary emits the identical bytes via \
         std::io::stdout (asserted byte-for-byte by configs.rs/errors.rs)",
    ),
    (
        "fprintf@GLIBC_2.2.5",
        "output formatting only; see printf",
    ),
];

/// The imports that DEFINE this program's numeric behaviour.  These must be
/// shared, otherwise the Rust side reimplemented them and could drift.
const MUST_SHARE_IMPORTS: &[&str] = &[
    "strtod@GLIBC_2.2.5",
    "pow@GLIBC_2.29",
    "__errno_location@GLIBC_2.2.5",
];

#[test]
fn d1_every_c_exported_symbol_is_exported_by_rust() {
    let c = nm_dynamic(&c_bin());
    let r = nm_dynamic(&rust_bin());
    let excepted: BTreeSet<&str> = DOCUMENTED_EXCEPTIONS.iter().map(|(s, _)| *s).collect();

    let missing: Vec<&String> = c
        .defined
        .iter()
        .filter(|s| !r.defined.contains(*s) && !excepted.contains(s.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C artifact but missing from the Rust artifact: {missing:?}\n\
         (add the #[no_mangle] wrapper, or translate the missing C source -- never stub)"
    );

    // Weak CRT hooks must line up as well.
    let weak_missing: Vec<&String> = c.weak.iter().filter(|s| !r.weak.contains(*s)).collect();
    assert!(weak_missing.is_empty(), "weak symbols missing from Rust: {weak_missing:?}");
}

#[test]
fn d2_behaviour_defining_imports_are_shared() {
    let c = nm_dynamic(&c_bin());
    let r = nm_dynamic(&rust_bin());
    for want in MUST_SHARE_IMPORTS {
        assert!(
            c.imported.contains(*want),
            "expected the C artifact to import {want}; C imports: {:?}",
            c.imported
        );
        assert!(
            r.imported.contains(*want),
            "the Rust artifact does NOT import {want} -- it must delegate to the same \
             libc/libm routine instead of reimplementing it. Rust imports: {:?}",
            r.imported
        );
    }
}

#[test]
fn d3_rust_has_no_unresolved_non_libc_symbols() {
    let r = nm_dynamic(&rust_bin());
    let unresolved: Vec<&String> = r
        .imported
        .iter()
        .filter(|s| !s.contains("@GLIBC_") && !s.contains("@GCC_"))
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust artifact has undefined non-libc/libgcc symbols: {unresolved:?}"
    );

    // and the loader really can satisfy everything
    let out = Command::new("ldd").arg(rust_bin()).output().expect("ldd failed");
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(!txt.contains("not found"), "ldd reports unresolved libraries:\n{txt}");
}

#[test]
fn d4_documented_exceptions_are_actually_only_formatting_and_data() {
    // Guard against the exception list being used to hide a real gap: the only
    // permitted exceptions are the three below, and each must genuinely still be
    // present in the C artifact (otherwise the list is stale).
    let c = nm_dynamic(&c_bin());
    assert_eq!(DOCUMENTED_EXCEPTIONS.len(), 3);
    for (s, _why) in DOCUMENTED_EXCEPTIONS {
        assert!(
            c.defined.contains(*s) || c.imported.contains(*s),
            "stale exception entry {s}: the C artifact no longer references it"
        );
    }
}
