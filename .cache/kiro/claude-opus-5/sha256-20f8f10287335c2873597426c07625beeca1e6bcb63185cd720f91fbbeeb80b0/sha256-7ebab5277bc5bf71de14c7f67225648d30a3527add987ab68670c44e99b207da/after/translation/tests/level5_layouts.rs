//! Level 5: the same hash-map API driven with a range of element and key sizes,
//! mirroring the struct shapes the C source itself declares
//! (`stbds_struct { int key,b,c,d; }`, `stbds_struct2 { int key[2],b,c,d; }`)
//! plus smaller and larger layouts.
//!
//! Every byte of an element is written by the driver, so whole elements can be
//! compared without tripping over uninitialised padding.

mod common;

use common::*;
use std::ffi::c_void;

#[derive(Debug, Clone, Copy)]
enum Op {
    Put(u64),
    Get(u64),
    GetTs(u64),
    Del(u64),
    Default,
}

#[derive(Debug, PartialEq, Eq)]
struct Step {
    ret: isize,
    snap: MapSnap,
}

/// Builds a `keysize`-byte key from a small integer.
fn key_bytes(k: u64, keysize: usize) -> Vec<u8> {
    let mut v = vec![0u8; keysize.max(8)];
    v[..8].copy_from_slice(&k.to_le_bytes());
    v.truncate(keysize);
    v
}

unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    unsafe {
        let raw = (t as *mut u8).sub(elemsize) as *mut ArrayHeader;
        (*raw.sub(1)).temp
    }
}

unsafe fn run(api: &Api, seed: usize, elemsize: usize, keysize: usize, ops: &[Op]) -> Vec<Step> {
    unsafe {
        (api.rand_seed)(seed);
        let ranges = [(0usize, elemsize)];
        let mut t: *mut c_void = std::ptr::null_mut();
        let mut out = Vec::with_capacity(ops.len());

        for &op in ops {
            let ret;
            match op {
                Op::Put(k) => {
                    let mut kb = key_bytes(k, keysize);
                    t = (api.hmput_key)(
                        t,
                        elemsize,
                        kb.as_mut_ptr() as *mut c_void,
                        keysize,
                        HM_BINARY,
                    );
                    let idx = temp_of(t, elemsize);
                    // hmput(t,k,v): write the whole element -- key then payload
                    let e = (t as *mut u8).offset(elemsize as isize * idx);
                    std::ptr::copy_nonoverlapping(kb.as_ptr(), e, keysize);
                    for j in keysize..elemsize {
                        *e.add(j) = (k as u8).wrapping_mul(37).wrapping_add(j as u8);
                    }
                    ret = idx;
                }
                Op::Get(k) => {
                    let mut kb = key_bytes(k, keysize);
                    t = (api.hmget_key)(
                        t,
                        elemsize,
                        kb.as_mut_ptr() as *mut c_void,
                        keysize,
                        HM_BINARY,
                    );
                    ret = temp_of(t, elemsize);
                }
                Op::GetTs(k) => {
                    let mut kb = key_bytes(k, keysize);
                    let mut tmp: isize = 0;
                    t = (api.hmget_key_ts)(
                        t,
                        elemsize,
                        kb.as_mut_ptr() as *mut c_void,
                        keysize,
                        &raw mut tmp,
                        HM_BINARY,
                    );
                    ret = tmp;
                }
                Op::Del(k) => {
                    let mut kb = key_bytes(k, keysize);
                    t = (api.hmdel_key)(
                        t,
                        elemsize,
                        kb.as_mut_ptr() as *mut c_void,
                        keysize,
                        0,
                        HM_BINARY,
                    );
                    ret = if t.is_null() {
                        0
                    } else {
                        temp_of(t, elemsize)
                    };
                }
                Op::Default => {
                    t = (api.hmput_default)(t, elemsize);
                    ret = 0;
                }
            }
            out.push(Step {
                ret,
                snap: map_snap(t, elemsize, false, &ranges),
            });
        }

        if !t.is_null() {
            (api.hmfree_func)((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        out
    }
}

/// (elemsize, keysize) pairs. 16/4 is `stbds_struct`, 20/8 is `stbds_struct2`.
const LAYOUTS: &[(usize, usize)] = &[
    (1, 1),
    (2, 1),
    (2, 2),
    (4, 4),
    (8, 4),
    (8, 8),
    (16, 4),
    (20, 8),
    (24, 16),
    (32, 8),
    (48, 32),
    (64, 64),
];

const SEEDS: [usize; 2] = [0x3141_5926, 0xabcd_1234_5678_9f01];

fn compare(name: &str, seed: usize, elemsize: usize, keysize: usize, ops: &[Op]) {
    let _guard = global_lock();
    let (c, r) = both();
    let a = unsafe { run(&c, seed, elemsize, keysize, ops) };
    let b = unsafe { run(&r, seed, elemsize, keysize, ops) };
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x, y,
            "{name} elemsize={elemsize} keysize={keysize} seed={seed:#x} step {i} {:?}",
            ops[i]
        );
    }
}

#[test]
fn all_layouts_basic() {
    for &(e, k) in LAYOUTS {
        for seed in SEEDS {
            // key space is limited by keysize (1 byte -> 256 distinct keys)
            let span: u64 = if k == 1 { 200 } else { 60 };
            let mut ops: Vec<Op> = vec![Op::Default];
            for i in 0..span {
                ops.push(Op::Put(i));
            }
            for i in 0..span {
                ops.push(Op::Get(i));
            }
            ops.push(Op::Get(span + 5));
            ops.push(Op::GetTs(span + 5));
            compare("basic", seed, e, k, &ops);
        }
    }
}

#[test]
fn all_layouts_deletes() {
    for &(e, k) in LAYOUTS {
        for seed in SEEDS {
            let span: u64 = if k == 1 { 120 } else { 48 };
            let mut ops: Vec<Op> = (0..span).map(Op::Put).collect();
            for i in (0..span).step_by(2) {
                ops.push(Op::Del(i));
                ops.push(Op::Get(i));
            }
            for i in (0..span).rev() {
                ops.push(Op::Del(i));
            }
            for i in 0..span {
                ops.push(Op::Get(i));
                ops.push(Op::Put(i));
            }
            compare("deletes", seed, e, k, &ops);
        }
    }
}

#[test]
fn all_layouts_random() {
    for &(e, k) in LAYOUTS {
        let seed = SEEDS[0];
        let mut state: u64 = 0x243f_6a88_85a3_08d3 ^ (e as u64) << 32 ^ k as u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let span: u64 = if k == 1 { 150 } else { 80 };
        let mut ops = Vec::new();
        for _ in 0..900 {
            let v = next();
            let key = (v >> 11) % span;
            match v & 7 {
                0 | 1 | 2 => ops.push(Op::Put(key)),
                3 | 4 => ops.push(Op::Get(key)),
                5 => ops.push(Op::GetTs(key)),
                6 => ops.push(Op::Del(key)),
                _ => ops.push(Op::Default),
            }
        }
        compare("random", seed, e, k, &ops);
    }
}

/// A map created by `stbds_shmode_func` with `STBDS_SH_NONE` stores keys via
/// the `default:` arm of `hmput_key`'s switch (a plain `memcpy`), i.e. exactly
/// like a binary map that grew its own table.
#[test]
fn shmode_none_then_binary_puts() {
    for &(e, k) in LAYOUTS {
        for seed in SEEDS {
            let _guard = global_lock();
            let (c, r) = both();
            let span: u64 = if k == 1 { 100 } else { 40 };
            let drive = |api: &Api| unsafe {
                (api.rand_seed)(seed);
                let mut t = (api.shmode_func)(e, SH_NONE);
                let ranges = [(0usize, e)];
                let mut out = vec![map_snap(t, e, false, &ranges)];
                for i in 0..span {
                    let mut kb = key_bytes(i, k);
                    t = (api.hmput_key)(t, e, kb.as_mut_ptr() as *mut c_void, k, HM_BINARY);
                    let idx = temp_of(t, e);
                    let el = (t as *mut u8).offset(e as isize * idx);
                    std::ptr::copy_nonoverlapping(kb.as_ptr(), el, k);
                    for j in k..e {
                        *el.add(j) = (i as u8).wrapping_mul(29).wrapping_add(j as u8);
                    }
                    out.push(map_snap(t, e, false, &ranges));
                }
                for i in 0..span {
                    let mut kb = key_bytes(i, k);
                    t = (api.hmget_key)(t, e, kb.as_mut_ptr() as *mut c_void, k, HM_BINARY);
                    out.push(map_snap(t, e, false, &ranges));
                }
                for i in (0..span).step_by(3) {
                    let mut kb = key_bytes(i, k);
                    t = (api.hmdel_key)(t, e, kb.as_mut_ptr() as *mut c_void, k, 0, HM_BINARY);
                    out.push(map_snap(t, e, false, &ranges));
                }
                (api.hmfree_func)((t as *mut u8).sub(e) as *mut c_void, e);
                out
            };
            let a = drive(&c);
            let b = drive(&r);
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                assert_eq!(x, y, "shmode NONE elemsize={e} keysize={k} step {i}");
            }
        }
    }
}

