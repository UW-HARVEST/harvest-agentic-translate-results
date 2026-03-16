mod params;
mod context;
mod address;
mod sha2;
mod hash;
mod thash;
mod utils;
mod wots;
mod fors;
mod merkle;
mod rng;

use params::*;
use context::SpxCtx;
use address::*;
use hash::*;
use fors::*;
use merkle::*;
use wots::*;
use rng::*;

use std::ffi::c_int;
use std::os::raw::c_uchar;

// ============ crypto_sign API ============

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

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx::default();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes_urandom(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };

    let mut ctx = SpxCtx::default();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes_urandom(&mut optrand, SPX_N as u64);

    gen_message_random(sig, sk_prf, &optrand, m, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

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

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig_full = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx::default();
    let pub_root = &pk[SPX_N..];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_full, pk, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;
    let mut root = [0u8; SPX_N];
    let mut wots_pk_buf = [0u8; SPX_WOTS_BYTES];
    let mut leaf = [0u8; SPX_N];

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig_full[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for _i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, _i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk_buf, &sig_full[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash::thash(&mut leaf, &wots_pk_buf, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        utils::compute_root(&mut root, &leaf, idx_leaf, 0,
                            &sig_full[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root[..SPX_N] {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar, smlen: *mut u64, m: *const c_uchar, mlen: u64, sk: *const c_uchar,
) -> c_int {
    let sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };

    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen as *mut usize, m, mlen as usize, sk);

    // memmove sm + SPX_BYTES <- m
    sm_slice[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m_slice);
    unsafe { *smlen = siglen as u64 + mlen; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar, mlen: *mut u64, sm: *const c_uchar, smlen: u64, pk: *const c_uchar,
) -> c_int {
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        m_slice.fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    let msg_len = smlen_usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64; }

    let sm_slice = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };

    if crypto_sign_verify(sm, SPX_BYTES, unsafe { sm.add(SPX_BYTES) }, msg_len, pk) != 0 {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        m_slice.fill(0);
        unsafe { *mlen = 0; }
        return -1;
    }

    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, msg_len) };
    m_slice.copy_from_slice(&sm_slice[SPX_BYTES..SPX_BYTES + msg_len]);
    0
}

// ============ RNG API ============

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar, key: *mut c_uchar, v: *mut c_uchar,
) {
    let key_slice = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_slice = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    let data = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    aes256_ctr_drbg_update(data, key_slice, v_slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct, seed: *mut c_uchar, diversifier: *mut c_uchar, maxlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut AesXofStruct, x: *mut c_uchar, xlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx, x, xlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar, personalization_string: *mut c_uchar,
) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    randombytes_init_internal(entropy, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) -> c_int {
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes_internal(x, xlen)
}
