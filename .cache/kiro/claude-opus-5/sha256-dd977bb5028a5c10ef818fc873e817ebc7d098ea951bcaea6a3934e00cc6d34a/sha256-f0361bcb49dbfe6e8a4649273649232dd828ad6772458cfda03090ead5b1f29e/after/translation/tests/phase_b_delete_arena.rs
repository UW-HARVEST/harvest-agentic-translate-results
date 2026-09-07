//! Phase B — valid-path differential tests, rows 35..56 of CONFIGS.md:
//! `stbds_hmdel_key`, `stbds_hmfree_func`, `stbds_stralloc`, `stbds_strreset`,
//! `strkey`, `str_dups`, and the end-to-end pipelines.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

const L8: Layout = Layout::new(8, 4, 4, 4);
const L16: Layout = Layout::new(16, 8, 8, 8);
const L24: Layout = Layout::new(24, 8, 8, 16);
const LS: Layout = Layout::new(16, 8, 8, 4);

/// Compares two maps, tolerating the `NULL` that `stbds_hmdel_key` returns when
/// handed a `NULL` array.
fn assert_maps_eq(tc: *mut c_void, tr: *mut c_void, lay: Layout, repr: KeyRepr, msg: &str) {
    unsafe {
        assert_eq!(tc.is_null(), tr.is_null(), "{msg}: null-ness differs");
        if tc.is_null() {
            return;
        }
        let sc = snapshot_map_lay(tc, lay, repr);
        let sr = snapshot_map_lay(tr, lay, repr);
        assert_eq!(sc, sr, "{msg}");
    }
}

// ---------------------------------------------------------------------------
// Rows 35..38, 41 — binary-mode deletion
// ---------------------------------------------------------------------------

/// Inserts `n` distinct keys, then deletes them in the given order, comparing
/// state after every operation.
fn binary_delete_scenario(lay: Layout, n: usize, seed: usize, order: &[usize], rng_seed: u64) {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
        let mut rng = Rng::new(rng_seed);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let mut k = vec![0u8; lay.keysize];
            let src = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).to_ne_bytes();
            for (j, b) in k.iter_mut().enumerate() {
                *b = src[j % 8];
            }
            let mut k2 = k.clone();
            let v = rng.bytes(lay.value_size);
            let (nc, _) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            keys.push(k);
        }
        let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
        let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
        assert_eq!(sc, sr, "{lay:?} after inserts");

        for (step, &i) in order.iter().enumerate() {
            let mut kc = keys[i].clone();
            let mut kr = keys[i].clone();
            let (nc, dtc) = c.hmdel(tc, lay, kc.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            let (nr, dtr) = r.hmdel(tr, lay, kr.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(dtc, dtr, "{lay:?} step={step} del temp differs (key idx {i})");
            let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
            assert_eq!(sc, sr, "{lay:?} step={step} state differs after del(idx {i})");
            // every remaining key must resolve identically
            for (j, k) in keys.iter().enumerate() {
                let mut a = k.clone();
                let mut b = k.clone();
                let (nc, ta) = c.hmgeti(tc, lay, a.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                let (nr, tb) = r.hmgeti(tr, lay, b.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(ta, tb, "{lay:?} step={step} get idx {j}");
            }
        }
        c.hmfree(tc, lay.elemsize);
        r.hmfree(tr, lay.elemsize);
    }
}

/// Row 35 — delete the last element (`old_index == final_index`).
#[test]
fn row_35_delete_last_element() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        binary_delete_scenario(L16, 5, seed, &[4, 3, 2, 1, 0], SEED ^ 35);
        binary_delete_scenario(L8, 5, seed, &[4, 3, 2, 1, 0], SEED ^ 0x35);
    }
}

/// Row 36 — delete middle elements (`old_index != final_index` → memmove +
/// re-slot of the moved element).
#[test]
fn row_36_delete_middle_element() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        binary_delete_scenario(L16, 6, seed, &[0, 1, 2, 3, 4, 5], SEED ^ 36);
        binary_delete_scenario(L16, 7, seed, &[3, 0, 4, 1, 5, 2, 6], SEED ^ 0x36);
        binary_delete_scenario(L24, 7, seed, &[2, 4, 0, 6, 1, 5, 3], SEED ^ 0x360);
    }
}

