use crate::address::{set_chain_addr, set_hash_addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes;

fn gen_chain(
    out: *mut u8,
    inp: *const u8,
    start: u32,
    steps: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        std::ptr::copy_nonoverlapping(inp, out, SPX_N);
        let mut i = start;
        while i < start + steps && i < SPX_WOTS_W as u32 {
            set_hash_addr(addr, i);
            thash(out, out, 1, ctx, addr);
            i += 1;
        }
    }
}

fn base_w(output: *mut u32, out_len: i32, input: *const u8) {
    unsafe {
        let mut in_idx = 0usize;
        let mut out_idx = 0usize;
        let mut bits = 0i32;
        let mut total: u8 = 0;

        for _ in 0..out_len {
            if bits == 0 {
                total = *input.add(in_idx);
                in_idx += 1;
                bits += 8;
            }
            bits -= SPX_WOTS_LOGW as i32;
            *output.add(out_idx) = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
            out_idx += 1;
        }
    }
}

fn wots_checksum(csum_base_w: *mut u32, msg_base_w: *const u32) {
    unsafe {
        let mut csum: u32 = 0;
        const CSUM_BYTES_LEN: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
        let mut csum_bytes = [0u8; CSUM_BYTES_LEN];

        for i in 0..SPX_WOTS_LEN1 {
            csum += SPX_WOTS_W as u32 - 1 - *msg_base_w.add(i);
        }

        csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
        ull_to_bytes(csum_bytes.as_mut_ptr(), CSUM_BYTES_LEN as u32, csum as u64);
        base_w(csum_base_w, SPX_WOTS_LEN2 as i32, csum_bytes.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    unsafe {
        base_w(lengths, SPX_WOTS_LEN1 as i32, msg);
        wots_checksum(lengths.add(SPX_WOTS_LEN1), lengths);
    }
}

pub fn chain_lengths(lengths: *mut u32, msg: *const u8) {
    SPX_chain_lengths(lengths, msg);
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
        chain_lengths(lengths.as_mut_ptr(), msg);

        for i in 0..SPX_WOTS_LEN {
            set_chain_addr(addr, i as u32);
            gen_chain(
                pk.add(i * SPX_N),
                sig.add(i * SPX_N),
                lengths[i],
                SPX_WOTS_W as u32 - 1 - lengths[i],
                ctx,
                addr,
            );
        }
    }
}

pub fn wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    SPX_wots_pk_from_sig(pk, sig, msg, ctx, addr);
}
