use crate::common::are_coprime;
use crate::error::{AcesError, Result};
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
        let mut channel = Channel { p, q, w };
        // Match C's init_channel behavior: if p^2 < q && coprime(p, q) is false, use (p+1)^2.
        let p_sq = (p as u128).saturating_mul(p as u128);
        if !(p_sq < q as u128 && are_coprime(p, q)) {
            let pp = (p as u128).saturating_add(1);
            let pp_sq = pp.saturating_mul(pp);
            channel.q = pp_sq as u64;
        }
        channel
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        if p >= q {
            return Err(AcesError::GenericError(format!(
                "p ({}) must be less than q ({})",
                p, q
            )));
        }
        Ok(Channel::new(p, q, w))
    }
}
