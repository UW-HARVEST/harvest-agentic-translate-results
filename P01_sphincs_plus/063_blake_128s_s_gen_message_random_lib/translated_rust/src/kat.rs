// KAT transcript logic, feature-gated per hash backend.
use crate::params::*;

#[cfg(feature = "blake")]
pub mod kat_blake {
    use super::*;
    use crate::blake::*;

    pub struct KatTrCtx {
        state256: Option<BlakeState256>,
        state512: Option<BlakeState512>,
        use512: bool,
    }

    fn new_ctx() -> KatTrCtx {
        if SPX_N >= 24 {
            let mut s = BlakeState512 { h:[0;8], s:[0;4], t:[0;2], buflen:0, nullt:0, buf:[0;128] };
            blake512_init(&mut s);
            KatTrCtx { state256: None, state512: Some(s), use512: true }
        } else {
            let mut s = BlakeState256 { h:[0;8], s:[0;4], t:[0;2], buflen:0, nullt:0, buf:[0;64] };
            blake256_init(&mut s);
            KatTrCtx { state256: Some(s), state512: None, use512: false }
        }
    }

    fn update(ctx: &mut KatTrCtx, data: &[u8], len: usize) {
        if ctx.use512 {
            blake512_update(ctx.state512.as_mut().unwrap(), data, (len as u64) * 8);
        } else {
            blake256_update(ctx.state256.as_mut().unwrap(), data, (len as u64) * 8);
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        *ctx = new_ctx();
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        update(ctx, tag, tag.len());
        update(ctx, &[0x00], 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        update(ctx, label, label.len());
        update(ctx, &[0x00], 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        update(ctx, &lenle, 8);
        update(ctx, &le, 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        update(ctx, &lenle, 8);
        if len > 0 { update(ctx, buf, len); }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8]) {
        if ctx.use512 {
            let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
            blake512_final(ctx.state512.as_mut().unwrap(), &mut outbuf);
            out32[..32].copy_from_slice(&outbuf[..32]);
        } else {
            let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
            blake256_final(ctx.state256.as_mut().unwrap(), &mut outbuf);
            out32[..32].copy_from_slice(&outbuf[..32]);
        }
    }

    pub fn new_default() -> KatTrCtx { new_ctx() }
}

#[cfg(feature = "blake")]
pub use kat_blake::*;

// Stubs for other backends
#[cfg(all(feature = "sha2", not(feature = "blake")))]
pub mod kat_sha2 {
    pub struct KatTrCtx;
    pub fn new_default() -> KatTrCtx { KatTrCtx }
    pub fn kat_tr_init(_ctx: &mut KatTrCtx) { unimplemented!() }
    pub fn kat_tr_absorb_label(_ctx: &mut KatTrCtx, _label: &[u8]) { unimplemented!() }
    pub fn kat_tr_absorb_u64(_ctx: &mut KatTrCtx, _x: u64) { unimplemented!() }
    pub fn kat_tr_absorb_bytes(_ctx: &mut KatTrCtx, _buf: &[u8], _len: usize) { unimplemented!() }
    pub fn kat_tr_final(_ctx: &mut KatTrCtx, _out32: &mut [u8]) { unimplemented!() }
}
#[cfg(all(feature = "sha2", not(feature = "blake")))]
pub use kat_sha2::*;

#[cfg(all(feature = "shake", not(feature = "blake"), not(feature = "sha2")))]
pub mod kat_shake {
    pub struct KatTrCtx;
    pub fn new_default() -> KatTrCtx { KatTrCtx }
    pub fn kat_tr_init(_ctx: &mut KatTrCtx) { unimplemented!() }
    pub fn kat_tr_absorb_label(_ctx: &mut KatTrCtx, _label: &[u8]) { unimplemented!() }
    pub fn kat_tr_absorb_u64(_ctx: &mut KatTrCtx, _x: u64) { unimplemented!() }
    pub fn kat_tr_absorb_bytes(_ctx: &mut KatTrCtx, _buf: &[u8], _len: usize) { unimplemented!() }
    pub fn kat_tr_final(_ctx: &mut KatTrCtx, _out32: &mut [u8]) { unimplemented!() }
}
#[cfg(all(feature = "shake", not(feature = "blake"), not(feature = "sha2")))]
pub use kat_shake::*;

#[cfg(all(feature = "haraka", not(feature = "blake"), not(feature = "sha2"), not(feature = "shake")))]
pub mod kat_haraka {
    pub struct KatTrCtx;
    pub fn new_default() -> KatTrCtx { KatTrCtx }
    pub fn kat_tr_init(_ctx: &mut KatTrCtx) { unimplemented!() }
    pub fn kat_tr_absorb_label(_ctx: &mut KatTrCtx, _label: &[u8]) { unimplemented!() }
    pub fn kat_tr_absorb_u64(_ctx: &mut KatTrCtx, _x: u64) { unimplemented!() }
    pub fn kat_tr_absorb_bytes(_ctx: &mut KatTrCtx, _buf: &[u8], _len: usize) { unimplemented!() }
    pub fn kat_tr_final(_ctx: &mut KatTrCtx, _out32: &mut [u8]) { unimplemented!() }
}
#[cfg(all(feature = "haraka", not(feature = "blake"), not(feature = "sha2"), not(feature = "shake")))]
pub use kat_haraka::*;
