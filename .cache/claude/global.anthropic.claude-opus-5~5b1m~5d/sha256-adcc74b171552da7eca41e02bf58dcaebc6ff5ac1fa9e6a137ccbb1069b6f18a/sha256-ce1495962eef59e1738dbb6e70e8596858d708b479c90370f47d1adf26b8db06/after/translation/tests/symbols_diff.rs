//! Phase A / Phase D: exported-symbol parity between the two `.so`s.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm not found");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__cxa") && s != "__gmon_start__")
        .collect()
}

#[test]
fn sym01_every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {c:?}\nRust ({}) = {r:?}",
        c.len(),
        r.len()
    );
    // The C library exports exactly these nine.
    let expect: BTreeSet<String> = [
        "unfilter",
        "cp_inflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(c, expect, "the C surface changed - SYMBOLS.md needs updating");
}

#[test]
fn sym02_symbol_sizes_match() {
    fn sizes(so: &std::path::Path) -> Vec<(String, u64)> {
        let out = Command::new("nm")
            .args(["-D", "-S", "--defined-only"])
            .arg(so)
            .output()
            .unwrap();
        let mut v: Vec<(String, u64)> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                if f.len() == 4 && (f[2] == "D" || f[2] == "B") {
                    Some((f[3].to_string(), u64::from_str_radix(f[1], 16).unwrap()))
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v
    }
    assert_eq!(sizes(&c_so_path()), sizes(&rust_so_path()));
}

#[test]
fn sym03_rust_so_has_no_unresolved_symbols() {
    // RTLD_NOW forces every relocation to be resolved at dlopen() time, so a
    // successful open proves there are no dangling non-libc references.
    use libloading::os::unix::Library;
    let l = unsafe { Library::open(Some(rust_so_path()), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    l.expect("dlopen(RTLD_NOW) of the Rust .so failed - unresolved symbols");
    let l = unsafe { Library::open(Some(c_so_path()), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    l.expect("dlopen(RTLD_NOW) of the C .so failed");
}

#[test]
fn sym04_dead_static_helpers_are_not_exported_by_either_library() {
    // `cp_chunk`, `cp_find`, `cp_make32`, `cp_paeth`, ... are `static` in C.
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    for s in [
        "cp_chunk",
        "cp_find",
        "cp_make32",
        "cp_paeth",
        "cp_build",
        "cp_decode",
        "cp_block",
        "cp_stored",
        "cp_fixed",
        "cp_dynamic",
        "cp_read_bits",
        "cp_peak_bits",
        "cp_consume_bits",
        "cp_rev16",
        "cp_ptr",
        "cp_would_overflow",
        "cp_make_pixel",
        "cp_make_pixel_a",
    ] {
        assert!(!c.contains(s), "C unexpectedly exports {s}");
        assert!(!r.contains(s), "Rust unexpectedly exports {s}");
    }
}

/// CONFIGS row 48 + 53: the six writable tables must have identical contents,
/// and `cp_inflate` must not modify them.
#[test]
fn sym05_table_contents_identical() {
    let c = c_lib();
    let r = rust_lib();
    for t in Table::ALL {
        let a = c.read_table(t);
        let b = r.read_table(t);
        assert_eq!(a.len(), t.bytes());
        assert_eq!(a, b, "table {t:?} differs\n C   = {}\n Rust = {}", hex(&a), hex(&b));
    }
    // Cross-check against the values transcribed from the C initialisers.
    assert_eq!(c.read_table(Table::FixedTable), common::deflate::default_fixed_table());
    assert_eq!(c.read_table(Table::PermutationOrder), common::deflate::DEFAULT_PERMUTATION.to_vec());
    assert_eq!(c.read_table(Table::LenExtraBits), common::deflate::DEFAULT_LEN_EXTRA.to_vec());
    assert_eq!(c.read_table(Table::DistExtraBits), common::deflate::DEFAULT_DIST_EXTRA.to_vec());
    let as_u32 = |v: Vec<u8>| -> Vec<u32> {
        v.chunks(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect()
    };
    assert_eq!(as_u32(c.read_table(Table::LenBase)), common::deflate::DEFAULT_LEN_BASE.to_vec());
    assert_eq!(as_u32(c.read_table(Table::DistBase)), common::deflate::DEFAULT_DIST_BASE.to_vec());
}

/// `cp_state_t`'s layout is observable (`cp_decode` reads `tree[lo-1]`), so pin
/// it down against a C probe compiled from the very same struct definition.
#[test]
fn sym06_cp_state_t_layout_matches_c() {
    use std::mem::offset_of;

    #[repr(C)]
    struct CpState {
        bits: u64,
        count: std::ffi::c_int,
        words: *mut u32,
        word_count: std::ffi::c_int,
        word_index: std::ffi::c_int,
        bits_left: std::ffi::c_int,
        final_word_available: std::ffi::c_int,
        final_word: u32,
        out: *mut std::ffi::c_char,
        out_end: *mut std::ffi::c_char,
        begin: *mut std::ffi::c_char,
        lookup: [u16; 1 << 9],
        lit: [u32; 288],
        dst: [u32; 32],
        len: [u32; 19],
        nlit: u32,
        ndst: u32,
        nlen: u32,
    }

    let rust_offsets: Vec<(&str, usize)> = vec![
        ("bits", offset_of!(CpState, bits)),
        ("count", offset_of!(CpState, count)),
        ("words", offset_of!(CpState, words)),
        ("word_count", offset_of!(CpState, word_count)),
        ("word_index", offset_of!(CpState, word_index)),
        ("bits_left", offset_of!(CpState, bits_left)),
        ("final_word_available", offset_of!(CpState, final_word_available)),
        ("final_word", offset_of!(CpState, final_word)),
        ("out", offset_of!(CpState, out)),
        ("out_end", offset_of!(CpState, out_end)),
        ("begin", offset_of!(CpState, begin)),
        ("lookup", offset_of!(CpState, lookup)),
        ("lit", offset_of!(CpState, lit)),
        ("dst", offset_of!(CpState, dst)),
        ("len", offset_of!(CpState, len)),
        ("nlit", offset_of!(CpState, nlit)),
        ("ndst", offset_of!(CpState, ndst)),
        ("nlen", offset_of!(CpState, nlen)),
        ("sizeof", std::mem::size_of::<CpState>()),
    ];

    // Build a C probe using the struct definition copied verbatim out of
    // c_src/src/lib.c (lines 71-90).
    let probe = manifest_dir().join("target/state_probe.c");
    let bin = manifest_dir().join("target/state_probe");
    std::fs::create_dir_all(probe.parent().unwrap()).unwrap();
    let src = r#"
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
typedef struct cp_state_t {
  uint64_t bits;
  int count;
  uint32_t *words;
  int word_count;
  int word_index;
  int bits_left;
  int final_word_available;
  uint32_t final_word;
  char *out;
  char *out_end;
  char *begin;
  uint16_t lookup[(1 << 9)];
  uint32_t lit[288];
  uint32_t dst[32];
  uint32_t len[19];
  uint32_t nlit;
  uint32_t ndst;
  uint32_t nlen;
} cp_state_t;
#define P(f) printf("%s %zu\n", #f, offsetof(cp_state_t, f))
int main(void) {
  P(bits); P(count); P(words); P(word_count); P(word_index); P(bits_left);
  P(final_word_available); P(final_word); P(out); P(out_end); P(begin);
  P(lookup); P(lit); P(dst); P(len); P(nlit); P(ndst); P(nlen);
  printf("sizeof %zu\n", sizeof(cp_state_t));
  return 0;
}
"#;
    std::fs::write(&probe, src).unwrap();
    let st = Command::new("gcc").arg("-O0").arg(&probe).arg("-o").arg(&bin).output().unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    let out = Command::new(&bin).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let c_offsets: Vec<(String, usize)> = text
        .lines()
        .map(|l| {
            let mut it = l.split_whitespace();
            (it.next().unwrap().to_string(), it.next().unwrap().parse().unwrap())
        })
        .collect();

    assert_eq!(c_offsets.len(), rust_offsets.len());
    for ((cn, co), (rn, ro)) in c_offsets.iter().zip(rust_offsets.iter()) {
        assert_eq!(cn, rn);
        assert_eq!(co, ro, "offset of `{cn}` differs: C = {co}, Rust = {ro}");
    }
}
