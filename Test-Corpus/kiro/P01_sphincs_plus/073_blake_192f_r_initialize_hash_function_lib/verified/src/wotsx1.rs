use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;

#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    unsafe {
        let info = &mut *v_info;
        let leaf_addr = info.leaf_addr.as_mut_ptr();
        let pk_addr = info.pk_addr.as_mut_ptr();

        let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
            0
        } else {
            !0u32
        };

        set_keypair_addr(leaf_addr, leaf_idx);
        set_keypair_addr(pk_addr, leaf_idx);

        let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

        for i in 0..SPX_WOTS_LEN {
            let buffer = pk_buffer.as_mut_ptr().add(i * SPX_N);
            let wots_k = *info.wots_steps.add(i) | wots_k_mask;

            set_chain_addr(leaf_addr, i as u32);
            set_hash_addr(leaf_addr, 0);
            set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

            prf_addr(buffer, ctx, leaf_addr as *const u32);

            set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

            for k in 0u32.. {
                if k == wots_k {
                    std::ptr::copy_nonoverlapping(
                        buffer,
                        info.wots_sig.add(i * SPX_N),
                        SPX_N,
                    );
                }
                if k == SPX_WOTS_W as u32 - 1 {
                    break;
                }
                set_hash_addr(leaf_addr, k);
                thash(buffer, buffer, 1, ctx, leaf_addr);
            }
        }

        thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as u32, ctx, pk_addr);
    }
}

pub fn wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    SPX_wots_gen_leafx1(dest, ctx, leaf_idx, v_info);
}
