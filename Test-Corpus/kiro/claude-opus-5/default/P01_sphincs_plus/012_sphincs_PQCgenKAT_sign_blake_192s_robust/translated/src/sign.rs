//! Translation of `app/src/sign.c` (the `app/include/api.h` entry points).

use core::ffi::{c_int, c_ulonglong};

use crate::address::*;
use crate::backend::{gen_message_random, hash_message, initialize_hash_function, thash};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::*;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

/// Generates an SPX key pair given a seed.
///
/// Format sk: `[SK_SEED || SK_PRF || PUB_SEED || root]`
/// Format pk: `[PUB_SEED || root]`
pub fn crypto_sign_seed_keypair_impl(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> c_int {
    let mut ctx = SpxCtx::new();

    /* Initialize SK_SEED, SK_PRF and PUB_SEED from seed. */
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);

    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    initialize_hash_function(&mut ctx);

    /* Compute root node of the top-most subtree. */
    merkle_gen_root(&mut sk[3 * SPX_N..4 * SPX_N], &ctx);

    let root: [u8; SPX_N] = sk[3 * SPX_N..4 * SPX_N].try_into().unwrap();
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);

    0
}

/// Generates an SPX key pair.
pub fn crypto_sign_keypair_impl(pk: &mut [u8], sk: &mut [u8]) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    crate::randombytes_fill(&mut seed);
    crypto_sign_seed_keypair_impl(pk, sk, &seed);

    0
}

/// Returns an array containing a detached signature; the second element of the
/// result is `siglen`.
pub fn crypto_sign_signature_impl(sig: &mut [u8], m: &[u8], sk: &[u8]) -> (c_int, usize) {
    let mut ctx = SpxCtx::new();

    let sk_prf: [u8; SPX_N] = sk[SPX_N..2 * SPX_N].try_into().unwrap();
    let pk: [u8; SPX_PK_BYTES] = sk[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES].try_into().unwrap();

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    /* Optionally, signing can be made non-deterministic using optrand. */
    crate::randombytes_fill(&mut optrand);
    /* Compute the digest randomization value. */
    gen_message_random(&mut sig[..SPX_N], &sk_prf, &optrand, m, &ctx);

    /* Derive the message digest and leaf index from R, PK and M. */
    let r: [u8; SPX_N] = sig[..SPX_N].try_into().unwrap();
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &r, &pk, m, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    /* Sign the message hash using FORS. */
    fors_sign(
        &mut sig[sig_off..sig_off + SPX_FORS_BYTES],
        &mut root,
        &mhash,
        &ctx,
        &wots_addr,
    );
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let chunk = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        merkle_sign(
            &mut sig[sig_off..sig_off + chunk],
            &mut root,
            &ctx,
            &wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += chunk;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    (0, SPX_BYTES)
}

/// Verifies a detached signature and message under a given public key.
pub fn crypto_sign_verify_impl(sig: &[u8], siglen: usize, m: &[u8], pk: &[u8]) -> c_int {
    let mut ctx = SpxCtx::new();
    let pub_root: [u8; SPX_N] = pk[SPX_N..2 * SPX_N].try_into().unwrap();
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES {
        return -1;
    }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    /* Derive the message digest and leaf index from R || PK || M. */
    let r: [u8; SPX_N] = sig[..SPX_N].try_into().unwrap();
    hash_message(
        &mut mhash,
        &mut tree,
        &mut idx_leaf,
        &r,
        &pk[..SPX_PK_BYTES],
        m,
        &ctx,
    );
    let mut sig_off = SPX_N;

    /* Layer correctly defaults to 0, so no need to set_layer_addr */
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(
        &mut root,
        &sig[sig_off..sig_off + SPX_FORS_BYTES],
        &mhash,
        &ctx,
        &wots_addr,
    );
    sig_off += SPX_FORS_BYTES;

    /* For each subtree.. */
    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        /* The WOTS public key is only correct if the signature was correct.
           Initially, root is the FORS pk, but on subsequent iterations it is
           the root of the subtree below the currently processed subtree. */
        let msg = root;
        wots_pk_from_sig(
            &mut wots_pk,
            &sig[sig_off..sig_off + SPX_WOTS_BYTES],
            &msg,
            &ctx,
            &mut wots_addr,
        );
        sig_off += SPX_WOTS_BYTES;

        /* Compute the leaf node using the WOTS public key. */
        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        /* Compute the root node of this subtree. */
        let leaf_copy = leaf;
        compute_root(
            &mut root,
            &leaf_copy,
            idx_leaf,
            0,
            &sig[sig_off..sig_off + SPX_TREE_HEIGHT * SPX_N],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    /* Check if the root node equals the root node in the public key. */
    if root != pub_root {
        return -1;
    }

    0
}

/// Returns an array containing the signature followed by the message; the
/// second element of the result is `smlen`.
pub fn crypto_sign_impl(sm: &mut [u8], m: &[u8], sk: &[u8]) -> (c_int, u64) {
    let mlen = m.len();
    let (_ret, siglen) = crypto_sign_signature_impl(sm, m, sk);

    sm[SPX_BYTES..SPX_BYTES + mlen].copy_from_slice(m);

    (0, (siglen + mlen) as u64)
}

/// Verifies a given signature-message pair under a given public key; the
/// second element of the result is `mlen`.
pub fn crypto_sign_open_impl(m: &mut [u8], sm: &[u8], pk: &[u8]) -> (c_int, u64) {
    let smlen = sm.len();

    /* The API caller does not necessarily know what size a signature should be
       but SPHINCS+ signatures are always exactly SPX_BYTES. */
    if smlen < SPX_BYTES {
        m[..smlen].fill(0);
        return (-1, 0);
    }

    let mlen = smlen - SPX_BYTES;

    if crypto_sign_verify_impl(&sm[..SPX_BYTES], SPX_BYTES, &sm[SPX_BYTES..], pk) != 0 {
        m[..smlen].fill(0);
        return (-1, 0);
    }

    /* If verification was successful, move the message to the right place. */
    m[..mlen].copy_from_slice(&sm[SPX_BYTES..]);

    (0, mlen as u64)
}

// ---------------------------------------------------------------------------
// C ABI.  `api.h` does not rename these, so the plain names are used.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    CRYPTO_SECRETKEYBYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    CRYPTO_BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    CRYPTO_SEEDBYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe {
        crypto_sign_seed_keypair_impl(
            core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES),
            core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES),
            core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe {
        crypto_sign_keypair_impl(
            core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES),
            core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    unsafe {
        let (ret, len) = crypto_sign_signature_impl(
            core::slice::from_raw_parts_mut(sig, SPX_BYTES),
            core::slice::from_raw_parts(m, mlen),
            core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES),
        );
        *siglen = len;
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
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
        crypto_sign_verify_impl(
            core::slice::from_raw_parts(sig, siglen),
            siglen,
            core::slice::from_raw_parts(m, mlen),
            core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mlen = mlen as usize;
        let (ret, len) = crypto_sign_impl(
            core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen),
            core::slice::from_raw_parts(m, mlen),
            core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES),
        );
        *smlen = len as c_ulonglong;
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut c_ulonglong,
    sm: *const u8,
    smlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    unsafe {
        let smlen = smlen as usize;
        let (ret, len) = crypto_sign_open_impl(
            core::slice::from_raw_parts_mut(m, smlen),
            core::slice::from_raw_parts(sm, smlen),
            core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES),
        );
        *mlen = len as c_ulonglong;
        ret
    }
}
