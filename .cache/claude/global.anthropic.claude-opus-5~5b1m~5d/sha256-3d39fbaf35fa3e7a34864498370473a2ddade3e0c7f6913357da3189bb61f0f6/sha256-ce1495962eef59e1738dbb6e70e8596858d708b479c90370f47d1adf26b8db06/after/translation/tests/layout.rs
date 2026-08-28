//! Structural pre-conditions for the differential tests: ABI layout of the two
//! public structs, and `nm -D` symbol parity between the C and Rust `.so`.

mod common;

use common::*;
use std::process::Command;

#[test]
fn struct_offsets_match_the_c_header() {
    // `bs_t { const uint8_t *buf; int pos, limit; }`
    assert_eq!(std::mem::size_of::<BsT>(), 16, "sizeof(bs_t)");
    assert_eq!(std::mem::align_of::<BsT>(), 8, "alignof(bs_t)");
    assert_eq!(std::mem::offset_of!(BsT, buf), 0);
    assert_eq!(std::mem::offset_of!(BsT, pos), 8);
    assert_eq!(std::mem::offset_of!(BsT, limit), 12);

    // `L12_scale_info { float scf[192]; uint8_t total_bands, stereo_bands,
    //                   bitalloc[64], scfcod[64]; }`
    assert_eq!(std::mem::offset_of!(L12ScaleInfo, scf), OFF_SCF);
    assert_eq!(std::mem::offset_of!(L12ScaleInfo, total_bands), OFF_TOTAL_BANDS);
    assert_eq!(std::mem::offset_of!(L12ScaleInfo, stereo_bands), OFF_STEREO_BANDS);
    assert_eq!(std::mem::offset_of!(L12ScaleInfo, bitalloc), OFF_BITALLOC);
    assert_eq!(std::mem::offset_of!(L12ScaleInfo, scfcod), OFF_SCFCOD);
    assert_eq!(std::mem::size_of::<L12ScaleInfo>(), SIZEOF_SCI);

    // The out-of-bounds `bitalloc[i]` window the C code can reach must fit in
    // the region the harness allocates.
    assert!(OFF_BITALLOC + 2 * 255 <= SCI_REGION);
}

fn dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("running `nm` (binutils must be installed)");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| {
            // linker / crt boilerplate present in every ELF shared object
            !matches!(
                s.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
                    | "_ITM_registerTMCloneTable"
                    | "_ITM_deregisterTMCloneTable"
                    | "__gmon_start__"
                    | "rust_eh_personality"
            )
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn symbol_parity_between_c_and_rust_shared_objects() {
    let c = dynamic_symbols(&c_so_path());
    let r = dynamic_symbols(&rust_so_path());

    assert!(
        c.contains(&"dequantize_granule".to_string()),
        "C .so is expected to export dequantize_granule, got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );

    // `get_bits` is `static` in C: it must not leak into the Rust .so either.
    assert!(
        !r.iter().any(|s| s.contains("get_bits")),
        "Rust .so exports get_bits, but it has internal linkage in C: {r:?}"
    );
}

#[test]
fn both_libraries_load_and_expose_the_entry_point() {
    let c = c_lib();
    let r = rust_lib();
    assert_ne!(
        c.dequantize_granule as usize, r.dequantize_granule as usize,
        "the two libraries resolved to the same address — one of the paths is wrong"
    );
    eprintln!("C   .so: {}", c.path.display());
    eprintln!("Rust.so: {}", r.path.display());
}
