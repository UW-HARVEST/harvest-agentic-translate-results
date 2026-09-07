// Phase B (part 2) — CONFIGS.md rows 24-43: string/arena modes, out-of-range
// enum values, the string arena, `strkey`, `hm_geti`, and mixed pipelines.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const DEFAULT_SEED: usize = 0x31415926;
const ES: usize = 16; // struct { char *key; size_t value; }
const KS: usize = 8;

/// NUL-terminated key buffers, always at least `KS` bytes of payload so the
/// `default: memcpy(.., keysize)` arm of `stbds_hmput_key` never reads OOB.
fn keybufs(n: usize, seed: u64, minlen: usize) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while out.len() < n {
        let len = minlen + rng.below(24);
        let body = rng.ascii(len);
        if seen.insert(body.clone()) {
            let mut v = cstring(&body);
            v.resize(v.len().max(KS + 1), 0);
            out.push(v);
        }
    }
    out
}

// ===========================================================================
// Row 24 — shmode_func(SH_NONE) + hmput_key(mode=1): default: memcpy arm
// ===========================================================================
//
// With `string.mode == SH_NONE` the element's key field holds the first
// `keysize` BYTES OF THE STRING, not a pointer.  Any later `strcmp` through
// that field would deref garbage, so this row does distinct inserts only (the
// C code only calls `stbds_is_key_equal` on a full 64-bit hash collision).

fn shmode_default_arm(row: &str, mode: c_int, expect_string_mode: u8) {
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let keys = keybufs(5, 0xB0024u64.wrapping_add(mode as i64 as u64), 9);
    let mut mc = Map::new(&p.c, ES, KS);
    let mut mr = Map::new(&p.r, ES, KS);
    mc.shmode(mode);
    mr.shmode(mode);
    diff(row, &format!("snapshot after shmode({mode})"), mc.snap(), mr.snap());
    diff(
        row,
        &format!("string.mode truncation for {mode}"),
        mc.snap().string_mode,
        expect_string_mode,
    );
    for (i, k) in keys.iter().enumerate() {
        let mut kc = k.clone();
        let mut kr = k.clone();
        let v = (i as u64 * 7).to_ne_bytes();
        let tc = mc.sput(kc.as_mut_ptr() as *mut c_char, &v, STBDS_HM_STRING);
        let tr = mr.sput(kr.as_mut_ptr() as *mut c_char, &v, STBDS_HM_STRING);
        diff(row, &format!("sput#{i} temp mode={mode}"), tc, tr);
        diff(row, &format!("sput#{i} snap mode={mode}"), mc.snap(), mr.snap());
        // element bytes are pure string content here -> byte-identical
        diff(
            row,
            &format!("sput#{i} elems mode={mode}"),
            mc.elem_bytes(),
            mr.elem_bytes(),
        );
    }
    mc.free();
    mr.free();
}

#[test]
fn row24_shmode_sh_none() {
    let _g = guard();
    shmode_default_arm("24", SH_NONE, 0);
}

// ===========================================================================
// Rows 25 / 29 / 30 — SH_DEFAULT: the caller's `char*` is stored verbatim
// ===========================================================================

