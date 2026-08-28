//! Phase B rows C20..C49 -- the hash map: `stbds_hmput_key`,
//! `stbds_hmget_key(_ts)`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_shmode_func`, `stbds_hmfree_func`.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// The configuration cross-product the C actually distinguishes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Cfg {
    name: &'static str,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    arena: Arena,
    /// keys must be NUL-terminated (string hashing or `strdup`)
    string_keys: bool,
    /// safe to re-put an existing key (`is_key_equal` must not deref garbage)
    dups_ok: bool,
}

impl Cfg {
    /// `stbds_is_key_equal` in STRING mode dereferences element offset 0 as a
    /// `char *`.  That is only a valid pointer when `string.mode` stores one
    /// (SH_DEFAULT/STRDUP/ARENA).  With `SH_NONE` the element holds raw key
    /// *bytes*, so any lookup/delete that reaches `is_key_equal` faults --
    /// identically in both libraries (see ERRORS.md row E30, exercised in a
    /// forked child by `phase_c_errors`).
    fn lookups_ok(&self) -> bool {
        if self.mode >= HM_STRING {
            matches!(self.arena, Arena::Auto)
                || matches!(self.arena, Arena::Explicit(m) if m == SH_DEFAULT || m == SH_STRDUP || m == SH_ARENA)
        } else {
            true
        }
    }

    /// Whether a key that was put can be found again: BINARY mode compares the
    /// caller's key *bytes* against element offset 0, which only holds those
    /// bytes when `string.mode` falls to the `default:` memcpy branch.
    fn del_finds(&self) -> bool {
        if !self.lookups_ok() {
            return false;
        }
        if self.mode >= HM_STRING {
            true
        } else {
            match self.arena {
                Arena::Auto => true,
                Arena::Explicit(m) => (m & 0xff) == SH_NONE,
            }
        }
    }

    /// Distinct keys available for this configuration.
    fn max_keys(&self) -> usize {
        if self.string_keys {
            usize::MAX
        } else {
            BinKeys::max_distinct(self.keysize)
        }
    }
}

const fn cfg(
    name: &'static str,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    arena: Arena,
    string_keys: bool,
    dups_ok: bool,
) -> Cfg {
    Cfg { name, elemsize, keysize, mode, arena, string_keys, dups_ok }
}

