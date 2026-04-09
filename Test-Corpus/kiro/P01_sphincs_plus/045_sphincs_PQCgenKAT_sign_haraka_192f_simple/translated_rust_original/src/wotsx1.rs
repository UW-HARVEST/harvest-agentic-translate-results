use crate::params::*;
use crate::context::SpxCtx;
use crate::address::{set_keypair_addr, set_chain_addr, set_hash_addr, set_type};
use crate::hash::prf_addr;
use crate::thash::thash;

#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// Generate a WOTS public key (leaf node), and optionally a WOTS signature.
pub fn wots_gen_leafx1(
    dest: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    info: &mut LeafInfoX1,
) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    let sig_slice = unsafe {
                        std::slice::from_raw_parts_mut(info.wots_sig.add(i * SPX_N), SPX_N)
                    };
                    sig_slice.copy_from_slice(buffer);
                }
            }

            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            set_hash_addr(&mut info.leaf_addr, k);
            let tmp = buffer.to_vec();
            thash(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}

// --- extern "C" wrapper ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, SPX_N) };
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *v_info };
    wots_gen_leafx1(dest, ctx, leaf_idx, info);
}
