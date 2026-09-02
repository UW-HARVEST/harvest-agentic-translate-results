//! Translation of `app/src/wotsx1.c` and `app/include/wotsx1.h`.

use crate::address::{
    addr_mut, set_chain_addr, set_hash_addr, set_keypair_addr, set_type, Addr, ZERO_ADDR,
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF,
};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;

/// `leaf_info_x1`
///
/// This is here to provide an interface to the internal `wots_gen_leafx1`
/// routine.  While this routine is not referenced in the package outside of
/// `wots.c`, it is called from the stand-alone benchmark code to characterize
/// the performance.
#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    /// The index of the WOTS we're using to sign
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: Addr,
    pub pk_addr: Addr,
}

impl LeafInfoX1 {
    /// `struct leaf_info_x1 info = { 0 };`
    pub const fn zeroed() -> Self {
        LeafInfoX1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
            leaf_addr: ZERO_ADDR,
            pk_addr: ZERO_ADDR,
        }
    }
}

/// This generates a WOTS public key.
///
/// It also generates the WOTS signature if `leaf_info` indicates that we're
/// signing with this WOTS key.
pub unsafe fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        /* We're traversing the leaf that's signing; generate the WOTS
           signature */
        0
    } else {
        /* Nope, we're just generating pk's; turn off the signature logic */
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        /* Set wots_k to the step if we're generating a signature, ~0 if not */
        let wots_k: u32 = *info.wots_steps.add(i) | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        /* Start with the secret seed */
        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        /* Iterate down the WOTS chain */
        let mut k: u32 = 0;
        loop {
            /* Check if this is the value that needs to be saved as a part of
               the WOTS signature */
            if k == wots_k {
                core::ptr::copy_nonoverlapping(
                    buffer.as_ptr(),
                    info.wots_sig.add(i * SPX_N),
                    SPX_N,
                );
            }

            /* Check if we hit the top of the chain */
            if k as usize == SPX_WOTS_W - 1 {
                break;
            }

            /* Iterate one step on the chain */
            set_hash_addr(&mut info.leaf_addr, k);

            let mut src = [0u8; SPX_N];
            src.copy_from_slice(buffer);
            thash(buffer, &src, 1, ctx, &info.leaf_addr);

            k += 1;
        }
    }

    /* Do the final thash to generate the public keys */
    thash(
        dest,
        &pk_buffer,
        SPX_WOTS_LEN as u32,
        ctx,
        &info.pk_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    let dest_s = core::slice::from_raw_parts_mut(dest, SPX_N);
    wots_gen_leafx1(dest_s, &*ctx, leaf_idx, &mut *v_info);
}

/// Not used by the library itself; kept so that `addr_mut` has a consumer in
/// every configuration.
#[allow(dead_code)]
unsafe fn _unused(addr: *mut u32) -> &'static mut Addr {
    addr_mut(addr)
}
