use crate::error::{AcesError, Result};

fn err(msg: &str) -> AcesError {
    AcesError::GenericError(msg.to_string())
}

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
        if p == 0 {
            return Err(err("p must be non-zero"));
        }

        let mut channel = Self::new(p, q, w);
        if (p as u128) * (p as u128) >= q as u128 || !crate::common::are_coprime(p, q) {
            let next = p.saturating_add(1);
            channel.q = next.saturating_mul(next);
        }
        Ok(channel)
    }
}