fn string_map_cycle(row: &str, shmode: Option<c_int>, mode: c_int, n: usize, do_del: bool) {
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let mut keys = keybufs(n, 0xB0025u64.wrapping_add(n as u64 * 13).wrapping_add(mode as i64 as u64), 1);
    // include the empty string and a long string
    if n > 2 {
        keys[0] = cstring(b"");
        keys[0].resize(KS + 1, 0);
        keys[1] = cstring(&vec![b'z'; 200]);
    }
    let mut mc = Map::new(&p.c, ES, KS);
    let mut mr = Map::new(&p.r, ES, KS);
    if let Some(sm) = shmode {
        mc.shmode(sm);
        mr.shmode(sm);
        diff(row, "snap after shmode", mc.snap(), mr.snap());
    }
    // inserts (the SAME buffer address is handed to both libraries, so for
    // SH_DEFAULT the stored pointers are literally equal)
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let v = (i as u64 * 3 + 1).to_ne_bytes();
        let tc = mc.sput(kp, &v, mode);
        let tr = mr.sput(kp, &v, mode);
        diff(row, &format!("sput#{i}"), tc, tr);
        diff(row, &format!("sput#{i} snap"), mc.snap(), mr.snap());
        diff(
            row,
            &format!("sput#{i} elems(masked)"),
            mc.elem_bytes_masked(KS),
            mr.elem_bytes_masked(KS),
        );
        unsafe {
            for j in 1..=(mc.snap().length - 1) {
                diff(
                    row,
                    &format!("stored key string #{j} after sput#{i}"),
                    key_string_at(mc.t, ES, j),
                    key_string_at(mr.t, ES, j),
                );
            }
            diff(
                row,
                &format!("temp_key after sput#{i}"),
                temp_key_string(mc.t, ES),
                temp_key_string(mr.t, ES),
            );
        }
    }
    // duplicate re-put (update path + temp_key write in the first probe scan)
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let v = (i as u64 * 101).to_ne_bytes();
        let tc = mc.sput(kp, &v, mode);
        let tr = mr.sput(kp, &v, mode);
        diff(row, &format!("re-sput#{i}"), tc, tr);
        diff(row, &format!("re-sput#{i} snap"), mc.snap(), mr.snap());
        unsafe {
            diff(
                row,
                &format!("temp_key after re-sput#{i}"),
                temp_key_string(mc.t, ES),
                temp_key_string(mr.t, ES),
            );
        }
    }
    // lookups: hits
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let tc = mc.sgeti(kp, mode);
        let tr = mr.sgeti(kp, mode);
        diff(row, &format!("sgeti hit#{i}"), tc, tr);
        assert!(tc >= 0, "{row}: C lost string key #{i}");
        diff(row, &format!("sgeti value#{i}"), mc.value_at(tc), mr.value_at(tr));
    }
    // lookups: misses
    for (i, miss) in keybufs(10, 0xF0F0 + n as u64, 1).iter_mut().enumerate() {
        let kp = miss.as_mut_ptr() as *mut c_char;
        diff(row, &format!("sgeti miss#{i}"), mc.sgeti(kp, mode), mr.sgeti(kp, mode));
    }
    if do_del {
        // delete in reverse insertion order (keeps `old_index == final_index`
        // for the out-of-range-mode variants, and is a normal case otherwise)
        for (i, k) in keys.iter_mut().enumerate().rev() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let dc = mc.sdel(kp, mode, 0);
            let dr = mr.sdel(kp, mode, 0);
            diff(row, &format!("sdel#{i}"), dc, dr);
            diff(row, &format!("sdel#{i} snap"), mc.snap(), mr.snap());
            diff(
                row,
                &format!("sdel#{i} elems(masked)"),
                mc.elem_bytes_masked(KS),
                mr.elem_bytes_masked(KS),
            );
            // re-delete must miss
            diff(row, &format!("re-sdel#{i}"), mc.sdel(kp, mode, 0), mr.sdel(kp, mode, 0));
        }
        // re-insert everything after the deletions
        for (i, k) in keys.iter_mut().enumerate() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let v = (i as u64).to_ne_bytes();
            diff(row, &format!("reinsert#{i}"), mc.sput(kp, &v, mode), mr.sput(kp, &v, mode));
            diff(row, &format!("reinsert#{i} snap"), mc.snap(), mr.snap());
        }
    }
    mc.free();
    mr.free();
}

#[test]
fn row25_shmode_sh_default() {
    let _g = guard();
    for &n in &[1usize, 2, 7, 8, 50] {
        string_map_cycle("25", Some(SH_DEFAULT), STBDS_HM_STRING, n, true);
    }
}

#[test]
fn row29_string_mode_without_shmode() {
    let _g = guard();
    for &n in &[1usize, 2, 7, 8, 50, 300] {
        string_map_cycle("29", None, STBDS_HM_STRING, n, true);
    }
}

#[test]
fn row30_string_duplicate_reput_temp_key() {
    let _g = guard();
    // covered inside string_map_cycle's re-put phase; run the shapes that
    // straddle a bucket boundary so the wrap scan is also hit
    for &n in &[6usize, 7, 8, 9, 120] {
        string_map_cycle("30", None, STBDS_HM_STRING, n, false);
    }
}

// ===========================================================================
// Rows 26 / 31 — SH_STRDUP
// ===========================================================================

#[test]
fn row26_shmode_sh_strdup() {
    let _g = guard();
    for &n in &[1usize, 2, 7, 8, 50] {
        string_map_cycle("26", Some(SH_STRDUP), STBDS_HM_STRING, n, true);
    }
}

