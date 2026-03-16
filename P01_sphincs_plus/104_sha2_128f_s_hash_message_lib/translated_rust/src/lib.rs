#![allow(non_snake_case, clippy::missing_safety_doc)]

mod address;
mod context;
mod fors;
mod hash;
mod merkle;
mod params;
mod rng;
mod sha2;
mod thash;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;

use context::SpxCtx;
use params::*;

// ---- Public C API ----

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
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx::default();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);
    merkle::merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    unsafe { crypto_sign_seed_keypair(pk, sk, seed.as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sig_slice = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };

    let mut ctx = SpxCtx::default();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    hash::initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    rng::randombytes(&mut optrand, SPX_N as u64);

    hash::gen_message_random(sig_slice, sk_prf, &optrand, m_slice, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk, m_slice, mlen as u64, &ctx);

    let mut sig_offset = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_sign(&mut sig_slice[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D as u32 {
        address::set_layer_addr(&mut tree_addr, i);
        address::set_tree_addr(&mut tree_addr, tree);

        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle::merkle_sign(
            &mut sig_slice[sig_offset..],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let sig_slice = unsafe { std::slice::from_raw_parts(sig, SPX_BYTES) };

    let mut ctx = SpxCtx::default();
    let pub_root = &pk_slice[SPX_N..];

    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);
    hash::initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    address::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;

    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk_slice, m_slice, mlen as u64, &ctx);

    let mut sig_offset = SPX_N;
    let mut root = [0u8; SPX_N];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut leaf = [0u8; SPX_N];

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_pk_from_sig(&mut root, &sig_slice[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D as u32 {
        address::set_layer_addr(&mut tree_addr, i);
        address::set_tree_addr(&mut tree_addr, tree);

        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        address::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots::wots_pk_from_sig(&mut wots_pk, &sig_slice[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        thash::thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        utils::compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig_slice[sig_offset..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;
    let ret = unsafe { crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk) };

    // memmove sm + SPX_BYTES <- m
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    // Use copy_within-like approach for memmove safety
    sm_slice[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m_slice);

    unsafe { *smlen = siglen as u64 + mlen };
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        for b in m_slice.iter_mut() {
            *b = 0;
        }
        unsafe { *mlen = 0 };
        return -1;
    }

    unsafe { *mlen = smlen - SPX_BYTES as u64 };
    let msg_len = unsafe { *mlen } as usize;

    let sm_slice = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };

    if unsafe { crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), msg_len, pk) } != 0 {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        for b in m_slice.iter_mut() {
            *b = 0;
        }
        unsafe { *mlen = 0 };
        return -1;
    }

    // memmove m <- sm + SPX_BYTES
    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, msg_len) };
    m_slice.copy_from_slice(&sm_slice[SPX_BYTES..SPX_BYTES + msg_len]);

    0
}

// RNG exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(entropy, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(buf, xlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    // This is exported but we handle it internally via rng module.
    // For ABI compatibility, provide a stub that works through the raw pointers.
    let key_slice = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_slice = unsafe { std::slice::from_raw_parts_mut(v, 16) };

    let mut key_arr = [0u8; 32];
    let mut v_arr = [0u8; 16];
    key_arr.copy_from_slice(key_slice);
    v_arr.copy_from_slice(v_slice);

    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v_arr[j] == 0xff {
                v_arr[j] = 0x00;
            } else {
                v_arr[j] += 1;
                break;
            }
        }
        let cipher = openssl::symm::Cipher::aes_256_ecb();
        let mut crypter = openssl::symm::Crypter::new(cipher, openssl::symm::Mode::Encrypt, &key_arr, None).unwrap();
        crypter.pad(false);
        let count = crypter.update(&v_arr, &mut temp[16 * i..]).unwrap();
        let _ = crypter.finalize(&mut temp[16 * i + count..]).unwrap();
    }

    if !provided_data.is_null() {
        let pd = unsafe { std::slice::from_raw_parts(provided_data, 48) };
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }

    key_arr.copy_from_slice(&temp[..32]);
    v_arr.copy_from_slice(&temp[32..48]);

    key_slice.copy_from_slice(&key_arr);
    v_slice.copy_from_slice(&v_arr);
}
