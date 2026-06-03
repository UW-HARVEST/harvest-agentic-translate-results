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
        let mut q_final = q;
        if !((p * p) < q && are_coprime(p, q)) {
            q_final = (p + 1) * (p + 1);
        }
        Channel { p, q: q_final, w }
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
