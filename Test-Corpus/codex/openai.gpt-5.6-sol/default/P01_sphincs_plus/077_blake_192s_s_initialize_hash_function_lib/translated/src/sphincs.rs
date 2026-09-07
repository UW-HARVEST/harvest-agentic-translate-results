pub mod address;
mod aes;
pub mod api {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum SigError {
        Input,
        Verify,
    }
}
pub mod blake_impl;
pub mod context;
pub mod fors;
pub mod haraka;
pub mod hash;
pub mod merkle;
pub mod randombytes {
    pub fn randombytes(out: &mut [u8], len: usize) {
        super::rng::fill(&mut out[..len]);
    }
}
mod rng;
#[cfg(feature = "sha2")]
pub mod sha2_impl;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;
mod ffi;

pub use rng::*;

pub fn run_driver() -> i32 {
    use crate::params::*;
    use sign::{crypto_sign_keypair, crypto_sign_signature, crypto_sign_verify};

    let entropy = std::array::from_fn(|i| i as u8);
    rng::init(&entropy, None);
    let mut transcript = Transcript::new();
    transcript.label("CRYPTO_ALGNAME");
    transcript.bytes(b"SPHINCS+");
    transcript.label("SKBYTES");
    transcript.u64(SP_BYTES_SK as u64);
    transcript.label("PKBYTES");
    transcript.u64(SPX_PK_BYTES as u64);
    transcript.label("SIGBYTES");
    transcript.u64(SPX_BYTES as u64);

    for count in 0..7u64 {
        let mut seed = [0u8; 48];
        rng::fill(&mut seed);
        transcript.label("count");
        transcript.u64(count);
        transcript.label("seed");
        transcript.bytes(&seed);

        let mlen = 33 * (count as usize + 1);
        transcript.label("mlen");
        transcript.u64(mlen as u64);
        let mut msg = vec![0u8; mlen];
        rng::fill(&mut msg);
        transcript.label("msg");
        transcript.bytes(&msg);

        let mut pk = vec![0u8; SPX_PK_BYTES];
        let mut sk = vec![0u8; SP_BYTES_SK];
        crypto_sign_keypair(&mut pk, &mut sk, None);
        transcript.label("pk");
        transcript.bytes(&pk);
        transcript.label("sk");
        transcript.bytes(&sk);

        let mut sig = vec![0u8; SPX_BYTES];
        crypto_sign_signature(&mut sig, &msg, &sk, None);
        let smlen = SPX_BYTES + mlen;
        let mut sm = sig.clone();
        sm.extend_from_slice(&msg);
        transcript.label("smlen");
        transcript.u64(smlen as u64);
        transcript.label("sm");
        transcript.bytes(&sm);
        if crypto_sign_verify(&sig, &msg, &pk).is_err() {
            eprintln!("crypto_sign_open=-1");
            return -2;
        }
    }
    let digest = transcript.finish();
    print!("KAT transcript digest = ");
    for byte in digest {
        print!("{byte:02X}");
    }
    println!();
    0
}

const SP_BYTES_SK: usize = crate::params::SK_BYTES;

struct Transcript {
    data: Vec<u8>,
    blake_parts: Vec<(Vec<u8>, u64)>,
}

impl Transcript {
    fn new() -> Self {
        use crate::params::{Backend, BACKEND, N};
        let tag: &[u8] = match BACKEND {
            Backend::Blake => b"KAT-TRANSCRIPT-v1-BLAKE",
            Backend::Haraka => b"KAT-TRANSCRIPT-v1-HARAKA",
            Backend::Sha2 => b"KAT-TRANSCRIPT-v1-SHA2",
            Backend::Shake => b"KAT-TRANSCRIPT-v1-SHAKE",
        };
        let mut this = Self { data: Vec::new(), blake_parts: Vec::new() };
        if BACKEND == Backend::Blake {
            this.blake_parts.push((tag.to_vec(), tag.len() as u64));
            this.blake_parts.push((vec![0], 1));
        } else if BACKEND == Backend::Sha2 {
            let block = if N >= 24 { 128 } else { 64 };
            this.data.extend_from_slice(tag);
            this.data.resize(block, 0);
        } else {
            this.data.extend_from_slice(tag);
            this.data.push(0);
        }
        this
    }

