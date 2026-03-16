pub mod params;
pub mod context;
pub mod address;
pub mod sha2;
pub mod hash;
pub mod thash;
pub mod utils;
pub mod wots;
pub mod wotsx1;
pub mod utilsx1;
pub mod fors;
pub mod merkle;
pub mod rng;

use context::*;
use params::*;
use address::*;
use hash::*;
use fors::*;
use merkle::*;
use wots::*;
use thash::thash;
use utils::compute_root;
use rng::*;

use std::ptr;

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
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        let mut ctx = SpxCtx::default();

        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);

        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);

        initialize_hash_function(&mut ctx);

        merkle_gen_root(&mut sk_s[3 * SPX_N..], &ctx);

        pk_s[SPX_N..2 * SPX_N].copy_from_slice(&sk_s[3 * SPX_N..4 * SPX_N]);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);

        let mut ctx = SpxCtx::default();
        let sk_prf = &sk_s[SPX_N..2 * SPX_N];
        let pk = &sk_s[2 * SPX_N..];

        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        randombytes(&mut optrand, SPX_N as u64);

        gen_message_random(sig_s, sk_prf, &optrand, m_s, mlen as u64, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_sign(&mut sig_s[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);

            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            merkle_sign(&mut sig_s[sig_off..], &mut root, &ctx,
                        &wots_addr, &mut tree_addr, idx_leaf);
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        *siglen = SPX_BYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let sig_s = std::slice::from_raw_parts(sig, siglen);
        let m_s = std::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES {
            return -1;
        }

        let mut ctx = SpxCtx::default();
        let pub_root = &pk_s[SPX_N..];

        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk_s, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut leaf = [0u8; SPX_N];

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);

            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            wots_pk_from_sig(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

            compute_root(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..],
                         SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root != pub_root[..SPX_N] {
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m_out: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    unsafe {
        let smlen_usize = smlen as usize;
        if smlen_usize < SPX_BYTES {
            ptr::write_bytes(m_out, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            ptr::write_bytes(m_out, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        ptr::copy(sm.add(SPX_BYTES), m_out, *mlen as usize);
    }
    0
}

// RNG C API
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init_c(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    unsafe {
        let ei = std::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(personalization_string, 48))
        };
        randombytes_init(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_c(x: *mut u8, xlen: u64) -> i32 {
    unsafe {
        let buf = std::slice::from_raw_parts_mut(x, xlen as usize);
        randombytes(buf, xlen)
    }
}
