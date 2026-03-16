use crate::params::*;
use crate::address::*;
use crate::hash::*;
use crate::fors::*;
use crate::merkle::*;
use crate::thash::thash_rs;
use crate::utils::compute_root_rs;
use crate::wots::wots_pk_from_sig_rs;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 { CRYPTO_SECRETKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 { CRYPTO_PUBLICKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 { CRYPTO_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 { CRYPTO_SEEDBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function_rs(&mut ctx);
    merkle_gen_root_rs(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    crate::rng::randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function_rs(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type_rs(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_rs(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    crate::rng::randombytes(optrand.as_mut_ptr(), SPX_N as u64);

    gen_message_random_rs(&mut sig[..SPX_N], sk_prf, &optrand, m);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let (t, l) = hash_message_rs(&mut mhash, &sig[..SPX_N], pk, m);
    tree = t;
    idx_leaf = l;

    let mut sig_offset = SPX_N;

    set_tree_addr_rs(&mut wots_addr, tree);
    set_keypair_addr_rs(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_sign_rs(&mut sig[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr_rs(&mut tree_addr, i as u32);
        set_tree_addr_rs(&mut tree_addr, tree);
        copy_subtree_addr_rs(&mut wots_addr, &tree_addr);
        set_keypair_addr_rs(&mut wots_addr, idx_leaf);

        merkle_sign_rs(
            &mut sig[sig_offset..], &mut root, &ctx,
            &mut wots_addr, &mut tree_addr, idx_leaf,
        );
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };

    if siglen != SPX_BYTES { return -1; }

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function_rs(&mut ctx);

    let pub_root = &pk[SPX_N..];
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    set_type_rs(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_rs(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_rs(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64;
    let mut idx_leaf: u32;
    let (t, l) = hash_message_rs(&mut mhash, &sig[..SPX_N], pk, m);
    tree = t;
    idx_leaf = l;

    let mut sig_offset = SPX_N;

    set_tree_addr_rs(&mut wots_addr, tree);
    set_keypair_addr_rs(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_pk_from_sig_rs(&mut root, &sig[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr_rs(&mut tree_addr, i as u32);
        set_tree_addr_rs(&mut tree_addr, tree);
        copy_subtree_addr_rs(&mut wots_addr, &tree_addr);
        set_keypair_addr_rs(&mut wots_addr, idx_leaf);
        copy_keypair_addr_rs(&mut wots_pk_addr, &wots_addr);

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        wots_pk_from_sig_rs(&mut wots_pk, &sig[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        let mut leaf = [0u8; SPX_N];
        thash_rs(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root_rs(
            &mut root, &leaf, idx_leaf, 0,
            &sig[sig_offset..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr,
        );
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root[..SPX_N] { return -1; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    // memmove: use copy_within or manual copy
    sm.copy_within(0..SPX_BYTES, 0); // noop, already there
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m);
    unsafe { *smlen = (siglen as u64) + mlen; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64, sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    let smlen_usize = smlen as usize;
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };
    let m = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };

    if smlen_usize < SPX_BYTES {
        m[..smlen_usize].fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    unsafe { *mlen = (smlen_usize - SPX_BYTES) as u64; }

    let msg_len = unsafe { *mlen } as usize;
    if crypto_sign_verify(sm.as_ptr(), SPX_BYTES, sm[SPX_BYTES..].as_ptr(), msg_len, pk) != 0 {
        m[..smlen_usize].fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    // memmove
    m[..msg_len].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + msg_len]);
    0
}