#[test]
fn row31_strdup_put_get_del_reput_free() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let mut keys = keybufs(120, 0xB0031, 1);
    let mut mc = Map::new(&p.c, ES, KS);
    let mut mr = Map::new(&p.r, ES, KS);
    mc.shmode(SH_STRDUP);
    mr.shmode(SH_STRDUP);
    for round in 0..3 {
        for (i, k) in keys.iter_mut().enumerate() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let v = (i as u64 + round * 1000).to_ne_bytes();
            diff("31", &format!("sput#{i} r={round}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
            diff("31", &format!("snap sput#{i} r={round}"), mc.snap(), mr.snap());
            unsafe {
                let n = mc.snap().length;
                for j in 1..n {
                    diff(
                        "31",
                        &format!("strdup'd key #{j} (r={round},i={i})"),
                        key_string_at(mc.t, ES, j),
                        key_string_at(mr.t, ES, j),
                    );
                }
            }
        }
        for (i, k) in keys.iter_mut().enumerate() {
            if i % 3 != 0 {
                continue;
            }
            let kp = k.as_mut_ptr() as *mut c_char;
            diff(
                "31",
                &format!("sdel#{i} r={round}"),
                mc.sdel(kp, STBDS_HM_STRING, 0),
                mr.sdel(kp, STBDS_HM_STRING, 0),
            );
            diff("31", &format!("snap sdel#{i} r={round}"), mc.snap(), mr.snap());
            unsafe {
                let n = mc.snap().length;
                for j in 1..n {
                    diff(
                        "31",
                        &format!("key after sdel #{j} (r={round},i={i})"),
                        key_string_at(mc.t, ES, j),
                        key_string_at(mr.t, ES, j),
                    );
                }
            }
        }
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Rows 27 / 32 — SH_ARENA
// ===========================================================================

#[test]
fn row27_shmode_sh_arena() {
    let _g = guard();
    for &n in &[1usize, 2, 7, 8, 50] {
        string_map_cycle("27", Some(SH_ARENA), STBDS_HM_STRING, n, true);
    }
}

#[test]
fn row32_arena_multiple_blocks_and_huge_strings() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let mut rng = Rng::new(0xB0032);
    // a mixture of small keys (fill blocks) and huge keys (> blocksize)
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while keys.len() < 400 {
        let len = if keys.len() % 37 == 0 {
            600 + rng.below(3000)
        } else {
            1 + rng.below(60)
        };
        let body = rng.ascii(len);
        if seen.insert(body.clone()) {
            let mut v = cstring(&body);
            v.resize(v.len().max(KS + 1), 0);
            keys.push(v);
        }
    }
    let mut mc = Map::new(&p.c, ES, KS);
    let mut mr = Map::new(&p.r, ES, KS);
    mc.shmode(SH_ARENA);
    mr.shmode(SH_ARENA);
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let v = (i as u64).to_ne_bytes();
        diff("32", &format!("sput#{i}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
        let sc = mc.snap();
        diff("32", &format!("snap sput#{i}"), sc.clone(), mr.snap());
        unsafe {
            diff(
                "32",
                &format!("arena key #{i}"),
                key_string_at(mc.t, ES, sc.length - 1),
                key_string_at(mr.t, ES, sc.length - 1),
            );
        }
    }
    // every arena-copied key must still be readable and correct
    unsafe {
        let n = mc.snap().length;
        for j in 1..n {
            diff(
                "32",
                &format!("arena key survives #{j}"),
                key_string_at(mc.t, ES, j),
                key_string_at(mr.t, ES, j),
            );
        }
    }
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let gc = mc.sgeti(kp, STBDS_HM_STRING);
        let gr = mr.sgeti(kp, STBDS_HM_STRING);
        diff("32", &format!("arena sgeti#{i}"), gc, gr);
        assert!(gc >= 0, "32: C lost arena key #{i}");
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Row 28 — out-of-range shmode_func modes
// ===========================================================================

#[test]
fn row28_shmode_out_of_range() {
    let _g = guard();
    for &(m, expect) in &[
        (4i32, 4u8),
        (99, 99),
        (255, 255),
        (256, 0),
        (-1, 255),
        (1000, 232),
        (i32::MIN, 0),
        (i32::MAX, 255),
    ] {
        shmode_default_arm("28", m, expect);
    }
}

// ===========================================================================
// Row 33 — mode = 2 / 7 / 1000 (out-of-range, >= STBDS_HM_STRING)
// ===========================================================================

#[test]
fn row33_out_of_range_mode_string_side() {
    let _g = guard();
    for &m in &[2i32, 7, 1000, i32::MAX] {
        // no shmode -> first put sets string.mode = SH_DEFAULT (mode >= 1)
        string_map_cycle("33", None, m, 30, true);
        // explicit SH_DEFAULT arena mode too
        string_map_cycle("33", Some(SH_DEFAULT), m, 12, true);
    }
}

// ===========================================================================
// Row 34 — mode < 0 (out-of-range, binary side)
// ===========================================================================

#[test]
fn row34_out_of_range_mode_binary_side() {
    let _g = guard();
    let p = pair();
    for &m in &[-1i32, -1000, i32::MIN] {
        setup(&p, DEFAULT_SEED);
        let mut mc = Map::new(&p.c, 8, 4);
        let mut mr = Map::new(&p.r, 8, 4);
        for k in 0i32..80 {
            let v = (k * 11).to_ne_bytes();
            diff("34", &format!("put({k}) mode={m}"), mc.put(&mut k.to_ne_bytes(), &v, m), mr.put(&mut k.to_ne_bytes(), &v, m));
            diff("34", &format!("snap put({k}) mode={m}"), mc.snap(), mr.snap());
        }
        diff("34", &format!("string.mode==0 for mode={m}"), mc.snap().string_mode, 0u8);
        for k in -10i32..90 {
            diff("34", &format!("geti({k}) mode={m}"), mc.geti(&mut k.to_ne_bytes(), m), mr.geti(&mut k.to_ne_bytes(), m));
        }
        for k in 0i32..80 {
            diff("34", &format!("del({k}) mode={m}"), mc.del(&mut k.to_ne_bytes(), m, 0), mr.del(&mut k.to_ne_bytes(), m, 0));
            diff("34", &format!("snap del({k}) mode={m}"), mc.snap(), mr.snap());
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 35 — hmfree_func across every storage mode
// ===========================================================================

#[test]
fn row35_hmfree_all_modes() {
    let _g = guard();
    let p = pair();

    // (a) binary map
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, 8, 4);
    let mut mr = Map::new(&p.r, 8, 4);
    for k in 0i32..40 {
        mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
    }
    diff("35", "binary map before free", mc.snap(), mr.snap());
    mc.free();
    mr.free();

    // (b/c/d) string modes
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        setup(&p, DEFAULT_SEED);
        let mut keys = keybufs(60, 0xB0035u64.wrapping_add(sm as i64 as u64), 1);
        let mut mc = Map::new(&p.c, ES, KS);
        let mut mr = Map::new(&p.r, ES, KS);
        mc.shmode(sm);
        mr.shmode(sm);
        for (i, k) in keys.iter_mut().enumerate() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let v = (i as u64).to_ne_bytes();
            mc.sput(kp, &v, STBDS_HM_STRING);
            mr.sput(kp, &v, STBDS_HM_STRING);
        }
        diff("35", &format!("string map sm={sm} before free"), mc.snap(), mr.snap());
        mc.free();
        mr.free();
    }

    // (e) plain array with hash_table == NULL
    unsafe {
        for &elemsize in &[1usize, 8, 16] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            diff("35", "no-table array header", header(ac).capacity, header(ar).capacity);
            (p.c.hmfree_func)(ac, elemsize);
            (p.r.hmfree_func)(ar, elemsize);
        }
        // (f) NULL
        (p.c.hmfree_func)(std::ptr::null_mut(), 8);
        (p.r.hmfree_func)(std::ptr::null_mut(), 8);
    }
}

// ===========================================================================
// Rows 36-38 — stbds_stralloc / stbds_strreset
// ===========================================================================

fn arena_snapshot(a: &StringArena) -> (bool, usize, u8, u8) {
    (!a.storage.is_null(), a.remaining, a.block, a.mode)
}

#[test]
fn row36_stralloc_length_classes() {
    let _g = guard();
    let p = pair();
    for &len in &[
        0usize, 1, 5, 10, 100, 400, 510, 511, 512, 513, 600, 1000, 4096, 100_000,
    ] {
        let body: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let mut s = cstring(&body);
        unsafe {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("36", &format!("stralloc content len={len}"), cstr(pc), cstr(pr));
            diff("36", &format!("arena state len={len}"), arena_snapshot(&ac), arena_snapshot(&ar));
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            diff("36", &format!("arena after reset len={len}"), arena_snapshot(&ac), arena_snapshot(&ar));
        }
    }
}

#[test]
fn row37_stralloc_block_progression() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0037);
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut live: Vec<(*mut c_char, *mut c_char, Vec<u8>)> = Vec::new();
        for step in 0..2000 {
            let len = if step % 53 == 0 {
                1 + rng.below(200_000)
            } else {
                rng.below(300)
            };
            let body = rng.ascii(len);
            let mut s = cstring(&body);
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("37", &format!("content step={step} len={len}"), cstr(pc), cstr(pr));
            diff(
                "37",
                &format!("arena state step={step} len={len}"),
                arena_snapshot(&ac),
                arena_snapshot(&ar),
            );
            live.push((pc, pr, body));
            if live.len() > 40 {
                live.remove(0);
            }
            // all recent allocations must still read back identically
            if step % 25 == 0 {
                for (i, (a, b, expect)) in live.iter().enumerate() {
                    diff("37", &format!("still valid #{i} at step={step}"), cstr(*a), expect.clone());
                    diff("37", &format!("still valid #{i} at step={step}"), cstr(*b), expect.clone());
                }
            }
        }
        // the `block` counter must have saturated at the 1<<20 limit
        assert!(ac.block >= 20, "expected block saturation, got {}", ac.block);
        diff("37", "block saturation", ac.block, ar.block);
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        diff("37", "after reset", arena_snapshot(&ac), arena_snapshot(&ar));
    }
}

