//! Phase B — valid-path differential tests (one row per `CONFIGS.md` row).
//!
//! Both libraries are driven through their `.so` exports only; after every call
//! the *complete* state of the C and the Rust object is compared field by field
//! and slot by slot.

mod common;

use common::*;
use std::os::raw::c_int;

macro_rules! log {
    ($v:expr, $($t:tt)*) => { $v.push(format!($($t)*)) };
}

/// The FNV-1a hash of `hashmap.c`, re-implemented so the tests can construct
/// keys that deliberately collide.
fn hash(key: u64) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in key.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// `count` distinct keys that all land in bucket `bucket` of a `cap`-slot table.
fn colliding_keys(cap: u64, bucket: u64, count: usize, rng: &mut Rng) -> Vec<u64> {
    let mut v = Vec::new();
    while v.len() < count {
        let k = rng.next_u64();
        if hash(k) % cap == bucket && !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

fn data_shapes() -> Vec<(String, Option<Vec<u8>>)> {
    let mut v: Vec<(String, Option<Vec<u8>>)> = vec![
        ("NULL".into(), None),
        ("empty".into(), Some(cstring(b""))),
        ("x".into(), Some(cstring(b"x"))),
        ("short".into(), Some(cstring(b"hello world"))),
    ];
    for n in [253usize, 254, 255, 256, 257, 300, 1024] {
        v.push((format!("len{}", n), Some(cstring(&vec![b'a' + (n % 26) as u8; n]))));
    }
    v.push(("high_bytes".into(), Some(cstring(&(1u8..=255).collect::<Vec<u8>>()))));
    v.push(("printf_meta".into(), Some(cstring(b"%s %d %n \\ %% \x7f"))));
    v.push(("newlines".into(), Some(cstring(b"a\nb\tc\rd"))));
    v
}

// ---------------------------------------------------------------------------
// C1..C15 — hashmap
// ---------------------------------------------------------------------------

fn c1(p: &Pair, h: &mut Harness) {
    h.row("C1", |row| unsafe {
        let (c, r) = both(p, |a| {
            let m = (a.hashmap_create)();
            let mut log = Vec::new();
            log!(log, "null={}", m.is_null());
            log!(log, "size={}", (a.hashmap_size)(m));
            log!(log, "get0_null={}", (a.hashmap_get)(m, 0).is_null());
            log!(log, "contains0={}", (a.hashmap_contains)(m, 0));
            let s = snap_map_raw(m);
            (a.hashmap_destroy)(m);
            (log, s)
        });
        row.eq_logs("log", &c.0, &r.0);
        row.eq_map("fresh map", &c.1, &r.1);
        row.ok(
            "capacity is HASHMAP_INITIAL_CAPACITY",
            c.1.as_ref().unwrap().capacity == HASHMAP_INITIAL_CAPACITY,
        );
    });
}

fn c2(p: &Pair, h: &mut Harness) {
    h.row("C2", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 2);
        let mut keys: Vec<u64> = vec![0, 1, 2, 0x7FFF_FFFF_FFFF_FFFF, 1 << 63, u64::MAX, u64::MAX - 1];
        for _ in 0..200 {
            keys.push(rng.next_u64());
        }
        for (i, &k) in keys.iter().enumerate() {
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                log!(log, "put={}", (a.hashmap_put)(m, k, token(7)));
                log!(log, "get={:?}", (a.hashmap_get)(m, k) as u64);
                log!(log, "contains={}", (a.hashmap_contains)(m, k));
                log!(log, "size={}", (a.hashmap_size)(m));
                log!(log, "get_other={:?}", (a.hashmap_get)(m, k ^ 0x5555) as u64);
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("key[{}]={} log", i, k), &c.0, &r.0);
            row.eq_map(&format!("key[{}]={} map", i, k), &c.1, &r.1);
        }
    });
}

fn c3_c4_c5(p: &Pair, h: &mut Harness) {
    // The resize check runs *before* the insertion, so a table of capacity 16
    // still holds 13 entries ((12+0)/16 == 0.75 is not > 0.75) and only the
    // 14th insertion doubles it.  Same one-off at every later doubling.
    let plan: Vec<(&str, Vec<(usize, usize)>)> = vec![
        (
            "C3",
            (1usize..=13).map(|n| (n, 16usize)).collect(),
        ),
        ("C4", vec![(14, 32), (24, 32), (25, 32), (26, 64)]),
        ("C5", vec![(49, 64), (50, 128), (97, 128), (98, 256), (100, 256)]),
    ];
    for (name, cases) in plan {
        h.row(name, |row| unsafe {
            for (n, expect_cap) in cases {
                for trial in 0..6u64 {
                    let mut rng = Rng::new(SEED ^ (n as u64) ^ (trial << 32));
                    let mut keys = Vec::new();
                    while keys.len() < n {
                        let k = rng.key();
                        if !keys.contains(&k) {
                            keys.push(k);
                        }
                    }
                    let (c, r) = both(p, |a| {
                        let m = (a.hashmap_create)();
                        let mut log = Vec::new();
                        for (i, &k) in keys.iter().enumerate() {
                            log!(log, "put({})={}", k, (a.hashmap_put)(m, k, token(i as u64)));
                            log!(log, "  size={}", (a.hashmap_size)(m));
                            log!(log, "  cap={}", (*m).capacity);
                        }
                        for (i, &k) in keys.iter().enumerate() {
                            log!(
                                log,
                                "get({})={:?} want={:?}",
                                k,
                                (a.hashmap_get)(m, k) as u64,
                                token(i as u64) as u64
                            );
                            log!(log, "contains({})={}", k, (a.hashmap_contains)(m, k));
                        }
                        let s = snap_map_raw(m);
                        (a.hashmap_destroy)(m);
                        (log, s)
                    });
                    row.eq_logs(&format!("n={} trial{} log", n, trial), &c.0, &r.0);
                    row.eq_map(&format!("n={} trial{} map", n, trial), &c.1, &r.1);
                    let cs = c.1.as_ref().unwrap();
                    row.ok(
                        &format!(
                            "n={} trial{} capacity=={} (got {})",
                            n, trial, expect_cap, cs.capacity
                        ),
                        cs.capacity == expect_cap,
                    );
                    row.eq(&format!("n={} trial{} size", n, trial), cs.size, n);
                }
            }
        });
    }
}

