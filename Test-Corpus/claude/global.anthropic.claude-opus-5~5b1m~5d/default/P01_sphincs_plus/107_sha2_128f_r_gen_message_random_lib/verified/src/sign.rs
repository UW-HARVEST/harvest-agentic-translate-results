//! Translation of `app/src/sign.c` — the public SPHINCS+ API.
//!
//! These functions keep the exact C linker names (they are NOT namespaced in
//! `api.h`).

use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr,
    set_type,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::merkle_sign;
use crate::params::*;
use crate::rng::randombytes_slice as randombytes;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

fn crypto_sign_seed_keypair_impl(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);

    let pubseed: [u8; SPX_N] = sk[2 * SPX_N..3 * SPX_N].try_into().unwrap();
    pk[..SPX_N].copy_from_slice(&pubseed);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    // merkle_gen_root(sk + 3*SPX_N, &ctx)
    let mut root = [0u8; SPX_N];
    crate::merkle::merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
}

fn sign_signature(sig: &mut [u8], m: &[u8], sk: &[u8]) -> usize {
    let mut ctx = SpxCtx::new();

    let sk_prf: [u8; SPX_N] = sk[SPX_N..2 * SPX_N].try_into().unwrap();
    let pk: [u8; SPX_PK_BYTES] = sk[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES].try_into().unwrap();

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64;
    let mut idx_leaf: u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes(&mut optrand);

    // gen_message_random(sig, sk_prf, optrand, m, mlen, &ctx) -> R at sig[0..N].
    // The whole remaining buffer is handed over because the BLAKE backend
    // finalises its (32- or 64-byte) digest straight into `R`, exactly as the C
    // does; those extra bytes are overwritten by the FORS signature below.
    gen_message_random(sig, &sk_prf, &optrand, m, &ctx);

    {
        let r: [u8; SPX_N] = sig[..SPX_N].try_into().unwrap();
        let mut t: u64 = 0;
        let mut li: u32 = 0;
        hash_message(&mut mhash, &mut t, &mut li, &r, &pk, m, &ctx);
        tree = t;
        idx_leaf = li;
    }

    let mut so = SPX_N; // sig offset (advanced past R)

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(
        &mut sig[so..so + SPX_FORS_BYTES],
        &mut root,
        &mhash,
        &ctx,
        &wots_addr,
    );
    so += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let seg = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        merkle_sign(
            &mut sig[so..so + seg],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        so += seg;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    SPX_BYTES
}

fn sign_verify(sig: &[u8], m: &[u8], pk: &[u8]) -> i32 {
    if sig.len() != SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64;
    let mut idx_leaf: u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    {
        let r: [u8; SPX_N] = sig[..SPX_N].try_into().unwrap();
        let pkarr: [u8; SPX_PK_BYTES] = pk[..SPX_PK_BYTES].try_into().unwrap();
        let mut t: u64 = 0;
        let mut li: u32 = 0;
        hash_message(&mut mhash, &mut t, &mut li, &r, &pkarr, m, &ctx);
        tree = t;
        idx_leaf = li;
    }

    let mut so = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(
        &mut root,
        &sig[so..so + SPX_FORS_BYTES],
        &mhash,
        &ctx,
        &wots_addr,
    );
    so += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        {
            let rootcopy: [u8; SPX_N] = root;
            wots_pk_from_sig(
                &mut wots_pk,
                &sig[so..so + SPX_WOTS_BYTES],
                &rootcopy,
                &ctx,
                &mut wots_addr,
            );
        }
        so += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &wots_pk_addr);

        compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig[so..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        so += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

// ------------------------------------------------------------------
// Public C ABI (plain names, matching api.h).
// ------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn crypto_sign_secretkeybytes() -> core::ffi::c_ulonglong {
    CRYPTO_SECRETKEYBYTES as core::ffi::c_ulonglong
}

#[no_mangle]
pub extern "C" fn crypto_sign_publickeybytes() -> core::ffi::c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as core::ffi::c_ulonglong
}

#[no_mangle]
pub extern "C" fn crypto_sign_bytes() -> core::ffi::c_ulonglong {
    CRYPTO_BYTES as core::ffi::c_ulonglong
}

#[no_mangle]
pub extern "C" fn crypto_sign_seedbytes() -> core::ffi::c_ulonglong {
    CRYPTO_SEEDBYTES as core::ffi::c_ulonglong
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> core::ffi::c_int {
    let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    let seed_s = core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair_impl(pk_s, sk_s, seed_s);
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> core::ffi::c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed);
    let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    crypto_sign_seed_keypair_impl(pk_s, sk_s, &seed);
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> core::ffi::c_int {
    let sig_s = core::slice::from_raw_parts_mut(sig, SPX_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen);
    let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    let n = sign_signature(sig_s, m_s, sk_s);
    *siglen = n;
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> core::ffi::c_int {
    // `sign.c` performs this length check before touching any buffer, so the
    // rejection must not depend on `siglen` being a usable slice length (a
    // caller may legitimately pass a bogus huge value).
    if siglen != SPX_BYTES {
        return -1;
    }
    let sig_s = core::slice::from_raw_parts(sig, siglen);
    let m_s = core::slice::from_raw_parts(m, mlen);
    let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
    sign_verify(sig_s, m_s, pk_s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    sk: *const u8,
) -> core::ffi::c_int {
    let mlen_us = mlen as usize;
    // crypto_sign_signature(sm, &siglen, m, mlen, sk)
    let sm_sig = core::slice::from_raw_parts_mut(sm, SPX_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen_us);
    let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    let siglen = sign_signature(sm_sig, m_s, sk_s);

    // memmove(sm + SPX_BYTES, m, mlen)
    core::ptr::copy(m, sm.add(SPX_BYTES), mlen_us);
    *smlen = (siglen + mlen_us) as core::ffi::c_ulonglong;
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut core::ffi::c_ulonglong,
    sm: *const u8,
    smlen: core::ffi::c_ulonglong,
    pk: *const u8,
) -> core::ffi::c_int {
    let smlen_us = smlen as usize;

    if smlen_us < SPX_BYTES {
        core::ptr::write_bytes(m, 0, smlen_us);
        *mlen = 0;
        return -1;
    }

    let out_len = smlen_us - SPX_BYTES;

    let sig_s = core::slice::from_raw_parts(sm, SPX_BYTES);
    let m_in = core::slice::from_raw_parts(sm.add(SPX_BYTES), out_len);
    let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);

    if sign_verify(sig_s, m_in, pk_s) != 0 {
        core::ptr::write_bytes(m, 0, smlen_us);
        *mlen = 0;
        return -1;
    }

    *mlen = out_len as core::ffi::c_ulonglong;
    core::ptr::copy(sm.add(SPX_BYTES), m, out_len);
    0
}
