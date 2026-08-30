//! Stand-in for the C `SPX_VLA(__t, __x, __s)` variable length arrays.
//!
//! `app/include/utils.h` defines
//!
//! ```c
//! # define SPX_VLA(__t,__x,__s) __t __x[__s]
//! ```
//!
//! so the C buffers grow with whatever length the caller asks for.  Inside the
//! library the lengths are bounded by the parameter set, but the routines that
//! use them (`thash`, `mgf1_*`, `treehash`, ...) are exported symbols, and an
//! external caller can legitimately pass a larger `inblocks`, `inlen` or
//! `tree_height`.
//!
//! [`Vla`] therefore keeps an inline buffer sized for the library's own worst
//! case — which is what every internal call hits, so nothing is allocated on
//! the hot path — and falls back to the heap for anything larger.

/// A zero-initialised byte buffer of a length only known at run time.
///
/// `CAP` is the inline capacity; requests above it are served from the heap.
pub struct Vla<const CAP: usize> {
    inline: [u8; CAP],
    heap: Vec<u8>,
    len: usize,
}

impl<const CAP: usize> Vla<CAP> {
    /// `__t __x[len]`, zero initialised (the C arrays that matter are either
    /// fully overwritten before use or explicitly `= {0}`).
    #[inline]
    pub fn new(len: usize) -> Self {
        Vla {
            inline: [0u8; CAP],
            heap: if len > CAP { vec![0u8; len] } else { Vec::new() },
            len,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        if self.len > CAP {
            &self.heap[..self.len]
        } else {
            &self.inline[..self.len]
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.len > CAP {
            &mut self.heap[..self.len]
        } else {
            &mut self.inline[..self.len]
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A `uint32_t`/`unsigned int` counterpart of [`Vla`], for `heights` in
/// `treehash`.
pub struct VlaU32<const CAP: usize> {
    inline: [u32; CAP],
    heap: Vec<u32>,
    len: usize,
}

impl<const CAP: usize> VlaU32<CAP> {
    #[inline]
    pub fn new(len: usize) -> Self {
        VlaU32 {
            inline: [0u32; CAP],
            heap: if len > CAP { vec![0u32; len] } else { Vec::new() },
            len,
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        if self.len > CAP {
            &mut self.heap[..self.len]
        } else {
            &mut self.inline[..self.len]
        }
    }
}
