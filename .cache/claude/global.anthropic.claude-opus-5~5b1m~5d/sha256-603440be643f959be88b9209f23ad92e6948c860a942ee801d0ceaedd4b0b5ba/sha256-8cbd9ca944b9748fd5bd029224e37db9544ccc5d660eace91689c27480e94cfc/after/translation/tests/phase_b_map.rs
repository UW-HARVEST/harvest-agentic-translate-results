//! Phase B rows C11–C29: `stbds_hmput_key` / `stbds_hmget_key` /
//! `stbds_hmget_key_ts` across every `mode`, every `string.mode`, and every
//! `elemsize`/`keysize` shape the C branches on.

mod common;
use common::*;

fn u32key(v: u32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

/// Insert `n` binary keys of `keysize` bytes, re-get them all, re-put them all.
fn binary_roundtrip(row: &str, elemsize: usize, keysize: usize, n: usize, seed: u64) {
    let s = session(0x31415926);
    let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
    let mut rng = Rng::new(seed);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..n {
        let k = rng.bytes(keysize);
        let v = rng.bytes(elemsize);
        let ctx = format!("{row} put#{i}");
        d.hmput(&ctx, &k, &v, HM_BINARY);
        d.check(&ctx);
        keys.push(k);
    }
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("{row} get#{i}");
        d.hmgeti(&ctx, k, HM_BINARY);
        d.check(&ctx);
        let ctx = format!("{row} get_ts#{i}");
        d.hmgeti_ts(&ctx, k, HM_BINARY);
        d.check(&ctx);
    }
    // absent keys
    for i in 0..n.min(64) {
        let mut k = rng.bytes(keysize);
        k[0] ^= 0xA5;
        let ctx = format!("{row} miss#{i}");
        d.hmgeti(&ctx, &k, HM_BINARY);
        d.check(&ctx);
    }
    // duplicate re-put (the "found existing key" hit path)
    for (i, k) in keys.iter().enumerate() {
        let v = rng.bytes(elemsize);
        let ctx = format!("{row} reput#{i}");
        d.hmput(&ctx, k, &v, HM_BINARY);
        d.check(&ctx);
    }
    d.check(&format!("{row} final"));
    d.free();
}

// ---------------------------------------------------------------- C11-C14
#[test]
fn c11_binary_single_insert() {
    binary_roundtrip("C11", 16, 4, 1, 0x11);
}

#[test]
fn c12_binary_five_inserts_below_threshold() {
    binary_roundtrip("C12", 16, 4, 5, 0x12);
}

#[test]
fn c13_binary_crosses_grow_threshold() {
    for n in 6..=9usize {
        binary_roundtrip(&format!("C13 n={n}"), 16, 4, n, 0x13 + n as u64);
    }
}

#[test]
fn c14_binary_many_inserts_multiple_grows() {
    binary_roundtrip("C14", 16, 4, 500, 0x14);
}

// ---------------------------------------------------------------- C15-C18
#[test]
fn c15_keysize_8() {
    binary_roundtrip("C15", 16, 8, 200, 0x15);
}

#[test]
fn c16_keysize_equals_elemsize() {
    binary_roundtrip("C16", 16, 16, 200, 0x16);
}

#[test]
fn c17_keysize_1_and_2_heavy_collisions() {
    // keysize 1 => at most 256 distinct keys, so lots of duplicate re-puts.
    binary_roundtrip("C17 k=1", 16, 1, 300, 0x17);
    binary_roundtrip("C17 k=2", 16, 2, 300, 0x18);
}

