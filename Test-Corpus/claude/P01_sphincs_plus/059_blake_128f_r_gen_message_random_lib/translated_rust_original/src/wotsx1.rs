// WOTS leaf info struct + wots_gen_leafx1 implementation.

use crate::address::*;
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
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    let info = unsafe { &mut *v_info };
    let ctx_ref = unsafe { &*ctx };
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(dest, SPX_N) };

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let step = unsafe { *info.wots_steps.add(i) };
        let wots_k = step | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(
            &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N],
            ctx_ref,
            &info.leaf_addr,
        );

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    let sig_slice = unsafe {
                        core::slice::from_raw_parts_mut(info.wots_sig.add(i * SPX_N), SPX_N)
                    };
                    sig_slice.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
                }
            }

            if k == (SPX_WOTS_W - 1) as u32 {
                break;
            }

            set_hash_addr(&mut info.leaf_addr, k);
            // thash(buffer, buffer, 1, ctx, leaf_addr)
            let mut tmp = vec![0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            let mut out = vec![0u8; SPX_N];
            thash(&mut out, &tmp, 1, ctx_ref, &mut info.leaf_addr);
            pk_buffer[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&out);

            k += 1;
        }
    }

    // Final thash for the public key
    thash(dest_slice, &pk_buffer, SPX_WOTS_LEN as u32, ctx_ref, &mut info.pk_addr);
}
