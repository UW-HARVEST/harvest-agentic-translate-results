//! Phase D — symbol parity and anti-stub checks.
//!
//! Mechanically compares `nm -D --defined-only` on the C `.so` and the Rust
//! cdylib and requires the "missing from Rust" diff to be EMPTY. Also verifies
//! that no symbol was faked with a stub.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn dynamic_defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        // Drop the linker/loader bookkeeping symbols that are not part of the
        // library's API surface (they exist in every ELF shared object).
        .filter(|s| {
            !matches!(
                *s,
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__odr_asan_gen"
                    | "_ITM_registerTMCloneTable"
                    | "_ITM_deregisterTMCloneTable"
                    | "__gmon_start__"
                    | "__cxa_finalize"
            )
        })
        .filter(|s| !s.starts_with("rust_") && !s.starts_with("__rust"))
        .map(str::to_owned)
        .collect()
}

/// The 20 symbols the C library exports. Hard-coded from `nm -D` so that a
/// regression in the C build (fewer symbols) cannot silently weaken this test.
const EXPECTED: &[&str] = &[
    "agglom",
    "c2AABBtoAABB",
    "c2CircletoAABB",
    "c2CircletoCircle",
    "c2Clampv",
    "c2Dot",
    "c2Maxv",
    "c2Minv",
    "c2Sub",
    "c2V",
    "f10",
    "f11",
    "f12",
    "f13",
    "f2",
    "f3",
    "f4",
    "f5",
    "f7",
    "f9",
];

/// The symbol diff MUST reach empty.
#[test]
fn phase_d_symbol_parity() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let c_syms = dynamic_defined_symbols(&c_so);
    let r_syms = dynamic_defined_symbols(&r_so);

    // Sanity: the C side really does export the 20 documented symbols.
    for s in EXPECTED {
        assert!(
            c_syms.contains(*s),
            "C .so ({}) is missing `{s}` — the C build changed, re-derive SYMBOLS.md",
            c_so.display()
        );
    }

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust cdylib ({}) is MISSING {} symbol(s) exported by the C .so ({}): {:?}\n\
         Every missing symbol means either a missing #[no_mangle] wrapper or a \
         whole untranslated C module.",
        r_so.display(),
        missing.len(),
        c_so.display(),
        missing
    );

    // Report (but do not fail on) Rust-only symbols, so drift is visible.
    let extra: Vec<&String> = r_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "Rust cdylib exports {} symbol(s) the C .so does not: {:?}",
        extra.len(),
        extra
    );

    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "expected exactly {} public C symbols, found {:?}",
        EXPECTED.len(),
        c_syms
    );
}

