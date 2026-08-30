use crate::params::SPX_N;

pub struct KatTranscript {
    data: Vec<u8>,
    #[cfg(feature = "blake")]
    chunks: Vec<Vec<u8>>,
}

impl KatTranscript {
    pub fn new() -> Self {
        let tag: &[u8] = if cfg!(feature = "blake") {
            b"KAT-TRANSCRIPT-v1-BLAKE"
        } else if cfg!(feature = "haraka") {
            b"KAT-TRANSCRIPT-v1-HARAKA"
        } else if cfg!(feature = "sha2") {
            b"KAT-TRANSCRIPT-v1-SHA2"
        } else {
            b"KAT-TRANSCRIPT-v1-SHAKE"
        };

        #[cfg(feature = "sha2")]
        {
            let block_len = if SPX_N >= 24 { 128 } else { 64 };
            let mut data = vec![0u8; block_len];
            data[..tag.len()].copy_from_slice(tag);
            return Self { data };
        }

        #[cfg(all(not(feature = "sha2"), not(feature = "blake")))]
        {
            let mut data = tag.to_vec();
            data.push(0);
            Self { data }
        }
        #[cfg(feature = "blake")]
        {
            Self {
                data: Vec::new(),
                chunks: vec![tag.to_vec(), vec![0]],
            }
        }
    }

    pub fn absorb_label(&mut self, label: &str) {
        #[cfg(feature = "sha2")]
        {
            let block_len = if SPX_N >= 24 { 128 } else { 64 };
            let bytes = label.as_bytes();
            let block_count = (bytes.len() + 1).div_ceil(block_len);
            let mut blocks = vec![0u8; block_count * block_len];
            blocks[..bytes.len()].copy_from_slice(bytes);
            self.data.extend_from_slice(&blocks);
        }
        #[cfg(all(not(feature = "sha2"), not(feature = "blake")))]
        {
            self.data.extend_from_slice(label.as_bytes());
            self.data.push(0);
        }
        #[cfg(feature = "blake")]
        {
            self.chunks.push(label.as_bytes().to_vec());
            self.chunks.push(vec![0]);
        }
    }

    pub fn absorb_u64(&mut self, value: u64) {
        #[cfg(feature = "sha2")]
        {
            let block_len = if SPX_N >= 24 { 128 } else { 64 };
            let mut block = vec![0u8; block_len];
            block[..8].copy_from_slice(&8u64.to_le_bytes());
            block[8..16].copy_from_slice(&value.to_le_bytes());
            self.data.extend_from_slice(&block);
        }
        #[cfg(all(not(feature = "sha2"), not(feature = "blake")))]
        {
            self.absorb_bytes(&value.to_le_bytes());
        }
        #[cfg(feature = "blake")]
        {
            self.chunks.push(8u64.to_le_bytes().to_vec());
            self.chunks.push(value.to_le_bytes().to_vec());
        }
    }

    pub fn absorb_bytes(&mut self, bytes: &[u8]) {
        #[cfg(feature = "sha2")]
        {
            let block_len = if SPX_N >= 24 { 128 } else { 64 };
            let mut length_block = vec![0u8; block_len];
            length_block[..8].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
            self.data.extend_from_slice(&length_block);
            if !bytes.is_empty() {
                let block_count = bytes.len().div_ceil(block_len);
                let mut blocks = vec![0u8; block_count * block_len];
                blocks[..bytes.len()].copy_from_slice(bytes);
                self.data.extend_from_slice(&blocks);
            }
        }
        #[cfg(all(not(feature = "sha2"), not(feature = "blake")))]
        {
            self.data.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            self.data.extend_from_slice(bytes);
        }
        #[cfg(feature = "blake")]
        {
            self.chunks.push((bytes.len() as u64).to_le_bytes().to_vec());
            if !bytes.is_empty() {
                self.chunks.push(bytes.to_vec());
            }
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        #[cfg(feature = "haraka")]
        {
            use crate::context::SpxCtx;
            use crate::haraka::{haraka_s, tweak_constants};

            let mut ctx = SpxCtx::default();
            tweak_constants(&mut ctx);
            let mut out = [0u8; 32];
            haraka_s(&mut out, 32, &self.data, self.data.len(), &ctx);
            return out;
        }

        #[cfg(feature = "sha2")]
        {
            use sha2::{Digest, Sha256, Sha512};
            self.data.push(0);
            let digest = if SPX_N >= 24 {
                Sha512::digest(&self.data).to_vec()
            } else {
                Sha256::digest(&self.data).to_vec()
            };
            let mut out = [0u8; 32];
            out.copy_from_slice(&digest[..32]);
            return out;
        }

        #[cfg(feature = "shake")]
        {
            use sha3::{
                Shake256,
                digest::{ExtendableOutput, Update, XofReader},
            };
            let mut hasher = Shake256::default();
            hasher.update(&self.data);
            let mut reader = hasher.finalize_xof();
            let mut out = [0u8; 32];
            reader.read(&mut out);
            return out;
        }

        #[cfg(feature = "blake")]
        {
            use crate::blake::{Blake256State, Blake512State};
            let digest = if SPX_N >= 24 {
                let mut state = Blake512State::new();
                for chunk in &self.chunks {
                    state.update_bits(chunk, chunk.len() as u64);
                }
                state.finalize().to_vec()
            } else {
                let mut state = Blake256State::new();
                for chunk in &self.chunks {
                    state.update_bits(chunk, chunk.len() as u64);
                }
                state.finalize().to_vec()
            };
            let mut out = [0u8; 32];
            out.copy_from_slice(&digest[..32]);
            out
        }
    }
}
