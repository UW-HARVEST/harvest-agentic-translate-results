use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::hash::{initialize_hash_function, gen_message_random, hash_message};
use crate::thash::thash;
use crate::fors::{fors_sign, fors_pk_from_sig};
use crate::merkle::{merkle_sign, merkle_gen_root};
use crate::wots::wots_pk_from_sig;
use crate::utils::compute_root;
use crate::rng::randombytes;

pub fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

pub fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

pub fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

pub fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

pub fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::new();

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    merkle_gen_root(&mut sk[3 * SPX_N..4 * SPX_N], &ctx);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

pub fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = vec![0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

pub fn crypto_sign_signature(
    sig: &mut [u8],
    siglen: &mut usize,
    m: &[u8],
    mlen: usize,
    sk: &[u8],
) -> i32 {
    let mut ctx = SpxCtx::new();

    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = vec![0u8; SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N as u64);

    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, mlen as u64, &ctx);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen as u64, &ctx);

    let mut sig_off: usize = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(
            &mut sig[sig_off..],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

pub fn crypto_sign_verify(
    sig: &[u8],
    siglen: usize,
    m: &[u8],
    mlen: usize,
    pk: &[u8],
) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = vec![0u8; SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES {
        return -1;
    }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen as u64, &ctx);
    let mut sig_off: usize = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);

        compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig[sig_off..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }

    0
}

pub fn crypto_sign_fn(
    sm: &mut [u8],
    smlen: &mut u64,
    m: &[u8],
    mlen: u64,
    sk: &[u8],
) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    // memmove: sm + SPX_BYTES <- m
    let mlen_usize = mlen as usize;
    sm.copy_within(..0, 0); // no-op, just for clarity
    // We need to copy m into sm[SPX_BYTES..], but m might overlap with sm
    // Use unsafe memmove equivalent
    unsafe {
        std::ptr::copy(m.as_ptr(), sm[SPX_BYTES..].as_mut_ptr(), mlen_usize);
    }
    *smlen = (siglen as u64) + mlen;
    0
}

pub fn crypto_sign_open(
    m_out: &mut [u8],
    mlen: &mut u64,
    sm: &[u8],
    smlen: u64,
    pk: &[u8],
) -> i32 {
    let smlen_usize = smlen as usize;
    if smlen_usize < SPX_BYTES {
        for i in 0..smlen_usize {
            m_out[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;

    if crypto_sign_verify(&sm[..SPX_BYTES], SPX_BYTES, &sm[SPX_BYTES..SPX_BYTES + *mlen as usize], *mlen as usize, pk) != 0 {
        for i in 0..smlen_usize {
            m_out[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    let ml = *mlen as usize;
    unsafe {
        std::ptr::copy(sm[SPX_BYTES..].as_ptr(), m_out.as_mut_ptr(), ml);
    }

    0
}

// --- extern "C" wrappers ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_secretkeybytes() -> u64 {
    crypto_sign_secretkeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_publickeybytes() -> u64 {
    crypto_sign_publickeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_bytes() -> u64 {
    crypto_sign_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_seedbytes() -> u64 {
    crypto_sign_seedbytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let smlen = unsafe { &mut *smlen };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    crypto_sign_fn(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let m_out = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let mlen = unsafe { &mut *mlen };
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    crypto_sign_open(m_out, mlen, sm, smlen, pk)
}
