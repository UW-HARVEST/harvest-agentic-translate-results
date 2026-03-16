use std::ffi::c_int;

use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash_sha2::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::*;
use crate::thash::thash;
use crate::wots::wots_pk_from_sig;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx::default();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    crate::rng::randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig_slice = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_slice = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };

    let mut ctx = SpxCtx::default();
    let sk_prf = &sk_slice[SPX_N..2 * SPX_N];
    let pk = &sk_slice[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk_slice[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    crate::rng::randombytes(optrand.as_mut_ptr(), SPX_N as u64);

    gen_message_random(sig_slice, sk_prf, &optrand, m_slice, mlen as u64, &ctx);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk, m_slice, mlen as u64, &ctx);

    let mut sig_offset = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig_slice[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig_slice[sig_offset..], &mut root, &ctx, &wots_addr, &mut tree_addr, idx_leaf);
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let sig_slice = unsafe { std::slice::from_raw_parts(sig, SPX_BYTES) };

    let mut ctx = SpxCtx::default();
    let pub_root = &pk_slice[SPX_N..];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk_slice, m_slice, mlen as u64, &ctx);

    let mut sig_offset = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig_slice[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig_slice[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        crate::utils::compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig_slice[sig_offset..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let _sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let _m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };

    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    // memmove: sm + SPX_BYTES <- m, mlen
    // Use copy_within or manual copy since src/dst may overlap
    unsafe {
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
    }
    unsafe { *smlen = siglen as u64 + mlen; }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    let actual_mlen = smlen_usize - SPX_BYTES;
    unsafe { *mlen = actual_mlen as u64; }

    let ret = crypto_sign_verify(sm, SPX_BYTES, unsafe { sm.add(SPX_BYTES) }, actual_mlen, pk);
    if ret != 0 {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        std::ptr::copy(sm.add(SPX_BYTES), m, actual_mlen);
    }

    0
}
