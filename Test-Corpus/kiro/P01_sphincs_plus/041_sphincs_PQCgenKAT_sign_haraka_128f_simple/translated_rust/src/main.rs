#![allow(non_snake_case, unused_assignments)]

use sphincsplus::params::*;
use sphincsplus::rng::{randombytes_init, rng_randombytes};
use sphincsplus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn to_le8(x: u64) -> [u8; 8] {
    x.to_le_bytes()
}

// ============================================================
// Blake backend
// ============================================================
#[cfg(feature = "blake")]
mod kat_blake {
    use super::*;

    // Small variant (128-bit): use blake256
    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    mod inner {
        pub use sphincsplus::blake::blake256::{Blakestate256 as State, blake256_init as init, blake256_update as update, blake256_final as finalize};
        pub const OUTPUT_BYTES: usize = 32;
    }
    // Big variant (192/256-bit): use blake512
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mod inner {
        pub use sphincsplus::blake::blake512::{Blakestate512 as State, blake512_init as init, blake512_update as update, blake512_final as finalize};
        pub const OUTPUT_BYTES: usize = 64;
    }

    use inner::*;

    pub struct KatTrCtx { s: State }

    fn do_update(ctx: &mut KatTrCtx, data: &[u8]) {
        unsafe { update(&mut ctx.s, data.as_ptr(), data.len() as u64); }
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx { s: unsafe { core::mem::zeroed() } };
        init(&mut ctx.s);
        do_update(&mut ctx, b"KAT-TRANSCRIPT-v1-BLAKE");
        do_update(&mut ctx, &[0x00]);
        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        do_update(ctx, label);
        do_update(ctx, &[0x00]);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        do_update(ctx, &to_le8(8));
        do_update(ctx, &to_le8(x));
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        do_update(ctx, &to_le8(buf.len() as u64));
        if !buf.is_empty() { do_update(ctx, buf); }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx) -> [u8; 32] {
        let mut outbuf = [0u8; OUTPUT_BYTES];
        unsafe { finalize(&mut ctx.s, outbuf.as_mut_ptr()); }
        let mut out32 = [0u8; 32];
        out32.copy_from_slice(&outbuf[..32]);
        out32
    }
}

// ============================================================
// SHA2 backend
// ============================================================
#[cfg(feature = "sha2")]
mod kat_sha2 {
    use super::*;
    use sphincsplus::sha2::sha2::*;

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    mod consts {
        pub const BLOCK_BYTES: usize = 64;
        pub const OUTPUT_BYTES: usize = 32;
    }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mod consts {
        pub const BLOCK_BYTES: usize = 128;
        pub const OUTPUT_BYTES: usize = 64;
    }
    use consts::*;

    pub struct KatTrCtx { s: [u8; 72] } // max state size

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    fn inc_init(state: &mut [u8]) { unsafe { sha256_inc_init(state.as_mut_ptr()); } }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn inc_init(state: &mut [u8]) { unsafe { sha512_inc_init(state.as_mut_ptr()); } }

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    fn inc_blocks(state: &mut [u8], data: &[u8], n: usize) { unsafe { sha256_inc_blocks(state.as_mut_ptr(), data.as_ptr(), n); } }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn inc_blocks(state: &mut [u8], data: &[u8], n: usize) { unsafe { sha512_inc_blocks(state.as_mut_ptr(), data.as_ptr(), n); } }

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    fn inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) { unsafe { sha256_inc_finalize(out.as_mut_ptr(), state.as_mut_ptr(), data.as_ptr(), inlen); } }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) { unsafe { sha512_inc_finalize(out.as_mut_ptr(), state.as_mut_ptr(), data.as_ptr(), inlen); } }

    pub fn kat_tr_init() -> KatTrCtx {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; 128];
        block[..tag.len()].copy_from_slice(tag);
        let mut ctx = KatTrCtx { s: [0u8; 72] };
        inc_init(&mut ctx.s);
        inc_blocks(&mut ctx.s, &block[..BLOCK_BYTES], 1);
        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let n = label.len();
        let block_count = (n + 1 + (BLOCK_BYTES - 1)) / BLOCK_BYTES;
        for i in 0..block_count {
            let mut block = [0u8; 128];
            let mut j = 0;
            while i * BLOCK_BYTES + j < n && j < BLOCK_BYTES {
                block[j] = label[i * BLOCK_BYTES + j];
                j += 1;
            }
            if i * BLOCK_BYTES + j == n && j < BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }
            let _ = j;
            inc_blocks(&mut ctx.s, &block[..BLOCK_BYTES], 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; 128];
        block[..8].copy_from_slice(&to_le8(8));
        block[8..16].copy_from_slice(&to_le8(x));
        inc_blocks(&mut ctx.s, &block[..BLOCK_BYTES], 1);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle_block = [0u8; 128];
        lenle_block[..8].copy_from_slice(&to_le8(len as u64));
        inc_blocks(&mut ctx.s, &lenle_block[..BLOCK_BYTES], 1);
        if len != 0 {
            let block_count = (len + (BLOCK_BYTES - 1)) / BLOCK_BYTES;
            for i in 0..block_count {
                let mut block = [0u8; 128];
                let mut j = 0;
                while i * BLOCK_BYTES + j < len && j < BLOCK_BYTES {
                    block[j] = buf[i * BLOCK_BYTES + j];
                    j += 1;
                }
                inc_blocks(&mut ctx.s, &block[..BLOCK_BYTES], 1);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx) -> [u8; 32] {
        let mut outbuf = [0u8; 64];
        let final_block = [0u8; 128];
        inc_finalize(&mut outbuf[..OUTPUT_BYTES], &mut ctx.s, &final_block[..BLOCK_BYTES], 1);
        let mut out32 = [0u8; 32];
        out32.copy_from_slice(&outbuf[..32]);
        out32
    }
}

