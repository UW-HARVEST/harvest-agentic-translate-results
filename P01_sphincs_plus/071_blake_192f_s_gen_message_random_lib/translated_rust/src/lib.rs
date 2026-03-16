#![allow(clippy::all)]
#![allow(non_snake_case)]

mod params;
mod context;
mod address;
mod blake;
mod hash_blake;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod utils;
mod utilsx1;
mod merkle;
mod rng;

use params::*;
use context::SpxCtx;
use address::*;
use hash_blake::*;
use fors::*;
use merkle::*;
use wots::wots_pk_from_sig;
use thash::thash;
use utils::compute_root;

// ---- Public C API ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> libc::c_ulonglong {
    CRYPTO_SECRETKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> libc::c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> libc::c_ulonglong {
    CRYPTO_BYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> libc::c_ulonglong {
    CRYPTO_SEEDBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut libc::c_uchar,
    sk: *mut libc::c_uchar,
    seed: *const libc::c_uchar,
) -> libc::c_int {
    unsafe {
        let pk = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let seed = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
        pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

        initialize_hash_function(&mut ctx);
        merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
        pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(
    pk: *mut libc::c_uchar,
    sk: *mut libc::c_uchar,
) -> libc::c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut libc::size_t,
    m: *const u8,
    mlen: libc::size_t,
    sk: *const u8,
) -> libc::c_int {
    unsafe {
        let sig_slice = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_slice = std::slice::from_raw_parts(m, mlen);
        let sk_slice = std::slice::from_raw_parts(sk, SPX_SK_BYTES);

        let sk_prf = &sk_slice[SPX_N..2 * SPX_N];
        let pk = &sk_slice[2 * SPX_N..];

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.sk_seed.copy_from_slice(&sk_slice[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        rng::randombytes(&mut optrand, SPX_N as u64);

        gen_message_random(sig_slice, sk_prf, &optrand, m_slice, mlen as u64, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk, m_slice, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_sign(&mut sig_slice[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            merkle_sign(&mut sig_slice[sig_off..], &mut root, &ctx, &wots_addr, &mut tree_addr, idx_leaf);
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
    sig: *const u8,
    siglen: libc::size_t,
    m: *const u8,
    mlen: libc::size_t,
    pk: *const u8,
) -> libc::c_int {
    unsafe {
        let pk_slice = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_slice = std::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES { return -1; }
        let sig_slice = std::slice::from_raw_parts(sig, SPX_BYTES);

        let pub_root = &pk_slice[SPX_N..];
        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);
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
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk_slice, m_slice, mlen as u64, &ctx);

        let mut sig_off = SPX_N;
        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig(&mut root, &sig_slice[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut leaf = [0u8; SPX_N];

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            wots_pk_from_sig(&mut wots_pk, &sig_slice[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);
            compute_root(&mut root, &leaf, idx_leaf, 0, &sig_slice[sig_off..],
                         SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut libc::c_uchar,
    smlen: *mut libc::c_ulonglong,
    m: *const libc::c_uchar,
    mlen: libc::c_ulonglong,
    sk: *const libc::c_uchar,
) -> libc::c_int {
    unsafe {
        let mut siglen: libc::size_t = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as libc::size_t, sk);
        // memmove sm + SPX_BYTES <- m
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as libc::c_ulonglong) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut libc::c_uchar,
    mlen: *mut libc::c_ulonglong,
    sm: *const libc::c_uchar,
    smlen: libc::c_ulonglong,
    pk: *const libc::c_uchar,
) -> libc::c_int {
    unsafe {
        if (smlen as usize) < SPX_BYTES {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as libc::c_ulonglong;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as libc::size_t, pk) != 0 {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}

// RNG exports
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut libc::c_uchar,
    personalization_string: *mut libc::c_uchar,
) {
    unsafe {
        let ei = std::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(personalization_string, 48))
        };
        rng::randombytes_init(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut libc::c_uchar, xlen: libc::c_ulonglong) -> libc::c_int {
    unsafe {
        let buf = std::slice::from_raw_parts_mut(x, xlen as usize);
        rng::randombytes(buf, xlen)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut libc::c_uchar,
    key: *mut libc::c_uchar,
    v: *mut libc::c_uchar,
) {
    unsafe {
        let key_slice = std::slice::from_raw_parts_mut(key, 32);
        let v_slice = std::slice::from_raw_parts_mut(v, 16);
        let mut k = [0u8; 32];
        let mut vi = [0u8; 16];
        k.copy_from_slice(key_slice);
        vi.copy_from_slice(v_slice);
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(provided_data, 48) as &[u8])
        };
        rng::aes256_ctr_drbg_update(pd, &mut k, &mut vi);
        key_slice.copy_from_slice(&k);
        v_slice.copy_from_slice(&vi);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    _ctx: *mut libc::c_void,
    _seed: *mut libc::c_uchar,
    _diversifier: *mut libc::c_uchar,
    _maxlen: libc::c_ulong,
) -> libc::c_int {
    // Stub - not used in the main signing flow
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    _ctx: *mut libc::c_void,
    _x: *mut libc::c_uchar,
    _xlen: libc::c_ulong,
) -> libc::c_int {
    // Stub - not used in the main signing flow
    0
}
