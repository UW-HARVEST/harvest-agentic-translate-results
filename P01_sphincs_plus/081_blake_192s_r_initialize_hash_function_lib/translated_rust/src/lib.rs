#![allow(clippy::missing_safety_doc, clippy::not_unsafe_ptr_arg_deref)]

pub mod params;
pub mod context;
pub mod blake256;
pub mod blake512;
pub mod hash;
pub mod thash;
pub mod utils;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod rng;

use context::SpxCtx;
use params::*;
use std::ffi::c_int;

// ---- Public API (crypto_sign_*) ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    SPX_SK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    SPX_PK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    SPX_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let seed_s = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };

    sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
    pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);
    merkle::merkle_gen_root(&mut sk_s[3 * SPX_N..], &ctx);
    pk_s[SPX_N..2 * SPX_N].copy_from_slice(&sk_s[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    unsafe { crypto_sign_seed_keypair(pk, sk, seed.as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sk_s = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };

    let sk_prf = &sk_s[SPX_N..2 * SPX_N];
    let pk = &sk_s[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    utils::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    utils::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    rng::randombytes(&mut optrand, SPX_N as u64);

    hash::gen_message_random(sig_s, sk_prf, &optrand, m_s, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk, m_s, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    utils::set_tree_addr(&mut wots_addr, tree);
    utils::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_sign(&mut sig_s[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        utils::set_layer_addr(&mut tree_addr, i as u32);
        utils::set_tree_addr(&mut tree_addr, tree);
        utils::copy_subtree_addr(&mut wots_addr, &tree_addr);
        utils::set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle::merkle_sign(&mut sig_s[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let sig_s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };
    ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);

    let pub_root = &pk_s[SPX_N..];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    utils::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    utils::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    utils::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk_s, m_s, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    utils::set_tree_addr(&mut wots_addr, tree);
    utils::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_pk_from_sig(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for _i in 0..SPX_D {
        utils::set_layer_addr(&mut tree_addr, _i as u32);
        utils::set_tree_addr(&mut tree_addr, tree);
        utils::copy_subtree_addr(&mut wots_addr, &tree_addr);
        utils::set_keypair_addr(&mut wots_addr, idx_leaf);
        utils::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots::wots_pk_from_sig(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash::thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        utils::compute_root(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let mut siglen: usize = 0;
    let ret = unsafe { crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk) };

    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sm_s = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    // memmove: copy m after signature
    sm_s.copy_within(..0, 0); // no-op, just for clarity
    sm_s[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m_s);

    unsafe { *smlen = (siglen as u64) + mlen; }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let m_s = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };

    if (smlen as usize) < SPX_BYTES {
        m_s[..smlen as usize].fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    let msg_len = smlen as usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64; }

    let ret = unsafe {
        crypto_sign_verify(
            sm,
            SPX_BYTES,
            sm.add(SPX_BYTES),
            msg_len,
            pk,
        )
    };

    if ret != 0 {
        m_s[..smlen as usize].fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    // memmove m from sm + SPX_BYTES
    m_s[..msg_len].copy_from_slice(&sm_s[SPX_BYTES..SPX_BYTES + msg_len]);
    0
}

// ---- RNG exports ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> c_int {
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes_det(x_s, xlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let ei = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key_s = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_s = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    rng::aes256_ctr_drbg_update(pd, key_s, v_s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> c_int {
    let ctx_r = unsafe { &mut *ctx };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_r, seed_s, div_s, maxlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut u8,
    xlen: u64,
) -> c_int {
    let ctx_r = unsafe { &mut *ctx };
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_r, x_s, xlen)
}
