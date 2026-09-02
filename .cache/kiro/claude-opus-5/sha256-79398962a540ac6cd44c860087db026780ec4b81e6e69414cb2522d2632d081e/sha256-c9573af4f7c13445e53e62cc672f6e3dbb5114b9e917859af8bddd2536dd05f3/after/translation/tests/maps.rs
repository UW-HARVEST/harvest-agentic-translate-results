//! Phase B — differential tests for the hash-map entry points.
//!
//! Covers CONFIGS.md rows C15–C47 and C54–C60. Every operation is executed on
//! BOTH `.so`s and, after each one, the full observable state is diffed: the
//! array header (`length`/`capacity`/`temp`), every field of the internal
//! `stbds_hash_index`, every `stbds_hash_bucket`'s `hash[8]`/`index[8]`, the
//! initialised bytes of every element, and the key strings.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Key material.  `Box<[u8]>` keeps every buffer at a stable address, which
// matters because `STBDS_SH_DEFAULT` stores the caller's pointer verbatim.
// ---------------------------------------------------------------------------

struct Keys {
    bufs: Vec<Box<[u8]>>,
}

impl Keys {
    /// `n` distinct binary keys of exactly `keysize` bytes (plus a trailing NUL
    /// so the buffer is also usable as a C string).
    fn binary(rng: &mut Rng, n: usize, keysize: usize) -> Keys {
        let mut seen = std::collections::HashSet::new();
        let mut bufs = Vec::new();
        let mut guard = 0usize;
        while bufs.len() < n {
            guard += 1;
            assert!(guard < 100_000, "cannot find {n} distinct {keysize}-byte keys");
            let mut v = rng.bytes(keysize);
            // Stamp a counter into the low bytes to guarantee progress.
            let c = bufs.len() as u64;
            for i in 0..keysize.min(4) {
                v[i] = (c >> (8 * i)) as u8;
            }
            if seen.insert(v.clone()) {
                v.push(0);
                bufs.push(v.into_boxed_slice());
            }
        }
        Keys { bufs }
    }

    /// `n` distinct NUL-terminated strings.
    fn strings(rng: &mut Rng, n: usize, max_extra: usize) -> Keys {
        let mut bufs = Vec::new();
        for i in 0..n {
            let mut v = format!("k{i}_").into_bytes();
            let extra = rng.below(max_extra + 1);
            for _ in 0..extra {
                v.push(b'a' + (rng.next_u64() % 26) as u8);
            }
            v.push(0);
            bufs.push(v.into_boxed_slice());
        }
        Keys { bufs }
    }

    fn ptr(&mut self, i: usize) -> *mut u8 {
        self.bufs[i].as_mut_ptr()
    }
    fn len(&self) -> usize {
        self.bufs.len()
    }
}

// ---------------------------------------------------------------------------
// Generic pipeline driver
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
struct Cfg {
    elemsize: usize,
    keysize: usize,
    /// `Some(m)` -> call `stbds_shmode_func(elemsize, m)` first
    shmode: Option<c_int>,
    /// the `int mode` handed to hmput/hmget/hmdel
    mode: c_int,
    /// keys are C strings addressed by pointer (SH_DEFAULT/STRDUP/ARENA)
    ptr_keys: bool,
}

impl Cfg {
    fn keyrepr(&self) -> KeyRepr {
        if self.ptr_keys {
            KeyRepr::Pointer
        } else {
            KeyRepr::Inline(self.keysize)
        }
    }
    fn value_range(&self) -> std::ops::Range<usize> {
        let start = if self.ptr_keys { 8 } else { self.keysize };
        start.min(self.elemsize)..self.elemsize
    }
}

