use crate::context::SpxCtx;

#[cfg(feature = "sha2")]
pub use crate::sha2_thash::thash;

#[cfg(feature = "shake")]
pub use crate::shake_thash::thash;

#[cfg(feature = "haraka")]
pub use crate::haraka_thash::thash;

#[cfg(feature = "blake")]
pub use crate::blake_thash::thash;

// C ABI export
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, crate::params::SPX_N) };
    let in_slice = unsafe { std::slice::from_raw_parts(in_, inblocks as usize * crate::params::SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &mut *(addr as *mut [u32; 8]) };
    thash(out_slice, in_slice, inblocks, ctx_ref, addr_ref);
}
