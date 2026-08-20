//! Phase C — error-path differential tests (one row per `ERRORS.md` row).
//!
//! Every row constructs the exact invalid input/condition the C code rejects and
//! asserts that both `.so`s return the *same* sentinel/error code and print the
//! same diagnostic.

mod common;

use common::*;
use std::ffi::c_void;
use std::ptr;

extern "C" {
    #[link_name = "stdout"]
    static mut c_stdout: *mut libc::FILE;
    #[link_name = "stderr"]
    static mut c_stderr: *mut libc::FILE;
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Run `f` against both libraries and compare the result plus both streams.
unsafe fn diff<T: PartialEq + std::fmt::Debug>(
    row: &mut Row,
    p: &Pair,
    what: &str,
    f: impl Fn(&Api) -> T,
) {
    let (vc, oc, ec) = capture_ret(|| f(&p.c));
    let (vr, or_, er) = capture_ret(|| f(&p.r));
    row.eq(&format!("{} result", what), &vc, &vr);
    row.eq_bytes(&format!("{} stdout", what), &oc, &or_);
    row.eq_bytes(&format!("{} stderr", what), &ec, &er);
}

/// A `hashmap_t` built by the *test* (the struct is public in `hashmap.h`), used
/// to reach the "probe exhausted" branches: `size` is kept low enough that
/// `should_resize()` stays false, so the libraries never re-`calloc`/`free` the
/// test-owned entry array.
struct Crafted {
    map: Box<Hashmap>,
    _entries: Vec<HashmapEntry>,
}

impl Crafted {
    /// `capacity` slots, all `occupied` and not `deleted`, holding `keys`
    /// (padded with `pad_from..` filler keys), with a deliberately low `size`.
    fn full(capacity: usize, keys: &[u64], values: &[*mut c_void]) -> Crafted {
        assert_eq!(keys.len(), capacity);
        let mut entries: Vec<HashmapEntry> = (0..capacity)
            .map(|i| HashmapEntry {
                key: keys[i],
                value: values[i],
                occupied: 1,
                deleted: 0,
            })
            .collect();
        // `size` must keep (size + deleted_count) / capacity <= 0.75 so that
        // `should_resize()` stays false — otherwise the library would `calloc` a
        // new table and `free` this test-owned one.
        let size = capacity * 3 / 4;
        assert!(size as f64 / capacity as f64 <= 0.75);
        let map = Box::new(Hashmap {
            entries: entries.as_mut_ptr(),
            capacity,
            size,
            deleted_count: 0,
        });
        Crafted {
            map,
            _entries: entries,
        }
    }
    fn ptr(&mut self) -> *mut Hashmap {
        &mut *self.map as *mut Hashmap
    }
}

fn filler_keys(cap: usize, seed: u64) -> Vec<u64> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::new();
    while v.len() < cap {
        let k = rng.next_u64() | 1; // keep them distinct from the probe keys
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

// ---------------------------------------------------------------------------
// E1, E2, E5, E21, E25 — allocation failures (LD_PRELOAD injector)
// ---------------------------------------------------------------------------

/// The child half of the allocation-failure tests: loads one `.so`, arms the
/// injector and runs a single case, printing the observable outcome.
fn child_main(case: String) {
    let which = std::env::var("CDIFF_LIB").unwrap();
    let arm: i64 = std::env::var("CDIFF_ARM").unwrap().parse().unwrap();
    let path = if which == "c" {
        c_so_path()
    } else {
        rust_so_path()
    };
    let api = Api::load(&path, "child");

    // Resolve the injector controls out of the preloaded shim.
    let shim = unsafe { libloading::Library::new(failalloc_so_path()).unwrap() };
    let arm_fn: libloading::Symbol<unsafe extern "C" fn(i64)> =
        unsafe { shim.get(b"failalloc_arm\0").unwrap() };
    let disarm_fn: libloading::Symbol<unsafe extern "C" fn()> =
        unsafe { shim.get(b"failalloc_disarm\0").unwrap() };

    unsafe {
        // Force stdio buffer allocation *before* arming so the injected failure
        // hits the library, not `printf`'s internal buffer.
        libc::fputs(b"# warmup\n\0".as_ptr() as *const i8, c_stdout);
        libc::fputs(b"# warmup\n\0".as_ptr() as *const i8, c_stderr);
        libc::fflush(ptr::null_mut());

        match case.as_str() {
            // E1 (arm=1) and E2 (arm=2)
            "hashmap_create" => {
                arm_fn(arm);
                let m = (api.hashmap_create)();
                disarm_fn();
                println!("RESULT null={}", m.is_null());
                if !m.is_null() {
                    println!(
                        "  cap={} size={} del={}",
                        (*m).capacity,
                        (*m).size,
                        (*m).deleted_count
                    );
                    (api.hashmap_destroy)(m);
                }
            }
            // E21 (arm=1: tree malloc, 2: map malloc, 3: entries calloc)
            "tree_create" => {
                arm_fn(arm);
                let t = (api.tree_create)();
                disarm_fn();
                println!("RESULT null={}", t.is_null());
                if !t.is_null() {
                    println!(
                        "  root={} has_root={} count={}",
                        (*t).root_id,
                        (*t).has_root,
                        (*t).node_count
                    );
                    (api.tree_delete)(t);
                }
            }
            // E5: resize failure inside hashmap_put
            "hashmap_resize" => {
                let m = (api.hashmap_create)();
                for i in 0..13u64 {
                    (api.hashmap_put)(m, i + 1, token(i));
                }
                println!(
                    "before cap={} size={} del={}",
                    (*m).capacity,
                    (*m).size,
                    (*m).deleted_count
                );
                arm_fn(arm);
                let rc = (api.hashmap_put)(m, 1000, token(99));
                disarm_fn();
                println!("RESULT put={}", rc);
                println!(
                    "after cap={} size={} del={}",
                    (*m).capacity,
                    (*m).size,
                    (*m).deleted_count
                );
                for i in 0..13u64 {
                    println!("  get({})={}", i + 1, (api.hashmap_get)(m, i + 1) as u64);
                }
                println!("  get(1000)={}", (api.hashmap_get)(m, 1000) as u64);
                (api.hashmap_destroy)(m);
            }
            // E25: node malloc failure inside tree_add_node
            "tree_add_node_malloc" => {
                let t = (api.tree_create)();
                let d = cstring(b"root");
                (api.tree_add_node)(t, 1, 0, d.as_ptr());
                arm_fn(arm);
                let rc = (api.tree_add_node)(t, 2, 1, d.as_ptr());
                disarm_fn();
                println!("RESULT add={}", rc);
                println!("size={} contains2={}", (api.tree_size)(t), (api.tree_contains)(t, 2));
                let root = (api.tree_get_node)(t, 1);
                println!("root_child_count={}", (*root).child_count);
                (api.tree_delete)(t);
            }
            // E25 for the very first node (no parent involved)
            "tree_add_root_malloc" => {
                let t = (api.tree_create)();
                let d = cstring(b"root");
                arm_fn(arm);
                let rc = (api.tree_add_node)(t, 1, 0, d.as_ptr());
                disarm_fn();
                println!("RESULT add={}", rc);
                println!(
                    "size={} has_root={} root_id={}",
                    (api.tree_size)(t),
                    (*t).has_root,
                    (*t).root_id
                );
                (api.tree_delete)(t);
            }
            other => panic!("unknown child case {}", other),
        }
        libc::fflush(ptr::null_mut());
    }
}

fn alloc_failure_rows(h: &mut Harness) {
    let shim = failalloc_so_path();
    let exe = std::env::current_exe().unwrap();
    let run = |case: &str, arm: i64, which: &str| -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let out = std::process::Command::new(&exe)
            .env("CDIFF_CHILD_CASE", case)
            .env("CDIFF_LIB", which)
            .env("CDIFF_ARM", arm.to_string())
            .env("LD_PRELOAD", &shim)
            .output()
            .expect("spawn child");
        (out.stdout, out.stderr, out.status.code())
    };
    let cases: Vec<(&str, &str, i64)> = vec![
        ("E1", "hashmap_create", 1),
        ("E2", "hashmap_create", 2),
        ("E5", "hashmap_resize", 1),
        ("E21", "tree_create", 1),
        ("E21b", "tree_create", 2),
        ("E21c", "tree_create", 3),
        ("E25", "tree_add_node_malloc", 1),
        ("E25b", "tree_add_root_malloc", 1),
    ];
    for (row_name, case, arm) in cases {
        h.row(row_name, |row| {
            let (oc, ec, sc) = run(case, arm, "c");
            let (or_, er, sr) = run(case, arm, "rust");
            row.eq_bytes(&format!("{} stdout", case), &oc, &or_);
            row.eq_bytes(&format!("{} stderr", case), &ec, &er);
            row.eq(&format!("{} exit", case), sc, sr);
            row.ok(
                &format!("{} child produced a RESULT line", case),
                String::from_utf8_lossy(&oc).contains("RESULT"),
            );
            row.ok(
                &format!("{} allocation failure was actually injected", case),
                String::from_utf8_lossy(&oc).contains("null=true")
                    || String::from_utf8_lossy(&oc).contains("=-1"),
            );
        });
    }
}

// ---------------------------------------------------------------------------
// E3..E20 — hashmap
// ---------------------------------------------------------------------------

fn hashmap_rows(p: &Pair, h: &mut Harness) {
    h.row("E3", |row| unsafe {
        diff(row, p, "hashmap_destroy(NULL)", |a| {
            (a.hashmap_destroy)(ptr::null_mut());
            "returned"
        });
    });

    h.row("E4", |row| unsafe {
        for (k, v) in [(0u64, ptr::null_mut()), (7, token(1)), (u64::MAX, token(2))] {
            diff(row, p, &format!("hashmap_put(NULL,{})", k), |a| {
                (a.hashmap_put)(ptr::null_mut(), k, v)
            });
        }
    });

    h.row("E6", |row| unsafe {
        // every slot occupied, no match -> probe == capacity -> -1
        for cap in [1usize, 2, 4, 8, 16] {
            let keys = filler_keys(cap, SEED ^ cap as u64);
            let vals: Vec<*mut c_void> = (0..cap).map(|_| ptr::null_mut()).collect();
            let probe: u64 = 0xFFFF_FFFF_0000_0000; // even -> not in `keys`
            let (vc, oc, ec) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                let rc = (p.c.hashmap_put)(mp, probe, token(5));
                (rc, (*mp).size, (*mp).deleted_count, snap_map_raw(mp))
            });
            let (vr, or_, er) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                let rc = (p.r.hashmap_put)(mp, probe, token(5));
                (rc, (*mp).size, (*mp).deleted_count, snap_map_raw(mp))
            });
            row.eq(&format!("cap={} put rc/size", cap), (vc.0, vc.1, vc.2), (vr.0, vr.1, vr.2));
            row.eq_map(&format!("cap={} map", cap), &vc.3, &vr.3);
            row.eq_bytes(&format!("cap={} stdout", cap), &oc, &or_);
            row.eq_bytes(&format!("cap={} stderr", cap), &ec, &er);
            row.ok(&format!("cap={} put returned -1 (got {})", cap, vc.0), vc.0 == -1);
        }
    });

    h.row("E7", |row| unsafe {
        for k in [0u64, 1, u64::MAX] {
            diff(row, p, &format!("hashmap_get(NULL,{})", k), |a| {
                (a.hashmap_get)(ptr::null_mut(), k) as u64
            });
        }
    });

    h.row("E8", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 8008);
        for _ in 0..100 {
            let present: Vec<u64> = (0..5).map(|_| rng.key()).collect();
            let absent = rng.key();
            if present.contains(&absent) {
                continue;
            }
            diff(row, p, &format!("miss({})", absent), |a| {
                let m = (a.hashmap_create)();
                for (i, &k) in present.iter().enumerate() {
                    (a.hashmap_put)(m, k, token(i as u64));
                }
                let g = (a.hashmap_get)(m, absent) as u64;
                let c = (a.hashmap_contains)(m, absent);
                let s = (a.hashmap_size)(m);
                (a.hashmap_destroy)(m);
                (g, c, s)
            });
        }
    });

    h.row("E9", |row| unsafe {
        for cap in [1usize, 4, 16] {
            let keys = filler_keys(cap, SEED ^ 9 ^ cap as u64);
            let vals: Vec<*mut c_void> = (0..cap).map(|i| token(i as u64)).collect();
            let probe: u64 = 0xFFFF_FFFF_0000_0000;
            let (vc, oc, ec) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                (
                    (p.c.hashmap_get)(mp, probe) as u64,
                    (p.c.hashmap_contains)(mp, probe),
                    snap_map_raw(mp),
                )
            });
            let (vr, or_, er) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                (
                    (p.r.hashmap_get)(mp, probe) as u64,
                    (p.r.hashmap_contains)(mp, probe),
                    snap_map_raw(mp),
                )
            });
            row.eq(&format!("cap={} get/contains", cap), (vc.0, vc.1), (vr.0, vr.1));
            row.eq_map(&format!("cap={} map", cap), &vc.2, &vr.2);
            row.eq_bytes(&format!("cap={} stdout", cap), &oc, &or_);
            row.eq_bytes(&format!("cap={} stderr", cap), &ec, &er);
            row.ok(&format!("cap={} get returned NULL", cap), vc.0 == 0);
        }
    });

    h.row("E10", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 10010);
        for _ in 0..60 {
            let k = rng.key();
            diff(row, p, &format!("get-after-remove({})", k), |a| {
                let m = (a.hashmap_create)();
                (a.hashmap_put)(m, k, token(1));
                let rm = (a.hashmap_remove)(m, k) as u64;
                let g = (a.hashmap_get)(m, k) as u64;
                let c = (a.hashmap_contains)(m, k);
                let s = ((*m).size, (*m).deleted_count);
                let snap = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (rm, g, c, s, snap)
            });
        }
    });

    h.row("E11", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 11011);
        for _ in 0..60 {
            let k = rng.key();
            diff(row, p, &format!("null-value({})", k), |a| {
                let m = (a.hashmap_create)();
                let rc = (a.hashmap_put)(m, k, ptr::null_mut());
                let g = (a.hashmap_get)(m, k) as u64;
                let c = (a.hashmap_contains)(m, k);
                let s = (a.hashmap_size)(m);
                let snap = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (rc, g, c, s, snap)
            });
        }
    });

    h.row("E12", |row| unsafe {
        for k in [0u64, 3, u64::MAX] {
            diff(row, p, &format!("hashmap_remove(NULL,{})", k), |a| {
                (a.hashmap_remove)(ptr::null_mut(), k) as u64
            });
        }
    });

    h.row("E13", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 13013);
        for _ in 0..60 {
            let k = rng.key();
            let other = rng.key();
            if k == other {
                continue;
            }
            diff(row, p, &format!("remove-absent({})", other), |a| {
                let m = (a.hashmap_create)();
                (a.hashmap_put)(m, k, token(1));
                let rm = (a.hashmap_remove)(m, other) as u64;
                let st = ((*m).size, (*m).deleted_count);
                let snap = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (rm, st, snap)
            });
        }
    });

    h.row("E14", |row| unsafe {
        for cap in [1usize, 4, 16] {
            let keys = filler_keys(cap, SEED ^ 14 ^ cap as u64);
            let vals: Vec<*mut c_void> = (0..cap).map(|i| token(i as u64)).collect();
            let probe: u64 = 0xFFFF_FFFF_0000_0000;
            let (vc, oc, ec) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                ((p.c.hashmap_remove)(mp, probe) as u64, snap_map_raw(mp))
            });
            let (vr, or_, er) = capture_ret(|| {
                let mut m = Crafted::full(cap, &keys, &vals);
                let mp = m.ptr();
                ((p.r.hashmap_remove)(mp, probe) as u64, snap_map_raw(mp))
            });
            row.eq(&format!("cap={} remove", cap), vc.0, vr.0);
            row.eq_map(&format!("cap={} map", cap), &vc.1, &vr.1);
            row.eq_bytes(&format!("cap={} stdout", cap), &oc, &or_);
            row.eq_bytes(&format!("cap={} stderr", cap), &ec, &er);
            row.ok(&format!("cap={} remove returned NULL", cap), vc.0 == 0);
        }
    });

    h.row("E15", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 15015);
        for _ in 0..60 {
            let k = rng.key();
            diff(row, p, &format!("double-remove({})", k), |a| {
                let m = (a.hashmap_create)();
                (a.hashmap_put)(m, k, token(1));
                let first = (a.hashmap_remove)(m, k) as u64;
                let st1 = ((*m).size, (*m).deleted_count);
                let second = (a.hashmap_remove)(m, k) as u64;
                let st2 = ((*m).size, (*m).deleted_count);
                let snap = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (first, st1, second, st2, snap)
            });
        }
    });

    h.row("E16", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 16016);
        for _ in 0..60 {
            let k = rng.key();
            diff(row, p, &format!("remove-null-value({})", k), |a| {
                let m = (a.hashmap_create)();
                (a.hashmap_put)(m, k, ptr::null_mut());
                let before = ((*m).size, (*m).deleted_count);
                let rm = (a.hashmap_remove)(m, k) as u64;
                let after = ((*m).size, (*m).deleted_count);
                let snap = snap_map_raw(m);
                (a.hashmap_destroy)(m);
                (before, rm, after, snap)
            });
        }
    });

    h.row("E17", |row| unsafe {
        for k in [0u64, 5, u64::MAX] {
            diff(row, p, &format!("hashmap_contains(NULL,{})", k), |a| {
                (a.hashmap_contains)(ptr::null_mut(), k)
            });
        }
    });

    h.row("E18", |row| unsafe {
        // present-but-NULL-valued key reports 0 even though the slot is used
        diff(row, p, "contains(null-valued)", |a| {
            let m = (a.hashmap_create)();
            (a.hashmap_put)(m, 42, ptr::null_mut());
            let r = (
                (a.hashmap_contains)(m, 42),
                (a.hashmap_size)(m),
                (*m).size,
                snap_map_raw(m),
            );
            (a.hashmap_destroy)(m);
            r
        });
    });

    h.row("E19", |row| unsafe {
        diff(row, p, "hashmap_size(NULL)", |a| {
            (a.hashmap_size)(ptr::null_mut())
        });
    });

    h.row("E20", |row| unsafe {
        diff(row, p, "hashmap_clear(NULL)", |a| {
            (a.hashmap_clear)(ptr::null_mut());
            "returned"
        });
    });
}