/// Row 37 — cross `used_count_shrink_threshold` (`slot_count>>2`) to force the
/// shrink rehash.
#[test]
fn row_37_delete_shrink_rehash() {
    for seed in [0usize, 0x3141_5926] {
        // 40 entries -> slot_count 64; deleting down to <16 shrinks repeatedly
        let order: Vec<usize> = (0..40).collect();
        binary_delete_scenario(L16, 40, seed, &order, SEED ^ 37);
        let rev: Vec<usize> = (0..40).rev().collect();
        binary_delete_scenario(L16, 40, seed, &rev, SEED ^ 0x37);
    }
}

/// Row 38 — insert/delete churn crossing `tombstone_count_threshold`.
#[test]
fn row_38_tombstone_rebuild() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let lay = L16;
    unsafe {
        for seed in [0usize, 0x3141_5926] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            let mut rng = Rng::new(SEED ^ 38);
            // build up a 128-slot table
            let mut live: Vec<u64> = Vec::new();
            for i in 0u64..80 {
                let mut k = i.wrapping_mul(0x1000_0001).to_ne_bytes().to_vec();
                let mut k2 = k.clone();
                let v = rng.bytes(8);
                let (nc, _) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
                let (nr, _) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                live.push(i);
            }
            // alternate delete + reinsert so used_count stays high while
            // tombstone_count climbs past its threshold
            for step in 0..400usize {
                let idx = rng.below(live.len());
                let key = live[idx];
                let mut kc = key.wrapping_mul(0x1000_0001).to_ne_bytes().to_vec();
                let mut kr = kc.clone();
                let (nc, a) = c.hmdel(tc, lay, kc.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
                let (nr, b) = r.hmdel(tr, lay, kr.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "seed={seed:#x} step={step} del");
                let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
                let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
                assert_eq!(sc, sr, "seed={seed:#x} step={step} after del");

                let newk = 1000 + step as u64;
                let mut kc = newk.wrapping_mul(0x1000_0001).to_ne_bytes().to_vec();
                let mut kr = kc.clone();
                let v = rng.bytes(8);
                let (nc, a) = c.hmput(tc, lay, &mut kc, &v, STBDS_HM_BINARY);
                let (nr, b) = r.hmput(tr, lay, &mut kr, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "seed={seed:#x} step={step} put");
                let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
                let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
                assert_eq!(sc, sr, "seed={seed:#x} step={step} after put");
                live[idx] = newk;
            }
            c.hmfree(tc, lay.elemsize);
            r.hmfree(tr, lay.elemsize);
        }
    }
}

/// Row 41 — long randomised insert/get/delete churn.
#[test]
fn row_41_binary_random_churn() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for lay in [L8, L16, L24] {
            for seed in [0usize, 0x3141_5926] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut tc: *mut c_void = std::ptr::null_mut();
                let mut tr: *mut c_void = std::ptr::null_mut();
                let mut rng = Rng::new(SEED ^ 41 ^ lay.elemsize as u64);
                for step in 0..2000usize {
                    let kv = rng.below(200) as u64;
                    let mut kc = vec![0u8; lay.keysize];
                    let src = kv.wrapping_mul(0x9E37_79B9).to_ne_bytes();
                    for (j, b) in kc.iter_mut().enumerate() {
                        *b = src[j % 8];
                    }
                    let mut kr = kc.clone();
                    match rng.below(4) {
                        0 | 1 => {
                            let v = rng.bytes(lay.value_size);
                            let (nc, a) = c.hmput(tc, lay, &mut kc, &v, STBDS_HM_BINARY);
                            let (nr, b) = r.hmput(tr, lay, &mut kr, &v, STBDS_HM_BINARY);
                            tc = nc;
                            tr = nr;
                            assert_eq!(a, b, "{lay:?} seed={seed:#x} step={step} put");
                        }
                        2 => {
                            let (nc, a) =
                                c.hmgeti(tc, lay, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                            let (nr, b) =
                                r.hmgeti(tr, lay, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                            tc = nc;
                            tr = nr;
                            assert_eq!(a, b, "{lay:?} seed={seed:#x} step={step} get");
                        }
                        _ => {
                            let (nc, a) = c.hmdel(
                                tc,
                                lay,
                                kc.as_mut_ptr() as *mut c_void,
                                0,
                                STBDS_HM_BINARY,
                            );
                            let (nr, b) = r.hmdel(
                                tr,
                                lay,
                                kr.as_mut_ptr() as *mut c_void,
                                0,
                                STBDS_HM_BINARY,
                            );
                            tc = nc;
                            tr = nr;
                            assert_eq!(a, b, "{lay:?} seed={seed:#x} step={step} del");
                        }
                    }
                    assert_maps_eq(
                        tc,
                        tr,
                        lay,
                        KeyRepr::Inline,
                        &format!("{lay:?} seed={seed:#x} step={step}"),
                    );
                }
                c.hmfree(tc, lay.elemsize);
                r.hmfree(tr, lay.elemsize);
            }
        }    }
}

