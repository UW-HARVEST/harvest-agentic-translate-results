#[path = "params.rs"]
mod params;
#[path = "context.rs"]
mod context;
#[cfg(feature = "shake")]
#[path = "shake/mod.rs"]
mod shake;
#[cfg(feature = "sha2")]
#[path = "sha2/mod.rs"]
mod sha2;
#[cfg(feature = "blake")]
#[path = "blake/mod.rs"]
mod blake;
#[cfg(feature = "haraka")]
#[path = "haraka/mod.rs"]
mod haraka;
#[path = "hash.rs"]
mod hash;
#[path = "thash.rs"]
mod thash;
#[path = "address.rs"]
mod address;
#[path = "utils.rs"]
mod utils;
#[path = "wots.rs"]
mod wots;
#[path = "wotsx1.rs"]
mod wotsx1;
#[path = "fors.rs"]
mod fors;
#[path = "utilsx1.rs"]
mod utilsx1;
#[path = "merkle.rs"]
mod merkle;
#[path = "sign.rs"]
mod sign;
#[path = "rng.rs"]
mod rng;
#[path = "randombytes.rs"]
mod randombytes;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;
const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// ============================================================
// BLAKE transcript backend
// ============================================================
#[cfg(feature = "blake")]
mod tr {
    use super::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mod inner {
        use crate::blake::blake512::{
            blake512_final, blake512_init, blake512_update, Blakestate512,
        };
        pub const OUTPUT_BYTES: usize = 64;
        pub type State = Blakestate512;
        pub unsafe fn init(s: *mut State) { unsafe { blake512_init(s) } }
        pub unsafe fn update(s: *mut State, data: *const u8, datalen: u64) {
            unsafe { blake512_update(s, data, datalen) }
        }
        pub unsafe fn finalize(s: *mut State, digest: *mut u8) {
            unsafe { blake512_final(s, digest) }
        }
    }

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    mod inner {
        use crate::blake::blake256::{
            blake256_final, blake256_init, blake256_update, Blakestate256,
        };
        pub const OUTPUT_BYTES: usize = 32;
        pub type State = Blakestate256;
        pub unsafe fn init(s: *mut State) { unsafe { blake256_init(s) } }
        pub unsafe fn update(s: *mut State, data: *const u8, datalen: u64) {
            unsafe { blake256_update(s, data, datalen) }
        }
        pub unsafe fn finalize(s: *mut State, digest: *mut u8) {
            unsafe { blake256_final(s, digest) }
        }
    }

    pub struct KatTrCtx {
        state: inner::State,
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        unsafe {
            inner::init(&mut ctx.state);
            let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
            inner::update(&mut ctx.state, tag.as_ptr(), tag.len() as u64);
            let sep: u8 = 0x00;
            inner::update(&mut ctx.state, &sep, 1);
        }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        unsafe {
            inner::update(&mut ctx.state, label.as_ptr(), label.len() as u64);
            let sep: u8 = 0x00;
            inner::update(&mut ctx.state, &sep, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();
        unsafe {
            inner::update(&mut ctx.state, lenle.as_ptr(), 8);
            inner::update(&mut ctx.state, le.as_ptr(), 8);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        unsafe {
            inner::update(&mut ctx.state, lenle.as_ptr(), 8);
            if !buf.is_empty() {
                inner::update(&mut ctx.state, buf.as_ptr(), buf.len() as u64);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; inner::OUTPUT_BYTES];
        unsafe {
            inner::finalize(&mut ctx.state, outbuf.as_mut_ptr());
        }
        out32.copy_from_slice(&outbuf[..32]);
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx {
            state: unsafe { std::mem::zeroed() },
        }
    }
}

// ============================================================
// HARAKA transcript backend
// ============================================================
#[cfg(feature = "haraka")]
mod tr {
    use super::*;
    use crate::context::SpxCtx;
    use crate::haraka::haraka::{
        SPX_haraka_S_inc_absorb, SPX_haraka_S_inc_finalize, SPX_haraka_S_inc_init,
        SPX_haraka_S_inc_squeeze, SPX_tweak_constants,
    };

    pub struct KatTrCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }
        unsafe {
            SPX_tweak_constants(&mut ctx.inner);
            SPX_haraka_S_inc_init(ctx.s.as_mut_ptr());
            let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len(), &ctx.inner);
            let sep: u8 = 0x00;
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1, &ctx.inner);
        }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), label.as_ptr(), label.len(), &ctx.inner);
            let sep: u8 = 0x00;
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1, &ctx.inner);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8, &ctx.inner);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            if !buf.is_empty() {
                SPX_haraka_S_inc_absorb(
                    ctx.s.as_mut_ptr(),
                    buf.as_ptr(),
                    buf.len(),
                    &ctx.inner,
                );
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        unsafe {
            SPX_haraka_S_inc_finalize(ctx.s.as_mut_ptr());
            SPX_haraka_S_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr(), &ctx.inner);
        }
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx {
            inner: unsafe { std::mem::zeroed() },
            s: [0u8; 65],
        }
    }
}

