//! Phase B — valid-path differential tests for the backend-independent surface.
//!
//! One test per row of `CONFIGS.md` (rows C01–C44).  Every call goes through
//! `dlsym` on the two shared objects; nothing is called directly.
//!
//! `randombytes()` is global state in `rng.c`, so this suite must run
//! single-threaded (`RUST_TEST_THREADS=1`, set by `run_tests_all.sh`).

mod common;

use common::*;
use std::ffi::{c_int, c_uint};

// ---------------------------------------------------------------------------
// C01 — the four size accessors
// ---------------------------------------------------------------------------

#[test]
fn cfg_c01_size_accessors() {
    type F = unsafe extern "C" fn() -> u64;
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let (c, r) = libs().pair::<F>(name);
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        same_val(name, cv, rv);
        // Sanity: the harness's independent derivation agrees with the C.
        same_val(&format!("{name} (expected)"), cv, expect);
    }
}

// ---------------------------------------------------------------------------
// C02/C03 — ull_to_bytes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c02_c03_ull_to_bytes() {
    type F = unsafe extern "C" fn(*mut u8, c_uint, u64);
    let (c, r) = libs().pair::<F>("SPX_ull_to_bytes");
    let mut rng = Rng::new(SEED);

    let mut vals: Vec<u64> = vec![0, 1, u64::MAX, 0x0102_0304_0506_0708];
    for _ in 0..iters_cheap() {
        vals.push(rng.next_u64());
    }

    for outlen in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16] {
        for &v in &vals {
            let mut cb = vec![0xAAu8; 32];
            let mut rb = vec![0xAAu8; 32];
            unsafe {
                c(cb.as_mut_ptr(), outlen as c_uint, v);
                r(rb.as_mut_ptr(), outlen as c_uint, v);
            }
            same(&format!("ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
        }
    }
}

// ---------------------------------------------------------------------------
// C04 — u32_to_bytes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c04_u32_to_bytes() {
    type F = unsafe extern "C" fn(*mut u8, u32);
    let (c, r) = libs().pair::<F>("SPX_u32_to_bytes");
    let mut rng = Rng::new(SEED ^ 4);

    let mut vals: Vec<u32> = vec![0, 1, u32::MAX, 0x0102_0304];
    for _ in 0..iters_cheap() {
        vals.push(rng.next_u32());
    }
    for v in vals {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
        }
        same(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

// ---------------------------------------------------------------------------
// C05 — bytes_to_ull
// ---------------------------------------------------------------------------

#[test]
fn cfg_c05_bytes_to_ull() {
    type F = unsafe extern "C" fn(*const u8, c_uint) -> u64;
    let (c, r) = libs().pair::<F>("SPX_bytes_to_ull");
    let mut rng = Rng::new(SEED ^ 5);

    for inlen in 0usize..=8 {
        for _ in 0..iters_cheap() {
            let buf = rng.bytes(16);
            let cv = unsafe { c(buf.as_ptr(), inlen as c_uint) };
            let rv = unsafe { r(buf.as_ptr(), inlen as c_uint) };
            same_val(&format!("bytes_to_ull(inlen={inlen})"), cv, rv);
        }
        // extremes
        for fill in [0x00u8, 0xff] {
            let buf = vec![fill; 16];
            let cv = unsafe { c(buf.as_ptr(), inlen as c_uint) };
            let rv = unsafe { r(buf.as_ptr(), inlen as c_uint) };
            same_val(&format!("bytes_to_ull(inlen={inlen}, fill={fill:#x})"), cv, rv);
        }
    }
}

// ---------------------------------------------------------------------------
// C06/C07/C08/C09 — address setters and copiers
// ---------------------------------------------------------------------------

#[test]
fn cfg_c06_byte_field_setters() {
    type F = unsafe extern "C" fn(*mut u32, u32);
    let mut rng = Rng::new(SEED ^ 6);
    for name in [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ] {
        let (c, r) = libs().pair::<F>(name);
        let mut vals: Vec<u32> = vec![0, 1, 6, 7, 0xFF, 0x100, 0x1FF, u32::MAX];
        for _ in 0..iters_cheap() {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let base = rand_addr(&mut rng);
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr() as *mut u32, v);
                r(ra.as_mut_ptr() as *mut u32, v);
            }
            same(&format!("{name}({v:#x})"), &ca, &ra);
        }
    }
}

#[test]
fn cfg_c07_set_tree_addr() {
    type F = unsafe extern "C" fn(*mut u32, u64);
    let (c, r) = libs().pair::<F>("SPX_set_tree_addr");
    let mut rng = Rng::new(SEED ^ 7);
    let mut vals: Vec<u64> = vec![0, 1, 1 << 32, u64::MAX, 0x0102_0304_0506_0708];
    for _ in 0..iters_cheap() {
        vals.push(rng.next_u64());
    }
    for v in vals {
        let base = rand_addr(&mut rng);
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr() as *mut u32, v);
            r(ra.as_mut_ptr() as *mut u32, v);
        }
        same(&format!("set_tree_addr({v:#x})"), &ca, &ra);
    }
}

#[test]
fn cfg_c08_u32_field_setters() {
    type F = unsafe extern "C" fn(*mut u32, u32);
    let mut rng = Rng::new(SEED ^ 8);
    for name in ["SPX_set_keypair_addr", "SPX_set_tree_index"] {
        let (c, r) = libs().pair::<F>(name);
        let mut vals: Vec<u32> = vec![0, 1, u32::MAX, 0x0102_0304];
        for _ in 0..iters_cheap() {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let base = rand_addr(&mut rng);
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr() as *mut u32, v);
                r(ra.as_mut_ptr() as *mut u32, v);
            }
            same(&format!("{name}({v:#x})"), &ca, &ra);
        }
    }
}

