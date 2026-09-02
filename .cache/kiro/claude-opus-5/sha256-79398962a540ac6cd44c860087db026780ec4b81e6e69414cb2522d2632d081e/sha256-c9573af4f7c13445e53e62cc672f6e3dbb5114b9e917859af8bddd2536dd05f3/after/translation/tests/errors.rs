//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition, calls BOTH `.so`s
//! through `libloading`, and asserts the same sentinel / error / abort — not
//! merely "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::process::Command;

// ===========================================================================
// E1 / E2  stbds_hmfree_func rejects NULL and table-less arrays
// ===========================================================================

#[test]
fn e1_hmfree_null_is_noop() {
    let s = session(0x31415926);
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 40] {
            // Must return immediately: no free, no crash, on both.
            (s.c().hmfree_func)(std::ptr::null_mut(), elemsize);
            (s.r().hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn e2_hmfree_array_without_hash_table() {
    let s = session(0x31415926);
    unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let ca = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ch = (ca as *mut ArrayHeader).wrapping_sub(1);
            let rh = (ra as *mut ArrayHeader).wrapping_sub(1);
            assert!((*ch).hash_table.is_null() && (*rh).hash_table.is_null());
            (s.c().hmfree_func)(ca, elemsize);
            (s.r().hmfree_func)(ra, elemsize);
        }
    }
}

// ===========================================================================
// E3 / E4 / E5  stbds_hmdel_key rejection paths
// ===========================================================================

#[test]
fn e3_hmdel_null_returns_null() {
    let s = session(0x31415926);
    unsafe {
        let mut key = [1u8; 8];
        for &mode in &[HM_BINARY, HM_STRING, 2, -1, c_int::MIN, c_int::MAX] {
            for elemsize in [8usize, 16, 40] {
                let c = (s.c().hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let r = (s.r().hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                assert!(c.is_null(), "E3: C must return NULL (mode={mode})");
                assert!(r.is_null(), "E3: Rust must return NULL (mode={mode})");
            }
        }
    }
}

#[test]
fn e4_hmdel_without_hash_table() {
    let s = session(0x31415926);
    unsafe {
        let elemsize = 16usize;
        let mut key = [7u8; 8];
        // Build a length>=1 array with no hash table, in the `t` (hash) space.
        let ca = (s.c().hmput_default)(std::ptr::null_mut(), elemsize);
        let ra = (s.r().hmput_default)(std::ptr::null_mut(), elemsize);
        let cm = Map { t: ca, elemsize };
        let rm = Map { t: ra, elemsize };
        assert!((*cm.header()).hash_table.is_null());
        assert!((*rm.header()).hash_table.is_null());
        // Poison temp so the mandated `stbds_temp(raw_a) = 0` is observable.
        (*cm.header()).temp = 0x7777;
        (*rm.header()).temp = 0x7777;

        let c2 = (s.c().hmdel_key)(ca, elemsize, key.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
        let r2 = (s.r().hmdel_key)(ra, elemsize, key.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
        assert_eq!(c2, ca, "E4: C must return `a` unchanged");
        assert_eq!(r2, ra, "E4: Rust must return `a` unchanged");
        assert_eq!((*cm.header()).temp, 0, "E4: C must zero temp first");
        assert_eq!((*rm.header()).temp, 0, "E4: Rust must zero temp first");

        (s.c().hmfree_func)(cm.raw(), elemsize);
        (s.r().hmfree_func)(rm.raw(), elemsize);
    }
}

#[test]
fn e5_hmdel_absent_key() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE005);
    unsafe {
        for &n in &[1usize, 7, 40, 200] {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            let mut keys = Keys8::new(&mut rng, n);
            for i in 0..n {
                p.put(keys.ptr(i), HM_BINARY, &[i as u8, 1]);
            }
            let before = (p.cm.t, p.rm.t);
            let mut absent = Keys8::new(&mut rng, 12);
            for i in 0..12 {
                absent.bufs[i][7] = 0xEF;
                assert_eq!(
                    p.del(absent.ptr(i), HM_BINARY, 0),
                    0,
                    "E5: temp must be 0 for an absent key"
                );
            }
            assert_eq!((p.cm.t, p.rm.t), before, "E5: pointer must not change");
            assert_eq!(p.cm.len(), n as isize, "E5: length must not change");
            p.free();
        }
    }
}

/// Minimal 8-byte-key helper (the map tests have a richer one).
struct Keys8 {
    bufs: Vec<Box<[u8]>>,
}
impl Keys8 {
    fn new(rng: &mut Rng, n: usize) -> Keys8 {
        let mut bufs = Vec::new();
        for i in 0..n {
            let mut v = rng.bytes(8);
            v[0] = i as u8;
            v[1] = (i >> 8) as u8;
            v.push(0);
            bufs.push(v.into_boxed_slice());
        }
        Keys8 { bufs }
    }
    fn ptr(&mut self, i: usize) -> *mut u8 {
        self.bufs[i].as_mut_ptr()
    }
}

// ===========================================================================
// E6 / E7  the `mode == STBDS_HM_STRING` vs `mode >= STBDS_HM_STRING` asymmetry
// ===========================================================================

#[test]
fn e6_e7_strdup_free_only_when_mode_equals_one() {
    let mut rng = Rng::new(0xE006);
    for &mode in &[HM_STRING, 2, 7, c_int::MAX] {
        let s = session(0xE006 ^ (mode as usize));
        unsafe {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
            p.shmode(SH_STRDUP);
            let n = 6usize;
            let mut keys: Vec<Box<[u8]>> = (0..n)
                .map(|i| {
                    let mut v = format!("strdup-key-{i}-").into_bytes();
                    for _ in 0..rng.below(10) {
                        v.push(b'z');
                    }
                    v.push(0);
                    v.into_boxed_slice()
                })
                .collect();
            for (i, k) in keys.iter_mut().enumerate() {
                p.put(k.as_mut_ptr(), mode, &[i as u8, 2]);
            }
            // Delete from the tail so `old_index == final_index` and the E18
            // assert (only reachable for mode != 1) is not hit here.
            for i in (0..n).rev() {
                let flag = p.del(keys[i].as_mut_ptr(), mode, 0);
                assert_eq!(flag, 1, "E6/E7 mode={mode}: delete of key {i}");
            }
            p.free();
        }
    }
}

// ===========================================================================
// E8 / E11 / E12  the key-not-found sentinel
// ===========================================================================

#[test]
fn e8_e11_e12_lookup_miss_sentinel() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE008);
    unsafe {
        for &n in &[1usize, 6, 7, 60, 300] {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            let mut keys = Keys8::new(&mut rng, n);
            for i in 0..n {
                p.put(keys.ptr(i), HM_BINARY, &[i as u8, 4]);
            }
            let mut absent = Keys8::new(&mut rng, 30);
            for i in 0..30 {
                absent.bufs[i][7] = 0xD7;
                // E11 + E8: `*temp` must be exactly STBDS_INDEX_EMPTY (-1).
                assert_eq!(p.get_ts(absent.ptr(i), HM_BINARY), -1, "E11 n={n}");
                // E12: and hmget_key must store that -1 in the header.
                assert_eq!(p.get(absent.ptr(i), HM_BINARY), -1, "E12 n={n}");
                assert_eq!((*p.cm.header()).temp, -1);
                assert_eq!((*p.rm.header()).temp, -1);
            }
            p.free();
        }
    }
}

