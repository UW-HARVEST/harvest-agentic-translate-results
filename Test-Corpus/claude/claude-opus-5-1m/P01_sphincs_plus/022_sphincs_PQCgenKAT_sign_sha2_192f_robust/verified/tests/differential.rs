//! Differential tests: the C reference `.so`s vs. the Rust `cdylib`, both
//! reached exclusively through `dlopen`/`dlsym`.
//!
//! * Phase B — `mod configs`: one test per row of `CONFIGS.md` (valid inputs).
//! * Phase C — `mod errors`:  one test per row of `ERRORS.md` (rejections).
//!
//! Run with `./run_all.sh` to cover all 48 feature combinations, or
//! `cargo test --release-so --no-default-features --features "<bk> <th> <sp>"`
//! after `cargo build --release` + `./build_c_all.sh`.

mod common;

use common::*;

// ###########################################################################
// Phase B — CONFIGS.md
// ###########################################################################

mod configs {
    use super::*;

    // ---------------------------------------------------------------- C1
    #[test]
    fn cfg_ull_to_bytes() {
        let _g = serial();
        let l = libs();
        let (cf, rf) = (l.c.ull_to_bytes(), l.r.ull_to_bytes());
        let mut rng = Rng::new(TEST_SEED ^ 1);
        let mut vals: Vec<u64> = vec![0, 1, 2, 255, 256, 0xFFFF, 1 << 31, 1 << 32, 1 << 63, u64::MAX];
        for _ in 0..iters(200) {
            vals.push(rng.next_u64());
        }
        for &outlen in &[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
            for &v in &vals {
                let mut cb = vec![0xAAu8; 32];
                let mut rb = vec![0xAAu8; 32];
                unsafe {
                    cf(cb.as_mut_ptr(), outlen, v);
                    rf(rb.as_mut_ptr(), outlen, v);
                }
                eqb("SPX_ull_to_bytes", &format!("outlen={outlen} in={v:#x}"), &cb, &rb);
            }
        }
    }