#[test]
fn c18_elemsize_24_keysize_8() {
    binary_roundtrip("C18", 24, 8, 200, 0x19);
    binary_roundtrip("C18b e=40 k=8", 40, 8, 120, 0x1A);
    binary_roundtrip("C18c e=8 k=8", 8, 8, 120, 0x1B);
    binary_roundtrip("C18d e=4 k=4", 4, 4, 120, 0x1C);
    binary_roundtrip("C18e e=1 k=1", 1, 1, 120, 0x1D);
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_hmget_key_ts_low_level_binary() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let keysize = 4usize;

    // (a) `a == NULL` bootstrap through the _ts entry point directly.
    let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
    let k = u32key(7);
    let t = d.hmgeti_ts("C19 bootstrap", &k, HM_BINARY);
    assert_eq!(t, -1, "C19: *temp must be STBDS_INDEX_EMPTY on bootstrap");
    d.check("C19 bootstrap");
    // (b) array exists but hash_table == NULL -> *temp = -1
    let t = d.hmgeti_ts("C19 no-table", &k, HM_BINARY);
    assert_eq!(t, -1);
    d.check("C19 no-table");
    // (c) populate, then present / absent
    let mut rng = Rng::new(0x19);
    let mut keys = Vec::new();
    for i in 0..40 {
        let kk = u32key(i * 3 + 1);
        d.hmput(&format!("C19 put#{i}"), &kk, &rng.bytes(elemsize), HM_BINARY);
        keys.push(kk);
    }
    for (i, kk) in keys.iter().enumerate() {
        let a = d.hmgeti_ts(&format!("C19 hit#{i}"), kk, HM_BINARY);
        assert!(a >= 0, "C19: key {i} should be present");
        d.check(&format!("C19 hit#{i}"));
    }
    for i in 0..40u32 {
        let kk = u32key(i * 3 + 2); // never inserted
        let a = d.hmgeti_ts(&format!("C19 miss#{i}"), &kk, HM_BINARY);
        assert_eq!(a, -1, "C19: key {i} must be absent");
        d.check(&format!("C19 miss#{i}"));
    }
    d.free();
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_hmget_key_ts_low_level_string() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    // SH_NONE is excluded: with `string.mode == 0` the key slot holds raw string
    // BYTES, so any lookup would make `is_key_equal` dereference them as a
    // `char *`.  That configuration is covered put-only by `c24`.
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let kind = KeyKind::StringAt(0);
        let mut d = Driver::shmode(&s, elemsize, 8, kind, mode);
        let mut rng = Rng::new(0x20 + mode as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60 {
            let k = rng.ascii_range(1, 20);
            let ctx = format!("C20 m={mode} put#{i}");
            d.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
            d.check(&ctx);
            keys.push(k);
        }
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("C20 m={mode} ts-hit#{i}");
            let a = d.shgeti_ts(&ctx, k, HM_STRING);
            assert!(a >= 0, "{ctx}: should be present");
            d.check(&ctx);
        }
        for i in 0..40 {
            let k = format!("--absent-{i}--").into_bytes();
            let ctx = format!("C20 m={mode} ts-miss#{i}");
            let a = d.shgeti_ts(&ctx, &k, HM_STRING);
            assert_eq!(a, -1, "{ctx}: should be absent");
            d.check(&ctx);
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C21
#[test]
fn c21_string_implicit_default_mode() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    // `t == NULL` + mode>=1 => hmput_key sets nt->string.mode = STBDS_SH_DEFAULT
    let mut d = Driver::lazy(&s, elemsize, 8, KeyKind::StringAt(0));
    let mut rng = Rng::new(0x21);
    let mut keys = Vec::new();
    for i in 0..150 {
        let k = rng.ascii_range(1, 24);
        let ctx = format!("C21 put#{i}");
        d.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
        d.check(&ctx);
        keys.push(k);
    }
    unsafe {
        let t = header(d.c_raw()).hash_table as *mut HashIndex;
        assert_eq!((*t).string.mode, SH_DEFAULT as u8, "C21: implicit SH_DEFAULT");
    }
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("C21 get#{i}");
        assert!(d.shgeti(&ctx, k, HM_STRING) >= 0, "{ctx}");
        d.check(&ctx);
    }
    d.free();
}

// ---------------------------------------------------------------- C22-C26
fn string_mode_row(row: &str, mode: i32, elemsize: usize, n: usize, seed: u64) {
    let s = session(0x31415926);
    let kind = if mode == SH_NONE {
        KeyKind::Binary
    } else {
        KeyKind::StringAt(0)
    };
    let mut d = Driver::shmode(&s, elemsize, 8, kind, mode);
    unsafe {
        let ct = header(d.c_raw()).hash_table as *mut HashIndex;
        let rt = header(d.r_raw()).hash_table as *mut HashIndex;
        assert_eq!((*ct).string.mode, mode as u8, "{row}: C string.mode");
        assert_eq!((*rt).string.mode, mode as u8, "{row}: Rust string.mode");
    }
    let mut rng = Rng::new(seed);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..n {
        let k = rng.ascii_range(1, 30);
        let ctx = format!("{row} put#{i}");
        d.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
        d.check(&ctx);
        keys.push(k);
    }
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("{row} get#{i}");
        d.shgeti(&ctx, k, HM_STRING);
        d.check(&ctx);
    }
    // Re-put every key: hits the existing-key branch, refreshing `temp_key`.
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("{row} reput#{i}");
        d.shput(&ctx, k, &rng.bytes(elemsize), HM_STRING);
        d.check(&ctx);
    }
    // `shputs` form (writes the struct back, then re-reads temp_key).  Only on
    // FRESH keys: see `c26b_shputs_existing_key_aliases_temp_key` for why
    // `shputs` on an existing key is not free()-able in the C either.
    for i in 0..20usize {
        let k = format!("~shputs~{row}~{i}").into_bytes();
        let ctx = format!("{row} shputs#{i}");
        d.shputs(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
        d.check(&ctx);
    }
    d.check(&format!("{row} final"));
    d.free();
}

