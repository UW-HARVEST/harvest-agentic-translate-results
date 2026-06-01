use crate::common::are_coprime;
use crate::error::Result;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let mut q_final = q;
        let p_sq = (p as f64).powi(2);
        if !(p_sq < q as f64 && are_coprime(p, q)) {
            q_final = ((p + 1) as f64).powi(2) as u64;
        }
        Channel { p, q: q_final, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        Ok(Self::new(p, q, w))
    }
}
