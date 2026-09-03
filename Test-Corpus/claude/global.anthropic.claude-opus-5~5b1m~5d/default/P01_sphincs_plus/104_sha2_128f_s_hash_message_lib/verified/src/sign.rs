//! Translation of `app/src/sign.c` (+ `app/include/api.h`).
//!
//! None of the functions defined in `sign.c` are wrapped in `SPX_NAMESPACE()`
//! (only `address.h`, `fors.h`, `hash.h`, `merkle.h`, `thash.h`, `utils.h`,
//! `wots.h` do that for their own symbols), so all exported symbols here keep
//! their plain C names.
//!
//! The `sphincs_core_det` library variant links `src/rng.c` (the NIST
//! AES-256-CTR-DRBG) as the provider of `randombytes()`, therefore every
//! `randombytes()` call below routes through [`crate::rng::randombytes`].

use core::ffi::c_int;

use crate::address::{
    copy_keypair_addr, copy_subtree_addr, set_keypair_addr, set_layer_addr, set_tree_addr,
    set_type, SPX_ADDR_TYPE_HASHTREE, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPK,
};
use crate::backend::{gen_message_random, hash_message, initialize_hash_function, thash};
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES, SPX_BYTES,
    SPX_D, SPX_FORS_BYTES, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
    SPX_WOTS_BYTES, SPX_WOTS_LEN,
};
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;

// ---------------------------------------------------------------------------
// Safe Rust API
// ---------------------------------------------------------------------------

/// Returns the length of a secret key, in bytes
///
/// ```c
/// unsigned long long crypto_sign_secretkeybytes(void)
/// ```
pub fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

/// Returns the length of a public key, in bytes
///
/// ```c
/// unsigned long long crypto_sign_publickeybytes(void)
/// ```
pub fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

/// Returns the length of a signature, in bytes
///
/// ```c
/// unsigned long long crypto_sign_bytes(void)
/// ```
pub fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

/// Returns the length of the seed required to generate a key pair, in bytes
///
/// ```c
/// unsigned long long crypto_sign_seedbytes(void)
/// ```
pub fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

/// Generates an SPX key pair given a seed of length
/// Format sk: `[SK_SEED || SK_PRF || PUB_SEED || root]`
/// Format pk: `[PUB_SEED || root]`
pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    /* Initialize SK_SEED, SK_PRF and PUB_SEED from seed. */
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);

    /* memcpy(pk, sk + 2*SPX_N, SPX_N); -- disjoint buffers in C, but `sk` is
       borrowed mutably here so stage the bytes through a temporary. */
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    pk[..SPX_N].copy_from_slice(&tmp);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    /* This hook allows the hash function instantiation to do whatever
       preparation or computation it needs, based on the public seed. */
    initialize_hash_function(&mut ctx);

    /* Compute root node of the top-most subtree. */
    merkle_gen_root(&mut sk[3 * SPX_N..4 * SPX_N], &ctx);

    tmp.copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&tmp);

    0
}

/// Generates an SPX key pair.
/// Format sk: `[SK_SEED || SK_PRF || PUB_SEED || root]`
/// Format pk: `[PUB_SEED || root]`
pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    crate::rng::randombytes(&mut seed);
    crypto_sign_seed_keypair(pk, sk, &seed);

    0
}

/// Returns an array containing a detached signature.
pub fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    /* const unsigned char *sk_prf = sk + SPX_N;
       const unsigned char *pk     = sk + 2*SPX_N; */
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES];

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

    /* Optionally, signing can be made non-deterministic using optrand.
       This can help counter side-channel attacks that would benefit from
       getting a large number of traces when the signer uses the same nodes. */
    crate::rng::randombytes(&mut optrand);
    /* Compute the digest randomization value. */
    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, &ctx);

    /* Derive the message digest and leaf index from R, PK and M. */
    {
        let mut r = [0u8; SPX_N];
        r.copy_from_slice(&sig[..SPX_N]);
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, &r, pk, m, &ctx);
    }
    /* sig += SPX_N; -- emulated by an explicit running offset. */
    let mut off: usize = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    /* Sign the message hash using FORS. */
    fors_sign(
        &mut sig[off..off + SPX_FORS_BYTES],
        &mut root,
        &mhash,
        &ctx,
        &wots_addr,
    );
    off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let chunk = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
        merkle_sign(
            &mut sig[off..off + chunk],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        off += chunk;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;

    0
}