// ============================================================
// SHAKE backend - self-contained keccak implementation since
// the library's shake functions are pub(crate)
// ============================================================
#[cfg(feature = "shake")]
mod kat_shake {
    use super::*;
    use sphincsplus::shake::fips202::*;

    pub struct KatTrCtx { s: [u64; 26] }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx { s: [0u64; 26] };
        shake256_inc_init(&mut ctx.s);
        shake256_inc_absorb(&mut ctx.s, b"KAT-TRANSCRIPT-v1-SHAKE");
        shake256_inc_absorb(&mut ctx.s, &[0x00]);
        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        shake256_inc_absorb(&mut ctx.s, label);
        shake256_inc_absorb(&mut ctx.s, &[0x00]);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        shake256_inc_absorb(&mut ctx.s, &to_le8(8));
        shake256_inc_absorb(&mut ctx.s, &to_le8(x));
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        shake256_inc_absorb(&mut ctx.s, &to_le8(buf.len() as u64));
        if !buf.is_empty() { shake256_inc_absorb(&mut ctx.s, buf); }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx) -> [u8; 32] {
        let mut out32 = [0u8; 32];
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(&mut out32, 32, &mut ctx.s);
        out32
    }
}

// ============================================================
// Haraka backend
// ============================================================
#[cfg(feature = "haraka")]
mod kat_haraka {
    use super::*;
    use sphincsplus::context::SpxCtx;
    use sphincsplus::haraka::haraka::*;

    pub struct KatTrCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx { inner: SpxCtx::new(), s: [0u8; 65] };
        unsafe {
            SPX_tweak_constants(&mut ctx.inner);
            SPX_haraka_S_inc_init(ctx.s.as_mut_ptr());
            let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len(), &ctx.inner);
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), [0x00u8].as_ptr(), 1, &ctx.inner);
        }
        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), label.as_ptr(), label.len(), &ctx.inner);
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), [0x00u8].as_ptr(), 1, &ctx.inner);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let lenle = to_le8(8);
        let le = to_le8(x);
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8, &ctx.inner);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let lenle = to_le8(buf.len() as u64);
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            if !buf.is_empty() {
                SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), buf.len(), &ctx.inner);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx) -> [u8; 32] {
        let mut out32 = [0u8; 32];
        unsafe {
            SPX_haraka_S_inc_finalize(ctx.s.as_mut_ptr());
            SPX_haraka_S_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr(), &ctx.inner);
        }
        out32
    }
}

