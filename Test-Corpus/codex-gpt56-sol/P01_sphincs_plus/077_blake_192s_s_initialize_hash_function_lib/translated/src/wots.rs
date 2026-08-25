//! Translation of `app/src/wots.c` (+ `app/include/wots.h`).

use crate::address::{SPX_set_chain_addr, SPX_set_hash_addr};
use crate::backend::thash;
use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_WOTS_LEN, SPX_WOTS_LEN1, SPX_WOTS_LEN2, SPX_WOTS_LOGW, SPX_WOTS_W};
use crate::utils::SPX_ull_to_bytes;

// TODO clarify address expectations, and make them more uniform.
// TODO i.e. do we expect types to be set already?
// TODO and do we expect modifications or copies?

/// Size of the `csum_bytes` scratch buffer in `wots_checksum`:
/// `(SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8`.
const CSUM_BYTES: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;

/**
 * Computes the chaining function.
 * out and in have to be n-byte arrays.
 *
 * Interprets in as start-th value of the chain.
 * addr has to contain the address of the chain.
 */
unsafe fn gen_chain(
    out: *mut u8,
    input: *const u8,
    start: u32,
    steps: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let mut i: u32;

        /* Initialize out with the value at position 'start'. */
        core::ptr::copy_nonoverlapping(input, out, SPX_N);

        /* Iterate 'steps' calls to the hash function. */
        i = start;
        while i < start.wrapping_add(steps) && i < SPX_WOTS_W as u32 {
            SPX_set_hash_addr(addr, i);
            thash(out, out as *const u8, 1, ctx, addr);
            i = i.wrapping_add(1);
        }
    }
}

/**
 * base_w algorithm as described in draft.
 * Interprets an array of bytes as integers in base w.
 * This only works when log_w is a divisor of 8.
 */
unsafe fn base_w(output: *mut u32, out_len: i32, input: *const u8) {
    unsafe {
        let mut in_: i32 = 0;
        let mut out: i32 = 0;
        let mut total: u8 = 0;
        let mut bits: i32 = 0;
        let mut consumed: i32;

        consumed = 0;
        while consumed < out_len {
            if bits == 0 {
                total = *input.offset(in_ as isize);
                in_ += 1;
                bits += 8;
            }
            bits -= SPX_WOTS_LOGW as i32;
            *output.offset(out as isize) =
                ((total as u32) >> (bits as u32)) & (SPX_WOTS_W as u32).wrapping_sub(1);
            out += 1;
            consumed += 1;
        }
    }
}

/* Computes the WOTS+ checksum over a message (in base_w). */
unsafe fn wots_checksum(csum_base_w: *mut u32, msg_base_w: *const u32) {
    unsafe {
        let mut csum: u32 = 0;
        let mut csum_bytes: [u8; CSUM_BYTES] = [0u8; CSUM_BYTES];
        let mut i: u32;

        /* Compute checksum. */
        i = 0;
        while (i as usize) < SPX_WOTS_LEN1 {
            csum = csum.wrapping_add(
                (SPX_WOTS_W as u32)
                    .wrapping_sub(1)
                    .wrapping_sub(*msg_base_w.add(i as usize)),
            );
            i = i.wrapping_add(1);
        }

        /* Convert checksum to base_w. */
        /* Make sure expected empty zero bits are the least significant bits. */
        csum = csum.wrapping_shl(((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8) as u32);
        SPX_ull_to_bytes(csum_bytes.as_mut_ptr(), CSUM_BYTES as u32, csum as u64);
        base_w(csum_base_w, SPX_WOTS_LEN2 as i32, csum_bytes.as_ptr());
    }
}

/* Takes a message and derives the matching chain lengths. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    unsafe {
        base_w(lengths, SPX_WOTS_LEN1 as i32, msg);
        wots_checksum(lengths.add(SPX_WOTS_LEN1), lengths as *const u32);
    }
}

/**
 * Takes a WOTS signature and an n-byte message, computes a WOTS public key.
 *
 * Writes the computed public key to 'pk'.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let mut lengths: [u32; SPX_WOTS_LEN] = [0u32; SPX_WOTS_LEN];
        let mut i: u32;

        SPX_chain_lengths(lengths.as_mut_ptr(), msg);

        i = 0;
        while (i as usize) < SPX_WOTS_LEN {
            SPX_set_chain_addr(addr, i);
            gen_chain(
                pk.add(i as usize * SPX_N),
                sig.add(i as usize * SPX_N),
                lengths[i as usize],
                (SPX_WOTS_W as u32)
                    .wrapping_sub(1)
                    .wrapping_sub(lengths[i as usize]),
                ctx,
                addr,
            );
            i = i.wrapping_add(1);
        }
    }
}