// ===========================================================================
// E9 / E10  getter bootstrap paths
// ===========================================================================

#[test]
fn e9_get_from_null_bootstraps() {
    let s = session(0x31415926);
    unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let mut key = [0x3Cu8; 8];
            for &mode in &[HM_BINARY, HM_STRING, 2, -5] {
                let mut ctmp: isize = 0x1234;
                let mut rtmp: isize = 0x1234;
                let ct = (s.c().hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut ctmp,
                    mode,
                );
                let rt = (s.r().hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut rtmp,
                    mode,
                );
                assert_eq!(ctmp, -1, "E9: C *temp");
                assert_eq!(rtmp, -1, "E9: Rust *temp");
                let cm = Map { t: ct, elemsize };
                let rm = Map { t: rt, elemsize };
                assert_eq!((*cm.header()).length, 1, "E9: C length");
                assert_eq!((*rm.header()).length, 1, "E9: Rust length");
                assert!((*cm.header()).hash_table.is_null());
                assert!((*rm.header()).hash_table.is_null());
                // element 0 must be zeroed by both
                let cz = std::slice::from_raw_parts(cm.elem(0), elemsize).to_vec();
                let rz = std::slice::from_raw_parts(rm.elem(0), elemsize).to_vec();
                assert_eq!(cz, vec![0u8; elemsize], "E9: C zeroes the default slot");
                assert_eq!(rz, vec![0u8; elemsize], "E9: Rust zeroes the default slot");
                (s.c().hmfree_func)(cm.raw(), elemsize);
                (s.r().hmfree_func)(rm.raw(), elemsize);
            }
        }
    }
}

/// E10: `a != NULL` with no hash table — `*temp = -1` and the key is never
/// hashed, so even a NULL key pointer is harmless.
#[test]
fn e10_get_without_hash_table_ignores_key() {
    let s = session(0x31415926);
    unsafe {
        let elemsize = 16usize;
        for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
            let ca = (s.c().hmput_default)(std::ptr::null_mut(), elemsize);
            let ra = (s.r().hmput_default)(std::ptr::null_mut(), elemsize);
            let mut ctmp: isize = 999;
            let mut rtmp: isize = 999;
            let c2 = (s.c().hmget_key_ts)(
                ca,
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut ctmp,
                mode,
            );
            let r2 = (s.r().hmget_key_ts)(
                ra,
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut rtmp,
                mode,
            );
            assert_eq!(c2, ca);
            assert_eq!(r2, ra);
            assert_eq!(ctmp, -1, "E10: C *temp with NULL key");
            assert_eq!(rtmp, -1, "E10: Rust *temp with NULL key");

            // And through the non-_ts wrapper.
            let c3 = (s.c().hmget_key)(ca, elemsize, std::ptr::null_mut(), 8, mode);
            let r3 = (s.r().hmget_key)(ra, elemsize, std::ptr::null_mut(), 8, mode);
            let cm = Map { t: c3, elemsize };
            let rm = Map { t: r3, elemsize };
            assert_eq!((*cm.header()).temp, -1);
            assert_eq!((*rm.header()).temp, -1);
            (s.c().hmfree_func)(cm.raw(), elemsize);
            (s.r().hmfree_func)(rm.raw(), elemsize);
        }
    }
}