/// Full end-to-end run: shmode -> puts -> gets (hit+miss) -> get_ts -> deletes
/// -> re-puts -> gets -> free.  Every step is diffed C vs Rust.
///
/// `mode > STBDS_HM_STRING` forces reverse-order deletion: `stbds_hmdel_key`
/// only takes its `*(char**)` key-reload branch when `mode == STBDS_HM_STRING`,
/// so for `mode == 2` the post-move re-lookup hashes the raw pointer bytes,
/// fails, and trips the live `STBDS_ASSERT(slot >= 0)` (ERRORS.md E18).
/// Deleting from the tail keeps `old_index == final_index`, which skips that
/// block entirely.  The abort itself is covered by `tests/errors.rs`.
unsafe fn pipeline(s: &Session, cfg: Cfg, n: usize, rng: &mut Rng, tag: &str) {
    let tail_only_delete = cfg.mode > HM_STRING;
    let mut p = Pair::new(s, cfg.elemsize, cfg.keysize, cfg.keyrepr(), cfg.value_range());
    if let Some(m) = cfg.shmode {
        p.shmode(m);
    }

    let mut keys = if cfg.ptr_keys {
        Keys::strings(rng, n, 24)
    } else {
        Keys::binary(rng, n, cfg.keysize)
    };
    let mut absent = if cfg.ptr_keys {
        Keys::strings(rng, 8, 40)
    } else {
        Keys::binary(rng, 8, cfg.keysize)
    };
    // Make the "absent" set genuinely disjoint.
    for b in absent.bufs.iter_mut() {
        b[0] ^= 0xA5;
        if cfg.ptr_keys && b[0] == 0 {
            b[0] = 0x5A;
        }
    }

    // --- puts -----------------------------------------------------------
    for i in 0..keys.len() {
        let v = [(i as u8).wrapping_mul(31), (i >> 8) as u8, 0xC3, 0x5A];
        let (idx, _) = p.put(keys.ptr(i), cfg.mode, &v);
        assert_eq!(idx, i as isize, "{tag}: put #{i} landed at {idx}");
        assert_eq!(p.cm.len(), (i + 1) as isize, "{tag}: hmlen after put #{i}");
    }

    // --- lookups --------------------------------------------------------
    for i in 0..keys.len() {
        let got = p.get(keys.ptr(i), cfg.mode);
        assert_eq!(got, i as isize, "{tag}: get of key #{i}");
        let got_ts = p.get_ts(keys.ptr(i), cfg.mode);
        assert_eq!(got_ts, i as isize, "{tag}: get_ts of key #{i}");
    }
    for i in 0..absent.len() {
        assert_eq!(
            p.get(absent.ptr(i), cfg.mode),
            -1,
            "{tag}: miss for absent key #{i}"
        );
        assert_eq!(p.get_ts(absent.ptr(i), cfg.mode), -1, "{tag}: miss _ts");
    }

    // --- re-put (existing-key branch) -----------------------------------
    for i in (0..keys.len()).step_by(3) {
        let v = [0x11u8, (i as u8), 0x22, 0x33];
        let (idx, _) = p.put(keys.ptr(i), cfg.mode, &v);
        assert_eq!(idx, i as isize, "{tag}: re-put of key #{i}");
        assert_eq!(p.cm.len(), keys.len() as isize, "{tag}: hmlen after re-put");
    }

    // --- deletes in randomised order ------------------------------------
    let mut order: Vec<usize> = (0..keys.len()).collect();
    if tail_only_delete {
        order.reverse();
    } else {
        for i in (1..order.len()).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
    }
    // absent deletes first: must report 0 and leave the map alone
    for i in 0..absent.len() {
        assert_eq!(
            p.del(absent.ptr(i), cfg.mode, 0),
            0,
            "{tag}: delete of absent key must report 0"
        );
    }
    let mut alive = keys.len();
    for (step, &k) in order.iter().enumerate() {
        let flag = p.del(keys.ptr(k), cfg.mode, 0);
        assert_eq!(flag, 1, "{tag}: delete #{step} (key {k}) must report 1");
        alive -= 1;
        assert_eq!(p.cm.len(), alive as isize, "{tag}: hmlen after delete #{step}");
        // deleting again must now report 0
        assert_eq!(
            p.del(keys.ptr(k), cfg.mode, 0),
            0,
            "{tag}: second delete of key {k}"
        );
    }

    // --- re-insert into the tombstoned table ----------------------------
    for i in 0..keys.len().min(16) {
        let v = [0xAAu8, 0xBB, i as u8, 0xDD];
        p.put(keys.ptr(i), cfg.mode, &v);
    }
    for i in 0..keys.len().min(16) {
        assert_eq!(p.get(keys.ptr(i), cfg.mode), i as isize, "{tag}: re-get #{i}");
    }

    p.free();
}

// ===========================================================================
// C15–C17 / E13  stbds_hmput_default
// ===========================================================================

#[test]
fn c15_c17_hmput_default() {
    let s = session(0x31415926);
    for elemsize in [8usize, 12, 16, 24, 40] {
        unsafe {
            // C15 / E13: from NULL
            let mut p = Pair::new(&s, elemsize, 8, KeyRepr::Inline(8), 8..elemsize);
            p.hmput_default();
            assert_eq!(p.cm.len(), 0, "hmlen must still be 0");
            // C17: twice
            p.hmput_default();
            assert_eq!(p.cm.len(), 0);
            p.free();

            // C16: on a shmode_func table (length already 1) -> no-op
            for m in [SH_DEFAULT, SH_STRDUP, SH_ARENA, SH_NONE] {
                let mut p = Pair::new(&s, elemsize, 8, KeyRepr::Inline(8), 8..elemsize);
                p.shmode(m);
                let before = (p.cm.t, p.rm.t);
                p.hmput_default();
                assert_eq!(
                    (p.cm.t, p.rm.t),
                    before,
                    "hmput_default on a length-1 array must be a no-op"
                );
                p.hmput_default();
                p.free();
            }
        }
    }
}

// ===========================================================================
// C18–C20  implicit binary mode, every count and key width
// ===========================================================================

#[test]
fn c18_binary_counts() {
    let mut rng = Rng::new(0x1801_0000);
    for &n in &[0usize, 1, 2, 5, 6, 7, 8, 9, 12, 13, 24, 25, 48, 100, 500] {
        let s = session(0x31415926 ^ n);
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: None,
            mode: HM_BINARY,
            ptr_keys: false,
        };
        unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C18 n={n}")) };
    }
}

