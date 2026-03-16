use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::hash::prf_addr_rs;
use crate::thash::thash_rs;

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
    dest: *mut u8, ctx: *const SpxCtx, leaf_idx: u32, v_info: *mut LeafInfoX1,
) {
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *v_info };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, SPX_N) };
    wots_gen_leafx1_rs(dest, ctx, leaf_idx, info);
}

pub fn wots_gen_leafx1_rs(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr_rs(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr_rs(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr_rs(&mut info.leaf_addr, i as u32);
        set_hash_addr_rs(&mut info.leaf_addr, 0);
        set_type_rs(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        let ab = addr_as_bytes(&info.leaf_addr);
        prf_addr_rs(buffer, ctx, ab);

        set_type_rs(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    let sig_slice = unsafe { std::slice::from_raw_parts_mut(info.wots_sig.add(i * SPX_N), SPX_N) };
                    sig_slice.copy_from_slice(buffer);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr_rs(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash_rs(buffer, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash_rs(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}
