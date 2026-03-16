pub mod blake256;
pub mod blake512;

pub use blake256::*;
pub use blake512::*;

use crate::params::*;
use crate::utils;

// For N >= 24, use blake512 variants
pub fn blake_x(out: &mut [u8], inp: &[u8], inlen: u64) {
    blake512::blake512(out, inp, inlen);
}

pub fn blake_x_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    blake512::blake512_mgf1(out, outlen, inp, inlen);
}

pub struct BlakeStateX {
    pub inner: BlakeState512,
}

impl BlakeStateX {
    pub fn new() -> Self {
        let mut s = BlakeState512::new();
        blake512::blake512_init(&mut s);
        BlakeStateX { inner: s }
    }
    pub fn init(&mut self) {
        blake512::blake512_init(&mut self.inner);
    }
    pub fn update(&mut self, data: &[u8], datalen: u64) {
        blake512::blake512_update(&mut self.inner, data, datalen);
    }
    pub fn finalize(&mut self, out: &mut [u8]) {
        blake512::blake512_final(&mut self.inner, out);
    }
}
