use crate::address::{set_chain_addr_rs, set_hash_addr_rs};
use crate::context::spx_ctx;
use crate::params::*;
use crate::sha2_backend::SPX_thash_rs;
use crate::utils::ull_to_bytes_into;

fn gen_chain(out: &mut [u8], input: &[u8], start: u32, steps: u32, ctx: &spx_ctx, addr: &mut [u32; 8]) {
    out.copy_from_slice(input);
    let end = (start + steps).min(SPX_WOTS_W as u32);
    for i in start..end {
        set_hash_addr_rs(addr, i);
        let tmp = out.to_vec();
        SPX_thash_rs(out, &tmp, 1, ctx, addr);
    }
}

fn base_w(output: &mut [u32], input: &[u8]) {
    let mut in_idx = 0usize;
    let mut total = 0u8;
    let mut bits = 0i32;
    for out in output.iter_mut() {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        *out = ((total >> bits) & ((SPX_WOTS_W - 1) as u8)) as u32;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    for &v in msg_base_w.iter().take(SPX_WOTS_LEN1) {
        csum += (SPX_WOTS_W - 1) as u32 - v;
    }
    csum <<= ((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8) as u32;
    let mut csum_bytes = vec![0u8; (SPX_WOTS_LEN2 * SPX_WOTS_LOGW).div_ceil(8)];
    ull_to_bytes_into(&mut csum_bytes, csum as u64);
    base_w(csum_base_w, &csum_bytes);
}

pub(crate) fn chain_lengths_rs(lengths: &mut [u32], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(right, left);
}

pub(crate) fn wots_pk_from_sig_rs(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &spx_ctx, addr: &mut [u32; 8]) {
    let mut lengths = vec![0u32; SPX_WOTS_LEN];
    chain_lengths_rs(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr_rs(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..(i + 1) * SPX_N],
            lengths[i],
            (SPX_WOTS_W - 1) as u32 - lengths[i],
            ctx,
            addr,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    chain_lengths_rs(
        unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) },
        unsafe { std::slice::from_raw_parts(msg, SPX_N) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const spx_ctx,
    addr: *mut u32,
) {
    wots_pk_from_sig_rs(
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) },
        unsafe { std::slice::from_raw_parts(sig, SPX_WOTS_BYTES) },
        unsafe { std::slice::from_raw_parts(msg, SPX_N) },
        unsafe { &*ctx },
        unsafe { &mut *(addr as *mut [u32; 8]) },
    );
}
