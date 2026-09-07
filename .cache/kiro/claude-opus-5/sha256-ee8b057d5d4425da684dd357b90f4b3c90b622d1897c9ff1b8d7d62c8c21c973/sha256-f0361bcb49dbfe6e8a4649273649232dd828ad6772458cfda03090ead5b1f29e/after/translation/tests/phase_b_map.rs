//! Phase B — valid-path differential tests for the hash-map entry points.
//!
//! Covers CONFIGS.md rows C21–C53 and C64–C65. Every scenario drives the
//! low-level `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmget_key_ts` /
//! `stbds_hmdel_key` / `stbds_shmode_func` / `stbds_hmfree_func` exports
//! directly (there are no convenience wrappers in the `.so`), reproducing what
//! the `stbds_hm*` macros expand to, and compares the complete map state —
//! array header, every hash-index field, every bucket slot, and every element's
//! key and value bytes — after every single call.

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Deterministic binary keys of `keysize` bytes derived from an index.
fn bin_key(i: u64, keysize: usize) -> Vec<u8> {
    let mut r = Rng::new(0x1000_0000 ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut v = r.bytes(keysize);
    // Bake the index into the low bytes so distinct `i` give distinct keys even
    // for keysize 1.
    for (k, b) in v.iter_mut().enumerate() {
        if k < 8 {
            *b = ((i >> (8 * k)) & 0xff) as u8;
        }
    }
    v
}

fn val_for(i: u64, valsize: usize) -> Vec<u8> {
    let mut r = Rng::new(0x2000_0000 ^ i.wrapping_mul(0xD1B5_4A32_D192_ED03));
    r.bytes(valsize)
}

/// Stable storage for NUL-terminated string keys. `STBDS_SH_DEFAULT` stores the
/// caller's pointer verbatim, so these must outlive the map.
struct StrKeys {
    keys: Vec<Vec<u8>>,
}

impl StrKeys {
    fn new(n: usize, seed: u64, style: u8) -> Self {
        let mut rng = Rng::new(seed);
        let mut keys = Vec::with_capacity(n);
        for i in 0..n {
            let mut k = match style {
                // `strkey`-like, short and highly similar.
                0 => format!("test_{i}").into_bytes(),
                // Random ASCII of varying length (drives the arena hard).
                1 => {
                    let l = rng.below(70);
                    let mut v = rng.ascii(l);
                    v.pop();
                    v.extend_from_slice(format!("#{i}").as_bytes());
                    v
                }
                // Long keys spanning many siphash blocks / arena blocks.
                _ => {
                    let l = 100 + rng.below(500);
                    let mut v = rng.ascii(l);
                    v.pop();
                    v.extend_from_slice(format!("#{i}").as_bytes());
                    v
                }
            };
            k.push(0);
            keys.push(k);
        }
        StrKeys { keys }
    }
    fn absent(&self, i: usize) -> Vec<u8> {
        let mut v = format!("ABSENT_{i}").into_bytes();
        v.push(0);
        v
    }
}

// ===========================================================================
// C21–C27: BINARY-mode stbds_hmput_key
// ===========================================================================

#[test]
fn c21_binary_single_key() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 8, 4, 4, KeyRepr::Bytes, STBDS_HM_BINARY);
    let mut k = bin_key(0, 4);
    m.put(&mut k, &val_for(0, 4), "C21");
    let idx = m.geti(&mut k, "C21 lookup");
    assert_eq!(idx, 0, "C21 first key must land at index 0");
    let (cv, rv) = m.value_at(idx);
    diff_eq!("C21 value", cv, rv);
    m.free();
}

fn binary_n_keys(b: &Both, n: u64, elemsize: usize, keysize: usize, valsize: usize, tag: &str) {
    let mut m = MapPair::new(b, elemsize, keysize, valsize, KeyRepr::Bytes, STBDS_HM_BINARY);
    let keys: Vec<Vec<u8>> = (0..n).map(|i| bin_key(i, keysize)).collect();
    for i in 0..n {
        let mut k = keys[i as usize].clone();
        m.put(&mut k, &val_for(i, valsize), &format!("{tag} put#{i}"));
    }
    // Every key must be found, at the same index, with the same value.
    for i in 0..n {
        let mut k = keys[i as usize].clone();
        let idx = m.geti(&mut k, &format!("{tag} get#{i}"));
        assert!(idx >= 0, "{tag}: key {i} unexpectedly missing (idx {idx})");
        let (cv, rv) = m.value_at(idx);
        diff_eq!(format!("{tag} value#{i}"), cv, rv);
    }
    // Absent keys must miss identically.
    for i in n..(n + 20) {
        let mut k = bin_key(i ^ 0xDEAD_0000, keysize);
        m.geti_ts(&mut k, &format!("{tag} absent#{i}"));
    }
    m.free();
}

#[test]
fn c22_binary_six_keys_no_grow() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    binary_n_keys(&b, 6, 8, 4, 4, "C22");
}

#[test]
fn c23_binary_seven_keys_first_grow() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    for n in [6u64, 7, 8, 9, 12, 13] {
        binary_n_keys(&b, n, 8, 4, 4, &format!("C23 n={n}"));
    }
}

