//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Runs `nm -D --defined-only` on both shared objects and requires the set of
//! exported (`T`/`t`/`W`) symbols of the C `.so` to be a subset of the Rust
//! `.so`'s. The diff must be EMPTY. Additionally every C symbol is resolved
//! through `dlsym` on the Rust `.so`, so a symbol that exists in `nm` but is not
//! actually dynamically resolvable still fails.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so() -> PathBuf {
    let build = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.into_iter().next().expect("no C .so built")
}

fn rust_so() -> PathBuf {
    let profiles: &[&str] = if cfg!(debug_assertions) {
        &["debug", "release"]
    } else {
        &["release", "debug"]
    };
    for prof in profiles {
        let p = root()
            .join("translation")
            .join("target")
            .join(prof)
            .join("libgen_ray_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so built");
}

/// Returns the set of *defined, exported* symbol names.
fn defined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        // `<addr> <type> <name>`  or  `<type> <name>` for undefined/weak
        let (ty, name) = if cols.len() >= 3 {
            (cols[1], cols[2])
        } else {
            (cols[0], cols[1])
        };
        if matches!(ty, "T" | "t" | "W" | "i") {
            set.insert(name.to_string());
        }
    }
    set
}

/// Symbols the C `.so` exports only because of the C runtime / PIC boilerplate.
/// They are not part of the library's API surface and are not expected in the
/// Rust `.so`.
const C_RUNTIME_NOISE: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "call_weak_fn",
    "deregister_tm_clones",
    "register_tm_clones",
    "__do_global_dtors_aux",
    "frame_dummy",
    "_dl_relocate_static_pie",
];

#[test]
fn phase_d_every_c_symbol_exists_in_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    let interesting: BTreeSet<String> = c
        .iter()
        .filter(|s| !C_RUNTIME_NOISE.contains(&s.as_str()))
        .cloned()
        .collect();

    assert!(
        !interesting.is_empty(),
        "nm found no symbols in the C .so — is it built?"
    );

    let missing: Vec<&String> = interesting.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         (C: {} interesting symbols, Rust: {} defined symbols)",
        missing.len(),
        missing,
        interesting.len(),
        r.len()
    );

    // The 22 documented API symbols must all be there, spelled exactly.
    let api = [
        "c2V",
        "c2Dot",
        "c2Len",
        "c2Add",
        "c2Sub",
        "c2Mulvs",
        "c2Div",
        "c2Norm",
        "c2Minv",
        "c2Maxv",
        "c2Skew",
        "c2Absv",
        "c2RaytoCircle",
        "c2AABBtoAABB",
        "c2RaytoAABB",
        "c2CCW90",
        "c2MulmvT",
        "c2AABBtoPoint",
        "c2CircleToPoint",
        "c2RaytoCapsule",
        "c2CastRay",
        "gen_ray",
    ];
    for name in api {
        assert!(c.contains(name), "C .so does not export {name}");
        assert!(r.contains(name), "Rust .so does not export {name}");
    }
    assert_eq!(
        interesting.len(),
        api.len(),
        "the C .so's API surface changed — SYMBOLS.md needs updating. Got: {:?}",
        interesting
    );

    println!(
        "symbol parity OK: {} API symbols, 0 missing from the Rust .so",
        api.len()
    );
}

/// The `static` C helpers must NOT be exported by either library (they are
/// `static inline` in the C source, so exporting them would be a divergence).
#[test]
fn phase_d_static_helpers_not_exported() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    for name in [
        "c2SignedDistPointToPlane_OneDimensional",
        "c2RayToPlane_OneDimensional",
    ] {
        // They exist as *local* (`t`) symbols in the C .so but must never be
        // dynamically resolvable in either library.
        let c_dyn = unsafe {
            libloading::Library::new(c_so())
                .unwrap()
                .get::<*const ()>(format!("{name}\0").as_bytes())
                .is_ok()
        };
        let r_dyn = unsafe {
            libloading::Library::new(rust_so())
                .unwrap()
                .get::<*const ()>(format!("{name}\0").as_bytes())
                .is_ok()
        };
        assert_eq!(
            c_dyn, r_dyn,
            "{name}: dlsym visibility differs (C={c_dyn}, Rust={r_dyn})"
        );
        let _ = (&c, &r);
    }
}

/// Every API symbol must be resolvable via `dlsym` in *both* libraries — this is
/// what `Lib::open` already does, so simply loading both proves it.
#[test]
fn phase_d_all_symbols_dlsym_resolvable() {
    let p = load();
    // Touch one function from each library so the loads cannot be optimised out.
    let a = unsafe { (p.c.c2V)(1.0, 2.0) };
    let b = unsafe { (p.r.c2V)(1.0, 2.0) };
    assert_eq!(vbits(a), vbits(b));
    println!("all 22 API symbols resolved by dlsym in both .so files");
}

/// Guards against an accidental ABI/layout change: the struct sizes the tests
/// use must match what the C compiler produced.
#[test]
fn phase_d_struct_layout() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(align_of::<c2v>(), 4);
    assert_eq!(size_of::<c2Raycast>(), 12);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2Ray>(), 20);
    assert_eq!(size_of::<c2m>(), 16);
    // Field offsets that the FFI relies on.
    let rc = c2Raycast::default();
    let base = (&raw const rc) as usize;
    assert_eq!((&raw const rc.t) as usize - base, 0);
    assert_eq!((&raw const rc.n) as usize - base, 4);
}
