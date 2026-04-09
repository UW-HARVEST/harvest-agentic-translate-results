use crate::params::*;
use crate::context::SpxCtx;
use crate::address::{set_chain_addr, set_hash_addr};
use crate::thash::thash;
use crate::utils::ull_to_bytes;

/// Computes the chaining function.
fn gen_chain(
    out: &mut [u8],
    in_val: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&in_val[..SPX_N]);

    for i in start..(start + steps).min(SPX_WOTS_W as u32) {
        set_hash_addr(addr, i);
        let tmp = out[..SPX_N].to_vec();
        thash(out, &tmp, 1, ctx, addr);
    }
}

/// base_w algorithm: interprets bytes as integers in base w.
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
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
        output[out_idx] = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
        out_idx += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];

    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

/// Takes a message and derives the matching chain lengths.
pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (first, second) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(second, first);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
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
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

// --- extern "C" wrappers ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES) };
    let sig = unsafe { std::slice::from_raw_parts(sig, SPX_WOTS_BYTES) };
    let msg = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    wots_pk_from_sig(pk, sig, msg, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths = unsafe { std::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN) };
    let msg = unsafe { std::slice::from_raw_parts(msg, SPX_N) };
    chain_lengths(lengths, msg);
}
