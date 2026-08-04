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
        let mut q_eff = q;
        // Mirror C init_channel: if !(p^2 < q && coprime(p,q)), q = (p+1)^2
        if !(p.saturating_mul(p) < q && are_coprime(p, q)) {
            let p1 = p + 1;
            q_eff = p1 * p1;
        }
        Channel { p, q: q_eff, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        Ok(Self::new(p, q, w))
    }
}
