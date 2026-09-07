//! Phase B — valid-path differential tests, rows 15..34 of CONFIGS.md:
//! the binary-mode and string-mode `stbds_hmput_key` / `stbds_hmget_key` /
//! `stbds_hmget_key_ts` / `stbds_hmput_default` / `stbds_shmode_func` surface.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Layouts mirroring the shapes the C code and its test block actually use.
const L8: Layout = Layout::new(8, 4, 4, 4); // { int key; int value; }
const L16: Layout = Layout::new(16, 8, 8, 8); // { long key; long value; }
const L20: Layout = Layout::new(20, 8, 8, 12); // stbds_struct2 { int key[2]; int b,c,d; }
const L24: Layout = Layout::new(24, 16, 16, 8); // wider inline key
const L128: Layout = Layout::new(128, 128, 0, 0); // keysize == elemsize
const LK0: Layout = Layout::new(16, 0, 8, 8); // keysize == 0

/// Reset both libraries' global hash seed so their tables are laid out
/// identically, then run `f` on each.
fn with_seed<T, F: FnMut(&Lib) -> T>(c: &Lib, r: &Lib, seed: usize, mut f: F) -> (T, T) {
    unsafe {
        (c.rand_seed)(seed);
        let a = f(c);
        (r.rand_seed)(seed);
        let b = f(r);
        (a, b)
    }
}

// ---------------------------------------------------------------------------
// Rows 15..23 — binary mode
// ---------------------------------------------------------------------------