// ---------------------------------------------------------------------------
// E22..E45 — tree
// ---------------------------------------------------------------------------

fn tree_rows(p: &Pair, h: &mut Harness) {
    h.row("E22", |row| unsafe {
        diff(row, p, "tree_delete(NULL)", |a| {
            (a.tree_delete)(ptr::null_mut());
            "returned"
        });
    });

    h.row("E23", |row| unsafe {
        let d = cstring(b"x");
        for (id, parent) in [(0u64, 0u64), (1, 0), (u64::MAX, 7)] {
            diff(row, p, &format!("tree_add_node(NULL,{},{})", id, parent), |a| {
                (a.tree_add_node)(ptr::null_mut(), id, parent, d.as_ptr())
            });
            diff(row, p, &format!("tree_add_node(NULL,{},{},NULL)", id, parent), |a| {
                (a.tree_add_node)(ptr::null_mut(), id, parent, ptr::null())
            });
        }
    });

    h.row("E24", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 24024);
        let mut ids: Vec<u64> = vec![0, 1, u64::MAX];
        for _ in 0..40 {
            ids.push(rng.key());
        }
        for id in ids {
            diff(row, p, &format!("duplicate({})", id), |a| {
                let d = cstring(b"first");
                let d2 = cstring(b"second");
                let t = (a.tree_create)();
                let r1 = (a.tree_add_node)(t, id, 0, d.as_ptr());
                let r2 = (a.tree_add_node)(t, id, 0, d2.as_ptr());
                let r3 = (a.tree_add_node)(t, id, id, ptr::null());
                let s = ((a.tree_size)(t), (*t).root_id, (*t).has_root);
                let snap = snap_tree(t);
                (a.tree_delete)(t);
                (r1, r2, r3, s, snap)
            });
        }
    });

    h.row("E26", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 26026);
        for _ in 0..40 {
            let root = rng.key();
            let parent = rng.key();
            if parent == root {
                continue;
            }
            diff(row, p, &format!("bad-parent({})", parent), |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                (a.tree_add_node)(t, root, 0, d.as_ptr());
                let rc = (a.tree_add_node)(t, root ^ 0x1234_5678, parent, d.as_ptr());
                let s = ((a.tree_size)(t), (*t).root_id, (*t).has_root);
                let snap = snap_tree(t);
                (a.tree_delete)(t);
                (rc, s, snap)
            });
        }
    });

    h.row("E27", |row| unsafe {
        diff(row, p, "max-children", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            let mut log = Vec::new();
            for i in 0..MAX_CHILDREN as u64 + 4 {
                log.push((a.tree_add_node)(t, 100 + i, 1, d.as_ptr()));
            }
            let root = (a.tree_get_node)(t, 1);
            let s = ((a.tree_size)(t), (*root).child_count);
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (log, s, snap)
        });
        // and on a non-root parent
        diff(row, p, "max-children-nonroot", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_add_node)(t, 2, 1, d.as_ptr());
            let mut log = Vec::new();
            for i in 0..MAX_CHILDREN as u64 + 2 {
                log.push((a.tree_add_node)(t, 200 + i, 2, d.as_ptr()));
            }
            let n2 = (a.tree_get_node)(t, 2);
            let s = ((a.tree_size)(t), (*n2).child_count);
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (log, s, snap)
        });
    });

    h.row("E28", |row| unsafe {
        // `hashmap_put` failure inside `tree_add_node`: swap in a full,
        // test-owned entry table (all values NULL except the parent node) so the
        // probe loop is exhausted.  The C code frees the node but leaves the id
        // in the parent's child list.
        let mut res = Vec::new();
        for a in [&p.c, &p.r] {
            let (v, o, e) = capture_ret(|| {
                let d = cstring(b"root");
                let t = (a.tree_create)();
                (a.tree_add_node)(t, 1, 0, d.as_ptr());
                let node1 = (a.tree_get_node)(t, 1) as *mut c_void;

                let m = (*t).node_map;
                let saved = ((*m).entries, (*m).capacity, (*m).size, (*m).deleted_count);
                let cap = 8usize;
                let mut keys = filler_keys(cap, SEED ^ 28);
                keys[3] = 1; // the parent must still be findable
                let vals: Vec<*mut c_void> = (0..cap)
                    .map(|i| if i == 3 { node1 } else { ptr::null_mut() })
                    .collect();
                let mut entries: Vec<HashmapEntry> = (0..cap)
                    .map(|i| HashmapEntry {
                        key: keys[i],
                        value: vals[i],
                        occupied: 1,
                        deleted: 0,
                    })
                    .collect();
                (*m).entries = entries.as_mut_ptr();
                (*m).capacity = cap;
                (*m).size = 2;
                (*m).deleted_count = 0;

                let rc = (a.tree_add_node)(t, 2, 1, d.as_ptr());
                let root = (a.tree_get_node)(t, 1);
                let out = (
                    rc,
                    (a.tree_size)(t),
                    (a.tree_contains)(t, 2),
                    (*root).child_count,
                    (*root).child_ids[0],
                );

                // restore the library-owned table before tearing the tree down
                (*m).entries = saved.0;
                (*m).capacity = saved.1;
                (*m).size = saved.2;
                (*m).deleted_count = saved.3;
                drop(entries);
                (a.tree_delete)(t);
                out
            });
            res.push((v, o, e));
        }
        row.eq("result", res[0].0, res[1].0);
        row.eq_bytes("stdout", &res[0].1, &res[1].1);
        row.eq_bytes("stderr", &res[0].2, &res[1].2);
        row.ok(&format!("add rejected (got {})", res[0].0 .0), res[0].0 .0 == -1);
        row.ok(
            "stale id left in the parent's child list (C behaviour)",
            res[0].0 .3 == 1 && res[0].0 .4 == 2,
        );
    });

    h.row("E29", |row| unsafe {
        for id in [0u64, 1, u64::MAX] {
            diff(row, p, &format!("tree_remove_node(NULL,{})", id), |a| {
                (a.tree_remove_node)(ptr::null_mut(), id)
            });
        }
    });

    h.row("E30", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 30030);
        for _ in 0..40 {
            let id = rng.key();
            diff(row, p, &format!("remove-missing({})", id), |a| {
                let t = (a.tree_create)();
                let r0 = (a.tree_remove_node)(t, id); // empty tree
                let d = cstring(b"n");
                (a.tree_add_node)(t, id ^ 0xFF, 0, d.as_ptr());
                let r1 = (a.tree_remove_node)(t, id);
                let s = ((a.tree_size)(t), (*t).root_id, (*t).has_root);
                let snap = snap_tree(t);
                (a.tree_delete)(t);
                (r0, r1, s, snap)
            });
        }
    });

    h.row("E31", |row| unsafe {
        diff(row, p, "remove-twice", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_add_node)(t, 2, 1, d.as_ptr());
            let r1 = (a.tree_remove_node)(t, 2);
            let r2 = (a.tree_remove_node)(t, 2);
            let r3 = (a.tree_remove_node)(t, 1);
            let r4 = (a.tree_remove_node)(t, 1);
            let s = ((a.tree_size)(t), (*t).root_id, (*t).has_root);
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (r1, r2, r3, r4, s, snap)
        });
    });

    h.row("E32+E33+E40", |row| unsafe {
        // `tree_node_t` is public, so a consumer can point a node at a parent
        // that is not in the map: the unlink step is skipped and
        // `tree_remove_subtree` sees a stale child id.
        diff(row, p, "orphan-parent", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_add_node)(t, 2, 1, d.as_ptr());
            (a.tree_add_node)(t, 3, 2, d.as_ptr());
            let n2 = (a.tree_get_node)(t, 2);
            (*n2).parent_id = 999; // not in the map
            let rc = (a.tree_remove_node)(t, 2);
            let root = (a.tree_get_node)(t, 1);
            let out = (
                rc,
                (a.tree_size)(t),
                (*root).child_count,
                (*root).child_ids[0],
                (a.tree_contains)(t, 2),
                (a.tree_contains)(t, 3),
                // stale child id -> tree_get_height/count_descendants see -1
                (a.tree_get_height)(t, 1),
                (a.tree_count_descendants)(t, 1),
                (a.tree_get_depth)(t, 1),
            );
            (a.tree_print)(t); // E40: a child id that is gone prints nothing
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (out, snap)
        });
        // E33: removing the root while its child list holds a stale id
        diff(row, p, "stale-child-remove-root", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_add_node)(t, 2, 1, d.as_ptr());
            let root = (a.tree_get_node)(t, 1);
            (*root).child_ids[0] = 12345; // never existed
            let rc = (a.tree_remove_node)(t, 1);
            let out = (
                rc,
                (a.tree_size)(t),
                (*t).has_root,
                (*t).root_id,
                (a.tree_contains)(t, 2),
            );
            let snap = snap_tree(t);
            // node 2 is now unreachable through the tree; free it via the map
            (a.tree_delete)(t);
            (out, snap)
        });
        // E40: has_root set but root_id absent from the map
        diff(row, p, "root_id-not-in-map", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_add_node)(t, 2, 1, d.as_ptr());
            (*t).root_id = 4242;
            (a.tree_print)(t);
            let out = (
                (a.tree_get_depth)(t, 2),
                (a.tree_get_depth)(t, 1),
                (a.tree_get_height)(t, 1),
                (a.tree_count_descendants)(t, 1),
            );
            let mut buf = [0u64; 8];
            let fp = (a.tree_find_path)(t, 2, buf.as_mut_ptr(), 8);
            (*t).root_id = 1;
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (out, fp, buf, snap)
        });
    });

    h.row("E34", |row| unsafe {
        for id in [0u64, 9, u64::MAX] {
            diff(row, p, &format!("tree_get_node(NULL,{})", id), |a| {
                (a.tree_get_node)(ptr::null_mut(), id).is_null()
            });
        }
    });

    h.row("E35", |row| unsafe {
        let mut rng = Rng::new(SEED ^ 35035);
        for _ in 0..40 {
            let id = rng.key();
            diff(row, p, &format!("get_node-missing({})", id), |a| {
                let t = (a.tree_create)();
                let a1 = (a.tree_get_node)(t, id).is_null();
                let d = cstring(b"n");
                (a.tree_add_node)(t, id ^ 1, 0, d.as_ptr());
                let a2 = (a.tree_get_node)(t, id).is_null();
                (a.tree_delete)(t);
                (a1, a2)
            });
        }
    });

    h.row("E36", |row| unsafe {
        for id in [0u64, 5, u64::MAX] {
            diff(row, p, &format!("tree_contains(NULL,{})", id), |a| {
                (a.tree_contains)(ptr::null_mut(), id)
            });
        }
    });

    h.row("E37", |row| unsafe {
        diff(row, p, "tree_size(NULL)", |a| (a.tree_size)(ptr::null_mut()));
    });

    h.row("E38", |row| unsafe {
        diff(row, p, "tree_print(NULL)", |a| {
            (a.tree_print)(ptr::null_mut());
            "printed"
        });
    });

    h.row("E39", |row| unsafe {
        diff(row, p, "tree_print(empty)", |a| {
            let t = (a.tree_create)();
            (a.tree_print)(t);
            let d = cstring(b"n");
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (a.tree_remove_node)(t, 1); // root removed -> has_root = 0
            (a.tree_print)(t);
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            snap
        });
    });

    h.row("E41", |row| unsafe {
        for id in [0u64, 3, u64::MAX] {
            diff(row, p, &format!("tree_get_depth(NULL,{})", id), |a| {
                (a.tree_get_depth)(ptr::null_mut(), id)
            });
        }
        let mut rng = Rng::new(SEED ^ 41041);
        for _ in 0..40 {
            let id = rng.key();
            diff(row, p, &format!("depth-missing({})", id), |a| {
                let t = (a.tree_create)();
                let e0 = (a.tree_get_depth)(t, id);
                let d = cstring(b"n");
                (a.tree_add_node)(t, id ^ 3, 0, d.as_ptr());
                let e1 = (a.tree_get_depth)(t, id);
                (a.tree_delete)(t);
                (e0, e1)
            });
        }
    });

    h.row("E42", |row| unsafe {
        for id in [0u64, 3, u64::MAX] {
            diff(row, p, &format!("tree_get_height(NULL,{})", id), |a| {
                (a.tree_get_height)(ptr::null_mut(), id)
            });
        }
        let mut rng = Rng::new(SEED ^ 42042);
        for _ in 0..40 {
            let id = rng.key();
            diff(row, p, &format!("height-missing({})", id), |a| {
                let t = (a.tree_create)();
                let e0 = (a.tree_get_height)(t, id);
                let d = cstring(b"n");
                (a.tree_add_node)(t, id ^ 3, 0, d.as_ptr());
                let e1 = (a.tree_get_height)(t, id);
                (a.tree_delete)(t);
                (e0, e1)
            });
        }
    });

    h.row("E43", |row| unsafe {
        for id in [0u64, 3, u64::MAX] {
            diff(row, p, &format!("tree_count_descendants(NULL,{})", id), |a| {
                (a.tree_count_descendants)(ptr::null_mut(), id)
            });
        }
        let mut rng = Rng::new(SEED ^ 43043);
        for _ in 0..40 {
            let id = rng.key();
            diff(row, p, &format!("desc-missing({})", id), |a| {
                let t = (a.tree_create)();
                let e0 = (a.tree_count_descendants)(t, id);
                let d = cstring(b"n");
                (a.tree_add_node)(t, id ^ 3, 0, d.as_ptr());
                let e1 = (a.tree_count_descendants)(t, id);
                (a.tree_delete)(t);
                (e0, e1)
            });
        }
    });

    h.row("E44", |row| unsafe {
        // NULL tree, NULL path, missing id — for a range of max_length values
        for &ml in [i32::MIN, -1, 0, 1, 10, i32::MAX].iter() {
            diff(row, p, &format!("find_path(NULL,..,{})", ml), |a| {
                let mut buf = [0xEEu64; 8];
                let r = (a.tree_find_path)(ptr::null_mut(), 1, buf.as_mut_ptr(), ml);
                (r, buf)
            });
            diff(row, p, &format!("find_path(t,1,NULL,{})", ml), |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                (a.tree_add_node)(t, 1, 0, d.as_ptr());
                let r = (a.tree_find_path)(t, 1, ptr::null_mut(), ml);
                (a.tree_delete)(t);
                r
            });
            diff(row, p, &format!("find_path(t,missing,..,{})", ml), |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                (a.tree_add_node)(t, 1, 0, d.as_ptr());
                let mut buf = [0xEEu64; 8];
                let r = (a.tree_find_path)(t, 777, buf.as_mut_ptr(), ml);
                (a.tree_delete)(t);
                (r, buf)
            });
        }
    });

    h.row("E45", |row| unsafe {
        // truncation: the return value follows `max_length`, even when negative
        for depth in [1usize, 2, 5, 40] {
            for &ml in [
                i32::MIN,
                -7,
                -1,
                0,
                1,
                2,
                depth as i32 - 1,
                depth as i32,
                depth as i32 + 1,
                i32::MAX,
            ]
            .iter()
            {
                diff(row, p, &format!("depth={} max_len={}", depth, ml), |a| {
                    let d = cstring(b"n");
                    let t = (a.tree_create)();
                    for i in 0..depth as u64 {
                        let parent = if i == 0 { 0 } else { i };
                        (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                    }
                    let mut buf = vec![0xEEu64; depth + 4];
                    let r = (a.tree_find_path)(t, depth as u64, buf.as_mut_ptr(), ml);
                    (a.tree_delete)(t);
                    (r, buf)
                });
            }
        }
    });
}

