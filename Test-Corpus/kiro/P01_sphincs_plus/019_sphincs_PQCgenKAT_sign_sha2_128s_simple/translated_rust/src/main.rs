#![allow(non_snake_case, non_upper_case_globals, unused, non_camel_case_types)]

#[path = "params.rs"]
mod params;

mod context {
    use crate::params::*;
    #[repr(C)]
    pub struct SpxCtx {
        pub pub_seed: [u8; SPX_N],
        pub sk_seed: [u8; SPX_N],
        #[cfg(feature = "sha2")]
        pub state_seeded: [u8; 40],
        #[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
        pub state_seeded_512: [u8; 72],
        #[cfg(feature = "haraka")]
        pub tweaked512_rc64: [[u64; 8]; 10],
        #[cfg(feature = "haraka")]
        pub tweaked256_rc32: [[u32; 8]; 10],
    }
}

#[path = "address.rs"]
mod address;
#[path = "utils.rs"]
mod utils;

mod hash {
    #[cfg(feature = "shake")]
    pub use crate::shake_backend::{initialize_hash_function, gen_message_random, hash_message, prf_addr};
    #[cfg(feature = "sha2")]
    pub use crate::sha2_backend::{initialize_hash_function, gen_message_random, hash_message, prf_addr};
    #[cfg(feature = "blake")]
    pub use crate::blake_backend::{initialize_hash_function, gen_message_random, hash_message, prf_addr};
    #[cfg(feature = "haraka")]
    pub use crate::haraka_backend::{initialize_hash_function, gen_message_random, hash_message, prf_addr};
}

mod thash {
    #[cfg(feature = "shake")]
    pub use crate::shake_backend::thash;
    #[cfg(feature = "sha2")]
    pub use crate::sha2_backend::thash;
    #[cfg(feature = "blake")]
    pub use crate::blake_backend::thash;
    #[cfg(feature = "haraka")]
    pub use crate::haraka_backend::thash;
}

#[path = "wots.rs"]
mod wots;
#[path = "fors.rs"]
mod fors;
#[path = "wotsx1.rs"]
mod wotsx1;
#[path = "utilsx1.rs"]
mod utilsx1;
#[path = "merkle.rs"]
mod merkle;
#[path = "rng.rs"]
mod rng;
#[path = "sign.rs"]
mod sign;

// Backends that need private function access use include! with transcript code inline.
// Blake uses #[path] since it has #![allow(..)] inner attribute.

#[cfg(feature = "shake")]
mod shake_backend {
    include!("shake_backend.rs");

    // Transcript
    pub struct TrCtx { pub s: [u64; 26] }
    impl TrCtx { pub fn new() -> Self { TrCtx { s: [0u64; 26] } } }
    pub fn tr_init(c: &mut TrCtx) {
        shake256_inc_init(&mut c.s);
        shake256_inc_absorb(&mut c.s, b"KAT-TRANSCRIPT-v1-SHAKE", 24);
        shake256_inc_absorb(&mut c.s, &[0u8], 1);
    }
    pub fn tr_label(c: &mut TrCtx, l: &str) {
        shake256_inc_absorb(&mut c.s, l.as_bytes(), l.len());
        shake256_inc_absorb(&mut c.s, &[0u8], 1);
    }
    pub fn tr_u64(c: &mut TrCtx, x: u64) {
        shake256_inc_absorb(&mut c.s, &8u64.to_le_bytes(), 8);
        shake256_inc_absorb(&mut c.s, &x.to_le_bytes(), 8);
    }
    pub fn tr_bytes(c: &mut TrCtx, b: &[u8]) {
        shake256_inc_absorb(&mut c.s, &(b.len() as u64).to_le_bytes(), 8);
        if !b.is_empty() { shake256_inc_absorb(&mut c.s, b, b.len()); }
    }
    pub fn tr_final(c: &mut TrCtx, out: &mut [u8; 32]) {
        shake256_inc_finalize(&mut c.s);
        shake256_inc_squeeze(out, 32, &mut c.s);
    }
}

#[cfg(feature = "sha2")]
mod sha2_backend {
    include!("sha2_backend.rs");