/// Drives `n` random inserts (some duplicated) plus lookups, in lockstep,
/// snapshotting after every single operation.
fn binary_insert_scenario(lay: Layout, n: usize, key_space: usize, seed: usize, rng_seed: u64) {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let mut rng = Rng::new(rng_seed);

        for step in 0..n {
            let kv = rng.below(key_space.max(1));
            let mut kc = vec![0u8; lay.keysize.max(1)];
            for (i, b) in kc.iter_mut().enumerate() {
                *b = ((kv >> (8 * (i % 8))) & 0xff) as u8;
            }
            let mut kr = kc.clone();
            let value = rng.bytes(lay.value_size.max(1));

            let (nc, itc) = c.hmput(tc, lay, &mut kc, &value, STBDS_HM_BINARY);
            let (nr, itr) = r.hmput(tr, lay, &mut kr, &value, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(itc, itr, "{lay:?} step={step} put temp differs");
            let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
            assert_eq!(sc, sr, "{lay:?} step={step} state differs after put(key={kv})");

            // interleaved lookup of a random key (hit or miss)
            let gv = rng.below(key_space.max(1) * 2);
            let mut gc = vec![0u8; lay.keysize.max(1)];
            for (i, b) in gc.iter_mut().enumerate() {
                *b = ((gv >> (8 * (i % 8))) & 0xff) as u8;
            }
            let mut gr = gc.clone();
            let (nc, gtc) = c.hmgeti(tc, lay, gc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            let (nr, gtr) = r.hmgeti(tr, lay, gr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(gtc, gtr, "{lay:?} step={step} get temp differs (key={gv})");
            let (nc, gtc) = c.hmgeti_ts(tc, lay, gc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            let (nr, gtr) = r.hmgeti_ts(tr, lay, gr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(gtc, gtr, "{lay:?} step={step} get_ts temp differs (key={gv})");
        }

        let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
        let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
        assert_eq!(sc, sr, "{lay:?} final state differs");
        c.hmfree(tc, lay.elemsize);
        r.hmfree(tr, lay.elemsize);
    }
}

/// Row 15 — single insert, self-bootstrapped table.
#[test]
fn row_15_binary_single_insert() {
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        binary_insert_scenario(L8, 1, 1, seed, SEED ^ 15);
    }
}

/// Row 16 — 7 inserts (stays inside the initial 8-slot table).
#[test]
fn row_16_binary_seven_inserts() {
    for seed in [0usize, 0x3141_5926, 0xABCD_1234_5678_9ABC] {
        binary_insert_scenario(L8, 7, 64, seed, SEED ^ 16);
    }
}

/// Row 17 — 9 inserts (crosses `used_count_threshold` = 6 → grow to 16).
#[test]
fn row_17_binary_nine_inserts() {
    for seed in [0usize, 1, 0x3141_5926, usize::MAX, 7] {
        binary_insert_scenario(L8, 9, 1024, seed, SEED ^ 17);
    }
}

/// Row 18 — 500 random inserts: repeated doubling 8→16→…→1024.
#[test]
fn row_18_binary_many_inserts() {
    binary_insert_scenario(L8, 500, 4096, 0x3141_5926, SEED ^ 18);
    binary_insert_scenario(L8, 500, 4096, 0, SEED ^ 0x18);
}

/// Row 19 — `elemsize=16`, `keysize=8` (64-bit keys).
#[test]
fn row_19_binary_elemsize16() {
    binary_insert_scenario(L16, 300, 2048, 0x3141_5926, SEED ^ 19);
    binary_insert_scenario(L16, 300, 64, usize::MAX, SEED ^ 0x19);
}

/// Row 20 — `stbds_struct2` shape: `elemsize=20`, `keysize=8`.
#[test]
fn row_20_binary_struct2_shape() {
    binary_insert_scenario(L20, 200, 512, 0x3141_5926, SEED ^ 20);
    binary_insert_scenario(L24, 200, 512, 0x3141_5926, SEED ^ 0x20);
}

/// Row 21 — `keysize == elemsize == 128`.
#[test]
fn row_21_binary_keysize_equals_elemsize() {
    binary_insert_scenario(L128, 80, 256, 0x3141_5926, SEED ^ 21);
}

/// Row 22 — `keysize == 0`: `memcmp(...,0)` makes every key compare equal, so
/// only the hash distinguishes entries.
#[test]
fn row_22_binary_keysize_zero() {
    binary_insert_scenario(LK0, 60, 64, 0x3141_5926, SEED ^ 22);
    binary_insert_scenario(LK0, 60, 1, 0, SEED ^ 0x22);
}

/// Row 23 — heavy duplicate-key traffic (the update path).
#[test]
fn row_23_binary_duplicate_keys() {
    binary_insert_scenario(L8, 400, 8, 0x3141_5926, SEED ^ 23);
    binary_insert_scenario(L16, 400, 3, 0x3141_5926, SEED ^ 0x23);
}

// ---------------------------------------------------------------------------
// Rows 24..29, 32 — string mode
// ---------------------------------------------------------------------------

/// Keeps caller-owned NUL-terminated key buffers alive for the whole scenario,
/// which `STBDS_SH_DEFAULT` requires (it stores the caller's pointer).
struct KeyPool {
    bufs: Vec<Vec<u8>>,
}

impl KeyPool {
    fn new() -> KeyPool {
        KeyPool { bufs: Vec::new() }
    }
    /// NUL-terminated, and at least `min_len` bytes long so that a `memcpy` of
    /// `keysize` bytes from it (the `default:` branch) stays in bounds.
    fn add(&mut self, s: &[u8], min_len: usize) -> *mut c_char {
        let mut v = s.to_vec();
        v.push(0);
        while v.len() < min_len {
            v.push(0);
        }
        self.bufs.push(v);
        self.bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char
    }
}

/// `table_mode`: `None` = let `hmput_key` bootstrap the table (→ SH_DEFAULT),
/// `Some(m)` = pre-create it with `shmode_func(elemsize, m)`.
fn string_insert_scenario(
    lay: Layout,
    table_mode: Option<c_int>,
    put_mode: c_int,
    n: usize,
    key_space: usize,
    seed: usize,
    rng_seed: u64,
    use_shputs: bool,
) {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
        let (mut tc, mut tr) = match table_mode {
            None => (std::ptr::null_mut(), std::ptr::null_mut()),
            Some(m) => (
                (c.shmode_func)(lay.elemsize, m),
                (r.shmode_func)(lay.elemsize, m),
            ),
        };
        let mut rng = Rng::new(rng_seed);
        let mut pool_c = KeyPool::new();
        let mut pool_r = KeyPool::new();

        for step in 0..n {
            // key_space == 0 means "all keys distinct" (use the step index).
            let kv = if key_space == 0 { step } else { rng.below(key_space) };
            let s = format!("key_{kv:04}_{}", "x".repeat(kv % 13));
            let kc = pool_c.add(s.as_bytes(), lay.keysize + 1);
            let kr = pool_r.add(s.as_bytes(), lay.keysize + 1);
            let value = rng.bytes(lay.value_size.max(1));

            let (nc, itc) = if use_shputs {
                c.shputs(tc, lay, kc, &value, put_mode)
            } else {
                c.shput(tc, lay, kc, &value, put_mode)
            };
            let (nr, itr) = if use_shputs {
                r.shputs(tr, lay, kr, &value, put_mode)
            } else {
                r.shput(tr, lay, kr, &value, put_mode)
            };
            tc = nc;
            tr = nr;
            assert_eq!(itc, itr, "step={step} put temp differs (key={s})");
            let sc = snapshot_map_lay(tc, lay, KeyRepr::Pointer);
            let sr = snapshot_map_lay(tr, lay, KeyRepr::Pointer);
            assert_eq!(
                sc, sr,
                "table_mode={table_mode:?} step={step} state differs after put({s})"
            );

            // interleaved lookup
            let gv = rng.below(key_space.max(1) * 2);
            let gs = format!("key_{gv:04}_{}", "x".repeat(gv % 13));
            let gkc = pool_c.add(gs.as_bytes(), lay.keysize + 1);
            let gkr = pool_r.add(gs.as_bytes(), lay.keysize + 1);
            let (nc, gtc) = c.hmgeti(tc, lay, gkc as *mut c_void, put_mode);
            let (nr, gtr) = r.hmgeti(tr, lay, gkr as *mut c_void, put_mode);
            tc = nc;
            tr = nr;
            assert_eq!(gtc, gtr, "step={step} shgeti temp differs (key={gs})");
            let (nc, gtc) = c.hmgeti_ts(tc, lay, gkc as *mut c_void, put_mode);
            let (nr, gtr) = r.hmgeti_ts(tr, lay, gkr as *mut c_void, put_mode);
            tc = nc;
            tr = nr;
            assert_eq!(gtc, gtr, "step={step} shgeti_ts temp differs (key={gs})");
        }

        let sc = snapshot_map_lay(tc, lay, KeyRepr::Pointer);
        let sr = snapshot_map_lay(tr, lay, KeyRepr::Pointer);
        assert_eq!(sc, sr, "final state differs");
        c.hmfree(tc, lay.elemsize);
        r.hmfree(tr, lay.elemsize);
        drop(pool_c);
        drop(pool_r);
    }
}

/// `{ char *key; int value; }` — the layout `str_dups` uses.
const LS: Layout = Layout::new(16, 8, 8, 4);

/// Row 24 — string mode with the table auto-created by `hmput_key`
/// (`string.mode` becomes `SH_DEFAULT`).
#[test]
fn row_24_string_autocreated_table() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        string_insert_scenario(LS, None, STBDS_HM_STRING, 120, 64, seed, SEED ^ 24, false);
    }
}