#[test]
fn c19_binary_key_widths() {
    let mut rng = Rng::new(0x1901_0000);
    for &keysize in &[1usize, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 24, 33] {
        for &n in &[1usize, 7, 8, 9, 50] {
            if keysize == 1 && n > 50 {
                continue;
            }
            let elemsize = keysize + 8;
            let s = session(0xABCD_0000 ^ (keysize * 131) ^ n);
            let cfg = Cfg {
                elemsize,
                keysize,
                shmode: None,
                mode: HM_BINARY,
                ptr_keys: false,
            };
            unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C19 ks={keysize} n={n}")) };
        }
    }
}

#[test]
fn c20_binary_elemsize_gt_keysize() {
    let mut rng = Rng::new(0x2001_0000);
    for &(elemsize, keysize) in &[
        (8usize, 8usize),
        (12, 4),
        (16, 8),
        (24, 8),
        (32, 16),
        (40, 33),
        (40, 8),
    ] {
        let s = session(0x777 ^ elemsize ^ (keysize << 8));
        let cfg = Cfg {
            elemsize,
            keysize,
            shmode: None,
            mode: HM_BINARY,
            ptr_keys: false,
        };
        unsafe {
            pipeline(
                &s,
                cfg,
                40,
                &mut rng,
                &format!("C20 es={elemsize} ks={keysize}"),
            )
        };
    }
}

// ===========================================================================
// C21–C23 / E37  out-of-range `int mode`
// ===========================================================================

/// C21: `mode = 1` with no `shmode_func` -> l.706 installs `SH_DEFAULT`.
#[test]
fn c21_implicit_string_mode() {
    let mut rng = Rng::new(0x2101_0000);
    for &n in &[1usize, 6, 7, 12, 13, 60] {
        let s = session(0x2121 ^ n);
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: None,
            mode: HM_STRING,
            ptr_keys: true,
        };
        unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C21 n={n}")) };
        // The implicit mode must be SH_DEFAULT on both sides.
        let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
        let mut k = Keys::strings(&mut rng, 1, 4);
        unsafe {
            p.put(k.ptr(0), HM_STRING, &[1, 2, 3, 4]);
            assert_eq!((*p.cm.table()).string.mode, SH_DEFAULT as u8);
            assert_eq!((*p.rm.table()).string.mode, SH_DEFAULT as u8);
            p.free();
        }
    }
}

/// C22 / E37: `mode` values > 1 take the string branch but are `!= HM_STRING`.
#[test]
fn c22_mode_above_one() {
    let mut rng = Rng::new(0x2201_0000);
    for &mode in &[2 as c_int, 7, 1000, c_int::MAX] {
        for &shmode in &[None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            let s = session(0x2222 ^ (mode as usize));
            let cfg = Cfg {
                elemsize: 16,
                keysize: 8,
                shmode,
                mode,
                ptr_keys: true,
            };
            unsafe {
                pipeline(
                    &s,
                    cfg,
                    30,
                    &mut rng,
                    &format!("C22 mode={mode} shmode={shmode:?}"),
                )
            };
        }
    }
}

/// C23 / E37: negative `mode` takes the binary branch.
#[test]
fn c23_negative_mode() {
    let mut rng = Rng::new(0x2301_0000);
    for &mode in &[-1 as c_int, -2, -1000, c_int::MIN] {
        let s = session(0x2323 ^ (mode as usize));
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: None,
            mode,
            ptr_keys: false,
        };
        unsafe { pipeline(&s, cfg, 30, &mut rng, &format!("C23 mode={mode}")) };
    }
}

// ===========================================================================
// C24–C27  explicit shmodes
// ===========================================================================

#[test]
fn c24_sh_strdup() {
    let mut rng = Rng::new(0x2401_0000);
    for &n in &[0usize, 1, 5, 6, 7, 12, 13, 24, 25, 48, 100, 300] {
        let s = session(0x2424 ^ n);
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: Some(SH_STRDUP),
            mode: HM_STRING,
            ptr_keys: true,
        };
        unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C24 strdup n={n}")) };
    }
}

#[test]
fn c25_sh_arena() {
    let mut rng = Rng::new(0x2501_0000);
    for &n in &[0usize, 1, 6, 7, 13, 25, 48, 100, 300] {
        let s = session(0x2525 ^ n);
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: Some(SH_ARENA),
            mode: HM_STRING,
            ptr_keys: true,
        };
        unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C25 arena n={n}")) };
    }
    // long keys force the arena's dedicated-block path
    let s = session(0x2526);
    let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
    unsafe {
        p.shmode(SH_ARENA);
        let mut long_keys: Vec<Box<[u8]>> = Vec::new();
        for i in 0..30usize {
            let mut v = format!("L{i}_").into_bytes();
            v.resize(v.len() + 200 + i * 60, b'q');
            v.push(0);
            long_keys.push(v.into_boxed_slice());
        }
        for (i, k) in long_keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), HM_STRING, &[i as u8, 0, 0, 0]);
        }
        for (i, k) in long_keys.iter_mut().enumerate() {
            assert_eq!(p.get(k.as_mut_ptr(), HM_STRING), i as isize);
        }
        p.free();
    }
}