    // ---------------------------------------------------------------- C2
    #[test]
    fn cfg_u32_to_bytes() {
        let _g = serial();
        let l = libs();
        let (cf, rf) = (l.c.u32_to_bytes(), l.r.u32_to_bytes());
        let mut rng = Rng::new(TEST_SEED ^ 2);
        let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0xFF00, 0x00FF_0000, 0xFF00_0000, u32::MAX];
        for _ in 0..iters(500) {
            vals.push(rng.next_u32());
        }
        for &v in &vals {
            let mut cb = [0xAAu8; 8];
            let mut rb = [0xAAu8; 8];
            unsafe {
                cf(cb.as_mut_ptr(), v);
                rf(rb.as_mut_ptr(), v);
            }
            eqb("SPX_u32_to_bytes", &format!("in={v:#x}"), &cb, &rb);
        }
    }

    // ---------------------------------------------------------------- C3
    #[test]
    fn cfg_bytes_to_ull() {
        let _g = serial();
        let l = libs();
        let (cf, rf) = (l.c.bytes_to_ull(), l.r.bytes_to_ull());
        let mut rng = Rng::new(TEST_SEED ^ 3);
        for _ in 0..iters(300) {
            let b = rng.vec(16);
            for inlen in 0u32..=8 {
                let c = unsafe { cf(b.as_ptr(), inlen) };
                let r = unsafe { rf(b.as_ptr(), inlen) };
                eq("SPX_bytes_to_ull", &format!("inlen={inlen} in={b:02x?}"), c, r);
            }
        }
        for pat in [0u8, 0xFF] {
            let b = [pat; 16];
            for inlen in 0u32..=8 {
                let c = unsafe { cf(b.as_ptr(), inlen) };
                let r = unsafe { rf(b.as_ptr(), inlen) };
                eq("SPX_bytes_to_ull", &format!("pat={pat:#x} inlen={inlen}"), c, r);
            }
        }
    }

    // ---------------------------------------------------------------- C4
    #[test]
    fn cfg_address_setters() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 4);

        // Each setter, applied to identical starting states with identical values.
        for _ in 0..iters(300) {
            let start = rng.addr();
            let v32 = rng.next_u32();
            let v64 = rng.next_u64();

            macro_rules! one32 {
                ($name:literal, $get:ident, $val:expr) => {{
                    let mut ca = start;
                    let mut ra = start;
                    unsafe {
                        (l.c.$get())(ca.as_mut_ptr(), $val);
                        (l.r.$get())(ra.as_mut_ptr(), $val);
                    }
                    eqb(
                        concat!("SPX_", $name),
                        &format!("start={start:08x?} v={:#x}", $val),
                        addr_bytes(&ca),
                        addr_bytes(&ra),
                    );
                }};
            }
            one32!("set_layer_addr", set_layer_addr, v32);
            one32!("set_type", set_type, v32);
            one32!("set_keypair_addr", set_keypair_addr, v32);
            one32!("set_chain_addr", set_chain_addr, v32);
            one32!("set_hash_addr", set_hash_addr, v32);
            one32!("set_tree_height", set_tree_height, v32);
            one32!("set_tree_index", set_tree_index, v32);
            {
                let mut ca = start;
                let mut ra = start;
                unsafe {
                    (l.c.set_tree_addr())(ca.as_mut_ptr(), v64);
                    (l.r.set_tree_addr())(ra.as_mut_ptr(), v64);
                }
                eqb("SPX_set_tree_addr", &format!("v={v64:#x}"), addr_bytes(&ca), addr_bytes(&ra));
            }
        }

        // Boundary / "enum" values for every byte-wide field.
        for &v in &[0u32, 1, 2, 3, 4, 5, 6, 7, 127, 128, 254, 255, 256, 0x0000_01FF, 0xFFFF_FF01, u32::MAX] {
            let start = [0u32; 8];
            for (name, cg, rg) in [
                ("set_layer_addr", l.c.set_layer_addr(), l.r.set_layer_addr()),
                ("set_type", l.c.set_type(), l.r.set_type()),
                ("set_chain_addr", l.c.set_chain_addr(), l.r.set_chain_addr()),
                ("set_hash_addr", l.c.set_hash_addr(), l.r.set_hash_addr()),
                ("set_tree_height", l.c.set_tree_height(), l.r.set_tree_height()),
                ("set_keypair_addr", l.c.set_keypair_addr(), l.r.set_keypair_addr()),
                ("set_tree_index", l.c.set_tree_index(), l.r.set_tree_index()),
            ] {
                let mut ca = start;
                let mut ra = start;
                unsafe {
                    cg(ca.as_mut_ptr(), v);
                    rg(ra.as_mut_ptr(), v);
                }
                eqb(name, &format!("v={v:#x}"), addr_bytes(&ca), addr_bytes(&ra));
            }
        }

        // A random sequence of setters, to catch interactions between the
        // overlapping byte offsets (TREE_HGT == CHAIN_ADDR, for instance).
        for _ in 0..iters(200) {
            let mut ca = rng.addr();
            let mut ra = ca;
            for _ in 0..12 {
                let which = rng.next_u32() % 10;
                let v32 = rng.next_u32();
                let v64 = rng.next_u64();
                unsafe {
                    match which {
                        0 => { (l.c.set_layer_addr())(ca.as_mut_ptr(), v32); (l.r.set_layer_addr())(ra.as_mut_ptr(), v32); }
                        1 => { (l.c.set_tree_addr())(ca.as_mut_ptr(), v64); (l.r.set_tree_addr())(ra.as_mut_ptr(), v64); }
                        2 => { (l.c.set_type())(ca.as_mut_ptr(), v32); (l.r.set_type())(ra.as_mut_ptr(), v32); }
                        3 => { (l.c.set_keypair_addr())(ca.as_mut_ptr(), v32); (l.r.set_keypair_addr())(ra.as_mut_ptr(), v32); }
                        4 => { (l.c.set_chain_addr())(ca.as_mut_ptr(), v32); (l.r.set_chain_addr())(ra.as_mut_ptr(), v32); }
                        5 => { (l.c.set_hash_addr())(ca.as_mut_ptr(), v32); (l.r.set_hash_addr())(ra.as_mut_ptr(), v32); }
                        6 => { (l.c.set_tree_height())(ca.as_mut_ptr(), v32); (l.r.set_tree_height())(ra.as_mut_ptr(), v32); }
                        7 => { (l.c.set_tree_index())(ca.as_mut_ptr(), v32); (l.r.set_tree_index())(ra.as_mut_ptr(), v32); }
                        8 => { let s = rng.addr(); (l.c.copy_subtree_addr())(ca.as_mut_ptr(), s.as_ptr()); (l.r.copy_subtree_addr())(ra.as_mut_ptr(), s.as_ptr()); }
                        _ => { let s = rng.addr(); (l.c.copy_keypair_addr())(ca.as_mut_ptr(), s.as_ptr()); (l.r.copy_keypair_addr())(ra.as_mut_ptr(), s.as_ptr()); }
                    }
                }
                eqb("address setter sequence", "mixed", addr_bytes(&ca), addr_bytes(&ra));
            }
        }
    }

    // ---------------------------------------------------------------- C5
    #[test]
    fn cfg_address_copy() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 5);
        for _ in 0..iters(400) {
            let src = rng.addr();
            let dst = rng.addr();
            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                (l.c.copy_subtree_addr())(ca.as_mut_ptr(), src.as_ptr());
                (l.r.copy_subtree_addr())(ra.as_mut_ptr(), src.as_ptr());
            }
            eqb("SPX_copy_subtree_addr", "random", addr_bytes(&ca), addr_bytes(&ra));
            // and the C semantics: only OFFSET_TREE+8 bytes are copied
            let cb = addr_bytes(&ca).to_vec();
            let sb = addr_bytes(&src).to_vec();
            let db = addr_bytes(&dst).to_vec();
            assert_eq!(&cb[..SPX_OFFSET_TREE + 8], &sb[..SPX_OFFSET_TREE + 8]);
            assert_eq!(&cb[SPX_OFFSET_TREE + 8..], &db[SPX_OFFSET_TREE + 8..]);

            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                (l.c.copy_keypair_addr())(ca.as_mut_ptr(), src.as_ptr());
                (l.r.copy_keypair_addr())(ra.as_mut_ptr(), src.as_ptr());
            }
            eqb("SPX_copy_keypair_addr", "random", addr_bytes(&ca), addr_bytes(&ra));
        }
        // degenerate: identical buffers, all-zero, all-0xFF
        for pat in [[0u32; 8], [u32::MAX; 8]] {
            let mut ca = pat;
            let mut ra = pat;
            unsafe {
                (l.c.copy_keypair_addr())(ca.as_mut_ptr(), pat.as_ptr());
                (l.r.copy_keypair_addr())(ra.as_mut_ptr(), pat.as_ptr());
            }
            eqb("SPX_copy_keypair_addr", "degenerate", addr_bytes(&ca), addr_bytes(&ra));
        }
    }

    // ---------------------------------------------------------------- C6
    #[test]
    fn cfg_initialize_hash_function() {
        let _g = serial();
        let mut rng = Rng::new(TEST_SEED ^ 6);
        for i in 0..iters(40) {
            let (ps, ss) = match i {
                0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
                1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
                2 => (vec![0u8; SPX_N], vec![0xFFu8; SPX_N]),
                _ => (rng.vec(SPX_N), rng.vec(SPX_N)),
            };
            // make_ctx already compares the whole live ctx image
            let _ = make_ctx(&ps, &ss);
        }
    }

    // ---------------------------------------------------------------- C7
    #[test]
    fn cfg_prf_addr() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 7);
        for i in 0..iters(60) {
            let ps = if i == 0 { vec![0u8; SPX_N] } else { rng.vec(SPX_N) };
            let ss = if i == 0 { vec![0xFFu8; SPX_N] } else { rng.vec(SPX_N) };
            let (cc, rc) = make_ctx(&ps, &ss);
            let addrs: Vec<[u32; 8]> = vec![[0; 8], [u32::MAX; 8], rng.addr(), rng.addr()];
            for a in addrs {
                let mut co = vec![0xAAu8; SPX_N + 8];
                let mut ro = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    (l.c.prf_addr())(co.as_mut_ptr(), cc.as_ptr(), a.as_ptr());
                    (l.r.prf_addr())(ro.as_mut_ptr(), rc.as_ptr(), a.as_ptr());
                }
                eqb("SPX_prf_addr", &format!("addr={a:08x?}"), &co, &ro);
            }
        }
    }

    // ------------------------------------------------------- C8 / C9 / C10
    #[test]
    fn cfg_thash_inblocks() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 8);
        let mut blocks: Vec<u32> = vec![0, 1, 2, 3, SPX_WOTS_LEN as u32, SPX_FORS_TREES as u32, 255];
        blocks.dedup();
        for i in 0..iters(20) {
            let ps = if i == 0 { vec![0u8; SPX_N] } else { rng.vec(SPX_N) };
            let ss = rng.vec(SPX_N);
            let (cc, rc) = make_ctx(&ps, &ss);
            for &nb in &blocks {
                let data = rng.vec((nb as usize) * SPX_N + 8);
                for a in [[0u32; 8], [u32::MAX; 8], rng.addr()] {
                    let mut ca = a;
                    let mut ra = a;
                    let mut co = vec![0xAAu8; SPX_N + 8];
                    let mut ro = vec![0xAAu8; SPX_N + 8];
                    unsafe {
                        (l.c.thash())(co.as_mut_ptr(), data.as_ptr(), nb, cc.as_ptr(), ca.as_mut_ptr());
                        (l.r.thash())(ro.as_mut_ptr(), data.as_ptr(), nb, rc.as_ptr(), ra.as_mut_ptr());
                    }
                    eqb("SPX_thash", &format!("inblocks={nb} addr={a:08x?}"), &co, &ro);
                    eqb("SPX_thash addr", &format!("inblocks={nb}"), addr_bytes(&ca), addr_bytes(&ra));
                }
            }
        }
    }

    // ---------------------------------------------------------------- C11
    #[test]
    fn cfg_gen_message_random() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 11);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &mlen in MLENS {
            for _ in 0..iters(4) {
                let sk_prf = rng.vec(SPX_N);
                let optrand = rng.vec(SPX_N);
                let m = rng.vec(mlen);
                // NOTE: the BLAKE backend's `gen_message_random` writes the
                // *full* 32-/64-byte BLAKE digest into `R` (the C does
                // `blakeX_final(&S, R)`), not just SPX_N bytes, so the output
                // buffer must be over-sized — and the extra bytes are compared
                // too, so an over-write difference would be caught.
                let mut co = vec![0xAAu8; 128];
                let mut ro = vec![0xAAu8; 128];
                unsafe {
                    (l.c.gen_message_random())(
                        co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(),
                        m.as_ptr(), mlen as u64, cc.as_ptr());
                    (l.r.gen_message_random())(
                        ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(),
                        m.as_ptr(), mlen as u64, rc.as_ptr());
                }
                eqb("SPX_gen_message_random", &format!("mlen={mlen}"), &co, &ro);
            }
        }
    }

    // ---------------------------------------------------------------- C12
    #[test]
    fn cfg_hash_message() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 12);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &mlen in MLENS {
            for _ in 0..iters(4) {
                let rr = rng.vec(SPX_N);
                let pk = rng.vec(SPX_PK_BYTES);
                let m = rng.vec(mlen);
                let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
                let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
                let (mut ct, mut rt) = (0xDEAD_BEEF_DEAD_BEEFu64, 0u64);
                let (mut cl, mut rl) = (0xDEAD_BEEFu32, 0u32);
                unsafe {
                    (l.c.hash_message())(cd.as_mut_ptr(), &mut ct, &mut cl,
                        rr.as_ptr(), pk.as_ptr(), m.as_ptr(), mlen as u64, cc.as_ptr());
                    (l.r.hash_message())(rd.as_mut_ptr(), &mut rt, &mut rl,
                        rr.as_ptr(), pk.as_ptr(), m.as_ptr(), mlen as u64, rc.as_ptr());
                }
                eqb("SPX_hash_message digest", &format!("mlen={mlen}"), &cd, &rd);
                eq("SPX_hash_message tree", &format!("mlen={mlen}"), ct, rt);
                eq("SPX_hash_message leaf_idx", &format!("mlen={mlen}"), cl, rl);
                if SPX_TREE_BITS < 64 {
                    assert!(rt < (1u64 << SPX_TREE_BITS), "tree not masked to SPX_TREE_BITS");
                }
                assert!(rl < (1u32 << SPX_TREE_HEIGHT), "leaf_idx not masked");
            }
        }
    }

    // ---------------------------------------------------------------- C13
    #[test]
    fn cfg_chain_lengths() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 13);
        let mut msgs: Vec<Vec<u8>> = vec![vec![0u8; SPX_N], vec![0xFFu8; SPX_N]];
        for _ in 0..iters(400) {
            msgs.push(rng.vec(SPX_N));
        }
        for m in &msgs {
            let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
            let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
            unsafe {
                (l.c.chain_lengths())(cl.as_mut_ptr(), m.as_ptr());
                (l.r.chain_lengths())(rl.as_mut_ptr(), m.as_ptr());
            }
            eq("SPX_chain_lengths", &format!("msg={m:02x?}"), &cl, &rl);
        }
    }

    // ---------------------------------------------------------------- C14
    #[test]
    fn cfg_wots_pk_from_sig() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 14);
        for i in 0..iters(8) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for _ in 0..3 {
                let sig = rng.vec(SPX_WOTS_BYTES);
                let msg = if i == 0 { vec![0u8; SPX_N] } else { rng.vec(SPX_N) };
                let a = rng.addr();
                let mut ca = a;
                let mut ra = a;
                let mut cpk = vec![0xAAu8; SPX_WOTS_BYTES];
                let mut rpk = vec![0xAAu8; SPX_WOTS_BYTES];
                unsafe {
                    (l.c.wots_pk_from_sig())(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
                    (l.r.wots_pk_from_sig())(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
                }
                eqb("SPX_wots_pk_from_sig", "random", &cpk, &rpk);
                eqb("SPX_wots_pk_from_sig addr", "random", addr_bytes(&ca), addr_bytes(&ra));
            }
            // extreme messages: all-zero (max steps) and all-0xFF (min steps)
            for pat in [0u8, 0xFF] {
                let sig = rng.vec(SPX_WOTS_BYTES);
                let msg = vec![pat; SPX_N];
                let a = [0u32; 8];
                let mut ca = a;
                let mut ra = a;
                let mut cpk = vec![0xAAu8; SPX_WOTS_BYTES];
                let mut rpk = vec![0xAAu8; SPX_WOTS_BYTES];
                unsafe {
                    (l.c.wots_pk_from_sig())(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
                    (l.r.wots_pk_from_sig())(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
                }
                eqb("SPX_wots_pk_from_sig", &format!("msg=all {pat:#x}"), &cpk, &rpk);
            }
        }
    }

    // ---------------------------------------------------------------- C15
    #[test]
    fn cfg_wots_gen_leafx1_nosig() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 15);
        for _ in 0..iters(10) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for _ in 0..3 {
                let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
                    .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
                    .collect();
                steps[0] = 0;
                steps[SPX_WOTS_LEN - 1] = SPX_WOTS_W as u32 - 1;
                let base = rng.addr();
                let leaf_idx = rng.next_u32();

                let run = |imp: &Impl, ctx: *const u8, steps: &mut Vec<u32>| -> (Vec<u8>, ([u32; 8], [u32; 8], u32)) {
                    let mut info = LeafInfoX1::zeroed();
                    info.wots_sign_leaf = u32::MAX;
                    info.wots_steps = steps.as_mut_ptr();
                    info.leaf_addr = base;
                    info.pk_addr = base;
                    let mut dest = vec![0xAAu8; SPX_N + 8];
                    unsafe { (imp.wots_gen_leafx1())(dest.as_mut_ptr(), ctx, leaf_idx, &mut info) };
                    (dest, info.observable())
                };
                let mut cs = steps.clone();
                let mut rs = steps.clone();
                let (cd, ci) = run(&l.c, cc.as_ptr(), &mut cs);
                let (rd, ri) = run(&l.r, rc.as_ptr(), &mut rs);
                eqb("SPX_wots_gen_leafx1 dest", "nosig", &cd, &rd);
                eq("SPX_wots_gen_leafx1 info", "nosig", ci, ri);
                eq("SPX_wots_gen_leafx1 steps", "nosig", &cs, &rs);
            }
        }
    }

    // ---------------------------------------------------------------- C16
    #[test]
    fn cfg_wots_gen_leafx1_sig() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 16);
        for _ in 0..iters(10) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for k in 0..4 {
                // steps taken both from chain_lengths (the real use) and random
                let mut steps: Vec<u32> = vec![0; SPX_WOTS_LEN];
                if k % 2 == 0 {
                    let root = rng.vec(SPX_N);
                    unsafe { (l.c.chain_lengths())(steps.as_mut_ptr(), root.as_ptr()) };
                } else {
                    for s in steps.iter_mut() {
                        *s = rng.next_u32() % SPX_WOTS_W as u32;
                    }
                    steps[0] = 0;
                    steps[SPX_WOTS_LEN - 1] = SPX_WOTS_W as u32 - 1;
                }
                let base = rng.addr();
                let leaf_idx = rng.next_u32() % 64;

                let run = |imp: &Impl, ctx: *const u8, steps: &mut Vec<u32>|
                    -> (Vec<u8>, Vec<u8>, ([u32; 8], [u32; 8], u32)) {
                    let mut sig = vec![0xAAu8; SPX_WOTS_BYTES];
                    let mut info = LeafInfoX1::zeroed();
                    info.wots_sig = sig.as_mut_ptr();
                    info.wots_sign_leaf = leaf_idx;
                    info.wots_steps = steps.as_mut_ptr();
                    info.leaf_addr = base;
                    info.pk_addr = base;
                    let mut dest = vec![0xAAu8; SPX_N + 8];
                    unsafe { (imp.wots_gen_leafx1())(dest.as_mut_ptr(), ctx, leaf_idx, &mut info) };
                    (dest, sig, info.observable())
                };
                let mut cs = steps.clone();
                let mut rs = steps.clone();
                let (cd, csig, ci) = run(&l.c, cc.as_ptr(), &mut cs);
                let (rd, rsig, ri) = run(&l.r, rc.as_ptr(), &mut rs);
                eqb("SPX_wots_gen_leafx1 dest", "sig", &cd, &rd);
                eqb("SPX_wots_gen_leafx1 wots_sig", "sig", &csig, &rsig);
                eq("SPX_wots_gen_leafx1 info", "sig", ci, ri);
            }
        }
    }

    // ---------------------------------------------------------------- C17
    #[test]
    fn cfg_treehash() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 17);
        for _ in 0..iters(6) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for &h in &[0u32, 1, 2, 3] {
                for &leaf_idx in &[0u32, 1, 2, 3, u32::MAX] {
                    let idx_offset = rng.next_u32() & 0xFFFF;
                    let base = rng.addr();
                    // `treehash` may write `auth_path + tree_height*SPX_N`
                    // (the last `heights[offset-1]++` can reach tree_height), so
                    // the buffer must hold tree_height+1 nodes; +1 more node and
                    // 64 guard bytes so an over-write is *detected*, not fatal.
                    let ap = (h as usize + 2) * SPX_N + 64;

                    let mut ca = base;
                    let mut ra = base;
                    let mut croot = vec![0xAAu8; SPX_N + 8];
                    let mut rroot = vec![0xAAu8; SPX_N + 8];
                    let mut cap = vec![0xAAu8; ap];
                    let mut rap = vec![0xAAu8; ap];
                    unsafe {
                        (l.c.treehash())(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(),
                            leaf_idx, idx_offset, h, test_gen_leaf, ca.as_mut_ptr());
                        (l.r.treehash())(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(),
                            leaf_idx, idx_offset, h, test_gen_leaf, ra.as_mut_ptr());
                    }
                    let c = format!("h={h} leaf_idx={leaf_idx} off={idx_offset}");
                    eqb("SPX_treehash root", &c, &croot, &rroot);
                    eqb("SPX_treehash auth_path", &c, &cap, &rap);
                    eqb("SPX_treehash tree_addr", &c, addr_bytes(&ca), addr_bytes(&ra));
                }
            }
        }
    }

    // ---------------------------------------------------------------- C18
    #[test]
    fn cfg_compute_root() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 18);
        let mut heights: Vec<u32> = vec![1, 2, 3, SPX_FORS_HEIGHT as u32, SPX_TREE_HEIGHT as u32];
        heights.sort_unstable();
        heights.dedup();
        for _ in 0..iters(8) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for &h in &heights {
                let mut idxs: Vec<u32> = vec![0, 1, 2, 3, (1u32 << h.min(30)) - 1];
                idxs.push(rng.next_u32());
                for &leaf_idx in &idxs {
                    let idx_offset = rng.next_u32();
                    let leaf = rng.vec(SPX_N);
                    let auth = rng.vec(h as usize * SPX_N);
                    let base = rng.addr();
                    let mut ca = base;
                    let mut ra = base;
                    let mut croot = vec![0xAAu8; SPX_N + 8];
                    let mut rroot = vec![0xAAu8; SPX_N + 8];
                    unsafe {
                        (l.c.compute_root())(croot.as_mut_ptr(), leaf.as_ptr(), leaf_idx,
                            idx_offset, auth.as_ptr(), h, cc.as_ptr(), ca.as_mut_ptr());
                        (l.r.compute_root())(rroot.as_mut_ptr(), leaf.as_ptr(), leaf_idx,
                            idx_offset, auth.as_ptr(), h, rc.as_ptr(), ra.as_mut_ptr());
                    }
                    let c = format!("h={h} leaf_idx={leaf_idx} off={idx_offset}");
                    eqb("SPX_compute_root", &c, &croot, &rroot);
                    eqb("SPX_compute_root addr", &c, addr_bytes(&ca), addr_bytes(&ra));
                }
            }
        }
    }

    // ---------------------------------------------------------------- C19
    #[test]
    fn cfg_fors_gen_leafx1() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 19);
        for _ in 0..iters(20) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let base = rng.addr();
            let mut idxs: Vec<u32> = vec![0, 1, (1u32 << SPX_FORS_HEIGHT) - 1, u32::MAX];
            idxs.push(rng.next_u32());
            for &ai in &idxs {
                let mut ci = ForsGenLeafInfo { leaf_addrx: base };
                let mut ri = ForsGenLeafInfo { leaf_addrx: base };
                let mut cl = vec![0xAAu8; SPX_N + 8];
                let mut rl = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    (l.c.fors_gen_leafx1())(cl.as_mut_ptr(), cc.as_ptr(), ai, &mut ci);
                    (l.r.fors_gen_leafx1())(rl.as_mut_ptr(), rc.as_ptr(), ai, &mut ri);
                }
                eqb("SPX_fors_gen_leafx1", &format!("addr_idx={ai}"), &cl, &rl);
                eqb("SPX_fors_gen_leafx1 info", &format!("addr_idx={ai}"),
                    addr_bytes(&ci.leaf_addrx), addr_bytes(&ri.leaf_addrx));
            }
        }
    }

    // ---------------------------------------------------------------- C20
    #[test]
    fn cfg_fors_treehashx1() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 20);
        let mut heights: Vec<u32> = vec![1, 2, 3, SPX_FORS_HEIGHT as u32];
        heights.sort_unstable();
        heights.dedup();
        for _ in 0..iters(4) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for &h in &heights {
                let mut idxs: Vec<u32> = vec![0, 1, (1u32 << h) - 1];
                idxs.push(rng.next_u32() % (1u32 << h));
                for &leaf_idx in &idxs {
                    let idx_offset = rng.next_u32() & 0xFFFF;
                    let base = rng.addr();
                    let mut ca = base;
                    let mut ra = base;
                    let mut ci = ForsGenLeafInfo { leaf_addrx: base };
                    let mut ri = ForsGenLeafInfo { leaf_addrx: base };
                    let mut croot = vec![0xAAu8; SPX_N + 8];
                    let mut rroot = vec![0xAAu8; SPX_N + 8];
                    let mut cap = vec![0xAAu8; (h as usize + 1) * SPX_N + 64];
                    let mut rap = vec![0xAAu8; (h as usize + 1) * SPX_N + 64];
                    unsafe {
                        (l.c.fors_treehashx1())(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(),
                            leaf_idx, idx_offset, h, ca.as_mut_ptr(), &mut ci);
                        (l.r.fors_treehashx1())(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(),
                            leaf_idx, idx_offset, h, ra.as_mut_ptr(), &mut ri);
                    }
                    let c = format!("h={h} leaf_idx={leaf_idx} off={idx_offset}");
                    eqb("SPX_fors_treehashx1 root", &c, &croot, &rroot);
                    eqb("SPX_fors_treehashx1 auth_path", &c, &cap, &rap);
                    eqb("SPX_fors_treehashx1 tree_addr", &c, addr_bytes(&ca), addr_bytes(&ra));
                    eqb("SPX_fors_treehashx1 info", &c,
                        addr_bytes(&ci.leaf_addrx), addr_bytes(&ri.leaf_addrx));
                }
            }
        }
    }

    // ---------------------------------------------------------------- C21
    #[test]
    fn cfg_wots_treehashx1() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 21);
        let mut heights: Vec<u32> = vec![1, 2, SPX_TREE_HEIGHT as u32];
        heights.sort_unstable();
        heights.dedup();
        for _ in 0..iters(2) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            for &h in &heights {
                let mut idxs: Vec<u32> = vec![0, 1, (1u32 << h) - 1, u32::MAX];
                idxs.push(rng.next_u32() % (1u32 << h));
                for &leaf_idx in &idxs {
                    let idx_offset = 0u32; // as merkle_sign uses
                    let base = rng.addr();
                    let mut steps: Vec<u32> = vec![0; SPX_WOTS_LEN];
                    let root_in = rng.vec(SPX_N);
                    unsafe { (l.c.chain_lengths())(steps.as_mut_ptr(), root_in.as_ptr()) };

                    let run = |imp: &Impl, ctx: *const u8| -> (Vec<u8>, Vec<u8>, Vec<u8>, [u32; 8], ([u32; 8], [u32; 8], u32)) {
                        let mut st = steps.clone();
                        let mut sig = vec![0xAAu8; SPX_WOTS_BYTES];
                        let mut info = LeafInfoX1::zeroed();
                        info.wots_sig = sig.as_mut_ptr();
                        info.wots_sign_leaf = leaf_idx;
                        info.wots_steps = st.as_mut_ptr();
                        info.leaf_addr = base;
                        info.pk_addr = base;
                        let mut ta = base;
                        let mut root = vec![0xAAu8; SPX_N + 8];
                        let mut ap = vec![0xAAu8; (h as usize + 1) * SPX_N + 64];
                        unsafe {
                            (imp.wots_treehashx1())(root.as_mut_ptr(), ap.as_mut_ptr(), ctx,
                                leaf_idx, idx_offset, h, ta.as_mut_ptr(), &mut info);
                        }
                        (root, ap, sig, ta, info.observable())
                    };
                    let (croot, cap, csig, cta, cinfo) = run(&l.c, cc.as_ptr());
                    let (rroot, rap, rsig, rta, rinfo) = run(&l.r, rc.as_ptr());
                    let c = format!("h={h} leaf_idx={leaf_idx}");
                    eqb("SPX_wots_treehashx1 root", &c, &croot, &rroot);
                    eqb("SPX_wots_treehashx1 auth_path", &c, &cap, &rap);
                    eqb("SPX_wots_treehashx1 wots_sig", &c, &csig, &rsig);
                    eqb("SPX_wots_treehashx1 tree_addr", &c, addr_bytes(&cta), addr_bytes(&rta));
                    eq("SPX_wots_treehashx1 info", &c, cinfo, rinfo);
                }
            }
        }
    }

    // ---------------------------------------------------------------- C22
    #[test]
    fn cfg_fors_sign() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 22);
        let n = if SLOW_PARAMS { iters(2) } else { iters(5) };
        for i in 0..n {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let msgs: Vec<Vec<u8>> = match i {
                0 => vec![vec![0u8; SPX_FORS_MSG_BYTES], vec![0xFFu8; SPX_FORS_MSG_BYTES]],
                _ => vec![rng.vec(SPX_FORS_MSG_BYTES)],
            };
            for m in msgs {
                let fa = rng.addr();
                let mut csig = vec![0xAAu8; SPX_FORS_BYTES];
                let mut rsig = vec![0xAAu8; SPX_FORS_BYTES];
                let mut cpk = vec![0xAAu8; SPX_N + 8];
                let mut rpk = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    (l.c.fors_sign())(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr());
                    (l.r.fors_sign())(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), rc.as_ptr(), fa.as_ptr());
                }
                eqb("SPX_fors_sign sig", "random", &csig, &rsig);
                eqb("SPX_fors_sign pk", "random", &cpk, &rpk);
            }
        }
    }

    // ---------------------------------------------------------------- C23
    #[test]
    fn cfg_fors_pk_from_sig() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 23);
        let n = if SLOW_PARAMS { iters(2) } else { iters(5) };
        for i in 0..n {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let m = if i == 0 { vec![0u8; SPX_FORS_MSG_BYTES] } else { rng.vec(SPX_FORS_MSG_BYTES) };
            let fa = rng.addr();

            // (a) round trip: sign with C, recover with both
            let mut sig = vec![0u8; SPX_FORS_BYTES];
            let mut pk0 = vec![0u8; SPX_N];
            unsafe { (l.c.fors_sign())(sig.as_mut_ptr(), pk0.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr()) };
            for (label, s) in [("roundtrip", sig.clone()), ("random-sig", rng.vec(SPX_FORS_BYTES))] {
                let mut cpk = vec![0xAAu8; SPX_N + 8];
                let mut rpk = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    (l.c.fors_pk_from_sig())(cpk.as_mut_ptr(), s.as_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr());
                    (l.r.fors_pk_from_sig())(rpk.as_mut_ptr(), s.as_ptr(), m.as_ptr(), rc.as_ptr(), fa.as_ptr());
                }
                eqb("SPX_fors_pk_from_sig", label, &cpk, &rpk);
                if label == "roundtrip" {
                    assert_eq!(&cpk[..SPX_N], &pk0[..], "fors round trip must reproduce pk");
                }
            }
        }
    }

    // ---------------------------------------------------------------- C24
    #[test]
    fn cfg_merkle_sign() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 24);
        let n = if SLOW_PARAMS { iters(1) } else { iters(3) };
        let outlen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        for _ in 0..n {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let mut idxs: Vec<u32> = vec![0, 1, (1u32 << SPX_TREE_HEIGHT) - 1, u32::MAX];
            idxs.push(rng.next_u32() % (1u32 << SPX_TREE_HEIGHT));
            for &idx_leaf in &idxs {
                let wa = rng.addr();
                let ta = rng.addr();
                let root_in = rng.vec(SPX_N);
                let run = |imp: &Impl, ctx: *const u8| -> (Vec<u8>, Vec<u8>, [u32; 8], [u32; 8]) {
                    let mut sig = vec![0xAAu8; outlen];
                    let mut root = root_in.clone();
                    let mut w = wa;
                    let mut t = ta;
                    unsafe {
                        (imp.merkle_sign())(sig.as_mut_ptr(), root.as_mut_ptr(), ctx,
                            w.as_mut_ptr(), t.as_mut_ptr(), idx_leaf);
                    }
                    (sig, root, w, t)
                };
                let (cs, cr, cw, ct) = run(&l.c, cc.as_ptr());
                let (rs, rr, rw, rt) = run(&l.r, rc.as_ptr());
                let c = format!("idx_leaf={idx_leaf}");
                eqb("SPX_merkle_sign sig", &c, &cs, &rs);
                eqb("SPX_merkle_sign root", &c, &cr, &rr);
                eqb("SPX_merkle_sign wots_addr", &c, addr_bytes(&cw), addr_bytes(&rw));
                eqb("SPX_merkle_sign tree_addr", &c, addr_bytes(&ct), addr_bytes(&rt));
            }
        }
    }

    // ---------------------------------------------------------------- C25
    #[test]
    fn cfg_merkle_gen_root() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 25);
        let n = if SLOW_PARAMS { 3 } else { iters(4) };
        for i in 0..n {
            let (ps, ss) = match i {
                0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
                1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
                _ => (rng.vec(SPX_N), rng.vec(SPX_N)),
            };
            let (cc, rc) = make_ctx(&ps, &ss);
            let mut croot = vec![0xAAu8; SPX_N + 8];
            let mut rroot = vec![0xAAu8; SPX_N + 8];
            unsafe {
                (l.c.merkle_gen_root())(croot.as_mut_ptr(), cc.as_ptr());
                (l.r.merkle_gen_root())(rroot.as_mut_ptr(), rc.as_ptr());
            }
            eqb("SPX_merkle_gen_root", &format!("case {i}"), &croot, &rroot);
        }
    }

    // ---------------------------------------------------------------- C26
    #[test]
    fn cfg_sizes() {
        let _g = serial();
        let l = libs();
        unsafe {
            eq("crypto_sign_secretkeybytes", "", (l.c.crypto_sign_secretkeybytes())(), (l.r.crypto_sign_secretkeybytes())());
            eq("crypto_sign_publickeybytes", "", (l.c.crypto_sign_publickeybytes())(), (l.r.crypto_sign_publickeybytes())());
            eq("crypto_sign_bytes", "", (l.c.crypto_sign_bytes())(), (l.r.crypto_sign_bytes())());
            eq("crypto_sign_seedbytes", "", (l.c.crypto_sign_seedbytes())(), (l.r.crypto_sign_seedbytes())());
            // and against the locally derived constants
            eq("SPX_SK_BYTES", "", (l.c.crypto_sign_secretkeybytes())(), SPX_SK_BYTES as u64);
            eq("SPX_PK_BYTES", "", (l.c.crypto_sign_publickeybytes())(), SPX_PK_BYTES as u64);
            eq("SPX_BYTES", "", (l.c.crypto_sign_bytes())(), SPX_BYTES as u64);
            eq("CRYPTO_SEEDBYTES", "", (l.c.crypto_sign_seedbytes())(), CRYPTO_SEEDBYTES as u64);
        }
    }

    // ---------------------------------------------------------------- C27
    #[test]
    fn cfg_seed_keypair() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 27);
        let n = if SLOW_PARAMS { 3 } else { iters(4) };
        for i in 0..n {
            let seed = match i {
                0 => vec![0u8; CRYPTO_SEEDBYTES],
                1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
                _ => rng.vec(CRYPTO_SEEDBYTES),
            };
            let (cpk, csk) = keypair_from_seed(&l.c, &seed);
            let (rpk, rsk) = keypair_from_seed(&l.r, &seed);
            eqb("crypto_sign_seed_keypair pk", &format!("case {i}"), &cpk, &rpk);
            eqb("crypto_sign_seed_keypair sk", &format!("case {i}"), &csk, &rsk);
            // documented format: sk = SK_SEED || SK_PRF || PUB_SEED || root
            assert_eq!(&csk[2 * SPX_N..3 * SPX_N], &cpk[..SPX_N]);
            assert_eq!(&csk[3 * SPX_N..4 * SPX_N], &cpk[SPX_N..2 * SPX_N]);
        }
    }

    // ---------------------------------------------------------------- C28
    #[test]
    fn cfg_keypair_from_drbg() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 28);
        let n = if SLOW_PARAMS { 2 } else { iters(3) };
        for _ in 0..n {
            let mut entropy = rng.vec(48);
            let (cpk, csk, cd) = keypair_random(&l.c, &mut entropy);
            let (rpk, rsk, rd) = keypair_random(&l.r, &mut entropy);
            eqb("crypto_sign_keypair pk", "drbg", &cpk, &rpk);
            eqb("crypto_sign_keypair sk", "drbg", &csk, &rsk);
            eq("DRBG_ctx after keypair", "drbg", cd, rd);
        }
    }

    // ---------------------------------------------------------------- C29
    #[test]
    fn cfg_sign_signature() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 29);
        let mlens: &[usize] = if SLOW_PARAMS { &[0, 33, 137] } else { &[0, 1, 31, 32, 33, 63, 64, 65, 135, 136, 137, 1000] };
        let seed = rng.vec(CRYPTO_SEEDBYTES);
        let (cpk, csk) = keypair_from_seed(&l.c, &seed);
        let (rpk, rsk) = keypair_from_seed(&l.r, &seed);
        eqb("keypair", "shared", &cpk, &rpk);
        eqb("keypair sk", "shared", &csk, &rsk);
        for &mlen in mlens {
            let m = rng.vec(mlen);
            let mut entropy = rng.vec(48);
            let (cs, cl2) = sign_detached(&l.c, &csk, &m, &mut entropy);
            let (rs, rl2) = sign_detached(&l.r, &rsk, &m, &mut entropy);
            eq("crypto_sign_signature siglen", &format!("mlen={mlen}"), cl2, rl2);
            eqb("crypto_sign_signature sig", &format!("mlen={mlen}"), &cs, &rs);
            eq("siglen == SPX_BYTES", &format!("mlen={mlen}"), cl2, SPX_BYTES);
        }
    }

    // ---------------------------------------------------------------- C30
    #[test]
    fn cfg_verify_cross() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 30);
        let mlens: &[usize] = if SLOW_PARAMS { &[0, 137] } else { &[0, 1, 32, 64, 136, 1000] };
        let seed = rng.vec(CRYPTO_SEEDBYTES);
        let (cpk, csk) = keypair_from_seed(&l.c, &seed);
        let (_rpk, _rsk) = keypair_from_seed(&l.r, &seed);
        for &mlen in mlens {
            let m = rng.vec(mlen);
            let mut entropy = rng.vec(48);
            let (sig, _) = sign_detached(&l.c, &csk, &m, &mut entropy);
            unsafe {
                let cv = (l.c.crypto_sign_verify())(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr());
                let rv = (l.r.crypto_sign_verify())(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr());
                eq("crypto_sign_verify (C sig)", &format!("mlen={mlen}"), cv, rv);
                eq("crypto_sign_verify accepts", &format!("mlen={mlen}"), cv, 0);
            }
            let (sig2, _) = sign_detached(&l.r, &csk, &m, &mut entropy);
            eqb("C sig == Rust sig", &format!("mlen={mlen}"), &sig, &sig2);
            unsafe {
                let cv = (l.c.crypto_sign_verify())(sig2.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr());
                let rv = (l.r.crypto_sign_verify())(sig2.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr());
                eq("crypto_sign_verify (Rust sig)", &format!("mlen={mlen}"), cv, rv);
                eq("crypto_sign_verify accepts", &format!("mlen={mlen}"), rv, 0);
            }
        }
    }

    // ---------------------------------------------------------------- C31
    #[test]
    fn cfg_sign_open_roundtrip() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 31);
        let mlens: &[usize] = if SLOW_PARAMS { &[0, 64] } else { &[0, 1, 32, 64, 136, 1000] };
        let seed = rng.vec(CRYPTO_SEEDBYTES);
        let (cpk, csk) = keypair_from_seed(&l.c, &seed);
        for &mlen in mlens {
            let m = rng.vec(mlen);
            let mut entropy = rng.vec(48);
            let (csm, cslen) = sign_attached(&l.c, &csk, &m, &mut entropy);
            let (rsm, rslen) = sign_attached(&l.r, &csk, &m, &mut entropy);
            eq("crypto_sign smlen", &format!("mlen={mlen}"), cslen, rslen);
            eqb("crypto_sign sm", &format!("mlen={mlen}"), &csm, &rsm);
            eq("smlen", &format!("mlen={mlen}"), cslen, (SPX_BYTES + mlen) as u64);

            for (label, sm) in [("C-signed", &csm), ("Rust-signed", &rsm)] {
                let mut cm = vec![0xAAu8; sm.len() + 8];
                let mut rm = vec![0xAAu8; sm.len() + 8];
                let mut cml = 0xDEADu64;
                let mut rml = 0xDEADu64;
                unsafe {
                    let cr = (l.c.crypto_sign_open())(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), cslen, cpk.as_ptr());
                    let rr = (l.r.crypto_sign_open())(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), cslen, cpk.as_ptr());
                    eq("crypto_sign_open rc", &format!("{label} mlen={mlen}"), cr, rr);
                    eq("crypto_sign_open accepts", &format!("{label} mlen={mlen}"), cr, 0);
                }
                eq("crypto_sign_open mlen", &format!("{label} mlen={mlen}"), cml, rml);
                eqb("crypto_sign_open m", &format!("{label} mlen={mlen}"), &cm, &rm);
                assert_eq!(&cm[..mlen], &m[..]);
            }
        }
    }

    // ---------------------------------------------------------------- C32
    #[test]
    fn cfg_drbg_stream() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 32);
        for case in 0..iters(12) {
            let mut entropy = match case {
                0 => vec![0u8; 48],
                1 => vec![0xFFu8; 48],
                _ => rng.vec(48),
            };
            let mut ps: Option<Vec<u8>> = match case % 3 {
                0 => None,
                _ => Some(rng.vec(48)),
            };
            let draws: Vec<usize> = vec![0, 1, 15, 16, 17, 48, 64, 100, 3];
            let cout = drbg_run(&l.c, &mut entropy, ps.as_mut(), &draws);
            let rout = drbg_run(&l.r, &mut entropy, ps.as_mut(), &draws);
            for (i, (a, b)) in cout.0.iter().zip(rout.0.iter()).enumerate() {
                eqb("randombytes draw", &format!("case={case} draw#{i}"), a, b);
            }
            eq("DRBG_ctx", &format!("case={case}"), cout.1, rout.1);
            eq("randombytes rc", &format!("case={case}"), cout.2, rout.2);
        }
    }

    // ---------------------------------------------------------------- C33
    #[test]
    fn cfg_drbg_update() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 33);
        for case in 0..iters(200) {
            let key = match case {
                0 => vec![0u8; 32],
                1 => vec![0xFFu8; 32],
                _ => rng.vec(32),
            };
            let v = match case {
                0 => vec![0u8; 16],
                1 => vec![0xFFu8; 16],
                2 => {
                    let mut x = vec![0xFFu8; 16];
                    x[15] = 0xFE;
                    x
                }
                _ => rng.vec(16),
            };
            let pd: Option<Vec<u8>> = if case % 4 == 0 { None } else { Some(rng.vec(48)) };
            let run = |imp: &Impl| -> (Vec<u8>, Vec<u8>) {
                let mut k = key.clone();
                let mut vv = v.clone();
                let mut p = pd.clone();
                let pp = match p.as_mut() {
                    Some(x) => x.as_mut_ptr(),
                    None => std::ptr::null_mut(),
                };
                unsafe { (imp.drbg_update())(pp, k.as_mut_ptr(), vv.as_mut_ptr()) };
                (k, vv)
            };
            let (ck, cv) = run(&l.c);
            let (rk, rv) = run(&l.r);
            eqb("AES256_CTR_DRBG_Update Key", &format!("case={case}"), &ck, &rk);
            eqb("AES256_CTR_DRBG_Update V", &format!("case={case}"), &cv, &rv);
        }
    }

    // ---------------------------------------------------------------- C34
    #[test]
    fn cfg_aes256_ecb() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 34);
        for case in 0..iters(300) {
            let (key, ctr) = match case {
                0 => (vec![0u8; 32], vec![0u8; 16]),
                1 => (vec![0xFFu8; 32], vec![0xFFu8; 16]),
                2 => (vec![0u8; 32], vec![0xFFu8; 16]),
                _ => (rng.vec(32), rng.vec(16)),
            };
            let run = |imp: &Impl| -> Vec<u8> {
                let mut k = key.clone();
                let mut c = ctr.clone();
                let mut out = vec![0xAAu8; 16];
                unsafe { (imp.aes256_ecb())(k.as_mut_ptr(), c.as_mut_ptr(), out.as_mut_ptr()) };
                // C must not modify key/ctr
                assert_eq!(k, key);
                assert_eq!(c, ctr);
                out
            };
            eqb("AES256_ECB", &format!("case={case}"), &run(&l.c), &run(&l.r));
        }
    }

    // ---------------------------------------------------------------- C35
    #[test]
    fn cfg_seedexpander_stream() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 35);
        for &maxlen in &[1u64, 16, 17, 1000, 0xFFFF_FFFF] {
            for _ in 0..iters(6) {
                let seed = rng.vec(32);
                let div = rng.vec(8);
                let draws: Vec<u64> = vec![0, 1, 15, 16, 17, 33];
                let run = |imp: &Impl| -> (Vec<i32>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
                    let mut ctx = AesXofStruct::zeroed();
                    let mut s = seed.clone();
                    let mut d = div.clone();
                    let mut rcs = Vec::new();
                    let mut outs = Vec::new();
                    let mut states = Vec::new();
                    unsafe {
                        rcs.push((imp.seedexpander_init())(&mut ctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen));
                    }
                    states.push(ctx.bytes());
                    for &n in &draws {
                        let mut o = vec![0xAAu8; n as usize + 4];
                        let rc = unsafe { (imp.seedexpander())(&mut ctx, o.as_mut_ptr(), n) };
                        rcs.push(rc);
                        outs.push(o);
                        states.push(ctx.bytes());
                    }
                    (rcs, outs, states)
                };
                let (crc, co, cst) = run(&l.c);
                let (rrc, ro, rst) = run(&l.r);
                eq("seedexpander rc", &format!("maxlen={maxlen}"), &crc, &rrc);
                for (i, (a, b)) in co.iter().zip(ro.iter()).enumerate() {
                    eqb("seedexpander out", &format!("maxlen={maxlen} draw#{i}"), a, b);
                }
                for (i, (a, b)) in cst.iter().zip(rst.iter()).enumerate() {
                    eqb("AES_XOF_struct", &format!("maxlen={maxlen} step#{i}"), a, b);
                }
            }
        }
    }
}