/// Row 25 — `SH_STRDUP`.
#[test]
fn row_25_string_strdup() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        string_insert_scenario(
            LS,
            Some(STBDS_SH_STRDUP),
            STBDS_HM_STRING,
            200,
            96,
            seed,
            SEED ^ 25,
            false,
        );
    }
    // and via shputs, which re-reads stbds_temp_key. Distinct keys only: with
    // duplicates the C code hits its stale-`stbds_temp_key` bug in the wrapped
    // probe scan and double-frees. That is genuine C behaviour and is compared
    // separately, in forked children, by
    // `phase_c_errors::err_30b_shputs_stale_temp_key_double_free`.
    string_insert_scenario(
        LS,
        Some(STBDS_SH_STRDUP),
        STBDS_HM_STRING,
        200,
        0,
        0x3141_5926,
        SEED ^ 0x25,
        true,
    );
}

/// Row 26 — `SH_ARENA`, including keys longer than the 512-byte first block.
#[test]
fn row_26_string_arena() {
    for seed in [0usize, 0x3141_5926] {
        string_insert_scenario(
            LS,
            Some(STBDS_SH_ARENA),
            STBDS_HM_STRING,
            200,
            96,
            seed,
            SEED ^ 26,
            false,
        );
    }
    // long keys to force oversized arena blocks
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc = (c.shmode_func)(LS.elemsize, STBDS_SH_ARENA);
        let mut tr = (r.shmode_func)(LS.elemsize, STBDS_SH_ARENA);
        let mut rng = Rng::new(SEED ^ 0x26);
        let mut pc = KeyPool::new();
        let mut pr = KeyPool::new();
        for step in 0..120usize {
            let n = [3usize, 40, 500, 511, 512, 513, 1024, 4096][rng.below(8)];
            let s = format!("{:0width$}", step, width = n);
            let kc = pc.add(s.as_bytes(), LS.keysize + 1);
            let kr = pr.add(s.as_bytes(), LS.keysize + 1);
            let v = rng.bytes(4);
            let (nc, itc) = c.shput(tc, LS, kc, &v, STBDS_HM_STRING);
            let (nr, itr) = r.shput(tr, LS, kr, &v, STBDS_HM_STRING);
            tc = nc;
            tr = nr;
            assert_eq!(itc, itr, "step={step}");
            let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
            let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
            assert_eq!(sc, sr, "arena long-key step={step} len={n}");
        }
        c.hmfree(tc, LS.elemsize);
        r.hmfree(tr, LS.elemsize);
    }
}

