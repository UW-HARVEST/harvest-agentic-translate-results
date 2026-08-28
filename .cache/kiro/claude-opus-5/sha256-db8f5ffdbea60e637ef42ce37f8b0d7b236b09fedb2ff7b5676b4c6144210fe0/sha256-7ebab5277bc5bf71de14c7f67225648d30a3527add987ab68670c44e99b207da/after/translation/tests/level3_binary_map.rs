//! Level 3: binary-keyed hash maps.
//!
//! `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_shmode_func` and
//! `stbds_hmfree_func` driven exactly as the stb_ds macros drive them.
//!
//! Every test holds `shared_lock()` because `stbds_make_hash_index` mutates the
//! library-global `stbds_hash_seed`, and the seed sequence must stay in lockstep
//! between the two libraries.

mod harness;

use harness::*;
use std::ffi::c_void;

/// One layout per column of stb_ds' own unit tests.
#[derive(Clone, Copy)]
struct Layout {
    name: &'static str,
    elemsize: usize,
    keysize: usize,
    value_offset: usize,
}

const LAYOUTS: &[Layout] = &[
    // struct { int key, value; }
    Layout {
        name: "int/int",
        elemsize: 8,
        keysize: 4,
        value_offset: 4,
    },
    // struct { size_t key; size_t value; }
    Layout {
        name: "u64/u64",
        elemsize: 16,
        keysize: 8,
        value_offset: 8,
    },
    // stbds_struct: struct { int key,b,c,d; }
    Layout {
        name: "stbds_struct",
        elemsize: 16,
        keysize: 4,
        value_offset: 4,
    },
    // stbds_struct2: struct { int key[2],b,c,d; }
    Layout {
        name: "stbds_struct2",
        elemsize: 20,
        keysize: 8,
        value_offset: 8,
    },
    // single-byte key, exercises the siphash tail
    Layout {
        name: "u8 key",
        elemsize: 8,
        keysize: 1,
        value_offset: 4,
    },
];

fn key_bytes(layout: &Layout, k: u64) -> Vec<u8> {
    // widen to the element size so hmput can copy `keysize` bytes safely
    let mut v = k.to_le_bytes().to_vec();
    v.resize(layout.elemsize.max(8), 0);
    v
}

fn value_bytes(v: u64) -> Vec<u8> {
    v.to_le_bytes()[..4].to_vec()
}

struct Maps {
    c: *mut c_void,
    rs: *mut c_void,
}

fn assert_same(m: &Maps, layout: &Layout, ctx: &str) {
    let defined = [(0usize, layout.keysize), (layout.value_offset, 4usize)];
    let (a, b) = unsafe {
        (
            snapshot_binary(m.c, layout.elemsize, &defined),
            snapshot_binary(m.rs, layout.elemsize, &defined),
        )
    };
    if a != b {
        panic!(
            "{} [{}] diverged\n C: {:?}\nRS: {:?}",
            ctx, layout.name, a, b
        );
    }
}

// ---------------------------------------------------------------------------
// stbds_shmode_func
// ---------------------------------------------------------------------------