#[test]
fn cfg_c09_address_copiers() {
    type F = unsafe extern "C" fn(*mut u32, *const u32);
    let mut rng = Rng::new(SEED ^ 9);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let (c, r) = libs().pair::<F>(name);
        for _ in 0..iters_cheap() {
            let src = rand_addr(&mut rng);
            let dst = rand_addr(&mut rng);
            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                c(ca.as_mut_ptr() as *mut u32, src.as_ptr() as *const u32);
                r(ra.as_mut_ptr() as *mut u32, src.as_ptr() as *const u32);
            }
            same(name, &ca, &ra);
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — initialize_hash_function (compares the whole spx_ctx)
// ---------------------------------------------------------------------------

fn init_ctx_pair(rng: &mut Rng) -> (Ctx, Ctx) {
    type F = unsafe extern "C" fn(*mut u8);
    let (c, r) = libs().pair::<F>("SPX_initialize_hash_function");
    let pub_seed = rng.bytes(SPX_N);
    let sk_seed = rng.bytes(SPX_N);
    let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
    let mut rc = Ctx::with_seeds(&pub_seed, &sk_seed);
    unsafe {
        c(cc.as_mut_ptr());
        r(rc.as_mut_ptr());
    }
    (cc, rc)
}

#[test]
fn cfg_c10_initialize_hash_function() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..iters_cheap() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        same("initialize_hash_function -> spx_ctx", &cc.0, &rc.0);
    }
}

// ---------------------------------------------------------------------------
// C11 — prf_addr
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_prf_addr() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
    let (c, r) = libs().pair::<F>("SPX_prf_addr");
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..iters_cheap() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let addr = rand_addr(&mut rng);
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr() as *const u32);
            r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr() as *const u32);
        }
        same("prf_addr", &co, &ro);
    }
}

// ---------------------------------------------------------------------------
// C12–C16 — thash at every inblocks value the library uses
// ---------------------------------------------------------------------------

fn thash_row(inblocks: usize, label: &str, seed: u64) {
    type F = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
    let (c, r) = libs().pair::<F>("SPX_thash");
    let mut rng = Rng::new(seed);
    for _ in 0..iters_cheap() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let inp = rng.bytes((inblocks * SPX_N).max(1));
        let addr = rand_addr(&mut rng);
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(
                co.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as c_uint,
                cc.as_ptr(),
                ca.as_mut_ptr() as *mut u32,
            );
            r(
                ro.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as c_uint,
                rc.as_ptr(),
                ra.as_mut_ptr() as *mut u32,
            );
        }
        same(&format!("thash({label}) out"), &co, &ro);
        same(&format!("thash({label}) addr"), &ca, &ra);
    }
}

#[test]
fn cfg_c12_thash_1() {
    thash_row(1, "inblocks=1", SEED ^ 12);
}

#[test]
fn cfg_c13_thash_2() {
    thash_row(2, "inblocks=2", SEED ^ 13);
}

#[test]
fn cfg_c14_thash_wots_len() {
    thash_row(SPX_WOTS_LEN, "inblocks=SPX_WOTS_LEN", SEED ^ 14);
}

#[test]
fn cfg_c15_thash_fors_trees() {
    thash_row(SPX_FORS_TREES, "inblocks=SPX_FORS_TREES", SEED ^ 15);
}

#[test]
fn cfg_c16_thash_0() {
    thash_row(0, "inblocks=0", SEED ^ 16);
}

// ---------------------------------------------------------------------------
// C17 — gen_message_random
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_gen_message_random() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
    let (c, r) = libs().pair::<F>("SPX_gen_message_random");
    let mut rng = Rng::new(SEED ^ 17);
    let mut mlens: Vec<usize> = vec![
        0,
        1,
        SPX_N.saturating_sub(1),
        SPX_N,
        55,
        63,
        64,
        65,
        127,
        128,
        129,
        1000,
    ];
    // sha2 branches exactly at SPX_SHAX_BLOCK_BYTES - SPX_N
    let block = if WIDE { 128usize } else { 64 };
    for d in [-1i64, 0, 1] {
        let v = block as i64 - SPX_N as i64 + d;
        if v >= 0 {
            mlens.push(v as usize);
        }
    }
    mlens.sort_unstable();
    mlens.dedup();

    for mlen in mlens {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let sk_prf = rng.bytes(SPX_N);
        let optrand = rng.bytes(SPX_N);
        let m = rng.bytes(mlen.max(1));
        // blake writes its full digest into R, so allow for that.
        let out = GEN_MSG_RANDOM_OUT + 8;
        let mut co = vec![0xAAu8; out];
        let mut ro = vec![0xAAu8; out];
        unsafe {
            c(
                co.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                cc.as_ptr(),
            );
            r(
                ro.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                rc.as_ptr(),
            );
        }
        same(&format!("gen_message_random(mlen={mlen})"), &co, &ro);
    }
}

// ---------------------------------------------------------------------------
// C18 — hash_message
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_hash_message() {
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *mut u32,
        *const u8,
        *const u8,
        *const u8,
        u64,
        *const u8,
    );
    let (c, r) = libs().pair::<F>("SPX_hash_message");
    let mut rng = Rng::new(SEED ^ 18);

    let mut mlens: Vec<usize> = vec![
        0, 1, 16, 31, 32, 33, 63, 64, 65, 95, 96, 127, 128, 129, 1000,
    ];
    // sha2 branches at SPX_INBLOCKS*BLOCK - SPX_N - SPX_PK_BYTES
    let block = if WIDE { 128usize } else { 64 };
    let inblocks = (SPX_N + SPX_PK_BYTES + block - 1) / block;
    for d in [-1i64, 0, 1] {
        let v = (inblocks * block) as i64 - SPX_N as i64 - SPX_PK_BYTES as i64 + d;
        if v >= 0 {
            mlens.push(v as usize);
        }
    }
    mlens.sort_unstable();
    mlens.dedup();

    for mlen in mlens {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let rr = rng.bytes(SPX_N);
        let pk = rng.bytes(SPX_PK_BYTES);
        let m = rng.bytes(mlen.max(1));
        let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
        let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
        let mut ct = 0u64;
        let mut rt = 0u64;
        let mut cl = 0u32;
        let mut rl = 0u32;
        unsafe {
            c(
                cd.as_mut_ptr(),
                &mut ct,
                &mut cl,
                rr.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                cc.as_ptr(),
            );
            r(
                rd.as_mut_ptr(),
                &mut rt,
                &mut rl,
                rr.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                rc.as_ptr(),
            );
        }
        same(&format!("hash_message(mlen={mlen}) digest"), &cd, &rd);
        same_val(&format!("hash_message(mlen={mlen}) tree"), ct, rt);
        same_val(&format!("hash_message(mlen={mlen}) leaf_idx"), cl, rl);
    }
}

