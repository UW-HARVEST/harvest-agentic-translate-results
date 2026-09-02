//! Phase D — symbol parity enforced as a test.
//!
//! Runs `nm -D --defined-only` on both shared objects and requires that every
//! symbol exported by the C `.so` is also exported by the Rust `.so` under the
//! exact same name, and that the Rust `.so` has no undefined non-libc symbols.

mod common;
use common::*;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("failed to run nm — is binutils installed?");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

fn sorted_unique(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let p = pair();

    let c_syms = sorted_unique(nm(&["-D", "--defined-only"], &p.c.path));
    let rust_syms = sorted_unique(nm(&["-D", "--defined-only"], &p.rust.path));

    println!("C   .so ({}) exports {} symbol(s):", p.c.path.display(), c_syms.len());
    for s in &c_syms {
        println!("  {s}");
    }
    println!("Rust .so ({}) exports {} symbol(s)", p.rust.path.display(), rust_syms.len());

    assert!(!c_syms.is_empty(), "nm found no exported symbols in the C .so");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] export if the impl exists, or translate \
         the missing C source if a whole module was skipped.",
        missing.len(),
        missing
    );
}

#[test]
fn d2_rust_so_has_no_undefined_non_libc_symbols() {
    let p = pair();

    // `nm -D --undefined-only` reports versioned names like `memcpy@GLIBC_2.14`;
    // strip the version suffix before matching.
    let strip = |s: &str| s.split('@').next().unwrap_or(s).to_string();
    let undef: Vec<String> = sorted_unique(nm(&["-D", "--undefined-only"], &p.rust.path))
        .iter()
        .map(|s| strip(s))
        .collect();

    // Build the allowed set mechanically: every symbol defined by the system
    // libraries the dynamic linker actually resolves this .so against. Nothing
    // is hand-listed, so a genuinely missing symbol cannot slip through.
    let ldd = std::process::Command::new("ldd")
        .arg(&p.rust.path)
        .output()
        .expect("failed to run ldd");
    assert!(ldd.status.success(), "ldd failed");
    let ldd_out = String::from_utf8_lossy(&ldd.stdout);
    println!("ldd {}:\n{ldd_out}", p.rust.path.display());
    assert!(
        !ldd_out.contains("not found"),
        "Rust .so has unresolved shared-library dependencies:\n{ldd_out}"
    );

    let mut provided: Vec<String> = Vec::new();
    for line in ldd_out.lines() {
        // "libc.so.6 => /lib64/libc.so.6 (0x...)"  or  "/lib64/ld-linux... (0x...)"
        let tok = if let Some((_, rhs)) = line.split_once("=>") {
            rhs.split_whitespace().next().unwrap_or("")
        } else {
            line.split_whitespace().next().unwrap_or("")
        };
        let path = std::path::Path::new(tok);
        if path.is_absolute() && path.exists() {
            for sym in nm(&["-D", "--defined-only"], path) {
                provided.push(strip(&sym));
            }
        }
    }
    let provided = sorted_unique(provided);
    assert!(
        !provided.is_empty(),
        "could not enumerate any system-library symbols via ldd/nm"
    );

    // Weak/linker-synthesised symbols that are optional by design and remain
    // unresolved in a normal process.
    const OPTIONAL: &[&str] = &[
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
    ];

    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| !provided.contains(s) && !OPTIONAL.contains(&s.as_str()))
        .collect();

    println!(
        "Rust .so: {} undefined symbol(s); {} symbol(s) provided by linked system libs",
        undef.len(),
        provided.len()
    );
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined symbols that no linked library provides: {leftovers:?}"
    );

    // Definitive check: it dlopen'd, the export resolved, and it is callable.
    let s = Bw::zeroed();
    let (r, _) = p.rust.add(s, 8, 0xFF);
    assert_eq!(r, 0);
}

#[test]
fn d3_struct_layout_matches_c_abi() {
    // Layout confirmed independently with an offsetof probe compiled against
    // the real header: size=32 align=8 val=0 bits=8 pos=12 len=16 tot=20
    // buffer=24. Re-assert it here so a layout regression fails loudly.
    assert_eq!(std::mem::size_of::<Bw>(), 32);
    assert_eq!(std::mem::align_of::<Bw>(), 8);
    assert_eq!((OFF_VAL, OFF_BITS, OFF_POS, OFF_LEN, OFF_TOT, OFF_BUFFER), (0, 8, 12, 16, 20, 24));

    // A byte-level probe: only the intended field moves when we set it.
    let p = pair();
    let mut s = Bw::zeroed();
    s.set_bits(0);
    let (_, a) = p.c.add(s, 4, 0xF);
    let (_, b) = p.rust.add(s, 4, 0xF);
    assert_eq!(a, b, "C={a:?} Rust={b:?}");
    assert_eq!(a.bits(), 4);
    assert_eq!(a.tot(), 4);
    assert_eq!(a.pos(), 0);
    assert_eq!(a.len(), 0);
    assert_eq!(a.buffer(), 0);
}