// ---------------------------------------------------------------------------
// Rows 39, 40, 42 — string-mode deletion
// ---------------------------------------------------------------------------

struct Pool(Vec<Vec<u8>>);
impl Pool {
    fn new() -> Pool {
        Pool(Vec::new())
    }
    fn add(&mut self, s: &str) -> *mut c_char {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        while v.len() < 16 {
            v.push(0);
        }
        self.0.push(v);
        self.0.last_mut().unwrap().as_mut_ptr() as *mut c_char
    }
}

fn string_delete_scenario(table_mode: c_int, put_mode: c_int, seed: usize, n: usize, rng_seed: u64) {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
        let mut tc = (c.shmode_func)(LS.elemsize, table_mode);
        let mut tr = (r.shmode_func)(LS.elemsize, table_mode);
        let mut rng = Rng::new(rng_seed);
        let mut pc = Pool::new();
        let mut pr = Pool::new();
        let mut names: Vec<String> = Vec::new();
        for i in 0..n {
            let s = format!("k{i}_{}", "y".repeat(i % 7));
            let kc = pc.add(&s);
            let kr = pr.add(&s);
            let v = rng.bytes(4);
            let (nc, _) = c.shput(tc, LS, kc, &v, put_mode);
            let (nr, _) = r.shput(tr, LS, kr, &v, put_mode);
            tc = nc;
            tr = nr;
            names.push(s);
        }
        let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
        let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
        assert_eq!(sc, sr, "table_mode={table_mode} after inserts");

        // delete in a shuffled order
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (step, &i) in order.iter().enumerate() {
            let kc = pc.add(&names[i]);
            let kr = pr.add(&names[i]);
            let (nc, a) = c.hmdel(tc, LS, kc as *mut c_void, 0, put_mode);
            let (nr, b) = r.hmdel(tr, LS, kr as *mut c_void, 0, put_mode);
            tc = nc;
            tr = nr;
            assert_eq!(a, b, "table_mode={table_mode} step={step} del {}", names[i]);
            let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
            let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
            assert_eq!(sc, sr, "table_mode={table_mode} step={step} after del");
            for (j, nm) in names.iter().enumerate() {
                let a1 = pc.add(nm);
                let b1 = pr.add(nm);
                let (nc, x) = c.hmgeti(tc, LS, a1 as *mut c_void, put_mode);
                let (nr, y) = r.hmgeti(tr, LS, b1 as *mut c_void, put_mode);
                tc = nc;
                tr = nr;
                assert_eq!(x, y, "table_mode={table_mode} step={step} get {j}");
            }
        }
        c.hmfree(tc, LS.elemsize);
        r.hmfree(tr, LS.elemsize);
    }
}

/// Row 39 — `SH_STRDUP` deletion (the duped key is freed).
#[test]
fn row_39_delete_strdup() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        string_delete_scenario(STBDS_SH_STRDUP, STBDS_HM_STRING, seed, 24, SEED ^ 39);
    }
}

/// Row 40 — `SH_ARENA` / `SH_DEFAULT` deletion (no free).
#[test]
fn row_40_delete_arena_and_default() {
    for seed in [0usize, 0x3141_5926] {
        string_delete_scenario(STBDS_SH_ARENA, STBDS_HM_STRING, seed, 24, SEED ^ 40);
        string_delete_scenario(STBDS_SH_DEFAULT, STBDS_HM_STRING, seed, 24, SEED ^ 0x40);
    }
}

