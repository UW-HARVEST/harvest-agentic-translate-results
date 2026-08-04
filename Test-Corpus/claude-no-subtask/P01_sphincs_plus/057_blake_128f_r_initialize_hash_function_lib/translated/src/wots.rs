// WOTS+ implementation

use crate::address;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes;

fn gen_chain(
    out: &mut [u8],
    input: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&input[..SPX_N]);
    let mut i = start;
    while i < start + steps && (i as usize) < SPX_WOTS_W {
        address::set_hash_addr(addr, i);
        let in_data = out[..SPX_N].to_vec();
        thash(&mut out[..SPX_N], &in_data, 1, ctx, addr);
        i += 1;
    }
}

fn base_w(output: &mut [u32], out_len: i32, input: &[u8]) {
    let mut in_idx = 0;
    let mut out_idx = 0;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) as u32) & ((SPX_WOTS_W - 1) as u32);
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    const CSUM_BYTES_LEN: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; CSUM_BYTES_LEN];

    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, CSUM_BYTES_LEN, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2 as i32, &csum_bytes);
}

pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1 as i32, msg);
    let (first, rest) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(rest, first);
}

pub fn wots_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    msg: &[u8],
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        address::set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..(i + 1) * SPX_N],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let p = unsafe { std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    let s = unsafe { std::slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    wots_pk_from_sig(p, s, m, c, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let l = unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let m = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    chain_lengths(l, m);
}
