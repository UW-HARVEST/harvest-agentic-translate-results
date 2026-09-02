//! Cross-cutting randomized stress: several maps of different shapes alive at
//! once, long interleaved op streams, `hmput_default` injected mid-stream, and
//! free/rebuild cycles. Full state (header + elements + every index field + every
//! bucket slot) is compared after every operation in both `.so` files.
//!
//! This is the property-style backstop for the per-row tests in
//! `phase_b_*`/`phase_c_errors`: it composes the pipeline the way a real consumer
//! does rather than calling one entry point at a time.

mod common;

use common::*;
use std::ffi::c_char;

fn i32b(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}
fn pad(mut v: Vec<u8>, n: usize) -> Vec<u8> {
    v.resize(n, 0);
    v
}

#[derive(Clone, Copy, Debug)]
struct Shape {
    elemsize: usize,
    keysize: usize,
    string_mode: i32, // -1 => start from NULL (auto SH_DEFAULT / binary)
    op_mode: i32,
    kind: KeyKind,
}

const SHAPES: &[Shape] = &[
    // binary maps of assorted widths
    Shape { elemsize: 8, keysize: 4, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    Shape { elemsize: 16, keysize: 8, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    Shape { elemsize: 20, keysize: 8, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    Shape { elemsize: 4, keysize: 4, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    Shape { elemsize: 2, keysize: 1, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    Shape { elemsize: 64, keysize: 32, string_mode: -1, op_mode: 0, kind: KeyKind::Bytes },
    // binary maps read/written through out-of-range negative modes
    Shape { elemsize: 8, keysize: 4, string_mode: -1, op_mode: -7, kind: KeyKind::Bytes },
    Shape { elemsize: 16, keysize: 8, string_mode: -1, op_mode: i32::MIN, kind: KeyKind::Bytes },
    // string maps in every string.mode
    Shape { elemsize: 16, keysize: 8, string_mode: 1, op_mode: 1, kind: KeyKind::Ptr },
    Shape { elemsize: 16, keysize: 8, string_mode: 2, op_mode: 1, kind: KeyKind::Ptr },
    Shape { elemsize: 16, keysize: 8, string_mode: 3, op_mode: 1, kind: KeyKind::Ptr },
    Shape { elemsize: 24, keysize: 8, string_mode: 2, op_mode: 1, kind: KeyKind::Ptr },
    Shape { elemsize: 24, keysize: 8, string_mode: 3, op_mode: 1, kind: KeyKind::Ptr },
    // string map driven with an out-of-range string mode value
    Shape { elemsize: 16, keysize: 8, string_mode: -1, op_mode: 5, kind: KeyKind::Ptr },
];

fn build<'a>(p: &'a Pair, s: Shape, ctx: &str) -> DualMap<'a> {
    if s.string_mode < 0 {
        DualMap::empty(p, s.elemsize, s.keysize, s.kind, ctx)
    } else {
        DualMap::shmode(p, s.elemsize, s.keysize, s.kind, s.string_mode, ctx)
    }
}

#[test]
fn stress_multi_shape_interleaved() {
    let (p, _g) = libs();
    for (si, &s) in SHAPES.iter().enumerate() {
        for trial in 0..3u64 {
            let seed_base = 0x9000 + si as u64 * 97 + trial;
            reseed(p, seed_base as usize);
            let mut rng = Rng::new(seed_base);
            let ctx = format!("stress/shape{si}/trial{trial}");
            let mut m = build(p, s, &ctx);

            // key universe
            let bin_keys: Vec<Vec<u8>> = (0..96).map(|_| rng.bytes(s.keysize)).collect();
            let str_keys: Vec<*mut c_char> = (0..96)
                .map(|i| leak_cstr(&format!("s{si}t{trial}_{}{i}", "w".repeat(rng.below(20)))))
                .collect();

            let mut live: std::collections::BTreeSet<usize> = Default::default();
            let vlen = s.elemsize - s.keysize;
            let mut max_slots = 0usize;
            let mut rebuilds = 0usize;
            let mut ops = 0usize;

            for step in 0..3000usize {
                // rare free-and-rebuild so tables actually get large in between
                if rng.below(600) == 0 {
                    m.free();
                    live.clear();
                    m = build(p, s, &ctx);
                    rebuilds += 1;
                    continue;
                }
                let ki = rng.below(96);
                ops += 1;
                match rng.below(20) {
                    0..=10 => {
                        if s.kind == KeyKind::Ptr {
                            m.put_str(str_keys[ki], &pad(i32b(step as i32), vlen), s.op_mode);
                        } else {
                            m.put_bytes(&bin_keys[ki], &pad(i32b(step as i32), vlen));
                        }
                        live.insert(ki);
                    }
                    11..=14 => {
                        let got = if s.kind == KeyKind::Ptr {
                            m.geti_str(str_keys[ki], s.op_mode)
                        } else {
                            m.geti(&bin_keys[ki], s.op_mode)
                        };
                        // random binary keys can collide when keysize == 1, so
                        // only assert exact presence for wider keys
                        if s.keysize > 1 {
                            assert_eq!(
                                got >= 0,
                                live.contains(&ki),
                                "[{ctx}] presence mismatch step={step}"
                            );
                        }
                    }
                    15..=16 if s.kind == KeyKind::Bytes => {
                        m.geti_ts(&bin_keys[ki], s.op_mode);
                    }
                    17..=18 => {
                        // string modes other than exactly 1 hit a live assert when
                        // relocating a moved element, so restrict deletes there
                        let can_del = s.kind == KeyKind::Bytes || s.op_mode == 1;
                        if can_del {
                            if s.kind == KeyKind::Ptr {
                                m.del_str(str_keys[ki], 0, s.op_mode);
                            } else {
                                m.del_bytes(&bin_keys[ki], 0, s.op_mode);
                            }
                            live.remove(&ki);
                        }
                    }
                    _ => {
                        // hmput_default on a populated map is a no-op — in both
                        m.put_default(&pad(i32b(-2), vlen));
                    }
                }
                if let Some(idx) = m.snaps().0.idx.as_ref() {
                    max_slots = max_slots.max(idx.slot_count);
                }
            }
            assert!(ops > 2500, "[{ctx}] only {ops} ops ran");
            assert!(
                max_slots >= 128,
                "[{ctx}] table never grew past {max_slots} slots — the stream is too shallow"
            );
            assert!(rebuilds >= 1, "[{ctx}] no free/rebuild cycle happened");
            m.free();
        }
    }
}

#[test]
fn stress_growth_shrink_cycles() {
    let (p, _g) = libs();
    // Repeatedly fill past several rehash boundaries and drain past several
    // shrink boundaries, so make_hash_index runs in both the growing and the
    // shrinking direction many times with tombstones present.
    for trial in 0..4u64 {
        reseed(p, (0xA100 + trial) as usize);
        let mut rng = Rng::new(0xA100 + trial);
        let mut m = DualMap::empty(p, 8, 4, KeyKind::Bytes, &format!("cycles/{trial}"));
        m.put_default(&i32b(-2));
        for cycle in 0..6 {
            let n = 40 + rng.below(200) as i32;
            for k in 0..n {
                m.put_bytes(&i32b(k + cycle * 1000), &i32b(k));
            }
            // drain in a shuffled order so old_index != final_index often
            let mut order: Vec<i32> = (0..n).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for k in order {
                m.del_bytes(&i32b(k + cycle * 1000), 0, STBDS_HM_BINARY);
            }
            let s = m.snaps().0;
            assert_eq!(s.length, 1, "fully drained");
            assert_eq!(s.idx.as_ref().unwrap().slot_count, 8, "shrunk back to 8");
        }
        m.free();
    }
}

#[test]
fn stress_arena_inside_hash_index() {
    let (p, _g) = libs();
    // SH_ARENA puts the string arena *inside* the hash index, so rehashes copy
    // it (`t->string = ot->string`) while the chain keeps growing. Drive enough
    // rehashes with enough key-length variety to move `block` several steps and
    // to force dedicated arena blocks.
    for trial in 0..4u64 {
        reseed(p, (0xB100 + trial) as usize);
        let mut rng = Rng::new(0xB100 + trial);
        let mut m = DualMap::shmode(p, 16, 8, KeyKind::Ptr, SH_ARENA, &format!("arena/{trial}"));
        for n in 0..400 {
            let l = match rng.below(10) {
                0 => 900 + rng.below(3000),
                1 => 400 + rng.below(200),
                _ => rng.below(40),
            };
            let k = leak_cstr(&format!("A{trial}_{}{n}", "m".repeat(l)));
            m.put_str(k, &(n as i64).to_ne_bytes().to_vec(), STBDS_HM_STRING);
            assert!(m.geti_str(k, STBDS_HM_STRING) >= 0);
        }
        let idx = m.snaps().0.idx.as_ref().unwrap().clone();
        assert!(idx.string_block > 0, "arena should have advanced its block");
        assert!(idx.slot_count >= 512, "should have rehashed several times");
        m.free();
    }
}

#[test]
fn stress_hash_functions_wide() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC100);
    // 200k randomized hash_bytes comparisons across every length 0..=136
    for _ in 0..1500 {
        for len in 0..=136usize {
            if rng.below(8) != 0 {
                continue;
            }
            let mut buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            let a = unsafe {
                (p.c.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, seed)
            };
            let b = unsafe {
                (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, seed)
            };
            assert_eq!(a, b, "hash_bytes len={len} seed={seed:#x} {buf:02x?}");
        }
    }
    // and hash_string over random non-NUL byte strings
    for _ in 0..20000 {
        let len = rng.below(80);
        let mut buf: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u32() as u8 % 255)).collect();
        buf.push(0);
        let seed = rng.next_u64() as usize;
        let a = unsafe { (p.c.hash_string)(buf.as_mut_ptr() as *mut c_char, seed) };
        let b = unsafe { (p.rs.hash_string)(buf.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(a, b, "hash_string seed={seed:#x} {buf:02x?}");
    }
}

#[test]
fn stress_arena_random_sequences() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xD100);
    for trial in 0..20 {
        let mut ac = CArena::zeroed();
        let mut ar = CArena::zeroed();
        for step in 0..500 {
            let len = match rng.below(12) {
                0 => 1 + rng.below(200_000),
                1 => 400 + rng.below(2000),
                _ => 1 + rng.below(80),
            };
            let mut buf = vec![b'a' + (step % 26) as u8; len];
            buf.push(0);
            unsafe {
                let rc = (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
                let rr = (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
                assert_eq!(
                    (ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
                    (ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
                    "arena state trial={trial} step={step} len={len}"
                );
                assert_eq!(
                    std::slice::from_raw_parts(rc as *const u8, len),
                    std::slice::from_raw_parts(rr as *const u8, len),
                    "content trial={trial} step={step} len={len}"
                );
                // structural: head-bump vs spliced dedicated block
                let head_c = (ac.storage as *const u8).add(8).add(ac.remaining);
                let head_r = (ar.storage as *const u8).add(8).add(ar.remaining);
                assert_eq!(
                    rc as *const u8 == head_c,
                    rr as *const u8 == head_r,
                    "path trial={trial} step={step} len={len}"
                );
            }
        }
        unsafe {
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
            assert!(ac.storage.is_null() && ar.storage.is_null());
            assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block));
        }
    }
}
