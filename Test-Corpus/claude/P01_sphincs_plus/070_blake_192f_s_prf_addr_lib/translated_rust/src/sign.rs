use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr,
    set_type, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::{
    SPX_BYTES, SPX_D, SPX_FORS_BYTES, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_SK_BYTES,
    SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN,
};
use crate::rng::randombytes;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub fn crypto_sign_seed_keypair_rs(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::zeroed();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

pub fn crypto_sign_keypair_rs(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair_rs(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature_rs(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::zeroed();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes(&mut optrand, SPX_N as u64);

    // Compute R; first SPX_N bytes of sig.
    let mut r = [0u8; SPX_N];
    gen_message_random(&mut r, sk_prf, &optrand, m, mlen as u64, &ctx);
    sig[..SPX_N].copy_from_slice(&r);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &r, pk, m, mlen as u64, &ctx);
    let mut sig_pos = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_sign(&mut sig[sig_pos..], &mut root, &mhash, &ctx, &wots_addr);
    sig_pos += SPX_FORS_BYTES;

    let mut tree = tree;
    let mut idx_leaf = idx_leaf;
    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_pos..], &mut root, &ctx, &wots_addr, &mut tree_addr, idx_leaf);
        sig_pos += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT as u32;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify_rs(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::zeroed();
    if siglen != SPX_BYTES {
        return -1;
    }
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen as u64, &ctx);
    let mut sig_pos = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_pos..], &mhash, &ctx, &wots_addr);
    sig_pos += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_pos..], &root, &ctx, &mut wots_addr);
        sig_pos += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_pos..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_pos += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT as u32;
    }

    if &root[..] != pub_root {
        return -1;
    }
    0
}

pub fn crypto_sign_rs(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature_rs(sm, &mut siglen, m, mlen as usize, sk);
    // Move message in
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(&m[..mlen as usize]);
    *smlen = (siglen + mlen as usize) as u64;
    0
}

pub fn crypto_sign_open_rs(m: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if smlen < SPX_BYTES as u64 {
        for b in &mut m[..smlen as usize] {
            *b = 0;
        }
        *mlen = 0;
        return -1;
    }
    *mlen = smlen - SPX_BYTES as u64;
    if crypto_sign_verify_rs(sm, SPX_BYTES, &sm[SPX_BYTES..SPX_BYTES + *mlen as usize], *mlen as usize, pk) != 0 {
        for b in &mut m[..smlen as usize] {
            *b = 0;
        }
        *mlen = 0;
        return -1;
    }
    let mlen_v = *mlen as usize;
    m[..mlen_v].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + mlen_v]);
    0
}

// ---------- C-ABI exports ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> core::ffi::c_ulonglong {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> core::ffi::c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> core::ffi::c_ulonglong {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> core::ffi::c_ulonglong {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(export_name = "crypto_sign_seed_keypair")]
pub unsafe extern "C" fn c_crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> core::ffi::c_int {
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk_slice = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed_slice = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    crypto_sign_seed_keypair_rs(pk_slice, sk_slice, seed_slice)
}

#[unsafe(export_name = "crypto_sign_keypair")]
pub unsafe extern "C" fn c_crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> core::ffi::c_int {
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk_slice = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    crypto_sign_keypair_rs(pk_slice, sk_slice)
}

#[unsafe(export_name = "crypto_sign_signature")]
pub unsafe extern "C" fn c_crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> core::ffi::c_int {
    let sig_slice = unsafe { core::slice::from_raw_parts_mut(sig, CRYPTO_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen) };
    let sk_slice = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let mut len: usize = 0;
    let r = crypto_sign_signature_rs(sig_slice, &mut len, m_slice, mlen, sk_slice);
    unsafe { *siglen = len; }
    r
}

#[unsafe(export_name = "crypto_sign_verify")]
pub unsafe extern "C" fn c_crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> core::ffi::c_int {
    let sig_slice = unsafe { core::slice::from_raw_parts(sig, siglen) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    crypto_sign_verify_rs(sig_slice, siglen, m_slice, mlen, pk_slice)
}

#[unsafe(export_name = "crypto_sign")]
pub unsafe extern "C" fn c_crypto_sign(
    sm: *mut u8,
    smlen: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    sk: *const u8,
) -> core::ffi::c_int {
    let sm_slice = unsafe { core::slice::from_raw_parts_mut(sm, CRYPTO_BYTES + mlen as usize) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let sk_slice = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let mut len: u64 = 0;
    let r = crypto_sign_rs(sm_slice, &mut len, m_slice, mlen, sk_slice);
    unsafe { *smlen = len; }
    r
}

#[unsafe(export_name = "crypto_sign_open")]
pub unsafe extern "C" fn c_crypto_sign_open(
    m: *mut u8,
    mlen: *mut core::ffi::c_ulonglong,
    sm: *const u8,
    smlen: core::ffi::c_ulonglong,
    pk: *const u8,
) -> core::ffi::c_int {
    let m_slice = unsafe { core::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm_slice = unsafe { core::slice::from_raw_parts(sm, smlen as usize) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    let mut len: u64 = 0;
    let r = crypto_sign_open_rs(m_slice, &mut len, sm_slice, smlen, pk_slice);
    unsafe { *mlen = len; }
    r
}
