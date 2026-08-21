//! Translation of `app/src/PQCgenKAT_sign.c`.
//!
//! Performs an in-memory sign/verify test loop and prints a SHAKE256-style
//! transcript digest. The transcript hash uses the same primitive as the
//! selected hash backend (BLAKE / Haraka / SHA2 / SHAKE), matching the C
//! `#ifdef` selection.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use sphincsplus::params::*;
use sphincsplus::rng::{randombytes, randombytes_init};
use sphincsplus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

// ===========================================================================
// SHAKE transcript
// ===========================================================================
#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod kat {
    use sphincsplus::backend::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    pub struct KatCtx {
        s: [u64; 26],
    }

    impl KatCtx {
        pub fn new() -> Self {
            let mut s = [0u64; 26];
            unsafe {
                shake256_inc_init(s.as_mut_ptr());
                let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
                shake256_inc_absorb(s.as_mut_ptr(), tag.as_ptr(), tag.len());
                let sep = [0u8; 1];
                shake256_inc_absorb(s.as_mut_ptr(), sep.as_ptr(), 1);
            }
            KatCtx { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            unsafe {
                let p = label.as_bytes();
                shake256_inc_absorb(self.s.as_mut_ptr(), p.as_ptr(), p.len());
                let sep = [0u8; 1];
                shake256_inc_absorb(self.s.as_mut_ptr(), sep.as_ptr(), 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            unsafe {
                let le = x.to_le_bytes();
                let lenle = 8u64.to_le_bytes();
                shake256_inc_absorb(self.s.as_mut_ptr(), lenle.as_ptr(), 8);
                shake256_inc_absorb(self.s.as_mut_ptr(), le.as_ptr(), 8);
            }
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            unsafe {
                let lenle = (buf.len() as u64).to_le_bytes();
                shake256_inc_absorb(self.s.as_mut_ptr(), lenle.as_ptr(), 8);
                if !buf.is_empty() {
                    shake256_inc_absorb(self.s.as_mut_ptr(), buf.as_ptr(), buf.len());
                }
            }
        }

        pub fn finalize(&mut self) -> [u8; 32] {
            let mut out = [0u8; 32];
            unsafe {
                shake256_inc_finalize(self.s.as_mut_ptr());
                shake256_inc_squeeze(out.as_mut_ptr(), 32, self.s.as_mut_ptr());
            }
            out
        }
    }
}

// ===========================================================================
// SHA2 transcript
// ===========================================================================
#[cfg(feature = "sha2")]
mod kat {
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    mod shax {
        pub use sphincsplus::backend::{
            sha512_inc_blocks as inc_blocks, sha512_inc_finalize as inc_finalize,
            sha512_inc_init as inc_init,
        };
        pub const BLOCK: usize = 128;
        pub const OUT: usize = 64;
        pub const STATE: usize = 72;
    }
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    mod shax {
        pub use sphincsplus::backend::{
            sha256_inc_blocks as inc_blocks, sha256_inc_finalize as inc_finalize,
            sha256_inc_init as inc_init,
        };
        pub const BLOCK: usize = 64;
        pub const OUT: usize = 32;
        pub const STATE: usize = 40;
    }

    use shax::{inc_blocks, inc_finalize, inc_init, BLOCK, OUT, STATE};

    pub struct KatCtx {
        s: [u8; STATE],
    }

    impl KatCtx {
        pub fn new() -> Self {
            let mut s = [0u8; STATE];
            let tag = b"KAT-TRANSCRIPT-v1-SHA2";
            let mut block = [0u8; BLOCK];
            block[..tag.len()].copy_from_slice(tag);
            unsafe {
                inc_init(s.as_mut_ptr());
                inc_blocks(s.as_mut_ptr(), block.as_ptr(), 1);
            }
            KatCtx { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            let block_count = (n + 1 + (BLOCK - 1)) / BLOCK;
            for i in 0..block_count {
                let mut block = [0u8; BLOCK];
                let mut j = 0usize;
                while i * BLOCK + j < n && j < BLOCK {
                    block[j] = p[i * BLOCK + j];
                    j += 1;
                }
                if i * BLOCK + j == n && j < BLOCK {
                    block[j] = 0x00;
                    // j += 1; // no further writes needed (rest is zero)
                }
                unsafe {
                    inc_blocks(self.s.as_mut_ptr(), block.as_ptr(), 1);
                }
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut block = [0u8; BLOCK];
            let le = x.to_le_bytes();
            let lenle = 8u64.to_le_bytes();
            block[0..8].copy_from_slice(&lenle);
            block[8..16].copy_from_slice(&le);
            unsafe {
                inc_blocks(self.s.as_mut_ptr(), block.as_ptr(), 1);
            }
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; BLOCK];
            lenle[0..8].copy_from_slice(&(len as u64).to_le_bytes());
            let block_count = (len + (BLOCK - 1)) / BLOCK;
            unsafe {
                inc_blocks(self.s.as_mut_ptr(), lenle.as_ptr(), 1);
            }
            if len != 0 {
                for i in 0..block_count {
                    let mut block = [0u8; BLOCK];
                    let mut j = 0usize;
                    while i * BLOCK + j < len && j < BLOCK {
                        block[j] = buf[i * BLOCK + j];
                        j += 1;
                    }
                    unsafe {
                        inc_blocks(self.s.as_mut_ptr(), block.as_ptr(), 1);
                    }
                }
            }
        }

        pub fn finalize(&mut self) -> [u8; 32] {
            let mut out = [0u8; 32];
            let mut outbuf = [0u8; OUT];
            let final_block = [0u8; BLOCK];
            unsafe {
                inc_finalize(
                    outbuf.as_mut_ptr(),
                    self.s.as_mut_ptr(),
                    final_block.as_ptr(),
                    1,
                );
            }
            out.copy_from_slice(&outbuf[0..32]);
            out
        }
    }
}

// ===========================================================================
// Haraka transcript
// ===========================================================================
#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "blake")
))]
mod kat {
    use sphincsplus::backend::{
        haraka_S_inc_absorb, haraka_S_inc_finalize, haraka_S_inc_init, haraka_S_inc_squeeze,
        tweak_constants,
    };
    use sphincsplus::SpxCtx;

