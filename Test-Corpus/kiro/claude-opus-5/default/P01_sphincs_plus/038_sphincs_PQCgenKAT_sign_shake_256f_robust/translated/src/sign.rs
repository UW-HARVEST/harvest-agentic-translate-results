use core::ffi::{c_int, c_ulonglong};

use crate::context::SpxCtx;
use crate::params::*;

/*
 * Returns the length of a secret key, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    CRYPTO_SECRETKEYBYTES as c_ulonglong
}

/*
 * Returns the length of a public key, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as c_ulonglong
}

/*
 * Returns the length of a signature, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    CRYPTO_BYTES as c_ulonglong
}

/*
 * Returns the length of the seed required to generate a key pair, in bytes
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    CRYPTO_SEEDBYTES as c_ulonglong
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
) -> c_int {
    let mut ctx = SpxCtx::new();

    /* Initialize SK_SEED, SK_PRF and PUB_SEED from seed. */
    core::ptr::copy_nonoverlapping(seed, sk, CRYPTO_SEEDBYTES);

    core::ptr::copy_nonoverlapping(sk.add(2 * SPX_N), pk, SPX_N);

    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    crate::hash::SPX_initialize_hash_function(&mut ctx);

    /* Compute root node of the top-most subtree. */
    crate::merkle::SPX_merkle_gen_root(sk.add(3 * SPX_N), &ctx);

    core::ptr::copy_nonoverlapping(sk.add(3 * SPX_N), pk.add(SPX_N), SPX_N);

    0
}