// ---------------------------------------------------------------------------
// C19 — chain_lengths
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_chain_lengths() {
    type F = unsafe extern "C" fn(*mut c_uint, *const u8);
    let (c, r) = libs().pair::<F>("SPX_chain_lengths");
    let mut rng = Rng::new(SEED ^ 19);

    let mut msgs: Vec<Vec<u8>> = vec![vec![0x00; SPX_N], vec![0xFF; SPX_N]];
    for _ in 0..iters_cheap() {
        msgs.push(rng.bytes(SPX_N));
    }
    for m in msgs {
        let mut cl = vec![0xAAAA_AAAAu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xAAAA_AAAAu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr() as *mut c_uint, m.as_ptr());
            r(rl.as_mut_ptr() as *mut c_uint, m.as_ptr());
        }
        same_val(&format!("chain_lengths({})", hex(&m)), cl, rl);
    }
}

// ---------------------------------------------------------------------------
// C20 — wots_pk_from_sig
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_wots_pk_from_sig() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
    let (c, r) = libs().pair::<F>("SPX_wots_pk_from_sig");
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..iters_mid() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let msg = rng.bytes(SPX_N);
        let addr = rand_addr(&mut rng);
        let mut ca = addr;
        let mut ra = addr;
        let mut cp = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut rp = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        unsafe {
            c(
                cp.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cc.as_ptr(),
                ca.as_mut_ptr() as *mut u32,
            );
            r(
                rp.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                rc.as_ptr(),
                ra.as_mut_ptr() as *mut u32,
            );
        }
        same("wots_pk_from_sig pk", &cp, &rp);
        same("wots_pk_from_sig addr", &ca, &ra);
    }
    // Extremes of the chain-length distribution.
    for fill in [0x00u8, 0xFFu8] {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let msg = vec![fill; SPX_N];
        let addr = rand_addr(&mut rng);
        let mut ca = addr;
        let mut ra = addr;
        let mut cp = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut rp = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        unsafe {
            c(
                cp.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cc.as_ptr(),
                ca.as_mut_ptr() as *mut u32,
            );
            r(
                rp.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                rc.as_ptr(),
                ra.as_mut_ptr() as *mut u32,
            );
        }
        same(&format!("wots_pk_from_sig msg=all {fill:#x}"), &cp, &rp);
        same("wots_pk_from_sig addr (extreme)", &ca, &ra);
    }
}

// ---------------------------------------------------------------------------
// C21/C22 — wots_gen_leafx1, both wots_k_mask arms
// ---------------------------------------------------------------------------

fn wots_gen_leafx1_row(signing: bool, seed: u64) {
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);
    let (c, r) = libs().pair::<F>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(seed);
    for _ in 0..iters_mid() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let leaf_idx = rng.next_u32() & 0xFFFF;
        let addr = rand_addr(&mut rng);

        let mut steps: Vec<u32> = (0..SPX_WOTS_LEN).map(|_| rng.below(16)).collect();
        // Also hit both ends of the chain explicitly.
        steps[0] = 0;
        steps[SPX_WOTS_LEN - 1] = (SPX_WOTS_W - 1) as u32;

        let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
        let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
        let mut csteps = steps.clone();
        let mut rsteps = steps.clone();

        let mut ci = LeafInfoX1::zeroed();
        let mut ri = LeafInfoX1::zeroed();
        for i in 0..8 {
            let w = u32::from_le_bytes(addr[4 * i..4 * i + 4].try_into().unwrap());
            ci.leaf_addr[i] = w;
            ci.pk_addr[i] = w;
            ri.leaf_addr[i] = w;
            ri.pk_addr[i] = w;
        }
        ci.wots_sig = csig.as_mut_ptr();
        ri.wots_sig = rsig.as_mut_ptr();
        ci.wots_steps = csteps.as_mut_ptr();
        ri.wots_steps = rsteps.as_mut_ptr();
        ci.wots_sign_leaf = if signing { leaf_idx } else { !0u32 };
        ri.wots_sign_leaf = ci.wots_sign_leaf;

        let mut cd = vec![0xAAu8; SPX_N + 8];
        let mut rd = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut ci);
            r(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut ri);
        }
        let tag = if signing { "signing" } else { "pk-only" };
        same(&format!("wots_gen_leafx1({tag}) dest"), &cd, &rd);
        same(&format!("wots_gen_leafx1({tag}) wots_sig"), &csig, &rsig);
        same_val(
            &format!("wots_gen_leafx1({tag}) leaf_addr"),
            ci.leaf_addr,
            ri.leaf_addr,
        );
        same_val(
            &format!("wots_gen_leafx1({tag}) pk_addr"),
            ci.pk_addr,
            ri.pk_addr,
        );
    }
}

#[test]
fn cfg_c21_wots_gen_leafx1_pk_only() {
    wots_gen_leafx1_row(false, SEED ^ 21);
}

#[test]
fn cfg_c22_wots_gen_leafx1_signing() {
    wots_gen_leafx1_row(true, SEED ^ 22);
}

// ---------------------------------------------------------------------------
// C23 — fors_gen_leafx1
// ---------------------------------------------------------------------------

#[test]
fn cfg_c23_fors_gen_leafx1() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);
    let (c, r) = libs().pair::<F>("SPX_fors_gen_leafx1");
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..iters_cheap() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let addr = rand_addr(&mut rng);
        let mut words = [0u32; 8];
        for i in 0..8 {
            words[i] = u32::from_le_bytes(addr[4 * i..4 * i + 4].try_into().unwrap());
        }
        let addr_idx = rng.next_u32();
        let mut ci = ForsGenLeafInfo { leaf_addrx: words };
        let mut ri = ForsGenLeafInfo { leaf_addrx: words };
        let mut cd = vec![0xAAu8; SPX_N + 8];
        let mut rd = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(cd.as_mut_ptr(), cc.as_ptr(), addr_idx, &mut ci);
            r(rd.as_mut_ptr(), rc.as_ptr(), addr_idx, &mut ri);
        }
        same("fors_gen_leafx1 leaf", &cd, &rd);
        same_val("fors_gen_leafx1 info", ci, ri);
    }
}

