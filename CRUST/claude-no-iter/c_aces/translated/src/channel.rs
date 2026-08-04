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
        // Mirror init_channel: if !(p^2 < q && coprime(p,q)) then q = (p+1)^2
        let mut q_final = q;
        let p2 = (p as u128).saturating_mul(p as u128);
        if !(p2 < q as u128 && are_coprime(p, q)) {
            let p1 = p.saturating_add(1);
            q_final = (p1 as u128).saturating_mul(p1 as u128) as u64;
        }
        Channel { p, q: q_final, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        Ok(Channel::new(p, q, w))
    }
}