/// Every combination of (`mode`, `string.mode`, `elemsize`, `keysize`) the C
/// treats differently.
const CFGS: &[Cfg] = &[
    // --- binary keys, lazily created table (string.mode == SH_NONE) ---------
    cfg("bin/auto e8 k4",    8, 4, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e16 k8",  16, 8, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e24 k16", 24, 16, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e40 k8",  40, 8, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e12 k4",  12, 4, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e5 k1",    5, 1, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e8 k8",    8, 8, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e16 k2",  16, 2, HM_BINARY, Arena::Auto, false, true),
    cfg("bin/auto e3 k1",    3, 1, HM_BINARY, Arena::Auto, false, true),
    // --- string keys, lazily created table (string.mode becomes SH_DEFAULT) -
    cfg("str/auto e16 k8",  16, 8, HM_STRING, Arena::Auto, true, true),
    cfg("str/auto e8 k8",    8, 8, HM_STRING, Arena::Auto, true, true),
    cfg("str/auto e24 k8",  24, 8, HM_STRING, Arena::Auto, true, true),
    cfg("str/auto e40 k8",  40, 8, HM_STRING, Arena::Auto, true, true),
    // --- string keys, explicit arena modes ---------------------------------
    cfg("str/default e16",  16, 8, HM_STRING, Arena::Explicit(SH_DEFAULT), true, true),
    cfg("str/strdup e16",   16, 8, HM_STRING, Arena::Explicit(SH_STRDUP), true, true),
    cfg("str/arena e16",    16, 8, HM_STRING, Arena::Explicit(SH_ARENA), true, true),
    cfg("str/strdup e40",   40, 8, HM_STRING, Arena::Explicit(SH_STRDUP), true, true),
    cfg("str/arena e8",      8, 8, HM_STRING, Arena::Explicit(SH_ARENA), true, true),
    // SH_NONE + HM_STRING: `default:` memcpy of the key *bytes* into the
    // element; a duplicate put would strcmp() a garbage pointer, so no dups.
    cfg("str/none e16",     16, 8, HM_STRING, Arena::Explicit(SH_NONE), true, false),
    // --- binary mode crossed with the pointer-storing arena modes ----------
    cfg("bin/default e16",  16, 8, HM_BINARY, Arena::Explicit(SH_DEFAULT), false, true),
    cfg("bin/strdup e16",   16, 8, HM_BINARY, Arena::Explicit(SH_STRDUP), true, true),
    cfg("bin/arena e16",    16, 8, HM_BINARY, Arena::Explicit(SH_ARENA), true, true),
    cfg("bin/none e16",     16, 8, HM_BINARY, Arena::Explicit(SH_NONE), false, true),
];

/// Owns the key buffers for one test run and hands out identical pointers to
/// both libraries.
enum KeySet {
    Str(Keys),
    Bin(BinKeys),
}

impl KeySet {
    /// A present-set and a guaranteed-**disjoint** absent-set.
    fn make_pair(cfg: &Cfg, rng: &mut Rng, n: usize, m: usize) -> (KeySet, KeySet) {
        let n = n.min(cfg.max_keys()).max(1);
        let m = m.min(cfg.max_keys().saturating_sub(n)).max(1);
        if cfg.string_keys {
            let (a, b) = Keys::random_disjoint(rng, n, m, 24);
            (KeySet::Str(a), KeySet::Str(b))
        } else {
            let (a, b) = BinKeys::random_disjoint(rng, n, m, cfg.keysize, cfg.keysize.max(8) + 8);
            (KeySet::Bin(a), KeySet::Bin(b))
        }
    }

    fn make(cfg: &Cfg, rng: &mut Rng, n: usize) -> KeySet {
        let n = n.min(cfg.max_keys()).max(1);
        if cfg.string_keys {
            KeySet::Str(Keys::random(rng, n, 24))
        } else {
            KeySet::Bin(BinKeys::random_prefix(
                rng,
                n,
                cfg.keysize,
                cfg.keysize.max(8) + 8,
            ))
        }
    }
    fn ptr(&self, i: usize) -> *mut c_void {
        match self {
            KeySet::Str(k) => k.ptr(i),
            KeySet::Bin(k) => k.ptr(i),
        }
    }
    fn len(&self) -> usize {
        match self {
            KeySet::Str(k) => k.len(),
            KeySet::Bin(k) => k.len(),
        }
    }
}

fn pair<'a>(c: &'a Api, rs: &'a Api, cfg: &Cfg) -> MapPair<'a> {
    MapPair::new(c, rs, cfg.elemsize, cfg.keysize, cfg.mode, cfg.arena)
}

/// Insert `n` keys, checking full internal state after every put.
fn build(m: &mut MapPair, ks: &KeySet, n: usize, tag: &str) {
    for i in 0..n {
        m.put(ks.ptr(i), 0x1000_0000u64 + i as u64);
        m.check(&format!("{tag}: after put #{i}"));
    }
}

// --- C20 --------------------------------------------------------------------
#[test]
fn cfg_c20_binary_single() {
    let mut rng = Rng::new(20);
    for seed in [0usize, 1, 0x31415926, usize::MAX] {
        with_libs(seed, |c, rs| {
            let cf = cfg("e8k4", 8, 4, HM_BINARY, Arena::Auto, false, true);
            for _ in 0..200 {
                let ks = KeySet::make(&cf, &mut rng, 1);
                let mut m = pair(c, rs, &cf);
                let i = m.put(ks.ptr(0), 0xDEAD_BEEF);
                assert_eq!(i, 0, "first insert must land at hash-index 0");
                m.check("c20 single insert");
                assert_eq!(m.len(), 1);
                m.free();
            }
        });
    }
}

// --- C21 --------------------------------------------------------------------
#[test]
fn cfg_c21_binary_keysize_matrix() {
    let mut rng = Rng::new(21);
    with_libs(0x31415926, |c, rs| {
        for keysize in [1usize, 2, 4, 8, 16] {
            for elemsize in [keysize, keysize + 4, keysize + 8, 40] {
                if elemsize == 0 {
                    continue;
                }
                let cf = cfg("m", elemsize, keysize, HM_BINARY, Arena::Auto, false, true);
                for _ in 0..30 {
                    let ks = KeySet::make(&cf, &mut rng, 5);
                    let mut m = pair(c, rs, &cf);
                    let n = ks.len();
                    build(&mut m, &ks, n, &format!("c21 e{elemsize} k{keysize}"));
                    m.free();
                }
            }
        }
    });
}

// --- C22 / C23 / C24 --------------------------------------------------------
#[test]
fn cfg_c22_binary_at_threshold() {
    let mut rng = Rng::new(22);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..60 {
            let ks = KeySet::make(&cf, &mut rng, 6);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 6, "c22");
            let t = m.table_c().expect("table");
            assert_eq!(t.slot_count, 8, "must not have grown yet");
            assert_eq!(t.used_count, 6);
            assert_eq!(t.used_count_threshold, 6);
            m.free();
        }
    });
}

