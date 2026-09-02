//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` over both objects and requires the exported-symbol sets to be
//! equal, and requires the Rust object to have no undefined symbol that is not
//! satisfied by the platform runtime. Kept as a `#[test]` so a regression shows
//! up in `cargo test` rather than only in `check_symbols.sh`.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Runs `nm` with `args` on `so` and returns the (type, name) pairs.
fn nm(so: &Path, args: &[&str]) -> Vec<(char, String)> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "0000000000001164 T bad"  or  "                 U printf@GLIBC_2.2.5"
            let mut parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }
            let name = parts.pop()?.to_string();
            let kind = parts.pop()?.chars().next()?;
            Some((kind, name))
        })
        .collect()
}

fn exported(so: &Path) -> BTreeSet<String> {
    nm(so, &["-D", "--defined-only"])
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}

fn exported_symbol_sets_are_identical() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert_eq!(
        c,
        BTreeSet::from([
            "bad".to_string(),
            "driver".to_string(),
            "good".to_string(),
            "printIntPtrLine".to_string(),
        ]),
        "the C library's export set changed; SYMBOLS.md needs updating"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         Add the #[no_mangle] wrapper, or translate the C source if a whole \
         module was skipped. Never stub."
    );

    // Not required by the gate, but this library has no reason to export extras,
    // and an unexpected extra would mean the cdylib is leaking Rust internals.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

fn every_exported_symbol_is_a_global_text_symbol() {
    // All four are functions in .text with external linkage in the C, so both
    // objects must report them as `T`.
    for so in [c_so_path(), rust_so_path()] {
        let syms: Vec<(char, String)> = nm(&so, &["-D", "--defined-only"]);
        for name in ["bad", "driver", "good", "printIntPtrLine"] {
            let found = syms
                .iter()
                .find(|(_, n)| n == name)
                .unwrap_or_else(|| panic!("{name} not found in {}", so.display()));
            assert_eq!(
                found.0,
                'T',
                "{name} in {} should be a global text symbol, got {:?}",
                so.display(),
                found.0
            );
        }
    }
}

fn rust_so_has_no_unresolved_symbols() {
    // Every undefined symbol in the Rust object must be satisfied at load time.
    // `ldd -r` reports both missing objects and unresolved symbols, so an empty
    // "undefined symbol" section is the check.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so_path())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}

fn both_objects_import_printf_and_nothing_exotic() {
    // The C library's only real import is printf. The Rust library additionally
    // pulls in glibc allocator/IO/TLS entry points and libgcc's unwinder via
    // libstd; all of those live in libc.so.6, libgcc_s.so.1 or the dynamic
    // loader, which are recorded as NEEDED. Assert there is nothing outside that
    // set, which is what "0 missing/undefined non-libc symbols" means here.
    let allowed_libs = ["libc.so.6", "libgcc_s.so.1", "ld-linux-x86-64.so.2"];

    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("readelf")
            .args(["-d"])
            .arg(&so)
            .output()
            .expect("run readelf");
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().filter(|l| l.contains("(NEEDED)")) {
            let name = line
                .rsplit_once('[')
                .and_then(|(_, t)| t.strip_suffix(']'))
                .unwrap_or("");
            assert!(
                allowed_libs.contains(&name),
                "{} depends on an unexpected library: {name}",
                so.display()
            );
        }
    }

    // printf must be imported by both, since both delegate formatting to libc.
    for so in [c_so_path(), rust_so_path()] {
        let undef = nm(&so, &["-D", "-u"]);
        assert!(
            undef.iter().any(|(_, n)| n.starts_with("printf")),
            "{} does not import printf",
            so.display()
        );
    }
}

fn lazy_binding_matches_the_c_library() {
    // Observable through bad(): on a lazily bound object the first call through a
    // PLT slot runs _dl_runtime_resolve, whose stack usage lands in the slot
    // bad() reads. The C .so as built by c_src/CMakeLists.txt is lazily bound, so
    // the Rust cdylib must be too (see translation/.cargo/config.toml).
    let flags = |so: &Path| -> String {
        let out = Command::new("readelf")
            .args(["-d"])
            .arg(so)
            .output()
            .expect("run readelf");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains("(FLAGS"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let c = flags(&c_so_path());
    let r = flags(&rust_so_path());
    assert!(
        !c.contains("BIND_NOW"),
        "the C library is unexpectedly BIND_NOW; SYMBOLS.md needs updating:\n{c}"
    );
    assert!(
        !r.contains("BIND_NOW") && !r.contains("NOW"),
        "the Rust cdylib is eagerly bound but the C library is lazily bound.\n\
         Ensure translation/.cargo/config.toml passes -Wl,-z,lazy.\nGot:\n{r}"
    );
}

// ---------------------------------------------------------------------------
// Entry point (`harness = false`; see the comment in Cargo.toml)
// ---------------------------------------------------------------------------
fn main() -> ! {
    common::run_tests(driver_tests())
}

fn driver_tests() -> &'static [common::Test] {
    tests![
        exported_symbol_sets_are_identical,
        every_exported_symbol_is_a_global_text_symbol,
        rust_so_has_no_unresolved_symbols,
        both_objects_import_printf_and_nothing_exotic,
        lazy_binding_matches_the_c_library,
    ]
}
