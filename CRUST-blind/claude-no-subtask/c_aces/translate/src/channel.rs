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
        // Mirror the C `init_channel`: ensure p^2 < q and gcd(p, q) == 1; otherwise
        // override q with (p + 1)^2.
        let (p_sq, ovf1) = p.overflowing_mul(p);
        let condition_ok = !ovf1 && p_sq < q && are_coprime(p, q);
        let q = if condition_ok {
            q
        } else {
            let plus = p.saturating_add(1);
            plus.saturating_mul(plus)
        };
        Channel { p, q, w }
    }
    /// Initializes a channel and checks that `p < q`.
    pub fn init(p: u64, q: u64, w: u64) -> Result<Self> {
        if p >= q {
            return Err(AcesError::GenericError(
                "Channel requires p < q".to_string(),
            ));
        }
        Ok(Channel::new(p, q, w))
    }
}