#[test]
fn cfg_c23_binary_first_grow() {
    let mut rng = Rng::new(23);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..60 {
            let ks = KeySet::make(&cf, &mut rng, 7);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 7, "c23");
            let t = m.table_c().expect("table");
            assert_eq!(t.slot_count, 16, "7th insert must double the table");
            assert_eq!(t.used_count, 7);
            m.free();
        }
    });
}

#[test]
fn cfg_c24_binary_multi_grow() {
    let mut rng = Rng::new(24);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for (n, want_slots) in [(13usize, 32usize), (25, 64), (49, 128), (97, 256)] {
            for _ in 0..10 {
                let ks = KeySet::make(&cf, &mut rng, n);
                let mut m = pair(c, rs, &cf);
                for i in 0..n {
                    m.put(ks.ptr(i), i as u64);
                }
                m.check(&format!("c24 n={n}"));
                let t = m.table_c().expect("table");
                assert_eq!(t.slot_count, want_slots, "n={n}");
                m.free();
            }
        }
    });
}

// --- C25 --------------------------------------------------------------------
#[test]
fn cfg_c25_binary_1000() {
    let mut rng = Rng::new(25);
    for seed in [0usize, 0x31415926, usize::MAX] {
        with_libs(seed, |c, rs| {
            let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
            let ks = KeySet::make(&cf, &mut rng, 1000);
            let mut m = pair(c, rs, &cf);
            for i in 0..1000 {
                m.put(ks.ptr(i), (i as u64) * 0x0101_0101);
            }
            m.check("c25 1000 inserts");
            // look every key up again
            for i in 0..1000 {
                let idx = m.get(ks.ptr(i));
                assert!(idx >= 0, "key {i} not found");
            }
            m.check("c25 after 1000 lookups");
            m.free();
        });
    }
}

// --- C26 --------------------------------------------------------------------
#[test]
fn cfg_c26_binary_duplicates() {
    let mut rng = Rng::new(26);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 30);
            let mut m = pair(c, rs, &cf);
            for i in 0..30 {
                m.put(ks.ptr(i), i as u64);
            }
            let len_before = m.len();
            // re-put every key (update path)
            for i in 0..30 {
                let idx = m.put(ks.ptr(i), 0xF000 + i as u64);
                assert!(idx >= 0);
                m.check(&format!("c26 re-put #{i}"));
            }
            assert_eq!(m.len(), len_before, "duplicates must not grow the map");
            m.free();
        }
    });
}

// --- C27 / C28 --------------------------------------------------------------
#[test]
fn cfg_c27_binary_keysize0() {
    // keysize == 0: memcmp(...,0) == 0 so every key "equals" every other one
    let mut rng = Rng::new(27);
    with_libs(0x31415926, |c, rs| {
        for elemsize in [8usize, 16, 40] {
            let cf = cfg("k0", elemsize, 0, HM_BINARY, Arena::Auto, false, true);
            for _ in 0..40 {
                let ks = KeySet::make(&cf, &mut rng, 20);
                let mut m = pair(c, rs, &cf);
                for i in 0..ks.len() {
                    m.put(ks.ptr(i), i as u64);
                    m.check(&format!("c27 e{elemsize} put#{i}"));
                }
                assert_eq!(m.len(), 1, "all keysize==0 keys collapse into one");
                m.free();
            }
        }
    });
}