#[test]
fn c24_binary_many_keys_multiple_grows() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // 200 keys forces 8 -> 16 -> 32 -> 64 -> 128 -> 256.
    binary_n_keys(&b, 200, 16, 8, 8, "C24");
    // And a much larger table to be sure the rehash loop's wrapped scan is hit.
    binary_n_keys(&b, 1000, 16, 8, 8, "C24 big");
}

#[test]
fn c25_binary_duplicate_keys() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut rng = Rng::new(TEST_SEED ^ 25);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    let keys: Vec<Vec<u8>> = (0..120u64).map(|i| bin_key(i, 8)).collect();
    let mut inserted = 0u64;
    for step in 0..900 {
        // 50/50 new key vs. re-put of an existing one (both found paths).
        let pick = if inserted == 0 || rng.below(2) == 0 {
            let p = inserted.min(119);
            if inserted < 120 {
                inserted += 1;
            }
            p
        } else {
            rng.below(inserted as usize) as u64
        };
        let mut k = keys[pick as usize].clone();
        m.put(
            &mut k,
            &val_for(pick ^ step as u64, 8),
            &format!("C25 step={step} key={pick}"),
        );
    }
    for i in 0..inserted {
        let mut k = keys[i as usize].clone();
        let idx = m.geti(&mut k, &format!("C25 final get#{i}"));
        assert!(idx >= 0);
        let (cv, rv) = m.value_at(idx);
        diff_eq!(format!("C25 final value#{i}"), cv, rv);
    }
    m.free();
}

#[test]
fn c26_binary_elemsize_keysize_matrix() {
    let b = libs();
    for elemsize in [4usize, 8, 12, 16, 24, 32] {
        for keysize in [1usize, 2, 4, 8, 16] {
            if keysize >= elemsize {
                continue;
            }
            let valsize = (elemsize - keysize).min(4);
            reset_seed(&b, 0x3141_5926);
            binary_n_keys(
                &b,
                40,
                elemsize,
                keysize,
                valsize,
                &format!("C26 es={elemsize} ks={keysize}"),
            );
        }
    }
}

#[test]
fn c27_binary_keysize_zero() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // keysize 0: `stbds_hash_bytes(key, 0, seed)` gives the same hash for every
    // key and `memcmp(.., 0) == 0` always matches, so the very first insert
    // "already exists" from the 2nd put onwards.
    let mut m = MapPair::new(&b, 8, 0, 4, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..30u64 {
        let mut k = bin_key(i, 1);
        m.put(&mut k, &val_for(i, 4), &format!("C27 put#{i}"));
    }
    for i in 0..10u64 {
        let mut k = bin_key(i, 1);
        m.geti(&mut k, &format!("C27 get#{i}"));
        m.geti_ts(&mut k, &format!("C27 get_ts#{i}"));
    }
    m.free();
}

// ===========================================================================
// C28–C35: STRING modes and out-of-range `mode` / `sh_mode`
// ===========================================================================

fn string_map_run(
    b: &Both,
    sh_mode: Option<c_int>,
    mode: c_int,
    n: usize,
    style: u8,
    tag: &str,
) {
    reset_seed(b, 0x3141_5926);
    let sk = StrKeys::new(n, TEST_SEED ^ (n as u64) ^ (style as u64) << 8, style);
    // IMPORTANT: `STBDS_SH_DEFAULT` stores the caller's `char *` verbatim, so
    // the key buffers must stay alive (and at a fixed address) for the whole
    // lifetime of the map.
    let mut owned = sk.keys.clone();
    // For `mode >= STBDS_HM_STRING` the element's key is a `char *`.
    let repr = if mode >= STBDS_HM_STRING {
        KeyRepr::Ptr
    } else {
        KeyRepr::Bytes
    };
    let (keysize, valsize) = (8usize, 8usize);
    let mut m = MapPair::new(b, 16, keysize, valsize, repr, mode);
    if let Some(sm) = sh_mode {
        m = m.with_shmode(sm);
    }
    for i in 0..n {
        let kp: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len()) };
        let v = val_for(i as u64, valsize);
        m.put(kp, &v, &format!("{tag} put#{i}"));
    }
    for i in 0..n {
        let kp: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len()) };
        let idx = m.geti(kp, &format!("{tag} get#{i}"));
        let (cv, rv) = m.value_at(idx);
        diff_eq!(format!("{tag} value#{i}"), cv, rv);
        m.geti_ts(kp, &format!("{tag} get_ts#{i}"));
    }
    for i in 0..10 {
        let mut k = sk.absent(i);
        m.geti(&mut k, &format!("{tag} absent#{i}"));
        m.geti_ts(&mut k, &format!("{tag} absent_ts#{i}"));
    }
    m.free();
    drop(owned);
}

#[test]
fn c28_string_implicit_sh_default() {
    let b = libs();
    for n in [1usize, 6, 7, 8, 60, 200] {
        for style in [0u8, 1, 2] {
            string_map_run(
                &b,
                None,
                STBDS_HM_STRING,
                n,
                style,
                &format!("C28 n={n} style={style}"),
            );
        }
    }
}