#[test]
fn c26_sh_default() {
    let mut rng = Rng::new(0x2601_0000);
    for &n in &[0usize, 1, 6, 7, 13, 25, 48, 100, 300] {
        let s = session(0x2626 ^ n);
        let cfg = Cfg {
            elemsize: 16,
            keysize: 8,
            shmode: Some(SH_DEFAULT),
            mode: HM_STRING,
            ptr_keys: true,
        };
        unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C26 default n={n}")) };
    }
}

/// C27 / E38: `shmode_func(_, SH_NONE)` -> the `switch` `default:` arm stores
/// keys with `memcpy`, i.e. binary storage.  Driven with `mode = 0` so the
/// lookups agree with the storage (a `mode >= 1` lookup would reinterpret the
/// copied key bytes as a `char *` — see `c28b`).
#[test]
fn c27_sh_none_binary() {
    let mut rng = Rng::new(0x2701_0000);
    for &keysize in &[4usize, 8, 16] {
        for &n in &[1usize, 7, 13, 40] {
            let s = session(0x2727 ^ keysize ^ n);
            let cfg = Cfg {
                elemsize: keysize + 8,
                keysize,
                shmode: Some(SH_NONE),
                mode: HM_BINARY,
                ptr_keys: false,
            };
            unsafe { pipeline(&s, cfg, n, &mut rng, &format!("C27 ks={keysize} n={n}")) };
        }
    }
}

/// C28 / E38 (a): out-of-range shmode values.  `stbds_shmode_func` stores
/// `(unsigned char) mode`, so values whose low byte lands in `1..=3` alias the
/// named SH_* modes (e.g. `257 -> SH_DEFAULT`); everything else hits the
/// `switch` `default:` arm and degrades to binary key storage.
#[test]
fn c28a_out_of_range_shmode_binary() {
    let mut rng = Rng::new(0x2801_0000);
    for &m in &[
        4 as c_int,
        5,
        255,
        256,
        257,
        258,
        259,
        260,
        -1,
        -256,
        1000,
        c_int::MAX,
        c_int::MIN,
    ] {
        let s = session(0x2828 ^ (m as usize));
        let low = m as u8;
        let aliases_named_mode = (1..=3).contains(&low);
        let cfg = if aliases_named_mode {
            Cfg {
                elemsize: 16,
                keysize: 8,
                shmode: Some(m),
                mode: HM_STRING,
                ptr_keys: true,
            }
        } else {
            Cfg {
                elemsize: 16,
                keysize: 8,
                shmode: Some(m),
                mode: HM_BINARY,
                ptr_keys: false,
            }
        };
        unsafe {
            // The stored mode is `(unsigned char) m` in both implementations.
            let mut probe = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            probe.shmode(m);
            assert_eq!(
                (*probe.cm.table()).string.mode,
                low,
                "C must store (unsigned char)mode"
            );
            assert_eq!((*probe.rm.table()).string.mode, low);
            probe.free();
            pipeline(
                &s,
                cfg,
                25,
                &mut rng,
                &format!("C28a shmode={m} (low={low})"),
            );
        }
    }
}

/// C28 / E38 (b): out-of-range shmode with `mode >= HM_STRING`.  A single
/// insert only: `default:` memcpy's the first `keysize` bytes of the *string*
/// into the element, so any later string lookup would dereference those bytes
/// as a pointer (genuine C UB that would crash both libraries).  The insert
/// itself is well-defined and must match byte-for-byte.
#[test]
fn c28b_out_of_range_shmode_string_single_insert() {
    for &m in &[0 as c_int, 4, 255, 256, -1, c_int::MAX] {
        let s = session(0x28B ^ (m as usize));
        unsafe {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            p.shmode(m);
            // The `default:` arm memcpy's `keysize` bytes *from the string*, so
            // the key must be at least `keysize` bytes long or the C library
            // reads past it.
            let mut kbuf: Vec<u8> = format!("key-for-{m}-padded").into_bytes();
            kbuf.push(0);
            let kbuf = kbuf.into_boxed_slice();
            let mut kbuf = kbuf;
            p.put(kbuf.as_mut_ptr(), HM_STRING, &[9, 8, 7, 6]);
            // The element must hold the first 8 bytes of the key string.
            let ce = std::slice::from_raw_parts(p.cm.elem(1), 8).to_vec();
            let re = std::slice::from_raw_parts(p.rm.elem(1), 8).to_vec();
            assert_eq!(ce, re, "C28b shmode={m}: element key bytes");
            assert_eq!(
                ce,
                &kbuf[..8],
                "C28b shmode={m}: expected memcpy of the string bytes"
            );
            p.free();
        }
    }
}

// ===========================================================================
// C29 / C30  repeated puts of the same key, across rebuild boundaries
// ===========================================================================

