use crate::params::*;
use crate::sign;
use crate::context::SpxCtx;
use std::ffi::{c_int, c_ulong, c_void};

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
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk, None)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    message: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, CRYPTO_BYTES) };
    let message = unsafe { std::slice::from_raw_parts(message, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature(sig, message, sk, None);
    unsafe { *siglen = CRYPTO_BYTES };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    message: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let message = unsafe { std::slice::from_raw_parts(message, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    if sign::crypto_sign_verify(sig, message, pk).is_ok() { 0 } else { -1 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    message: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let mlen = mlen as usize;
    let message = unsafe { std::slice::from_raw_parts(message, mlen) }.to_vec();
    let sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, CRYPTO_BYTES + mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature(sm_slice, &message, sk, None);
    sm_slice[CRYPTO_BYTES..].copy_from_slice(&message);
    unsafe { *smlen = (CRYPTO_BYTES + mlen) as u64 };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    message: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let smlen = smlen as usize;
    if smlen < CRYPTO_BYTES {
        unsafe {
            std::ptr::write_bytes(message, 0, smlen);
            *mlen = 0;
        }
        return -1;
    }
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    let message_len = smlen - CRYPTO_BYTES;
    if sign::crypto_sign_verify(&sm[..CRYPTO_BYTES], &sm[CRYPTO_BYTES..], pk).is_err() {
        unsafe {
            std::ptr::write_bytes(message, 0, smlen);
            *mlen = 0;
        }
        return -1;
    }
    unsafe {
        std::ptr::copy(sm.as_ptr().add(CRYPTO_BYTES), message, message_len);
        *mlen = message_len as u64;
    }
    0
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
) -> c_int {
    let input = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    let digest = crate::blake::blake256(input);
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
    0
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
) -> c_int {
    let input = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    let digest = crate::blake::blake512(input);
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(out: *mut u8, outlen: u64) -> c_int {
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    crate::randombytes::randombytes(out, out.len());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy: *mut u8,
    personalization: *mut u8,
) {
    let entropy = unsafe { &*(entropy as *const [u8; 48]) };
    let personalization = if personalization.is_null() {
        None
    } else {
        Some(unsafe { &*(personalization as *const [u8; 48]) })
    };
    crate::randombytes::randombytes_init(entropy, personalization);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, out: *mut u8) {
    let key = unsafe { &*(key as *const [u8; 32]) };
    let ctr = unsafe { &*(ctr as *const [u8; 16]) };
    let out = unsafe { &mut *(out as *mut [u8; 16]) };
    crate::randombytes::aes256_ecb(key, ctr, out);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let provided = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { &*(provided_data as *const [u8; 48]) })
    };
    let key = unsafe { &mut *(key as *mut [u8; 32]) };
    let v = unsafe { &mut *(v as *mut [u8; 16]) };
    crate::randombytes::aes256_ctr_drbg_update(provided, key, v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut crate::randombytes::AesXof,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: c_ulong,
) -> c_int {
    crate::randombytes::seedexpander_init(
        unsafe { &mut *ctx },
        unsafe { &*(seed as *const [u8; 32]) },
        unsafe { &*(diversifier as *const [u8; 8]) },
        maxlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut crate::randombytes::AesXof,
    out: *mut u8,
    outlen: c_ulong,
) -> c_int {
    if out.is_null() {
        return crate::randombytes::RNG_BAD_OUTBUF;
    }
    crate::randombytes::seedexpander(
        unsafe { &mut *ctx },
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        outlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, input: u64) {
    crate::utils::ull_to_bytes(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        outlen as usize,
        input,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    crate::utils::u32_to_bytes(
        unsafe { std::slice::from_raw_parts_mut(out, 4) },
        input,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: u32) -> u64 {
    crate::utils::bytes_to_ull(
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
        inlen as usize,
    )
}

macro_rules! address_setter {
    ($name:ident, $inner:path, $value:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(addr: *mut u32, value: $value) {
            $inner(unsafe { std::slice::from_raw_parts_mut(addr, 8) }, value);
        }
    };
}

address_setter!(SPX_set_layer_addr, crate::address::set_layer_addr, u32);
address_setter!(SPX_set_tree_addr, crate::address::set_tree_addr, u64);
address_setter!(SPX_set_type, crate::address::set_type, u32);
address_setter!(SPX_set_keypair_addr, crate::address::set_keypair_addr, u32);
address_setter!(SPX_set_chain_addr, crate::address::set_chain_addr, u32);
address_setter!(SPX_set_hash_addr, crate::address::set_hash_addr, u32);
address_setter!(SPX_set_tree_height, crate::address::set_tree_height, u32);
address_setter!(SPX_set_tree_index, crate::address::set_tree_index, u32);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    crate::address::copy_subtree_addr(
        unsafe { std::slice::from_raw_parts_mut(out, 8) },
        unsafe { std::slice::from_raw_parts_mut(input as *mut u32, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    crate::address::copy_keypair_addr(
        unsafe { std::slice::from_raw_parts_mut(out, 8) },
        unsafe { std::slice::from_raw_parts(input, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    crate::hash::initialize_hash_function(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    crate::hash::prf_addr(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts_mut(addr as *mut u32, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    out: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    message: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    crate::hash::gen_message_random(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        unsafe { std::slice::from_raw_parts(sk_prf, SPX_N) },
        unsafe { std::slice::from_raw_parts(optrand, SPX_N) },
        unsafe { std::slice::from_raw_parts(message, mlen as usize) },
        mlen as usize,
        unsafe { &*ctx },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    message: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    crate::hash::hash_message(
        unsafe { std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) },
        unsafe { &mut *tree },
        unsafe { &mut *leaf_idx },
        unsafe { std::slice::from_raw_parts(r, SPX_N) },
        unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) },
        unsafe { std::slice::from_raw_parts(message, mlen as usize) },
        mlen as usize,
        unsafe { &*ctx },
    );
}

fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    blocks: usize,
    ctx: &SpxCtx,
    addr: &[u32],
) {
    if blocks == 1 {
        crate::thash::thash::<1>(out, Some(input), ctx, addr);
    } else if blocks == 2 {
        crate::thash::thash::<2>(out, Some(input), ctx, addr);
    } else if blocks == SPX_WOTS_LEN {
        crate::thash::thash::<SPX_WOTS_LEN>(out, Some(input), ctx, addr);
    } else if blocks == SPX_FORS_TREES {
        crate::thash::thash::<SPX_FORS_TREES>(out, Some(input), ctx, addr);
    } else {
        panic!("unsupported thash block count {blocks}");
    }
}

fn thash_dynamic(
    out: &mut [u8],
    input: &[u8],
    blocks: usize,
    ctx: &SpxCtx,
    addr: &[u32],
) {
    let input_len = blocks * SPX_N;
    let addr_bytes = crate::utils::address_to_bytes(addr);

    #[cfg(feature = "haraka")]
    {
        if blocks == 1 {
            let mut outbuf = [0u8; 32];
            let mut block = [0u8; 64];
            block[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
            #[cfg(feature = "robust")]
            {
                crate::haraka::haraka256(&mut outbuf, &block, ctx);
                for i in 0..SPX_N {
                    block[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
                }
            }
            #[cfg(feature = "simple")]
            block[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N]
                .copy_from_slice(&input[..SPX_N]);
            crate::haraka::haraka512(&mut outbuf, &block, ctx);
            out.copy_from_slice(&outbuf[..SPX_N]);
        } else {
            let mut buf = vec![0u8; SPX_ADDR_BYTES + input_len];
            buf[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
            #[cfg(feature = "robust")]
            {
                let mut bitmask = vec![0u8; input_len];
                crate::haraka::haraka_s(
                    &mut bitmask,
                    input_len,
                    &buf,
                    SPX_ADDR_BYTES,
                    ctx,
                );
                for i in 0..input_len {
                    buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
                }
            }
            #[cfg(feature = "simple")]
            buf[SPX_ADDR_BYTES..].copy_from_slice(input);
            crate::haraka::haraka_s(
                out,
                SPX_N,
                &buf,
                SPX_ADDR_BYTES + input_len,
                ctx,
            );
        }
    }

    #[cfg(feature = "sha2")]
    {
        let addr_len = crate::sha2::SPX_SHA256_ADDR_BYTES;
        let mut hash_input = vec![0u8; addr_len + input_len];
        hash_input[..addr_len].copy_from_slice(&addr_bytes[..addr_len]);
        #[cfg(feature = "robust")]
        {
            let mut mgf_input = vec![0u8; SPX_N + addr_len];
            mgf_input[..SPX_N].copy_from_slice(&ctx.pub_seed);
            mgf_input[SPX_N..].copy_from_slice(&addr_bytes[..addr_len]);
            let mut bitmask = vec![0u8; input_len];
            #[cfg(any(feature = "128f", feature = "128s"))]
            crate::sha2::mgf1_256(&mut bitmask, input_len, &mgf_input);
            #[cfg(not(any(feature = "128f", feature = "128s")))]
            if blocks > 1 {
                crate::sha2::mgf1_512(&mut bitmask, input_len, &mgf_input);
            } else {
                crate::sha2::mgf1_256(&mut bitmask, input_len, &mgf_input);
            }
            for i in 0..input_len {
                hash_input[addr_len + i] = input[i] ^ bitmask[i];
            }
        }
        #[cfg(feature = "simple")]
        hash_input[addr_len..].copy_from_slice(input);

        #[cfg(not(any(feature = "128f", feature = "128s")))]
        if blocks > 1 {
            let mut state = ctx.state_seeded_512;
            let mut digest = [0u8; 64];
            crate::sha2::sha512_inc_finalize(
                &mut digest,
                &mut state,
                &hash_input,
                hash_input.len(),
            );
            out.copy_from_slice(&digest[..SPX_N]);
            return;
        }
        let mut state = ctx.state_seeded;
        let mut digest = [0u8; 32];
        crate::sha2::sha256_inc_finalize(
            &mut digest,
            &mut state,
            &hash_input,
            hash_input.len(),
        );
        out.copy_from_slice(&digest[..SPX_N]);
    }

    #[cfg(feature = "shake")]
    {
        let prefix_len = SPX_N + SPX_ADDR_BYTES;
        let mut buf = vec![0u8; prefix_len + input_len];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..prefix_len].copy_from_slice(&addr_bytes);
        #[cfg(feature = "robust")]
        {
            let mut bitmask = vec![0u8; input_len];
            crate::fips202::shake256(&mut bitmask, &buf[..prefix_len]);
            for i in 0..input_len {
                buf[prefix_len + i] = input[i] ^ bitmask[i];
            }
        }
        #[cfg(feature = "simple")]
        buf[prefix_len..].copy_from_slice(input);
        crate::fips202::shake256(out, &buf);
    }

    #[cfg(feature = "blake")]
    {
        let prefix_len = SPX_N + SPX_ADDR_BYTES;
        let mut buf = vec![0u8; prefix_len + input_len];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..prefix_len].copy_from_slice(&addr_bytes);
        #[cfg(feature = "robust")]
        {
            let mut bitmask = vec![0u8; input_len];
            if SPX_N >= 24 && blocks > 1 {
                crate::blake::blake512_mgf1(&mut bitmask, &buf[..prefix_len]);
            } else {
                crate::blake::blake256_mgf1(&mut bitmask, &buf[..prefix_len]);
            }
            for i in 0..input_len {
                buf[prefix_len + i] = input[i] ^ bitmask[i];
            }
        }
        #[cfg(feature = "simple")]
        buf[prefix_len..].copy_from_slice(input);
        let digest_input = if cfg!(feature = "robust") {
            &buf[SPX_N..]
        } else {
            &buf
        };
        if SPX_N >= 24 && blocks > 1 {
            out.copy_from_slice(&crate::blake::blake512(digest_input)[..SPX_N]);
        } else {
            out.copy_from_slice(&crate::blake::blake256(digest_input)[..SPX_N]);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let blocks = inblocks as usize;
    let input_len = blocks * SPX_N;
    let input = if input_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(input, input_len) }.to_vec()
    };
    thash_dynamic(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        &input,
        blocks,
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts(addr, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, message: *const u8) {
    crate::wots::chain_lengths(
        unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) },
        unsafe { std::slice::from_raw_parts(message, SPX_N) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    message: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    crate::wots::wots_pk_from_sig(
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) },
        unsafe { std::slice::from_raw_parts(sig, SPX_WOTS_BYTES) },
        unsafe { std::slice::from_raw_parts(message, SPX_N) },
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts_mut(addr, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    crate::utils::compute_root(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { std::slice::from_raw_parts(leaf, SPX_N) },
        leaf_idx,
        idx_offset,
        unsafe { std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) },
        tree_height,
        unsafe { &*ctx },
        unsafe { &mut *(addr as *mut [u32; 8]) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    message: *const u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    crate::fors::fors_sign(
        unsafe { std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) },
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) },
        unsafe { std::slice::from_raw_parts(message, SPX_FORS_MSG_BYTES) },
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts_mut(addr as *mut u32, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    message: *const u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    crate::fors::fors_pk_from_sig(
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) },
        unsafe { std::slice::from_raw_parts(sig, SPX_FORS_BYTES) },
        unsafe { std::slice::from_raw_parts(message, SPX_FORS_MSG_BYTES) },
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts_mut(addr as *mut u32, 8) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const SpxCtx,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    crate::merkle::merkle_sign(
        unsafe {
            std::slice::from_raw_parts_mut(
                sig,
                SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N,
            )
        },
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { &*ctx },
        unsafe { std::slice::from_raw_parts_mut(wots_addr, 8) },
        unsafe { std::slice::from_raw_parts_mut(tree_addr, 8) },
        idx_leaf,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const SpxCtx) {
    crate::merkle::merkle_gen_root(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe { &*ctx },
    );
}

#[repr(C)]
pub struct CLeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *mut u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn import_leaf_info(info: &CLeafInfoX1) -> crate::wotsx1::LeafInfoX1 {
    let mut rust_info = crate::wotsx1::LeafInfoX1::default();
    rust_info.wots_sign_leaf = info.wots_sign_leaf;
    rust_info.leaf_addr = info.leaf_addr;
    rust_info.pk_addr = info.pk_addr;
    if !info.wots_steps.is_null() {
        rust_info.wots_steps.copy_from_slice(unsafe {
            std::slice::from_raw_parts(info.wots_steps, SPX_WOTS_LEN)
        });
    }
    rust_info
}

fn export_wots_signature(info: &CLeafInfoX1, rust_info: &crate::wotsx1::LeafInfoX1) {
    if !info.wots_sig.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(
                rust_info.wots_sig.as_ptr(),
                info.wots_sig,
                SPX_WOTS_BYTES,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    info: *mut CLeafInfoX1,
) {
    let c_info = unsafe { &mut *info };
    let mut rust_info = import_leaf_info(c_info);
    crate::wotsx1::wots_gen_leafx1(
        unsafe { std::slice::from_raw_parts_mut(dest, SPX_N) },
        unsafe { &*ctx },
        leaf_idx,
        &mut rust_info,
    );
    export_wots_signature(c_info, &rust_info);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut c_void,
) {
    let mut rust_info = crate::fors::ForsGenLeafInfo {
        leaf_addrx: unsafe { *(info as *const [u32; 8]) },
    };
    crate::fors::fors_gen_leafx1(
        unsafe { std::slice::from_raw_parts_mut(dest, SPX_N) },
        unsafe { &*ctx },
        addr_idx,
        &mut rust_info,
    );
    unsafe { *(info as *mut [u32; 8]) = rust_info.leaf_addrx };
}

type GenLeaf = unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: Option<GenLeaf>,
    tree_addr: *mut u32,
) {
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth = unsafe {
        std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N)
    };
    let ctx_ref = unsafe { &*ctx };
    let addr = unsafe { std::slice::from_raw_parts_mut(tree_addr, 8) };
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset = 0usize;
    let callback = gen_leaf.expect("treehash requires a leaf callback");

    for idx in 0..(1u32 << tree_height) {
        unsafe {
            callback(
                stack.as_mut_ptr().add(offset * SPX_N),
                ctx,
                idx + idx_offset,
                tree_addr,
            )
        };
        offset += 1;
        heights[offset - 1] = 0;
        if (leaf_idx ^ 1) == idx {
            auth[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let height = heights[offset - 1];
            let tree_idx = idx >> (height + 1);
            crate::address::set_tree_height(addr, height + 1);
            crate::address::set_tree_index(
                addr,
                tree_idx + (idx_offset >> (height + 1)),
            );
            let start = (offset - 2) * SPX_N;
            let input = stack[start..start + 2 * SPX_N].to_vec();
            thash_inner(&mut stack[start..start + SPX_N], &input, 2, ctx_ref, addr);
            offset -= 1;
            heights[offset - 1] += 1;
            let new_height = heights[offset - 1];
            if ((leaf_idx >> new_height) ^ 1) == tree_idx {
                let auth_start = new_height as usize * SPX_N;
                auth[auth_start..auth_start + SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root.copy_from_slice(&stack[..SPX_N]);
}

fn treehash_x1<F>(
    root: &mut [u8],
    auth: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32],
    mut gen_leaf: F,
) where
    F: FnMut(&mut [u8], u32),
{
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0.. {
        let mut current = vec![0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..], idx + idx_offset);
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut height = 0u32;
        loop {
            if height == tree_height {
                root.copy_from_slice(&current[SPX_N..]);
                return;
            }
            let start = height as usize * SPX_N;
            if (internal_idx ^ internal_leaf) == 1 {
                auth[start..start + SPX_N].copy_from_slice(&current[SPX_N..]);
            }
            if internal_idx & 1 == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            crate::address::set_tree_height(tree_addr, height + 1);
            crate::address::set_tree_index(
                tree_addr,
                internal_idx / 2 + internal_idx_offset,
            );
            current[..SPX_N].copy_from_slice(&stack[start..start + SPX_N]);
            let input = current.clone();
            thash_inner(&mut current[SPX_N..], &input, 2, ctx, tree_addr);
            height += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let start = height as usize * SPX_N;
        stack[start..start + SPX_N].copy_from_slice(&current[SPX_N..]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut CLeafInfoX1,
) {
    let c_info = unsafe { &mut *info };
    let mut rust_info = import_leaf_info(c_info);
    let ctx_ref = unsafe { &*ctx };
    let addr = unsafe { std::slice::from_raw_parts_mut(tree_addr, 8) };
    treehash_x1(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe {
            std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N)
        },
        ctx_ref,
        leaf_idx,
        idx_offset,
        tree_height,
        addr,
        |leaf, idx| crate::wotsx1::wots_gen_leafx1(leaf, ctx_ref, idx, &mut rust_info),
    );
    export_wots_signature(c_info, &rust_info);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut c_void,
) {
    let mut rust_info = crate::fors::ForsGenLeafInfo {
        leaf_addrx: unsafe { *(info as *const [u32; 8]) },
    };
    let ctx_ref = unsafe { &*ctx };
    let addr = unsafe { std::slice::from_raw_parts_mut(tree_addr, 8) };
    treehash_x1(
        unsafe { std::slice::from_raw_parts_mut(root, SPX_N) },
        unsafe {
            std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N)
        },
        ctx_ref,
        leaf_idx,
        idx_offset,
        tree_height,
        addr,
        |leaf, idx| crate::fors::fors_gen_leafx1(leaf, ctx_ref, idx, &mut rust_info),
    );
    unsafe { *(info as *mut [u32; 8]) = rust_info.leaf_addrx };
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    crate::sha2::sha256_inc_init(unsafe { std::slice::from_raw_parts_mut(state, 40) });
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_blocks(
    state: *mut u8,
    input: *const u8,
    inblocks: usize,
) {
    crate::sha2::sha256_inc_blocks(
        unsafe { std::slice::from_raw_parts_mut(state, 40) },
        unsafe { std::slice::from_raw_parts(input, inblocks * 64) },
        inblocks,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    crate::sha2::sha256_inc_finalize(
        unsafe { std::slice::from_raw_parts_mut(out, 32) },
        unsafe { std::slice::from_raw_parts_mut(state, 40) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256(out: *mut u8, input: *const u8, inlen: usize) {
    crate::sha2::sha256(
        unsafe { std::slice::from_raw_parts_mut(out, 32) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    crate::sha2::sha512_inc_init(unsafe { std::slice::from_raw_parts_mut(state, 72) });
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_blocks(
    state: *mut u8,
    input: *const u8,
    inblocks: usize,
) {
    crate::sha2::sha512_inc_blocks(
        unsafe { std::slice::from_raw_parts_mut(state, 72) },
        unsafe { std::slice::from_raw_parts(input, inblocks * 128) },
        inblocks,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    crate::sha2::sha512_inc_finalize(
        unsafe { std::slice::from_raw_parts_mut(out, 64) },
        unsafe { std::slice::from_raw_parts_mut(state, 72) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512(out: *mut u8, input: *const u8, inlen: usize) {
    crate::sha2::sha512(
        unsafe { std::slice::from_raw_parts_mut(out, 64) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_256(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    let input = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    let mut inbuf = input.to_vec();
    inbuf.extend_from_slice(&[0; 4]);
    for (counter, chunk) in out.chunks_mut(32).enumerate() {
        inbuf[inlen as usize..].copy_from_slice(&(counter as u32).to_be_bytes());
        let mut digest = [0u8; 32];
        crate::sha2::sha256(&mut digest, &inbuf, inbuf.len());
        chunk.copy_from_slice(&digest[..chunk.len()]);
    }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_512(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    let input = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    let mut inbuf = input.to_vec();
    inbuf.extend_from_slice(&[0; 4]);
    for (counter, chunk) in out.chunks_mut(64).enumerate() {
        inbuf[inlen as usize..].copy_from_slice(&(counter as u32).to_be_bytes());
        let mut digest = [0u8; 64];
        crate::sha2::sha512(&mut digest, &inbuf, inbuf.len());
        chunk.copy_from_slice(&digest[..chunk.len()]);
    }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    crate::sha2::seed_state(unsafe { &mut *ctx });
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(state: *mut u64) {
    crate::fips202::shake256_inc_init(unsafe { &mut *(state as *mut [u64; 26]) });
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(
    state: *mut u64,
    input: *const u8,
    inlen: usize,
) {
    crate::fips202::shake256_inc_absorb(
        unsafe { &mut *(state as *mut [u64; 26]) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(state: *mut u64) {
    crate::fips202::shake256_inc_finalize(unsafe { &mut *(state as *mut [u64; 26]) });
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    state: *mut u64,
) {
    crate::fips202::shake256_inc_squeeze(
        unsafe { std::slice::from_raw_parts_mut(out, outlen) },
        unsafe { &mut *(state as *mut [u64; 26]) },
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(
    state: *mut u64,
    input: *const u8,
    inlen: usize,
) {
    crate::fips202::shake256_absorb(
        unsafe { &mut *(state as *mut [u64; 25]) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(
    out: *mut u8,
    nblocks: usize,
    state: *mut u64,
) {
    crate::fips202::shake256_squeezeblocks(
        unsafe {
            std::slice::from_raw_parts_mut(out, nblocks * crate::fips202::SHAKE256_RATE)
        },
        unsafe { &mut *(state as *mut [u64; 25]) },
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(
    out: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: usize,
) {
    crate::fips202::shake256(
        unsafe { std::slice::from_raw_parts_mut(out, outlen) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    crate::haraka::tweak_constants(unsafe { &mut *ctx });
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(state: *mut u8) {
    unsafe { std::ptr::write_bytes(state, 0, 65) };
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    state: *mut u8,
    input: *const u8,
    inlen: usize,
    ctx: *const SpxCtx,
) {
    crate::haraka::haraka_s_inc_absorb(
        unsafe { std::slice::from_raw_parts_mut(state, 65) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
        inlen,
        unsafe { &*ctx },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(state: *mut u8) {
    crate::haraka::haraka_s_inc_finalize(
        unsafe { std::slice::from_raw_parts_mut(state, 65) },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    state: *mut u8,
    ctx: *const SpxCtx,
) {
    crate::haraka::haraka_s_inc_squeeze(
        unsafe { std::slice::from_raw_parts_mut(out, outlen) },
        outlen,
        unsafe { std::slice::from_raw_parts_mut(state, 65) },
        unsafe { &*ctx },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S(
    out: *mut u8,
    outlen: u64,
    input: *const u8,
    inlen: u64,
    ctx: *const SpxCtx,
) {
    crate::haraka::haraka_s(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        outlen as usize,
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
        inlen as usize,
        unsafe { &*ctx },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512_perm(
    out: *mut u8,
    input: *const u8,
    ctx: *const SpxCtx,
) {
    let mut block = [0u8; 64];
    block.copy_from_slice(unsafe { std::slice::from_raw_parts(input, 64) });
    crate::haraka::haraka512_perm(&mut block, unsafe { &*ctx });
    unsafe { std::ptr::copy_nonoverlapping(block.as_ptr(), out, 64) };
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512(
    out: *mut u8,
    input: *const u8,
    ctx: *const SpxCtx,
) {
    crate::haraka::haraka512(
        unsafe { std::slice::from_raw_parts_mut(out, 32) },
        unsafe { std::slice::from_raw_parts(input, 64) },
        unsafe { &*ctx },
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka256(
    out: *mut u8,
    input: *const u8,
    ctx: *const SpxCtx,
) {
    crate::haraka::haraka256(
        unsafe { std::slice::from_raw_parts_mut(out, 32) },
        unsafe { std::slice::from_raw_parts(input, 32) },
        unsafe { &*ctx },
    );
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_init(state: *mut crate::blake::Blake256State) {
    let initialized = crate::blake::Blake256State::new();
    let state = unsafe { &mut *state };
    state.h = initialized.h;
    state.s = initialized.s;
    state.t = initialized.t;
    state.buflen = initialized.buflen;
    state.nullt = initialized.nullt;
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_compress(
    state: *mut crate::blake::Blake256State,
    block: *const u8,
) {
    unsafe { &mut *state }.compress(unsafe { std::slice::from_raw_parts(block, 64) });
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_update(
    state: *mut crate::blake::Blake256State,
    input: *const u8,
    bitlen: u64,
) {
    unsafe { &mut *state }.update_bits(
        unsafe { std::slice::from_raw_parts(input, bitlen.div_ceil(8) as usize) },
        bitlen,
    );
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_final(
    state: *mut crate::blake::Blake256State,
    out: *mut u8,
) {
    let digest = unsafe { &mut *state }.finalize();
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 32) };
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init(state: *mut crate::blake::Blake512State) {
    let initialized = crate::blake::Blake512State::new();
    let state = unsafe { &mut *state };
    state.h = initialized.h;
    state.s = initialized.s;
    state.t = initialized.t;
    state.buflen = initialized.buflen;
    state.nullt = initialized.nullt;
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_compress(
    state: *mut crate::blake::Blake512State,
    block: *const u8,
) {
    unsafe { &mut *state }.compress(unsafe { std::slice::from_raw_parts(block, 128) });
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update(
    state: *mut crate::blake::Blake512State,
    input: *const u8,
    bitlen: u64,
) {
    unsafe { &mut *state }.update_bits(
        unsafe { std::slice::from_raw_parts(input, bitlen.div_ceil(8) as usize) },
        bitlen,
    );
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final(
    state: *mut crate::blake::Blake512State,
    out: *mut u8,
) {
    let digest = unsafe { &mut *state }.finalize();
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 64) };
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake256_mgf1(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    crate::blake::blake256_mgf1(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
    );
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    crate::blake::blake512_mgf1(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
    );
}