#[test]
fn c29_string_out_of_range_positive_mode() {
    let b = libs();
    // Any `mode >= STBDS_HM_STRING` is a string mode as far as hashing and key
    // comparison are concerned.
    for mode in [2i32, 3, 7, 1000, c_int::MAX] {
        for n in [1usize, 7, 60] {
            string_map_run(&b, None, mode, n, 0, &format!("C29 mode={mode} n={n}"));
        }
    }
}

#[test]
fn c30_string_negative_mode_is_binary() {
    let b = libs();
    // `mode < STBDS_HM_STRING` -> BINARY: keys are memcmp'd/memcpy'd inline.
    for mode in [-1i32, -2, c_int::MIN] {
        for n in [1usize, 7, 60] {
            string_map_run(&b, None, mode, n, 0, &format!("C30 mode={mode} n={n}"));
        }
    }
}

#[test]
fn c31_sh_strdup() {
    let b = libs();
    for n in [1usize, 6, 7, 60, 200] {
        for style in [0u8, 1, 2] {
            string_map_run(
                &b,
                Some(STBDS_SH_STRDUP),
                STBDS_HM_STRING,
                n,
                style,
                &format!("C31 n={n} style={style}"),
            );
        }
    }
}

#[test]
fn c32_sh_arena() {
    let b = libs();
    for n in [1usize, 6, 7, 60, 200] {
        for style in [0u8, 1, 2] {
            string_map_run(
                &b,
                Some(STBDS_SH_ARENA),
                STBDS_HM_STRING,
                n,
                style,
                &format!("C32 n={n} style={style}"),
            );
        }
    }
}

#[test]
fn c33_sh_none_with_string_mode() {
    let b = libs();
    // `string.mode == STBDS_SH_NONE` takes the `switch` `default:` branch, i.e.
    // `memcpy(elem, key, keysize)` — an inline byte copy even though `mode`
    // says STRING (so lookups then `strcmp` against those inline bytes).
    for n in [1usize, 4] {
        reset_seed(&b, 0x3141_5926);
        let sk = StrKeys::new(n, TEST_SEED ^ 33, 0);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_STRING)
            .with_shmode(STBDS_SH_NONE);
        for i in 0..n {
            let mut k = sk.keys[i].clone();
            m.put(&mut k, &val_for(i as u64, 8), &format!("C33 n={n} put#{i}"));
        }
        m.free();
    }
}

#[test]
fn c34_sh_default_with_binary_mode() {
    let b = libs();
    // `string.mode == STBDS_SH_DEFAULT` stores the caller's pointer, but
    // `mode == STBDS_HM_BINARY` hashes and compares the raw key bytes.
    reset_seed(&b, 0x3141_5926);
    let sk = StrKeys::new(40, TEST_SEED ^ 34, 0);
    let mut m =
        MapPair::new(&b, 16, 8, 8, KeyRepr::Ptr, STBDS_HM_BINARY).with_shmode(STBDS_SH_DEFAULT);
    let mut owned: Vec<Vec<u8>> = sk.keys.clone();
    for i in 0..40 {
        let k = &mut owned[i];
        let v = val_for(i as u64, 8);
        // Safety: the map stores this exact pointer; `owned` outlives the map.
        let kp: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(k.as_mut_ptr(), k.len()) };
        m.put(kp, &v, &format!("C34 put#{i}"));
    }
    for i in 0..40 {
        let k = &mut owned[i];
        let kp: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(k.as_mut_ptr(), k.len()) };
        m.geti(kp, &format!("C34 get#{i}"));
    }
    m.free();
    drop(owned);
}

#[test]
fn c35_shmode_func_out_of_range() {
    let b = libs();
    // `stbds_shmode_func` truncates `mode` to `unsigned char`.
    for sh in [-1i32, 0, 1, 2, 3, 4, 5, 255, 256, 257, 1000, c_int::MAX, c_int::MIN] {
        for hm_mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2] {
            reset_seed(&b, 0x3141_5926);
            unsafe {
                let cm = (b.c.shmode_func)(16, sh);
                let rm = (b.r.shmode_func)(16, sh);
                diff_eq!(
                    format!("C35 sh={sh} table"),
                    snap_map(cm, 16, KeyRepr::Bytes, 8, 0),
                    snap_map(rm, 16, KeyRepr::Bytes, 8, 0)
                );
                // The truncated `string.mode` decides the `switch` branch taken
                // by the next `stbds_hmput_key`.
                let effective = (sh as u32 & 0xff) as u8;
                let repr = if effective == 1 || effective == 2 || effective == 3 {
                    KeyRepr::Ptr
                } else {
                    KeyRepr::Bytes
                };
                let sk = StrKeys::new(5, TEST_SEED ^ 35, 0);
                // Keys must outlive the map: `STBDS_SH_DEFAULT` (and any
                // truncated `sh` that lands on 1) stores the pointer verbatim.
                let mut owned = sk.keys.clone();
                let mut c = cm;
                let mut r = rm;
                for i in 0..5 {
                    let kp = owned[i].as_mut_ptr() as *mut c_void;
                    c = (b.c.hmput_key)(c, 16, kp, 8, hm_mode);
                    r = (b.r.hmput_key)(r, 16, kp, 8, hm_mode);
                    diff_eq!(
                        format!("C35 sh={sh} hm={hm_mode} put#{i}"),
                        snap_map(c, 16, repr, 8, 0),
                        snap_map(r, 16, repr, 8, 0)
                    );
                }
                (b.c.hmfree_func)((c as *mut u8).wrapping_sub(16) as *mut c_void, 16);
                (b.r.hmfree_func)((r as *mut u8).wrapping_sub(16) as *mut c_void, 16);
                drop(owned);
            }
        }
    }
}

