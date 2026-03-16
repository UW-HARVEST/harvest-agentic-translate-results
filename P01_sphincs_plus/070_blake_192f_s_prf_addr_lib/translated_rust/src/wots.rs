use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash;

fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..start + steps {
        if i >= SPX_WOTS_W as u32 {
            break;
        }
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash::thash(out, &tmp, 1, ctx, addr);
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W - 1) as u32 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (first, second) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(second, first);
}

pub fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..],
            lengths[i],
            (SPX_WOTS_W as u32) - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}
