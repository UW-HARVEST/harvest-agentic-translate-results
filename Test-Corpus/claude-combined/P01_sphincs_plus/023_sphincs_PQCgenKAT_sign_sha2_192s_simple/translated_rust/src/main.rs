// Translation of c_src/app/src/PQCgenKAT_sign.c

use sphincs_plus::params::{CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, SPX_N};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;
const CRYPTO_ALGNAME: &str = "SPHINCS+";

unsafe extern "C" {
    fn randombytes_init(entropy_input: *mut u8, personalization_string: *mut u8);
    fn randombytes(x: *mut u8, xlen: u64) -> i32;
    fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32;
    fn crypto_sign(sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8) -> i32;
    fn crypto_sign_open(
        m: *mut u8,
        mlen: *mut u64,
        sm: *const u8,
        smlen: u64,
        pk: *const u8,
    ) -> i32;
}

// ----- KAT transcript -----
// The transcript hash mirrors the C code exactly:
//  - HARAKA: uses haraka_S_inc_* (rate 32)
//  - SHA2: uses sha256/sha512 inc_blocks helpers with each absorb writing one full block
//  - SHAKE: uses shake256_inc_*
//  - BLAKE: uses blakeX_init/update/final

#[cfg(feature = "haraka")]
mod tr {
    use sphincs_plus::context::SpxCtx;
    use sphincs_plus::params::SPX_N;

    pub struct Ctx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    pub fn init() -> Ctx {
        unsafe extern "C" {
            fn SPX_tweak_constants(ctx: *mut SpxCtx);
            fn SPX_haraka_S_inc_init(s_inc: *mut u8);
            fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const SpxCtx);
        }
        let mut inner = SpxCtx::new();
        // pub_seed and sk_seed already zero
        unsafe {
            SPX_tweak_constants(&mut inner);
        }
        let mut s = [0u8; 65];
        unsafe { SPX_haraka_S_inc_init(s.as_mut_ptr()); }
        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        unsafe {
            SPX_haraka_S_inc_absorb(s.as_mut_ptr(), tag.as_ptr(), tag.len(), &inner);
        }
        let sep = [0u8];
        unsafe {
            SPX_haraka_S_inc_absorb(s.as_mut_ptr(), sep.as_ptr(), 1, &inner);
        }
        Ctx { inner, s }
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &str) {
        unsafe extern "C" {
            fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const SpxCtx);
        }
        let bytes = label.as_bytes();
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), bytes.as_ptr(), bytes.len(), &ctx.inner);
            let sep = [0u8];
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1, &ctx.inner);
        }
    }

    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        unsafe extern "C" {
            fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const SpxCtx);
        }
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = (x >> (8 * i)) as u8; }
        let l: u64 = 8;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8, &ctx.inner);
        }
    }

    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        unsafe extern "C" {
            fn SPX_haraka_S_inc_absorb(s_inc: *mut u8, m: *const u8, mlen: usize, ctx: *const SpxCtx);
        }
        let l = buf.len() as u64;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        unsafe {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
            if !buf.is_empty() {
                SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), buf.len(), &ctx.inner);
            }
        }
    }

    pub fn finalize(ctx: &mut Ctx, out: &mut [u8; 32]) {
        unsafe extern "C" {
            fn SPX_haraka_S_inc_finalize(s_inc: *mut u8);
            fn SPX_haraka_S_inc_squeeze(out: *mut u8, outlen: usize, s_inc: *mut u8, ctx: *const SpxCtx);
        }
        unsafe {
            SPX_haraka_S_inc_finalize(ctx.s.as_mut_ptr());
            SPX_haraka_S_inc_squeeze(out.as_mut_ptr(), 32, ctx.s.as_mut_ptr(), &ctx.inner);
        }
        let _ = SPX_N;
    }
}

