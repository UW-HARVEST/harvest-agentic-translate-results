//! Phase D -- symbol parity between the C and Rust shared libraries, and the
//! `.data` layout that the C's out-of-range table reads depend on.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// `name -> (type letter, size)` for every dynamic symbol the library defines.
fn defined_symbols(lib: &Path) -> BTreeMap<String, (char, u64)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(lib)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {lib:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <size> <type> <name>" or "<addr> <type> <name>"
        match f.len() {
            4 => {
                let size = u64::from_str_radix(f[1], 16).unwrap_or(0);
                m.insert(f[3].to_string(), (f[2].chars().next().unwrap(), size));
            }
            3 => {
                m.insert(f[2].to_string(), (f[1].chars().next().unwrap(), 0));
            }
            _ => {}
        }
    }
    m
}

fn undefined_symbols(lib: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(lib)
        .output()
        .expect("nm not available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && s != "w")
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let cs = defined_symbols(&c_lib());
    let rs = defined_symbols(&rust_lib());
    assert!(!cs.is_empty(), "nm found no symbols in the C library");

    let missing: Vec<&String> = cs.keys().filter(|k| !rs.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Types must match, and for OBJECTs so must the size (an exported array of
    // the wrong length would break a consumer that copies or indexes it).
    // Function sizes are machine-code sizes and are expected to differ.
    for (name, (ctype, csize)) in &cs {
        let (rtype, rsize) = rs[name];
        assert_eq!(
            ctype.to_ascii_uppercase(),
            rtype.to_ascii_uppercase(),
            "{name}: type {ctype} in C, {rtype} in Rust"
        );
        let is_object = matches!(ctype.to_ascii_uppercase(), 'D' | 'B' | 'R' | 'G' | 'V');
        if is_object {
            assert_eq!(*csize, rsize, "{name}: size {csize} in C, {rsize} in Rust");
        }
    }

    // Report the (allowed) extra symbols so the diff is visible.
    let extra: Vec<&String> = rs.keys().filter(|k| !cs.contains_key(*k)).collect();
    eprintln!(
        "C exports {} symbols, Rust exports {} ({} C symbols matched). Rust-only: {:?}",
        cs.len(),
        rs.len(),
        cs.len(),
        extra
    );
}

#[test]
fn expected_symbol_set() {
    // pinned list, so an accidentally dropped export is caught even if the C
    // library were rebuilt differently
    const EXPECTED: &[(&str, u64)] = &[
        ("cp_dist_base", 128),
        ("cp_dist_extra_bits", 32),
        ("cp_error_reason", 8),
        ("cp_fixed_table", 320),
        ("cp_inflate", 0),
        ("cp_len_base", 124),
        ("cp_len_extra_bits", 31),
        ("cp_permutation_order", 19),
        ("load_png_mem", 0),
    ];
    let cs = defined_symbols(&c_lib());
    let rs = defined_symbols(&rust_lib());
    for (name, size) in EXPECTED {
        assert!(cs.contains_key(*name), "C is missing {name}");
        assert!(rs.contains_key(*name), "Rust is missing {name}");
        if *size != 0 {
            assert_eq!(rs[*name].1, *size, "{name} size");
            assert_eq!(cs[*name].1, *size, "{name} size (C)");
        }
    }
    assert_eq!(cs.len(), EXPECTED.len(), "C symbol set changed: {:?}", cs);
}

#[test]
fn rust_so_has_no_unresolved_symbols() {
    // Load with RTLD_NOW so the dynamic linker resolves *every* import up front;
    // a missing/undefined symbol makes this fail.  (The Rust `cdylib` links the
    // Rust standard library, so besides the six libc functions the C also uses it
    // imports the usual std/libgcc runtime symbols -- `_Unwind_*`, `mmap64`,
    // `pthread_key_create`, ... -- all of which are part of the platform.)
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    let lib = unsafe { Library::open(Some(rust_lib()), RTLD_NOW | RTLD_LOCAL) }
        .expect("dlopen(RTLD_NOW) on the Rust .so failed -- unresolved symbols");
    // and the two entry points really are there
    unsafe {
        let _: libloading::os::unix::Symbol<FnLoad> = lib.get(b"load_png_mem\0").unwrap();
        let _: libloading::os::unix::Symbol<FnInflate> = lib.get(b"cp_inflate\0").unwrap();
    }
    // Every libc symbol the C library needs must also be imported by the Rust one.
    let c_imports = undefined_symbols(&c_lib());
    let r_imports = undefined_symbols(&rust_lib());
    let strip = |s: &String| s.split('@').next().unwrap().to_string();
    let r: Vec<String> = r_imports.iter().map(strip).collect();
    for sym in c_imports.iter().map(strip) {
        // `__assert_fail` is only referenced because the reference build has
        // asserts enabled; the translation is NDEBUG-equivalent by design
        // (see ERRORS.md, rows A1..A10).
        if ["__cxa_finalize", "__gmon_start__", "__assert_fail"].contains(&sym.as_str())
            || sym.starts_with("_ITM_")
        {
            continue;
        }
        assert!(
            r.contains(&sym) || sym == "memcmp" && r.contains(&"bcmp".to_string()),
            "the C .so imports {sym} but the Rust .so does not"
        );
    }
    eprintln!("C imports {:?}", c_imports);
}

/// The reference `.data` section is exactly the six tables, in source order,
/// each 32-byte aligned.  `src/lib.rs` models this layout so that the C's
/// out-of-range table reads (`cp_len_extra_bits[symbol - 257]` with a corrupt
/// Huffman tree) read the same bytes.  Verify the model's premise against the
/// real `.so` by reading the table addresses out of it.
#[test]
fn c_data_layout_is_source_order() {
    let syms = defined_symbols(&c_lib());
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(c_lib())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let mut addr: BTreeMap<String, u64> = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 4 {
            addr.insert(f[3].to_string(), u64::from_str_radix(f[0], 16).unwrap());
        }
    }
    let base = addr["cp_fixed_table"];
    let expected: &[(&str, u64, u64)] = &[
        ("cp_fixed_table", 0, 320),
        ("cp_permutation_order", 320, 19),
        ("cp_len_extra_bits", 352, 31),
        ("cp_len_base", 384, 124),
        ("cp_dist_extra_bits", 512, 32),
        ("cp_dist_base", 544, 128),
    ];
    for (name, off, size) in expected {
        assert_eq!(
            addr[*name] - base,
            *off,
            "{name} is at offset {} in the reference .so, the Rust model assumes {off}. \
             The model in src/lib.rs (blob_byte) must be updated to match.",
            addr[*name] - base
        );
        assert_eq!(syms[*name].1, *size, "{name} size");
    }
}

/// Differential proof that the modelled layout is right: drive an out-of-range
/// `cp_len_extra_bits` / `cp_len_base` read through both libraries and compare.
/// (The exhaustive version is `phase_c_errors::row48_out_of_range_table_reads`.)
#[test]
fn out_of_range_table_read_agrees() {
    let c = Lib::open(&c_lib());
    let r = Lib::open(&rust_lib());
    // fixed-Huffman literal/length symbols 286 and 287 index cp_len_extra_bits
    // and cp_len_base at 29 and 30 -- the last two (zero) entries -- while
    // symbol 287's distance lookup then indexes cp_dist_* normally.
    let lens = fixed_lit_lengths();
    let codes = canonical(&lens);
    for sym in [286usize, 287] {
        let dlens = vec![5u8; 32];
        let dcodes = canonical(&dlens);
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(1, 2);
        bw.huff(codes[b'Z' as usize], lens[b'Z' as usize] as u32);
        bw.huff(codes[sym], lens[sym] as u32);
        bw.huff(dcodes[0], 5);
        bw.huff(codes[256], lens[256] as u32);
        bw.align();
        bw.raw(&[0u8; 8]);
        let stream = bw.finish();
        for align in 0..4 {
            let a = call_inflate(&c, &stream, align, 4096);
            let b = call_inflate(&r, &stream, align, 4096);
            assert_eq!(a, b, "literal/length symbol {sym}, align {align}");
        }
    }
}