// ============================================================
// SHA2 transcript backend
// ============================================================
#[cfg(feature = "sha2")]
mod tr {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mod inner {
        use crate::sha2::sha2::{sha512_inc_blocks, sha512_inc_finalize, sha512_inc_init};
        pub const STATE_LEN: usize = 72;
        pub const BLOCK_BYTES: usize = 128;
        pub const OUTPUT_BYTES: usize = 64;
        pub unsafe fn inc_init(state: *mut u8) { unsafe { sha512_inc_init(state) } }
        pub unsafe fn inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
            unsafe { sha512_inc_blocks(state, inp, inblocks) }
        }
        pub unsafe fn inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
            unsafe { sha512_inc_finalize(out, state, inp, inlen) }
        }
    }

    #[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    mod inner {
        use crate::sha2::sha2::{sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init};
        pub const STATE_LEN: usize = 40;
        pub const BLOCK_BYTES: usize = 64;
        pub const OUTPUT_BYTES: usize = 32;
        pub unsafe fn inc_init(state: *mut u8) { unsafe { sha256_inc_init(state) } }
        pub unsafe fn inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
            unsafe { sha256_inc_blocks(state, inp, inblocks) }
        }
        pub unsafe fn inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
            unsafe { sha256_inc_finalize(out, state, inp, inlen) }
        }
    }

    pub struct KatTrCtx {
        s: [u8; inner::STATE_LEN],
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; inner::BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        unsafe {
            inner::inc_init(ctx.s.as_mut_ptr());
            inner::inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
        }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let n = label.len();
        let block_count = (n + 1 + (inner::BLOCK_BYTES - 1)) / inner::BLOCK_BYTES;

        for i in 0..block_count {
            let mut block = [0u8; inner::BLOCK_BYTES];
            let mut j = 0;
            while i * inner::BLOCK_BYTES + j < n && j < inner::BLOCK_BYTES {
                block[j] = label[i * inner::BLOCK_BYTES + j];
                j += 1;
            }
            if i * inner::BLOCK_BYTES + j == n && j < inner::BLOCK_BYTES {
                block[j] = 0x00;
                // j += 1; // rest already zero
            }
            unsafe {
                inner::inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
            }
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; inner::BLOCK_BYTES];
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();
        block[..8].copy_from_slice(&lenle);
        block[8..16].copy_from_slice(&le);
        unsafe {
            inner::inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenblock = [0u8; inner::BLOCK_BYTES];
        let lenle = (len as u64).to_le_bytes();
        lenblock[..8].copy_from_slice(&lenle);
        unsafe {
            inner::inc_blocks(ctx.s.as_mut_ptr(), lenblock.as_ptr(), 1);
        }

        if len != 0 {
            let block_count = (len + (inner::BLOCK_BYTES - 1)) / inner::BLOCK_BYTES;
            for i in 0..block_count {
                let mut block = [0u8; inner::BLOCK_BYTES];
                let mut j = 0;
                while i * inner::BLOCK_BYTES + j < len && j < inner::BLOCK_BYTES {
                    block[j] = buf[i * inner::BLOCK_BYTES + j];
                    j += 1;
                }
                unsafe {
                    inner::inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
                }
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; inner::OUTPUT_BYTES];
        let final_block = [0u8; inner::BLOCK_BYTES];
        unsafe {
            inner::inc_finalize(
                outbuf.as_mut_ptr(),
                ctx.s.as_mut_ptr(),
                final_block.as_ptr(),
                1,
            );
        }
        out32.copy_from_slice(&outbuf[..32]);
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx {
            s: [0u8; inner::STATE_LEN],
        }
    }
}

// ============================================================
// SHAKE transcript backend
// ============================================================
#[cfg(feature = "shake")]
mod tr {
    use crate::shake::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    pub struct KatTrCtx {
        s: [u64; 26],
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        unsafe {
            shake256_inc_init(ctx.s.as_mut_ptr());
            let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
            shake256_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len());
            let sep: u8 = 0x00;
            shake256_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1);
        }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), label.as_ptr(), label.len());
            let sep: u8 = 0x00;
            shake256_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            shake256_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            if !buf.is_empty() {
                shake256_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), buf.len());
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        unsafe {
            shake256_inc_finalize(ctx.s.as_mut_ptr());
            shake256_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr());
        }
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx { s: [0u64; 26] }
    }
}

