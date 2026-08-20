//! Extra property-style stress on top of the CONFIGS.md rows: wide random
//! cross-products of every runtime axis, driven through the low-level exported
//! entry points with a deep state comparison after every single operation.

mod common;
use common::*;

/// Random walk over the whole *binary*-mode axis cross-product:
/// `elemsize` x `keysize` x `mode` x table `string.mode` x op mix x global seed.
#[test]
fn random_axis_cross_product_binary() {
    let _g = lock();
    let mut outer = Rng::new(0x57_7E_55);
    for run in 0..40usize {
        let gseed = outer.next_u64() as usize;
        sync_seed(gseed);
        let mut rng = Rng::new(outer.next_u64());

        let keysize = [1usize, 2, 4, 8, 12, 16][rng.below(6)];
        let extra = [0usize, 1, 4, 8, 16][rng.below(5)];
        let elemsize = keysize + extra;
        // pre-create the table in a random string.mode about half the time; the
        // `default:` memcpy arm is the one binary keys go through, and the other
        // arms are reached by pre-seeding the table with `shmode_func`.
        let pre = [None, Some(SH_NONE), Some(0x100), Some(4)][rng.below(4)];
        let mode: i32 = [0i32, -1, -7, i32::MIN][rng.below(4)];

        let mut m = Dual::new(elemsize, false);
        if let Some(sm) = pre {
            m.shmode(sm);
        }
        if rng.below(3) == 0 {
            let sentinel = rng.bytes(elemsize);
            m.put_default(&sentinel);
            m.check(&format!("stress run {run} default"));
        }

        let domain = [4usize, 16, 64, 300][rng.below(4)];
        let mut live: Vec<Vec<u8>> = Vec::new();
        let ops = 150 + rng.below(250);
        for step in 0..ops {
            let roll = rng.below(100);
            if roll < 50 {
                let mut key = rng.bytes(keysize);
                key[0] = (rng.below(domain) % 256) as u8;
                if keysize > 1 {
                    key[1] = (rng.below(domain) / 256) as u8;
                }
                let payload = rng.bytes(extra);
                let (a, b) = m.put_bin(&key, keysize, &payload, mode);
                assert_eq!(
                    a, b,
                    "run {run} step {step} put diverged (es={elemsize} ks={keysize} mode={mode} pre={pre:?})"
                );
                if !live.contains(&key) {
                    live.push(key);
                }
            } else if roll < 80 {
                let key = if !live.is_empty() && rng.below(4) != 0 {
                    live[rng.below(live.len())].clone()
                } else {
                    let mut k = rng.bytes(keysize);
                    k[0] = (rng.below(domain) % 256) as u8;
                    k
                };
                let keyoffset = if extra >= 8 && rng.below(8) == 0 { 0 } else { 0 };
                let (a, b) = m.del(&key, keysize, keyoffset, mode, false);
                assert_eq!(a, b, "run {run} step {step} del diverged");
                live.retain(|x| x != &key);
            } else if roll < 90 {
                let mut key = rng.bytes(keysize);
                key[0] = (rng.below(domain) % 256) as u8;
                let (a, b) = m.get(&key, keysize, mode, false);
                assert_eq!(a, b, "run {run} step {step} get diverged");
            } else {
                let mut key = rng.bytes(keysize);
                key[0] = (rng.below(domain) % 256) as u8;
                let (a, b) = m.get_ts(&key, keysize, mode, false);
                assert_eq!(a, b, "run {run} step {step} get_ts diverged");
            }
            m.check(&format!(
                "stress run {run} step {step} (es={elemsize} ks={keysize} mode={mode} pre={pre:?} gseed={gseed:#x})"
            ));
            assert_eq!(
                m.len().0,
                live.len() as isize,
                "run {run} step {step} length wrong"
            );
        }
        m.free();
    }
}