// ===========================================================================
// C36–C39: stbds_hmget_key / stbds_hmget_key_ts
// ===========================================================================

#[test]
fn c36_hmget_hit_and_miss_positions() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // NULL map.
    unsafe {
        let mut ct: isize = 0x5a;
        let mut rt: isize = 0x5a;
        let mut k = bin_key(1, 8);
        let kp = k.as_mut_ptr() as *mut c_void;
        let c = (b.c.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut ct, STBDS_HM_BINARY);
        let r = (b.r.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut rt, STBDS_HM_BINARY);
        diff_eq!("C36 null temp", ct, rt);
        diff_eq!(
            "C36 null state",
            snap_map(c, 16, KeyRepr::Bytes, 8, 0),
            snap_map(r, 16, KeyRepr::Bytes, 8, 0)
        );
        (b.c.hmfree_func)((c as *mut u8).wrapping_sub(16) as *mut c_void, 16);
        (b.r.hmfree_func)((r as *mut u8).wrapping_sub(16) as *mut c_void, 16);
    }
    // Un-hashed map (from hmput_default only).
    unsafe {
        let mut ct: isize = 0x5a;
        let mut rt: isize = 0x5a;
        let mut k = bin_key(1, 8);
        let kp = k.as_mut_ptr() as *mut c_void;
        let c0 = (b.c.hmput_default)(std::ptr::null_mut(), 16);
        let r0 = (b.r.hmput_default)(std::ptr::null_mut(), 16);
        let c = (b.c.hmget_key_ts)(c0, 16, kp, 8, &mut ct, STBDS_HM_BINARY);
        let r = (b.r.hmget_key_ts)(r0, 16, kp, 8, &mut rt, STBDS_HM_BINARY);
        diff_eq!("C36 unhashed temp", ct, rt);
        diff_eq!(
            "C36 unhashed state",
            snap_map(c, 16, KeyRepr::Bytes, 8, 0),
            snap_map(r, 16, KeyRepr::Bytes, 8, 0)
        );
        (b.c.hmfree_func)((c as *mut u8).wrapping_sub(16) as *mut c_void, 16);
        (b.r.hmfree_func)((r as *mut u8).wrapping_sub(16) as *mut c_void, 16);
    }
    // Populated map: first / middle / last key plus misses.
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    let n = 40u64;
    for i in 0..n {
        let mut k = bin_key(i, 8);
        m.put(&mut k, &val_for(i, 8), &format!("C36 put#{i}"));
    }
    for i in [0u64, 1, n / 2, n - 2, n - 1] {
        let mut k = bin_key(i, 8);
        m.geti(&mut k, &format!("C36 hit#{i}"));
        m.geti_ts(&mut k, &format!("C36 hit_ts#{i}"));
    }
    m.free();
}

#[test]
fn c37_binary_full_lookup_sweep() {
    let b = libs();
    for &n in &[1u64, 7, 33, 100, 300] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..n {
            let mut k = bin_key(i, 8);
            m.put(&mut k, &val_for(i, 8), &format!("C37 n={n} put#{i}"));
        }
        for i in 0..n {
            let mut k = bin_key(i, 8);
            let idx = m.geti(&mut k, &format!("C37 n={n} get#{i}"));
            assert!(idx >= 0, "C37 n={n}: key {i} missing");
            let (cv, rv) = m.value_at(idx);
            diff_eq!(format!("C37 n={n} val#{i}"), cv, rv);
            let ts = m.geti_ts(&mut k, &format!("C37 n={n} get_ts#{i}"));
            assert_eq!(idx, ts, "C37: hmget_key and hmget_key_ts disagree");
        }
        for i in 0..n {
            let mut k = bin_key(i ^ 0xABCD_0000_0000, 8);
            let idx = m.geti_ts(&mut k, &format!("C37 n={n} miss#{i}"));
            assert_eq!(idx, -1, "C37: absent key reported as present");
        }
        m.free();
    }
}

#[test]
fn c38_string_full_lookup_sweep() {
    let b = libs();
    for sh in [None, Some(STBDS_SH_DEFAULT), Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)] {
        for &n in &[1usize, 7, 60, 150] {
            string_map_run(
                &b,
                sh,
                STBDS_HM_STRING,
                n,
                1,
                &format!("C38 sh={sh:?} n={n}"),
            );
        }
    }
}

