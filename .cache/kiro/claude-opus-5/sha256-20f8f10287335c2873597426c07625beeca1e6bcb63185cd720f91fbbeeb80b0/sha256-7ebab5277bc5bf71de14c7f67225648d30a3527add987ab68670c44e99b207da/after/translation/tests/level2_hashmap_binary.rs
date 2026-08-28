//! Level 2: binary-keyed hash maps.
//!   stbds_hmput_key, stbds_hmget_key, stbds_hmget_key_ts,
//!   stbds_hmput_default, stbds_hmdel_key, stbds_hmfree_func, stbds_rand_seed
//!
//! The driver replays the exact macro expansions from `lib.c`:
//!   hmput(t,k,v)  -> t = hmput_key(t, sizeof*t, &k, sizeof t->key, 0);
//!                    t[temp(t-1)].key = k; t[temp(t-1)].value = v;
//!   hmgeti(t,k)   -> t = hmget_key(t, sizeof*t, &k, sizeof t->key, HM_BINARY);
//!                    temp(t-1)
//!   hmdel(t,k)    -> t = hmdel_key(t, sizeof*t, &k, sizeof t->key, 0,
//!                                  HM_BINARY); t ? temp(t-1) : 0

mod common;

use common::*;
use std::ffi::c_void;

/// `typedef struct { int key, value; }` -- 8 bytes, no padding.
const ELEM: usize = 8;
const KEYSZ: usize = 4;
const RANGES: &[(usize, usize)] = &[(0, 8)];

#[derive(Debug, Clone, Copy)]
enum Op {
    Put(i32, i32),
    Get(i32),
    GetTs(i32),
    Del(i32),
    Default(i32),
    PutDefaultOnly,
}

#[derive(Debug, PartialEq, Eq)]
struct Step {
    ret: isize,
    snap: MapSnap,
}

unsafe fn temp_of(t: *mut c_void) -> isize {
    unsafe {
        let raw = (t as *mut u8).sub(ELEM) as *mut ArrayHeader;
        (*raw.sub(1)).temp
    }
}

unsafe fn run(api: &Api, seed: usize, ops: &[Op]) -> Vec<Step> {
    unsafe {
        (api.rand_seed)(seed);
        let mut t: *mut c_void = std::ptr::null_mut();
        let mut out = Vec::with_capacity(ops.len());

        for &op in ops {
            let ret;
            match op {
                Op::Put(k, v) => {
                    let mut key = k;
                    t = (api.hmput_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, HM_BINARY);
                    let idx = temp_of(t);
                    let e = (t as *mut u8).offset(ELEM as isize * idx) as *mut i32;
                    *e = k;
                    *e.add(1) = v;
                    ret = idx;
                }
                Op::Get(k) => {
                    let mut key = k;
                    t = (api.hmget_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, HM_BINARY);
                    ret = temp_of(t);
                }
                Op::GetTs(k) => {
                    let mut key = k;
                    let mut tmp: isize = 0;
                    t = (api.hmget_key_ts)(
                        t,
                        ELEM,
                        &raw mut key as *mut c_void,
                        KEYSZ,
                        &raw mut tmp,
                        HM_BINARY,
                    );
                    ret = tmp;
                }
                Op::Del(k) => {
                    let mut key = k;
                    t = (api.hmdel_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, 0, HM_BINARY);
                    ret = if t.is_null() { 0 } else { temp_of(t) };
                }
                Op::Default(v) => {
                    t = (api.hmput_default)(t, ELEM);
                    // hmdefault(t, v): t[-1].value = v
                    let e = (t as *mut u8).sub(ELEM) as *mut i32;
                    *e.add(1) = v;
                    ret = 0;
                }
                Op::PutDefaultOnly => {
                    t = (api.hmput_default)(t, ELEM);
                    ret = 0;
                }
            }
            out.push(Step {
                ret,
                snap: map_snap(t, ELEM, false, RANGES),
            });
        }

        if !t.is_null() {
            (api.hmfree_func)((t as *mut u8).sub(ELEM) as *mut c_void, ELEM);
        }
        out
    }
}