#[test]
fn row38_strreset_reuse_and_idempotence() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0038);
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        // reset an untouched arena
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        diff("38", "reset untouched", arena_snapshot(&ac), arena_snapshot(&ar));
        for cycle in 0..8 {
            for i in 0..200 {
                let bl = 1 + rng.below(700);
                let body = rng.ascii(bl);
                let mut s = cstring(&body);
                let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                diff("38", &format!("content c={cycle} i={i}"), cstr(pc), cstr(pr));
                diff("38", &format!("state c={cycle} i={i}"), arena_snapshot(&ac), arena_snapshot(&ar));
            }
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            diff("38", &format!("reset c={cycle}"), arena_snapshot(&ac), arena_snapshot(&ar));
            diff("38", "reset zeroes everything", arena_snapshot(&ac), (false, 0, 0, 0));
            // reset twice
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            diff("38", "double reset", arena_snapshot(&ac), arena_snapshot(&ar));
        }
        // a non-zero `mode` must also be cleared by strreset
        ac.mode = 3;
        ar.mode = 3;
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        diff("38", "mode cleared", arena_snapshot(&ac), arena_snapshot(&ar));
    }
}

// ===========================================================================
// Row 39 — strkey
// ===========================================================================

#[test]
fn row39_strkey() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0039);
    let mut cases: Vec<c_int> = vec![0, 1, -1, 7, 12345, -98765, i32::MAX, i32::MIN, 100000000];
    for _ in 0..500 {
        cases.push(rng.next_u32() as c_int);
    }
    unsafe {
        for n in cases {
            let a = cstr((p.c.strkey)(n));
            let b = cstr((p.r.strkey)(n));
            diff("39", &format!("strkey({n})"), a.clone(), b);
            diff("39", &format!("strkey({n}) text"), a, format!("test_{n}").into_bytes());
        }
    }
}