#[test]
fn c39_hmget_key_writes_header_temp() {
    let b = libs();
    // The non-`_ts` wrapper must write `stbds_temp(raw_a)`; `MapPair::geti`
    // reads exactly that field, and `check` compares it, so this sweeps it
    // across all four string modes plus binary.
    for sh in [None, Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)] {
        string_map_run(&b, sh, STBDS_HM_STRING, 80, 0, &format!("C39 sh={sh:?}"));
    }
    reset_seed(&b, 0x3141_5926);
    binary_n_keys(&b, 80, 16, 8, 8, "C39 binary");
}

// ===========================================================================
// C40–C51: stbds_hmdel_key
// ===========================================================================

#[test]
fn c40_del_only_element() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    let mut k = bin_key(0, 8);
    m.put(&mut k, &val_for(0, 8), "C40 put");
    let t = m.del(&mut k, 0, "C40 del");
    assert_eq!(t, 1, "C40: delete of a present key must report 1");
    let t2 = m.del(&mut k, 0, "C40 del again");
    assert_eq!(t2, 0, "C40: second delete must report 0");
    m.free();
}

#[test]
fn c41_del_last_element() {
    let b = libs();
    for n in [2u64, 3, 7, 8, 40] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..n {
            let mut k = bin_key(i, 8);
            m.put(&mut k, &val_for(i, 8), &format!("C41 n={n} put#{i}"));
        }
        // The element at array index `length-1` is the most recently inserted,
        // i.e. `old_index == final_index` -> no move.
        let mut k = bin_key(n - 1, 8);
        m.del(&mut k, 0, &format!("C41 n={n} del last"));
        for i in 0..(n - 1) {
            let mut kk = bin_key(i, 8);
            let idx = m.geti(&mut kk, &format!("C41 n={n} survivor#{i}"));
            assert!(idx >= 0, "C41: survivor {i} lost");
            let (cv, rv) = m.value_at(idx);
            diff_eq!(format!("C41 n={n} survivor val#{i}"), cv, rv);
        }
        m.free();
    }
}

#[test]
fn c42_del_non_last_element() {
    let b = libs();
    for n in [2u64, 3, 7, 8, 40, 120] {
        for target in [0u64, 1, n / 2] {
            if target >= n - 1 {
                continue;
            }
            reset_seed(&b, 0x3141_5926);
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
            for i in 0..n {
                let mut k = bin_key(i, 8);
                m.put(&mut k, &val_for(i, 8), &format!("C42 n={n} put#{i}"));
            }
            let mut k = bin_key(target, 8);
            let t = m.del(&mut k, 0, &format!("C42 n={n} del#{target}"));
            assert_eq!(t, 1);
            // The last element was moved into the hole; its slot index must
            // have been re-pointed, so every survivor is still findable with
            // its own value.
            for i in 0..n {
                if i == target {
                    continue;
                }
                let mut kk = bin_key(i, 8);
                let idx = m.geti(&mut kk, &format!("C42 n={n} t={target} survivor#{i}"));
                assert!(idx >= 0, "C42: survivor {i} lost after deleting {target}");
                let (cv, rv) = m.value_at(idx);
                diff_eq!(format!("C42 n={n} t={target} val#{i}"), &cv, &rv);
                assert_eq!(cv, val_for(i, 8), "C42: value/key association broken");
            }
            m.free();
        }
    }
}

#[test]
fn c43_del_all_forward_and_reverse() {
    let b = libs();
    for n in [1u64, 7, 8, 9, 40, 130] {
        for reverse in [false, true] {
            reset_seed(&b, 0x3141_5926);
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
            for i in 0..n {
                let mut k = bin_key(i, 8);
                m.put(&mut k, &val_for(i, 8), &format!("C43 n={n} put#{i}"));
            }
            let order: Vec<u64> = if reverse {
                (0..n).rev().collect()
            } else {
                (0..n).collect()
            };
            for (step, i) in order.iter().enumerate() {
                let mut k = bin_key(*i, 8);
                let t = m.del(&mut k, 0, &format!("C43 n={n} rev={reverse} del#{i}"));
                assert_eq!(t, 1, "C43: delete of present key {i} reported {t}");
                // Everything not yet deleted must still be intact.
                for j in order.iter().skip(step + 1) {
                    let mut kk = bin_key(*j, 8);
                    let idx = m.geti(&mut kk, &format!("C43 n={n} rev={reverse} chk#{j}"));
                    assert!(idx >= 0, "C43: key {j} lost");
                    let (cv, rv) = m.value_at(idx);
                    diff_eq!(format!("C43 val#{j}"), &cv, &rv);
                    assert_eq!(cv, val_for(*j, 8));
                }
            }
            m.free();
        }
    }
}