/// Row 40b — deletion with `mode = STBDS_HM_PTR_TO_STRING` (2). `hmdel_key`
/// takes its `mode == STBDS_HM_STRING` branch as FALSE, so the moved element is
/// re-looked-up by passing the *address* of the key pointer while
/// `stbds_hm_find_slot` still hashes it as a string (`mode >= STBDS_HM_STRING`).
/// The lookup misses and `STBDS_ASSERT(slot >= 0)` at lib.c:846 fires. Compared
/// in forked children so the abort itself is the observable.
#[test]
fn row_40b_delete_ptr_to_string_mode_asserts() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, table_mode: c_int, n: usize, del: usize| {
        unsafe {
            (l.rand_seed)(0x3141_5926);
            let mut t = (l.shmode_func)(LS.elemsize, table_mode);
            let mut p = Pool::new();
            let mut names = Vec::new();
            for i in 0..n {
                let s = format!("ptr_{i}");
                let (nt, _) = l.shput(t, LS, p.add(&s), &(i as u32).to_ne_bytes(), STBDS_HM_STRING);
                t = nt;
                names.push(s);
            }
            let (nt, _) = l.hmdel(
                t,
                LS,
                p.add(&names[del]) as *mut c_void,
                0,
                STBDS_HM_PTR_TO_STRING,
            );
            t = nt;
            l.hmfree(t, LS.elemsize);
            drop(p);
        }
    };
    for table_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA, STBDS_SH_DEFAULT] {
        for n in [1usize, 2, 5, 10] {
            for del in 0..n {
                let oc = probe_abort(|| run(c, table_mode, n, del));
                let or = probe_abort(|| run(r, table_mode, n, del));
                assert_eq!(oc, or, "table_mode={table_mode} n={n} del={del}");
            }
        }
    }
}