/// Row 27 — `SH_DEFAULT` (caller-owned key pointers).
#[test]
fn row_27_string_sh_default() {
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        string_insert_scenario(
            LS,
            Some(STBDS_SH_DEFAULT),
            STBDS_HM_STRING,
            200,
            96,
            seed,
            SEED ^ 27,
            false,
        );
    }
}

/// Row 28 — `SH_NONE` table driven with `mode = STBDS_HM_STRING`: the switch
/// falls to `default:` and `memcpy`s `keysize` bytes out of the *string*, while
/// hashing/comparison still uses the string path. Inserts of distinct keys are
/// well defined; a lookup would dereference string bytes as a pointer, so only
/// the insert path is driven here.
#[test]
fn row_28_string_mode_over_sh_none_table() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for seed in [0usize, 0x3141_5926] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc = (c.shmode_func)(LS.elemsize, STBDS_SH_NONE);
            let mut tr = (r.shmode_func)(LS.elemsize, STBDS_SH_NONE);
            let mut pc = KeyPool::new();
            let mut pr = KeyPool::new();
            for step in 0..5usize {
                let s = format!("distinct_key_number_{step:08}");
                let kc = pc.add(s.as_bytes(), LS.keysize + 1);
                let kr = pr.add(s.as_bytes(), LS.keysize + 1);
                let v = (step as u32).to_ne_bytes();
                let (nc, itc) = c.shput(tc, LS, kc, &v, STBDS_HM_STRING);
                let (nr, itr) = r.shput(tr, LS, kr, &v, STBDS_HM_STRING);
                tc = nc;
                tr = nr;
                assert_eq!(itc, itr, "seed={seed:#x} step={step}");
                // key bytes are now INLINE string bytes, so compare raw.
                let sc = snapshot_map_lay(tc, LS, KeyRepr::Inline);
                let sr = snapshot_map_lay(tr, LS, KeyRepr::Inline);
                assert_eq!(sc, sr, "seed={seed:#x} step={step}");
            }
            // hmfree_func with string.mode == SH_NONE must not free key pointers
            c.hmfree(tc, LS.elemsize);
            r.hmfree(tr, LS.elemsize);
        }
    }
}

/// Row 29 — `mode = 2` (`STBDS_HM_PTR_TO_STRING`): `>= STBDS_HM_STRING` is
/// true (string hashing/compare) but `== STBDS_HM_STRING` is false, so
/// `hmdel_key` skips the strdup free.
#[test]
fn row_29_mode_ptr_to_string() {
    for tm in [
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
        None,
    ] {
        string_insert_scenario(LS, tm, STBDS_HM_PTR_TO_STRING, 150, 64, 0x3141_5926, SEED ^ 29, false);
    }
}

// ---------------------------------------------------------------------------
// Rows 30..32 — get-only paths across the layout cross product
// ---------------------------------------------------------------------------