#[test]
fn c22_strdup_mode() {
    string_mode_row("C22", SH_STRDUP, 16, 200, 0x22);
    string_mode_row("C22b e=24", SH_STRDUP, 24, 80, 0x23);
}

#[test]
fn c23_arena_mode() {
    string_mode_row("C23", SH_ARENA, 16, 200, 0x24);
    string_mode_row("C23b e=32", SH_ARENA, 32, 80, 0x25);
}

#[test]
fn c24_shmode_none_with_string_put() {
    // `shmode_func(elemsize, STBDS_SH_NONE)` then a `mode == STBDS_HM_STRING`
    // put: the `switch (table->string.mode)` at lib.c:785 falls to `default:`
    // and `memcpy`s `keysize` bytes of the *string itself* into the key slot
    // (no pointer is stored, and `temp_key` is left untouched).  Distinct keys
    // only -- any lookup would then dereference those bytes as a `char *`.
    let s = session(0x31415926);
    for (elemsize, keysize) in [(16usize, 8usize), (24, 8), (16, 4), (8, 8), (16, 16)] {
        let mut d = Driver::shmode(&s, elemsize, keysize, KeyKind::Binary, SH_NONE);
        let mut rng = Rng::new(0x26 ^ (elemsize * 31 + keysize) as u64);
        for i in 0..120usize {
            // Unique keys: `shput` here goes straight down the insert path.
            let k = format!("none-{i:04}-{}", rng.below(1 << 20)).into_bytes();
            let cs = CStrBuf::new(&k);
            let ctx = format!("C24 e={elemsize} k={keysize} put#{i}");
            unsafe {
                let kp = cs.ptr() as *mut std::ffi::c_void;
                d.cm = (s.c.hmput_key)(d.cm, elemsize, kp, keysize, HM_STRING);
                d.rm = (s.r.hmput_key)(d.rm, elemsize, kp, keysize, HM_STRING);
                let ci = header(d.c_raw()).temp;
                let ri = header(d.r_raw()).temp;
                assert_eq!(ci, ri, "{ctx}: temp index");
                // `arrgrowf` does not zero new slots and the `default:` arm only
                // writes `keysize` bytes, so the tail of the element is
                // uninitialised heap in BOTH libraries.  The macro
                // (`shput`/`hmput`) always assigns `.value` right afterwards;
                // do the same so every compared byte is defined.
                if elemsize > keysize {
                    let tail = rng.bytes(elemsize - keysize);
                    for (m, base) in [(d.cm, 0usize), (d.rm, 1)] {
                        let _ = base;
                        std::ptr::copy_nonoverlapping(
                            tail.as_ptr(),
                            (m as *mut u8).offset(ci * elemsize as isize).add(keysize),
                            tail.len(),
                        );
                    }
                }
            }
            d.check(&ctx);
        }
        // `string.mode == 0 != STBDS_SH_STRDUP` -> hmfree_func frees no keys.
        d.free();
    }
}

