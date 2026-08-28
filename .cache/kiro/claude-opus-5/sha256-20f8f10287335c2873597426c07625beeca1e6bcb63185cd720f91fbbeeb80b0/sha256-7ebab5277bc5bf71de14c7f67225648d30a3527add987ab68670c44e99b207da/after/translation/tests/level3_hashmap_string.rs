//! Level 3: string-keyed hash maps in all three key-storage modes
//! (`STBDS_SH_DEFAULT` via a plain `shput`, `STBDS_SH_STRDUP` and
//! `STBDS_SH_ARENA` via `stbds_shmode_func`).
//!
//! Macro expansions replayed here:
//!   sh_new_strdup(t) -> t = shmode_func(sizeof*t, STBDS_SH_STRDUP)
//!   sh_new_arena(t)  -> t = shmode_func(sizeof*t, STBDS_SH_ARENA)
//!   shput(t,k,v)     -> t = hmput_key(t, sizeof*t, (void*)k,
//!                                     sizeof t->key, STBDS_HM_STRING);
//!                       t[temp(t-1)].value = v;
//!   shgeti(t,k)      -> t = hmget_key(t, sizeof*t, (void*)k,
//!                                     sizeof t->key, STBDS_HM_STRING);
//!                       temp(t-1)
//!   shdel(t,k)       -> t = hmdel_key(t, sizeof*t, (void*)k, sizeof t->key,
//!                                     offsetof(key), STBDS_HM_STRING);
//!                       t ? temp(t-1) : 0

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// `typedef struct { char *key; int value; }` -- 16 bytes, 4 trailing padding
/// bytes that are never written, so only `key` (by pointee) and `value` are
/// compared.
const ELEM: usize = 16;
const KEYSZ: usize = 8; // sizeof(char *)
const RANGES: &[(usize, usize)] = &[(8, 4)];

#[derive(Debug, Clone)]
enum Op {
    Put(&'static str, i32),
    PutOwned(String, i32),
    Get(String),
    GetTs(String),
    Del(String),
    Default(i32),
}

#[derive(Debug, PartialEq, Eq)]
struct Step {
    ret: isize,
    snap: MapSnap,
}

/// Keeps every key buffer alive for the whole replay and hands out identical
/// pointers to both libraries.
struct Keys(Vec<Box<[c_char]>>);

impl Keys {
    fn new() -> Keys {
        Keys(Vec::new())
    }
    fn make(&mut self, s: &str) -> *mut c_char {
        let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
        v.push(0);
        let mut b = v.into_boxed_slice();
        let p = b.as_mut_ptr();
        self.0.push(b);
        p
    }
}

unsafe fn temp_of(t: *mut c_void) -> isize {
    unsafe {
        let raw = (t as *mut u8).sub(ELEM) as *mut ArrayHeader;
        (*raw.sub(1)).temp
    }
}

unsafe fn run(api: &Api, seed: usize, init_mode: Option<c_int>, ops: &[Op]) -> Vec<Step> {
    unsafe {
        (api.rand_seed)(seed);
        let mut keys = Keys::new();
        let mut t: *mut c_void = match init_mode {
            Some(m) => (api.shmode_func)(ELEM, m),
            None => std::ptr::null_mut(),
        };
        let mut out = Vec::with_capacity(ops.len() + 1);
        if init_mode.is_some() {
            out.push(Step {
                ret: 0,
                snap: map_snap(t, ELEM, true, RANGES),
            });
        }

        for op in ops {
            let ret;
            match op {
                Op::Put(k, v) => {
                    let kp = keys.make(k);
                    t = (api.hmput_key)(t, ELEM, kp as *mut c_void, KEYSZ, HM_STRING);
                    let idx = temp_of(t);
                    let e = (t as *mut u8).offset(ELEM as isize * idx);
                    *(e.add(8) as *mut i32) = *v;
                    ret = idx;
                }
                Op::PutOwned(k, v) => {
                    let kp = keys.make(k);
                    t = (api.hmput_key)(t, ELEM, kp as *mut c_void, KEYSZ, HM_STRING);
                    let idx = temp_of(t);
                    let e = (t as *mut u8).offset(ELEM as isize * idx);
                    *(e.add(8) as *mut i32) = *v;
                    ret = idx;
                }
                Op::Get(k) => {
                    let kp = keys.make(k);
                    t = (api.hmget_key)(t, ELEM, kp as *mut c_void, KEYSZ, HM_STRING);
                    ret = temp_of(t);
                }
                Op::GetTs(k) => {
                    let kp = keys.make(k);
                    let mut tmp: isize = 0;
                    t = (api.hmget_key_ts)(
                        t,
                        ELEM,
                        kp as *mut c_void,
                        KEYSZ,
                        &raw mut tmp,
                        HM_STRING,
                    );
                    ret = tmp;
                }
                Op::Del(k) => {
                    let kp = keys.make(k);
                    t = (api.hmdel_key)(t, ELEM, kp as *mut c_void, KEYSZ, 0, HM_STRING);
                    ret = if t.is_null() { 0 } else { temp_of(t) };
                }
                Op::Default(v) => {
                    t = (api.hmput_default)(t, ELEM);
                    *((t as *mut u8).sub(ELEM).add(8) as *mut i32) = *v;
                    ret = 0;
                }
            }
            out.push(Step {
                ret,
                snap: map_snap(t, ELEM, true, RANGES),
            });
        }

        if !t.is_null() {
            (api.hmfree_func)((t as *mut u8).sub(ELEM) as *mut c_void, ELEM);
        }
        out
    }
}

fn compare(name: &str, seed: usize, init_mode: Option<c_int>, ops: &[Op]) {
    let _guard = global_lock();
    let (c, r) = both();
    let a = unsafe { run(&c, seed, init_mode, ops) };
    let b = unsafe { run(&r, seed, init_mode, ops) };
    assert_eq!(a.len(), b.len(), "{name}: step count");
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "{name} (seed={seed:#x}, mode={init_mode:?}) step {i}: {:?}", ops[i]);
    }
}

