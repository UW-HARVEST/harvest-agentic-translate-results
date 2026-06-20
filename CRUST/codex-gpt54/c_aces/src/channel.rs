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
        Self { p, q, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        let mut channel = Self::new(p, q, w);
        if !(p.saturating_mul(p) < q && are_coprime(p, q)) {
            channel.q = p.saturating_add(1).saturating_mul(p.saturating_add(1));
        }
        Ok(channel)
    }
}