#[cfg(feature = "shake")]
mod tr {
    pub struct Ctx {
        pub s: [u64; 26],
    }
    pub fn init() -> Ctx {
        unsafe extern "C" {
            fn shake256_inc_init(s_inc: *mut u64);
            fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize);
        }
        let mut s = [0u64; 26];
        unsafe { shake256_inc_init(s.as_mut_ptr()); }
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        unsafe { shake256_inc_absorb(s.as_mut_ptr(), tag.as_ptr(), tag.len()); }
        let sep = [0u8];
        unsafe { shake256_inc_absorb(s.as_mut_ptr(), sep.as_ptr(), 1); }
        Ctx { s }
    }
    pub fn absorb_label(ctx: &mut Ctx, label: &str) {
        unsafe extern "C" {
            fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize);
        }
        let b = label.as_bytes();
        unsafe { shake256_inc_absorb(ctx.s.as_mut_ptr(), b.as_ptr(), b.len()); }
        let sep = [0u8];
        unsafe { shake256_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1); }
    }
    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        unsafe extern "C" {
            fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize);
        }
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = (x >> (8 * i)) as u8; }
        let l: u64 = 8;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            shake256_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8);
        }
    }
    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        unsafe extern "C" {
            fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize);
        }
        let l = buf.len() as u64;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            if !buf.is_empty() {
                shake256_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), buf.len());
            }
        }
    }
    pub fn finalize(ctx: &mut Ctx, out: &mut [u8; 32]) {
        unsafe extern "C" {
            fn shake256_inc_finalize(s_inc: *mut u64);
            fn shake256_inc_squeeze(out: *mut u8, outlen: usize, s_inc: *mut u64);
        }
        unsafe {
            shake256_inc_finalize(ctx.s.as_mut_ptr());
            shake256_inc_squeeze(out.as_mut_ptr(), 32, ctx.s.as_mut_ptr());
        }
    }
}

#[cfg(feature = "sha2")]
mod tr {
    use sphincs_plus::params::SPX_N;
    const BLOCK_BYTES: usize = if SPX_N >= 24 { 128 } else { 64 };
    const STATE_LEN: usize = if SPX_N >= 24 { 72 } else { 40 };
    const OUTPUT_BYTES: usize = if SPX_N >= 24 { 64 } else { 32 };