    // Transcript
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const TR_BLK: usize = 128;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const TR_BLK: usize = 64;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const TR_SLEN: usize = 72;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const TR_SLEN: usize = 40;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const TR_OUTLEN: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const TR_OUTLEN: usize = 32;

    fn tr_s_init(s: &mut [u8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_init(s);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_init(s);
    }
    fn tr_s_blocks(s: &mut [u8], d: &[u8], n: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_blocks(s, d, n);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_blocks(s, d, n);
    }
    fn tr_s_finalize(out: &mut [u8], s: &mut [u8], d: &[u8], n: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_finalize(out, s, d, n);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_finalize(out, s, d, n);
    }

    pub struct TrCtx { pub s: [u8; TR_SLEN] }
    impl TrCtx { pub fn new() -> Self { TrCtx { s: [0u8; TR_SLEN] } } }

    pub fn tr_init(c: &mut TrCtx) {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; TR_BLK];
        block[..tag.len()].copy_from_slice(tag);
        tr_s_init(&mut c.s);
        tr_s_blocks(&mut c.s, &block, 1);
    }
    pub fn tr_label(c: &mut TrCtx, l: &str) {
        let p = l.as_bytes();
        let n = p.len();
        let bc = (n + 1 + TR_BLK - 1) / TR_BLK;
        for i in 0..bc {
            let mut block = [0u8; TR_BLK];
            let mut j = 0;
            while i * TR_BLK + j < n && j < TR_BLK { block[j] = p[i * TR_BLK + j]; j += 1; }
            if i * TR_BLK + j == n && j < TR_BLK { block[j] = 0x00; j += 1; }
            tr_s_blocks(&mut c.s, &block, 1);
        }
    }
    pub fn tr_u64(c: &mut TrCtx, x: u64) {
        let mut block = [0u8; TR_BLK];
        block[..8].copy_from_slice(&8u64.to_le_bytes());
        block[8..16].copy_from_slice(&x.to_le_bytes());
        tr_s_blocks(&mut c.s, &block, 1);
    }
    pub fn tr_bytes(c: &mut TrCtx, buf: &[u8]) {
        let mut lb = [0u8; TR_BLK];
        lb[..8].copy_from_slice(&(buf.len() as u64).to_le_bytes());
        tr_s_blocks(&mut c.s, &lb, 1);
        if !buf.is_empty() {
            let bc = (buf.len() + TR_BLK - 1) / TR_BLK;
            for i in 0..bc {
                let mut block = [0u8; TR_BLK];
                let mut j = 0;
                while i * TR_BLK + j < buf.len() && j < TR_BLK { block[j] = buf[i * TR_BLK + j]; j += 1; }
                tr_s_blocks(&mut c.s, &block, 1);
            }
        }
    }
    pub fn tr_final(c: &mut TrCtx, out: &mut [u8; 32]) {
        let mut outbuf = [0u8; TR_OUTLEN];
        let fb = [0u8; TR_BLK];
        tr_s_finalize(&mut outbuf, &mut c.s, &fb, TR_BLK);
        out.copy_from_slice(&outbuf[..32]);
    }
}

#[cfg(feature = "blake")]
#[path = "blake_backend.rs"]
mod blake_backend;

#[cfg(feature = "blake")]
mod blake_tr {
    use crate::blake_backend::*;
    use crate::params::SPX_N;

    pub struct TrCtx {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        s: BlakeState512,
        #[cfg(any(feature = "128s", feature = "128f"))]
        s: BlakeState256,
    }
    impl TrCtx {
        pub fn new() -> Self {
            TrCtx {
                #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
                s: BlakeState512 { h:[0;8],s:[0;4],t:[0;2],buflen:0,nullt:0,buf:[0;128] },
                #[cfg(any(feature = "128s", feature = "128f"))]
                s: BlakeState256 { h:[0;8],s:[0;4],t:[0;2],buflen:0,nullt:0,buf:[0;64] },
            }
        }
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn upd(s: &mut BlakeState512, d: &[u8]) { blake512_update(s, d, (d.len() as u64) * 8); }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn upd(s: &mut BlakeState256, d: &[u8]) { blake256_update(s, d, (d.len() as u64) * 8); }

