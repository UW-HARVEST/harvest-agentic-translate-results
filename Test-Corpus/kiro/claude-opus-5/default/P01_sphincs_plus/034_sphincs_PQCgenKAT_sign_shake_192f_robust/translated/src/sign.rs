//! Translation of `app/src/sign.c` and `app/include/api.h`.

use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr, set_type,
    ZERO_ADDR, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK,
};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::*;
use crate::randombytes::randombytes_rs;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;
use core::ffi::{c_int, c_ulonglong};

/// Returns the length of a secret key, in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    CRYPTO_SECRETKEYBYTES as c_ulonglong
}

/// Returns the length of a public key, in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as c_ulonglong
}

/// Returns the length of a signature, in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    CRYPTO_BYTES as c_ulonglong
}

/// Returns the length of the seed required to generate a key pair, in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    CRYPTO_SEEDBYTES as c_ulonglong
}

/// Generates an SPX key pair given a seed.
///
/// Format sk: `[SK_SEED || SK_PRF || PUB_SEED || root]`
/// Format pk: `[PUB_SEED || root]`
pub fn crypto_sign_seed_keypair_rs(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
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
    unsafe {
        merkle_gen_root(&mut sk[3 * SPX_N..4 * SPX_N], &ctx);
    }

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

/// Generates an SPX key pair.
pub fn crypto_sign_keypair_rs(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes_rs(&mut seed);
    crypto_sign_seed_keypair_rs(pk, sk, &seed);
    0
}

/// Returns an array containing a detached signature.
///
/// Returns `(ret, siglen)`.
pub fn crypto_sign_signature_rs(sig: &mut [u8], m: &[u8], sk: &[u8]) -> (i32, usize) {
    let mut ctx = SpxCtx::new();

    let sk_prf_off = SPX_N;
    let pk_off = 2 * SPX_N;

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = ZERO_ADDR;
    let mut tree_addr = ZERO_ADDR;

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&sk[pk_off..pk_off + SPX_N]);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    /* Optionally, signing can be made non-deterministic using optrand.
       This can help counter side-channel attacks that would benefit from
       getting a large number of traces when the signer uses the same nodes. */
    randombytes_rs(&mut optrand);
    /* Compute the digest randomization value. */
    gen_message_random(
        &mut sig[..],
        &sk[sk_prf_off..sk_prf_off + SPX_N],
        &optrand,
        m,
        m.len() as u64,
        &ctx,
    );

    /* Derive the message digest and leaf index from R, PK and M. */
    {
        let (r, _) = sig.split_at(SPX_N);
        hash_message(
            &mut mhash,
            &mut tree,
            &mut idx_leaf,
            r,
            &sk[pk_off..pk_off + SPX_PK_BYTES],
            m,
            m.len() as u64,
            &ctx,
        );
    }
    let mut sigp = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    /* Sign the message hash using FORS. */
    unsafe {
        fors_sign(
            &mut sig[sigp..sigp + SPX_FORS_BYTES],
            &mut root,
            &mhash,
            &ctx,
            &wots_addr,
        );
    }
    sigp += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        unsafe {
            let end = sigp + SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
            merkle_sign(
                &mut sig[sigp..end],
                &mut root,
                &ctx,
                &wots_addr,
                &mut tree_addr,
                idx_leaf,
            );
        }
        sigp += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree = tree >> SPX_TREE_HEIGHT;
    }

    (0, SPX_BYTES)
}

/// Verifies a detached signature and message under a given public key.
pub fn crypto_sign_verify_rs(sig: &[u8], siglen: usize, m: &[u8], pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root_off = SPX_N;
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = ZERO_ADDR;
    let mut tree_addr = ZERO_ADDR;
    let mut wots_pk_addr = ZERO_ADDR;

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
    /* The additional SPX_N is a result of the hash domain separator. */
    hash_message(
        &mut mhash,
        &mut tree,
        &mut idx_leaf,
        &sig[..SPX_N],
        &pk[..SPX_PK_BYTES],
        m,
        m.len() as u64,
        &ctx,
    );
    let mut sigp = SPX_N;

    /* Layer correctly defaults to 0, so no need to set_layer_addr */
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(
        &mut root,
        &sig[sigp..sigp + SPX_FORS_BYTES],
        &mhash,
        &ctx,
        &wots_addr,
    );
    sigp += SPX_FORS_BYTES;

    /* For each subtree.. */
    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        /* The WOTS public key is only correct if the signature was correct. */
        /* Initially, root is the FORS pk, but on subsequent iterations it is
           the root of the subtree below the currently processed subtree. */
        {
            let r = root;
            wots_pk_from_sig(
                &mut wots_pk,
                &sig[sigp..sigp + SPX_WOTS_BYTES],
                &r,
                &ctx,
                &mut wots_addr,
            );
        }
        sigp += SPX_WOTS_BYTES;

        /* Compute the leaf node using the WOTS public key. */
        thash(
            &mut leaf,
            &wots_pk,
            SPX_WOTS_LEN as u32,
            &ctx,
            &wots_pk_addr,
        );

        /* Compute the root node of this subtree. */
        {
            let l = leaf;
            compute_root(
                &mut root,
                &l,
                idx_leaf,
                0,
                &sig[sigp..sigp + SPX_TREE_HEIGHT * SPX_N],
                SPX_TREE_HEIGHT as u32,
                &ctx,
                &mut tree_addr,
            );
        }
        sigp += SPX_TREE_HEIGHT * SPX_N;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree = tree >> SPX_TREE_HEIGHT;
    }

    /* Check if the root node equals the root node in the public key. */
    if root[..SPX_N] != pk[pub_root_off..pub_root_off + SPX_N] {
        return -1;
    }

    0
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    let seed_s = core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair_rs(pk_s, sk_s, seed_s) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
    let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
    crypto_sign_keypair_rs(pk_s, sk_s) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig_s = core::slice::from_raw_parts_mut(sig, SPX_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen);
    let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    let (ret, len) = crypto_sign_signature_rs(sig_s, m_s, sk_s);
    *siglen = len;
    ret as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    if siglen != SPX_BYTES {
        return -1;
    }
    let sig_s = core::slice::from_raw_parts(sig, siglen);
    let m_s = core::slice::from_raw_parts(m, mlen);
    let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
    crypto_sign_verify_rs(sig_s, siglen, m_s, pk_s) as c_int
}

/// Returns an array containing the signature followed by the message.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    let sig_s = core::slice::from_raw_parts_mut(sm, SPX_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen as usize);
    let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
    let (_ret, siglen) = crypto_sign_signature_rs(sig_s, m_s, sk_s);

    core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
    *smlen = (siglen as u64 + mlen) as c_ulonglong;

    0
}

/// Verifies a given signature-message pair under a given public key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut c_ulonglong,
    sm: *const u8,
    smlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    /* The API caller does not necessarily know what size a signature should be
       but SPHINCS+ signatures are always exactly SPX_BYTES. */
    if (smlen as usize) < SPX_BYTES {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as c_ulonglong;

    let sig_s = core::slice::from_raw_parts(sm, SPX_BYTES);
    let msg_s = core::slice::from_raw_parts(sm.add(SPX_BYTES), *mlen as usize);
    let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
    if crypto_sign_verify_rs(sig_s, SPX_BYTES, msg_s, pk_s) != 0 {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    /* If verification was successful, move the message to the right place. */
    core::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);

    0
}