#[test]
fn c29_repeated_same_key() {
    let s = session(0x2929);
    let mut rng = Rng::new(0x2901_0000);
    for (shmode, mode, ptr_keys) in [
        (None, HM_BINARY, false),
        (None, HM_STRING, true),
        (Some(SH_STRDUP), HM_STRING, true),
        (Some(SH_ARENA), HM_STRING, true),
        (Some(SH_DEFAULT), HM_STRING, true),
        (Some(SH_NONE), HM_BINARY, false),
    ] {
        unsafe {
            let cfg = Cfg {
                elemsize: 16,
                keysize: 8,
                shmode,
                mode,
                ptr_keys,
            };
            let mut p = Pair::new(&s, 16, 8, cfg.keyrepr(), cfg.value_range());
            if let Some(m) = shmode {
                p.shmode(m);
            }
            let mut keys = if ptr_keys {
                Keys::strings(&mut rng, 1, 12)
            } else {
                Keys::binary(&mut rng, 1, 8)
            };
            for r in 0..200usize {
                let (idx, _) = p.put(keys.ptr(0), mode, &[r as u8, 0x5C, 0x3B, 0x11]);
                assert_eq!(idx, 0, "C29: same key must keep index 0");
                assert_eq!(p.cm.len(), 1, "C29: hmlen must stay 1");
            }
            p.free();
        }
    }
}

#[test]
fn c30_reput_across_rebuilds() {
    let mut rng = Rng::new(0x3001_0000);
    for &n in &[6usize, 7, 12, 13, 24, 25, 48, 49, 96, 97] {
        let s = session(0x3030 ^ n);
        unsafe {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            let mut keys = Keys::binary(&mut rng, n, 8);
            for i in 0..n {
                p.put(keys.ptr(i), HM_BINARY, &[i as u8, 1, 2, 3]);
                // re-put everything inserted so far on every third step
                if i % 3 == 0 {
                    for j in 0..=i {
                        let idx = p.put(keys.ptr(j), HM_BINARY, &[j as u8, 4, 5, 6]).0;
                        assert_eq!(idx, j as isize, "C30 n={n}: re-put {j} moved");
                    }
                }
            }
            p.free();
        }
    }
}

// ===========================================================================
// C31–C34  lookups
// ===========================================================================

#[test]
fn c31_c33_lookups_across_table_sizes() {
    let mut rng = Rng::new(0x3101_0000);
    for &n in &[1usize, 6, 12, 24, 48, 96, 192, 400] {
        let s = session(0x3131 ^ n);
        unsafe {
            let mut p = Pair::new(&s, 24, 16, KeyRepr::Inline(16), 16..24);
            let mut keys = Keys::binary(&mut rng, n, 16);
            for i in 0..n {
                p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0xEE]);
            }
            // C33: `_ts` must NOT write the header temp.
            let sentinel = -12345isize;
            (*p.cm.header()).temp = sentinel;
            (*p.rm.header()).temp = sentinel;
            for i in 0..n {
                let t = p.get_ts(keys.ptr(i), HM_BINARY);
                assert_eq!(t, i as isize, "C33 n={n}: _ts index");
                assert_eq!(
                    (*p.cm.header()).temp,
                    sentinel,
                    "C33: hmget_key_ts must not touch header temp"
                );
                assert_eq!((*p.rm.header()).temp, sentinel);
            }
            // C31: hits and misses through the non-_ts variant.
            for i in 0..n {
                assert_eq!(p.get(keys.ptr(i), HM_BINARY), i as isize);
            }
            let mut miss = Keys::binary(&mut rng, 20, 16);
            for i in 0..miss.len() {
                miss.bufs[i][15] ^= 0xFF;
                assert_eq!(p.get(miss.ptr(i), HM_BINARY), -1, "C31 n={n}: miss");
            }
            p.free();
        }
    }
}

#[test]
fn c32_string_lookups_all_shmodes() {
    let mut rng = Rng::new(0x3201_0000);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[1usize, 7, 13, 50, 200] {
            let s = session(0x3232 ^ (shmode as usize) ^ n);
            unsafe {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
                p.shmode(shmode);
                let mut keys = Keys::strings(&mut rng, n, 30);
                for i in 0..n {
                    p.put(keys.ptr(i), HM_STRING, &[i as u8, 0x77]);
                }
                for i in 0..n {
                    assert_eq!(p.get(keys.ptr(i), HM_STRING), i as isize);
                    assert_eq!(p.get_ts(keys.ptr(i), HM_STRING), i as isize);
                }
                let mut miss = Keys::strings(&mut rng, 10, 5);
                for i in 0..miss.len() {
                    miss.bufs[i][0] = b'Z';
                    assert_eq!(p.get(miss.ptr(i), HM_STRING), -1);
                }
                p.free();
            }
        }
    }
}

/// C34 / E9 / E12: the `a == NULL` bootstrap of both getters.
#[test]
fn c34_get_from_null() {
    let s = session(0x3434);
    for elemsize in [8usize, 16, 24, 40] {
        unsafe {
            let mut key = [0xABu8; 8];
            // hmget_key_ts
            let mut p = Pair::new(&s, elemsize, 8, KeyRepr::Inline(8), 8..elemsize);
            let t = p.get_ts(key.as_mut_ptr(), HM_BINARY);
            assert_eq!(t, -1, "E9: *temp must be STBDS_INDEX_EMPTY");
            assert_eq!(p.cm.len(), 0);
            assert!(p.cm.table().is_null() && p.rm.table().is_null());
            p.free();

            // hmget_key
            let mut p = Pair::new(&s, elemsize, 8, KeyRepr::Inline(8), 8..elemsize);
            let t = p.get(key.as_mut_ptr(), HM_BINARY);
            assert_eq!(t, -1, "E12: header temp must be -1");
            p.free();
        }
    }
}

