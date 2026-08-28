//! Phase B — top-level entry points and randomized whole-pipeline fuzzing.
//!
//! Covers `CONFIGS.md` rows 33b, 52, 53, 54, 55.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0xC0FF_EE00_1234_5678;

// ===========================================================================
// row 33b - the realistic `keyoffset != 0` shape.
//
// `stbds_hmput(t,k,v)` memcpy's the key into element offset 0 (the default
// switch arm) AND separately assigns `t[temp].key = k` at the struct's real
// offset. `stbds_hmdel(t,k)` then passes `STBDS_OFFSETOF(t,key)`. Emulating
// both halves is what makes the delete actually find its key, which is the
// only way to reach the `memmove` + re-`find_slot` fix-up with a non-zero
// keyoffset (c_src/src/lib.c:839-850).
// ===========================================================================
unsafe fn hmput_ko(
    lib: &Lib,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keyoffset: usize,
    mode: c_int,
    value: u64,
) -> *mut c_void {
    let mut k = key.to_vec();
    let t = (lib.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, key.len(), mode);
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let idx = (*header(raw)).temp;
    let e = (t as *mut u8).wrapping_offset(idx * elemsize as isize);
    // deterministic filler for the whole element first
    let mut b = value;
    for j in 0..elemsize {
        *e.wrapping_add(j) = (b & 0xff) as u8;
        b = b.rotate_left(8);
    }
    // then the key at BOTH offset 0 (what hmput_key's memcpy arm did) and at
    // `keyoffset` (what the macro's `t[temp].key = k` does)
    for (j, &bv) in key.iter().enumerate() {
        *e.wrapping_add(j) = bv;
        *e.wrapping_add(keyoffset + j) = bv;
    }
    t
}

#[test]
fn cfg33b_keyoffset_macro_shape() {
    diff("cfg33b", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x33B);
        for &keysize in &[4usize, 8] {
            for &keyoffset in &[8usize, 16] {
                let es = keyoffset + keysize + 8;
                for &n in &[1usize, 2, 10, 60] {
                    let hs = rng.next_u64() as usize;
                    (lib.rand_seed)(hs);
                    let keys: Vec<Vec<u8>> = (0..n)
                        .map(|i| (i as u64).to_le_bytes()[..keysize].to_vec())
                        .collect();
                    let mut t: *mut c_void = std::ptr::null_mut();
                    for (i, k) in keys.iter().enumerate() {
                        t = hmput_ko(lib, t, es, k, keyoffset, HM_BINARY, 0x2000 + i as u64);
                    }
                    log.usz("ks", keysize);
                    log.usz("ko", keyoffset);
                    log.usz("n", n);
                    snap_map(log, t, es, KeyKind::Binary);
                    // random deletion order -> exercises the memmove fix-up
                    let mut order: Vec<usize> = (0..n).collect();
                    for i in (1..n).rev() {
                        let j = rng.below(i + 1);
                        order.swap(i, j);
                    }
                    for &i in &order {
                        let (nt, d) = hmdel(lib, t, es, &keys[i], keysize, keyoffset, HM_BINARY);
                        t = nt;
                        log.usz("del", i);
                        log.isz("d", d);
                        snap_map(log, t, es, KeyKind::Binary);
                    }
                    hmfree(lib, t, es);
                }
            }
        }
    });
}

// ===========================================================================
// row 52 - strkey
// ===========================================================================
#[test]
fn cfg52_strkey() {
    diff("cfg52", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 52);
        let mut ns: Vec<c_int> = vec![
            0,
            1,
            -1,
            9,
            10,
            99,
            100,
            999,
            1000,
            12345,
            -12345,
            c_int::MIN,
            c_int::MAX,
            c_int::MIN + 1,
            c_int::MAX - 1,
        ];
        for _ in 0..64 {
            ns.push(rng.next_u32() as c_int);
        }
        for &n in &ns {
            let p = (lib.strkey)(n);
            log.i32v("n", n);
            log.blob("s", &cstr_bytes(p));
        }
    });
}

