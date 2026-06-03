// Translation of c_src/app/src/PQCgenKAT_sign.c
//
// Performs an in-memory KAT signing/verification loop and prints a
// transcript digest. Uses the deterministic AES256-CTR DRBG from `rng.rs`
// (matching the C `rng.c`).

use sphincs_plus_rs::params::{
    CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES,
};
use sphincs_plus_rs::rng::{randombytes, randombytes_init};
use sphincs_plus_rs::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// Each backend has its own KAT transcript construction. We keep the
// implementations close to the C source.

#[cfg(feature = "haraka")]
mod kat_tr {
    use sphincs_plus_rs::context::SpxCtx;
    use sphincs_plus_rs::haraka::{
        haraka_s_inc_absorb, haraka_s_inc_finalize, haraka_s_inc_init, haraka_s_inc_squeeze,
        tweak_constants,
    };
    use sphincs_plus_rs::params::SPX_N;

    pub struct KatTrCtx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    pub fn init(ctx: &mut KatTrCtx) {
        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }
        tweak_constants(&mut ctx.inner);
        haraka_s_inc_init(&mut ctx.s);

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_s_inc_absorb(&mut ctx.s, tag, tag.len(), &ctx.inner);
        let sep: [u8; 1] = [0x00];
        haraka_s_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let bytes = label.as_bytes();
        haraka_s_inc_absorb(&mut ctx.s, bytes, bytes.len(), &ctx.inner);
        let sep: [u8; 1] = [0x00];
        haraka_s_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = (x >> (8 * i)) as u8;
        }
        let mut lenle = [0u8; 8];
        let l = 8u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &le, 8, &ctx.inner);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        if len != 0 {
            haraka_s_inc_absorb(&mut ctx.s, buf, len, &ctx.inner);
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        haraka_s_inc_finalize(&mut ctx.s);
        let mut tmp = [0u8; 32];
        haraka_s_inc_squeeze(&mut tmp, 32, &mut ctx.s, &ctx.inner);
        out32.copy_from_slice(&tmp);
    }

    pub fn new() -> KatTrCtx {
        KatTrCtx {
            inner: SpxCtx::new(),
            s: [0u8; 65],
        }
    }
}

#[cfg(feature = "shake")]
mod kat_tr {
    use sphincs_plus_rs::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    pub struct KatTrCtx {
        pub s: [u64; 26],
    }

    pub fn init(ctx: &mut KatTrCtx) {
        shake256_inc_init(&mut ctx.s);
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(&mut ctx.s, tag, tag.len());
        let sep = [0x00u8];
        shake256_inc_absorb(&mut ctx.s, &sep, 1);
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let bytes = label.as_bytes();
        shake256_inc_absorb(&mut ctx.s, bytes, bytes.len());
        let sep = [0x00u8];
        shake256_inc_absorb(&mut ctx.s, &sep, 1);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = (x >> (8 * i)) as u8;
        }
        let mut lenle = [0u8; 8];
        let l = 8u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        shake256_inc_absorb(&mut ctx.s, &le, 8);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        if len != 0 {
            shake256_inc_absorb(&mut ctx.s, buf, len);
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, 32, &mut ctx.s);
    }

    pub fn new() -> KatTrCtx {
        KatTrCtx { s: [0u64; 26] }
    }
}

