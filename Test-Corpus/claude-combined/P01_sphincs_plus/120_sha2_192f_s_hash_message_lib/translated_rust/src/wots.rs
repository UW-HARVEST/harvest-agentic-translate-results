// Translation of c_src/app/src/wots.c

use core::slice;

use crate::address::{set_chain_addr_inner, set_hash_addr_inner};
use crate::context::SpxCtx;
use crate::params::{
    SPX_N, SPX_WOTS_LEN, SPX_WOTS_LEN1, SPX_WOTS_LEN2, SPX_WOTS_LOGW, SPX_WOTS_W,
};
use crate::thash::thash_inner;
use crate::utils::ull_to_bytes;

pub fn gen_chain(
    out: &mut [u8],
    input: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    out[..SPX_N].copy_from_slice(&input[..SPX_N]);
    let mut i = start;
    while i < start + steps && i < SPX_WOTS_W as u32 {
        set_hash_addr_inner(addr, i);
        let mut input_copy = vec![0u8; SPX_N];
        input_copy.copy_from_slice(&out[..SPX_N]);
        thash_inner(&mut out[..SPX_N], &input_copy, 1, ctx, addr);
        i += 1;
    }
}

pub fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_pos = 0usize;
    let mut out_pos = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;
    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_pos];
            in_pos += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_pos] = ((total >> bits) & ((SPX_WOTS_W - 1) as u8)) as u32;
        out_pos += 1;
    }
}

pub fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];
    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W as u32) - 1 - msg_base_w[i];
    }
    let shift = (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    csum <<= shift;
    ull_to_bytes(&mut csum_bytes, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut core::ffi::c_uint, msg: *const u8) {
    let lengths = unsafe { slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let msg = unsafe { slice::from_raw_parts(msg, SPX_N) };
    let mut tmp = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_inner(&mut tmp, msg);
    for i in 0..SPX_WOTS_LEN {
        lengths[i] = tmp[i] as core::ffi::c_uint;
    }
}

pub fn chain_lengths_inner(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(right, left);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_WOTS_LEN * SPX_N) };
    let sig = unsafe { slice::from_raw_parts(sig, SPX_WOTS_LEN * SPX_N) };
    let msg = unsafe { slice::from_raw_parts(msg, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };

    let mut lengths = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_inner(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr_inner(addr, i as u32);
        let start = lengths[i];
        let steps = (SPX_WOTS_W as u32) - 1 - lengths[i];
        let pk_off = i * SPX_N;
        let sig_off = i * SPX_N;
        // Copy sig piece into a temp because pk and sig may not be the same buffer
        let mut tmp = vec![0u8; SPX_N];
        tmp.copy_from_slice(&sig[sig_off..sig_off + SPX_N]);
        gen_chain(&mut pk[pk_off..pk_off + SPX_N], &tmp, start, steps, ctx, addr);
    }
}