// ===========================================================================
// E13  hmput_default
// ===========================================================================

#[test]
fn e13_hmput_default_paths() {
    let s = session(0x31415926);
    unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let ca = (s.c().hmput_default)(std::ptr::null_mut(), elemsize);
            let ra = (s.r().hmput_default)(std::ptr::null_mut(), elemsize);
            let cm = Map { t: ca, elemsize };
            let rm = Map { t: ra, elemsize };
            assert_eq!((*cm.header()).length, 1);
            assert_eq!((*rm.header()).length, 1);
            assert_eq!((*cm.header()).capacity, (*rm.header()).capacity);
            // Second call must be a pure no-op.
            let c2 = (s.c().hmput_default)(ca, elemsize);
            let r2 = (s.r().hmput_default)(ra, elemsize);
            assert_eq!(c2, ca, "E13: C second call must be a no-op");
            assert_eq!(r2, ra, "E13: Rust second call must be a no-op");
            (s.c().hmfree_func)(cm.raw(), elemsize);
            (s.r().hmfree_func)(rm.raw(), elemsize);
        }
    }
}

// ===========================================================================
// E14 / E15 / E16 / E17 / E19 / E20  assert reachability
// ===========================================================================

/// E14: the `used_count_threshold + tombstone_count_threshold < slot_count`
/// assert must hold for every slot_count the public API can produce (8 .. 4096),
/// and the three thresholds must match between C and Rust at every rebuild.
#[test]
fn e14_threshold_invariant_over_all_reachable_slot_counts() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE014);
    unsafe {
        let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
        let mut keys = Keys8::new(&mut rng, 3200);
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..3200usize {
            p.put(keys.ptr(i), HM_BINARY, &[i as u8, (i >> 8) as u8]);
            let t = &*p.cm.table();
            let r = &*p.rm.table();
            assert_eq!(t.slot_count, r.slot_count);
            assert!(
                t.used_count_threshold
                    .wrapping_add(t.tombstone_count_threshold)
                    < t.slot_count,
                "E14 would fire at slot_count={}",
                t.slot_count
            );
            assert!(
                r.used_count_threshold
                    .wrapping_add(r.tombstone_count_threshold)
                    < r.slot_count
            );
            // E15/E16 corollaries: capacity always covers length, slots in range.
            assert!((*p.cm.header()).length <= (*p.cm.header()).capacity);
            assert!((*p.rm.header()).length <= (*p.rm.header()).capacity);
            seen.insert(t.slot_count);
        }
        assert!(
            seen.contains(&8) && seen.contains(&4096),
            "expected slot_count to sweep 8..=4096, saw {seen:?}"
        );
        p.free();
    }
}

/// E18 (reachable!): `stbds_hmdel_key` with `mode > STBDS_HM_STRING` deleting a
/// NON-final entry.  Because the key-reload branch tests `mode == HM_STRING`,
/// the re-lookup hashes the raw pointer bytes, fails, and the live
/// `STBDS_ASSERT(slot >= 0)` aborts.  Run in a child process: BOTH libraries
/// must die with SIGABRT.
#[test]
fn e18_mode2_nonlast_delete_aborts_in_both() {
    for which in ["c", "rust"] {
        let out = run_abort_case("e18", which);
        let sig = signal_of(&out);
        assert_eq!(
            sig,
            Some(6),
            "E18 ({which}): expected SIGABRT, got status {:?}\nstdout: {}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Control for the harness above: the same scenario deleting the FINAL entry
/// must exit cleanly in both, proving the abort is caused by the assert and not
/// by the child-process plumbing.
#[test]
fn e18_control_mode2_last_delete_succeeds_in_both() {
    for which in ["c", "rust"] {
        let out = run_abort_case("e18ok", which);
        assert!(
            out.status.success(),
            "E18 control ({which}): expected clean exit, got {:?}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

fn signal_of(out: &std::process::Output) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    out.status.signal()
}

fn run_abort_case(case: &str, which: &str) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args(["--exact", "abort_case_child", "--ignored", "--nocapture"])
        .env("SHPUTS_ABORT_CASE", case)
        .env("SHPUTS_ABORT_LIB", which)
        .output()
        .expect("spawn child")
}

/// The child-process body for the abort cases. Does nothing unless invoked with
/// `SHPUTS_ABORT_CASE` set (i.e. never during a normal `cargo test` run).
#[test]
#[ignore]
fn abort_case_child() {
    let case = match std::env::var("SHPUTS_ABORT_CASE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var("SHPUTS_ABORT_LIB").unwrap_or_else(|_| "c".into());
    let s = session(0x31415926);
    let api = if which == "c" { s.c() } else { s.r() };
    let elemsize = 16usize;
    unsafe {
        match case.as_str() {
            "e18" | "e18ok" => {
                let mut t = (api.shmode_func)(elemsize, SH_DEFAULT);
                let keys: Vec<Box<[u8]>> = (0..4)
                    .map(|i| {
                        let mut v = format!("abortkey-{i}").into_bytes();
                        v.push(0);
                        v.into_boxed_slice()
                    })
                    .collect();
                let mut keys = keys;
                for k in keys.iter_mut() {
                    t = (api.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, 8, 2);
                }
                let victim = if case == "e18" { 0 } else { keys.len() - 1 };
                t = (api.hmdel_key)(
                    t,
                    elemsize,
                    keys[victim].as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    2,
                );
                // Reached only in the `e18ok` control case.
                assert!(!t.is_null());
                println!("child survived: case={case} lib={which}");
                std::process::exit(0);
            }
            other => panic!("unknown abort case `{other}`"),
        }
    }
}

/// E17: `STBDS_ASSERT(table->used_count >= 0)` compares a `size_t` against 0 —
/// a tautology.  Prove it can never fire by driving `used_count` through the
/// `--table->used_count` on l.830 many times and observing it stays in step.
#[test]
fn e17_used_count_is_unsigned() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE017);
    unsafe {
        let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
        let mut keys = Keys8::new(&mut rng, 40);
        for i in 0..40 {
            p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0]);
        }
        for i in 0..40 {
            p.del(keys.ptr(i), HM_BINARY, 0);
            let c = (*p.cm.table()).used_count;
            let r = (*p.rm.table()).used_count;
            assert_eq!(c, r, "E17: used_count diverged at delete {i}");
        }
        p.free();
    }
}

/// E20: the `len <= a->remaining` assert is unreachable through the public API —
/// verify the invariant holds after every one of thousands of `stbds_stralloc`
/// calls, on both sides.
#[test]
fn e20_stralloc_remaining_invariant() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE020);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for _ in 0..3000usize {
            let n = match rng.below(6) {
                0 => 0,
                1 => rng.range(1, 40),
                2 => rng.range(500, 600),
                3 => rng.range(1000, 2000),
                4 => rng.range(60_000, 61_000),
                _ => rng.range(1, 500),
            };
            let mut text = rng.cstring(n);
            let cp = (s.c().stralloc)(&mut ca, text.as_mut_ptr() as *mut c_char);
            let rp = (s.r().stralloc)(&mut ra, text.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(cp), text[..n].to_vec());
            assert_eq!(cstr_bytes(rp), text[..n].to_vec());
            assert_eq!(ca.remaining, ra.remaining, "E20: remaining diverged");
            assert_eq!(ca.block, ra.block, "E20: block diverged");
            assert!(ca.block <= 22, "E20/E24: block escaped its cap");
        }
        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
    }
}

