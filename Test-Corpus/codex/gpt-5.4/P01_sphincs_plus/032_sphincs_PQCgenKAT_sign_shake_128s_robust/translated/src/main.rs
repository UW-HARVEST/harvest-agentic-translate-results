use translated_rust::params::*;
use translated_rust::rng::randombytes_init;
use translated_rust::sha2_backend::{sha256_inc_blocks_rs, sha256_inc_finalize_rs, sha256_inc_init_rs, sha512_inc_blocks_rs, sha512_inc_finalize_rs, sha512_inc_init_rs};
use translated_rust::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;
const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

struct Sha2Transcript {
    sha256: [u8; 40],
    sha512: [u8; 72],
}

impl Sha2Transcript {
    fn new() -> Self {
        if SPX_N >= 24 {
            let mut s = [0u8; 72];
            let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
            let tag = b"KAT-TRANSCRIPT-v1-SHA2";
            block[..tag.len()].copy_from_slice(tag);
            sha512_inc_init_rs(&mut s);
            sha512_inc_blocks_rs(&mut s, &block, 1);
            Self { sha256: [0; 40], sha512: s }
        } else {
            let mut s = [0u8; 40];
            let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
            let tag = b"KAT-TRANSCRIPT-v1-SHA2";
            block[..tag.len()].copy_from_slice(tag);
            sha256_inc_init_rs(&mut s);
            sha256_inc_blocks_rs(&mut s, &block, 1);
            Self { sha256: s, sha512: [0; 72] }
        }
    }

    fn absorb_label(&mut self, label: &str) {
        if SPX_N >= 24 {
            let block_bytes = SPX_SHA512_BLOCK_BYTES;
            let bytes = label.as_bytes();
            let block_count = (bytes.len() + 1).div_ceil(block_bytes);
            for i in 0..block_count {
                let mut block = vec![0u8; block_bytes];
                let start = i * block_bytes;
                let end = (start + block_bytes).min(bytes.len());
                let len = end.saturating_sub(start);
                if len > 0 {
                    block[..len].copy_from_slice(&bytes[start..end]);
                }
                if start + len == bytes.len() && len < block_bytes {
                    block[len] = 0;
                }
                sha512_inc_blocks_rs(&mut self.sha512, &block, 1);
            }
        } else {
            let block_bytes = SPX_SHA256_BLOCK_BYTES;
            let bytes = label.as_bytes();
            let block_count = (bytes.len() + 1).div_ceil(block_bytes);
            for i in 0..block_count {
                let mut block = vec![0u8; block_bytes];
                let start = i * block_bytes;
                let end = (start + block_bytes).min(bytes.len());
                let len = end.saturating_sub(start);
                if len > 0 {
                    block[..len].copy_from_slice(&bytes[start..end]);
                }
                if start + len == bytes.len() && len < block_bytes {
                    block[len] = 0;
                }
                sha256_inc_blocks_rs(&mut self.sha256, &block, 1);
            }
        }
    }

    fn absorb_u64(&mut self, x: u64) {
        let mut le = [0u8; 8];
        for (i, b) in le.iter_mut().enumerate() {
            *b = ((x >> (8 * i)) & 0xff) as u8;
        }
        let lenle = [8u8, 0, 0, 0, 0, 0, 0, 0];
        if SPX_N >= 24 {
            let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
            block[..8].copy_from_slice(&lenle);
            block[8..16].copy_from_slice(&le);
            sha512_inc_blocks_rs(&mut self.sha512, &block, 1);
        } else {
            let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
            block[..8].copy_from_slice(&lenle);
            block[8..16].copy_from_slice(&le);
            sha256_inc_blocks_rs(&mut self.sha256, &block, 1);
        }
    }

