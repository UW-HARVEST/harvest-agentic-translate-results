use crate::params::*;
use crate::context::SpxCtx;
use crate::hash::{initialize_hash_function, gen_message_random, hash_message};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::fors::{fors_sign, fors_pk_from_sig};
use crate::thash::thash;
use crate::wots::wots_pk_from_sig;
use crate::utils::compute_root;
use crate::address::{set_type, set_tree_addr, set_keypair_addr, set_layer_addr, copy_subtree_addr, copy_keypair_addr};
use crate::randombytes::randombytes;

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    sk[..3 * SPX_N].copy_from_slice(&seed[..3 * SPX_N]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    let mut ctx = SpxCtx::default();
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut root = vec![0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);

    0
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = vec![0u8; 3 * SPX_N];
    randombytes(&mut seed);
    crypto_sign_seed_keypair(pk, sk, &seed)
}

pub fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::default();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..4 * SPX_N];

    let mut optrand = vec![0u8; SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand);
    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, &ctx);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, &ctx);
    let mut sig_pos = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_pos..sig_pos + SPX_FORS_BYTES], &mut root, &mhash, &ctx, &wots_addr);
    sig_pos += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_pos..sig_pos + SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_pos += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify(sig: &[u8], m: &[u8], pk: &[u8]) -> i32 {
    if sig.len() != SPX_BYTES { return -1; }

    let mut ctx = SpxCtx::default();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, &ctx);
    let mut sig_pos = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_pos..sig_pos + SPX_FORS_BYTES], &mhash, &ctx, &wots_addr);
    sig_pos += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_pos..sig_pos + SPX_WOTS_BYTES], &root, &ctx, &mut wots_addr);
        sig_pos += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_pos..sig_pos + SPX_TREE_HEIGHT * SPX_N], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_pos += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root { return -1; }
    0
}

pub fn crypto_sign(sm: &mut [u8], smlen: &mut usize, m: &[u8], sk: &[u8]) -> i32 {
    let mut siglen = 0;
    crypto_sign_signature(sm, &mut siglen, m, sk);
    sm[SPX_BYTES..SPX_BYTES + m.len()].copy_from_slice(m);
    *smlen = siglen + m.len();
    0
}

pub fn crypto_sign_open(m: &mut [u8], mlen: &mut usize, sm: &[u8], pk: &[u8]) -> i32 {
    if sm.len() < SPX_BYTES {
        *mlen = 0;
        return -1;
    }
    *mlen = sm.len() - SPX_BYTES;
    if crypto_sign_verify(&sm[..SPX_BYTES], &sm[SPX_BYTES..], pk) != 0 {
        *mlen = 0;
        return -1;
    }
    m[..*mlen].copy_from_slice(&sm[SPX_BYTES..]);
    0
}
