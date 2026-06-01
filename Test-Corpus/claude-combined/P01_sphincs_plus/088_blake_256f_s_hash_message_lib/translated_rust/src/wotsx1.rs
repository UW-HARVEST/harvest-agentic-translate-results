// Translation of c_src/app/src/wotsx1.c

use core::slice;

use crate::address::{
    set_chain_addr_inner, set_hash_addr_inner, set_keypair_addr_inner, set_type_inner,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_TYPE_WOTS, SPX_ADDR_TYPE_WOTSPRF, SPX_N, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W,
};
use crate::thash::thash_inner;

#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

// hash backend's prf_addr
#[cfg(feature = "haraka")]
use crate::hash::haraka::hash::prf_addr_inner;

#[cfg(feature = "sha2")]
use crate::hash::sha2::hash::prf_addr_inner;

#[cfg(any(feature = "shake", feature = "blake"))]
fn prf_addr_inner(out: &mut [u8], ctx: &SpxCtx, addr: &[u32]) {
    use crate::params::{SPX_ADDR_BYTES, SPX_N};
    #[cfg(feature = "shake")]
    use crate::hash::shake::fips202::shake256_inner;
    #[cfg(feature = "shake")]
    {
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        shake256_inner(&mut out[..SPX_N], &buf);
    }
    #[cfg(feature = "blake")]
    {
        use crate::hash::blake::blake256::blake256_oneshot;
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; 32];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        blake256_oneshot(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    v_info: *mut LeafInfoX1,
) {
    let dest = unsafe { slice::from_raw_parts_mut(dest, SPX_N) };
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *v_info };
    wots_gen_leafx1_inner(dest, ctx, leaf_idx, info);
}

pub fn wots_gen_leafx1_inner(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr_inner(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr_inner(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_steps_slice = unsafe { slice::from_raw_parts(info.wots_steps, SPX_WOTS_LEN) };
        let wots_k = wots_steps_slice[i] | wots_k_mask;

        set_chain_addr_inner(&mut info.leaf_addr, i as u32);
        set_hash_addr_inner(&mut info.leaf_addr, 0);
        set_type_inner(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        let buf_off = i * SPX_N;
        prf_addr_inner(
            &mut pk_buffer[buf_off..buf_off + SPX_N],
            ctx,
            &info.leaf_addr,
        );

        set_type_inner(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        let mut k: u32 = 0;
        loop {
            if k == wots_k && !info.wots_sig.is_null() {
                let sig_slice = unsafe {
                    slice::from_raw_parts_mut(info.wots_sig.add(i * SPX_N), SPX_N)
                };
                sig_slice.copy_from_slice(&pk_buffer[buf_off..buf_off + SPX_N]);
            }
            if k == (SPX_WOTS_W as u32) - 1 {
                break;
            }
            set_hash_addr_inner(&mut info.leaf_addr, k);
            let mut tmp = vec![0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[buf_off..buf_off + SPX_N]);
            thash_inner(
                &mut pk_buffer[buf_off..buf_off + SPX_N],
                &tmp,
                1,
                ctx,
                &mut info.leaf_addr,
            );
            k += 1;
        }
    }

    thash_inner(dest, &pk_buffer, SPX_WOTS_LEN as u32, ctx, &mut info.pk_addr);
}
