use sphincs_plus::context::SpxCtx;
use sphincs_plus::haraka::{
    haraka_s_inc_absorb_safe, haraka_s_inc_finalize_safe, haraka_s_inc_init_safe,
    haraka_s_inc_squeeze_safe, tweak_constants_safe,
};
use sphincs_plus::params::*;
use sphincs_plus::rng::{randombytes_init, SPX_randombytes};
use sphincs_plus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

struct KatTrCtx {
    inner: SpxCtx,
    s: [u8; 65],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    for i in 0..SPX_N {
        ctx.inner.pub_seed[i] = 0;
        ctx.inner.sk_seed[i] = 0;
    }
    tweak_constants_safe(&mut ctx.inner);
    haraka_s_inc_init_safe(&mut ctx.s);

    let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
    haraka_s_inc_absorb_safe(&mut ctx.s, tag, &ctx.inner);

    let sep = [0u8; 1];
    haraka_s_inc_absorb_safe(&mut ctx.s, &sep, &ctx.inner);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
    haraka_s_inc_absorb_safe(&mut ctx.s, label.as_bytes(), &ctx.inner);
    let sep = [0u8; 1];
    haraka_s_inc_absorb_safe(&mut ctx.s, &sep, &ctx.inner);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 {
        le[i] = ((x >> (8 * i)) & 0xFF) as u8;
    }
    let l: u64 = 8;
    let mut lenle = [0u8; 8];
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }

    haraka_s_inc_absorb_safe(&mut ctx.s, &lenle, &ctx.inner);
    haraka_s_inc_absorb_safe(&mut ctx.s, &le, &ctx.inner);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
    let l = buf.len() as u64;
    let mut lenle = [0u8; 8];
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    haraka_s_inc_absorb_safe(&mut ctx.s, &lenle, &ctx.inner);
    if !buf.is_empty() {
        haraka_s_inc_absorb_safe(&mut ctx.s, buf, &ctx.inner);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out: &mut [u8; 32]) {
    haraka_s_inc_finalize_safe(&mut ctx.s);
    haraka_s_inc_squeeze_safe(out, &mut ctx.s, &ctx.inner);
}

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
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
    randombytes_init(entropy_input.as_mut_ptr(), std::ptr::null_mut());

    let mut tctx = KatTrCtx {
        inner: SpxCtx::new(),
        s: [0u8; 65],
    };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    kat_tr_absorb_label(&mut tctx, "SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        SPX_randombytes(seed.as_mut_ptr(), seed.len() as u64);

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            return KAT_OVERFLOW;
        }

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        SPX_randombytes(msg.as_mut_ptr(), mlen as u64);
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        for b in &mut m[..mlen] { *b = 0; }
        for b in &mut m1[..mlen + CRYPTO_BYTES] { *b = 0; }
        for b in &mut sm[..mlen + CRYPTO_BYTES] { *b = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr());
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(
            sm.as_mut_ptr(),
            &mut smlen,
            m.as_ptr(),
            mlen as u64,
            sk.as_ptr(),
        );
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(
            m1.as_mut_ptr(),
            &mut mlen1,
            sm.as_ptr(),
            smlen,
            pk.as_ptr(),
        );
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        if mlen1 != mlen as u64 {
            eprintln!("mlen mismatch");
            return KAT_CRYPTO_FAILURE;
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            return KAT_CRYPTO_FAILURE;
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();

    KAT_SUCCESS
}