/// Row 42 — randomised string-mode churn over an `SH_STRDUP` table.
#[test]
fn row_42_string_random_churn() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for table_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA, STBDS_SH_DEFAULT] {
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let mut tc = (c.shmode_func)(LS.elemsize, table_mode);
            let mut tr = (r.shmode_func)(LS.elemsize, table_mode);
            let mut rng = Rng::new(SEED ^ 42 ^ table_mode as u64);
            let mut pc = Pool::new();
            let mut pr = Pool::new();
            for step in 0..1200usize {
                let kv = rng.below(150);
                let s = format!("s{kv}_{}", "z".repeat(kv % 11));
                let kc = pc.add(&s);
                let kr = pr.add(&s);
                match rng.below(4) {
                    0 | 1 => {
                        let v = rng.bytes(4);
                        let (nc, a) = c.shput(tc, LS, kc, &v, STBDS_HM_STRING);
                        let (nr, b) = r.shput(tr, LS, kr, &v, STBDS_HM_STRING);
                        tc = nc;
                        tr = nr;
                        assert_eq!(a, b, "mode={table_mode} step={step} put {s}");
                    }
                    2 => {
                        let (nc, a) = c.hmgeti(tc, LS, kc as *mut c_void, STBDS_HM_STRING);
                        let (nr, b) = r.hmgeti(tr, LS, kr as *mut c_void, STBDS_HM_STRING);
                        tc = nc;
                        tr = nr;
                        assert_eq!(a, b, "mode={table_mode} step={step} get {s}");
                    }
                    _ => {
                        let (nc, a) = c.hmdel(tc, LS, kc as *mut c_void, 0, STBDS_HM_STRING);
                        let (nr, b) = r.hmdel(tr, LS, kr as *mut c_void, 0, STBDS_HM_STRING);
                        tc = nc;
                        tr = nr;
                        assert_eq!(a, b, "mode={table_mode} step={step} del {s}");
                    }
                }
                let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
                let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
                assert_eq!(sc, sr, "mode={table_mode} step={step}");
            }
            c.hmfree(tc, LS.elemsize);
            r.hmfree(tr, LS.elemsize);
            // Pools must outlive the maps (SH_DEFAULT stores caller pointers).
            drop(pc);
            drop(pr);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 43 — stbds_hmfree_func across table modes
// ---------------------------------------------------------------------------

/// Row 43 — `hmfree_func` must take the same branches; run it in forked
/// children so an invalid free would be observable as a differing exit status.
#[test]
fn row_43_hmfree_across_table_modes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let build_and_free = |l: &Lib, table_mode: Option<c_int>, n: usize| {
        unsafe {
            (l.rand_seed)(0x3141_5926);
            let mut t = match table_mode {
                None => std::ptr::null_mut(),
                Some(m) => (l.shmode_func)(LS.elemsize, m),
            };
            let mut p = Pool::new();
            for i in 0..n {
                let k = p.add(&format!("free_{i}"));
                let v = (i as u32).to_ne_bytes();
                let (nt, _) = l.shput(t, LS, k, &v, STBDS_HM_STRING);
                t = nt;
            }
            l.hmfree(t, LS.elemsize);
            drop(p);
        }
    };
    for table_mode in [
        None,
        Some(STBDS_SH_NONE),
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        for n in [0usize, 1, 9, 40] {
            let oc = probe_abort(|| build_and_free(c, table_mode, n));
            let or = probe_abort(|| build_and_free(r, table_mode, n));
            assert_eq!(oc, or, "table_mode={table_mode:?} n={n}");
            assert_eq!(
                oc,
                ChildOutcome::Exited(0),
                "table_mode={table_mode:?} n={n} unexpected abort"
            );
        }
    }
    // an array with no hash table at all (built by arrgrowf)
    unsafe {
        for elemsize in [8usize, 16, 24] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            (*(ac as *mut ArrayHeader).sub(1)).length = 4;
            (*(ar as *mut ArrayHeader).sub(1)).length = 4;
            (c.hmfree_func)(ac, elemsize);
            (r.hmfree_func)(ar, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 44..50 — string arena
// ---------------------------------------------------------------------------

fn arena_check(c: &Lib, r: &Lib, strings: &[Vec<u8>], label: &str) {
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for (i, s) in strings.iter().enumerate() {
            let mut bc = s.clone();
            let mut br = s.clone();
            let pc = (c.stralloc)(&mut ac, bc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, br.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr), "{label} step={i}: contents differ");
            assert_eq!(
                cstr(pc),
                s[..s.len() - 1].to_vec(),
                "{label} step={i}: C returned the wrong string"
            );
            let sc = snapshot_arena(&ac);
            let sr = snapshot_arena(&ar);
            assert_eq!(sc, sr, "{label} step={i}: arena state differs");
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "{label} after reset");
        assert_eq!(snapshot_arena(&ac), snapshot_arena(&StringArena::new()));
    }
}

fn cs(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Row 44 — fresh arena, one short string.
#[test]
fn row_44_arena_single_short() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for s in ["", "a", "hello", &"x".repeat(511)] {
        arena_check(c, r, &[cs(s)], &format!("single len={}", s.len()));
    }
}

/// Row 45 — exhaust one block, then roll over to the next (and watch `block`
/// increment through the geometric schedule).
#[test]
fn row_45_arena_block_rollover() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    // 512-byte block; 64 x 8-byte strings exactly fills it, the 65th rolls over
    let many: Vec<Vec<u8>> = (0..400).map(|i| cs(&format!("{i:07}"))).collect();
    arena_check(c, r, &many, "rollover 8B");
    // strings sized right at the block boundary
    for len in [1usize, 2, 255, 256, 511, 512, 513] {
        let v: Vec<Vec<u8>> = (0..40).map(|i| cs(&format!("{:0width$}", i, width = len))).collect();
        arena_check(c, r, &v, &format!("boundary len={len}"));
    }
}

/// Row 46 — oversized first string (`len > blocksize`, `storage == NULL`).
#[test]
fn row_46_arena_oversized_first() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for len in [512usize, 513, 1024, 4096, 100_000] {
        arena_check(c, r, &[cs(&"q".repeat(len))], &format!("oversized first {len}"));
    }
}

/// Row 47 — oversized string when `storage != NULL`: spliced in after the head
/// block and `remaining` deliberately left alone.
#[test]
fn row_47_arena_oversized_after_head() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for big in [513usize, 2048, 70_000] {
        let mut v = vec![cs("short")];
        v.push(cs(&"Q".repeat(big)));
        v.push(cs("short2"));
        v.push(cs(&"R".repeat(big)));
        v.push(cs("short3"));
        arena_check(c, r, &v, &format!("oversized after head {big}"));
    }
}

