//! Translation of `app/src/wots.c` / `app/include/wots.h`.

use crate::address::set_chain_addr;
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::params::{
    SPX_N, SPX_WOTS_LEN, SPX_WOTS_LEN1, SPX_WOTS_LEN2, SPX_WOTS_LOGW, SPX_WOTS_W,
};
use crate::utils::ull_to_bytes;

// TODO clarify address expectations, and make them more uniform.
// TODO i.e. do we expect types to be set already?
// TODO and do we expect modifications or copies?

/// Computes the chaining function.
/// out and in have to be n-byte arrays.
///
/// Interprets in as start-th value of the chain.
/// addr has to contain the address of the chain.
fn gen_chain(
    out: &mut [u8],
    input: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    /* Initialize out with the value at position 'start'. */
    out[..SPX_N].copy_from_slice(&input[..SPX_N]);

    /* Iterate 'steps' calls to the hash function. */
    let mut i: u32 = start;
    while i < start.wrapping_add(steps) && i < SPX_WOTS_W as u32 {
        crate::address::set_hash_addr(addr, i);
        // thash(out, out, 1, ctx, addr) -- out and in overlap in C.
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(&mut out[..SPX_N], &tmp, 1, ctx, addr);
        i = i.wrapping_add(1);
    }
}

/// base_w algorithm as described in draft.
/// Interprets an array of bytes as integers in base w.
/// This only works when log_w is a divisor of 8.
fn base_w(output: &mut [u32], out_len: i32, input: &[u8]) {
    let mut in_: i32 = 0;
    let mut out: i32 = 0;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    let mut consumed: i32 = 0;
    while consumed < out_len {
        if bits == 0 {
            total = input[in_ as usize];
            in_ += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        // In C `total` is promoted to `int` before the shift.
        output[out as usize] = (((total as i32) >> bits) & (SPX_WOTS_W as i32 - 1)) as u32;
        out += 1;
        consumed += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    const CSUM_BYTES: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;

    let mut csum: u32 = 0;
    let mut csum_bytes = [0u8; CSUM_BYTES];

    /* Compute checksum. */
    for i in 0..SPX_WOTS_LEN1 {
        csum = csum.wrapping_add(
            (SPX_WOTS_W as u32)
                .wrapping_sub(1)
                .wrapping_sub(msg_base_w[i]),
        );
    }

    /* Convert checksum to base_w. */
    /* Make sure expected empty zero bits are the least significant bits. */
    csum = csum.wrapping_shl(((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8) as u32);
    ull_to_bytes(&mut csum_bytes, CSUM_BYTES as u32, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2 as i32, &csum_bytes);
}

/// Takes a message and derives the matching chain lengths.
pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1 as i32, msg);
    // wots_checksum(lengths + SPX_WOTS_LEN1, lengths); the written region
    // (LEN2 words) and the read region (LEN1 words) are disjoint.
    let (msg_base_w, csum_base_w) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(csum_base_w, msg_base_w);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
///
/// Writes the computed public key to 'pk'.
pub fn wots_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    msg: &[u8],
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];

    chain_lengths(&mut lengths, msg);

    let mut i: u32 = 0;
    while (i as usize) < SPX_WOTS_LEN {
        set_chain_addr(addr, i);
        let off = i as usize * SPX_N;
        gen_chain(
            &mut pk[off..],
            &sig[off..],
            lengths[i as usize],
            (SPX_WOTS_W as u32)
                .wrapping_sub(1)
                .wrapping_sub(lengths[i as usize]),
            ctx,
            addr,
        );
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        wots_pk_from_sig(
            core::slice::from_raw_parts_mut(pk, SPX_WOTS_LEN * SPX_N),
            core::slice::from_raw_parts(sig, SPX_WOTS_LEN * SPX_N),
            core::slice::from_raw_parts(msg, SPX_N),
            &*ctx,
            &mut *(addr as *mut [u32; 8]),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_chain_lengths(lengths: *mut core::ffi::c_uint, msg: *const u8) {
    unsafe {
        chain_lengths(
            core::slice::from_raw_parts_mut(lengths as *mut u32, SPX_WOTS_LEN),
            core::slice::from_raw_parts(msg, SPX_N),
        );
    }
}