// ============================================================
// Main
// ============================================================
fn main() {
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    for i in 0..48 { entropy_input[i] = i as u8; }
    unsafe { randombytes_init(entropy_input.as_ptr(), core::ptr::null()); }

    #[cfg(feature = "blake")]
    let mut tctx = kat_blake::kat_tr_init();
    #[cfg(feature = "sha2")]
    let mut tctx = kat_sha2::kat_tr_init();
    #[cfg(feature = "shake")]
    let mut tctx = kat_shake::kat_tr_init();
    #[cfg(feature = "haraka")]
    let mut tctx = kat_haraka::kat_tr_init();

    macro_rules! absorb_label { ($l:expr) => {
        #[cfg(feature = "blake")] kat_blake::kat_tr_absorb_label(&mut tctx, $l);
        #[cfg(feature = "sha2")] kat_sha2::kat_tr_absorb_label(&mut tctx, $l);
        #[cfg(feature = "shake")] kat_shake::kat_tr_absorb_label(&mut tctx, $l);
        #[cfg(feature = "haraka")] kat_haraka::kat_tr_absorb_label(&mut tctx, $l);
    }}
    macro_rules! absorb_u64 { ($x:expr) => {
        #[cfg(feature = "blake")] kat_blake::kat_tr_absorb_u64(&mut tctx, $x);
        #[cfg(feature = "sha2")] kat_sha2::kat_tr_absorb_u64(&mut tctx, $x);
        #[cfg(feature = "shake")] kat_shake::kat_tr_absorb_u64(&mut tctx, $x);
        #[cfg(feature = "haraka")] kat_haraka::kat_tr_absorb_u64(&mut tctx, $x);
    }}
    macro_rules! absorb_bytes { ($b:expr) => {
        #[cfg(feature = "blake")] kat_blake::kat_tr_absorb_bytes(&mut tctx, $b);
        #[cfg(feature = "sha2")] kat_sha2::kat_tr_absorb_bytes(&mut tctx, $b);
        #[cfg(feature = "shake")] kat_shake::kat_tr_absorb_bytes(&mut tctx, $b);
        #[cfg(feature = "haraka")] kat_haraka::kat_tr_absorb_bytes(&mut tctx, $b);
    }}

    absorb_label!(b"CRYPTO_ALGNAME");
    absorb_bytes!(CRYPTO_ALGNAME);
    absorb_label!(b"SKBYTES"); absorb_u64!(CRYPTO_SECRETKEYBYTES as u64);
    absorb_label!(b"PKBYTES"); absorb_u64!(CRYPTO_PUBLICKEYBYTES as u64);
    absorb_label!(b"SIGBYTES"); absorb_u64!(CRYPTO_BYTES as u64);

    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut msg = [0u8; BASE_MLEN * LOOP_COUNT];
    let mut m = [0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = [0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = [0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];

    for i in 0..LOOP_COUNT {
        unsafe { rng_randombytes(seed.as_mut_ptr(), 48); }

        absorb_label!(b"count"); absorb_u64!(i as u64);
        absorb_label!(b"seed"); absorb_bytes!(&seed);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        absorb_label!(b"mlen"); absorb_u64!(mlen);

        unsafe { rng_randombytes(msg.as_mut_ptr(), mlen); }
        absorb_label!(b"msg"); absorb_bytes!(&msg[..mlen as usize]);

        m[..mlen as usize].fill(0);
        m1[..(mlen as usize + CRYPTO_BYTES)].fill(0);
        sm[..(mlen as usize + CRYPTO_BYTES)].fill(0);
        m[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        let ret = unsafe { crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        if ret != 0 { eprintln!("crypto_sign_keypair={}", ret); std::process::exit(-2); }
        absorb_label!(b"pk"); absorb_bytes!(&pk);
        absorb_label!(b"sk"); absorb_bytes!(&sk);

        let mut smlen: u64 = 0;
        let ret = unsafe { crypto_sign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen, sk.as_ptr()) };
        if ret != 0 { eprintln!("crypto_sign={}", ret); std::process::exit(-2); }
        absorb_label!(b"smlen"); absorb_u64!(smlen);
        absorb_label!(b"sm"); absorb_bytes!(&sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = unsafe { crypto_sign_open(m1.as_mut_ptr(), &mut mlen1, sm.as_ptr(), smlen, pk.as_ptr()) };
        if ret != 0 { eprintln!("crypto_sign_open={}", ret); std::process::exit(-2); }
        if mlen1 != mlen { eprintln!("mlen mismatch"); std::process::exit(-2); }
        if m[..mlen as usize] != m1[..mlen as usize] { eprintln!("m mismatch"); std::process::exit(-2); }
    }

    #[cfg(feature = "blake")]
    let digest = kat_blake::kat_tr_final(&mut tctx);
    #[cfg(feature = "sha2")]
    let digest = kat_sha2::kat_tr_final(&mut tctx);
    #[cfg(feature = "shake")]
    let digest = kat_shake::kat_tr_final(&mut tctx);
    #[cfg(feature = "haraka")]
    let digest = kat_haraka::kat_tr_final(&mut tctx);

    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}