fn c6(p: &Pair, h: &mut Harness) {
    h.row("C6", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 6);
        // Long probe runs inside capacity 16, plus wrap-around: bucket 15 and 14.
        for bucket in [0u64, 1, 7, 14, 15] {
            let keys = colliding_keys(16, bucket, 12, &mut rng);
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    log!(log, "put({})={}", k, (a.hashmap_put)(m, k, token(i as u64)));
                }
                for &k in keys.iter() {
                    log!(log, "get({})={:?}", k, (a.hashmap_get)(m, k) as u64);
                }
                // remove from the middle of the probe run, then look the rest up
                log!(log, "rm={:?}", (a.hashmap_remove)(m, keys[5]) as u64);
                for &k in keys.iter() {
                    log!(log, "get2({})={:?}", k, (a.hashmap_get)(m, k) as u64);
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("bucket{} log", bucket), &c.0, &r.0);
            row.eq_map(&format!("bucket{} map", bucket), &c.1, &r.1);
        }
    });
}

fn c7(p: &Pair, h: &mut Harness) {
    h.row("C7", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 7);
        for _ in 0..50 {
            let k = rng.key();
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for i in 0..5u64 {
                    log!(log, "put={}", (a.hashmap_put)(m, k, token(i)));
                    log!(log, " size={}", (a.hashmap_size)(m));
                    log!(log, " get={:?}", (a.hashmap_get)(m, k) as u64);
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs("update log", &c.0, &r.0);
            row.eq_map("update map", &c.1, &r.1);
        }
    });
}

fn c8(p: &Pair, h: &mut Harness) {
    h.row("C8", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 8);
        for _ in 0..40 {
            let mut keys = Vec::new();
            while keys.len() < 10 {
                let k = rng.key();
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    let v = if i % 2 == 0 {
                        std::ptr::null_mut()
                    } else {
                        token(i as u64)
                    };
                    log!(log, "put({})={}", k, (a.hashmap_put)(m, k, v));
                    log!(log, " size={}", (a.hashmap_size)(m));
                }
                for &k in keys.iter() {
                    log!(log, "get({})={:?}", k, (a.hashmap_get)(m, k) as u64);
                    log!(log, "contains({})={}", k, (a.hashmap_contains)(m, k));
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs("null-value log", &c.0, &r.0);
            row.eq_map("null-value map", &c.1, &r.1);
        }
    });
}

fn c9_c10_c11(p: &Pair, h: &mut Harness) {
    h.row("C9", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 9);
        for bucket in [0u64, 9, 15] {
            let keys = colliding_keys(16, bucket, 8, &mut rng);
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    (a.hashmap_put)(m, k, token(i as u64));
                }
                for &k in [keys[0], keys[3], keys[3], keys[7]].iter() {
                    log!(log, "rm({})={:?}", k, (a.hashmap_remove)(m, k) as u64);
                    log!(log, " size={} del={}", (*m).size, (*m).deleted_count);
                    for &q in keys.iter() {
                        log!(log, "  get({})={:?}", q, (a.hashmap_get)(m, q) as u64);
                    }
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("bucket{} log", bucket), &c.0, &r.0);
            row.eq_map(&format!("bucket{} map", bucket), &c.1, &r.1);
        }
    });

    h.row("C10", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 10);
        for bucket in [0u64, 5, 15] {
            let keys = colliding_keys(16, bucket, 6, &mut rng);
            let fresh = colliding_keys(16, bucket, 2, &mut rng);
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    (a.hashmap_put)(m, k, token(i as u64));
                }
                (a.hashmap_remove)(m, keys[2]);
                (a.hashmap_remove)(m, keys[4]);
                // re-insert the same key -> "reuse deleted slot"
                log!(log, "reput={}", (a.hashmap_put)(m, keys[2], token(99)));
                log!(log, " size={} del={}", (*m).size, (*m).deleted_count);
                log!(log, " get={:?}", (a.hashmap_get)(m, keys[2]) as u64);
                // insert a *different* colliding key into the other tombstone
                log!(log, "newput={}", (a.hashmap_put)(m, fresh[0], token(98)));
                log!(log, " size={} del={}", (*m).size, (*m).deleted_count);
                for &q in keys.iter().chain(fresh.iter()) {
                    log!(log, "  get({})={:?}", q, (a.hashmap_get)(m, q) as u64);
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("bucket{} log", bucket), &c.0, &r.0);
            row.eq_map(&format!("bucket{} map", bucket), &c.1, &r.1);
        }
    });

    h.row("C11", |row| unsafe {
        // tombstones alone push (size + deleted_count) past the load factor
        for trial in 0..8u64 {
            let mut rng = Rng::new(SEED ^ 11 ^ trial);
            let keys: Vec<u64> = (0..40).map(|_| rng.key()).collect();
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    log!(log, "put({})={}", k, (a.hashmap_put)(m, k, token(i as u64)));
                    if i % 3 == 0 {
                        log!(log, "rm({})={:?}", k, (a.hashmap_remove)(m, k) as u64);
                    }
                    log!(
                        log,
                        " cap={} size={} del={}",
                        (*m).capacity,
                        (*m).size,
                        (*m).deleted_count
                    );
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("trial{} log", trial), &c.0, &r.0);
            row.eq_map(&format!("trial{} map", trial), &c.1, &r.1);
        }
    });
}

