// Translation of c_src/app/src/sign.c

use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr, set_type,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES,
    SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK, SPX_BYTES, SPX_D,
    SPX_FORS_BYTES, SPX_FORS_MSG_BYTES, SPX_N, SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN,
};
use crate::rng::randombytes;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

pub fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}
pub fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}
pub fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}
pub fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    // Compute root node into sk[3N..4N]
    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..4 * SPX_N];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N as u64);
    {
        let (sig_r, _) = sig.split_at_mut(SPX_N);
        gen_message_random(sig_r, sk_prf, &optrand, m, mlen as u64, &ctx);
    }

    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    {
        let r_block = sig[..SPX_N].to_vec();
        hash_message(
            &mut mhash,
            &mut tree,
            &mut idx_leaf,
            &r_block,
            pk,
            m,
            mlen as u64,
            &ctx,
        );
    }

    let mut sig_off = SPX_N;
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(
        &mut sig[sig_off..sig_off + SPX_FORS_BYTES],
        &mut root,
        &mhash,
        &ctx,
        &wots_addr,
    );
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let layer_len = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        merkle_sign(
            &mut sig[sig_off..sig_off + layer_len],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += layer_len;

        idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];

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
    hash_message(
        &mut mhash,
        &mut tree,
        &mut idx_leaf,
        &sig[..SPX_N],
        pk,
        m,
        mlen as u64,
        &ctx,
    );

    let mut sig_off = SPX_N;
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(
        &mut root,
        &sig[sig_off..sig_off + SPX_FORS_BYTES],
        &mhash,
        &ctx,
        &wots_addr,
    );
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(
            &mut wots_pk,
            &sig[sig_off..sig_off + SPX_WOTS_BYTES],
            &root,
            &ctx,
            &mut wots_addr,
        );
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);

        let auth_len = SPX_TREE_HEIGHT * SPX_N;
        compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig[sig_off..sig_off + auth_len],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += auth_len;

        idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

pub fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    // memmove(sm + SPX_BYTES, m, mlen);
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(&m[..mlen as usize]);
    *smlen = (siglen + mlen as usize) as u64;
    0
}

pub fn crypto_sign_open(m: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        for x in m[..smlen as usize].iter_mut() {
            *x = 0;
        }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;
    let m_part_len = *mlen as usize;

    if crypto_sign_verify(&sm[..SPX_BYTES], SPX_BYTES, &sm[SPX_BYTES..SPX_BYTES + m_part_len], m_part_len, pk) != 0 {
        for x in m[..smlen as usize].iter_mut() {
            *x = 0;
        }
        *mlen = 0;
        return -1;
    }

    // memmove(m, sm + SPX_BYTES, *mlen);
    m[..m_part_len].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + m_part_len]);
    0
}