fn compare(name: &str, seed: usize, ops: &[Op]) {
    let _guard = global_lock();
    let (c, r) = both();
    let a = unsafe { run(&c, seed, ops) };
    let b = unsafe { run(&r, seed, ops) };
    assert_eq!(a.len(), b.len(), "{name}: step count");
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x, y,
            "{name} (seed={seed:#x}) diverged at step {i} ({:?})",
            ops[i]
        );
    }
}

const SEEDS: [usize; 4] = [0x3141_5926, 0, 1, 0xdead_beef_cafe_0001];

#[test]
fn empty_and_missing() {
    for seed in SEEDS {
        compare("get on null", seed, &[Op::Get(5)]);
        compare("get_ts on null", seed, &[Op::GetTs(5)]);
        compare("del on null", seed, &[Op::Del(5)]);
        compare("default on null", seed, &[Op::Default(-1)]);
        compare("put_default twice", seed, &[Op::PutDefaultOnly, Op::PutDefaultOnly]);
        compare(
            "get then get again",
            seed,
            &[Op::Get(1), Op::Get(1), Op::GetTs(2), Op::Del(3)],
        );
    }
}

#[test]
fn single_insert_lookup() {
    for seed in SEEDS {
        compare(
            "one entry",
            seed,
            &[Op::Put(42, 4242), Op::Get(42), Op::Get(43), Op::GetTs(42)],
        );
        compare(
            "overwrite",
            seed,
            &[Op::Put(7, 1), Op::Put(7, 2), Op::Get(7), Op::Put(7, 3), Op::Get(7)],
        );
    }
}

#[test]
fn sequential_inserts_force_growth() {
    for seed in SEEDS {
        // 8 slots -> threshold 6 -> grows to 16 -> 12 -> 32 ...
        for n in [1i32, 2, 5, 6, 7, 8, 13, 24, 25, 100] {
            let mut ops: Vec<Op> = (0..n).map(|i| Op::Put(i, i * 3 + 1)).collect();
            for i in 0..n {
                ops.push(Op::Get(i));
            }
            ops.push(Op::Get(n + 1000));
            compare(&format!("grow n={n}"), seed, &ops);
        }
    }
}

#[test]
fn deletes_and_tombstones() {
    for seed in SEEDS {
        let mut ops: Vec<Op> = (0..32).map(|i| Op::Put(i, i * 7)).collect();
        // delete every other key, then look everything up again
        for i in (0..32).step_by(2) {
            ops.push(Op::Del(i));
            ops.push(Op::Get(i));
            ops.push(Op::Get(i + 1));
        }
        compare("alternating delete", seed, &ops);

        // delete-all: exercises the shrink path
        let mut ops: Vec<Op> = (0..64).map(|i| Op::Put(i, i)).collect();
        for i in 0..64 {
            ops.push(Op::Del(i));
        }
        for i in 0..64 {
            ops.push(Op::Get(i));
        }
        compare("delete all", seed, &ops);

        // reverse-order deletion (old_index == final_index path)
        let mut ops: Vec<Op> = (0..40).map(|i| Op::Put(i, i)).collect();
        for i in (0..40).rev() {
            ops.push(Op::Del(i));
        }
        compare("delete reverse", seed, &ops);

        // delete then reinsert: reuses tombstones
        let mut ops: Vec<Op> = (0..20).map(|i| Op::Put(i, i)).collect();
        for i in 0..20 {
            ops.push(Op::Del(i));
            ops.push(Op::Put(i + 100, i));
        }
        compare("delete/reinsert", seed, &ops);

        compare("delete missing", seed, &[Op::Put(1, 1), Op::Del(2), Op::Del(2), Op::Get(1)]);
    }
}

#[test]
fn interleaved_pseudorandom_workload() {
    for seed in SEEDS {
        let mut state: u64 = 0x1234_5678_9abc_def0 ^ seed as u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut ops = Vec::new();
        for _ in 0..1500 {
            let v = next();
            let k = (v >> 8) as i32 % 90;
            match v & 7 {
                0 | 1 | 2 => ops.push(Op::Put(k, (v >> 32) as i32)),
                3 | 4 => ops.push(Op::Get(k)),
                5 => ops.push(Op::GetTs(k)),
                6 => ops.push(Op::Del(k)),
                _ => ops.push(Op::Default((v >> 40) as i32)),
            }
        }
        compare("random workload", seed, &ops);
    }
}

