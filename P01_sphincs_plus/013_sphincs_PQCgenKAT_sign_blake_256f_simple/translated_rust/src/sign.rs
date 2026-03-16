use crate::params::*;
use crate::address;
use crate::hash_blake::{initialize_hash_function, gen_message_random, hash_message};
use crate::thash::thash;
use crate::fors::{fors_sign, fors_pk_from_sig};
use crate::merkle::{merkle_sign, merkle_gen_root};
use crate::wots::wots_pk_from_sig;
use crate::utils::compute_root;
use crate::rng;

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    rng::randombytes(&mut optrand, SPX_N as u64);
    gen_message_random(sig, sk_prf, &optrand, &m[..mlen], mlen as u64, &ctx);
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, &m[..mlen], mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, i as u32);
        address::set_tree_addr(&mut tree_addr, tree);
        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES { return -1; }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    address::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, i as u32);
        address::set_tree_addr(&mut tree_addr, tree);
        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);
        address::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

pub fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    let ml = mlen as usize;
    // memmove: copy m after signature
    for i in (0..ml).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    *smlen = (siglen as u64) + mlen;
    0
}

pub fn crypto_sign_open(m: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if smlen < SPX_BYTES as u64 {
        for i in 0..smlen as usize { m[i] = 0; }
        *mlen = 0;
        return -1;
    }
    *mlen = smlen - SPX_BYTES as u64;
    let ml = *mlen as usize;

    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], ml, pk) != 0 {
        for i in 0..smlen as usize { m[i] = 0; }
        *mlen = 0;
        return -1;
    }

    // memmove
    for i in 0..ml {
        m[i] = sm[SPX_BYTES + i];
    }
    0
}
