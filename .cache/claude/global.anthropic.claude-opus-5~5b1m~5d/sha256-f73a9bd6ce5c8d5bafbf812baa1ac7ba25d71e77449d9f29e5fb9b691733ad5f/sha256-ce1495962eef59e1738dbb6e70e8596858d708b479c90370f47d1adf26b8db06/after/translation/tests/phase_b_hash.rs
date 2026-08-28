//! Phase B — CONFIGS.md rows 1-9: the hash primitives and the global seed.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn seeds() -> Vec<usize> {
    let mut v = vec![0usize, 1, 2, DEFAULT_SEED, usize::MAX, usize::MAX - 1, 1 << 63];
    let mut rng = Rng::new(0xA5A5_1234);
    for _ in 0..64 {
        v.push(rng.next_u64() as usize);
    }
    v
}

/// row 1 — `stbds_hash_bytes` with `len == 0` (pointer never dereferenced)
#[test]
fn hash_bytes_len0() {
    let p = seeded(DEFAULT_SEED);
    let mut buf = [0u8; 8];
    for s in seeds() {
        let hc = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s) };
        let hr = unsafe { (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s) };
        assert_eq!(hc, hr, "hash_bytes(len=0, seed={s:#x})");
    }
}

/// row 2 — tail `switch` fall-through: every `case 1..7`
#[test]
fn hash_bytes_tail() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0001);
    let mut trace_c = Vec::new();
    let mut trace_r = Vec::new();
    for len in 0usize..8 {
        for _ in 0..400 {
            let mut b = rng.bytes(8);
            for s in [0usize, 1, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
                let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                trace_c.push(format!("len={len} seed={s:#x} bytes={} -> {hc:#x}", hex(&b[..len])));
                trace_r.push(format!("len={len} seed={s:#x} bytes={} -> {hr:#x}", hex(&b[..len])));
            }
        }
    }
    assert_traces_eq("hash_bytes tail", &trace_c, &trace_r);
}

/// row 3 — whole words only (`len % 8 == 0`, main loop, empty tail)
#[test]
fn hash_bytes_words() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0002);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for words in 1usize..=8 {
        let len = words * 8;
        for _ in 0..200 {
            let mut b = rng.bytes(len);
            for s in [0usize, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
                let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                tc.push(format!("{len} {s:#x} {} {hc:#x}", hex(&b)));
                tr.push(format!("{len} {s:#x} {} {hr:#x}", hex(&b)));
            }
        }
    }
    assert_traces_eq("hash_bytes words", &tc, &tr);
}

/// row 4 — main loop *and* tail
#[test]
fn hash_bytes_mixed() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0003);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for len in [9usize, 10, 15, 16, 17, 23, 31, 32, 33, 63, 64, 65, 127, 128, 200, 255] {
        for _ in 0..60 {
            let mut b = rng.bytes(len);
            for s in [0usize, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
                let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                tc.push(format!("{len} {s:#x} {} {hc:#x}", hex(&b)));
                tr.push(format!("{len} {s:#x} {} {hr:#x}", hex(&b)));
            }
        }
    }
    assert_traces_eq("hash_bytes mixed", &tc, &tr);
}

/// row 5 — the C integer-promotion sign-extension quirk: bytes >= 0x80 at
/// index 3 and 7 of each 8-byte group make `d[3] << 24` negative.
#[test]
fn hash_bytes_signext() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0004);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for len in [4usize, 5, 6, 7, 8, 12, 16, 20, 24] {
        for _ in 0..200 {
            let mut b = rng.bytes(len);
            // force the high bit at every position that feeds a << 24 shift
            for i in 0..len {
                if i % 8 == 3 || i % 8 == 7 {
                    b[i] |= 0x80;
                }
            }
            for s in [0usize, DEFAULT_SEED, usize::MAX] {
                let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                tc.push(format!("{len} {s:#x} {} {hc:#x}", hex(&b)));
                tr.push(format!("{len} {s:#x} {} {hr:#x}", hex(&b)));
            }
        }
    }
    // also the all-0xff and all-0x80 extremes
    for len in 0usize..=32 {
        for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
            let mut b = vec![fill; 32];
            for s in [0usize, DEFAULT_SEED, usize::MAX] {
                let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
                tc.push(format!("fill{fill:02x} {len} {s:#x} {hc:#x}"));
                tr.push(format!("fill{fill:02x} {len} {s:#x} {hr:#x}"));
            }
        }
    }
    assert_traces_eq("hash_bytes signext", &tc, &tr);
}

