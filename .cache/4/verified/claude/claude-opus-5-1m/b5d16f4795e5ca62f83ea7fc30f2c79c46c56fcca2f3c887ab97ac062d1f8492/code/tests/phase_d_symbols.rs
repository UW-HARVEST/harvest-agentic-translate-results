// Phase D — symbol parity, enforced mechanically rather than by hand.
//
// Asserts that every symbol exported by the C `.so` is also exported by the
// Rust `.so` under the exact same name, and that each is independently
// resolvable via `dlsym` and callable through the FFI boundary.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .join("libdriver.so")
}

/// `nm -D --defined-only <so>` -> set of exported symbol names.
fn exported(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (kind, name) = match (it.next(), it.next(), it.next()) {
                // "<addr> <kind> <name>"
                (Some(_), Some(k), Some(n)) => (k, n),
                // "<kind> <name>" (weak/undefined-style rows without an address)
                (Some(k), Some(n), None) => (k, n),
                _ => return None,
            };
            // Keep code/data exports; drop Rust's internal read-only entries.
            matches!(kind, "T" | "t" | "W" | "w" | "D" | "d" | "B" | "b")
                .then(|| name.to_string())
        })
        .collect()
}

/// The 5 symbols the C library is known to export (see SYMBOLS.md). Pinned so
/// that a C-side change that adds a symbol can't silently shrink this test.
const EXPECTED_C_SYMBOLS: [&str; 5] = ["bad", "driver", "good", "printIntLine", "printLine"];

#[test]
fn sym_c_library_exports_the_expected_surface() {
    let c = exported(&c_so());
    for s in EXPECTED_C_SYMBOLS {
        assert!(
            c.contains(s),
            "C .so is missing {s}; exported = {c:?}\n\
             (rebuild c_src if this is unexpected)"
        );
    }
}

/// The Phase D gate: the symbol diff must be EMPTY.
#[test]
fn sym_rust_exports_every_c_symbol() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    // Weak toolchain stubs are emitted by both toolchains but are not part of
    // the library's own API surface; they are not translation units.
    let toolchain_noise = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__gmon_start__",
        "__cxa_thread_atexit_impl",
        "gettid",
        "statx",
    ];

    let missing: Vec<&String> = c
        .iter()
        .filter(|s| !r.contains(*s))
        .filter(|s| !toolchain_noise.contains(&s.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        missing.len(),
        c.len(),
        r.len(),
    );
}

/// Exported names are necessary but not sufficient: each must actually resolve
/// and be callable in BOTH libraries. `common::libs()` dlsym's all five and
/// panics if any lookup fails.
#[test]
fn sym_all_symbols_resolve_and_are_callable() {
    let l = common::libs();
    for lib in [&l.c, &l.rust] {
        // A call through each of the 5 pointers; output correctness is covered
        // by phases B and C, here we only require they are live entry points.
        let out = common::capture(|| unsafe {
            (lib.print_int_line)(7);
            (lib.print_line)(c"x".as_ptr());
            (lib.bad)();
            (lib.good)();
            (lib.driver)(1);
        });
        assert_eq!(
            out, b"7\nx\n0\n0\n0\n",
            "{}: all five symbols must be callable",
            lib.name
        );
    }
}

/// `bad` and `good` are folded to one address in the optimized Rust build
/// (identical bodies). Both names must still be present and independently
/// resolvable — folding is fine, a missing name is not.
#[test]
fn sym_bad_and_good_are_both_present_despite_icf() {
    let r = exported(&rust_so());
    assert!(r.contains("bad"), "Rust .so must export `bad`");
    assert!(r.contains("good"), "Rust .so must export `good`");
}
