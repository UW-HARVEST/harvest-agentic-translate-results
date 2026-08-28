//! Level 3: the binary-keyed hash map – `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key` and
//! `stbds_hmfree_func`.

mod common;

use common::*;
use std::ffi::c_void;

/// (keysize, elemsize) pairs mirroring realistic stb_ds element layouts,
/// including `stbds_struct` (int key + 3 ints) and `stbds_struct2`
/// (int key[2] + 3 ints).
const LAYOUTS: &[(usize, usize)] = &[
    (1, 2),
    (2, 4),
    (4, 8),
    (4, 16),
    (8, 16),
    (8, 20),
    (8, 24),
    (16, 32),
];

fn key_bytes(keysize: usize, n: i64) -> Vec<u8> {
    let mut v = vec![0u8; keysize];
    let le = n.to_le_bytes();
    for i in 0..keysize.min(8) {
        v[i] = le[i];
    }
    // fill any remaining bytes deterministically so wide keys differ too
    for i in 8..keysize {
        v[i] = (n as u8).wrapping_mul(31).wrapping_add(i as u8);
    }
    v
}

fn value_bytes(vsize: usize, n: i64) -> Vec<u8> {
    (0..vsize)
        .map(|i| (n as u8).wrapping_mul(7).wrapping_add(i as u8))
        .collect()
}

struct Map {
    t: *mut u8,
}

impl Map {
    fn new() -> Map {
        Map {
            t: std::ptr::null_mut(),
        }
    }
}

/// Run the same operation script against both libraries, comparing the full map
/// state after every single step.
#[derive(Clone, Copy, Debug)]
enum Op {
    Put(i64),
    Get(i64),
    GetTs(i64),
    Del(i64),
    PutDefault,
}