// ###########################################################################
// Helpers shared by the test modules
// ###########################################################################

pub fn addr_bytes(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

/// `gen_leaf` callback handed to `SPX_treehash` across the FFI boundary.
/// Deterministic in `(addr_idx, tree_addr)` so both implementations see the
/// exact same leaves.
pub unsafe extern "C" fn test_gen_leaf(
    leaf: *mut u8,
    _ctx: *const u8,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let ta = std::slice::from_raw_parts(tree_addr, 8);
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ (addr_idx as u64);
    for w in ta {
        h ^= *w as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    for i in 0..SPX_N {
        h ^= i as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
        *leaf.add(i) = (h >> 24) as u8;
    }
}

/// Message lengths that hit every block/rate boundary of all four backends.
pub const MLENS: &[usize] = &[
    0, 1, 15, 16, 17, 23, 24, 25, 31, 32, 33, 39, 40, 47, 48, 55, 56, 57, 63, 64, 65, 71, 72, 73,
    79, 80, 81, 87, 88, 95, 96, 97, 103, 104, 111, 112, 113, 119, 120, 127, 128, 129, 135, 136,
    137, 143, 144, 151, 152, 159, 160, 167, 168, 175, 176, 199, 200, 255, 256, 271, 272, 273, 1000,
];

pub fn keypair_from_seed(imp: &Impl, seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut pk = vec![0xAAu8; SPX_PK_BYTES + 8];
    let mut sk = vec![0xAAu8; SPX_SK_BYTES + 8];
    let rc = unsafe { (imp.crypto_sign_seed_keypair())(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    assert_eq!(rc, 0, "crypto_sign_seed_keypair must return 0");
    (pk, sk)
}

pub fn keypair_random(imp: &Impl, entropy: &mut [u8]) -> (Vec<u8>, Vec<u8>, DrbgStruct) {
    unsafe { (imp.randombytes_init())(entropy.as_mut_ptr(), std::ptr::null_mut()) };
    let mut pk = vec![0xAAu8; SPX_PK_BYTES + 8];
    let mut sk = vec![0xAAu8; SPX_SK_BYTES + 8];
    let rc = unsafe { (imp.crypto_sign_keypair())(pk.as_mut_ptr(), sk.as_mut_ptr()) };
    assert_eq!(rc, 0);
    let d = unsafe { *imp.drbg_ctx() };
    (pk, sk, d)
}

pub fn sign_detached(imp: &Impl, sk: &[u8], m: &[u8], entropy: &mut [u8]) -> (Vec<u8>, usize) {
    unsafe { (imp.randombytes_init())(entropy.as_mut_ptr(), std::ptr::null_mut()) };
    let mut sig = vec![0xAAu8; SPX_BYTES + 8];
    let mut siglen = 0usize;
    let rc = unsafe {
        (imp.crypto_sign_signature())(sig.as_mut_ptr(), &mut siglen, m.as_ptr(), m.len(), sk.as_ptr())
    };
    assert_eq!(rc, 0);
    sig.truncate(SPX_BYTES);
    (sig, siglen)
}

pub fn sign_attached(imp: &Impl, sk: &[u8], m: &[u8], entropy: &mut [u8]) -> (Vec<u8>, u64) {
    unsafe { (imp.randombytes_init())(entropy.as_mut_ptr(), std::ptr::null_mut()) };
    let mut sm = vec![0xAAu8; SPX_BYTES + m.len() + 8];
    let mut smlen = 0u64;
    let rc = unsafe {
        (imp.crypto_sign())(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), m.len() as u64, sk.as_ptr())
    };
    assert_eq!(rc, 0);
    sm.truncate(SPX_BYTES + m.len());
    (sm, smlen)
}

pub fn drbg_run(
    imp: &Impl,
    entropy: &mut [u8],
    ps: Option<&mut Vec<u8>>,
    draws: &[usize],
) -> (Vec<Vec<u8>>, DrbgStruct, Vec<i32>) {
    let psp = match ps {
        Some(v) => v.as_mut_ptr(),
        None => std::ptr::null_mut(),
    };
    unsafe { (imp.randombytes_init())(entropy.as_mut_ptr(), psp) };
    let mut outs = Vec::new();
    let mut rcs = Vec::new();
    for &n in draws {
        let mut o = vec![0xAAu8; n + 4];
        let rc = unsafe { (imp.randombytes())(o.as_mut_ptr(), n as u64) };
        rcs.push(rc);
        outs.push(o);
    }
    let d = unsafe { *imp.drbg_ctx() };
    (outs, d, rcs)
}

// ###########################################################################
// Phase B (cont.) — backend-specific primitives (CONFIGS.md C36..C52)
// ###########################################################################

mod backend_blake {
    use super::*;

    fn skip() -> bool {
        !IS_BLAKE
    }

    // ---------------------------------------------------------------- C36
    #[test]
    fn cfg_blake_oneshot() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 36);
        let lens: &[usize] = &[0, 1, 2, 53, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113,
                               119, 120, 127, 128, 129, 191, 192, 193, 255, 256, 257, 1000];
        for &n in lens {
            for _ in 0..iters(3) {
                let m = rng.vec(n);
                for (name, cf, rf, olen) in [
                    ("blake256", l.c.blake256(), l.r.blake256(), 32usize),
                    ("blake512", l.c.blake512(), l.r.blake512(), 64usize),
                ] {
                    let mut co = vec![0xAAu8; olen + 8];
                    let mut ro = vec![0xAAu8; olen + 8];
                    let crc = unsafe { cf(co.as_mut_ptr(), m.as_ptr(), n as u64) };
                    let rrc = unsafe { rf(ro.as_mut_ptr(), m.as_ptr(), n as u64) };
                    eq(name, &format!("rc inlen={n}"), crc, rrc);
                    eqb(name, &format!("inlen={n}"), &co, &ro);
                }
            }
        }
    }

    // ---------------------------------------------------------------- C37
    #[test]
    fn cfg_blake_incremental() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 37);
        for _ in 0..iters(60) {
            let nchunks = 1 + (rng.next_u32() % 5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|i| {
                    let n = if i == 0 { (rng.next_u64() % 200) as usize } else { (rng.next_u64() % 140) as usize };
                    rng.vec(n)
                })
                .collect();

            // blake256
            let run256 = |imp: &Impl| -> (Vec<u8>, Vec<Vec<u8>>) {
                let mut s = BlakeState256::zeroed();
                let mut states = Vec::new();
                unsafe { (imp.blake256_init())(&mut s) };
                states.push(s.bytes());
                for c in &chunks {
                    unsafe { (imp.blake256_update())(&mut s, c.as_ptr(), (c.len() * 8) as u64) };
                    states.push(s.bytes());
                }
                // a zero-length update must be a no-op except for buflen
                unsafe { (imp.blake256_update())(&mut s, std::ptr::null(), 0) };
                states.push(s.bytes());
                let mut d = vec![0xAAu8; 40];
                unsafe { (imp.blake256_final())(&mut s, d.as_mut_ptr()) };
                states.push(s.bytes());
                (d, states)
            };
            let (cd, cs) = run256(&l.c);
            let (rd, rs) = run256(&l.r);
            eqb("blake256 incremental digest", "", &cd, &rd);
            for (i, (a, b)) in cs.iter().zip(rs.iter()).enumerate() {
                eqb("blakestate256", &format!("step#{i}"), a, b);
            }

            // blake512
            let run512 = |imp: &Impl| -> (Vec<u8>, Vec<Vec<u8>>) {
                let mut s = BlakeState512::zeroed();
                let mut states = Vec::new();
                unsafe { (imp.blake512_init())(&mut s) };
                states.push(s.bytes());
                for c in &chunks {
                    unsafe { (imp.blake512_update())(&mut s, c.as_ptr(), (c.len() * 8) as u64) };
                    states.push(s.bytes());
                }
                unsafe { (imp.blake512_update())(&mut s, std::ptr::null(), 0) };
                states.push(s.bytes());
                let mut d = vec![0xAAu8; 72];
                unsafe { (imp.blake512_final())(&mut s, d.as_mut_ptr()) };
                states.push(s.bytes());
                (d, states)
            };
            let (cd, cs) = run512(&l.c);
            let (rd, rs) = run512(&l.r);
            eqb("blake512 incremental digest", "", &cd, &rd);
            for (i, (a, b)) in cs.iter().zip(rs.iter()).enumerate() {
                eqb("blakestate512", &format!("step#{i}"), a, b);
            }
        }
    }

    // ---------------------------------------------------------------- C38
    #[test]
    fn cfg_blake_compress() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 38);
        for case in 0..iters(200) {
            // 256
            let mut s = BlakeState256::zeroed();
            if case > 0 {
                for x in s.h.iter_mut() { *x = rng.next_u32(); }
                for x in s.s.iter_mut() { *x = rng.next_u32(); }
                for x in s.t.iter_mut() { *x = rng.next_u32(); }
                s.buflen = (rng.next_u32() % 512) as i32;
                s.nullt = (rng.next_u32() % 2) as i32;
                rng.fill(&mut s.buf);
            }
            let block = rng.vec(64);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                (l.c.blake256_compress())(&mut cs, block.as_ptr());
                (l.r.blake256_compress())(&mut rs, block.as_ptr());
            }
            eqb("blake256_compress", &format!("case={case}"), &cs.bytes(), &rs.bytes());

            // 512
            let mut s = BlakeState512::zeroed();
            if case > 0 {
                for x in s.h.iter_mut() { *x = rng.next_u64(); }
                for x in s.s.iter_mut() { *x = rng.next_u64(); }
                for x in s.t.iter_mut() { *x = rng.next_u64(); }
                s.buflen = (rng.next_u32() % 1024) as i32;
                s.nullt = (rng.next_u32() % 2) as i32;
                rng.fill(&mut s.buf);
            }
            let block = rng.vec(128);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                (l.c.blake512_compress())(&mut cs, block.as_ptr());
                (l.r.blake512_compress())(&mut rs, block.as_ptr());
            }
            eqb("blake512_compress", &format!("case={case}"), &cs.bytes(), &rs.bytes());
        }
    }

    // ---------------------------------------------------------------- C39
    #[test]
    fn cfg_blake_mgf1() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 39);
        for &inlen in &[1usize, 4, 16, 32, 48, 64, 100] {
            let inp = rng.vec(inlen);
            for &outlen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 96, 127, 128, 129, 1000] {
                for (name, cf, rf) in [
                    ("SPX_blake256_mgf1", l.c.blake256_mgf1(), l.r.blake256_mgf1()),
                    ("SPX_blake512_mgf1", l.c.blake512_mgf1(), l.r.blake512_mgf1()),
                ] {
                    let mut co = vec![0xAAu8; outlen + 16];
                    let mut ro = vec![0xAAu8; outlen + 16];
                    unsafe {
                        cf(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        rf(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    eqb(name, &format!("inlen={inlen} outlen={outlen}"), &co, &ro);
                }
            }
        }
    }

    // ---------------------------------------------------------------- C40
    #[test]
    fn cfg_blake_cst() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let c = unsafe { std::slice::from_raw_parts(l.c.cst(), 16) };
        let r = unsafe { std::slice::from_raw_parts(l.r.cst(), 16) };
        eq("cst[16]", "exported .rodata", c, r);
    }
}