    fn label(&mut self, label: &str) {
        use crate::params::{Backend, BACKEND, N};
        if BACKEND == Backend::Blake {
            self.blake_parts.push((label.as_bytes().to_vec(), label.len() as u64));
            self.blake_parts.push((vec![0], 1));
        } else if BACKEND == Backend::Sha2 {
            let block = if N >= 24 { 128 } else { 64 };
            let start = self.data.len();
            self.data.extend_from_slice(label.as_bytes());
            self.data.push(0);
            let blocks = (label.len() + 1).div_ceil(block);
            self.data.resize(start + blocks * block, 0);
        } else {
            self.data.extend_from_slice(label.as_bytes());
            self.data.push(0);
        }
    }

    fn u64(&mut self, value: u64) {
        use crate::params::{Backend, BACKEND, N};
        let mut field = Vec::from(8u64.to_le_bytes());
        field.extend_from_slice(&value.to_le_bytes());
        if BACKEND == Backend::Blake {
            self.blake_parts.push((8u64.to_le_bytes().to_vec(), 8));
            self.blake_parts.push((value.to_le_bytes().to_vec(), 8));
        } else if BACKEND == Backend::Sha2 {
            field.resize(if N >= 24 { 128 } else { 64 }, 0);
        }
        self.data.extend_from_slice(&field);
    }

    fn bytes(&mut self, value: &[u8]) {
        use crate::params::{Backend, BACKEND, N};
        if BACKEND == Backend::Blake {
            self.blake_parts.push(((value.len() as u64).to_le_bytes().to_vec(), 8));
            if !value.is_empty() {
                self.blake_parts.push((value.to_vec(), value.len() as u64));
            }
        } else if BACKEND == Backend::Sha2 {
            let block = if N >= 24 { 128 } else { 64 };
            let mut len = vec![0u8; block];
            len[..8].copy_from_slice(&(value.len() as u64).to_le_bytes());
            self.data.extend_from_slice(&len);
            if !value.is_empty() {
                let start = self.data.len();
                self.data.extend_from_slice(value);
                self.data.resize(start + value.len().div_ceil(block) * block, 0);
            }
        } else {
            self.data.extend_from_slice(&(value.len() as u64).to_le_bytes());
            self.data.extend_from_slice(value);
        }
    }

    fn finish(mut self) -> [u8; 32] {
        use crate::params::{Backend, BACKEND, N};
        match BACKEND {
            Backend::Blake => {
                let refs: Vec<(&[u8], u64)> = self.blake_parts.iter()
                    .map(|(bytes, bits)| (bytes.as_slice(), *bits)).collect();
                if N >= 24 {
                    blake_impl::blake512_updates(&refs)[..32].try_into().unwrap()
                } else {
                    blake_impl::blake256_updates(&refs)
                }
            }
            Backend::Shake => {
                use sha3::digest::{ExtendableOutput, Update, XofReader};
                let mut h = sha3::Shake256::default();
                h.update(&self.data);
                let mut out = [0u8; 32];
                h.finalize_xof().read(&mut out);
                out
            }
            Backend::Sha2 => {
                use sha256::Digest;
                self.data.push(0);
                if N >= 24 {
                    sha256::Sha512::digest(&self.data)[..32].try_into().unwrap()
                } else {
                    sha256::Sha256::digest(&self.data).into()
                }
            }
            Backend::Haraka => {
                let mut ctx = context::SpxCtx::default();
                haraka::tweak_constants(&mut ctx);
                let mut out = [0u8; 32];
                haraka::haraka_s(&mut out, 32, &self.data, self.data.len(), &ctx);
                out
            }
        }
    }
}