// ===========================================================================
// Rows 40 / 41 — the hm_geti driver
// ===========================================================================

/// Observable side effect of `hm_geti`: the number of `make_hash_index(..,NULL)`
/// calls it performs, visible through the advanced global seed.
fn probe_seed(p: &Pair) -> (usize, usize) {
    let mut mc = Map::new(&p.c, 8, 4);
    let mut mr = Map::new(&p.r, 8, 4);
    let mut k = 1i32.to_ne_bytes();
    mc.put(&mut k, &1i32.to_ne_bytes(), STBDS_HM_BINARY);
    mr.put(&mut k, &1i32.to_ne_bytes(), STBDS_HM_BINARY);
    let s = (mc.snap().seed, mr.snap().seed);
    mc.free();
    mr.free();
    s
}

#[test]
fn row40_hm_geti_sweep() {
    let _g = guard();
    let p = pair();
    let nums: Vec<c_int> = vec![
        -1000, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100,
        127, 128, 129, 255, 256, 257, 512, 1000, 2000, 5000,
    ];
    for n in nums {
        setup(&p, DEFAULT_SEED);
        unsafe {
            (p.c.hm_geti)(n);
            (p.r.hm_geti)(n);
        }
        let (sc, sr) = probe_seed(&p);
        diff("40", &format!("seed advance after hm_geti({n})"), sc, sr);
    }
    // i32::MIN / large negatives also just skip every loop
    for n in [i32::MIN, -123456] {
        setup(&p, DEFAULT_SEED);
        unsafe {
            (p.c.hm_geti)(n);
            (p.r.hm_geti)(n);
        }
        let (sc, sr) = probe_seed(&p);
        diff("40", &format!("seed advance after hm_geti({n})"), sc, sr);
    }
}