// ===========================================================================
// C35–C45  deletion
// ===========================================================================

/// C35 / E41 (delete last) and C36 / E42 (delete middle, move + repatch).
#[test]
fn c35_c36_delete_last_and_middle() {
    let mut rng = Rng::new(0x3501_0000);
    for &n in &[1usize, 2, 3, 8, 13, 40] {
        for pick in ["last", "first", "middle"] {
            let s = session(0x3535 ^ n ^ pick.len());
            unsafe {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
                let mut keys = Keys::binary(&mut rng, n, 8);
                for i in 0..n {
                    p.put(keys.ptr(i), HM_BINARY, &[i as u8, 3]);
                }
                let k = match pick {
                    "last" => n - 1,
                    "first" => 0,
                    _ => n / 2,
                };
                assert_eq!(p.del(keys.ptr(k), HM_BINARY, 0), 1, "C35/36 n={n} {pick}");
                assert_eq!(p.cm.len(), (n - 1) as isize);
                // the moved-in entry must still be findable
                for i in 0..n {
                    let want = if i == k {
                        -1
                    } else if i == n - 1 && k != n - 1 {
                        k as isize
                    } else {
                        i as isize
                    };
                    assert_eq!(
                        p.get(keys.ptr(i), HM_BINARY),
                        want,
                        "C36 n={n} {pick}: index of key {i} after deleting {k}"
                    );
                }
                p.free();
            }
        }
    }
}

/// C37 / C44 / C45: delete every entry in random order, driving shrink and
/// tombstone rebuilds from slot_count 512 all the way back down to 8.
#[test]
fn c37_c44_c45_delete_all_random_order() {
    let mut rng = Rng::new(0x3701_0000);
    for &n in &[1usize, 8, 13, 50, 200, 400] {
        for round in 0..3 {
            let s = session(0x3737 ^ n ^ round);
            unsafe {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
                let mut keys = Keys::binary(&mut rng, n, 8);
                for i in 0..n {
                    p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0x5A]);
                }
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..order.len()).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for (step, &k) in order.iter().enumerate() {
                    assert_eq!(
                        p.del(keys.ptr(k), HM_BINARY, 0),
                        1,
                        "C37 n={n} round={round} step={step}"
                    );
                }
                assert_eq!(p.cm.len(), 0);
                assert_eq!((*p.cm.table()).slot_count, (*p.rm.table()).slot_count);
                p.free();
            }
        }
    }
}

/// C38 / C39 / C40 / E6: string-mode deletes for each key-storage strategy.
#[test]
fn c38_c40_string_deletes() {
    let mut rng = Rng::new(0x3801_0000);
    for &shmode in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
        for &n in &[1usize, 7, 13, 60, 200] {
            let s = session(0x3838 ^ (shmode as usize) ^ n);
            unsafe {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
                p.shmode(shmode);
                let mut keys = Keys::strings(&mut rng, n, 20);
                for i in 0..n {
                    p.put(keys.ptr(i), HM_STRING, &[i as u8, 2]);
                }
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..order.len()).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for &k in &order {
                    assert_eq!(
                        p.del(keys.ptr(k), HM_STRING, 0),
                        1,
                        "C38 shmode={shmode} n={n} key={k}"
                    );
                }
                p.free();
            }
        }
    }
}

/// C41 / E7: `mode == 2` on a `SH_STRDUP` table — string comparison (because
/// `find_slot` tests `>=`) but *no* key free (because the free tests `==`).
#[test]
fn c41_mode2_on_strdup_table() {
    let mut rng = Rng::new(0x4101_0000);
    for &mode in &[2 as c_int, 7, c_int::MAX] {
        for &n in &[1usize, 9, 40] {
            let s = session(0x4141 ^ (mode as usize) ^ n);
            unsafe {
                let mut p = Pair::new(&s, 16, 8, KeyRepr::Pointer, 8..16);
                p.shmode(SH_STRDUP);
                let mut keys = Keys::strings(&mut rng, n, 16);
                for i in 0..n {
                    p.put(keys.ptr(i), mode, &[i as u8, 1]);
                }
                // Tail-first: `mode != STBDS_HM_STRING` makes the post-move
                // re-lookup in hmdel_key hash the pointer bytes and fail, which
                // trips the live `STBDS_ASSERT(slot >= 0)` (E18, covered
                // separately in tests/errors.rs).  Deleting the last entry keeps
                // `old_index == final_index` so that block is skipped.
                for i in (0..n).rev() {
                    assert_eq!(
                        p.del(keys.ptr(i), mode, 0),
                        1,
                        "C41 mode={mode}: delete must succeed via strcmp"
                    );
                }
                // The strdup'd keys were intentionally leaked by the C code.
                p.free();
            }
        }
    }
}