/*
 * Generates an SPX key pair.
 * Format sk: [SK_SEED || SK_PRF || PUB_SEED || root]
 * Format pk: [PUB_SEED || root]
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed: [u8; CRYPTO_SEEDBYTES] = [0u8; CRYPTO_SEEDBYTES];
    let _ = crate::rng::randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as c_ulonglong);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());

    0
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
) -> c_int {
    let mut ctx = SpxCtx::new();

    let sk_prf: *const u8 = sk.add(SPX_N);
    let pk: *const u8 = sk.add(2 * SPX_N);

    let mut optrand: [u8; SPX_N] = [0u8; SPX_N];
    let mut mhash: [u8; SPX_FORS_MSG_BYTES] = [0u8; SPX_FORS_MSG_BYTES];
    let mut root: [u8; SPX_N] = [0u8; SPX_N];
    let mut i: u32;
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr: [u32; 8] = [0u32; 8];
    let mut tree_addr: [u32; 8] = [0u32; 8];

    let mut sig = sig;

    core::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    crate::hash::SPX_initialize_hash_function(&mut ctx);

    crate::address::SPX_set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
    crate::address::SPX_set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);

    /* Optionally, signing can be made non-deterministic using optrand.
       This can help counter side-channel attacks that would benefit from
       getting a large number of traces when the signer uses the same nodes. */
    let _ = crate::rng::randombytes(optrand.as_mut_ptr(), SPX_N as c_ulonglong);
    /* Compute the digest randomization value. */
    crate::hash::SPX_gen_message_random(
        sig,
        sk_prf,
        optrand.as_ptr(),
        m,
        mlen as c_ulonglong,
        &ctx,
    );

    /* Derive the message digest and leaf index from R, PK and M. */
    crate::hash::SPX_hash_message(
        mhash.as_mut_ptr(),
        &mut tree,
        &mut idx_leaf,
        sig,
        pk,
        m,
        mlen as c_ulonglong,
        &ctx,
    );
    sig = sig.add(SPX_N);

    crate::address::SPX_set_tree_addr(wots_addr.as_mut_ptr(), tree);
    crate::address::SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

    /* Sign the message hash using FORS. */
    crate::fors::SPX_fors_sign(sig, root.as_mut_ptr(), mhash.as_ptr(), &ctx, wots_addr.as_ptr());
    sig = sig.add(SPX_FORS_BYTES);

    i = 0;
    while i < SPX_D {
        crate::address::SPX_set_layer_addr(tree_addr.as_mut_ptr(), i);
        crate::address::SPX_set_tree_addr(tree_addr.as_mut_ptr(), tree);

        crate::address::SPX_copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
        crate::address::SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        crate::merkle::SPX_merkle_sign(
            sig,
            root.as_mut_ptr(),
            &ctx,
            wots_addr.as_mut_ptr(),
            tree_addr.as_mut_ptr(),
            idx_leaf,
        );
        sig = sig.add(SPX_WOTS_BYTES + SPX_TREE_HEIGHT as usize * SPX_N);

        /* Update the indices for the next layer. */
        idx_leaf = (tree & (((1i32 << SPX_TREE_HEIGHT) - 1) as u64)) as u32;
        tree = tree >> SPX_TREE_HEIGHT;

        i += 1;
    }

    *siglen = SPX_BYTES;

    0
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
) -> c_int {
    let mut ctx = SpxCtx::new();
    let pub_root: *const u8 = pk.add(SPX_N);
    let mut mhash: [u8; SPX_FORS_MSG_BYTES] = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk: [u8; SPX_WOTS_BYTES] = [0u8; SPX_WOTS_BYTES];
    let mut root: [u8; SPX_N] = [0u8; SPX_N];
    let mut leaf: [u8; SPX_N] = [0u8; SPX_N];
    let mut i: core::ffi::c_uint;
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr: [u32; 8] = [0u32; 8];
    let mut tree_addr: [u32; 8] = [0u32; 8];
    let mut wots_pk_addr: [u32; 8] = [0u32; 8];

    let mut sig = sig;

    if siglen != SPX_BYTES {
        return -1;
    }

    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    crate::hash::SPX_initialize_hash_function(&mut ctx);

    crate::address::SPX_set_type(wots_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTS);
    crate::address::SPX_set_type(tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_HASHTREE);
    crate::address::SPX_set_type(wots_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_WOTSPK);

    /* Derive the message digest and leaf index from R || PK || M. */
    /* The additional SPX_N is a result of the hash domain separator. */
    crate::hash::SPX_hash_message(
        mhash.as_mut_ptr(),
        &mut tree,
        &mut idx_leaf,
        sig,
        pk,
        m,
        mlen as c_ulonglong,
        &ctx,
    );
    sig = sig.add(SPX_N);

    /* Layer correctly defaults to 0, so no need to set_layer_addr */
    crate::address::SPX_set_tree_addr(wots_addr.as_mut_ptr(), tree);
    crate::address::SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

    crate::fors::SPX_fors_pk_from_sig(root.as_mut_ptr(), sig, mhash.as_ptr(), &ctx, wots_addr.as_ptr());
    sig = sig.add(SPX_FORS_BYTES);

    /* For each subtree.. */
    i = 0;
    while i < SPX_D {
        crate::address::SPX_set_layer_addr(tree_addr.as_mut_ptr(), i);
        crate::address::SPX_set_tree_addr(tree_addr.as_mut_ptr(), tree);

        crate::address::SPX_copy_subtree_addr(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
        crate::address::SPX_set_keypair_addr(wots_addr.as_mut_ptr(), idx_leaf);

        crate::address::SPX_copy_keypair_addr(wots_pk_addr.as_mut_ptr(), wots_addr.as_ptr());

        /* The WOTS public key is only correct if the signature was correct. */
        /* Initially, root is the FORS pk, but on subsequent iterations it is
           the root of the subtree below the currently processed subtree. */
        crate::wots::SPX_wots_pk_from_sig(
            wots_pk.as_mut_ptr(),
            sig,
            root.as_ptr(),
            &ctx,
            wots_addr.as_mut_ptr(),
        );
        sig = sig.add(SPX_WOTS_BYTES);

        /* Compute the leaf node using the WOTS public key. */
        crate::hash::SPX_thash(
            leaf.as_mut_ptr(),
            wots_pk.as_ptr(),
            SPX_WOTS_LEN as core::ffi::c_uint,
            &ctx,
            wots_pk_addr.as_mut_ptr(),
        );

        /* Compute the root node of this subtree. */
        crate::utils::SPX_compute_root(
            root.as_mut_ptr(),
            leaf.as_ptr(),
            idx_leaf,
            0,
            sig,
            SPX_TREE_HEIGHT,
            &ctx,
            tree_addr.as_mut_ptr(),
        );
        sig = sig.add(SPX_TREE_HEIGHT as usize * SPX_N);

        /* Update the indices for the next layer. */
        idx_leaf = (tree & (((1i32 << SPX_TREE_HEIGHT) - 1) as u64)) as u32;
        tree = tree >> SPX_TREE_HEIGHT;

        i += 1;
    }

    /* Check if the root node equals the root node in the public key. */
    if libc_memcmp(root.as_ptr(), pub_root, SPX_N) != 0 {
        return -1;
    }

    0
}

#[inline]
unsafe fn libc_memcmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    let sa = core::slice::from_raw_parts(a, n);
    let sb = core::slice::from_raw_parts(b, n);
    let mut i = 0usize;
    while i < n {
        if sa[i] != sb[i] {
            return (sa[i] as c_int) - (sb[i] as c_int);
        }
        i += 1;
    }
    0
}

/**
 * Returns an array containing the signature followed by the message.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    let mut siglen: usize = 0;

    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
    *smlen = siglen as c_ulonglong + mlen;

    0
}

/**
 * Verifies a given signature-message pair under a given public key.
 */
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
    if smlen < SPX_BYTES as c_ulonglong {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as c_ulonglong;

    if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    /* If verification was successful, move the message to the right place. */
    core::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);

    0
}