/// Verifies a detached signature and message under a given public key.
pub fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();
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

    /* const unsigned char *pub_root = pk + SPX_N; */
    let pub_root = &pk[SPX_N..2 * SPX_N];

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
        &ctx,
    );
    /* sig += SPX_N; */
    let mut off: usize = SPX_N;

    /* Layer correctly defaults to 0, so no need to set_layer_addr */
    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(
        &mut root,
        &sig[off..off + SPX_FORS_BYTES],
        &mhash,
        &ctx,
        &wots_addr,
    );
    off += SPX_FORS_BYTES;

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
        wots_pk_from_sig(
            &mut wots_pk,
            &sig[off..off + SPX_WOTS_BYTES],
            &root,
            &ctx,
            &mut wots_addr,
        );
        off += SPX_WOTS_BYTES;

        /* Compute the leaf node using the WOTS public key. */
        thash(
            &mut leaf,
            &wots_pk,
            SPX_WOTS_LEN as u32,
            &ctx,
            &mut wots_pk_addr,
        );

        /* Compute the root node of this subtree. */
        compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig[off..off + SPX_TREE_HEIGHT * SPX_N],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        off += SPX_TREE_HEIGHT * SPX_N;

        /* Update the indices for the next layer. */
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    /* Check if the root node equals the root node in the public key. */
    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }

    0
}

/// Returns an array containing the signature followed by the message.
pub fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], sk: &[u8]) -> i32 {
    let mlen = m.len();
    let mut siglen: usize = 0;

    crypto_sign_signature(sm, &mut siglen, m, sk);

    /* memmove(sm + SPX_BYTES, m, mlen);  -- `m` and `sm` are distinct slices in
       Rust, so a plain copy reproduces the memmove byte-for-byte. */
    sm[SPX_BYTES..SPX_BYTES + mlen].copy_from_slice(&m[..mlen]);
    *smlen = (siglen + mlen) as u64;

    0
}

/// Verifies a given signature-message pair under a given public key.
pub fn crypto_sign_open(m: &mut [u8], mlen: &mut u64, sm: &[u8], pk: &[u8]) -> i32 {
    let smlen = sm.len();

    /* The API caller does not necessarily know what size a signature should be
       but SPHINCS+ signatures are always exactly SPX_BYTES. */
    if smlen < SPX_BYTES {
        /* memset(m, 0, smlen); */
        let n = core::cmp::min(smlen, m.len());
        m[..n].fill(0);
        *mlen = 0;
        return -1;
    }

    *mlen = (smlen - SPX_BYTES) as u64;

    if crypto_sign_verify(
        &sm[..SPX_BYTES],
        SPX_BYTES,
        &sm[SPX_BYTES..SPX_BYTES + *mlen as usize],
        pk,
    ) != 0
    {
        /* memset(m, 0, smlen); */
        let n = core::cmp::min(smlen, m.len());
        m[..n].fill(0);
        *mlen = 0;
        return -1;
    }

    /* If verification was successful, move the message to the right place. */
    let n = *mlen as usize;
    m[..n].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + n]);

    0
}

// ---------------------------------------------------------------------------
// C ABI wrappers -- exact signatures from app/include/api.h, plain symbol names
// ---------------------------------------------------------------------------

#[unsafe(export_name = "crypto_sign_secretkeybytes")]
pub extern "C" fn crypto_sign_secretkeybytes_c() -> u64 {
    crypto_sign_secretkeybytes()
}