fn main() {
    static mut M: [u8; BASE_MLEN * LOOP_COUNT] = [0u8; BASE_MLEN * LOOP_COUNT];
    static mut SM: [u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES] =
        [0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    static mut M1: [u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES] =
        [0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    static mut PK: [u8; CRYPTO_PUBLICKEYBYTES] = [0u8; CRYPTO_PUBLICKEYBYTES];
    static mut SK: [u8; CRYPTO_SECRETKEYBYTES] = [0u8; CRYPTO_SECRETKEYBYTES];
    static mut SEED: [u8; 48] = [0u8; 48];
    static mut ENTROPY_INPUT: [u8; 48] = [0u8; 48];
    static mut MSG: [u8; BASE_MLEN * LOOP_COUNT] = [0u8; BASE_MLEN * LOOP_COUNT];

    unsafe {
        for i in 0..48 {
            ENTROPY_INPUT[i] = i as u8;
        }
        rng::randombytes_init(ENTROPY_INPUT.as_mut_ptr(), std::ptr::null_mut());

        let mut tctx = tr::new_ctx();
        tr::kat_tr_init(&mut tctx);
        tr::kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
        tr::kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME);
        tr::kat_tr_absorb_label(&mut tctx, b"SKBYTES");
        tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
        tr::kat_tr_absorb_label(&mut tctx, b"PKBYTES");
        tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
        tr::kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
        tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

        for i in 0..LOOP_COUNT {
            rng::randombytes(SEED.as_mut_ptr(), 48);

            tr::kat_tr_absorb_label(&mut tctx, b"count");
            tr::kat_tr_absorb_u64(&mut tctx, i as u64);
            tr::kat_tr_absorb_label(&mut tctx, b"seed");
            tr::kat_tr_absorb_bytes(&mut tctx, &SEED[..48]);

            let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
            if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
                eprintln!("mlen overflow");
                std::process::exit(-1);
            }

            tr::kat_tr_absorb_label(&mut tctx, b"mlen");
            tr::kat_tr_absorb_u64(&mut tctx, mlen);

            rng::randombytes(MSG.as_mut_ptr(), mlen);
            tr::kat_tr_absorb_label(&mut tctx, b"msg");
            tr::kat_tr_absorb_bytes(&mut tctx, &MSG[..mlen as usize]);

            std::ptr::write_bytes(M.as_mut_ptr(), 0, mlen as usize);
            std::ptr::write_bytes(M1.as_mut_ptr(), 0, mlen as usize + CRYPTO_BYTES);
            std::ptr::write_bytes(SM.as_mut_ptr(), 0, mlen as usize + CRYPTO_BYTES);
            std::ptr::copy_nonoverlapping(MSG.as_ptr(), M.as_mut_ptr(), mlen as usize);

            let ret = sign::crypto_sign_keypair(PK.as_mut_ptr(), SK.as_mut_ptr());
            if ret != 0 {
                eprintln!("crypto_sign_keypair={}", ret);
                std::process::exit(-2);
            }
            tr::kat_tr_absorb_label(&mut tctx, b"pk");
            tr::kat_tr_absorb_bytes(&mut tctx, &PK[..CRYPTO_PUBLICKEYBYTES]);
            tr::kat_tr_absorb_label(&mut tctx, b"sk");
            tr::kat_tr_absorb_bytes(&mut tctx, &SK[..CRYPTO_SECRETKEYBYTES]);

            let mut smlen: u64 = 0;
            let ret = sign::crypto_sign(
                SM.as_mut_ptr(),
                &mut smlen,
                M.as_ptr(),
                mlen,
                SK.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign={}", ret);
                std::process::exit(-2);
            }
            tr::kat_tr_absorb_label(&mut tctx, b"smlen");
            tr::kat_tr_absorb_u64(&mut tctx, smlen);
            tr::kat_tr_absorb_label(&mut tctx, b"sm");
            tr::kat_tr_absorb_bytes(&mut tctx, &SM[..smlen as usize]);

            let mut mlen1: u64 = 0;
            let ret = sign::crypto_sign_open(
                M1.as_mut_ptr(),
                &mut mlen1,
                SM.as_ptr(),
                smlen,
                PK.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign_open={}", ret);
                std::process::exit(-2);
            }
            if mlen1 != mlen {
                eprintln!("mlen mismatch");
                std::process::exit(-2);
            }
            if M[..mlen as usize] != M1[..mlen as usize] {
                eprintln!("m mismatch");
                std::process::exit(-2);
            }
        }

        let mut digest = [0u8; 32];
        tr::kat_tr_final(&mut tctx, &mut digest);

        print!("KAT transcript digest = ");
        for b in &digest {
            print!("{:02X}", b);
        }
        println!();
    }
}
