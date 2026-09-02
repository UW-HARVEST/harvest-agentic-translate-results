//! Translation of `app/src/wots.c` and `app/include/wots.h`.

use crate::address::{addr_mut, set_chain_addr, set_hash_addr, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::utils::ull_to_bytes;
use core::ffi::c_uint;

/// Number of bytes of `csum_bytes` in `wots_checksum`:
/// `(SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8`
const CSUM_BYTES: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;

/// Computes the chaining function.
///
/// `out` and `in` have to be n-byte arrays.
///
/// Interprets `in` as start-th value of the chain.
/// `addr` has to contain the address of the chain.
fn gen_chain(
    out: &mut [u8],
    inp: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut Addr,
) {
    /* Initialize out with the value at position 'start'. */
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);

    /* Iterate 'steps' calls to the hash function. */
    let end = start.wrapping_add(steps);
    let mut i = start;
    while i < end && (i as usize) < SPX_WOTS_W {
        set_hash_addr(addr, i);
        let mut src = [0u8; SPX_N];
        src.copy_from_slice(&out[..SPX_N]);
        thash(&mut out[..SPX_N], &src, 1, ctx, addr);
        i = i.wrapping_add(1);
    }
}

/// `base_w` algorithm as described in draft.
///
/// Interprets an array of bytes as integers in base w.
/// This only works when `log_w` is a divisor of 8.
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut inp = 0usize;
    let mut out = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _consumed in 0..out_len {
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
    let mut csum_bytes = [0u8; CSUM_BYTES];

    /* Compute checksum. */
    for i in 0..SPX_WOTS_LEN1 {
        csum = csum.wrapping_add(SPX_WOTS_W as u32 - 1 - msg_base_w[i]);
    }

    /* Convert checksum to base_w. */
    /* Make sure expected empty zero bits are the least significant bits. */
    csum = csum << ((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8);
    ull_to_bytes(&mut csum_bytes, CSUM_BYTES, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

/// Takes a message and derives the matching chain lengths.
pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (head, tail) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(tail, head);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
///
/// Writes the computed public key to `pk`.
pub fn wots_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    msg: &[u8],
    ctx: &SpxCtx,
    addr: &mut Addr,
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];

    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..(i + 1) * SPX_N],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
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
    wots_pk_from_sig(pk_s, sig_s, msg_s, &*ctx, addr_mut(addr));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut c_uint, msg: *const u8) {
    let l = core::slice::from_raw_parts_mut(lengths as *mut u32, SPX_WOTS_LEN);
    let m = core::slice::from_raw_parts(msg, SPX_N);
    chain_lengths(l, m);
}
