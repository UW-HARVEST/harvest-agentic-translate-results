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
        // Mirror init_channel: if p^2 < q is not satisfied or p,q are not coprime,
        // then q = (p+1)^2.
        let p_sq = (p as f64).powi(2);
        if !(p_sq < q as f64 && are_coprime(p, q)) {
            channel.q = ((p + 1) as f64).powi(2) as u64;
        }
        channel
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        if p >= q {
            return Err(AcesError::GenericError(
                "p must be less than q".to_string(),
            ));
        }
        Ok(Channel::new(p, q, w))
    }
}
