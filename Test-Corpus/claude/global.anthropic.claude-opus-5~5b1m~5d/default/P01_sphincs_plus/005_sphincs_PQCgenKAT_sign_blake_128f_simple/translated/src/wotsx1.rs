//! Translation of `app/src/wotsx1.c` and the `leaf_info_x1` struct
//! (`app/include/wotsx1.h`).

use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;

/// Mirrors `struct leaf_info_x1`. `wots_sig` and `wots_steps` are raw pointers
/// into caller-owned buffers, exactly as in the C code.
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn new() -> Self {
        LeafInfoX1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

impl Default for LeafInfoX1 {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a WOTS public key; also generates the WOTS signature if
/// `leaf_info` indicates that we're signing with this WOTS key.
pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buf_off = i * SPX_N;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[buf_off..buf_off + SPX_N], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        pk_buffer.as_ptr().add(buf_off),
                        info.wots_sig.add(i * SPX_N),
                        SPX_N,
                    );
                }
            }
            if k == (SPX_WOTS_W - 1) as u32 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let tmp: [u8; SPX_N] = pk_buffer[buf_off..buf_off + SPX_N].try_into().unwrap();
            thash(
                &mut pk_buffer[buf_off..buf_off + SPX_N],
                &tmp,
                1,
                ctx,
                &info.leaf_addr,
            );
            k += 1;
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &info.pk_addr);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    let dest_s = core::slice::from_raw_parts_mut(dest, SPX_N);
    wots_gen_leafx1(dest_s, &*ctx, leaf_idx, &mut *v_info);
}
