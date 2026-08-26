//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both shared objects and requires the Rust `.so` to export
//! *exactly* the same global symbol set as the C `.so` (no missing, no extra),
//! and to have no unresolved non-libc symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Global (`T`/`D`/`B`/`R`) defined dynamic symbols, weak symbols excluded.
fn defined_globals(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let (kind, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b), // "         T name" style
            };
            // skip weak / unique / indirect symbols
            if matches!(kind, "w" | "W" | "v" | "V" | "u") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn d_symbol_parity() {
    let c = c_so_path();
    let r = rust_so_path();
    eprintln!("C   : {}", c.display());
    eprintln!("Rust: {}", r.display());

    let cs = defined_globals(&c);
    let rs = defined_globals(&r);

    // Every symbol the C exports must be exported by the Rust .so.
    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // ...and the Rust .so must not export anything the C does not.
    let extra: Vec<&String> = rs.difference(&cs).collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but not by the C .so: {extra:?}"
    );

    // Sanity: the surface is the expected 16 entry points.
    let expected: BTreeSet<String> = [
        "intput",
        "strkey",
        "stbds_arrfreef",
        "stbds_arrgrowf",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmdel_key",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_rand_seed",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(cs, expected, "the C .so's symbol set changed");
    assert_eq!(rs, expected, "the Rust .so's symbol set changed");
}

#[test]
fn d_no_unresolved_project_symbols() {
    let r = rust_so_path();
    let undef: Vec<String> = nm(&["-D", "-u"], &r)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|n| {
            // libc / libgcc_s / linker-provided symbols are fine
            !(n.contains("@GLIBC")
                || n.contains("@GCC")
                || n.starts_with("_ITM_")
                || n.starts_with("__gmon_start__")
                || n.starts_with("__cxa")
                || n.starts_with("_Unwind_")
                || n.starts_with("__tls_get_addr")
                || n == "U")
        })
        .collect();
    assert!(
        undef.is_empty(),
        "the Rust .so has unresolved non-libc symbols: {undef:?}"
    );
}

/// Every exported symbol must actually be loadable via `dlsym` from *both*
/// libraries (this is what `Pair::new()` does — it resolves all 16 by name).
#[test]
fn d_all_symbols_resolvable() {
    let p = Pair::new();
    // Constructing `Pair` already `dlsym`'d all 16 symbols in both libraries;
    // touching a couple proves the handles are live.
    unsafe {
        p.seed(1);
        let mut b = [1u8, 2, 3, 4];
        assert_eq!(
            (p.c.hash_bytes)(b.as_mut_ptr() as *mut std::ffi::c_void, 4, 1),
            (p.r.hash_bytes)(b.as_mut_ptr() as *mut std::ffi::c_void, 4, 1)
        );
    }
}

// ---------------------------------------------------------------------------
// ABI / struct-layout parity
// ---------------------------------------------------------------------------