/// Documents (and pins) the C's own `shputs`-on-an-existing-key behaviour: the
/// wrapped-scan hit branch at lib.c:746-751 sets `stbds_temp(a)` but *not*
/// `stbds_temp_key(a)`, so `stbds_shputs` can copy a STALE `temp_key` -- another
/// element's pointer -- into the element.  In `STBDS_SH_STRDUP` mode that makes
/// two elements alias one `malloc` block, and `hmfree_func` then double-frees.
/// The Rust must reproduce that state exactly; the map is deliberately LEAKED
/// instead of freed so the (shared) allocator is not corrupted.
#[test]
fn c26b_shputs_existing_key_aliases_temp_key() {
    let s = session(0x31415926);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), mode);
        let mut rng = Rng::new(0x26B + mode as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..120usize {
            let k = format!("dup-{i}-{}", rng.below(1 << 16)).into_bytes();
            let ctx = format!("C26b m={mode} put#{i}");
            d.shput(&ctx, &k, &rng.bytes(16), HM_STRING);
            d.check(&ctx);
            keys.push(k);
        }
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("C26b m={mode} shputs-existing#{i}");
            d.shputs(&ctx, k, &rng.bytes(16), HM_STRING);
            d.check(&ctx);
        }
        d.check(&format!("C26b m={mode} final"));
        // deliberately NOT freed -- see the doc comment.
        std::mem::forget(std::mem::replace(&mut d.keep, Vec::new()));
        d.cm = std::ptr::null_mut();
        d.rm = std::ptr::null_mut();
    }
}

#[test]
fn c25_explicit_default_mode() {
    string_mode_row("C25", SH_DEFAULT, 16, 200, 0x27);
}

#[test]
fn c26_all_string_modes_many_keys() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        string_mode_row(&format!("C26 m={mode}"), mode, 16, 260, 0x2600 + mode as u64);
    }
}

// ---------------------------------------------------------------- C27
#[test]
fn c27_string_prefix_and_duplicate_keys() {
    let s = session(0x31415926);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), mode);
        let mut rng = Rng::new(0x27 + mode as u64);
        let base = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut keys: Vec<Vec<u8>> = Vec::new();
        // long shared prefixes, differing only in the last byte
        for c in b'a'..=b'z' {
            let mut k = base.to_vec();
            k.push(c);
            keys.push(k);
        }
        // strict prefixes of each other
        for l in 0..=32usize {
            keys.push(base[..l].to_vec());
        }
        // exact duplicates
        keys.push(b"a".to_vec());
        keys.push(b"a".to_vec());
        keys.push(base.to_vec());
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("C27 m={mode} put#{i}");
            d.shput(&ctx, k, &rng.bytes(16), HM_STRING);
            d.check(&ctx);
        }
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("C27 m={mode} get#{i}");
            assert!(d.shgeti(&ctx, k, HM_STRING) >= 0, "{ctx}");
            d.check(&ctx);
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C28
#[test]
fn c28_empty_string_key() {
    let s = session(0x31415926);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), mode);
        let mut rng = Rng::new(0x28 + mode as u64);
        d.shput("C28 empty-first", b"", &rng.bytes(16), HM_STRING);
        d.check("C28 empty-first");
        for i in 0..30 {
            let k = format!("k{i}").into_bytes();
            let ctx = format!("C28 m={mode} put#{i}");
            d.shput(&ctx, &k, &rng.bytes(16), HM_STRING);
            d.check(&ctx);
        }
        d.shput("C28 empty-again", b"", &rng.bytes(16), HM_STRING);
        d.check("C28 empty-again");
        assert!(d.shgeti("C28 empty-get", b"", HM_STRING) >= 0);
        d.check("C28 empty-get");
        d.free();
    }
}

// ---------------------------------------------------------------- C29
#[test]
fn c29_mode_two_ptr_to_string() {
    // mode == 2 (STBDS_HM_PTR_TO_STRING) — `mode >= STBDS_HM_STRING` so it
    // hashes and compares as a string.
    let s = session(0x31415926);
    let mut d = Driver::lazy(&s, 16, 8, KeyKind::StringAt(0));
    let mut rng = Rng::new(0x29);
    let mut keys = Vec::new();
    for i in 0..120 {
        let k = rng.ascii_range(1, 20);
        let ctx = format!("C29 put#{i}");
        d.shput(&ctx, &k, &rng.bytes(16), 2);
        d.check(&ctx);
        keys.push(k);
    }
    unsafe {
        let t = header(d.c_raw()).hash_table as *mut HashIndex;
        assert_eq!(
            (*t).string.mode,
            SH_DEFAULT as u8,
            "C29: mode 2 >= HM_STRING => SH_DEFAULT table"
        );
    }
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("C29 get#{i}");
        assert!(d.shgeti(&ctx, k, 2) >= 0, "{ctx}");
        d.check(&ctx);
    }
    d.free();
}
