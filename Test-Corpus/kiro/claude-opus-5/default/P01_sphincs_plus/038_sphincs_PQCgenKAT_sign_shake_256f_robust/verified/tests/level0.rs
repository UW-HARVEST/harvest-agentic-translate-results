//! Phase B, level 0 — `utils.c` and `address.c` (`CONFIGS.md` rows C1–C9).

mod common;
use common::*;

type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type U32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
type SetU32 = unsafe extern "C" fn(*mut u32, u32);
type SetU64 = unsafe extern "C" fn(*mut u32, u64);
type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
type ComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
type TreeHash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u32,
    u32,
    u32,
    GenLeafFn,
    *mut u32,
);

/* ---- C1 ---------------------------------------------------------- */

#[test]
fn c01_ull_to_bytes() {
    let l = libs();
    let (c, r) = l.pair::<UllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(SEED);
    let mut fixed = vec![0u64, 1, 0xff, 0x100, u64::MAX, u64::MAX - 1, 0x0123_4567_89ab_cdef];
    for _ in 0..NUM_ITERS {
        fixed.push(rng.next_u64());
    }
    for outlen in 0u32..=16 {
        for &v in &fixed {
            // Guard bytes on both sides so an over-write is caught.
            let mut cb = vec![0xAAu8; 32];
            let mut rb = vec![0xAAu8; 32];
            unsafe {
                c(cb.as_mut_ptr().add(4), outlen, v);
                r(rb.as_mut_ptr().add(4), outlen, v);
            }
            eq_bytes(&format!("ull_to_bytes(outlen={outlen}, v={v:#x})"), &cb, &rb);
        }
    }
}

/* ---- C2 ---------------------------------------------------------- */

