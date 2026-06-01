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
        // Mirrors init_channel logic: uses provided q if p^2 < q && coprime(p,q),
        // otherwise replaces q with (p+1)^2.
        let q_eff = if (p * p) < q && are_coprime(p, q) {
            q
        } else {
            (p + 1) * (p + 1)
        };
        Channel { p, q: q_eff, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        Ok(Channel::new(p, q, w))
    }
}