// ===========================================================================
// row 53 - str_dups: full pipeline, stdout captured and compared byte-for-byte
// ===========================================================================
#[test]
fn cfg53_str_dups_stdout() {
    let p = pair();
    let _g = lock();
    let nums: Vec<c_int> = vec![
        0,
        1,
        -1,
        -2,
        2,
        3,
        7,
        8,
        9,
        16,
        63,
        64,
        65,
        100,
        511,
        512,
        513,
        1000,
        5000,
        c_int::MIN,
    ];
    for &n in &nums {
        let oc = capture_stdout("c", || unsafe {
            (p.c.rand_seed)(0x3141_5926);
            (p.c.str_dups)(n);
        });
        let orr = capture_stdout("rs", || unsafe {
            (p.rs.rand_seed)(0x3141_5926);
            (p.rs.str_dups)(n);
        });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&orr),
            "str_dups({}) stdout mismatch\n  C  = {:?}\n  RS = {:?}",
            n,
            oc,
            orr
        );
        // and it must actually have printed something
        assert!(
            !oc.is_empty(),
            "str_dups({}) printed nothing - the C loop should always run once",
            n
        );
    }
}

/// `str_dups` repeatedly, to make sure the process-global `stbds_hash_seed`
/// evolves identically across calls in both libraries.
#[test]
fn cfg53b_str_dups_repeated() {
    let p = pair();
    let _g = lock();
    let oc = capture_stdout("c_rep", || unsafe {
        (p.c.rand_seed)(0x3141_5926);
        for n in 0..40 {
            (p.c.str_dups)(n * 37);
        }
    });
    let orr = capture_stdout("rs_rep", || unsafe {
        (p.rs.rand_seed)(0x3141_5926);
        for n in 0..40 {
            (p.rs.str_dups)(n * 37);
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&oc),
        String::from_utf8_lossy(&orr),
        "repeated str_dups stdout mismatch"
    );
    assert_eq!(oc.iter().filter(|&&b| b == b'\n').count(), 40, "expected one line per call");
}

// ===========================================================================
// rows 54 / 55 - randomized whole-pipeline op streams
// ===========================================================================

#[derive(Copy, Clone, Debug)]
enum Op {
    Put,
    Get,
    GetTs,
    Del,
    PutDefault,
}

fn pick_op(rng: &mut Rng, allow_del: bool) -> Op {
    loop {
        let o = match rng.below(10) {
            0..=3 => Op::Put,
            4..=5 => Op::Get,
            6 => Op::GetTs,
            7..=8 => Op::Del,
            _ => Op::PutDefault,
        };
        if matches!(o, Op::Del) && !allow_del {
            continue;
        }
        return o;
    }
}

/// Binary-key op stream (string.mode == SH_NONE or an implicit map).
#[allow(clippy::too_many_arguments)]
unsafe fn fuzz_binary(
    lib: &Lib,
    log: &mut Log,
    rng: &mut Rng,
    hash_seed: usize,
    elemsize: usize,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    ops: usize,
    start_from_shmode: bool,
) {
    (lib.rand_seed)(hash_seed);
    let universe = 48usize;
    let keys: Vec<Vec<u8>> = (0..universe * 2)
        .map(|i| {
            let idx = (i as u64).to_le_bytes();
            (0..keysize).map(|b| if b < 8 { idx[b] } else { 0xA5 }).collect()
        })
        .collect();
    let mut t: *mut c_void = if start_from_shmode {
        (lib.shmode_func)(elemsize, SH_NONE)
    } else {
        std::ptr::null_mut()
    };
    for step in 0..ops {
        let op = pick_op(rng, true);
        // half the lookups target keys that were never inserted
        let ki = rng.below(universe * 2);
        log.usz("step", step);
        log.usz("ki", ki);
        match op {
            Op::Put => {
                log.tag("put");
                if keyoffset == 0 {
                    t = hmput(lib, t, elemsize, &keys[ki], mode, rng.next_u64());
                } else {
                    t = hmput_ko(lib, t, elemsize, &keys[ki], keyoffset, mode, rng.next_u64());
                }
            }
            Op::Get => {
                log.tag("get");
                if t.is_null() {
                    continue;
                }
                let (nt, idx) = hmgeti(lib, t, elemsize, &keys[ki], mode);
                t = nt;
                log.isz("idx", idx);
            }
            Op::GetTs => {
                log.tag("get_ts");
                let mut k = keys[ki].clone();
                let mut temp: isize = 0x4242;
                t = (lib.hmget_key_ts)(
                    t,
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut temp,
                    mode,
                );
                log.isz("temp", temp);
            }
            Op::Del => {
                log.tag("del");
                if t.is_null() {
                    continue;
                }
                let (nt, d) = hmdel(lib, t, elemsize, &keys[ki], keysize, keyoffset, mode);
                t = nt;
                log.isz("d", d);
            }
            Op::PutDefault => {
                log.tag("put_default");
                t = (lib.hmput_default)(t, elemsize);
                // stbds_hmdefault(t, v): t[-1] = ...
                let raw = (t as *mut u8).wrapping_sub(elemsize);
                let mut b = rng.next_u64();
                for j in 0..elemsize {
                    *raw.wrapping_add(j) = (b & 0xff) as u8;
                    b = b.rotate_left(8);
                }
            }
        }
        snap_map(log, t, elemsize, KeyKind::Binary);
    }
    hmfree(lib, t, elemsize);
}

/// String-key op stream over an explicit `string.mode`.
#[allow(clippy::too_many_arguments)]
unsafe fn fuzz_string(
    lib: &Lib,
    log: &mut Log,
    rng: &mut Rng,
    hash_seed: usize,
    elemsize: usize,
    sh_mode: c_int,
    mode: c_int,
    ops: usize,
    allow_del: bool,
) {
    (lib.rand_seed)(hash_seed);
    let universe = 48usize;
    // owned key buffers - SH_DEFAULT stores our pointers
    let mut keys: Vec<Vec<u8>> = (0..universe * 2)
        .map(|i| {
            let mut v = format!("fz{:06}", i).into_bytes();
            v.push(0);
            v
        })
        .collect();
    let mut t: *mut c_void = (lib.shmode_func)(elemsize, sh_mode);
    // `mode >= 2` deletes reach the address-hashing fix-up branch at
    // c_src/src/lib.c:845 whenever `old_index != final_index`, which is not
    // comparable across libraries - so only mode 0/1 deletes are ever fuzzed.
    let allow_del = allow_del && mode <= 1;
    // `hash_index::temp_key` is only comparable when nothing can have freed the
    // key it points at. A `mode == 1` delete on a `SH_STRDUP` map frees the
    // key, and the wrap-around duplicate-hit branch (c_src/src/lib.c:746-759)
    // deliberately does NOT refresh `temp_key`, so it can dangle. Reading a
    // dangling pointer is allocator state, not library behaviour, so
    // `temp_key` is only snapshotted for delete-free streams. (Its exact
    // update/no-update semantics are pinned down by
    // `cfg43_string_duplicates_temp_key`, which never deletes.)
    let mut tk = TkValid::new();
    for step in 0..ops {
        let op = pick_op(rng, allow_del);
        let ki = rng.below(universe * 2);
        log.usz("step", step);
        log.usz("ki", ki);
        match op {
            Op::Put => {
                log.tag("put");
                let p = keys[ki].as_mut_ptr() as *mut c_char;
                let before = map_shape(t, elemsize);
                t = shput(lib, t, elemsize, p, mode, 8, rng.next_u64(), false);
                tk.after_put(before, map_shape(t, elemsize));
            }
            Op::Get => {
                log.tag("get");
                let p = keys[ki].as_mut_ptr() as *mut c_char;
                let (nt, idx) = shgeti(lib, t, elemsize, p, mode);
                t = nt;
                log.isz("idx", idx);
            }
            Op::GetTs => {
                log.tag("get_ts");
                let p = keys[ki].as_mut_ptr() as *mut c_char;
                let mut temp: isize = 0x4242;
                t = (lib.hmget_key_ts)(t, elemsize, p as *mut c_void, 8, &mut temp, mode);
                log.isz("temp", temp);
            }
            Op::Del => {
                log.tag("del");
                let p = keys[ki].as_mut_ptr() as *mut c_char;
                let (nt, d) = shdel(lib, t, elemsize, p, 0, mode);
                t = nt;
                log.isz("d", d);
                // a delete can free the key `temp_key` points at (SH_STRDUP)
                // and/or rebuild the table -> conservatively dead
                tk.invalidate();
            }
            Op::PutDefault => {
                log.tag("put_default");
                t = (lib.hmput_default)(t, elemsize);
                let raw = (t as *mut u8).wrapping_sub(elemsize);
                // only touch the bytes *after* the key pointer of element -1;
                // element -1's key pointer must stay NULL so the snapshot can
                // read it safely
                let mut b = rng.next_u64();
                for j in 8..elemsize {
                    *raw.wrapping_add(j) = (b & 0xff) as u8;
                    b = b.rotate_left(8);
                }
            }
        }
        snap_map_tkv(log, t, elemsize, KeyKind::StringAt(0), &tk);
    }
    hmfree(lib, t, elemsize);
}

#[test]
fn cfg54_fuzz_binary_streams() {
    diff("cfg54_binary", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 54);
        for &mode in &[HM_BINARY, -1, c_int::MIN] {
            for &start in &[false, true] {
                let hs = rng.next_u64() as usize;
                log.i32v("mode", mode);
                log.flag("shmode_start", start);
                fuzz_binary(lib, log, &mut rng, hs, 16, 8, 0, mode, 2000, start);
            }
        }
    });
}