/// Row 48 — drive `a->block` up to the 1 MiB saturation cap.
#[test]
fn row_48_arena_block_saturation() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // `block` is only bumped when a new block is needed, so force a new
        // block every time with a string bigger than `remaining`.
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for step in 0..48usize {
            // exactly the current blocksize forces "len > remaining" each time
            let s = cs(&"w".repeat(400));
            let mut bc = s.clone();
            let mut br = s.clone();
            // burn the remainder of the current block first
            loop {
                let rem = ac.remaining;
                assert_eq!(rem, ar.remaining, "step={step}");
                if rem < 401 {
                    break;
                }
                let f = cs(&"f".repeat(399));
                let mut fc = f.clone();
                let mut fr = f.clone();
                (c.stralloc)(&mut ac, fc.as_mut_ptr() as *mut c_char);
                (r.stralloc)(&mut ar, fr.as_mut_ptr() as *mut c_char);
            }
            let pc = (c.stralloc)(&mut ac, bc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, br.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr), "step={step}");
            assert_eq!(
                snapshot_arena(&ac),
                snapshot_arena(&ar),
                "step={step} block/remaining differ"
            );
        }
        // block must have saturated identically
        assert_eq!(ac.block, ar.block);
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar));
    }
}

/// Row 49 — randomised mixed-length arena traffic.
#[test]
fn row_49_arena_random_mixed() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 49);
    for trial in 0..8usize {
        let v: Vec<Vec<u8>> = (0..400)
            .map(|_| {
                let n = match rng.below(10) {
                    0 => 0,
                    1..=6 => rng.below(64),
                    7 | 8 => rng.below(600),
                    _ => rng.below(3000),
                };
                let mut b = rng.cstr_bytes(n);
                b.truncate(n + 1);
                b
            })
            .collect();
        arena_check(c, r, &v, &format!("random trial {trial}"));
    }
}

/// Row 50 — `strreset` on fresh / single-block / multi-block arenas, plus reuse
/// after reset.
#[test]
fn row_50_strreset_states() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // fresh
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar));

        for n in [1usize, 2, 100, 400] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            for i in 0..n {
                let s = cs(&format!("{:0width$}", i, width = 1 + i % 700));
                let mut bc = s.clone();
                let mut br = s.clone();
                (c.stralloc)(&mut ac, bc.as_mut_ptr() as *mut c_char);
                (r.stralloc)(&mut ar, br.as_mut_ptr() as *mut c_char);
            }
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "n={n} before reset");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "n={n} after reset");
            // reuse
            let s = cs("reused");
            let mut bc = s.clone();
            let mut br = s.clone();
            let pc = (c.stralloc)(&mut ac, bc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, br.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr));
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "n={n} after reuse");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 51..53 — strkey and str_dups
// ---------------------------------------------------------------------------

/// Row 51 — `strkey`.
#[test]
fn row_51_strkey() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 51);
    unsafe {
        let mut ns: Vec<c_int> = vec![
            0,
            1,
            -1,
            9,
            10,
            99,
            100,
            999,
            1000,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        for _ in 0..500 {
            ns.push(rng.next_u32() as c_int);
        }
        for n in ns {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_eq!(cstr(pc), cstr(pr), "n={n}");
            assert_eq!(
                String::from_utf8(cstr(pc)).unwrap(),
                format!("test_{n}"),
                "n={n}"
            );
        }
    }
}

/// Row 52 — `str_dups`: stdout compared byte-for-byte.
#[test]
fn row_52_str_dups_stdout() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for num in [
        0i32,
        1,
        2,
        3,
        7,
        8,
        9,
        100,
        1000,
        -1,
        -100,
        i32::MAX,
        i32::MIN,
    ] {
        // i32::MAX would allocate for ~2^31 iterations; clamp the huge positive
        // case to something that still crosses many arena blocks.
        let n = if num == i32::MAX { 5000 } else { num };
        let oc = capture_stdout(|| unsafe {
            (c.rand_seed)(0x3141_5926);
            (c.str_dups)(n);
        });
        let or = capture_stdout(|| unsafe {
            (r.rand_seed)(0x3141_5926);
            (r.str_dups)(n);
        });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or),
            "str_dups({n}) stdout differs"
        );
        assert_eq!(oc, or, "str_dups({n}) stdout differs (bytes)");
    }
}