fn run_script(c: &Lib, r: &Lib, keysize: usize, elemsize: usize, seed: usize, ops: &[Op], tag: &str) {
    let _guard = serial();
    let vsize = elemsize - keysize;
    let mut cm = Map::new();
    let mut rm = Map::new();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);

        for (i, op) in ops.iter().enumerate() {
            let ctx = format!(
                "{tag} keysize={keysize} elemsize={elemsize} seed={seed:#x} step={i} op={op:?}"
            );
            match *op {
                Op::Put(n) => {
                    let k = key_bytes(keysize, n);
                    let v = value_bytes(vsize, n);
                    cm.t = hmput_bytes(c, cm.t, elemsize, &k, &v);
                    rm.t = hmput_bytes(r, rm.t, elemsize, &k, &v);
                }
                Op::Get(n) => {
                    let k = key_bytes(keysize, n);
                    let (ct, ci) = hmgeti_bytes(c, cm.t, elemsize, &k);
                    let (rt, ri) = hmgeti_bytes(r, rm.t, elemsize, &k);
                    cm.t = ct;
                    rm.t = rt;
                    assert_eq!(ci, ri, "hmgeti index mismatch: {ctx}");
                }
                Op::GetTs(n) => {
                    let k = key_bytes(keysize, n);
                    let mut ck = k.clone();
                    let mut rk = k.clone();
                    let mut ctmp: isize = 0xAAAA;
                    let mut rtmp: isize = 0xAAAA;
                    cm.t = (c.hmget_key_ts)(
                        cm.t as *mut c_void,
                        elemsize,
                        ck.as_mut_ptr() as *mut c_void,
                        keysize,
                        &mut ctmp,
                        HM_BINARY,
                    ) as *mut u8;
                    rm.t = (r.hmget_key_ts)(
                        rm.t as *mut c_void,
                        elemsize,
                        rk.as_mut_ptr() as *mut c_void,
                        keysize,
                        &mut rtmp,
                        HM_BINARY,
                    ) as *mut u8;
                    assert_eq!(ctmp, rtmp, "hmget_key_ts temp mismatch: {ctx}");
                }
                Op::Del(n) => {
                    let k = key_bytes(keysize, n);
                    let (ct, cr) = hmdel_bytes(c, cm.t, elemsize, &k);
                    let (rt, rr) = hmdel_bytes(r, rm.t, elemsize, &k);
                    cm.t = ct;
                    rm.t = rt;
                    assert_eq!(cr, rr, "hmdel result mismatch: {ctx}");
                }
                Op::PutDefault => {
                    cm.t = (c.hmput_default)(cm.t as *mut c_void, elemsize) as *mut u8;
                    rm.t = (r.hmput_default)(rm.t as *mut c_void, elemsize) as *mut u8;
                }
            }
            let cs = snapshot(cm.t, elemsize, false);
            let rs = snapshot(rm.t, elemsize, false);
            assert_eq!(cs, rs, "map state mismatch: {ctx}");
        }

        if !cm.t.is_null() {
            (c.hmfree_func)(cm.t.sub(elemsize) as *mut c_void, elemsize);
        }
        if !rm.t.is_null() {
            (r.hmfree_func)(rm.t.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state >> 11
}

#[test]
fn hm_sequential_insert_matches() {
    let (c, r) = both();
    let ops: Vec<Op> = (0..200i64).map(Op::Put).collect();
    for &(ks, es) in LAYOUTS {
        run_script(&c, &r, ks, es, 0x31415926, &ops, "sequential-insert");
    }
}

#[test]
fn hm_insert_then_lookup_matches() {
    let (c, r) = both();
    let mut ops: Vec<Op> = (0..64i64).map(Op::Put).collect();
    // hits and misses, interleaved with the _ts variant
    for n in -8..80i64 {
        ops.push(Op::Get(n));
        ops.push(Op::GetTs(n));
    }
    // overwriting existing keys must reuse their slot
    for n in 0..64i64 {
        ops.push(Op::Put(n));
    }
    for &(ks, es) in LAYOUTS {
        run_script(&c, &r, ks, es, 0x31415926, &ops, "insert-lookup");
    }
}

#[test]
fn hm_lookup_on_empty_and_default_matches() {
    let (c, r) = both();
    // hmget / hmdel / hmput_default against a null or table-less map
    let scripts: Vec<Vec<Op>> = vec![
        vec![Op::Get(1), Op::Get(1), Op::Get(2)],
        vec![Op::GetTs(1), Op::GetTs(2)],
        vec![Op::Del(1), Op::Del(2)],
        vec![Op::PutDefault, Op::PutDefault, Op::Get(1), Op::Del(1)],
        vec![Op::PutDefault, Op::Put(5), Op::Get(5), Op::Del(5), Op::Get(5)],
        vec![Op::Get(3), Op::Put(3), Op::Get(3), Op::Del(3), Op::Del(3)],
        vec![Op::Del(9), Op::Put(9), Op::PutDefault, Op::Get(9)],
    ];
    for &(ks, es) in LAYOUTS {
        for (i, s) in scripts.iter().enumerate() {
            run_script(&c, &r, ks, es, 0x31415926, s, &format!("edge-{i}"));
        }
    }
}

#[test]
fn hm_delete_tombstones_and_shrink_match() {
    let (c, r) = both();
    // grow well past several table doublings, then delete most keys to trigger
    // both the tombstone rebuild and the shrink path.
    let mut ops: Vec<Op> = (0..300i64).map(Op::Put).collect();
    for n in 0..300i64 {
        ops.push(Op::Del(n));
        ops.push(Op::Get(n));
    }
    // reinsert into the shrunken table
    for n in 300..360i64 {
        ops.push(Op::Put(n));
    }
    for &(ks, es) in LAYOUTS {
        run_script(&c, &r, ks, es, 0x31415926, &ops, "delete-shrink");
    }
}

#[test]
fn hm_interleaved_delete_reinsert_matches() {
    let (c, r) = both();
    // deleting a non-final entry moves the last element into the hole and
    // rewrites its slot index – exercise that path heavily.
    let mut ops: Vec<Op> = Vec::new();
    for n in 0..80i64 {
        ops.push(Op::Put(n));
    }
    for n in (0..80i64).step_by(3) {
        ops.push(Op::Del(n));
    }
    for n in 0..80i64 {
        ops.push(Op::Get(n));
        ops.push(Op::Put(n));
    }
    for n in (1..80i64).step_by(2) {
        ops.push(Op::Del(n));
        ops.push(Op::Put(n + 1000));
    }
    for &(ks, es) in LAYOUTS {
        run_script(&c, &r, ks, es, 0x31415926, &ops, "delete-reinsert");
    }
}

#[test]
fn hm_random_workload_matches() {
    let (c, r) = both();
    for &seed in &[0usize, 1, 0x31415926, usize::MAX, 0xDEAD_BEEFu64 as usize] {
        let mut st = seed as u64 ^ 0x9E37_79B9_7F4A_7C15;
        let mut ops: Vec<Op> = Vec::new();
        for _ in 0..900 {
            let x = lcg(&mut st);
            let n = (x % 120) as i64 - 10;
            match x % 10 {
                0..=4 => ops.push(Op::Put(n)),
                5..=6 => ops.push(Op::Get(n)),
                7 => ops.push(Op::GetTs(n)),
                8 => ops.push(Op::Del(n)),
                _ => ops.push(Op::PutDefault),
            }
        }
        for &(ks, es) in &[(4usize, 8usize), (8, 16), (8, 20)] {
            run_script(&c, &r, ks, es, seed, &ops, "random");
        }
    }
}

#[test]
fn hm_keys_with_high_bit_bytes_match() {
    let (c, r) = both();
    // keys whose byte 3 / byte 7 have the high bit set exercise the C
    // `int` sign-extension quirk inside stbds_siphash_bytes.
    let mut ops: Vec<Op> = Vec::new();
    for n in 0..100i64 {
        let k = -(n * 0x0100_0000 + 0x8080_8080);
        ops.push(Op::Put(k));
        ops.push(Op::Get(k));
    }
    for n in 0..100i64 {
        ops.push(Op::Del(-(n * 0x0100_0000 + 0x8080_8080)));
    }
    for &(ks, es) in LAYOUTS {
        run_script(&c, &r, ks, es, 0x31415926, &ops, "high-bit-keys");
    }
}

#[test]
fn hmfree_func_on_null_matches() {
    let (c, r) = both();
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), 16);
        (r.hmfree_func)(std::ptr::null_mut(), 16);
    }
}