#[test]
fn cfg_c28_binary_elemsize0() {
    let mut rng = Rng::new(28);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e0", 0, 0, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 10);
            let mut m = pair(c, rs, &cf);
            for i in 0..ks.len() {
                m.put(ks.ptr(i), i as u64);
                m.check(&format!("c28 put#{i}"));
            }
            m.free();
        }
    });
}

// --- C29..C34: string modes -------------------------------------------------
#[test]
fn cfg_c29_string_default_mode() {
    let mut rng = Rng::new(29);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("str/auto", 16, 8, HM_STRING, Arena::Auto, true, true);
        for _ in 0..60 {
            let ks = KeySet::make(&cf, &mut rng, 12);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 12, "c29");
            let t = m.table_c().expect("table");
            assert_eq!(t.arena_mode, SH_DEFAULT as u8, "auto table must be SH_DEFAULT");
            m.free();
        }
    });
}

#[test]
fn cfg_c30_strdup_mode() {
    let mut rng = Rng::new(30);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("strdup", 16, 8, HM_STRING, Arena::Explicit(SH_STRDUP), true, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 20);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 20, "c30");
            let t = m.table_c().expect("table");
            assert_eq!(t.arena_mode, SH_STRDUP as u8);
            m.free();
        }
    });
}

#[test]
fn cfg_c31_arena_mode() {
    let mut rng = Rng::new(31);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("arena", 16, 8, HM_STRING, Arena::Explicit(SH_ARENA), true, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 200);
            let mut m = pair(c, rs, &cf);
            for i in 0..200 {
                m.put(ks.ptr(i), i as u64);
                m.check(&format!("c31 put#{i}"));
            }
            let t = m.table_c().expect("table");
            assert_eq!(t.arena_mode, SH_ARENA as u8);
            assert!(t.arena_has_storage, "arena must have allocated a block");
            m.free();
        }
    });
}

#[test]
fn cfg_c32_sh_none_with_string_mode() {
    // string hashing, but `switch(string.mode)` falls to `default:` and
    // memcpy's the raw key bytes into the element.
    let mut rng = Rng::new(32);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("none/str", 16, 8, HM_STRING, Arena::Explicit(SH_NONE), true, false);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 6);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 6, "c32");
            let t = m.table_c().expect("table");
            assert_eq!(t.arena_mode, SH_NONE as u8);
            m.free();
        }
    });
}

#[test]
fn cfg_c33_sh_default_binary_mode() {
    let mut rng = Rng::new(33);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("default/bin", 16, 8, HM_BINARY, Arena::Explicit(SH_DEFAULT), false, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 10);
            let mut m = pair(c, rs, &cf);
            build(&mut m, &ks, 10, "c33");
            m.free();
        }
    });
}

#[test]
fn cfg_c34_string_many_all_modes() {
    let mut rng = Rng::new(34);
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(0x31415926, |c, rs| {
            let cf = cfg("many", 16, 8, HM_STRING, Arena::Explicit(sh), true, true);
            let ks = KeySet::make(&cf, &mut rng, 400);
            let mut m = pair(c, rs, &cf);
            for i in 0..400 {
                m.put(ks.ptr(i), i as u64);
            }
            m.check(&format!("c34 sh={sh} 400 inserts"));
            let t = m.table_c().expect("table");
            assert!(t.slot_count >= 512, "expected several grows, got {}", t.slot_count);
            for i in 0..400 {
                assert!(m.get(ks.ptr(i)) >= 0, "missing key {i}");
            }
            m.check(&format!("c34 sh={sh} lookups"));
            m.free();
        });
    }
}