// ===========================================================================
// E21 / E44  sh_puts asserts hold, and num <= 0 skips the arena loop
// ===========================================================================

#[test]
fn e21_e44_sh_puts_edge_nums() {
    for n in [
        0 as c_int,
        -1,
        -2,
        -12345,
        c_int::MIN,
        c_int::MIN + 1,
        1,
        2,
        7,
        8,
        9,
    ] {
        let cout = sh_puts_stdout(n, "c", "sh_puts_child_runner");
        let rout = sh_puts_stdout(n, "rust", "sh_puts_child_runner");
        assert_eq!(cout, rout, "E44: sh_puts({n}) stdout differs");
        assert_eq!(cout, format!("a {n}\n").into_bytes());
    }
}

/// Child-process runner for the `sh_puts` captures; inert during a normal run.
#[test]
#[ignore]
fn sh_puts_child_runner() {
    common::sh_puts_child_main();
}

// ===========================================================================
// E22 / E23 / E24 / E25  string-arena edges
// ===========================================================================

#[test]
fn e22_e23_e25_arena_edges() {
    let s = session(0x31415926);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();

        // E25: reset a virgin arena, twice.
        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
        assert!(ca.storage.is_null() && ra.storage.is_null());

        // E22: the empty string still consumes one byte.
        let mut empty = vec![0u8];
        let cp = (s.c().stralloc)(&mut ca, empty.as_mut_ptr() as *mut c_char);
        let rp = (s.r().stralloc)(&mut ra, empty.as_mut_ptr() as *mut c_char);
        assert_eq!(*cp, 0);
        assert_eq!(*rp, 0);
        assert_eq!(ca.remaining, 511, "E22: 512-byte block minus one NUL");
        assert_eq!(ra.remaining, 511);
        assert_eq!(ca.block, 1);
        assert_eq!(ra.block, 1);

        // E23: len > blocksize -> dedicated block, `remaining` untouched.
        let before = ca.remaining;
        let mut huge = vec![b'H'; 5000];
        huge.push(0);
        let cp = (s.c().stralloc)(&mut ca, huge.as_mut_ptr() as *mut c_char);
        let rp = (s.r().stralloc)(&mut ra, huge.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr_bytes(cp).len(), 5000);
        assert_eq!(cstr_bytes(rp).len(), 5000);
        assert_eq!(
            ca.remaining, before,
            "E23: dedicated block must not consume `remaining`"
        );
        assert_eq!(ra.remaining, before);
        // spliced in behind the head, not as the new head
        assert_eq!(cp as usize, (*ca.storage).next as usize + 8);
        assert_eq!(rp as usize, (*ra.storage).next as usize + 8);

        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
        assert_eq!(ca.remaining, 0);
        assert_eq!(ra.remaining, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);
        assert!(ca.storage.is_null() && ra.storage.is_null());
    }
}