mod backend_sha2 {
    use super::*;

    fn skip() -> bool {
        !IS_SHA2
    }

    // ---------------------------------------------------------------- C41
    #[test]
    fn cfg_sha_oneshot() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 41);
        let lens: &[usize] = &[0, 1, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113,
                               119, 120, 127, 128, 129, 255, 256, 1000];
        for &n in lens {
            for _ in 0..iters(3) {
                let m = rng.vec(n);
                for (name, cf, rf, olen) in [
                    ("sha256", l.c.sha256(), l.r.sha256(), 32usize),
                    ("sha512", l.c.sha512(), l.r.sha512(), 64usize),
                ] {
                    let mut co = vec![0xAAu8; olen + 8];
                    let mut ro = vec![0xAAu8; olen + 8];
                    unsafe {
                        cf(co.as_mut_ptr(), m.as_ptr(), n);
                        rf(ro.as_mut_ptr(), m.as_ptr(), n);
                    }
                    eqb(name, &format!("inlen={n}"), &co, &ro);
                }
            }
        }
    }

    // ---------------------------------------------------------------- C42
    #[test]
    fn cfg_sha_incremental() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 42);
        for &nblocks in &[0usize, 1, 2, 3, 4] {
            for &tail in &[0usize, 1, 54, 55, 56, 57, 63, 110, 111, 112, 113, 127, 200] {
                let blk256 = rng.vec(nblocks * 64 + 8);
                let blk512 = rng.vec(nblocks * 128 + 8);
                let t = rng.vec(tail);
                // sha256
                let run = |sz: usize, blk: &[u8],
                           init: FnShaIncInit, blocks: FnShaIncBlocks, fin: FnShaIncFinalize,
                           olen: usize| -> (Vec<u8>, Vec<u8>) {
                    let mut st = vec![0xAAu8; sz];
                    unsafe { init(st.as_mut_ptr()) };
                    if nblocks > 0 {
                        unsafe { blocks(st.as_mut_ptr(), blk.as_ptr(), nblocks) };
                    } else {
                        unsafe { blocks(st.as_mut_ptr(), blk.as_ptr(), 0) };
                    }
                    let mid = st.clone();
                    let mut out = vec![0xAAu8; olen + 8];
                    unsafe { fin(out.as_mut_ptr(), st.as_mut_ptr(), t.as_ptr(), tail) };
                    let mut all = mid;
                    all.extend_from_slice(&st);
                    all.extend_from_slice(&out);
                    (out, all)
                };
                let (co, ca) = run(40, &blk256, l.c.sha256_inc_init(),
                                   l.c.sha256_inc_blocks(), l.c.sha256_inc_finalize(), 32);
                let (ro, ra) = run(40, &blk256, l.r.sha256_inc_init(),
                                   l.r.sha256_inc_blocks(), l.r.sha256_inc_finalize(), 32);
                eqb("sha256 incremental", &format!("nblocks={nblocks} tail={tail}"), &co, &ro);
                eqb("sha256 state", &format!("nblocks={nblocks} tail={tail}"), &ca, &ra);
                let (co, ca) = run(72, &blk512, l.c.sha512_inc_init(),
                                   l.c.sha512_inc_blocks(), l.c.sha512_inc_finalize(), 64);
                let (ro, ra) = run(72, &blk512, l.r.sha512_inc_init(),
                                   l.r.sha512_inc_blocks(), l.r.sha512_inc_finalize(), 64);
                eqb("sha512 incremental", &format!("nblocks={nblocks} tail={tail}"), &co, &ro);
                eqb("sha512 state", &format!("nblocks={nblocks} tail={tail}"), &ca, &ra);
            }
        }
    }

    // ---------------------------------------------------------------- C43
    #[test]
    fn cfg_sha_mgf1() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 43);
        for &inlen in &[1usize, 4, 16, 22, 32, 38, 48, 64, 100] {
            let inp = rng.vec(inlen);
            for &outlen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 96, 127, 128, 129, 1000] {
                for (name, cf, rf) in [
                    ("SPX_mgf1_256", l.c.mgf1_256(), l.r.mgf1_256()),
                    ("SPX_mgf1_512", l.c.mgf1_512(), l.r.mgf1_512()),
                ] {
                    let mut co = vec![0xAAu8; outlen + 16];
                    let mut ro = vec![0xAAu8; outlen + 16];
                    unsafe {
                        cf(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                        rf(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    }
                    eqb(name, &format!("inlen={inlen} outlen={outlen}"), &co, &ro);
                }
            }
        }
    }

    // ---------------------------------------------------------------- C44
    #[test]
    fn cfg_sha_seed_state() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 44);
        for case in 0..iters(60) {
            let ps = match case {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.vec(SPX_N),
            };
            let ss = rng.vec(SPX_N);
            let mut cc = CtxBuf::new();
            let mut rc = CtxBuf::new();
            cc.set_seeds(&ps, &ss);
            rc.set_seeds(&ps, &ss);
            unsafe {
                (l.c.seed_state())(cc.as_mut_ptr());
                (l.r.seed_state())(rc.as_mut_ptr());
            }
            eqb("SPX_seed_state", &format!("case={case}"), cc.live(), rc.live());
            // Rust must not write past the C struct either
            eqb("SPX_seed_state tail untouched", "",
                &cc.0[CTX_LIVE_BYTES..CTX_LIVE_BYTES + 64],
                &rc.0[CTX_LIVE_BYTES..CTX_LIVE_BYTES + 64]);
        }
    }
}