#[unsafe(export_name = "crypto_sign_publickeybytes")]
pub extern "C" fn crypto_sign_publickeybytes_c() -> u64 {
    crypto_sign_publickeybytes()
}

#[unsafe(export_name = "crypto_sign_bytes")]
pub extern "C" fn crypto_sign_bytes_c() -> u64 {
    crypto_sign_bytes()
}

#[unsafe(export_name = "crypto_sign_seedbytes")]
pub extern "C" fn crypto_sign_seedbytes_c() -> u64 {
    crypto_sign_seedbytes()
}

#[unsafe(export_name = "crypto_sign_seed_keypair")]
pub extern "C" fn crypto_sign_seed_keypair_c(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe {
        let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        let seed_s = core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        crypto_sign_seed_keypair(pk_s, sk_s, seed_s) as c_int
    }
}

#[unsafe(export_name = "crypto_sign_keypair")]
pub extern "C" fn crypto_sign_keypair_c(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe {
        let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        crypto_sign_keypair(pk_s, sk_s) as c_int
    }
}

#[unsafe(export_name = "crypto_sign_signature")]
pub extern "C" fn crypto_sign_signature_c(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = core::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = core::slice::from_raw_parts(m, mlen);
        let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        let mut len: usize = 0;
        let ret = crypto_sign_signature(sig_s, &mut len, m_s, sk_s);
        *siglen = len;
        ret as c_int
    }
}

#[unsafe(export_name = "crypto_sign_verify")]
pub extern "C" fn crypto_sign_verify_c(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    /* The C code bails out before touching `sig` when the length is wrong. */
    if siglen != SPX_BYTES {
        return -1;
    }
    unsafe {
        let sig_s = core::slice::from_raw_parts(sig, SPX_BYTES);
        let m_s = core::slice::from_raw_parts(m, mlen);
        let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        crypto_sign_verify(sig_s, siglen, m_s, pk_s) as c_int
    }
}

#[unsafe(export_name = "crypto_sign")]
pub extern "C" fn crypto_sign_c(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    unsafe {
        /* The C is
         *     crypto_sign_signature(sm, &siglen, m, mlen, sk);
         *     memmove(sm + SPX_BYTES, m, mlen);
         *     *smlen = siglen + mlen;
         * `memmove`, not `memcpy`: `m` is allowed to overlap `sm` (the usual
         * in-place `crypto_sign(sm, &len, sm + SPX_BYTES, mlen, sk)` idiom), so
         * the move is done here with `ptr::copy` (memmove semantics) rather than
         * `copy_from_slice`, which would be a non-overlapping copy. */
        let sig_s = core::slice::from_raw_parts_mut(sm, SPX_BYTES);
        let m_s = core::slice::from_raw_parts(m, mlen as usize);
        let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        let mut siglen: usize = 0;
        let ret = crypto_sign_signature(sig_s, &mut siglen, m_s, sk_s);
        core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
        ret as c_int
    }
}

#[unsafe(export_name = "crypto_sign_open")]
pub extern "C" fn crypto_sign_open_c(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    unsafe {
        /* Transcribed straight from `sign.c:263-287` with raw pointers, because
         * the final step is a `memmove(m, sm + SPX_BYTES, *mlen)` and `m` is
         * allowed to overlap `sm`.  Note also that BOTH failure paths memset
         * `smlen` bytes of `m` -- not `smlen - SPX_BYTES`. */
        let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);

        if smlen < SPX_BYTES as u64 {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        let sig_s = core::slice::from_raw_parts(sm, SPX_BYTES);
        let msg_s = core::slice::from_raw_parts(sm.add(SPX_BYTES), *mlen as usize);
        if crypto_sign_verify(sig_s, SPX_BYTES, msg_s, pk_s) != 0 {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        /* If verification was successful, move the message to the right place. */
        core::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);

        0
    }
}