fn c12_c13_c14(p: &Pair, h: &mut Harness) {
    h.row("C12", |row| unsafe {
        for n in [0usize, 1, 5, 12, 13, 40, 100] {
            let mut rng = Rng::new(SEED ^ 12 ^ n as u64);
            let keys: Vec<u64> = (0..n).map(|_| rng.key()).collect();
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    (a.hashmap_put)(m, k, token(i as u64));
                    if i % 4 == 3 {
                        (a.hashmap_remove)(m, k);
                    }
                }
                let before = snap_map_raw(m);
                (a.hashmap_clear)(m);
                log!(
                    log,
                    "after clear cap={} size={} del={}",
                    (*m).capacity,
                    (*m).size,
                    (*m).deleted_count
                );
                for &k in keys.iter() {
                    log!(log, " get({})={:?}", k, (a.hashmap_get)(m, k) as u64);
                    log!(log, " contains({})={}", k, (a.hashmap_contains)(m, k));
                }
                let after = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, before, after)
            });
            row.eq_logs(&format!("n={} log", n), &c.0, &r.0);
            row.eq_map(&format!("n={} before", n), &c.1, &r.1);
            row.eq_map(&format!("n={} after", n), &c.2, &r.2);
            // `hashmap_clear` must keep key/value bytes intact
            if let (Some(b), Some(af)) = (c.1.as_ref(), c.2.as_ref()) {
                row.ok(
                    &format!("n={} clear keeps keys/values", n),
                    b.slots
                        .iter()
                        .zip(af.slots.iter())
                        .all(|(x, y)| x.key == y.key && x.value == y.value),
                );
            }
        }
    });

    h.row("C13", |row| unsafe {
        for n in [1usize, 12, 30] {
            let mut rng = Rng::new(SEED ^ 13 ^ n as u64);
            let keys: Vec<u64> = (0..n).map(|_| rng.key()).collect();
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    (a.hashmap_put)(m, k, token(i as u64));
                }
                (a.hashmap_clear)(m);
                for (i, &k) in keys.iter().enumerate() {
                    log!(
                        log,
                        "reput({})={}",
                        k,
                        (a.hashmap_put)(m, k, token(1000 + i as u64))
                    );
                    log!(log, " get={:?}", (a.hashmap_get)(m, k) as u64);
                    log!(
                        log,
                        " cap={} size={} del={}",
                        (*m).capacity,
                        (*m).size,
                        (*m).deleted_count
                    );
                }
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("n={} log", n), &c.0, &r.0);
            row.eq_map(&format!("n={} map", n), &c.1, &r.1);
        }
    });

    h.row("C14", |row| unsafe {
        for n in [10usize, 200] {
            let mut rng = Rng::new(SEED ^ 14 ^ n as u64);
            let keys: Vec<u64> = (0..n).map(|_| rng.key()).collect();
            let probes: Vec<u64> = keys
                .iter()
                .cloned()
                .chain((0..30).map(|_| rng.key()))
                .collect();
            let (c, r) = both(p, |a| {
                let m = (a.hashmap_create)();
                let mut log = Vec::new();
                for (i, &k) in keys.iter().enumerate() {
                    let v = if i % 5 == 0 {
                        std::ptr::null_mut()
                    } else {
                        token(i as u64)
                    };
                    (a.hashmap_put)(m, k, v);
                    if i % 7 == 6 {
                        (a.hashmap_remove)(m, k);
                    }
                }
                for &q in probes.iter() {
                    log!(log, "contains({})={}", q, (a.hashmap_contains)(m, q));
                }
                log!(log, "cap={}", (*m).capacity);
                let s = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (log, s)
            });
            row.eq_logs(&format!("n={} log", n), &c.0, &r.0);
            row.eq_map(&format!("n={} map", n), &c.1, &r.1);
        }
    });
}

#[derive(Debug, Clone, Copy)]
enum MapOp {
    Put(u64, u64),
    PutNull(u64),
    Get(u64),
    Remove(u64),
    Contains(u64),
    Size,
    Clear,
}

unsafe fn apply_map(a: &Api, m: *mut Hashmap, op: MapOp) -> String {
    match op {
        MapOp::Put(k, v) => format!("put({},{})={}", k, v, (a.hashmap_put)(m, k, token(v))),
        MapOp::PutNull(k) => format!(
            "put({},NULL)={}",
            k,
            (a.hashmap_put)(m, k, std::ptr::null_mut())
        ),
        MapOp::Get(k) => format!("get({})={}", k, (a.hashmap_get)(m, k) as u64),
        MapOp::Remove(k) => format!("rm({})={}", k, (a.hashmap_remove)(m, k) as u64),
        MapOp::Contains(k) => format!("has({})={}", k, (a.hashmap_contains)(m, k)),
        MapOp::Size => format!("size={}", (a.hashmap_size)(m)),
        MapOp::Clear => {
            (a.hashmap_clear)(m);
            "clear".to_string()
        }
    }
}

fn c15(p: &Pair, h: &mut Harness) {
    h.row("C15", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 15);
        // A small key space so collisions, updates and tombstone reuse all hit.
        let space: Vec<u64> = (0..64).map(|_| rng.key()).collect();
        let mc = (p.c.hashmap_create)();
        let mr = (p.r.hashmap_create)();
        for step in 0..4000u32 {
            let k = space[rng.usize_below(space.len())];
            let op = match rng.below(100) {
                0..=34 => MapOp::Put(k, rng.below(1000)),
                35..=39 => MapOp::PutNull(k),
                40..=64 => MapOp::Get(k),
                65..=79 => MapOp::Remove(k),
                80..=91 => MapOp::Contains(k),
                92..=98 => MapOp::Size,
                _ => MapOp::Clear,
            };
            let rc = apply_map(&p.c, mc, op);
            let rr = apply_map(&p.r, mr, op);
            row.eq(&format!("step {} {:?}", step, op), &rc, &rr);
            let sc = snap_map_raw(mc);
            let sr = snap_map_raw(mr);
            if digest_map(&sc) != digest_map(&sr) {
                row.eq_map(&format!("step {} {:?} state", step, op), &sc, &sr);
                break;
            }
        }
        (p.c.hashmap_destroy)(mc);
        (p.r.hashmap_destroy)(mr);
    });
}

// ---------------------------------------------------------------------------
// C16..C33 — tree
// ---------------------------------------------------------------------------

