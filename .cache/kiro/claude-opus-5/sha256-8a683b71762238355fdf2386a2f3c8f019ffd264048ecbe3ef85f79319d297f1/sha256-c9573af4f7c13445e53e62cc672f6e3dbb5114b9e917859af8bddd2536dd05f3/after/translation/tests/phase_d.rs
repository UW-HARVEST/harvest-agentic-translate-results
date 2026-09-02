//! Phase D — symbol parity, ABI/layout parity, and the completion gate.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// The full list of symbols the C `.so` exports (see `SYMBOLS.md`).  Kept here as
/// a literal so the test fails if the C surface ever grows without the Rust one.
const C_SYMBOLS: [&str; 16] = [
    "arr_ins",
    "strkey",
    "stbds_rand_seed",
    "stbds_hash_bytes",
    "stbds_hash_string",
    "stbds_arrgrowf",
    "stbds_arrfreef",
    "stbds_hmfree_func",
    "stbds_hmget_key",
    "stbds_hmget_key_ts",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_hmdel_key",
    "stbds_shmode_func",
    "stbds_stralloc",
    "stbds_strreset",
];

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm");
    assert!(out.status.success(), "nm failed on {:?}", path);
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "R" | "r") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d_01_symbol_diff_is_empty() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {:?}",
        missing
    );
    // and the expected surface is exactly what we documented
    for s in C_SYMBOLS.iter() {
        assert!(c.contains(&s.to_string()), "C .so no longer exports {}", s);
        assert!(r.contains(&s.to_string()), "Rust .so does not export {}", s);
    }
    assert_eq!(
        c.len(),
        C_SYMBOLS.len(),
        "the C .so surface changed: {:?}",
        c
    );
}

#[test]
fn d_02_every_symbol_is_callable_through_dlsym() {
    // Loading via `Lib::load` resolves all 16 symbols in BOTH libraries; if any
    // #[no_mangle] wrapper were missing this panics.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.r.name, "Rust");
}

#[test]
fn d_03_no_unresolved_imports_in_rust_so() {
    let out = std::process::Command::new("ldd")
        .arg(rust_so_path())
        .output()
        .expect("ldd");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        !s.contains("not found"),
        "the Rust .so has unresolved dependencies:\n{}",
        s
    );
}

// ---------------------------------------------------------------------------
// ABI / layout parity, proved against the C library's OWN memory.
//
// The differential harness reads `stbds_hash_index` fields out of buffers that
// the C library allocated and filled.  These invariants only hold if the Rust
// mirror's field offsets match the C struct exactly, so they are a direct
// empirical check of the layout (and of `#[repr(C)]` in src/lib.rs).
// ---------------------------------------------------------------------------

fn check_table_invariants(lib: &Lib, label: &str) {
    unsafe {
        (lib.rand_seed)(0x31415926);
        let mut rng = Rng::new(0xD04);
        let mut hm = Hm::new(lib, 16, 8, 0);
        let mut keys = Vec::new();
        let mut seen_slot_counts = std::collections::BTreeSet::new();
        for i in 0..3000usize {
            let k = rng.bytes(8);
            hm.put_kv(&k, &(i as u64).to_le_bytes().to_vec(), STBDS_HM_BINARY);
            keys.push(k);
            let tb = snap_table(hm.raw());
            assert!(tb.present, "{}: table missing after put", label);
            let sc = tb.slot_count;
            seen_slot_counts.insert(sc);
            assert!(sc >= 8 && sc.is_power_of_two(), "{}: slot_count={}", label, sc);
            assert_eq!(tb.used_count_threshold, sc - (sc >> 2), "{}", label);
            assert_eq!(
                tb.tombstone_count_threshold,
                (sc >> 3) + (sc >> 4),
                "{}",
                label
            );
            assert_eq!(
                tb.used_count_shrink_threshold,
                if sc <= 8 { 0 } else { sc >> 2 },
                "{}",
                label
            );
            assert_eq!(tb.slot_count_log2, sc.trailing_zeros() as usize, "{}", label);
            assert_eq!(tb.hashes.len(), sc, "{}", label);
            assert_eq!(tb.indices.len(), sc, "{}", label);
            assert!(tb.used_count < tb.slot_count, "{}", label);
            assert_eq!(
                tb.used_count,
                (*hm.header()).length - 1,
                "{}: used_count vs array length",
                label
            );
            assert_eq!(tb.str_mode, 0, "{}: binary map must have string.mode 0", label);
            // exactly `used_count` slots are in use and `tombstone_count` deleted
            let in_use = tb.indices.iter().filter(|&&x| x >= 0).count();
            let dead = tb.indices.iter().filter(|&&x| x == -2).count();
            assert_eq!(in_use, tb.used_count, "{}", label);
            assert_eq!(dead, tb.tombstone_count, "{}", label);
        }
        assert!(
            seen_slot_counts.len() >= 8,
            "{}: expected several table growths, saw {:?}",
            label,
            seen_slot_counts
        );
        // storage must be 64-byte aligned (STBDS_ALIGN_FWD(..., CACHE_LINE_SIZE))
        let h = hm.header();
        let ti = (*h).hash_table as *mut HashIndex;
        assert_eq!(
            (*ti).storage as usize % STBDS_CACHE_LINE_SIZE_T,
            0,
            "{}: bucket storage is not cache-line aligned",
            label
        );
        assert!(
            ((*ti).storage as usize) >= (ti as usize) + std::mem::size_of::<HashIndex>(),
            "{}: bucket storage overlaps the header",
            label
        );
        hm.free();
    }
}