#[test]
fn c44_random_put_del_mix() {
    let b = libs();
    for trial in 0..6u64 {
        reset_seed(&b, 0x3141_5926);
        let mut rng = Rng::new(TEST_SEED ^ 44 ^ trial);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        let universe = 250u64;
        let mut present = vec![false; universe as usize];
        for step in 0..1200 {
            let i = rng.below(universe as usize) as u64;
            let mut k = bin_key(i, 8);
            if rng.below(100) < 60 {
                m.put(
                    &mut k,
                    &val_for(i, 8),
                    &format!("C44 t={trial} s={step} put#{i}"),
                );
                present[i as usize] = true;
            } else {
                let t = m.del(&mut k, 0, &format!("C44 t={trial} s={step} del#{i}"));
                let expect = if present[i as usize] { 1 } else { 0 };
                assert_eq!(t, expect, "C44: del#{i} reported {t}, expected {expect}");
                present[i as usize] = false;
            }
        }
        // Full consistency sweep.
        for i in 0..universe {
            let mut k = bin_key(i, 8);
            let idx = m.geti_ts(&mut k, &format!("C44 t={trial} final#{i}"));
            if present[i as usize] {
                assert!(idx >= 0, "C44: key {i} should be present");
                let (cv, rv) = m.value_at(idx);
                diff_eq!(format!("C44 t={trial} val#{i}"), &cv, &rv);
                assert_eq!(cv, val_for(i, 8));
            } else {
                assert_eq!(idx, -1, "C44: key {i} should be absent");
            }
        }
        m.free();
    }
}

#[test]
fn c45_shrink_threshold() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // Fill past 48 entries so `slot_count` reaches 64+, then delete down past
    // `used_count_shrink_threshold` (slot_count>>2) to force the shrink path.
    let n = 100u64;
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..n {
        let mut k = bin_key(i, 8);
        m.put(&mut k, &val_for(i, 8), &format!("C45 put#{i}"));
    }
    // Deleting one at a time crosses several shrink thresholds
    // (256 -> 128 -> 64 -> 32 -> 16 -> 8).
    for i in 0..n {
        let mut k = bin_key(i, 8);
        m.del(&mut k, 0, &format!("C45 del#{i}"));
    }
    m.free();
}

#[test]
fn c46_tombstone_rebuild() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // Repeated delete+reinsert of the same working set keeps `used_count` above
    // the shrink threshold while accumulating tombstones past
    // `tombstone_count_threshold` = (slot_count>>3)+(slot_count>>4).
    let live = 30u64;
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..live {
        let mut k = bin_key(i, 8);
        m.put(&mut k, &val_for(i, 8), &format!("C46 seed#{i}"));
    }
    for round in 0..40u64 {
        let victim = 1000 + round;
        let mut k = bin_key(victim, 8);
        m.put(&mut k, &val_for(victim, 8), &format!("C46 r={round} put"));
        m.del(&mut k, 0, &format!("C46 r={round} del"));
    }
    for i in 0..live {
        let mut k = bin_key(i, 8);
        let idx = m.geti(&mut k, &format!("C46 survivor#{i}"));
        assert!(idx >= 0, "C46: live key {i} lost");
        let (cv, rv) = m.value_at(idx);
        diff_eq!(format!("C46 val#{i}"), cv, rv);
    }
    m.free();
}

fn string_del_run(b: &Both, sh: Option<c_int>, mode: c_int, n: usize, tag: &str) {
    reset_seed(b, 0x3141_5926);
    let sk = StrKeys::new(n, TEST_SEED ^ 47 ^ n as u64, 1);
    let repr = if mode >= STBDS_HM_STRING {
        KeyRepr::Ptr
    } else {
        KeyRepr::Bytes
    };
    let mut owned = sk.keys.clone();
    let mut m = MapPair::new(b, 16, 8, 8, repr, mode);
    if let Some(sm) = sh {
        m = m.with_shmode(sm);
    }
    for i in 0..n {
        let kp: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len()) };
        let v = val_for(i as u64, 8);
        m.put(kp, &v, &format!("{tag} put#{i}"));
    }
    // Delete in an interleaved order so both the "last element" and
    // "move-last-into-hole" paths are hit.
    let mut order: Vec<usize> = (0..n).collect();
    let mut rng = Rng::new(TEST_SEED ^ 0x47 ^ n as u64);
    for i in (1..n).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    let mut deleted = vec![false; n];
    for (step, &i) in order.iter().enumerate() {
        let kp: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len()) };
        let t = m.del(kp, 0, &format!("{tag} del#{i} step={step}"));
        assert_eq!(t, 1, "{tag}: delete of present key {i} reported {t}");
        deleted[i] = true;
        // Every remaining key must still be findable with its own value.
        for j in 0..n {
            if deleted[j] {
                continue;
            }
            let kp2: &mut [u8] =
                unsafe { std::slice::from_raw_parts_mut(owned[j].as_mut_ptr(), owned[j].len()) };
            let idx = m.geti(kp2, &format!("{tag} chk#{j} after del#{i}"));
            assert!(idx >= 0, "{tag}: key {j} lost after deleting {i}");
            let (cv, rv) = m.value_at(idx);
            diff_eq!(format!("{tag} val#{j}"), &cv, &rv);
            assert_eq!(cv, val_for(j as u64, 8), "{tag}: key/value association broken");
        }
    }
    m.free();
    drop(owned);
}

#[test]
fn c47_del_sh_strdup() {
    let b = libs();
    for n in [1usize, 2, 7, 30] {
        string_del_run(
            &b,
            Some(STBDS_SH_STRDUP),
            STBDS_HM_STRING,
            n,
            &format!("C47 n={n}"),
        );
    }
}