/// C42 / E43: non-zero `keyoffset`.
#[test]
fn c42_keyoffset() {
    let mut rng = Rng::new(0x4201_0000);
    let s = session(0x4242);
    let (elemsize, keysize) = (24usize, 8usize);
    unsafe {
        // (a) absent keys with several keyoffsets — always a clean miss.
        let mut p = Pair::new(&s, elemsize, keysize, KeyRepr::Inline(keysize), keysize..elemsize);
        let mut keys = Keys::binary(&mut rng, 30, keysize);
        for i in 0..keys.len() {
            p.put(keys.ptr(i), HM_BINARY, &[i as u8, 0x11, 0x22, 0x33]);
        }
        let mut absent = Keys::binary(&mut rng, 6, keysize);
        for i in 0..absent.len() {
            absent.bufs[i][7] = 0xFE;
            for &ko in &[0usize, 4, 8, 16] {
                assert_eq!(
                    p.del(absent.ptr(i), HM_BINARY, ko),
                    0,
                    "C42a: absent delete with keyoffset={ko}"
                );
            }
        }
        p.free();

        // (b) a *matching* keyoffset delete of the final entry, where the
        //     move-and-repatch block is skipped (old_index == final_index).
        let mut p = Pair::new(&s, elemsize, keysize, KeyRepr::Inline(keysize), keysize..elemsize);
        let mut keys = Keys::binary(&mut rng, 5, keysize);
        for i in 0..keys.len() {
            p.put(keys.ptr(i), HM_BINARY, &[0u8; 4]);
        }
        let last = keys.len() - 1;
        // Mirror the key bytes to offset 8 of the last element on both sides.
        for m in [&p.cm, &p.rm] {
            let e = m.elem(last + 1);
            std::ptr::copy_nonoverlapping(keys.bufs[last].as_ptr(), e.add(8), keysize);
        }
        p.check("C42b setup");
        assert_eq!(
            p.del(keys.ptr(last), HM_BINARY, 8),
            1,
            "C42b: keyoffset=8 delete of the last entry must succeed"
        );
        assert_eq!(p.cm.len(), (keys.len() - 1) as isize);
        p.free();
    }
}

/// C43: delete then re-insert, so `found_empty_slot` reuses a tombstone.
#[test]
fn c43_tombstone_reuse() {
    let mut rng = Rng::new(0x4301_0000);
    for &n in &[8usize, 20, 60, 150] {
        let s = session(0x4343 ^ n);
        unsafe {
            let mut p = Pair::new(&s, 16, 8, KeyRepr::Inline(8), 8..16);
            let mut keys = Keys::binary(&mut rng, n, 8);
            for i in 0..n {
                p.put(keys.ptr(i), HM_BINARY, &[i as u8, 7]);
            }
            for cycle in 0..6usize {
                // remove half, then put them back
                let half: Vec<usize> = (0..n).filter(|i| i % 2 == cycle % 2).collect();
                for &k in &half {
                    p.del(keys.ptr(k), HM_BINARY, 0);
                }
                for &k in &half {
                    p.put(keys.ptr(k), HM_BINARY, &[k as u8, cycle as u8]);
                }
                for i in 0..n {
                    assert!(
                        p.get(keys.ptr(i), HM_BINARY) >= 0,
                        "C43 n={n} cycle={cycle}: key {i} vanished"
                    );
                }
            }
            p.free();
        }
    }
}

// ===========================================================================
// C46 / C47 / E1 / E2  stbds_hmfree_func
// ===========================================================================

#[test]
fn c46_c47_hmfree() {
    let s = session(0x4646);
    let mut rng = Rng::new(0x4601_0000);
    unsafe {
        // E1: NULL is a no-op on both.
        (s.c().hmfree_func)(std::ptr::null_mut(), 16);
        (s.r().hmfree_func)(std::ptr::null_mut(), 16);

        // C46/C47: every shmode, with and without entries.
        for &shmode in &[SH_STRDUP, SH_ARENA, SH_DEFAULT, SH_NONE] {
            for &n in &[0usize, 1, 5, 40] {
                let ptr_keys = shmode != SH_NONE;
                let mode = if ptr_keys { HM_STRING } else { HM_BINARY };
                let repr = if ptr_keys {
                    KeyRepr::Pointer
                } else {
                    KeyRepr::Inline(8)
                };
                let mut p = Pair::new(&s, 16, 8, repr, 8..16);
                p.shmode(shmode);
                let mut keys = if ptr_keys {
                    Keys::strings(&mut rng, n.max(1), 300)
                } else {
                    Keys::binary(&mut rng, n.max(1), 8)
                };
                for i in 0..n {
                    p.put(keys.ptr(i), mode, &[i as u8, 6]);
                }
                p.free();
            }
        }

        // E2: an array with no hash table at all.
        for elemsize in [8usize, 16, 40] {
            let ca = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
            let ra = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
            assert!((*(ca as *mut ArrayHeader).wrapping_sub(1)).hash_table.is_null());
            assert!((*(ra as *mut ArrayHeader).wrapping_sub(1)).hash_table.is_null());
            (s.c().hmfree_func)(ca, elemsize);
            (s.r().hmfree_func)(ra, elemsize);
        }
    }
}

// ===========================================================================
// C54–C58  full randomized pipelines
// ===========================================================================

