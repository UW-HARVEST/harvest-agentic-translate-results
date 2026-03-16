use crate::address::*;
use crate::context::SpxCtx;
use crate::hash::*;
use crate::params::*;
use crate::rng::randombytes;
use crate::spx::*;
use crate::thash::thash;

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut ctx = SpxCtx::default();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path_offset = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1::default();
    let mut steps = [0u32; SPX_WOTS_LEN];
    info.wots_sig = vec![0u8; SPX_WOTS_BYTES];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps;
    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);
    info.wots_sign_leaf = idx_leaf;

    wots_treehashx1(root, &mut sig[auth_path_offset..], ctx,
                    idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

pub fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize,
                             m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::default();
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

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N);
    gen_message_random(sig, sk_prf, &optrand, m, mlen, &ctx);
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen, &ctx);

    let mut sig_offset = SPX_N;
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_offset..], &mut root, &ctx,
                    &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    if siglen != SPX_BYTES { return -1; }
    let mut ctx = SpxCtx::default();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen, &ctx);
    let mut sig_offset = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);
        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_offset..],
                     SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

pub fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen = 0usize;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    // memmove sm + SPX_BYTES <- m
    for i in (0..mlen as usize).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    *smlen = siglen as u64 + mlen;
    0
}

pub fn crypto_sign_open(m_out: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        for i in 0..smlen as usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }
    *mlen = smlen - SPX_BYTES as u64;
    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..smlen as usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }
    let ml = *mlen as usize;
    m_out[..ml].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + ml]);
    0
}