/// Same idea for the three *string* key-ownership modes.
#[test]
fn random_axis_cross_product_string() {
    let _g = lock();
    let mut outer = Rng::new(0x5A_11_ED);
    for run in 0..40usize {
        let gseed = outer.next_u64() as usize;
        sync_seed(gseed);
        let mut rng = Rng::new(outer.next_u64());

        let extra = [0usize, 8, 16, 24][rng.below(4)];
        let elemsize = 8 + extra;
        let sm = [SH_DEFAULT, SH_STRDUP, SH_ARENA][rng.below(3)];
        let mode: i32 = [1i32, 2, 5, i32::MAX][rng.below(4)];
        let explicit = rng.below(2) == 0;

        let mut m = Dual::new(elemsize, true);
        if explicit {
            m.shmode(sm);
        }
        // when the table is created implicitly by hmput_key the mode is
        // STBDS_SH_DEFAULT, whatever `sm` said
        let eff = if explicit { sm } else { SH_DEFAULT };

        let alpha = [2u8, 4, 8, 26][rng.below(4)];
        let maxlen = [1usize, 4, 12, 40][rng.below(4)];
        let mut live: Vec<Vec<u8>> = Vec::new();
        let ops = 120 + rng.below(200);
        for step in 0..ops {
            let roll = rng.below(100);
            let fresh = || -> Vec<u8> { Vec::new() };
            let _ = fresh;
            if roll < 50 {
                let key = rng.cbytes_len(0, maxlen + 1, b'a', b'a' + alpha - 1);
                let payload = rng.bytes(extra);
                let (a, b) = m.put_str(&key, &payload, mode);
                assert_eq!(
                    a, b,
                    "run {run} step {step} str put diverged (es={elemsize} sm={eff} mode={mode})"
                );
                if !live.contains(&key) {
                    live.push(key);
                }
            } else if roll < 78 {
                let key = if !live.is_empty() && rng.below(4) != 0 {
                    live[rng.below(live.len())].clone()
                } else {
                    rng.cbytes_len(0, maxlen + 1, b'a', b'a' + alpha - 1)
                };
                // hmdel_key with mode != 1 and a memmove fix-up hashes raw
                // pointer bytes (documented in ERRORS.md); restrict deletes to
                // mode == 1 so the workload stays address-independent.
                if mode == 1 {
                    let (a, b) = m.del(&key, 8, 0, 1, true);
                    assert_eq!(a, b, "run {run} step {step} str del diverged");
                    live.retain(|x| x != &key);
                }
            } else if roll < 90 {
                let key = rng.cbytes_len(0, maxlen + 1, b'a', b'a' + alpha - 1);
                let (a, b) = m.get(&key, 8, mode, true);
                assert_eq!(a, b, "run {run} step {step} str get diverged");
            } else {
                let key = rng.cbytes_len(0, maxlen + 1, b'a', b'a' + alpha - 1);
                let (a, b) = m.get_ts(&key, 8, mode, true);
                assert_eq!(a, b, "run {run} step {step} str get_ts diverged");
            }
            m.check(&format!(
                "str stress run {run} step {step} (es={elemsize} sm={eff} mode={mode} gseed={gseed:#x})"
            ));
            assert_eq!(m.len().0, live.len() as isize, "run {run} step {step} length");
        }
        m.free();
    }
}

/// Long / odd hash inputs beyond the sizes used by `hash.rs`.
#[test]
fn hash_bytes_large() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xB16);
    for len in [255usize, 256, 257, 511, 512, 1023, 1024, 4095, 4096, 65535] {
        for _ in 0..4 {
            let buf = rng.bytes(len);
            let b = CBuf::new(&buf);
            let seed = rng.next_u64() as usize;
            unsafe {
                assert_eq!(
                    (c.hash_bytes)(b.as_void(), len, seed),
                    (r.hash_bytes)(b.as_void(), len, seed),
                    "hash_bytes len={len} seed={seed:#x}"
                );
            }
        }
    }
    // every length 0..=200 with a fixed pattern, over several seeds
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        for len in 0..=200usize {
            let buf: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37) ^ 0x80).collect();
            let b = CBuf::new(&buf);
            unsafe {
                assert_eq!(
                    (c.hash_bytes)(b.as_void(), len, seed),
                    (r.hash_bytes)(b.as_void(), len, seed),
                    "hash_bytes pattern len={len} seed={seed:#x}"
                );
            }
        }
    }
}

