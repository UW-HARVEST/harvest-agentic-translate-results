// Translation of PQCgenKAT_sign.c. Reproduces stdout byte-for-byte.

use sphincs_plus::params::{
    SPX_BYTES, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_SK_BYTES,
};
use sphincs_plus::rng::{randombytes, randombytes_init_internal};
use sphincs_plus::sign::{
    crypto_sign_keypair_rs as crypto_sign_keypair_internal,
    crypto_sign_open_rs as crypto_sign_open_internal,
    crypto_sign_rs as crypto_sign_internal,
};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ---------------------------------------------------------------------
// CRYPTO_ALGNAME — must match the C #define for the chosen backend/secpar.
// ---------------------------------------------------------------------
const CRYPTO_ALGNAME: &str = "SPHINCS+";

// ---------------------------------------------------------------------
// Transcript helpers — picked at compile time based on hash backend.
// ---------------------------------------------------------------------

#[cfg(feature = "blake")]
mod transcript {
    use super::*;
    use sphincs_plus::blake::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub struct KatTrCtx {
        pub state: Blakestate512,
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub struct KatTrCtx {
        pub state: Blakestate256,
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const OUTPUT_BYTES: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const OUTPUT_BYTES: usize = 32;

    pub fn init() -> KatTrCtx {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        let mut ctx = KatTrCtx { state: Blakestate512::new() };
        #[cfg(any(feature = "128s", feature = "128f"))]
        let mut ctx = KatTrCtx { state: Blakestate256::new() };

        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_init(&mut ctx.state);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_init(&mut ctx.state);

        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        update(&mut ctx, tag);
        update(&mut ctx, &[0u8; 1]);
        ctx
    }

    fn update(ctx: &mut KatTrCtx, data: &[u8]) {
        // NOTE: The reference C transcript invokes blake*_update with byte
        // counts even though that function expects bit counts (a bug in C).
        // We mirror that here for byte-identical output.
        let datalen = data.len() as u64;
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_update(&mut ctx.state, data, datalen);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_update(&mut ctx.state, data, datalen);
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        update(ctx, label.as_bytes());
        update(ctx, &[0u8; 1]);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let l: u64 = 8;
        let lenle = l.to_le_bytes();
        update(ctx, &lenle);
        update(ctx, &le);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let l = buf.len() as u64;
        let lenle = l.to_le_bytes();
        update(ctx, &lenle);
        if !buf.is_empty() {
            update(ctx, buf);
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; OUTPUT_BYTES];
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_final(&mut ctx.state, &mut outbuf);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_final(&mut ctx.state, &mut outbuf);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

#[cfg(feature = "haraka")]
mod transcript {
    use super::*;
    use sphincs_plus::context::SpxCtx;
    use sphincs_plus::haraka::*;
    use sphincs_plus::params::SPX_N;

    pub struct KatTrCtx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    pub fn init() -> KatTrCtx {
        let mut ctx = KatTrCtx {
            inner: SpxCtx::zeroed(),
            s: [0u8; 65],
        };
        // pub_seed and sk_seed are already zero
        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }
        tweak_constants(&mut ctx.inner);
        haraka_s_inc_init(&mut ctx.s);

        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        let inner_copy_state = (ctx.inner.tweaked512_rc64, ctx.inner.tweaked256_rc32, ctx.inner.pub_seed, ctx.inner.sk_seed);
        haraka_s_inc_absorb(&mut ctx.s, tag, tag.len(), &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &[0u8; 1], 1, &ctx.inner);
        let _ = inner_copy_state;
        ctx
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let bytes = label.as_bytes();
        haraka_s_inc_absorb(&mut ctx.s, bytes, bytes.len(), &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &[0u8; 1], 1, &ctx.inner);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let l: u64 = 8;
        let lenle = l.to_le_bytes();
        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &le, 8, &ctx.inner);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let l = buf.len() as u64;
        let lenle = l.to_le_bytes();
        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        if !buf.is_empty() {
            haraka_s_inc_absorb(&mut ctx.s, buf, buf.len(), &ctx.inner);
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        haraka_s_inc_finalize(&mut ctx.s);
        haraka_s_inc_squeeze(out32, 32, &mut ctx.s, &ctx.inner);
    }
}

#[cfg(feature = "shake")]
mod transcript {
    use super::*;
    use sphincs_plus::fips202::*;

    pub struct KatTrCtx {
        pub s: [u64; 26],
    }

    pub fn init() -> KatTrCtx {
        let mut ctx = KatTrCtx { s: [0u64; 26] };
        shake256_inc_init(&mut ctx.s);

        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(&mut ctx.s, tag, tag.len());
        shake256_inc_absorb(&mut ctx.s, &[0u8; 1], 1);
        ctx
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let bytes = label.as_bytes();
        shake256_inc_absorb(&mut ctx.s, bytes, bytes.len());
        shake256_inc_absorb(&mut ctx.s, &[0u8; 1], 1);
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let l: u64 = 8;
        let lenle = l.to_le_bytes();
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        shake256_inc_absorb(&mut ctx.s, &le, 8);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let l = buf.len() as u64;
        let lenle = l.to_le_bytes();
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        if !buf.is_empty() {
            shake256_inc_absorb(&mut ctx.s, buf, buf.len());
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, 32, &mut ctx.s);
    }
}

#[cfg(feature = "sha2")]
mod transcript {
    use super::*;
    use sphincs_plus::sha2_impl::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_STATE_LEN: usize = 72;
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_STATE_LEN: usize = 40;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_BLOCK_BYTES: usize = 128;
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_BLOCK_BYTES: usize = 64;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub const SHAX_OUTPUT_BYTES: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub const SHAX_OUTPUT_BYTES: usize = 32;

    pub struct KatTrCtx {
        pub s: [u8; SHAX_STATE_LEN],
    }

    fn shax_inc_init(s: &mut [u8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_init(s);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_init(s);
    }
    fn shax_inc_blocks(s: &mut [u8], input: &[u8], n: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_blocks(s, input, n);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_blocks(s, input, n);
    }
    fn shax_inc_finalize(out: &mut [u8], s: &mut [u8], input: &[u8], inlen: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        sha512_inc_finalize(out, s, input, inlen);
        #[cfg(any(feature = "128s", feature = "128f"))]
        sha256_inc_finalize(out, s, input, inlen);
    }

    pub fn init() -> KatTrCtx {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        let mut ctx = KatTrCtx { s: [0u8; SHAX_STATE_LEN] };
        shax_inc_init(&mut ctx.s);
        shax_inc_blocks(&mut ctx.s, &block, 1);
        ctx
    }

    pub fn absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        for i in 0..block_count {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut j = 0usize;
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
            shax_inc_blocks(&mut ctx.s, &block, 1);
        }
    }

    pub fn absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let le = x.to_le_bytes();
        let l: u64 = 8;
        let lenle = l.to_le_bytes();
        for i in 0..8 {
            block[i] = lenle[i];
        }
        for i in 0..8 {
            block[8 + i] = le[i];
        }
        // remaining bytes already zero
        shax_inc_blocks(&mut ctx.s, &block, 1);
    }

    pub fn absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; SHAX_BLOCK_BYTES];
        let l = len as u64;
        let l_le = l.to_le_bytes();
        for i in 0..8 {
            lenle[i] = l_le[i];
        }
        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        shax_inc_blocks(&mut ctx.s, &lenle, 1);

        if len != 0 {
            for i in 0..block_count {
                let mut block = [0u8; SHAX_BLOCK_BYTES];
                let mut j = 0usize;
                while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                    block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                    j += 1;
                }
                for k in j..SHAX_BLOCK_BYTES {
                    block[k] = 0;
                }
                shax_inc_blocks(&mut ctx.s, &block, 1);
            }
        }
    }

    pub fn finalize(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
        let final_block = [0u8; SHAX_BLOCK_BYTES];
        shax_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

fn main() -> std::process::ExitCode {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init_internal(&entropy_input, None);

    let mut tctx = transcript::init();
    transcript::absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    transcript::absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    transcript::absorb_label(&mut tctx, "SKBYTES");
    transcript::absorb_u64(&mut tctx, SPX_SK_BYTES as u64);
    transcript::absorb_label(&mut tctx, "PKBYTES");
    transcript::absorb_u64(&mut tctx, SPX_PK_BYTES as u64);
    transcript::absorb_label(&mut tctx, "SIGBYTES");
    transcript::absorb_u64(&mut tctx, SPX_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);
        transcript::absorb_label(&mut tctx, "count");
        transcript::absorb_u64(&mut tctx, i as u64);
        transcript::absorb_label(&mut tctx, "seed");
        transcript::absorb_bytes(&mut tctx, &seed);

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            return std::process::ExitCode::from((KAT_OVERFLOW & 0xFF) as u8);
        }
        transcript::absorb_label(&mut tctx, "mlen");
        transcript::absorb_u64(&mut tctx, mlen);

        randombytes(&mut msg[..mlen as usize], mlen);
        transcript::absorb_label(&mut tctx, "msg");
        transcript::absorb_bytes(&mut tctx, &msg[..mlen as usize]);

        for b in &mut m[..mlen as usize] { *b = 0; }
        for b in &mut m1[..mlen as usize + SPX_BYTES] { *b = 0; }
        for b in &mut sm[..mlen as usize + SPX_BYTES] { *b = 0; }
        m[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        let ret = crypto_sign_keypair_internal(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return std::process::ExitCode::from((KAT_CRYPTO_FAILURE & 0xFF) as u8);
        }
        transcript::absorb_label(&mut tctx, "pk");
        transcript::absorb_bytes(&mut tctx, &pk);
        transcript::absorb_label(&mut tctx, "sk");
        transcript::absorb_bytes(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let m_copy = m[..mlen as usize].to_vec();
        let ret = crypto_sign_internal(&mut sm, &mut smlen, &m_copy, mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return std::process::ExitCode::from((KAT_CRYPTO_FAILURE & 0xFF) as u8);
        }
        transcript::absorb_label(&mut tctx, "smlen");
        transcript::absorb_u64(&mut tctx, smlen);
        transcript::absorb_label(&mut tctx, "sm");
        transcript::absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let sm_copy = sm[..smlen as usize].to_vec();
        let ret = crypto_sign_open_internal(&mut m1, &mut mlen1, &sm_copy, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            return std::process::ExitCode::from((KAT_CRYPTO_FAILURE & 0xFF) as u8);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            return std::process::ExitCode::from((KAT_CRYPTO_FAILURE & 0xFF) as u8);
        }
        if m[..mlen as usize] != m1[..mlen as usize] {
            eprintln!("m mismatch");
            return std::process::ExitCode::from((KAT_CRYPTO_FAILURE & 0xFF) as u8);
        }
    }

    let mut digest = [0u8; 32];
    transcript::finalize(&mut tctx, &mut digest);

    let mut s = String::from("KAT transcript digest = ");
    for b in &digest {
        s.push_str(&format!("{:02X}", b));
    }
    println!("{}", s);

    let _ = SPX_FORS_MSG_BYTES;
    let _ = SPX_N;
    std::process::ExitCode::from((KAT_SUCCESS & 0xFF) as u8)
}
