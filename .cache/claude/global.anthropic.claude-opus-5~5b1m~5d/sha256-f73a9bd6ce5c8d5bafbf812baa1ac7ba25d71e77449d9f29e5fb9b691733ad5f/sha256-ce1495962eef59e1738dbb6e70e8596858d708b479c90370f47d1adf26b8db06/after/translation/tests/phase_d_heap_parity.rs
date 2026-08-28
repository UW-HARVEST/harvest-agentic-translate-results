//! Phase D — heap-accounting parity.
//!
//! Some divergences are invisible in returned values: if `stbds_hmfree_func`
//! skipped `stbds_strreset` (the arena blocks) or the `STBDS_SH_STRDUP` key
//! sweep, or if `stbds_hmdel_key` freed a key where the C deliberately leaks
//! it, every call would still return identical bytes while the heap behaved
//! differently.
//!
//! Measurement: glibc's `mallinfo2().uordblks` counts tcache-held chunks as
//! still-in-use, so a single before/after pair is noisy by a few hundred bytes.
//! The *slope* over many identical iterations is not: a real leak grows
//! linearly.  Both libraries must show the same per-iteration slope — including
//! the configurations where the C leaks on purpose (`hmdel_key` only frees the
//! duplicated key when `mode == STBDS_HM_STRING` exactly, so out-of-enum string
//! modes leak).

mod common;

use common::*;
use std::ffi::{c_char, c_void};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

unsafe extern "C" {
    fn mallinfo2() -> MallInfo2;
}

fn used() -> usize {
    unsafe { mallinfo2().uordblks }
}

fn mkkey(m: usize, i: usize, keylen: usize) -> Box<[u8]> {
    let mut v = format!("{m}-{i}-").into_bytes();
    while v.len() < keylen {
        v.push(b'k');
    }
    v.push(0);
    v.into_boxed_slice()
}

/// create → fill → delete the newest `dels` → free
fn map_workload(api: &Api, sh: i32, mode: i32, keys: usize, dels: usize, keylen: usize) {
    let es = 16usize;
    unsafe {
        let mut owned: Vec<Box<[u8]>> = (0..keys).map(|i| mkkey(0, i, keylen)).collect();
        let mut delkeys: Vec<Box<[u8]>> =
            (0..dels).map(|i| mkkey(0, keys - 1 - i, keylen)).collect();
        let mut h = (api.shmode_func)(es, sh);
        for kb in owned.iter_mut() {
            let kp = kb.as_mut_ptr() as *mut c_void;
            h = (api.hmput_key)(h, es, kp, 8, mode);
            let idx = map_temp(h, es);
            let elem = map_raw(h, es).offset((idx + 1) * es as isize);
            std::ptr::write_bytes(elem.add(8), 0x5a, es - 8);
        }
        for kb in delkeys.iter_mut() {
            h = (api.hmdel_key)(h, es, kb.as_mut_ptr() as *mut c_void, 8, 0, mode);
        }
        (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
    }
}

fn arena_workload(api: &Api, allocs: usize, len: usize) {
    unsafe {
        let mut arena = StringArena::zeroed();
        let ap: *mut StringArena = &mut arena;
        for i in 0..allocs {
            let mut v = vec![b'a'; len + i % 11];
            v.push(0);
            (api.stralloc)(ap as *mut c_void, v.as_mut_ptr() as *mut c_char);
        }
        (api.strreset)(ap as *mut c_void);
    }
}

fn binary_workload(api: &Api, keys: usize, dels: usize) {
    let es = 8usize;
    unsafe {
        let mut h: *mut c_void = std::ptr::null_mut();
        for i in 0..keys as u32 {
            let mut key = i.to_le_bytes();
            h = (api.hmput_key)(h, es, key.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            let idx = map_temp(h, es);
            let elem = map_raw(h, es).offset((idx + 1) * es as isize);
            std::ptr::copy_nonoverlapping(key.as_ptr(), elem, 4);
            std::ptr::write_bytes(elem.add(4), 0x11, 4);
        }
        for i in 0..dels as u32 {
            let mut key = i.to_le_bytes();
            h = (api.hmdel_key)(h, es, key.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
        }
        (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
    }
}

const ITERS: usize = 400;

fn slope(api: &Api, w: &dyn Fn(&Api)) -> f64 {
    for _ in 0..20 {
        w(api); // warm the allocator and any lazy std buffers
    }
    let u0 = used();
    for _ in 0..ITERS {
        w(api);
    }
    (used() as f64 - u0 as f64) / ITERS as f64
}

#[test]
fn heap_accounting_parity() {
    let p = seeded(DEFAULT_SEED);
    let mut fails = Vec::new();
    let mut report = Vec::new();

    type W = (String, Box<dyn Fn(&Api)>);
    let mut workloads: Vec<W> = Vec::new();
    for &(sh, mode) in &[
        (SH_NONE, HM_BINARY),
        (SH_DEFAULT, HM_STRING),
        (SH_STRDUP, HM_STRING),
        (SH_ARENA, HM_STRING),
        (SH_STRDUP, HM_BINARY),
        (SH_ARENA, HM_BINARY),
        // out-of-enum string modes: hmdel_key's `mode == STBDS_HM_STRING`
        // guard is false, so the C leaks the duplicated keys on purpose
        (SH_STRDUP, 2),
        (SH_STRDUP, 7),
        (SH_ARENA, 2),
    ] {
        for &(keys, dels, klen) in &[
            (1usize, 0usize, 8usize),
            (10, 4, 8),
            (10, 4, 300),
            (40, 14, 300),
            (40, 40, 300),
        ] {
            workloads.push((
                format!("map sh={sh} mode={mode} keys={keys} dels={dels} klen={klen}"),
                Box::new(move |a: &Api| map_workload(a, sh, mode, keys, dels, klen)),
            ));
        }
    }
    for &(n, len) in &[(1usize, 8usize), (10, 8), (40, 300), (8, 3000), (60, 500)] {
        workloads.push((
            format!("arena allocs={n} len={len}"),
            Box::new(move |a: &Api| arena_workload(a, n, len)),
        ));
    }
    for &(keys, dels) in &[(1usize, 0usize), (20, 10), (60, 60), (200, 100)] {
        workloads.push((
            format!("binary keys={keys} dels={dels}"),
            Box::new(move |a: &Api| binary_workload(a, keys, dels)),
        ));
    }

    for (name, w) in &workloads {
        let dc = slope(p.c, w.as_ref());
        let dr = slope(p.r, w.as_ref());
        report.push(format!("{name}: C={dc:.2} B/iter RUST={dr:.2} B/iter"));
        // tcache accounting noise is a few hundred bytes over 400 iterations;
        // a real leak difference is orders of magnitude larger
        let tol = (0.20 * dc.max(dr)).max(128.0);
        if (dc - dr).abs() > tol {
            fails.push(format!("{name}: C={dc:.2} B/iter but RUST={dr:.2} B/iter"));
        }
    }
    for line in &report {
        println!("{line}");
    }
    assert!(fails.is_empty(), "heap accounting diverges:\n{}", fails.join("\n"));
}
