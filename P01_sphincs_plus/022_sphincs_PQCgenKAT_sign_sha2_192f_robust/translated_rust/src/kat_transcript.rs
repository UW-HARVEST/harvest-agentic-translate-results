// KAT transcript using SHA2 (sha512 since SPX_N >= 24)
use crate::sha2::*;

const SHAX_STATE_LEN: usize = 72;   // sha512
const SHAX_BLOCK_BYTES: usize = 128; // sha512
const SHAX_OUTPUT_BYTES: usize = 64; // sha512

pub struct KatTrCtx {
    pub s: [u8; SHAX_STATE_LEN],
}

impl KatTrCtx {
    pub fn new() -> Self {
        KatTrCtx { s: [0u8; SHAX_STATE_LEN] }
    }

    pub fn init(&mut self) {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        // rest already zero

        sha512_inc_init(&mut self.s);
        sha512_inc_blocks(&mut self.s, &block, 1);
    }

    pub fn absorb_label(&mut self, label: &[u8]) {
        let n = label.len();
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        for i in 0..block_count {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut j = 0usize;
            while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
                block[j] = label[i * SHAX_BLOCK_BYTES + j];
                j += 1;
            }
            if i * SHAX_BLOCK_BYTES + j == n && j < SHAX_BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }
            while j < SHAX_BLOCK_BYTES {
                block[j] = 0;
                j += 1;
            }
            sha512_inc_blocks(&mut self.s, &block, 1);
        }
    }

    pub fn absorb_u64(&mut self, x: u64) {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        block[..8].copy_from_slice(&lenle);
        block[8..16].copy_from_slice(&le);
        // rest already zero

        sha512_inc_blocks(&mut self.s, &block, 1);
    }

    pub fn absorb_bytes(&mut self, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; SHAX_BLOCK_BYTES];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        sha512_inc_blocks(&mut self.s, &lenle, 1);

        if len != 0 {
            for i in 0..block_count {
                let mut block = [0u8; SHAX_BLOCK_BYTES];
                let mut j = 0usize;
                while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                    block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                    j += 1;
                }
                while j < SHAX_BLOCK_BYTES {
                    block[j] = 0;
                    j += 1;
                }
                sha512_inc_blocks(&mut self.s, &block, 1);
            }
        }
    }

    pub fn finalize(&mut self, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
        let final_block = [0u8; SHAX_BLOCK_BYTES];
        // C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1);
        // inlen=1: finalize with 1 byte of zero data
        sha512_inc_finalize(&mut outbuf, &mut self.s, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}
