#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

mod params;
mod context;
mod blake256;
mod blake512;
mod hash;
mod utils;
mod wots;
mod fors;
mod merkle;

use context::*;
use hash::*;
use merkle::*;
use params::*;
use fors::*;
use wots::*;

use std::fs::File;
use std::io::Read;

fn randombytes(x: &mut [u8], xlen: usize) {
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut remaining = xlen;
    let mut offset = 0;
    while remaining > 0 {
        let to_read = std::cmp::min(remaining, 1048576);
        match f.read(&mut x[offset..offset + to_read]) {
            Ok(n) if n > 0 => {
                offset += n;
                remaining -= n;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    SPX_SK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    SPX_PK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    SPX_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    unsafe { crypto_sign_seed_keypair(pk, sk, seed.as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };

    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
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
    randombytes(&mut optrand, SPX_N);

    gen_message_random(sig, sk_prf, &optrand, m, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);

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

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let sig = unsafe { std::slice::from_raw_parts(sig, SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };

    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
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
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        let mut leaf = [0u8; SPX_N];
        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        utils::compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
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
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;
    unsafe {
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

        // memmove(sm + SPX_BYTES, m, mlen)
        let m_slice = std::slice::from_raw_parts(m, mlen as usize);
        let sm_dest = std::slice::from_raw_parts_mut(sm.add(SPX_BYTES), mlen as usize);
        // Use copy_within-like behavior for potential overlap
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        let _ = (m_slice, sm_dest); // suppress unused warnings

        *smlen = (siglen as u64) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    unsafe {
        if (smlen as usize) < SPX_BYTES {
            let m_slice = std::slice::from_raw_parts_mut(m, smlen as usize);
            for b in m_slice.iter_mut() { *b = 0; }
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            let m_slice = std::slice::from_raw_parts_mut(m, smlen as usize);
            for b in m_slice.iter_mut() { *b = 0; }
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}