const SEEDS: [usize; 3] = [0x3141_5926, 0, 0x0bad_f00d_1234_5678];

/// `None` = plain `shput` on a fresh NULL map (STBDS_SH_DEFAULT is selected
/// inside `hmput_key`); the others go through `stbds_shmode_func`.
const MODES: [Option<c_int>; 4] = [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)];

#[test]
fn shmode_func_initial_state() {
    let _guard = global_lock();
    let (c, r) = both();
    for seed in SEEDS {
        for m in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let snap = |api: &Api| unsafe {
                (api.rand_seed)(seed);
                let t = (api.shmode_func)(ELEM, m);
                let s = map_snap(t, ELEM, false, &[(0, ELEM)]);
                (api.hmfree_func)((t as *mut u8).sub(ELEM) as *mut c_void, ELEM);
                s
            };
            assert_eq!(snap(&c), snap(&r), "shmode_func(mode={m}, seed={seed:#x})");
        }
    }
}

#[test]
fn empty_and_missing() {
    for &m in &MODES {
        for seed in SEEDS {
            compare("get null", seed, m, &[Op::Get("a".into())]);
            compare("get_ts null", seed, m, &[Op::GetTs("a".into())]);
            compare("del null", seed, m, &[Op::Del("a".into())]);
            compare("default null", seed, m, &[Op::Default(-3)]);
            compare(
                "empty-string key",
                seed,
                m,
                &[Op::Put("", 1), Op::Get("".into()), Op::Get("x".into())],
            );
        }
    }
}

#[test]
fn basic_put_get_overwrite() {
    for &m in &MODES {
        for seed in SEEDS {
            compare(
                "three keys",
                seed,
                m,
                &[
                    Op::Put("alpha", 1),
                    Op::Put("beta", 2),
                    Op::Put("gamma", 3),
                    Op::Get("alpha".into()),
                    Op::Get("beta".into()),
                    Op::Get("gamma".into()),
                    Op::Get("delta".into()),
                    Op::GetTs("beta".into()),
                ],
            );
            compare(
                "overwrite",
                seed,
                m,
                &[
                    Op::Put("k", 1),
                    Op::Put("k", 2),
                    Op::Get("k".into()),
                    Op::Put("k", 3),
                    Op::Get("k".into()),
                ],
            );
        }
    }
}

#[test]
fn growth_and_rehash() {
    for &m in &MODES {
        for seed in SEEDS {
            for n in [6usize, 7, 13, 50, 200] {
                let mut ops: Vec<Op> = (0..n)
                    .map(|i| Op::PutOwned(format!("test_{i}"), i as i32 * 5))
                    .collect();
                for i in 0..n {
                    ops.push(Op::Get(format!("test_{i}")));
                }
                ops.push(Op::Get(format!("test_{}", n + 999)));
                compare(&format!("grow n={n}"), seed, m, &ops);
            }
        }
    }
}

#[test]
fn deletes_shrink_and_rebuild() {
    for &m in &MODES {
        for seed in SEEDS {
            let mut ops: Vec<Op> = (0..40).map(|i| Op::PutOwned(format!("s{i}"), i)).collect();
            for i in (0..40).step_by(3) {
                ops.push(Op::Del(format!("s{i}")));
                ops.push(Op::Get(format!("s{i}")));
            }
            compare("sparse delete", seed, m, &ops);

            let mut ops: Vec<Op> = (0..64).map(|i| Op::PutOwned(format!("k{i}"), i)).collect();
            for i in 0..64 {
                ops.push(Op::Del(format!("k{i}")));
            }
            for i in 0..64 {
                ops.push(Op::Get(format!("k{i}")));
            }
            compare("delete all", seed, m, &ops);

            let mut ops: Vec<Op> = (0..30).map(|i| Op::PutOwned(format!("k{i}"), i)).collect();
            for i in (0..30).rev() {
                ops.push(Op::Del(format!("k{i}")));
            }
            compare("delete reverse", seed, m, &ops);

            let mut ops: Vec<Op> = (0..24).map(|i| Op::PutOwned(format!("k{i}"), i)).collect();
            for i in 0..24 {
                ops.push(Op::Del(format!("k{i}")));
                ops.push(Op::PutOwned(format!("n{i}"), i + 1000));
            }
            compare("delete/reinsert", seed, m, &ops);
        }
    }
}

