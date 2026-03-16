use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::*;

pub fn crypto_sign_seed_keypair_internal(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
        state_seeded_512: [0u8; 72],
    };
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    crate::hash::initialize_hash_function_internal(&mut ctx);
    crate::merkle::merkle_gen_root_internal(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

pub fn crypto_sign_keypair_internal(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    crate::rng::randombytes_urandom(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair_internal(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature_internal(
    sig: &mut [u8],
    siglen: &mut usize,
    m: &[u8],
    mlen: usize,
    sk: &[u8],
) -> i32 {
    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
        state_seeded_512: [0u8; 72],
    };
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
    crate::hash::initialize_hash_function_internal(&mut ctx);

    set_type_internal(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_internal(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    crate::rng::randombytes_urandom(&mut optrand, SPX_N as u64);
    crate::hash::gen_message_random_internal(sig, sk_prf, &optrand, m, mlen as u64, &ctx);
    crate::hash::hash_message_internal(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);

    let mut sig_offset = SPX_N;
    set_tree_addr_internal(&mut wots_addr, tree);
    set_keypair_addr_internal(&mut wots_addr, idx_leaf);

    crate::fors::fors_sign_internal(&mut sig[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr_internal(&mut tree_addr, i as u32);
        set_tree_addr_internal(&mut tree_addr, tree);
        copy_subtree_addr_internal(&mut wots_addr, &tree_addr);
        set_keypair_addr_internal(&mut wots_addr, idx_leaf);

        crate::merkle::merkle_sign_internal(
            &mut sig[sig_offset..],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify_internal(
    sig: &[u8],
    siglen: usize,
    m: &[u8],
    mlen: usize,
    pk: &[u8],
) -> i32 {
    if siglen != SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
        state_seeded_512: [0u8; 72],
    };
    let pub_root = &pk[SPX_N..];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    crate::hash::initialize_hash_function_internal(&mut ctx);

    set_type_internal(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_internal(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_internal(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    crate::hash::hash_message_internal(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);
    let mut sig_offset = SPX_N;

    set_tree_addr_internal(&mut wots_addr, tree);
    set_keypair_addr_internal(&mut wots_addr, idx_leaf);

    crate::fors::fors_pk_from_sig_internal(&mut root, &sig[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr_internal(&mut tree_addr, i as u32);
        set_tree_addr_internal(&mut tree_addr, tree);
        copy_subtree_addr_internal(&mut wots_addr, &tree_addr);
        set_keypair_addr_internal(&mut wots_addr, idx_leaf);
        copy_keypair_addr_internal(&mut wots_pk_addr, &wots_addr);

        crate::wots::wots_pk_from_sig_internal(&mut wots_pk, &sig[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        crate::thash::thash_internal(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);
        compute_root_internal(&mut root, &leaf, idx_leaf, 0, &sig[sig_offset..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

pub fn crypto_sign_internal(
    sm: &mut [u8],
    smlen: &mut u64,
    m: &[u8],
    mlen: u64,
    sk: &[u8],
) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature_internal(sm, &mut siglen, m, mlen as usize, sk);
    // memmove: sm + SPX_BYTES <- m
    let ml = mlen as usize;
    unsafe {
        core::ptr::copy(m.as_ptr(), sm[SPX_BYTES..].as_mut_ptr(), ml);
    }
    *smlen = (siglen as u64) + mlen;
    0
}

pub fn crypto_sign_open_internal(
    m_out: &mut [u8],
    mlen: &mut u64,
    sm: &[u8],
    smlen: u64,
    pk: &[u8],
) -> i32 {
    let smlen_usize = smlen as usize;
    if (smlen as usize) < SPX_BYTES {
        // memset m to 0
        for i in 0..smlen_usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;

    if crypto_sign_verify_internal(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..smlen_usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }

    let ml = *mlen as usize;
    unsafe {
        core::ptr::copy(sm[SPX_BYTES..].as_ptr(), m_out.as_mut_ptr(), ml);
    }
    0
}