#[test]
fn c48_del_sh_arena() {
    let b = libs();
    for n in [1usize, 2, 7, 30] {
        string_del_run(
            &b,
            Some(STBDS_SH_ARENA),
            STBDS_HM_STRING,
            n,
            &format!("C48 n={n}"),
        );
    }
}

#[test]
fn c49_del_sh_default() {
    let b = libs();
    for n in [1usize, 2, 7, 30] {
        string_del_run(&b, None, STBDS_HM_STRING, n, &format!("C49 implicit n={n}"));
        string_del_run(
            &b,
            Some(STBDS_SH_DEFAULT),
            STBDS_HM_STRING,
            n,
            &format!("C49 explicit n={n}"),
        );
    }
}

/// Build a `mode`-mismatched string map and delete a NON-last element.
///
/// With `mode = 2` (or any value `> STBDS_HM_STRING`), `stbds_hm_find_slot`
/// treats the key as a string (`mode >= STBDS_HM_STRING`) but the two
/// `mode == STBDS_HM_STRING` checks inside `stbds_hmdel_key` are FALSE. So after
/// the last element is memmoved into the hole, the slot re-find is done on the
/// ADDRESS of the moved element's `char *` field instead of the string it points
/// at — the hash does not match, `find_slot` returns -1, and the live
/// `STBDS_ASSERT(slot >= 0)` aborts the process.
fn mode_mismatch_delete(api: &Api, sh: Option<c_int>, mode: c_int, n: usize) {
    unsafe {
        let sk = StrKeys::new(n, 0xBEEF, 0);
        let mut owned = sk.keys.clone();
        let mut a: *mut c_void = match sh {
            Some(sm) => (api.shmode_func)(16, sm),
            None => std::ptr::null_mut(),
        };
        for i in 0..n {
            let kp = owned[i].as_mut_ptr() as *mut c_void;
            a = (api.hmput_key)(a, 16, kp, 8, mode);
        }
        // Delete element 0, which is NOT the last one when n >= 2.
        let kp = owned[0].as_mut_ptr() as *mut c_void;
        a = (api.hmdel_key)(a, 16, kp, 8, 0, mode);
        std::hint::black_box(a);
        std::mem::forget(owned);
    }
}

#[test]
fn c50_del_mode_two_on_strdup_map() {
    let b = libs();

    // n == 1: `old_index == final_index`, so the buggy re-find is never reached
    // and both libraries complete normally.
    for sh in [Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA), None] {
        for mode in [2i32, 7, 1000] {
            reset_seed(&b, 0x3141_5926);
            let sk = StrKeys::new(1, TEST_SEED ^ 50, 0);
            let mut owned = sk.keys.clone();
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Ptr, mode);
            if let Some(sm) = sh {
                m = m.with_shmode(sm);
            }
            let kp: &mut [u8] =
                unsafe { std::slice::from_raw_parts_mut(owned[0].as_mut_ptr(), owned[0].len()) };
            let v = val_for(0, 8);
            m.put(kp, &v, &format!("C50 n=1 sh={sh:?} mode={mode} put"));
            let t = m.del(kp, 0, &format!("C50 n=1 sh={sh:?} mode={mode} del"));
            assert_eq!(t, 1);
            m.free();
            drop(owned);
        }
    }

    // n >= 2: the C's live `STBDS_ASSERT(slot >= 0)` fires. Each side is run in
    // a forked child so the abort does not take the test harness down, and the
    // two termination outcomes are compared.
    for sh in [Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA), None] {
        for mode in [2i32, 7] {
            for n in [2usize, 5, 30] {
                reset_seed(&b, 0x3141_5926);
                let co = run_in_child(|| mode_mismatch_delete(&b.c, sh, mode, n));
                reset_seed(&b, 0x3141_5926);
                let ro = run_in_child(|| mode_mismatch_delete(&b.r, sh, mode, n));
                diff_eq!(format!("C50 sh={sh:?} mode={mode} n={n} outcome"), co, ro);
                assert_eq!(
                    co,
                    Outcome::Signaled(6),
                    "C50: expected SIGABRT from the live STBDS_ASSERT"
                );
            }
        }
    }
}

#[test]
fn c51_del_keyoffset_variants() {
    let b = libs();
    for keyoffset in [0usize, 1, 4, 8, 12] {
        for n in [1u64, 7, 30] {
            reset_seed(&b, 0x3141_5926);
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
            for i in 0..n {
                let mut k = bin_key(i, 8);
                m.put(
                    &mut k,
                    &val_for(i, 8),
                    &format!("C51 ko={keyoffset} n={n} put#{i}"),
                );
            }
            for i in 0..n {
                let mut k = bin_key(i, 8);
                m.del(
                    &mut k,
                    keyoffset,
                    &format!("C51 ko={keyoffset} n={n} del#{i}"),
                );
            }
            m.free();
        }
    }
}

// ===========================================================================
// C52–C53: stbds_hmfree_func
// ===========================================================================