    fn absorb_bytes(&mut self, bytes: &[u8]) {
        let len = bytes.len() as u64;
        if SPX_N >= 24 {
            let mut len_block = [0u8; SPX_SHA512_BLOCK_BYTES];
            for i in 0..8 {
                len_block[i] = ((len >> (8 * i)) & 0xff) as u8;
            }
            sha512_inc_blocks_rs(&mut self.sha512, &len_block, 1);
            if !bytes.is_empty() {
                let block_count = bytes.len().div_ceil(SPX_SHA512_BLOCK_BYTES);
                for i in 0..block_count {
                    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
                    let start = i * SPX_SHA512_BLOCK_BYTES;
                    let end = (start + SPX_SHA512_BLOCK_BYTES).min(bytes.len());
                    block[..end - start].copy_from_slice(&bytes[start..end]);
                    sha512_inc_blocks_rs(&mut self.sha512, &block, 1);
                }
            }
        } else {
            let mut len_block = [0u8; SPX_SHA256_BLOCK_BYTES];
            for i in 0..8 {
                len_block[i] = ((len >> (8 * i)) & 0xff) as u8;
            }
            sha256_inc_blocks_rs(&mut self.sha256, &len_block, 1);
            if !bytes.is_empty() {
                let block_count = bytes.len().div_ceil(SPX_SHA256_BLOCK_BYTES);
                for i in 0..block_count {
                    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
                    let start = i * SPX_SHA256_BLOCK_BYTES;
                    let end = (start + SPX_SHA256_BLOCK_BYTES).min(bytes.len());
                    block[..end - start].copy_from_slice(&bytes[start..end]);
                    sha256_inc_blocks_rs(&mut self.sha256, &block, 1);
                }
            }
        }
    }

    fn final_digest(mut self) -> [u8; 32] {
        if SPX_N >= 24 {
            let mut out = [0u8; 64];
            sha512_inc_finalize_rs(&mut out, &mut self.sha512, &[0u8]);
            let mut digest = [0u8; 32];
            digest.copy_from_slice(&out[..32]);
            digest
        } else {
            let mut out = [0u8; 32];
            sha256_inc_finalize_rs(&mut out, &mut self.sha256, &[0u8]);
            out
        }
    }
}

fn main() {
    std::process::exit(run());
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
    for (i, b) in entropy_input.iter_mut().enumerate() {
        *b = i as u8;
    }
    randombytes_init(entropy_input.as_mut_ptr(), std::ptr::null_mut());
    let mut tctx = Sha2Transcript::new();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME.as_bytes());
    tctx.absorb_label("SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);
    for i in 0..LOOP_COUNT {
        translated_rust::rng::randombytes(seed.as_mut_ptr(), seed.len() as u64);
        tctx.absorb_label("count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed");
        tctx.absorb_bytes(&seed);
        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            return KAT_OVERFLOW;
        }
        tctx.absorb_label("mlen");
        tctx.absorb_u64(mlen as u64);
        translated_rust::rng::randombytes(msg.as_mut_ptr(), mlen as u64);
        tctx.absorb_label("msg");
        tctx.absorb_bytes(&msg[..mlen]);
        m[..mlen].copy_from_slice(&msg[..mlen]);
        m1[..mlen + CRYPTO_BYTES].fill(0);
        sm[..mlen + CRYPTO_BYTES].fill(0);
        let ret = crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr());
        if ret != 0 {
            eprintln!("crypto_sign_keypair={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label("pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label("sk");
        tctx.absorb_bytes(&sk);
        let mut smlen = 0u64;
        let ret = crypto_sign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen as u64, sk.as_ptr());
        if ret != 0 {
            eprintln!("crypto_sign={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label("smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label("sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);
        let mut mlen1 = 0u64;
        let ret = crypto_sign_open(m1.as_mut_ptr(), &mut mlen1, sm.as_ptr(), smlen, pk.as_ptr());
        if ret != 0 {
            eprintln!("crypto_sign_open={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        if mlen1 as usize != mlen {
            eprintln!("mlen mismatch");
            return KAT_CRYPTO_FAILURE;
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            return KAT_CRYPTO_FAILURE;
        }
    }
    let digest = tctx.final_digest();
    print!("KAT transcript digest = ");
    for b in digest {
        print!("{b:02X}");
    }
    println!();
    KAT_SUCCESS
}