    pub struct KatCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    impl KatCtx {
        pub fn new() -> Self {
            let mut inner = SpxCtx::new(); // pub_seed / sk_seed already zero
            let mut s = [0u8; 65];
            unsafe {
                tweak_constants(&mut inner);
                haraka_S_inc_init(s.as_mut_ptr());
                let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
                haraka_S_inc_absorb(s.as_mut_ptr(), tag.as_ptr(), tag.len(), &inner);
                let sep = [0u8; 1];
                haraka_S_inc_absorb(s.as_mut_ptr(), sep.as_ptr(), 1, &inner);
            }
            KatCtx { inner, s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            unsafe {
                let p = label.as_bytes();
                haraka_S_inc_absorb(self.s.as_mut_ptr(), p.as_ptr(), p.len(), &self.inner);
                let sep = [0u8; 1];
                haraka_S_inc_absorb(self.s.as_mut_ptr(), sep.as_ptr(), 1, &self.inner);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            unsafe {
                let le = x.to_le_bytes();
                let lenle = 8u64.to_le_bytes();
                haraka_S_inc_absorb(self.s.as_mut_ptr(), lenle.as_ptr(), 8, &self.inner);
                haraka_S_inc_absorb(self.s.as_mut_ptr(), le.as_ptr(), 8, &self.inner);
            }
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            unsafe {
                let lenle = (buf.len() as u64).to_le_bytes();
                haraka_S_inc_absorb(self.s.as_mut_ptr(), lenle.as_ptr(), 8, &self.inner);
                if !buf.is_empty() {
                    haraka_S_inc_absorb(self.s.as_mut_ptr(), buf.as_ptr(), buf.len(), &self.inner);
                }
            }
        }

        pub fn finalize(&mut self) -> [u8; 32] {
            let mut out = [0u8; 32];
            unsafe {
                haraka_S_inc_finalize(self.s.as_mut_ptr());
                haraka_S_inc_squeeze(out.as_mut_ptr(), 32, self.s.as_mut_ptr(), &self.inner);
            }
            out
        }
    }
}

// ===========================================================================
// BLAKE transcript
// ===========================================================================
#[cfg(all(feature = "blake", not(feature = "sha2"), not(feature = "shake")))]
mod kat {
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    mod blakex {
        pub use sphincsplus::backend::{
            blake512_final as final_x, blake512_init as init_x, blake512_update as update_x,
            blakestate512 as StateX,
        };
        pub const OUT: usize = 64;
    }
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    mod blakex {
        pub use sphincsplus::backend::{
            blake256_final as final_x, blake256_init as init_x, blake256_update as update_x,
            blakestate256 as StateX,
        };
        pub const OUT: usize = 32;
    }