fn c16(p: &Pair, h: &mut Harness) {
    h.row("C16", |row| unsafe {
        let (c, r) = both(p, |a| {
            let t = (a.tree_create)();
            let mut log = Vec::new();
            log!(log, "null={}", t.is_null());
            log!(log, "size={}", (a.tree_size)(t));
            log!(log, "contains(0)={}", (a.tree_contains)(t, 0));
            log!(log, "get(0)_null={}", (a.tree_get_node)(t, 0).is_null());
            log!(log, "depth(0)={}", (a.tree_get_depth)(t, 0));
            log!(log, "height(0)={}", (a.tree_get_height)(t, 0));
            log!(log, "desc(0)={}", (a.tree_count_descendants)(t, 0));
            let s = snap_tree(t);
            (a.tree_delete)(t);
            (log, s)
        });
        row.eq_logs("log", &c.0, &r.0);
        row.eq_tree("fresh tree", &c.1, &r.1);
    });
}

fn c17(p: &Pair, h: &mut Harness) {
    h.row("C17", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 17);
        let mut cases: Vec<(u64, u64)> = vec![(0, 0), (0, 7), (1, 0), (1, 99), (u64::MAX, 5), (7, 7)];
        for _ in 0..60 {
            cases.push((rng.key(), rng.key()));
        }
        for (id, parent) in cases {
            let (c, r) = both(p, |a| {
                let t = (a.tree_create)();
                let d = cstring(b"root");
                let mut log = Vec::new();
                log!(
                    log,
                    "add={}",
                    (a.tree_add_node)(t, id, parent, d.as_ptr())
                );
                log!(log, "size={}", (a.tree_size)(t));
                log!(log, "root_id={} has_root={}", (*t).root_id, (*t).has_root);
                let n = (a.tree_get_node)(t, id);
                log!(log, "node_null={}", n.is_null());
                if !n.is_null() {
                    log!(log, "node={:?}", snap_node(n));
                }
                log!(log, "depth={}", (a.tree_get_depth)(t, id));
                log!(log, "height={}", (a.tree_get_height)(t, id));
                log!(log, "desc={}", (a.tree_count_descendants)(t, id));
                let mut path = [0u64; 4];
                log!(
                    log,
                    "path={} {:?}",
                    (a.tree_find_path)(t, id, path.as_mut_ptr(), 4),
                    path
                );
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("id={} parent={} log", id, parent), &c.0, &r.0);
            row.eq_tree(&format!("id={} parent={} tree", id, parent), &c.1, &r.1);
        }
    });
}

fn c18_c19(p: &Pair, h: &mut Harness) {
    for (name, width) in [("C18", 31usize), ("C19", 32usize)] {
        h.row(name, |row| unsafe {
            for trial in 0..6u64 {
                let mut rng = Rng::new(SEED ^ 18 ^ (width as u64) ^ (trial << 16));
                let root = rng.key();
                let mut ids = vec![root];
                while ids.len() < width + 1 {
                    let k = rng.key();
                    if !ids.contains(&k) {
                        ids.push(k);
                    }
                }
                let (c, r) = both(p, |a| {
                    let t = (a.tree_create)();
                    let d = cstring(b"n");
                    let mut log = Vec::new();
                    log!(log, "root={}", (a.tree_add_node)(t, root, 0, d.as_ptr()));
                    for &id in ids[1..].iter() {
                        log!(
                            log,
                            "add({})={}",
                            id,
                            (a.tree_add_node)(t, id, root, d.as_ptr())
                        );
                    }
                    log!(log, "size={}", (a.tree_size)(t));
                    let rn = (a.tree_get_node)(t, root);
                    log!(log, "root_node={:?}", snap_node(rn));
                    log!(log, "height={}", (a.tree_get_height)(t, root));
                    log!(log, "desc={}", (a.tree_count_descendants)(t, root));
                    for &id in ids.iter() {
                        log!(log, "depth({})={}", id, (a.tree_get_depth)(t, id));
                    }
                    let s = snap_tree(t);
                    (a.tree_delete)(t);
                    (log, s)
                });
                row.eq_logs(&format!("trial{} log", trial), &c.0, &r.0);
                row.eq_tree(&format!("trial{} tree", trial), &c.1, &r.1);
            }
        });
    }
}