// ---------------------------------------------------------------------------
// C24/C25/C26 — compute_root
// ---------------------------------------------------------------------------

fn compute_root_row(tree_height: u32, offsets: &[u32], label: &str, seed: u64) {
    type F = unsafe extern "C" fn(
        *mut u8,
        *const u8,
        u32,
        u32,
        *const u8,
        u32,
        *const u8,
        *mut u32,
    );
    let (c, r) = libs().pair::<F>("SPX_compute_root");
    let mut rng = Rng::new(seed);
    for &idx_offset in offsets {
        for k in 0..iters_cheap() {
            let (cc, rc) = init_ctx_pair(&mut rng);
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes(tree_height as usize * SPX_N);
            let addr = rand_addr(&mut rng);
            // Cover both parities deterministically as well as randomly.
            let leaf_idx = if k < 2 {
                k as u32
            } else if tree_height >= 32 {
                rng.next_u32()
            } else {
                rng.next_u32() % (1u32 << tree_height)
            };
            let mut ca = addr;
            let mut ra = addr;
            let mut cr = vec![0xAAu8; SPX_N + 8];
            let mut rr = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(
                    cr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    tree_height,
                    cc.as_ptr(),
                    ca.as_mut_ptr() as *mut u32,
                );
                r(
                    rr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    tree_height,
                    rc.as_ptr(),
                    ra.as_mut_ptr() as *mut u32,
                );
            }
            same(&format!("compute_root({label}, leaf_idx={leaf_idx}, off={idx_offset}) root"), &cr, &rr);
            same(&format!("compute_root({label}) addr"), &ca, &ra);
        }
    }
}

#[test]
fn cfg_c24_compute_root_h1() {
    compute_root_row(1, &[0], "h=1", SEED ^ 24);
}

#[test]
fn cfg_c25_compute_root_fors() {
    let h = SPX_FORS_HEIGHT as u32;
    compute_root_row(h, &[0, 1 << h, 3 * (1 << h)], "h=FORS_HEIGHT", SEED ^ 25);
}

#[test]
fn cfg_c26_compute_root_tree() {
    compute_root_row(SPX_TREE_HEIGHT as u32, &[0], "h=TREE_HEIGHT", SEED ^ 26);
}

// ---------------------------------------------------------------------------
// C27/C28 — treehash (function-pointer variant)
// ---------------------------------------------------------------------------

/// Deterministic stand-in for a `gen_leaf` callback.  Depends on both
/// `addr_idx` and the current `tree_addr`, so any divergence in how `treehash`
/// drives the callback shows up in the output.
unsafe extern "C" fn test_gen_leaf(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, tree_addr: *const u32) {
    let out = std::slice::from_raw_parts_mut(leaf, SPX_N);
    let a = std::slice::from_raw_parts(tree_addr as *const u8, 32);
    for (i, b) in out.iter_mut().enumerate() {
        *b = (addr_idx as u8)
            .wrapping_mul(31)
            .wrapping_add(a[i % 32])
            .wrapping_add((i as u8).wrapping_mul(7))
            ^ ((addr_idx >> 8) as u8);
    }
}

fn treehash_row(tree_height: u32, leaf_idx: u32, idx_offset: u32, seed: u64) {
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        u32,
        u32,
        u32,
        unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32),
        *mut u32,
    );
    let (c, r) = libs().pair::<F>("SPX_treehash");
    let mut rng = Rng::new(seed);
    for _ in 0..iters_mid() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let addr = rand_addr(&mut rng);
        let mut ca = addr;
        let mut ra = addr;
        let mut croot = vec![0xAAu8; SPX_N + 8];
        let mut rroot = vec![0xAAu8; SPX_N + 8];
        let mut cauth = vec![0xAAu8; tree_height as usize * SPX_N + 8];
        let mut rauth = vec![0xAAu8; tree_height as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                tree_height,
                test_gen_leaf,
                ca.as_mut_ptr() as *mut u32,
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                rc.as_ptr(),
                leaf_idx,
                idx_offset,
                tree_height,
                test_gen_leaf,
                ra.as_mut_ptr() as *mut u32,
            );
        }
        let tag = format!("treehash(h={tree_height}, leaf={leaf_idx}, off={idx_offset})");
        same(&format!("{tag} root"), &croot, &rroot);
        same(&format!("{tag} auth"), &cauth, &rauth);
        same(&format!("{tag} addr"), &ca, &ra);
    }
}

#[test]
fn cfg_c27_treehash_in_range() {
    for h in 1u32..=4 {
        for leaf in 0..(1u32 << h) {
            treehash_row(h, leaf, 0, SEED ^ 27 ^ (h as u64) ^ ((leaf as u64) << 8));
        }
    }
}

#[test]
fn cfg_c28_treehash_no_auth_path() {
    for h in 1u32..=4 {
        treehash_row(h, !0u32, 8 * (1 << h), SEED ^ 28 ^ (h as u64));
    }
}

// ---------------------------------------------------------------------------
// C29 — fors_treehashx1
// ---------------------------------------------------------------------------

#[test]
fn cfg_c29_fors_treehashx1() {
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        u32,
        u32,
        u32,
        *mut u32,
        *mut ForsGenLeafInfo,
    );
    let (c, r) = libs().pair::<F>("SPX_fors_treehashx1");
    let h = SPX_FORS_HEIGHT as u32;
    let mut rng = Rng::new(SEED ^ 29);
    for t in 0..iters_mid() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let addr = rand_addr(&mut rng);
        let mut words = [0u32; 8];
        for i in 0..8 {
            words[i] = u32::from_le_bytes(addr[4 * i..4 * i + 4].try_into().unwrap());
        }
        let idx_offset = (t as u32) * (1u32 << h);
        let leaf_idx = rng.next_u32() % (1u32 << h);
        let mut ci = ForsGenLeafInfo { leaf_addrx: words };
        let mut ri = ForsGenLeafInfo { leaf_addrx: words };
        let mut ca = addr;
        let mut ra = addr;
        let mut croot = vec![0xAAu8; SPX_N + 8];
        let mut rroot = vec![0xAAu8; SPX_N + 8];
        let mut cauth = vec![0xAAu8; h as usize * SPX_N + 8];
        let mut rauth = vec![0xAAu8; h as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ca.as_mut_ptr() as *mut u32,
                &mut ci,
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                rc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ra.as_mut_ptr() as *mut u32,
                &mut ri,
            );
        }
        same("fors_treehashx1 root", &croot, &rroot);
        same("fors_treehashx1 auth", &cauth, &rauth);
        same("fors_treehashx1 tree_addr", &ca, &ra);
        same_val("fors_treehashx1 info", ci, ri);
    }
}