const STBDS_CACHE_LINE_SIZE_T: usize = 64;

#[test]
fn d_04_layout_invariants_hold_in_both_libraries() {
    let p = pair();
    check_table_invariants(&p.c, "C");
    check_table_invariants(&p.r, "Rust");
}

#[test]
fn d_05_array_header_layout() {
    // length / capacity / hash_table / temp, in that order, 32 bytes total.
    assert_eq!(HEADER_SIZE, 32);
    let p = pair();
    for lib in [&p.c, &p.r] {
        unsafe {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), 8, 0, 5);
            let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
            assert_eq!((*h).length, 0, "{}", lib.name);
            // min_cap=5: not <= arrcap(NULL)=0, not < 2*0, not < 4 -> stays 5
            assert_eq!((*h).capacity, 5, "{}: capacity", lib.name);
            assert!((*h).hash_table.is_null(), "{}", lib.name);
            assert_eq!((*h).temp, 0, "{}", lib.name);
            (lib.arrfreef)(a);
        }
    }
}

#[test]
fn d_06_string_arena_layout() {
    assert_eq!(std::mem::size_of::<StringArena>(), 24);
    let p = pair();
    for lib in [&p.c, &p.r] {
        unsafe {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            let s = b"hello\0";
            let q = (lib.stralloc)(&mut a, s.as_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(q), b"hello".to_vec(), "{}", lib.name);
            // 512-byte block, 6 bytes consumed
            assert_eq!(a.remaining, 512 - 6, "{}", lib.name);
            assert_eq!(a.block, 1, "{}", lib.name);
            assert!(!a.storage.is_null(), "{}", lib.name);
            (lib.strreset)(&mut a);
            assert_eq!(a.remaining, 0, "{}", lib.name);
            assert_eq!(a.block, 0, "{}", lib.name);
            assert!(a.storage.is_null(), "{}", lib.name);
        }
    }
}

#[test]
fn d_07_hash_index_size_and_field_order() {
    assert_eq!(std::mem::size_of::<HashIndex>(), 104);
    assert_eq!(std::mem::size_of::<HashBucket>(), 128);
    // `stbds_temp_key(t)` is *(char**)hash_table, i.e. the FIRST field must be
    // temp_key.  Verify by driving a string put and reading both spellings.
    let p = pair();
    for lib in [&p.c, &p.r] {
        unsafe {
            (lib.rand_seed)(0x31415926);
            let key = b"the_only_key\0";
            let mut hm = Hm::from_shmode(lib, 16, 8, STBDS_SH_DEFAULT);
            hm.put_str(
                key.as_ptr() as *mut c_char,
                &0u64.to_le_bytes().to_vec(),
                STBDS_HM_STRING,
            );
            let ti = (*hm.header()).hash_table as *mut c_void;
            let first_field = *(ti as *mut *mut c_char);
            assert_eq!(
                first_field, hm.temp_key(),
                "{}: hash_index's first field is not temp_key",
                lib.name
            );
            assert_eq!(cstr_bytes(first_field), b"the_only_key".to_vec(), "{}", lib.name);
            hm.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Completion gate, self-documenting.
// ---------------------------------------------------------------------------

#[test]
fn d_08_artifacts_exist_and_all_rows_are_checked() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for f in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let p = dir.join(f);
        let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{:?}: {}", p, e));
        assert!(s.len() > 500, "{} looks empty", f);
    }
    // every CONFIGS.md row must be ticked
    let cfg = std::fs::read_to_string(dir.join("CONFIGS.md")).unwrap();
    let unticked: Vec<&str> = cfg
        .lines()
        .filter(|l| l.starts_with("| ") && l.contains("| [ ]"))
        .collect();
    assert!(
        unticked.is_empty(),
        "CONFIGS.md still has unchecked rows: {:?}",
        unticked
    );
}