    use blakex::{final_x, init_x, update_x, StateX, OUT};

    pub struct KatCtx {
        s: StateX,
    }

    impl KatCtx {
        pub fn new() -> Self {
            let mut s = StateX::new_zeroed();
            unsafe {
                init_x(&mut s);
                let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
                update_x(&mut s, tag.as_ptr(), tag.len() as u64);
                let sep = [0u8; 1];
                update_x(&mut s, sep.as_ptr(), 1);
            }
            KatCtx { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            unsafe {
                let p = label.as_bytes();
                update_x(&mut self.s, p.as_ptr(), p.len() as u64);
                let sep = [0u8; 1];
                update_x(&mut self.s, sep.as_ptr(), 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            unsafe {
                let le = x.to_le_bytes();
                let lenle = 8u64.to_le_bytes();
                update_x(&mut self.s, lenle.as_ptr(), 8);
                update_x(&mut self.s, le.as_ptr(), 8);
            }
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            unsafe {
                let lenle = (buf.len() as u64).to_le_bytes();
                update_x(&mut self.s, lenle.as_ptr(), 8);
                if !buf.is_empty() {
                    update_x(&mut self.s, buf.as_ptr(), buf.len() as u64);
                }
            }
        }

        pub fn finalize(&mut self) -> [u8; 32] {
            let mut out = [0u8; 32];
            let mut outbuf = [0u8; OUT];
            unsafe {
                final_x(&mut self.s, outbuf.as_mut_ptr());
            }
            out.copy_from_slice(&outbuf[0..32]);
            out
        }
    }
}

fn main() {
    let total = BASE_MLEN * LOOP_COUNT;

    let mut m = vec![0u8; total];
    let mut sm = vec![0u8; total + CRYPTO_BYTES];
    let mut m1 = vec![0u8; total + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; total];

    let mut smlen: u64 = 0;
    let mut mlen1: u64 = 0;

    // Deterministic entropy to seed DRBG.
    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    unsafe {
        randombytes_init(entropy_input.as_mut_ptr(), core::ptr::null_mut());
    }

    // Initialize transcript.
    let mut tctx = kat::KatCtx::new();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME.as_bytes());
    tctx.absorb_label("SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        unsafe {
            randombytes(seed.as_mut_ptr(), seed.len() as u64);
        }

        tctx.absorb_label("count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed");
        tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > total {
            eprintln!("mlen overflow");
            std::process::exit((-1i32) as u8 as i32);
        }

        tctx.absorb_label("mlen");
        tctx.absorb_u64(mlen as u64);

        unsafe {
            randombytes(msg.as_mut_ptr(), mlen as u64);
        }
        tctx.absorb_label("msg");
        tctx.absorb_bytes(&msg[..mlen]);

        for b in m[..mlen].iter_mut() {
            *b = 0;
        }
        for b in m1[..mlen + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        for b in sm[..mlen + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = unsafe { crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit((-2i32) as u8 as i32);
        }
        tctx.absorb_label("pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label("sk");
        tctx.absorb_bytes(&sk);

        // Sign
        let ret = unsafe {
            crypto_sign(
                sm.as_mut_ptr(),
                &mut smlen,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit((-2i32) as u8 as i32);
        }
        tctx.absorb_label("smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label("sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        // Verify
        let ret = unsafe {
            crypto_sign_open(
                m1.as_mut_ptr(),
                &mut mlen1,
                sm.as_ptr(),
                smlen,
                pk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit((-2i32) as u8 as i32);
        }
        if mlen1 != mlen as u64 {
            eprintln!("mlen mismatch");
            std::process::exit((-2i32) as u8 as i32);
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            std::process::exit((-2i32) as u8 as i32);
        }
    }

    let digest = tctx.finalize();

    let mut s = String::from("KAT transcript digest = ");
    for b in digest.iter() {
        s.push_str(&format!("{:02X}", b));
    }
    println!("{}", s);
}
