use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::thash::thash_rs;
use crate::utils::ull_to_bytes_rs;

fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..std::cmp::min(start + steps, SPX_WOTS_W as u32) {
        set_hash_addr_rs(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash_rs(out, &tmp, 1, ctx, addr);
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut bits = 0i32;
    let mut total = 0u8;
    for consumed in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[consumed] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];
    ull_to_bytes_rs(&mut csum_bytes, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths = unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let msg = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    chain_lengths_rs(lengths, msg);
}

pub fn chain_lengths_rs(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let mut tmp = [0u32; SPX_WOTS_LEN1];
    tmp.copy_from_slice(&lengths[..SPX_WOTS_LEN1]);
    wots_checksum(&mut lengths[SPX_WOTS_LEN1..], &tmp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8, sig: *const u8, msg: *const u8,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let sig = unsafe { std::slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let msg = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    wots_pk_from_sig_rs(pk, sig, msg, ctx, addr);
}

pub fn wots_pk_from_sig_rs(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr_rs(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..],
            lengths[i], SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx, addr,
        );
    }
}
