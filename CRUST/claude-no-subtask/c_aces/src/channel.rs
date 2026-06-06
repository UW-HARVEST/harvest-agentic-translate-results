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
        // Mirrors `init_channel`: enforce p^2 < q and gcd(p, q) = 1, otherwise
        // adjust q to (p+1)^2.
        let mut q_eff = q;
        if !((p * p) < q && are_coprime(p, q)) {
            let pp1 = p + 1;
            q_eff = pp1 * pp1;
        }
        Channel { p, q: q_eff, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        if p >= q {
            return Err(AcesError::GenericError(
                "p must be strictly less than q".to_string(),
            ));
        }
        Ok(Self::new(p, q, w))
    }
}