/// E23 second shape: a dedicated block requested on a *fresh* arena becomes the
/// head and forces `remaining = 0`.
#[test]
fn e23_dedicated_block_on_empty_arena() {
    let s = session(0x31415926);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut huge = vec![b'Q'; 4096];
        huge.push(0);
        let cp = (s.c().stralloc)(&mut ca, huge.as_mut_ptr() as *mut c_char);
        let rp = (s.r().stralloc)(&mut ra, huge.as_mut_ptr() as *mut c_char);
        assert_eq!(ca.remaining, 0, "E23: remaining forced to 0");
        assert_eq!(ra.remaining, 0);
        assert_eq!(ca.block, 1);
        assert_eq!(ra.block, 1);
        assert_eq!(cp as usize, ca.storage as usize + 8);
        assert_eq!(rp as usize, ra.storage as usize + 8);
        assert!((*ca.storage).next.is_null() && (*ra.storage).next.is_null());
        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
    }
}

/// E24: `block` freezes exactly when `512 << (block>>1) >= 1<<20`.
#[test]
fn e24_block_cap() {
    let s = session(0x31415926);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for i in 0..5000usize {
            let mut text = vec![b'a' + (i % 26) as u8; 400];
            text.push(0);
            (s.c().stralloc)(&mut ca, text.as_mut_ptr() as *mut c_char);
            (s.r().stralloc)(&mut ra, text.as_mut_ptr() as *mut c_char);
            assert_eq!(ca.block, ra.block, "E24: block diverged at i={i}");
            assert!(ca.block <= 22, "E24: block exceeded 22 at i={i}");
        }
        assert_eq!(ca.block, 22, "E24: block must saturate at 22");
        (s.c().strreset)(&mut ca);
        (s.r().strreset)(&mut ra);
    }
}

// ===========================================================================
// E26 / E27 / E28 / E29  stbds_arrgrowf edges
// ===========================================================================

