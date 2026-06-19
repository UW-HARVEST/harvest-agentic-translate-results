use crate::address::{
    copy_keypair_addr_rs, copy_subtree_addr_rs, set_keypair_addr_rs, set_layer_addr_rs, set_tree_addr_rs, set_type_rs,
};
use crate::context::spx_ctx;
use crate::fors::{fors_pk_from_sig_rs, fors_sign_rs};
use crate::merkle::{merkle_gen_root_rs, merkle_sign_rs};
use crate::params::*;
use crate::rng::randombytes;
use crate::sha2_backend::{SPX_gen_message_random_rs, SPX_hash_message_rs, SPX_initialize_hash_function, SPX_thash_rs};
use crate::utils::compute_root_rs;
use crate::wots::wots_pk_from_sig_rs;

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

fn crypto_sign_seed_keypair_rs(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = spx_ctx::default();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    SPX_initialize_hash_function(&mut ctx);
    merkle_gen_root_rs(&mut sk[3 * SPX_N..4 * SPX_N], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

fn crypto_sign_keypair_rs(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = vec![0u8; CRYPTO_SEEDBYTES];
    randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair_rs(pk, sk, &seed)
}

fn crypto_sign_signature_rs(sig: &mut [u8], siglen: &mut usize, m: &[u8], sk: &[u8]) -> i32 {
    let mut ctx = spx_ctx::default();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES];
    let mut optrand = [0u8; SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    SPX_initialize_hash_function(&mut ctx);
    set_type_rs(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_rs(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    randombytes(optrand.as_mut_ptr(), SPX_N as u64);
    SPX_gen_message_random_rs(&mut sig[..SPX_N], sk_prf, &optrand, m);
    SPX_hash_message_rs(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m);
    let mut sig_off = SPX_N;
    set_tree_addr_rs(&mut wots_addr, tree);
    set_keypair_addr_rs(&mut wots_addr, idx_leaf);
    fors_sign_rs(&mut sig[sig_off..sig_off + SPX_FORS_BYTES], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;
    for i in 0..SPX_D {
        set_layer_addr_rs(&mut tree_addr, i as u32);
        set_tree_addr_rs(&mut tree_addr, tree);
        copy_subtree_addr_rs(&mut wots_addr, &tree_addr);
        set_keypair_addr_rs(&mut wots_addr, idx_leaf);
        merkle_sign_rs(
            &mut sig[sig_off..sig_off + SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    *siglen = SPX_BYTES;
    0
}

fn crypto_sign_verify_rs(sig: &[u8], m: &[u8], pk: &[u8]) -> i32 {
    if sig.len() != SPX_BYTES {
        return -1;
    }
    let mut ctx = spx_ctx::default();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    SPX_initialize_hash_function(&mut ctx);
    set_type_rs(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_rs(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_rs(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);
    SPX_hash_message_rs(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m);
    let mut sig_off = SPX_N;
    set_tree_addr_rs(&mut wots_addr, tree);
    set_keypair_addr_rs(&mut wots_addr, idx_leaf);
    fors_pk_from_sig_rs(&mut root, &sig[sig_off..sig_off + SPX_FORS_BYTES], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;
    for i in 0..SPX_D {
        set_layer_addr_rs(&mut tree_addr, i as u32);
        set_tree_addr_rs(&mut tree_addr, tree);
        copy_subtree_addr_rs(&mut wots_addr, &tree_addr);
        set_keypair_addr_rs(&mut wots_addr, idx_leaf);
        copy_keypair_addr_rs(&mut wots_pk_addr, &wots_addr);
        wots_pk_from_sig_rs(&mut wots_pk, &sig[sig_off..sig_off + SPX_WOTS_BYTES], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;
        SPX_thash_rs(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);
        compute_root_rs(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig[sig_off..sig_off + SPX_TREE_HEIGHT * SPX_N],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    if root != pub_root {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    crypto_sign_seed_keypair_rs(
        unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) },
        unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) },
        unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    crypto_sign_keypair_rs(
        unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) },
        unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let mut len = 0usize;
    let rc = crypto_sign_signature_rs(
        unsafe { std::slice::from_raw_parts_mut(sig, CRYPTO_BYTES) },
        &mut len,
        unsafe { std::slice::from_raw_parts(m, mlen) },
        unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) },
    );
    unsafe { *siglen = len };
    rc
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8) -> i32 {
    crypto_sign_verify_rs(
        unsafe { std::slice::from_raw_parts(sig, siglen) },
        unsafe { std::slice::from_raw_parts(m, mlen) },
        unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, CRYPTO_BYTES + mlen as usize) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let mut siglen = 0usize;
    let rc = crypto_sign_signature_rs(&mut sm[..CRYPTO_BYTES], &mut siglen, m, sk);
    sm[CRYPTO_BYTES..CRYPTO_BYTES + m.len()].copy_from_slice(m);
    unsafe { *smlen = (siglen + m.len()) as u64 };
    rc
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let m_out = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    if smlen < SPX_BYTES as u64 {
        m_out.fill(0);
        unsafe { *mlen = 0 };
        return -1;
    }
    let msg_len = smlen as usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64 };
    if crypto_sign_verify_rs(&sm[..SPX_BYTES], &sm[SPX_BYTES..], unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) }) != 0 {
        m_out.fill(0);
        unsafe { *mlen = 0 };
        return -1;
    }
    m_out[..msg_len].copy_from_slice(&sm[SPX_BYTES..]);
    0
}