#[test]
fn c02_u32_to_bytes() {
    let l = libs();
    let (c, r) = l.pair::<U32ToBytes>("SPX_u32_to_bytes");
    let mut rng = Rng::new(SEED + 2);
    let mut vals = vec![0u32, 1, 0xff, 0x100, 0xffff, u32::MAX];
    for _ in 0..NUM_ITERS {
        vals.push(rng.next_u32());
    }
    for v in vals {
        let mut cb = [0xAAu8; 12];
        let mut rb = [0xAAu8; 12];
        unsafe {
            c(cb.as_mut_ptr().add(4), v);
            r(rb.as_mut_ptr().add(4), v);
        }
        eq_bytes(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

/* ---- C3 ---------------------------------------------------------- */

#[test]
fn c03_bytes_to_ull() {
    let l = libs();
    let (c, r) = l.pair::<BytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(SEED + 3);
    for inlen in 0u32..=8 {
        for _ in 0..NUM_ITERS {
            let b = rng.bytes(16);
            let (cv, rv) = unsafe { (c(b.as_ptr(), inlen), r(b.as_ptr(), inlen)) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={inlen}, {})", hex(&b[..16]));
        }
        // extremes
        for pat in [0x00u8, 0xff] {
            let b = vec![pat; 16];
            let (cv, rv) = unsafe { (c(b.as_ptr(), inlen), r(b.as_ptr(), inlen)) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={inlen}, all {pat:#02x})");
        }
    }
}

/* ---- C4 ---------------------------------------------------------- */

#[test]
fn c04_ull_roundtrip() {
    let l = libs();
    let (cw, rw) = l.pair::<UllToBytes>("SPX_ull_to_bytes");
    let (cr, rr) = l.pair::<BytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(SEED + 4);
    for outlen in 1u32..=8 {
        for _ in 0..NUM_ITERS {
            let v = rng.next_u64();
            let mut cb = [0u8; 8];
            let mut rb = [0u8; 8];
            unsafe {
                cw(cb.as_mut_ptr(), outlen, v);
                rw(rb.as_mut_ptr(), outlen, v);
            }
            eq_bytes("ull_to_bytes roundtrip write", &cb, &rb);
            let (cv, rv) = unsafe { (cr(cb.as_ptr(), outlen), rr(rb.as_ptr(), outlen)) };
            assert_eq!(cv, rv, "roundtrip(outlen={outlen}, v={v:#x})");
        }
    }
}

/* ---- C5 ---------------------------------------------------------- */

#[test]
fn c05_addr_setters_sequence() {
    let l = libs();
    let mut rng = Rng::new(SEED + 5);

    let u32_setters: [&str; 6] = [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_keypair_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ];

    for _ in 0..NUM_ITERS {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;

        // Interleave every setter, checking the whole 32-byte ADRS each time.
        for name in u32_setters {
            let (c, r) = l.pair::<SetU32>(name);
            let v = match name {
                "SPX_set_layer_addr" => rng.below(SPX_D as u64) as u32,
                "SPX_set_type" => rng.below(7) as u32,
                "SPX_set_keypair_addr" => rng.next_u32() & ((1 << SPX_TREE_HEIGHT) - 1),
                "SPX_set_chain_addr" => rng.below(SPX_WOTS_LEN as u64) as u32,
                "SPX_set_hash_addr" => rng.below(SPX_WOTS_W as u64) as u32,
                _ => rng.below(SPX_TREE_HEIGHT as u64 + 1) as u32,
            };
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(&format!("{name}({v})"), as_bytes(&ca), as_bytes(&ra));
        }

        let (c, r) = l.pair::<SetU32>("SPX_set_tree_index");
        let v = rng.next_u32();
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        eq_bytes("SPX_set_tree_index", as_bytes(&ca), as_bytes(&ra));

        let (c, r) = l.pair::<SetU64>("SPX_set_tree_addr");
        let t = rng.next_u64() >> (64 - SPX_TREE_HEIGHT * (SPX_D - 1)).min(63);
        unsafe {
            c(ca.as_mut_ptr(), t);
            r(ra.as_mut_ptr(), t);
        }
        eq_bytes("SPX_set_tree_addr", as_bytes(&ca), as_bytes(&ra));
    }
}

fn as_bytes(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

/* ---- C6 ---------------------------------------------------------- */

#[test]
fn c06_addr_copy() {
    let l = libs();
    let mut rng = Rng::new(SEED + 6);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let (c, r) = l.pair::<CopyAddr>(name);
        for _ in 0..NUM_ITERS {
            let src = rng.addr();
            let dst = rng.addr(); // pre-filled: the *unwritten* bytes matter too
            let mut cd = dst;
            let mut rd = dst;
            unsafe {
                c(cd.as_mut_ptr(), src.as_ptr());
                r(rd.as_mut_ptr(), src.as_ptr());
            }
            eq_bytes(name, as_bytes(&cd), as_bytes(&rd));
        }
    }
}

/* ---- C7 ---------------------------------------------------------- */

#[test]
fn c07_set_type_all_valid() {
    let l = libs();
    let (c, r) = l.pair::<SetU32>("SPX_set_type");
    let mut rng = Rng::new(SEED + 7);
    for t in ADDR_TYPES {
        for _ in 0..NUM_ITERS {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), t);
                r(ra.as_mut_ptr(), t);
            }
            eq_bytes(&format!("set_type({t})"), as_bytes(&ca), as_bytes(&ra));
            assert_eq!(as_bytes(&ca)[off::TYPE], t as u8);
        }
    }
}

/* ---- C8 ---------------------------------------------------------- */

#[test]
fn c08_compute_root() {
    let l = libs();
    let (c, r) = l.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(SEED + 8);

    let heights: Vec<u32> = {
        let mut v = vec![1u32, 2, SPX_FORS_HEIGHT, SPX_TREE_HEIGHT];
        v.sort_unstable();
        v.dedup();
        v
    };

    for &h in &heights {
        let mut leaf_idxs = vec![0u32, 1, (1u32 << h) - 1];
        if h >= 2 {
            leaf_idxs.push((1u32 << h) - 2);
        }
        for _ in 0..4 {
            leaf_idxs.push(rng.next_u32() & ((1u32 << h) - 1));
        }
        for &ty in &[3u32 /* FORSTREE */, 2 /* HASHTREE */] {
            for &leaf_idx in &leaf_idxs {
                for iter in 0..NUM_ITERS {
                    let pub_seed = rng.bytes(SPX_N);
                    let sk_seed = rng.bytes(SPX_N);
                    let (cc, rc) = init_ctx_pair(&pub_seed, &sk_seed);
                    let leaf = rng.bytes(SPX_N);
                    let auth = rng.bytes(h as usize * SPX_N);
                    let idx_offset = if iter % 2 == 0 { 0 } else { rng.next_u32() >> 4 };
                    let mut addr = rng.addr();
                    // set_type is applied via the byte offset so no extra call
                    as_bytes_mut(&mut addr)[off::TYPE] = ty as u8;
                    let mut ca = addr;
                    let mut ra = addr;
                    let mut croot = vec![0u8; SPX_N];
                    let mut rroot = vec![0u8; SPX_N];
                    unsafe {
                        c(
                            croot.as_mut_ptr(),
                            leaf.as_ptr(),
                            leaf_idx,
                            idx_offset,
                            auth.as_ptr(),
                            h,
                            cc.as_ptr(),
                            ca.as_mut_ptr(),
                        );
                        r(
                            rroot.as_mut_ptr(),
                            leaf.as_ptr(),
                            leaf_idx,
                            idx_offset,
                            auth.as_ptr(),
                            h,
                            rc.as_ptr(),
                            ra.as_mut_ptr(),
                        );
                    }
                    eq_bytes(
                        &format!("compute_root(h={h},leaf_idx={leaf_idx},off={idx_offset},ty={ty}) root"),
                        &croot,
                        &rroot,
                    );
                    eq_bytes(
                        "compute_root mutated addr",
                        as_bytes(&ca),
                        as_bytes(&ra),
                    );
                }
            }
        }
    }
}

fn as_bytes_mut(a: &mut [u32; 8]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(a.as_mut_ptr() as *mut u8, 32) }
}

/* ---- C9 ---------------------------------------------------------- */

/// Deterministic `gen_leaf` callback: both libraries get the *same* leaves, so
/// only `treehash`'s own stack / auth-path logic is under test.
unsafe extern "C" fn synth_leaf(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, tree_addr: *const u32) {
    let ta = std::slice::from_raw_parts(tree_addr, 8);
    let mut h = 0x1234_5678_9abc_def0u64 ^ (addr_idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for w in ta {
        h ^= (*w as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = h.rotate_left(17).wrapping_add(0x94D0_49BB_1331_11EB);
    }
    let out = std::slice::from_raw_parts_mut(leaf, SPX_N);
    for (i, b) in out.iter_mut().enumerate() {
        h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(i as u64);
        *b = (h >> 24) as u8;
    }
}

#[test]
fn c09_treehash_callback() {
    let l = libs();
    let (c, r) = l.pair::<TreeHash>("SPX_treehash");
    let mut rng = Rng::new(SEED + 9);

    for h in 0u32..=3 {
        for leaf_idx in 0u32..(1u32 << h) {
            for iter in 0..NUM_ITERS {
                let pub_seed = rng.bytes(SPX_N);
                let sk_seed = rng.bytes(SPX_N);
                let (cc, rc) = init_ctx_pair(&pub_seed, &sk_seed);
                let idx_offset = if iter % 3 == 0 {
                    0
                } else {
                    (rng.next_u32() >> 8) & !((1u32 << h) - 1)
                };
                let mut ta = rng.addr();
                as_bytes_mut(&mut ta)[off::TYPE] = 2; // HASHTREE
                let mut cta = ta;
                let mut rta = ta;
                let apl = (h as usize + 1) * SPX_N;
                let mut cauth = vec![0xCCu8; apl];
                let mut rauth = vec![0xCCu8; apl];
                let mut croot = vec![0u8; SPX_N];
                let mut rroot = vec![0u8; SPX_N];
                unsafe {
                    c(
                        croot.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        cc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        synth_leaf,
                        cta.as_mut_ptr(),
                    );
                    r(
                        rroot.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        rc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        synth_leaf,
                        rta.as_mut_ptr(),
                    );
                }
                let tag = format!("treehash(h={h},leaf_idx={leaf_idx},off={idx_offset})");
                eq_bytes(&format!("{tag} root"), &croot, &rroot);
                eq_bytes(&format!("{tag} auth_path"), &cauth, &rauth);
                eq_bytes(&format!("{tag} tree_addr"), as_bytes(&cta), as_bytes(&rta));
            }
        }
    }
}

/* ---- constants cross-check --------------------------------------- */

type SizeFn = unsafe extern "C" fn() -> u64;

/// The harness re-derives the parameter constants from the Cargo features; this
/// checks them against what the *C* library reports, so a wrong constant in the
/// harness cannot silently weaken every other test.
#[test]
fn c00_harness_constants_match_c() {
    let l = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let c = l.c::<SizeFn>(name);
        assert_eq!(unsafe { c() }, expect, "harness constant for {name} is wrong");
    }
}
