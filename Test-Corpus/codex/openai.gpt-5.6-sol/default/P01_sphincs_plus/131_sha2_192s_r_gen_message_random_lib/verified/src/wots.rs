//! Translation of `app/src/wots.c`.

use crate::address::{set_chain_addr, set_hash_addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes;

/// Computes the chaining function. `out` and `in` are n-byte arrays.
fn gen_chain(
    out: &mut [u8],
    inp: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);

    let mut i = start;
    while i < start + steps && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        let tmp: [u8; SPX_N] = out[..SPX_N].try_into().unwrap();
        thash(&mut out[..SPX_N], &tmp, 1, ctx, addr);
        i += 1;
    }
}

/// base_w: interprets an array of bytes as integers in base w.
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut inp = 0usize;
    let mut out = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[inp];
            inp += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out] = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
        out += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    const CSUM_BYTES: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; CSUM_BYTES];

    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, CSUM_BYTES, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

/// Takes a message and derives the matching chain lengths.
pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (a, b) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(b, a);
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
        let off = i * SPX_N;
        // gen_chain(pk+i*N, sig+i*N, lengths[i], W-1-lengths[i], ctx, addr)
        let mut out = [0u8; SPX_N];
        gen_chain(
            &mut out,
            &sig[off..off + SPX_N],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
        pk[off..off + SPX_N].copy_from_slice(&out);
    }
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let l = core::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
    let m = core::slice::from_raw_parts(msg, SPX_N);
    chain_lengths(l, m);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let pk_s = core::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES);
    let sig_s = core::slice::from_raw_parts(sig, SPX_WOTS_BYTES);
    let msg_s = core::slice::from_raw_parts(msg, SPX_N);
    let addr_s = &mut *(addr as *mut [u32; 8]);
    wots_pk_from_sig(pk_s, sig_s, msg_s, &*ctx, addr_s);
}
