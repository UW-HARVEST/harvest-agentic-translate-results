//! Translation of `app/src/wots.c`.

use crate::address::{SPX_set_chain_addr, SPX_set_hash_addr};
use crate::backend::SPX_thash;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_ull_to_bytes;

/// Computes the chaining function.
/// `out` and `in` have to be n-byte arrays.
unsafe fn gen_chain(
    out: *mut u8,
    inp: *const u8,
    start: u32,
    steps: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    // Initialize out with the value at position 'start'.
    core::ptr::copy_nonoverlapping(inp, out, SPX_N);

    // Iterate 'steps' calls to the hash function.
    let mut i = start;
    while i < (start + steps) && i < SPX_WOTS_W as u32 {
        SPX_set_hash_addr(addr, i);
        SPX_thash(out, out, 1, ctx, addr);
        i += 1;
    }
}

/// base_w algorithm as described in draft.
/// Interprets an array of bytes as integers in base w.
unsafe fn base_w(output: *mut u32, out_len: i32, input: *const u8) {
    let mut inx: isize = 0;
    let mut out: isize = 0;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    let mut consumed: i32 = 0;
    while consumed < out_len {
        if bits == 0 {
            total = *input.offset(inx);
            inx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        *output.offset(out) = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
        out += 1;
        consumed += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
unsafe fn wots_checksum(csum_base_w: *mut u32, msg_base_w: *const u32) {
    let mut csum: u32 = 0;
    let mut csum_bytes = [0u8; (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8];

    // Compute checksum.
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - *msg_base_w.add(i);
    }

    // Convert checksum to base_w.
    // Make sure expected empty zero bits are the least significant bits.
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    SPX_ull_to_bytes(csum_bytes.as_mut_ptr(), csum_bytes.len() as u32, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2 as i32, csum_bytes.as_ptr());
}

/// Takes a message and derives the matching chain lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    base_w(lengths, SPX_WOTS_LEN1 as i32, msg);
    wots_checksum(lengths.add(SPX_WOTS_LEN1), lengths);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];

    SPX_chain_lengths(lengths.as_mut_ptr(), msg);

    for i in 0..SPX_WOTS_LEN {
        SPX_set_chain_addr(addr, i as u32);
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
