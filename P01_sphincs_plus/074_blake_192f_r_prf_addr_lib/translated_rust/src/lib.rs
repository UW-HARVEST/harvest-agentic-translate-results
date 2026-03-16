#![allow(non_snake_case, clippy::missing_safety_doc)]

mod params;
mod context;
mod blake256;
mod blake512;
mod utils_impl;
mod hash_blake;
mod thash_blake_robust;

use context::SpxCtx;
use params::*;

// ============================================================
// utils.c exports
// ============================================================

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(
    out: *mut u8, outlen: u32, val: u64,
) {
    let s = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    utils_impl::ull_to_bytes_internal(s, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let s = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    utils_impl::u32_to_bytes_internal(s, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let s = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    utils_impl::bytes_to_ull_internal(s, inlen as usize)
}

// ============================================================
// blake256 exports
// ============================================================

#[repr(C)]
pub struct CBlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

fn c256_to_rust(c: &CBlakeState256) -> blake256::BlakeState256 {
    blake256::BlakeState256 {
        h: c.h, s: c.s, t: c.t, buflen: c.buflen, nullt: c.nullt, buf: c.buf,
    }
}

fn rust256_to_c(r: &blake256::BlakeState256, c: &mut CBlakeState256) {
    c.h = r.h; c.s = r.s; c.t = r.t; c.buflen = r.buflen; c.nullt = r.nullt; c.buf = r.buf;
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_init(s: *mut CBlakeState256) {
    let cs = unsafe { &mut *s };
    let mut rs = c256_to_rust(cs);
    blake256::blake256_init(&mut rs);
    rust256_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress(s: *mut CBlakeState256, block: *const u8) {
    let cs = unsafe { &mut *s };
    let blk = unsafe { core::slice::from_raw_parts(block, 64) };
    let mut rs = c256_to_rust(cs);
    blake256::blake256_compress(&mut rs, blk);
    rust256_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_update(s: *mut CBlakeState256, data: *const u8, datalen_bits: u64) {
    let cs = unsafe { &mut *s };
    let len_bytes = if datalen_bits > 0 { ((datalen_bits + 7) / 8) as usize } else { 0 };
    let d = unsafe { core::slice::from_raw_parts(data, len_bytes) };
    let mut rs = c256_to_rust(cs);
    blake256::blake256_update(&mut rs, d, datalen_bits);
    rust256_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_final(s: *mut CBlakeState256, digest: *mut u8) {
    let cs = unsafe { &mut *s };
    let out = unsafe { core::slice::from_raw_parts_mut(digest, SPX_BLAKE256_OUTPUT_BYTES) };
    let mut rs = c256_to_rust(cs);
    blake256::blake256_final(&mut rs, out);
    rust256_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_BLAKE256_OUTPUT_BYTES) };
    let i = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake256::blake256_hash(o, i, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(
    out: *mut u8, outlen: u64, inp: *const u8, inlen: u64,
) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let i = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake256::blake256_mgf1_internal(o, outlen as usize, i, inlen as usize);
}

// ============================================================
// blake512 exports
// ============================================================

#[repr(C)]
pub struct CBlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

fn c512_to_rust(c: &CBlakeState512) -> blake512::BlakeState512 {
    blake512::BlakeState512 {
        h: c.h, s: c.s, t: c.t, buflen: c.buflen, nullt: c.nullt, buf: c.buf,
    }
}

fn rust512_to_c(r: &blake512::BlakeState512, c: &mut CBlakeState512) {
    c.h = r.h; c.s = r.s; c.t = r.t; c.buflen = r.buflen; c.nullt = r.nullt; c.buf = r.buf;
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_init(s: *mut CBlakeState512) {
    let cs = unsafe { &mut *s };
    let mut rs = c512_to_rust(cs);
    blake512::blake512_init(&mut rs);
    rust512_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress(s: *mut CBlakeState512, block: *const u8) {
    let cs = unsafe { &mut *s };
    let blk = unsafe { core::slice::from_raw_parts(block, 128) };
    let mut rs = c512_to_rust(cs);
    blake512::blake512_compress(&mut rs, blk);
    rust512_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_update(s: *mut CBlakeState512, data: *const u8, datalen_bits: u64) {
    let cs = unsafe { &mut *s };
    let len_bytes = if datalen_bits > 0 { ((datalen_bits + 7) / 8) as usize } else { 0 };
    let d = unsafe { core::slice::from_raw_parts(data, len_bytes) };
    let mut rs = c512_to_rust(cs);
    blake512::blake512_update(&mut rs, d, datalen_bits);
    rust512_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_final(s: *mut CBlakeState512, digest: *mut u8) {
    let cs = unsafe { &mut *s };
    let out = unsafe { core::slice::from_raw_parts_mut(digest, SPX_BLAKE512_OUTPUT_BYTES) };
    let mut rs = c512_to_rust(cs);
    blake512::blake512_final(&mut rs, out);
    rust512_to_c(&rs, cs);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_BLAKE512_OUTPUT_BYTES) };
    let i = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake512::blake512_hash(o, i, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(
    out: *mut u8, outlen: u64, inp: *const u8, inlen: u64,
) {
    let o = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    let i = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake512::blake512_mgf1_internal(o, outlen as usize, i, inlen as usize);
}

// ============================================================
// hash_blake.c exports
// ============================================================

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let c = unsafe { &mut *ctx };
    hash_blake::initialize_hash_function_internal(c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(
    out: *mut u8, ctx: *const SpxCtx, addr: *const u32,
) {
    let c = unsafe { &*ctx };
    let a = unsafe { &*(addr as *const [u32; 8]) };
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    hash_blake::prf_addr_internal(o, c, a);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let c = unsafe { &*ctx };
    let sk = unsafe { core::slice::from_raw_parts(sk_prf, SPX_N) };
    let opt = unsafe { core::slice::from_raw_parts(optrand, SPX_N) };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let out = unsafe { core::slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES) };
    hash_blake::gen_message_random_internal(out, sk, opt, msg, mlen, c);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let c = unsafe { &*ctx };
    let r_val = unsafe { core::slice::from_raw_parts(r, SPX_N) };
    let pk_val = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let msg = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let dig = unsafe { core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let t = unsafe { &mut *tree };
    let l = unsafe { &mut *leaf_idx };
    hash_blake::hash_message_internal(dig, t, l, r_val, pk_val, msg, mlen, c);
}

// ============================================================
// thash export
// ============================================================

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let c = unsafe { &*ctx };
    let a = unsafe { &*(addr as *const [u32; 8]) };
    let ib = inblocks as usize;
    let i = unsafe { core::slice::from_raw_parts(inp, ib * SPX_N) };
    let o = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    thash_blake_robust::thash_internal(o, i, ib, c, a);
}
