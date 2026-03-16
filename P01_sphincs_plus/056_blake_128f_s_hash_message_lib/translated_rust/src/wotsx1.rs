use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::hash_blake::prf_addr;
use crate::thash::thash;

pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

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
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            buffer.as_ptr(),
                            info.wots_sig.add(i * SPX_N),
                            SPX_N,
                        );
                    }
                }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}
