#![allow(non_snake_case)]

use super::{
    address, context::SpxCtx, fors, hash, merkle, sign, utils, wots, wotsx1,
};
use crate::params::*;
use std::ffi::{c_int, c_uchar, c_uint, c_ulong, c_ulonglong};

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

unsafe fn bytes_mut<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
    if len == 0 {
        unsafe {
            std::slice::from_raw_parts_mut(std::ptr::NonNull::<u8>::dangling().as_ptr(), 0)
        }
    } else {
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}

unsafe fn words8<'a>(ptr: *const u32) -> &'a [u32; 8] {
    unsafe { &*(ptr as *const [u32; 8]) }
}

unsafe fn words8_mut<'a>(ptr: *mut u32) -> &'a mut [u32; 8] {
    unsafe { &mut *(ptr as *mut [u32; 8]) }
}

#[repr(C)]
struct CContext {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
    #[cfg(feature = "sha2")]
    state_seeded: [u8; 40],
    #[cfg(all(feature = "sha2", not(any(feature = "128f", feature = "128s"))))]
    state_seeded_512: [u8; 72],
    #[cfg(feature = "haraka")]
    tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(feature = "haraka")]
    tweaked256_rc32: [[u32; 8]; 10],
}

fn import_ctx(ctx: &CContext) -> SpxCtx {
    let mut result = SpxCtx::default();
    result.pub_seed = ctx.pub_seed;
    result.sk_seed = ctx.sk_seed;
    #[cfg(feature = "sha2")]
    {
        result.state_seeded = ctx.state_seeded;
    }
    #[cfg(all(feature = "sha2", not(any(feature = "128f", feature = "128s"))))]
    {
        result.state_seeded_512 = ctx.state_seeded_512;
    }
    #[cfg(feature = "haraka")]
    {
        result.tweaked512_rc64 = ctx.tweaked512_rc64;
        result.tweaked256_rc32 = ctx.tweaked256_rc32;
    }
    result
}