    pub struct Ctx {
        pub s: [u8; STATE_LEN],
    }
    fn inc_init(s: &mut [u8]) {
        unsafe extern "C" {
            fn sha256_inc_init(state: *mut u8);
            fn sha512_inc_init(state: *mut u8);
        }
        unsafe {
            if SPX_N >= 24 { sha512_inc_init(s.as_mut_ptr()); } else { sha256_inc_init(s.as_mut_ptr()); }
        }
    }
    fn inc_blocks(s: &mut [u8], block: &[u8], nblocks: usize) {
        unsafe extern "C" {
            fn sha256_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize);
            fn sha512_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize);
        }
        unsafe {
            if SPX_N >= 24 { sha512_inc_blocks(s.as_mut_ptr(), block.as_ptr(), nblocks); }
            else { sha256_inc_blocks(s.as_mut_ptr(), block.as_ptr(), nblocks); }
        }
    }
    fn inc_finalize(s: &mut [u8], out: &mut [u8], block: &[u8], nblocks: usize) {
        // C uses shaX_inc_finalize(outbuf, state, final_block, 1) which is
        // sha256_inc_finalize(out, state, in, inlen) — inlen is in bytes.
        // Looking at the C code:
        //   shaX_inc_finalize(outbuf, ctx->s, final_block, 1);
        // But sha256_inc_finalize signature is (out, state, in, inlen).
        // So they pass inlen=1 — meaning a one-byte final input. Wait, checking again...
        // Actually in C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1);
        // translation: sha256_inc_finalize takes (out, state, input, inlen)
        // so this call is finalize with inlen=1. The block buffer is shaX_block_bytes, all zeros.
        unsafe extern "C" {
            fn sha256_inc_finalize(out: *mut u8, state: *mut u8, input: *const u8, inlen: usize);
            fn sha512_inc_finalize(out: *mut u8, state: *mut u8, input: *const u8, inlen: usize);
        }
        unsafe {
            if SPX_N >= 24 {
                sha512_inc_finalize(out.as_mut_ptr(), s.as_mut_ptr(), block.as_ptr(), nblocks);
            } else {
                sha256_inc_finalize(out.as_mut_ptr(), s.as_mut_ptr(), block.as_ptr(), nblocks);
            }
        }
    }
    pub fn init() -> Ctx {
        let mut s = [0u8; STATE_LEN];
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; BLOCK_BYTES];
        for i in 0..tag.len() { block[i] = tag[i]; }
        for i in tag.len()..BLOCK_BYTES { block[i] = 0; }
        inc_init(&mut s);
        inc_blocks(&mut s, &block, 1);
        Ctx { s }
    }
    pub fn absorb_label(ctx: &mut Ctx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (BLOCK_BYTES - 1)) / BLOCK_BYTES;
        for i in 0..block_count {
            let mut block = [0u8; BLOCK_BYTES];
            let mut j = 0usize;
            while i * BLOCK_BYTES + j < n && j < BLOCK_BYTES {
                block[j] = p[i * BLOCK_BYTES + j];
                j += 1;
            }
            if i * BLOCK_BYTES + j == n && j < BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }
            while j < BLOCK_BYTES { block[j] = 0; j += 1; }
            inc_blocks(&mut ctx.s, &block, 1);
        }
    }
    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let mut block = [0u8; BLOCK_BYTES];
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = (x >> (8 * i)) as u8; }
        let l: u64 = 8;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        for i in 0..8 { block[i] = lenle[i]; }
        for i in 0..8 { block[8 + i] = le[i]; }
        for i in 16..BLOCK_BYTES { block[i] = 0; }
        inc_blocks(&mut ctx.s, &block, 1);
    }
    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; BLOCK_BYTES];
        let l = len as u64;
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        let block_count = (len + (BLOCK_BYTES - 1)) / BLOCK_BYTES;
        inc_blocks(&mut ctx.s, &lenle, 1);
        if len != 0 {
            for i in 0..block_count {
                let mut block = [0u8; BLOCK_BYTES];
                let mut j = 0usize;
                while i * BLOCK_BYTES + j < len && j < BLOCK_BYTES {
                    block[j] = buf[i * BLOCK_BYTES + j];
                    j += 1;
                }
                while j < BLOCK_BYTES { block[j] = 0; j += 1; }
                inc_blocks(&mut ctx.s, &block, 1);
            }
        }
    }
    pub fn finalize(ctx: &mut Ctx, out: &mut [u8; 32]) {
        let mut outbuf = [0u8; OUTPUT_BYTES];
        let final_block = [0u8; BLOCK_BYTES];
        inc_finalize(&mut ctx.s, &mut outbuf, &final_block, 1);
        out.copy_from_slice(&outbuf[..32]);
    }
}

#[cfg(feature = "blake")]
mod tr {
    use sphincs_plus::params::SPX_N;
    use sphincs_plus::hash::blake::blake256::BlakeState256;
    use sphincs_plus::hash::blake::blake512::BlakeState512;

    pub enum Ctx {
        Small(BlakeState256),
        Big(BlakeState512),
    }

    pub fn init() -> Ctx {
        let mut c = if SPX_N >= 24 {
            let mut s: BlakeState512 = unsafe { core::mem::zeroed() };
            unsafe extern "C" {
                fn blake512_init(s: *mut BlakeState512);
            }
            unsafe { blake512_init(&mut s); }
            Ctx::Big(s)
        } else {
            let mut s: BlakeState256 = unsafe { core::mem::zeroed() };
            unsafe extern "C" {
                fn blake256_init(s: *mut BlakeState256);
            }
            unsafe { blake256_init(&mut s); }
            Ctx::Small(s)
        };
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        feed(&mut c, tag);
        let sep = [0u8];
        feed(&mut c, &sep);
        c
    }

