use crate::address::{set_chain_addr, set_hash_addr, set_keypair_addr, set_type, SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::{SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W};
use crate::thash::thash;

pub struct LeafInfoX1<'a> {
    pub wots_sig: &'a mut [u8],
    pub wots_sign_leaf: u32,
    pub wots_steps: &'a [u32],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        u32::MAX
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..(i + 1) * SPX_N], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k {
                let dst = &mut info.wots_sig[i * SPX_N..(i + 1) * SPX_N];
                dst.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let in_copy = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            thash(
                &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N],
                &in_copy,
                1,
                ctx,
                &mut info.leaf_addr,
            );
            k += 1;
        }
    }

    let mut pk_addr_copy = info.pk_addr;
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut pk_addr_copy);
    info.pk_addr = pk_addr_copy;
}

// ---------- C-ABI exports ----------

// The C `leaf_info_x1` struct uses raw pointers for wots_sig and wots_steps,
// but the Rust struct uses references. Define a separate ABI-compatible struct
// for FFI.
#[repr(C)]
pub struct CLeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

#[unsafe(export_name = "SPX_wots_gen_leafx1")]
pub unsafe extern "C" fn spx_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut CLeafInfoX1,
) {
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(dest, SPX_N) };
    let info = unsafe { &mut *v_info };
    let sig_slice = unsafe { core::slice::from_raw_parts_mut(info.wots_sig, SPX_WOTS_BYTES) };
    let steps_slice = unsafe { core::slice::from_raw_parts(info.wots_steps, SPX_WOTS_LEN) };
    let mut rust_info = LeafInfoX1 {
        wots_sig: sig_slice,
        wots_sign_leaf: info.wots_sign_leaf,
        wots_steps: steps_slice,
        leaf_addr: info.leaf_addr,
        pk_addr: info.pk_addr,
    };
    wots_gen_leafx1(dest_slice, unsafe { &*ctx }, leaf_idx, &mut rust_info);
    info.leaf_addr = rust_info.leaf_addr;
    info.pk_addr = rust_info.pk_addr;
}