#[test]
fn shmode_func_matches() {
    let _g = shared_lock();
    let p = pair();
    for &mode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for layout in LAYOUTS {
            unsafe {
                p.c.rand_seed(0x1234_5678);
                p.rs.rand_seed(0x1234_5678);
                let ct = p.c.shmode_func(layout.elemsize, mode);
                let rt = p.rs.shmode_func(layout.elemsize, mode);
                let m = Maps { c: ct, rs: rt };
                assert_same(&m, layout, &format!("shmode_func(mode={})", mode));
                assert_eq!(hmlen(ct, layout.elemsize), 0);
                hmfree(&p.c, ct, layout.elemsize);
                hmfree(&p.rs, rt, layout.elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn hmput_default_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        unsafe {
            // from NULL
            let mut ct = p.c.hmput_default(std::ptr::null_mut(), layout.elemsize);
            let mut rt = p.rs.hmput_default(std::ptr::null_mut(), layout.elemsize);
            let m = Maps { c: ct, rs: rt };
            assert_same(&m, layout, "hmput_default(NULL)");

            // hmdefault(t, v): (t)[-1].value = v
            *((ct as *mut u8).sub(layout.elemsize).add(layout.value_offset) as *mut i32) = 4242;
            *((rt as *mut u8).sub(layout.elemsize).add(layout.value_offset) as *mut i32) = 4242;

            // idempotent on a non-empty map
            ct = p.c.hmput_default(ct, layout.elemsize);
            rt = p.rs.hmput_default(rt, layout.elemsize);
            let m = Maps { c: ct, rs: rt };
            assert_same(&m, layout, "hmput_default(existing)");

            // a miss must report the default slot index -1
            let k = key_bytes(layout, 99);
            let (ct2, ci) = hmgeti(&p.c, ct, layout.elemsize, &k, layout.keysize);
            let (rt2, ri) = hmgeti(&p.rs, rt, layout.elemsize, &k, layout.keysize);
            assert_eq!(ci, ri, "hmgeti on default-only map [{}]", layout.name);
            let m = Maps { c: ct2, rs: rt2 };
            assert_same(&m, layout, "hmgeti after hmput_default");

            hmfree(&p.c, ct2, layout.elemsize);
            hmfree(&p.rs, rt2, layout.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// get on a NULL map (allocates the default slot)
// ---------------------------------------------------------------------------

#[test]
fn get_on_null_map_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        let k = key_bytes(layout, 7);
        unsafe {
            let (ct, ci) = hmgeti(&p.c, std::ptr::null_mut(), layout.elemsize, &k, layout.keysize);
            let (rt, ri) = hmgeti(&p.rs, std::ptr::null_mut(), layout.elemsize, &k, layout.keysize);
            assert_eq!(ci, ri, "hmgeti(NULL) index [{}]", layout.name);
            assert_same(&Maps { c: ct, rs: rt }, layout, "hmgeti(NULL)");
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);

            let (ct, ci) =
                hmgeti_ts(&p.c, std::ptr::null_mut(), layout.elemsize, &k, layout.keysize);
            let (rt, ri) =
                hmgeti_ts(&p.rs, std::ptr::null_mut(), layout.elemsize, &k, layout.keysize);
            assert_eq!(ci, ri, "hmgeti_ts(NULL) temp [{}]", layout.name);
            assert_same(&Maps { c: ct, rs: rt }, layout, "hmgeti_ts(NULL)");
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

/// `stbds_hmdel_key(NULL, ...)` must return NULL from both.
#[test]
fn del_on_null_map_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        let k = key_bytes(layout, 7);
        unsafe {
            let (ct, cr) = hmdel(
                &p.c,
                std::ptr::null_mut(),
                layout.elemsize,
                &k,
                layout.keysize,
                0,
            );
            let (rt, rr) = hmdel(
                &p.rs,
                std::ptr::null_mut(),
                layout.elemsize,
                &k,
                layout.keysize,
                0,
            );
            assert!(ct.is_null() && rt.is_null(), "[{}]", layout.name);
            assert_eq!(cr, rr);
        }
    }
}

// ---------------------------------------------------------------------------
// sequential inserts: covers table creation and every growth step
// ---------------------------------------------------------------------------

#[test]
fn sequential_inserts_match() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        unsafe {
            p.c.rand_seed(0xC0FF_EE01);
            p.rs.rand_seed(0xC0FF_EE01);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            let n: u64 = if layout.keysize == 1 { 200 } else { 400 };
            for i in 0..n {
                let k = key_bytes(layout, if layout.keysize == 1 { i % 256 } else { i });
                let v = value_bytes(i.wrapping_mul(7).wrapping_add(3));
                ct = hmput(
                    &p.c,
                    ct,
                    layout.elemsize,
                    &k,
                    layout.keysize,
                    &v,
                    layout.value_offset,
                );
                rt = hmput(
                    &p.rs,
                    rt,
                    layout.elemsize,
                    &k,
                    layout.keysize,
                    &v,
                    layout.value_offset,
                );
                assert_same(
                    &Maps { c: ct, rs: rt },
                    layout,
                    &format!("insert #{}", i),
                );
            }
            // read every key back
            for i in 0..n {
                let k = key_bytes(layout, if layout.keysize == 1 { i % 256 } else { i });
                let (c2, ci) = hmgeti(&p.c, ct, layout.elemsize, &k, layout.keysize);
                let (r2, ri) = hmgeti(&p.rs, rt, layout.elemsize, &k, layout.keysize);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "lookup {} [{}]", i, layout.name);
                assert!(ci >= 0, "key {} should be present [{}]", i, layout.name);
            }
            // misses
            for i in n..(n + 50) {
                let k = key_bytes(layout, i + 100_000);
                let (c2, ci) = hmgeti(&p.c, ct, layout.elemsize, &k, layout.keysize);
                let (r2, ri) = hmgeti(&p.rs, rt, layout.elemsize, &k, layout.keysize);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "miss {} [{}]", i, layout.name);
            }
            assert_same(&Maps { c: ct, rs: rt }, layout, "after lookups");
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// overwriting an existing key: hits the "key found" branches of hmput_key
// ---------------------------------------------------------------------------

#[test]
fn repeated_inserts_of_same_keys_match() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        unsafe {
            p.c.rand_seed(0x5EED_5EED);
            p.rs.rand_seed(0x5EED_5EED);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            let mut rng = Rng::new(0xBEEF);
            for round in 0..600 {
                let key = rng.below(40);
                let k = key_bytes(layout, key);
                let v = value_bytes(round);
                ct = hmput(
                    &p.c,
                    ct,
                    layout.elemsize,
                    &k,
                    layout.keysize,
                    &v,
                    layout.value_offset,
                );
                rt = hmput(
                    &p.rs,
                    rt,
                    layout.elemsize,
                    &k,
                    layout.keysize,
                    &v,
                    layout.value_offset,
                );
                assert_same(
                    &Maps { c: ct, rs: rt },
                    layout,
                    &format!("round {} key {}", round, key),
                );
            }
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// delete: tombstones, the tail-swap, the shrink and the rebuild paths
// ---------------------------------------------------------------------------

#[test]
fn delete_all_in_insertion_order_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        if layout.keysize == 1 {
            continue; // too few distinct keys for a meaningful shrink walk
        }
        unsafe {
            p.c.rand_seed(0xDEAD_0001);
            p.rs.rand_seed(0xDEAD_0001);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            let n: u64 = 300;
            for i in 0..n {
                let k = key_bytes(layout, i);
                let v = value_bytes(i);
                ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
                rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            }
            assert_same(&Maps { c: ct, rs: rt }, layout, "before deletes");
            for i in 0..n {
                let k = key_bytes(layout, i);
                let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
                let (r2, rr) = hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "hmdel result for {} [{}]", i, layout.name);
                assert_same(&Maps { c: ct, rs: rt }, layout, &format!("delete #{}", i));
            }
            assert_eq!(hmlen(ct, layout.elemsize), 0);
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

#[test]
fn delete_in_reverse_order_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        if layout.keysize == 1 {
            continue;
        }
        unsafe {
            p.c.rand_seed(0xDEAD_0002);
            p.rs.rand_seed(0xDEAD_0002);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            let n: u64 = 200;
            for i in 0..n {
                let k = key_bytes(layout, i);
                let v = value_bytes(i);
                ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
                rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            }
            for i in (0..n).rev() {
                let k = key_bytes(layout, i);
                let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
                let (r2, rr) = hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "hmdel result for {} [{}]", i, layout.name);
                assert_same(&Maps { c: ct, rs: rt }, layout, &format!("rdelete #{}", i));
            }
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

#[test]
fn delete_missing_keys_matches() {
    let _g = shared_lock();
    let p = pair();
    for layout in LAYOUTS {
        unsafe {
            p.c.rand_seed(0xDEAD_0003);
            p.rs.rand_seed(0xDEAD_0003);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            for i in 0..20u64 {
                let k = key_bytes(layout, i);
                let v = value_bytes(i);
                ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
                rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            }
            for i in 1000..1050u64 {
                let k = key_bytes(layout, i);
                let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
                let (r2, rr) = hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "hmdel(miss {}) [{}]", i, layout.name);
                assert_same(&Maps { c: ct, rs: rt }, layout, "delete miss");
            }
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// randomised mixed workload
// ---------------------------------------------------------------------------

#[test]
fn randomised_mixed_workload_matches() {
    let _g = shared_lock();
    let p = pair();
    for (trial, layout) in LAYOUTS.iter().enumerate() {
        for seed in [1u64, 2, 3] {
            unsafe {
                let hs = 0x1000_0000usize + trial * 977 + seed as usize;
                p.c.rand_seed(hs);
                p.rs.rand_seed(hs);
                let mut ct: *mut c_void = std::ptr::null_mut();
                let mut rt: *mut c_void = std::ptr::null_mut();
                let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9));
                let space = if layout.keysize == 1 { 64 } else { 250 };

                for op in 0..1200 {
                    let key = rng.below(space);
                    let k = key_bytes(layout, key);
                    match rng.below(10) {
                        0..=4 => {
                            let v = value_bytes(op);
                            ct = hmput(
                                &p.c, ct, layout.elemsize, &k, layout.keysize, &v,
                                layout.value_offset,
                            );
                            rt = hmput(
                                &p.rs, rt, layout.elemsize, &k, layout.keysize, &v,
                                layout.value_offset,
                            );
                        }
                        5..=6 => {
                            let (c2, ci) = hmgeti(&p.c, ct, layout.elemsize, &k, layout.keysize);
                            let (r2, ri) = hmgeti(&p.rs, rt, layout.elemsize, &k, layout.keysize);
                            ct = c2;
                            rt = r2;
                            assert_eq!(ci, ri, "op {} get {} [{}]", op, key, layout.name);
                        }
                        7 => {
                            let (c2, ci) = hmgeti_ts(&p.c, ct, layout.elemsize, &k, layout.keysize);
                            let (r2, ri) =
                                hmgeti_ts(&p.rs, rt, layout.elemsize, &k, layout.keysize);
                            ct = c2;
                            rt = r2;
                            assert_eq!(ci, ri, "op {} get_ts {} [{}]", op, key, layout.name);
                        }
                        _ => {
                            let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
                            let (r2, rr) =
                                hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
                            ct = c2;
                            rt = r2;
                            assert_eq!(cr, rr, "op {} del {} [{}]", op, key, layout.name);
                        }
                    }
                    assert_same(
                        &Maps { c: ct, rs: rt },
                        layout,
                        &format!("seed {} op {}", seed, op),
                    );
                }
                hmfree(&p.c, ct, layout.elemsize);
                hmfree(&p.rs, rt, layout.elemsize);
            }
        }
    }
}

/// Insert/delete churn on a *small* key space keeps `tombstone_count` climbing,
/// which is the only way to reach the rebuild-at-same-size branch of
/// `stbds_hmdel_key`.
#[test]
fn tombstone_rebuild_and_shrink_paths_match() {
    let _g = shared_lock();
    let p = pair();
    let layout = &LAYOUTS[1]; // u64/u64
    unsafe {
        p.c.rand_seed(0x7777_1111);
        p.rs.rand_seed(0x7777_1111);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();

        // fill to a decent slot_count first
        for i in 0..500u64 {
            let k = key_bytes(layout, i);
            let v = value_bytes(i);
            ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
        }
        let sc = snapshot_binary(ct, layout.elemsize, &[]).slot_count;
        assert!(sc >= 512, "expected a grown table, got slot_count {}", sc);

        // churn: delete then reinsert the same key over and over
        for i in 0..3000u64 {
            let key = 100 + (i % 40);
            let k = key_bytes(layout, key);
            let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
            let (r2, rr) = hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
            ct = c2;
            rt = r2;
            assert_eq!(cr, rr, "churn del {}", i);
            assert_same(&Maps { c: ct, rs: rt }, layout, &format!("churn del {}", i));

            let v = value_bytes(i);
            ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
            assert_same(&Maps { c: ct, rs: rt }, layout, &format!("churn put {}", i));
        }

        // drain everything to force repeated shrinks
        for i in 0..500u64 {
            let k = key_bytes(layout, i);
            let (c2, cr) = hmdel(&p.c, ct, layout.elemsize, &k, layout.keysize, 0);
            let (r2, rr) = hmdel(&p.rs, rt, layout.elemsize, &k, layout.keysize, 0);
            ct = c2;
            rt = r2;
            assert_eq!(cr, rr, "drain del {}", i);
            assert_same(&Maps { c: ct, rs: rt }, layout, &format!("drain {}", i));
        }
        assert_eq!(snapshot_binary(ct, layout.elemsize, &[]).slot_count, 8);
        hmfree(&p.c, ct, layout.elemsize);
        hmfree(&p.rs, rt, layout.elemsize);
    }
}

/// A binary map created through `sh_new_arena`/`sh_new_strdup` keeps
/// `string.mode` set even though the keys are binary; `hmput_key`'s switch then
/// takes the string branches. This is unusual but it is what the C code does.
#[test]
fn binary_map_over_preset_string_modes_matches() {
    let _g = shared_lock();
    let p = pair();
    let layout = &LAYOUTS[1];
    for &mode in &[SH_NONE, SH_DEFAULT] {
        unsafe {
            p.c.rand_seed(0x3333_2222);
            p.rs.rand_seed(0x3333_2222);
            let mut ct = p.c.shmode_func(layout.elemsize, mode);
            let mut rt = p.rs.shmode_func(layout.elemsize, mode);
            for i in 0..60u64 {
                let k = key_bytes(layout, i);
                let v = value_bytes(i);
                ct = hmput(&p.c, ct, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
                rt = hmput(&p.rs, rt, layout.elemsize, &k, layout.keysize, &v, layout.value_offset);
                assert_same(
                    &Maps { c: ct, rs: rt },
                    layout,
                    &format!("preset mode {} insert {}", mode, i),
                );
            }
            hmfree(&p.c, ct, layout.elemsize);
            hmfree(&p.rs, rt, layout.elemsize);
        }
    }
}