// ---------------------------------------------------------------------------
// C30/C31 — wots_treehashx1
// ---------------------------------------------------------------------------

fn wots_treehashx1_row(tree_height: u32, sign_leaf: Option<u32>, seed: u64) {
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        u32,
        u32,
        u32,
        *mut u32,
        *mut LeafInfoX1,
    );
    let (c, r) = libs().pair::<F>("SPX_wots_treehashx1");
    let mut rng = Rng::new(seed);
    let (cc, rc) = init_ctx_pair(&mut rng);
    let addr = rand_addr(&mut rng);
    let mut words = [0u32; 8];
    for i in 0..8 {
        words[i] = u32::from_le_bytes(addr[4 * i..4 * i + 4].try_into().unwrap());
    }
    let leaf_idx = sign_leaf.unwrap_or(!0u32);
    let mut steps: Vec<u32> = (0..SPX_WOTS_LEN).map(|_| rng.below(16)).collect();
    steps[0] = 0;
    steps[SPX_WOTS_LEN - 1] = (SPX_WOTS_W - 1) as u32;

    let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut csteps = steps.clone();
    let mut rsteps = steps.clone();
    let mut ci = LeafInfoX1::zeroed();
    let mut ri = LeafInfoX1::zeroed();
    ci.leaf_addr = words;
    ci.pk_addr = words;
    ri.leaf_addr = words;
    ri.pk_addr = words;
    ci.wots_sig = csig.as_mut_ptr();
    ri.wots_sig = rsig.as_mut_ptr();
    ci.wots_steps = csteps.as_mut_ptr();
    ri.wots_steps = rsteps.as_mut_ptr();
    ci.wots_sign_leaf = leaf_idx;
    ri.wots_sign_leaf = leaf_idx;

    let mut ca = addr;
    let mut ra = addr;
    let mut croot = vec![0xAAu8; SPX_N + 8];
    let mut rroot = vec![0xAAu8; SPX_N + 8];
    let mut cauth = vec![0xAAu8; tree_height as usize * SPX_N + 8];
    let mut rauth = vec![0xAAu8; tree_height as usize * SPX_N + 8];
    unsafe {
        c(
            croot.as_mut_ptr(),
            cauth.as_mut_ptr(),
            cc.as_ptr(),
            leaf_idx,
            0,
            tree_height,
            ca.as_mut_ptr() as *mut u32,
            &mut ci,
        );
        r(
            rroot.as_mut_ptr(),
            rauth.as_mut_ptr(),
            rc.as_ptr(),
            leaf_idx,
            0,
            tree_height,
            ra.as_mut_ptr() as *mut u32,
            &mut ri,
        );
    }
    let tag = format!("wots_treehashx1(h={tree_height}, sign_leaf={leaf_idx})");
    same(&format!("{tag} root"), &croot, &rroot);
    same(&format!("{tag} auth"), &cauth, &rauth);
    same(&format!("{tag} tree_addr"), &ca, &ra);
    same(&format!("{tag} wots_sig"), &csig, &rsig);
    same_val(&format!("{tag} leaf_addr"), ci.leaf_addr, ri.leaf_addr);
    same_val(&format!("{tag} pk_addr"), ci.pk_addr, ri.pk_addr);
}

#[test]
fn cfg_c30_wots_treehashx1_signing() {
    for h in 1u32..=3 {
        for leaf in 0..(1u32 << h).min(4) {
            wots_treehashx1_row(h, Some(leaf), SEED ^ 30 ^ (h as u64) ^ ((leaf as u64) << 8));
        }
    }
}

#[test]
fn cfg_c31_wots_treehashx1_gen_root_mode() {
    for h in 1u32..=3 {
        wots_treehashx1_row(h, None, SEED ^ 31 ^ (h as u64));
    }
}

// ---------------------------------------------------------------------------
// C32 — merkle_sign
// ---------------------------------------------------------------------------

#[test]
fn cfg_c32_merkle_sign() {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
    let (c, r) = libs().pair::<F>("SPX_merkle_sign");
    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..iters_heavy() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let root = rng.bytes(SPX_N);
        let wots_addr = rand_addr(&mut rng);
        let tree_addr = rand_addr(&mut rng);
        let idx_leaf = rng.next_u32() % (1u32 << SPX_TREE_HEIGHT);

        let mut cwa = wots_addr;
        let mut rwa = wots_addr;
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        let mut cr = root.clone();
        let mut rr = root.clone();
        let mut cs = vec![0xAAu8; siglen + 8];
        let mut rs = vec![0xAAu8; siglen + 8];
        unsafe {
            c(
                cs.as_mut_ptr(),
                cr.as_mut_ptr(),
                cc.as_ptr(),
                cwa.as_mut_ptr() as *mut u32,
                cta.as_mut_ptr() as *mut u32,
                idx_leaf,
            );
            r(
                rs.as_mut_ptr(),
                rr.as_mut_ptr(),
                rc.as_ptr(),
                rwa.as_mut_ptr() as *mut u32,
                rta.as_mut_ptr() as *mut u32,
                idx_leaf,
            );
        }
        same("merkle_sign sig", &cs, &rs);
        same("merkle_sign root", &cr, &rr);
        same("merkle_sign wots_addr", &cwa, &rwa);
        same("merkle_sign tree_addr", &cta, &rta);
    }
}

// ---------------------------------------------------------------------------
// C33 — merkle_gen_root
// ---------------------------------------------------------------------------