#[cfg(feature = "blake")]
mod kat_tr {
    use sphincs_plus_rs::params::SPX_N;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    use sphincs_plus_rs::blake::{
        blake512_final, blake512_init, blake512_update, BlakeState512,
    };
    #[cfg(any(feature = "128s", feature = "128f"))]
    use sphincs_plus_rs::blake::{
        blake256_final, blake256_init, blake256_update, BlakeState256,
    };

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub struct KatTrCtx {
        pub state: BlakeState512,
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub struct KatTrCtx {
        pub state: BlakeState256,
    }

    // The C kat transcript helper passes byte counts as `datalen` (which
    // blake256/512_update internally treats as bit counts). We mirror this
    // exactly so output matches byte-for-byte.
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn update(ctx: &mut KatTrCtx, data: &[u8], datalen_bytes: usize) {
        blake512_update(&mut ctx.state, data, datalen_bytes as u64);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn update(ctx: &mut KatTrCtx, data: &[u8], datalen_bytes: usize) {
        blake256_update(&mut ctx.state, data, datalen_bytes as u64);
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub fn new() -> KatTrCtx {
        KatTrCtx {
            state: BlakeState512::zero(),
        }
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub fn new() -> KatTrCtx {
        KatTrCtx {
            state: BlakeState256::zero(),
        }
    }

    pub fn init(ctx: &mut KatTrCtx) {
        let _ = SPX_N;
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_init(&mut ctx.state);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_init(&mut ctx.state);

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE";
        update(ctx, tag, tag.len());
        let sep = [0x00u8];
        update(ctx, &sep, 1);
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let bytes = label.as_bytes();
        update(ctx, bytes, bytes.len());
        let sep = [0x00u8];
        update(ctx, &sep, 1);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = (x >> (8 * i)) as u8;
        }
        let mut lenle = [0u8; 8];
        let l = 8u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        update(ctx, &lenle, 8);
        update(ctx, &le, 8);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        update(ctx, &lenle, 8);
        if len != 0 {
            update(ctx, buf, len);
        }
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut tmp = [0u8; 64];
        blake512_final(&mut ctx.state, &mut tmp);
        out32.copy_from_slice(&tmp[..32]);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut tmp = [0u8; 32];
        blake256_final(&mut ctx.state, &mut tmp);
        out32.copy_from_slice(&tmp);
    }
}

#[cfg(feature = "sha2")]
mod kat_tr {
    use sphincs_plus_rs::sha2::{
        sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
        sha512_inc_finalize, sha512_inc_init,
    };

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_BLOCK_BYTES: usize = 128;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_STATE_LEN: usize = 72;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_OUTPUT_BYTES: usize = 64;

    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_BLOCK_BYTES: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_STATE_LEN: usize = 40;
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_OUTPUT_BYTES: usize = 32;

    pub struct KatTrCtx {
        pub state: Vec<u8>,
    }

    pub fn new() -> KatTrCtx {
        KatTrCtx {
            state: vec![0u8; SHAX_STATE_LEN],
        }
    }

    fn shax_inc_init(state: &mut [u8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_init(state);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_init(state);
    }
    fn shax_inc_blocks(state: &mut [u8], in_buf: &[u8], inblocks: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_blocks(state, in_buf, inblocks);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_blocks(state, in_buf, inblocks);
    }
    fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], in_buf: &[u8], inlen: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_finalize(out, state, in_buf, inlen);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_finalize(out, state, in_buf, inlen);
    }

    pub fn init(ctx: &mut KatTrCtx) {
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = vec![0u8; SHAX_BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        for i in tag.len()..SHAX_BLOCK_BYTES {
            block[i] = 0;
        }
        shax_inc_init(&mut ctx.state);
        shax_inc_blocks(&mut ctx.state, &block, 1);
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        for i in 0..block_count {
            let mut block = vec![0u8; SHAX_BLOCK_BYTES];
            let mut j = 0;
            while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
                block[j] = p[i * SHAX_BLOCK_BYTES + j];
                j += 1;
            }

            if i * SHAX_BLOCK_BYTES + j == n && j < SHAX_BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }

            for k in j..SHAX_BLOCK_BYTES {
                block[k] = 0;
            }

            shax_inc_blocks(&mut ctx.state, &block, 1);
        }
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = vec![0u8; SHAX_BLOCK_BYTES];
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = (x >> (8 * i)) as u8;
        }
        let mut lenle = [0u8; 8];
        let l = 8u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        for i in 0..8 {
            block[i] = lenle[i];
        }
        for i in 0..8 {
            block[8 + i] = le[i];
        }
        for i in 16..SHAX_BLOCK_BYTES {
            block[i] = 0;
        }
        shax_inc_blocks(&mut ctx.state, &block, 1);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = vec![0u8; SHAX_BLOCK_BYTES];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = (l >> (8 * i)) as u8;
        }
        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        shax_inc_blocks(&mut ctx.state, &lenle, 1);

        if len != 0 {
            for i in 0..block_count {
                let mut block = vec![0u8; SHAX_BLOCK_BYTES];
                let mut j = 0;
                while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                    block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                    j += 1;
                }
                for k in j..SHAX_BLOCK_BYTES {
                    block[k] = 0;
                }
                shax_inc_blocks(&mut ctx.state, &block, 1);
            }
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = vec![0u8; SHAX_OUTPUT_BYTES];
        let final_block = vec![0u8; SHAX_BLOCK_BYTES];
        shax_inc_finalize(&mut outbuf, &mut ctx.state, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

fn main() {
    let mut entropy_input = [0u8; 48];
    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    let mut tctx = kat_tr::new();
    kat_tr::init(&mut tctx);

    kat_tr::absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr::absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    kat_tr::absorb_label(&mut tctx, "SKBYTES");
    kat_tr::absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr::absorb_label(&mut tctx, "PKBYTES");
    kat_tr::absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr::absorb_label(&mut tctx, "SIGBYTES");
    kat_tr::absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    let mut m_buf = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        kat_tr::absorb_label(&mut tctx, "count");
        kat_tr::absorb_u64(&mut tctx, i as u64);
        kat_tr::absorb_label(&mut tctx, "seed");
        kat_tr::absorb_bytes(&mut tctx, &seed, 48);

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen as usize > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit((KAT_OVERFLOW as i32) as i32 & 0xFF);
        }

        kat_tr::absorb_label(&mut tctx, "mlen");
        kat_tr::absorb_u64(&mut tctx, mlen);

        randombytes(&mut msg, mlen);
        kat_tr::absorb_label(&mut tctx, "msg");
        kat_tr::absorb_bytes(&mut tctx, &msg, mlen as usize);

        for x in m_buf[..mlen as usize].iter_mut() { *x = 0; }
        for x in m1[..(mlen as usize) + CRYPTO_BYTES].iter_mut() { *x = 0; }
        for x in sm[..(mlen as usize) + CRYPTO_BYTES].iter_mut() { *x = 0; }
        m_buf[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit((KAT_CRYPTO_FAILURE as i32) as i32 & 0xFF);
        }
        kat_tr::absorb_label(&mut tctx, "pk");
        kat_tr::absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr::absorb_label(&mut tctx, "sk");
        kat_tr::absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m_buf[..mlen as usize], mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit((KAT_CRYPTO_FAILURE as i32) as i32 & 0xFF);
        }
        kat_tr::absorb_label(&mut tctx, "smlen");
        kat_tr::absorb_u64(&mut tctx, smlen);
        kat_tr::absorb_label(&mut tctx, "sm");
        kat_tr::absorb_bytes(&mut tctx, &sm[..smlen as usize], smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit((KAT_CRYPTO_FAILURE as i32) as i32 & 0xFF);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit((KAT_CRYPTO_FAILURE as i32) as i32 & 0xFF);
        }
        if m_buf[..mlen as usize] != m1[..mlen as usize] {
            eprintln!("m mismatch");
            std::process::exit((KAT_CRYPTO_FAILURE as i32) as i32 & 0xFF);
        }

    }

    let mut digest = [0u8; 32];
    kat_tr::finalize(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}
