//! Translation of `app/src/sign.c`.
//!
//! These are the `api.h` entry points, which are *not* namespaced in the C
//! reference implementation, so the exported symbol names are kept verbatim.

use core::ptr::{copy, copy_nonoverlapping, write_bytes};

use crate::address::{
    SPX_copy_keypair_addr, SPX_copy_subtree_addr, SPX_set_keypair_addr, SPX_set_layer_addr,
    SPX_set_tree_addr, SPX_set_type,
};
use crate::backend::{gen_message_random, hash_message, initialize_hash_function, thash};
use crate::context::SpxCtx;
use crate::fors::{SPX_fors_pk_from_sig, SPX_fors_sign};
use crate::merkle::{SPX_merkle_gen_root, SPX_merkle_sign};
use crate::params::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES,
    SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK, SPX_BYTES, SPX_D,
    SPX_FORS_BYTES, SPX_FORS_MSG_BYTES, SPX_N, SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN,
};
use crate::rng::randombytes;
use crate::utils::SPX_compute_root;
use crate::wots::SPX_wots_pk_from_sig;

/*
 * Returns the length of a secret key, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

/*
 * Returns the length of a public key, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

/*
 * Returns the length of a signature, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

/*
 * Returns the length of the seed required to generate a key pair, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

/*
 * Generates an SPX key pair given a seed of length
 * Format sk: [SK_SEED || SK_PRF || PUB_SEED || root]
 * Format pk: [PUB_SEED || root]
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    unsafe {
        let mut ctx = SpxCtx::new();

        /* Initialize SK_SEED, SK_PRF and PUB_SEED from seed. */
        copy_nonoverlapping(seed, sk, CRYPTO_SEEDBYTES);

        copy_nonoverlapping(sk.add(2 * SPX_N) as *const u8, pk, SPX_N);

        copy_nonoverlapping(pk as *const u8, ctx.pub_seed.as_mut_ptr(), SPX_N);
        copy_nonoverlapping(sk as *const u8, ctx.sk_seed.as_mut_ptr(), SPX_N);

        /* This hook allows the hash function instantiation to do whatever
        preparation or computation it needs, based on the public seed. */
        initialize_hash_function(&mut ctx);

        /* Compute root node of the top-most subtree. */
        SPX_merkle_gen_root(sk.add(3 * SPX_N), &ctx);

        copy_nonoverlapping(sk.add(3 * SPX_N) as *const u8, pk.add(SPX_N), SPX_N);

        0
    }
}

/*
 * Generates an SPX key pair.
 * Format sk: [SK_SEED || SK_PRF || PUB_SEED || root]
 * Format pk: [PUB_SEED || root]
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    unsafe {
        let mut seed = [0u8; CRYPTO_SEEDBYTES];
        let _ = randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
        crypto_sign_seed_keypair(pk, sk, seed.as_ptr());

        0
    }
}

/**
 * Returns an array containing a detached signature.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    unsafe {
        let mut sig = sig;

        let mut ctx = SpxCtx::new();

        let sk_prf: *const u8 = sk.add(SPX_N);
        let pk: *const u8 = sk.add(2 * SPX_N);

        let mut optrand = [0u8; SPX_N];
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut root = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        let mut wots_addr: [u32; 8] = [0; 8];
        let mut tree_addr: [u32; 8] = [0; 8];

        copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);
        copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

        /* This hook allows the hash function instantiation to do whatever
        preparation or computation it needs, based on the public seed. */
        initialize_hash_function(&mut ctx);

        SPX_set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
        SPX_set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);

        /* Optionally, signing can be made non-deterministic using optrand.
        This can help counter side-channel attacks that would benefit from
        getting a large number of traces when the signer uses the same nodes. */
        let _ = randombytes(optrand.as_mut_ptr(), SPX_N as u64);
        /* Compute the digest randomization value. */
        gen_message_random(sig, sk_prf, optrand.as_ptr(), m, mlen as u64, &ctx);

        /* Derive the message digest and leaf index from R, PK and M. */
        hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig,
            pk,
            m,
            mlen as u64,
            &ctx,
        );
        sig = sig.add(SPX_N);

        SPX_set_tree_addr(wots_addr.as_mut_ptr(), tree);
        SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        /* Sign the message hash using FORS. */
        SPX_fors_sign(
            sig,
            root.as_mut_ptr(),
            mhash.as_ptr(),
            &ctx,
            wots_addr.as_ptr(),
        );
        sig = sig.add(SPX_FORS_BYTES);

        let mut i: u32 = 0;
        while (i as usize) < SPX_D {
            SPX_set_layer_addr(tree_addr.as_mut_ptr(), i);
            SPX_set_tree_addr(tree_addr.as_mut_ptr(), tree);

            SPX_copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
            SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

            SPX_merkle_sign(
                sig,
                root.as_mut_ptr(),
                &ctx,
                wots_addr.as_mut_ptr(),
                tree_addr.as_mut_ptr(),
                idx_leaf,
            );
            sig = sig.add(SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);

            /* Update the indices for the next layer. */
            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;

            i += 1;
        }

        *siglen = SPX_BYTES;

        0
    }
}

