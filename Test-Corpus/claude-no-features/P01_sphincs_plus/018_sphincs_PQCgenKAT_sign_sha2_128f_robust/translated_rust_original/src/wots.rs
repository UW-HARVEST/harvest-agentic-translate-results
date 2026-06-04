use crate::address::{set_chain_addr, set_hash_addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes_slice;

#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

fn gen_chain(out: *mut u8, input: *const u8, start: u32, steps: u32, ctx: *const SpxCtx, addr: *mut u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(input, out, SPX_N);
        let mut i = start;
        while i < (start + steps) && i < SPX_WOTS_W as u32 {
            set_hash_addr(addr, i);
            thash(out, out, 1, ctx, addr);
            i += 1;
        }
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut input_pos = 0usize;
    let mut total: u8 = 0;
    let mut bits = 0i32;
    for consumed in 0..out_len {
        if bits == 0 {
            total = input[input_pos];
            input_pos += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[consumed] = ((total >> bits) as u32) & ((SPX_WOTS_W - 1) as u32);
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    const CSUM_BYTES_LEN: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; CSUM_BYTES_LEN];

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W as u32) - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes_slice(&mut csum_bytes, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    unsafe {
        let len_slice = std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
        let msg_slice = std::slice::from_raw_parts(msg, (SPX_WOTS_LEN1 * SPX_WOTS_LOGW + 7) / 8);
        base_w(&mut len_slice[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg_slice);
        let (left, right) = len_slice.split_at_mut(SPX_WOTS_LEN1);
        wots_checksum(right, left);
    }
}

pub fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    let mut tmp = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut tmp, left);
    right.copy_from_slice(&tmp);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let mut lengths = [0u32; SPX_WOTS_LEN];
        SPX_chain_lengths(lengths.as_mut_ptr(), msg);

        for i in 0..(SPX_WOTS_LEN as u32) {
            set_chain_addr(addr, i);
            gen_chain(
                pk.add(i as usize * SPX_N),
                sig.add(i as usize * SPX_N),
                lengths[i as usize],
                (SPX_WOTS_W as u32) - 1 - lengths[i as usize],
                ctx,
                addr,
            );
        }
    }
}
