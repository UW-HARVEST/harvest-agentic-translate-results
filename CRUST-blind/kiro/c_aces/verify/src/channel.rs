use crate::common::are_coprime;
use crate::error::Result;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Parameters {
    pub dim: u64,
    pub N: u64,
}
/// Represents an arithmetic channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub p: u64,
    pub q: u64,
    pub w: u64,
}
impl Channel {
    /// Creates a new channel.
    pub fn new(p: u64, q: u64, w: u64) -> Self {
        Channel { p, q, w }
    }
    /// Initializes a channel and checks that `p^2 < q` and coprime.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        let mut ch = Channel { w, p, q };
        if !(p * p < q && are_coprime(p, q)) {
            ch.q = (p + 1) * (p + 1);
        }
        Ok(ch)
    }
}
