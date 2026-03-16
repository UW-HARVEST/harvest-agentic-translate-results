use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{fors_pk_from_sig, fors_sign};
use crate::hash_blake::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{merkle_gen_root, merkle_sign};
use crate::params::*;
use crate::rng::{randombytes, randombytes_init};
use crate::thash_blake_robust::thash;
use crate::utils::compute_root;
use crate::wots::wots_pk_from_sig;
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
        let pk = core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk = core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let seed = core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        let mut ctx = SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
        };

        sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
        pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

        initialize_hash_function(&mut ctx);

        merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);

        pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
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
        let sig_slice = core::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_slice = core::slice::from_raw_parts(m, mlen);
        let sk_slice = core::slice::from_raw_parts(sk, SPX_SK_BYTES);

        let mut ctx = SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
        };

        let sk_prf = &sk_slice[SPX_N..2 * SPX_N];
        let pk_part = &sk_slice[2 * SPX_N..];

        ctx.sk_seed.copy_from_slice(&sk_slice[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk_part[..SPX_N]);

        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        randombytes(&mut optrand, SPX_N as u64);

        gen_message_random(sig_slice, sk_prf, &optrand, m_slice, mlen as u64, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(
            &mut mhash,
            &mut tree,
            &mut idx_leaf,
            sig_slice,
            pk_part,
            m_slice,
            mlen as u64,
            &ctx,
        );

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_sign(&mut sig_slice[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D as u32 {
            set_layer_addr(&mut tree_addr, i);
            set_tree_addr(&mut tree_addr, tree);

            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            merkle_sign(
                &mut sig_slice[sig_off..],
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
        let pk_slice = core::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_slice = core::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES {
            return -1;
        }
        let sig_slice = core::slice::from_raw_parts(sig, SPX_BYTES);

        let pub_root = &pk_slice[SPX_N..];

        let mut ctx = SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
        };
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
        hash_message(
            &mut mhash,
            &mut tree,
            &mut idx_leaf,
            sig_slice,
            pk_slice,
            m_slice,
            mlen as u64,
            &ctx,
        );

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig(&mut root, &sig_slice[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for _i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, _i as u32);
            set_tree_addr(&mut tree_addr, tree);

            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            let mut wots_pk = [0u8; SPX_WOTS_BYTES];
            wots_pk_from_sig(&mut wots_pk, &sig_slice[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            let mut leaf = [0u8; SPX_N];
            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

            compute_root(
                &mut root,
                &leaf,
                idx_leaf,
                0,
                &sig_slice[sig_off..],
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

        let m_slice = core::slice::from_raw_parts(m, mlen as usize);
        let sm_slice = core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize);
        // memmove: copy m after signature
        sm_slice.copy_within(0..0, 0); // no-op placeholder
        sm_slice[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m_slice);
        *smlen = (siglen as u64) + mlen;

        0
    }
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
        let sm_slice = core::slice::from_raw_parts(sm, smlen as usize);
        let m_slice = core::slice::from_raw_parts_mut(m, smlen as usize);

        if (smlen as usize) < SPX_BYTES {
            for i in 0..smlen as usize {
                m_slice[i] = 0;
            }
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            for i in 0..smlen as usize {
                m_slice[i] = 0;
            }
            *mlen = 0;
            return -1;
        }

        // memmove
        let msg_len = *mlen as usize;
        core::ptr::copy(sm.add(SPX_BYTES), m, msg_len);

        0
    }
}

// Also export randombytes_init and randombytes for the deterministic test driver
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init_ffi(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    unsafe {
        let ei = core::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(core::slice::from_raw_parts(personalization_string, 48))
        };
        randombytes_init(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_ffi(x: *mut u8, xlen: u64) -> c_int {
    unsafe {
        let buf = core::slice::from_raw_parts_mut(x, xlen as usize);
        randombytes(buf, xlen)
    }
}
