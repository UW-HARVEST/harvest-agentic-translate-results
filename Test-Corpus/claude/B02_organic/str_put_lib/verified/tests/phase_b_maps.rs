//! Phase B — valid-path differential tests, CONFIGS.md rows 9..22, 45, 46.
//! Exercises `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmput_default`, `stbds_shmode_func`, `stbds_hmfree_func` in every
//! `mode` x `string.mode` x element-shape combination the C distinguishes.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

// element shapes -------------------------------------------------------------
// `struct { int key; int value; }`
fn int_map() -> ElemDesc {
    ElemDesc::all_raw(8)
}
// `struct { int key[2]; int b,c,d; }` (stbds_struct2)
fn struct2_map() -> ElemDesc {
    ElemDesc::all_raw(20)
}
// `struct { char *key; int value; int pad; }` with a library-owned key pointer
fn str_map() -> ElemDesc {
    ElemDesc::ptr_key(16)
}
// same shape but the library memcpy'd raw bytes into the key field
fn str_map_raw() -> ElemDesc {
    ElemDesc::all_raw(16)
}

fn i32k(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn i64k(v: i64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// long enough that `hash_bytes(key, 8)` / `memcpy(elem, key, 8)` stay inside
/// the allocation (needed for the "mixed mode" configurations)
fn long_key(rng: &mut Rng, min: usize) -> Vec<u8> {
    let n = min + rng.below(24);
    rng.cstr_bytes(n, false)
}

// ---------------------------------------------------------------------------
// row 9 — BINARY, auto-created table (string.mode = NONE), int keys,
//         counts that cross every growth boundary
// ---------------------------------------------------------------------------
#[test]
fn cfg_09_binary_int_growth_boundaries() {
    for &count in &[0usize, 1, 2, 3, 4, 5, 6, 7, 11, 12, 13, 23, 24, 25, 48, 49] {
        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            let mut m = MapPair::new(int_map(), 4, &format!("bin-int count={count} seed={seed:#x}"));
            m.seed(seed);
            for i in 0..count {
                m.put_binary(&i32k(i as i32), &i32k(1000 + i as i32), HM_BINARY);
            }
            // every key must be found, at the same index in both
            for i in 0..count {
                let k = i32k(i as i32);
                let idx = m.get(k.as_ptr() as *mut c_void, 4, HM_BINARY);
                assert!(idx >= 0, "key {i} lost (count={count})");
            }
            // absent keys
            for i in count..count + 8 {
                let k = i32k(i as i32);
                assert_eq!(m.get(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
            }
            assert_eq!(unsafe { hmlen(m.ct, 8) }, count as isize);
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 10 — BINARY, `struct2` shape (elemsize 20 / keysize 8), random keys
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_binary_struct2_random() {
    let mut rng = Rng::new(0xB0_0010);
    for &count in &[1usize, 2, 7, 40, 300, 1000] {
        let mut m = MapPair::new(struct2_map(), 8, &format!("bin-struct2 count={count}"));
        m.seed(0x3141_5926);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..count {
            let k = i64k(rng.next_u64() as i64);
            let v: Vec<u8> = (0..12u8).map(|b| b.wrapping_add(i as u8)).collect();
            m.put_binary(&k, &v, HM_BINARY);
            keys.push(k);
        }
        for k in &keys {
            assert!(
                m.get(k.as_ptr() as *mut c_void, 8, HM_BINARY) >= 0,
                "struct2 key lost"
            );
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 11 — keysize == elemsize (no value area) and keysize < elemsize
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_keysize_shapes() {
    let mut rng = Rng::new(0xB0_0011);
    // keysize == elemsize == 8: the library memcpy covers the whole element
    {
        let mut m = MapPair::new(ElemDesc::all_raw(8), 8, "keysize==elemsize");
        m.seed(0x3141_5926);
        for _ in 0..200 {
            let k = i64k(rng.next_u64() as i64);
            m.put_binary(&k, &[], HM_BINARY);
        }
        m.free();
    }
    // keysize < elemsize, several widths
    for &(es, ks) in &[(4usize, 1usize), (8, 2), (12, 4), (16, 4), (24, 8), (32, 16)] {
        let mut m = MapPair::new(ElemDesc::all_raw(es), ks, &format!("es={es} ks={ks}"));
        m.seed(0x3141_5926);
        for i in 0..120 {
            let k = rng.bytes(ks);
            let v: Vec<u8> = (0..(es - ks)).map(|j| ((i * 7 + j) & 0xff) as u8).collect();
            m.put_binary(&k, &v, HM_BINARY);
        }
        m.free();
    }
    // keysize == 0: every key hashes identically and memcmp always matches
    {
        let mut m = MapPair::new(ElemDesc::all_raw(8), 0, "keysize==0");
        m.seed(0x3141_5926);
        for _ in 0..20 {
            let k: Vec<u8> = vec![];
            m.put_binary(&k, &i64k(7), HM_BINARY);
        }
        assert_eq!(unsafe { hmlen(m.ct, 8) }, 1, "keysize 0 => all keys equal");
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 12 — duplicate keys (both dup-found loops), value overwritten
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_binary_duplicates() {
    let mut rng = Rng::new(0xB0_0012);
    for &n in &[1usize, 3, 6, 7, 20, 200] {
        let mut m = MapPair::new(int_map(), 4, &format!("dup n={n}"));
        m.seed(0x3141_5926);
        let keys: Vec<i32> = (0..n as i32).collect();
        for pass in 0..4 {
            for &k in &keys {
                m.put_binary(&i32k(k), &i32k(k * 100 + pass), HM_BINARY);
            }
            assert_eq!(unsafe { hmlen(m.ct, 8) }, n as isize, "dup grew the map");
        }
        // random re-puts
        for _ in 0..500 {
            let k = keys[rng.below(keys.len())];
            m.put_binary(&i32k(k), &i32k(rng.next_u32() as i32), HM_BINARY);
        }
        assert_eq!(unsafe { hmlen(m.ct, 8) }, n as isize);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 13 — STRING mode, auto-created table => string.mode == SH_DEFAULT
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_string_default_autotable() {
    let mut rng = Rng::new(0xB0_0013);
    for &count in &[0usize, 1, 2, 6, 7, 13, 60, 500] {
        let mut m = MapPair::new(str_map(), 8, &format!("sh-default count={count}"));
        m.seed(0x3141_5926);
        let mut keys = Vec::new();
        for i in 0..count {
            let k = leak_cstr(&long_key(&mut rng, 1));
            let before = unsafe { hmlen(m.ct, 16) };
            m.put_string(k, &i64k(i as i64), HM_STRING);
            let after = unsafe { hmlen(m.ct, 16) };
            // `stbds_temp_key` must agree between the two libraries ...
            m.assert_temp_key_matches("cfg_13 put");
            let ctk = unsafe { temp_key_of(m.ct, 16) };
            let rtk = unsafe { temp_key_of(m.rt, 16) };
            // ... and for SH_DEFAULT it is a *caller* pointer, identical in
            // both. On a fresh insert it is this call's key; on a duplicate the
            // C code deliberately reports the already-stored pointer instead.
            assert_eq!(ctk, rtk, "SH_DEFAULT temp_key pointer must be identical");
            if after == before + 1 {
                assert_eq!(ctk, k, "fresh insert must report the caller's pointer");
            }
            keys.push(k);
        }
        // check string.mode really is SH_DEFAULT in both
        if count > 0 {
            unsafe {
                let ct = (*header_of(m.ct, 16)).hash_table as *const HashIndex;
                let rt = (*header_of(m.rt, 16)).hash_table as *const HashIndex;
                assert_eq!((*ct).string.mode, 1);
                assert_eq!((*rt).string.mode, 1);
            }
        }
        for &k in &keys {
            assert!(m.get(k as *mut c_void, 8, HM_STRING) >= 0, "string key lost");
        }
        let absent = leak_cstr(b"\x01absent-key-that-cannot-collide");
        assert_eq!(m.get(absent as *mut c_void, 8, HM_STRING), -1);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 14 — SH_STRDUP table (`stbds_sh_new_strdup`)
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_string_strdup() {
    let mut rng = Rng::new(0xB0_0014);
    for &count in &[0usize, 1, 2, 6, 7, 13, 60, 400] {
        let mut m = MapPair::new(str_map(), 8, &format!("sh-strdup count={count}"));
        m.seed(0x3141_5926);
        m.shmode(SH_STRDUP);
        let mut keys = Vec::new();
        for i in 0..count {
            let kb = long_key(&mut rng, 1);
            let k = leak_cstr(&kb);
            m.put_string(k, &i64k(i as i64), HM_STRING);
            m.assert_temp_key_matches("cfg_14 strdup put");
            // strdup => the stored pointer must NOT be the caller's
            unsafe {
                let stored = *(m.ct.add(16 * temp_of(m.ct, 16) as usize) as *const *const i8);
                assert_ne!(stored, k as *const i8, "SH_STRDUP must copy the key");
            }
            keys.push((k, kb));
        }
        for (k, _) in &keys {
            assert!(m.get(*k as *mut c_void, 8, HM_STRING) >= 0);
        }
        // duplicate puts must not allocate new elements
        let before = unsafe { hmlen(m.ct, 16) };
        for (k, _) in &keys {
            m.put_string(*k, &i64k(-1), HM_STRING);
        }
        assert_eq!(unsafe { hmlen(m.ct, 16) }, before);
        m.free(); // frees every strdup'd key in both libraries
        for (k, _) in keys {
            free_raw(k);
        }
    }
}

// ---------------------------------------------------------------------------
// row 15 — SH_ARENA table: keys allocated from the table's string arena,
//          including keys longer than the current arena block
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_string_arena() {
    let mut rng = Rng::new(0xB0_0015);
    for &(count, minlen) in &[
        (1usize, 1usize),
        (7, 1),
        (60, 1),
        (300, 1),
        (40, 400),  // some keys exceed the first 512-byte block
        (20, 700),  // every key exceeds the 512-byte block
        (12, 2000), // forces the dedicated-block path repeatedly
    ] {
        let mut m = MapPair::new(str_map(), 8, &format!("sh-arena count={count} min={minlen}"));
        m.seed(0x3141_5926);
        m.shmode(SH_ARENA);
        let mut keys = Vec::new();
        for i in 0..count {
            let kb = if minlen == 1 {
                long_key(&mut rng, 1)
            } else {
                { let n = minlen + rng.below(600); rng.cstr_bytes(n, false) }
            };
            let k = leak_cstr(&kb);
            m.put_string(k, &i64k(i as i64), HM_STRING);
            m.assert_temp_key_matches("cfg_15 arena put");
            keys.push(k);
        }
        for &k in &keys {
            assert!(m.get(k as *mut c_void, 8, HM_STRING) >= 0);
        }
        m.free();
        for k in keys {
            free_raw(k);
        }
    }
}

// ---------------------------------------------------------------------------
// row 16 — MIXED: STRING mode (mode>=1) on a SH_NONE table.
// The hash comes from `stbds_hash_string` but the store goes through the
// `default:` branch => `memcpy(elem, key, keysize)` copies raw string BYTES.
// Only *distinct* keys are inserted: a hash match would make
// `stbds_is_key_equal` reinterpret those bytes as a `char*` and dereference it
// (identical UB in both libraries, but not observable).
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_string_mode_on_none_table() {
    let mut rng = Rng::new(0xB0_0016);
    for &count in &[1usize, 2, 5, 6, 7, 20, 100] {
        for &mode in &[HM_STRING, 2, 7, 1000] {
            let mut m = MapPair::new(
                str_map_raw(),
                8,
                &format!("none-table string mode={mode} count={count}"),
            );
            m.seed(0x3141_5926);
            m.shmode(SH_NONE);
            for i in 0..count {
                // >= 8 bytes so the 8-byte memcpy / hash stays in bounds
                let kb = { let n = 8 + rng.below(20); rng.cstr_bytes(n, false) };
                let k = leak_cstr(&kb);
                m.put_raw_keysize(k as *mut _, 8, &i64k(i as i64), mode);
                free_raw(k);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 17 — MIXED: BINARY mode (mode=0) on a SH_DEFAULT table.
// `stbds_hash_bytes(key, 8)` hashes the first 8 bytes of the *string*, while
// the `SH_DEFAULT` branch stores the caller's `char*`, so `memcmp` compares
// string bytes against pointer bytes. Fully deterministic (the caller's
// pointer is shared by both libraries).
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_binary_mode_on_default_table() {
    let mut rng = Rng::new(0xB0_0017);
    for &count in &[1usize, 2, 5, 6, 20, 80] {
        for &mode in &[HM_BINARY, -1, i32::MIN] {
            let mut m = MapPair::new(
                str_map(),
                8,
                &format!("default-table binary mode={mode} count={count}"),
            );
            m.seed(0x3141_5926);
            m.shmode(SH_DEFAULT);
            let mut keys = Vec::new();
            for i in 0..count {
                let kb = { let n = 8 + rng.below(20); rng.cstr_bytes(n, false) };
                let k = leak_cstr(&kb);
                m.put_string(k, &i64k(i as i64), mode);
                keys.push(k);
            }
            for &k in &keys {
                m.get(k as *mut c_void, 8, mode);
                m.get_ts(k as *mut c_void, 8, mode);
            }
            m.free();
            for k in keys {
                free_raw(k);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 18 — out-of-range enum `mode` values that mean "STRING"
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_mode_out_of_range_string() {
    let mut rng = Rng::new(0xB0_0018);
    for &mode in &[1i32, 2, 3, 7, 1000, i32::MAX] {
        for &table_mode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let mut m = MapPair::new(str_map(), 8, &format!("mode={mode} table={table_mode}"));
            m.seed(0x3141_5926);
            m.shmode(table_mode);
            let mut keys = Vec::new();
            for i in 0..40 {
                let k = leak_cstr(&long_key(&mut rng, 1));
                m.put_string(k, &i64k(i), mode);
                m.assert_temp_key_matches("cfg_18 put");
                keys.push(k);
            }
            for &k in &keys {
                assert!(m.get(k as *mut c_void, 8, mode) >= 0);
                m.get_ts(k as *mut c_void, 8, mode);
            }
            // duplicates: `temp_key` is only rewritten when the key is found in
            // the FIRST probe loop (ERRORS.md row 24), so just require the two
            // libraries to agree.
            for &k in &keys {
                m.put_string(k, &i64k(-7), mode);
                m.assert_temp_key_matches("cfg_18 duplicate put");
            }
            m.free();
            for k in keys {
                free_raw(k);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 19 — out-of-range negative `mode` values that mean "BINARY"
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_mode_out_of_range_binary() {
    let mut rng = Rng::new(0xB0_0019);
    for &mode in &[0i32, -1, -2, -1000, i32::MIN] {
        let mut m = MapPair::new(int_map(), 4, &format!("binary mode={mode}"));
        m.seed(0x3141_5926);
        for i in 0..80 {
            m.put_binary(&i32k(i), &i32k(i * 3), mode);
        }
        // auto-created table must have string.mode == SH_NONE for mode <= 0
        unsafe {
            let ct = (*header_of(m.ct, 8)).hash_table as *const HashIndex;
            let rt = (*header_of(m.rt, 8)).hash_table as *const HashIndex;
            assert_eq!((*ct).string.mode, 0, "mode {mode} => SH_NONE");
            assert_eq!((*rt).string.mode, 0, "mode {mode} => SH_NONE");
        }
        for i in 0..80 {
            let k = i32k(i);
            assert!(m.get(k.as_ptr() as *mut c_void, 4, mode) >= 0);
        }
        for _ in 0..200 {
            let k = i32k(rng.next_u32() as i32);
            m.get_ts(k.as_ptr() as *mut c_void, 4, mode);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 20 — `stbds_hmget_key_ts` (lowest-level getter) in every state
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_hmget_key_ts_states() {
    let mut rng = Rng::new(0xB0_0020);

    // (a) NULL map: allocates and reports -1
    {
        let mut m = MapPair::new(int_map(), 4, "get_ts on NULL map");
        m.seed(0x3141_5926);
        let k = i32k(42);
        assert_eq!(m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
        assert!(!m.ct.is_null() && !m.rt.is_null());
        assert_eq!(unsafe { hmlen(m.ct, 8) }, 0);
        m.free();
    }
    // (b) map with no hash table (from hmput_default): reports -1
    {
        let mut m = MapPair::new(int_map(), 4, "get_ts, no table");
        m.seed(0x3141_5926);
        m.hmput_default_raw();
        let k = i32k(42);
        assert_eq!(m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
        m.free();
    }
    // (c) populated, present / absent, both modes
    {
        let mut m = MapPair::new(int_map(), 4, "get_ts populated");
        m.seed(0x3141_5926);
        for i in 0..200 {
            m.put_binary(&i32k(i), &i32k(i ^ 0x5a5a), HM_BINARY);
        }
        for i in 0..200 {
            let k = i32k(i);
            let idx = m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY);
            assert!(idx >= 0);
            // and the value really is ours, in both
            unsafe {
                let cv = *(m.ct.add(8 * idx as usize + 4) as *const i32);
                let rv = *(m.rt.add(8 * idx as usize + 4) as *const i32);
                assert_eq!(cv, i ^ 0x5a5a);
                assert_eq!(rv, i ^ 0x5a5a);
            }
        }
        for _ in 0..2000 {
            let k = i32k(rng.next_u32() as i32);
            m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 21 — `stbds_hmget_key` wrapper: copies temp into the array header
// ---------------------------------------------------------------------------
#[test]
fn cfg_21_hmget_key_wrapper() {
    let mut rng = Rng::new(0xB0_0021);
    // NULL map
    {
        let mut m = MapPair::new(int_map(), 4, "get on NULL map");
        m.seed(0x3141_5926);
        let k = i32k(1);
        assert_eq!(m.get(k.as_ptr() as *mut c_void, 4, HM_BINARY), -1);
        m.free();
    }
    // string map
    {
        let mut m = MapPair::new(str_map(), 8, "get on string map");
        m.seed(0x3141_5926);
        let mut keys = Vec::new();
        for i in 0..150 {
            let k = leak_cstr(&long_key(&mut rng, 1));
            m.put_string(k, &i64k(i), HM_STRING);
            keys.push(k);
        }
        for &k in &keys {
            assert!(m.get(k as *mut c_void, 8, HM_STRING) >= 0);
        }
        for _ in 0..500 {
            let k = leak_cstr(&long_key(&mut rng, 1));
            m.get(k as *mut c_void, 8, HM_STRING);
            free_raw(k);
        }
        m.free();
        for k in keys {
            free_raw(k);
        }
    }
}

// ---------------------------------------------------------------------------
// row 22 — `stbds_hmput_default` (hmdefault): NULL map, length==0, populated
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_hmput_default() {
    // (a) NULL map -> creates a 1-element table-less map; then set the default
    {
        let mut m = MapPair::new(int_map(), 4, "hmdefault from NULL");
        m.seed(0x3141_5926);
        m.hmput_default_raw();
        unsafe {
            // (t)[-1].value = 0x1234  (the macro writes the default element)
            *(m.ct.sub(8).add(4) as *mut i32) = 0x1234;
            *(m.rt.sub(8).add(4) as *mut i32) = 0x1234;
        }
        m.check("write default value");
        // repeated calls are no-ops now that length != 0
        for _ in 0..5 {
            m.hmput_default_raw();
        }
        // and a real put still works and preserves the default
        for i in 0..20 {
            m.put_binary(&i32k(i), &i32k(i + 1), HM_BINARY);
        }
        unsafe {
            assert_eq!(*(m.ct.sub(8).add(4) as *const i32), 0x1234);
            assert_eq!(*(m.rt.sub(8).add(4) as *const i32), 0x1234);
        }
        m.free();
    }
    // (b) hmdefault AFTER puts (length != 0): must be a pure no-op
    {
        let mut m = MapPair::new(int_map(), 4, "hmdefault after puts");
        m.seed(0x3141_5926);
        for i in 0..30 {
            m.put_binary(&i32k(i), &i32k(-i), HM_BINARY);
        }
        let (before_c, before_r) = m.snapshots();
        m.hmput_default_raw();
        let (after_c, after_r) = m.snapshots();
        assert_eq!(before_c, after_c, "hmput_default must be a no-op (C)");
        assert_eq!(before_r, after_r, "hmput_default must be a no-op (RUST)");
        m.free();
    }
    // (c) hmdefault on a hand-made length==0 array
    {
        let l = libs();
        for es in [8usize, 16, 20] {
            unsafe {
                // arrgrowf gives length == 0
                let ca = (l.c.arrgrowf)(std::ptr::null_mut(), es, 0, 1) as *mut u8;
                let ra = (l.r.arrgrowf)(std::ptr::null_mut(), es, 0, 1) as *mut u8;
                let ch = (l.c.hmput_default)(ca.add(es) as *mut c_void, es) as *mut u8;
                let rh = (l.r.hmput_default)(ra.add(es) as *mut c_void, es) as *mut u8;
                let d = ElemDesc::all_raw(es);
                assert_eq!(
                    snapshot_map(ch, &d),
                    snapshot_map(rh, &d),
                    "hmput_default on length==0 array, es={es}"
                );
                (l.c.hmfree_func)(ch.sub(es) as *mut c_void, es);
                (l.r.hmfree_func)(rh.sub(es) as *mut c_void, es);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 45 — hmfree over every table flavour, then reuse the handle
// ---------------------------------------------------------------------------
#[test]
fn cfg_45_hmfree_all_flavours() {
    let mut rng = Rng::new(0xB0_0045);
    for &table_mode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &count in &[0usize, 1, 2, 9, 60] {
            let mut m = MapPair::new(str_map(), 8, &format!("free table={table_mode} n={count}"));
            m.seed(0x3141_5926);
            m.shmode(table_mode);
            let mut keys = Vec::new();
            for i in 0..count {
                let kb = { let n = 8 + rng.below(30); rng.cstr_bytes(n, false) };
                let k = leak_cstr(&kb);
                if table_mode == SH_NONE {
                    // SH_NONE stores raw bytes; use the raw-byte descriptor
                    m.desc = str_map_raw();
                    m.put_raw_keysize(k as *mut _, 8, &i64k(i as i64), HM_STRING);
                    free_raw(k);
                } else {
                    m.put_string(k, &i64k(i as i64), HM_STRING);
                    keys.push(k);
                }
            }
            m.free();
            // reuse from scratch after the free
            m.desc = str_map();
            for i in 0..5 {
                let k = leak_cstr(&long_key(&mut rng, 1));
                m.put_string(k, &i64k(i), HM_STRING);
                keys.push(k);
            }
            m.free();
            for k in keys {
                free_raw(k);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 46 — probe-collision shapes: dense 8-slot table forcing the
//          `0 .. limit` wrap loop, and multi-bucket `pos += step` probing
// ---------------------------------------------------------------------------
#[test]
fn cfg_46_probe_shapes() {
    let l = libs();
    // 8 slots hold at most 6 entries. Fill to 5, then probe for many absent
    // keys so every possible `pos & MASK` start offset is exercised, then add
    // the 6th and the 7th (which grows to 16 slots).
    for &seed in &[0usize, 1, 0x3141_5926, 0xffff_ffff_ffff_ffff] {
        let mut m = MapPair::new(int_map(), 4, &format!("probe seed={seed:#x}"));
        m.seed(seed);
        for i in 0..5i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        // 4096 lookups: guaranteed to start at every slot offset in the bucket
        for i in 0..4096i32 {
            let k = i32k(100_000 + i);
            m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY);
        }
        for i in 5..40i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
            for j in 0..64i32 {
                let k = i32k(500_000 + j);
                m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY);
            }
        }
        m.free();
    }
    // hand-built collisions: find keys whose hash lands in the same bucket of
    // a 16-slot table, then insert them all so `pos += step` must run.
    unsafe {
        (l.c.rand_seed)(0x3141_5926);
        (l.r.rand_seed)(0x3141_5926);
        // discover the seed a fresh table will use
        let probe = (l.c.shmode_func)(8, SH_NONE) as *mut u8;
        let tbl_seed = (*((*header_of(probe, 8)).hash_table as *const HashIndex)).seed;
        (l.c.hmfree_func)(probe.sub(8) as *mut c_void, 8);
        (l.c.rand_seed)(0x3141_5926);

        let mut same_bucket: Vec<i32> = Vec::new();
        let mut i = 0i32;
        while same_bucket.len() < 12 && i < 2_000_000 {
            let k = i32k(i);
            let mut h = (l.c.hash_bytes)(k.as_ptr() as *mut c_void, 4, tbl_seed);
            if h < 2 {
                h += 2;
            }
            // slot_count 64 => bucket index (h & 63) >> 3
            if ((h & 63) >> 3) == 0 {
                same_bucket.push(i);
            }
            i += 1;
        }
        assert!(same_bucket.len() >= 12, "could not build collisions");
        let mut m = MapPair::new(int_map(), 4, "same-bucket keys");
        m.seed(0x3141_5926);
        for &k in &same_bucket {
            m.put_binary(&i32k(k), &i32k(k), HM_BINARY);
        }
        for &k in &same_bucket {
            assert!(m.get(i32k(k).as_ptr() as *mut c_void, 4, HM_BINARY) >= 0);
        }
        for j in 0..2000i32 {
            let k = i32k(-j - 1);
            m.get_ts(k.as_ptr() as *mut c_void, 4, HM_BINARY);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 46 (extended) — force the MULTI-BUCKET probe walk inside the rehash loop
// of `stbds_make_hash_index`.
//
// `pos += step; step += STBDS_BUCKET_LENGTH; pos &= slot_count-1` only runs when
// a whole destination bucket is full, and it only takes its *second* step when
// two buckets in a row are full. Ordinary key distributions essentially never
// do that (the load factor is capped at 3/4). So: hand-pick >= 24 keys that all
// hash into bucket 0 of a 64-slot table, insert them, and let the table grow
// 8 -> 16 -> 32 -> 64. The grow to 64 then rehashes 24 colliding entries and
// must spill bucket 0 -> bucket 1 -> bucket 3.
// ---------------------------------------------------------------------------
#[test]
fn cfg_46b_rehash_multibucket_spill() {
    let l = libs();
    for &seed in &[0usize, 1, 0x3141_5926, 0xfeed_face_dead_beef] {
        // The first table created after `rand_seed(seed)` uses `seed` verbatim,
        // and every grow/shrink/rebuild inherits it, so one hash per key suffices.
        let mut colliding: Vec<i32> = Vec::new();
        let mut i = 0i32;
        unsafe {
            while colliding.len() < 40 && i < 5_000_000 {
                let k = i32k(i);
                let mut h = (l.c.hash_bytes)(k.as_ptr() as *mut c_void, 4, seed);
                if h < 2 {
                    h += 2;
                }
                if ((h & 63) >> 3) == 0 {
                    colliding.push(i);
                }
                i += 1;
            }
        }
        assert!(
            colliding.len() >= 40,
            "could not find 40 keys colliding in bucket 0 of a 64-slot table"
        );

        let mut m = MapPair::new(int_map(), 4, &format!("rehash-spill seed={seed:#x}"));
        m.seed(seed);
        let mut saw_double_spill = false;
        let mut prev_slots = 0usize;
        for &k in colliding.iter() {
            m.put_binary(&i32k(k), &i32k(k ^ 0x5eed), HM_BINARY);
            unsafe {
                let ti = *((*header_of(m.ct, 8)).hash_table as *const HashIndex);
                if ti.slot_count != prev_slots {
                    prev_slots = ti.slot_count;
                    if ti.slot_count >= 64 {
                        let used: Vec<usize> = (0..(ti.slot_count >> 3))
                            .map(|b| {
                                let bk = *ti.storage.add(b);
                                bk.index.iter().filter(|&&x| x >= 0).count()
                            })
                            .collect();
                        // Two consecutive FULL buckets is exactly what makes the
                        // rehash loop take its SECOND `pos += step` advance.
                        if used[0] == 8 && used[1] == 8 && used.iter().skip(2).any(|&u| u > 0) {
                            saw_double_spill = true;
                        }
                    }
                }
            }
        }
        assert!(
            saw_double_spill,
            "expected a rehash into >= 64 slots with two consecutive full \
             buckets (that is what exercises the multi-bucket `pos += step` walk)"
        );
        // every key must still be findable, and lookups must walk the same chain
        for &k in &colliding {
            let kb = i32k(k);
            assert!(m.get(kb.as_ptr() as *mut c_void, 4, HM_BINARY) >= 0, "key {k} lost");
            m.get_ts(kb.as_ptr() as *mut c_void, 4, HM_BINARY);
        }
        // absent lookups also traverse the long chain
        for j in 0..2000i32 {
            let kb = i32k(-j - 1);
            m.get_ts(kb.as_ptr() as *mut c_void, 4, HM_BINARY);
        }
        // now delete them all, exercising rehash/shrink with the same collisions
        let mut live = colliding.clone();
        while !live.is_empty() {
            let k = live.pop().unwrap();
            let kb = i32k(k);
            assert_eq!(m.del(kb.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
        }
        m.free();
    }
}

// unused-import silencer
const _: c_int = 0;
