#![allow(non_snake_case)]

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha2::*;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_SHA256_ADDR_BYTES);
    SPX_mgf1_512(bitmask.as_mut_ptr(), (inblocks * SPX_N) as u64, buf.as_ptr(), (SPX_N + SPX_SHA256_ADDR_BYTES) as u64);

    core::ptr::copy_nonoverlapping((*ctx).state_seeded_512.as_ptr(), sha2_state.as_mut_ptr(), 72);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    sha512_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr().add(SPX_N), SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_, inblocks, ctx, addr);
            return;
        }
    }

    let inblocks_usize = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks_usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks_usize * SPX_N];
    let mut sha2_state = [0u8; 40];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_SHA256_ADDR_BYTES);
    SPX_mgf1_256(bitmask.as_mut_ptr(), (inblocks_usize * SPX_N) as u64, buf.as_ptr(), (SPX_N + SPX_SHA256_ADDR_BYTES) as u64);

    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);

    for i in 0..inblocks_usize * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    sha256_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr().add(SPX_N), SPX_SHA256_ADDR_BYTES + inblocks_usize * SPX_N);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