fn export_ctx(ctx: &mut CContext, source: &SpxCtx) {
    ctx.pub_seed = source.pub_seed;
    ctx.sk_seed = source.sk_seed;
    #[cfg(feature = "sha2")]
    {
        ctx.state_seeded = source.state_seeded;
    }
    #[cfg(all(feature = "sha2", not(any(feature = "128f", feature = "128s"))))]
    {
        ctx.state_seeded_512 = source.state_seeded_512;
    }
    #[cfg(feature = "haraka")]
    {
        ctx.tweaked512_rc64 = source.tweaked512_rc64;
        ctx.tweaked256_rc32 = source.tweaked256_rc32;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    SK_BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    PK_BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    SEED_BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> c_int {
    sign::crypto_sign_seed_keypair(
        unsafe { bytes_mut(pk, PK_BYTES) },
        unsafe { bytes_mut(sk, SK_BYTES) },
        unsafe { bytes(seed, SEED_BYTES) },
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    sign::crypto_sign_keypair(
        unsafe { bytes_mut(pk, PK_BYTES) },
        unsafe { bytes_mut(sk, SK_BYTES) },
        None,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    sign::crypto_sign_signature(
        unsafe { bytes_mut(sig, BYTES) },
        unsafe { bytes(m, mlen) },
        unsafe { bytes(sk, SK_BYTES) },
        None,
    );
    unsafe { *siglen = BYTES };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    if siglen != BYTES {
        return -1;
    }
    sign::crypto_sign_verify(
        unsafe { bytes(sig, siglen) },
        unsafe { bytes(m, mlen) },
        unsafe { bytes(pk, PK_BYTES) },
    )
    .map_or(-1, |_| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    let mlen = mlen as usize;
    let msg = unsafe { bytes(m, mlen) }.to_vec();
    let out = unsafe { bytes_mut(sm, BYTES + mlen) };
    sign::crypto_sign_signature(
        &mut out[..BYTES],
        &msg,
        unsafe { bytes(sk, SK_BYTES) },
        None,
    );
    out[BYTES..].copy_from_slice(&msg);
    unsafe { *smlen = (BYTES + mlen) as u64 };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut c_ulonglong,
    sm: *const u8,
    smlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    let smlen = smlen as usize;
    if smlen < BYTES {
        if smlen != 0 {
            unsafe { std::ptr::write_bytes(m, 0, smlen) };
        }
        unsafe { *mlen = 0 };
        return -1;
    }
    let signed = unsafe { bytes(sm, smlen) };
    let msg = &signed[BYTES..];
    if sign::crypto_sign_verify(
        &signed[..BYTES],
        msg,
        unsafe { bytes(pk, PK_BYTES) },
    )
    .is_err()
    {
        unsafe { std::ptr::write_bytes(m, 0, smlen) };
        unsafe { *mlen = 0 };
        return -1;
    }
    unsafe { std::ptr::copy(msg.as_ptr(), m, msg.len()) };
    unsafe { *mlen = msg.len() as u64 };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: c_uint, input: u64) {
    utils::ull_to_bytes(unsafe { bytes_mut(out, outlen as usize) }, outlen as usize, input);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, input: u32) {
    utils::u32_to_bytes(unsafe { bytes_mut(out, 4) }, input);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(input: *const u8, inlen: c_uint) -> u64 {
    utils::bytes_to_ull(unsafe { bytes(input, inlen as usize) }, inlen as usize)
}

macro_rules! addr_setter {
    ($ffi:ident, $rust:path) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $ffi(addr: *mut u32, value: u32) {
            $rust(unsafe { words8_mut(addr) }, value);
        }
    };
}

addr_setter!(SPX_set_layer_addr, address::set_layer_addr);
addr_setter!(SPX_set_type, address::set_type);
addr_setter!(SPX_set_keypair_addr, address::set_keypair_addr);
addr_setter!(SPX_set_chain_addr, address::set_chain_addr);
addr_setter!(SPX_set_hash_addr, address::set_hash_addr);
addr_setter!(SPX_set_tree_height, address::set_tree_height);
addr_setter!(SPX_set_tree_index, address::set_tree_index);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    address::set_tree_addr(unsafe { words8_mut(addr) }, tree);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_subtree_addr(out: *mut u32, input: *const u32) {
    let mut source = *unsafe { words8(input) };
    address::copy_subtree_addr(unsafe { words8_mut(out) }, &mut source);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_copy_keypair_addr(out: *mut u32, input: *const u32) {
    address::copy_keypair_addr(unsafe { words8_mut(out) }, unsafe { words8(input) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut c_uint, msg: *const u8) {
    wots::chain_lengths(
        unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) },
        unsafe { bytes(msg, SPX_N) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut CContext) {
    let c_ctx = unsafe { &mut *ctx };
    let mut rust_ctx = import_ctx(c_ctx);
    hash::initialize_hash_function(&mut rust_ctx);
    export_ctx(c_ctx, &rust_ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const CContext, addr: *const u32) {
    let mut address = *unsafe { words8(addr) };
    let rust_ctx = import_ctx(unsafe { &*ctx });
    hash::prf_addr(
        unsafe { bytes_mut(out, SPX_N) },
        &rust_ctx,
        &mut address,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    out: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    msg: *const u8,
    mlen: c_ulonglong,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    #[cfg(feature = "blake")]
    {
        let parts = [
            (unsafe { bytes(sk_prf, SPX_N) }, SPX_N as u64),
            (unsafe { bytes(optrand, SPX_N) }, SPX_N as u64),
            (unsafe { bytes(msg, mlen as usize) }, mlen),
        ];
        if SPX_N >= 24 {
            let digest = super::blake_impl::blake512_updates(&parts);
            unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
        } else {
            let digest = super::blake_impl::blake256_updates(&parts);
            unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
        }
        return;
    }
    #[cfg(not(feature = "blake"))]
    hash::gen_message_random(
        unsafe { bytes_mut(out, SPX_N) },
        unsafe { bytes(sk_prf, SPX_N) },
        unsafe { bytes(optrand, SPX_N) },
        unsafe { bytes(msg, mlen as usize) },
        mlen as usize,
        &rust_ctx,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    msg: *const u8,
    mlen: c_ulonglong,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    hash::hash_message(
        unsafe { bytes_mut(digest, SPX_FORS_MSG_BYTES) },
        unsafe { &mut *tree },
        unsafe { &mut *leaf_idx },
        unsafe { bytes(r, SPX_N) },
        unsafe { bytes(pk, SPX_PK_BYTES) },
        unsafe { bytes(msg, mlen as usize) },
        mlen as usize,
        &rust_ctx,
    );
}

fn thash_runtime(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let input_len = inblocks * SPX_N;
    let source = &input[..input_len];
    let addr_bytes = utils::address_to_bytes(addr);

    #[cfg(feature = "blake")]
    {
        use super::blake_impl::{blake256, blake512, mgf1};
        let wide = SPX_N >= 24 && inblocks > 1;
        if ROBUST {
            let mut seed = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES);
            seed.extend_from_slice(&ctx.pub_seed);
            seed.extend_from_slice(&addr_bytes);
            let mut mask = vec![0u8; input_len];
            mgf1(&mut mask, &seed, wide);
            let mut buf = Vec::with_capacity(SPX_ADDR_BYTES + input_len);
            buf.extend_from_slice(&addr_bytes);
            buf.extend((0..input_len).map(|i| source[i] ^ mask[i]));
            let digest = if wide {
                blake512(&buf).to_vec()
            } else {
                blake256(&buf).to_vec()
            };
            out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
        } else {
            let mut buf = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES + input_len);
            buf.extend_from_slice(&ctx.pub_seed);
            buf.extend_from_slice(&addr_bytes);
            buf.extend_from_slice(source);
            let digest = if wide {
                blake512(&buf).to_vec()
            } else {
                blake256(&buf).to_vec()
            };
            out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
        }
    }

    #[cfg(feature = "sha2")]
    {
        use super::sha2_impl::{
            sha256_inc_finalize, sha512_inc_finalize,
            SPX_SHA256_ADDR_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_OUTPUT_BYTES,
        };
        let wide = SPX_N >= 24 && inblocks > 1;
        if wide {
            let mut state = [0u8; 72];
            state.copy_from_slice(&ctx.state_seeded_512);
            let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + input_len];
            if ROBUST {
                buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
                buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
                    .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
                let mut mask = vec![0u8; input_len];
                sha_mgf(&mut mask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], true);
                for i in 0..input_len {
                    buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = source[i] ^ mask[i];
                }
            } else {
                buf[..SPX_SHA256_ADDR_BYTES]
                    .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
                buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + input_len]
                    .copy_from_slice(source);
            }
            let offset = if ROBUST { SPX_N } else { 0 };
            let mut digest = [0u8; SPX_SHA512_OUTPUT_BYTES];
            sha512_inc_finalize(
                &mut digest,
                &mut state,
                &buf[offset..],
                SPX_SHA256_ADDR_BYTES + input_len,
            );
            out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
        } else {
            let mut state = [0u8; 40];
            state.copy_from_slice(&ctx.state_seeded);
            let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + input_len];
            if ROBUST {
                buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
                buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
                    .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
                let mut mask = vec![0u8; input_len];
                sha_mgf(&mut mask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], false);
                for i in 0..input_len {
                    buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = source[i] ^ mask[i];
                }
            } else {
                buf[..SPX_SHA256_ADDR_BYTES]
                    .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
                buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + input_len]
                    .copy_from_slice(source);
            }
            let offset = if ROBUST { SPX_N } else { 0 };
            let mut digest = [0u8; SPX_SHA256_OUTPUT_BYTES];
            sha256_inc_finalize(
                &mut digest,
                &mut state,
                &buf[offset..],
                SPX_SHA256_ADDR_BYTES + input_len,
            );
            out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
        }
    }

    #[cfg(feature = "shake")]
    {
        use sha3::{
            digest::{ExtendableOutput, Update, XofReader},
            Shake256,
        };
        if ROBUST {
            let mut prefix = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES);
            prefix.extend_from_slice(&ctx.pub_seed);
            prefix.extend_from_slice(&addr_bytes);
            let mut h = Shake256::default();
            h.update(&prefix);
            let mut mask = vec![0u8; input_len];
            h.finalize_xof().read(&mut mask);
            let mut buf = prefix;
            buf.extend((0..input_len).map(|i| source[i] ^ mask[i]));
            let mut h = Shake256::default();
            h.update(&buf);
            h.finalize_xof().read(&mut out[..SPX_N]);
        } else {
            let mut buf = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES + input_len);
            buf.extend_from_slice(&ctx.pub_seed);
            buf.extend_from_slice(&addr_bytes);
            buf.extend_from_slice(source);
            let mut h = Shake256::default();
            h.update(&buf);
            h.finalize_xof().read(&mut out[..SPX_N]);
        }
    }

    #[cfg(feature = "haraka")]
    {
        use super::haraka::{haraka256, haraka512, haraka_s};
        if inblocks == 1 {
            let mut temp = [0u8; 64];
            temp[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
            let mut digest = [0u8; 32];
            if ROBUST {
                haraka256(&mut digest, &temp, ctx);
                for i in 0..SPX_N {
                    temp[SPX_ADDR_BYTES + i] = source[i] ^ digest[i];
                }
            } else {
                temp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(source);
            }
            haraka512(&mut digest, &temp, ctx);
            out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
        } else {
            let mut buf = Vec::with_capacity(SPX_ADDR_BYTES + input_len);
            buf.extend_from_slice(&addr_bytes);
            if ROBUST {
                let mut mask = vec![0u8; input_len];
                haraka_s(&mut mask, input_len, &buf, SPX_ADDR_BYTES, ctx);
                buf.extend((0..input_len).map(|i| source[i] ^ mask[i]));
            } else {
                buf.extend_from_slice(source);
            }
            haraka_s(out, SPX_N, &buf, buf.len(), ctx);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: c_uint,
    ctx: *const CContext,
    addr: *mut u32,
) {
    let len = inblocks as usize * SPX_N;
    let source = unsafe { bytes(input, len) }.to_vec();
    let rust_ctx = import_ctx(unsafe { &*ctx });
    thash_runtime(
        unsafe { bytes_mut(out, SPX_N) },
        &source,
        inblocks as usize,
        &rust_ctx,
        unsafe { words8(addr) },
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
    ctx: *const CContext,
    addr: *mut u32,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    utils::compute_root(
        unsafe { bytes_mut(root, SPX_N) },
        unsafe { bytes(leaf, SPX_N) },
        leaf_idx,
        idx_offset,
        unsafe { bytes(auth_path, tree_height as usize * SPX_N) },
        tree_height,
        &rust_ctx,
        unsafe { words8_mut(addr) },
    );
}

type GenLeaf = unsafe extern "C" fn(*mut u8, *const CContext, u32, *const u32);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const CContext,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: Option<GenLeaf>,
    tree_addr: *mut u32,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    let height = tree_height as usize;
    let mut stack = vec![0u8; (height + 1) * SPX_N];
    let mut heights = vec![0u32; height + 1];
    let mut offset = 0usize;
    let count = 1u32 << tree_height;
    let callback = gen_leaf.expect("C treehash requires gen_leaf");
    for idx in 0..count {
        unsafe {
            callback(
                stack[offset * SPX_N..].as_mut_ptr(),
                ctx,
                idx + idx_offset,
                tree_addr,
            )
        };
        offset += 1;
        heights[offset - 1] = 0;
        if (leaf_idx ^ 1) == idx {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    stack[(offset - 1) * SPX_N..].as_ptr(),
                    auth_path,
                    SPX_N,
                )
            };
        }
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            address::set_tree_height(unsafe { words8_mut(tree_addr) }, heights[offset - 1] + 1);
            address::set_tree_index(
                unsafe { words8_mut(tree_addr) },
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );
            let start = (offset - 2) * SPX_N;
            let source = stack[start..start + 2 * SPX_N].to_vec();
            thash_runtime(
                &mut stack[start..start + SPX_N],
                &source,
                2,
                &rust_ctx,
                unsafe { words8(tree_addr) },
            );
            offset -= 1;
            heights[offset - 1] += 1;
            if ((leaf_idx >> heights[offset - 1]) ^ 1) == tree_idx {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        stack[(offset - 1) * SPX_N..].as_ptr(),
                        auth_path.add(heights[offset - 1] as usize * SPX_N),
                        SPX_N,
                    )
                };
            }
        }
    }
    unsafe { std::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N) };
}

