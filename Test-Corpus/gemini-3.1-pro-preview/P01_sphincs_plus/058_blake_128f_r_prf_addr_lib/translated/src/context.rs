use crate::params::*;

#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    #[cfg(feature = "hash-sha2")]
    pub state_seeded: sha2::Sha256,
    #[cfg(all(feature = "hash-sha2", any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s")))]
    pub state_seeded_512: sha2::Sha512,
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self {
            pub_seed: [0; SPX_N],
            sk_seed: [0; SPX_N],
            #[cfg(feature = "hash-sha2")]
            state_seeded: sha2::Digest::new(),
            #[cfg(all(feature = "hash-sha2", any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s")))]
            state_seeded_512: sha2::Digest::new(),
        }
    }
}