#[test]
fn row41_hm_geti_repeated_and_reseeded() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    for i in 0..20 {
        unsafe {
            (p.c.hm_geti)(37);
            (p.r.hm_geti)(37);
        }
        let (sc, sr) = probe_seed(&p);
        diff("41", &format!("repeat #{i}"), sc, sr);
    }
    for &s in &[0usize, 1, usize::MAX, 999] {
        setup(&p, s);
        for i in 0..5 {
            unsafe {
                (p.c.hm_geti)(64);
                (p.r.hm_geti)(64);
            }
            let (sc, sr) = probe_seed(&p);
            diff("41", &format!("reseeded {s:#x} #{i}"), sc, sr);
        }
    }
}

// ===========================================================================
// Rows 42 / 43 — mixed pipelines with a full snapshot after every operation
// ===========================================================================

#[test]
fn row42_mixed_strdup_pipeline() {
    let _g = guard();
    let p = pair();
    setup(&p, DEFAULT_SEED);
    let mut keys = keybufs(160, 0xB0042, 1);
    let mut mc = Map::new(&p.c, ES, KS);
    let mut mr = Map::new(&p.r, ES, KS);
    mc.shmode(SH_STRDUP);
    mr.shmode(SH_STRDUP);
    let mut op = 0usize;
    for (i, k) in keys.iter_mut().take(100).enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let v = (i as u64).to_ne_bytes();
        diff("42", &format!("op{op} put#{i}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
        diff("42", &format!("op{op} snap"), mc.snap(), mr.snap());
        op += 1;
    }
    for (i, k) in keys.iter_mut().take(40).enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        diff("42", &format!("op{op} del#{i}"), mc.sdel(kp, STBDS_HM_STRING, 0), mr.sdel(kp, STBDS_HM_STRING, 0));
        diff("42", &format!("op{op} snap"), mc.snap(), mr.snap());
        op += 1;
    }
    for (i, k) in keys.iter_mut().skip(100).enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let v = (i as u64 + 500).to_ne_bytes();
        diff("42", &format!("op{op} put2#{i}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
        diff("42", &format!("op{op} snap"), mc.snap(), mr.snap());
        op += 1;
    }
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr() as *mut c_char;
        let gc = mc.sgeti(kp, STBDS_HM_STRING);
        let gr = mr.sgeti(kp, STBDS_HM_STRING);
        diff("42", &format!("op{op} get#{i}"), gc, gr);
        if gc >= 0 {
            diff("42", &format!("op{op} value#{i}"), mc.value_at(gc), mr.value_at(gr));
            unsafe {
                diff(
                    "42",
                    &format!("op{op} key#{i}"),
                    key_string_at(mc.t, ES, gc as usize),
                    key_string_at(mr.t, ES, gr as usize),
                );
            }
        }
        op += 1;
    }
    mc.free();
    mr.free();
}

