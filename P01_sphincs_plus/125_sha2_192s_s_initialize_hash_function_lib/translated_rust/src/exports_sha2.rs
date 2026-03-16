// SHA2 backend exports
use crate::context::SpxCtx;
use crate::params::*;

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_init(state: *mut u8) {
    let s = unsafe { core::slice::from_raw_parts_mut(state, 40) };
    crate::sha2::sha256_inc_init_internal(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_blocks(state: *mut u8, in_data: *const u8, inblocks: usize) {
    let s = unsafe { core::slice::from_raw_parts_mut(state, 40) };
    let d = unsafe { core::slice::from_raw_parts(in_data, 64 * inblocks) };
    crate::sha2::sha256_inc_blocks_internal(s, d, inblocks);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, in_data: *const u8, inlen: usize) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_SHA256_OUTPUT_BYTES) };
    let s = unsafe { core::slice::from_raw_parts_mut(state, 40) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen) };
    crate::sha2::sha256_inc_finalize_internal(o, s, d, inlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256(out: *mut u8, in_data: *const u8, inlen: usize) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_SHA256_OUTPUT_BYTES) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen) };
    crate::sha2::sha256_internal(o, d, inlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_init(state: *mut u8) {
    let s = unsafe { core::slice::from_raw_parts_mut(state, 72) };
    crate::sha2::sha512_inc_init_internal(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_blocks(state: *mut u8, in_data: *const u8, inblocks: usize) {
    let s = unsafe { core::slice::from_raw_parts_mut(state, 72) };
    let d = unsafe { core::slice::from_raw_parts(in_data, 128 * inblocks) };
    crate::sha2::sha512_inc_blocks_internal(s, d, inblocks);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, in_data: *const u8, inlen: usize) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_SHA512_OUTPUT_BYTES) };
    let s = unsafe { core::slice::from_raw_parts_mut(state, 72) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen) };
    crate::sha2::sha512_inc_finalize_internal(o, s, d, inlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512(out: *mut u8, in_data: *const u8, inlen: usize) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_SHA512_OUTPUT_BYTES) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen) };
    crate::sha2::sha512_internal(o, d, inlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, in_data: *const u8, inlen: u64) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen as usize) };
    crate::sha2::mgf1_256_internal(o, outlen as usize, d, inlen as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, in_data: *const u8, inlen: u64) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let d = unsafe { core::slice::from_raw_parts(in_data, inlen as usize) };
    crate::sha2::mgf1_512_internal(o, outlen as usize, d, inlen as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    let c = unsafe { &mut *ctx };
    crate::sha2::seed_state_internal(c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let c = unsafe { &mut *ctx };
    crate::hash::initialize_hash_function_internal(c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &*(addr as *const [u32; 8]) };
    crate::hash::prf_addr_internal(o, c, a);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let r_s = unsafe { core::slice::from_raw_parts_mut(r, SPX_N) };
    let sk = unsafe { core::slice::from_raw_parts(sk_prf, SPX_N) };
    let opt = unsafe { core::slice::from_raw_parts(optrand, SPX_N) };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let c = unsafe { &*ctx };
    crate::hash::gen_message_random_internal(r_s, sk, opt, msg, mlen, c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r: *const u8, pk: *const u8, m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let d = unsafe { core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let t = unsafe { &mut *tree };
    let l = unsafe { &mut *leaf_idx };
    let r_s = unsafe { core::slice::from_raw_parts(r, SPX_N) };
    let pk_s = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_s = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let c = unsafe { &*ctx };
    crate::hash::hash_message_internal(d, t, l, r_s, pk_s, m_s, mlen, c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, in_data: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let d = unsafe { core::slice::from_raw_parts(in_data, (inblocks as usize) * SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::thash::thash_internal(o, d, inblocks, c, a);
}