mod backend_shake {
    use super::*;

    fn skip() -> bool {
        !IS_SHAKE
    }

    // ---------------------------------------------------------------- C45
    #[test]
    fn cfg_shake_oneshot() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 45);
        for &inlen in &[0usize, 1, 135, 136, 137, 271, 272, 273, 1000] {
            let inp = rng.vec(inlen);
            for &outlen in &[1usize, 32, 135, 136, 137, 272, 1000] {
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    (l.c.shake256())(co.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                    (l.r.shake256())(ro.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                }
                eqb("shake256", &format!("inlen={inlen} outlen={outlen}"), &co, &ro);
            }
        }
    }

    // ---------------------------------------------------------------- C46
    #[test]
    fn cfg_shake_absorb_squeeze() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 46);
        for &inlen in &[0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
            let inp = rng.vec(inlen);
            for &nblocks in &[0usize, 1, 2, 3] {
                let run = |imp: &Impl| -> (Vec<u8>, Vec<u64>) {
                    let mut s = [0u64; 26];
                    unsafe { (imp.shake256_absorb())(s.as_mut_ptr(), inp.as_ptr(), inlen) };
                    let mut out = vec![0xAAu8; nblocks * 136 + 8];
                    unsafe { (imp.shake256_squeezeblocks())(out.as_mut_ptr(), nblocks, s.as_mut_ptr()) };
                    (out, s.to_vec())
                };
                let (co, cs) = run(&l.c);
                let (ro, rs) = run(&l.r);
                eqb("shake256_squeezeblocks", &format!("inlen={inlen} nblocks={nblocks}"), &co, &ro);
                eq("shake256 state", &format!("inlen={inlen} nblocks={nblocks}"), &cs[..25], &rs[..25]);
            }
        }
    }

    // ---------------------------------------------------------------- C47
    #[test]
    fn cfg_shake_incremental() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 47);
        for _ in 0..iters(40) {
            let nchunks = 1 + (rng.next_u32() % 5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| { let n = (rng.next_u64() % 300) as usize; rng.vec(n) })
                .collect();
            let squeezes: Vec<usize> = vec![0, 1, 32, 136, 137, 300];
            let run = |imp: &Impl| -> (Vec<Vec<u8>>, Vec<Vec<u64>>) {
                let mut s = [0u64; 26];
                let mut states = Vec::new();
                unsafe { (imp.shake256_inc_init())(s.as_mut_ptr()) };
                states.push(s.to_vec());
                for c in &chunks {
                    unsafe { (imp.shake256_inc_absorb())(s.as_mut_ptr(), c.as_ptr(), c.len()) };
                    states.push(s.to_vec());
                }
                unsafe { (imp.shake256_inc_finalize())(s.as_mut_ptr()) };
                states.push(s.to_vec());
                let mut outs = Vec::new();
                for &n in &squeezes {
                    let mut o = vec![0xAAu8; n + 8];
                    unsafe { (imp.shake256_inc_squeeze())(o.as_mut_ptr(), n, s.as_mut_ptr()) };
                    outs.push(o);
                    states.push(s.to_vec());
                }
                (outs, states)
            };
            let (co, cs) = run(&l.c);
            let (ro, rs) = run(&l.r);
            for (i, (a, b)) in co.iter().zip(ro.iter()).enumerate() {
                eqb("shake256_inc_squeeze", &format!("squeeze#{i}"), a, b);
            }
            for (i, (a, b)) in cs.iter().zip(rs.iter()).enumerate() {
                eq("shake inc state", &format!("step#{i}"), a, b);
            }
        }
    }
}

mod backend_haraka {
    use super::*;

    fn skip() -> bool {
        !IS_HARAKA
    }

    // ---------------------------------------------------------------- C48
    #[test]
    fn cfg_haraka_tweak_constants() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 48);
        for case in 0..iters(40) {
            let ps = match case {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.vec(SPX_N),
            };
            let ss = rng.vec(SPX_N);
            let mut cc = CtxBuf::new();
            let mut rc = CtxBuf::new();
            cc.set_seeds(&ps, &ss);
            rc.set_seeds(&ps, &ss);
            unsafe {
                (l.c.tweak_constants())(cc.as_mut_ptr());
                (l.r.tweak_constants())(rc.as_mut_ptr());
            }
            eqb("SPX_tweak_constants", &format!("case={case}"), cc.live(), rc.live());
        }
    }

    // ---------------------------------------------------------------- C49
    #[test]
    fn cfg_haraka_perm() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 49);
        for case in 0..iters(30) {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let in64: Vec<u8> = match case {
                0 => vec![0u8; 64],
                1 => vec![0xFFu8; 64],
                _ => rng.vec(64),
            };
            let in32 = in64[..32].to_vec();
            for (name, cf, rf, ilen, olen) in [
                ("SPX_haraka512", l.c.haraka512(), l.r.haraka512(), 64usize, 32usize),
                ("SPX_haraka512_perm", l.c.haraka512_perm(), l.r.haraka512_perm(), 64, 64),
                ("SPX_haraka256", l.c.haraka256(), l.r.haraka256(), 32, 32),
            ] {
                let inp = if ilen == 64 { &in64 } else { &in32 };
                let mut co = vec![0xAAu8; olen + 8];
                let mut ro = vec![0xAAu8; olen + 8];
                unsafe {
                    cf(co.as_mut_ptr(), inp.as_ptr(), cc.as_ptr());
                    rf(ro.as_mut_ptr(), inp.as_ptr(), rc.as_ptr());
                }
                eqb(name, &format!("case={case}"), &co, &ro);
            }
        }
    }

    // ---------------------------------------------------------------- C50
    #[test]
    fn cfg_haraka_s() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 50);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &inlen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 100] {
            let inp = rng.vec(inlen);
            for &outlen in &[1usize, 16, 31, 32, 33, 64, 100] {
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    (l.c.haraka_S())(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64, cc.as_ptr());
                    (l.r.haraka_S())(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64, rc.as_ptr());
                }
                eqb("SPX_haraka_S", &format!("inlen={inlen} outlen={outlen}"), &co, &ro);
            }
        }
    }

    // ---------------------------------------------------------------- C51
    #[test]
    fn cfg_haraka_s_incremental() {
        if skip() { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 51);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for _ in 0..iters(40) {
            let nchunks = 1 + (rng.next_u32() % 5) as usize;
            let chunks: Vec<Vec<u8>> = (0..nchunks)
                .map(|_| { let n = (rng.next_u64() % 90) as usize; rng.vec(n) })
                .collect();
            let squeezes: Vec<usize> = vec![0, 1, 16, 32, 33, 100];
            let run = |imp: &Impl, ctx: *const u8| -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
                let mut s = [0u8; 65];
                let mut states = Vec::new();
                unsafe { (imp.haraka_S_inc_init())(s.as_mut_ptr()) };
                states.push(s.to_vec());
                for c in &chunks {
                    unsafe { (imp.haraka_S_inc_absorb())(s.as_mut_ptr(), c.as_ptr(), c.len(), ctx) };
                    states.push(s.to_vec());
                }
                unsafe { (imp.haraka_S_inc_finalize())(s.as_mut_ptr()) };
                states.push(s.to_vec());
                let mut outs = Vec::new();
                for &n in &squeezes {
                    let mut o = vec![0xAAu8; n + 8];
                    unsafe { (imp.haraka_S_inc_squeeze())(o.as_mut_ptr(), n, s.as_mut_ptr(), ctx) };
                    outs.push(o);
                    states.push(s.to_vec());
                }
                (outs, states)
            };
            let (co, cs) = run(&l.c, cc.as_ptr());
            let (ro, rs) = run(&l.r, rc.as_ptr());
            for (i, (a, b)) in co.iter().zip(ro.iter()).enumerate() {
                eqb("SPX_haraka_S_inc_squeeze", &format!("squeeze#{i}"), a, b);
            }
            for (i, (a, b)) in cs.iter().zip(rs.iter()).enumerate() {
                eqb("haraka s_inc state", &format!("step#{i}"), a, b);
            }
        }
    }
}

// ---------------------------------------------------------------- C52
#[test]
fn cfg_ctx_layout() {
    let _g = serial();
    let l = libs();
    let mut rng = Rng::new(TEST_SEED ^ 52);
    // Seed a ctx with each implementation, then *cross*-use it: the C ctx fed to
    // the Rust prf_addr/thash and vice versa.  If the field offsets disagreed,
    // these would diverge.
    let ps = rng.vec(SPX_N);
    let ss = rng.vec(SPX_N);
    let (cc, rc) = make_ctx(&ps, &ss);
    let a = rng.addr();
    let data = rng.vec(2 * SPX_N);
    for (label, ctx) in [("C-seeded", cc.as_ptr()), ("Rust-seeded", rc.as_ptr())] {
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            (l.c.prf_addr())(co.as_mut_ptr(), ctx, a.as_ptr());
            (l.r.prf_addr())(ro.as_mut_ptr(), ctx, a.as_ptr());
        }
        eqb("prf_addr cross-ctx", label, &co, &ro);
        let mut ca = a;
        let mut ra = a;
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            (l.c.thash())(co.as_mut_ptr(), data.as_ptr(), 2, ctx, ca.as_mut_ptr());
            (l.r.thash())(ro.as_mut_ptr(), data.as_ptr(), 2, ctx, ra.as_mut_ptr());
        }
        eqb("thash cross-ctx", label, &co, &ro);
    }
}

// ###########################################################################
// Phase C — ERRORS.md
// ###########################################################################

mod errors {
    use super::*;

    /// Signs a fixed message once and caches (pk, sk, sig, m) for the whole
    /// module.  Both implementations produce the identical triple (asserted).
    struct Fixture {
        pk: Vec<u8>,
        sk: Vec<u8>,
        sig: Vec<u8>,
        m: Vec<u8>,
    }