#[test]
fn cfg_c35_string_duplicates() {
    // distinct pointers, equal content -> the first inner loop finds the dup
    // and updates `temp_key`.
    let mut rng = Rng::new(35);
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(0x31415926, |c, rs| {
            let cf = cfg("dups", 16, 8, HM_STRING, Arena::Explicit(sh), true, true);
            for _ in 0..30 {
                let ks = KeySet::make(&cf, &mut rng, 20);
                // aliases with identical content but different addresses
                let aliases: Vec<Vec<u8>> = (0..20)
                    .map(|i| match &ks {
                        KeySet::Str(k) => k.bufs[i].clone(),
                        _ => unreachable!(),
                    })
                    .collect();
                let mut m = pair(c, rs, &cf);
                for i in 0..20 {
                    m.put(ks.ptr(i), i as u64);
                }
                let len_before = m.len();
                for i in 0..20 {
                    let p = aliases[i].as_ptr() as *mut c_void;
                    let idx = m.put(p, 0xAA00 + i as u64);
                    assert!(idx >= 0);
                    m.check(&format!("c35 sh={sh} alias put#{i}"));
                }
                assert_eq!(m.len(), len_before);
                m.free();
            }
        });
    }
}

// --- C36 / C37 / C38 --------------------------------------------------------
#[test]
fn cfg_c36_hmget_ts_binary() {
    let mut rng = Rng::new(36);
    with_libs(0x31415926, |c, rs| {
        for cf in CFGS.iter().filter(|c| c.mode == HM_BINARY && c.lookups_ok()) {
            let (ks, absent) = KeySet::make_pair(cf, &mut rng, 40, 40);
            let n = ks.len().min(absent.len());
            let mut m = pair(c, rs, cf);
            for i in 0..n {
                m.put(ks.ptr(i), i as u64);
            }
            for i in 0..n {
                m.get_ts(ks.ptr(i));
                m.get_ts(absent.ptr(i));
            }
            m.check(&format!("c36 {}", cf.name));
            m.free();
        }
    });
}

#[test]
fn cfg_c37_hmget_ts_string() {
    let mut rng = Rng::new(37);
    with_libs(0x31415926, |c, rs| {
        for cf in CFGS.iter().filter(|c| c.mode == HM_STRING && c.lookups_ok()) {
            let (ks, absent) = KeySet::make_pair(cf, &mut rng, 40, 40);
            let n = ks.len().min(absent.len());
            let mut m = pair(c, rs, cf);
            for i in 0..n {
                m.put(ks.ptr(i), i as u64);
            }
            for i in 0..n {
                let hit = m.get_ts(ks.ptr(i));
                if cf.del_finds() {
                    assert!(hit >= 0, "{}: key {i} lost", cf.name);
                }
                m.get_ts(absent.ptr(i));
            }
            m.check(&format!("c37 {}", cf.name));
            m.free();
        }
    });
}

#[test]
fn cfg_c38_hmget_key_wrapper() {
    let mut rng = Rng::new(38);
    with_libs(0x31415926, |c, rs| {
        for cf in CFGS.iter().filter(|c| c.lookups_ok()) {
            let (ks, absent) = KeySet::make_pair(cf, &mut rng, 30, 30);
            let n = ks.len().min(absent.len());
            let mut m = pair(c, rs, cf);
            for i in 0..n {
                m.put(ks.ptr(i), i as u64);
            }
            for i in 0..n {
                m.get(ks.ptr(i));
                m.check(&format!("c38 {} hit#{i}", cf.name));
                m.get(absent.ptr(i));
                m.check(&format!("c38 {} miss#{i}", cf.name));
            }
            m.free();
        }
    });
}

