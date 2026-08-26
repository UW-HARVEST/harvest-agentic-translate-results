//! Phase B — CONFIGS.md rows 48..66: the string hash map (`stbds_shmode_func`
//! + `stbds_hmput_key`/`hmget_key`/`hmdel_key` with `mode >= STBDS_HM_STRING`)
//! and the string arena (`stbds_stralloc` / `stbds_strreset`).

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const BLOCK_MIN: usize = 512;
const BLOCK_MAX: usize = 1 << 20;

/// Stable, NUL-terminated key buffers.  Every buffer is padded with
/// deterministic bytes *after* the terminator so that the `STBDS_SH_NONE`
/// `default: memcpy(elem, key, keysize)` branch (which copies `keysize` bytes of
/// the string, possibly past the NUL) never reads indeterminate memory.
struct Keys(Vec<Vec<u8>>);

impl Keys {
    fn build(n: usize, mut mk: impl FnMut(usize) -> Vec<u8>) -> Keys {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let mut b = mk(i);
            b.push(0);
            // padding: 16 deterministic bytes so an 8-byte over-read is defined
            for j in 0..16 {
                b.push(0x40u8.wrapping_add((i as u8).wrapping_mul(11)).wrapping_add(j));
            }
            v.push(b);
        }
        Keys(v)
    }
    fn ptr(&mut self, i: usize) -> *mut c_char {
        self.0[i].as_mut_ptr() as *mut c_char
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn ascii_keys(n: usize, minlen: usize, maxlen: usize, seed: u64) -> Keys {
    let mut rng = Rng::new(seed);
    Keys::build(n, |i| {
        let span = (maxlen - minlen + 1) as u64;
        let l = minlen + rng.below(span) as usize;
        let mut s: Vec<u8> = format!("k{i}_").into_bytes();
        while s.len() < l {
            s.push(0x21 + (rng.below(0x5e) as u8));
        }
        s.truncate(l.max(1));
        if s.is_empty() {
            s.push(b'x');
        }
        s
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn table_string_mode(t: *mut c_void, elemsize: usize) -> Option<u8> {
    map_table(t, elemsize).map(|ti| ti.string.mode)
}

unsafe fn temp_key_str(t: *mut c_void, elemsize: usize) -> String {
    match map_table(t, elemsize) {
        None => "<no-table>".into(),
        Some(ti) => cstr(ti.temp_key),
    }
}

// ---------------------------------------------------------- rows 48 & 52
#[test]
fn row48_52_shmode_func_all_modes_incl_out_of_range() {
    let _g = global_lock();
    let (c, r) = load_both();
    // (unsigned char) truncation of the mode argument
    let modes: [(c_int, u8); 13] = [
        (STBDS_SH_NONE, 0),
        (STBDS_SH_DEFAULT, 1),
        (STBDS_SH_STRDUP, 2),
        (STBDS_SH_ARENA, 3),
        (4, 4),
        (5, 5),
        (255, 255),
        (256, 0),
        (300, 44),
        (-1, 255),
        (-2, 254),
        (c_int::MAX, 255),
        (c_int::MIN, 0),
    ];
    unsafe {
        for &es in &[8usize, 16] {
            for &(mode, expect) in &modes {
                pin_seed(&c, &r, 0x9001);
                let ct = (c.shmode_func)(es, mode);
                let rt = (r.shmode_func)(es, mode);
                diff_eq(
                    &format!("row48/52 shmode_func({es},{mode}) fresh"),
                    &snapshot_map(ct, es, KeyKind::Raw),
                    &snapshot_map(rt, es, KeyKind::Raw),
                );
                diff_eq_val(
                    &format!("row52 shmode_func({es},{mode}) string.mode"),
                    table_string_mode(ct, es),
                    Some(expect),
                );
                diff_eq_val(
                    &format!("row52 shmode_func({es},{mode}) string.mode rust"),
                    table_string_mode(ct, es),
                    table_string_mode(rt, es),
                );
                (c.hmfree_func)(hash_to_arr(ct, es), es);
                (r.hmfree_func)(hash_to_arr(rt, es), es);
            }
        }
    }
}

/// For every `string.mode` that lands in the `default:` (`memcpy`) branch, the
/// keys are compared with `memcmp` when `mode = STBDS_HM_BINARY`, which makes a
/// full put/get/del cycle perfectly well-defined.  This is the exhaustive
/// coverage of ERRORS.md row 31 / CONFIGS.md rows 48 & 52.
#[test]
fn row48_52_memcpy_branch_full_cycle_binary_compare() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &shmode in &[0 as c_int, 4, 5, 44, 255, 256, 300, -1, c_int::MAX, c_int::MIN] {
            for &es in &[8usize, 16] {
                let cfg = MapCfg {
                    elemsize: es,
                    keysize: 8,
                    mode: STBDS_HM_BINARY,
                    del_keyoffset: 0,
                    kind: KeyKind::Raw,
                };
                pin_seed(&c, &r, 0x9002);
                let mut ct = (c.shmode_func)(es, shmode);
                let mut rt = (r.shmode_func)(es, shmode);
                for k in 0..30u64 {
                    let key = k.to_le_bytes();
                    let pl = [0xA0u8; 64];
                    ct = map_put_binary(&c, ct, &cfg, &key, &pl);
                    rt = map_put_binary(&r, rt, &cfg, &key, &pl);
                    diff_eq(
                        &format!("row48/52 shmode={shmode} es={es} put({k})"),
                        &snapshot_map(ct, es, KeyKind::Raw),
                        &snapshot_map(rt, es, KeyKind::Raw),
                    );
                }
                for k in 0..40u64 {
                    let mut key = k.to_le_bytes();
                    let (nct, ci) = map_geti(&c, ct, &cfg, &mut key);
                    let mut key = k.to_le_bytes();
                    let (nrt, ri) = map_geti(&r, rt, &cfg, &mut key);
                    ct = nct;
                    rt = nrt;
                    diff_eq_val(&format!("row48/52 shmode={shmode} get({k})"), ci, ri);
                }
                for k in 0..30u64 {
                    let mut key = k.to_le_bytes();
                    let (nct, cr) = map_del(&c, ct, &cfg, &mut key);
                    let mut key = k.to_le_bytes();
                    let (nrt, rr) = map_del(&r, rt, &cfg, &mut key);
                    ct = nct;
                    rt = nrt;
                    diff_eq_val(&format!("row48/52 shmode={shmode} del({k})"), cr, rr);
                    diff_eq(
                        &format!("row48/52 shmode={shmode} es={es} del({k}) state"),
                        &snapshot_map(ct, es, KeyKind::Raw),
                        &snapshot_map(rt, es, KeyKind::Raw),
                    );
                }
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }
        }
    }
}

/// `string.mode` in the `default:` branch combined with `mode = STBDS_HM_STRING`
/// makes the library `memcpy` `keysize` bytes *of the string* into the element
/// and then interpret those bytes as a `char *` on any later comparison, so only
/// insertion of pairwise-distinct keys is well defined.  That insertion path is
/// still fully compared.
#[test]
fn row48_memcpy_branch_string_mode_distinct_inserts() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &shmode in &[0 as c_int, 4, 44, 255] {
            for &es in &[8usize, 16] {
                let cfg = MapCfg { elemsize: es, keysize: 8, mode: STBDS_HM_STRING, del_keyoffset: 0, kind: KeyKind::Raw };
                let mut keys = ascii_keys(12, 3, 20, 480 + shmode as u64);
                pin_seed(&c, &r, 0x9003);
                let mut ct = (c.shmode_func)(es, shmode);
                let mut rt = (r.shmode_func)(es, shmode);
                for i in 0..keys.len() {
                    let p = keys.ptr(i);
                    ct = map_put_string(&c, ct, &cfg, p, &[0xB0u8; 64]);
                    rt = map_put_string(&r, rt, &cfg, p, &[0xB0u8; 64]);
                    diff_eq(
                        &format!("row48 shmode={shmode} es={es} put(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Raw),
                        &snapshot_map(rt, es, KeyKind::Raw),
                    );
                }
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }
        }
    }
}

// ---------------------------------------------------------- rows 49..51
fn string_map_cycle(tag: &str, shmode: c_int, es: usize, counts: &[usize], minlen: usize, maxlen: usize, with_del: bool) {
    let (c, r) = load_both();
    unsafe {
        for &n in counts {
            let cfg = MapCfg::string(es, STBDS_HM_STRING);
            let mut keys = ascii_keys(n.max(1), minlen, maxlen, 1000 + n as u64 + shmode as u64);
            pin_seed(&c, &r, 0xA100);
            let mut ct = (c.shmode_func)(es, shmode);
            let mut rt = (r.shmode_func)(es, shmode);
            diff_eq(
                &format!("{tag} n={n} fresh"),
                &snapshot_map(ct, es, KeyKind::Pointer),
                &snapshot_map(rt, es, KeyKind::Pointer),
            );
            for i in 0..n {
                let p = keys.ptr(i);
                ct = map_put_string(&c, ct, &cfg, p, &[0xC0u8; 64]);
                rt = map_put_string(&r, rt, &cfg, p, &[0xC0u8; 64]);
                diff_eq(
                    &format!("{tag} n={n} put(#{i})"),
                    &snapshot_map(ct, es, KeyKind::Pointer),
                    &snapshot_map(rt, es, KeyKind::Pointer),
                );
                // `stbds_temp_key` is written by the insert path in every string
                // mode; compare its *contents* (the address is library-local).
                diff_eq_val(
                    &format!("{tag} n={n} put(#{i}) temp_key"),
                    temp_key_str(ct, es),
                    temp_key_str(rt, es),
                );
            }
            // duplicate puts (update path, both the forward and the wrap-around
            // inner scan)
            for i in (0..n).rev() {
                let p = keys.ptr(i);
                ct = map_put_string(&c, ct, &cfg, p, &[0xC1u8; 64]);
                rt = map_put_string(&r, rt, &cfg, p, &[0xC1u8; 64]);
                diff_eq(
                    &format!("{tag} n={n} dup-put(#{i})"),
                    &snapshot_map(ct, es, KeyKind::Pointer),
                    &snapshot_map(rt, es, KeyKind::Pointer),
                );
                // `stbds_hmput_key` sets `temp_key` when the duplicate is found in
                // the FORWARD inner scan but deliberately NOT in the wrap-around
                // scan, so after a duplicate put `temp_key` is either the matched
                // key or whatever the previous put left behind.  Both cases must
                // agree, which pins down that quirk.
                diff_eq_val(
                    &format!("{tag} n={n} dup-put(#{i}) temp_key"),
                    temp_key_str(ct, es),
                    temp_key_str(rt, es),
                );
            }
            // hits
            for i in 0..n {
                let p = keys.ptr(i);
                let nct = (c.hmget_key)(ct, es, p as *mut c_void, 8, STBDS_HM_STRING);
                let nrt = (r.hmget_key)(rt, es, p as *mut c_void, 8, STBDS_HM_STRING);
                ct = nct;
                rt = nrt;
                diff_eq_val(
                    &format!("{tag} n={n} get(#{i})"),
                    hm_temp(ct, es),
                    hm_temp(rt, es),
                );
                assert!(hm_temp(ct, es) >= 0, "{tag}: key #{i} must be found");
            }
            // misses
            let mut miss = ascii_keys(8, 6, 12, 77_000 + n as u64);
            for i in 0..miss.len() {
                let p = miss.ptr(i);
                let nct = (c.hmget_key)(ct, es, p as *mut c_void, 8, STBDS_HM_STRING);
                let nrt = (r.hmget_key)(rt, es, p as *mut c_void, 8, STBDS_HM_STRING);
                ct = nct;
                rt = nrt;
                diff_eq_val(
                    &format!("{tag} n={n} miss(#{i})"),
                    hm_temp(ct, es),
                    hm_temp(rt, es),
                );
            }
            if with_del {
                for i in 0..n {
                    let p = keys.ptr(i);
                    let nct = (c.hmdel_key)(ct, es, p as *mut c_void, 8, 0, STBDS_HM_STRING);
                    let nrt = (r.hmdel_key)(rt, es, p as *mut c_void, 8, 0, STBDS_HM_STRING);
                    ct = nct;
                    rt = nrt;
                    diff_eq_val(
                        &format!("{tag} n={n} del(#{i}) null-ness"),
                        ct.is_null(),
                        rt.is_null(),
                    );
                    diff_eq(
                        &format!("{tag} n={n} del(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
            }
            map_free(&c, ct, es);
            map_free(&r, rt, es);
        }
    }
}

#[test]
fn row49_55_sh_default() {
    let _g = global_lock();
    for &es in &[8usize, 16] {
        string_map_cycle("row49/55 SH_DEFAULT", STBDS_SH_DEFAULT, es, &[0, 1, 5, 6, 7, 20, 60], 1, 80, true);
    }
}

#[test]
fn row50_56_sh_strdup() {
    let _g = global_lock();
    for &es in &[8usize, 16] {
        string_map_cycle("row50/56 SH_STRDUP", STBDS_SH_STRDUP, es, &[0, 1, 5, 6, 7, 20, 60], 1, 80, true);
    }
}

#[test]
fn row51_57_sh_arena() {
    let _g = global_lock();
    for &es in &[8usize, 16] {
        string_map_cycle("row51/57 SH_ARENA", STBDS_SH_ARENA, es, &[0, 1, 5, 6, 7, 20, 60], 1, 80, true);
    }
    // long keys -> several arena blocks (511/512/513/2000 byte keys)
    let (c, r) = load_both();
    unsafe {
        for &es in &[8usize, 16] {
            let cfg = MapCfg::string(es, STBDS_HM_STRING);
            let lens = [1usize, 200, 400, 511, 512, 513, 700, 1000, 2000, 3000, 5];
            let mut keys = Keys::build(lens.len() * 3, |i| {
                let l = lens[i % lens.len()];
                let mut s = format!("A{i}_").into_bytes();
                while s.len() < l {
                    s.push(b'a' + ((i + s.len()) % 26) as u8);
                }
                s.truncate(l.max(1));
                s
            });
            pin_seed(&c, &r, 0xA200);
            let mut ct = (c.shmode_func)(es, STBDS_SH_ARENA);
            let mut rt = (r.shmode_func)(es, STBDS_SH_ARENA);
            for i in 0..keys.len() {
                let p = keys.ptr(i);
                ct = map_put_string(&c, ct, &cfg, p, &[0xC2u8; 64]);
                rt = map_put_string(&r, rt, &cfg, p, &[0xC2u8; 64]);
                diff_eq(
                    &format!("row57 long-keys es={es} put(#{i})"),
                    &snapshot_map(ct, es, KeyKind::Pointer),
                    &snapshot_map(rt, es, KeyKind::Pointer),
                );
            }
            // the arena must really have grown past its first block
            let ti = map_table(ct, es).unwrap();
            assert!(ti.string.block >= 2, "arena block ladder not exercised (block={})", ti.string.block);
            map_free(&c, ct, es);
            map_free(&r, rt, es);
        }
    }
}

// ---------------------------------------------------------- rows 53..54
#[test]
fn row53_54_implicit_arena_mode_from_hmput_key() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &es in &[8usize, 16] {
            // row 53: mode >= STBDS_HM_STRING on a NULL map -> SH_DEFAULT
            for &mode in &[1 as c_int, 2, 3, 999, c_int::MAX] {
                let cfg = MapCfg::string(es, mode);
                let mut keys = ascii_keys(6, 3, 30, 5300 + mode as u64);
                pin_seed(&c, &r, 0xA300);
                let mut ct: *mut c_void = std::ptr::null_mut();
                let mut rt: *mut c_void = std::ptr::null_mut();
                for i in 0..keys.len() {
                    let p = keys.ptr(i);
                    ct = map_put_string(&c, ct, &cfg, p, &[0xD0u8; 64]);
                    rt = map_put_string(&r, rt, &cfg, p, &[0xD0u8; 64]);
                    diff_eq(
                        &format!("row53 mode={mode} es={es} put(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
                diff_eq_val(
                    &format!("row53 mode={mode} implicit string.mode == SH_DEFAULT"),
                    table_string_mode(ct, es),
                    Some(STBDS_SH_DEFAULT as u8),
                );
                diff_eq_val(
                    &format!("row53 mode={mode} implicit string.mode rust"),
                    table_string_mode(ct, es),
                    table_string_mode(rt, es),
                );
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }

            // row 54: mode <= 0 on a NULL map -> string.mode stays 0
            for &mode in &[0 as c_int, -1, c_int::MIN] {
                let cfg = MapCfg {
                    elemsize: es,
                    keysize: 8,
                    mode,
                    del_keyoffset: 0,
                    kind: KeyKind::Raw,
                };
                pin_seed(&c, &r, 0xA301);
                let mut ct: *mut c_void = std::ptr::null_mut();
                let mut rt: *mut c_void = std::ptr::null_mut();
                for k in 0..6u64 {
                    let key = k.to_le_bytes();
                    ct = map_put_binary(&c, ct, &cfg, &key, &[0xD1u8; 64]);
                    rt = map_put_binary(&r, rt, &cfg, &key, &[0xD1u8; 64]);
                }
                diff_eq_val(
                    &format!("row54 mode={mode} implicit string.mode == 0"),
                    table_string_mode(ct, es),
                    Some(0),
                );
                diff_eq_val(
                    &format!("row54 mode={mode} implicit string.mode rust"),
                    table_string_mode(ct, es),
                    table_string_mode(rt, es),
                );
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }
        }
    }
}

// --------------------------------------------------------------- row 58
#[test]
fn row58_string_mode_ge_2() {
    let _g = global_lock();
    let (c, r) = load_both();
    // `mode >= 2` hashes/compares as a string but is NOT `== STBDS_HM_STRING`,
    // so `hmdel_key` skips the strdup free and uses the *raw address* of the key
    // field on the back-fill re-find.  Deleting the LAST element avoids the
    // back-fill entirely (ERRORS.md row 39) and is therefore well defined; the
    // back-fill abort itself is covered in tests/errors.rs (ERRORS.md row 41).
    unsafe {
        for &mode in &[2 as c_int, 3, 999, c_int::MAX] {
            for &es in &[8usize, 16] {
                let cfg = MapCfg::string(es, mode);
                let mut keys = ascii_keys(20, 2, 40, 5800 + mode as u64);
                pin_seed(&c, &r, 0xA400);
                let mut ct: *mut c_void = std::ptr::null_mut();
                let mut rt: *mut c_void = std::ptr::null_mut();
                for i in 0..keys.len() {
                    let p = keys.ptr(i);
                    ct = map_put_string(&c, ct, &cfg, p, &[0xE0u8; 64]);
                    rt = map_put_string(&r, rt, &cfg, p, &[0xE0u8; 64]);
                    diff_eq(
                        &format!("row58 mode={mode} es={es} put(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
                // duplicate puts: forward + wrap-around match paths
                for i in (0..keys.len()).rev() {
                    let p = keys.ptr(i);
                    ct = map_put_string(&c, ct, &cfg, p, &[0xE1u8; 64]);
                    rt = map_put_string(&r, rt, &cfg, p, &[0xE1u8; 64]);
                    diff_eq(
                        &format!("row58 mode={mode} es={es} dup(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
                for i in 0..keys.len() {
                    let p = keys.ptr(i);
                    ct = (c.hmget_key)(ct, es, p as *mut c_void, 8, mode);
                    rt = (r.hmget_key)(rt, es, p as *mut c_void, 8, mode);
                    diff_eq_val(
                        &format!("row58 mode={mode} get(#{i})"),
                        hm_temp(ct, es),
                        hm_temp(rt, es),
                    );
                }
                // delete strictly in reverse insertion order -> always the last
                // element -> no back-fill.
                for i in (0..keys.len()).rev() {
                    let p = keys.ptr(i);
                    ct = (c.hmdel_key)(ct, es, p as *mut c_void, 8, 0, mode);
                    rt = (r.hmdel_key)(rt, es, p as *mut c_void, 8, 0, mode);
                    diff_eq(
                        &format!("row58 mode={mode} es={es} del(#{i})"),
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }
        }
    }
}

// --------------------------------------------------------------- row 59
#[test]
fn row59_randomized_string_pipeline() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &shmode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            for &es in &[8usize, 16] {
                let cfg = MapCfg::string(es, STBDS_HM_STRING);
                let mut keys = ascii_keys(40, 1, 120, 5900 + shmode as u64 + es as u64);
                let mut rng = Rng::new(59 + shmode as u64 * 7 + es as u64);
                pin_seed(&c, &r, 0xA500);
                let mut ct = (c.shmode_func)(es, shmode);
                let mut rt = (r.shmode_func)(es, shmode);
                let mut live: std::collections::BTreeSet<usize> = Default::default();
                for step in 0..2000u32 {
                    let ctx = format!("row59 shmode={shmode} es={es} step={step}");
                    let i = rng.below(40) as usize;
                    let p = keys.ptr(i);
                    match rng.below(10) {
                        0..=4 => {
                            let pl = [rng.byte(); 64];
                            ct = map_put_string(&c, ct, &cfg, p, &pl);
                            rt = map_put_string(&r, rt, &cfg, p, &pl);
                            live.insert(i);
                        }
                        5..=6 => {
                            ct = (c.hmget_key)(ct, es, p as *mut c_void, 8, STBDS_HM_STRING);
                            rt = (r.hmget_key)(rt, es, p as *mut c_void, 8, STBDS_HM_STRING);
                            diff_eq_val(&format!("{ctx} get idx"), hm_temp(ct, es), hm_temp(rt, es));
                            diff_eq_val(
                                &format!("{ctx} presence"),
                                hm_temp(ct, es) >= 0,
                                live.contains(&i),
                            );
                        }
                        7 => {
                            let mut tc: isize = 0;
                            let mut tr: isize = 0;
                            ct = (c.hmget_key_ts)(ct, es, p as *mut c_void, 8, &mut tc, STBDS_HM_STRING);
                            rt = (r.hmget_key_ts)(rt, es, p as *mut c_void, 8, &mut tr, STBDS_HM_STRING);
                            diff_eq_val(&format!("{ctx} get_ts idx"), tc, tr);
                        }
                        _ => {
                            ct = (c.hmdel_key)(ct, es, p as *mut c_void, 8, 0, STBDS_HM_STRING);
                            rt = (r.hmdel_key)(rt, es, p as *mut c_void, 8, 0, STBDS_HM_STRING);
                            live.remove(&i);
                        }
                    }
                    diff_eq(
                        &ctx,
                        &snapshot_map(ct, es, KeyKind::Pointer),
                        &snapshot_map(rt, es, KeyKind::Pointer),
                    );
                }
                map_free(&c, ct, es);
                map_free(&r, rt, es);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// String arena — rows 60..66
// ---------------------------------------------------------------------------

/// The arena's block chain as block start addresses, **in chain order**.
unsafe fn arena_chain(a: &StringArena) -> Vec<usize> {
    let mut out = Vec::new();
    let mut p = a.storage as *mut c_void;
    while !p.is_null() && out.len() < 100_000 {
        out.push(p as usize);
        p = *(p as *mut *mut c_void);
    }
    out
}

/// Snapshot of an arena that is comparable across libraries plus the offset of
/// `p` inside the *head* block (address-free) when that is meaningful.
unsafe fn arena_state(a: &StringArena) -> String {
    format!(
        "remaining={} block={} mode={} storage={} blocks={}",
        a.remaining,
        a.block,
        a.mode,
        if a.storage.is_null() { "NULL" } else { "SET" },
        arena_chain(a).len()
    )
}

/// Independent re-implementation of `stbds_stralloc`'s bookkeeping, used as an
/// oracle so that *where in the block chain* a string lands is verified exactly
/// (the addresses themselves are library-local, so the chain must be checked
/// structurally).  `sizeof(stbds_string_block) - 8 == 8` bytes of header precede
/// each block's `storage[]`.
#[derive(Default)]
struct ArenaModel {
    /// block payload sizes, in chain order (head first)
    blocks: Vec<usize>,
    remaining: usize,
    block: u8,
}

impl ArenaModel {
    fn with_block(block: u8) -> Self {
        ArenaModel { blocks: Vec::new(), remaining: 0, block }
    }

    /// Returns `(chain_index, byte_offset_from_block_start)` for the string that
    /// `stbds_stralloc(a, str)` with `strlen(str)+1 == len` must return.
    fn alloc(&mut self, len: usize) -> (usize, usize) {
        if len > self.remaining {
            // `blocksize = (size_t)512u << (a->block >> 1)` — the shift count can
            // exceed 63, which x86-64 masks to 6 bits (`wrapping_shl`).
            let blocksize = BLOCK_MIN.wrapping_shl((self.block >> 1) as u32);
            if blocksize < BLOCK_MAX {
                self.block = self.block.wrapping_add(1);
            }
            if len > blocksize {
                // oversize block
                if !self.blocks.is_empty() {
                    // sb->next = storage->next; storage->next = sb  => index 1
                    self.blocks.insert(1, len);
                    return (1, 8);
                } else {
                    self.blocks.insert(0, len);
                    self.remaining = 0;
                    return (0, 8);
                }
            } else {
                // sb->next = a->storage; a->storage = sb  => new head
                self.blocks.insert(0, blocksize);
                self.remaining = blocksize;
            }
        }
        // p = a->storage->storage + a->remaining - len
        let off = 8 + self.remaining - len;
        self.remaining -= len;
        (0, off)
    }
}

unsafe fn head_offset(a: &StringArena, p: *const c_char) -> isize {
    if a.storage.is_null() || p.is_null() {
        isize::MIN
    } else {
        // plain integer subtraction (the two pointers may belong to different
        // allocations, so `offset_from` would not be legal)
        (p as usize).wrapping_sub((a.storage as usize).wrapping_add(8)) as isize
    }
}

struct ArenaDuo<'a> {
    c: &'a Api,
    r: &'a Api,
    ca: StringArena,
    ra: StringArena,
    model: ArenaModel,
}

impl<'a> ArenaDuo<'a> {
    fn new(c: &'a Api, r: &'a Api, block: u8, mode: u8) -> ArenaDuo<'a> {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = block;
        ra.block = block;
        ca.mode = mode;
        ra.mode = mode;
        ArenaDuo { c, r, ca, ra, model: ArenaModel::with_block(block) }
    }

    /// One `stbds_stralloc` on both arenas.  Classifies the branch the C code
    /// takes from the *pre*-state so that only meaningful things are compared.
    unsafe fn alloc(&mut self, s: &mut Vec<u8>, ctx: &str) {
        let len = s.len(); // s already contains the NUL
        let pre_remaining = self.ca.remaining;
        let pre_block = self.ca.block;
        let pre_storage_null = self.ca.storage.is_null();
        diff_eq_val(&format!("{ctx} pre-state"), arena_state(&self.ca), arena_state(&self.ra));

        // replicate the C branch selection
        let mut oversize_with_storage = false;
        if len > pre_remaining {
            let blocksize = BLOCK_MIN.wrapping_shl((pre_block >> 1) as u32);
            if len > blocksize {
                oversize_with_storage = !pre_storage_null;
            }
        }

        // oracle prediction, computed BEFORE the calls
        let (exp_idx, exp_off) = self.model.alloc(len);

        let cp = (self.c.stralloc)(&mut self.ca, s.as_mut_ptr() as *mut c_char);
        let rp = (self.r.stralloc)(&mut self.ra, s.as_mut_ptr() as *mut c_char);

        diff_eq_val(&format!("{ctx} content"), cstr(cp), cstr(rp));
        diff_eq_val(&format!("{ctx} post-state"), arena_state(&self.ca), arena_state(&self.ra));
        if !oversize_with_storage {
            // the returned pointer is inside the (new) head block, so its offset
            // from the head is deterministic
            diff_eq_val(
                &format!("{ctx} head offset"),
                head_offset(&self.ca, cp),
                head_offset(&self.ra, rp),
            );
        }

        // Structural check against the oracle: the string must live in chain slot
        // `exp_idx` at byte offset `exp_off` from that block's start, in BOTH
        // libraries.  This pins down the splice position (`sb->next =
        // storage->next; storage->next = sb` for oversize blocks vs. pushing onto
        // the head) and the block sizes, without comparing any address.
        let cch = arena_chain(&self.ca);
        let rch = arena_chain(&self.ra);
        diff_eq_val(&format!("{ctx} chain length"), cch.len(), rch.len());
        assert_eq!(cch.len(), self.model.blocks.len(), "{ctx}: chain length vs oracle");
        assert_eq!(
            (cp as usize).wrapping_sub(cch[exp_idx]),
            exp_off,
            "{ctx}: C put the string at the wrong place (block {exp_idx})"
        );
        assert_eq!(
            (rp as usize).wrapping_sub(rch[exp_idx]),
            exp_off,
            "{ctx}: RUST put the string at the wrong place (block {exp_idx})"
        );
        // and the arena bookkeeping must match the oracle in both libraries
        diff_eq_val(
            &format!("{ctx} vs oracle (remaining, block)"),
            (self.ca.remaining, self.ca.block),
            (self.model.remaining, self.model.block),
        );
        diff_eq_val(
            &format!("{ctx} rust vs oracle (remaining, block)"),
            (self.ra.remaining, self.ra.block),
            (self.model.remaining, self.model.block),
        );
    }

    unsafe fn reset(&mut self, ctx: &str) {
        (self.c.strreset)(&mut self.ca);
        (self.r.strreset)(&mut self.ra);
        diff_eq_val(&format!("{ctx} after reset"), arena_state(&self.ca), arena_state(&self.ra));
        assert!(self.ca.storage.is_null() && self.ca.remaining == 0 && self.ca.block == 0 && self.ca.mode == 0);
        assert!(self.ra.storage.is_null() && self.ra.remaining == 0 && self.ra.block == 0 && self.ra.mode == 0);
        self.model = ArenaModel::default();
    }
}

fn cstring(body: Vec<u8>) -> Vec<u8> {
    let mut v = body;
    v.push(0);
    v
}

fn make_str(len: usize, tag: u8) -> Vec<u8> {
    cstring((0..len).map(|i| b'a' + ((i as u8).wrapping_add(tag) % 26)).collect())
}

// --------------------------------------------------------------- row 60
#[test]
fn row60_stralloc_fresh_arena() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &l in &[0usize, 1, 2, 10, 100, 510, 511, 512] {
            let mut d = ArenaDuo::new(&c, &r, 0, 0);
            let mut s = make_str(l, 1);
            d.alloc(&mut s, &format!("row60 len={l}"));
            d.reset(&format!("row60 len={l}"));
        }
    }
}

// --------------------------------------------------------------- row 61
#[test]
fn row61_stralloc_fill_block_then_overflow() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        let mut d = ArenaDuo::new(&c, &r, 0, 0);
        // 63 allocations of 8 bytes each = 504 of the 512-byte block, then a
        // 16-byte one that no longer fits -> next block at 512 << (block>>1)
        for i in 0..200 {
            let mut s = make_str(7, i as u8);
            d.alloc(&mut s, &format!("row61 step={i}"));
        }
        for i in 0..40 {
            let mut s = make_str(300, i as u8);
            d.alloc(&mut s, &format!("row61 big step={i}"));
        }
        assert!(d.ca.block >= 3, "block ladder not climbed (block={})", d.ca.block);
        d.reset("row61");
    }
}

// ---------------------------------------------------------- rows 62..63
#[test]
fn row62_63_stralloc_oversize_blocks() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        // row 62: len > blocksize with storage == NULL
        for &l in &[513usize, 600, 1000, 5000, 100_000] {
            let mut d = ArenaDuo::new(&c, &r, 0, 0);
            let mut s = make_str(l, 2);
            d.alloc(&mut s, &format!("row62 len={l}"));
            assert_eq!(d.ca.remaining, 0, "oversize-on-empty must set remaining=0");
            assert!(!d.ca.storage.is_null());
            d.reset(&format!("row62 len={l}"));
        }
        // row 63: len > blocksize with storage != NULL
        for &l in &[513usize, 1000, 20_000] {
            let mut d = ArenaDuo::new(&c, &r, 0, 0);
            let mut small = make_str(10, 3);
            d.alloc(&mut small, "row63 seed-block");
            let rem_before = d.ca.remaining;
            let mut s = make_str(l, 4);
            d.alloc(&mut s, &format!("row63 len={l}"));
            assert_eq!(d.ca.remaining, rem_before, "remaining must be preserved");
            // and the head block is still usable afterwards
            let mut more = make_str(10, 5);
            d.alloc(&mut more, &format!("row63 len={l} after"));
            d.reset(&format!("row63 len={l}"));
        }
    }
}

// --------------------------------------------------------------- row 64
#[test]
fn row64_stralloc_block_ladder_and_clamp() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        // block values chosen to cover: the ladder, the 1<<20 clamp (block>=22)
        // and the shift-count masking (block=254/255 -> 512<<127 -> masked to
        // 512<<63 == 0 -> every length takes the oversize path).
        for &blk in &[0u8, 1, 2, 3, 4, 5, 10, 20, 21, 22, 23, 24, 30, 254, 255] {
            for &l in &[0usize, 1, 100, 511, 512, 513, 2000] {
                let mut d = ArenaDuo::new(&c, &r, blk, 0);
                let mut s = make_str(l, 6);
                d.alloc(&mut s, &format!("row64 block={blk} len={l}"));
                d.reset(&format!("row64 block={blk} len={l}"));
            }
        }
        // the clamp: once blocksize reaches 1<<20 the block counter freezes
        let mut d = ArenaDuo::new(&c, &r, 22, 0);
        for i in 0..4 {
            let mut s = make_str(2_000_000, i as u8); // > 1<<20 -> oversize path
            d.alloc(&mut s, &format!("row64 clamp step={i}"));
            diff_eq_val(&format!("row64 clamp block frozen {i}"), d.ca.block, d.ra.block);
        }
        assert_eq!(d.ca.block, 22, "block must stay frozen at the 1<<20 clamp");
        d.reset("row64 clamp");
        assert!(BLOCK_MAX == 1 << 20);
    }
}

// --------------------------------------------------------------- row 65
#[test]
fn row65_stralloc_randomized() {
    let _g = global_lock();
    let (c, r) = load_both();
    let mut rng = Rng::new(65);
    unsafe {
        for round in 0..6 {
            let mut d = ArenaDuo::new(&c, &r, (round * 3) as u8, 0);
            for step in 0..500 {
                let l = rng.below(3000) as usize;
                let mut s = make_str(l, rng.byte());
                d.alloc(&mut s, &format!("row65 round={round} step={step} len={l}"));
            }
            d.reset(&format!("row65 round={round}"));
        }
    }
}

// --------------------------------------------------------------- row 66
#[test]
fn row66_strreset_variants() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        // (a) zeroed arena -> nothing freed
        let mut d = ArenaDuo::new(&c, &r, 0, 0);
        d.reset("row66 empty");
        // (b) one block
        let mut d = ArenaDuo::new(&c, &r, 0, 0);
        let mut s = make_str(10, 7);
        d.alloc(&mut s, "row66 one-block");
        d.reset("row66 one-block");
        // (c) many blocks incl. oversize ones
        let mut d = ArenaDuo::new(&c, &r, 0, 0);
        for i in 0..60 {
            let mut s = make_str(if i % 7 == 0 { 4000 } else { 300 }, i as u8);
            d.alloc(&mut s, &format!("row66 many step={i}"));
        }
        d.reset("row66 many");
        // (d) reset twice in a row
        d.reset("row66 twice");
        // (e) non-zero mode/block fields are also cleared
        let mut d = ArenaDuo::new(&c, &r, 9, 3);
        let mut s = make_str(50, 8);
        d.alloc(&mut s, "row66 mode-set");
        d.reset("row66 mode-set");
    }
}