    pub fn tr_init(c: &mut TrCtx) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_init(&mut c.s);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_init(&mut c.s);
        upd(&mut c.s, b"KAT-TRANSCRIPT-v1-BLAKE");
        upd(&mut c.s, &[0u8]);
    }
    pub fn tr_label(c: &mut TrCtx, l: &str) { upd(&mut c.s, l.as_bytes()); upd(&mut c.s, &[0u8]); }
    pub fn tr_u64(c: &mut TrCtx, x: u64) { upd(&mut c.s, &8u64.to_le_bytes()); upd(&mut c.s, &x.to_le_bytes()); }
    pub fn tr_bytes(c: &mut TrCtx, b: &[u8]) {
        upd(&mut c.s, &(b.len() as u64).to_le_bytes());
        if !b.is_empty() { upd(&mut c.s, b); }
    }
    pub fn tr_final(c: &mut TrCtx, out: &mut [u8; 32]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        { let mut t = [0u8; 64]; blake512_final(&mut c.s, &mut t); out.copy_from_slice(&t[..32]); }
        #[cfg(any(feature = "128s", feature = "128f"))]
        { let mut t = [0u8; 32]; blake256_final(&mut c.s, &mut t); out.copy_from_slice(&t); }
    }
}

#[cfg(feature = "haraka")]
mod haraka_backend {
    include!("haraka_backend.rs");

    // Transcript
    pub struct TrCtx { pub inner: SpxCtx, pub s: [u8; 65] }
    impl TrCtx {
        pub fn new() -> Self { TrCtx { inner: unsafe { std::mem::zeroed() }, s: [0u8; 65] } }
    }
    pub fn tr_init(c: &mut TrCtx) {
        for i in 0..SPX_N { c.inner.pub_seed[i] = 0; c.inner.sk_seed[i] = 0; }
        tweak_constants(&mut c.inner);
        haraka_S_inc_init(&mut c.s);
        haraka_S_inc_absorb(&mut c.s, b"KAT-TRANSCRIPT-v1-HARAKA", 24, &c.inner);
        haraka_S_inc_absorb(&mut c.s, &[0u8], 1, &c.inner);
    }
    pub fn tr_label(c: &mut TrCtx, l: &str) {
        haraka_S_inc_absorb(&mut c.s, l.as_bytes(), l.len(), &c.inner);
        haraka_S_inc_absorb(&mut c.s, &[0u8], 1, &c.inner);
    }
    pub fn tr_u64(c: &mut TrCtx, x: u64) {
        haraka_S_inc_absorb(&mut c.s, &8u64.to_le_bytes(), 8, &c.inner);
        haraka_S_inc_absorb(&mut c.s, &x.to_le_bytes(), 8, &c.inner);
    }
    pub fn tr_bytes(c: &mut TrCtx, b: &[u8]) {
        haraka_S_inc_absorb(&mut c.s, &(b.len() as u64).to_le_bytes(), 8, &c.inner);
        if !b.is_empty() { haraka_S_inc_absorb(&mut c.s, b, b.len(), &c.inner); }
    }
    pub fn tr_final(c: &mut TrCtx, out: &mut [u8; 32]) {
        haraka_S_inc_finalize(&mut c.s);
        haraka_S_inc_squeeze(out, 32, &mut c.s, &c.inner);
    }
}

// ============================================================
// Main
// ============================================================

use params::*;