/// row 6 — seed sweep
#[test]
fn hash_bytes_seeds() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0005);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for s in seeds() {
        for len in 0usize..=40 {
            let mut b = rng.bytes(len.max(1));
            let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
            let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s) };
            tc.push(format!("{s:#x} {len} {hc:#x}"));
            tr.push(format!("{s:#x} {len} {hr:#x}"));
        }
    }
    assert_traces_eq("hash_bytes seeds", &tc, &tr);
}

/// row 7 — `stbds_hash_string` shapes, including bytes >= 0x80 (the C casts
/// through `(unsigned char)`)
#[test]
fn hash_string_shapes() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0006);
    let mut tc = Vec::new();
    let mut tr = Vec::new();

    let mut fixed: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"ab\0".to_vec(),
        b"test_0\0".to_vec(),
        b"test_123456\0".to_vec(),
        vec![0xffu8, 0],
        vec![0x80u8, 0x81, 0xfe, 0xff, 0],
        vec![0x41u8; 65],
    ];
    fixed.last_mut().unwrap().push(0);
    for b in fixed.iter_mut() {
        for s in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            let hc = unsafe { (p.c.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
            let hr = unsafe { (p.r.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
            tc.push(format!("{} {s:#x} {hc:#x}", hex(b)));
            tr.push(format!("{} {s:#x} {hr:#x}", hex(b)));
        }
    }
    for len in 0usize..=64 {
        for _ in 0..40 {
            let mut b = rng.nul_free(len);
            for s in [0usize, DEFAULT_SEED, usize::MAX, rng.next_u64() as usize] {
                let hc = unsafe { (p.c.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
                let hr = unsafe { (p.r.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
                tc.push(format!("{} {s:#x} {hc:#x}", hex(&b)));
                tr.push(format!("{} {s:#x} {hr:#x}", hex(&b)));
            }
        }
    }
    assert_traces_eq("hash_string shapes", &tc, &tr);
}

/// row 8 — `stbds_hash_string` seed sweep
#[test]
fn hash_string_seeds() {
    let p = seeded(DEFAULT_SEED);
    let mut rng = Rng::new(0xBEEF_0007);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for s in seeds() {
        for len in [0usize, 1, 2, 3, 7, 8, 9, 31, 32, 33, 100] {
            let mut b = rng.nul_free(len);
            let hc = unsafe { (p.c.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
            let hr = unsafe { (p.r.hash_string)(b.as_mut_ptr() as *mut c_char, s) };
            tc.push(format!("{s:#x} {len} {hc:#x}"));
            tr.push(format!("{s:#x} {len} {hr:#x}"));
        }
    }
    assert_traces_eq("hash_string seeds", &tc, &tr);
}

/// row 9 — `stbds_rand_seed`: the seed handed to each new table and the
/// `seed = seed*a + b` advance, observed through `table->seed`.
#[test]
fn rand_seed_sequence() {
    let mut rng = Rng::new(0xBEEF_0008);
    let mut starts = vec![0usize, 1, DEFAULT_SEED, usize::MAX, usize::MAX / 3];
    for _ in 0..8 {
        starts.push(rng.next_u64() as usize);
    }
    for s in starts {
        let p = seeded(s);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            unsafe {
                (api.rand_seed)(s);
                for k in 0..12 {
                    let h = (api.shmode_func)(8, SH_NONE);
                    let tbl = map_table(h, 8);
                    t.push(format!("k={k} table.seed={:#x}", (*tbl).seed));
                    t.extend(snap_map(h, 8, KeyKind::Binary));
                    (api.hmfree_func)(map_raw(h, 8) as *mut c_void, 8);
                }
                // rehash inherits the seed instead of consuming a new one
                let mut hh = (api.shmode_func)(8, SH_NONE);
                let mut keys: Vec<Box<[u8]>> = Vec::new();
                for i in 0..40u32 {
                    let mut kb: Box<[u8]> = i.to_le_bytes().to_vec().into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keys.push(kb);
                    hh = (api.hmput_key)(hh, 8, kp, 4, HM_BINARY);
                }
                let tbl = map_table(hh, 8);
                t.push(format!("after 40 puts table.seed={:#x}", (*tbl).seed));
                t.push(format!("slot_count={}", (*tbl).slot_count));
                (api.hmfree_func)(map_raw(hh, 8) as *mut c_void, 8);
                // and the next fresh table gets the advanced global seed
                let h2 = (api.shmode_func)(8, SH_NONE);
                t.push(format!("next fresh seed={:#x}", (*map_table(h2, 8)).seed));
                (api.hmfree_func)(map_raw(h2, 8) as *mut c_void, 8);
            }
        }
        assert_traces_eq(&format!("rand_seed({s:#x})"), &tc, &tr);
    }
}
