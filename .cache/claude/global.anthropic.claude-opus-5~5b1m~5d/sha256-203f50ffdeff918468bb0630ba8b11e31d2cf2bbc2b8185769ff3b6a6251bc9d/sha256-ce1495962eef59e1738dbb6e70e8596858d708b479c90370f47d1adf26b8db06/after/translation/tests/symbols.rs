// Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//
// Re-derives both symbol lists with `nm -D` at test time so SYMBOLS.md cannot
// silently drift away from reality.

mod common;

use common::{c_so_path, harness, ARRAY_BYTES, N};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm` -- binutils is required for this test");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>" or "         <type> <name>"
        let mut it = line.split_whitespace();
        let (a, b, c) = (it.next(), it.next(), it.next());
        let name = match (a, b, c) {
            (Some(_), Some(_), Some(name)) => name,
            (Some(_), Some(name), None) => name,
            _ => continue,
        };
        set.insert(name.to_string());
    }
    set
}

/// Symbols produced by the ELF/Rust runtime rather than by the translated C.
/// These may appear only in the Rust object; they are never *missing* from it,
/// so they are irrelevant to the C-must-be-covered direction. Listed only so
/// the informational diff printout stays readable.
fn is_runtime_noise(name: &str) -> bool {
    name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("__")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name == "_edata"
        || name == "_end"
        || name == "_IO_stdin_used"
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let h = harness();
    let c_syms = nm_defined(&c_so_path());

    // The three symbols the C library is documented (in SYMBOLS.md) to export.
    let expected: BTreeSet<String> = ["array", "long_exec", "perform_expensive_operations"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let c_real: BTreeSet<String> = c_syms
        .iter()
        .filter(|s| !is_runtime_noise(s))
        .cloned()
        .collect();
    assert_eq!(
        c_real, expected,
        "the C library's exported surface changed; SYMBOLS.md must be regenerated"
    );

    for t in &h.rust {
        let r_syms = nm_defined(&t.path);
        let missing: Vec<&String> = c_real.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "[{}] Rust .so is MISSING {} C symbol(s): {:?}\n\
             Either add the #[no_mangle] extern \"C\" export, or translate the \
             missing C source.",
            t.name,
            missing.len(),
            missing
        );
        let extra: Vec<&String> = r_syms
            .difference(&c_real)
            .filter(|s| !is_runtime_noise(s))
            .collect();
        println!(
            "[{}] symbol parity OK ({} C symbols covered); extra non-runtime \
             symbols: {:?}",
            t.name,
            c_real.len(),
            extra
        );
    }
}

/// Every C symbol must also be *callable/usable* via `dlsym`, not merely
/// present in the symbol table.
#[test]
fn every_c_symbol_resolves_via_dlsym() {
    let h = harness();
    for t in h.all() {
        assert!(
            !t.array_ptr().is_null(),
            "[{}] dlsym(array) returned NULL",
            t.name
        );
        // These panic inside the harness if dlsym fails; a successful lookup is
        // proven by reaching the assertion below.
        let base = t.array_ptr();
        unsafe {
            let saved = std::ptr::read(base);
            std::ptr::write(base, 0x5A5A_5A5A);
            assert_eq!(
                std::ptr::read(base),
                0x5A5A_5A5A,
                "[{}] array is not writable through dlsym",
                t.name
            );
            std::ptr::write(base, saved);
        }
    }
}

/// The `array` object must have the same `st_size` in both objects, because a
/// consumer may `dlsym("array")` and index all 262144 elements.
#[test]
fn array_object_size_matches() {
    let h = harness();
    let size_of = |p: &Path| -> u64 {
        let out = Command::new("readelf")
            .arg("-sW")
            .arg(p)
            .output()
            .expect("failed to run `readelf`");
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        for line in text.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            // idx: value size type bind vis ndx name
            if f.len() >= 8 && f.last() == Some(&"array") && f[3] == "OBJECT" {
                let s = f[2];
                return if let Some(hex) = s.strip_prefix("0x") {
                    u64::from_str_radix(hex, 16).unwrap()
                } else {
                    s.parse().unwrap()
                };
            }
        }
        panic!("no OBJECT symbol named `array` in {}", p.display());
    };
    let c = size_of(&c_so_path());
    assert_eq!(
        c, ARRAY_BYTES,
        "C `array` is {c} bytes, expected {ARRAY_BYTES} (256*1024*sizeof(int))"
    );
    for t in &h.rust {
        let r = size_of(&t.path);
        assert_eq!(
            r, c,
            "[{}] `array` is {r} bytes but the C `array` is {c} bytes",
            t.name
        );
    }
}

/// Both libraries must depend on the *same* libc for `srand`/`rand`/`printf`,
/// otherwise the pseudo-random stream (and therefore `long_exec`'s output)
/// could differ for reasons unrelated to the translation.
#[test]
fn both_libraries_import_the_same_libc_prng() {
    let h = harness();
    let undef = |p: &Path| -> BTreeSet<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--undefined-only")
            .arg(p)
            .output()
            .expect("nm failed");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
            .collect()
    };
    let c = undef(&c_so_path());
    for want in ["srand", "rand"] {
        assert!(
            c.iter().any(|s| s.starts_with(want)),
            "C .so does not import `{want}`; ERRORS/CONFIGS assumptions are stale"
        );
    }
    for t in &h.rust {
        let r = undef(&t.path);
        for want in ["srand", "rand"] {
            assert!(
                r.iter().any(|s| s.starts_with(want)),
                "[{}] Rust .so does not import `{want}` -- it must use the same \
                 libc PRNG as the C code, not a reimplementation",
                t.name
            );
        }
        assert!(
            r.iter().any(|s| s.contains("printf") || s.contains("puts")),
            "[{}] Rust .so does not import a libc printf-family function",
            t.name
        );
    }
}

/// CONFIGS.md claims there is exactly one feature combination. Assert it, so a
/// later addition of a `[features]` table forces the matrix to be revisited.
#[test]
fn features_surface_is_empty() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("cannot read Cargo.toml");
    let has_features = manifest
        .lines()
        .any(|l| l.trim_start().starts_with("[features]"));
    assert!(
        !has_features,
        "Cargo.toml now declares [features]; CONFIGS.md and the test matrix must \
         be extended to cover every feature combination"
    );
}

// ---------------------------------------------------------------------------
// Neither `.so` may enter the process-global symbol search scope.
//
// Both libraries define `array`, `long_exec` AND `perform_expensive_operations`
// under the same unmangled names. `long_exec` calls
// `perform_expensive_operations` through the PLT, so if either library were
// loaded with RTLD_GLOBAL, the C `long_exec` could bind to the *Rust* worker
// (which churns the Rust `array`), leaving the C to fold its own untouched
// array -- silently corrupting the reference values this whole suite compares
// against.
//
// `libloading` uses RTLD_LOCAL, so nothing is published globally. This test
// proves it via `dlsym(RTLD_DEFAULT, ...)`: RTLD_DEFAULT (a NULL handle on
// glibc) searches only the global scope, so it must NOT find these symbols.
// ---------------------------------------------------------------------------
extern "C" {
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const std::ffi::c_char) -> *mut std::ffi::c_void;
}

#[test]
fn no_global_symbol_interposition_between_the_two_libraries() {
    let h = harness();
    // Force both libraries to be loaded before probing.
    assert!(!h.c.array_ptr().is_null());
    for t in &h.rust {
        assert!(!t.array_ptr().is_null());
    }

    for sym in [
        b"perform_expensive_operations\0".as_ref(),
        b"long_exec\0".as_ref(),
        b"array\0".as_ref(),
    ] {
        let found = unsafe { dlsym(std::ptr::null_mut(), sym.as_ptr() as *const std::ffi::c_char) };
        assert!(
            found.is_null(),
            "`{}` is visible in the process-global scope; the C library could \
             bind to the Rust definition (or vice versa) and the differential \
             comparison would be measuring the wrong thing",
            String::from_utf8_lossy(&sym[..sym.len() - 1])
        );
    }

    // Corroborate behaviourally: each library's `long_exec`/worker pair must
    // operate on that library's own `array`. Running only the C worker must
    // leave every Rust `array` untouched.
    let sentinel = vec![0x0BAD_F00Du32 as i32; N];
    let zeros = vec![0i32; N];
    h.c.write_array(&sentinel);
    for t in &h.rust {
        t.write_array(&zeros);
    }
    h.c.peo();
    assert_ne!(h.c.read_array()[0], sentinel[0], "the C worker did not run");
    for t in &h.rust {
        assert!(
            t.read_array().iter().all(|&v| v == 0),
            "[{}] the C worker mutated the Rust `array` -- symbols are crossing",
            t.name
        );
    }
}