#[repr(C)]
struct CLeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *mut u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn import_leaf_info(info: &CLeafInfoX1) -> wotsx1::LeafInfoX1 {
    let mut result = wotsx1::LeafInfoX1::default();
    result.wots_sign_leaf = info.wots_sign_leaf;
    result.leaf_addr = info.leaf_addr;
    result.pk_addr = info.pk_addr;
    if !info.wots_steps.is_null() {
        result.wots_steps.copy_from_slice(unsafe {
            std::slice::from_raw_parts(info.wots_steps, SPX_WOTS_LEN)
        });
    }
    result
}

unsafe fn export_wots_sig(info: &CLeafInfoX1, rust: &wotsx1::LeafInfoX1) {
    if !info.wots_sig.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(rust.wots_sig.as_ptr(), info.wots_sig, SPX_WOTS_BYTES)
        };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const CContext,
    leaf_idx: u32,
    info: *mut CLeafInfoX1,
) {
    let c_info = unsafe { &mut *info };
    let mut rust_info = import_leaf_info(c_info);
    let rust_ctx = import_ctx(unsafe { &*ctx });
    wotsx1::wots_gen_leafx1(
        unsafe { bytes_mut(dest, SPX_N) },
        &rust_ctx,
        leaf_idx,
        &mut rust_info,
    );
    c_info.leaf_addr = rust_info.leaf_addr;
    c_info.pk_addr = rust_info.pk_addr;
    if leaf_idx == c_info.wots_sign_leaf {
        unsafe { export_wots_sig(c_info, &rust_info) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const CContext,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut CLeafInfoX1,
) {
    let c_info = unsafe { &mut *info };
    let mut rust_info = import_leaf_info(c_info);
    let rust_ctx = import_ctx(unsafe { &*ctx });
    let height = tree_height as usize;
    let mut stack = vec![0u8; height * SPX_N];
    let mut idx = 0u32;
    let max_idx = (1u32 << tree_height) - 1;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        wotsx1::wots_gen_leafx1(
            &mut current[SPX_N..],
            &rust_ctx,
            idx + idx_offset,
            &mut rust_info,
        );
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0usize;
        loop {
            if h == height {
                unsafe {
                    std::ptr::copy_nonoverlapping(current[SPX_N..].as_ptr(), root, SPX_N)
                };
                if c_info.wots_sign_leaf >= idx_offset
                    && c_info.wots_sign_leaf <= idx_offset + max_idx
                {
                    unsafe { export_wots_sig(c_info, &rust_info) };
                }
                return;
            }
            if (internal_idx ^ internal_leaf) == 1 {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        current[SPX_N..].as_ptr(),
                        auth_path.add(h * SPX_N),
                        SPX_N,
                    )
                };
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(unsafe { words8_mut(tree_addr) }, h as u32 + 1);
            address::set_tree_index(
                unsafe { words8_mut(tree_addr) },
                internal_idx / 2 + internal_idx_offset,
            );
            current[..SPX_N].copy_from_slice(&stack[h * SPX_N..(h + 1) * SPX_N]);
            let source = current.clone();
            thash_runtime(
                &mut current[SPX_N..],
                &source,
                2,
                &rust_ctx,
                unsafe { words8(tree_addr) },
            );
            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
        idx += 1;
    }
}

#[repr(C)]
struct CForsInfo {
    leaf_addrx: [u32; 8],
}

#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const CContext,
    addr_idx: u32,
    info: *mut CForsInfo,
) {
    let c_info = unsafe { &mut *info };
    let rust_ctx = import_ctx(unsafe { &*ctx });
    let mut rust_info = fors::ForsGenLeafInfo {
        leaf_addrx: c_info.leaf_addrx,
    };
    fors::fors_gen_leafx1(
        unsafe { bytes_mut(leaf, SPX_N) },
        &rust_ctx,
        addr_idx,
        &mut rust_info,
    );
    c_info.leaf_addrx = rust_info.leaf_addrx;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const CContext,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut CForsInfo,
) {
    let c_info = unsafe { &mut *info };
    let rust_ctx = import_ctx(unsafe { &*ctx });
    let mut rust_info = fors::ForsGenLeafInfo {
        leaf_addrx: c_info.leaf_addrx,
    };
    let height = tree_height as usize;
    let mut stack = vec![0u8; height * SPX_N];
    let mut idx = 0u32;
    let max_idx = (1u32 << tree_height) - 1;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        fors::fors_gen_leafx1(
            &mut current[SPX_N..],
            &rust_ctx,
            idx + idx_offset,
            &mut rust_info,
        );
        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0usize;
        loop {
            if h == height {
                unsafe {
                    std::ptr::copy_nonoverlapping(current[SPX_N..].as_ptr(), root, SPX_N)
                };
                c_info.leaf_addrx = rust_info.leaf_addrx;
                return;
            }
            if (internal_idx ^ internal_leaf) == 1 {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        current[SPX_N..].as_ptr(),
                        auth_path.add(h * SPX_N),
                        SPX_N,
                    )
                };
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(unsafe { words8_mut(tree_addr) }, h as u32 + 1);
            address::set_tree_index(
                unsafe { words8_mut(tree_addr) },
                internal_idx / 2 + internal_idx_offset,
            );
            current[..SPX_N].copy_from_slice(&stack[h * SPX_N..(h + 1) * SPX_N]);
            let source = current.clone();
            thash_runtime(
                &mut current[SPX_N..],
                &source,
                2,
                &rust_ctx,
                unsafe { words8(tree_addr) },
            );
            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        stack[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&current[SPX_N..]);
        idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const CContext,
    addr: *mut u32,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    wots::wots_pk_from_sig(
        unsafe { bytes_mut(pk, SPX_WOTS_BYTES) },
        unsafe { bytes(sig, SPX_WOTS_BYTES) },
        unsafe { bytes(msg, SPX_N) },
        &rust_ctx,
        unsafe { words8_mut(addr) },
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    msg: *const u8,
    ctx: *const CContext,
    fors_addr: *const u32,
) {
    let mut addr = *unsafe { words8(fors_addr) };
    let rust_ctx = import_ctx(unsafe { &*ctx });
    fors::fors_sign(
        unsafe { bytes_mut(sig, SPX_FORS_BYTES) },
        unsafe { bytes_mut(pk, SPX_N) },
        unsafe { bytes(msg, SPX_FORS_MSG_BYTES) },
        &rust_ctx,
        &mut addr,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const CContext,
    fors_addr: *const u32,
) {
    let mut addr = *unsafe { words8(fors_addr) };
    let rust_ctx = import_ctx(unsafe { &*ctx });
    fors::fors_pk_from_sig(
        unsafe { bytes_mut(pk, SPX_N) },
        unsafe { bytes(sig, SPX_FORS_BYTES) },
        unsafe { bytes(msg, SPX_FORS_MSG_BYTES) },
        &rust_ctx,
        &mut addr,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_sign(
    sig: *mut u8,
    root: *mut u8,
    ctx: *const CContext,
    wots_addr: *mut u32,
    tree_addr: *mut u32,
    idx_leaf: u32,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    merkle::merkle_sign(
        unsafe { bytes_mut(sig, SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N) },
        unsafe { bytes_mut(root, SPX_N) },
        &rust_ctx,
        unsafe { words8_mut(wots_addr) },
        unsafe { words8_mut(tree_addr) },
        idx_leaf,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_merkle_gen_root(root: *mut u8, ctx: *const CContext) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    merkle::merkle_gen_root(unsafe { bytes_mut(root, SPX_N) }, &rust_ctx);
}

#[cfg(feature = "blake")]
#[repr(C)]
struct CBlake256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 64],
}

#[cfg(feature = "blake")]
#[repr(C)]
struct CBlake512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 128],
}

#[cfg(feature = "blake")]
fn import_blake256(state: &CBlake256) -> super::blake_impl::B256 {
    super::blake_impl::B256 {
        h: state.h,
        t: state.t,
        buf: state.buf,
        n: state.buflen as u32,
        null: state.nullt != 0,
    }
}

#[cfg(feature = "blake")]
fn export_blake256(state: &mut CBlake256, source: &super::blake_impl::B256) {
    state.h = source.h;
    state.t = source.t;
    state.buf = source.buf;
    state.buflen = source.n as c_int;
    state.nullt = source.null as c_int;
}

#[cfg(feature = "blake")]
fn import_blake512(state: &CBlake512) -> super::blake_impl::B512 {
    super::blake_impl::B512 {
        h: state.h,
        t: state.t,
        buf: state.buf,
        n: state.buflen as u64,
        null: state.nullt != 0,
    }
}

#[cfg(feature = "blake")]
fn export_blake512(state: &mut CBlake512, source: &super::blake_impl::B512) {
    state.h = source.h;
    state.t = source.t;
    state.buf = source.buf;
    state.buflen = source.n as c_int;
    state.nullt = source.null as c_int;
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake256_init(state: *mut CBlake256) {
    let c_state = unsafe { &mut *state };
    let rust_state = super::blake_impl::B256::new();
    c_state.s = [0; 4];
    export_blake256(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake256_compress(state: *mut CBlake256, block: *const u8) {
    let c_state = unsafe { &mut *state };
    let mut rust_state = import_blake256(c_state);
    rust_state.comp(unsafe { bytes(block, 64) });
    export_blake256(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake256_update(
    state: *mut CBlake256,
    input: *const u8,
    inbits: c_ulonglong,
) {
    let c_state = unsafe { &mut *state };
    let mut rust_state = import_blake256(c_state);
    rust_state.up(unsafe { bytes(input, (inbits as usize).div_ceil(8)) }, inbits);
    export_blake256(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake256_final(state: *mut CBlake256, out: *mut u8) {
    let rust_state = import_blake256(unsafe { &*state });
    let digest = rust_state.fin();
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake512_init(state: *mut CBlake512) {
    let c_state = unsafe { &mut *state };
    let rust_state = super::blake_impl::B512::new();
    c_state.s = [0; 4];
    export_blake512(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake512_compress(state: *mut CBlake512, block: *const u8) {
    let c_state = unsafe { &mut *state };
    let mut rust_state = import_blake512(c_state);
    rust_state.comp(unsafe { bytes(block, 128) });
    export_blake512(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake512_update(
    state: *mut CBlake512,
    input: *const u8,
    inbits: c_ulonglong,
) {
    let c_state = unsafe { &mut *state };
    let mut rust_state = import_blake512(c_state);
    rust_state.up(unsafe { bytes(input, (inbits as usize).div_ceil(8)) }, inbits);
    export_blake512(c_state, &rust_state);
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn blake512_final(state: *mut CBlake512, out: *mut u8) {
    let rust_state = import_blake512(unsafe { &*state });
    let digest = rust_state.fin();
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256(out: *mut u8, input: *const u8, inlen: c_ulonglong) -> c_int {
    let digest = super::blake_impl::blake256(unsafe { bytes(input, inlen as usize) });
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
    0
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(out: *mut u8, input: *const u8, inlen: c_ulonglong) -> c_int {
    let digest = super::blake_impl::blake512(unsafe { bytes(input, inlen as usize) });
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), out, digest.len()) };
    0
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake256_mgf1(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    super::blake_impl::mgf1(
        unsafe { bytes_mut(out, outlen as usize) },
        unsafe { bytes(input, inlen as usize) },
        false,
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
    super::blake_impl::mgf1(
        unsafe { bytes_mut(out, outlen as usize) },
        unsafe { bytes(input, inlen as usize) },
        true,
    );
}

#[cfg(feature = "blake")]
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = [
    0x243f6a8885a308d3,
    0x13198a2e03707344,
    0xa4093822299f31d0,
    0x082efa98ec4e6c89,
    0x452821e638d01377,
    0xbe5466cf34e90c6c,
    0xc0ac29b7c97c50dd,
    0x3f84d5b5b5470917,
    0x9216d5d98979fb1b,
    0xd1310ba698dfb5ac,
    0x2ffd72dbd01adfb7,
    0xb8e1afed6a267e96,
    0xba7c9045f12c7f99,
    0x24a19947b3916cf7,
    0x0801f2e2858efc16,
    0x636920d871574e69,
];

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    super::sha2_impl::sha256_inc_init(unsafe { bytes_mut(state, 40) });
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    super::sha2_impl::sha256_inc_blocks(
        unsafe { bytes_mut(state, 40) },
        unsafe { bytes(input, inblocks * 64) },
        inblocks,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha256_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    super::sha2_impl::sha256_inc_finalize(
        unsafe { bytes_mut(out, 32) },
        unsafe { bytes_mut(state, 40) },
        unsafe { bytes(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha256(out: *mut u8, input: *const u8, inlen: usize) {
    super::sha2_impl::sha256(
        unsafe { bytes_mut(out, 32) },
        unsafe { bytes(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    super::sha2_impl::sha512_inc_init(unsafe { bytes_mut(state, 72) });
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    super::sha2_impl::sha512_inc_blocks(
        unsafe { bytes_mut(state, 72) },
        unsafe { bytes(input, inblocks * 128) },
        inblocks,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha512_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    super::sha2_impl::sha512_inc_finalize(
        unsafe { bytes_mut(out, 64) },
        unsafe { bytes_mut(state, 72) },
        unsafe { bytes(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn sha512(out: *mut u8, input: *const u8, inlen: usize) {
    super::sha2_impl::sha512(
        unsafe { bytes_mut(out, 64) },
        unsafe { bytes(input, inlen) },
        inlen,
    );
}

#[cfg(feature = "sha2")]
fn sha_mgf(out: &mut [u8], input: &[u8], wide: bool) {
    let step = if wide { 64 } else { 32 };
    let mut inbuf = Vec::with_capacity(input.len() + 4);
    inbuf.extend_from_slice(input);
    inbuf.extend_from_slice(&[0; 4]);
    let mut offset = 0usize;
    let mut counter = 0u32;
    while offset < out.len() {
        let end = inbuf.len();
        inbuf[end - 4..].copy_from_slice(&counter.to_be_bytes());
        let mut digest = [0u8; 64];
        if wide {
            super::sha2_impl::sha512(&mut digest, &inbuf, inbuf.len());
        } else {
            super::sha2_impl::sha256(&mut digest[..32], &inbuf, inbuf.len());
        }
        let take = step.min(out.len() - offset);
        out[offset..offset + take].copy_from_slice(&digest[..take]);
        offset += take;
        counter = counter.wrapping_add(1);
    }
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_mgf1_256(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    sha_mgf(
        unsafe { bytes_mut(out, outlen as usize) },
        unsafe { bytes(input, inlen as usize) },
        false,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_mgf1_512(
    out: *mut u8,
    outlen: c_ulong,
    input: *const u8,
    inlen: c_ulong,
) {
    sha_mgf(
        unsafe { bytes_mut(out, outlen as usize) },
        unsafe { bytes(input, inlen as usize) },
        true,
    );
}

#[cfg(feature = "sha2")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_seed_state(ctx: *mut CContext) {
    let c_ctx = unsafe { &mut *ctx };
    let mut rust_ctx = import_ctx(c_ctx);
    super::sha2_impl::seed_state(&mut rust_ctx);
    export_ctx(c_ctx, &rust_ctx);
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_tweak_constants(ctx: *mut CContext) {
    let c_ctx = unsafe { &mut *ctx };
    let mut rust_ctx = import_ctx(c_ctx);
    super::haraka::tweak_constants(&mut rust_ctx);
    export_ctx(c_ctx, &rust_ctx);
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka_S_inc_init(state: *mut u8) {
    unsafe { std::ptr::write_bytes(state, 0, 65) };
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    state: *mut u8,
    input: *const u8,
    inlen: usize,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    super::haraka::haraka_s_inc_absorb(
        unsafe { bytes_mut(state, 65) },
        unsafe { bytes(input, inlen) },
        inlen,
        &rust_ctx,
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka_S_inc_finalize(state: *mut u8) {
    super::haraka::haraka_s_inc_finalize(unsafe { bytes_mut(state, 65) });
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    state: *mut u8,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    super::haraka::haraka_s_inc_squeeze(
        unsafe { bytes_mut(out, outlen) },
        outlen,
        unsafe { bytes_mut(state, 65) },
        &rust_ctx,
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka_S(
    out: *mut u8,
    outlen: c_ulonglong,
    input: *const u8,
    inlen: c_ulonglong,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    super::haraka::haraka_s(
        unsafe { bytes_mut(out, outlen as usize) },
        outlen as usize,
        unsafe { bytes(input, inlen as usize) },
        inlen as usize,
        &rust_ctx,
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka512_perm(
    out: *mut u8,
    input: *const u8,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    let mut block = [0u8; 64];
    block.copy_from_slice(unsafe { bytes(input, 64) });
    super::haraka::haraka512_perm(&mut block, &rust_ctx);
    unsafe { std::ptr::copy_nonoverlapping(block.as_ptr(), out, block.len()) };
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka512(
    out: *mut u8,
    input: *const u8,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    super::haraka::haraka512(
        unsafe { bytes_mut(out, 32) },
        unsafe { bytes(input, 64) },
        &rust_ctx,
    );
}

#[cfg(feature = "haraka")]
#[unsafe(no_mangle)]
unsafe extern "C" fn SPX_haraka256(
    out: *mut u8,
    input: *const u8,
    ctx: *const CContext,
) {
    let rust_ctx = import_ctx(unsafe { &*ctx });
    super::haraka::haraka256(
        unsafe { bytes_mut(out, 32) },
        unsafe { bytes(input, 32) },
        &rust_ctx,
    );
}

#[cfg(feature = "shake")]
const SHAKE256_RATE: usize = 136;

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_inc_init(state: *mut u64) {
    super::keccak::inc_init(unsafe { &mut *(state as *mut [u64; 26]) });
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_inc_absorb(state: *mut u64, input: *const u8, inlen: usize) {
    super::keccak::inc_absorb(
        unsafe { &mut *(state as *mut [u64; 26]) },
        SHAKE256_RATE,
        unsafe { bytes(input, inlen) },
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_inc_finalize(state: *mut u64) {
    super::keccak::inc_finalize(
        unsafe { &mut *(state as *mut [u64; 26]) },
        SHAKE256_RATE,
        0x1f,
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_inc_squeeze(out: *mut u8, outlen: usize, state: *mut u64) {
    super::keccak::inc_squeeze(
        unsafe { bytes_mut(out, outlen) },
        unsafe { &mut *(state as *mut [u64; 26]) },
        SHAKE256_RATE,
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_absorb(state: *mut u64, input: *const u8, inlen: usize) {
    super::keccak::absorb(
        unsafe { &mut *(state as *mut [u64; 25]) },
        SHAKE256_RATE,
        unsafe { bytes(input, inlen) },
        0x1f,
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256_squeezeblocks(out: *mut u8, blocks: usize, state: *mut u64) {
    super::keccak::squeeze_blocks(
        unsafe { bytes_mut(out, blocks * SHAKE256_RATE) },
        blocks,
        unsafe { &mut *(state as *mut [u64; 25]) },
        SHAKE256_RATE,
    );
}

#[cfg(feature = "shake")]
#[unsafe(no_mangle)]
unsafe extern "C" fn shake256(
    out: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: usize,
) {
    let mut state = [0u64; 25];
    super::keccak::absorb(
        &mut state,
        SHAKE256_RATE,
        unsafe { bytes(input, inlen) },
        0x1f,
    );
    let blocks = outlen / SHAKE256_RATE;
    if blocks != 0 {
        super::keccak::squeeze_blocks(
            unsafe { bytes_mut(out, blocks * SHAKE256_RATE) },
            blocks,
            &mut state,
            SHAKE256_RATE,
        );
    }
    let remainder = outlen - blocks * SHAKE256_RATE;
    if remainder != 0 {
        let mut block = [0u8; SHAKE256_RATE];
        super::keccak::squeeze_blocks(&mut block, 1, &mut state, SHAKE256_RATE);
        unsafe {
            std::ptr::copy_nonoverlapping(
                block.as_ptr(),
                out.add(blocks * SHAKE256_RATE),
                remainder,
            )
        };
    }
}