/// Pin down the *absolute* on-disk layout of every shared struct, independently
/// for each library.
///
/// The differential snapshots would happily agree if BOTH sides used the same
/// wrong layout as the test harness, so this test checks each library against
/// hard-coded x86-64 LP64 offsets and values derived from the C source instead
/// of against the other library.
#[test]
fn d_struct_layout_matches_c_abi() {
    use std::ffi::{c_char, c_void};

    // The harness mirrors must themselves match the C sizes.
    assert_eq!(std::mem::size_of::<ArrayHeader>(), 32, "stbds_array_header");
    assert_eq!(std::mem::size_of::<StringBlock>(), 16, "stbds_string_block");
    assert_eq!(std::mem::size_of::<Arena>(), 24, "stbds_string_arena");
    assert_eq!(std::mem::size_of::<HashBucket>(), 128, "stbds_hash_bucket");
    assert_eq!(std::mem::size_of::<HashIndex>(), 104, "stbds_hash_index");

    let p = Pair::new();
    for l in p.both() {
        let who = l.name;

        // ---- stbds_array_header: 4 x 8 bytes at data[-32 .. 0] -------------
        unsafe {
            let a = (l.arrgrowf)(std::ptr::null_mut(), 1, 0, 37) as *mut u8;
            assert!(!a.is_null());
            let w = |off: isize| *(a.offset(off) as *const usize);
            assert_eq!(w(-32), 0, "{who}: header.length must be at data[-32]");
            assert_eq!(w(-24), 37, "{who}: header.capacity must be at data[-24]");
            assert_eq!(w(-16), 0, "{who}: header.hash_table must be at data[-16]");
            assert_eq!(w(-8), 0, "{who}: header.temp must be at data[-8]");
            (l.arrfreef)(a as *mut c_void);
        }

        // ---- stbds_hash_index + stbds_hash_bucket -------------------------
        unsafe {
            let gseed = 0x0123_4567_89AB_CDEFusize;
            (l.rand_seed)(gseed);
            let elemsize = 16usize;
            let h = (l.shmode_func)(elemsize, 3 /* STBDS_SH_ARENA */) as *mut u8;
            let hdr = h.sub(elemsize).sub(32);
            let t = *(hdr.add(16) as *const *mut u8);
            assert!(!t.is_null(), "{who}: hash_table must be set");
            let u = |off: usize| *(t.add(off) as *const usize);
            // field order: temp_key, slot_count, used_count, used_count_threshold,
            //              used_count_shrink_threshold, tombstone_count,
            //              tombstone_count_threshold, seed, slot_count_log2,
            //              string{storage,remaining,block,mode}, storage
            assert_eq!(u(8), 8, "{who}: slot_count @ +8");
            assert_eq!(u(16), 0, "{who}: used_count @ +16");
            assert_eq!(u(24), 6, "{who}: used_count_threshold @ +24 (8 - 8>>2)");
            assert_eq!(u(32), 0, "{who}: used_count_shrink_threshold @ +32 (<=8 => 0)");
            assert_eq!(u(40), 0, "{who}: tombstone_count @ +40");
            assert_eq!(u(48), 1, "{who}: tombstone_count_threshold @ +48 (8>>3 + 8>>4)");
            assert_eq!(u(56), gseed, "{who}: seed @ +56 (taken from the global)");
            assert_eq!(u(64), 3, "{who}: slot_count_log2 @ +64 (log2 8)");
            assert_eq!(u(72), 0, "{who}: string.storage @ +72");
            assert_eq!(u(80), 0, "{who}: string.remaining @ +80");
            assert_eq!(*t.add(88), 0u8, "{who}: string.block @ +88");
            assert_eq!(*t.add(89), 3u8, "{who}: string.mode @ +89 (STBDS_SH_ARENA)");
            let storage = *(t.add(96) as *const *mut u8);
            assert!(!storage.is_null(), "{who}: string arena storage ptr @ +96");
            assert_eq!(
                storage as usize % 64,
                0,
                "{who}: bucket storage must be 64-byte (cache-line) aligned"
            );
            // one bucket for slot_count 8: 8 x size_t hash then 8 x ptrdiff_t index
            for i in 0..8usize {
                assert_eq!(
                    *(storage.add(i * 8) as *const usize),
                    0,
                    "{who}: bucket.hash[{i}] must be STBDS_HASH_EMPTY"
                );
                assert_eq!(
                    *(storage.add(64 + i * 8) as *const isize),
                    -1,
                    "{who}: bucket.index[{i}] must be STBDS_INDEX_EMPTY"
                );
            }
            (l.hmfree_func)(hdr.add(32) as *mut c_void, elemsize);
        }

        // ---- stbds_string_block: `storage` starts 8 bytes into the block ---
        unsafe {
            let mut arena = Arena::zeroed();
            let mut s = *b"abc\0";
            let q = (l.stralloc)(&mut arena as *mut Arena, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(q), b"abc".to_vec(), "{who}: stralloc content");
            let block = arena.storage as usize;
            assert_eq!(
                q as usize - block,
                8 + arena.remaining,
                "{who}: stbds_string_block.storage must be at +8 and \
                 p == storage + remaining"
            );
            assert_eq!(arena.remaining, 512 - 4, "{who}: 512-byte first block");
            assert_eq!(arena.block, 1, "{who}: block advanced to 1");
            (l.strreset)(&mut arena as *mut Arena);
            assert!(arena.storage.is_null());
        }
    }
}
