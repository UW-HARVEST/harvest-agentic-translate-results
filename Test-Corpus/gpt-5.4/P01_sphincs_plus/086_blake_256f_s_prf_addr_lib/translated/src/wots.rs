use crate::address::{set_chain_addr, set_hash_addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes;

fn gen_chain(out: &mut [u8], input: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&input[..SPX_N]);
    let mut i = start;
    while i < start + steps && (i as usize) < SPX_WOTS_W {
        set_hash_addr(addr, i);
        let hashed = thash(&out[..SPX_N], 1, ctx, addr);
        out[..SPX_N].copy_from_slice(&hashed);
        i += 1;
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total = 0u8;
    let mut bits = 0i32;
    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) & ((SPX_WOTS_W - 1) as u8)) as u32;
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    let mut csum_bytes = vec![0u8; (SPX_WOTS_LEN2 * SPX_WOTS_LOGW).div_ceil(8)];
    for &v in msg_base_w.iter().take(SPX_WOTS_LEN1) {
        csum += (SPX_WOTS_W - 1) as u32 - v;
    }
    csum <<= ((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8) as u32;
    ull_to_bytes(&mut csum_bytes, csum_bytes.len(), csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (a, b) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(b, a);
}

pub fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = vec![0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for (i, &len) in lengths.iter().enumerate() {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..(i + 1) * SPX_N],
            len,
            (SPX_WOTS_W - 1) as u32 - len,
            ctx,
            addr,
        );
    }
}