// --- C39 --------------------------------------------------------------------
#[test]
fn cfg_c39_hmput_default_paths() {
    let mut rng = Rng::new(39);
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [8usize, 16, 24, 40] {
            // (a) fresh
            let mut tc = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut tr = (rs.hmput_default)(std::ptr::null_mut(), elemsize);
            assert_same(
                "c39 fresh hmput_default",
                &snap_map(tc, elemsize, KeyKind::Binary),
                &snap_map(tr, elemsize, KeyKind::Binary),
            );
            // (b) idempotent no-op on a non-empty map
            for _ in 0..5 {
                tc = (c.hmput_default)(tc, elemsize);
                tr = (rs.hmput_default)(tr, elemsize);
                assert_same(
                    "c39 repeated hmput_default",
                    &snap_map(tc, elemsize, KeyKind::Binary),
                    &snap_map(tr, elemsize, KeyKind::Binary),
                );
            }
            // (c) interleaved with real puts
            let cf = cfg("d", elemsize, 8, HM_BINARY, Arena::Auto, false, true);
            let ks = KeySet::make(&cf, &mut rng, 20);
            let mut m = MapPair {
                c: tc,
                rs: tr,
                elemsize,
                keysize: 8,
                mode: HM_BINARY,
                skip: 8,
                stores_ptr: false,
                sh: SH_NONE,
                kind: KeyKind::Binary,
                capi: c,
                rapi: rs,
            };
            for i in 0..20 {
                m.put(ks.ptr(i), i as u64);
                let a = (c.hmput_default)(m.c, elemsize);
                let b = (rs.hmput_default)(m.rs, elemsize);
                assert_eq!(a, m.c);
                assert_eq!(b, m.rs);
                m.check(&format!("c39 mixed e{elemsize} #{i}"));
            }
            m.free();
        }
    });
}

// --- C40 / C41 --------------------------------------------------------------
#[test]
fn cfg_c40_del_last() {
    let mut rng = Rng::new(40);
    with_libs(0x31415926, |c, rs| {
        let mut exercised = 0usize;
        for cf in CFGS.iter().filter(|c| c.del_finds()) {
            for _ in 0..12 {
                let ks = KeySet::make(cf, &mut rng, 5);
                let n = ks.len();
                let mut m = pair(c, rs, cf);
                for i in 0..n {
                    m.put(ks.ptr(i), i as u64);
                }
                // delete in reverse insertion order: old_index == final_index
                for i in (0..n).rev() {
                    let t = m.del(ks.ptr(i), 0);
                    assert_eq!(t, 1, "{}: delete of last elem must set temp=1", cf.name);
                    m.check(&format!("c40 {} del#{i}", cf.name));
                    exercised += 1;
                }
                assert_eq!(m.len(), 0, "{}", cf.name);
                m.free();
            }
        }
        assert!(exercised > 100, "delete-hit path barely exercised");
    });
}

#[test]
fn cfg_c41_del_middle() {
    let mut rng = Rng::new(41);
    with_libs(0x31415926, |c, rs| {
        let mut exercised = 0usize;
        for cf in CFGS.iter().filter(|c| c.del_finds()) {
            for _ in 0..12 {
                let ks = KeySet::make(cf, &mut rng, 6);
                let n = ks.len();
                let mut m = pair(c, rs, cf);
                for i in 0..n {
                    m.put(ks.ptr(i), i as u64);
                }
                // delete in insertion order: forces relocation + re-index
                for i in 0..n {
                    let t = m.del(ks.ptr(i), 0);
                    assert_eq!(t, 1, "{}", cf.name);
                    m.check(&format!("c41 {} del#{i}", cf.name));
                    exercised += 1;
                }
                assert_eq!(m.len(), 0, "{}", cf.name);
                m.free();
            }
        }
        assert!(exercised > 100, "delete-relocate path barely exercised");
    });
}

// --- C42 / C43 --------------------------------------------------------------
#[test]
fn cfg_c42_del_tombstone_rebuild() {
    // tombstone_count_threshold for slot_count=64 is (64>>3)+(64>>4) = 8+4 = 12
    let mut rng = Rng::new(42);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..25 {
            let ks = KeySet::make(&cf, &mut rng, 60);
            let mut m = pair(c, rs, &cf);
            for i in 0..60 {
                m.put(ks.ptr(i), i as u64);
            }
            let t0 = m.table_c().expect("table");
            let sc0 = t0.slot_count;
            let mut saw_rebuild = false;
            let mut saw_shrink = false;
            for i in 0..40 {
                m.del(ks.ptr(i), 0);
                m.check(&format!("c42 del#{i}"));
                let t = m.table_c().expect("table");
                if t.slot_count == sc0 && t.tombstone_count == 0 && i > 0 {
                    saw_rebuild = true;
                }
                if t.slot_count < sc0 {
                    saw_shrink = true;
                }
            }
            assert!(
                saw_rebuild || saw_shrink,
                "neither the tombstone rebuild nor the shrink path was taken"
            );
            m.free();
        }
    });
}

