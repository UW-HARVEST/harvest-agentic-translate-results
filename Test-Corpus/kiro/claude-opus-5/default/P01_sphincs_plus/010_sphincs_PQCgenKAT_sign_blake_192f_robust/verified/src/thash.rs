//! `app/include/thash.h`: forwards to the selected backend's
//! `thash_<backend>_<THASH>.c`.

pub use crate::backend::active::thash;