#[test]
fn c52_hmfree_all_string_modes() {
    let b = libs();
    for sh in [
        None,
        Some(STBDS_SH_NONE),
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        for n in [0usize, 1, 60] {
            reset_seed(&b, 0x3141_5926);
            let sk = StrKeys::new(n.max(1), TEST_SEED ^ 52, 1);
            let mut owned = sk.keys.clone();
            let repr = match sh {
                Some(STBDS_SH_NONE) | None if false => KeyRepr::Bytes,
                Some(x) if x == STBDS_SH_NONE => KeyRepr::Bytes,
                _ => KeyRepr::Ptr,
            };
            let mut m = MapPair::new(&b, 16, 8, 8, repr, STBDS_HM_STRING);
            if let Some(sm) = sh {
                m = m.with_shmode(sm);
            }
            for i in 0..n {
                let kp: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
                };
                let v = val_for(i as u64, 8);
                m.put(kp, &v, &format!("C52 sh={sh:?} n={n} put#{i}"));
            }
            m.check(&format!("C52 sh={sh:?} n={n} before free"));
            m.free();
            drop(owned);
        }
    }
}

#[test]
fn c53_hmfree_unhashed_array() {
    let b = libs();
    for elemsize in [4usize, 8, 16, 24] {
        unsafe {
            let c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            diff_eq!(
                format!("C53 es={elemsize}"),
                snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            (b.c.hmfree_func)((c as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            (b.r.hmfree_func)((r as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// C64–C65: end-to-end randomized pipelines
// ===========================================================================

#[test]
fn c64_pipeline_binary() {
    let b = libs();
    for (elemsize, keysize, valsize) in [(8usize, 4usize, 4usize), (16, 8, 8), (24, 16, 8), (12, 4, 8)]
    {
        for trial in 0..3u64 {
            reset_seed(&b, 0x3141_5926 ^ trial as usize);
            let mut rng = Rng::new(TEST_SEED ^ 64 ^ trial ^ (elemsize as u64) << 16);
            let mut m =
                MapPair::new(&b, elemsize, keysize, valsize, KeyRepr::Bytes, STBDS_HM_BINARY);
            let universe = 150u64;
            let mut present = vec![false; universe as usize];
            for step in 0..2000 {
                let i = rng.below(universe as usize) as u64;
                let mut k = bin_key(i, keysize);
                let tag = format!("C64 es={elemsize} ks={keysize} t={trial} s={step}");
                match rng.below(10) {
                    0..=4 => {
                        m.put(&mut k, &val_for(i, valsize), &format!("{tag} put#{i}"));
                        present[i as usize] = true;
                    }
                    5..=6 => {
                        let idx = m.geti(&mut k, &format!("{tag} get#{i}"));
                        if present[i as usize] {
                            assert!(idx >= 0, "{tag}: key {i} should be present");
                        }
                    }
                    7 => {
                        m.geti_ts(&mut k, &format!("{tag} get_ts#{i}"));
                    }
                    _ => {
                        let t = m.del(&mut k, 0, &format!("{tag} del#{i}"));
                        let expect = if present[i as usize] { 1 } else { 0 };
                        assert_eq!(t, expect, "{tag}: del#{i} -> {t}, expected {expect}");
                        present[i as usize] = false;
                    }
                }
            }
            for i in 0..universe {
                let mut k = bin_key(i, keysize);
                let idx = m.geti_ts(&mut k, &format!("C64 final#{i}"));
                assert_eq!(
                    idx >= 0,
                    present[i as usize],
                    "C64: presence mismatch for key {i}"
                );
            }
            m.free();
        }
    }
}

#[test]
fn c65_pipeline_string_all_modes() {
    let b = libs();
    for sh in [
        None,
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        for style in [0u8, 1] {
            reset_seed(&b, 0x3141_5926);
            let universe = 150usize;
            let sk = StrKeys::new(universe, TEST_SEED ^ 65 ^ style as u64, style);
            let mut owned = sk.keys.clone();
            let mut rng = Rng::new(TEST_SEED ^ 0x65 ^ style as u64);
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Ptr, STBDS_HM_STRING);
            if let Some(sm) = sh {
                m = m.with_shmode(sm);
            }
            let mut present = vec![false; universe];
            for step in 0..2000 {
                let i = rng.below(universe);
                let kp: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
                };
                let tag = format!("C65 sh={sh:?} style={style} s={step}");
                match rng.below(10) {
                    0..=4 => {
                        let v = val_for(i as u64, 8);
                        m.put(kp, &v, &format!("{tag} put#{i}"));
                        present[i] = true;
                    }
                    5..=6 => {
                        let idx = m.geti(kp, &format!("{tag} get#{i}"));
                        if present[i] {
                            assert!(idx >= 0, "{tag}: key {i} should be present");
                        }
                    }
                    7 => {
                        m.geti_ts(kp, &format!("{tag} get_ts#{i}"));
                    }
                    _ => {
                        let t = m.del(kp, 0, &format!("{tag} del#{i}"));
                        let expect = if present[i] { 1 } else { 0 };
                        assert_eq!(t, expect, "{tag}: del#{i} -> {t}, expected {expect}");
                        present[i] = false;
                    }
                }
            }
            for i in 0..universe {
                let kp: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
                };
                let idx = m.geti_ts(kp, &format!("C65 final#{i}"));
                assert_eq!(idx >= 0, present[i], "C65: presence mismatch for key {i}");
            }
            m.free();
            drop(owned);
        }
    }
}