    fn fixture() -> &'static Fixture {
        static F: std::sync::OnceLock<Fixture> = std::sync::OnceLock::new();
        F.get_or_init(|| {
            let l = libs();
            let mut rng = Rng::new(TEST_SEED ^ 0xE0);
            let seed = rng.vec(CRYPTO_SEEDBYTES);
            let (cpk, csk) = keypair_from_seed(&l.c, &seed);
            let (rpk, rsk) = keypair_from_seed(&l.r, &seed);
            assert_eq!(cpk, rpk);
            assert_eq!(csk, rsk);
            let m = rng.vec(64);
            let mut entropy = rng.vec(48);
            let (csig, _) = sign_detached(&l.c, &csk, &m, &mut entropy);
            let (rsig, _) = sign_detached(&l.r, &rsk, &m, &mut entropy);
            assert_eq!(csig, rsig);
            Fixture { pk: cpk, sk: csk, sig: csig, m }
        })
    }

    fn both_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8], what: &str) -> i32 {
        let l = libs();
        let cv = unsafe { (l.c.crypto_sign_verify())(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()) };
        let rv = unsafe { (l.r.crypto_sign_verify())(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()) };
        eq("crypto_sign_verify rc", what, cv, rv);
        cv
    }

    // ------------------------------------------------------- E1..E4
    #[test]
    fn err_verify_siglen_zero() {
        let _g = serial();
        let f = fixture();
        eq("rc", "siglen=0", both_verify(&f.sig, 0, &f.m, f.m.len(), &f.pk, "siglen=0"), -1);
    }

    #[test]
    fn err_verify_siglen_minus1() {
        let _g = serial();
        let f = fixture();
        eq("rc", "siglen=SPX_BYTES-1",
           both_verify(&f.sig, SPX_BYTES - 1, &f.m, f.m.len(), &f.pk, "siglen-1"), -1);
    }

    #[test]
    fn err_verify_siglen_plus1() {
        let _g = serial();
        let f = fixture();
        eq("rc", "siglen=SPX_BYTES+1",
           both_verify(&f.sig, SPX_BYTES + 1, &f.m, f.m.len(), &f.pk, "siglen+1"), -1);
    }

    #[test]
    fn err_verify_siglen_huge() {
        let _g = serial();
        let f = fixture();
        for &sl in &[usize::MAX, usize::MAX - 1, 1usize << 40, 1] {
            eq("rc", &format!("siglen={sl}"),
               both_verify(&f.sig, sl, &f.m, f.m.len(), &f.pk, "siglen huge"), -1);
        }
    }

    // ------------------------------------------------------- E5
    #[test]
    fn err_verify_corrupt_sig() {
        let _g = serial();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE5);
        // hit each structural region of the signature deterministically
        let mut positions: Vec<usize> = vec![
            0,                                  // R
            SPX_N,                              // start of FORS
            SPX_N + SPX_FORS_BYTES - 1,          // end of FORS
            SPX_N + SPX_FORS_BYTES,              // first WOTS sig
            SPX_N + SPX_FORS_BYTES + SPX_WOTS_BYTES, // first auth path
            SPX_BYTES - 1,                      // last byte
        ];
        for _ in 0..iters(10) {
            positions.push((rng.next_u64() % SPX_BYTES as u64) as usize);
        }
        for p in positions {
            let mut s = f.sig.clone();
            s[p] ^= 0x01;
            eq("rc", &format!("flip@{p}"),
               both_verify(&s, SPX_BYTES, &f.m, f.m.len(), &f.pk, &format!("flip@{p}")), -1);
        }
    }

    // ------------------------------------------------------- E6
    //
    // NOTE — a genuine quirk of this C reference (see ERRORS.md, E6):
    // `lib/blake/src/hash_blake.c` calls `blakeX_update(&S, m, mlen)`, but
    // `blake256_update`/`blake512_update` take a length in **bits**
    // (`(datalen >> 3) & 0x3F` bytes are copied).  So for the BLAKE backend only
    // roughly `mlen / 8` bytes of the message are absorbed, and flipping a byte
    // beyond that prefix is *accepted* by the C.  The Rust reproduces this
    // exactly (verified below), so the assertion here is the differential one;
    // the absolute `-1` is only asserted where the C provably does look at the
    // byte (message byte 0, which every backend absorbs).
    #[test]
    fn err_verify_corrupt_msg() {
        let _g = serial();
        let f = fixture();
        let mut rejected = 0usize;
        for p in 0..f.m.len() {
            let mut m = f.m.clone();
            m[p] ^= 0x80;
            // differential: this is the contract
            let rc = both_verify(&f.sig, SPX_BYTES, &m, m.len(), &f.pk, &format!("msgflip@{p}"));
            if rc == -1 {
                rejected += 1;
            }
            if p == 0 {
                eq("rc", "msgflip@0", rc, -1);
            }
        }
        assert!(rejected > 0, "no message corruption was rejected at all");
        if !IS_BLAKE {
            assert_eq!(rejected, f.m.len(),
                "every message byte must be covered by the digest for {BACKEND}");
        }
        // truncated / extended message length
        for ml in [0usize, 1, f.m.len() - 1, f.m.len() + 1] {
            let mut m = f.m.clone();
            m.resize(f.m.len() + 8, 0);
            let rc = both_verify(&f.sig, SPX_BYTES, &m, ml, &f.pk, &format!("mlen={ml}"));
            // A different mlen always changes the length fed to the hash, hence
            // the digest, for every backend.
            eq("rc", &format!("mlen={ml}"), rc, -1);
        }
    }

    // ------------------------------------------------------- E7 / E8
    #[test]
    fn err_verify_wrong_pk() {
        let _g = serial();
        let f = fixture();
        for p in 0..SPX_PK_BYTES {
            let mut pk = f.pk.clone();
            pk[p] ^= 0x01;
            let what = if p < SPX_N { "pub_seed flip" } else { "root flip" };
            eq("rc", &format!("{what}@{p}"),
               both_verify(&f.sig, SPX_BYTES, &f.m, f.m.len(), &pk, what), -1);
        }
    }

    #[test]
    fn err_verify_wrong_pubseed() {
        let _g = serial();
        let f = fixture();
        let mut pk = f.pk.clone();
        for b in pk[..SPX_N].iter_mut() {
            *b ^= 0xFF;
        }
        eq("rc", "pub_seed inverted",
           both_verify(&f.sig, SPX_BYTES, &f.m, f.m.len(), &pk, "pub_seed inverted"), -1);
    }

    // ------------------------------------------------------- E9
    #[test]
    fn err_verify_all_zero() {
        let _g = serial();
        let f = fixture();
        let zsig = vec![0u8; SPX_BYTES];
        let zpk = vec![0u8; SPX_PK_BYTES];
        let fsig = vec![0xFFu8; SPX_BYTES];
        let fpk = vec![0xFFu8; SPX_PK_BYTES];
        both_verify(&zsig, SPX_BYTES, &[], 0, &zpk, "all-zero");
        both_verify(&fsig, SPX_BYTES, &[], 0, &fpk, "all-FF");
        both_verify(&zsig, SPX_BYTES, &f.m, f.m.len(), &f.pk, "zero sig / real pk");
        both_verify(&f.sig, SPX_BYTES, &f.m, f.m.len(), &zpk, "real sig / zero pk");
    }

    // ------------------------------------------------------- E10
    #[test]
    fn err_verify_empty_msg_ok() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xEA);
        let mut entropy = rng.vec(48);
        let (sig, _) = sign_detached(&l.c, &f.sk, &[], &mut entropy);
        let (sig2, _) = sign_detached(&l.r, &f.sk, &[], &mut entropy);
        eqb("empty-message signature", "", &sig, &sig2);
        eq("rc", "mlen=0", both_verify(&sig, SPX_BYTES, &[], 0, &f.pk, "mlen=0"), 0);
    }

    // ------------------------------------------------------- E11..E15
    fn both_open(sm: &[u8], smlen: u64, pk: &[u8], what: &str) -> i32 {
        let l = libs();
        let bufsz = std::cmp::max(sm.len(), smlen as usize) + 32;
        let mut cm = vec![0xAAu8; bufsz];
        let mut rm = vec![0xAAu8; bufsz];
        let mut cml = 0xDEAD_BEEFu64;
        let mut rml = 0xDEAD_BEEFu64;
        let cr = unsafe { (l.c.crypto_sign_open())(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk.as_ptr()) };
        let rr = unsafe { (l.r.crypto_sign_open())(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk.as_ptr()) };
        eq("crypto_sign_open rc", what, cr, rr);
        eq("crypto_sign_open *mlen", what, cml, rml);
        eqb("crypto_sign_open m", what, &cm, &rm);
        cr
    }

    #[test]
    fn err_open_smlen_zero() {
        let _g = serial();
        let f = fixture();
        eq("rc", "smlen=0", both_open(&f.sig, 0, &f.pk, "smlen=0"), -1);
    }

    #[test]
    fn err_open_smlen_short() {
        let _g = serial();
        let f = fixture();
        let mut sm = f.sig.clone();
        sm.extend_from_slice(&f.m);
        for &n in &[1u64, 2, SPX_N as u64, (SPX_BYTES - 1) as u64] {
            eq("rc", &format!("smlen={n}"), both_open(&sm, n, &f.pk, "short smlen"), -1);
        }
    }

    #[test]
    fn err_open_smlen_exact() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE13);
        let mut entropy = rng.vec(48);
        let (sm, smlen) = sign_attached(&l.c, &f.sk, &[], &mut entropy);
        eq("smlen", "empty msg", smlen, SPX_BYTES as u64);
        eq("rc", "smlen=SPX_BYTES", both_open(&sm, SPX_BYTES as u64, &f.pk, "exact"), 0);
    }

    #[test]
    fn err_open_corrupt() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE14);
        let mut entropy = rng.vec(48);
        let (sm, smlen) = sign_attached(&l.c, &f.sk, &f.m, &mut entropy);
        // sanity: intact opens
        eq("rc", "intact", both_open(&sm, smlen, &f.pk, "intact"), 0);
        for p in [0usize, SPX_N, SPX_BYTES - 1, SPX_BYTES, sm.len() - 1] {
            let mut s = sm.clone();
            s[p] ^= 0x01;
            // Differential is the contract.  The absolute `-1` only holds for
            // bytes the C actually hashes: everything inside the signature, and
            // message byte 0.  See the note on `err_verify_corrupt_msg` for the
            // BLAKE bit/byte-length quirk that makes the message *tail*
            // invisible to the digest.
            let rc = both_open(&s, smlen, &f.pk, &format!("flip@{p}"));
            if p <= SPX_BYTES {
                eq("rc", &format!("flip@{p}"), rc, -1);
            }
        }
        // wrong pk
        let mut pk = f.pk.clone();
        pk[SPX_PK_BYTES - 1] ^= 0x01;
        eq("rc", "wrong pk", both_open(&sm, smlen, &pk, "wrong pk"), -1);
    }

    #[test]
    fn err_open_smlen_extra() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE15);
        let mut entropy = rng.vec(48);
        let (mut sm, smlen) = sign_attached(&l.c, &f.sk, &f.m, &mut entropy);
        sm.push(0x5A);
        // A different smlen changes *mlen, hence the length hashed, for every
        // backend, so both must reject.
        eq("rc", "smlen+1", both_open(&sm, smlen + 1, &f.pk, "extra byte"), -1);
        eq("rc", "smlen-1", both_open(&sm, smlen - 1, &f.pk, "one short"), -1);
    }

    // ------------------------------------------------------- E16..E19
    fn se_init(imp: &Impl, seed: &[u8], div: &[u8], maxlen: u64) -> (i32, AesXofStruct) {
        let mut ctx = AesXofStruct::zeroed();
        // fill with a recognisable pattern so "untouched" is observable
        ctx.buffer = [0x11; 16];
        ctx.buffer_pos = 0x2222_2222_2222_2222;
        ctx.length_remaining = 0x3333_3333_3333_3333;
        ctx.key = [0x44; 32];
        ctx.ctr = [0x55; 16];
        let mut s = seed.to_vec();
        let mut d = div.to_vec();
        let rc = unsafe { (imp.seedexpander_init())(&mut ctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        (rc, ctx)
    }

    #[test]
    fn err_seedexpander_init_maxlen_bound() {
        let _g = serial();
        let l = libs();
        let seed = [0x01u8; 32];
        let div = [0x02u8; 8];
        let (crc, cctx) = se_init(&l.c, &seed, &div, 0x1_0000_0000);
        let (rrc, rctx) = se_init(&l.r, &seed, &div, 0x1_0000_0000);
        eq("seedexpander_init rc", "maxlen=2^32", crc, rrc);
        eq("seedexpander_init rc", "maxlen=2^32", crc, -1);
        eqb("ctx untouched", "maxlen=2^32", &cctx.bytes(), &rctx.bytes());
    }

    #[test]
    fn err_seedexpander_init_maxlen_huge() {
        let _g = serial();
        let l = libs();
        let seed = [0x01u8; 32];
        let div = [0x02u8; 8];
        for &m in &[u64::MAX, 0x1_0000_0001, 1u64 << 63] {
            let (crc, cctx) = se_init(&l.c, &seed, &div, m);
            let (rrc, rctx) = se_init(&l.r, &seed, &div, m);
            eq("seedexpander_init rc", &format!("maxlen={m:#x}"), crc, rrc);
            eq("seedexpander_init rc", &format!("maxlen={m:#x}"), crc, -1);
            eqb("ctx untouched", &format!("maxlen={m:#x}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    #[test]
    fn err_seedexpander_init_maxlen_ok() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE18);
        for &m in &[0xFFFF_FFFFu64, 0xFFFF_FFFE, 1, 255, 256, 65535] {
            let seed = rng.vec(32);
            let div = rng.vec(8);
            let (crc, cctx) = se_init(&l.c, &seed, &div, m);
            let (rrc, rctx) = se_init(&l.r, &seed, &div, m);
            eq("seedexpander_init rc", &format!("maxlen={m}"), crc, rrc);
            eq("seedexpander_init rc", &format!("maxlen={m}"), crc, 0);
            eqb("ctx", &format!("maxlen={m}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    #[test]
    fn err_seedexpander_init_maxlen_zero() {
        let _g = serial();
        let l = libs();
        let seed = [0x07u8; 32];
        let div = [0x08u8; 8];
        let (crc, mut cctx) = se_init(&l.c, &seed, &div, 0);
        let (rrc, mut rctx) = se_init(&l.r, &seed, &div, 0);
        eq("rc", "maxlen=0", crc, rrc);
        eq("rc", "maxlen=0", crc, 0);
        eqb("ctx", "maxlen=0", &cctx.bytes(), &rctx.bytes());
        // every subsequent draw must fail with RNG_BAD_REQ_LEN
        for &n in &[0u64, 1, 16] {
            let mut co = vec![0xAAu8; 32];
            let mut ro = vec![0xAAu8; 32];
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), n) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), n) };
            eq("seedexpander rc", &format!("n={n}"), cr, rr);
            eq("seedexpander rc", &format!("n={n}"), cr, -3);
            eqb("out untouched", &format!("n={n}"), &co, &ro);
            eqb("ctx untouched", &format!("n={n}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    // ------------------------------------------------------- E20
    #[test]
    fn err_seedexpander_null_out() {
        let _g = serial();
        let l = libs();
        let seed = [0x09u8; 32];
        let div = [0x0Au8; 8];
        for &n in &[0u64, 1, 16, 1000] {
            let (_, mut cctx) = se_init(&l.c, &seed, &div, 1000);
            let (_, mut rctx) = se_init(&l.r, &seed, &div, 1000);
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, std::ptr::null_mut(), n) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, std::ptr::null_mut(), n) };
            eq("seedexpander rc", &format!("x=NULL n={n}"), cr, rr);
            eq("seedexpander rc", &format!("x=NULL n={n}"), cr, -2);
            eqb("ctx untouched", &format!("x=NULL n={n}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    // ------------------------------------------------------- E21 / E22 / E23 / E24
    #[test]
    fn err_seedexpander_xlen_eq_remaining() {
        let _g = serial();
        let l = libs();
        let seed = [0x0Bu8; 32];
        let div = [0x0Cu8; 8];
        for &maxlen in &[1u64, 16, 17, 100] {
            let (_, mut cctx) = se_init(&l.c, &seed, &div, maxlen);
            let (_, mut rctx) = se_init(&l.r, &seed, &div, maxlen);
            let mut co = vec![0xAAu8; maxlen as usize + 8];
            let mut ro = vec![0xAAu8; maxlen as usize + 8];
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), maxlen) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), maxlen) };
            eq("seedexpander rc", &format!("xlen==remaining={maxlen}"), cr, rr);
            eq("seedexpander rc", &format!("xlen==remaining={maxlen}"), cr, -3);
            eqb("out untouched", "", &co, &ro);
            eqb("ctx untouched", "", &cctx.bytes(), &rctx.bytes());
        }
    }

    #[test]
    fn err_seedexpander_xlen_gt_remaining() {
        let _g = serial();
        let l = libs();
        let seed = [0x0Du8; 32];
        let div = [0x0Eu8; 8];
        let (_, mut cctx) = se_init(&l.c, &seed, &div, 16);
        let (_, mut rctx) = se_init(&l.r, &seed, &div, 16);
        for &n in &[17u64, 100, u64::MAX] {
            let mut co = vec![0xAAu8; 32];
            let mut ro = vec![0xAAu8; 32];
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), n) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), n) };
            eq("seedexpander rc", &format!("xlen={n}>remaining=16"), cr, rr);
            eq("seedexpander rc", &format!("xlen={n}"), cr, -3);
            eqb("ctx untouched", "", &cctx.bytes(), &rctx.bytes());
        }
    }

    #[test]
    fn err_seedexpander_xlen_zero() {
        let _g = serial();
        let l = libs();
        let seed = [0x0Fu8; 32];
        let div = [0x10u8; 8];
        let (_, mut cctx) = se_init(&l.c, &seed, &div, 1000);
        let (_, mut rctx) = se_init(&l.r, &seed, &div, 1000);
        let before = cctx.bytes();
        let mut co = vec![0xAAu8; 16];
        let mut ro = vec![0xAAu8; 16];
        let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), 0) };
        let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), 0) };
        eq("seedexpander rc", "xlen=0", cr, rr);
        eq("seedexpander rc", "xlen=0", cr, 0);
        eqb("out untouched", "xlen=0", &co, &ro);
        eqb("ctx", "xlen=0", &cctx.bytes(), &rctx.bytes());
        // the C only decrements length_remaining by 0 and never enters the loop
        assert_eq!(before, cctx.bytes(), "xlen=0 must leave the ctx unchanged");
    }

    #[test]
    fn err_seedexpander_xlen_max_ok() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE24);
        for &maxlen in &[2u64, 17, 33, 1000] {
            let seed = rng.vec(32);
            let div = rng.vec(8);
            let (_, mut cctx) = se_init(&l.c, &seed, &div, maxlen);
            let (_, mut rctx) = se_init(&l.r, &seed, &div, maxlen);
            let n = maxlen - 1;
            let mut co = vec![0xAAu8; n as usize + 8];
            let mut ro = vec![0xAAu8; n as usize + 8];
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), n) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), n) };
            eq("seedexpander rc", &format!("xlen={n}"), cr, rr);
            eq("seedexpander rc", &format!("xlen={n}"), cr, 0);
            eqb("out", &format!("xlen={n}"), &co, &ro);
            eqb("ctx", &format!("xlen={n}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    // ------------------------------------------------------- E25
    #[test]
    fn err_seedexpander_buffer_pos_overflow() {
        let _g = serial();
        let l = libs();
        // `16 - ctx->buffer_pos` underflows in `unsigned long`; the C then takes
        // the "buffer has what we need" branch and memcpy's out of the 16-byte
        // buffer.  Keep the requested length tiny (1 byte) so the out-of-bounds
        // read stays inside the struct itself and cannot fault, but the branch
        // and the return value are still observed.
        for &bp in &[17u64, 20, 24] {
            let mut cctx = AesXofStruct::zeroed();
            cctx.buffer = [0x77; 16];
            cctx.key = [0x11; 32];
            cctx.ctr = [0x22; 16];
            cctx.length_remaining = 1000;
            cctx.buffer_pos = bp;
            let mut rctx = cctx;
            let mut co = vec![0xAAu8; 8];
            let mut ro = vec![0xAAu8; 8];
            let cr = unsafe { (l.c.seedexpander())(&mut cctx, co.as_mut_ptr(), 1) };
            let rr = unsafe { (l.r.seedexpander())(&mut rctx, ro.as_mut_ptr(), 1) };
            eq("seedexpander rc", &format!("buffer_pos={bp}"), cr, rr);
            eq("seedexpander rc", &format!("buffer_pos={bp}"), cr, 0);
            eqb("out", &format!("buffer_pos={bp}"), &co, &ro);
            eqb("ctx", &format!("buffer_pos={bp}"), &cctx.bytes(), &rctx.bytes());
        }
    }

    // ------------------------------------------------------- E26
    #[test]
    fn err_randombytes_init_null_ps() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE26);
        for _ in 0..iters(10) {
            let mut e = rng.vec(48);
            let draws = [32usize, 48];
            let a = drbg_run(&l.c, &mut e, None, &draws);
            let b = drbg_run(&l.r, &mut e, None, &draws);
            for (i, (x, y)) in a.0.iter().zip(b.0.iter()).enumerate() {
                eqb("randombytes (NULL ps)", &format!("draw#{i}"), x, y);
            }
            eq("DRBG_ctx", "NULL ps", a.1, b.1);
        }
    }

    // ------------------------------------------------------- E27 / E28
    #[test]
    fn err_randombytes_xlen_zero() {
        let _g = serial();
        let l = libs();
        let e = vec![0x5Au8; 48];
        let run = |imp: &Impl| -> (i32, DrbgStruct, DrbgStruct, Vec<u8>) {
            let mut ee = e.clone();
            unsafe { (imp.randombytes_init())(ee.as_mut_ptr(), std::ptr::null_mut()) };
            let before = unsafe { *imp.drbg_ctx() };
            let mut o = vec![0xAAu8; 16];
            let rc = unsafe { (imp.randombytes())(o.as_mut_ptr(), 0) };
            let after = unsafe { *imp.drbg_ctx() };
            (rc, before, after, o)
        };
        let (crc, cb, ca, co) = run(&l.c);
        let (rrc, rb, ra, ro) = run(&l.r);
        eq("randombytes rc", "xlen=0", crc, rrc);
        eq("randombytes rc", "xlen=0", crc, 0);
        eq("DRBG before", "xlen=0", cb, rb);
        eq("DRBG after", "xlen=0", ca, ra);
        eqb("out untouched", "xlen=0", &co, &ro);
        // C still runs the tail update: the state must have advanced
        assert_ne!(cb, ca, "xlen=0 must still update the DRBG state");
        assert_eq!(ca.reseed_counter, cb.reseed_counter + 1);
    }

    #[test]
    fn err_randombytes_partial_block() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE28);
        for &n in &[1usize, 2, 15, 16, 17, 31, 32, 33, 47, 48, 49] {
            let mut e = rng.vec(48);
            let draws = [n];
            let a = drbg_run(&l.c, &mut e, None, &draws);
            let b = drbg_run(&l.r, &mut e, None, &draws);
            eqb("randombytes", &format!("xlen={n}"), &a.0[0], &b.0[0]);
            eq("DRBG_ctx", &format!("xlen={n}"), a.1, b.1);
            // the guard bytes past `xlen` must be untouched
            assert!(a.0[0][n..].iter().all(|&x| x == 0xAA));
        }
    }

    // ------------------------------------------------------- E29 / E30
    #[test]
    fn err_drbg_update_null_pd() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE29);
        for _ in 0..iters(50) {
            let key = rng.vec(32);
            let v = rng.vec(16);
            let run = |imp: &Impl| -> (Vec<u8>, Vec<u8>) {
                let mut k = key.clone();
                let mut vv = v.clone();
                unsafe { (imp.drbg_update())(std::ptr::null_mut(), k.as_mut_ptr(), vv.as_mut_ptr()) };
                (k, vv)
            };
            let (ck, cv) = run(&l.c);
            let (rk, rv) = run(&l.r);
            eqb("DRBG_Update Key (NULL pd)", "", &ck, &rk);
            eqb("DRBG_Update V (NULL pd)", "", &cv, &rv);
        }
    }

    #[test]
    fn err_drbg_update_v_all_ff() {
        let _g = serial();
        let l = libs();
        let key = vec![0x33u8; 32];
        for v in [vec![0xFFu8; 16], {
            let mut x = vec![0u8; 16];
            x[15] = 0xFF;
            x
        }] {
            for pd in [None, Some(vec![0x77u8; 48])] {
                let run = |imp: &Impl| -> (Vec<u8>, Vec<u8>) {
                    let mut k = key.clone();
                    let mut vv = v.clone();
                    let mut p = pd.clone();
                    let pp = match p.as_mut() {
                        Some(x) => x.as_mut_ptr(),
                        None => std::ptr::null_mut(),
                    };
                    unsafe { (imp.drbg_update())(pp, k.as_mut_ptr(), vv.as_mut_ptr()) };
                    (k, vv)
                };
                let (ck, cv) = run(&l.c);
                let (rk, rv) = run(&l.r);
                eqb("DRBG_Update Key (V=FF..)", "", &ck, &rk);
                eqb("DRBG_Update V (V=FF..)", "", &cv, &rv);
            }
        }
    }

    // ------------------------------------------------------- E31
    #[test]
    fn err_randombytes_v_all_ff() {
        let _g = serial();
        let l = libs();
        for vpat in [vec![0xFFu8; 16], vec![0u8; 16]] {
            let run = |imp: &Impl| -> (Vec<u8>, DrbgStruct) {
                let d = imp.drbg_ctx();
                unsafe {
                    (*d).key = [0x5Cu8; 32];
                    (*d).v.copy_from_slice(&vpat);
                    (*d).reseed_counter = 7;
                }
                let mut o = vec![0xAAu8; 80];
                unsafe { (imp.randombytes())(o.as_mut_ptr(), 64) };
                (o, unsafe { *d })
            };
            let (co, cd) = run(&l.c);
            let (ro, rd) = run(&l.r);
            eqb("randombytes (V=pattern)", "", &co, &ro);
            eq("DRBG_ctx", "V=pattern", cd, rd);
        }
    }

    // ------------------------------------------------------- E32 / E33
    #[test]
    fn err_ull_to_bytes_outlen_zero() {
        let _g = serial();
        let l = libs();
        for &v in &[0u64, 1, u64::MAX] {
            let mut cb = vec![0xAAu8; 16];
            let mut rb = vec![0xAAu8; 16];
            unsafe {
                (l.c.ull_to_bytes())(cb.as_mut_ptr(), 0, v);
                (l.r.ull_to_bytes())(rb.as_mut_ptr(), 0, v);
            }
            eqb("SPX_ull_to_bytes", "outlen=0", &cb, &rb);
            assert!(cb.iter().all(|&x| x == 0xAA), "outlen=0 must write nothing");
        }
    }

    #[test]
    fn err_ull_to_bytes_outlen_gt8() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE33);
        for &outlen in &[9u32, 10, 12, 16, 24] {
            for _ in 0..iters(30) {
                let v = rng.next_u64();
                let mut cb = vec![0xAAu8; 40];
                let mut rb = vec![0xAAu8; 40];
                unsafe {
                    (l.c.ull_to_bytes())(cb.as_mut_ptr(), outlen, v);
                    (l.r.ull_to_bytes())(rb.as_mut_ptr(), outlen, v);
                }
                eqb("SPX_ull_to_bytes", &format!("outlen={outlen} v={v:#x}"), &cb, &rb);
            }
        }
    }

    // ------------------------------------------------------- E34 / E35
    #[test]
    fn err_bytes_to_ull_inlen_zero() {
        let _g = serial();
        let l = libs();
        let b = [0xFFu8; 16];
        let c = unsafe { (l.c.bytes_to_ull())(b.as_ptr(), 0) };
        let r = unsafe { (l.r.bytes_to_ull())(b.as_ptr(), 0) };
        eq("SPX_bytes_to_ull", "inlen=0", c, r);
        eq("SPX_bytes_to_ull", "inlen=0", c, 0);
        // also with a NULL input, which the C never dereferences
        let c = unsafe { (l.c.bytes_to_ull())(std::ptr::null(), 0) };
        let r = unsafe { (l.r.bytes_to_ull())(std::ptr::null(), 0) };
        eq("SPX_bytes_to_ull", "inlen=0 in=NULL", c, r);
    }

    #[test]
    fn err_bytes_to_ull_inlen_gt8() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE35);
        // 8*(inlen-1-i) exceeds 63 for inlen > 8: UB in C, in practice the
        // x86-64 `shl` masks the count.  Whatever it does, both sides must agree.
        for &inlen in &[9u32, 10, 12, 16] {
            for _ in 0..iters(20) {
                let b = rng.vec(32);
                let c = unsafe { (l.c.bytes_to_ull())(b.as_ptr(), inlen) };
                let r = unsafe { (l.r.bytes_to_ull())(b.as_ptr(), inlen) };
                eq("SPX_bytes_to_ull", &format!("inlen={inlen}"), c, r);
            }
        }
    }

    // ------------------------------------------------------- E36 / E37
    #[test]
    fn err_set_type_out_of_range() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE36);
        let mut vals: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 255, 256, 257, 0x0000_01FF, 0xFFFF_FF01, u32::MAX];
        for _ in 0..iters(50) {
            vals.push(rng.next_u32());
        }
        for &v in &vals {
            for start in [[0u32; 8], [u32::MAX; 8], rng.addr()] {
                let mut ca = start;
                let mut ra = start;
                unsafe {
                    (l.c.set_type())(ca.as_mut_ptr(), v);
                    (l.r.set_type())(ra.as_mut_ptr(), v);
                }
                eqb("SPX_set_type", &format!("type={v:#x}"), addr_bytes(&ca), addr_bytes(&ra));
                // and the C semantics: only the low byte lands in the address
                let mut want = addr_bytes(&start).to_vec();
                want[SPX_OFFSET_TYPE] = v as u8;
                eqb("SPX_set_type (expected)", &format!("type={v:#x}"), &want, addr_bytes(&ca));
            }
        }
    }

    #[test]
    fn err_addr_setters_truncate() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE37);
        for &v in &[256u32, 257, 0x1_0000, 0xFFFF_FF01, u32::MAX] {
            let start = rng.addr();
            for (name, off, cg, rg) in [
                ("set_layer_addr", SPX_OFFSET_LAYER, l.c.set_layer_addr(), l.r.set_layer_addr()),
                ("set_chain_addr", SPX_OFFSET_CHAIN_ADDR, l.c.set_chain_addr(), l.r.set_chain_addr()),
                ("set_hash_addr", SPX_OFFSET_HASH_ADDR, l.c.set_hash_addr(), l.r.set_hash_addr()),
                ("set_tree_height", SPX_OFFSET_TREE_HGT, l.c.set_tree_height(), l.r.set_tree_height()),
            ] {
                let mut ca = start;
                let mut ra = start;
                unsafe {
                    cg(ca.as_mut_ptr(), v);
                    rg(ra.as_mut_ptr(), v);
                }
                eqb(name, &format!("v={v:#x}"), addr_bytes(&ca), addr_bytes(&ra));
                let mut want = addr_bytes(&start).to_vec();
                want[off] = v as u8;
                eqb(name, &format!("truncation v={v:#x}"), &want, addr_bytes(&ca));
            }
        }
    }

    // ------------------------------------------------------- E38 / E40
    #[test]
    fn err_thash_inblocks_zero() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE38);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        let data = rng.vec(64);
        for a in [[0u32; 8], [u32::MAX; 8], rng.addr()] {
            let mut ca = a;
            let mut ra = a;
            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                (l.c.thash())(co.as_mut_ptr(), data.as_ptr(), 0, cc.as_ptr(), ca.as_mut_ptr());
                (l.r.thash())(ro.as_mut_ptr(), data.as_ptr(), 0, rc.as_ptr(), ra.as_mut_ptr());
            }
            eqb("SPX_thash", "inblocks=0", &co, &ro);
            eqb("SPX_thash addr", "inblocks=0", addr_bytes(&ca), addr_bytes(&ra));
        }
    }

    #[test]
    fn err_thash_inblocks_large() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE40);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &nb in &[SPX_WOTS_LEN as u32, SPX_FORS_TREES as u32, 200, 255] {
            let data = rng.vec(nb as usize * SPX_N);
            let a = rng.addr();
            let mut ca = a;
            let mut ra = a;
            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                (l.c.thash())(co.as_mut_ptr(), data.as_ptr(), nb, cc.as_ptr(), ca.as_mut_ptr());
                (l.r.thash())(ro.as_mut_ptr(), data.as_ptr(), nb, rc.as_ptr(), ra.as_mut_ptr());
            }
            eqb("SPX_thash", &format!("inblocks={nb}"), &co, &ro);
        }
    }

    // ------------------------------------------------------- E41
    #[test]
    fn err_treehash_height_zero() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE41);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &leaf_idx in &[0u32, 1, u32::MAX] {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            let mut croot = vec![0xAAu8; SPX_N + 8];
            let mut rroot = vec![0xAAu8; SPX_N + 8];
            let mut cap = vec![0xAAu8; 2 * SPX_N + 64];
            let mut rap = vec![0xAAu8; 2 * SPX_N + 64];
            unsafe {
                (l.c.treehash())(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(),
                    leaf_idx, 0, 0, test_gen_leaf, ca.as_mut_ptr());
                (l.r.treehash())(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(),
                    leaf_idx, 0, 0, test_gen_leaf, ra.as_mut_ptr());
            }
            eqb("SPX_treehash root", "height=0", &croot, &rroot);
            eqb("SPX_treehash auth_path", "height=0", &cap, &rap);
            eqb("SPX_treehash tree_addr", "height=0", addr_bytes(&ca), addr_bytes(&ra));
        }
    }

    // ------------------------------------------------------- E42
    #[test]
    fn err_compute_root_height_one() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE42);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        for &leaf_idx in &[0u32, 1, 2, 3, u32::MAX] {
            for &off in &[0u32, 1, u32::MAX] {
                let leaf = rng.vec(SPX_N);
                let auth = rng.vec(SPX_N);
                let base = rng.addr();
                let mut ca = base;
                let mut ra = base;
                let mut croot = vec![0xAAu8; SPX_N + 8];
                let mut rroot = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    (l.c.compute_root())(croot.as_mut_ptr(), leaf.as_ptr(), leaf_idx, off,
                        auth.as_ptr(), 1, cc.as_ptr(), ca.as_mut_ptr());
                    (l.r.compute_root())(rroot.as_mut_ptr(), leaf.as_ptr(), leaf_idx, off,
                        auth.as_ptr(), 1, rc.as_ptr(), ra.as_mut_ptr());
                }
                let c = format!("h=1 leaf_idx={leaf_idx} off={off}");
                eqb("SPX_compute_root", &c, &croot, &rroot);
                eqb("SPX_compute_root addr", &c, addr_bytes(&ca), addr_bytes(&ra));
            }
        }
    }

    // ------------------------------------------------------- E45
    #[test]
    fn err_fors_extreme_indices() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE45);
        let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
        let fa = rng.addr();
        for m in [vec![0u8; SPX_FORS_MSG_BYTES], vec![0xFFu8; SPX_FORS_MSG_BYTES],
                  vec![0xAAu8; SPX_FORS_MSG_BYTES], vec![0x55u8; SPX_FORS_MSG_BYTES]] {
            let mut csig = vec![0xAAu8; SPX_FORS_BYTES];
            let mut rsig = vec![0xAAu8; SPX_FORS_BYTES];
            let mut cpk = vec![0xAAu8; SPX_N + 8];
            let mut rpk = vec![0xAAu8; SPX_N + 8];
            unsafe {
                (l.c.fors_sign())(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr());
                (l.r.fors_sign())(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), rc.as_ptr(), fa.as_ptr());
            }
            eqb("SPX_fors_sign sig", &format!("m={:#x}", m[0]), &csig, &rsig);
            eqb("SPX_fors_sign pk", &format!("m={:#x}", m[0]), &cpk, &rpk);
            let mut cpk2 = vec![0xAAu8; SPX_N + 8];
            let mut rpk2 = vec![0xAAu8; SPX_N + 8];
            unsafe {
                (l.c.fors_pk_from_sig())(cpk2.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr());
                (l.r.fors_pk_from_sig())(rpk2.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), rc.as_ptr(), fa.as_ptr());
            }
            eqb("SPX_fors_pk_from_sig", &format!("m={:#x}", m[0]), &cpk2, &rpk2);
            assert_eq!(cpk2, cpk, "fors round trip");
        }
    }

    // ------------------------------------------------------- E46
    #[test]
    fn err_chain_lengths_extremes() {
        let _g = serial();
        let l = libs();
        for pat in [0u8, 0x01, 0x0F, 0x10, 0x7F, 0x80, 0xF0, 0xFE, 0xFF] {
            let m = vec![pat; SPX_N];
            let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
            let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
            unsafe {
                (l.c.chain_lengths())(cl.as_mut_ptr(), m.as_ptr());
                (l.r.chain_lengths())(rl.as_mut_ptr(), m.as_ptr());
            }
            eq("SPX_chain_lengths", &format!("pat={pat:#x}"), &cl, &rl);
            // every base-w digit must be < SPX_WOTS_W
            for x in &cl[..SPX_WOTS_LEN] {
                assert!(*x < SPX_WOTS_W as u32);
            }
        }
    }

    // ------------------------------------------------------- E47 / E48
    #[test]
    fn err_sign_mlen_zero() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE47);
        let mut entropy = rng.vec(48);
        let (cs, cl2) = sign_detached(&l.c, &f.sk, &[], &mut entropy);
        let (rs, rl2) = sign_detached(&l.r, &f.sk, &[], &mut entropy);
        eq("siglen", "mlen=0", cl2, rl2);
        eq("siglen", "mlen=0", cl2, SPX_BYTES);
        eqb("signature", "mlen=0", &cs, &rs);
        eq("rc", "verify mlen=0", both_verify(&cs, SPX_BYTES, &[], 0, &f.pk, "mlen=0"), 0);
    }

    #[test]
    fn err_sign_open_mlen_zero() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        let mut rng = Rng::new(TEST_SEED ^ 0xE48);
        let mut entropy = rng.vec(48);
        let (csm, cslen) = sign_attached(&l.c, &f.sk, &[], &mut entropy);
        let (rsm, rslen) = sign_attached(&l.r, &f.sk, &[], &mut entropy);
        eq("smlen", "mlen=0", cslen, rslen);
        eq("smlen", "mlen=0", cslen, SPX_BYTES as u64);
        eqb("sm", "mlen=0", &csm, &rsm);
        eq("rc", "open mlen=0", both_open(&csm, cslen, &f.pk, "mlen=0"), 0);
    }

    // ------------------------------------------------------- E49 (blake only)
    #[test]
    fn err_blake_update_zero() {
        if !IS_BLAKE { return; }
        let _g = serial();
        let l = libs();
        let run256 = |imp: &Impl| -> (Vec<u8>, Vec<u8>) {
            let mut s = BlakeState256::zeroed();
            unsafe { (imp.blake256_init())(&mut s) };
            unsafe { (imp.blake256_update())(&mut s, std::ptr::null(), 0) };
            let st = s.bytes();
            let mut d = vec![0xAAu8; 40];
            unsafe { (imp.blake256_final())(&mut s, d.as_mut_ptr()) };
            (d, st)
        };
        let (cd, cs) = run256(&l.c);
        let (rd, rs) = run256(&l.r);
        eqb("blake256 update(0)", "state", &cs, &rs);
        eqb("blake256 update(0)", "digest", &cd, &rd);
        let run512 = |imp: &Impl| -> (Vec<u8>, Vec<u8>) {
            let mut s = BlakeState512::zeroed();
            unsafe { (imp.blake512_init())(&mut s) };
            unsafe { (imp.blake512_update())(&mut s, std::ptr::null(), 0) };
            let st = s.bytes();
            let mut d = vec![0xAAu8; 72];
            unsafe { (imp.blake512_final())(&mut s, d.as_mut_ptr()) };
            (d, st)
        };
        let (cd, cs) = run512(&l.c);
        let (rd, rs) = run512(&l.r);
        eqb("blake512 update(0)", "state", &cs, &rs);
        eqb("blake512 update(0)", "digest", &cd, &rd);
    }

    // ------------------------------------------------------- E50 / E51
    #[test]
    fn err_mgf1_outlen_zero() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE50);
        let inp = rng.vec(32);
        let pairs: Vec<(&str, FnMgf1, FnMgf1)> = if IS_BLAKE {
            vec![("SPX_blake256_mgf1", l.c.blake256_mgf1(), l.r.blake256_mgf1()),
                 ("SPX_blake512_mgf1", l.c.blake512_mgf1(), l.r.blake512_mgf1())]
        } else if IS_SHA2 {
            vec![("SPX_mgf1_256", l.c.mgf1_256(), l.r.mgf1_256()),
                 ("SPX_mgf1_512", l.c.mgf1_512(), l.r.mgf1_512())]
        } else {
            vec![]
        };
        for (name, cf, rf) in pairs {
            let mut co = vec![0xAAu8; 64];
            let mut ro = vec![0xAAu8; 64];
            unsafe {
                cf(co.as_mut_ptr(), 0, inp.as_ptr(), 32);
                rf(ro.as_mut_ptr(), 0, inp.as_ptr(), 32);
            }
            eqb(name, "outlen=0", &co, &ro);
            assert!(co.iter().all(|&x| x == 0xAA), "{name}: outlen=0 must write nothing");
        }
    }

    #[test]
    fn err_mgf1_outlen_boundaries() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE51);
        let inp = rng.vec(40);
        let sets: Vec<(&str, FnMgf1, FnMgf1, usize)> = if IS_BLAKE {
            vec![("SPX_blake256_mgf1", l.c.blake256_mgf1(), l.r.blake256_mgf1(), 32),
                 ("SPX_blake512_mgf1", l.c.blake512_mgf1(), l.r.blake512_mgf1(), 64)]
        } else if IS_SHA2 {
            vec![("SPX_mgf1_256", l.c.mgf1_256(), l.r.mgf1_256(), 32),
                 ("SPX_mgf1_512", l.c.mgf1_512(), l.r.mgf1_512(), 64)]
        } else {
            vec![]
        };
        for (name, cf, rf, blk) in sets {
            for &outlen in &[blk - 1, blk, blk + 1, 2 * blk - 1, 2 * blk, 2 * blk + 1] {
                let mut co = vec![0xAAu8; outlen + 16];
                let mut ro = vec![0xAAu8; outlen + 16];
                unsafe {
                    cf(co.as_mut_ptr(), outlen as u64, inp.as_ptr(), 40);
                    rf(ro.as_mut_ptr(), outlen as u64, inp.as_ptr(), 40);
                }
                eqb(name, &format!("outlen={outlen}"), &co, &ro);
                assert!(co[outlen..].iter().all(|&x| x == 0xAA), "{name}: wrote past outlen");
            }
        }
    }

    // ------------------------------------------------------- E52
    #[test]
    fn err_squeeze_outlen_zero() {
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE52);
        if IS_SHAKE {
            let run = |imp: &Impl| -> (Vec<u8>, Vec<u64>) {
                let mut s = [0u64; 26];
                unsafe { (imp.shake256_inc_init())(s.as_mut_ptr()) };
                let m = [1u8, 2, 3];
                unsafe { (imp.shake256_inc_absorb())(s.as_mut_ptr(), m.as_ptr(), 3) };
                unsafe { (imp.shake256_inc_finalize())(s.as_mut_ptr()) };
                let mut o = vec![0xAAu8; 32];
                unsafe { (imp.shake256_inc_squeeze())(o.as_mut_ptr(), 0, s.as_mut_ptr()) };
                (o, s.to_vec())
            };
            let (co, cs) = run(&l.c);
            let (ro, rs) = run(&l.r);
            eqb("shake256_inc_squeeze", "outlen=0", &co, &ro);
            eq("shake inc state", "outlen=0", cs, rs);
            assert!(co.iter().all(|&x| x == 0xAA));
            // and the one-shot with outlen=0
            let inp = rng.vec(16);
            let mut co = vec![0xAAu8; 32];
            let mut ro = vec![0xAAu8; 32];
            unsafe {
                (l.c.shake256())(co.as_mut_ptr(), 0, inp.as_ptr(), 16);
                (l.r.shake256())(ro.as_mut_ptr(), 0, inp.as_ptr(), 16);
            }
            eqb("shake256", "outlen=0", &co, &ro);
        }
        if IS_HARAKA {
            let (cc, rc) = make_ctx(&rng.vec(SPX_N), &rng.vec(SPX_N));
            let run = |imp: &Impl, ctx: *const u8| -> (Vec<u8>, Vec<u8>) {
                let mut s = [0u8; 65];
                unsafe { (imp.haraka_S_inc_init())(s.as_mut_ptr()) };
                let m = [1u8, 2, 3];
                unsafe { (imp.haraka_S_inc_absorb())(s.as_mut_ptr(), m.as_ptr(), 3, ctx) };
                unsafe { (imp.haraka_S_inc_finalize())(s.as_mut_ptr()) };
                let mut o = vec![0xAAu8; 32];
                unsafe { (imp.haraka_S_inc_squeeze())(o.as_mut_ptr(), 0, s.as_mut_ptr(), ctx) };
                (o, s.to_vec())
            };
            let (co, cs) = run(&l.c, cc.as_ptr());
            let (ro, rs) = run(&l.r, rc.as_ptr());
            eqb("SPX_haraka_S_inc_squeeze", "outlen=0", &co, &ro);
            eqb("haraka inc state", "outlen=0", &cs, &rs);
            let inp = rng.vec(16);
            let mut co = vec![0xAAu8; 32];
            let mut ro = vec![0xAAu8; 32];
            unsafe {
                (l.c.haraka_S())(co.as_mut_ptr(), 0, inp.as_ptr(), 16, cc.as_ptr());
                (l.r.haraka_S())(ro.as_mut_ptr(), 0, inp.as_ptr(), 16, rc.as_ptr());
            }
            eqb("SPX_haraka_S", "outlen=0", &co, &ro);
        }
    }

    // ------------------------------------------------------- E55
    #[test]
    fn err_open_null_out_smlen_zero() {
        let _g = serial();
        let l = libs();
        let f = fixture();
        // C: `if (smlen < SPX_BYTES) { memset(m, 0, smlen); *mlen = 0; return -1; }`
        // With smlen == 0 the memset writes nothing, so a NULL `m` is never
        // dereferenced and the call must simply return -1.
        let mut cml = 0xDEADu64;
        let mut rml = 0xDEADu64;
        let cr = unsafe {
            (l.c.crypto_sign_open())(std::ptr::null_mut(), &mut cml, f.sig.as_ptr(), 0, f.pk.as_ptr())
        };
        let rr = unsafe {
            (l.r.crypto_sign_open())(std::ptr::null_mut(), &mut rml, f.sig.as_ptr(), 0, f.pk.as_ptr())
        };
        eq("crypto_sign_open rc", "m=NULL smlen=0", cr, rr);
        eq("crypto_sign_open rc", "m=NULL smlen=0", cr, -1);
        eq("crypto_sign_open *mlen", "m=NULL smlen=0", cml, rml);
        eq("crypto_sign_open *mlen", "m=NULL smlen=0", cml, 0);
    }

    // ------------------------------------------------------- E56
    #[test]
    fn err_null_out_zero_len() {
        let _g = serial();
        let l = libs();
        // `SPX_ull_to_bytes(NULL, 0, x)` — the loop starts at -1, so nothing is
        // written and the NULL is never dereferenced.
        unsafe {
            (l.c.ull_to_bytes())(std::ptr::null_mut(), 0, 0x0123_4567_89AB_CDEF);
            (l.r.ull_to_bytes())(std::ptr::null_mut(), 0, 0x0123_4567_89AB_CDEF);
        }
        // `SPX_bytes_to_ull(NULL, 0)` — already covered, repeated here for the row.
        let c = unsafe { (l.c.bytes_to_ull())(std::ptr::null(), 0) };
        let r = unsafe { (l.r.bytes_to_ull())(std::ptr::null(), 0) };
        eq("SPX_bytes_to_ull", "in=NULL inlen=0", c, r);
        // mgf1 with outlen == 0 and a NULL output (nothing is written).
        if IS_BLAKE {
            let inp = [0u8; 16];
            unsafe {
                (l.c.blake256_mgf1())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
                (l.r.blake256_mgf1())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
            }
        } else if IS_SHA2 {
            let inp = [0u8; 16];
            unsafe {
                (l.c.mgf1_256())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
                (l.r.mgf1_256())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
            }
        } else if IS_SHAKE {
            let inp = [0u8; 16];
            unsafe {
                (l.c.shake256())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
                (l.r.shake256())(std::ptr::null_mut(), 0, inp.as_ptr(), 16);
            }
        }
    }

    // ------------------------------------------------------- E53
    #[test]
    fn err_sha_inc_finalize_padding() {
        if !IS_SHA2 { return; }
        let _g = serial();
        let l = libs();
        let mut rng = Rng::new(TEST_SEED ^ 0xE53);
        for &inlen in &[0usize, 1, 54, 55, 56, 57, 63, 64, 110, 111, 112, 113, 119, 120, 127, 128] {
            let m = rng.vec(inlen);
            // sha256
            let run = |sz: usize, init: FnShaIncInit, fin: FnShaIncFinalize, olen: usize|
                -> (Vec<u8>, Vec<u8>) {
                let mut st = vec![0xAAu8; sz];
                unsafe { init(st.as_mut_ptr()) };
                let mut o = vec![0xAAu8; olen + 8];
                unsafe { fin(o.as_mut_ptr(), st.as_mut_ptr(), m.as_ptr(), inlen) };
                (o, st)
            };
            let (co, cs) = run(40, l.c.sha256_inc_init(), l.c.sha256_inc_finalize(), 32);
            let (ro, rs) = run(40, l.r.sha256_inc_init(), l.r.sha256_inc_finalize(), 32);
            eqb("sha256_inc_finalize", &format!("inlen={inlen}"), &co, &ro);
            eqb("sha256 state", &format!("inlen={inlen}"), &cs, &rs);
            let (co, cs) = run(72, l.c.sha512_inc_init(), l.c.sha512_inc_finalize(), 64);
            let (ro, rs) = run(72, l.r.sha512_inc_init(), l.r.sha512_inc_finalize(), 64);
            eqb("sha512_inc_finalize", &format!("inlen={inlen}"), &co, &ro);
            eqb("sha512 state", &format!("inlen={inlen}"), &cs, &rs);
        }
    }
}