/**
 * Verifies a detached signature and message under a given public key.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    unsafe {
        let mut sig = sig;

        let mut ctx = SpxCtx::new();
        let pub_root: *const u8 = pk.add(SPX_N);
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut root = [0u8; SPX_N];
        let mut leaf = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        let mut wots_addr: [u32; 8] = [0; 8];
        let mut tree_addr: [u32; 8] = [0; 8];
        let mut wots_pk_addr: [u32; 8] = [0; 8];

        if siglen != SPX_BYTES {
            return -1;
        }

        copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

        /* This hook allows the hash function instantiation to do whatever
        preparation or computation it needs, based on the public seed. */
        initialize_hash_function(&mut ctx);

        SPX_set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
        SPX_set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);
        SPX_set_type(wots_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTSPK);

        /* Derive the message digest and leaf index from R || PK || M. */
        /* The additional SPX_N is a result of the hash domain separator. */
        hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            sig,
            pk,
            m,
            mlen as u64,
            &ctx,
        );
        sig = sig.add(SPX_N);

        /* Layer correctly defaults to 0, so no need to set_layer_addr */
        SPX_set_tree_addr(wots_addr.as_mut_ptr(), tree);
        SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        SPX_fors_pk_from_sig(
            root.as_mut_ptr(),
            sig,
            mhash.as_ptr(),
            &ctx,
            wots_addr.as_ptr(),
        );
        sig = sig.add(SPX_FORS_BYTES);

        /* For each subtree.. */
        let mut i: u32 = 0;
        while (i as usize) < SPX_D {
            SPX_set_layer_addr(tree_addr.as_mut_ptr(), i);
            SPX_set_tree_addr(tree_addr.as_mut_ptr(), tree);

            SPX_copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
            SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

            SPX_copy_keypair_addr(wots_pk_addr.as_mut_ptr(), wots_addr.as_ptr());

            /* The WOTS public key is only correct if the signature was correct. */
            /* Initially, root is the FORS pk, but on subsequent iterations it is
            the root of the subtree below the currently processed subtree. */
            SPX_wots_pk_from_sig(
                wots_pk.as_mut_ptr(),
                sig,
                root.as_ptr(),
                &ctx,
                wots_addr.as_mut_ptr(),
            );
            sig = sig.add(SPX_WOTS_BYTES);

            /* Compute the leaf node using the WOTS public key. */
            thash(
                leaf.as_mut_ptr(),
                wots_pk.as_ptr(),
                SPX_WOTS_LEN as u32,
                &ctx,
                wots_pk_addr.as_mut_ptr(),
            );

            /* Compute the root node of this subtree. */
            SPX_compute_root(
                root.as_mut_ptr(),
                leaf.as_ptr(),
                idx_leaf,
                0,
                sig,
                SPX_TREE_HEIGHT as u32,
                &ctx,
                tree_addr.as_mut_ptr(),
            );
            sig = sig.add(SPX_TREE_HEIGHT * SPX_N);

            /* Update the indices for the next layer. */
            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;

            i += 1;
        }

        /* Check if the root node equals the root node in the public key. */
        if root[..] != *core::slice::from_raw_parts(pub_root, SPX_N) {
            return -1;
        }

        0
    }
}

/**
 * Returns an array containing the signature followed by the message.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    unsafe {
        let mut siglen: usize = 0;

        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

        copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;

        0
    }
}

/**
 * Verifies a given signature-message pair under a given public key.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    unsafe {
        /* The API caller does not necessarily know what size a signature should be
        but SPHINCS+ signatures are always exactly SPX_BYTES. */
        if smlen < SPX_BYTES as u64 {
            write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        /* If verification was successful, move the message to the right place. */
        copy(sm.add(SPX_BYTES), m, *mlen as usize);

        0
    }
}