#[test]
fn cfg_c43_del_shrink() {
    let mut rng = Rng::new(43);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..25 {
            let ks = KeySet::make(&cf, &mut rng, 200);
            let mut m = pair(c, rs, &cf);
            for i in 0..200 {
                m.put(ks.ptr(i), i as u64);
            }
            let big = m.table_c().unwrap().slot_count;
            assert!(big >= 256);
            for i in 0..200 {
                m.del(ks.ptr(i), 0);
                m.check(&format!("c43 del#{i}"));
            }
            let small = m.table_c().unwrap().slot_count;
            assert!(small < big, "table must have shrunk: {big} -> {small}");
            assert_eq!(small, 8, "shrinking must bottom out at STBDS_BUCKET_LENGTH");
            m.free();
        }
    });
}

// --- C44 --------------------------------------------------------------------
#[test]
fn cfg_c44_del_string_all_modes() {
    let mut rng = Rng::new(44);
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        with_libs(0x31415926, |c, rs| {
            let cf = cfg("del/str", 16, 8, HM_STRING, Arena::Explicit(sh), true, true);
            for _ in 0..20 {
                let ks = KeySet::make(&cf, &mut rng, 60);
                let mut m = pair(c, rs, &cf);
                for i in 0..60 {
                    m.put(ks.ptr(i), i as u64);
                }
                let order: Vec<usize> = {
                    let mut v: Vec<usize> = (0..60).collect();
                    for i in (1..60).rev() {
                        let j = rng.below(i + 1);
                        v.swap(i, j);
                    }
                    v
                };
                for (n, &i) in order.iter().enumerate() {
                    let t = m.del(ks.ptr(i), 0);
                    assert_eq!(t, 1, "sh={sh} key {i}");
                    m.check(&format!("c44 sh={sh} del#{n}"));
                }
                assert_eq!(m.len(), 0);
                m.free();
            }
        });
    }
}

// --- C45 --------------------------------------------------------------------
#[test]
fn cfg_c45_del_nonzero_keyoffset() {
    // `stbds_hmput_key` hardcodes keyoffset = 0, but `stbds_hmdel_key` takes it
    // as a parameter (STBDS_OFFSETOF). The asymmetry must be reproduced.
    let mut rng = Rng::new(45);
    with_libs(0x31415926, |c, rs| {
        for (elemsize, keysize, keyoffset) in
            [(16usize, 8usize, 8usize), (24, 8, 8), (24, 8, 16), (16, 4, 4), (40, 16, 8)]
        {
            let cf = cfg("ko", elemsize, keysize, HM_BINARY, Arena::Auto, false, true);
            for _ in 0..25 {
                let ks = KeySet::make(&cf, &mut rng, 8);
                let n = ks.len();
                let mut m = pair(c, rs, &cf);
                for i in 0..n {
                    m.put(ks.ptr(i), i as u64);
                }
                for i in 0..n {
                    m.del(ks.ptr(i), keyoffset);
                    m.check(&format!("c45 e{elemsize} k{keysize} off{keyoffset} del#{i}"));
                }
                m.free();
            }
        }
    });
}

// --- C46 --------------------------------------------------------------------
#[test]
fn cfg_c46_insert_into_tombstone() {
    let mut rng = Rng::new(46);
    with_libs(0x31415926, |c, rs| {
        let cf = cfg("e16k8", 16, 8, HM_BINARY, Arena::Auto, false, true);
        for _ in 0..40 {
            let ks = KeySet::make(&cf, &mut rng, 80);
            let mut m = pair(c, rs, &cf);
            for i in 0..40 {
                m.put(ks.ptr(i), i as u64);
            }
            // delete a few, then insert new keys so the probe hits tombstones
            for i in 0..10 {
                m.del(ks.ptr(i), 0);
            }
            m.check("c46 after deletes");
            for i in 40..80 {
                m.put(ks.ptr(i), i as u64);
                m.check(&format!("c46 reinsert #{i}"));
            }
            // and re-insert the deleted keys
            for i in 0..10 {
                m.put(ks.ptr(i), 0x9000 + i as u64);
                m.check(&format!("c46 revive #{i}"));
            }
            m.free();
        }
    });
}