#[test]
fn cfg_c33_merkle_gen_root() {
    type F = unsafe extern "C" fn(*mut u8, *const u8);
    let (c, r) = libs().pair::<F>("SPX_merkle_gen_root");
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..iters_heavy() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let mut cr = vec![0u8; SPX_N + 8];
        let mut rr = vec![0u8; SPX_N + 8];
        unsafe {
            c(cr.as_mut_ptr(), cc.as_ptr());
            r(rr.as_mut_ptr(), rc.as_ptr());
        }
        same("merkle_gen_root", &cr, &rr);
    }
}

// ---------------------------------------------------------------------------
// C34/C35 — fors_sign / fors_pk_from_sig
// ---------------------------------------------------------------------------

#[test]
fn cfg_c34_c35_fors() {
    type Sign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
    type FromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
    let (csign, rsign) = libs().pair::<Sign>("SPX_fors_sign");
    let (cfs, rfs) = libs().pair::<FromSig>("SPX_fors_pk_from_sig");
    let mut rng = Rng::new(SEED ^ 34);

    for _ in 0..iters_mid() {
        let (cc, rc) = init_ctx_pair(&mut rng);
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let addr = rand_addr(&mut rng);

        let mut csig = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut rsig_b = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut cpk = vec![0xAAu8; SPX_N + 8];
        let mut rpk = vec![0xAAu8; SPX_N + 8];
        unsafe {
            csign(
                csig.as_mut_ptr(),
                cpk.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
            rsign(
                rsig_b.as_mut_ptr(),
                rpk.as_mut_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
        }
        same("fors_sign sig", &csig, &rsig_b);
        same("fors_sign pk", &cpk, &rpk);

        // Round-trip: pk_from_sig on the produced signature.
        let mut cpk2 = vec![0xAAu8; SPX_N + 8];
        let mut rpk2 = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cfs(
                cpk2.as_mut_ptr(),
                csig.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
            rfs(
                rpk2.as_mut_ptr(),
                rsig_b.as_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
        }
        same("fors_pk_from_sig (round-trip)", &cpk2, &rpk2);
        same("fors round-trip equals fors_sign pk", &cpk, &cpk2);

        // Independent random signature (the failure-shaped input).
        let junk = rng.bytes(SPX_FORS_BYTES);
        let mut cpk3 = vec![0xAAu8; SPX_N + 8];
        let mut rpk3 = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cfs(
                cpk3.as_mut_ptr(),
                junk.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
            rfs(
                rpk3.as_mut_ptr(),
                junk.as_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                addr.as_ptr() as *const u32,
            );
        }
        same("fors_pk_from_sig (random sig)", &cpk3, &rpk3);
    }
}

// ---------------------------------------------------------------------------
// C36 — crypto_sign_seed_keypair
// ---------------------------------------------------------------------------

pub fn seed_keypair(seed: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
    let (c, r) = libs().pair::<F>("crypto_sign_seed_keypair");
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    let cv = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
    let rv = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
    same_val("crypto_sign_seed_keypair return", cv, rv);
    same("crypto_sign_seed_keypair pk", &cpk, &rpk);
    same("crypto_sign_seed_keypair sk", &csk, &rsk);
    (cpk, csk, rpk, rsk)
}

#[test]
fn cfg_c36_seed_keypair() {
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..iters_heavy() {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        seed_keypair(&seed);
    }
}

// ---------------------------------------------------------------------------
// C37 — crypto_sign_signature + crypto_sign_verify
// ---------------------------------------------------------------------------

#[test]
fn cfg_c37_signature_and_verify() {
    type Sig = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type Ver = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
    let (csig_f, rsig_f) = libs().pair::<Sig>("crypto_sign_signature");
    let (cver, rver) = libs().pair::<Ver>("crypto_sign_verify");
    let mut rng = Rng::new(SEED ^ 37);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, rsk) = seed_keypair(&seed);

    let mut mlens: Vec<usize> = vec![0, 1, 33];
    if iters_heavy() > 1 {
        mlens.push(1000);
    }

    for mlen in mlens {
        let m = rng.bytes(mlen.max(1));
        // Re-seed both DRBGs so the `optrand` drawn inside signing matches.
        let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
        seed_both_drbgs(&entropy, None);

        let mut cs = vec![0xAAu8; SPX_BYTES + 16];
        let mut rs = vec![0xAAu8; SPX_BYTES + 16];
        let mut cl = 0usize;
        let mut rl = 0usize;
        let cret = unsafe {
            csig_f(
                cs.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen,
                csk.as_ptr(),
            )
        };
        // The urandom provider makes signing nondeterministic; re-seed only
        // matters for the DRBG build.
        if !URANDOM {
            seed_both_drbgs(&entropy, None);
        }
        let rret = unsafe {
            rsig_f(
                rs.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen,
                rsk.as_ptr(),
            )
        };
        same_val(&format!("crypto_sign_signature(mlen={mlen}) return"), cret, rret);
        same_val(&format!("crypto_sign_signature(mlen={mlen}) siglen"), cl, rl);
        same_val("siglen == SPX_BYTES", cl, SPX_BYTES);
        if !URANDOM {
            same(&format!("crypto_sign_signature(mlen={mlen}) sig"), &cs, &rs);
        }

        // Cross verification: each library must accept the other's signature.
        for (label, sig, pk) in [
            ("C sig / C pk", &cs, &cpk),
            ("C sig / R pk", &cs, &rpk),
            ("R sig / C pk", &rs, &cpk),
            ("R sig / R pk", &rs, &rpk),
        ] {
            let cv = unsafe { cver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            let rv = unsafe { rver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            same_val(&format!("crypto_sign_verify({label}, mlen={mlen})"), cv, rv);
            same_val(&format!("crypto_sign_verify({label}) accepts"), cv, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// C38 — crypto_sign + crypto_sign_open
// ---------------------------------------------------------------------------

#[test]
fn cfg_c38_sign_and_open() {
    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    let (csign, rsign) = libs().pair::<Sign>("crypto_sign");
    let (copen, ropen) = libs().pair::<Open>("crypto_sign_open");
    let mut rng = Rng::new(SEED ^ 38);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, rsk) = seed_keypair(&seed);

    let mut mlens: Vec<u64> = vec![0, 1, 33];
    if iters_heavy() > 1 {
        mlens.push(1000);
    }

    for mlen in mlens {
        let m = rng.bytes(mlen.max(1) as usize);
        let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();

        seed_both_drbgs(&entropy, None);
        let mut csm = vec![0xAAu8; SPX_BYTES + mlen as usize + 16];
        let mut cslen = 0u64;
        let cret = unsafe {
            csign(
                csm.as_mut_ptr(),
                &mut cslen,
                m.as_ptr(),
                mlen,
                csk.as_ptr(),
            )
        };
        seed_both_drbgs(&entropy, None);
        let mut rsm = vec![0xAAu8; SPX_BYTES + mlen as usize + 16];
        let mut rslen = 0u64;
        let rret = unsafe {
            rsign(
                rsm.as_mut_ptr(),
                &mut rslen,
                m.as_ptr(),
                mlen,
                rsk.as_ptr(),
            )
        };
        same_val(&format!("crypto_sign(mlen={mlen}) return"), cret, rret);
        same_val(&format!("crypto_sign(mlen={mlen}) smlen"), cslen, rslen);
        same_val("smlen == SPX_BYTES + mlen", cslen, SPX_BYTES as u64 + mlen);
        if !URANDOM {
            same(&format!("crypto_sign(mlen={mlen}) sm"), &csm, &rsm);
        }

        for (label, sm, pk) in [
            ("C sm / C pk", &csm, &cpk),
            ("C sm / R pk", &csm, &rpk),
            ("R sm / C pk", &rsm, &cpk),
            ("R sm / R pk", &rsm, &rpk),
        ] {
            let mut cm = vec![0x55u8; SPX_BYTES + mlen as usize + 16];
            let mut rm = vec![0x55u8; SPX_BYTES + mlen as usize + 16];
            let mut cml = u64::MAX;
            let mut rml = u64::MAX;
            let cv = unsafe { copen(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), cslen, pk.as_ptr()) };
            let rv = unsafe { ropen(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), cslen, pk.as_ptr()) };
            same_val(&format!("crypto_sign_open({label}, mlen={mlen}) ret"), cv, rv);
            same_val(&format!("crypto_sign_open({label}, mlen={mlen}) mlen"), cml, rml);
            same(&format!("crypto_sign_open({label}, mlen={mlen}) m"), &cm, &rm);
            same_val(&format!("crypto_sign_open({label}) accepts"), cv, 0);
            same_val("recovered mlen", cml, mlen);
            same("recovered message", &cm[..mlen as usize], &m[..mlen as usize]);
        }
    }
}

// ---------------------------------------------------------------------------
// C39 — crypto_sign_keypair
// ---------------------------------------------------------------------------

#[test]
fn cfg_c39_keypair() {
    type KP = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
    type Sign = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type Ver = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
    let (ckp, rkp) = libs().pair::<KP>("crypto_sign_keypair");
    let (csig_f, rsig_f) = libs().pair::<Sign>("crypto_sign_signature");
    let (cver, rver) = libs().pair::<Ver>("crypto_sign_verify");
    let mut rng = Rng::new(SEED ^ 39);

    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);

    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    let cv = unsafe { ckp(cpk.as_mut_ptr(), csk.as_mut_ptr()) };
    if !URANDOM {
        seed_both_drbgs(&entropy, None);
    }
    let rv = unsafe { rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr()) };
    same_val("crypto_sign_keypair return", cv, rv);

    if URANDOM {
        // Nondeterministic provider: cross-check that each side's key works in
        // the other implementation.
        let m = rng.bytes(37);
        for (label, sk, pk) in [("C key", &csk, &cpk), ("R key", &rsk, &rpk)] {
            let mut cs = vec![0u8; SPX_BYTES];
            let mut rs = vec![0u8; SPX_BYTES];
            let mut cl = 0usize;
            let mut rl = 0usize;
            unsafe {
                csig_f(cs.as_mut_ptr(), &mut cl, m.as_ptr(), m.len(), sk.as_ptr());
                rsig_f(rs.as_mut_ptr(), &mut rl, m.as_ptr(), m.len(), sk.as_ptr());
            }
            same_val(&format!("{label} siglen"), cl, rl);
            let a = unsafe { rver(cs.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
            let b = unsafe { cver(rs.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
            same_val(&format!("{label}: Rust verifies C signature"), a, 0);
            same_val(&format!("{label}: C verifies Rust signature"), b, 0);
        }
    } else {
        same("crypto_sign_keypair pk", &cpk, &rpk);
        same("crypto_sign_keypair sk", &csk, &rsk);
    }
}

// ---------------------------------------------------------------------------
// C40/C41 — randombytes / randombytes_init and the DRBG state
// ---------------------------------------------------------------------------

#[test]
fn cfg_c40_randombytes_sequence() {
    if URANDOM {
        // /dev/urandom: nondeterministic by construction, nothing to compare.
        // The exercised property is only that both write the requested length.
        type F = unsafe extern "C" fn(*mut u8, u64);
        let (c, r) = libs().pair::<F>("randombytes");
        for xlen in [0usize, 1, 16, 17, 100] {
            let mut cb = vec![0xAAu8; xlen + 8];
            let mut rb = vec![0xAAu8; xlen + 8];
            unsafe {
                c(cb.as_mut_ptr(), xlen as u64);
                r(rb.as_mut_ptr(), xlen as u64);
            }
            same(
                &format!("randombytes(urandom, xlen={xlen}) tail untouched"),
                &cb[xlen..],
                &rb[xlen..],
            );
        }
        return;
    }

    type F = unsafe extern "C" fn(*mut u8, u64) -> c_int;
    let (c, r) = libs().pair::<F>("randombytes");
    let mut rng = Rng::new(SEED ^ 40);
    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);

    let (cdrbg, rdrbg) = data_pair::<DrbgCtx>("DRBG_ctx");

    for xlen in [0usize, 1, 15, 16, 17, 31, 32, 33, 48, 255, 1000] {
        let mut cb = vec![0xAAu8; xlen + 8];
        let mut rb = vec![0xAAu8; xlen + 8];
        let cv = unsafe { c(cb.as_mut_ptr(), xlen as u64) };
        let rv = unsafe { r(rb.as_mut_ptr(), xlen as u64) };
        same_val(&format!("randombytes(xlen={xlen}) return"), cv, rv);
        same(&format!("randombytes(xlen={xlen}) out"), &cb, &rb);
        let cs = unsafe { *cdrbg };
        let rs = unsafe { *rdrbg };
        same_val(&format!("DRBG_ctx after xlen={xlen}"), cs, rs);
    }

    // Force a V carry: set V to all 0xff in both and step once.
    unsafe {
        (*cdrbg).v = [0xff; 16];
        (*rdrbg).v = [0xff; 16];
    }
    let mut cb = vec![0u8; 32];
    let mut rb = vec![0u8; 32];
    unsafe {
        c(cb.as_mut_ptr(), 32);
        r(rb.as_mut_ptr(), 32);
    }
    same("randombytes after V=0xff..ff", &cb, &rb);
    same_val("DRBG_ctx after V carry", unsafe { *cdrbg }, unsafe {
        *rdrbg
    });
}

#[test]
fn cfg_c41_randombytes_init_variants() {
    if URANDOM {
        return; // randombytes_init has no effect on the /dev/urandom provider
    }
    let mut rng = Rng::new(SEED ^ 41);
    let (cdrbg, rdrbg) = data_pair::<DrbgCtx>("DRBG_ctx");
    for with_pers in [false, true] {
        for _ in 0..iters_cheap() {
            let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
            let pers: [u8; 48] = rng.bytes(48).try_into().unwrap();
            seed_both_drbgs(&entropy, if with_pers { Some(&pers) } else { None });
            same_val(
                &format!("DRBG_ctx after randombytes_init(pers={with_pers})"),
                unsafe { *cdrbg },
                unsafe { *rdrbg },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C42 — AES256_ECB
// ---------------------------------------------------------------------------

#[test]
fn cfg_c42_aes256_ecb() {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let (c, r) = libs().pair::<F>("AES256_ECB");
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..iters_cheap() {
        let mut key = rng.bytes(32);
        let mut ctr = rng.bytes(16);
        let mut cb = vec![0xAAu8; 16 + 8];
        let mut rb = vec![0xAAu8; 16 + 8];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), cb.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), rb.as_mut_ptr());
        }
        same("AES256_ECB", &cb, &rb);
    }
    // All-zero and all-ff inputs.
    for fill in [0x00u8, 0xffu8] {
        let mut key = vec![fill; 32];
        let mut ctr = vec![fill; 16];
        let mut cb = vec![0xAAu8; 24];
        let mut rb = vec![0xAAu8; 24];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), cb.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), rb.as_mut_ptr());
        }
        same(&format!("AES256_ECB(all {fill:#x})"), &cb, &rb);
    }
}

// ---------------------------------------------------------------------------
// C43 — AES256_CTR_DRBG_Update
// ---------------------------------------------------------------------------

#[test]
fn cfg_c43_drbg_update() {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let (c, r) = libs().pair::<F>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(SEED ^ 43);
    for with_data in [true, false] {
        for i in 0..iters_cheap() {
            let mut pd = rng.bytes(48);
            let key = rng.bytes(32);
            let v = if i == 0 { vec![0xffu8; 16] } else { rng.bytes(16) };
            let mut ck = key.clone();
            let mut rk = key.clone();
            let mut cv = v.clone();
            let mut rv = v.clone();
            let pdp = if with_data {
                pd.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            };
            unsafe {
                c(pdp, ck.as_mut_ptr(), cv.as_mut_ptr());
                r(pdp, rk.as_mut_ptr(), rv.as_mut_ptr());
            }
            same(&format!("DRBG_Update(data={with_data}) Key"), &ck, &rk);
            same(&format!("DRBG_Update(data={with_data}) V"), &cv, &rv);
        }
    }
}

// ---------------------------------------------------------------------------
// C44 — seedexpander_init + seedexpander
// ---------------------------------------------------------------------------

#[test]
fn cfg_c44_seedexpander() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let (cexp, rexp) = libs().pair::<Exp>("seedexpander");
    let mut rng = Rng::new(SEED ^ 44);

    for maxlen in [16u64, 100, 4096, 0xFFFF_FFFF] {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut cs = AesXof::zeroed();
        let mut rs = AesXof::zeroed();
        let cv = unsafe { cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        let rv = unsafe { rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        same_val(&format!("seedexpander_init(maxlen={maxlen}) ret"), cv, rv);
        same_val(&format!("seedexpander_init(maxlen={maxlen}) ctx"), cs, rs);

        for xlen in [1u64, 15, 16, 17, 32, 100] {
            if xlen >= maxlen {
                continue;
            }
            let mut cb = vec![0xAAu8; xlen as usize + 8];
            let mut rb = vec![0xAAu8; xlen as usize + 8];
            let cr = unsafe { cexp(&mut cs, cb.as_mut_ptr(), xlen) };
            let rr = unsafe { rexp(&mut rs, rb.as_mut_ptr(), xlen) };
            same_val(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"), cr, rr);
            same(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) out"), &cb, &rb);
            same_val(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ctx"), cs, rs);
            if cr != 0 {
                break;
            }
        }

        // Force a ctr[12..16] carry and keep expanding.
        cs.ctr[12..16].copy_from_slice(&[0xff; 4]);
        rs.ctr[12..16].copy_from_slice(&[0xff; 4]);
        cs.length_remaining = 4096;
        rs.length_remaining = 4096;
        cs.buffer_pos = 16;
        rs.buffer_pos = 16;
        let mut cb = vec![0u8; 64];
        let mut rb = vec![0u8; 64];
        let cr = unsafe { cexp(&mut cs, cb.as_mut_ptr(), 64) };
        let rr = unsafe { rexp(&mut rs, rb.as_mut_ptr(), 64) };
        same_val("seedexpander ctr-carry ret", cr, rr);
        same("seedexpander ctr-carry out", &cb, &rb);
        same_val("seedexpander ctr-carry ctx", cs, rs);
    }
}