/// `stbds_make_hash_index` places the bucket array at
/// `ALIGN_FWD((char*)(t+1), 64)`, so the offset of `storage` relative to the
/// table encodes `sizeof(stbds_hash_index)`. Comparing it catches any layout
/// drift between the C struct and the Rust `#[repr(C)]` mirror.
#[test]
fn hash_index_layout_matches() {
    let _guard = global_lock();
    let (c, r) = both();
    let probe = |api: &Api| unsafe {
        (api.rand_seed)(0x3141_5926);
        let mut offsets = Vec::new();
        for &(e, k) in LAYOUTS {
            let mut t: *mut c_void = std::ptr::null_mut();
            // enough inserts to force two table growths (8 -> 16 -> 32 slots)
            for i in 0..14u64 {
                let mut kb = key_bytes(i, k);
                t = (api.hmput_key)(t, e, kb.as_mut_ptr() as *mut c_void, k, HM_BINARY);
                let idx = temp_of(t, e);
                let el = (t as *mut u8).offset(e as isize * idx);
                std::ptr::copy_nonoverlapping(kb.as_ptr(), el, k);
                for j in k..e {
                    *el.add(j) = 0;
                }
            }
            let h = ((t as *mut u8).sub(e) as *mut ArrayHeader).sub(1);
            let tbl = (*h).hash_table as *mut HashIndex;
            // What the offset must be if sizeof(stbds_hash_index) == 104:
            //   ALIGN_FWD((size_t)(t+1), 64) - (size_t) t
            let base = tbl as usize;
            let expected = ((base + 104 + 63) & !63usize) - base;
            offsets.push((((*tbl).storage as usize) - base, expected));
            (api.hmfree_func)((t as *mut u8).sub(e) as *mut c_void, e);
        }
        offsets
    };
    let a = probe(&c);
    let b = probe(&r);
    assert_eq!(a, b, "storage offset within stbds_hash_index");
    for (got, expected) in &a {
        assert_eq!(
            got, expected,
            "bucket storage offset implies sizeof(stbds_hash_index) != 104"
        );
    }
}
