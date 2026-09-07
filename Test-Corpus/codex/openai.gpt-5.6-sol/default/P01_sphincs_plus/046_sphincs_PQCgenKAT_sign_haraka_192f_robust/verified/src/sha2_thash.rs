//! Translation of `lib/sha2/src/thash_sha2_robust.c` and
//! `lib/sha2/src/thash_sha2_simple.c` (selected by the `spx_thash` cfg).

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::sha2::sha256_inc_finalize;

const SPX_SHA256_ADDR_BYTES: usize = 22;

// ================= ROBUST =================
#[cfg(spx_thash = "robust")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(spx_sha512)]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    use crate::sha2::{mgf1_256, SPX_SHA256_OUTPUT_BYTES};
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(spx_thash = "robust", spx_sha512))]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    use crate::sha2::{mgf1_512, sha512_inc_finalize, SPX_SHA512_OUTPUT_BYTES};
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ================= SIMPLE =================
#[cfg(spx_thash = "simple")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(spx_sha512)]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    use crate::sha2::SPX_SHA256_OUTPUT_BYTES;
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let ab = addr_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&inp[..inblocks * SPX_N]);

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(spx_thash = "simple", spx_sha512))]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    use crate::sha2::{sha512_inc_finalize, SPX_SHA512_OUTPUT_BYTES};
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let ab = addr_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&inp[..inblocks * SPX_N]);

    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ------------------------------------------------------------------
// Exported C ABI wrapper.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: core::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let o = core::slice::from_raw_parts_mut(out, SPX_N);
    let i = core::slice::from_raw_parts(inp, inblocks as usize * SPX_N);
    thash(o, i, inblocks, &*ctx, &*(addr as *const [u32; 8]));
}
