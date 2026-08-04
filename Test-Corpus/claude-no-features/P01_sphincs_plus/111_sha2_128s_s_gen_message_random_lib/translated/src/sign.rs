use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr, set_type,
};
use crate::context::SpxCtx;
use crate::fors::{SPX_fors_pk_from_sig, SPX_fors_sign};
use crate::hash::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function,
};
use crate::merkle::{SPX_merkle_gen_root, SPX_merkle_sign};
use crate::params::*;
use crate::rng::SPX_randombytes;
use crate::thash::SPX_thash;
use crate::utils::SPX_compute_root;
use crate::wots::SPX_wots_pk_from_sig;
use std::ffi::c_int;

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
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe {
        let mut ctx = SpxCtx::new();

        std::ptr::copy_nonoverlapping(seed, sk, CRYPTO_SEEDBYTES);
        std::ptr::copy_nonoverlapping(sk.add(2 * SPX_N), pk, SPX_N);

        std::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);
        std::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);

        SPX_initialize_hash_function(&mut ctx);

        SPX_merkle_gen_root(sk.add(3 * SPX_N), &ctx);

        std::ptr::copy_nonoverlapping(sk.add(3 * SPX_N), pk.add(SPX_N), SPX_N);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    SPX_randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut ctx = SpxCtx::new();

        let sk_prf = sk.add(SPX_N);
        let pk = sk.add(2 * SPX_N);

        let mut optrand = [0u8; SPX_N];
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut root = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];

        std::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);
        std::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

        SPX_initialize_hash_function(&mut ctx);

        set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
        set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);

        SPX_randombytes(optrand.as_mut_ptr(), SPX_N as u64);

        SPX_gen_message_random(sig, sk_prf, optrand.as_ptr(), m, mlen as u64, &ctx);

        SPX_hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig,
            pk,
            m,
            mlen as u64,
            &ctx,
        );
        let mut sig_off = SPX_N;

        set_tree_addr(wots_addr.as_mut_ptr(), tree);
        set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        SPX_fors_sign(
            sig.add(sig_off),
            root.as_mut_ptr(),
            mhash.as_ptr(),
            &ctx,
            wots_addr.as_ptr(),
        );
        sig_off += SPX_FORS_BYTES;

        for i in 0..(SPX_D as u32) {
            set_layer_addr(tree_addr.as_mut_ptr(), i);
            set_tree_addr(tree_addr.as_mut_ptr(), tree);

            copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
            set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

            SPX_merkle_sign(
                sig.add(sig_off),
                root.as_mut_ptr(),
                &ctx,
                wots_addr.as_mut_ptr(),
                tree_addr.as_mut_ptr(),
                idx_leaf,
            );
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
            tree >>= SPX_TREE_HEIGHT;
        }

        *siglen = SPX_BYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    unsafe {
        if siglen != SPX_BYTES {
            return -1;
        }

        let mut ctx = SpxCtx::new();
        let pub_root = pk.add(SPX_N);
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
        let mut root = [0u8; SPX_N];
        let mut leaf = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];

        std::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

        SPX_initialize_hash_function(&mut ctx);

        set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
        set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);
        set_type(wots_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTSPK);

        SPX_hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig,
            pk,
            m,
            mlen as u64,
            &ctx,
        );
        let mut sig_off = SPX_N;

        set_tree_addr(wots_addr.as_mut_ptr(), tree);
        set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        SPX_fors_pk_from_sig(
            root.as_mut_ptr(),
            sig.add(sig_off),
            mhash.as_ptr(),
            &ctx,
            wots_addr.as_ptr(),
        );
        sig_off += SPX_FORS_BYTES;

        for i in 0..(SPX_D as u32) {
            set_layer_addr(tree_addr.as_mut_ptr(), i);
            set_tree_addr(tree_addr.as_mut_ptr(), tree);

            copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
            set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

            copy_keypair_addr(wots_pk_addr.as_mut_ptr(), wots_addr.as_ptr());

            SPX_wots_pk_from_sig(
                wots_pk.as_mut_ptr(),
                sig.add(sig_off),
                root.as_ptr(),
                &ctx,
                wots_addr.as_mut_ptr(),
            );
            sig_off += SPX_WOTS_BYTES;

            SPX_thash(
                leaf.as_mut_ptr(),
                wots_pk.as_ptr(),
                SPX_WOTS_LEN as u32,
                &ctx,
                wots_pk_addr.as_mut_ptr(),
            );

            SPX_compute_root(
                root.as_mut_ptr(),
                leaf.as_ptr(),
                idx_leaf,
                0,
                sig.add(sig_off),
                SPX_TREE_HEIGHT as u32,
                &ctx,
                tree_addr.as_mut_ptr(),
            );
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
            tree >>= SPX_TREE_HEIGHT;
        }

        let root_slice = std::slice::from_raw_parts(root.as_ptr(), SPX_N);
        let pub_root_slice = std::slice::from_raw_parts(pub_root, SPX_N);
        if root_slice != pub_root_slice {
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    unsafe {
        if smlen < (SPX_BYTES as u64) {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - (SPX_BYTES as u64);

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}
