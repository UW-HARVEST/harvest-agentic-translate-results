// thash dispatcher: forwards to the active hash backend's implementation.

use core::slice;

use crate::context::SpxCtx;
use crate::params::SPX_N;

#[cfg(all(feature = "haraka", feature = "robust"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::haraka::thash::thash_robust(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "haraka", feature = "simple"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::haraka::thash::thash_simple(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "sha2", feature = "robust"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::sha2::thash::thash_robust(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "sha2", feature = "simple"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::sha2::thash::thash_simple(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "shake", feature = "robust"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::shake::thash::thash_robust(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "shake", feature = "simple"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::shake::thash::thash_simple(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "blake", feature = "robust"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::blake::thash::thash_robust(out, input, inblocks as usize, ctx, addr);
}

#[cfg(all(feature = "blake", feature = "simple"))]
pub fn thash_inner(
    out: &mut [u8],
    input: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32],
) {
    crate::hash::blake::thash::thash_simple(out, input, inblocks as usize, ctx, addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: core::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let in_len = (inblocks as usize) * SPX_N;
    let input = unsafe { slice::from_raw_parts(input, in_len) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts_mut(addr, 8) };
    thash_inner(out, input, inblocks as u32, ctx, addr);
}