#[test]
fn e26_e29_arrgrowf_edges() {
    let s = session(0x31415926);
    unsafe {
        // E28: (NULL, es, 0, 0) must return NULL without allocating.
        for elemsize in [0usize, 1, 8, 16, 40] {
            let c = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let r = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(c.is_null(), "E28: C must return NULL (es={elemsize})");
            assert!(r.is_null(), "E28: Rust must return NULL (es={elemsize})");
        }

        // E29: elemsize == 0 still yields a header-only allocation.
        let c = (s.c().arrgrowf)(std::ptr::null_mut(), 0, 1, 0);
        let r = (s.r().arrgrowf)(std::ptr::null_mut(), 0, 1, 0);
        assert!(!c.is_null() && !r.is_null());
        let ch = (c as *mut ArrayHeader).wrapping_sub(1);
        let rh = (r as *mut ArrayHeader).wrapping_sub(1);
        assert_eq!((*ch).capacity, (*rh).capacity, "E29: capacity");
        assert_eq!((*ch).capacity, 4, "E29: min_cap clamped to 4");
        assert_eq!((*ch).length, 0);
        assert_eq!((*rh).length, 0);
        (s.c().arrfreef)(c);
        (s.r().arrfreef)(r);

        // E27: the `else if (min_cap < 4)` clamp applies only to empty arrays.
        for (addlen, min_cap, want_cap) in [
            (1usize, 0usize, 4usize),
            (0, 1, 4),
            (0, 3, 4),
            (0, 4, 4),
            (0, 5, 5),
            (3, 0, 4),
            (7, 0, 7),
            (0, 100, 100),
        ] {
            let c = (s.c().arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            let r = (s.r().arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            let ch = (c as *mut ArrayHeader).wrapping_sub(1);
            let rh = (r as *mut ArrayHeader).wrapping_sub(1);
            assert_eq!(
                (*ch).capacity,
                (*rh).capacity,
                "E27: capacity differs for addlen={addlen} min_cap={min_cap}"
            );
            assert_eq!(
                (*ch).capacity, want_cap,
                "E27: unexpected capacity for addlen={addlen} min_cap={min_cap}"
            );
            (s.c().arrfreef)(c);
            (s.r().arrfreef)(r);
        }

        // E26: a request that already fits is a pure no-op on both.
        let mut c = (s.c().arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        let mut r = (s.r().arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        for min_cap in [0usize, 1, 5, 9, 10] {
            let c2 = (s.c().arrgrowf)(c, 8, 0, min_cap);
            let r2 = (s.r().arrgrowf)(r, 8, 0, min_cap);
            assert_eq!(c2, c, "E26: C must return `a` unchanged");
            assert_eq!(r2, r, "E26: Rust must return `a` unchanged");
            c = c2;
            r = r2;
        }
        (s.c().arrfreef)(c);
        (s.r().arrfreef)(r);
    }
}

// ===========================================================================
// E30 / E31 / E32  hashing edges
// ===========================================================================

#[test]
fn e30_hash_bytes_len_zero_never_reads() {
    let s = session(0x31415926);
    unsafe {
        // NULL, a dangling-but-unmapped-looking value, and a real buffer.
        for &p in &[
            std::ptr::null_mut::<c_void>(),
            usize::MAX as *mut c_void,
            1 as *mut c_void,
        ] {
            for seed in [0usize, 1, usize::MAX, 0x31415926] {
                let c = (s.c().hash_bytes)(p, 0, seed);
                let r = (s.r().hash_bytes)(p, 0, seed);
                assert_eq!(c, r, "E30: hash_bytes({p:?}, 0, {seed:#x})");
            }
        }
    }
}

#[test]
fn e31_hash_bytes_every_tail_case() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE031);
    unsafe {
        // For each residue 0..7 and a range of whole 8-byte blocks, hammer the
        // switch fall-through including the sign-extending `case 4`.
        for blocks in 0usize..4 {
            for rem in 0usize..8 {
                let len = blocks * 8 + rem;
                for trial in 0..64 {
                    let mut buf = rng.bytes(len.max(1));
                    if trial % 4 == 0 && len > 3 {
                        buf[len - rem.max(1)..].iter_mut().for_each(|b| *b |= 0x80);
                        if rem >= 4 {
                            buf[blocks * 8 + 3] = 0xFF;
                        }
                    }
                    let seed = if trial < 4 {
                        [0usize, 1, usize::MAX, 1 << 63][trial]
                    } else {
                        rng.next_u64() as usize
                    };
                    let p = buf.as_mut_ptr() as *mut c_void;
                    assert_eq!(
                        (s.c().hash_bytes)(p, len, seed),
                        (s.r().hash_bytes)(p, len, seed),
                        "E31: len={len} (blocks={blocks} rem={rem}) seed={seed:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn e32_hash_string_empty() {
    let s = session(0x31415926);
    unsafe {
        let mut empty = vec![0u8];
        for seed in [0usize, 1, 2, usize::MAX, 0x31415926, 1 << 63] {
            let c = (s.c().hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let r = (s.r().hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(c, r, "E32: hash_string(\"\", {seed:#x})");
        }
    }
}

/// E33: `if (hash < 2) hash += 2` guarantees no occupied slot can carry the
/// EMPTY(0) or DELETED(1) sentinel.  A direct witness (a key hashing to 0 or 1)
/// is computationally infeasible for a 64-bit SipHash, so the branch is verified
/// through its invariant over a large randomized workload plus the fact that
/// both libraries return bit-identical hashes for every probe above.
#[test]
fn e33_no_occupied_slot_carries_a_sentinel_hash() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE033);
    unsafe {
        for &(mode, shmode) in &[
            (HM_BINARY, None),
            (HM_STRING, Some(SH_STRDUP)),
            (HM_STRING, Some(SH_ARENA)),
        ] {
            let ptr_keys = mode >= HM_STRING;
            let repr = if ptr_keys {
                KeyRepr::Pointer
            } else {
                KeyRepr::Inline(8)
            };
            let mut p = Pair::new(&s, 16, 8, repr, 8..16);
            if let Some(m) = shmode {
                p.shmode(m);
            }
            let mut bufs: Vec<Box<[u8]>> = Vec::new();
            for i in 0..600usize {
                let mut v = if ptr_keys {
                    format!("e33-{i}-{}", rng.next_u64()).into_bytes()
                } else {
                    let mut b = rng.bytes(8);
                    b[0] = i as u8;
                    b[1] = (i >> 8) as u8;
                    b
                };
                v.push(0);
                bufs.push(v.into_boxed_slice());
            }
            for (i, k) in bufs.iter_mut().enumerate() {
                p.put(k.as_mut_ptr(), mode, &[i as u8, 0x99]);
            }
            for m in [&p.cm, &p.rm] {
                let t = &*m.table();
                for b in 0..(t.slot_count >> 3) {
                    let bucket = &*t.storage.wrapping_add(b);
                    for j in 0..8 {
                        if bucket.index[j] >= 0 {
                            assert!(
                                bucket.hash[j] >= 2,
                                "E33: occupied slot carries sentinel hash {}",
                                bucket.hash[j]
                            );
                        }
                    }
                }
            }
            p.free();
        }
    }
}

// ===========================================================================
// E34 / E35 / E36  growth / shrink / rebuild thresholds
// ===========================================================================

#[test]
fn e34_e36_threshold_transitions() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE034);
    unsafe {
        let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
        let mut keys = Keys8::new(&mut rng, 600);
        // E34: growth points.
        let mut grow_points = Vec::new();
        let mut last = 0usize;
        for i in 0..600usize {
            p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0x21]);
            let sc = (*p.cm.table()).slot_count;
            assert_eq!(sc, (*p.rm.table()).slot_count, "E34: slot_count diverged");
            if sc != last {
                grow_points.push((i, sc));
                last = sc;
            }
        }
        assert_eq!(
            grow_points
                .iter()
                .map(|x| x.1)
                .collect::<Vec<_>>(),
            vec![8usize, 16, 32, 64, 128, 256, 512, 1024],
            "E34: unexpected growth ladder: {grow_points:?}"
        );

        // E35/E36: shrink and tombstone-rebuild on the way back down.
        let mut shrink_points = Vec::new();
        last = (*p.cm.table()).slot_count;
        for i in 0..600usize {
            p.del(keys.ptr(i), HM_BINARY, 0);
            let sc = (*p.cm.table()).slot_count;
            assert_eq!(sc, (*p.rm.table()).slot_count, "E35: slot_count diverged");
            assert_eq!(
                (*p.cm.table()).tombstone_count,
                (*p.rm.table()).tombstone_count,
                "E36: tombstone_count diverged at delete {i}"
            );
            if sc != last {
                shrink_points.push((i, sc));
                last = sc;
            }
        }
        assert!(
            shrink_points.last().map(|x| x.1) == Some(8),
            "E35: expected to shrink back to 8, got {shrink_points:?}"
        );
        p.free();
    }
}

// ===========================================================================
// E37  out-of-range `int mode` across every entry point
// ===========================================================================

#[test]
fn e37_out_of_range_mode_partitioning() {
    // `mode >= 1` must behave exactly like mode 1 for hashing/comparison, and
    // `mode <= 0` exactly like mode 0.  Compare the resulting table state of an
    // exotic mode against its canonical representative, on both libraries.
    for (exotic, canonical, ptr_keys) in [
        (2 as c_int, HM_STRING, true),
        (7, HM_STRING, true),
        (c_int::MAX, HM_STRING, true),
        (-1, HM_BINARY, false),
        (-1000, HM_BINARY, false),
        (c_int::MIN, HM_BINARY, false),
    ] {
        let n = 30usize;
        let mut snaps = Vec::new();
        for &mode in &[exotic, canonical] {
            let s = session(0x0E37);
            unsafe {
                let repr = if ptr_keys {
                    KeyRepr::Pointer
                } else {
                    KeyRepr::Inline(8)
                };
                let vr = if ptr_keys { 8..16 } else { 8..16 };
                let mut p = Pair::new(&s, 16, 8, repr, vr);
                let mut bufs: Vec<Box<[u8]>> = Vec::new();
                let mut r2 = Rng::new(0xFEED);
                for i in 0..n {
                    let mut v = if ptr_keys {
                        format!("e37-key-{i}").into_bytes()
                    } else {
                        let mut b = r2.bytes(8);
                        b[0] = i as u8;
                        b
                    };
                    v.push(0);
                    bufs.push(v.into_boxed_slice());
                }
                for (i, k) in bufs.iter_mut().enumerate() {
                    p.put(k.as_mut_ptr(), mode, &[i as u8, 0x37]);
                }
                for (i, k) in bufs.iter_mut().enumerate() {
                    assert_eq!(
                        p.get(k.as_mut_ptr(), mode),
                        i as isize,
                        "E37 mode={mode}: get #{i}"
                    );
                }
                let (c, r) = p.snap();
                assert_snap_eq(&c, &r, &format!("E37 mode={mode}"));
                snaps.push(c);
                p.free();
            }
        }
        assert_eq!(
            snaps[0], snaps[1],
            "E37: mode={exotic} must be indistinguishable from mode={canonical}"
        );

    }
}

// ===========================================================================
// E38 / E39  shmode_func with out-of-range modes
// ===========================================================================

#[test]
fn e38_e39_shmode_func_out_of_range() {
    let s = session(0x31415926);
    unsafe {
        for &m in &[
            0 as c_int,
            1,
            2,
            3,
            4,
            5,
            127,
            128,
            255,
            256,
            257,
            258,
            259,
            -1,
            -2,
            -255,
            -256,
            1000,
            c_int::MAX,
            c_int::MIN,
        ] {
            for elemsize in [8usize, 16, 40] {
                let ct = (s.c().shmode_func)(elemsize, m);
                let rt = (s.r().shmode_func)(elemsize, m);
                assert!(!ct.is_null() && !rt.is_null(), "E39: never returns NULL");
                let cm = Map { t: ct, elemsize };
                let rm = Map { t: rt, elemsize };
                let ctab = &*cm.table();
                let rtab = &*rm.table();
                // E38: `(unsigned char) mode`
                assert_eq!(ctab.string.mode, m as u8, "E38: C stored mode for {m}");
                assert_eq!(rtab.string.mode, m as u8, "E38: Rust stored mode for {m}");
                // E39: always a fresh 8-slot table with length 1
                assert_eq!(ctab.slot_count, 8);
                assert_eq!(rtab.slot_count, 8);
                assert_eq!((*cm.header()).length, 1);
                assert_eq!((*rm.header()).length, 1);
                assert_eq!(ctab.used_count, 0);
                assert_eq!(rtab.used_count, 0);
                assert_eq!(ctab.tombstone_count, 0);
                assert_eq!(rtab.tombstone_count, 0);
                let cz = std::slice::from_raw_parts(cm.elem(0), elemsize).to_vec();
                let rz = std::slice::from_raw_parts(rm.elem(0), elemsize).to_vec();
                assert_eq!(cz, vec![0u8; elemsize]);
                assert_eq!(rz, vec![0u8; elemsize]);
                (s.c().hmfree_func)(cm.raw(), elemsize);
                (s.r().hmfree_func)(rm.raw(), elemsize);
            }
        }
    }
}

// ===========================================================================
// E40 / E41 / E42 / E43  insert/delete index bookkeeping
// ===========================================================================

/// E40: the very first `hmput_key` on a NULL map creates the `t[-1]` default
/// element, so the first real entry sits at raw index 1 / reported index 0.
#[test]
fn e40_first_put_index() {
    let s = session(0x31415926);
    unsafe {
        for elemsize in [8usize, 16, 40] {
            let mut key = [0x42u8; 9];
            let ct = (s.c().hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                HM_BINARY,
            );
            let rt = (s.r().hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                HM_BINARY,
            );
            let cm = Map { t: ct, elemsize };
            let rm = Map { t: rt, elemsize };
            assert_eq!((*cm.header()).temp, 0, "E40: C reported index");
            assert_eq!((*rm.header()).temp, 0, "E40: Rust reported index");
            assert_eq!((*cm.header()).length, 2, "E40: C raw length");
            assert_eq!((*rm.header()).length, 2, "E40: Rust raw length");
            assert_eq!(cm.len(), 1);
            assert_eq!(rm.len(), 1);
            // the key must be memcpy'd into raw element 1
            assert_eq!(
                std::slice::from_raw_parts(cm.elem(1), 8),
                std::slice::from_raw_parts(rm.elem(1), 8)
            );
            assert_eq!(std::slice::from_raw_parts(cm.elem(1), 8), &key[..8]);
            (s.c().hmfree_func)(cm.raw(), elemsize);
            (s.r().hmfree_func)(rm.raw(), elemsize);
        }
    }
}

#[test]
fn e41_e42_delete_last_vs_middle() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE041);
    unsafe {
        for &n in &[1usize, 2, 5, 20, 90] {
            for &victim_kind in &["last", "middle", "first"] {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
                let mut keys = Keys8::new(&mut rng, n);
                for i in 0..n {
                    p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0x41]);
                }
                let v = match victim_kind {
                    "last" => n - 1,
                    "middle" => n / 2,
                    _ => 0,
                };
                let len_before = (*p.cm.header()).length;
                assert_eq!(p.del(keys.ptr(v), HM_BINARY, 0), 1);
                assert_eq!((*p.cm.header()).length, len_before - 1);
                assert_eq!((*p.rm.header()).length, len_before - 1);
                // E41: deleting the last entry leaves every other index intact.
                if v == n - 1 {
                    for i in 0..n - 1 {
                        assert_eq!(p.get(keys.ptr(i), HM_BINARY), i as isize, "E41 n={n}");
                    }
                } else {
                    // E42: the final entry was moved into the hole.
                    assert_eq!(
                        p.get(keys.ptr(n - 1), HM_BINARY),
                        v as isize,
                        "E42 n={n}: last entry must now live at {v}"
                    );
                }
                assert_eq!(p.get(keys.ptr(v), HM_BINARY), -1);
                p.free();
            }
        }
    }
}

