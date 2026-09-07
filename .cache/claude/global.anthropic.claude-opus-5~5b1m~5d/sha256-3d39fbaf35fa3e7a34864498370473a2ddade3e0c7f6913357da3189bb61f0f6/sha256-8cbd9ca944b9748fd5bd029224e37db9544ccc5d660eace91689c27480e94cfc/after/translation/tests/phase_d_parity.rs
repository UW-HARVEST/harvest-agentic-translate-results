//! Phase D — symbol parity and ABI/layout parity between the C `.so` and the
//! Rust `.so`.

mod harness;

use harness::*;
use std::process::Command;

fn dynamic_defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols the Rust `.so` may legitimately import from libc / the runtime.
fn is_runtime_symbol(s: &str) -> bool {
    // Anything carrying a glibc version tag (`sym@GLIBC_x.y`) comes from libc /
    // the Rust std runtime, not from the translated library.
    if s.contains("@GLIBC_") || s.contains("@GCC_") || s.contains("@CXXABI") {
        return true;
    }
    const PREFIXES: &[&str] = &[
        "__",
        "_ITM_",
        "_Unwind",
        "_ZN",   // Rust-internal (panic machinery), not a C API
        "_ZSt",
        "_ZTI",
        "_ZTV",
        "abort",
        "calloc",
        "dl",
        "free",
        "getenv",
        "malloc",
        "mem",
        "posix_",
        "pthread",
        "rea",
        "sig",
        "str",
        "sys",
        "write",
        "close",
        "open",
        "read",
        "poll",
        "bcmp",
    ];
    s.is_empty() || PREFIXES.iter().any(|p| s.starts_with(p))
}

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c = dynamic_defined_symbols(c_so_path());
    let r = dynamic_defined_symbols(rust_so_path());

    eprintln!("C   .so ({}): {:?}", c_so_path().display(), c);
    eprintln!("Rust.so ({}): {:?}", rust_so_path().display(), r);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The one documented public entry point must be there on both sides.
    assert!(
        c.iter().any(|s| s == "dequantize_granule"),
        "C .so does not export dequantize_granule: {c:?}"
    );
    assert!(
        r.iter().any(|s| s == "dequantize_granule"),
        "Rust .so does not export dequantize_granule: {r:?}"
    );

    // `get_bits` is `static` in the C source and must stay unexported on both.
    assert!(
        !c.iter().any(|s| s == "get_bits"),
        "C .so unexpectedly exports get_bits"
    );
    assert!(
        !r.iter().any(|s| s == "get_bits"),
        "Rust .so exports get_bits, but it is `static` in C"
    );
}

#[test]
fn no_unresolved_non_libc_symbols_in_rust_so() {
    let und: Vec<String> = undefined_symbols(rust_so_path())
        .into_iter()
        .filter(|s| !is_runtime_symbol(s))
        .collect();
    assert!(
        und.is_empty(),
        "Rust .so has undefined non-libc symbols: {und:?}"
    );
}

#[test]
fn layout_parity() {
    // c_src/include/lib.h on x86-64 LP64:
    //   bs_t          -> const uint8_t* (8) + int (4) + int (4) = 16, align 8
    //   L12_scale_info-> float[192] (768) + 2 + 64 + 64 = 898 -> padded to 900
    assert_eq!(std::mem::size_of::<BsT>(), 16, "sizeof(bs_t)");
    assert_eq!(std::mem::align_of::<BsT>(), 8, "alignof(bs_t)");
    assert_eq!(std::mem::size_of::<Sci>(), 900, "sizeof(L12_scale_info)");
    assert_eq!(std::mem::align_of::<Sci>(), 4, "alignof(L12_scale_info)");

    let s = Sci::zeroed();
    let base = &s as *const Sci as usize;
    assert_eq!(&s.scf as *const _ as usize - base, 0, "offsetof(scf)");
    assert_eq!(
        &s.total_bands as *const _ as usize - base,
        768,
        "offsetof(total_bands)"
    );
    assert_eq!(
        &s.stereo_bands as *const _ as usize - base,
        769,
        "offsetof(stereo_bands)"
    );
    assert_eq!(
        &s.bitalloc as *const _ as usize - base,
        770,
        "offsetof(bitalloc)"
    );
    assert_eq!(
        &s.scfcod as *const _ as usize - base,
        834,
        "offsetof(scfcod)"
    );

    let b = BsT {
        buf: std::ptr::null(),
        pos: 0,
        limit: 0,
    };
    let bbase = &b as *const BsT as usize;
    assert_eq!(&b.buf as *const _ as usize - bbase, 0, "offsetof(buf)");
    assert_eq!(&b.pos as *const _ as usize - bbase, 8, "offsetof(pos)");
    assert_eq!(&b.limit as *const _ as usize - bbase, 12, "offsetof(limit)");
}

/// The C project builds a `SHARED` library only (see `c_src/CMakeLists.txt`:
/// a single `add_library(... SHARED src/lib.c)` target, no `add_executable`).
/// There is therefore no driver binary whose stdout could be compared; this
/// test pins that fact down so the gate is not silently skipped.
#[test]
fn project_has_no_driver_binary() {
    let cml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/CMakeLists.txt"),
    )
    .expect("cannot read c_src/CMakeLists.txt");
    assert!(
        !cml.contains("add_executable"),
        "c_src/CMakeLists.txt now builds an executable — the stdout comparison \
         gate must be implemented"
    );
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "translation/Cargo.toml now builds a binary — the stdout comparison \
         gate must be implemented"
    );
    let bins = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin");
    assert!(!bins.exists(), "src/bin exists — add the stdout gate");
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/main.rs")
            .exists(),
        "src/main.rs exists — add the stdout gate"
    );
}

/// `translation/Cargo.toml` has no `[features]` table, so the default build is
/// the only configuration. If a feature is ever added this test fails and the
/// Phase B/C suites must be re-run per combination (see scripts/check_all.sh).
#[test]
fn only_one_feature_combination_exists() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml declares features — run the suites for every combination"
    );
}

/// The Rust `.so` must be callable purely through `dlsym`, i.e. the exported
/// wrapper is what does the work. This is a smoke test that the symbol we
/// loaded is genuinely a working entry point on both sides.
#[test]
fn exported_wrapper_is_the_entry_point() {
    let buf: Vec<u8> = (0..256u32).map(|i| (i * 37) as u8).collect();
    let mut sci = Sci::zeroed();
    sci.total_bands = 2;
    sci.bitalloc[..4].copy_from_slice(&[4, 0, 9, 17]);
    diff_case_must_run("D-smoke", &buf, &sci, 4, 0, (buf.len() * 8) as i32);
}