    fn feed(c: &mut Ctx, b: &[u8]) {
        // NOTE: matches the C code's KAT transcript which passes byte count
        // (not bit count) as the datalen argument to blakeX_update — i.e. the
        // same "C bug" that's in c_src/lib/blake/src/hash_blake.c.
        unsafe extern "C" {
            fn blake256_update(s: *mut BlakeState256, data: *const u8, datalen: u64);
            fn blake512_update(s: *mut BlakeState512, data: *const u8, datalen: u64);
        }
        match c {
            Ctx::Small(s) => unsafe { blake256_update(s, b.as_ptr(), b.len() as u64) },
            Ctx::Big(s) => unsafe { blake512_update(s, b.as_ptr(), b.len() as u64) },
        }
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &str) {
        feed(ctx, label.as_bytes());
        feed(ctx, &[0u8]);
    }
    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = (x >> (8 * i)) as u8; }
        let l: u64 = 8;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        feed(ctx, &lenle);
        feed(ctx, &le);
    }
    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let l = buf.len() as u64;
        let mut lenle = [0u8; 8];
        for i in 0..8 { lenle[i] = (l >> (8 * i)) as u8; }
        feed(ctx, &lenle);
        if !buf.is_empty() {
            feed(ctx, buf);
        }
    }
    pub fn finalize(ctx: &mut Ctx, out: &mut [u8; 32]) {
        unsafe extern "C" {
            fn blake256_final(s: *mut BlakeState256, digest: *mut u8);
            fn blake512_final(s: *mut BlakeState512, digest: *mut u8);
        }
        match ctx {
            Ctx::Small(s) => {
                let mut buf = [0u8; 32];
                unsafe { blake256_final(s, buf.as_mut_ptr()); }
                out.copy_from_slice(&buf);
            }
            Ctx::Big(s) => {
                let mut buf = [0u8; 64];
                unsafe { blake512_final(s, buf.as_mut_ptr()); }
                out.copy_from_slice(&buf[..32]);
            }
        }
    }
}

fn main() -> std::process::ExitCode {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    unsafe { randombytes_init(entropy_input.as_mut_ptr(), core::ptr::null_mut()); }

    let mut tctx = tr::init();
    tr::absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    tr::absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    tr::absorb_label(&mut tctx, "SKBYTES"); tr::absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    tr::absorb_label(&mut tctx, "PKBYTES"); tr::absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    tr::absorb_label(&mut tctx, "SIGBYTES"); tr::absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    let max_mlen = BASE_MLEN * LOOP_COUNT;
    for i in 0..LOOP_COUNT {
        unsafe { randombytes(seed.as_mut_ptr(), 48); }
        tr::absorb_label(&mut tctx, "count"); tr::absorb_u64(&mut tctx, i as u64);
        tr::absorb_label(&mut tctx, "seed"); tr::absorb_bytes(&mut tctx, &seed);

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen > max_mlen as u64 {
            eprintln!("mlen overflow");
            return std::process::ExitCode::from(255); // -1
        }
        tr::absorb_label(&mut tctx, "mlen"); tr::absorb_u64(&mut tctx, mlen);

        unsafe { randombytes(msg.as_mut_ptr(), mlen); }
        tr::absorb_label(&mut tctx, "msg"); tr::absorb_bytes(&mut tctx, &msg[..mlen as usize]);

        let mlen_us = mlen as usize;
        for v in m[..mlen_us].iter_mut() { *v = 0; }
        for v in m1[..mlen_us + CRYPTO_BYTES].iter_mut() { *v = 0; }
        for v in sm[..mlen_us + CRYPTO_BYTES].iter_mut() { *v = 0; }
        m[..mlen_us].copy_from_slice(&msg[..mlen_us]);

        let ret = unsafe { crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return std::process::ExitCode::from(254);
        }
        tr::absorb_label(&mut tctx, "pk"); tr::absorb_bytes(&mut tctx, &pk);
        tr::absorb_label(&mut tctx, "sk"); tr::absorb_bytes(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let ret = unsafe {
            crypto_sign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen, sk.as_ptr())
        };
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return std::process::ExitCode::from(254);
        }
        tr::absorb_label(&mut tctx, "smlen"); tr::absorb_u64(&mut tctx, smlen);
        tr::absorb_label(&mut tctx, "sm"); tr::absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = unsafe {
            crypto_sign_open(m1.as_mut_ptr(), &mut mlen1, sm.as_ptr(), smlen, pk.as_ptr())
        };
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            return std::process::ExitCode::from(254);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            return std::process::ExitCode::from(254);
        }
        if m[..mlen_us] != m1[..mlen_us] {
            eprintln!("m mismatch");
            return std::process::ExitCode::from(254);
        }
    }

    let mut digest = [0u8; 32];
    tr::finalize(&mut tctx, &mut digest);
    print!("KAT transcript digest = ");
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();

    let _ = SPX_N;
    std::process::ExitCode::SUCCESS
}
