use core::ffi::c_uint;

use crate::context::SpxCtx;
use crate::params::*;

/// Computes the chaining function.
/// out and in have to be n-byte arrays.
///
/// Interprets in as start-th value of the chain.
/// addr has to contain the address of the chain.
unsafe fn gen_chain(
    out: *mut u8,
    in_: *const u8,
    start: c_uint,
    steps: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut i: u32;

    /* Initialize out with the value at position 'start'. */
    core::ptr::copy_nonoverlapping(in_, out, SPX_N);

    /* Iterate 'steps' calls to the hash function. */
    i = start;
    while i < start.wrapping_add(steps) && i < SPX_WOTS_W {
        crate::address::SPX_set_hash_addr(addr, i);
        crate::hash::SPX_thash(out, out, 1, ctx, addr);
        i = i.wrapping_add(1);
    }
}

/// base_w algorithm as described in draft.
/// Interprets an array of bytes as integers in base w.
/// This only works when log_w is a divisor of 8.
unsafe fn base_w(output: *mut c_uint, out_len: core::ffi::c_int, input: *const u8) {
    let mut in_: core::ffi::c_int = 0;
    let mut out: core::ffi::c_int = 0;
    let mut total: u8 = 0;
    let mut bits: core::ffi::c_int = 0;
    let mut consumed: core::ffi::c_int;

    consumed = 0;
    while consumed < out_len {
        if bits == 0 {
            total = *input.offset(in_ as isize);
            in_ += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as core::ffi::c_int;
        /* C promotes `total` (unsigned char) to int before the shift. */
        *output.offset(out as isize) =
            (((total as core::ffi::c_int) >> bits) as c_uint) & (SPX_WOTS_W - 1);
        out += 1;
        consumed += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
unsafe fn wots_checksum(csum_base_w: *mut c_uint, msg_base_w: *const c_uint) {
    let mut csum: c_uint = 0;
    const CSUM_BYTES_LEN: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW as usize + 7) / 8;
    let mut csum_bytes: [u8; CSUM_BYTES_LEN] = [0u8; CSUM_BYTES_LEN];
    let mut i: c_uint;

    /* Compute checksum. */
    i = 0;
    while (i as usize) < SPX_WOTS_LEN1 {
        csum = csum.wrapping_add((SPX_WOTS_W - 1).wrapping_sub(*msg_base_w.offset(i as isize)));
        i = i.wrapping_add(1);
    }

    /* Convert checksum to base_w. */
    /* Make sure expected empty zero bits are the least significant bits. */
    csum = csum
        << ((8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW as usize) % 8)) % 8);
    crate::utils::SPX_ull_to_bytes(
        csum_bytes.as_mut_ptr(),
        core::mem::size_of::<[u8; CSUM_BYTES_LEN]>() as c_uint,
        csum as core::ffi::c_ulonglong,
    );
    base_w(
        csum_base_w,
        SPX_WOTS_LEN2 as core::ffi::c_int,
        csum_bytes.as_ptr(),
    );
}

/// Takes a message and derives the matching chain lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut c_uint, msg: *const u8) {
    base_w(lengths, SPX_WOTS_LEN1 as core::ffi::c_int, msg);
    wots_checksum(lengths.add(SPX_WOTS_LEN1), lengths as *const c_uint);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
///
/// Writes the computed public key to 'pk'.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut lengths: [c_uint; SPX_WOTS_LEN] = [0; SPX_WOTS_LEN];
    let mut i: u32;

    SPX_chain_lengths(lengths.as_mut_ptr(), msg);

    i = 0;
    while (i as usize) < SPX_WOTS_LEN {
        crate::address::SPX_set_chain_addr(addr, i);
        gen_chain(
            pk.add(i as usize * SPX_N),
            sig.add(i as usize * SPX_N),
            lengths[i as usize],
            (SPX_WOTS_W - 1).wrapping_sub(lengths[i as usize]),
            ctx,
            addr,
        );
        i = i.wrapping_add(1);
    }
}
