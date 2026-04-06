use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;

extern "C" {
    fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32);
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

unsafe fn prf_addr(out: *mut u8, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_prf_addr(out, ctx as *const SpxCtx, addr.as_ptr());
}

unsafe fn thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_thash(out, in_, inblocks, ctx as *const SpxCtx, addr.as_mut_ptr());
}

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
    let info = &mut *v_info;
    let leaf_addr = &mut info.leaf_addr;
    let pk_addr = &mut info.pk_addr;
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];
        let wots_k = *info.wots_steps.add(i) | wots_k_mask;

        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer.as_mut_ptr(), &*ctx, leaf_addr);

        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                core::ptr::copy_nonoverlapping(
                    buffer.as_ptr(),
                    info.wots_sig.add(i * SPX_N),
                    SPX_N,
                );
            }

            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }

            set_hash_addr(leaf_addr, k);
            thash(buffer.as_mut_ptr(), buffer.as_ptr(), 1, &*ctx, leaf_addr);
        }
    }

    thash(dest, pk_buffer.as_ptr(), SPX_WOTS_LEN as u32, &*ctx, pk_addr);
}