/// Long / odd string inputs beyond the sizes used by `hash.rs`.
#[test]
fn hash_string_large() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xB17);
    for len in [255usize, 256, 1023, 1024, 16384, 65535] {
        let s = rng.cbytes(len, 0x01, 0xff);
        let b = CBuf::cstr(&s);
        for seed in [0usize, 1, usize::MAX, 0x3141_5926] {
            unsafe {
                assert_eq!(
                    (c.hash_string)(b.as_char(), seed),
                    (r.hash_string)(b.as_char(), seed),
                    "hash_string len={len} seed={seed:#x}"
                );
            }
        }
    }
    // every length 0..=200
    for len in 0..=200usize {
        let s: Vec<u8> = (0..len).map(|i| 1 + ((i * 7) % 255) as u8).collect();
        let b = CBuf::cstr(&s);
        unsafe {
            assert_eq!(
                (c.hash_string)(b.as_char(), 0x3141_5926),
                (r.hash_string)(b.as_char(), 0x3141_5926),
                "hash_string pattern len={len}"
            );
        }
    }
}

/// Many independent maps alive at once — each `make_hash_index` advances the
/// global seed, so the interleaving order is itself part of the state.
#[test]
fn interleaved_maps() {
    let _g = lock();
    sync_seed(0x1DEA);
    let es = 16usize;
    let mut maps: Vec<Dual> = (0..8).map(|_| Dual::new(es, false)).collect();
    let mut live: Vec<Vec<i64>> = vec![Vec::new(); 8];
    let mut rng = Rng::new(0x1DEA);
    for step in 0..1200usize {
        let idx = rng.below(maps.len());
        let m = &mut maps[idx];
        if rng.below(100) < 60 {
            let k = (rng.next_u64() % 150) as i64;
            let (a, b) = m.put_bin(&le64(k), 8, &le64(step as i64), HM_BINARY);
            assert_eq!(a, b, "interleaved put diverged (map {idx} step {step})");
            if !live[idx].contains(&k) {
                live[idx].push(k);
            }
        } else if !live[idx].is_empty() {
            let k = live[idx][rng.below(live[idx].len())];
            let (a, b) = m.del(&le64(k), 8, 0, HM_BINARY, false);
            assert_eq!(a, b, "interleaved del diverged (map {idx} step {step})");
            live[idx].retain(|&x| x != k);
        }
        m.check(&format!("interleaved map {idx} step {step}"));
        assert_eq!(m.len().0, live[idx].len() as isize);
    }
    for m in maps.iter_mut() {
        m.free();
    }
}

/// Arenas used concurrently with hash maps that own arenas of their own.
#[test]
fn interleaved_arenas_and_maps() {
    let _g = lock();
    let (c, r) = pair();
    sync_seed(0xA4E4A);
    let es = 16usize;
    let mut m = Dual::new(es, true);
    m.shmode(SH_ARENA);
    let mut ac = StringArena::new();
    let mut ar = StringArena::new();
    let mut rng = Rng::new(0xA4E4A);
    for step in 0..600usize {
        if rng.below(2) == 0 {
            let s = rng.cbytes_len(0, 40, b'a', b'z');
            let buf = CBuf::cstr(&s);
            unsafe {
                let pc = (c.stralloc)(&mut ac, buf.as_char());
                let pr = (r.stralloc)(&mut ar, buf.as_char());
                assert_eq!(cstr(pc), cstr(pr), "standalone arena content step {step}");
                assert_eq!(
                    (ac.remaining, ac.block, ac.mode),
                    (ar.remaining, ar.block, ar.mode),
                    "standalone arena state step {step}"
                );
            }
        } else {
            let key = rng.cbytes_len(1, 30, b'A', b'Z');
            let (a, b) = m.put_str(&key, &le64(step as i64), HM_STRING);
            assert_eq!(a, b, "arena-map put diverged step {step}");
            m.check(&format!("arena-map step {step}"));
        }
    }
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
    m.free();
}
