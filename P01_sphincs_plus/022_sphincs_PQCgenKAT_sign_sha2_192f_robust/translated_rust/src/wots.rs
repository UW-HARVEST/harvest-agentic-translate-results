use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::thash::thash;
use crate::hash_sha2::prf_addr;

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut bits = 0i32;
    let mut total = 0u8;

    for out_idx in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4]; // max needed
    crate::utils::ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

pub fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(right, left);
}

fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..std::cmp::min(start + steps, SPX_WOTS_W as u32) {
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(out, &tmp, 1, ctx, addr);
    }
}

pub fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..],
            &sig[i * SPX_N..],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

pub fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut crate::wotsx1::LeafInfoX1,
) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if let Some(ref mut ws) = info.wots_sig {
                    ws[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            thash(&mut pk_buffer[i * SPX_N..], &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}
