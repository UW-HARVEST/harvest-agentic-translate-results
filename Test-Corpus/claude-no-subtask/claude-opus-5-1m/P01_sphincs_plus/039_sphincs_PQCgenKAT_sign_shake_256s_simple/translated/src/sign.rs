// Sign / verify implementation

use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr,
    set_type, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::*;
use crate::randombytes::randombytes;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

pub fn crypto_sign_secretkeybytes_rs() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}
pub fn crypto_sign_publickeybytes_rs() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}
pub fn crypto_sign_bytes_rs() -> u64 {
    CRYPTO_BYTES as u64
}
pub fn crypto_sign_seedbytes_rs() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

pub fn crypto_sign_seed_keypair_rs(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut root = vec![0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);
    0
}

pub fn crypto_sign_keypair_rs(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = vec![0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair_rs(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature_rs(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = vec![0u8; SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N);

    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, mlen as u64, &ctx);

    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    {
        let r = &sig[..SPX_N].to_vec();
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, r, pk, m, mlen as u64, &ctx);
    }
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    {
        let (sig_fors, _) = sig[sig_off..].split_at_mut(SPX_FORS_BYTES);
        fors_sign(sig_fors, &mut root, &mhash, &ctx, &wots_addr);
    }
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let layer_size = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        let (sig_layer, _) = sig[sig_off..].split_at_mut(layer_size);
        merkle_sign(sig_layer, &mut root, &ctx, &wots_addr, &mut tree_addr, idx_leaf);
        sig_off += layer_size;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify_rs(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES {
        return -1;
    }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let r = &sig[..SPX_N];
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, r, pk, m, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);

        let leaf_clone = leaf.clone();
        compute_root(
            &mut root,
            &leaf_clone,
            idx_leaf,
            0,
            &sig[sig_off..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

pub fn crypto_sign_rs(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen = 0usize;
    crypto_sign_signature_rs(sm, &mut siglen, m, mlen as usize, sk);
    // memmove(sm + SPX_BYTES, m, mlen)
    sm.copy_within(0..0, 0); // no-op
    let mlen_usize = mlen as usize;
    // We need a memmove (allow overlap). The C uses memmove(sm + SPX_BYTES, m, mlen)
    // m and sm could be the same buffer in theory, but in practice they're separate
    // here. We'll handle overlap by copying via temp.
    let m_copy: Vec<u8> = m[..mlen_usize].to_vec();
    sm[SPX_BYTES..SPX_BYTES + mlen_usize].copy_from_slice(&m_copy);
    *smlen = (siglen + mlen_usize) as u64;
    0
}

pub fn crypto_sign_open_rs(m: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        for i in 0..(smlen as usize) {
            m[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;
    let actual_mlen = *mlen as usize;

    if crypto_sign_verify_rs(
        sm,
        SPX_BYTES,
        &sm[SPX_BYTES..SPX_BYTES + actual_mlen],
        actual_mlen,
        pk,
    ) != 0
    {
        for i in 0..(smlen as usize) {
            m[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    let m_copy: Vec<u8> = sm[SPX_BYTES..SPX_BYTES + actual_mlen].to_vec();
    m[..actual_mlen].copy_from_slice(&m_copy);
    0
}

// C-ABI exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    crypto_sign_secretkeybytes_rs()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> u64 {
    crypto_sign_publickeybytes_rs()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> u64 {
    crypto_sign_bytes_rs()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> u64 {
    crypto_sign_seedbytes_rs()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let p = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let s = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let e = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    crypto_sign_seed_keypair_rs(p, s, e)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let p = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let s = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    crypto_sign_keypair_rs(p, s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let s = unsafe { std::slice::from_raw_parts_mut(sig, CRYPTO_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let sl = unsafe { &mut *siglen };
    crypto_sign_signature_rs(s, sl, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    crypto_sign_verify_rs(s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let total_len = CRYPTO_BYTES + mlen as usize;
    let sm_s = unsafe { std::slice::from_raw_parts_mut(sm, total_len) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let sl = unsafe { &mut *smlen };
    crypto_sign_rs(sm_s, sl, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let m_s = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    let ml = unsafe { &mut *mlen };
    crypto_sign_open_rs(m_s, ml, sm_s, smlen, pk_s)
}
