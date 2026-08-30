//! The C ABI for `app/include/thash.h`.
//!
//! The tweakable hash itself is provided by the selected backend
//! (`thash_<backend>_<robust|simple>.c`); this module only exposes it under the
//! `SPX_thash` linker name that `SPX_NAMESPACE(thash)` expands to.

use core::ffi::c_uint;

use crate::context::SpxCtx;
use crate::params::SPX_N;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        crate::backend::thash(
            core::slice::from_raw_parts_mut(out, SPX_N),
            core::slice::from_raw_parts(inp, inblocks as usize * SPX_N),
            inblocks as usize,
            &*ctx,
            &*(addr as *const [u32; 8]),
        )
    }
}