fn c20(p: &Pair, h: &mut Harness) {
    h.row("C20", |row| unsafe {
        for depth in [1usize, 2, 13, 14, 50] {
            let mut rng = Rng::new(SEED ^ 20 ^ depth as u64);
            let mut ids: Vec<u64> = Vec::new();
            while ids.len() < depth {
                let k = rng.key();
                if !ids.contains(&k) {
                    ids.push(k);
                }
            }
            let (c, r) = both(p, |a| {
                let t = (a.tree_create)();
                let d = cstring(b"chain");
                let mut log = Vec::new();
                for (i, &id) in ids.iter().enumerate() {
                    let parent = if i == 0 { 0 } else { ids[i - 1] };
                    log!(
                        log,
                        "add({},{})={}",
                        id,
                        parent,
                        (a.tree_add_node)(t, id, parent, d.as_ptr())
                    );
                }
                log!(log, "size={} cap={}", (a.tree_size)(t), (*(*t).node_map).capacity);
                for &id in ids.iter() {
                    log!(log, "depth({})={}", id, (a.tree_get_depth)(t, id));
                    log!(log, "height({})={}", id, (a.tree_get_height)(t, id));
                    log!(log, "desc({})={}", id, (a.tree_count_descendants)(t, id));
                    let mut path = vec![0u64; depth + 2];
                    let n = (a.tree_find_path)(t, id, path.as_mut_ptr(), (depth + 2) as c_int);
                    log!(log, "path({})={} {:?}", id, n, &path[..]);
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("depth{} log", depth), &c.0, &r.0);
            row.eq_tree(&format!("depth{} tree", depth), &c.1, &r.1);
        }
    });
}

/// Build a pseudo-random tree; returns the ids in insertion order.
unsafe fn build_random_tree(a: &Api, rng: &mut Rng, n: usize, log: &mut Vec<String>) -> (*mut Tree, Vec<u64>) {
    let t = (a.tree_create)();
    let shapes = data_shapes();
    let mut ids: Vec<u64> = Vec::new();
    let mut fanout = std::collections::HashMap::new();
    while ids.len() < n {
        let id = rng.key();
        if ids.contains(&id) {
            continue;
        }
        let parent = if ids.is_empty() {
            0
        } else {
            // prefer parents that still have room
            let cands: Vec<u64> = ids
                .iter()
                .cloned()
                .filter(|i| *fanout.get(i).unwrap_or(&0usize) < MAX_CHILDREN)
                .collect();
            if cands.is_empty() {
                break;
            }
            cands[rng.usize_below(cands.len())]
        };
        let (_dn, d) = &shapes[rng.usize_below(shapes.len())];
        let dp = match d {
            None => std::ptr::null(),
            Some(v) => v.as_ptr(),
        };
        let rc = (a.tree_add_node)(t, id, parent, dp);
        log.push(format!("add({},{})={}", id, parent, rc));
        if rc == 0 {
            if !ids.is_empty() {
                *fanout.entry(parent).or_insert(0usize) += 1;
            }
            ids.push(id);
        }
    }
    (t, ids)
}

fn c21_c24(p: &Pair, h: &mut Harness) {
    h.row("C21+C24", |row| unsafe {
        for trial in 0..6u64 {
            let (c, r) = both(p, |a| {
                let mut rng = Rng::new(SEED ^ 21 ^ (trial << 8));
                let mut log = Vec::new();
                let (t, ids) = build_random_tree(a, &mut rng, 200, &mut log);
                log!(log, "size={} cap={}", (a.tree_size)(t), (*(*t).node_map).capacity);
                for &id in ids.iter() {
                    let n = (a.tree_get_node)(t, id);
                    log!(log, "node({})={:?}", id, snap_node(n));
                    log!(log, "depth={}", (a.tree_get_depth)(t, id));
                    log!(log, "height={}", (a.tree_get_height)(t, id));
                    log!(log, "desc={}", (a.tree_count_descendants)(t, id));
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("trial{} log", trial), &c.0, &r.0);
            row.eq_tree(&format!("trial{} tree", trial), &c.1, &r.1);
        }
    });
}

fn c22_c23(p: &Pair, h: &mut Harness) {
    h.row("C22+C23", |row| unsafe {
        let shapes = data_shapes();
        for (name, d) in shapes.iter() {
            let (c, r) = both(p, |a| {
                let t = (a.tree_create)();
                let mut log = Vec::new();
                let dp = match d {
                    None => std::ptr::null(),
                    Some(v) => v.as_ptr(),
                };
                log!(log, "root={}", (a.tree_add_node)(t, 1, 0, dp));
                log!(log, "child={}", (a.tree_add_node)(t, 2, 1, dp));
                let n = (a.tree_get_node)(t, 1);
                log!(log, "data={:?}", cstr_bytes(&(*n).data));
                log!(log, "data_len={}", cstr_bytes(&(*n).data).len());
                // when `data != NULL` strncpy defines all 256 bytes
                if d.is_some() {
                    log!(log, "full_data={:?}", (*n).data.to_vec());
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("{} log", name), &c.0, &r.0);
            row.eq_tree(&format!("{} tree", name), &c.1, &r.1);
        }
        // truncation boundary: exactly MAX_DATA_LENGTH-1 characters survive
        for n in [254usize, 255, 256, 300] {
            let d = cstring(&vec![b'Z'; n]);
            let (c, r) = both(p, |a| {
                let t = (a.tree_create)();
                (a.tree_add_node)(t, 1, 0, d.as_ptr());
                let nd = (a.tree_get_node)(t, 1);
                let out = cstr_bytes(&(*nd).data);
                let s = (out.len(), out, (*nd).data[MAX_DATA_LENGTH - 1]);
                (a.tree_delete)(t);
                s
            });
            row.eq(&format!("trunc len={}", n), &c, &r);
            row.ok(
                &format!("trunc len={} -> {}", n, c.0),
                c.0 == n.min(MAX_DATA_LENGTH - 1),
            );
        }
    });
}

fn c25_c26(p: &Pair, h: &mut Harness) {
    h.row("C25", |row| unsafe {
        // remove first / middle / last child of a 32-wide parent, a subtree root
        // and the tree root
        for victim in [0usize, 1, 15, 30, 31] {
            let (c, r) = both(p, |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                let mut log = Vec::new();
                (a.tree_add_node)(t, 1, 0, d.as_ptr());
                for i in 0..32u64 {
                    (a.tree_add_node)(t, 100 + i, 1, d.as_ptr());
                }
                // give one child a subtree of its own
                (a.tree_add_node)(t, 500, 100 + victim as u64, d.as_ptr());
                (a.tree_add_node)(t, 501, 500, d.as_ptr());
                log!(
                    log,
                    "rm({})={}",
                    100 + victim as u64,
                    (a.tree_remove_node)(t, 100 + victim as u64)
                );
                log!(log, "size={}", (a.tree_size)(t));
                log!(log, "root={:?}", snap_node((a.tree_get_node)(t, 1)));
                log!(log, "contains500={}", (a.tree_contains)(t, 500));
                log!(log, "contains501={}", (a.tree_contains)(t, 501));
                log!(log, "rm_root={}", (a.tree_remove_node)(t, 1));
                log!(
                    log,
                    "size={} root_id={} has_root={}",
                    (a.tree_size)(t),
                    (*t).root_id,
                    (*t).has_root
                );
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("victim{} log", victim), &c.0, &r.0);
            row.eq_tree(&format!("victim{} tree", victim), &c.1, &r.1);
        }
    });

    h.row("C26", |row| unsafe {
        for trial in 0..6u64 {
            let (c, r) = both(p, |a| {
                let mut rng = Rng::new(SEED ^ 26 ^ (trial << 8));
                let mut log = Vec::new();
                let (t, ids) = build_random_tree(a, &mut rng, 30, &mut log);
                let root = (*t).root_id;
                log!(log, "rm_root({})={}", root, (a.tree_remove_node)(t, root));
                log!(
                    log,
                    "size={} root_id={} has_root={}",
                    (a.tree_size)(t),
                    (*t).root_id,
                    (*t).has_root
                );
                for &id in ids.iter() {
                    log!(log, "contains({})={}", id, (a.tree_contains)(t, id));
                }
                // re-root the (now empty but tombstoned) map
                let d = cstring(b"new-root");
                let nid = rng.key();
                log!(log, "new_root({})={}", nid, (a.tree_add_node)(t, nid, 0, d.as_ptr()));
                let kid = rng.key();
                log!(log, "kid({})={}", kid, (a.tree_add_node)(t, kid, nid, d.as_ptr()));
                log!(
                    log,
                    "size={} root_id={} has_root={}",
                    (a.tree_size)(t),
                    (*t).root_id,
                    (*t).has_root
                );
                log!(log, "depth={}", (a.tree_get_depth)(t, kid));
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("trial{} log", trial), &c.0, &r.0);
            row.eq_tree(&format!("trial{} tree", trial), &c.1, &r.1);
        }
    });
}

fn c27(p: &Pair, h: &mut Harness) {
    h.row("C27", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 27);
        let mut logc = Vec::new();
        let mut logr = Vec::new();
        let mut rc0 = Rng::new(SEED ^ 27 ^ 0xABCD);
        let (tc, ids) = build_random_tree(&p.c, &mut rc0, 200, &mut logc);
        let mut rr0 = Rng::new(SEED ^ 27 ^ 0xABCD);
        let (tr, ids2) = build_random_tree(&p.r, &mut rr0, 200, &mut logr);
        row.eq_logs("build log", &logc, &logr);
        row.eq("same ids", &ids, &ids2);
        let mut order = ids.clone();
        // shuffle
        for i in (1..order.len()).rev() {
            let j = rng.usize_below(i + 1);
            order.swap(i, j);
        }
        let mut red = Redirect::start("c27");
        for (step, id) in order.iter().enumerate() {
            let a = (p.c.tree_remove_node)(tc, *id);
            let (oc, ec) = red.take();
            let b = (p.r.tree_remove_node)(tr, *id);
            let (or_, er) = red.take();
            row.eq(&format!("step {} rm({})", step, id), a, b);
            row.eq_bytes(&format!("step {} rm({}) stdout", step, id), &oc, &or_);
            row.eq_bytes(&format!("step {} rm({}) stderr", step, id), &ec, &er);
            row.eq(
                &format!("step {} size", step),
                (p.c.tree_size)(tc),
                (p.r.tree_size)(tr),
            );
            let sc = snap_tree(tc);
            let sr = snap_tree(tr);
            if sc != sr {
                row.eq_tree(&format!("step {} state", step), &sc, &sr);
                break;
            }
        }
        red.stop();
        (p.c.tree_delete)(tc);
        (p.r.tree_delete)(tr);
    });
}

fn c28_c29_c30(p: &Pair, h: &mut Harness) {
    h.row("C28+C29+C30", |row| unsafe {
        // queries on every node of: chain, star, random tree, root id 0
        for kind in 0..4u64 {
            let (c, r) = both(p, |a| {
                let mut rng = Rng::new(SEED ^ 28 ^ kind);
                let mut log = Vec::new();
                let d = cstring(b"q");
                let t;
                let ids: Vec<u64>;
                match kind {
                    0 => {
                        // 50-deep chain
                        t = (a.tree_create)();
                        let mut v = Vec::new();
                        for i in 0..50u64 {
                            let parent = if i == 0 { 0 } else { i };
                            (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                            v.push(i + 1);
                        }
                        ids = v;
                    }
                    1 => {
                        // 32-wide star
                        t = (a.tree_create)();
                        let mut v = vec![7u64];
                        (a.tree_add_node)(t, 7, 0, d.as_ptr());
                        for i in 0..32u64 {
                            (a.tree_add_node)(t, 1000 + i, 7, d.as_ptr());
                            v.push(1000 + i);
                        }
                        ids = v;
                    }
                    2 => {
                        let (tt, v) = build_random_tree(a, &mut rng, 120, &mut log);
                        t = tt;
                        ids = v;
                    }
                    _ => {
                        // root id 0 (root_id == 0 is also the "no root" sentinel)
                        t = (a.tree_create)();
                        let mut v = vec![0u64];
                        (a.tree_add_node)(t, 0, 0, d.as_ptr());
                        for i in 1..10u64 {
                            (a.tree_add_node)(t, i, i - 1, d.as_ptr());
                            v.push(i);
                        }
                        ids = v;
                    }
                }
                for &id in ids.iter() {
                    log!(log, "depth({})={}", id, (a.tree_get_depth)(t, id));
                    log!(log, "height({})={}", id, (a.tree_get_height)(t, id));
                    log!(log, "desc({})={}", id, (a.tree_count_descendants)(t, id));
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("kind{} log", kind), &c.0, &r.0);
            row.eq_tree(&format!("kind{} tree", kind), &c.1, &r.1);
        }
    });
}

fn c31(p: &Pair, h: &mut Harness) {
    h.row("C31", |row| unsafe {
        let depth = 50usize;
        for &max_len in [
            -2147483648i32,
            -100,
            -1,
            0,
            1,
            2,
            depth as i32 - 1,
            depth as i32,
            depth as i32 + 1,
            1000,
            2147483647,
        ]
        .iter()
        {
            let (c, r) = both(p, |a| {
                let d = cstring(b"p");
                let t = (a.tree_create)();
                for i in 0..depth as u64 {
                    let parent = if i == 0 { 0 } else { i };
                    (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                }
                let mut log = Vec::new();
                // deepest node, root, and a middle node
                for &id in [depth as u64, 1u64, 25u64].iter() {
                    let mut buf = vec![0xEEu64; depth + 8];
                    let n = (a.tree_find_path)(t, id, buf.as_mut_ptr(), max_len);
                    log!(log, "find_path({},{})={}", id, max_len, n);
                    log!(log, "  buf={:?}", buf);
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("max_len={} log", max_len), &c.0, &r.0);
            row.eq_tree(&format!("max_len={} tree", max_len), &c.1, &r.1);
        }
    });
}

fn c32(p: &Pair, h: &mut Harness) {
    h.row("C32", |row| unsafe {
        for kind in 0..7u64 {
            let mut outs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            for a in [&p.c, &p.r] {
                let mut red = Redirect::start("c32");
                let d = cstring(b"data");
                let t = (a.tree_create)();
                match kind {
                    0 => {}
                    1 => {
                        (a.tree_add_node)(t, 1, 0, d.as_ptr());
                    }
                    2 => {
                        for i in 0..50u64 {
                            let parent = if i == 0 { 0 } else { i };
                            (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                        }
                    }
                    3 => {
                        (a.tree_add_node)(t, 1, 0, d.as_ptr());
                        for i in 0..32u64 {
                            (a.tree_add_node)(t, 10 + i, 1, d.as_ptr());
                        }
                    }
                    4 => {
                        // the complex tree of main.c
                        let names: [&[u8]; 10] = [
                            b"root", b"child1", b"child2", b"child3", b"gc1", b"gc2", b"gc3",
                            b"gc4", b"gc5", b"ggc1",
                        ];
                        let parents = [0u64, 1, 1, 1, 2, 2, 3, 4, 4, 7];
                        for i in 0..10usize {
                            let n = cstring(names[i]);
                            (a.tree_add_node)(t, i as u64 + 1, parents[i], n.as_ptr());
                        }
                    }
                    5 => {
                        // truncated + non-UTF-8 data
                        let long = cstring(&vec![b'L'; 400]);
                        let bin = cstring(&(1u8..=255).collect::<Vec<u8>>());
                        let empty = cstring(b"");
                        (a.tree_add_node)(t, 1, 0, long.as_ptr());
                        (a.tree_add_node)(t, 2, 1, bin.as_ptr());
                        (a.tree_add_node)(t, 3, 1, std::ptr::null());
                        (a.tree_add_node)(t, 4, 1, empty.as_ptr());
                    }
                    _ => {
                        // after removals
                        for i in 0..20u64 {
                            let parent = if i == 0 { 0 } else { i / 2 + 1 };
                            (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                        }
                        (a.tree_remove_node)(t, 3);
                        (a.tree_remove_node)(t, 5);
                    }
                }
                (a.tree_print)(t);
                (a.tree_delete)(t);
                let cap = red.take();
                red.stop();
                outs.push(cap);
            }
            row.eq_bytes(&format!("kind{} stdout", kind), &outs[0].0, &outs[1].0);
            row.eq_bytes(&format!("kind{} stderr", kind), &outs[0].1, &outs[1].1);
        }
    });
}

#[derive(Debug, Clone, Copy)]
enum TreeOp {
    Add(u64, u64, usize),
    Remove(u64),
    Get(u64),
    Contains(u64),
    Size,
    Depth(u64),
    Height(u64),
    Desc(u64),
    FindPath(u64, c_int),
    Print,
}

unsafe fn apply_tree(a: &Api, t: *mut Tree, op: TreeOp, shapes: &[(String, Option<Vec<u8>>)]) -> String {
    match op {
        TreeOp::Add(id, parent, di) => {
            let dp = match &shapes[di].1 {
                None => std::ptr::null(),
                Some(v) => v.as_ptr(),
            };
            format!("add({},{},{})={}", id, parent, shapes[di].0, (a.tree_add_node)(t, id, parent, dp))
        }
        TreeOp::Remove(id) => format!("rm({})={}", id, (a.tree_remove_node)(t, id)),
        TreeOp::Get(id) => {
            let n = (a.tree_get_node)(t, id);
            if n.is_null() {
                format!("get({})=NULL", id)
            } else {
                format!("get({})={:?}", id, snap_node(n))
            }
        }
        TreeOp::Contains(id) => format!("has({})={}", id, (a.tree_contains)(t, id)),
        TreeOp::Size => format!("size={}", (a.tree_size)(t)),
        TreeOp::Depth(id) => format!("depth({})={}", id, (a.tree_get_depth)(t, id)),
        TreeOp::Height(id) => format!("height({})={}", id, (a.tree_get_height)(t, id)),
        TreeOp::Desc(id) => format!("desc({})={}", id, (a.tree_count_descendants)(t, id)),
        TreeOp::FindPath(id, ml) => {
            let mut buf = vec![0xAAu64; 64];
            let n = (a.tree_find_path)(t, id, buf.as_mut_ptr(), ml);
            format!("path({},{})={} {:?}", id, ml, n, buf)
        }
        TreeOp::Print => {
            (a.tree_print)(t);
            "print".to_string()
        }
    }
}

fn c33(p: &Pair, h: &mut Harness) {
    h.row("C33", |row| unsafe {
        let shapes = data_shapes();
        let mut rng = Rng::new(SEED ^ 33);
        let space: Vec<u64> = {
            let mut v: Vec<u64> = vec![0, 1, u64::MAX];
            while v.len() < 40 {
                let k = rng.key();
                if !v.contains(&k) {
                    v.push(k);
                }
            }
            v
        };
        let tc = (p.c.tree_create)();
        let tr = (p.r.tree_create)();
        let mut red = Redirect::start("c33");
        let mut diverged = false;
        for step in 0..3000u32 {
            let id = space[rng.usize_below(space.len())];
            let parent = space[rng.usize_below(space.len())];
            let op = match rng.below(100) {
                0..=29 => TreeOp::Add(id, parent, rng.usize_below(shapes.len())),
                30..=44 => TreeOp::Remove(id),
                45..=54 => TreeOp::Get(id),
                55..=62 => TreeOp::Contains(id),
                63..=66 => TreeOp::Size,
                67..=73 => TreeOp::Depth(id),
                74..=80 => TreeOp::Height(id),
                81..=87 => TreeOp::Desc(id),
                88..=96 => TreeOp::FindPath(
                    id,
                    match rng.below(6) {
                        0 => 0,
                        1 => -1,
                        2 => 1,
                        3 => 3,
                        4 => 64,
                        _ => i32::MAX,
                    },
                ),
                _ => TreeOp::Print,
            };
            let sc = apply_tree(&p.c, tc, op, &shapes);
            let (oc, ec) = red.take();
            let sr = apply_tree(&p.r, tr, op, &shapes);
            let (or_, er) = red.take();
            row.eq(&format!("step {} {:?}", step, op), &sc, &sr);
            row.eq_bytes(&format!("step {} {:?} stdout", step, op), &oc, &or_);
            row.eq_bytes(&format!("step {} {:?} stderr", step, op), &ec, &er);
            let snc = snap_tree(tc);
            let snr = snap_tree(tr);
            if snc != snr {
                row.eq_tree(&format!("step {} {:?} state", step, op), &snc, &snr);
                diverged = true;
                break;
            }
        }
        red.stop();
        (p.c.tree_delete)(tc);
        (p.r.tree_delete)(tr);
        let _ = diverged;
    });
}

// ---------------------------------------------------------------------------
// C34, C35 — the exported wrappers of main.c
// ---------------------------------------------------------------------------

fn c34(p: &Pair, h: &mut Harness) {
    h.row("C34", |row| unsafe {
        for name in TEST_FUNCS.iter() {
            let fc = p.c.test_fn(name);
            let fr = p.r.test_fn(name);
            let cc = capture_fork(|| fc());
            let cr = capture_fork(|| fr());
            row.eq_bytes(&format!("{} stdout", name), &cc.out, &cr.out);
            row.eq_bytes(&format!("{} stderr", name), &cc.err, &cr.err);
            row.eq(&format!("{} exit", name), cc.exit, cr.exit);
            row.eq(&format!("{} signal", name), cc.signal, cr.signal);
            row.ok(
                &format!("{} did not abort (C)", name),
                cc.exit == Some(0) && cc.signal.is_none(),
            );
        }
    });
}

fn c35(p: &Pair, h: &mut Harness) {
    h.row("C35", |row| unsafe {
        let mc = p.c.main;
        let mr = p.r.main;
        let cc = capture_fork(|| {
            let rc = mc();
            libc::fflush(std::ptr::null_mut());
            if rc != 0 {
                libc::_exit(rc);
            }
        });
        let cr = capture_fork(|| {
            let rc = mr();
            libc::fflush(std::ptr::null_mut());
            if rc != 0 {
                libc::_exit(rc);
            }
        });
        row.eq_bytes("main stdout", &cc.out, &cr.out);
        row.eq_bytes("main stderr", &cc.err, &cr.err);
        row.eq("main exit", cc.exit, cr.exit);
        row.eq("main signal", cc.signal, cr.signal);
        row.ok("main stdout non-empty", !cc.out.is_empty());
    });
}

/// C36 — the `length < 1000` cap of `tree_find_path` (reachable with a chain
/// deeper than 1000) and re-adding a child after the 32-child limit was hit.
fn c36(p: &Pair, h: &mut Harness) {
    h.row("C36", |row| unsafe {
        for depth in [999usize, 1000, 1001, 1200] {
            let (c, r) = both(p, |a| {
                let d = cstring(b"deep");
                let t = (a.tree_create)();
                for i in 0..depth as u64 {
                    let parent = if i == 0 { 0 } else { i };
                    (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                }
                let mut log = Vec::new();
                log!(log, "size={}", (a.tree_size)(t));
                log!(log, "depth={}", (a.tree_get_depth)(t, depth as u64));
                log!(log, "height={}", (a.tree_get_height)(t, 1));
                log!(log, "desc={}", (a.tree_count_descendants)(t, 1));
                for &ml in [0i32, 1, 999, 1000, 1001, 1100, i32::MAX].iter() {
                    let mut buf = vec![0xEEu64; 1100];
                    let n = (a.tree_find_path)(t, depth as u64, buf.as_mut_ptr(), ml.min(1100));
                    log!(log, "find_path(max={})={}", ml.min(1100), n);
                    log!(log, "  first={:?} last={:?}", &buf[..4], &buf[996..1004]);
                }
                let s = snap_tree(t);
                (a.tree_delete)(t);
                (log, s)
            });
            row.eq_logs(&format!("depth={} log", depth), &c.0, &r.0);
            row.eq_tree(&format!("depth={} tree", depth), &c.1, &r.1);
        }
        // re-add a child after MAX_CHILDREN was reached and one child removed
        let (c, r) = both(p, |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            let mut log = Vec::new();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            for i in 0..MAX_CHILDREN as u64 {
                (a.tree_add_node)(t, 100 + i, 1, d.as_ptr());
            }
            log!(log, "overflow={}", (a.tree_add_node)(t, 999, 1, d.as_ptr()));
            log!(log, "rm={}", (a.tree_remove_node)(t, 100));
            log!(log, "readd={}", (a.tree_add_node)(t, 999, 1, d.as_ptr()));
            let root = (a.tree_get_node)(t, 1);
            log!(log, "root={:?}", snap_node(root));
            log!(log, "again={}", (a.tree_add_node)(t, 1000, 1, d.as_ptr()));
            let s = snap_tree(t);
            (a.tree_delete)(t);
            (log, s)
        });
        row.eq_logs("refill log", &c.0, &r.0);
        row.eq_tree("refill tree", &c.1, &r.1);
    });
}

fn main() {
    let p = load_pair();
    let mut h = Harness::new("Phase B - valid-path differential tests");
    c1(&p, &mut h);
    c2(&p, &mut h);
    c3_c4_c5(&p, &mut h);
    c6(&p, &mut h);
    c7(&p, &mut h);
    c8(&p, &mut h);
    c9_c10_c11(&p, &mut h);
    c12_c13_c14(&p, &mut h);
    c15(&p, &mut h);
    c16(&p, &mut h);
    c17(&p, &mut h);
    c18_c19(&p, &mut h);
    c20(&p, &mut h);
    c21_c24(&p, &mut h);
    c22_c23(&p, &mut h);
    c25_c26(&p, &mut h);
    c27(&p, &mut h);
    c28_c29_c30(&p, &mut h);
    c31(&p, &mut h);
    c32(&p, &mut h);
    c33(&p, &mut h);
    c34(&p, &mut h);
    c35(&p, &mut h);
    c36(&p, &mut h);
    h.finish();
}
