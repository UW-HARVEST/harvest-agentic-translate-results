// Translation of c_src/app/src/sign.c

use core::slice;

use crate::address::{
    copy_keypair_addr_inner, copy_subtree_addr_inner, set_keypair_addr_inner,
    set_layer_addr_inner, set_tree_addr_inner, set_type_inner,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig_inner, fors_sign_inner};
use crate::merkle::{merkle_gen_root_inner, merkle_sign_inner};
use crate::params::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES, SPX_ADDR_TYPE_HASHTREE,
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK, SPX_BYTES, SPX_D, SPX_FORS_BYTES, SPX_FORS_MSG_BYTES,
    SPX_N, SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN,
};
use crate::thash::thash_inner;
use crate::utils::compute_root_inner;
use crate::wots::SPX_wots_pk_from_sig as wots_pk_from_sig_call;

// Hash backend functions
#[cfg(feature = "haraka")]
use crate::hash::haraka::hash::{SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function};
#[cfg(feature = "sha2")]
use crate::hash::sha2::hash::{SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function};
#[cfg(feature = "shake")]
use crate::hash::shake::hash::{SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function};
#[cfg(feature = "blake")]
use crate::hash::blake::hash::{SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function};

// randombytes - uses rng module's deterministic implementation
unsafe extern "C" {
    fn randombytes(x: *mut u8, xlen: u64) -> i32;
}

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
    let pk = unsafe { slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    unsafe { SPX_initialize_hash_function(&mut ctx); }

    {
        let (_, sk_root_part) = sk.split_at_mut(3 * SPX_N);
        merkle_gen_root_inner(&mut sk_root_part[..SPX_N], &ctx);
    }
    let root = sk[3 * SPX_N..3 * SPX_N + SPX_N].to_vec();
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = vec![0u8; CRYPTO_SEEDBYTES];
    unsafe { randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64); }
    unsafe { crypto_sign_seed_keypair(pk, sk, seed.as_ptr()); }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let mut sig_slice = unsafe { slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let sk = unsafe { slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let mut ctx = SpxCtx::new();

    let sk_prf_offset = SPX_N;
    let pk_offset = 2 * SPX_N;
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&sk[pk_offset..pk_offset + SPX_N]);

    unsafe { SPX_initialize_hash_function(&mut ctx); }

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type_inner(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_inner(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = vec![0u8; SPX_N];
    unsafe { randombytes(optrand.as_mut_ptr(), SPX_N as u64); }

    // gen_message_random writes R to sig (first SPX_N bytes)
    unsafe {
        SPX_gen_message_random(
            sig_slice.as_mut_ptr(),
            sk[sk_prf_offset..].as_ptr(),
            optrand.as_ptr(),
            m,
            mlen as u64,
            &ctx,
        );
    }

    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    unsafe {
        SPX_hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig_slice.as_ptr(),
            sk[pk_offset..].as_ptr(),
            m,
            mlen as u64,
            &ctx,
        );
    }
    sig_slice = &mut sig_slice[SPX_N..];

    set_tree_addr_inner(&mut wots_addr, tree);
    set_keypair_addr_inner(&mut wots_addr, idx_leaf);

    let mut root = vec![0u8; SPX_N];
    fors_sign_inner(&mut sig_slice[..SPX_FORS_BYTES], &mut root, &mhash, &ctx, &wots_addr);
    sig_slice = &mut sig_slice[SPX_FORS_BYTES..];

    let mut tree_var = tree;
    let mut idx_leaf_var = idx_leaf;
    for i in 0..SPX_D {
        set_layer_addr_inner(&mut tree_addr, i as u32);
        set_tree_addr_inner(&mut tree_addr, tree_var);

        copy_subtree_addr_inner(&mut wots_addr, &tree_addr);
        set_keypair_addr_inner(&mut wots_addr, idx_leaf_var);

        let chunk_len = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        merkle_sign_inner(
            &mut sig_slice[..chunk_len],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf_var,
        );
        sig_slice = &mut sig_slice[chunk_len..];

        idx_leaf_var = (tree_var & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree_var >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES; }
    let _ = (SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK);
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
    if siglen != SPX_BYTES {
        return -1;
    }
    let mut sig_slice = unsafe { slice::from_raw_parts(sig, siglen) };
    let m_ptr = m;
    let pk_slice = unsafe { slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    let mut ctx = SpxCtx::new();

    let pub_root = &pk_slice[SPX_N..2 * SPX_N];
    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);

    unsafe { SPX_initialize_hash_function(&mut ctx); }

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];
    set_type_inner(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type_inner(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type_inner(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    unsafe {
        SPX_hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig_slice.as_ptr(),
            pk,
            m_ptr,
            mlen as u64,
            &ctx,
        );
    }
    sig_slice = &sig_slice[SPX_N..];

    set_tree_addr_inner(&mut wots_addr, tree);
    set_keypair_addr_inner(&mut wots_addr, idx_leaf);

    let mut root = vec![0u8; SPX_N];
    fors_pk_from_sig_inner(&mut root, &sig_slice[..SPX_FORS_BYTES], &mhash, &ctx, &wots_addr);
    sig_slice = &sig_slice[SPX_FORS_BYTES..];

    let mut tree_var = tree;
    let mut idx_leaf_var = idx_leaf;
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut leaf = vec![0u8; SPX_N];
    for i in 0..SPX_D {
        set_layer_addr_inner(&mut tree_addr, i as u32);
        set_tree_addr_inner(&mut tree_addr, tree_var);

        copy_subtree_addr_inner(&mut wots_addr, &tree_addr);
        set_keypair_addr_inner(&mut wots_addr, idx_leaf_var);

        copy_keypair_addr_inner(&mut wots_pk_addr, &wots_addr);

        // wots_pk_from_sig
        unsafe {
            wots_pk_from_sig_call(
                wots_pk.as_mut_ptr(),
                sig_slice.as_ptr(),
                root.as_ptr(),
                &ctx,
                wots_addr.as_mut_ptr(),
            );
        }
        sig_slice = &sig_slice[SPX_WOTS_BYTES..];

        thash_inner(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);

        compute_root_inner(
            &mut root,
            &leaf,
            idx_leaf_var,
            0,
            &sig_slice[..SPX_TREE_HEIGHT * SPX_N],
            SPX_TREE_HEIGHT,
            &ctx,
            &mut tree_addr,
        );
        sig_slice = &sig_slice[SPX_TREE_HEIGHT * SPX_N..];

        idx_leaf_var = (tree_var & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree_var >>= SPX_TREE_HEIGHT;
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
    unsafe {
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    }
    // memmove sm + SPX_BYTES <- m
    unsafe {
        core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    if smlen < SPX_BYTES as u64 {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }
    let mlen_val = smlen - SPX_BYTES as u64;
    unsafe { *mlen = mlen_val; }
    let r = unsafe {
        crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), mlen_val as usize, pk)
    };
    if r != 0 {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }
    unsafe {
        core::ptr::copy(sm.add(SPX_BYTES), m, mlen_val as usize);
    }
    0
}