// --- C47 / C48 --------------------------------------------------------------
fn random_pipeline(c: &Api, rs: &Api, cf: &Cfg, rng: &mut Rng, ops: usize) {
    let ks = KeySet::make(cf, rng, 200);
    let mut m = pair(c, rs, cf);
    let mut live = vec![false; ks.len()];
    for op in 0..ops {
        let which = rng.below(100);
        let i = rng.below(ks.len());
        if which < 45 {
            if !cf.dups_ok && live[i] {
                continue;
            }
            m.put(ks.ptr(i), (op as u64) << 8 | i as u64);
            live[i] = true;
        } else if which < 75 {
            m.get(ks.ptr(i));
        } else if which < 90 {
            m.get_ts(ks.ptr(i));
        } else {
            let t = m.del(ks.ptr(i), 0);
            if t == 1 {
                live[i] = false;
            }
        }
        m.check(&format!("{} op#{op}", cf.name));
    }
    m.free();
}

#[test]
fn cfg_c47_random_pipeline_binary() {
    let mut rng = Rng::new(47);
    for seed in [0usize, 0x31415926, usize::MAX] {
        with_libs(seed, |c, rs| {
            for cf in CFGS.iter().filter(|c| c.mode == HM_BINARY && c.lookups_ok()) {
                random_pipeline(c, rs, cf, &mut rng, 700);
            }
        });
    }
}

#[test]
fn cfg_c48_random_pipeline_string() {
    let mut rng = Rng::new(48);
    for seed in [0usize, 0x31415926, usize::MAX] {
        with_libs(seed, |c, rs| {
            for cf in CFGS.iter().filter(|c| c.mode == HM_STRING && c.lookups_ok()) {
                random_pipeline(c, rs, cf, &mut rng, 700);
            }
        });
    }
}

// --- C49 --------------------------------------------------------------------
#[test]
fn cfg_c49_hmfree_all_modes() {
    let mut rng = Rng::new(49);
    with_libs(0x31415926, |c, rs| {
        for cf in CFGS {
            for n in [0usize, 1, 7, 40, 300] {
                let ks = KeySet::make(cf, &mut rng, n);
                let n = if n == 0 { 0 } else { ks.len() };
                let mut m = pair(c, rs, cf);
                for i in 0..n {
                    m.put(ks.ptr(i), i as u64);
                }
                m.check(&format!("c49 {} n={n} before free", cf.name));
                m.free(); // must not double-free / leak-crash in any mode
            }
        }
    });
}

// --- C61 --------------------------------------------------------------------
#[test]
fn cfg_c61_mode_out_of_range_valid_path() {
    // `mode >= STBDS_HM_STRING` (not `== 1`) selects string behaviour.
    let mut rng = Rng::new(61);
    with_libs(0x31415926, |c, rs| {
        // string-ish modes
        for mode in [1i32, 2, 7, 1000, c_int::MAX] {
            let cf = cfg("oor/str", 16, 8, mode, Arena::Explicit(SH_STRDUP), true, true);
            let ks = KeySet::make(&cf, &mut rng, 25);
            let mut m = pair(c, rs, &cf);
            for i in 0..25 {
                m.put(ks.ptr(i), i as u64);
                m.check(&format!("c61 mode={mode} put#{i}"));
            }
            for i in 0..25 {
                assert!(m.get(ks.ptr(i)) >= 0, "mode={mode} key {i}");
            }
            m.free();
        }
        // binary-ish modes
        for mode in [0i32, -1, -7, c_int::MIN] {
            let cf = cfg("oor/bin", 16, 8, mode, Arena::Auto, false, true);
            let ks = KeySet::make(&cf, &mut rng, 25);
            let mut m = pair(c, rs, &cf);
            for i in 0..25 {
                m.put(ks.ptr(i), i as u64);
                m.check(&format!("c61 mode={mode} put#{i}"));
            }
            for i in 0..25 {
                assert!(m.get(ks.ptr(i)) >= 0, "mode={mode} key {i}");
            }
            for i in 0..25 {
                m.del(ks.ptr(i), 0);
                m.check(&format!("c61 mode={mode} del#{i}"));
            }
            m.free();
        }
    });
}
