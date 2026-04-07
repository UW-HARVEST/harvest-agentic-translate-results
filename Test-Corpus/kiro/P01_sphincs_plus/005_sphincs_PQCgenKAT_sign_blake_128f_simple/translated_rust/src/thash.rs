use crate::context::SpxCtx;
use crate::params::*;

pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(feature = "shake")]
    { crate::shake_backend::thash(out, input, inblocks, ctx, addr); }
    #[cfg(feature = "sha2")]
    { crate::sha2_backend::thash(out, input, inblocks, ctx, addr); }
    #[cfg(feature = "blake")]
    { crate::blake_backend::thash(out, input, inblocks, ctx, addr); }
    #[cfg(feature = "haraka")]
    { crate::haraka_backend::thash(out, input, inblocks, ctx, addr); }
}