#[test]
fn default_element_interacts_with_inserts() {
    for seed in SEEDS {
        compare(
            "default then puts",
            seed,
            &[
                Op::Default(-999),
                Op::Put(1, 10),
                Op::Put(2, 20),
                Op::Default(-888),
                Op::Get(1),
                Op::Get(3),
                Op::Del(1),
                Op::Get(1),
                Op::Default(-777),
            ],
        );
        compare(
            "puts then default",
            seed,
            &[Op::Put(1, 10), Op::Default(-1), Op::Put(2, 20), Op::PutDefaultOnly],
        );
    }
}

/// `stbds_rand_seed` must change the seed that new tables pick up, and the
/// per-table seed derivation must advance identically in both libraries.
#[test]
fn rand_seed_controls_table_seeds() {
    let _guard = global_lock();
    let (c, r) = both();
    for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0x9e37_79b9_7f4a_7c15] {
        let collect = |api: &Api| unsafe {
            (api.rand_seed)(seed);
            let mut seeds = Vec::new();
            for _ in 0..6 {
                let mut t: *mut c_void = std::ptr::null_mut();
                let mut key = 1i32;
                t = (api.hmput_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, HM_BINARY);
                let raw = (t as *mut u8).sub(ELEM) as *mut c_void;
                let h = (raw as *mut ArrayHeader).sub(1);
                let tbl = (*h).hash_table as *mut HashIndex;
                seeds.push((*tbl).seed);
                (api.hmfree_func)(raw, ELEM);
            }
            seeds
        };
        let a = collect(&c);
        let b = collect(&r);
        assert_eq!(a[0], seed, "first table must use the seed just set");
        assert_eq!(a, b, "seed chain for rand_seed({seed:#x})");
    }
}

/// Large maps: exercises many consecutive table growths (8 slots up to 8192)
/// and the shrink chain on the way back down. Snapshotting every step would be
/// quadratic, so state is compared periodically plus once at the end.
#[test]
fn large_map_growth_and_shrink_chain() {
    let _guard = global_lock();
    let (c, r) = both();
    const N: i32 = 5000;
    let drive = |api: &Api| unsafe {
        (api.rand_seed)(0x3141_5926);
        let mut t: *mut c_void = std::ptr::null_mut();
        let mut out = Vec::new();
        for i in 0..N {
            let mut key = i;
            t = (api.hmput_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, HM_BINARY);
            let idx = temp_of(t);
            let e = (t as *mut u8).offset(ELEM as isize * idx) as *mut i32;
            *e = i;
            *e.add(1) = i * 3;
            if i % 250 == 0 || i == N - 1 {
                out.push((i, map_snap(t, ELEM, false, RANGES)));
            }
        }
        // every key must still be findable
        let mut found = Vec::new();
        for i in 0..N {
            let mut key = i;
            t = (api.hmget_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, HM_BINARY);
            found.push(temp_of(t));
        }
        for i in 0..N {
            let mut key = i;
            t = (api.hmdel_key)(t, ELEM, &raw mut key as *mut c_void, KEYSZ, 0, HM_BINARY);
            if i % 250 == 0 || i == N - 1 {
                out.push((N + i, map_snap(t, ELEM, false, RANGES)));
            }
        }
        (api.hmfree_func)((t as *mut u8).sub(ELEM) as *mut c_void, ELEM);
        (out, found)
    };
    let (sa, fa) = drive(&c);
    let (sb, fb) = drive(&r);
    assert_eq!(fa, fb, "lookup results on a 5000-entry map");
    assert_eq!(sa.len(), sb.len());
    for ((ia, x), (ib, y)) in sa.iter().zip(sb.iter()) {
        assert_eq!(ia, ib);
        assert_eq!(x, y, "large map checkpoint {ia}");
    }
    // sanity: the table really did grow far past the initial 8 slots
    let peak = sa
        .iter()
        .filter_map(|(_, s)| s.table.as_ref().map(|t| t.slot_count))
        .max()
        .unwrap();
    assert!(peak >= 8192, "peak slot_count was only {peak}");
}