#[test]
fn long_and_varied_keys() {
    for &m in &MODES {
        for seed in [SEEDS[0]] {
            // long keys stress the arena's oversized-block path
            let mut ops: Vec<Op> = Vec::new();
            for i in 0..30 {
                ops.push(Op::PutOwned("L".repeat(i * 37) + &format!("{i}"), i as i32));
            }
            for i in 0..30 {
                ops.push(Op::Get("L".repeat(i * 37) + &format!("{i}")));
            }
            compare("long keys", seed, m, &ops);

            // keys with high-bit bytes
            let mut ops: Vec<Op> = Vec::new();
            for i in 1..40u8 {
                let k: String = String::from_utf8_lossy(&[0xC3, 0x80 | (i & 0x3f), b'a' + i % 26])
                    .into_owned();
                ops.push(Op::PutOwned(k.clone(), i as i32));
                ops.push(Op::Get(k));
            }
            compare("high-bit keys", seed, m, &ops);

            // keys that share long prefixes
            let mut ops: Vec<Op> = Vec::new();
            for i in 0..60 {
                ops.push(Op::PutOwned(format!("{}{i}", "prefix_".repeat(9)), i));
            }
            for i in 0..60 {
                ops.push(Op::Del(format!("{}{i}", "prefix_".repeat(9))));
            }
            compare("shared prefixes", seed, m, &ops);
        }
    }
}

#[test]
fn interleaved_pseudorandom_workload() {
    for &m in &MODES {
        for seed in SEEDS {
            let mut state: u64 = 0xfeed_face_dead_beef ^ seed as u64;
            let mut next = move || {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state
            };
            let mut ops = Vec::new();
            for _ in 0..1200 {
                let v = next();
                let k = format!("key_{}", (v >> 8) % 70);
                match v & 7 {
                    0 | 1 | 2 => ops.push(Op::PutOwned(k, (v >> 32) as i32)),
                    3 | 4 => ops.push(Op::Get(k)),
                    5 => ops.push(Op::GetTs(k)),
                    6 => ops.push(Op::Del(k)),
                    _ => ops.push(Op::Default((v >> 40) as i32)),
                }
            }
            compare("random workload", seed, m, &ops);
        }
    }
}

/// `stbds_shputs` relies on `table->temp_key` pointing at the stored copy of
/// the key. `stbds_make_hash_index` leaves that field uninitialised, so it is
/// only well-defined right after an insert -- which is exactly what this test
/// arranges (every key is fresh, so `hmput_key` always takes the insert path).
#[test]
fn temp_key_contents_after_insert() {
    let _guard = global_lock();
    let (c, r) = both();
    for &m in &MODES {
        for seed in SEEDS {
            let collect = |api: &Api| unsafe {
                (api.rand_seed)(seed);
                let mut keys = Keys::new();
                let mut t: *mut c_void = match m {
                    Some(mode) => (api.shmode_func)(ELEM, mode),
                    None => std::ptr::null_mut(),
                };
                let mut got = Vec::new();
                for i in 0..120 {
                    let s = format!("unique_key_{i}_{}", "p".repeat(i % 13));
                    let kp = keys.make(&s);
                    t = (api.hmput_key)(t, ELEM, kp as *mut c_void, KEYSZ, HM_STRING);
                    let raw = (t as *mut u8).sub(ELEM) as *mut ArrayHeader;
                    let h = raw.sub(1);
                    let tbl = (*h).hash_table as *mut HashIndex;
                    let idx = (*h).temp;
                    let stored = *((t as *mut u8).offset(ELEM as isize * idx) as *mut *mut c_char);
                    got.push((
                        idx,
                        c_string((*tbl).temp_key),
                        c_string(stored),
                        (*tbl).temp_key == stored,
                        s,
                    ));
                }
                (api.hmfree_func)((t as *mut u8).sub(ELEM) as *mut c_void, ELEM);
                got
            };
            let a = collect(&c);
            let b = collect(&r);
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                assert_eq!(x, y, "temp_key step {i} (mode={m:?}, seed={seed:#x})");
                assert_eq!(x.1.as_deref(), Some(x.4.as_str()), "temp_key content");
                assert!(x.3, "temp_key must alias the stored key pointer");
            }
        }
    }
}
