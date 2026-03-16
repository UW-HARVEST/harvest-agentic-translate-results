use crate::blake256;
use crate::blake512;
use crate::context::spx_ctx;
use crate::params::*;

// thash_blake_robust: uses blake512 for inblocks > 1 (since SPX_BLAKE512=1)
pub fn thash(out: *mut u8, inp: *const u8, inblocks: u32, ctx: &spx_ctx, addr: *mut u32) {
    unsafe {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
        let ib = inblocks as usize;
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; ib * SPX_N];
        let buf_len = SPX_N + SPX_ADDR_BYTES + ib * SPX_N;
        let mut buf = vec![0u8; buf_len];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

        blake256::blake256_mgf1(&mut bitmask, ib * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

        let in_slice = std::slice::from_raw_parts(inp, ib * SPX_N);
        for i in 0..ib * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = in_slice[i] ^ bitmask[i];
        }

        blake256::blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + ib * SPX_N) as u64);
        std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    }
}

unsafe fn thash_512(out: *mut u8, inp: *const u8, inblocks: u32, ctx: &spx_ctx, addr: *mut u32) {
    let ib = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; ib * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + ib * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

    blake512::blake512_mgf1(&mut bitmask, ib * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    let in_slice = std::slice::from_raw_parts(inp, ib * SPX_N);
    for i in 0..ib * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = in_slice[i] ^ bitmask[i];
    }

    blake512::blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + ib * SPX_N) as u64);
    std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, inp: *const u8, inblocks: u32,
    ctx: *const spx_ctx, addr: *mut u32,
) {
    unsafe { thash(out, inp, inblocks, &*ctx, addr); }
}