/// Row 30 — `hmget_key` hits and misses across every binary layout.
#[test]
fn row_30_binary_get_cross_product() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 30);
    unsafe {
        for lay in [L8, L16, L20, L24, L128, LK0] {
            for seed in [0usize, 0x3141_5926, usize::MAX] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut tc: *mut c_void = std::ptr::null_mut();
                let mut tr: *mut c_void = std::ptr::null_mut();
                let mut inserted = Vec::new();
                for _ in 0..40usize {
                    let mut k = rng.bytes(lay.keysize.max(1));
                    let mut k2 = k.clone();
                    let v = rng.bytes(lay.value_size.max(1));
                    let (nc, _) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
                    let (nr, _) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
                    tc = nc;
                    tr = nr;
                    inserted.push(k);
                }
                // hits
                for (i, k) in inserted.iter().enumerate() {
                    let mut kc = k.clone();
                    let mut kr = k.clone();
                    let (nc, a) = c.hmgeti(tc, lay, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                    let (nr, b) = r.hmgeti(tr, lay, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "{lay:?} seed={seed:#x} hit {i}");
                }
                // misses
                for i in 0..40usize {
                    let mut kc = rng.bytes(lay.keysize.max(1));
                    let mut kr = kc.clone();
                    let (nc, a) = c.hmgeti(tc, lay, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                    let (nr, b) = r.hmgeti(tr, lay, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "{lay:?} seed={seed:#x} miss {i}");
                }
                let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
                let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
                assert_eq!(sc, sr, "{lay:?} seed={seed:#x}");
                c.hmfree(tc, lay.elemsize);
                r.hmfree(tr, lay.elemsize);
            }
        }
    }
}

/// Row 31 — `hmget_key_ts` used as the primary entry point (its `temp`
/// out-parameter, and the fact it does NOT write the header's `temp`).
#[test]
fn row_31_hmget_key_ts_out_param() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 31);
    unsafe {
        let lay = L16;
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let mut keys = Vec::new();
        for _ in 0..64usize {
            let mut k = rng.bytes(8);
            let mut k2 = k.clone();
            let v = rng.bytes(8);
            let (nc, _) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            keys.push(k);
        }
        // sentinel in the header temp, then _ts lookups must leave it alone
        (*(raw_of(tc, lay.elemsize) as *mut ArrayHeader).sub(1)).temp = 0x7777;
        (*(raw_of(tr, lay.elemsize) as *mut ArrayHeader).sub(1)).temp = 0x7777;
        for k in keys.iter() {
            let mut kc = k.clone();
            let mut kr = k.clone();
            let (nc, a) = c.hmgeti_ts(tc, lay, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            let (nr, b) = r.hmgeti_ts(tr, lay, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, b);
            assert_eq!(header_of(raw_of(tc, lay.elemsize)).temp, 0x7777);
            assert_eq!(header_of(raw_of(tr, lay.elemsize)).temp, 0x7777);
        }
        c.hmfree(tc, lay.elemsize);
        r.hmfree(tr, lay.elemsize);
    }
}