#[test]
fn c54_c58_full_pipelines() {
    let combos: &[(Option<c_int>, c_int, bool, &str)] = &[
        (None, HM_BINARY, false, "C54 binary"),
        (Some(SH_STRDUP), HM_STRING, true, "C55 strdup"),
        (Some(SH_ARENA), HM_STRING, true, "C56 arena"),
        (Some(SH_DEFAULT), HM_STRING, true, "C57 default"),
        (None, HM_STRING, true, "C58 implicit string"),
    ];
    let mut rng = Rng::new(0x5400_0000);
    for &(shmode, mode, ptr_keys, tag) in combos {
        for &n in &[0usize, 1, 2, 6, 7, 12, 13, 24, 25, 48, 100, 250] {
            for round in 0..2usize {
                let seed = 0x5454usize
                    .wrapping_mul(n + 1)
                    .wrapping_add(round)
                    .wrapping_add(mode as usize);
                let s = session(seed);
                let cfg = Cfg {
                    elemsize: 24,
                    keysize: 8,
                    shmode,
                    mode,
                    ptr_keys,
                };
                unsafe { pipeline(&s, cfg, n, &mut rng, &format!("{tag} n={n} r={round}")) };
            }
        }
    }
}

// ===========================================================================
// C59  engineered bucket collisions: quadratic probe + wrap-around
// ===========================================================================

#[test]
fn c59_engineered_collisions() {
    const SEED: usize = 0x0123_4567_89AB_CDEF;
    let s = session(SEED);
    let mut rng = Rng::new(0x5901_0000);
    let (elemsize, keysize) = (16usize, 8usize);
    unsafe {
        let mut p = Pair::new(&s, elemsize, keysize, KeyRepr::Inline(keysize), keysize..elemsize);
        // shmode_func(_, SH_NONE) creates the table with seed == SEED and the
        // `default:` (binary) key-storage arm.
        p.shmode(SH_NONE);
        assert_eq!((*p.cm.table()).seed, SEED);
        assert_eq!((*p.rm.table()).seed, SEED);

        // Collect keys whose probe position stays inside bucket 0 for
        // slot_count 8, 16 and 32 (i.e. `hash & 31 < 8`).
        let mut colliding: Vec<Box<[u8]>> = Vec::new();
        let mut tries = 0usize;
        while colliding.len() < 24 {
            tries += 1;
            assert!(tries < 2_000_000, "could not find colliding keys");
            let mut v = rng.bytes(keysize);
            v.push(0);
            let mut h = (s.c().hash_bytes)(v.as_mut_ptr() as *mut _, keysize, SEED);
            let hr = (s.r().hash_bytes)(v.as_mut_ptr() as *mut _, keysize, SEED);
            assert_eq!(h, hr, "hash_bytes disagreed while building the collision set");
            if h < 2 {
                h += 2;
            }
            if h & 31 < 8 {
                colliding.push(v.into_boxed_slice());
            }
        }

        for (i, k) in colliding.iter_mut().enumerate() {
            let idx = p.put(k.as_mut_ptr(), HM_BINARY, &[i as u8, 0x33]).0;
            assert_eq!(idx, i as isize, "C59: put #{i}");
        }
        for (i, k) in colliding.iter_mut().enumerate() {
            assert_eq!(p.get(k.as_mut_ptr(), HM_BINARY), i as isize, "C59: get #{i}");
        }
        // Delete in an order that leaves long tombstone runs inside bucket 0.
        let mut order: Vec<usize> = (0..colliding.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for &k in &order {
            assert_eq!(p.del(colliding[k].as_mut_ptr(), HM_BINARY, 0), 1);
        }
        // and re-insert, so tombstone reuse also happens under collisions
        for (i, k) in colliding.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), HM_BINARY, &[i as u8, 0x44]);
        }
        p.free();
    }
}

// ===========================================================================
// C60  extreme element/key geometries across all four shmodes
// ===========================================================================

#[test]
fn c60_extreme_geometries() {
    let mut rng = Rng::new(0x6001_0000);
    // minimal binary: elemsize == keysize == 8 (no value bytes at all)
    for &shmode in &[None, Some(SH_NONE), Some(4 as c_int)] {
        let s = session(0x6060 ^ (shmode.unwrap_or(-9) as usize));
        let cfg = Cfg {
            elemsize: 8,
            keysize: 8,
            shmode,
            mode: HM_BINARY,
            ptr_keys: false,
        };
        unsafe { pipeline(&s, cfg, 40, &mut rng, &format!("C60 min shmode={shmode:?}")) };
    }
    // maximal binary: elemsize 40, keysize 33
    for &shmode in &[None, Some(SH_NONE)] {
        let s = session(0x6061 ^ (shmode.unwrap_or(-9) as usize));
        let cfg = Cfg {
            elemsize: 40,
            keysize: 33,
            shmode,
            mode: HM_BINARY,
            ptr_keys: false,
        };
        unsafe { pipeline(&s, cfg, 40, &mut rng, &format!("C60 max shmode={shmode:?}")) };
    }
    // string modes with a big element
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let s = session(0x6062 ^ (shmode as usize));
        let cfg = Cfg {
            elemsize: 40,
            keysize: 8,
            shmode: Some(shmode),
            mode: HM_STRING,
            ptr_keys: true,
        };
        unsafe { pipeline(&s, cfg, 40, &mut rng, &format!("C60 str shmode={shmode}")) };
    }
}


const _: Option<*mut c_char> = None;
