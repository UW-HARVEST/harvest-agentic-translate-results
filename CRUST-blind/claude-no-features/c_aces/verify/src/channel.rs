use crate::common::are_coprime;
use crate::error::{AcesError, Result};

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
        // Mirror C `init_channel`: assigns p,q,w and adjusts q if needed.
        let mut channel = Channel { p, q, w };
        let p_squared = (p as f64).powi(2);
        if !((p_squared < q as f64) && are_coprime(p, q)) {
            let v = (p + 1) as f64;
            channel.q = v.powi(2) as u64;
        }
        channel
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        if p >= q {
            return Err(AcesError::GenericError(
                "Channel parameter p must be less than q".to_string(),
            ));
        }
        Ok(Self::new(p, q, w))
    }
}
