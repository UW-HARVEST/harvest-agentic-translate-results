use crate::address::*;
use crate::context::SpxCtx;
use crate::fors;
use crate::hash;
use crate::merkle;
use crate::params::*;
use crate::rng;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    hash::initialize_hash_function(&ctx);

    let mut root = [0u8; SPX_N];
    merkle::merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature(sig: &mut [u8], m: &[u8], sk: &[u8]) -> (i32, usize) {
    let mut ctx = SpxCtx::new();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = addr_zero();
    let mut tree_addr = addr_zero();

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    hash::initialize_hash_function(&ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    rng::randombytes(&mut optrand);
    hash::gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, &ctx);

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle::merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    (0, SPX_BYTES)
}

pub fn crypto_sign_verify(sig: &[u8], m: &[u8], pk: &[u8]) -> i32 {
    if sig.len() < SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk_buf = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = addr_zero();
    let mut tree_addr = addr_zero();
    let mut wots_pk_addr = addr_zero();

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    hash::initialize_hash_function(&ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk_buf, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk_buf, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

pub fn crypto_sign(sm: &mut [u8], m: &[u8], sk: &[u8]) -> (i32, u64) {
    let (ret, siglen) = crypto_sign_signature(sm, m, sk);
    // memmove sm + SPX_BYTES, m
    sm.copy_within(0..0, 0); // noop placeholder
    let mlen = m.len();
    // copy m after signature
    for i in (0..mlen).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    (0, (siglen + mlen) as u64)
}

pub fn crypto_sign_open(m_out: &mut [u8], sm: &[u8], pk: &[u8]) -> (i32, u64) {
    let smlen = sm.len();
    if smlen < SPX_BYTES {
        for i in 0..smlen { m_out[i] = 0; }
        return (-1, 0);
    }

    let mlen = smlen - SPX_BYTES;

    if crypto_sign_verify(&sm[..SPX_BYTES], &sm[SPX_BYTES..SPX_BYTES + mlen], pk) != 0 {
        for i in 0..smlen { m_out[i] = 0; }
        return (-1, 0);
    }

    m_out[..mlen].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + mlen]);
    (0, mlen as u64)
}