#[test]
fn row43_mixed_binary_pipeline() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xB0043);
    setup(&p, DEFAULT_SEED);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&(-2i32).to_ne_bytes());
    mr.put_default(&(-2i32).to_ne_bytes());
    let mut live: Vec<i32> = Vec::new();
    for op in 0..3000 {
        let choice = rng.below(100);
        if choice < 45 || live.is_empty() {
            let k = rng.next_u32() as i32;
            let v = (k ^ 0x5A5A5A5A).to_ne_bytes();
            diff("43", &format!("op{op} put({k})"), mc.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY), mr.put(&mut k.to_ne_bytes(), &v, STBDS_HM_BINARY));
            if !live.contains(&k) {
                live.push(k);
            }
        } else if choice < 75 {
            let i = rng.below(live.len());
            let k = live.swap_remove(i);
            diff("43", &format!("op{op} del({k})"), mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0), mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0));
        } else if choice < 95 {
            let k = if rng.below(2) == 0 {
                live[rng.below(live.len())]
            } else {
                rng.next_u32() as i32
            };
            let gc = mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            let gr = mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            diff("43", &format!("op{op} geti({k})"), gc, gr);
            diff("43", &format!("op{op} value({k})"), mc.value_at(gc), mr.value_at(gr));
            let tc = mc.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            let tr = mr.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            diff("43", &format!("op{op} geti_ts({k})"), tc, tr);
        } else {
            mc.put_default(&(op as i32).to_ne_bytes());
            mr.put_default(&(op as i32).to_ne_bytes());
        }
        diff("43", &format!("op{op} snapshot"), mc.snap(), mr.snap());
        diff("43", &format!("op{op} elems"), mc.elem_bytes(), mr.elem_bytes());
    }
    // final full verification
    for &k in &live {
        let gc = mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        let gr = mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        diff("43", &format!("final geti({k})"), gc, gr);
        assert!(gc >= 0, "43: C lost live key {k}");
    }
    mc.free();
    mr.free();
    let _: *mut c_void = std::ptr::null_mut();
}

// ===========================================================================
// Rows 25 / 27 (cont.) — forward deletion so the "move the final element and
// re-find its slot" path runs for SH_DEFAULT and SH_ARENA too.
// ===========================================================================

#[test]
fn row25_27_forward_delete_moves_final_element() {
    let _g = guard();
    let p = pair();
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[3usize, 8, 9, 40, 150] {
            setup(&p, DEFAULT_SEED);
            let mut keys = keybufs(n, 0xB2527u64.wrapping_add(sm as u64 * 7).wrapping_add(n as u64), 1);
            let mut mc = Map::new(&p.c, ES, KS);
            let mut mr = Map::new(&p.r, ES, KS);
            mc.shmode(sm);
            mr.shmode(sm);
            for (i, k) in keys.iter_mut().enumerate() {
                let kp = k.as_mut_ptr() as *mut c_char;
                let v = (i as u64).to_ne_bytes();
                diff("25/27", &format!("put#{i} sm={sm}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
            }
            // forward deletion => old_index != final_index for most steps
            let mut survivors = keys.clone();
            for i in 0..survivors.len() {
                let kp = survivors[i].as_mut_ptr() as *mut c_char;
                let dc = mc.sdel(kp, STBDS_HM_STRING, 0);
                let dr = mr.sdel(kp, STBDS_HM_STRING, 0);
                diff("25/27", &format!("fwd sdel#{i} sm={sm} n={n}"), dc, dr);
                diff("25/27", &format!("fwd sdel#{i} snap sm={sm} n={n}"), mc.snap(), mr.snap());
                diff(
                    "25/27",
                    &format!("fwd sdel#{i} elems(masked) sm={sm} n={n}"),
                    mc.elem_bytes_masked(KS),
                    mr.elem_bytes_masked(KS),
                );
                unsafe {
                    let len = mc.snap().length;
                    for j in 1..len {
                        diff(
                            "25/27",
                            &format!("key #{j} after fwd sdel#{i} sm={sm}"),
                            key_string_at(mc.t, ES, j),
                            key_string_at(mr.t, ES, j),
                        );
                    }
                }
                // survivors must all still resolve
                for j in (i + 1)..survivors.len() {
                    let sp = survivors[j].as_mut_ptr() as *mut c_char;
                    let gc = mc.sgeti(sp, STBDS_HM_STRING);
                    let gr = mr.sgeti(sp, STBDS_HM_STRING);
                    diff("25/27", &format!("survivor#{j} after fwd sdel#{i} sm={sm}"), gc, gr);
                    assert!(gc >= 0, "25/27: C lost survivor #{j} (sm={sm})");
                }
            }
            mc.free();
            mr.free();
        }
    }
}
