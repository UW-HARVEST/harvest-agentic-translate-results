use crate::address::{set_chain_addr_rs, set_hash_addr_rs, set_keypair_addr_rs, set_type_rs};
use crate::context::spx_ctx;
use crate::params::*;
use crate::sha2_backend::{SPX_prf_addr_rs, SPX_thash_rs};

#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub(crate) fn wots_gen_leafx1_rs(dest: &mut [u8], ctx: &spx_ctx, leaf_idx: u32, info: &mut leaf_info_x1) {
    let wots_steps = unsafe { std::slice::from_raw_parts(info.wots_steps, SPX_WOTS_LEN) };
    let mut wots_sig = if info.wots_sig.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts_mut(info.wots_sig, SPX_WOTS_BYTES) })
    };
    let wots_k_mask = if leaf_idx == info.wots_sign_leaf { 0 } else { u32::MAX };
    set_keypair_addr_rs(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr_rs(&mut info.pk_addr, leaf_idx);
    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];
    for i in 0..SPX_WOTS_LEN {
        let wots_k = wots_steps[i] | wots_k_mask;
        set_chain_addr_rs(&mut info.leaf_addr, i as u32);
        set_hash_addr_rs(&mut info.leaf_addr, 0);
        set_type_rs(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        SPX_prf_addr_rs(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], ctx, &info.leaf_addr);
        set_type_rs(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);
        for k in 0.. {
            if k == wots_k {
                if let Some(sig) = wots_sig.as_deref_mut() {
                    sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
                }
            }
            if k == (SPX_WOTS_W - 1) as u32 {
                break;
            }
            set_hash_addr_rs(&mut info.leaf_addr, k);
            let tmp = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            SPX_thash_rs(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    SPX_thash_rs(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(dest: *mut u8, ctx: *const spx_ctx, leaf_idx: u32, info: *mut leaf_info_x1) {
    wots_gen_leafx1_rs(
        unsafe { std::slice::from_raw_parts_mut(dest, SPX_N) },
        unsafe { &*ctx },
        leaf_idx,
        unsafe { &mut *info },
    );
}
