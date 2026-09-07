//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The check is done twice, independently:
//!   1. structurally, by parsing the ELF `.dynsym` table of both files with no
//!      external tooling (so the test is self-contained), and
//!   2. via `nm -D`, when `nm` is available on the machine.
//!
//! Additionally every C-exported symbol is proven to be *usable* through
//! `dlsym` on the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

// ---------------------------------------------------------------------------
// Minimal ELF64 dynamic-symbol reader
// ---------------------------------------------------------------------------

fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn rd_u64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ])
}

/// (defined_global_symbols, undefined_symbols) from `.dynsym`.
fn dynsyms(path: &std::path::Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_eq!(&b[0..4], b"\x7fELF", "{} is not an ELF file", path.display());
    assert_eq!(b[4], 2, "{}: only ELF64 is handled", path.display());
    assert_eq!(b[5], 1, "{}: only little-endian is handled", path.display());

    let e_shoff = rd_u64(&b, 0x28) as usize;
    let e_shentsize = rd_u16(&b, 0x3A) as usize;
    let e_shnum = rd_u16(&b, 0x3C) as usize;

    // Find SHT_DYNSYM (11) and its linked string table.
    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_type = rd_u32(&b, sh + 0x04);
        if sh_type != 11 {
            continue;
        }
        let sh_link = rd_u32(&b, sh + 0x28) as usize;
        let sh_offset = rd_u64(&b, sh + 0x18) as usize;
        let sh_size = rd_u64(&b, sh + 0x20) as usize;
        let sh_entsize = rd_u64(&b, sh + 0x38) as usize;

        let str_sh = e_shoff + sh_link * e_shentsize;
        let str_off = rd_u64(&b, str_sh + 0x18) as usize;
        let str_size = rd_u64(&b, str_sh + 0x20) as usize;
        let strtab = &b[str_off..str_off + str_size];

        let count = sh_size / sh_entsize;
        for k in 0..count {
            let sym = sh_offset + k * sh_entsize;
            let st_name = rd_u32(&b, sym) as usize;
            let st_info = b[sym + 4];
            let st_shndx = rd_u16(&b, sym + 6);
            if st_name == 0 {
                continue;
            }
            let end = strtab[st_name..].iter().position(|&c| c == 0).unwrap() + st_name;
            let name = String::from_utf8_lossy(&strtab[st_name..end]).into_owned();
            let bind = st_info >> 4; // 0 LOCAL, 1 GLOBAL, 2 WEAK
            if st_shndx == 0 {
                undefined.insert(name);
            } else if bind == 1 || bind == 2 {
                defined.insert(name);
            }
        }
    }
    (defined, undefined)
}

/// Names that belong to the platform runtime rather than to the library's own
/// API surface. They are filtered out of the comparison; nothing from the C
/// side is ever filtered.
fn is_runtime_name(n: &str) -> bool {
    n.starts_with("_ITM_")
        || n.starts_with("__cxa_")
        || n.starts_with("_Unwind_")
        || n == "__gmon_start__"
        || n == "_init"
        || n == "_fini"
        || n == "__bss_start"
        || n == "_edata"
        || n == "_end"
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let (c_def, _) = dynsyms(&c_so_path());
    let (r_def, _) = dynsyms(&rust_so_path());

    let c_api: BTreeSet<&String> = c_def.iter().filter(|n| !is_runtime_name(n)).collect();
    assert!(
        c_api.contains(&"flip_horizontal".to_string()),
        "sanity: the C .so must export flip_horizontal; got {c_api:?}"
    );

    let missing: Vec<&&String> = c_api.iter().filter(|n| !r_def.contains(**n)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_api:?}\n\
         Rust exports: {:?}",
        missing.len(),
        r_def.iter().filter(|n| !is_runtime_name(n)).collect::<Vec<_>>()
    );
}

#[test]
fn d02_every_c_symbol_resolves_via_dlsym_in_rust_so() {
    let (c_def, _) = dynsyms(&c_so_path());
    let path = rust_so_path();
    unsafe {
        let lib = libloading::Library::new(&path).expect("dlopen rust .so");
        for name in c_def.iter().filter(|n| !is_runtime_name(n)) {
            let mut key = name.clone().into_bytes();
            key.push(0);
            let sym: Result<libloading::Symbol<*mut std::ffi::c_void>, _> = lib.get(&key);
            assert!(sym.is_ok(), "dlsym({name}) failed on {}", path.display());
        }
    }
}

#[test]
fn d03_rust_so_has_no_unresolved_non_libc_symbols() {
    let (_, undef) = dynsyms(&rust_so_path());
    // Everything the Rust std runtime imports lives in libc / libgcc / libm /
    // libpthread / libdl. Anything else would be a missing translation unit.
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|n| {
            let base = n.split('@').next().unwrap();
            if is_runtime_name(base) {
                return false;
            }
            // Resolvable from the already-loaded process image (libc & friends)?
            unsafe {
                let mut key = base.as_bytes().to_vec();
                key.push(0);
                let this = libloading::os::unix::Library::this();
                this.get::<*mut std::ffi::c_void>(&key).is_err()
            }
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols: {suspicious:?}"
    );
}

#[test]
fn d04_nm_symbol_diff_is_empty() {
    let names = |path: &std::path::Path| -> Option<BTreeSet<String>> {
        let out = Command::new("nm").args(["-D", "--defined-only"]).arg(path).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| l.split_whitespace().last().map(str::to_string))
                .filter(|n| !is_runtime_name(n))
                .collect(),
        )
    };
    let (Some(c), Some(r)) = (names(&c_so_path()), names(&rust_so_path())) else {
        eprintln!("`nm` unavailable; d01 already covers this structurally");
        return;
    };
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(missing.is_empty(), "nm -D diff (in C, not in Rust): {missing:?}");
    assert!(c.contains("flip_horizontal"), "sanity: nm found no flip_horizontal in the C .so");
}

#[test]
fn d05_no_stub_behaviour() {
    // A symbol that exists but lies (a stub / unimplemented!()) would either do
    // nothing or abort. Prove the exported Rust symbol really performs the swap.
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xD05);
    let (w, h) = (5i32, 4i32);
    let arena = random_arena(&mut rng, w, h);
    let out = run(&p.rust, w, h, &arena);
    assert_ne!(
        out.bytes, arena.bytes,
        "the Rust flip_horizontal did not modify a 5x4 image — it looks like a stub"
    );
    let mut expected = arena.clone();
    model_flip(w, h, expected.payload_mut());
    assert_eq!(out.bytes, expected.bytes, "the Rust export does not implement the C loop");
}