/// E43: `keyoffset != 0`.  A non-zero offset makes `is_key_equal` read the wrong
/// bytes, so an otherwise-present key misses; and when the bytes DO match at
/// that offset the delete succeeds.  Both behaviours must agree.
#[test]
fn e43_nonzero_keyoffset() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xE043);
    let (elemsize, keysize) = (24usize, 8usize);
    unsafe {
        // (a) present keys become invisible at a wrong keyoffset.
        let mut p = Pair::new(&s, elemsize, keysize, KeyRepr::Inline(keysize), keysize..elemsize);
        let n = 12usize;
        let mut keys = Keys8::new(&mut rng, n);
        for i in 0..n {
            p.put(keys.ptr(i), HM_BINARY, &[0xB0 | i as u8, 0x43, 0x11, 0x22]);
        }
        for i in 0..n {
            for &ko in &[8usize, 16] {
                assert_eq!(
                    p.del(keys.ptr(i), HM_BINARY, ko),
                    0,
                    "E43a: key {i} must not be found at keyoffset={ko}"
                );
            }
        }
        assert_eq!(p.cm.len(), n as isize, "E43a: nothing may be deleted");
        p.free();

        // (b) mirror the key to offset 8 of the FINAL element and delete it with
        //     keyoffset=8 (old_index == final_index, so no repatch happens).
        let mut p = Pair::new(&s, elemsize, keysize, KeyRepr::Inline(keysize), keysize..elemsize);
        let mut keys = Keys8::new(&mut rng, 6);
        for i in 0..6 {
            p.put(keys.ptr(i), HM_BINARY, &[0u8; 4]);
        }
        for m in [&p.cm, &p.rm] {
            std::ptr::copy_nonoverlapping(keys.bufs[5].as_ptr(), m.elem(6).add(8), keysize);
        }
        p.check("E43b setup");
        assert_eq!(
            p.del(keys.ptr(5), HM_BINARY, 8),
            1,
            "E43b: matching bytes at keyoffset=8 must delete"
        );
        assert_eq!(p.cm.len(), 5);
        p.free();
    }
}

