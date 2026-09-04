//! Stand-in for the C `SPX_VLA` macro of `app/include/utils.h`
//! (`#define SPX_VLA(__t,__x,__s) __t __x[__s]`, a C99 variable length array).
//!
//! Several `thash()` scratch buffers are sized from the caller supplied
//! `inblocks`, which the C never range checks, so there is no compile time
//! upper bound on them.  Every call the library itself makes stays at or below
//! [`crate::params::SPX_THASH_MAX_INBLOCKS`], so that case is served from a
//! fixed size stack array; anything larger — which an external caller of the
//! exported `SPX_thash` may legitimately ask for — spills to the heap instead
//! of panicking, so the behaviour matches the C for every `inblocks`.

/// A byte buffer of run-time length `len`, stack allocated while
/// `len <= N` and heap allocated beyond that.
pub struct Vla<const N: usize> {
    inline: [u8; N],
    heap: Vec<u8>,
    len: usize,
}

impl<const N: usize> Vla<N> {
    #[inline]
    pub fn new(len: usize) -> Self {
        Vla {
            inline: [0u8; N],
            heap: if len > N { vec![0u8; len] } else { Vec::new() },
            len,
        }
    }

    #[inline]
    pub fn as_mut(&mut self) -> &mut [u8] {
        if self.len > N {
            &mut self.heap[..]
        } else {
            &mut self.inline[..self.len]
        }
    }
}