fn e47(p: &Pair, h: &mut Harness) {
    h.row("E47", |row| unsafe {
        // `while (length < 1000)`: a chain deeper than 1000 makes the loop stop
        // *before* it reaches the root, so the returned path is capped at 1000
        // entries and does not start at the root.
        for depth in [1000usize, 1001, 1005] {
            diff(row, p, &format!("cap1000 depth={}", depth), |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                for i in 0..depth as u64 {
                    let parent = if i == 0 { 0 } else { i };
                    (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                }
                let mut buf = vec![0xEEu64; 1100];
                let n = (a.tree_find_path)(t, depth as u64, buf.as_mut_ptr(), 1100);
                let first = buf[0];
                let last = buf[999];
                let untouched = buf[1000];
                (a.tree_delete)(t);
                (n, first, last, untouched)
            });
        }
    });
}

/// E48/E49 — the `size_t` counters of the C code are decremented without any
/// guard, so a hand-built `hashmap_t`/`tree_t` (both structs are public) makes
/// them wrap to `SIZE_MAX`.  The translation must wrap identically instead of
/// panicking.
fn e48_e49(p: &Pair, h: &mut Harness) {
    h.row("E48", |row| unsafe {
        // (a) `hashmap_put` into a tombstone with deleted_count == 0
        let mut res = Vec::new();
        for a in [&p.c, &p.r] {
            let (v, o, e) = capture_ret(|| {
                let mut entries = vec![HashmapEntry {
                    key: 7,
                    value: ptr::null_mut(),
                    occupied: 1,
                    deleted: 1,
                }];
                let mut map = Hashmap {
                    entries: entries.as_mut_ptr(),
                    capacity: 1,
                    size: 0,
                    deleted_count: 0,
                };
                let mp = &mut map as *mut Hashmap;
                let rc = (a.hashmap_put)(mp, 42, token(1));
                let out = (rc, map.size, map.deleted_count, snap_map_raw(mp));
                drop(entries);
                out
            });
            res.push((v, o, e));
        }
        row.eq("put-into-tombstone", &res[0].0, &res[1].0);
        row.eq_bytes("stdout", &res[0].1, &res[1].1);
        row.eq_bytes("stderr", &res[0].2, &res[1].2);
        row.ok(
            &format!("deleted_count wrapped to SIZE_MAX (got {})", res[0].0 .2),
            res[0].0 .2 == usize::MAX,
        );

        // (b) `hashmap_remove` of a live entry with size == 0
        let mut res = Vec::new();
        for a in [&p.c, &p.r] {
            let (v, o, e) = capture_ret(|| {
                let mut entries = vec![HashmapEntry {
                    key: 42,
                    value: token(3),
                    occupied: 1,
                    deleted: 0,
                }];
                let mut map = Hashmap {
                    entries: entries.as_mut_ptr(),
                    capacity: 1,
                    size: 0,
                    deleted_count: 0,
                };
                let mp = &mut map as *mut Hashmap;
                let rm = (a.hashmap_remove)(mp, 42) as u64;
                let out = (
                    rm,
                    map.size,
                    map.deleted_count,
                    (a.hashmap_size)(mp),
                    snap_map_raw(mp),
                );
                drop(entries);
                out
            });
            res.push((v, o, e));
        }
        row.eq("remove-with-size-0", &res[0].0, &res[1].0);
        row.eq_bytes("stdout(b)", &res[0].1, &res[1].1);
        row.eq_bytes("stderr(b)", &res[0].2, &res[1].2);
        row.ok(
            &format!("size wrapped to SIZE_MAX (got {})", res[0].0 .1),
            res[0].0 .1 == usize::MAX,
        );
    });

    h.row("E49", |row| unsafe {
        // `tree_remove_subtree` decrements `node_count` without a guard
        diff(row, p, "node_count underflow", |a| {
            let d = cstring(b"n");
            let t = (a.tree_create)();
            (a.tree_add_node)(t, 1, 0, d.as_ptr());
            (*t).node_count = 0;
            let rc = (a.tree_remove_node)(t, 1);
            let out = (rc, (*t).node_count, (a.tree_size)(t), (*t).has_root, (*t).root_id);
            (*t).node_count = 0; // keep tree_delete tidy
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (out, snap)
        });
    });
}