/// Every exported Rust symbol must be resolvable through `dlsym` AND actually
/// callable — a symbol that exists but traps is worse than a missing one.
#[test]
fn phase_d_every_symbol_is_callable() {
    let a = apis();
    // `apis()` already `dlsym`s all 20 names on both libraries (it panics if a
    // name is absent). Now touch each one once so nothing is merely a symbol.
    let mut rng = Rng::new();
    unsafe {
        let v = c2v { x: 1.0, y: 2.0 };
        let w = c2v { x: 3.0, y: 4.0 };
        let ci = c2Circle { p: v, r: 1.0 };
        let bx = c2AABB { min: v, max: w };
        let l1 = lm_vec2 { x: 0.0, y: 0.0 };
        let l2 = lm_vec2 { x: 1.0, y: 0.0 };
        let l3 = lm_vec2 { x: 0.0, y: 1.0 };
        let l4 = lm_vec2 { x: 0.25, y: 0.25 };

        eq_v2("c2V", &(), (a.c.c2V)(1.0, 2.0), (a.r.c2V)(1.0, 2.0));
        eq_v2("c2Maxv", &(), (a.c.c2Maxv)(v, w), (a.r.c2Maxv)(v, w));
        eq_v2("c2Minv", &(), (a.c.c2Minv)(v, w), (a.r.c2Minv)(v, w));
        eq_v2(
            "c2Clampv",
            &(),
            (a.c.c2Clampv)(v, v, w),
            (a.r.c2Clampv)(v, v, w),
        );
        eq_v2("c2Sub", &(), (a.c.c2Sub)(v, w), (a.r.c2Sub)(v, w));
        eq_f32("c2Dot", &(), (a.c.c2Dot)(v, w), (a.r.c2Dot)(v, w));
        eq_i32(
            "c2CircletoCircle",
            &(),
            (a.c.c2CircletoCircle)(ci, ci),
            (a.r.c2CircletoCircle)(ci, ci),
        );
        eq_i32(
            "c2CircletoAABB",
            &(),
            (a.c.c2CircletoAABB)(ci, bx),
            (a.r.c2CircletoAABB)(ci, bx),
        );
        eq_i32(
            "c2AABBtoAABB",
            &(),
            (a.c.c2AABBtoAABB)(bx, bx),
            (a.r.c2AABBtoAABB)(bx, bx),
        );
        let pc = &ci as *const c2Circle as *const std::os::raw::c_void;
        eq_i32(
            "f2",
            &(),
            (a.c.f2)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE),
            (a.r.f2)(pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE),
        );
        eq_i32("f3", &(), (a.c.f3)(7, 3), (a.r.f3)(7, 3));
        let mut sc = cn_rnd_t { state: [1, 2] };
        let mut sr = cn_rnd_t { state: [1, 2] };
        eq_f64("f4", &(), (a.c.f4)(&mut sc), (a.r.f4)(&mut sr));
        eq_u32("f5", &(), (a.c.f5)(0x1234), (a.r.f5)(0x1234));
        eq_u32("f7", &(), (a.c.f7)(4096, 2, 16), (a.r.f7)(4096, 2, 16));
        eq_lm(
            "f9",
            &(),
            (a.c.f9)(l1, l2, l3, l4),
            (a.r.f9)(l1, l2, l3, l4),
        );
        eq_f32("f10", &(), (a.c.f10)(0x3C00), (a.r.f10)(0x3C00));
        for (name, pick) in [
            ("f11", (|x: &Api| x.f11) as fn(&Api) -> FnTri),
            ("f12", (|x: &Api| x.f12) as fn(&Api) -> FnTri),
            ("f13", (|x: &Api| x.f13) as fn(&Api) -> FnTri),
        ] {
            let src = [180.0f32, 0.5, 0.5];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            (pick(&a.c))(dc.as_mut_ptr(), src.as_ptr());
            (pick(&a.r))(dr.as_mut_ptr(), src.as_ptr());
            eq_tri(name, &BitsTri(src), &dc, &dr);
        }
        let z = 0.0f32;
        let ac = (a.c.agglom)(
            z, z, z, z, z, z, z, 1, 1, 1, 2, 3, 4, 2, 16, z, z, z, z, z, z, z, z, 0x3C00, z, z, z,
            z, z, z, z, z, z,
        );
        let ar = (a.r.agglom)(
            z, z, z, z, z, z, z, 1, 1, 1, 2, 3, 4, 2, 16, z, z, z, z, z, z, z, z, 0x3C00, z, z, z,
            z, z, z, z, z, z,
        );
        eq_f64("agglom", &(), ac, ar);
        let _ = rng.next_u32();
    }
}

/// Anti-stub guard: no exported function may be a placeholder.
#[test]
fn phase_d_no_stubs() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for bad in [
        "unimplemented!",
        "todo!",
        "unreachable!(\"stub",
        "panic!(\"not implemented",
    ] {
        assert!(
            !src.contains(bad),
            "src/lib.rs contains `{bad}` — a stub that lies about behaviour is \
             worse than a missing symbol"
        );
    }
}

/// The crate declares no `[features]`, so "every feature combination" is the
/// single default one. Assert that premise so a future feature addition forces
/// this file (and `run_diff_tests.sh`) to be revisited.
#[test]
fn phase_d_feature_surface_is_single() {
    let toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml now has a [features] table: Phases B–C must be re-run for \
         every combination (run_diff_tests.sh enumerates them automatically)"
    );
    // Also: no `#[cfg(feature = ...)]` in the library source.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    assert!(
        !src.contains("feature ="),
        "src/lib.rs is feature-gated but Cargo.toml declares no features"
    );
    // And the C source has no conditional compilation either.
    let c = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    for tok in ["#ifdef", "#ifndef", "#if "] {
        assert!(
            !c.contains(tok),
            "c_src/src/lib.c contains `{tok}` — CONFIGS.md must gain compile-time axes"
        );
    }
}

/// The project builds no driver binary on either side, so there is no stdout to
/// compare. Assert that premise explicitly.
#[test]
fn phase_d_no_binary_target() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).expect("read CMakeLists");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable: Phase B must compare C/Rust stdout"
    );
    let toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !toml.contains("[[bin]]"),
        "translation now builds a binary: Phase B must compare C/Rust stdout"
    );
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/main.rs")
            .exists(),
        "src/main.rs appeared: Phase B must compare C/Rust stdout"
    );
}