#[test]
fn cfg54_fuzz_string_streams() {
    diff("cfg54_string", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x54_5);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &mode in &[HM_BINARY, HM_STRING, 2] {
                let hs = rng.next_u64() as usize;
                log.i32v("sh", sh);
                log.i32v("mode", mode);
                fuzz_string(lib, log, &mut rng, hs, 16, sh, mode, 1500, true);
            }
        }
    });
}

#[test]
fn cfg55_fuzz_randomized_shapes() {
    diff("cfg55", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 55);
        for run in 0..60usize {
            let keysize = rng.range(1, 24);
            let keyoffset = if rng.below(2) == 0 {
                0
            } else {
                // must not overlap the key at offset 0
                keysize + rng.below(9)
            };
            let elemsize = keyoffset + keysize + 8 + rng.below(17);
            let mode = *[HM_BINARY, -1].get(rng.below(2)).unwrap();
            let hs = rng.next_u64() as usize;
            let start_from_shmode = rng.below(2) == 0;
            log.usz("run", run);
            log.usz("keysize", keysize);
            log.usz("keyoffset", keyoffset);
            log.usz("elemsize", elemsize);
            log.i32v("mode", mode);
            fuzz_binary(
                lib,
                log,
                &mut rng,
                hs,
                elemsize,
                keysize,
                keyoffset,
                mode,
                200,
                start_from_shmode,
            );
        }
    });
}

