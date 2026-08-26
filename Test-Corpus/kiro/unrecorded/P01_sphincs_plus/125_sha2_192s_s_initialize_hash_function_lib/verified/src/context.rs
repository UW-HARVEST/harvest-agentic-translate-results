use crate::params::SPX_N;

#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    #[cfg(feature = "sha2")]
    pub state_seeded: [u8; 40],
    #[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    pub state_seeded_512: [u8; 72],
    #[cfg(feature = "haraka")]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(feature = "haraka")]
    pub tweaked256_rc32: [[u32; 8]; 10],
}
