#![allow(non_snake_case, unused_unsafe, unused_variables, unused_assignments, static_mut_refs, dead_code)]

mod params;
mod context;
mod sha2;
mod address;
mod utils;
mod hash;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod rng;

use params::*;
use context::SpxCtx;

// ---- Public C API ----

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
) -> i32 {
    unsafe {
        let pk = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let seed = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        let mut ctx = SpxCtx::default();

        sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
        pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

        hash::initialize_hash_function(&mut ctx);
        merkle::merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
        pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    rng::randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sk = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        let m = std::slice::from_raw_parts(m, mlen);
        let sig_slice = std::slice::from_raw_parts_mut(sig, SPX_BYTES);

        let mut ctx = SpxCtx::default();
        let sk_prf = &sk[SPX_N..2 * SPX_N];
        let pk = &sk[2 * SPX_N..];

        ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

        hash::initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        rng::randombytes(&mut optrand, SPX_N as u64);

        hash::gen_message_random(sig_slice, sk_prf, &optrand, m, mlen as u64, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut root = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;

        hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk, m, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        address::set_tree_addr(&mut wots_addr, tree);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        fors::fors_sign(&mut sig_slice[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            address::set_layer_addr(&mut tree_addr, i);
            address::set_tree_addr(&mut tree_addr, tree);

            address::copy_subtree_addr(&mut wots_addr, &tree_addr);
            address::set_keypair_addr(&mut wots_addr, idx_leaf);

            merkle::merkle_sign(
                &mut sig_slice[sig_off..],
                &mut root,
                &ctx,
                &mut wots_addr,
                &mut tree_addr,
                idx_leaf,
            );
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT as usize * SPX_N;

            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
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
) -> i32 {
    unsafe {
        let pk = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m = std::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES {
            return -1;
        }

        let sig_all = std::slice::from_raw_parts(sig, SPX_BYTES);

        let mut ctx = SpxCtx::default();
        let pub_root = &pk[SPX_N..2 * SPX_N];

        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        hash::initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];

        address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        address::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;

        hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_all, pk, m, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        address::set_tree_addr(&mut wots_addr, tree);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors::fors_pk_from_sig(&mut root, &sig_all[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut leaf = [0u8; SPX_N];

        for i in 0..SPX_D as usize {
            address::set_layer_addr(&mut tree_addr, i as u32);
            address::set_tree_addr(&mut tree_addr, tree);

            address::copy_subtree_addr(&mut wots_addr, &tree_addr);
            address::set_keypair_addr(&mut wots_addr, idx_leaf);

            address::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            wots::wots_pk_from_sig(&mut wots_pk, &sig_all[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            thash::thash(&mut leaf, &wots_pk, SPX_WOTS_LEN as usize, &ctx, &wots_pk_addr);

            utils::compute_root(
                &mut root,
                &leaf,
                idx_leaf,
                0,
                &sig_all[sig_off..],
                SPX_TREE_HEIGHT,
                &ctx,
                &mut tree_addr,
            );
            sig_off += SPX_TREE_HEIGHT as usize * SPX_N;

            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root[..SPX_N] != pub_root[..SPX_N] {
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
) -> i32 {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

        // memmove sm + SPX_BYTES <- m
        let m_slice = std::slice::from_raw_parts(m, mlen as usize);
        let sm_dest = std::slice::from_raw_parts_mut(sm.add(SPX_BYTES), mlen as usize);
        // Use copy_within-like logic for potential overlap
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
) -> i32 {
    unsafe {
        if smlen < SPX_BYTES as u64 {
            // memset m to 0
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        // memmove
        std::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}

// RNG exports
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
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
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    unsafe {
        let buf = std::slice::from_raw_parts_mut(x, xlen as usize);
        rng::randombytes(buf, xlen);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    unsafe {
        let key_slice = std::slice::from_raw_parts_mut(key, 32);
        let v_slice = std::slice::from_raw_parts_mut(v, 16);
        let mut key_arr = [0u8; 32];
        let mut v_arr = [0u8; 16];
        key_arr.copy_from_slice(key_slice);
        v_arr.copy_from_slice(v_slice);

        let pd = if provided_data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(provided_data, 48) as &[u8])
        };

        // Inline the update logic to match C exactly
        let mut temp = [0u8; 48];
        for i in 0..3 {
            for j in (0..16).rev() {
                if v_arr[j] == 0xff {
                    v_arr[j] = 0x00;
                } else {
                    v_arr[j] += 1;
                    break;
                }
            }
            rng_aes256_ecb(&key_arr, &v_arr, &mut temp[16 * i..]);
        }
        if let Some(data) = pd {
            for i in 0..48 {
                temp[i] ^= data[i];
            }
        }
        key_slice.copy_from_slice(&temp[..32]);
        v_slice.copy_from_slice(&temp[32..48]);
    }
}

// Helper for AES256_CTR_DRBG_Update export
fn rng_aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    unsafe {
        let ctx = EVP_CIPHER_CTX_new();
        if ctx.is_null() { std::process::abort(); }
        if EVP_EncryptInit_ex(ctx, EVP_aes_256_ecb(), std::ptr::null(), key.as_ptr(), std::ptr::null()) != 1 {
            std::process::abort();
        }
        let mut len: i32 = 0;
        if EVP_EncryptUpdate(ctx, buffer.as_mut_ptr(), &mut len, ctr.as_ptr(), 16) != 1 {
            std::process::abort();
        }
        EVP_CIPHER_CTX_free(ctx);
    }
}

extern "C" {
    fn EVP_CIPHER_CTX_new() -> *mut std::ffi::c_void;
    fn EVP_CIPHER_CTX_free(ctx: *mut std::ffi::c_void);
    fn EVP_aes_256_ecb() -> *const std::ffi::c_void;
    fn EVP_EncryptInit_ex(
        ctx: *mut std::ffi::c_void,
        cipher: *const std::ffi::c_void,
        engine: *const std::ffi::c_void,
        key: *const u8,
        iv: *const u8,
    ) -> i32;
    fn EVP_EncryptUpdate(
        ctx: *mut std::ffi::c_void,
        out: *mut u8,
        outl: *mut i32,
        inp: *const u8,
        inl: i32,
    ) -> i32;
}

// seedexpander exports
#[repr(C)]
pub struct AES_XOF_struct {
    buffer: [u8; 16],
    buffer_pos: u64,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    unsafe {
        let ctx = &mut *ctx;
        if maxlen >= 0x100000000 {
            return -1; // RNG_BAD_MAXLEN
        }
        ctx.length_remaining = maxlen;
        ctx.key.copy_from_slice(std::slice::from_raw_parts(seed, 32));
        ctx.ctr[..8].copy_from_slice(std::slice::from_raw_parts(diversifier, 8));
        let mut ml = maxlen;
        ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[8] = (ml % 256) as u8;
        ctx.ctr[12..16].fill(0);
        ctx.buffer_pos = 16;
        ctx.buffer.fill(0);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut AES_XOF_struct,
    x: *mut u8,
    xlen: u64,
) -> i32 {
    unsafe {
        let ctx = &mut *ctx;
        if x.is_null() { return -2; } // RNG_BAD_OUTBUF
        let mut xlen = xlen as u64;
        if xlen >= ctx.length_remaining { return -3; } // RNG_BAD_REQ_LEN

        ctx.length_remaining -= xlen;
        let mut offset: u64 = 0;

        while xlen > 0 {
            let avail = 16 - ctx.buffer_pos;
            if xlen <= avail {
                std::ptr::copy_nonoverlapping(
                    ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                    x.add(offset as usize),
                    xlen as usize,
                );
                ctx.buffer_pos += xlen;
                return 0;
            }

            std::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset as usize),
                avail as usize,
            );
            xlen -= avail;
            offset += avail;

            rng_aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
            ctx.buffer_pos = 0;

            // increment counter
            for i in (12..=15).rev() {
                if ctx.ctr[i] == 0xff {
                    ctx.ctr[i] = 0x00;
                } else {
                    ctx.ctr[i] += 1;
                    break;
                }
            }
        }
    }
    0
}