#[test]
fn cfg55b_fuzz_randomized_string_shapes() {
    diff("cfg55b", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x55B);
        for run in 0..60usize {
            // Any elemsize >= 16 is allowed, including sizes that are NOT a
            // multiple of 8: the C then performs an *unaligned* `char *` store
            // at `a + elemsize*i` (c_src/src/lib.c:786-788), which the
            // translation must reproduce. (The snapshot helper uses
            // `read_unaligned` for exactly this reason.)
            let elemsize = 16 + rng.below(33);
            let sh = *[SH_DEFAULT, SH_STRDUP, SH_ARENA].get(rng.below(3)).unwrap();
            let mode = *[HM_BINARY, HM_STRING].get(rng.below(2)).unwrap();
            let hs = rng.next_u64() as usize;
            log.usz("run", run);
            log.usz("elemsize", elemsize);
            log.i32v("sh", sh);
            log.i32v("mode", mode);
            fuzz_string(lib, log, &mut rng, hs, elemsize, sh, mode, 200, true);
        }
    });
}

/// Delete-free string streams. Because nothing can free a key, this variant can
/// additionally compare `hash_index::temp_key` after every single operation,
/// covering both the refresh (first-loop duplicate hit / fresh insert) and the
/// deliberate no-refresh (wrap-around duplicate hit) branches.
#[test]
fn cfg54c_fuzz_string_streams_no_delete_with_temp_key() {
    diff("cfg54c", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 0x54C);
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &mode in &[HM_BINARY, HM_STRING, 2, 255, c_int::MAX] {
                let hs = rng.next_u64() as usize;
                log.i32v("sh", sh);
                log.i32v("mode", mode);
                fuzz_string(lib, log, &mut rng, hs, 24, sh, mode, 900, false);
            }
        }
    });
}