/// Row 53 — `str_dups` called repeatedly, so the advancing global seed changes
/// each call's table layout.
#[test]
fn row_53_str_dups_repeated() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for start in [0usize, 1, 0x3141_5926, usize::MAX] {
        let oc = capture_stdout(|| unsafe {
            (c.rand_seed)(start);
            for i in 0..25 {
                (c.str_dups)(i);
            }
        });
        let or = capture_stdout(|| unsafe {
            (r.rand_seed)(start);
            for i in 0..25 {
                (r.str_dups)(i);
            }
        });
        assert_eq!(oc, or, "repeated str_dups (start={start:#x}) stdout differs");
        assert_eq!(oc.iter().filter(|&&b| b == b'\n').count(), 25);
    }
}

// ---------------------------------------------------------------------------
// Rows 54..56 — end-to-end pipelines
// ---------------------------------------------------------------------------

/// Row 54 / 55 — full `sh_new_*` → `shput` → `shget` → `shdel` → `shfree`
/// pipeline, comparing every intermediate result.
#[test]
fn row_54_55_string_pipeline() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for table_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            for seed in [0usize, 0x3141_5926, usize::MAX, 1] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut tc = (c.shmode_func)(LS.elemsize, table_mode);
                let mut tr = (r.shmode_func)(LS.elemsize, table_mode);
                let mut rng = Rng::new(SEED ^ 54 ^ table_mode as u64 ^ seed as u64);
                let mut pc = Pool::new();
                let mut pr = Pool::new();
                let n = 60;
                let names: Vec<String> = (0..n)
                    .map(|i| format!("path_{i}_{}", rng.below(1_000_000)))
                    .collect();
                for (i, nm) in names.iter().enumerate() {
                    let v = (i as u32).to_ne_bytes();
                    let (nc, a) = c.shput(tc, LS, pc.add(nm), &v, STBDS_HM_STRING);
                    let (nr, b) = r.shput(tr, LS, pr.add(nm), &v, STBDS_HM_STRING);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={table_mode} seed={seed:#x} put {i}");
                }
                for nm in names.iter() {
                    let (nc, a) = c.hmgeti(tc, LS, pc.add(nm) as *mut c_void, STBDS_HM_STRING);
                    let (nr, b) = r.hmgeti(tr, LS, pr.add(nm) as *mut c_void, STBDS_HM_STRING);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={table_mode} seed={seed:#x} get {nm}");
                    assert!(a >= 0);
                }
                for nm in names.iter().step_by(3) {
                    let (nc, a) = c.hmdel(tc, LS, pc.add(nm) as *mut c_void, 0, STBDS_HM_STRING);
                    let (nr, b) = r.hmdel(tr, LS, pr.add(nm) as *mut c_void, 0, STBDS_HM_STRING);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={table_mode} seed={seed:#x} del {nm}");
                    let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
                    let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
                    assert_eq!(sc, sr, "mode={table_mode} seed={seed:#x} after del {nm}");
                }
                let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
                let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
                assert_eq!(sc, sr, "mode={table_mode} seed={seed:#x} final");
                c.hmfree(tc, LS.elemsize);
                r.hmfree(tr, LS.elemsize);
                drop(pc);
                drop(pr);
            }
        }
    }
}

/// Row 56 — binary pipeline with `elemsize=24`, `keysize=8`, including
/// `hmput_default` and a full drain.
#[test]
fn row_56_binary_pipeline() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let lay = L24;
    unsafe {
        for seed in [0usize, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc = (c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            let mut tr = (r.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            let mut rng = Rng::new(SEED ^ 56 ^ seed as u64);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..120usize {
                let mut k = rng.bytes(8);
                let mut k2 = k.clone();
                let v = rng.bytes(lay.value_size);
                let (nc, a) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
                let (nr, b) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b);
                keys.push(k);
            }
            // drain everything
            for (i, k) in keys.iter().enumerate() {
                let mut a = k.clone();
                let mut b = k.clone();
                let (nc, x) = c.hmdel(tc, lay, a.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
                let (nr, y) = r.hmdel(tr, lay, b.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(x, y, "seed={seed:#x} drain {i}");
                let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
                let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
                assert_eq!(sc, sr, "seed={seed:#x} drain {i}");
            }
            assert_eq!(header_of(raw_of(tc, lay.elemsize)).length, 1);
            c.hmfree(tc, lay.elemsize);
            r.hmfree(tr, lay.elemsize);
        }
    }
}