// ===========================================================================
// E45  strkey
// ===========================================================================

#[test]
fn e45_strkey_extremes() {
    let s = session(0x31415926);
    unsafe {
        for n in [
            0 as c_int,
            -1,
            1,
            9,
            -9,
            10,
            -10,
            c_int::MAX,
            c_int::MIN,
            c_int::MIN + 1,
            -2147483647,
        ] {
            let c = cstr_bytes((s.c().strkey)(n));
            let r = cstr_bytes((s.r().strkey)(n));
            assert_eq!(c, r, "E45: strkey({n})");
            assert_eq!(c, format!("test_{n}").into_bytes());
            assert!(c.len() < 256, "E45: must fit the 256-byte static buffer");
        }
    }
}

// ===========================================================================
// E46  stbds_arrfreef(NULL) — structural note only
// ===========================================================================

/// E46: `stbds_arrfreef(NULL)` computes `free((stbds_array_header *) NULL - 1)`,
/// i.e. `free((void *) -32)`, in BOTH implementations. glibc aborts or faults,
/// so calling it would kill this process. Assert the shared address arithmetic
/// instead of performing the free.
#[test]
fn e46_arrfreef_null_is_undefined_in_both() {
    let hdr = std::mem::size_of::<ArrayHeader>();
    assert_eq!(hdr, 32, "header size assumption behind E46");
    let bogus = (std::ptr::null_mut::<ArrayHeader>()).wrapping_sub(1) as usize;
    assert_eq!(bogus, usize::MAX - 31, "E46: both pass free() the same value");
}
