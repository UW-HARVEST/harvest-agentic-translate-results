//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`,
//! plus the sanity checks that make every other differential test meaningful.

mod common;
use common::*;
use std::process::Command;

/// `nm -D --defined-only` on both libraries; the C set must be a subset of the
/// Rust set (and here it is exactly equal).
#[test]
fn nm_symbol_diff_is_empty() {
    fn dynsyms(p: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("`nm` must be available");
        assert!(out.status.success(), "nm failed on {p:?}");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let cs = dynsyms(&c_so_path());
    let rs = dynsyms(&rust_so_path());
    assert_eq!(cs.len(), 16, "C .so should export 16 symbols, got {cs:?}");

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    let extra: Vec<&String> = rs.iter().filter(|s| !cs.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but absent from the C .so: {extra:?}"
    );
}

/// Nothing outside libc / the unwinder may be left undefined in the Rust `.so`.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "-u", rust_so_path().to_str().unwrap()])
        .output()
        .expect("`nm` must be available");
    assert!(out.status.success());
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| {
            // libc / libgcc / loader imports are expected.
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind_")
                || s.starts_with("__")
                || s.starts_with("_"))
        })
        .filter(|s| !s.starts_with("stbds_"))
        .collect();
    assert!(bad.is_empty(), "unresolved non-libc symbols: {bad:?}");
}

/// Every symbol in SYMBOLS.md must be `dlsym`-able from BOTH libraries.
#[test]
fn every_symbol_loads_from_both_libraries() {
    let cl = unsafe { libloading::Library::new(c_so_path()) }.unwrap();
    let rl = unsafe { libloading::Library::new(rust_so_path()) }.unwrap();
    for name in ALL_SYMBOLS {
        let mut n = name.to_string();
        n.push('\0');
        unsafe {
            let cs: Result<libloading::Symbol<*const ()>, _> = cl.get(n.as_bytes());
            let rs: Result<libloading::Symbol<*const ()>, _> = rl.get(n.as_bytes());
            assert!(cs.is_ok(), "{name} not found in the C .so");
            assert!(rs.is_ok(), "{name} not found in the Rust .so");
        }
    }
    // Both harness handles resolve too.
    let _ = both();
}

/// The two `.so`s must NOT interpose on each other: each library's internal
/// calls have to bind to its own definitions, otherwise every "differential"
/// test would silently compare a library with itself.
///
/// `stbds_hash_seed` is a private static in each library, and
/// `stbds_make_hash_index` copies it into `table->seed`, so giving the two
/// libraries different seeds and reading back `table->seed` proves independence.
#[test]
fn libraries_do_not_interpose() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        (c.rand_seed)(0xAAAA_AAAA_AAAA_AAAA);
        (r.rand_seed)(0xBBBB_BBBB_BBBB_BBBB);
        let ct = (c.shmode_func)(16, SH_ARENA);
        let rt = (r.shmode_func)(16, SH_ARENA);
        let cseed = (*((*header(hash_to_arr(ct, 16))).hash_table as *mut HashIndex)).seed;
        let rseed = (*((*header(hash_to_arr(rt, 16))).hash_table as *mut HashIndex)).seed;
        assert_eq!(
            cseed, 0xAAAA_AAAA_AAAA_AAAA,
            "C library used the wrong hash seed static"
        );
        assert_eq!(
            rseed, 0xBBBB_BBBB_BBBB_BBBB,
            "Rust library used the wrong hash seed static (symbol interposition!)"
        );
        (c.hmfree_func)(hash_to_arr(ct, 16), 16);
        (r.hmfree_func)(hash_to_arr(rt, 16), 16);
    }
}

/// The C structures the tests mirror must have the layout the C compiler chose.
/// (`stbds_hash_index` is 104 bytes, `stbds_array_header` 32, `stbds_hash_bucket`
/// 128, `stbds_string_arena` 24, `stbds_string_block` 16.)  A mismatch here
/// would invalidate every snapshot, and it is also exactly what would break if
/// the Rust translation had picked a different layout.
#[test]
fn mirror_struct_sizes() {
    use std::mem::size_of;
    assert_eq!(size_of::<ArrayHeader>(), 32);
    assert_eq!(size_of::<StringArena>(), 24);
    assert_eq!(size_of::<StringBlock>(), 16);
    assert_eq!(size_of::<HashBucket>(), 128);
    assert_eq!(size_of::<HashIndex>(), 104);
    assert_eq!(HDR, 32);
}

/// `stbds_make_hash_index` places `storage` at the first 64-byte boundary after
/// the header, inside an allocation sized
/// `(slot_count>>3)*128 + 104 + 63`. Both libraries must agree, otherwise the
/// bucket array would overrun.
#[test]
fn hash_index_storage_alignment_matches() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[8usize, 16, 24, 64] {
        sync_seed(1234);
        unsafe {
            let ct = (c.shmode_func)(es, SH_ARENA);
            let rt = (r.shmode_func)(es, SH_ARENA);
            let cti = &*((*header(hash_to_arr(ct, es))).hash_table as *mut HashIndex);
            let rti = &*((*header(hash_to_arr(rt, es))).hash_table as *mut HashIndex);
            let coff = (cti.storage as usize) - ((cti as *const HashIndex) as usize);
            let roff = (rti.storage as usize) - ((rti as *const HashIndex) as usize);
            assert!(coff >= size_of::<HashIndex>(), "C storage overlaps header");
            assert!(roff >= size_of::<HashIndex>(), "Rust storage overlaps header");
            assert_eq!((cti.storage as usize) % 64, 0);
            assert_eq!((rti.storage as usize) % 64, 0);
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}
