use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes_rs;

fn gen_chain(
    out: &mut [u8],
    inp: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);

    let end = start + steps;
    let mut i = start;
    while i < end && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        let mut tmp = vec![0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        let mut out_buf = vec![0u8; SPX_N];
        thash(&mut out_buf, &tmp, 1, ctx, addr);
        out[..SPX_N].copy_from_slice(&out_buf);
        i += 1;
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx: usize = 0;
    let mut out_idx: usize = 0;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;
    let mut consumed: usize = 0;

    while consumed < out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) as u32) & ((SPX_WOTS_W - 1) as u32);
        out_idx += 1;
        consumed += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W - 1) as u32 - msg_base_w[i];
    }

    csum <<= ((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8) as u32;
    ull_to_bytes_rs(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

pub fn chain_lengths_rs(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (a, b) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(b, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths_slice = unsafe { core::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let msg_slice = unsafe { core::slice::from_raw_parts(msg, SPX_N) };
    chain_lengths_rs(lengths_slice, msg_slice);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut [u32; 8],
) {
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    let sig_slice = unsafe { core::slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let msg_slice = unsafe { core::slice::from_raw_parts(msg, SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &mut *addr };

    let mut lengths = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut lengths, msg_slice);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr_ref, i as u32);
        let start = lengths[i];
        let steps = (SPX_WOTS_W - 1) as u32 - lengths[i];
        // copy input to a separate buffer to avoid aliasing
        let mut in_buf = vec![0u8; SPX_N];
        in_buf.copy_from_slice(&sig_slice[i * SPX_N..(i + 1) * SPX_N]);
        let mut out_buf = vec![0u8; SPX_N];
        gen_chain(&mut out_buf, &in_buf, start, steps, ctx_ref, addr_ref);
        pk_slice[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&out_buf);
    }
}