// ---------------------------------------------------------------------------
// E46 — the assertions of main.c
// ---------------------------------------------------------------------------

fn e46(p: &Pair, h: &mut Harness) {
    h.row("E46", |row| unsafe {
        for name in TEST_FUNCS.iter() {
            let fc = p.c.test_fn(name);
            let fr = p.r.test_fn(name);
            let cc = capture_fork(|| fc());
            let cr = capture_fork(|| fr());
            row.eq(&format!("{} exit", name), cc.exit, cr.exit);
            row.eq(&format!("{} signal (SIGABRT?)", name), cc.signal, cr.signal);
            row.eq_bytes(&format!("{} stderr", name), &cc.err, &cr.err);
            row.eq_bytes(&format!("{} stdout", name), &cc.out, &cr.out);
        }
    });
}

// ---------------------------------------------------------------------------
// B1..B6 — generic ABI boundaries
// ---------------------------------------------------------------------------

fn boundary_rows(p: &Pair, h: &mut Harness) {
    h.row("B1", |row| unsafe {
        // NULL for every pointer parameter of all 20 entry points
        diff(row, p, "all-null", |a| {
            let n = ptr::null_mut();
            let mut log: Vec<i64> = Vec::new();
            log.push((a.hashmap_put)(n, 1, ptr::null_mut()) as i64);
            log.push((a.hashmap_get)(n, 1) as i64);
            log.push((a.hashmap_remove)(n, 1) as i64);
            log.push((a.hashmap_contains)(n, 1) as i64);
            log.push((a.hashmap_size)(n) as i64);
            (a.hashmap_clear)(n);
            (a.hashmap_destroy)(n);
            let tn: *mut Tree = ptr::null_mut();
            (a.tree_delete)(tn);
            log.push((a.tree_add_node)(tn, 1, 0, ptr::null()) as i64);
            log.push((a.tree_remove_node)(tn, 1) as i64);
            log.push((a.tree_get_node)(tn, 1) as i64);
            log.push((a.tree_contains)(tn, 1) as i64);
            log.push((a.tree_size)(tn) as i64);
            (a.tree_print)(tn);
            log.push((a.tree_get_depth)(tn, 1) as i64);
            log.push((a.tree_get_height)(tn, 1) as i64);
            log.push((a.tree_count_descendants)(tn, 1) as i64);
            log.push((a.tree_find_path)(tn, 1, ptr::null_mut(), 4) as i64);
            log
        });
        // valid tree, NULL data / NULL path
        diff(row, p, "null-data-and-path", |a| {
            let t = (a.tree_create)();
            let r1 = (a.tree_add_node)(t, 1, 0, ptr::null());
            let r2 = (a.tree_add_node)(t, 2, 1, ptr::null());
            let n = (a.tree_get_node)(t, 1);
            let data0 = (*n).data[0];
            let r3 = (a.tree_find_path)(t, 1, ptr::null_mut(), 8);
            let snap = snap_tree(t);
            (a.tree_delete)(t);
            (r1, r2, data0, r3, snap)
        });
    });

    h.row("B2", |row| unsafe {
        let extremes: [u64; 8] = [
            0,
            1,
            2,
            u64::MAX,
            u64::MAX - 1,
            1 << 63,
            (1 << 63) - 1,
            0x0101_0101_0101_0101,
        ];
        for &id in extremes.iter() {
            for &parent in extremes.iter() {
                diff(row, p, &format!("id={} parent={}", id, parent), |a| {
                    let d = cstring(b"n");
                    let t = (a.tree_create)();
                    let r1 = (a.tree_add_node)(t, id, parent, d.as_ptr());
                    let r2 = (a.tree_add_node)(t, id ^ 0xAAAA, parent, d.as_ptr());
                    let r3 = (a.tree_add_node)(t, id ^ 0xAAAA, id, d.as_ptr());
                    let q = (
                        (a.tree_contains)(t, id),
                        (a.tree_get_depth)(t, id),
                        (a.tree_get_height)(t, id),
                        (a.tree_count_descendants)(t, id),
                        (a.tree_size)(t),
                    );
                    let snap = snap_tree(t);
                    (a.tree_delete)(t);
                    (r1, r2, r3, q, snap)
                });
            }
        }
    });

    h.row("B3", |row| unsafe {
        for &ml in [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX].iter() {
            diff(row, p, &format!("max_length={}", ml), |a| {
                let d = cstring(b"n");
                let t = (a.tree_create)();
                for i in 0..5u64 {
                    let parent = if i == 0 { 0 } else { i };
                    (a.tree_add_node)(t, i + 1, parent, d.as_ptr());
                }
                let mut buf = [0xEEu64; 16];
                let r = (a.tree_find_path)(t, 5, buf.as_mut_ptr(), ml);
                (a.tree_delete)(t);
                (r, buf)
            });
        }
    });

    h.row("B4+B5+B6", |row| unsafe {
        let mut cases: Vec<(String, Option<Vec<u8>>)> = vec![
            ("NULL".into(), None),
            ("empty".into(), Some(cstring(b""))),
        ];
        for n in [1usize, 2, 100, 254, 255, 256, 257, 300, 1024, 4096] {
            cases.push((format!("len{}", n), Some(cstring(&vec![b'q'; n]))));
        }
        cases.push(("all-0xFF-300".into(), Some(cstring(&vec![0xFFu8; 300]))));
        cases.push((
            "bytes-1..255".into(),
            Some(cstring(&(1u8..=255).collect::<Vec<u8>>())),
        ));
        let mut rng = Rng::new(SEED ^ 4);
        for i in 0..10 {
            let len = 1 + rng.usize_below(600);
            let v: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u64() % 255) as u8).collect();
            cases.push((format!("random{}", i), Some(cstring(&v))));
        }
        for (name, d) in cases.iter() {
            diff(row, p, name, |a| {
                let t = (a.tree_create)();
                let dp = match d {
                    None => ptr::null(),
                    Some(v) => v.as_ptr(),
                };
                let r1 = (a.tree_add_node)(t, 1, 0, dp);
                let r2 = (a.tree_add_node)(t, 2, 1, dp);
                let n = (a.tree_get_node)(t, 1);
                let s = cstr_bytes(&(*n).data);
                let full = if d.is_some() {
                    Some((*n).data.to_vec())
                } else {
                    None
                };
                (a.tree_print)(t);
                let snap = snap_tree(t);
                (a.tree_delete)(t);
                (r1, r2, s, full, snap)
            });
        }
    });
}

fn main() {
    if let Ok(case) = std::env::var("CDIFF_CHILD_CASE") {
        child_main(case);
        return;
    }
    let p = load_pair();
    let mut h = Harness::new("Phase C - error-path differential tests");
    hashmap_rows(&p, &mut h);
    tree_rows(&p, &mut h);
    e47(&p, &mut h);
    e48_e49(&p, &mut h);
    alloc_failure_rows(&mut h);
    e46(&p, &mut h);
    boundary_rows(&p, &mut h);
    h.finish();
}