fn main() {
    const BASE_MLEN: usize = 33;
    const LOOP_COUNT: usize = 7;

    let mut entropy_input = [0u8; 48];
    for i in 0..48 { entropy_input[i] = i as u8; }
    rng::randombytes_init(entropy_input.as_ptr(), std::ptr::null());

    #[cfg(feature = "shake")]
    let mut tctx = shake_backend::TrCtx::new();
    #[cfg(feature = "blake")]
    let mut tctx = blake_tr::TrCtx::new();
    #[cfg(feature = "haraka")]
    let mut tctx = haraka_backend::TrCtx::new();
    #[cfg(feature = "sha2")]
    let mut tctx = sha2_backend::TrCtx::new();

    macro_rules! tr_init { ($c:expr) => {{
        #[cfg(feature = "shake")] shake_backend::tr_init($c);
        #[cfg(feature = "blake")] blake_tr::tr_init($c);
        #[cfg(feature = "haraka")] haraka_backend::tr_init($c);
        #[cfg(feature = "sha2")] sha2_backend::tr_init($c);
    }}}
    macro_rules! tr_label { ($c:expr, $l:expr) => {{
        #[cfg(feature = "shake")] shake_backend::tr_label($c, $l);
        #[cfg(feature = "blake")] blake_tr::tr_label($c, $l);
        #[cfg(feature = "haraka")] haraka_backend::tr_label($c, $l);
        #[cfg(feature = "sha2")] sha2_backend::tr_label($c, $l);
    }}}
    macro_rules! tr_u64 { ($c:expr, $v:expr) => {{
        #[cfg(feature = "shake")] shake_backend::tr_u64($c, $v);
        #[cfg(feature = "blake")] blake_tr::tr_u64($c, $v);
        #[cfg(feature = "haraka")] haraka_backend::tr_u64($c, $v);
        #[cfg(feature = "sha2")] sha2_backend::tr_u64($c, $v);
    }}}
    macro_rules! tr_bytes { ($c:expr, $b:expr) => {{
        #[cfg(feature = "shake")] shake_backend::tr_bytes($c, $b);
        #[cfg(feature = "blake")] blake_tr::tr_bytes($c, $b);
        #[cfg(feature = "haraka")] haraka_backend::tr_bytes($c, $b);
        #[cfg(feature = "sha2")] sha2_backend::tr_bytes($c, $b);
    }}}
    macro_rules! tr_final { ($c:expr, $o:expr) => {{
        #[cfg(feature = "shake")] shake_backend::tr_final($c, $o);
        #[cfg(feature = "blake")] blake_tr::tr_final($c, $o);
        #[cfg(feature = "haraka")] haraka_backend::tr_final($c, $o);
        #[cfg(feature = "sha2")] sha2_backend::tr_final($c, $o);
    }}}

    tr_init!(&mut tctx);
    tr_label!(&mut tctx, "CRYPTO_ALGNAME");
    tr_bytes!(&mut tctx, b"SPHINCS+");
    tr_label!(&mut tctx, "SKBYTES");
    tr_u64!(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    tr_label!(&mut tctx, "PKBYTES");
    tr_u64!(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    tr_label!(&mut tctx, "SIGBYTES");
    tr_u64!(&mut tctx, CRYPTO_BYTES as u64);

    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];

    for i in 0..LOOP_COUNT {
        rng::randombytes_rust(&mut seed);

        tr_label!(&mut tctx, "count");
        tr_u64!(&mut tctx, i as u64);
        tr_label!(&mut tctx, "seed");
        tr_bytes!(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tr_label!(&mut tctx, "mlen");
        tr_u64!(&mut tctx, mlen as u64);

        rng::randombytes_rust(&mut msg[..mlen]);
        tr_label!(&mut tctx, "msg");
        tr_bytes!(&mut tctx, &msg[..mlen]);

        for b in sm.iter_mut() { *b = 0; }
        for b in m1.iter_mut() { *b = 0; }

        let ret = unsafe { sign::crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        if ret != 0 { eprintln!("crypto_sign_keypair={}", ret); std::process::exit(-2); }
        tr_label!(&mut tctx, "pk");
        tr_bytes!(&mut tctx, &pk);
        tr_label!(&mut tctx, "sk");
        tr_bytes!(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let ret = unsafe {
            sign::crypto_sign(sm.as_mut_ptr(), &mut smlen, msg.as_ptr(), mlen as u64, sk.as_ptr())
        };
        if ret != 0 { eprintln!("crypto_sign={}", ret); std::process::exit(-2); }
        tr_label!(&mut tctx, "smlen");
        tr_u64!(&mut tctx, smlen);
        tr_label!(&mut tctx, "sm");
        tr_bytes!(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = unsafe {
            sign::crypto_sign_open(m1.as_mut_ptr(), &mut mlen1, sm.as_ptr(), smlen, pk.as_ptr())
        };
        if ret != 0 { eprintln!("crypto_sign_open={}", ret); std::process::exit(-2); }
        if mlen1 as usize != mlen { eprintln!("mlen mismatch"); std::process::exit(-2); }
        if msg[..mlen] != m1[..mlen] { eprintln!("m mismatch"); std::process::exit(-2); }
    }

    let mut digest = [0u8; 32];
    tr_final!(&mut tctx, &mut digest);
    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}
