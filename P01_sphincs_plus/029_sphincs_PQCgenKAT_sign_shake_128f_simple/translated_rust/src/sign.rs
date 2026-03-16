use crate::params::*;
use crate::address::*;
use crate::hash::*;
use crate::rng;
use crate::spx::*;

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&ctx);

    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);

    0
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature(sig: &mut [u8], m: &[u8], sk: &[u8]) -> usize {
    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&ctx);

    let mut wots_addr = addr_zero();
    let mut tree_addr = addr_zero();
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    rng::randombytes(&mut optrand, SPX_N);

    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    SPX_BYTES
}

pub fn crypto_sign_verify(sig: &[u8], m: &[u8], pk: &[u8]) -> i32 {
    if sig.len() < SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
    };
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    let pub_root = &pk[SPX_N..2 * SPX_N];

    initialize_hash_function(&ctx);

    let mut wots_addr = addr_zero();
    let mut tree_addr = addr_zero();
    let mut wots_pk_addr = addr_zero();
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    let mut wots_pk_buf = [0u8; SPX_WOTS_BYTES];
    let mut leaf = [0u8; SPX_N];

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

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root[..SPX_N] {
        return -1;
    }
    0
}

pub fn crypto_sign(sm: &mut [u8], m: &[u8], sk: &[u8]) -> usize {
    let siglen = crypto_sign_signature(sm, m, sk);
    // memmove sm + SPX_BYTES, m, mlen
    let mlen = m.len();
    sm[SPX_BYTES..SPX_BYTES + mlen].copy_from_slice(m);
    siglen + mlen
}

pub fn crypto_sign_open(m: &mut [u8], sm: &[u8]) -> Result<usize, i32> {
    let smlen = sm.len();
    if smlen < SPX_BYTES {
        m[..smlen].fill(0);
        return Err(-1);
    }
    let mlen = smlen - SPX_BYTES;
    let pk_start = smlen; // pk is passed separately
    // This is called differently - we need pk
    // Actually the C API passes pk separately. Let's adjust.
    unreachable!("use crypto_sign_open_with_pk instead");
}

pub fn crypto_sign_open_with_pk(m: &mut [u8], sm: &[u8], pk: &[u8]) -> Result<usize, i32> {
    let smlen = sm.len();
    if smlen < SPX_BYTES {
        m[..smlen].fill(0);
        return Err(-1);
    }
    let mlen = smlen - SPX_BYTES;
    if crypto_sign_verify(&sm[..SPX_BYTES], &sm[SPX_BYTES..SPX_BYTES + mlen], pk) != 0 {
        m[..smlen].fill(0);
        return Err(-1);
    }
    m[..mlen].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + mlen]);
    Ok(mlen)
}

// merkle
pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
                   wots_addr: &mut Addr, tree_addr: &mut Addr, idx_leaf: u32) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES],
        wots_sign_leaf: idx_leaf,
        wots_steps: [0u32; SPX_WOTS_LEN],
        leaf_addr: addr_zero(),
        pk_addr: addr_zero(),
    };

    chain_lengths(&mut info.wots_steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    let mut root_buf = [0u8; SPX_N];
    let mut auth_buf = vec![0u8; SPX_TREE_HEIGHT * SPX_N];

    wots_treehashx1(&mut root_buf, &mut auth_buf, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);

    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
    sig[auth_path_off..auth_path_off + SPX_TREE_HEIGHT * SPX_N].copy_from_slice(&auth_buf);
    root.copy_from_slice(&root_buf);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = addr_zero();
    let mut wots_addr = addr_zero();

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
