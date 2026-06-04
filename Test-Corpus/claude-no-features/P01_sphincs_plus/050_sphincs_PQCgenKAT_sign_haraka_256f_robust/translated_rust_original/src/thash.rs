// Robust thash for haraka backend
use crate::context::SpxCtx;
use crate::haraka::{haraka256_safe, haraka512_safe, haraka_s_safe};
use crate::params::{SPX_ADDR_BYTES, SPX_N};

pub fn thash_safe(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf_tmp[..32].copy_from_slice(addr_bytes);

        let mut outbuf = [0u8; 32];
        haraka256_safe(&mut outbuf, &buf_tmp[..32].try_into().unwrap(), ctx);
        for i in 0..(inblocks * SPX_N) {
            buf_tmp[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
        }
        let mut outbuf512 = [0u8; 32];
        haraka512_safe(&mut outbuf512, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf512[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
        buf[..32].copy_from_slice(addr_bytes);

        haraka_s_safe(&mut bitmask, &buf[..SPX_ADDR_BYTES], ctx);

        for i in 0..(inblocks * SPX_N) {
            buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        let mut out_tmp = vec![0u8; SPX_N];
        haraka_s_safe(&mut out_tmp, &buf, ctx);
        out[..SPX_N].copy_from_slice(&out_tmp[..SPX_N]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: std::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let in_slice = std::slice::from_raw_parts(input, inblocks as usize * SPX_N);
        let addr_arr = &*(addr as *const [u32; 8]);
        let mut out_buf = [0u8; SPX_N];
        thash_safe(&mut out_buf, in_slice, inblocks, &*ctx, addr_arr);
        std::ptr::copy_nonoverlapping(out_buf.as_ptr(), out, SPX_N);
    }
}

// Convenience for internal callers using raw pointers
pub fn thash(out: *mut u8, input: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32) {
    SPX_thash(out, input, inblocks, ctx, addr);
}