/// Row 32 — string-mode `hmget_key` / `_ts` over each table mode.
#[test]
fn row_32_string_get_across_table_modes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for tm in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            let mut rng = Rng::new(SEED ^ 32 ^ tm as u64);
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let mut tc = (c.shmode_func)(LS.elemsize, tm);
            let mut tr = (r.shmode_func)(LS.elemsize, tm);
            let mut pc = KeyPool::new();
            let mut pr = KeyPool::new();
            let mut names = Vec::new();
            for i in 0..80usize {
                let s = format!("sym_{i}_{}", rng.below(1000));
                let kc = pc.add(s.as_bytes(), LS.keysize + 1);
                let kr = pr.add(s.as_bytes(), LS.keysize + 1);
                let v = rng.bytes(4);
                let (nc, _) = c.shput(tc, LS, kc, &v, STBDS_HM_STRING);
                let (nr, _) = r.shput(tr, LS, kr, &v, STBDS_HM_STRING);
                tc = nc;
                tr = nr;
                names.push(s);
            }
            for s in names.iter() {
                let kc = pc.add(s.as_bytes(), LS.keysize + 1);
                let kr = pr.add(s.as_bytes(), LS.keysize + 1);
                let (nc, a) = c.hmgeti(tc, LS, kc as *mut c_void, STBDS_HM_STRING);
                let (nr, b) = r.hmgeti(tr, LS, kr as *mut c_void, STBDS_HM_STRING);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={tm} hit {s}");
                let (nc, a) = c.hmgeti_ts(tc, LS, kc as *mut c_void, STBDS_HM_STRING);
                let (nr, b) = r.hmgeti_ts(tr, LS, kr as *mut c_void, STBDS_HM_STRING);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={tm} hit_ts {s}");
            }
            for i in 0..80usize {
                let s = format!("absent_{i}");
                let kc = pc.add(s.as_bytes(), LS.keysize + 1);
                let kr = pr.add(s.as_bytes(), LS.keysize + 1);
                let (nc, a) = c.hmgeti(tc, LS, kc as *mut c_void, STBDS_HM_STRING);
                let (nr, b) = r.hmgeti(tr, LS, kr as *mut c_void, STBDS_HM_STRING);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={tm} miss {s}");
            }
            let sc = snapshot_map_lay(tc, LS, KeyRepr::Pointer);
            let sr = snapshot_map_lay(tr, LS, KeyRepr::Pointer);
            assert_eq!(sc, sr, "mode={tm}");
            c.hmfree(tc, LS.elemsize);
            r.hmfree(tr, LS.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 33..34 — stbds_hmput_default
// ---------------------------------------------------------------------------

/// Row 33 — the three `hmput_default` paths.
#[test]
fn row_33_hmput_default_paths() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [8usize, 16, 20, 24, 128] {
            // (a) a == NULL
            let (tc, tr) = with_seed(c, r, 0x3141_5926, |l| {
                (l.hmput_default)(std::ptr::null_mut(), elemsize)
            });
            let sc = snapshot_map(tc, elemsize, KeyRepr::Inline);
            let sr = snapshot_map(tr, elemsize, KeyRepr::Inline);
            assert_eq!(sc, sr, "elemsize={elemsize} path a");
            assert_eq!(sc.length, 1);

            // (b) a != NULL, length > 0 -> no-op
            let tc2 = (c.hmput_default)(tc, elemsize);
            let tr2 = (r.hmput_default)(tr, elemsize);
            assert_eq!(tc2, tc, "C: hmput_default should be a no-op here");
            assert_eq!(tr2, tr, "Rust: hmput_default should be a no-op here");
            let sc = snapshot_map(tc2, elemsize, KeyRepr::Inline);
            let sr = snapshot_map(tr2, elemsize, KeyRepr::Inline);
            assert_eq!(sc, sr, "elemsize={elemsize} path b");

            // (c) a != NULL but length == 0
            (*(raw_of(tc2, elemsize) as *mut ArrayHeader).sub(1)).length = 0;
            (*(raw_of(tr2, elemsize) as *mut ArrayHeader).sub(1)).length = 0;
            let tc3 = (c.hmput_default)(tc2, elemsize);
            let tr3 = (r.hmput_default)(tr2, elemsize);
            let sc = snapshot_map(tc3, elemsize, KeyRepr::Inline);
            let sr = snapshot_map(tr3, elemsize, KeyRepr::Inline);
            assert_eq!(sc, sr, "elemsize={elemsize} path c");
            assert_eq!(sc.length, 1);

            c.hmfree(tc3, elemsize);
            r.hmfree(tr3, elemsize);
        }
    }
}

/// Row 34 — `hmput_default` followed by random inserts (the default element
/// stays at raw index 0 and must survive rehashing).
#[test]
fn row_34_hmput_default_then_inserts() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 34);
    unsafe {
        let lay = L16;
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc = (c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
        let mut tr = (r.hmput_default)(std::ptr::null_mut(), lay.elemsize);
        // hmdefault(t, v): (t)[-1].value = v
        let dv: u64 = 0xFEED_FACE_DEAD_BEEF;
        std::ptr::copy_nonoverlapping(
            dv.to_ne_bytes().as_ptr(),
            (tc as *mut u8).sub(lay.elemsize).add(lay.value_offset),
            8,
        );
        std::ptr::copy_nonoverlapping(
            dv.to_ne_bytes().as_ptr(),
            (tr as *mut u8).sub(lay.elemsize).add(lay.value_offset),
            8,
        );
        for step in 0..300usize {
            let mut k = rng.bytes(8);
            let mut k2 = k.clone();
            let v = rng.bytes(8);
            let (nc, a) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
            let (nr, b) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, b, "step={step}");
            let sc = snapshot_map_lay(tc, lay, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, lay, KeyRepr::Inline);
            assert_eq!(sc, sr, "step={step}");
        }
        c.hmfree(tc, lay.elemsize);
        r.hmfree(tr, lay.elemsize);
    }
}

// Referenced to keep the c_char import used in every configuration.
const _: usize = std::mem::size_of::<*mut c_char>();
