use crate::params::SPX_N;

/// Sphincs context
pub struct SpxCtx {
  pub pub_seed: [u8; SPX_N],
  pub sk_seed: [u8; SPX_N],
  
  pub state_seeded: [u8; 40],
  
  pub state_seeded_512: [u8; 72],
  
  pub tweaked512_rc64: [[u64; 8]; 10],

  pub tweaked256_rc32: [[u32; 8]; 10],
}

impl Default for SpxCtx {
  fn default() -> Self {
      Self { 
        pub_seed: [0u8; SPX_N], 
        sk_seed: [0u8; SPX_N],

        state_seeded: [0u8; 40],

        state_seeded_512: [0u8; 72],

        tweaked512_rc64: [[0u64; 8]; 10], 

        tweaked256_rc32: [[0u32; 8]; 10] 
    }
  }
}
